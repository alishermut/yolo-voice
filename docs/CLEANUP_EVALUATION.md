# Cleanup Evaluation Report

## Scope

This report evaluates the current deterministic cleanup logic against a refined rule set. The data is synthetic but designed to mirror realistic dictation input.

The refined rule set is not app code yet. It is a proposed behavior model used for evaluation.

## Summary

- Total cases: `33`
- Current cleanup pass rate: `19/33`
- Refined cleanup pass rate: `33/33`

## What Changed In The Refined Variant

- Keep only hard fillers as default deletions: `um`, `uh`, `er`, `ah`, `huh`, `mm`, `hm`, `uh-huh`.
- Remove `I mean,` and `you know,` only when they appear as sentence-start discourse markers with a comma.
- Preserve semantic hedges and intensifiers such as `actually`, `basically`, `literally`, `kind of`, and `sort of`.
- Remove duplicate restart words like `I I`, `the the`, `we we`, and `to to`.
- Preserve meaningful repetition like `very very`, `had had`, `maybe maybe`, and `no no`.
- Fix the `uh-huh` ordering issue so it is matched before bare `uh`.

## Category Breakdown

| Category | Cases | Current Passed | Refined Passed |
| --- | ---: | ---: | ---: |
| discourse markers | 4 | 2 | 4 |
| hard fillers | 6 | 6 | 6 |
| meaningful repetition | 5 | 0 | 5 |
| mixed | 5 | 3 | 5 |
| punctuation | 4 | 4 | 4 |
| soft semantics | 5 | 0 | 5 |
| stutters | 4 | 4 | 4 |

## Per-Test Results

| # | Category | Spoken Input | Expected Cleanup | Current Output | Refined Output | Current | Refined |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | hard fillers | `um I think this is good` | `I think this is good` | `I think this is good` | `I think this is good` | PASS | PASS |
| 2 | hard fillers | `uh hello there` | `Hello there` | `Hello there` | `Hello there` | PASS | PASS |
| 3 | hard fillers | `mm I don't know` | `I don't know` | `I don't know` | `I don't know` | PASS | PASS |
| 4 | hard fillers | `well uh can you open GitHub` | `Well can you open GitHub` | `Well can you open GitHub` | `Well can you open GitHub` | PASS | PASS |
| 5 | hard fillers | `er we should start again` | `We should start again` | `We should start again` | `We should start again` | PASS | PASS |
| 6 | hard fillers | `ah that's the one` | `That's the one` | `That's the one` | `That's the one` | PASS | PASS |
| 7 | discourse markers | `I mean, we should probably deploy today` | `We should probably deploy today` | `We should probably deploy today` | `We should probably deploy today` | PASS | PASS |
| 8 | discourse markers | `you know, this is the right file` | `This is the right file` | `This is the right file` | `This is the right file` | PASS | PASS |
| 9 | discourse markers | `you know this can fail` | `You know this can fail` | `This can fail` | `You know this can fail` | FAIL | PASS |
| 10 | discourse markers | `I mean this is not ideal` | `I mean this is not ideal` | `This is not ideal` | `I mean this is not ideal` | FAIL | PASS |
| 11 | soft semantics | `it's basically done` | `It's basically done` | `It's done` | `It's basically done` | FAIL | PASS |
| 12 | soft semantics | `I literally saw it happen` | `I literally saw it happen` | `I saw it happen` | `I literally saw it happen` | FAIL | PASS |
| 13 | soft semantics | `it's kind of working` | `It's kind of working` | `It's working` | `It's kind of working` | FAIL | PASS |
| 14 | soft semantics | `this is sort of fragile` | `This is sort of fragile` | `This is fragile` | `This is sort of fragile` | FAIL | PASS |
| 15 | soft semantics | `actually I want to keep that` | `Actually I want to keep that` | `I want to keep that` | `Actually I want to keep that` | FAIL | PASS |
| 16 | stutters | `I I think we should go` | `I think we should go` | `I think we should go` | `I think we should go` | PASS | PASS |
| 17 | stutters | `the the problem is the API key` | `The problem is the API key` | `The problem is the API key` | `The problem is the API key` | PASS | PASS |
| 18 | stutters | `we we were going to ship today` | `We were going to ship today` | `We were going to ship today` | `We were going to ship today` | PASS | PASS |
| 19 | stutters | `to to be clear we need logs` | `To be clear we need logs` | `To be clear we need logs` | `To be clear we need logs` | PASS | PASS |
| 20 | meaningful repetition | `this is very very important` | `This is very very important` | `This is very important` | `This is very very important` | FAIL | PASS |
| 21 | meaningful repetition | `I had had enough` | `I had had enough` | `I had enough` | `I had had enough` | FAIL | PASS |
| 22 | meaningful repetition | `maybe maybe we should wait` | `Maybe maybe we should wait` | `Maybe we should wait` | `Maybe maybe we should wait` | FAIL | PASS |
| 23 | meaningful repetition | `no no that is not what I said` | `No no that is not what I said` | `No that is not what I said` | `No no that is not what I said` | FAIL | PASS |
| 24 | meaningful repetition | `it felt so so slow` | `It felt so so slow` | `It felt so slow` | `It felt so so slow` | FAIL | PASS |
| 25 | punctuation | `hello. how are you` | `Hello. How are you` | `Hello. How are you` | `Hello. How are you` | PASS | PASS |
| 26 | punctuation | `hello , world !` | `Hello, world!` | `Hello, world!` | `Hello, world!` | PASS | PASS |
| 27 | punctuation | `this is  a   test` | `This is a test` | `This is a test` | `This is a test` | PASS | PASS |
| 28 | punctuation | `what is this ? it looks wrong` | `What is this? It looks wrong` | `What is this? It looks wrong` | `What is this? It looks wrong` | PASS | PASS |
| 29 | mixed | `um actually I think it's fine` | `Actually I think it's fine` | `I think it's fine` | `Actually I think it's fine` | FAIL | PASS |
| 30 | mixed | `uh-huh yes that's correct` | `Yes that's correct` | `Yes that's correct` | `Yes that's correct` | PASS | PASS |
| 31 | mixed | `I mean, uh, we can probably merge this` | `We can probably merge this` | `We can probably merge this` | `We can probably merge this` | PASS | PASS |
| 32 | mixed | `you know, I I think this is basically okay` | `I think this is basically okay` | `I think this is okay` | `I think this is basically okay` | FAIL | PASS |
| 33 | mixed | `well, um, the the API endpoint is down` | `Well, the API endpoint is down` | `Well, the API endpoint is down` | `Well, the API endpoint is down` | PASS | PASS |

## Current Cleanup Failure Themes

- Over-removes semantic hedges and intensifiers.
- Removes all duplicate words, including meaningful repetition.
- Removes `you know` and `I mean` too aggressively, even when not clearly filler.
- Mishandles `uh-huh` because the regex matches bare `uh` first.

## Suggested Test Set Size

- `20-25` cases: enough to spot obvious regressions quickly.
- `30-40` cases: enough for rule tuning like the work in this report.
- `75-100+` cases: a better target before freezing cleanup behavior for release.
- Best next step after this synthetic set: collect anonymized real dictation snippets and promote them into the suite.

## How To Regenerate

Run:

```powershell
python scripts/evaluate_cleanup.py
```

This rewrites `docs/CLEANUP_EVALUATION.md`.
