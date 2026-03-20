# YOLO Voice Architecture Refactor Plan

This document defines the refactor plan for YOLO Voice.

The goal is not to apply quick fixes or cosmetic restructuring. The goal is to move the app toward a modular, feature-driven architecture that is easier to extend, test, and maintain over time.

## Goals

- Make the codebase modular by feature ownership, not just by technical layer.
- Reduce orchestration and business logic concentrated in a few god-files.
- Separate product logic from platform and infrastructure details.
- Replace fragile or duplicated workflows with explicit feature boundaries.
- Improve performance by fixing architectural bottlenecks, not by layering temporary workarounds.
- Keep behavior stable while refactoring.

## Non-Goals

- Large behavior changes during the initial refactor phases.
- Rewriting the entire app in one pass.
- Introducing a global state solution as a shortcut.
- Adding thin wrappers that preserve the same coupling under a different folder layout.

## Current Architecture Problems

### Rust / Tauri

- `src-tauri/src/lib.rs` is acting as a composition root, workflow orchestrator, event hub, and runtime policy layer at the same time.
- `src-tauri/src/commands.rs` mixes Tauri transport, app state ownership, and feature behavior.
- `src-tauri/src/transcription.rs` contains multiple unrelated domains: local/cloud transcription, model management, post-processing, profiles, dictionary persistence, and industry packs.
- `src-tauri/src/sidecar.rs` mixes process lifecycle, path resolution, bundled environment setup, restart policy, and feature behavior.
- `src-tauri/src/recorder.rs` depends on UI/command-layer state (`PillUiState`), which is the wrong ownership direction.

### Frontend

- `src/pages/Settings.tsx` is a god-page coordinating many unrelated features.
- Leaf components directly call Tauri APIs instead of going through feature-owned controllers/hooks.
- Multiple flows are duplicated across pages, including config save flows and update handling.
- Some UI surfaces poll backend state instead of using the event model already present in the app.

### Product / Policy Drift

- The product currently exposes model-selection UI while also forcing `tiny` as the runtime model in startup flow.
- Version metadata is inconsistent across app files.
- Some boundaries exist only partially, which makes the code look modular while remaining tightly coupled.

## Architecture Principles

### 1. Composition Root Only

`src-tauri/src/lib.rs` should become a thin bootstrap layer only:

- register commands
- initialize services/state
- wire app startup
- configure tray/windows/plugins

It should not own end-to-end feature workflows.

### 2. Feature Ownership

Each feature should own:

- its state
- its commands/events
- its business rules
- its persistence or service boundary
- its UI/controller surface

### 3. Infra Does Not Own Product Policy

Infrastructure modules should provide capabilities, not make feature decisions.

For example:

- sidecar runtime should manage spawning, health, protocol, restart hooks
- transcription feature should decide what model to load and when

### 4. Platform Isolation

Windows-specific behavior should be isolated behind platform modules instead of being spread across multiple features.

### 5. Frontend Feature Modules

Frontend components should not directly own backend integration.

Instead:

- feature modules own Tauri command/event integration
- presentational components stay focused on rendering and local UI state

### 6. Stable Contracts

Commands and events should be explicit, centralized, and typed where possible.

Avoid stringly-typed contracts scattered throughout the app.

### 7. Behavior-Preserving Refactor First

The first refactor phases should improve ownership and boundaries without changing user-visible behavior.

### 8. Earn Your Directory

A module earns a directory when it has 3+ files or 400+ lines of logic. Until then, it stays a single file. Do not create directories for 100-line modules.

## Target Structure

### Rust / Tauri

```text
src-tauri/src/
  lib.rs                    // thin bootstrap only (~80 lines)
  main.rs                   // unchanged

  app/
    mod.rs
    commands.rs             // thin Tauri adapters, no logic
    state.rs                // AppState struct, all managed state in one place
    events.rs               // event names + emit helpers

  features/
    capture/
      mod.rs                // recording lifecycle, hotkey routing, level metering
      recorder.rs           // cpal stream, WAV encoding
      hotkey.rs             // global hotkey listener, hold/toggle state machine

    speech/
      mod.rs                // transcription orchestration (local + cloud + post-process)
      vocabulary.rs         // dictionary, replacements, industry packs

    output/
      mod.rs                // text insertion, sounds, focused window

    settings/
      mod.rs                // config persistence, startup prefs

  infra/
    sidecar.rs              // process lifecycle, protocol, paths (single file)
    platform.rs             // Windows audio devices, registry, SendInput helpers
```

### Frontend

```text
src/
  App.tsx
  main.tsx
  pill-main.tsx

  features/
    audio-input/            // MicSelector, WaveformDisplay, device hooks
    transcription/          // ModelSelector, engine selection, sidecar status
    vocabulary/             // IndustryPackSelector, ReplacementRules, dictionary
    post-processing/        // ProfileEditor, LLMSettings
    settings/               // config hook, startup prefs

  shared/
    platform.ts             // typed Tauri invoke/listen wrappers
    types.ts                // AppConfig, GlobalDictionary, etc.

  pages/
    Settings.tsx            // composition of feature components
    About.tsx
    Onboarding.tsx

  components/
    Pill.tsx
    KeybindingInput.tsx
    EqualizerBars.tsx
```

### Python Sidecar

No structural changes planned. `transcribe.py` (740 lines) stays as a single file. The sidecar's improvement opportunities are architectural (timeouts, restart resilience), not organizational.

## Feature Boundaries

### Capture

Owns:

- hotkey-triggered start/stop
- recording session lifecycle
- focused window capture at record start
- live level metering
- recording state transitions

Does not own:

- transcription policy
- sidecar lifecycle policy
- text insertion

### Speech

Owns:

- local transcription
- cloud transcription
- initial prompt construction
- post-processing orchestration
- model selection/loading policy
- profiles (CRUD, persistence, tied to post-processing)
- vocabulary (global dictionary, replacement rules, industry packs)

Does not own:

- sidecar runtime internals

### Output

Owns:

- foreground app targeting
- clipboard write and paste
- terminal-aware paste behavior
- notification sounds

### Settings

Owns:

- config persistence
- startup preferences
- feature settings composition

## Anti-Patterns To Avoid

These are explicitly not acceptable long-term solutions.

### 1. Folder Moves Without Ownership Changes

Moving files into `features/` while still reading global state from everywhere is not a real refactor.

### 2. Thin Service Wrappers Around Existing Coupling

Do not create wrappers that still pass large mutable config blobs or route every feature through one parent coordinator.

### 3. Frontend-Wide Global Store As A Shortcut

Do not introduce a global state layer just to avoid refactoring feature ownership.

### 4. Polling As A Permanent Runtime Model

Polling may be tolerated as a temporary migration seam, but event-driven flow is the architectural target when the app already emits events.

### 5. Infra Reading Product State Directly

Infrastructure code should not reach into app-level config or feature state to determine business behavior.

### 6. Duplicate Workflow Ownership

The same product flow should not be separately implemented in multiple screens or modules.

### 7. Mixed Domain Modules

Do not keep unrelated domains bundled together just because they touch text.

Examples:

- profiles are not the same thing as transcription transport
- vocabulary is not the same thing as LLM post-processing
- sounds are not the same thing as text insertion

## Refactor Phases

## Phase 1: Rust Feature Extraction ✅ COMPLETED (2026-03-20)

Primary objective:

- Extract feature logic from `lib.rs`, `commands.rs`, and `transcription.rs` into feature modules
- Establish the `app/`, `features/`, and `infra/` directory structure
- Split `transcription.rs` by domain (speech, vocabulary)

Results:

- `lib.rs`: 439 → 213 lines (thin bootstrap, only wiring)
- `transcription.rs` (495 lines, 4 mixed domains) → `features/speech/mod.rs` (274 lines) + `features/speech/vocabulary.rs` (221 lines)
- Pipeline orchestration extracted from `lib.rs` → `features/capture/mod.rs` (258 lines)
- `commands.rs` → `app/commands.rs` (390 lines, thin adapters delegating to features)
- `config.rs` + `startup.rs` → `features/settings/mod.rs` (266 lines)
- `sidecar.rs` cleaned: `ensure_running` now takes explicit model params instead of reaching into ConfigState
- `PillUiState` moved from commands to `app/state.rs` (correct dependency direction)
- 9 old files deleted, 16 new files created
- Zero warnings, clean build

## Phase 2: Frontend Modularization ✅ COMPLETED (2026-03-20)

Primary objective:

- Replace page-centric coordination with feature-owned modules
- Deduplicate flows across pages

Results:

- `shared/types.ts`: single source of truth for all TypeScript interfaces (AppConfig, DeviceInfo, Profile, etc.)
- `shared/platform.ts`: typed Tauri invoke wrappers — zero raw `invoke()` calls remain in components
- `hooks/useUpdater.ts`: unified update flow replacing duplicate implementations in App.tsx and About.tsx
- 10 components updated to import from shared types/platform instead of inline interfaces and raw invoke
- All inline interface duplication eliminated (AppConfig, DeviceInfo, Profile, IndustryPackInfo, ReplacementRule, PillState)
- Zero TypeScript errors, clean Vite build

## Phase 3: Runtime & Performance ✅ COMPLETED (2026-03-20)

Primary objective:

- Move runtime UI synchronization to event-driven contracts
- Address architectural performance bottlenecks

Results:

- Pill UI rewritten from 80ms polling loop (12.5 IPC/sec) to event-driven via `recording-state` and `recording-level` events
- Sidecar status in ModelSelector replaced from 2s polling to event-driven via new `sidecar-status` event
- WaveformDisplay updated to use typed `onAudioLevel` listener from platform.ts
- Typed event listen wrappers added to `shared/platform.ts`: `onRecordingState`, `onRecordingLevel`, `onAudioLevel`, `onSidecarStatus`
- `sidecar-status` event emitted from Rust after sidecar spawn (success or failure)
- Replacement regex compilation cached in `LazyLock<Mutex<HashMap>>` — avoids recompile on every transcription
- Cache invalidation added when dictionary or industry packs change
- Zero raw `listen()` imports outside `shared/platform.ts`
- Both TypeScript and Rust build clean, zero errors

## Phase 4: Hardening ✅ COMPLETED (2026-03-20)

Primary objective:

- Remove remaining inconsistencies and platform risk

Results:

- Version metadata synchronized: `tauri.conf.json` updated from 0.4.3 to 0.4.4 (matching `Cargo.toml` and `package.json`)
- CSP policy set: was `null` (wide open), now restricts to `self`, IPC, and GitHub (for updater)
- `PillUiState` fully removed: deleted `app/state.rs`, removed `get_pill_state` command, cleaned `emit_all`, removed PillUiState from recorder.rs and lib.rs managed state
- Eliminated unnecessary `unsafe` in `play_sound`: was using `from_raw_parts` to reconstruct a `&'static [u8]` from pointer arithmetic — now passes the static reference directly since `&'static [u8]` is `Send`
- Model strategy resolved: product policy is `tiny`-only, startup enforcement simplified, dead migration logging removed
- Remaining `unsafe` blocks documented and reviewed:
  - `lib.rs`: `SystemParametersInfoW` for work area query (pill positioning) — necessary Win32 API
  - `output/mod.rs`: `GetForegroundWindow`, `SetForegroundWindow`, `SendInput`, `OpenProcess`, `QueryFullProcessImageNameW`, `PlaySoundW` — necessary Win32 text insertion and sound playback
  - `infra/platform.rs`: `CoInitializeEx`, `CoCreateInstance` for IMMDeviceEnumerator — necessary COM API for audio device enumeration
  - `recorder.rs`, `platform.rs`: `unsafe impl Send for RecordingStream/AudioStream` — required because `cpal::Stream` does not implement `Send`
- `cfg!(debug_assertions)` used correctly in `vocabulary.rs` and `sidecar.rs` for dev/prod path resolution
- Both TypeScript and Rust build clean, zero errors, zero warnings

## Migration Rules

- Refactor one workflow at a time.
- Keep existing external behavior stable during ownership refactors.
- Preserve command names temporarily if needed, but move logic behind the correct feature boundary.
- Temporary adapters are allowed only as migration seams, not as permanent architecture.
- Do not refactor multiple high-risk runtime paths at once.

## Validation Checklist Per Phase

- App launches correctly
- Tray works
- Main window hide/show behavior works
- Pill renders and updates correctly
- Hotkey starts and stops recording
- Offline transcription works
- Cloud transcription works
- Post-processing works
- Text insertion works in target apps
- Settings persist across restart
- Onboarding still completes successfully

## Resolved Product Decision

### Model Strategy — DECIDED: `tiny` only

The product uses `tiny` as the permanent runtime model. Startup enforces this policy.
The multi-model infrastructure is retained in `speech/mod.rs` (load_model, list_downloaded_models)
but the UI does not expose model switching. This can be re-evaluated if user demand emerges.

## Execution History

1. Phase 1 - Rust feature extraction ✅ 2026-03-20
2. Phase 2 - Frontend modularization ✅ 2026-03-20
3. Phase 3 - Runtime & performance ✅ 2026-03-20
4. Phase 4 - Hardening ✅ 2026-03-20

## Final Note

This refactor should be judged by whether it improves ownership, contracts, and replaceability of modules.

If a change only renames files, adds wrappers, or moves logic without reducing coupling, it does not satisfy the intent of this plan.
