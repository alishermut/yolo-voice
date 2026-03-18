# YOLO Voice - Project Specification

> A Windows-first, offline-focused voice dictation app that transcribes speech and pastes text into any focused application. Inspired by Willow Voice and Aqua Voice, but prioritizing local/offline transcription for speed and privacy.

---

## Tech Stack

| Layer | Technology | Notes |
|---|---|---|
| Desktop Shell | **Tauri 2.0** (Rust + WebView2) | ~20-40MB RAM, native Windows integration |
| Frontend UI | **React + TypeScript + Tailwind CSS** | Runs inside WebView2 |
| Offline Transcription | **faster-whisper** (Python sidecar) | CTranslate2-optimized Whisper inference, GPU-accelerated |
| Fallback STT (no GPU) | **Vosk** or **Moonshine** | CPU-friendly, small model footprint |
| Online Transcription | **Groq Whisper API** or **Deepgram** (optional) | Cloud fallback when user opts in |
| LLM Post-Processing | **Local**: Ollama (Llama 3.1 8B) / **Cloud**: Claude API | Formatting, punctuation, context-aware rewriting |
| Audio Capture | **cpal** (Rust crate, WASAPI backend) | Low-latency mic input on Windows |
| Voice Activity Detection | **Silero VAD** | Detects speech boundaries |
| Text Insertion | **Win32 SendInput API** + clipboard | System-wide paste into any app |
| Tray/Pill UI | **Tauri system tray** + custom overlay window | Always-visible pill at bottom of screen |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    TAURI APPLICATION                     │
│                                                         │
│  ┌──────────────┐   ┌──────────────────────────────┐    │
│  │  Rust Core   │   │   WebView2 (React Frontend)  │    │
│  │              │   │                              │    │
│  │  - Audio     │   │  - Settings Page             │    │
│  │    capture   │◄─►│  - Mic selector/test         │    │
│  │  - VAD       │   │  - Context profiles          │    │
│  │  - Hotkey    │   │  - Keybinding config         │    │
│  │    listener  │   │  - Pill/overlay UI           │    │
│  │  - Text      │   │  - Equalizer visualization   │    │
│  │    insertion  │   │                              │    │
│  │  - Tray icon │   │                              │    │
│  └──────┬───────┘   └──────────────────────────────┘    │
│         │                                               │
│         ▼                                               │
│  ┌──────────────┐                                       │
│  │ Python Sidecar│                                      │
│  │              │                                       │
│  │ faster-whisper│                                      │
│  │ Silero VAD   │                                       │
│  │ LLM (Ollama) │                                       │
│  └──────────────┘                                       │
└─────────────────────────────────────────────────────────┘

Flow:
  Microphone ──► WASAPI (cpal) ──► Silero VAD ──► Speech Chunk
       ──► faster-whisper (local) or Cloud API ──► Raw Text
       ──► LLM Post-Processing (formatting/context) ──► Final Text
       ──► SendInput / Clipboard Paste ──► Focused App
```

---

## Interaction Model

### Pill Widget (Always Visible)
- Small floating pill anchored to bottom-center of screen (like Willow/Aqua)
- Shows app state: idle, listening, transcribing
- Click to expand slightly and show equalizer bars

### Recording Modes
1. **Hold-to-record**: Press and hold hotkey (default: `Ctrl`) → records while held → release → transcribe → paste
2. **Toggle-record**: Double-press hotkey → starts persistent recording → single press again → stop → transcribe → paste

### Text Insertion
- Transcribed text is inserted at cursor position in whatever app is focused
- Primary method: clipboard write + simulated `Ctrl+V`
- Terminal detection: use `Ctrl+Shift+V` for terminal apps

---

## Context Profiles

Users can select or create context profiles that influence how the LLM post-processes transcriptions:

| Profile | Behavior |
|---|---|
| **General** | Natural prose, standard punctuation, casual tone |
| **Technical / Code** | Preserves technical terms, recognizes API names, uses code formatting when appropriate |
| **Email / Professional** | Formal tone, proper greeting/closing structure |
| **Medical** | Medical terminology recognition, structured notes |
| **Legal** | Legal terminology, formal language |
| **Custom** | User-defined system prompt + custom dictionary entries |

Each profile includes:
- A system prompt sent to the LLM for post-processing
- A custom dictionary (word list) fed to Whisper's `initial_prompt` for improved recognition
- Optional tone/formatting rules

---

## Settings

- **Microphone**: Select input device from dropdown, test with live waveform preview
- **Keybindings**: Configure hotkey for hold-to-record and toggle-record (default: `Ctrl`)
- **Transcription Engine**: Choose between Offline (faster-whisper) or Online (Groq/Deepgram), select Whisper model size
- **Context Profile**: Select active profile, create/edit custom profiles
- **LLM Post-Processing**: Enable/disable, choose local (Ollama) vs cloud (Claude API), set API keys
- **Language**: Primary dictation language
- **Appearance**: Light/dark theme, pill position, pill size
- **Startup**: Launch on Windows startup, start minimized to tray

---

## Phases

---

### Phase 1: Project Scaffolding & Audio Foundation

**Goal**: Get Tauri running on Windows with mic capture and a basic UI shell.

**Deliverables**:
1. Initialize Tauri 2.0 project with React + TypeScript + Tailwind frontend
2. Rust backend: enumerate audio input devices using `cpal`
3. Rust backend: capture microphone audio stream (WASAPI), output raw PCM
4. Frontend: settings page shell with mic selector dropdown
5. Frontend: mic test button that shows a live waveform/level meter
6. Tauri commands bridging Rust audio functions to frontend
7. Basic app window with close-to-tray behavior

**Key Files**:
```
src-tauri/
  src/
    main.rs              — Tauri app entry
    audio.rs             — cpal mic enumeration + capture
    commands.rs          — Tauri command handlers
  Cargo.toml
src/
  App.tsx                — Main React app
  pages/
    Settings.tsx         — Settings page
  components/
    MicSelector.tsx      — Mic dropdown + test
    WaveformDisplay.tsx  — Live audio level visualization
  styles/
    globals.css          — Tailwind base
package.json
tailwind.config.js
```

**Acceptance Criteria**:
- [ ] App launches as a Tauri window on Windows
- [ ] Dropdown lists all available microphones
- [ ] Selecting a mic and clicking "Test" shows a live audio level meter
- [ ] Closing the window minimizes to system tray (tray icon present)

---

### Phase 2: Hotkey System & Recording Modes

**Goal**: Global hotkey listener with hold-to-record and toggle-record modes. Pill overlay widget.

**Deliverables**:
1. Rust backend: global hotkey registration (using `rdev` or Windows raw input API)
2. Implement hold-to-record mode (press+hold → record, release → stop)
3. Implement toggle-record mode (double-press → start, single-press → stop)
4. Save recorded audio to a temporary WAV file
5. Pill overlay: always-on-top, frameless, small transparent window at bottom-center
6. Pill states: idle (small dot/pill), recording (expanded with equalizer bars animation)
7. Settings page: keybinding configuration UI (capture key combo)
8. Persist settings to local JSON config file

**Key Files**:
```
src-tauri/
  src/
    hotkey.rs            — Global hotkey listener
    recorder.rs          — Audio recording to WAV buffer
    config.rs            — Settings persistence (JSON)
    pill_window.rs       — Pill overlay window management
src/
  components/
    Pill.tsx             — Pill overlay UI
    Equalizer.tsx        — Animated equalizer bars
    KeybindingInput.tsx  — Hotkey capture input
  pages/
    Settings.tsx         — Updated with keybinding section
```

**Acceptance Criteria**:
- [ ] Holding configured hotkey records audio; releasing stops recording
- [ ] Double-pressing hotkey starts persistent recording; single press stops it
- [ ] Pill widget is visible at screen bottom, always on top
- [ ] Pill expands and shows animated equalizer bars while recording
- [ ] Pill returns to idle state when not recording
- [ ] Keybinding is configurable from settings and persisted across restarts
- [ ] Audio is captured as WAV and ready to be sent to transcription

---

### Phase 3: Offline Transcription Engine

**Goal**: Integrate faster-whisper as a Python sidecar for local speech-to-text.

**Deliverables**:
1. Python sidecar: faster-whisper wrapper script that accepts WAV input → returns text
2. Bundled Python environment (embedded Python or PyInstaller-packaged sidecar)
3. Tauri sidecar configuration to spawn and communicate with Python process
4. Communication protocol: stdin/stdout JSON messages or local socket
5. Model management: download/select Whisper model (tiny, base, small, medium, large-v3-turbo)
6. Silero VAD integration: trim silence before sending to Whisper
7. Settings page: model selector, GPU/CPU toggle, download progress
8. Transcription result returned to Rust → ready for text insertion

**Key Files**:
```
sidecar/
  transcribe.py          — faster-whisper inference entry point
  vad.py                 — Silero VAD preprocessing
  models/                — Downloaded Whisper models stored here
  requirements.txt       — faster-whisper, silero-vad, torch
  build.py               — PyInstaller build script for sidecar
src-tauri/
  src/
    sidecar.rs           — Sidecar process management
    transcription.rs     — Transcription request/response handling
src/
  components/
    ModelSelector.tsx     — Whisper model picker + download UI
  pages/
    Settings.tsx          — Updated with transcription engine section
```

**Acceptance Criteria**:
- [ ] User can download a Whisper model from settings
- [ ] Recording → transcription pipeline works end-to-end offline
- [ ] Silero VAD trims leading/trailing silence before transcription
- [ ] Transcription completes in under 2 seconds for a 10-second clip (on GPU)
- [ ] CPU fallback works (slower but functional)
- [ ] Model selection is persisted in settings

---

### Phase 4: Text Insertion & End-to-End Flow

**Goal**: Paste transcribed text into the focused application. Complete the core dictation loop.

**Deliverables**:
1. Rust: clipboard write using `arboard` crate
2. Rust: simulate `Ctrl+V` keystroke via Win32 `SendInput`
3. Terminal detection: identify if focused app is a terminal → use `Ctrl+Shift+V`
4. End-to-end flow: hotkey → record → transcribe → paste into focused app
5. Pill feedback: show transcription state (recording → transcribing → done)
6. Error handling: show toast/notification on failure
7. Basic notification sounds (start recording beep, done chime)

**Key Files**:
```
src-tauri/
  src/
    text_insert.rs       — Clipboard + SendInput paste logic
    window_detect.rs     — Focused window detection (terminal vs normal)
    pipeline.rs          — Orchestrates the full record→transcribe→paste flow
src/
  components/
    Toast.tsx            — Toast notification for errors/status
    Pill.tsx             — Updated with transcribing/done states
```

**Acceptance Criteria**:
- [ ] Full loop works: hold hotkey → speak → release → text appears in focused app
- [ ] Toggle mode works: double-press → speak freely → press → text appears
- [ ] Text pastes correctly in: Notepad, VS Code, Chrome text fields, Slack, Terminal
- [ ] Pill shows state transitions: idle → recording → transcribing → done (brief flash) → idle
- [ ] Errors show as toast notifications
- [ ] Works with any app that accepts clipboard paste

---

### Phase 5: LLM Post-Processing & Context Profiles

**Goal**: Add intelligence — clean up transcriptions, apply context-aware formatting, and implement profiles.

**Deliverables**:
1. Context profile system: data model, CRUD operations, persistence
2. Pre-built profiles: General, Technical, Email, Custom
3. LLM integration (local): Ollama HTTP API for post-processing
4. LLM integration (cloud): Claude API / OpenAI API as alternative
5. Post-processing pipeline: raw transcription → LLM with profile prompt → formatted text
6. Custom dictionary per profile: fed to Whisper's `initial_prompt`
7. Settings page: profile management UI (select, create, edit, delete)
8. Settings page: LLM provider selection (Ollama / Cloud API + key input)
9. Option to disable post-processing (raw transcription only)

**Key Files**:
```
src-tauri/
  src/
    profiles.rs          — Profile data model and storage
    llm.rs               — LLM client (Ollama + cloud APIs)
    postprocess.rs       — Post-processing pipeline
src/
  pages/
    Profiles.tsx         — Profile management page
  components/
    ProfileEditor.tsx    — Create/edit profile form
    DictionaryEditor.tsx — Custom word list editor
    LLMSettings.tsx      — LLM provider configuration
  data/
    default_profiles.json — Built-in profile definitions
```

**Acceptance Criteria**:
- [ ] Selecting "Technical" profile: transcription preserves terms like "kubectl", "React", "API"
- [ ] Selecting "Email" profile: output is formatted as a proper email
- [ ] Custom profile with custom dictionary improves recognition of user-specific terms
- [ ] LLM post-processing adds punctuation, fixes grammar, formats naturally
- [ ] User can toggle post-processing on/off
- [ ] Works with Ollama locally (no internet needed)
- [ ] Cloud API works when configured with valid key

---

### Phase 6: Polish, Online Fallback & Distribution

**Goal**: Add online transcription option, polish the UX, and prepare for distribution.

**Deliverables**:
1. Online transcription: Groq Whisper API or Deepgram integration as alternative engine
2. Auto-detect: if no GPU and no model downloaded, suggest online mode
3. Startup behavior: launch on Windows startup option, start minimized
4. Auto-update mechanism (Tauri updater plugin)
5. Installer: MSI/NSIS installer via Tauri bundler
6. Onboarding flow: first-launch wizard (select mic, choose engine, pick profile)
7. Pill UI polish: smooth animations, proper DPI scaling, multi-monitor support
8. Performance optimization: memory usage audit, sidecar lifecycle management
9. Keyboard shortcut overlay: brief on-screen hint when hotkey is pressed
10. About page, version info, links

**Key Files**:
```
src-tauri/
  src/
    cloud_transcription.rs — Groq/Deepgram API client
    updater.rs              — Auto-update configuration
  tauri.conf.json           — Installer + updater config
src/
  pages/
    Onboarding.tsx          — First-launch wizard
    About.tsx               — App info page
  components/
    OnboardingSteps/
      MicSetup.tsx
      EngineChoice.tsx
      ProfilePick.tsx
```

**Acceptance Criteria**:
- [ ] Online transcription works as fallback when configured
- [ ] First-launch wizard guides user through setup
- [ ] App installs cleanly via MSI installer
- [ ] App launches on Windows startup when enabled
- [ ] Auto-updater checks for and applies updates
- [ ] Pill UI is smooth, responsive, and handles multi-monitor setups
- [ ] App uses < 50MB RAM when idle (excluding sidecar)
- [ ] Sidecar process is killed cleanly on app exit

---

## File Structure (Final)

```
yolo-voice/
├── docs/
│   └── PROJECT_SPEC.md
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── audio.rs
│   │   ├── commands.rs
│   │   ├── hotkey.rs
│   │   ├── recorder.rs
│   │   ├── config.rs
│   │   ├── pill_window.rs
│   │   ├── sidecar.rs
│   │   ├── transcription.rs
│   │   ├── text_insert.rs
│   │   ├── window_detect.rs
│   │   ├── pipeline.rs
│   │   ├── profiles.rs
│   │   ├── llm.rs
│   │   ├── postprocess.rs
│   │   ├── cloud_transcription.rs
│   │   └── updater.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── sidecar/
│   ├── transcribe.py
│   ├── vad.py
│   ├── models/
│   ├── requirements.txt
│   └── build.py
├── src/
│   ├── App.tsx
│   ├── pages/
│   │   ├── Settings.tsx
│   │   ├── Profiles.tsx
│   │   ├── Onboarding.tsx
│   │   └── About.tsx
│   ├── components/
│   │   ├── MicSelector.tsx
│   │   ├── WaveformDisplay.tsx
│   │   ├── Pill.tsx
│   │   ├── Equalizer.tsx
│   │   ├── KeybindingInput.tsx
│   │   ├── ModelSelector.tsx
│   │   ├── Toast.tsx
│   │   ├── ProfileEditor.tsx
│   │   ├── DictionaryEditor.tsx
│   │   └── LLMSettings.tsx
│   ├── styles/
│   │   └── globals.css
│   └── data/
│       └── default_profiles.json
├── package.json
└── tailwind.config.js
```

---

## Notes

- **Windows-only** for now. No cross-platform concerns.
- **Offline-first**: the app must be fully functional without internet.
- **Privacy**: audio is processed locally by default. Never stored or sent anywhere unless user explicitly enables cloud.
- Each phase is designed to be built in a separate chat session, using this spec as the reference.
- Phase N+1 builds on Phase N — no phase should break what came before.
