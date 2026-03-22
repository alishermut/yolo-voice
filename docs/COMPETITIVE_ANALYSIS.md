# YoloVoice Competitive Analysis Report
**Date:** 2026-03-21

---

## Executive Summary

This report analyzes **10 open-source voice dictation / transcription projects** that compete in the same space as YoloVoice. The goal is to identify architectural patterns, feature gaps, and actionable improvements for YoloVoice's roadmap.

**Key finding:** YoloVoice's Tauri + Rust + React + faster-whisper stack is the modern standard — 5 of the 10 competitors use Tauri. However, several competitors have features YoloVoice lacks (smart presets, voice commands, auto-learn dictionary, meeting detection) that represent real improvement opportunities.

---

## 1. Competitor Profiles

---

### 1.1 Buzz (chidiwilliams/buzz)
- **GitHub:** https://github.com/chidiwilliams/buzz
- **Stars:** 18,300 | **Last commit:** Mar 14, 2026 | **License:** MIT
- **Stack:** Python (99%), PyQt
- **Platforms:** Windows, macOS, Linux

**What it does:** Primarily a **file transcription** tool (audio/video files, YouTube links), but also supports live microphone transcription. Not a dictation tool — it doesn't type text into other apps.

**Architecture:** Monolithic Python/PyQt desktop app. Supports multiple Whisper backends (whisper.cpp, faster-whisper, HuggingFace models, OpenAI API). Vulkan GPU acceleration for broad GPU support (NVIDIA, AMD, Intel integrated).

**Key features:**
- Speaker diarization (identify who's speaking)
- Speech separation (remove background noise before transcription)
- Export to TXT/SRT/VTT
- Watch folder automation (auto-transcribe new files)
- Advanced transcription viewer with search + playback
- CLI interface for scripting
- YouTube link transcription

**Where it's better than YoloVoice:**
- Massive community (18.3k stars)
- File/video transcription capabilities
- Speaker diarization
- Vulkan GPU support (not just CUDA)
- Watch folder automation

**Where it's worse than YoloVoice:**
- Not a dictation tool (doesn't insert text into apps)
- No hotkey-driven workflow
- No VAD for live recording
- No LLM post-processing
- No text insertion into focused windows
- Python/PyQt is slower and larger than Tauri/Rust

**Relevance to YoloVoice:** Low for core dictation features, but the diarization and Vulkan GPU support are worth noting.

---

### 1.2 Handy (cjpais/Handy)
- **GitHub:** https://github.com/cjpais/Handy
- **Stars:** 18,200 | **Last commit:** Feb 2025 | **License:** MIT
- **Stack:** Rust + TypeScript, Tauri, React
- **Platforms:** Windows, macOS, Linux

**What it does:** Desktop voice dictation app with toggle/push-to-talk modes. Designed to be "the most forkable STT app."

**Architecture:** Rust backend (CPAL for audio, whisper-rs for inference, rdev for global hotkeys) + Tauri bridge + React settings UI. Very similar to YoloVoice's architecture.

**Key features:**
- Toggle or push-to-talk modes
- Silero VAD
- Parakeet V2/V3 model support (NVIDIA's alternative to Whisper — 5x realtime on CPU)
- Whisper model support (small/medium/turbo/large via whisper.cpp)
- Auto language detection
- Debug mode (Ctrl+Shift+D)
- CLI parameters for remote control
- `winget install cjpais.Handy` on Windows

**Where it's better than YoloVoice:**
- **Parakeet model support** — dramatically faster CPU inference
- **winget installation** — one-command Windows install
- Whisper.cpp integration (Rust-native via whisper-rs, no Python sidecar needed)
- Huge star count / community
- Explicitly "forkable" design philosophy

**Where it's worse than YoloVoice:**
- No LLM post-processing
- No custom vocabulary / replacement rules
- No industry packs
- No profiles system
- No cloud transcription fallback
- Simpler UI (no onboarding wizard, no model selector UX)
- Last commit was Feb 2025 (may be slowing down)

**Relevance to YoloVoice:** **HIGH** — closest architectural twin. The Parakeet model support and elimination of the Python sidecar (using whisper-rs instead) are the biggest takeaways.

---

### 1.3 Whispering / Epicenter (EpicenterHQ/epicenter)
- **GitHub:** https://github.com/EpicenterHQ/epicenter
- **Stars:** 4,300 | **Last commit:** Dec 2025 | **License:** MIT (apps) / AGPL-3.0 (sync)
- **Stack:** Svelte 5 + TypeScript (70%) + Rust (3%), Tauri
- **Platforms:** Windows, macOS, Linux, Chrome Extension

**What it does:** Local-first voice dictation with CRDT-based sync across devices. Press shortcut → speak → text pasted at cursor.

**Architecture:** Tauri desktop shell + Svelte frontend. Yjs CRDTs as single source of truth, materialized to SQLite (fast reads) and markdown (human-readable). Self-hostable sync server.

**Key features:**
- Whisper.cpp local inference + BYOK cloud APIs (Groq, etc.)
- Custom transform chains on transcribed text (post-processing pipeline)
- CRDT sync across devices
- Chrome extension companion
- Local-first data philosophy (SQLite + plain text markdown)
- Self-hostable sync server

**Where it's better than YoloVoice:**
- **Transform pipeline** — composable post-processing chains
- Cross-device sync
- Chrome extension
- Multiple deployment targets (desktop, web, extension)
- SQLite-based history with search

**Where it's worse than YoloVoice:**
- More complex architecture (CRDTs may be over-engineering for dictation)
- No VAD
- No custom vocabulary or replacement rules
- No profiles
- Rust usage is minimal (3%) — most logic in TypeScript
- Development appears to have slowed (last release Dec 2025)

**Relevance to YoloVoice:** MEDIUM — the transform pipeline concept is worth borrowing. History/search is a nice-to-have.

---

### 1.4 OpenWhispr (OpenWhispr/openwhispr)
- **GitHub:** https://github.com/OpenWhispr/openwhispr
- **Stars:** 2,000 | **Last commit:** Jan 2025 | **License:** MIT
- **Stack:** Electron, React 19, TypeScript, Tailwind v4, Node.js 22+, SQLite with FTS5
- **Platforms:** Windows, macOS, Linux

**What it does:** Feature-rich voice dictation app with AI agent overlay, meeting transcription, and auto-learn dictionary.

**Architecture:** Electron main/renderer process split. Custom native C binaries per platform for text pasting (Windows uses Win32 `SendInput`). SQLite for transcription history with full-text search.

**Key features:**
- **Native C paste binary** — custom Win32 SendInput for reliable text insertion with terminal detection
- **Auto-learn dictionary** — detects when user corrects transcription, learns from corrections
- **AI agent overlay** — glassmorphism UI for AI assistant commands
- **Meeting transcription** — auto-detects Zoom/Teams/FaceTime processes + sustained audio + calendar awareness
- Agent naming ("Hey [name]" to distinguish dictation from AI commands)
- Google Calendar integration
- 25-language support
- Full-text search notes system
- NVIDIA Parakeet model support
- LLM support via llama.cpp (Qwen/LLaMA/Mistral/Gemma)

**Where it's better than YoloVoice:**
- **Auto-learn dictionary** is a killer feature YoloVoice doesn't have
- **Meeting detection + transcription** is a major differentiator
- **Native C SendInput binary** for text insertion (more reliable than clipboard)
- Parakeet model support
- Local LLM support (no cloud dependency)
- Full-text search across transcription history
- Calendar integration

**Where it's worse than YoloVoice:**
- **Electron** — much larger memory footprint and bundle size vs Tauri
- More complex, potentially harder to maintain
- Slower startup time
- Native C binaries per platform add build complexity
- Development slowed (last commit Jan 2025)

**Relevance to YoloVoice:** **HIGH** — auto-learn dictionary, meeting detection, and the native text insertion approach are all worth studying.

---

### 1.5 WhisperWriter (savbell/whisper-writer)
- **GitHub:** https://github.com/savbell/whisper-writer
- **Stars:** 1,000 | **Last commit:** Jun 2024 | **License:** GPL-3.0
- **Stack:** Python (100%), PyQt5, faster-whisper, pynput, sounddevice
- **Platforms:** Windows, macOS, Linux

**What it does:** Simple dictation app — press hotkey, speak, text is typed into active window via keyboard simulation.

**Architecture:** PyQt5 desktop app with background keyboard listener. Uses faster-whisper for local transcription or OpenAI API. Keyboard simulation via pynput for text insertion.

**Key features:**
- Four recording modes: continuous, VAD, press-to-toggle, hold-to-record
- Local (faster-whisper) or cloud (OpenAI API) transcription
- Silero VAD filtering
- Configurable keyboard shortcut activation
- Post-processing: trailing period/space removal, capitalization control
- CUDA GPU support (cuBLAS + cuDNN)
- Custom initial prompts for Whisper conditioning

**Where it's better than YoloVoice:**
- Four recording modes (YoloVoice has hold + double-tap)
- Whisper initial prompt conditioning (improves domain-specific accuracy)
- Simpler, easier to understand codebase

**Where it's worse than YoloVoice:**
- Pure Python — slower startup, larger memory
- Keyboard simulation (pynput) is less reliable than SendInput
- No LLM post-processing
- No custom vocabulary
- No profiles
- No UI beyond a status window
- No onboarding
- GPL-3.0 license (more restrictive)
- Development appears stalled (last commit Jun 2024)

**Relevance to YoloVoice:** LOW — simpler feature set, older architecture. The Whisper initial prompt conditioning is worth noting though.

---

### 1.6 VoiceTypr (moinulmoin/voicetypr)
- **GitHub:** https://github.com/moinulmoin/voicetypr
- **Stars:** 334 | **Last commit:** Feb 2026 | **License:** AGPL-3.0
- **Stack:** Rust (56%) + TypeScript (38%), Tauri, React, Tailwind, shadcn/ui
- **Platforms:** Windows, macOS

**What it does:** Open-source alternative to SuperWhisper/Wispr Flow. Voice dictation with AI enhancement.

**Architecture:** React frontend + Rust backend (audio recording, Whisper integration, Tauri commands). Automatic GPU detection at install time.

**Key features:**
- **Smart presets** — different post-processing for different use cases (prompts, email, commits, notes)
- AI enhancement via Groq/Gemini APIs
- 99+ language support
- Multiple Whisper model sizes with hardware acceleration
- Onboarding with model selection
- Auto GPU detection (Metal, NVIDIA, AMD, Intel)

**Where it's better than YoloVoice:**
- **Smart presets** concept — context-aware post-processing is more polished
- Auto GPU detection across all vendors
- shadcn/ui for polished component library
- 99+ languages (YoloVoice focuses on fewer)

**Where it's worse than YoloVoice:**
- No custom vocabulary / replacement rules
- No industry packs
- Cloud-only LLM (Groq/Gemini, no local)
- Smaller community
- AGPL-3.0 license

**Relevance to YoloVoice:** **MEDIUM-HIGH** — smart presets and auto GPU detection are worth borrowing. Same tech stack makes it easy to compare approaches.

---

### 1.7 whisper-key-local (PinW/whisper-key-local)
- **GitHub:** https://github.com/PinW/whisper-key-local
- **Stars:** 117 | **Last commit:** Mar 17, 2026 | **License:** Not specified
- **Stack:** Python (97%), faster-whisper, sounddevice, pywin32, pystray
- **Platforms:** Windows, macOS, Linux

**What it does:** Lightweight system tray dictation app with voice commands.

**Architecture:** Python system tray app. YAML config. Dual text insertion (clipboard paste + character injection).

**Key features:**
- **Voice commands** — spoken triggers that execute hotkeys, insert text snippets, or run shell commands
- Dual text insertion (Ctrl+V paste vs direct character injection)
- VAD to prevent hallucinations on silence
- YAML-based configuration
- Single exe download or pip install
- AMD ROCm GPU support (in addition to NVIDIA CUDA)
- Auto-send with Enter key

**Where it's better than YoloVoice:**
- **Voice commands** (spoken triggers for actions) — unique feature
- AMD ROCm GPU support
- Single exe distribution (no installer needed)
- Extremely lightweight

**Where it's worse than YoloVoice:**
- No GUI beyond system tray
- No LLM post-processing
- No profiles or vocabulary management
- Python-based (heavier runtime)
- Small community

**Relevance to YoloVoice:** MEDIUM — voice commands and dual text insertion methods are worth studying.

---

### 1.8 OmniDictate (gurjar1/OmniDictate)
- **GitHub:** https://github.com/gurjar1/OmniDictate
- **Stars:** 111 | **Last commit:** Dec 2025 | **License:** Not specified
- **Stack:** Python (96%), PyQt, faster-whisper 1.1.1, PyTorch 2.6.0 + CUDA 12.6
- **Platforms:** Windows 10/11 only

**What it does:** Windows-only voice dictation with spoken punctuation commands.

**Architecture:** PyQt desktop app with faster-whisper backend. Uses pywinauto for keyboard simulation.

**Key features:**
- **Spoken punctuation** ("comma", "period", "new line", etc.)
- VAD and push-to-talk modes
- Hallucination filtering
- Dark slate UI with frosted glass effects
- Whisper large-v3-turbo support
- Inno Setup installer or portable .7z

**Where it's better than YoloVoice:**
- **Spoken punctuation commands** — natural way to add punctuation
- Windows-only focus means optimized UX for that platform
- Hallucination filtering

**Where it's worse than YoloVoice:**
- Python/PyQt (heavier)
- pywinauto for text insertion (less reliable)
- No LLM post-processing
- No custom vocabulary
- Small community

**Relevance to YoloVoice:** MEDIUM — spoken punctuation and hallucination filtering are practical features worth implementing.

---

### 1.9 TurboWhisper (knowall-ai/turbo-whisper)
- **GitHub:** https://github.com/knowall-ai/turbo-whisper
- **Stars:** 22 | **Last commit:** Jan 22, 2026 | **License:** Not specified
- **Stack:** Python 3.10+, PyAudio
- **Platforms:** Linux (primary), macOS, Windows

**What it does:** SuperWhisper-like voice dictation for Linux with waveform UI.

**Architecture:** Python app with HTTP server on localhost:7878 for tool integration. Uses xdotool (Linux), keyboard sim (macOS), or pyperclip (Windows) for text insertion.

**Key features:**
- Real-time waveform visualization with animated orb
- Claude Code integration (experimental) — waits for "ready" signal before inserting
- System tray integration
- OpenAI API compatible backend
- Autostart capability
- Global hotkey (Ctrl+Shift+Space)

**Where it's better than YoloVoice:**
- **Claude Code integration** — interesting developer-focused feature
- Waveform visualization

**Where it's worse than YoloVoice:**
- Tiny community (22 stars)
- Linux-focused, Windows is secondary
- No local model inference (requires external server)
- No LLM post-processing
- Minimal features overall

**Relevance to YoloVoice:** LOW — the Claude Code integration concept is interesting but the project is too small.

---

### 1.10 Kalam (afaraha8403/kalam)
- **GitHub:** https://github.com/afaraha8403/kalam
- **Stars:** 3 | **Last commit:** Jan 2025 | **License:** MIT NC + Commercial
- **Stack:** Svelte (58%) + Rust (37%), Tauri
- **Platforms:** Windows (beta)

**What it does:** Voice notes + dictation + productivity tool with todos and reminders.

**Architecture:** Tauri + Svelte + Rust. Encrypted SQLite history. Audio kept in-memory only (never written to disk).

**Key features:**
- Voice notes with tagging/pinning/search
- Voice-to-todo conversion
- Voice reminders
- Text snippets (voice shortcuts for templates)
- SenseVoice model support (Alibaba's Whisper alternative)
- Audio-in-memory-only privacy policy

**Where it's better than YoloVoice:**
- Productivity features (notes, todos, reminders)
- SenseVoice model support
- Privacy-first audio handling

**Where it's worse than YoloVoice:**
- Very early stage (3 stars, beta)
- Minimal community
- Limited documentation

**Relevance to YoloVoice:** LOW — too early stage, but the productivity features are an interesting direction.

---

## 2. Comparative Analysis

### 2.1 Tech Stack Comparison

| Project | Language | Framework | Whisper Backend | Text Insertion |
|---------|----------|-----------|-----------------|----------------|
| **YoloVoice** | Rust + TS | Tauri + React | faster-whisper (Python sidecar) | Win32 SendInput (clipboard) |
| Handy | Rust + TS | Tauri + React | whisper-rs (Rust-native) | System input automation |
| VoiceTypr | Rust + TS | Tauri + React | Whisper (direct) | System text injection |
| Whispering | Svelte + Rust | Tauri + Svelte | whisper.cpp | Shortcut + paste |
| OpenWhispr | TS + React | Electron | whisper.cpp + sherpa-onnx | Native C SendInput binary |
| Buzz | Python | PyQt | Multiple backends | N/A (not dictation) |
| WhisperWriter | Python | PyQt5 | faster-whisper | pynput keyboard sim |
| whisper-key-local | Python | System tray | faster-whisper | Paste + char injection |
| OmniDictate | Python | PyQt | faster-whisper | pywinauto keyboard sim |
| TurboWhisper | Python | Custom | External server | xdotool/pyperclip |

**Key insight:** YoloVoice is the ONLY Tauri+Rust project that uses a **Python sidecar** for Whisper. Handy and VoiceTypr integrate Whisper directly in Rust. This is YoloVoice's biggest architectural gap.

### 2.2 Feature Matrix

| Feature | YoloVoice | Handy | OpenWhispr | VoiceTypr | Whispering | WhisperWriter | Buzz |
|---------|-----------|-------|------------|-----------|------------|---------------|------|
| Hold-to-talk | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Push-to-toggle | ✅ (double-tap) | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Continuous mode | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ |
| VAD | ✅ (Silero) | ✅ (Silero) | ❌ | ❌ | ❌ | ✅ (Silero) | ❌ |
| LLM post-processing | ✅ | ❌ | ✅ (local llama.cpp) | ✅ (Groq/Gemini) | ✅ (transform chains) | ❌ | ❌ |
| Custom vocabulary | ✅ | ❌ | ✅ (auto-learn) | ❌ | ❌ | ❌ | ❌ |
| Profiles | ✅ | ❌ | ❌ | ✅ (presets) | ❌ | ❌ | ❌ |
| Industry packs | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Replacement rules | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Spoken punctuation | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Voice commands | ❌ | ❌ | ✅ (agent naming) | ❌ | ❌ | ❌ | ❌ |
| Auto-learn dictionary | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Meeting detection | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Transcription history | ❌ | ❌ | ✅ (SQLite FTS5) | ❌ | ✅ (CRDT) | ❌ | ✅ |
| File transcription | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Speaker diarization | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Multi-GPU vendors | ❌ (CUDA only) | ✅ | ✅ (CUDA + Parakeet) | ✅ (auto-detect) | ❌ | ✅ (CUDA) | ✅ (Vulkan) |
| Onboarding wizard | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |
| System tray | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

### 2.3 YoloVoice's Competitive Advantages

1. **Industry packs + replacement rules** — No competitor has this. It's a genuine differentiator for professional/domain-specific use.
2. **Profile system** — Only VoiceTypr has something similar (presets). YoloVoice's is more comprehensive.
3. **Onboarding wizard** — Only VoiceTypr also has one. Most competitors drop users into a raw settings page.
4. **Tauri + Rust architecture with rich UI** — Modern, small bundle, fast startup. Better than Electron (OpenWhispr) or Python (WhisperWriter, OmniDictate, Buzz).
5. **Event-driven pill UI** — Clean, non-intrusive recording indicator. Most competitors use basic system tray icons.

### 2.4 YoloVoice's Competitive Weaknesses

1. **Python sidecar dependency** — The biggest architectural gap. Handy/VoiceTypr prove you can do Whisper inference in Rust natively.
2. **CUDA-only GPU support** — VoiceTypr auto-detects all GPU vendors. Buzz has Vulkan. YoloVoice only supports NVIDIA.
3. **No transcription history** — OpenWhispr and Whispering store and search past transcriptions.
4. **No auto-learn dictionary** — OpenWhispr's auto-correction learning is a killer feature.
5. **No spoken punctuation** — OmniDictate handles "comma", "period" etc. naturally.
6. **No Whisper prompt conditioning** — WhisperWriter supports initial prompts that dramatically improve domain-specific accuracy.
7. **No Parakeet model support** — Handy and OpenWhispr support NVIDIA Parakeet which is 5x faster on CPU.
8. **No continuous recording mode** — WhisperWriter has this; useful for long-form dictation.

---

## 3. Architectural Deep-Dive

### 3.1 Text Insertion Approaches (Critical Comparison)

Text insertion into arbitrary Windows apps is the hardest UX problem in this space. Here's how each project solves it:

| Project | Method | Reliability |
|---------|--------|-------------|
| **YoloVoice** | Clipboard + Win32 SendInput (Ctrl+V) | Good but overwrites clipboard |
| **OpenWhispr** | Custom native C binary using SendInput with terminal detection | Best — handles edge cases |
| **Handy** | System input automation (Rust-native) | Good |
| **WhisperWriter** | pynput keyboard simulation | Moderate — can miss chars |
| **whisper-key-local** | Dual: clipboard paste OR direct character injection | Flexible |
| **OmniDictate** | pywinauto keyboard simulation | Moderate |

**Takeaway:** OpenWhispr's approach of a dedicated native binary with terminal detection (different paste behavior for cmd/PowerShell vs regular apps) is the most robust. YoloVoice's clipboard approach works but overwrites the user's clipboard content.

### 3.2 Whisper Integration Approaches

| Approach | Used by | Pros | Cons |
|----------|---------|------|------|
| **Python sidecar (faster-whisper)** | YoloVoice, WhisperWriter, OmniDictate | Easy to set up, good Python ecosystem | Extra process, ~500MB+ bundled Python env, startup latency |
| **Rust-native (whisper-rs / whisper.cpp bindings)** | Handy, VoiceTypr | No sidecar, fast startup, smaller bundle | Less flexible model loading, C++ build complexity |
| **ONNX Runtime (sherpa-onnx)** | OpenWhispr (for Parakeet) | Cross-platform GPU, fast inference | Limited to ONNX-exported models |
| **External server** | TurboWhisper, Whispering | Decoupled, easy to swap backends | Requires separate process management |

**Takeaway:** The Rust-native approach (whisper-rs) eliminates YoloVoice's biggest complexity — the Python sidecar, bundled Python environment, and sidecar lifecycle management. Handy proves this works at scale (18k stars).

### 3.3 UI/UX Comparison

| Project | UI Quality | Notable UX Elements |
|---------|------------|---------------------|
| **YoloVoice** | ⭐⭐⭐⭐ | Pill overlay, onboarding wizard, waveform display, settings tabs |
| **Handy** | ⭐⭐⭐ | Clean but minimal settings page |
| **VoiceTypr** | ⭐⭐⭐⭐ | shadcn/ui components, onboarding, polished design |
| **OpenWhispr** | ⭐⭐⭐⭐ | Glassmorphism AI overlay, full-featured settings |
| **Whispering** | ⭐⭐⭐ | Svelte-based, functional but not flashy |
| **Buzz** | ⭐⭐⭐ | PyQt standard look, functional transcription viewer |
| **WhisperWriter** | ⭐⭐ | Minimal status window only |
| **OmniDictate** | ⭐⭐⭐ | Dark slate with frosted glass — decent aesthetics |

**Takeaway:** YoloVoice has one of the best UIs in the space. VoiceTypr's use of shadcn/ui is worth noting for component quality.

### 3.4 ASR Model Landscape — Beyond Whisper

YoloVoice currently uses OpenAI's Whisper (tiny model via faster-whisper). Two alternative models have emerged as serious contenders that competitors are already adopting.

#### NVIDIA Parakeet TDT 0.6B v3 (Latest: Aug 2025)

| Spec | Details |
|------|---------|
| **Parameters** | 600M |
| **Architecture** | FastConformer-TDT (Transducer with Duration) |
| **Languages** | 25 (auto-detected) — EN, RU, UK, DE, FR, ES, IT, PT, NL, PL, CS, SK, BG, HR, DA, EL, ET, FI, HU, LV, LT, MT, RO, SL, SV |
| **Avg English WER** | **6.34%** (Open ASR Leaderboard) |
| **Best English WER** | 1.93% (LibriSpeech clean) |
| **Russian WER** | Supported — included in 25-language training set (FLEURS/CoVoST benchmarks) |
| **Ukrainian WER** | 6.79% (FLEURS) |
| **Built-in punctuation** | ✅ All languages |
| **Built-in capitalization** | ✅ All languages |
| **Timestamps** | ✅ Character, word, and segment level |
| **Streaming** | ✅ Chunked inference mode |
| **Max audio length** | 24 min (full attention, A100 80GB) / 3 hours (local attention) |
| **ONNX quantized** | 14 variants available on HuggingFace |
| **GPU support** | V100, T4, A10/A30/A100, H100, Blackwell, Grace Hopper |
| **Framework** | NeMo 2.4 (`pip install nemo_toolkit['asr']`) |
| **License** | CC-BY-4.0 (commercial use OK) |
| **HF downloads** | 247k+/month |
| **Training data** | 10k hrs human-transcribed + 660k hrs pseudo-labeled (Granary dataset) |
| **Anti-hallucination** | Trained on 36k hrs of silence data |
| **Noise robustness** | 12% WER degradation at SNR 10dB — strong |

**English benchmark detail:**

| Dataset | WER |
|---------|-----|
| LibriSpeech clean | 1.93% |
| LibriSpeech other | 3.59% |
| TEDLIUM-v3 | 2.75% |
| GigaSpeech | 9.59% |
| Earnings-22 | 11.42% |
| AMI | 11.31% |
| VoxPopuli | 6.14% |
| SPGI Speech | 3.97% |

**Top multilingual performers:** Italian 3.0%, Spanish 3.45%, English 4.85%, Portuguese 4.76%, Ukrainian 6.79%

**Challenging languages (higher WER):** Slovenian 24%, Latvian 22.8%, Lithuanian 20.4%, Maltese 20.5%, Greek 20.7%

**Who uses it:** Handy (18.2k stars), OpenWhispr (2k stars)

#### Moonshine v2 Streaming (Latest: Feb 2026)

| Spec | Details |
|------|---------|
| **Architecture** | Ergodic Streaming Encoder (sliding-window Transformer, no positional embeddings) |
| **Models** | Tiny (34M), Small (123M), Medium (245M) |
| **Languages** | 8 — EN, ES, ZH, JA, KO, VI, UK, AR |
| **Medium WER** | **6.65%** |
| **Small WER** | 7.84% |
| **Tiny WER** | 12.00% |
| **Latency** | 107ms (Medium), 73ms (Small), 34ms (Tiny) — on MacBook Pro |
| **Built-in punctuation** | ❌ Needs post-processing |
| **Built-in capitalization** | ❌ Needs post-processing |
| **Streaming** | ✅ Native — purpose-built for 100ms audio chunks |
| **Inference engine** | ONNX Runtime (primary) |
| **Platforms** | Python, iOS, Android, macOS, Linux, Windows, Raspberry Pi |
| **Install** | `pip install moonshine-voice` |
| **Bonus** | Intent recognition via Gemma 300M sentence embeddings |
| **GitHub stars** | 7.4k |

#### Head-to-Head: Parakeet v3 vs Moonshine v2 vs Whisper (current YoloVoice)

| Dimension | Whisper tiny (current) | Parakeet TDT 0.6B v3 | Moonshine v2 Medium |
|-----------|----------------------|----------------------|---------------------|
| **English WER** | ~14% | **6.34%** | 6.65% |
| **Parameters** | 39M | 600M | 245M |
| **Languages** | 99 | 25 (European + RU/UK) | 8 |
| **Russian** | ✅ | ✅ Native | ❌ |
| **Punctuation built-in** | ❌ | ✅ | ❌ |
| **Capitalization built-in** | ❌ | ✅ | ❌ |
| **Streaming** | ❌ | ✅ (chunked) | ✅ (native, 107ms) |
| **Word timestamps** | ✅ | ✅ (char/word/segment) | Partial (chunk-based) |
| **ONNX support** | Via conversion | 14 quantized variants | ✅ Primary engine |
| **Runs on Raspberry Pi** | ❌ | ❌ | ✅ |
| **Anti-hallucination** | ❌ | ✅ (silence training) | Not documented |
| **Noise robustness** | Poor | ✅ Strong (documented) | Not documented |
| **License** | MIT | CC-BY-4.0 | Check repo |

#### Recommended Model Strategy for YoloVoice

```
Primary:     Parakeet TDT 0.6B v3  — best accuracy, Russian, built-in punctuation/caps, anti-hallucination
Lightweight: Moonshine v2 Small     — 123M params, 73ms latency, for weak hardware / no GPU
Fallback:    Whisper tiny (current)  — 99 languages, widest compatibility
```

**Why Parakeet v3 as primary:**
1. Built-in punctuation & capitalization eliminates an entire post-processing step
2. Russian support (confirmed working) covers YoloVoice's multilingual needs
3. Anti-hallucination training (36k hrs silence data) — critical for dictation with pauses
4. 6.34% avg WER vs Whisper tiny's ~14% — massive accuracy improvement
5. 14 ONNX quantized variants enable a unified ONNX Runtime backend

**Why Moonshine v2 as lightweight option:**
1. 2.4x smaller than Parakeet (245M vs 600M)
2. Purpose-built streaming with 107ms latency — best real-time feel
3. ONNX-first — same runtime as Parakeet ONNX variants
4. Runs on very weak hardware (even Raspberry Pi)

**Integration approach:** Use ONNX Runtime as unified inference backend for both Parakeet (ONNX quantized) and Moonshine (ONNX native). This eliminates the Python sidecar dependency entirely and provides a single, Rust-native inference path.

---

## 4. Actionable Items for YoloVoice

### Priority 1 — High Impact, Directly Implementable

#### 4.1 Eliminate Python Sidecar → Rust-native Whisper
- **What:** Replace the faster-whisper Python sidecar with `whisper-rs` (Rust bindings to whisper.cpp)
- **Why:** Removes ~500MB bundled Python environment, eliminates sidecar lifecycle management, faster startup, smaller installer
- **Evidence:** Handy (18k stars) and VoiceTypr (334 stars) both prove this works with Tauri+Rust
- **Effort:** HIGH (major refactor of `infra/sidecar.rs` and `features/speech/mod.rs`)
- **Impact:** HIGH — smaller bundle, faster startup, simpler architecture

#### 4.2 Add Spoken Punctuation Commands
- **What:** Recognize spoken words like "comma", "period", "new line", "question mark" and insert the corresponding punctuation
- **Why:** OmniDictate has this and it's a natural, expected feature for dictation
- **Effort:** LOW (post-processing regex in `features/speech/mod.rs`)
- **Impact:** MEDIUM — improves dictation usability significantly

#### 4.3 Add Whisper Initial Prompt Conditioning
- **What:** Allow users to set an "initial prompt" that biases Whisper toward specific terminology
- **Why:** WhisperWriter has this. It dramatically improves accuracy for domain-specific vocabulary (medical terms, code keywords, etc.)
- **Effort:** LOW (pass `initial_prompt` parameter to faster-whisper / whisper-rs)
- **Impact:** MEDIUM — synergizes with existing industry packs feature

#### 4.4 Add Transcription History
- **What:** Store past transcriptions in SQLite with full-text search
- **Why:** OpenWhispr and Whispering both have this. Users want to recall what they dictated previously
- **Effort:** MEDIUM (add SQLite via `rusqlite`, new UI component)
- **Impact:** MEDIUM — quality-of-life feature users expect

#### 4.5 Clipboard-Free Text Insertion
- **What:** Add direct character injection mode alongside clipboard paste, so dictation doesn't overwrite clipboard contents
- **Why:** whisper-key-local offers dual modes. OpenWhispr built a custom native binary. Users complain about clipboard being overwritten
- **Effort:** MEDIUM (extend `features/output/mod.rs` with character-by-character SendInput)
- **Impact:** MEDIUM — removes a real user frustration

### Priority 2 — Medium Impact, Strategic Features

#### 4.6 Add Parakeet TDT 0.6B v3 + Moonshine v2 Model Support
- **What:** Support NVIDIA Parakeet TDT 0.6B v3 (primary, 25 langs, 6.34% WER, built-in punctuation/caps) and Moonshine v2 (lightweight, 245M params, 107ms latency) via ONNX Runtime
- **Why:** Parakeet v3 has built-in punctuation/capitalization (eliminates post-processing), anti-hallucination training, Russian support, and 6.34% WER vs Whisper tiny's ~14%. Moonshine v2 covers weak hardware. Both have ONNX variants enabling a unified Rust-native inference backend — which also solves the Python sidecar problem (4.1)
- **Effort:** MEDIUM-HIGH (add ort Rust crate for ONNX Runtime, model download UI, replace sidecar inference path)
- **Impact:** HIGH — dramatically better accuracy, built-in punctuation, eliminates Python sidecar if combined with 4.1
- **See:** Section 3.4 for full model comparison and benchmarks

#### 4.7 Auto-Learn Dictionary
- **What:** Detect when a user manually corrects a transcription and learn the correction for future use
- **Why:** OpenWhispr has this and it's a killer feature — vocabulary improves automatically over time
- **Effort:** HIGH (requires tracking corrections, matching to audio, updating vocabulary)
- **Impact:** HIGH — but complex to implement well

#### 4.8 Multi-GPU Vendor Support
- **What:** Support AMD and Intel GPUs in addition to NVIDIA CUDA
- **Why:** VoiceTypr auto-detects all GPU vendors. Buzz uses Vulkan for universal GPU support. YoloVoice is CUDA-only
- **Effort:** MEDIUM-HIGH (if using whisper.cpp/whisper-rs, Vulkan support comes built-in)
- **Impact:** MEDIUM — expands addressable user base significantly

#### 4.9 Smart Presets (Context-Aware Post-Processing)
- **What:** Predefined post-processing modes: "Email" (formal tone), "Code Commit" (imperative tense, short), "Notes" (bullet points), "Chat" (casual)
- **Why:** VoiceTypr's smart presets are a polished version of what YoloVoice's profiles could become
- **Effort:** LOW-MEDIUM (extend existing profiles system with preset templates)
- **Impact:** MEDIUM — great UX for new users who don't want to configure profiles manually

#### 4.10 Continuous Recording Mode
- **What:** Recording restarts automatically after a pause (like WhisperWriter's continuous mode)
- **Why:** Useful for long-form dictation (writing documents, meeting notes)
- **Effort:** LOW (extend `features/capture/recorder.rs` state machine)
- **Impact:** LOW-MEDIUM — niche but valuable for power users

### Priority 3 — Lower Priority / Future Consideration

#### 4.11 Hallucination Filtering
- **What:** Detect and suppress Whisper hallucinations (repetitive text, common hallucination phrases)
- **Why:** OmniDictate has this. Whisper is known to hallucinate during silence or background noise
- **Effort:** LOW (regex-based post-processing filter)
- **Impact:** LOW-MEDIUM — improves reliability

#### 4.12 Voice Commands
- **What:** Spoken trigger phrases that execute actions (open app, insert snippet, run command)
- **Why:** whisper-key-local and OpenWhispr have this. Expands dictation into voice control
- **Effort:** MEDIUM (trigger phrase detection, action dispatch system)
- **Impact:** LOW-MEDIUM — power user feature

#### 4.13 winget / Package Manager Distribution
- **What:** Publish YoloVoice to winget, Chocolatey, or Scoop for one-command Windows installation
- **Why:** Handy has `winget install cjpais.Handy`. Makes discovery and installation frictionless
- **Effort:** LOW (create winget manifest, submit PR to winget-pkgs)
- **Impact:** LOW-MEDIUM — improves discoverability

#### 4.14 Meeting Transcription Mode
- **What:** Detect active video call (Zoom/Teams/Meet process running) and offer to transcribe the meeting
- **Why:** OpenWhispr auto-detects meetings. This is a high-value enterprise feature
- **Effort:** HIGH (process detection, system audio capture, speaker diarization)
- **Impact:** HIGH if targeting enterprise users, LOW for individual dictation users

---

## 5. Summary & Recommendations

### YoloVoice's Position in the Market

YoloVoice occupies a **strong middle ground**: it has better architecture than Python-based tools (WhisperWriter, OmniDictate, Buzz), comparable stack to the top Tauri projects (Handy, VoiceTypr), and **unique features** (industry packs, replacement rules, profiles) that no competitor matches.

### Top 3 Recommendations

1. **Eliminate the Python sidecar** (4.1) — This is the single biggest architectural improvement. Every peer Tauri project does Whisper natively in Rust. The sidecar adds ~500MB to the bundle, creates startup latency, and is the most fragile part of the architecture.

2. **Add spoken punctuation + Whisper prompt conditioning** (4.2 + 4.3) — Two LOW-effort changes that significantly improve dictation quality. These synergize with existing industry packs.

3. **Add transcription history with search** (4.4) — A table-stakes feature that multiple competitors have. Users expect to be able to review past dictations.

### What YoloVoice Should NOT Copy

- **CRDT sync** (Whispering) — Over-engineered for a dictation tool
- **Chrome extension** (Whispering) — Maintenance burden, minimal value for desktop-first tool
- **Electron** (OpenWhispr) — YoloVoice already has the better framework choice with Tauri
- **Meeting transcription** (OpenWhispr) — Very high effort, different product category. Only pursue if pivoting to enterprise.
