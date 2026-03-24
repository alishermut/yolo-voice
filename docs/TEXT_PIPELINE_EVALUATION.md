# End-To-End Text Pipeline Evaluation

## Scope

This report simulates the deterministic text pipeline for VAD-style chunked transcripts:

1. Per-segment cleanup
2. Segment recombination
3. Final cleanup pass

It compares the legacy pipeline to the Phase 2 deterministic pipeline. No LLM is involved.

## Summary

- Total cases: `26`
- Current pipeline pass rate: `2/26`
- Proposed pipeline pass rate: `26/26`

## Proposed Pipeline Behavior

- Use lightweight per-segment cleanup instead of full destructive cleanup on every chunk.
- Use heuristic segment joining instead of forcing periods at most segment boundaries.
- Run one stronger final cleanup pass after joining.

## Category Breakdown

| Category | Cases | Current Passed | Proposed Passed |
| --- | ---: | ---: | ---: |
| affirmation | 1 | 0 | 1 |
| comma continuation | 1 | 0 | 1 |
| continuation with article | 1 | 0 | 1 |
| continuation with infinitive | 1 | 0 | 1 |
| discourse lead-in | 2 | 0 | 2 |
| discourse marker + filler | 1 | 0 | 1 |
| discourse marker + hedge | 1 | 0 | 1 |
| explicit sentence boundary | 2 | 2 | 2 |
| filler + continuation | 2 | 0 | 2 |
| meaningful repetition | 2 | 0 | 2 |
| mixed filler + hedge | 1 | 0 | 1 |
| mixed realistic | 2 | 0 | 2 |
| multi-segment continuation | 2 | 0 | 2 |
| short discourse lead-in | 2 | 0 | 2 |
| soft semantics | 3 | 0 | 3 |
| stutter + continuation | 2 | 0 | 2 |

## Per-Test Results

| # | Category | Raw Segments | Expected | Current Pipeline | Proposed Pipeline | Current | Proposed |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | filler + continuation | `um I think we should`<br>`go tomorrow because the API is down` | `I think we should go tomorrow because the API is down` | `I think we should. Go tomorrow because the API is down` | `I think we should go tomorrow because the API is down` | FAIL | PASS |
| 2 | filler + continuation | `uh open the settings menu`<br>`and then click advanced` | `Open the settings menu and then click advanced` | `Open the settings menu. And then click advanced` | `Open the settings menu and then click advanced` | FAIL | PASS |
| 3 | discourse lead-in | `well`<br>`I guess that works` | `Well, I guess that works` | `Well. I guess that works` | `Well, I guess that works` | FAIL | PASS |
| 4 | discourse lead-in | `so`<br>`what do we change next` | `So, what do we change next` | `So. What do we change next` | `So, what do we change next` | FAIL | PASS |
| 5 | discourse marker + hedge | `you know, I I think`<br>`this is basically okay` | `I think this is basically okay` | `I think. This is okay` | `I think this is basically okay` | FAIL | PASS |
| 6 | discourse marker + filler | `I mean, uh, we can`<br>`probably merge this` | `We can probably merge this` | `We can. Probably merge this` | `We can probably merge this` | FAIL | PASS |
| 7 | stutter + continuation | `the the API endpoint`<br>`is down` | `The API endpoint is down` | `The API endpoint. Is down` | `The API endpoint is down` | FAIL | PASS |
| 8 | stutter + continuation | `we we were going to ship`<br>`today` | `We were going to ship today` | `We were going to ship. Today` | `We were going to ship today` | FAIL | PASS |
| 9 | soft semantics | `it's kind of`<br>`working` | `It's kind of working` | `It's. Working` | `It's kind of working` | FAIL | PASS |
| 10 | soft semantics | `I literally`<br>`saw it happen` | `I literally saw it happen` | `I. Saw it happen` | `I literally saw it happen` | FAIL | PASS |
| 11 | soft semantics | `this is sort of`<br>`fragile` | `This is sort of fragile` | `This is. Fragile` | `This is sort of fragile` | FAIL | PASS |
| 12 | meaningful repetition | `this is very very`<br>`important` | `This is very very important` | `This is very. Important` | `This is very very important` | FAIL | PASS |
| 13 | meaningful repetition | `I had had`<br>`enough` | `I had had enough` | `I had. Enough` | `I had had enough` | FAIL | PASS |
| 14 | comma continuation | `for the rollout,`<br>`we should notify support` | `For the rollout, we should notify support` | `For the rollout, We should notify support` | `For the rollout, we should notify support` | FAIL | PASS |
| 15 | explicit sentence boundary | `we shipped the fix`<br>`please verify production` | `We shipped the fix. Please verify production` | `We shipped the fix. Please verify production` | `We shipped the fix. Please verify production` | PASS | PASS |
| 16 | explicit sentence boundary | `what changed?`<br>`can you summarize the rollout` | `What changed? Can you summarize the rollout` | `What changed? Can you summarize the rollout` | `What changed? Can you summarize the rollout` | PASS | PASS |
| 17 | multi-segment continuation | `um the the thing`<br>`that worries me is`<br>`the rollback path` | `The thing that worries me is the rollback path` | `The thing. That worries me is. The rollback path` | `The thing that worries me is the rollback path` | FAIL | PASS |
| 18 | multi-segment continuation | `can you check`<br>`whether the migration ran`<br>`on staging` | `Can you check whether the migration ran on staging` | `Can you check. Whether the migration ran. On staging` | `Can you check whether the migration ran on staging` | FAIL | PASS |
| 19 | mixed realistic | `we should probably`<br>`wait until tomorrow`<br>`because support is offline` | `We should probably wait until tomorrow because support is offline` | `We should probably. Wait until tomorrow. Because support is offline` | `We should probably wait until tomorrow because support is offline` | FAIL | PASS |
| 20 | mixed realistic | `can you open`<br>`the billing page`<br>`and check the failed invoices` | `Can you open the billing page and check the failed invoices` | `Can you open. The billing page. And check the failed invoices` | `Can you open the billing page and check the failed invoices` | FAIL | PASS |
| 21 | short discourse lead-in | `actually`<br>`I want to keep that` | `Actually, I want to keep that` | `I want to keep that` | `Actually, I want to keep that` | FAIL | PASS |
| 22 | short discourse lead-in | `basically`<br>`we just need one more approval` | `Basically, we just need one more approval` | `We just need one more approval` | `Basically, we just need one more approval` | FAIL | PASS |
| 23 | continuation with infinitive | `I pushed the fix`<br>`to the release branch` | `I pushed the fix to the release branch` | `I pushed the fix. To the release branch` | `I pushed the fix to the release branch` | FAIL | PASS |
| 24 | continuation with article | `please open`<br>`the latest report` | `Please open the latest report` | `Please open. The latest report` | `Please open the latest report` | FAIL | PASS |
| 25 | mixed filler + hedge | `um actually I think`<br>`it's fine` | `Actually I think it's fine` | `I think. It's fine` | `Actually I think it's fine` | FAIL | PASS |
| 26 | affirmation | `uh-huh yes`<br>`that's correct` | `Yes that's correct` | `Yes. That's correct` | `Yes that's correct` | FAIL | PASS |

## Estimated CPU Cost

| Pipeline | Approx microseconds per case | Relative to Current |
| --- | ---: | ---: |
| Current deterministic pipeline | 14.758 | 1.00x |
| Proposed deterministic pipeline | 27.304 | 1.85x |

Even when the proposed pipeline is slower in relative terms, the absolute cost remains tiny compared with audio transcription.

## Interpretation

- The legacy pipeline mostly fails by over-removing words and over-inserting sentence boundaries.
- The Phase 2 pipeline improves lexical cleanup and sentence reconstruction together.
- This report is still synthetic, so the next upgrade after Phase 2 would be to add anonymized real chunk outputs from the app.

## How To Regenerate

Run:

```powershell
python scripts/evaluate_text_pipeline.py
```

This rewrites `docs/TEXT_PIPELINE_EVALUATION.md`.
