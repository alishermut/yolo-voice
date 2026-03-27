# YOLO Voice

Offline-first voice dictation for Windows and macOS. Press a hotkey, speak naturally, and your words appear wherever you type — no internet required.

Built with Tauri 2.0 (Rust backend + React frontend). Speech recognition runs locally via Parakeet TDT, with optional cloud fallback through Groq or Deepgram APIs.

## Features

### Core Dictation
- **Offline transcription** — Parakeet TDT v3 model runs locally with GPU acceleration (DirectML on Windows, CPU fallback everywhere)
- **Cloud fallback** — Groq (Whisper Large v3) and Deepgram (Nova-2) APIs when offline isn't available or preferred
- **Hold or toggle** — Hold the hotkey to record, or double-tap to toggle hands-free
- **Continuous mode** — Auto-restarts recording after each transcription for long dictation sessions
- **Voice Activity Detection** — Silero VAD v5 segments speech in real-time, enabling progressive transcription of long recordings

### Text Processing (English only)
- **Text cleanup** — Removes filler words ("uh", "um", "you know"), fixes stutters, normalizes punctuation and capitalization
- **Hallucination filter** — Strips phantom phrases and repetition loops from transcription output
- **Spoken punctuation** — Say "period", "comma", "question mark", "new line" to insert punctuation
- **Numbers as digits** — "twenty three" becomes "23"
- **Sentence capitalization** — Automatic capitalization after sentence-ending punctuation

### LLM Integration
- **Command mode** — Hold a separate hotkey to issue voice commands processed by an LLM (e.g., "write a Python function that sorts a list")
- **Post-processing** — Optional LLM pass to polish grammar, tone, and formatting after transcription
- **Dictation profiles** — General, Technical, Email/Professional, and Custom profiles with tailored system prompts
- **Style shortcuts** — Press Command/Ctrl + letter during recording to activate a profile on the fly
- **4 LLM providers** — Ollama (local), OpenAI, Claude, Groq; custom base URLs supported for self-hosted models

### Vocabulary & Customization
- **Custom vocabulary** — Add domain terms and misspelling variants to improve recognition accuracy
- **Normalization rules** — Regex-based find/replace rules applied after transcription (e.g., "jay son" → "JSON")
- **Industry packs** — Bundled vocabulary packs for specialized domains (medical, legal, technical)

### User Experience
- **Floating pill** — Transparent always-on-top overlay showing recording state with animated waveform, elapsed time, and active profile
- **Transcription history** — SQLite-backed history of past dictations; browse, search, and copy previous results
- **Auto-pause media** — Pauses media playback when recording starts, resumes when done
- **Sound effects** — 16 built-in start/stop sounds with preview
- **System tray** — Runs in background with tray icon
- **Auto-update** — In-app update checks via GitHub Releases with minisign signature verification
- **15 UI languages** — English, Russian, Ukrainian, Spanish, Portuguese, French, German, Italian, Polish, Dutch, Czech, Turkish, Chinese, Japanese, Korean

## How It Works

```
Hotkey Press → Audio Capture (cpal) → VAD Segmentation (Silero) → STT (Parakeet/Cloud)
    → Text Cleanup → Vocabulary Normalization → [LLM Post-processing] → Clipboard + Paste
```

### Recording Pipeline

1. **Hotkey detected** — Global listener via `rdev` catches the configured key. The foreground window (where text will be inserted) is captured at this point.
2. **Audio capture** — `cpal` opens the selected input device. Samples are buffered as f32 and resampled to 16 kHz mono via `rubato` if needed.
3. **VAD segmentation** (offline mode) — Audio streams through Silero VAD v5 in 512-sample chunks. Speech segments are detected with a 250 ms minimum duration and configurable silence threshold (default 2 seconds). Each segment is sent to a transcriber thread as it completes.
4. **Transcription** — Offline: Parakeet TDT v3 ONNX session processes each segment independently, emitting progressive results via `segment-transcribed` events. Cloud: the full recording is sent as a WAV file to Groq or Deepgram after recording stops.
5. **Text processing** — Per-segment cleanup (filler removal, stutter fixing) → final assembly with sentence capitalization and context-aware segment joining → vocabulary normalization → optional LLM post-processing.
6. **Output** — Text is copied to clipboard and pasted into the captured foreground window via `Ctrl+V` / `Cmd+V`. Falls back to direct key emission via `enigo` for simple text.

### State Management

The Rust backend manages state through Tauri's managed state system:

| State | Purpose |
|-------|---------|
| `InferenceState` | ONNX session for Parakeet (loaded after model download) |
| `RecordingState` | Active cpal audio stream |
| `WarmDeviceState` | Pre-cached audio device to avoid re-enumeration |
| `HotkeyCache` | Parsed rdev keys for fast hotkey matching |
| `ConfigState` | AppConfig loaded from disk |
| `UserDictionaryState` | Vocabulary + normalization rules |
| `RuntimeDictionaryCache` | Compiled regex cache for replacements |
| `ActiveStyleKey` | Profile shortcut pressed during recording |
| `ContinuousGeneration` | Atomic counter to manage auto-restart races |
| `TranscriptDiagnosticsState` | SQLite connection for history |

## Language Support

| Layer | Languages | Notes |
|-------|-----------|-------|
| **Speech recognition (offline)** | 25 (auto-detected) | Bulgarian, Croatian, Czech, Danish, Dutch, English, Estonian, Finnish, French, German, Greek, Hungarian, Italian, Latvian, Lithuanian, Maltese, Polish, Portuguese, Romanian, Russian, Slovak, Slovenian, Spanish, Swedish, Ukrainian |
| **Speech recognition (cloud)** | 57+ (Groq) / 30+ (Deepgram) | Language parameter passed to API |
| **Text processing** | English only | Cleanup, spoken punctuation, hallucination filter, numerals, capitalization |
| **UI** | 15 languages | i18next with JSON locale bundles |

> Text processing features currently work in English only. Multi-language support is planned — see [docs/MULTI_LANGUAGE_ROADMAP.md](docs/MULTI_LANGUAGE_ROADMAP.md).

## Architecture

```
src-tauri/src/
├── lib.rs                          # Tauri app setup, plugin registration, state init
├── app/
│   ├── commands.rs                 # 77 Tauri IPC commands
│   └── events.rs                   # Event broadcast helpers
├── features/
│   ├── capture/
│   │   ├── mod.rs                  # Hotkey → record → transcribe → output orchestration
│   │   ├── hotkey.rs               # rdev global listener, key parsing, chord detection
│   │   └── recorder.rs            # cpal stream setup, VAD thread, sample buffering
│   ├── speech/
│   │   ├── mod.rs                  # Speech engine interface
│   │   ├── inference.rs            # Parakeet ONNX wrapper (DirectML / CPU)
│   │   ├── cloud.rs                # Groq & Deepgram HTTP clients
│   │   ├── llm.rs                  # LLM post-processing & command mode
│   │   ├── cleanup.rs              # Filler removal, punctuation, numerals, capitalization
│   │   ├── vad.rs                  # Silero VAD v5 streaming processor
│   │   ├── accumulator.rs          # VAD segment accumulator & progressive transcription
│   │   ├── vocabulary.rs           # User dictionary, industry packs, regex cache
│   │   └── profiles.rs            # Dictation style management
│   ├── output/
│   │   └── mod.rs                  # Clipboard paste, enigo typing, sound playback
│   ├── settings/
│   │   └── mod.rs                  # Config persistence (JSON), registry (startup)
│   └── diagnostics/
│       └── mod.rs                  # SQLite transcript history
└── infra/
    ├── model.rs                    # HuggingFace model download with progress events
    └── platform.rs                 # Windows Core Audio device enumeration
```

```
src/
├── App.tsx                         # Router (onboarding vs settings)
├── pages/
│   ├── Onboarding.tsx              # First-run setup wizard
│   └── Settings.tsx                # Main settings UI (tabbed)
├── components/
│   ├── Pill.tsx                    # Floating overlay (separate window)
│   ├── Waveform.tsx                # Real-time audio level bars
│   ├── ModelSelector.tsx           # Model download UI with progress
│   ├── MicSelector.tsx             # Audio device picker
│   └── settings/                   # Settings tab sections
├── contexts/
│   └── UpdaterContext.tsx          # Auto-update state management
├── shared/
│   ├── platform.ts                 # Typed Tauri invoke wrappers
│   └── types.ts                    # TypeScript interfaces
├── i18n.ts                         # i18next configuration
└── locales/                        # 15 language JSON bundles
```

## Tech Stack

| Component | Technology | Details |
|-----------|-----------|---------|
| Desktop shell | Tauri 2.0 | Rust backend, WebView2 frontend |
| Frontend | React 19, TypeScript, Tailwind CSS 4 | Vite 7 bundler |
| Offline STT | Parakeet TDT v3 | ONNX Runtime, DirectML GPU on Windows |
| Cloud STT | Groq / Deepgram | Whisper Large v3 / Nova-2 |
| VAD | Silero VAD v5 | ONNX, 512-sample chunks at 16 kHz |
| LLM | Ollama, OpenAI, Claude, Groq | Post-processing and command mode |
| Audio capture | cpal | WASAPI (Windows), CoreAudio (macOS) |
| Resampling | rubato | Any sample rate → 16 kHz mono |
| Sound playback | rodio / Win32 PlaySoundW | 16 embedded WAV files |
| Text insertion | enigo + arboard | Direct typing or clipboard paste |
| Hotkey listener | rdev | Global keyboard hook |
| Config storage | serde_json + dirs-next | JSON file in app data directory |
| History storage | rusqlite | SQLite with async writer thread |
| HTTP client | reqwest | Blocking, 60-second timeout |
| Installer | NSIS (Windows) | Per-user install, auto-update via GitHub Releases |
| i18n | react-i18next | 15 languages, JSON locale bundles |

## System Requirements

- **Windows 10+** (primary) or **macOS 12+** (secondary)
- **~2.4 GB disk space** for the offline speech model (downloaded on first run)
- **GPU recommended** for offline mode — any DirectML-compatible GPU on Windows; macOS uses CPU only
- **~2-3 GB RAM** during offline inference
- **Internet required** for cloud STT, LLM features, model download, and auto-updates

## Constraints & Limitations

- **Text processing is English-only** — filler removal, spoken punctuation, hallucination filtering, number-to-digit conversion, and sentence capitalization only work for English. Other languages get raw transcription output.
- **macOS is secondary** — no GPU acceleration (CPU-only inference), no NSIS installer (manual build required), no Windows Registry integration for launch-on-startup.
- **API keys stored in plain text** — LLM and cloud STT API keys are saved unencrypted in `config.json` in the app data directory.
- **No real-time streaming to cloud** — cloud mode sends the full recording after you stop, not a live stream. Only offline mode does progressive segment-by-segment transcription via VAD.
- **VAD minimum segment is 250 ms** — very short utterances (under 250 ms) are discarded as noise.
- **Hotkey conflicts** — global hotkeys may conflict with OS shortcuts or other applications. The app uses `rdev` which requires accessibility permissions on macOS.
- **Single-window text insertion** — text is inserted into whatever window was focused when recording started. If you switch windows during recording, text still goes to the original window.
- **LLM responses are not streamed** — the full LLM response is awaited before inserting text, which adds latency for post-processing and command mode.

## Development

```bash
# Prerequisites: Node.js 18+, Rust toolchain, platform build tools
# See https://v2.tauri.app/start/prerequisites/

npm install
npm run dev          # Vite dev server + Tauri in watch mode
npm run build        # TypeScript + Vite production build
cd src-tauri && cargo test   # Rust tests
```

### Data Paths

| Platform | Config & Data |
|----------|--------------|
| Windows | `%APPDATA%\YOLO Voice\` |
| macOS | `~/Library/Application Support/com.alish.yolo-voice/` |

Key files: `config.json`, `dictionary.json`, `transcript_diagnostics.db`, `models/parakeet-tdt-v3/`

## License

MIT
