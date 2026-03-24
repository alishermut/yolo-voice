# Segmentation And Join Evaluation

## Scope

This report evaluates how chunked transcript segments are recombined into final text. It focuses on deterministic join behavior, not model inference.

The examples are synthetic but modeled after VAD-caused sentence splits that commonly happen in dictation when the speaker pauses mid-thought.

## Summary

- Total cases: `28`
- Current join pass rate: `6/28`
- Space-only join pass rate: `2/28`
- Heuristic join pass rate: `28/28`

## What The Current Code Did

- Legacy `smart_join` added a period between segments whenever the previous segment did not end in `.`, `!`, `?`, or `,`.
- It also capitalized the next segment unconditionally.
- This meant many VAD boundaries became sentence boundaries.

## What The Heuristic Variant Does

- Preserve real sentence boundaries when punctuation is already present.
- Join obvious continuations with a space instead of forcing a period.
- Lowercase function-word continuations like `And`, `If`, `Because`, `The`, and `Whether` when they were capitalized only because a new chunk started.
- Turn short lead-ins like `Well`, `So`, `Actually`, and `Basically` into comma continuations.
- Treat short affirmations like `Yes` and `No` as continuations instead of sentence starts when the next chunk clearly continues the same thought.

## Category Breakdown

| Category | Cases | Current Passed | Space-Only Passed | Heuristic Passed |
| --- | ---: | ---: | ---: | ---: |
| affirmation | 2 | 0 | 0 | 2 |
| comma continuation | 2 | 0 | 0 | 2 |
| explicit sentence boundary | 3 | 3 | 0 | 3 |
| mid-sentence continuation | 8 | 0 | 0 | 8 |
| mixed realistic | 6 | 1 | 0 | 6 |
| multi-segment sentence | 3 | 0 | 0 | 3 |
| punctuation already present | 2 | 2 | 2 | 2 |
| short discourse lead-in | 2 | 0 | 0 | 2 |

## Per-Test Results

| # | Category | Segments | Expected | Current | Space-Only | Heuristic | Current | Space | Heuristic |
| ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | mid-sentence continuation | `I think we should`<br>`Go tomorrow because the API is down` | `I think we should go tomorrow because the API is down` | `I think we should. Go tomorrow because the API is down` | `I think we should Go tomorrow because the API is down` | `I think we should go tomorrow because the API is down` | FAIL | FAIL | PASS |
| 2 | mid-sentence continuation | `Open the settings menu`<br>`And then click advanced` | `Open the settings menu and then click advanced` | `Open the settings menu. And then click advanced` | `Open the settings menu And then click advanced` | `Open the settings menu and then click advanced` | FAIL | FAIL | PASS |
| 3 | mid-sentence continuation | `This is probably fine`<br>`If we add a retry` | `This is probably fine if we add a retry` | `This is probably fine. If we add a retry` | `This is probably fine If we add a retry` | `This is probably fine if we add a retry` | FAIL | FAIL | PASS |
| 4 | mid-sentence continuation | `I left it disabled`<br>`Because staging was unstable` | `I left it disabled because staging was unstable` | `I left it disabled. Because staging was unstable` | `I left it disabled Because staging was unstable` | `I left it disabled because staging was unstable` | FAIL | FAIL | PASS |
| 5 | mid-sentence continuation | `The issue started yesterday`<br>`When the cron job retried` | `The issue started yesterday when the cron job retried` | `The issue started yesterday. When the cron job retried` | `The issue started yesterday When the cron job retried` | `The issue started yesterday when the cron job retried` | FAIL | FAIL | PASS |
| 6 | mid-sentence continuation | `This is the feature`<br>`That users asked for` | `This is the feature that users asked for` | `This is the feature. That users asked for` | `This is the feature That users asked for` | `This is the feature that users asked for` | FAIL | FAIL | PASS |
| 7 | mid-sentence continuation | `Please open`<br>`The latest report` | `Please open the latest report` | `Please open. The latest report` | `Please open The latest report` | `Please open the latest report` | FAIL | FAIL | PASS |
| 8 | mid-sentence continuation | `The thing that worries me is`<br>`The rollback path` | `The thing that worries me is the rollback path` | `The thing that worries me is. The rollback path` | `The thing that worries me is The rollback path` | `The thing that worries me is the rollback path` | FAIL | FAIL | PASS |
| 9 | multi-segment sentence | `We need to update`<br>`The documentation`<br>`Before the release` | `We need to update the documentation before the release` | `We need to update. The documentation. Before the release` | `We need to update The documentation Before the release` | `We need to update the documentation before the release` | FAIL | FAIL | PASS |
| 10 | multi-segment sentence | `Can you check`<br>`Whether the migration ran`<br>`On staging` | `Can you check whether the migration ran on staging` | `Can you check. Whether the migration ran. On staging` | `Can you check Whether the migration ran On staging` | `Can you check whether the migration ran on staging` | FAIL | FAIL | PASS |
| 11 | multi-segment sentence | `Open the dashboard`<br>`And check the alerts`<br>`Then ping me` | `Open the dashboard and check the alerts then ping me` | `Open the dashboard. And check the alerts. Then ping me` | `Open the dashboard And check the alerts Then ping me` | `Open the dashboard and check the alerts then ping me` | FAIL | FAIL | PASS |
| 12 | explicit sentence boundary | `We shipped the fix`<br>`Please verify production` | `We shipped the fix. Please verify production` | `We shipped the fix. Please verify production` | `We shipped the fix Please verify production` | `We shipped the fix. Please verify production` | PASS | FAIL | PASS |
| 13 | explicit sentence boundary | `The server restarted`<br>`It came back cleanly` | `The server restarted. It came back cleanly` | `The server restarted. It came back cleanly` | `The server restarted It came back cleanly` | `The server restarted. It came back cleanly` | PASS | FAIL | PASS |
| 14 | explicit sentence boundary | `I reviewed the logs`<br>`Nothing looked suspicious` | `I reviewed the logs. Nothing looked suspicious` | `I reviewed the logs. Nothing looked suspicious` | `I reviewed the logs Nothing looked suspicious` | `I reviewed the logs. Nothing looked suspicious` | PASS | FAIL | PASS |
| 15 | punctuation already present | `We shipped the fix.`<br>`Please verify production` | `We shipped the fix. Please verify production` | `We shipped the fix. Please verify production` | `We shipped the fix. Please verify production` | `We shipped the fix. Please verify production` | PASS | PASS | PASS |
| 16 | punctuation already present | `What changed?`<br>`Can you summarize the rollout` | `What changed? Can you summarize the rollout` | `What changed? Can you summarize the rollout` | `What changed? Can you summarize the rollout` | `What changed? Can you summarize the rollout` | PASS | PASS | PASS |
| 17 | comma continuation | `For the rollout,`<br>`We should notify support` | `For the rollout, we should notify support` | `For the rollout, We should notify support` | `For the rollout, We should notify support` | `For the rollout, we should notify support` | FAIL | FAIL | PASS |
| 18 | comma continuation | `If this fails,`<br>`We can revert quickly` | `If this fails, we can revert quickly` | `If this fails, We can revert quickly` | `If this fails, We can revert quickly` | `If this fails, we can revert quickly` | FAIL | FAIL | PASS |
| 19 | short discourse lead-in | `Well`<br>`I guess that works` | `Well, I guess that works` | `Well. I guess that works` | `Well I guess that works` | `Well, I guess that works` | FAIL | FAIL | PASS |
| 20 | short discourse lead-in | `So`<br>`What do we change next` | `So, what do we change next` | `So. What do we change next` | `So What do we change next` | `So, what do we change next` | FAIL | FAIL | PASS |
| 21 | affirmation | `Yes`<br>`That matches my logs` | `Yes that matches my logs` | `Yes. That matches my logs` | `Yes That matches my logs` | `Yes that matches my logs` | FAIL | FAIL | PASS |
| 22 | affirmation | `No`<br>`That is not the right file` | `No that is not the right file` | `No. That is not the right file` | `No That is not the right file` | `No that is not the right file` | FAIL | FAIL | PASS |
| 23 | mixed realistic | `We should probably`<br>`Wait until tomorrow`<br>`Because support is offline` | `We should probably wait until tomorrow because support is offline` | `We should probably. Wait until tomorrow. Because support is offline` | `We should probably Wait until tomorrow Because support is offline` | `We should probably wait until tomorrow because support is offline` | FAIL | FAIL | PASS |
| 24 | mixed realistic | `Can you open`<br>`The billing page`<br>`And check the failed invoices` | `Can you open the billing page and check the failed invoices` | `Can you open. The billing page. And check the failed invoices` | `Can you open The billing page And check the failed invoices` | `Can you open the billing page and check the failed invoices` | FAIL | FAIL | PASS |
| 25 | mixed realistic | `The update is live`<br>`Can you smoke test the login flow` | `The update is live. Can you smoke test the login flow` | `The update is live. Can you smoke test the login flow` | `The update is live Can you smoke test the login flow` | `The update is live. Can you smoke test the login flow` | PASS | FAIL | PASS |
| 26 | mixed realistic | `Actually`<br>`I think the first version was better` | `Actually, I think the first version was better` | `Actually. I think the first version was better` | `Actually I think the first version was better` | `Actually, I think the first version was better` | FAIL | FAIL | PASS |
| 27 | mixed realistic | `Basically`<br>`We just need one more approval` | `Basically, we just need one more approval` | `Basically. We just need one more approval` | `Basically We just need one more approval` | `Basically, we just need one more approval` | FAIL | FAIL | PASS |
| 28 | mixed realistic | `I pushed the fix`<br>`To the release branch` | `I pushed the fix to the release branch` | `I pushed the fix. To the release branch` | `I pushed the fix To the release branch` | `I pushed the fix to the release branch` | FAIL | FAIL | PASS |

## CPU Cost Of Join Strategies

| Strategy | Approx microseconds per join call | Relative to Current |
| --- | ---: | ---: |
| Current | 1.783 | 1.00x |
| Space-only | 0.516 | 0.29x |
| Heuristic | 4.096 | 2.30x |

These costs are far below transcription cost. In practice, join logic itself is unlikely to be user-visible.

## Endpointing Latency Context

User-visible latency is dominated by VAD endpointing and the stop-time drain wait, not by string joining.

| Configured silence threshold | Effective silence before endpoint | Plus stop drain wait | Approx total before finalization |
| ---: | ---: | ---: | ---: |
| 500 ms | 480 ms | 150 ms | 630 ms |
| 700 ms | 672 ms | 150 ms | 822 ms |
| 900 ms | 896 ms | 150 ms | 1046 ms |

## Recommended Interpretation

- Changing only the join logic should not create a noticeable slowdown.
- Increasing the VAD silence threshold can improve sentence boundaries, but it adds real waiting time.
- Best first experiment: keep the current endpointing threshold and replace the hard period-insertion join logic.

## How To Regenerate

Run:

```powershell
python scripts/evaluate_segmentation_join.py
```

This rewrites `docs/SEGMENTATION_JOIN_EVALUATION.md`.
