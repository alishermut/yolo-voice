# YOLO Voice — Comprehensive Code Review (2026-07-18)

Scope: full codebase analysis (~15k lines) — Rust backend (`src-tauri`), React frontend (`src`), Python sidecar (`sidecar`) — with a focused root-cause investigation of the unreliable push-to-talk hotkey.

---

## 1. Push-to-talk reliability — root cause analysis

The "sometimes the hotkey doesn't work" symptom has **one dominant root cause and several contributing bugs**.

### 1.1 ROOT CAUSE: heavy work runs inside the Windows low-level keyboard hook callback

The chain:

1. `rdev::listen` (`hotkey.rs:659`) installs a `WH_KEYBOARD_LL` hook. The callback runs **inside the OS hook procedure** on the listener thread.
2. The callback calls `app.emit("hotkey-action", ...)`. In Tauri v2, Rust-side `listen` handlers are invoked **synchronously on the emitting thread** — so `handle_start` / `handle_stop` (`capture/mod.rs:189, 413`) execute **inside the keyboard hook callback**.
3. `handle_start` does, synchronously: config mutex lock + clone, foreground-window capture, dropping any previous cpal stream, VAD state locks, event emits to both webviews, media play/pause keystroke injection, and — worst — **audio device resolution + WASAPI stream construction + `stream.play()`** (`recorder.rs:141-285`). Cold device enumeration alone can take 100–500 ms.
4. Windows enforces a timeout on low-level hooks (`LowLevelHooksTimeout`, ~200–1000 ms). A callback that repeatedly exceeds it gets its hook **silently removed by the OS**. From that moment the hotkey is dead until app restart — no error, no log, because `rdev::listen` doesn't return; events just stop arriving.

This exactly matches an intermittent "button stops working" symptom: it works after launch, then dies after a slow start (cold device, busy disk, AV scan), and only a restart fixes it.

Additional blocking work on the same hook thread:

- **`handle_stop` cloud path**: `stop_and_save` writes the entire WAV to disk sample-by-sample (`recorder.rs:315-338`) synchronously in the handler.
- **`handle_stop`**: reads all profile JSON files from disk (`capture/mod.rs:444-451`).
- **`resolve_style_shortcut_key`** (`hotkey.rs:397-409`): reads the profiles directory from disk on **every non-hotkey keypress while dictating**.
- **Style-switch handler** (`capture/mod.rs:1696-1730`): triggered from the hook callback, reads profiles from disk **and calls `save_config` (disk write + fsync) while holding the `ConfigState` mutex** — the same mutex the hook callback locks on *every keystroke and mouse event system-wide* (`hotkey.rs:456-461`). A slow fsync stalls all keyboard input machine-wide and burns the hook timeout budget.
- **Every input event** (including high-frequency mouse-move events, which rdev also delivers) locks two mutexes and clones `cmd_chord` (a `Vec` allocation) (`hotkey.rs:450-453`).

**Fix (highest priority):** make the rdev callback do nothing but state-machine transitions and enqueue actions to a channel; run `handle_start`/`handle_stop`/style-switch on a dedicated worker thread (or `tauri::async_runtime`). Early-return immediately on mouse events. Cache the voice-activated flag and profile shortcut keys in `HotkeyCache`-style atomics/`ArcSwap` instead of locking `ConfigState` and reading disk per event.

### 1.2 Letter/punctuation keys silently unbindable as the dictation hotkey

`KeybindingInput.tsx:118` converts `KeyA` → `"A"` and saves it, but the backend's single-key `parse_key()` (`hotkey.rs:42-112`) has **no arm for letter keys** — only the chord parser has the letter fallback. Setting the dictation hotkey to a letter yields `dict_key = None`: the UI shows the binding as saved, but push-to-talk never fires, with no error anywhere. Punctuation keys (`Backquote`, `Minus`, …) return `null` in `codeToRdev` and can't be bound at all.

**Fix:** add the letter fallback to `parse_key` (reuse `parse_chord`'s logic); surface "unsupported key" in the UI; add a Rust test asserting every frontend-emittable key name round-trips through `parse_key`/`parse_chord`.

### 1.3 Double-tap race: short tap can leave recording stuck on

Short tap (<500 ms) → `WaitingForDoubleTap` → a delayed-stop thread sleeps 600 ms and then only emits "stop" **if** `recording_mode() == Dictation` (`hotkey.rs:357-379`). But `recording_mode` is set to `Dictation` only *after* `start_recording` returns Ok (`capture/mod.rs:323`). If device init takes longer than tap + 600 ms (cold start — the same slow path as §1.1), the delayed stop sees `None`, does nothing, then the start completes → **recording runs forever**. The next press hits `Idle` state with backend mode `Dictation` and emits "stop" — so the press that should start dictation stops the stuck one instead. To the user: "I pressed the button and it did the opposite / did nothing."

**Fix:** have the delayed-stop worker wait for start completion (e.g., check a "start pending" flag / retry briefly) or make `handle_start` set an intent flag before device init so the stop token logic can act on it.

### 1.4 Listener death is unrecoverable and invisible

If `rdev::listen` errors (`hotkey.rs:659-661`) the thread logs once and exits; if the OS unhooks it (§1.1), nothing is logged at all. There is no watchdog, no restart, no user-facing indicator.

**Fix:** add a supervisor that restarts the listener thread and a periodic health check (e.g., timestamp of last event seen; if input is happening system-wide but no events arrive, re-install). Surface listener status in Settings/diagnostics.

### 1.5 Unsound `Send` on the cpal stream

`unsafe impl Send for RecordingStream` (`recorder.rs:107`) and `infra/platform.rs:119` force `cpal::Stream` (deliberately `!Send`, WASAPI/COM thread affinity) across threads: created on the hook thread, dropped on VAD/worker threads. Mostly works on WASAPI today but is UB by cpal's contract and a plausible source of rare hangs on stop.

**Fix:** own the stream on a dedicated audio thread; state holds only a stop flag/channel (the machinery half-exists already).

### 1.6 Smaller PTT-adjacent issues

- Short-tap stop is always delayed by the 600 ms double-tap window — a deliberate trade-off, but it makes quick taps feel laggy; consider making double-tap-to-toggle opt-in.
- Startup ordering (`lib.rs:299` vs `:406`): the hotkey listener starts before the `hotkey-action` handlers are registered — a keypress in the first instants after launch is dropped. Register handlers first.
- Chord capture UI wipes held keys when the Settings page re-renders mid-capture (`KeybindingInput.tsx:184-202`) — binding a chord can silently fail; Escape can't cancel capture and instead becomes the hotkey; window blur leaves capture armed.

---

## 2. Critical / High findings (rest of codebase)

### Rust backend

| # | Severity | Finding |
|---|----------|---------|
| B1 | Critical | **Sidecar RPC has no timeout** (`distil_whisper.rs:518-537`): blocking `read_line` while holding the `DistilWhisperState` mutex. A wedged Python process (CUDA hang, OOM) deadlocks all Distil features, status polls, and even app shutdown (`Drop` waits for a reply). Fix: reader thread + `recv_timeout`, kill/respawn on expiry; never wait for a reply in `shutdown()`. |
| B2 | High | **Respawned sidecar falsely marked `loaded = true`** (`distil_whisper.rs:282-300`): if the child dies right after `load_model`, `ensure_process` spawns a fresh one that gets flagged as loaded → every transcription fails until it dies again. |
| B3 | High | **Config/profile/vocabulary writes are not atomic and the fsync is a no-op** (`settings/mod.rs:471`, `profiles.rs:142`, `text_actions.rs:160`, `vocabulary.rs:316,410`): `sync_all` is called on a *read-only re-open* (fails silently on Windows) and `fs::write` truncates in place. A crash mid-write corrupts the file; `load_config` then does `unwrap_or_default()` — **silently wiping hotkey, API keys, and all settings**. Fix: one shared `write_json_atomic` helper (tmp + fsync + rename); back up corrupt configs instead of defaulting. |
| B4 | High | **Model download has no size/integrity check** (`infra/model.rs:321-358`): clean early EOF marks truncated multi-GB weights as installed (`is_model_downloaded` only checks `len > 0`); temp files leak on read errors. Compare against the already-fetched Content-Length; delete temp on failure. |
| B5 | Medium | **GPU runtime provisioning (multi-GB pip install) runs under the sidecar mutex from the transcribe path** (`distil_whisper.rs:348-364 → 875-890`) — app appears frozen for minutes. Provision only from background-prepare. |
| B6 | Medium | LLM/cloud responses parsed with `unwrap_or("")` (`llm.rs:348`, `cloud.rs:59,99`): HTTP 200 with unexpected shape returns `Ok("")` → user's dictation replaced with nothing. HTTP error bodies discarded. Treat missing content as `Err` with body snippet. |
| B7 | Medium | `save_config_cmd` read-compare-write race (`commands.rs:288-333`) + separate `set_launch_on_startup` save path: concurrent saves silently revert each other. Funnel all mutation through one `update(closure)` helper. |
| B8 | Medium | Windows device names zipped by index across two different enumerations (`infra/platform.rs:78-112`) — Settings can label the wrong microphone. Match by name instead. |
| B9 | Medium | `convert_number_words` drops the word "and" between numbers ("five and ten" → "5 10") (`cleanup.rs:874-890`). Sentence-loop collapse rewrites `!`/`?` to `.` (`cleanup.rs:540-548`). |
| B10 | Medium | `SegmentAccumulator::finalize` blocks unboundedly on a stalled inference worker (`accumulator.rs:66-73`); stop-recording hangs with no feedback. Add `recv_timeout` and return partial segments. |
| B11 | Medium | Regex compile-failure fallback `(?:)` matches everywhere → a rule over the size limit would inject its replacement between every character (`vocabulary.rs:110-117`). Skip failed rules instead. |
| B12 | Medium | `get_model_status` reports "error" during the legitimate multi-second load window (`commands.rs:842-857`) — UI flashes bogus errors at launch. Add a real `loading` state. |
| B13 | Medium | Diagnostics store failure aborts app startup (`lib.rs:161-163`) — dictation app refuses to start over optional telemetry. Degrade gracefully. |
| B14 | Medium | Startup loads Parakeet into RAM even when Distil/cloud is selected (`lib.rs:319-324`); switching cloud→offline doesn't warm the selected engine (`commands.rs:334-341`). |
| B15 | Medium | API keys stored in plaintext `config.json` although `keyring` is already a dependency (`settings/mod.rs:46-104`). |
| B16 | Medium | No SQLite `busy_timeout`; clearing history mid-dictation can hit "database is locked"; deleting `-wal`/`-shm` with a live connection is unsafe (`diagnostics/mod.rs:442,278`). |

### Frontend

| # | Severity | Finding |
|---|----------|---------|
| F1 | Critical | Letter-key hotkey contract mismatch — see §1.2. |
| F2 | High | **`updateConfig` stale-closure race** (`Settings.tsx:88-113`): concurrent saves build from the same stale base and apply responses in completion order — settings (including a just-set hotkey) silently revert. Serialize saves + sequence counter. |
| F3 | High | API-key/model text inputs call `updateConfig` per keystroke through an async round-trip (`TranscriptionSection.tsx:459`, `CommandSection.tsx:124-165`): dropped characters, config write storm, hotkey re-parse per keystroke. Local state + save on blur/debounce. |
| F4 | High | Chord capture wiped by parent re-render mid-capture (`KeybindingInput.tsx:184-202`) — see §1.6. |
| F5 | Medium | Settings digit shortcuts (1–7) fire while capturing a keybinding (`Settings.tsx:116-133`) — binding a digit also navigates sections. Use capture-phase + `stopImmediatePropagation` or a "capture active" flag. |
| F6 | Medium | No Escape-to-cancel / blur-to-cancel in key capture; Escape becomes the hotkey (`KeybindingInput.tsx:143-202`). |
| F7 | Medium | History search: per-keystroke invokes, no debounce, out-of-order responses (`HistorySection.tsx:42-69`). |
| F8 | Medium | Mic test keeps the microphone open after `MicSelector` unmounts (no `stopTest` cleanup) (`MicSelector.tsx:33-47`). |
| F9 | Medium | Pill overlay fully re-renders on every `recording-level` event although `Waveform` already reads a ref (`Pill.tsx:169-171`). Drop the state update. |
| F10 | Medium | `ModelSelector` auto-prepare effect can loop invokes with no backoff (`ModelSelector.tsx:201-226`). |
| F11 | Low | `ProfileEditor` has its own drifted key map (digits missing) and saves shortcuts that conflict with the PTT hotkey (warning only) (`ProfileEditor.tsx:198-237`). |

### Python sidecar / config

| # | Severity | Finding |
|---|----------|---------|
| S1 | High | **`sidecar/transcribe.py` is orphaned legacy code** — only `distil_whisper.py` is bundled and referenced. It also violates the one-line-per-request protocol (unsolicited "ready" line, multi-line progress) and `requirements.txt` can't run it (`faster-whisper`/`ctranslate2` absent). Delete it (or move to `attic/`) and document the sidecar protocol. |
| S2 | Medium | No CPU fallback when a CUDA model load fails despite GPU detection passing (`transcribe.py:100-138`; verify `distil_whisper.py` too). |
| S3 | Medium | Sidecar stdin decodes with the Windows ANSI code page — non-ASCII text (Cyrillic vocabulary, prompts) becomes mojibake. `sys.stdin.reconfigure(encoding="utf-8")` + `PYTHONUTF8=1` at spawn. |
| S4 | Low | Profile `id` from requests used unvalidated in file paths (path traversal); nearest-neighbor resampling aliases audio; pill window granted `updater`/`process` capabilities it doesn't need (`capabilities/default.json`). |
| S5 | Low | Repo root contains an untracked file literally named `nul` (Windows artifact of a `> NUL` redirect) — delete it from Git Bash: `rm ./nul`. |

---

## 3. Performance opportunities

1. **Input-hook hot path** (biggest win, also fixes reliability): no mutex locks, no allocations, no disk I/O per event — see §1.1.
2. **Warm-start latency**: warm device cache exists but is consumed on use and only re-warmed after start; a start during the re-warm window pays cold enumeration inside the hook. Keep the cache persistent (clone out, don't `take`).
3. **Cloud stop path**: write WAV on the worker thread, not the handler; `stop_and_save` writes sample-by-sample — buffer via `encode_wav_bytes` + single write.
4. **Unbounded recording buffer**: non-VAD samples grow without cap during capture (~11 MB/min at 48 kHz); the 5-minute cap is applied only at transcription. Cap at capture time.
5. **Pill render loop**: drop per-event re-renders (F9); keep the 30 fps `recording-level` emits but consider only emitting when the value changes materially.
6. **`get_models_dir` re-reads config from disk on every call** (`model.rs:73-76`); sounds open a fresh `OutputStream` per chime (`output/mod.rs:810-841`).
7. **HTTP client**: single shared 60 s timeout is too short for cold Ollama and long cloud uploads, too long for quick failures — per-use-case timeouts.

---

## 4. Architectural improvements

1. **Sidecar RPC layer**: a small supervisor owning the child process — reader thread, request IDs, per-request timeouts, health checks, kill-and-respawn — eliminates B1/B2/B5 and the whole "app frozen" class.
2. **One atomic-persistence helper** (`write_json_atomic`): fixes B3 across settings, profiles, text actions, vocabulary in one place; back up corrupt files instead of silently defaulting.
3. **Typed errors** (`thiserror` enums) instead of `Result<_, String>` everywhere — makes the silent-empty/swallow patterns (B6) impossible to write accidentally.
4. **Frontend config store**: one `useConfigStore` with optimistic state, serialized save queue, and stale-response rejection kills F2/F3 and the IPC storm.
5. **Single source of truth for key names**: shared `shared/keys.ts` map + Rust round-trip test — the three drifted frontend maps and `parse_key` mismatch (F1, F11) can't recur.
6. **Hotkey engine extraction**: move the state machine out of the rdev callback into a testable module fed by an event channel; the callback becomes ~10 lines. Enables unit tests for the tap/hold/double-tap logic that currently can't be tested at all.

---

## 5. Suggested fix order

1. §1.1 — move all work off the hook callback (root cause of flaky PTT).
2. §1.2 / F1 — letter-key parse fallback + shared key map.
3. §1.3 — delayed-stop race; §1.4 — listener watchdog.
4. B3 — atomic config writes + corrupt-config backup (data-loss risk).
5. B1/B2 — sidecar timeouts and load-state correctness.
6. F2/F3 — config save serialization and debounced inputs.
7. Remaining mediums opportunistically; delete `sidecar/transcribe.py` and the stray `nul` file.

---

*Generated from a multi-agent review (Rust pipeline, React frontend, Python sidecar/config) plus a manual trace of the hotkey → capture → recorder path.*
