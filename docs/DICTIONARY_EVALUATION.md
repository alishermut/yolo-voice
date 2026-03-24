# Dictionary Evaluation

## Scope

This report evaluates dictionary behavior for Phase One of the transcription refinement plan.

The current model simulates today's merged-pack behavior where all pack rules can remain active at once.
The proposed model simulates scoped runtime resolution: user rules plus the currently active pack only.

## Summary

- Total cases: `42`
- Current merged-pack behavior: `28/42`
- Proposed scoped-pack behavior: `42/42`

## Coverage

- Framework names
- Infra and CLI terminology
- Acronyms and capitalization
- Repo-specific user rules
- Cross-domain leakage
- Wrong-pack negative cases

## Category Breakdown

| Category | Cases | Current Passed | Proposed Passed |
| --- | ---: | ---: | ---: |
| acronyms + data | 6 | 6 | 6 |
| cross-domain leakage | 8 | 0 | 8 |
| framework names | 6 | 6 | 6 |
| infra + cli | 6 | 6 | 6 |
| repo-specific | 4 | 4 | 4 |
| scoped packs | 6 | 6 | 6 |
| wrong-pack negatives | 6 | 0 | 6 |

## Per-Test Results

| # | Category | Active Pack | Spoken | Expected | Current Output | Proposed Output | Current | Proposed |
| ---: | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | framework names | software_engineering | `type script` | `TypeScript` | `TypeScript` | `TypeScript` | PASS | PASS |
| 2 | framework names | software_engineering | `next yes` | `Next.js` | `Next.js` | `Next.js` | PASS | PASS |
| 3 | framework names | software_engineering | `lang chain` | `LangChain` | `LangChain` | `LangChain` | PASS | PASS |
| 4 | framework names | software_engineering | `hugging face` | `Hugging Face` | `Hugging Face` | `Hugging Face` | PASS | PASS |
| 5 | framework names | software_engineering | `open AI` | `OpenAI` | `OpenAI` | `OpenAI` | PASS | PASS |
| 6 | framework names | software_engineering | `tory app` | `Tauri app` | `Tauri app` | `Tauri app` | PASS | PASS |
| 7 | infra + cli | software_engineering | `super base project` | `Supabase project` | `Supabase project` | `Supabase project` | PASS | PASS |
| 8 | infra + cli | software_engineering | `verse cell deploy` | `Vercel deploy` | `Vercel deploy` | `Vercel deploy` | PASS | PASS |
| 9 | infra + cli | software_engineering | `cube control get pods` | `kubectl get pods` | `kubectl get pods` | `kubectl get pods` | PASS | PASS |
| 10 | infra + cli | software_engineering | `cloud flare tunnel` | `Cloudflare tunnel` | `Cloudflare tunnel` | `Cloudflare tunnel` | PASS | PASS |
| 11 | infra + cli | software_engineering | `engine X config` | `Nginx config` | `Nginx config` | `Nginx config` | PASS | PASS |
| 12 | infra + cli | software_engineering | `get hub actions` | `GitHub actions` | `GitHub actions` | `GitHub actions` | PASS | PASS |
| 13 | acronyms + data | software_engineering | `graph QL schema` | `GraphQL schema` | `GraphQL schema` | `GraphQL schema` | PASS | PASS |
| 14 | acronyms + data | software_engineering | `rest API client` | `REST API client` | `REST API client` | `REST API client` | PASS | PASS |
| 15 | acronyms + data | software_engineering | `VS code settings` | `VS Code settings` | `VS Code settings` | `VS Code settings` | PASS | PASS |
| 16 | acronyms + data | software_engineering | `pie torch model` | `PyTorch model` | `PyTorch model` | `PyTorch model` | PASS | PASS |
| 17 | acronyms + data | software_engineering | `deep gram websocket` | `Deepgram websocket` | `Deepgram websocket` | `Deepgram websocket` | PASS | PASS |
| 18 | acronyms + data | software_engineering | `post gress migration` | `PostgreSQL migration` | `PostgreSQL migration` | `PostgreSQL migration` | PASS | PASS |
| 19 | repo-specific | general | `yolo voice release` | `YOLO Voice release` | `YOLO Voice release` | `YOLO Voice release` | PASS | PASS |
| 20 | repo-specific | software_engineering | `parakeet t d t model` | `Parakeet TDT model` | `Parakeet TDT model` | `Parakeet TDT model` | PASS | PASS |
| 21 | repo-specific | general | `alish term checklist` | `AlishTerm checklist` | `AlishTerm checklist` | `AlishTerm checklist` | PASS | PASS |
| 22 | repo-specific | general | `qyzylorda build` | `Qyzylorda build` | `Qyzylorda build` | `Qyzylorda build` | PASS | PASS |
| 23 | cross-domain leakage | general | `the gap is widening` | `the gap is widening` | `the GAAP is widening` | `the gap is widening` | FAIL | PASS |
| 24 | cross-domain leakage | general | `I see you tomorrow` | `I see you tomorrow` | `ICU tomorrow` | `I see you tomorrow` | FAIL | PASS |
| 25 | cross-domain leakage | general | `force major issue` | `force major issue` | `force majeure issue` | `force major issue` | FAIL | PASS |
| 26 | cross-domain leakage | general | `series a of photos` | `series a of photos` | `Series A of photos` | `series a of photos` | FAIL | PASS |
| 27 | cross-domain leakage | general | `defy expectations` | `defy expectations` | `DeFi expectations` | `defy expectations` | FAIL | PASS |
| 28 | cross-domain leakage | general | `verse cell choir` | `verse cell choir` | `Vercel choir` | `verse cell choir` | FAIL | PASS |
| 29 | cross-domain leakage | general | `engine X noise` | `engine X noise` | `Nginx noise` | `engine X noise` | FAIL | PASS |
| 30 | cross-domain leakage | general | `type script notes` | `type script notes` | `TypeScript notes` | `type script notes` | FAIL | PASS |
| 31 | scoped packs | medical | `a fib episode` | `AFib episode` | `AFib episode` | `AFib episode` | PASS | PASS |
| 32 | scoped packs | medical | `transfer to I see you` | `transfer to ICU` | `transfer to ICU` | `transfer to ICU` | PASS | PASS |
| 33 | scoped packs | legal | `force major clause` | `force majeure clause` | `force majeure clause` | `force majeure clause` | PASS | PASS |
| 34 | scoped packs | legal | `sub peena response` | `subpoena response` | `subpoena response` | `subpoena response` | PASS | PASS |
| 35 | scoped packs | finance | `gap revenue policy` | `GAAP revenue policy` | `GAAP revenue policy` | `GAAP revenue policy` | PASS | PASS |
| 36 | scoped packs | finance | `S and P 500 tracker` | `S&P 500 tracker` | `S&P 500 tracker` | `S&P 500 tracker` | PASS | PASS |
| 37 | wrong-pack negatives | software_engineering | `gap analysis` | `gap analysis` | `GAAP analysis` | `gap analysis` | FAIL | PASS |
| 38 | wrong-pack negatives | software_engineering | `I see you later` | `I see you later` | `ICU later` | `I see you later` | FAIL | PASS |
| 39 | wrong-pack negatives | software_engineering | `force major risk` | `force major risk` | `force majeure risk` | `force major risk` | FAIL | PASS |
| 40 | wrong-pack negatives | legal | `type script notes` | `type script notes` | `TypeScript notes` | `type script notes` | FAIL | PASS |
| 41 | wrong-pack negatives | finance | `cube control command` | `cube control command` | `kubectl command` | `cube control command` | FAIL | PASS |
| 42 | wrong-pack negatives | medical | `graph QL endpoint` | `graph QL endpoint` | `GraphQL endpoint` | `graph QL endpoint` | FAIL | PASS |

## Interpretation

- The main improvement is not more replacements; it is stopping wrong-pack rules from leaking into the hot path.
- Software terminology still works when the software pack is active.
- User-owned normalization rules still apply across packs.
- Negative cases improve because ambiguous rules like `gap`, `I see you`, and `force major` stay scoped.

## How To Regenerate

```powershell
python scripts/evaluate_dictionary_rules.py
```

This rewrites `docs/DICTIONARY_EVALUATION.md`.
