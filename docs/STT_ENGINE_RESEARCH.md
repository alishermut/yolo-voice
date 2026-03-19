# STT Engine Research & Architecture Notes

## Current State (March 2026)

YOLO Voice uses **faster-whisper** (CTranslate2 backend) with the `base` model, INT8 on CPU.
Both local and Groq API feel equally slow due to **batch processing architecture** — the entire utterance is recorded, saved as WAV, then sent for processing. Users see nothing until the full pipeline completes.

---

## Model Landscape for CPU-Only Local Dictation

### 1. Moonshine
- Built for edge/mobile, low latency
- Good for real-time dictation
- **Downside:** Smaller ecosystem, limited language coverage

### 2. whisper.cpp
- Mature, portable, proven on desktops and weak hardware
- Supports quantization, x86 AVX, ARM NEON
- **Downside:** Not designed for ultra-low-latency streaming; larger models punish CPU

### 3. faster-whisper (current choice)
- CTranslate2 backend, competitive with whisper.cpp on CPU with INT8
- Mature Python API, good integration with Silero VAD
- **Downside:** Still batch-oriented; no native streaming partial output

### 4. sherpa-onnx
- Strong runtime for local streaming + non-streaming ASR
- Supports VAD workflows, many deployment options
- **Downside:** More moving parts, higher integration burden

### 5. Vosk
- Very lightweight (~50MB models, ~300MB RAM)
- Easy to ship, many languages, works on budget devices
- **Downside:** Quality won't feel premium; not "wow factor" level

---

## Decision: Single Model, No User Selection

**Ship `tiny` INT8 on CPU.** Rationale:
- ~4x faster than `base` on CPU
- For dictation (short utterances, known language), quality is sufficient
- Users don't care about model names — they care about speed
- Eliminates decision paralysis and download confusion

If quality complaints arise later, upgrade to `base` INT8 as the single default — but only after streaming is implemented so perceived latency stays low.

---

## Two-Tier Architecture (Recommended for Future)

### Tier A: Fast Draft (streaming, while user speaks)
- Moonshine or lightweight sherpa-onnx model
- VAD-first to skip silence
- Stream partial text to UI immediately

### Tier B: Final Correction (after end-of-utterance)
- faster-whisper or whisper.cpp with `tiny`/`base` INT8
- Replace draft text with cleaner final transcript
- Optional LLM post-processing for punctuation/cleanup

**Result:** Low perceived latency + better final quality.

---

## Local vs Cloud: Clear Differentiation Strategy

| Aspect | Local (Offline) | Cloud (Groq/Deepgram) |
|--------|----------------|----------------------|
| **Speed** | Fast — streaming partial results | Slower — network round-trip |
| **Quality** | Good enough for dictation | Noticeably better (large model) |
| **Model** | `tiny` INT8 | whisper-large-v3 |
| **Post-processing** | Optional | Always-on cleanup |
| **User expectation** | "Instant, good enough" | "Worth the wait, polished" |

---

## External Analysis (Provided Input)

### Key Insight: It's Not Just the Model
Users judge: startup delay, partial text while speaking, smart punctuation, filler word cleanup, app focus handling, hotkey reliability, correction after final transcript, cursor behavior across apps.

### Recommended Production Architecture
- **Runtime:** sherpa-onnx for streaming pipelines
- **Low-latency model:** Moonshine where supported
- **Fallback:** whisper.cpp or faster-whisper tiny/base quantized
- **VAD:** Silero VAD (already in use)

### What to Avoid
- Large Whisper models on CPU for real-time dictation
- No-VAD pipelines (latency + junk transcription)
- Assuming "works on dev laptop" = works on weak machines
- Universal copy-paste hacks without proper OS text injection

### Versioning Strategy (External Recommendation)
- **V1:** Push-to-talk, local VAD, local transcription, insert final text only
- **V2:** Streaming partial text, final correction replacement, custom vocabulary, punctuation/formatting commands
- **V3:** Hybrid device-aware engine selection, per-app handling rules

---

## Current Pipeline Latency Breakdown

| Stage | Time |
|-------|------|
| Mic → Recording buffer | <1ms |
| WAV write to disk | 10-50ms |
| Silero VAD processing | 50-200ms |
| Whisper inference (base, CPU) | 5-15s |
| Whisper inference (base, GPU) | 1-3s |
| Post-processing (LLM, optional) | 2-30s |
| Cloud transcription (Groq) | 1-5s |
| Text insertion (clipboard + SendInput) | <100ms |

**Critical path (offline, 10s utterance):** 15-25s total wait.
**The recording duration dominates**, masking the difference between local and cloud speed.

---

## Bottom Line

1. **Don't chase model selection** — ship one model that's fast
2. **The slowness is architectural** — batch processing makes everything feel sluggish
3. **Streaming partial transcription is the #1 highest-impact change**
4. **VAD silence threshold** can be tightened from 500ms → 300ms for snappier detection
5. Build a capability-based approach: optimize perceived latency first, then transcript quality
