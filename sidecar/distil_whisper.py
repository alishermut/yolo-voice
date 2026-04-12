"""
YOLO Voice - Distil-Whisper sidecar.

Runs Distil-Whisper through a simple stdin/stdout JSON protocol for the main
offline product flow.
"""

import base64
import io
import json
import os
import sys
import time
from typing import Optional


DISTIL_WHISPER_REPO = "distil-whisper/distil-large-v3"
RUNTIME_NAME = "transformers-distil-whisper"
CHUNK_THRESHOLD_S = 30.0
CHUNK_LENGTH_S = 25.0
BATCH_SIZE = 8

_loaded = False
_pipe = None
_device = "cpu"
_torch_dtype = None


def log(message: str) -> None:
    print(f"[distil-whisper] {message}", file=sys.stderr, flush=True)


def respond(payload: dict) -> None:
    print(json.dumps(payload), flush=True)


def detect_device(preference: str = "auto") -> str:
    if preference == "cpu":
        return "cpu"

    try:
        import torch

        if torch.cuda.is_available():
            return "cuda:0"
        if preference == "gpu":
            log("GPU preference requested, but CUDA is unavailable. Falling back to CPU.")
    except Exception as exc:
        log(f"CUDA detection failed: {exc}")
    return "cpu"


def cuda_available() -> bool:
    try:
        import torch

        return bool(torch.cuda.is_available())
    except Exception:
        return False


def ensure_loaded(model_source: str, device_preference: str = "auto") -> None:
    global _loaded, _pipe, _device, _torch_dtype

    if _loaded and _pipe is not None:
        return

    import torch
    from transformers import pipeline

    _device = detect_device(device_preference)
    _torch_dtype = torch.float16 if _device.startswith("cuda") else torch.float32
    pipeline_device = 0 if _device.startswith("cuda") else -1

    log(f"Loading Distil-Whisper from {model_source} on {_device}")
    _pipe = pipeline(
        "automatic-speech-recognition",
        model=model_source,
        torch_dtype=_torch_dtype,
        device=pipeline_device,
        trust_remote_code=True,
    )
    tokenizer = getattr(_pipe, "tokenizer", None)
    model = getattr(_pipe, "model", None)
    eos_token_id = getattr(getattr(model, "config", None), "eos_token_id", None)
    if tokenizer is not None and getattr(tokenizer, "pad_token_id", None) is None and eos_token_id is not None:
        tokenizer.pad_token_id = eos_token_id
        if getattr(tokenizer, "pad_token", None) is None and getattr(tokenizer, "eos_token", None) is not None:
            tokenizer.pad_token = tokenizer.eos_token
    if model is not None and getattr(getattr(model, "config", None), "pad_token_id", None) is None and eos_token_id is not None:
        model.config.pad_token_id = eos_token_id
    _loaded = True
    log(f"Loaded Distil-Whisper on {_device}")


def download_model(target_dir: str) -> dict:
    from huggingface_hub import snapshot_download

    started = time.time()
    os.environ.setdefault("HF_HUB_DISABLE_PROGRESS_BARS", "1")
    local_dir = snapshot_download(
        repo_id=DISTIL_WHISPER_REPO,
        local_dir=target_dir,
    )
    total_time = time.time() - started
    log(f"Downloaded {DISTIL_WHISPER_REPO} to {local_dir} in {total_time:.2f}s")
    return {"model_path": local_dir, "download_time": round(total_time, 2)}


def decode_wav_bytes(audio_data: str):
    import numpy as np
    import soundfile as sf

    wav_bytes = base64.b64decode(audio_data)
    audio_array, sample_rate = sf.read(io.BytesIO(wav_bytes), dtype="float32")
    if getattr(audio_array, "ndim", 1) > 1:
        audio_array = np.mean(audio_array, axis=1, dtype="float32")
    return audio_array, int(sample_rate)


def target_sample_rate() -> int:
    feature_extractor = getattr(_pipe, "feature_extractor", None)
    sampling_rate = getattr(feature_extractor, "sampling_rate", None)
    return int(sampling_rate) if sampling_rate else 16000


def resample_audio(audio_array, sample_rate: int, target_rate: int):
    import numpy as np

    if sample_rate <= 0 or target_rate <= 0 or sample_rate == target_rate:
        return np.ascontiguousarray(audio_array, dtype="float32")
    if len(audio_array) == 0:
        return np.ascontiguousarray(audio_array, dtype="float32")

    target_length = max(int(round(len(audio_array) * float(target_rate) / float(sample_rate))), 1)
    if target_length == len(audio_array):
        return np.ascontiguousarray(audio_array, dtype="float32")

    source_positions = np.linspace(0.0, len(audio_array) - 1, num=len(audio_array), dtype="float32")
    target_positions = np.linspace(0.0, len(audio_array) - 1, num=target_length, dtype="float32")
    resampled = np.interp(target_positions, source_positions, audio_array).astype("float32", copy=False)
    return np.ascontiguousarray(resampled, dtype="float32")


def transcribe_audio(audio_data: str) -> dict:
    if _pipe is None or not _loaded:
        raise RuntimeError("Distil-Whisper is not loaded")

    total_start = time.time()
    audio_array, sample_rate = decode_wav_bytes(audio_data)
    duration = float(len(audio_array) / sample_rate) if sample_rate > 0 else 0.0
    model_sample_rate = target_sample_rate()
    if sample_rate != model_sample_rate:
        log(f"Resampling Distil-Whisper audio from {sample_rate}Hz to {model_sample_rate}Hz")
        audio_array = resample_audio(audio_array, sample_rate, model_sample_rate)
        sample_rate = model_sample_rate

    effective_mode = "single_pass"
    kwargs = {}
    if duration > CHUNK_THRESHOLD_S:
        effective_mode = "chunked"
        kwargs["chunk_length_s"] = CHUNK_LENGTH_S
        kwargs["batch_size"] = BATCH_SIZE

    inference_start = time.time()
    result = _pipe(
        {"array": audio_array, "sampling_rate": sample_rate},
        **kwargs,
    )
    processing_time = time.time() - inference_start
    total_time = time.time() - total_start

    text = result["text"] if isinstance(result, dict) else str(result)
    return {
        "text": text,
        "duration": round(duration, 2),
        "processing_time": round(processing_time, 2),
        "total_time": round(total_time, 2),
        "device": _device,
        "runtime": RUNTIME_NAME,
        "requested_mode": "single_pass",
        "effective_mode": effective_mode,
    }


def handle_ping(_req: dict) -> None:
    respond(
        {
            "status": "ok",
            "cmd": "ping",
            "device": detect_device(),
            "gpu_available": cuda_available(),
        }
    )


def handle_download_model(req: dict) -> None:
    target_dir = req.get("target_dir")
    if not target_dir:
        respond({"status": "error", "cmd": "download_model", "message": "Missing target_dir"})
        return

    try:
        payload = download_model(target_dir)
        payload.update({"status": "ok", "cmd": "download_model", "model_id": DISTIL_WHISPER_REPO})
        respond(payload)
    except Exception as exc:
        log(f"Failed to download Distil-Whisper: {exc}")
        respond({"status": "error", "cmd": "download_model", "message": str(exc)})


def handle_load_model(req: dict) -> None:
    model_source = req.get("model_source")
    if not model_source:
        respond({"status": "error", "cmd": "load_model", "message": "Missing model_source"})
        return

    try:
        ensure_loaded(model_source, req.get("device_preference", "auto"))
        respond(
            {
                "status": "ok",
                "cmd": "load_model",
                "device": _device,
                "gpu_available": cuda_available(),
            }
        )
    except Exception as exc:
        log(f"Failed to load Distil-Whisper: {exc}")
        respond({"status": "error", "cmd": "load_model", "message": str(exc)})


def handle_transcribe_audio(req: dict) -> None:
    audio_data = req.get("audio_data")
    if not audio_data:
        respond({"status": "error", "cmd": "transcribe_audio", "message": "Missing audio_data"})
        return

    try:
        payload = transcribe_audio(audio_data)
        payload.update({"status": "ok", "cmd": "transcribe_audio"})
        respond(payload)
    except Exception as exc:
        log(f"Distil-Whisper transcription failed: {exc}")
        respond({"status": "error", "cmd": "transcribe_audio", "message": str(exc)})


def main() -> int:
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            req = json.loads(line)
        except json.JSONDecodeError as exc:
            respond({"status": "error", "message": f"Invalid JSON: {exc}"})
            continue

        cmd = req.get("cmd")
        if cmd == "ping":
            handle_ping(req)
        elif cmd == "download_model":
            handle_download_model(req)
        elif cmd == "load_model":
            handle_load_model(req)
        elif cmd == "transcribe_audio":
            handle_transcribe_audio(req)
        elif cmd == "shutdown":
            respond({"status": "ok", "cmd": "shutdown"})
            return 0
        else:
            respond({"status": "error", "message": f"Unknown command: {cmd}"})

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
