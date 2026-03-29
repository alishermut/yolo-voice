# YOLO Voice

Offline-first desktop dictation for Windows and macOS. Press a hotkey, speak naturally, and YOLO Voice inserts text into the app you were already using.

Built with Tauri 2, Rust, React, and local speech models.

## Highlights

- `Parakeet` is the default offline model for fast, responsive dictation.
- `Distil-Whisper` is the higher-quality English-focused offline model.
- Cloud transcription is still available through `Groq` and `Deepgram`.
- Command mode can turn spoken instructions into text with an LLM.
- Transcript history is built in, including a manual `Clear all history` action.

## Offline models

### Parakeet

- Default offline model
- Broadest offline language coverage in the app
- Very responsive on supported hardware
- Supports `Fast segmented mode`
  - On: uses VAD-based segmented transcription for much faster response
  - Off: waits until stop and transcribes the whole clip for cleaner wording
- Supports CPU and GPU switching from Settings

### Distil-Whisper

- English-focused offline model
- Whole-clip transcription with speech compaction before inference
- Better raw English dictation quality than Parakeet in many long-form cases
- Slower than Parakeet overall
- GPU is strongly recommended
- CPU fallback works, but is significantly slower on longer clips
- Auto-prepares in the background when selected
- Supports CPU and GPU switching from Settings

## Core features

- Offline dictation with local models
- Cloud fallback with Groq and Deepgram
- Hold-to-talk or toggle recording
- Continuous recording mode
- Command mode with configurable LLM provider/model
- Dictation profiles and style shortcuts
- Custom vocabulary and normalization rules
- Spoken punctuation
- Numbers as digits
- Hallucination filtering
- Built-in transcript history
- Floating recording pill
- System tray support
- Auto-update through GitHub Releases

## Current processing model

### Offline

- `Parakeet`
  - segmented VAD path for speed, or one-shot whole-clip path when segmented mode is off
- `Distil-Whisper`
  - whole-clip transcription with external speech compaction
  - native long-form chunking for longer audio

### Cloud

- `Groq`
- `Deepgram`

## Language support

- `Parakeet`: multi-language offline model with auto language detection
- `Distil-Whisper`: English-focused
- Text cleanup and the strongest post-transcription shaping are still best for English

## Tech stack

- Tauri 2
- Rust backend
- React + TypeScript frontend
- ONNX Runtime / DirectML for Parakeet
- Python Transformers sidecar for Distil-Whisper
- SQLite for transcript history
- cpal for audio capture
- Silero VAD for segmentation and speech compaction

## Development

```bash
npm install
npm run dev
npm run build
cd src-tauri
cargo test
```

## Release flow

- App versions are stored in:
  - `package.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
- GitHub releases are created by pushing a tag like `v0.9.6`
- Release notes live in `docs/releases/`

## License

MIT
