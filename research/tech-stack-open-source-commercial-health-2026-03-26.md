# YOLO Voice Tech Stack, Open Source, Commercial Use, and Health Review

Date: 2026-03-26

## Executive Summary

YOLO Voice is mostly built on commercially friendly open-source infrastructure. The frontend stack, Tauri desktop shell, Rust runtime crates, ONNX runtime layer, Silero VAD, and the `parakeet-rs` crate all use permissive licenses such as MIT, Apache-2.0, or dual MIT/Apache-2.0.

The biggest licensing nuance is the offline speech model itself. The app downloads `istupakov/parakeet-tdt-0.6b-v3-onnx` from Hugging Face, and that converted ONNX model points back to NVIDIA's `nvidia/parakeet-tdt-0.6b-v3` model. Both model pages currently show `cc-by-4.0`, and NVIDIA's model card explicitly says the model is ready for commercial and non-commercial use. That is commercially usable, but it is not "no-obligations" permissive like MIT. You should plan for attribution and keep a clear notice trail.

The biggest unresolved risk is not the code libraries. It is the bundled non-code assets:

- `src-tauri/resources/silero_vad_v5.onnx` is likely fine because Silero VAD is MIT.
- The bundled `.wav` sound files do not appear to have any local provenance, attribution, or license record in this repo.

For a commercial release, I would currently classify the repo as:

- Green: code dependencies and core runtime libraries
- Yellow: Parakeet model attribution and third-party notice hygiene
- Red until fixed: bundled sound asset provenance

## What The Repo Is Actually Using

Current frontend/runtime manifests:

- Frontend/web shell: React, React DOM, TypeScript, Vite, Tailwind, i18next, Tauri JS packages in [package.json](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/package.json#L1)
- Desktop/runtime layer: Tauri, SQLite (`rusqlite`), audio (`cpal`, `hound`), input automation (`rdev`, `enigo`, `arboard`), HTTP (`reqwest`), ONNX (`ort`), resampling (`rubato`), Parakeet Rust bindings in [Cargo.toml](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/Cargo.toml#L1)

Live speech/AI architecture in code:

- Offline STT model downloads from Hugging Face repo `istupakov/parakeet-tdt-0.6b-v3-onnx` in [model.rs](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/src/infra/model.rs#L7)
- Cloud STT uses Groq Whisper and Deepgram Nova in [cloud.rs](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/src/features/speech/cloud.rs#L3)
- LLM post-processing/providers include Ollama, OpenAI, Anthropic Claude, and Groq-compatible chat APIs in [llm.rs](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/src/features/speech/llm.rs#L3)
- Silero VAD ONNX is bundled locally and loaded through `ort` in [vad.rs](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/src/features/speech/vad.rs#L1)

Important note: `docs/PROJECT_SPEC.md` still describes an older faster-whisper Python sidecar design, but the current code has moved the main offline inference path into Rust with `parakeet-rs`. Treat the code as source of truth, not the older spec.

## Open Source and Commercial Use Matrix

### 1. Frontend and desktop framework

These are commercially friendly open source:

| Component | Repo version in app | License status | Commercial use view |
|---|---:|---|---|
| React | 19.2.4 resolved | MIT | Yes |
| React DOM | 19.2.4 resolved | MIT | Yes |
| Vite | 7.3.1 resolved | MIT | Yes |
| Tailwind CSS | 4.2.1 resolved | MIT | Yes |
| TypeScript | 5.8.3 resolved | Apache-2.0 | Yes |
| i18next / react-i18next | 25.10.9 / 16.6.6 | permissive OSS in installed metadata | Likely yes |
| Tauri core/plugins | 2.10.x | Apache-2.0 OR MIT | Yes |

Local evidence:

- [package.json](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/package.json#L12)
- [Cargo.toml](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/Cargo.toml#L15)

Conclusion:

- Yes, this layer is open source and appropriate for commercial distribution.

### 2. Rust native/runtime crates

Direct Rust dependencies resolved in this repo are mostly MIT, Apache-2.0, or dual MIT/Apache-2.0:

- `tauri`, `tauri-build`, `tauri-plugin-*`: Apache-2.0 OR MIT
- `serde`, `serde_json`, `regex`, `reqwest`, `arboard`, `dirs-next`, `ndarray`, `ort`, `rodio`: MIT OR Apache-2.0
- `rusqlite`, `rubato`, `enigo`, `rdev`: permissive OSS
- `cpal`, `hound`: Apache-2.0
- `parakeet-rs`: MIT OR Apache-2.0

Conclusion:

- Yes, this layer is open source and commercially usable.

### 3. Offline speech stack

#### `parakeet-rs`

- License: MIT OR Apache-2.0
- Commercial view: yes

#### ONNX runtime layer through `ort`

- License: Apache-2.0 and MIT in the wrapper repo
- Commercial view: yes

#### Silero VAD

- The Silero VAD repo states MIT license and explicitly markets it as permissive with "no strings attached."
- Commercial view: yes

#### Parakeet TDT model

This is the part that needs care.

Your code downloads:

- `istupakov/parakeet-tdt-0.6b-v3-onnx` in [model.rs](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/src/infra/model.rs#L7)

That Hugging Face page currently states:

- License: `cc-by-4.0`
- Base model: `nvidia/parakeet-tdt-0.6b-v3`

The NVIDIA base model card currently states:

- License: `cc-by-4.0`
- "This model is ready for commercial/non-commercial use."
- Released under a permissive CC BY 4.0 license

Commercial view:

- Yes, based on the current NVIDIA model card, commercial use appears allowed.
- But CC BY 4.0 is attribution-based, not equivalent to MIT/Apache.
- Because your app uses a converted ONNX repack from a third party (`istupakov`), you should preserve both:
  - attribution to NVIDIA as the base model author
  - attribution to the ONNX conversion source if you keep using that converted package

Operational recommendation:

- Add a `THIRD_PARTY_NOTICES.md` or installer notice covering the Parakeet model attribution.
- Record the exact model URL, version/date, and license snapshot you approved.

### 4. Cloud providers and APIs

These are not open source resources in the usual sense:

| Provider | Used for | Open source? | Commercial use view |
|---|---|---|---|
| Groq | STT + LLM API | No, proprietary service | Usually yes under service agreement, not OSS |
| Deepgram | STT API | No, proprietary service | Likely yes under contract/terms, not OSS |
| OpenAI | LLM API | No, proprietary service | Yes for business use under service terms, not OSS |
| Anthropic | LLM API | No, proprietary service | Yes for customer products under commercial terms, not OSS |
| Ollama | Local model runtime | Yes, MIT | Runtime yes, model license varies by model |

Key repo evidence:

- Groq/Deepgram STT: [cloud.rs](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/src/features/speech/cloud.rs#L26)
- Ollama/OpenAI/Claude/Groq chat calls: [llm.rs](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/src/features/speech/llm.rs#L29)

Commercial view:

- You are not purely "open source only."
- You are using a mixed stack:
  - open-source app/runtime components
  - proprietary cloud AI services

That is totally normal for a commercial product, but the proprietary services must be governed by their service agreements, not by OSS assumptions.

### 5. Bundled sounds and static assets

This is the weakest part of the current compliance posture.

Bundled audio files include:

- `src-tauri/sounds/*.wav`

I did not find any local attribution, source note, license file, or provenance record for those audio assets in:

- `src/`
- `src-tauri/`
- `docs/`
- `research/`
- `.github/`

Commercial view:

- Unknown
- This should be treated as a commercial release risk until the origin and license of every bundled sound file is documented

Recommendation:

- Either replace all sound assets with self-created or clearly licensed alternatives
- Or add a provenance file listing source URL, author, license, and attribution obligations for each sound

## Health Research

## Health Check 1: Ecosystem maturity and maintenance

### Strong / low concern

- Tauri is very active and large
- React is extremely mature and current
- Vite is highly active
- Tailwind is highly active
- Ollama is highly active
- `ort` is active and recently released
- Silero VAD is active and recently released

### Medium concern

- `parakeet-rs` is active and recently released, but it has a much smaller maintainer surface than React/Tauri/Vite/Tailwind.
- That makes it a dependency worth watching more closely for regressions and bus-factor risk.

### Higher concern than licensing: documentation drift

- The repo still contains older architecture docs describing a Python `faster-whisper` sidecar, while current code uses `parakeet-rs`.
- This is not a licensing problem by itself, but it is a maintenance/operational health issue because it can mislead future contributors and reviewers.

## Health Check 2: Version lag against current upstream

Based on local resolved versions plus live registry checks run on 2026-03-26:

### Healthy / current

- React: app uses 19.2.4, current npm version is 19.2.4
- React DOM: app uses 19.2.4, current npm version is 19.2.4
- Tauri JS/API: app uses 2.10.1, current npm version is 2.10.1
- i18next: app uses 25.10.9, current npm version is 25.10.9
- `parakeet-rs`: app uses 0.3.4, current crate version is 0.3.4
- `ort`: app uses 2.0.0-rc.12, current crate version is 2.0.0-rc.12

### Slight lag

- Tailwind CSS: app uses 4.2.1, current npm version is 4.2.2

### Meaningful lag

- Vite: app uses 7.3.1, current npm version is 8.0.3

This does not automatically mean YOLO Voice needs an urgent Vite upgrade, but it does mean the web tooling side is behind current upstream by a full major version.

Overall health conclusion:

- The core stack is healthy.
- The repo is not showing obvious abandonment risk in its major dependencies.
- Your main maintenance risk is not "dead dependencies"; it is:
  - asset provenance
  - third-party model notice hygiene
  - documentation drift between spec and implementation

## Commercial-Use Verdict

## Short answer

Yes, you are using a lot of open-source resources that are compatible with commercial use.

But the full answer is:

- Yes for most code dependencies
- Yes, with attribution obligations, for the current Parakeet offline model
- Unknown until documented for bundled sound assets
- Not open source for Groq, Deepgram, OpenAI, and Anthropic APIs

## Practical release decision

I would be comfortable calling the stack commercially viable after these cleanup items:

1. Add a third-party notices file with:
   - Tauri / React / Vite / Tailwind / Rust crate notice bundle
   - Silero VAD attribution
   - NVIDIA Parakeet CC BY 4.0 attribution
   - ONNX conversion source note for `istupakov/parakeet-tdt-0.6b-v3-onnx`
2. Create a provenance file for every bundled `.wav` asset or replace them
3. Keep copies/links of current service agreements for Groq, Deepgram, OpenAI, and Anthropic in your release/legal checklist
4. Update outdated architecture docs so the repo reflects the real implementation

## Bottom Line

The code stack is in good shape for commercial use.

The commercial/compliance blockers are not your OSS frameworks. They are:

- attribution discipline for the Parakeet model
- undocumented bundled sound assets
- proprietary AI service terms being handled outside of a formal vendor checklist

If those three are cleaned up, the stack looks commercially workable.

## Sources

Repo files:

- [package.json](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/package.json)
- [Cargo.toml](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/Cargo.toml)
- [src-tauri/src/infra/model.rs](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/src/infra/model.rs)
- [src-tauri/src/features/speech/cloud.rs](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/src/features/speech/cloud.rs)
- [src-tauri/src/features/speech/llm.rs](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/src/features/speech/llm.rs)
- [src-tauri/src/features/speech/vad.rs](/C:/Users/Alish/OneDrive/Desktop/Work/Projects/yolo%20voice/src-tauri/src/features/speech/vad.rs)

Official/external sources used:

- Tauri GitHub: https://github.com/tauri-apps/tauri
- React GitHub: https://github.com/facebook/react
- Vite GitHub: https://github.com/vitejs/vite
- Tailwind CSS GitHub: https://github.com/tailwindlabs/tailwindcss
- `parakeet-rs` docs.rs: https://docs.rs/parakeet-rs/latest/parakeet_rs/
- `parakeet-rs` GitHub: https://github.com/altunenes/parakeet-rs
- `ort` GitHub: https://github.com/pykeio/ort
- Silero VAD GitHub: https://github.com/snakers4/silero-vad
- ONNX Parakeet conversion model: https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx
- NVIDIA base Parakeet model: https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3
- OpenAI GPT-OSS model: https://huggingface.co/openai/gpt-oss-120b
- OpenAI Services Agreement: https://openai.com/policies/services-agreement/
- Anthropic Commercial Terms: https://www.anthropic.com/legal/commercial-terms
- Groq terms page: https://groq.com/terms-of-use

Live registry checks used:

- npm registry for `react`, `react-dom`, `vite`, `tailwindcss`, `@tauri-apps/api`, `i18next`
- crates.io via `cargo info` for `tauri`, `parakeet-rs`, `ort`

Notes on source confidence:

- I found a public Deepgram terms PDF, but it appears old and may not represent the current API/commercial agreement. I did not rely on it for a positive commercial-use conclusion.
- I did not find enough public evidence in this pass to certify the bundled `.wav` assets for commercial distribution.
