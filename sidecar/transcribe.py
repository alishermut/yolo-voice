"""
YOLO Voice - faster-whisper sidecar process.

Long-running process that communicates via stdin/stdout JSON (one object per line).
All logging goes to stderr. stdout is reserved for JSON responses only.
"""

import json
import os
import sys
import time


def log(msg: str) -> None:
    """Log to stderr (never stdout)."""
    print(f"[sidecar] {msg}", file=sys.stderr, flush=True)


def respond(data: dict) -> None:
    """Write a JSON response line to stdout."""
    print(json.dumps(data), flush=True)


# ---------------------------------------------------------------------------
# GPU detection
# ---------------------------------------------------------------------------

def detect_gpu() -> bool:
    """Check if CUDA is actually usable for inference."""
    try:
        import ctranslate2
        # Step 1: Check if ctranslate2 reports CUDA compute types
        types = ctranslate2.get_supported_compute_types("cuda")
        log(f"CUDA compute types reported: {types}")
        if not types:
            log("No CUDA compute types available")
            return False

        # Step 2: Actually verify CUDA works by creating a small storage
        # (get_supported_compute_types can return types even when CUDA runtime is missing)
        try:
            import numpy as np
            storage = ctranslate2.StorageView.from_array(np.zeros((2, 2), dtype=np.float32))
            cuda_storage = storage.to("cuda")
            log("CUDA verification: successfully moved tensor to GPU")
            return True
        except Exception as e:
            log(f"CUDA verification failed (runtime not working): {e}")
            return False

    except Exception as e:
        log(f"CUDA detection via ctranslate2 failed: {e}")

    try:
        import torch
        return torch.cuda.is_available()
    except Exception:
        return False


GPU_AVAILABLE = detect_gpu()
log(f"GPU available: {GPU_AVAILABLE}")

# ---------------------------------------------------------------------------
# Global model reference
# ---------------------------------------------------------------------------

_model = None
_model_name = None


# ---------------------------------------------------------------------------
# Command handlers
# ---------------------------------------------------------------------------

def handle_ping(_req: dict) -> None:
    respond({"status": "ok", "cmd": "ping", "gpu_available": GPU_AVAILABLE})


def handle_load_model(req: dict) -> None:
    global _model, _model_name

    model_size = req.get("model", "base")
    device = req.get("device", "auto")
    compute_type = req.get("compute_type", "float16")
    models_dir = req.get("models_dir")

    # Resolve device
    if device == "auto":
        device = "cuda" if GPU_AVAILABLE else "cpu"
    if device == "cuda" and not GPU_AVAILABLE:
        device = "cpu"
        log("CUDA requested but not available, falling back to CPU")

    # Adjust compute_type for CPU
    if device == "cpu" and compute_type == "float16":
        compute_type = "int8"
        log(f"Adjusted compute_type to int8 for CPU")

    try:
        from faster_whisper import WhisperModel

        log(f"Loading model '{model_size}' on {device} ({compute_type})...")
        kwargs = {
            "device": device,
            "compute_type": compute_type,
        }
        if models_dir:
            kwargs["download_root"] = models_dir

        _model = WhisperModel(model_size, **kwargs)
        _model_name = model_size
        log(f"Model '{model_size}' loaded successfully")
        respond({
            "status": "ok",
            "cmd": "load_model",
            "model": model_size,
            "device": device,
            "compute_type": compute_type,
        })
    except Exception as e:
        log(f"Failed to load model: {e}")
        respond({"status": "error", "cmd": "load_model", "message": str(e)})


def handle_transcribe(req: dict) -> None:
    global _model

    if _model is None:
        respond({
            "status": "error",
            "cmd": "transcribe",
            "message": "No model loaded. Download and load a model first.",
        })
        return

    wav_path = req.get("wav_path", "")
    language = req.get("language", "en")

    if not os.path.isfile(wav_path):
        respond({
            "status": "error",
            "cmd": "transcribe",
            "message": f"WAV file not found: {wav_path}",
        })
        return

    try:
        start_time = time.time()

        transcribe_kwargs = {
            "vad_filter": True,
            "vad_parameters": {"min_silence_duration_ms": 500},
        }
        if language and language != "auto":
            transcribe_kwargs["language"] = language

        segments, info = _model.transcribe(wav_path, **transcribe_kwargs)

        # Collect all segment texts
        text_parts = []
        for segment in segments:
            text_parts.append(segment.text.strip())

        full_text = " ".join(text_parts).strip()
        processing_time = time.time() - start_time

        log(f"Transcribed {info.duration:.1f}s audio in {processing_time:.1f}s: '{full_text[:80]}...'")
        respond({
            "status": "ok",
            "cmd": "transcribe",
            "text": full_text,
            "language": info.language,
            "duration": round(info.duration, 2),
            "processing_time": round(processing_time, 2),
        })
    except Exception as e:
        log(f"Transcription error: {e}")
        respond({"status": "error", "cmd": "transcribe", "message": str(e)})


def handle_list_models(req: dict) -> None:
    models_dir = req.get("models_dir", "")

    if not models_dir or not os.path.isdir(models_dir):
        respond({"status": "ok", "cmd": "list_models", "models": []})
        return

    models = []
    try:
        for entry in os.listdir(models_dir):
            entry_path = os.path.join(models_dir, entry)
            if os.path.isdir(entry_path):
                # Calculate directory size
                total_size = 0
                for dirpath, _dirnames, filenames in os.walk(entry_path):
                    for f in filenames:
                        fp = os.path.join(dirpath, f)
                        try:
                            total_size += os.path.getsize(fp)
                        except OSError:
                            pass

                # Extract model name from HuggingFace cache format
                # e.g. "models--Systran--faster-whisper-base" -> "base"
                name = entry
                if "faster-whisper-" in entry:
                    name = entry.split("faster-whisper-")[-1]
                elif "models--" in entry:
                    parts = entry.split("--")
                    if len(parts) >= 3:
                        name = parts[-1]
                        if name.startswith("faster-whisper-"):
                            name = name[len("faster-whisper-"):]

                models.append({
                    "name": name,
                    "size_mb": round(total_size / (1024 * 1024)),
                })
    except Exception as e:
        log(f"Error listing models: {e}")

    respond({"status": "ok", "cmd": "list_models", "models": models})


def handle_download_model(req: dict) -> None:
    model_name = req.get("model", "base")
    models_dir = req.get("models_dir", "")

    if not models_dir:
        respond({
            "status": "error",
            "cmd": "download_model",
            "message": "models_dir not specified",
        })
        return

    os.makedirs(models_dir, exist_ok=True)

    repo_id = f"Systran/faster-whisper-{model_name}"
    log(f"Downloading model '{model_name}' from {repo_id}...")

    try:
        from huggingface_hub import snapshot_download

        # Track progress via a custom callback
        last_percent = [0.0]

        def progress_callback(current: int, total: int):
            if total > 0:
                percent = round((current / total) * 100, 1)
                # Only emit if progress changed by >= 1%
                if percent - last_percent[0] >= 1.0 or percent >= 100:
                    last_percent[0] = percent
                    respond({
                        "status": "progress",
                        "cmd": "download_model",
                        "model": model_name,
                        "percent": percent,
                        "downloaded_mb": round(current / (1024 * 1024)),
                        "total_mb": round(total / (1024 * 1024)),
                    })

        # Download using huggingface_hub
        snapshot_download(
            repo_id=repo_id,
            local_dir=os.path.join(models_dir, f"faster-whisper-{model_name}"),
            local_dir_use_symlinks=False,
        )

        log(f"Model '{model_name}' downloaded successfully")
        respond({"status": "ok", "cmd": "download_model", "model": model_name})

    except Exception as e:
        log(f"Download error: {e}")
        respond({
            "status": "error",
            "cmd": "download_model",
            "message": str(e),
        })


def handle_shutdown(_req: dict) -> None:
    log("Shutdown requested")
    respond({"status": "ok", "cmd": "shutdown"})
    sys.exit(0)


# ---------------------------------------------------------------------------
# LLM Post-Processing
# ---------------------------------------------------------------------------

# Path to bundled default profiles (next to this script)
_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
_DEFAULT_PROFILES_PATH = os.path.join(_SCRIPT_DIR, "default_profiles.json")


def _ensure_profiles_dir(profiles_dir: str) -> None:
    """Create profiles dir and copy defaults if empty."""
    os.makedirs(profiles_dir, exist_ok=True)

    # If no profiles exist yet, seed from defaults
    existing = [f for f in os.listdir(profiles_dir) if f.endswith(".json")]
    if not existing and os.path.isfile(_DEFAULT_PROFILES_PATH):
        try:
            with open(_DEFAULT_PROFILES_PATH, "r", encoding="utf-8") as f:
                defaults = json.load(f)
            for profile in defaults:
                pid = profile.get("id", "unknown")
                path = os.path.join(profiles_dir, f"{pid}.json")
                with open(path, "w", encoding="utf-8") as f:
                    json.dump(profile, f, indent=2)
            log(f"Seeded {len(defaults)} default profiles")
        except Exception as e:
            log(f"Failed to seed default profiles: {e}")


def handle_list_profiles(req: dict) -> None:
    profiles_dir = req.get("profiles_dir", "")
    if not profiles_dir:
        respond({"status": "error", "cmd": "list_profiles", "message": "profiles_dir not specified"})
        return

    _ensure_profiles_dir(profiles_dir)

    profiles = []
    try:
        for fname in sorted(os.listdir(profiles_dir)):
            if not fname.endswith(".json"):
                continue
            fpath = os.path.join(profiles_dir, fname)
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    profile = json.load(f)
                profiles.append(profile)
            except Exception as e:
                log(f"Failed to read profile {fname}: {e}")
    except Exception as e:
        log(f"Error listing profiles: {e}")

    respond({"status": "ok", "cmd": "list_profiles", "profiles": profiles})


def handle_save_profile(req: dict) -> None:
    profiles_dir = req.get("profiles_dir", "")
    profile = req.get("profile")

    if not profiles_dir or not profile:
        respond({"status": "error", "cmd": "save_profile", "message": "profiles_dir and profile required"})
        return

    os.makedirs(profiles_dir, exist_ok=True)
    pid = profile.get("id", "unknown")
    path = os.path.join(profiles_dir, f"{pid}.json")

    try:
        with open(path, "w", encoding="utf-8") as f:
            json.dump(profile, f, indent=2)
        log(f"Saved profile '{pid}'")
        respond({"status": "ok", "cmd": "save_profile"})
    except Exception as e:
        respond({"status": "error", "cmd": "save_profile", "message": str(e)})


def handle_delete_profile(req: dict) -> None:
    profiles_dir = req.get("profiles_dir", "")
    pid = req.get("id", "")

    if not profiles_dir or not pid:
        respond({"status": "error", "cmd": "delete_profile", "message": "profiles_dir and id required"})
        return

    path = os.path.join(profiles_dir, f"{pid}.json")
    try:
        if os.path.isfile(path):
            os.remove(path)
            log(f"Deleted profile '{pid}'")
        respond({"status": "ok", "cmd": "delete_profile"})
    except Exception as e:
        respond({"status": "error", "cmd": "delete_profile", "message": str(e)})


def _call_ollama(text: str, system_prompt: str, model: str, base_url: str) -> str:
    """Call Ollama's generate API."""
    import requests

    url = f"{base_url.rstrip('/')}/api/generate"
    payload = {
        "model": model,
        "prompt": text,
        "system": system_prompt,
        "stream": False,
    }

    resp = requests.post(url, json=payload, timeout=30)
    resp.raise_for_status()
    data = resp.json()
    return data.get("response", "").strip()


def _call_openai(text: str, system_prompt: str, model: str, api_key: str, base_url: str) -> str:
    """Call OpenAI-compatible chat completions API."""
    import requests

    url = f"{base_url.rstrip('/')}/v1/chat/completions"
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }
    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": text},
        ],
        "temperature": 0.3,
    }

    resp = requests.post(url, json=payload, headers=headers, timeout=30)
    resp.raise_for_status()
    data = resp.json()
    return data["choices"][0]["message"]["content"].strip()


def _call_claude(text: str, system_prompt: str, model: str, api_key: str) -> str:
    """Call Anthropic's Messages API."""
    import requests

    url = "https://api.anthropic.com/v1/messages"
    headers = {
        "x-api-key": api_key,
        "anthropic-version": "2023-06-01",
        "Content-Type": "application/json",
    }
    payload = {
        "model": model,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [
            {"role": "user", "content": text},
        ],
    }

    resp = requests.post(url, json=payload, headers=headers, timeout=30)
    resp.raise_for_status()
    data = resp.json()
    # Extract text from content blocks
    content = data.get("content", [])
    return "".join(block.get("text", "") for block in content).strip()


def handle_post_process(req: dict) -> None:
    text = req.get("text", "")
    profile = req.get("profile", {})
    provider = req.get("provider", "ollama")
    api_key = req.get("api_key", "")
    model = req.get("model", "")
    base_url = req.get("base_url", "http://localhost:11434")

    if not text.strip():
        respond({"status": "ok", "cmd": "post_process", "text": ""})
        return

    system_prompt = profile.get("system_prompt", "Fix grammar and punctuation. Output only the corrected text.")

    # Append dictionary context if present
    dictionary = profile.get("dictionary", [])
    if dictionary:
        dict_str = ", ".join(dictionary)
        system_prompt += f"\n\nImportant terminology to preserve exactly: {dict_str}"

    try:
        start_time = time.time()

        if provider == "ollama":
            result = _call_ollama(text, system_prompt, model or "llama3.1:8b", base_url)
        elif provider == "openai":
            result = _call_openai(text, system_prompt, model or "gpt-4o-mini", api_key, base_url or "https://api.openai.com")
        elif provider == "claude":
            result = _call_claude(text, system_prompt, model or "claude-sonnet-4-20250514", api_key)
        else:
            respond({"status": "error", "cmd": "post_process", "message": f"Unknown provider: {provider}"})
            return

        elapsed = time.time() - start_time
        log(f"Post-processed in {elapsed:.1f}s via {provider}: '{result[:80]}...'")
        respond({"status": "ok", "cmd": "post_process", "text": result})

    except Exception as e:
        log(f"Post-processing error ({provider}): {e}")
        respond({"status": "error", "cmd": "post_process", "message": str(e)})


# ---------------------------------------------------------------------------
# Cloud Transcription (Phase 6)
# ---------------------------------------------------------------------------

def handle_cloud_transcribe(req: dict) -> None:
    """Transcribe audio via cloud API (Groq or Deepgram)."""
    import requests

    wav_path = req.get("wav_path", "")
    provider = req.get("provider", "groq")
    api_key = req.get("api_key", "")
    language = req.get("language", "en")

    if not os.path.isfile(wav_path):
        respond({"status": "error", "cmd": "cloud_transcribe", "message": f"WAV file not found: {wav_path}"})
        return

    if not api_key:
        respond({"status": "error", "cmd": "cloud_transcribe", "message": "API key required for cloud transcription"})
        return

    try:
        start_time = time.time()

        if provider == "groq":
            text = _cloud_groq(wav_path, api_key, language)
        elif provider == "deepgram":
            text = _cloud_deepgram(wav_path, api_key, language)
        else:
            respond({"status": "error", "cmd": "cloud_transcribe", "message": f"Unknown cloud provider: {provider}"})
            return

        elapsed = time.time() - start_time
        log(f"Cloud transcribed via {provider} in {elapsed:.1f}s: '{text[:80]}...'")
        respond({
            "status": "ok",
            "cmd": "cloud_transcribe",
            "text": text.strip(),
            "processing_time": round(elapsed, 2),
        })

    except Exception as e:
        log(f"Cloud transcription error ({provider}): {e}")
        respond({"status": "error", "cmd": "cloud_transcribe", "message": str(e)})


def _cloud_groq(wav_path: str, api_key: str, language: str) -> str:
    """Transcribe via Groq Whisper API (OpenAI-compatible)."""
    import requests

    url = "https://api.groq.com/openai/v1/audio/transcriptions"
    headers = {"Authorization": f"Bearer {api_key}"}

    with open(wav_path, "rb") as f:
        files = {"file": ("recording.wav", f, "audio/wav")}
        data = {"model": "whisper-large-v3-turbo", "response_format": "json"}
        if language and language != "auto":
            data["language"] = language

        resp = requests.post(url, headers=headers, files=files, data=data, timeout=60)
        resp.raise_for_status()
        return resp.json().get("text", "")


def _cloud_deepgram(wav_path: str, api_key: str, language: str) -> str:
    """Transcribe via Deepgram API."""
    import requests

    lang = language if language and language != "auto" else "en"
    url = f"https://api.deepgram.com/v1/listen?model=nova-2&language={lang}&smart_format=true"
    headers = {
        "Authorization": f"Token {api_key}",
        "Content-Type": "audio/wav",
    }

    with open(wav_path, "rb") as f:
        resp = requests.post(url, headers=headers, data=f, timeout=60)
        resp.raise_for_status()
        data = resp.json()

    # Extract transcript from Deepgram response
    channels = data.get("results", {}).get("channels", [])
    if channels:
        alternatives = channels[0].get("alternatives", [])
        if alternatives:
            return alternatives[0].get("transcript", "")
    return ""


# ---------------------------------------------------------------------------
# Command dispatch
# ---------------------------------------------------------------------------

HANDLERS = {
    "ping": handle_ping,
    "load_model": handle_load_model,
    "transcribe": handle_transcribe,
    "list_models": handle_list_models,
    "download_model": handle_download_model,
    "list_profiles": handle_list_profiles,
    "save_profile": handle_save_profile,
    "delete_profile": handle_delete_profile,
    "post_process": handle_post_process,
    "cloud_transcribe": handle_cloud_transcribe,
    "shutdown": handle_shutdown,
}


def main() -> None:
    # Emit ready signal
    respond({"status": "ok", "cmd": "ready", "gpu_available": GPU_AVAILABLE})
    log("Sidecar ready, waiting for commands...")

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            req = json.loads(line)
        except json.JSONDecodeError as e:
            respond({"status": "error", "cmd": "unknown", "message": f"Invalid JSON: {e}"})
            continue

        cmd = req.get("cmd", "")
        handler = HANDLERS.get(cmd)

        if handler:
            try:
                handler(req)
            except Exception as e:
                log(f"Unhandled error in '{cmd}': {e}")
                respond({"status": "error", "cmd": cmd, "message": f"Internal error: {e}"})
        else:
            respond({"status": "error", "cmd": cmd, "message": f"Unknown command: {cmd}"})


if __name__ == "__main__":
    main()
