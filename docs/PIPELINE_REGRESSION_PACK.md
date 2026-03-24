# Pipeline Regression Pack

## Scope

This report expands the remaining end-to-end pipeline failures into a focused regression pack.

- Total hard cases: `22`
- Current pipeline pass rate: `0/22`
- Proposed pipeline pass rate: `22/22`

## Family Breakdown

| Failure Family | Cases | Current Passed | Proposed Passed |
| --- | ---: | ---: | ---: |
| affirmation | 4 | 0 | 4 |
| incomplete clause | 6 | 0 | 6 |
| meaningful repetition | 4 | 0 | 4 |
| short phrase boundary | 4 | 0 | 4 |
| soft semantic carryover | 4 | 0 | 4 |

## Per-Test Results

| # | Family | Raw Segments | Expected | Current Pipeline | Proposed Pipeline | Current | Proposed |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | incomplete clause | `the the API endpoint`<br>`is down` | `The API endpoint is down` | `The API endpoint. Is down` | `The API endpoint is down` | FAIL | PASS |
| 2 | incomplete clause | `we we were going to ship`<br>`today` | `We were going to ship today` | `We were going to ship. Today` | `We were going to ship today` | FAIL | PASS |
| 3 | incomplete clause | `the config value`<br>`is missing` | `The config value is missing` | `The config value. Is missing` | `The config value is missing` | FAIL | PASS |
| 4 | incomplete clause | `our main concern`<br>`is latency` | `Our main concern is latency` | `Our main concern. Is latency` | `Our main concern is latency` | FAIL | PASS |
| 5 | incomplete clause | `the next step`<br>`is rollout` | `The next step is rollout` | `The next step. Is rollout` | `The next step is rollout` | FAIL | PASS |
| 6 | incomplete clause | `what worries me`<br>`is the rollback path` | `What worries me is the rollback path` | `What worries me. Is the rollback path` | `What worries me is the rollback path` | FAIL | PASS |
| 7 | soft semantic carryover | `I literally`<br>`saw it happen` | `I literally saw it happen` | `I. Saw it happen` | `I literally saw it happen` | FAIL | PASS |
| 8 | soft semantic carryover | `it is kind of`<br>`working` | `It is kind of working` | `It is. Working` | `It is kind of working` | FAIL | PASS |
| 9 | soft semantic carryover | `this is sort of`<br>`fragile` | `This is sort of fragile` | `This is. Fragile` | `This is sort of fragile` | FAIL | PASS |
| 10 | soft semantic carryover | `it is basically`<br>`done` | `It is basically done` | `It is. Done` | `It is basically done` | FAIL | PASS |
| 11 | meaningful repetition | `this is very very`<br>`important` | `This is very very important` | `This is very. Important` | `This is very very important` | FAIL | PASS |
| 12 | meaningful repetition | `I had had`<br>`enough` | `I had had enough` | `I had. Enough` | `I had had enough` | FAIL | PASS |
| 13 | meaningful repetition | `that was really really`<br>`helpful` | `That was really really helpful` | `That was really. Helpful` | `That was really really helpful` | FAIL | PASS |
| 14 | meaningful repetition | `it felt so so`<br>`slow` | `It felt so so slow` | `It felt so. Slow` | `It felt so so slow` | FAIL | PASS |
| 15 | short phrase boundary | `um actually I think`<br>`it's fine` | `Actually I think it's fine` | `I think. It's fine` | `Actually I think it's fine` | FAIL | PASS |
| 16 | short phrase boundary | `actually I think`<br>`it still works` | `Actually I think it still works` | `I think. It still works` | `Actually I think it still works` | FAIL | PASS |
| 17 | short phrase boundary | `basically we just`<br>`need one more approval` | `Basically we just need one more approval` | `We just. Need one more approval` | `Basically we just need one more approval` | FAIL | PASS |
| 18 | short phrase boundary | `well I guess`<br>`that works` | `Well I guess that works` | `Well I guess. That works` | `Well I guess that works` | FAIL | PASS |
| 19 | affirmation | `uh-huh yes`<br>`that's correct` | `Yes that's correct` | `Yes. That's correct` | `Yes that's correct` | FAIL | PASS |
| 20 | affirmation | `uh yes`<br>`that is right` | `Yes that is right` | `Yes. That is right` | `Yes that is right` | FAIL | PASS |
| 21 | affirmation | `mm yes`<br>`I saw that too` | `Yes I saw that too` | `Yes. I saw that too` | `Yes I saw that too` | FAIL | PASS |
| 22 | affirmation | `uh-huh no`<br>`that is not the issue` | `No that is not the issue` | `No. That is not the issue` | `No that is not the issue` | FAIL | PASS |

## Remaining Risk Areas

- Real-user chunk boundaries may still reveal edge cases not covered by the synthetic pack.
- Minimal-join behavior with cleanup disabled is intentionally not scored here; this pack focuses on cleanup-enabled dictation.
- If future tuning expands heuristics further, this report should grow before rules get more permissive.

## How To Regenerate

```powershell
python scripts/evaluate_pipeline_regressions.py
```

This rewrites `docs/PIPELINE_REGRESSION_PACK.md`.
