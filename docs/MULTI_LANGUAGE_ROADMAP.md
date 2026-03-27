# Multi-Language Text Processing Roadmap

## Current State (English Only)

The following text processing features in `src-tauri/src/features/speech/cleanup.rs` use hardcoded English word lists:

| Feature | English tokens | Location |
|---------|---------------|----------|
| Filler removal | `uh, um, er, ah, hm` + variants | `HARD_FILLER` regex |
| Discourse markers | `"you know"`, `"I mean"` | `LEADING_DISCOURSE_MARKER` regex |
| Restart/continuation words | ~50 words (`the, and, but, ...`) | `RESTART_WORDS`, `CONTINUATION_FIRST_WORDS`, `TRAILING_INCOMPLETE_WORDS` |
| Number-to-digit conversion | ~30 words (`one` through `trillion`) | `number_word_info()` match table |
| Spoken punctuation | ~20 patterns (`period, comma, ...`) | `SPOKEN_PUNCTUATION_RULES` |
| Hallucination phrases | ~14 phrases (`"thank you for watching"`, ...) | `HALLUCINATION_PHRASES` |
| Sentence capitalization | Latin-script aware | `SENTENCE_CAP` regex |

Speech recognition itself supports 25 languages (Parakeet TDT) — the limitation is only in post-transcription text processing.

## Proposed Architecture

### Locale JSON files

Extract all hardcoded word lists into per-language JSON files:

```
src-tauri/resources/text-processing/
  en.json
  fr.json
  de.json
  es.json
  ...
```

Each file contains:

```json
{
  "fillers": ["euh", "heu", "bah", "ben"],
  "discourse_markers": ["tu sais", "je veux dire"],
  "restart_words": ["le", "la", "les", "et", "mais"],
  "continuation_words": ["et", "mais", "donc", "parce que"],
  "trailing_incomplete_words": ["le", "la", "les", "un", "une"],
  "spoken_punctuation": {
    "point": ".",
    "virgule": ",",
    "point d'interrogation": "?",
    "point d'exclamation": "!",
    "nouvelle ligne": "\n",
    "nouveau paragraphe": "\n\n"
  },
  "hallucination_phrases": [
    "merci d'avoir regardé",
    "abonnez-vous"
  ],
  "number_words": null
}
```

### Loader

A new `TextProcessingLocale` struct in cleanup.rs:
- Loaded once at app startup based on `config.language`
- Reloaded when the user changes language
- Falls back to English if the requested locale file is missing

### Number conversion

Number word conversion is the hardest feature to internationalize because grammar varies dramatically:

| Language | Complexity | Example |
|----------|-----------|---------|
| Spanish, Portuguese, Italian | Medium | Similar structure to English, gendered (`uno/una`) |
| French | Hard | Vigesimal (80 = `quatre-vingts` = 4x20) |
| German | Hard | Reversed compounds (`dreiundzwanzig` = three-and-twenty, single word) |
| Slavic (Russian, Polish, Czech, etc.) | Very hard | Case/gender inflection on number words |
| Hungarian | Very hard | Agglutinative (`huszonhárom` = twenty-on-three) |
| Danish | Very hard | Complex base-20 system (`halvtreds` = 2.5x20 for 50) |

**Recommendation:** Ship number conversion for "easy" languages first (Spanish, Portuguese, Italian), skip complex ones until community contributions arrive.

## Per-Feature Effort Estimates

| Feature | Effort per language | Notes |
|---------|-------------------|-------|
| Fillers | ~10 min | 5-10 words, well-documented |
| Discourse markers | ~10 min | 2-5 phrases |
| Restart/continuation words | ~30 min | 30-50 function words |
| Spoken punctuation | ~15 min | 10-15 mappings |
| Hallucination phrases | ~15 min | 5-10 model-specific phrases |
| Number conversion | 2-8 hours | Full grammar parser per language |
| Sentence capitalization | ~5 min | Works for Latin scripts; CJK/Arabic need different logic |

**Total per language (excluding numbers):** ~1-2 hours

## Priority Languages

Based on Parakeet TDT support and user base:

1. **Spanish** (es) — large user base, similar grammar
2. **French** (fr) — large user base, complex numbers
3. **German** (de) — large user base, compound words
4. **Portuguese** (pt) — similar to Spanish
5. **Italian** (it) — similar to Spanish
6. **Russian** (ru) — Slavic, case inflection
7. **Polish** (pl) — Slavic, case inflection
8. **Dutch** (nl) — Germanic, similar to German
9. **Ukrainian** (uk) — Slavic
10. Remaining Parakeet languages

## Implementation Steps

1. Create the `TextProcessingLocale` struct and loader
2. Refactor `cleanup.rs` to accept a locale parameter instead of using static English lists
3. Create `en.json` by extracting current hardcoded values
4. Create locale files for priority languages (community contributions welcome)
5. Add a settings UI indicator showing which text processing features are available for the selected language
6. Ship number conversion incrementally, starting with Romance languages

## Contributing a Language

To add text processing for a new language:

1. Copy `src-tauri/resources/text-processing/en.json` as a template
2. Translate each word list to the target language
3. Set `"number_words": null` if number conversion is not implemented
4. Submit a PR with the new file

No Rust code changes needed — locale files are loaded dynamically.
