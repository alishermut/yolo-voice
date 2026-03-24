from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path
from typing import Callable


@dataclass(frozen=True)
class Case:
    category: str
    spoken: str
    expected: str


CASES: list[Case] = [
    Case("hard fillers", "um I think this is good", "I think this is good"),
    Case("hard fillers", "uh hello there", "Hello there"),
    Case("hard fillers", "mm I don't know", "I don't know"),
    Case("hard fillers", "well uh can you open GitHub", "Well can you open GitHub"),
    Case("hard fillers", "er we should start again", "We should start again"),
    Case("hard fillers", "ah that's the one", "That's the one"),
    Case(
        "discourse markers",
        "I mean, we should probably deploy today",
        "We should probably deploy today",
    ),
    Case(
        "discourse markers",
        "you know, this is the right file",
        "This is the right file",
    ),
    Case("discourse markers", "you know this can fail", "You know this can fail"),
    Case("discourse markers", "I mean this is not ideal", "I mean this is not ideal"),
    Case("soft semantics", "it's basically done", "It's basically done"),
    Case("soft semantics", "I literally saw it happen", "I literally saw it happen"),
    Case("soft semantics", "it's kind of working", "It's kind of working"),
    Case("soft semantics", "this is sort of fragile", "This is sort of fragile"),
    Case("soft semantics", "actually I want to keep that", "Actually I want to keep that"),
    Case("stutters", "I I think we should go", "I think we should go"),
    Case("stutters", "the the problem is the API key", "The problem is the API key"),
    Case("stutters", "we we were going to ship today", "We were going to ship today"),
    Case("stutters", "to to be clear we need logs", "To be clear we need logs"),
    Case(
        "meaningful repetition",
        "this is very very important",
        "This is very very important",
    ),
    Case("meaningful repetition", "I had had enough", "I had had enough"),
    Case("meaningful repetition", "maybe maybe we should wait", "Maybe maybe we should wait"),
    Case(
        "meaningful repetition",
        "no no that is not what I said",
        "No no that is not what I said",
    ),
    Case("meaningful repetition", "it felt so so slow", "It felt so so slow"),
    Case("punctuation", "hello. how are you", "Hello. How are you"),
    Case("punctuation", "hello , world !", "Hello, world!"),
    Case("punctuation", "this is  a   test", "This is a test"),
    Case("punctuation", "what is this ? it looks wrong", "What is this? It looks wrong"),
    Case("mixed", "um actually I think it's fine", "Actually I think it's fine"),
    Case("mixed", "uh-huh yes that's correct", "Yes that's correct"),
    Case("mixed", "I mean, uh, we can probably merge this", "We can probably merge this"),
    Case(
        "mixed",
        "you know, I I think this is basically okay",
        "I think this is basically okay",
    ),
    Case(
        "mixed",
        "well, um, the the API endpoint is down",
        "Well, the API endpoint is down",
    ),
]


# Mirrors the current Rust implementation in src-tauri/src/features/speech/cleanup.rs.
FILLER_SINGLE_CURRENT = re.compile(
    r"(?i)\b(um+|uh+|er+|ah+|huh|uh[\s-]huh|mm+|hm+)\b[,]?\s*"
)
FILLER_PHRASE_CURRENT = re.compile(
    r"(?i)\b(you know|I mean|sort of|kind of|basically|actually|literally)\b[,]?\s*"
)

# Proposed refined variant based on the synthetic evaluation in this report.
FILLER_SINGLE_REFINED = re.compile(
    r"(?i)\b(uh[\s-]huh|um+|uh+|er+|ah+|huh|mm+|hm+)\b[,]?\s*"
)
SOFT_MARKERS_REFINED = re.compile(r"(?i)^(?:you know|i mean)\b,\s*")

MULTI_SPACE = re.compile(r" {2,}")
SPACE_BEFORE_PUNCT = re.compile(r" ([.,!?;:])")
DOUBLE_PUNCT = re.compile(r"([.,!?;:])\s*([.,!?;:])")
SENTENCE_CAP = re.compile(r"([.!?])\s+([a-z])")
RESTART_WORDS = {
    "i",
    "the",
    "a",
    "an",
    "we",
    "you",
    "he",
    "she",
    "it",
    "they",
    "to",
    "of",
    "and",
    "but",
}


def normalize_token(token: str) -> str:
    return re.sub(r"(^[^A-Za-z']+|[^A-Za-z']+$)", "", token).lower()


def finish(text: str) -> str:
    text = DOUBLE_PUNCT.sub(r"\2", text)
    text = SPACE_BEFORE_PUNCT.sub(r"\1", text)
    text = MULTI_SPACE.sub(" ", text).strip(" -")
    if text:
        text = text[0].upper() + text[1:]
        text = SENTENCE_CAP.sub(lambda m: f"{m.group(1)} {m.group(2).upper()}", text)
    return text


def remove_stutters_current(text: str) -> str:
    words = text.split()
    if not words:
        return ""

    result = [words[0]]
    for word in words[1:]:
        if result[-1].lower() != word.lower():
            result.append(word)
    return " ".join(result)


def remove_stutters_refined(text: str) -> str:
    words = text.split()
    if not words:
        return ""

    result = [words[0]]
    for word in words[1:]:
        prev_norm = normalize_token(result[-1])
        curr_norm = normalize_token(word)
        if prev_norm and prev_norm == curr_norm and curr_norm in RESTART_WORDS:
            continue
        result.append(word)
    return " ".join(result)


def clean_current(text: str) -> str:
    if not text.strip():
        return ""

    text = FILLER_SINGLE_CURRENT.sub("", text)
    text = FILLER_PHRASE_CURRENT.sub("", text)
    text = remove_stutters_current(text)
    return finish(text)


def clean_refined(text: str) -> str:
    if not text.strip():
        return ""

    text = FILLER_SINGLE_REFINED.sub("", text)
    text = SOFT_MARKERS_REFINED.sub("", text)
    text = remove_stutters_refined(text)
    return finish(text)


def evaluate(cleaner: Callable[[str], str]) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for index, case in enumerate(CASES, start=1):
        output = cleaner(case.spoken)
        rows.append(
            {
                "id": index,
                "category": case.category,
                "spoken": case.spoken,
                "expected": case.expected,
                "output": output,
                "passed": output == case.expected,
            }
        )
    return rows


def score(rows: list[dict[str, object]]) -> tuple[int, int]:
    passed = sum(1 for row in rows if row["passed"])
    return passed, len(rows)


def markdown_escape(text: str) -> str:
    return text.replace("|", "\\|")


def build_report() -> str:
    current_rows = evaluate(clean_current)
    refined_rows = evaluate(clean_refined)

    current_passed, total = score(current_rows)
    refined_passed, _ = score(refined_rows)

    lines: list[str] = []
    lines.append("# Cleanup Evaluation Report")
    lines.append("")
    lines.append("## Scope")
    lines.append("")
    lines.append(
        "This report evaluates the current deterministic cleanup logic against a refined "
        "rule set. The data is synthetic but designed to mirror realistic dictation input."
    )
    lines.append("")
    lines.append("The refined rule set is not app code yet. It is a proposed behavior model used for evaluation.")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- Total cases: `{total}`")
    lines.append(f"- Current cleanup pass rate: `{current_passed}/{total}`")
    lines.append(f"- Refined cleanup pass rate: `{refined_passed}/{total}`")
    lines.append("")
    lines.append("## What Changed In The Refined Variant")
    lines.append("")
    lines.append("- Keep only hard fillers as default deletions: `um`, `uh`, `er`, `ah`, `huh`, `mm`, `hm`, `uh-huh`.")
    lines.append("- Remove `I mean,` and `you know,` only when they appear as sentence-start discourse markers with a comma.")
    lines.append("- Preserve semantic hedges and intensifiers such as `actually`, `basically`, `literally`, `kind of`, and `sort of`.")
    lines.append("- Remove duplicate restart words like `I I`, `the the`, `we we`, and `to to`.")
    lines.append("- Preserve meaningful repetition like `very very`, `had had`, `maybe maybe`, and `no no`.")
    lines.append("- Fix the `uh-huh` ordering issue so it is matched before bare `uh`.")
    lines.append("")
    lines.append("## Category Breakdown")
    lines.append("")
    lines.append("| Category | Cases | Current Passed | Refined Passed |")
    lines.append("| --- | ---: | ---: | ---: |")

    categories = sorted({case.category for case in CASES})
    for category in categories:
        current_in_category = [row for row in current_rows if row["category"] == category]
        refined_in_category = [row for row in refined_rows if row["category"] == category]
        lines.append(
            f"| {category} | {len(current_in_category)} | "
            f"{sum(1 for row in current_in_category if row['passed'])} | "
            f"{sum(1 for row in refined_in_category if row['passed'])} |"
        )

    lines.append("")
    lines.append("## Per-Test Results")
    lines.append("")
    lines.append(
        "| # | Category | Spoken Input | Expected Cleanup | Current Output | Refined Output | Current | Refined |"
    )
    lines.append("| ---: | --- | --- | --- | --- | --- | --- | --- |")

    for current_row, refined_row in zip(current_rows, refined_rows, strict=True):
        current_status = "PASS" if current_row["passed"] else "FAIL"
        refined_status = "PASS" if refined_row["passed"] else "FAIL"
        lines.append(
            f"| {current_row['id']} | {current_row['category']} | "
            f"`{markdown_escape(str(current_row['spoken']))}` | "
            f"`{markdown_escape(str(current_row['expected']))}` | "
            f"`{markdown_escape(str(current_row['output']))}` | "
            f"`{markdown_escape(str(refined_row['output']))}` | "
            f"{current_status} | {refined_status} |"
        )

    lines.append("")
    lines.append("## Current Cleanup Failure Themes")
    lines.append("")
    lines.append("- Over-removes semantic hedges and intensifiers.")
    lines.append("- Removes all duplicate words, including meaningful repetition.")
    lines.append("- Removes `you know` and `I mean` too aggressively, even when not clearly filler.")
    lines.append("- Mishandles `uh-huh` because the regex matches bare `uh` first.")
    lines.append("")
    lines.append("## Suggested Test Set Size")
    lines.append("")
    lines.append("- `20-25` cases: enough to spot obvious regressions quickly.")
    lines.append("- `30-40` cases: enough for rule tuning like the work in this report.")
    lines.append("- `75-100+` cases: a better target before freezing cleanup behavior for release.")
    lines.append("- Best next step after this synthetic set: collect anonymized real dictation snippets and promote them into the suite.")
    lines.append("")
    lines.append("## How To Regenerate")
    lines.append("")
    lines.append("Run:")
    lines.append("")
    lines.append("```powershell")
    lines.append("python scripts/evaluate_cleanup.py")
    lines.append("```")
    lines.append("")
    lines.append("This rewrites `docs/CLEANUP_EVALUATION.md`.")
    lines.append("")

    return "\n".join(lines)


def main() -> None:
    repo_root = Path(__file__).resolve().parents[1]
    report_path = repo_root / "docs" / "CLEANUP_EVALUATION.md"
    report_path.write_text(build_report(), encoding="utf-8")
    print(f"Wrote {report_path}")


if __name__ == "__main__":
    main()
