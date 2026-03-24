# Transcription Refinement Plan

Date: 2026-03-24

## Goal

Improve transcription quality without sacrificing the current local CPU-friendly speed profile.

This plan is intentionally split into three phases:

1. Fix the dictionary / substitution architecture
2. Implement the deterministic cleanup and recombination improvements
3. Add a local data layer for future refinement

The plan is designed so a new session can pick up from this file without re-doing the research.

## Constraints

- Keep the current local STT model
- Keep CPU-first performance as the default target
- Do not add another cleanup model
- Do not make API post-processing a required part of the hot path
- Prefer deterministic logic and measurable evaluation over subjective tuning
- Use synthetic evaluation first, then accumulate real local data later

## Current Evidence

### Existing reports

- `docs/CLEANUP_EVALUATION.md`
- `docs/SEGMENTATION_JOIN_EVALUATION.md`
- `docs/TEXT_PIPELINE_EVALUATION.md`
- `docs/PIPELINE_REGRESSION_PACK.md`

### Existing evaluation scripts

- `scripts/evaluate_cleanup.py`
- `scripts/evaluate_segmentation_join.py`
- `scripts/evaluate_text_pipeline.py`
- `scripts/evaluate_pipeline_regressions.py`

### Main findings already established

#### Dictionary / substitution

- `GlobalDictionary.vocabulary` exists but is not used by the offline recognizer
- replacements are downstream regex substitutions only
- industry packs are merged into one global dictionary
- on first install, all industry packs are auto-applied if the dictionary is empty
- this makes the active industry pack misleading, because terms from other packs can remain active

Relevant files:

- `src-tauri/src/features/speech/vocabulary.rs`
- `src-tauri/src/app/commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/features/capture/mod.rs`
- `src-tauri/src/features/speech/inference.rs`
- `src-tauri/src/features/speech/llm.rs`
- `src-tauri/resources/industry_packs/software_engineering.json`

#### Cleanup / joining

- the largest quality issue is not raw cleanup alone, but segment recombination
- the current join logic is too eager to turn chunk boundaries into sentence boundaries
- the current cleanup rules over-remove some semantic words and meaningful repetition
- deterministic improvements look strong in isolation and good end-to-end, but still need final tuning

Summary of synthetic results at the time of writing:

- cleanup evaluation:
  - current cleaner underperforms on semantic hedges and meaningful repetition
  - refined deterministic cleanup performed much better on the synthetic set
- segmentation / join evaluation:
  - current join: `6/28`
  - space-only join: `2/28`
  - heuristic join: `28/28`
- end-to-end text pipeline:
  - current pipeline: `2/26`
  - proposed deterministic pipeline: `18/26`
- focused regression pack:
  - current pipeline: `0/22`
  - proposed pipeline: `17/22`

#### Data layer

- there is currently no structured local dataset of:
  - raw transcription
  - cleaned text
  - joined text
  - final inserted text
  - active config context
- without this, future tuning depends too heavily on synthetic examples

## Recommended Execution Order

1. Phase 1 first
2. Phase 2 second
3. Phase 3 third

Reason:

- Phase 1 fixes a structural correctness issue in the vocabulary system
- Phase 2 improves the deterministic text pipeline itself
- Phase 3 creates the long-term feedback loop after the system behavior is more stable

---

## Phase 1: Fix Dictionary / Substitution Architecture

### Objective

Make dictionaries predictable, scoped, and safe for technical dictation, especially coding terms.

### Problem statement

The current system mixes together three different concepts:

- vocabulary hints
- replacement rules
- optional LLM terminology hints

At the same time, industry packs are merged globally, so domain-specific behavior is not truly scoped.

This likely causes:

- cross-domain contamination
- misleading "active pack" behavior
- substitutions firing in contexts where they should not
- weak coding terminology handling in offline mode

### Key code facts

- `GlobalDictionary` stores `vocabulary` and `replacements` in `src-tauri/src/features/speech/vocabulary.rs`
- `apply_replacements()` is a downstream regex replace function in `src-tauri/src/features/speech/vocabulary.rs`
- `apply_industry_pack()` merges packs into the current dictionary in `src-tauri/src/app/commands.rs`
- `auto_apply_all_packs()` auto-merges all packs on first install in `src-tauri/src/features/speech/vocabulary.rs`
- setup calls that auto-apply behavior in `src-tauri/src/lib.rs`
- offline transcription path does not accept vocabulary hints in `src-tauri/src/features/speech/mod.rs` and `src-tauri/src/features/speech/inference.rs`
- replacements are applied before cleanup and again after post-processing in `src-tauri/src/features/capture/mod.rs`

### Design direction

Refactor the feature into explicit layers:

1. Personal global dictionary
2. Active industry pack
3. Recognition hints
4. Normalization rules

These should not all be merged into one undifferentiated list.

### Target model

#### Suggested concepts

- `global_user_terms`
  - user-specific canonical terms across domains
- `active_pack_id`
  - selected domain pack
- `pack_terms`
  - scoped to the active industry
- `recognition_hints`
  - terms we want the recognizer to prefer
- `normalization_rules`
  - spoken aliases or common misrecognitions mapped to canonical output

#### Suggested future structure for a term entry

- canonical form
- spoken aliases
- scope
- priority
- case policy
- enabled flag

Example conceptual shape:

```json
{
  "canonical": "TypeScript",
  "aliases": ["type script", "typed script"],
  "scope": "software_engineering",
  "priority": "high",
  "preserve_case": true,
  "enabled": true
}
```

### Implementation tasks

#### 1. Stop auto-applying every industry pack

- remove or disable `auto_apply_all_packs()` behavior
- preserve backward compatibility for existing users if possible
- decide how to migrate existing global dictionaries that already contain merged pack entries

#### 2. Make industry packs truly scoped

- applying a pack should set the active pack
- pack rules should be resolved at runtime from the active pack, not copied permanently into the global dictionary
- keep a separate user-global dictionary for persistent personal overrides

#### 3. Separate normalization from vocabulary metadata

- treat `replacements` as explicit normalization rules
- treat `vocabulary` as terminology metadata, not as interchangeable with replacements
- rename or document these concepts clearly in code and UI

#### 4. Introduce canonical alias behavior

- support canonical term plus aliases rather than only flat `find -> replace`
- prioritize longer aliases before shorter ones
- support pack-aware filtering so coding aliases do not apply outside coding mode

#### 5. Review replacement safety

- identify rules that are too generic
- classify rules into:
  - safe
  - risky
- optional future extension:
  - allow aggressive rules only in a higher-tolerance mode

#### 6. Update UI semantics

- the UI should reflect that:
  - a pack is scoped and active
  - personal dictionary is separate from pack dictionary
  - substitutions are normalization rules, not the same thing as vocabulary

### Testing plan

Create a dedicated synthetic evaluation pack for dictionary behavior.

Recommended size:

- first pass: `40-60` coding-oriented cases
- before release: `100+` mixed cases

Recommended categories:

- framework names
- infrastructure names
- acronyms
- CLI terms
- file extensions
- common ASR homophones
- canonical capitalization cases
- repo-specific terms
- cases that should not be substituted

Suggested artifacts:

- `docs/DICTIONARY_EVALUATION.md`
- `scripts/evaluate_dictionary_rules.py`

### Acceptance criteria

- active industry pack no longer leaks previous pack behavior by default
- first install no longer silently enables all packs
- user-global terms and pack terms are clearly separated
- coding substitutions improve on the synthetic dictionary evaluation set
- no noticeable hot-path performance regression from scoped term resolution

### Risks / open questions

- migration of already-merged user dictionaries
- how much alias complexity to support in v1
- whether to keep backward compatibility with the old JSON structure or migrate immediately

---

## Phase 2: Implement Deterministic Cleanup and Recombination Improvements

### Objective

Improve punctuation, sentence joining, and cleanup quality while keeping the current CPU speed profile.

### Problem statement

The current pipeline does three things that interact poorly:

1. segments audio on silence
2. transcribes chunks independently
3. turns many chunk boundaries into sentence boundaries

This causes:

- misplaced periods
- awkward capitalization
- over-segmentation
- broken semantic repetition
- over-aggressive filler removal in some cases

### Key findings from evaluation

- join logic is the largest issue
- heuristic joining is computationally cheap compared with STT
- the real latency risk is increasing endpointing delay, not join heuristics
- cleanup still needs to preserve semantic words and meaningful repetition better

### Main code areas

- `src-tauri/src/features/speech/cleanup.rs`
- `src-tauri/src/features/speech/accumulator.rs`
- `src-tauri/src/features/capture/mod.rs`
- `src-tauri/src/features/speech/vad.rs`
- `src-tauri/src/features/capture/recorder.rs`

### Chosen design direction

Keep the deterministic hot path and improve it in place.

Do not:

- add another model
- make LLM cleanup mandatory
- change the STT model

Do:

- make joining more context-aware
- make cleanup less destructive
- delay stronger sentence shaping until final assembly

### Implementation tasks

#### 1. Replace hard sentence insertion with heuristic joining

- change the current join behavior so chunk boundaries do not automatically become periods
- use one-segment lookahead style heuristics where possible
- prefer spaces when the previous chunk is incomplete or the next chunk clearly continues the thought

Heuristic signals to consider:

- continuation openers:
  - `and`, `but`, `so`, `because`, `if`, `then`, `when`, `that`, `which`, `who`
- short continuation fragments
- lowercase continuation starts
- syntactically incomplete prior chunk
- time adverbs or trailing complements like `today`, `tomorrow`, `later`

#### 2. Narrow filler removal

- keep hard filler deletion for:
  - `um`, `uh`, `er`, `ah`, `mm`, `hm`
- do not default-delete semantic softeners:
  - `actually`, `basically`, `literally`, `kind of`, `sort of`
- only remove discourse markers like `I mean,` and `you know,` in narrow filler contexts

#### 3. Narrow stutter removal

- remove restart-like duplication
- preserve meaningful repetition such as:
  - `very very`
  - `had had`
  - `so so`

#### 4. Shift stronger formatting toward final pass

- keep cheap lexical cleanup per chunk if needed
- reserve stronger capitalization and punctuation decisions for final assembly

#### 5. Revisit double cleanup

- check whether the VAD path is cleaning too early and then again after assembly
- reduce duplicate destructive cleanup where possible

#### 6. Keep endpointing stable for the first implementation

- do not increase the silence threshold initially
- implement join improvements first because they are effectively free from a CPU perspective
- only revisit VAD threshold after the new join behavior is tested

### Testing plan

Use and extend the existing synthetic reports.

Required re-runs:

- `docs/CLEANUP_EVALUATION.md`
- `docs/SEGMENTATION_JOIN_EVALUATION.md`
- `docs/TEXT_PIPELINE_EVALUATION.md`
- `docs/PIPELINE_REGRESSION_PACK.md`

Recommended target before calling the phase complete:

- `TEXT_PIPELINE_EVALUATION`: move beyond the current `18/26`
- `PIPELINE_REGRESSION_PACK`: resolve the remaining failure buckets

Most important remaining failure buckets to target:

- short phrase boundary handling
- meaningful repetition preservation
- incomplete-clause continuation

### Acceptance criteria

- no meaningful user-visible slowdown from join logic changes
- sentence boundary behavior improves materially on the synthetic reports
- meaningful repetition is preserved in targeted cases
- semantic hedges are no longer over-removed by default
- the current model and CPU-first assumptions remain unchanged

### Risks / open questions

- heuristics can become brittle if they grow without tests
- cleanup and join changes should be validated together, not in isolation
- real-user traces may reveal failure modes not covered by synthetic tests

---

## Phase 3: Implement Local Data Layer for Future Refinement

### Objective

Create a privacy-conscious local dataset of transcription pipeline behavior so future refinements can be based on real usage, not only synthetic cases.

### Problem statement

Right now there is no structured record of:

- what the recognizer produced
- what cleanup changed
- what joining changed
- what the final inserted text looked like

This blocks evidence-driven iteration on real user behavior.

### Recommended storage choice

Use SQLite.

Why:

- queryable
- easy retention management
- easy export later
- better than loose JSON files once event volume grows

### Privacy model

- local only
- explicit opt-in
- easy clear / delete option
- no raw audio by default
- no clipboard snapshots
- no window titles unless explicitly justified and approved later

### Suggested data capture points

For each utterance, capture:

- timestamp
- app version
- session id
- utterance id
- transcription mode
- active industry pack
- cleanup enabled
- post-processing enabled
- VAD silence threshold
- raw segment texts
- joined text before final cleanup
- final text after cleanup
- inserted text

Optional future fields:

- user rating
- user-corrected final text
- notes / tags

### Suggested schema

#### Table: `transcript_samples`

- `id`
- `created_at`
- `app_version`
- `session_id`
- `utterance_id`
- `transcription_mode`
- `active_industry_pack`
- `cleanup_enabled`
- `post_processing_enabled`
- `vad_silence_threshold_ms`
- `raw_segments_json`
- `joined_text`
- `cleaned_text`
- `final_text`
- `inserted_text`

#### Optional later table: `transcript_feedback`

- `id`
- `sample_id`
- `rating`
- `corrected_text`
- `notes`
- `created_at`

### Implementation tasks

#### 1. Add storage module

- create a small local persistence layer
- initialize the database safely on startup
- add migrations or schema bootstrap

#### 2. Instrument pipeline checkpoints

- raw segment capture
- post-join capture
- post-cleanup capture
- final insert capture

#### 3. Add retention controls

- keep only the last N samples or use age-based retention
- suggested starting point:
  - last `1000` samples

#### 4. Add user controls

- enable / disable diagnostics logging
- clear logged data
- optional export of samples

#### 5. Future-proof for feedback

- design the schema so corrected text can be attached later
- do not block phase completion on feedback UI if it delays the first version too much

### Testing plan

- unit test schema initialization
- verify samples are written only when diagnostics are enabled
- verify retention works
- verify logging does not block the hot path in a noticeable way
- manually inspect a few real samples after Phase 2 lands

### Acceptance criteria

- diagnostics are local-only and opt-in
- data can be cleared easily
- enough pipeline state is logged to reconstruct failures
- storage overhead is small and does not materially affect dictation responsiveness

### Risks / open questions

- privacy expectations need to be explicit in the UI
- schema should avoid collecting more than is needed
- avoid introducing logging that becomes the new bottleneck

---

## Suggested Immediate Next Session

Start with Phase 1.

Recommended first actions:

1. inspect and redesign pack scoping
2. define migration behavior for already-merged dictionaries
3. create `DICTIONARY_EVALUATION` synthetic tests
4. implement the scoped dictionary model
5. verify behavior before moving to cleanup changes

## Session Notes for the Next Agent

- Do not re-open the cleanup strategy from scratch; the research direction is already established
- Keep the current STT model and CPU-first assumptions
- Prefer synthetic evaluation plus measurable outcomes over subjective “sounds better” tuning
- Treat the dictionary issue as an architecture problem first, not just a missing word list
- Treat cleanup quality as a combined pipeline problem, not only a regex problem

## External References Already Used

- Deepgram Keyterm Prompting
- Deepgram Find and Replace
- Google Speech Adaptation
- AWS Custom Vocabularies
- Azure Phrase List / Custom Speech guidance
- OpenAI Speech-to-Text Prompting
- contextual ASR biasing and contextual spelling-correction papers

These sources support the same pattern:

- upstream biasing should be scoped and focused
- downstream normalization should be explicit and controlled
- broad global term injection increases error risk

