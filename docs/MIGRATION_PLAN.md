# YoloVoice Architecture Migration Plan
## Parakeet TDT v3 + Sidecar Elimination + VAD-Segmented Pipeline

**Created:** 2026-03-22
**Status:** Planning

---

## 1. Context & Motivation

### Current Architecture
- **Inference:** faster-whisper (Python sidecar) with Whisper `tiny` model, INT8 on CPU
- **Pipeline:** Batch — record full audio → save WAV → send to sidecar → wait → insert text
- **Latency:** 15-25 seconds for a 10-second utterance (sidecar boot + WAV write + inference)
- **Bundle size:** ~500MB+ for bundled Python environment
- **GPU:** CUDA only (NVIDIA)
- **Languages:** Whisper tiny supports 99 languages but at ~14% WER (poor accuracy)

### Problems
1. Python sidecar is the biggest architectural complexity (spawn, health check, restart, bundled env)
2. Batch processing makes everything feel sluggish regardless of model speed
3. Whisper tiny has poor accuracy (14% WER) — no built-in punctuation or capitalization
4. CUDA-only GPU support excludes AMD and Intel GPU users
5. ~500MB Python environment inflates the installer

### Target Architecture
- **Inference:** Parakeet TDT 0.6B v3 via `parakeet-rs` (Rust-native ONNX Runtime)
- **Pipeline:** VAD-segmented — continuous recording with phrase-level chunking
- **Latency:** <1 second per segment on GPU
- **Bundle size:** ~50-100MB app + ~2.4GB model download (on first run)
- **GPU:** DirectML (Windows AMD/Intel/NVIDIA), CUDA (NVIDIA), CoreML (future macOS)
- **Languages:** 25 with auto-detection, built-in punctuation & capitalization, 6.34% WER

---

## 2. Key Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Model** | Parakeet TDT 0.6B v3 | Best accuracy (6.34% WER), built-in punctuation/caps, Russian support, anti-hallucination training |
| **Model format** | ONNX FP32 (istupakov export) | Community reference, 26k downloads, no quality loss vs NeMo original |
| **Inference engine** | `parakeet-rs` Rust crate (wraps `ort`) | Handles mel spectrogram, TDT decoding, tokenization — no Python needed |
| **GPU strategy** | DirectML primary, CUDA optional | DirectML covers all Windows GPUs (AMD, Intel, NVIDIA). CUDA as opt-in for NVIDIA users wanting max speed |
| **Chunking** | VAD-segmented (Silero VAD) | Natural phrase/sentence boundaries, each segment 3-15s, independent inference calls |
| **Streaming** | Not needed for MVP | Parakeet TDT is offline model. Sub-second inference on GPU makes batch-per-segment fast enough |
| **Single model** | Yes — no model selection UI | Users don't care about model names. One model that works well |
| **macOS** | Future phase, enabled by architecture | CoreML execution provider via same `parakeet-rs` + `ort` stack |

---

## 3. Model Details

### NVIDIA Parakeet TDT 0.6B v3

| Spec | Value |
|------|-------|
| Parameters | 600M |
| Architecture | FastConformer-TDT (Transducer with Duration) |
| ONNX source | [istupakov/parakeet-tdt-0.6b-v3-onnx](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx) |
| Files | `encoder.onnx` (~2.3GB) + `decoder_joint.onnx` (~70MB) + `vocab.txt` |
| Total download | ~2.4 GB |
| License | CC-BY-4.0 (commercial OK, attribution required) |
| Languages | 25 (auto-detected): EN, RU, UK, DE, FR, ES, IT, PT, NL, PL, CS, SK, BG, HR, DA, EL, ET, FI, HU, LV, LT, MT, RO, SL, SV |
| English WER | 6.34% average (Open ASR Leaderboard) |
| Russian | Supported natively |
| Punctuation | Built-in (all languages) |
| Capitalization | Built-in (all languages) |
| Timestamps | Character, word, and segment level |
| Input | 16kHz mono audio |
| Max per call | ~4-5 minutes (sufficient for VAD segments) |
| Anti-hallucination | Trained on 36k hours of silence data |
| Noise robustness | 12% WER degradation at SNR 10dB |

### Future Model Option: FP16 (Size Reduction)
- [grikdotnet/parakeet-tdt-0.6b-fp16](https://huggingface.co/grikdotnet/parakeet-tdt-0.6b-fp16)
- ~1.25 GB (half of FP32)
- Minimal quality loss on GPU inference
- Can be offered as alternative download after initial release

---

## 4. Rust Integration Stack

### Primary: `parakeet-rs` (v0.3.4+)

```toml
[dependencies]
parakeet-rs = { version = "0.3", features = ["directml"] }
```

**What it provides:**
- Audio preprocessing (16kHz mono → mel spectrogram via `realfft`)
- ONNX model loading and inference via `ort` crate
- TDT-specific transducer decoding (greedy + beam search)
- Tokenization via HuggingFace `tokenizers` crate
- GPU execution providers via feature flags

**Feature flags for GPU:**
- `directml` — Windows (AMD, Intel, NVIDIA via DirectX 12)
- `cuda` — NVIDIA only (requires CUDA 12.8+ and cuDNN 9.19+)
- `coreml` — macOS (future)
- `tensorrt` — NVIDIA optimized (future)

**Dependencies it brings:**
- `ort` (ONNX Runtime Rust bindings)
- `ndarray` (tensor operations)
- `realfft` (FFT for mel spectrogram)
- `hound` (WAV reading — we may bypass this since we have raw audio buffers)
- `tokenizers` (HuggingFace tokenizer)

### Fallback Strategy
If `parakeet-rs` becomes unmaintained or insufficient:
- **Option B:** sherpa-onnx official Rust bindings (links C library, pre-exported INT8 models)
- **Option C:** Use `ort` directly + vendor parakeet-rs preprocessing code (it's MIT-compatible)

---

## 5. Pipeline Architecture

### Current Pipeline (Batch)
```
User holds hotkey
  → recorder.rs captures all audio into single buffer
  → User releases hotkey
  → WAV file written to disk (10-50ms)
  → Sidecar spawned / health-checked
  → WAV sent to Python sidecar via command
  → faster-whisper processes entire audio (5-15s on CPU)
  → Text returned
  → Optional LLM post-processing (2-30s)
  → Text inserted at cursor via clipboard + SendInput
  → Total: 15-25 seconds
```

### New Pipeline (VAD-Segmented)
```
User holds hotkey
  → recorder.rs starts continuous audio stream
  → Audio flows through Silero VAD in real-time
  │
  ├─ VAD detects speech segment end (silence gap ≥ 400-600ms)
  │   → Audio segment (3-15s) sent to parakeet-rs
  │   → Parakeet TDT inference (~500ms on GPU)
  │   → Text with punctuation + capitalization returned
  │   → Optional LLM post-processing per segment
  │   → Segment text appended to accumulator
  │   → Pill UI updated with accumulated text
  │   → Continue recording...
  │
  ├─ VAD detects next segment end
  │   → Same process, append to accumulator
  │   → Pill UI updated
  │   → Continue recording...
  │
  └─ User releases hotkey
      → Final partial segment processed
      → Full accumulated text assembled
      → Text inserted at cursor via SendInput
      → Total perceived wait: <1 second after release
```

### Pipeline Data Flow
```
                    ┌──────────────┐
                    │   Mic Input   │
                    │   (cpal)      │
                    └──────┬───────┘
                           │ raw audio stream
                           ▼
                    ┌──────────────┐
                    │  Silero VAD   │
                    │  (segment     │
                    │   detection)  │
                    └──────┬───────┘
                           │ audio segments (3-15s each)
                           ▼
                    ┌──────────────┐
                    │ parakeet-rs   │
                    │ (ONNX Runtime │
                    │  + DirectML)  │
                    └──────┬───────┘
                           │ text with punctuation + caps
                           ▼
                    ┌──────────────┐
                    │ Post-process  │
                    │ • Replacement │
                    │   rules       │
                    │ • Industry    │
                    │   packs       │
                    │ • LLM (opt)   │
                    └──────┬───────┘
                           │ cleaned text segment
                           ▼
                    ┌──────────────┐
                    │ Accumulator   │
                    │ • Append      │
                    │ • Emit to UI  │
                    └──────┬───────┘
                           │ on hotkey release
                           ▼
                    ┌──────────────┐
                    │ Text Insert   │
                    │ (SendInput)   │
                    └──────────────┘
```

---

## 6. Codebase Changes

### Files to DELETE
```
sidecar/                          — entire Python sidecar directory
  ├── main.py
  ├── requirements.txt
  ├── pyproject.toml
  └── ... (all Python files)

src-tauri/src/infra/sidecar.rs    — SidecarProcess, SidecarState, spawn/ensure_running
```

### Files to CREATE
```
src-tauri/src/infra/model.rs      — Model download, storage, validation
                                    • Download from HuggingFace on first run
                                    • Store in app data directory
                                    • Verify file integrity (checksum)
                                    • Report download progress to UI

src-tauri/src/features/speech/
  inference.rs                    — Parakeet inference wrapper
                                    • Initialize parakeet-rs session (once at startup)
                                    • GPU auto-detection (DirectML → CPU fallback)
                                    • Accept audio buffer → return transcribed text
                                    • Handle errors gracefully

  accumulator.rs                  — Segment accumulator
                                    • Collect transcribed segments in order
                                    • Maintain full text state
                                    • Emit accumulated text to UI via events
                                    • Assemble final text on recording stop
```

### Files to MODIFY
```
src-tauri/Cargo.toml              — Add parakeet-rs dependency, remove sidecar-related deps

src-tauri/src/lib.rs              — Remove sidecar spawn/status logic
                                    • Replace with model session initialization
                                    • Remove sidecar status event emission
                                    • Add model status event emission

src-tauri/src/features/capture/
  mod.rs                          — Pipeline orchestration (MAJOR CHANGE)
                                    • Current: record → stop → transcribe → insert
                                    • New: record → VAD segment → transcribe → accumulate → continue → stop → insert
                                    • Add segment processing loop
                                    • Integrate with accumulator

  recorder.rs                     — Continuous recording with segment emission (MAJOR CHANGE)
                                    • Current: records into single buffer, stops on command
                                    • New: continuous stream, emits audio segments on VAD boundaries
                                    • VAD silence threshold configurable (default 400-600ms)
                                    • Keep recording while segments are being processed

src-tauri/src/features/speech/
  mod.rs                          — Replace sidecar transcription with parakeet-rs calls
                                    • Remove: HTTP calls to Python sidecar
                                    • Remove: WAV file creation
                                    • Add: direct audio buffer → parakeet-rs inference
                                    • Punctuation post-processing may be simplified
                                      (Parakeet provides it natively)

src-tauri/src/features/settings/
  mod.rs                          — Update AppConfig
                                    • Remove: model size selection (single model)
                                    • Remove: sidecar-related settings
                                    • Add: GPU preference (auto / DirectML / CUDA / CPU)
                                    • Add: VAD silence threshold setting
                                    • Keep: LLM settings, replacement rules, profiles, industry packs

src-tauri/src/app/
  events.rs                       — Update events
                                    • Remove: sidecar-status event
                                    • Add: model-status event (downloading / ready / error)
                                    • Add: segment-transcribed event (for pill UI updates)
                                    • Keep: recording-state, recording-level, audio-level

  commands.rs                     — Update Tauri commands
                                    • Remove: sidecar-related commands
                                    • Add: model download/status commands
                                    • Update: transcription commands to use new inference path

src/components/
  ModelSelector.tsx               — Replace with model download/status UI
                                    • Current: whisper model size + device selection
                                    • New: download progress, model status, GPU detection display

  Pill.tsx                        — Add accumulated text display
                                    • Current: recording state + level visualization
                                    • New: also show transcribed text segments as they arrive

src/pages/
  Settings.tsx                    — Update settings composition
                                    • Remove sidecar-related sections
                                    • Add GPU preference selector
                                    • Add VAD threshold slider

  Onboarding.tsx                  — Update first-run flow
                                    • Current: download Whisper tiny model via sidecar
                                    • New: download Parakeet TDT v3 ONNX (~2.4GB)
                                    • Show download progress
                                    • GPU auto-detection step

src/shared/
  platform.ts                     — Update invoke/listen wrappers
                                    • Remove sidecar-related calls
                                    • Add model download/status calls
                                    • Add segment-transcribed listener

  types.ts                        — Update TypeScript interfaces
                                    • Remove sidecar types
                                    • Add model status types
                                    • Add segment types
```

---

## 7. Implementation Phases

### Phase 1: Inference Engine Swap (Core Migration)
**Goal:** Replace Python sidecar with parakeet-rs. Keep batch pipeline for now.

**Steps:**
1. Add `parakeet-rs` to Cargo.toml with `directml` feature
2. Create `infra/model.rs` — model file management (download, store, validate)
3. Create `features/speech/inference.rs` — parakeet-rs session wrapper
4. Update `features/speech/mod.rs` — route transcription through parakeet-rs instead of sidecar
5. Remove WAV file writing from pipeline — pass raw audio buffer directly
6. Update `lib.rs` — initialize ONNX session at startup instead of spawning sidecar
7. Update events: `sidecar-status` → `model-status`
8. Update `commands.rs` — replace sidecar commands with model commands
9. Delete `sidecar/` directory and `infra/sidecar.rs`
10. Update frontend: `ModelSelector.tsx`, `platform.ts`, `types.ts`

**Verification:**
- Hold hotkey → speak → release → text appears (same UX as today)
- Inference time should be <1s on GPU vs 5-15s currently
- No Python process in task manager
- Punctuation and capitalization appear automatically (new!)

**Risk mitigation:**
- Keep a git branch with sidecar code until Phase 1 is fully verified
- Test on machines with AMD GPU (DirectML), NVIDIA GPU (DirectML + CUDA), and CPU-only

### Phase 2: VAD-Segmented Pipeline
**Goal:** Enable continuous recording with real-time segment processing.

**Steps:**
1. Refactor `recorder.rs` — continuous audio stream with VAD boundary detection
2. Create `features/speech/accumulator.rs` — segment collection and text assembly
3. Refactor `capture/mod.rs` — segment processing loop instead of single-shot
4. Add `segment-transcribed` event — emits each processed segment to frontend
5. Update `Pill.tsx` — display accumulating text as segments arrive
6. Add VAD silence threshold to settings (default 500ms, configurable 200-800ms)
7. Handle edge cases:
   - User releases hotkey mid-segment → process final partial segment
   - Very short utterances (< 1 second) → process as single segment
   - Very long silence → don't emit empty segments
   - Parakeet inference fails on one segment → skip, continue with next

**Verification:**
- Hold hotkey → speak for 60+ seconds with natural pauses → text appears in chunks in pill
- Release hotkey → full text inserted
- Each segment shows up within ~1 second of the pause
- Long dictation (2-5 minutes) works without degradation

### Phase 3: Onboarding & Polish
**Goal:** Smooth first-run experience and production readiness.

**Steps:**
1. Update `Onboarding.tsx` — model download flow with progress bar (~2.4GB download)
2. GPU auto-detection UI — show detected GPU, selected execution provider
3. Update settings page — remove sidecar sections, add GPU preference, VAD threshold
4. Error handling — model not found, GPU initialization failure, inference timeout
5. Graceful degradation — if DirectML fails, fall back to CPU with user notification
6. Update auto-updater — handle model file separately from app updates (don't re-download model on app update)
7. Update installer — remove Python bundling, shrink NSIS installer significantly

**Verification:**
- Fresh install → onboarding downloads model → app works
- App update → model persists, no re-download
- Machine without GPU → CPU fallback works (slower but functional)

### Phase 4: macOS Support (Future)
**Goal:** Cross-platform release.

**Steps:**
1. Add `coreml` feature flag to parakeet-rs dependency
2. Platform-specific text insertion: `features/output/mod.rs`
   - Windows: Win32 SendInput (existing)
   - macOS: CGEvent / Accessibility API
3. Platform-specific hotkey handling — verify `rdev` works on macOS
4. Platform-specific audio capture — verify `cpal` works on macOS
5. Code signing + notarization for macOS
6. DMG installer
7. Test on Apple Silicon (ARM) and Intel Macs

---

## 8. What Stays Unchanged

These features are unaffected by the migration:

| Feature | Why Unchanged |
|---------|---------------|
| Replacement rules | Applied after transcription — input is still text |
| Industry packs | Same — post-processing step |
| Profiles | Same — configuration layer |
| LLM post-processing | Same — receives text, returns enhanced text |
| Hotkey system (rdev) | No dependency on inference engine |
| Pill UI (visual) | Recording state events unchanged, just adding text display |
| System tray | No dependency on inference engine |
| Auto-updater | Independent system |
| Settings persistence | AppConfig structure changes slightly but mechanism is the same |
| Audio device selection | cpal-based, independent of inference |

---

## 9. What Gets Simplified

| Current Complexity | After Migration |
|--------------------|-----------------|
| Sidecar spawn + health check + restart | Gone — in-process inference |
| Bundled Python environment (~500MB) | Gone — pure Rust |
| WAV file write to temp directory | Gone — raw audio buffer passed directly |
| Manual punctuation post-processing | Simplified — Parakeet provides it natively |
| Model size selection UI | Gone — single model, no user choice |
| Language selection | Gone — Parakeet auto-detects from 25 languages |
| Sidecar path resolution (NSIS vs dev) | Gone — no sidecar |

---

## 10. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `parakeet-rs` crate abandoned | LOW (active, v0.3.4, Mar 2026) | MEDIUM | Vendor the code or migrate to sherpa-onnx Rust bindings |
| ONNX model quality differs from NeMo | VERY LOW (FP32 export is lossless) | HIGH | Test before committing; istupakov export has 26k downloads with no quality complaints |
| DirectML doesn't work on some GPUs | LOW | MEDIUM | CPU fallback is built-in; test on AMD, Intel, NVIDIA |
| 2.4GB model download deters users | MEDIUM | MEDIUM | Future: offer FP16 (1.25GB) as default, FP32 as quality option |
| VAD segment boundaries split words | LOW | LOW | Configurable silence threshold; natural speech has clear pause points |
| Memory usage too high (600M param model) | LOW | MEDIUM | ONNX Runtime manages memory efficiently; test on 8GB RAM machines |
| parakeet-rs 4-5 min limit per call | NONE | NONE | VAD segments are 3-15 seconds each |

---

## 11. Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Inference latency (10s utterance) | 5-15s (CPU) / 1-3s (GPU) | <500ms (GPU) / 2-3s (CPU) |
| End-to-end latency (10s utterance) | 15-25s | <2s |
| Installer size | ~500MB+ (with Python) | ~50-100MB (without model) |
| First segment visible | Never (batch) | ~1s after first pause |
| English WER | ~14% (Whisper tiny) | 6.34% (Parakeet v3) |
| Punctuation | Manual post-processing | Built-in |
| Capitalization | Manual post-processing | Built-in |
| GPU support | NVIDIA only (CUDA) | All vendors (DirectML) |
| Supported languages | 99 (poor quality) | 25 (high quality, auto-detected) |
| Startup time | 3-5s (sidecar boot) | <1s (in-process) |

---

## 12. Future Intelligence Layer (Out of Scope for This Plan)

The VAD-segmented architecture naturally supports adding intelligence modes later:

```
features/
  intelligence/          ← future module
    mod.rs              — mode router (dictation / answer / summarize / code)
    context.rs          — rolling LLM context across segments
    processor.rs        — LLM calls with progressive refinement
```

Each transcribed segment feeds into the intelligence layer progressively, so the LLM can start processing before the user finishes speaking. This is enabled by the segmented pipeline but not part of the current migration.

---

## 13. References

- [NVIDIA Parakeet TDT 0.6B v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) — original model
- [istupakov ONNX Export](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx) — FP32 ONNX files
- [parakeet-rs](https://github.com/altunenes/parakeet-rs) — Rust inference crate
- [ort](https://github.com/pykeio/ort) — Rust ONNX Runtime bindings
- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) — fallback inference option
- [parakeet.cpp](https://github.com/Frikallo/parakeet.cpp) — C++ reference implementation
- [grikdotnet FP16](https://huggingface.co/grikdotnet/parakeet-tdt-0.6b-fp16) — future smaller model option
- [Competitive Analysis](./COMPETITIVE_ANALYSIS.md) — full competitor research
- [STT Engine Research](./STT_ENGINE_RESEARCH.md) — prior model/architecture analysis
