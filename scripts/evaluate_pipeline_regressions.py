from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path
from typing import Callable


@dataclass(frozen=True)
class Case:
    family: str
    raw_segments: tuple[str, ...]
    expected: str


CASES: list[Case] = [
    Case("incomplete clause", ("the the API endpoint", "is down"), "The API endpoint is down"),
    Case("incomplete clause", ("we we were going to ship", "today"), "We were going to ship today"),
    Case("incomplete clause", ("the config value", "is missing"), "The config value is missing"),
    Case("incomplete clause", ("our main concern", "is latency"), "Our main concern is latency"),
    Case("incomplete clause", ("the next step", "is rollout"), "The next step is rollout"),
    Case("incomplete clause", ("what worries me", "is the rollback path"), "What worries me is the rollback path"),
    Case("soft semantic carryover", ("I literally", "saw it happen"), "I literally saw it happen"),
    Case("soft semantic carryover", ("it is kind of", "working"), "It is kind of working"),
    Case("soft semantic carryover", ("this is sort of", "fragile"), "This is sort of fragile"),
    Case("soft semantic carryover", ("it is basically", "done"), "It is basically done"),
    Case("meaningful repetition", ("this is very very", "important"), "This is very very important"),
    Case("meaningful repetition", ("I had had", "enough"), "I had had enough"),
    Case("meaningful repetition", ("that was really really", "helpful"), "That was really really helpful"),
    Case("meaningful repetition", ("it felt so so", "slow"), "It felt so so slow"),
    Case("short phrase boundary", ("um actually I think", "it's fine"), "Actually I think it's fine"),
    Case("short phrase boundary", ("actually I think", "it still works"), "Actually I think it still works"),
    Case("short phrase boundary", ("basically we just", "need one more approval"), "Basically we just need one more approval"),
    Case("short phrase boundary", ("well I guess", "that works"), "Well I guess that works"),
    Case("affirmation", ("uh-huh yes", "that's correct"), "Yes that's correct"),
    Case("affirmation", ("uh yes", "that is right"), "Yes that is right"),
    Case("affirmation", ("mm yes", "I saw that too"), "Yes I saw that too"),
    Case("affirmation", ("uh-huh no", "that is not the issue"), "No that is not the issue"),
]


FILLER_SINGLE_CURRENT = re.compile(r"(?i)\b(um+|uh+|er+|ah+|huh|uh[\s-]huh|mm+|hm+)\b[,]?\s*")
FILLER_PHRASE_CURRENT = re.compile(r"(?i)\b(you know|I mean|sort of|kind of|basically|actually|literally)\b[,]?\s*")
FILLER_SINGLE_REFINED = re.compile(r"(?i)\b(uh[\s-]huh|um+|uh+|er+|ah+|huh|mm+|hm+)\b[,]?\s*")
SOFT_MARKERS_REFINED = re.compile(r"(?i)^(?:you know|i mean)\b,\s*")

MULTI_SPACE = re.compile(r" {2,}")
SPACE_BEFORE_PUNCT = re.compile(r" ([.,!?;:])")
DOUBLE_PUNCT = re.compile(r"([.,!?;:])\s*([.,!?;:])")
SENTENCE_CAP = re.compile(r"([.!?])\s+([a-z])")

RESTART_WORDS = {"i", "the", "a", "an", "we", "you", "he", "she", "it", "they", "to", "of", "and", "but"}
CONTINUATION_FIRST_WORDS = {
    "and", "but", "so", "because", "if", "then", "or", "that", "which", "who", "whom", "when",
    "while", "unless", "though", "although", "after", "before", "until", "as", "whether", "to",
    "for", "with", "from", "of", "in", "on", "at", "by", "the", "a", "an", "is", "are", "was",
    "were", "today", "tomorrow", "yesterday", "later", "now", "soon",
}
TRAILING_INCOMPLETE_WORDS = {
    "and", "or", "but", "so", "because", "if", "then", "to", "for", "with", "from", "of", "in",
    "on", "at", "by", "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
    "should", "could", "would", "can", "will", "just", "probably", "really", "very", "kind",
    "sort", "literally", "basically", "actually", "think", "guess", "had",
}
COMMA_LEAD_INS = {"well", "so", "actually", "basically"}
AFFIRMATIONS = {"yes", "no"}


def normalize_token(token: str) -> str:
    return re.sub(r"(^[^A-Za-z']+|[^A-Za-z']+$)", "", token).lower()


def normalize_spacing_and_punctuation(text: str) -> str:
    text = DOUBLE_PUNCT.sub(r"\2", text)
    text = SPACE_BEFORE_PUNCT.sub(r"\1", text)
    text = MULTI_SPACE.sub(" ", text)
    return text.strip(" -")


def finish(text: str) -> str:
    text = normalize_spacing_and_punctuation(text)
    if text:
        text = text[0].upper() + text[1:]
        text = SENTENCE_CAP.sub(lambda m: f"{m.group(1)} {m.group(2).upper()}", text)
    return text


def clean_current(text: str) -> str:
    if not text.strip():
        return ""
    text = FILLER_SINGLE_CURRENT.sub("", text)
    text = FILLER_PHRASE_CURRENT.sub("", text)
    words = text.split()
    if words:
        deduped = [words[0]]
        for word in words[1:]:
            if deduped[-1].lower() != word.lower():
                deduped.append(word)
        text = " ".join(deduped)
    return finish(text)


def remove_restart_stutters(text: str) -> str:
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


def clean_refined_segment(text: str) -> str:
    if not text.strip():
        return ""
    text = FILLER_SINGLE_REFINED.sub("", text)
    text = SOFT_MARKERS_REFINED.sub("", text)
    text = remove_restart_stutters(text)
    return normalize_spacing_and_punctuation(text)


def clean_refined_final(text: str) -> str:
    if not text.strip():
        return ""
    text = FILLER_SINGLE_REFINED.sub("", text)
    text = SOFT_MARKERS_REFINED.sub("", text)
    text = remove_restart_stutters(text)
    return finish(text)


def capitalize_first(text: str) -> str:
    return text[:1].upper() + text[1:] if text else ""


def tokenize(text: str) -> list[str]:
    return [part for part in text.strip().split() if part]


def first_word_lower(text: str) -> str:
    tokens = tokenize(text)
    return normalize_token(tokens[0]) if tokens else ""


def last_word_lower(text: str) -> str:
    tokens = tokenize(text)
    return normalize_token(tokens[-1]) if tokens else ""


def lowercase_first_word_force(text: str) -> str:
    match = re.match(r"^(\W*)([A-Z][A-Za-z']*)(.*)$", text)
    if not match:
        return text
    prefix, word, suffix = match.groups()
    if word == "I":
        return text
    if word.isupper() and len(word) > 1:
        return text
    return f"{prefix}{word.lower()}{suffix}"


def ends_with_meaningful_repetition(tokens: list[str]) -> bool:
    if len(tokens) < 2:
        return False
    return normalize_token(tokens[-1]) == normalize_token(tokens[-2]) != ""


def is_short_affirmation(prev_tokens: list[str], first: str) -> bool:
    if len(prev_tokens) != 1:
        return False
    return normalize_token(prev_tokens[-1]) in AFFIRMATIONS and first != "" and first != "can"


def current_join(segments: list[str]) -> str:
    if not segments:
        return ""
    result = segments[0]
    for seg in segments[1:]:
        if not seg.strip():
            continue
        trimmed_prev = result.rstrip()
        needs_period = (
            not trimmed_prev.endswith(".")
            and not trimmed_prev.endswith("!")
            and not trimmed_prev.endswith("?")
            and not trimmed_prev.endswith(",")
        )
        if needs_period:
            result = trimmed_prev + "."
        result += " " + capitalize_first(seg.strip())
    return result


def heuristic_join(segments: list[str]) -> str:
    cleaned = [seg.strip() for seg in segments if seg.strip()]
    if not cleaned:
        return ""
    result = cleaned[0]
    for seg in cleaned[1:]:
        prev = result.rstrip()
        curr = seg
        first = first_word_lower(curr)
        last = last_word_lower(prev)
        prev_tokens = tokenize(prev)

        if prev.endswith((".", "!", "?")):
            result += " " + curr
            continue

        if prev.endswith((",", ":")):
            result += " " + lowercase_first_word_force(curr)
            continue

        if is_short_affirmation(prev_tokens, first):
            result += " " + lowercase_first_word_force(curr)
            continue

        if len(prev_tokens) <= 2 and last in COMMA_LEAD_INS:
            result = prev.rstrip(",") + ", " + lowercase_first_word_force(curr)
            continue

        if (
            first in CONTINUATION_FIRST_WORDS
            or last in TRAILING_INCOMPLETE_WORDS
            or ends_with_meaningful_repetition(prev_tokens)
        ):
            result += " " + lowercase_first_word_force(curr)
            continue

        result = prev + ". " + curr
    return result


def current_pipeline(raw_segments: tuple[str, ...]) -> str:
    cleaned = [clean_current(seg) for seg in raw_segments]
    cleaned = [seg for seg in cleaned if seg]
    return clean_current(current_join(cleaned))


def proposed_pipeline(raw_segments: tuple[str, ...]) -> str:
    cleaned = [clean_refined_segment(seg) for seg in raw_segments]
    cleaned = [seg for seg in cleaned if seg]
    return clean_refined_final(heuristic_join(cleaned))


def evaluate(pipeline: Callable[[tuple[str, ...]], str]) -> list[dict[str, object]]:
    rows = []
    for index, case in enumerate(CASES, start=1):
        output = pipeline(case.raw_segments)
        rows.append(
            {
                "id": index,
                "family": case.family,
                "raw_segments": case.raw_segments,
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


def segments_to_markdown(segments: tuple[str, ...]) -> str:
    return "<br>".join(f"`{markdown_escape(segment)}`" for segment in segments)


def build_report() -> str:
    current_rows = evaluate(current_pipeline)
    proposed_rows = evaluate(proposed_pipeline)
    current_passed, total = score(current_rows)
    proposed_passed, _ = score(proposed_rows)

    lines: list[str] = []
    lines.append("# Pipeline Regression Pack")
    lines.append("")
    lines.append("## Scope")
    lines.append("")
    lines.append("This report expands the remaining end-to-end pipeline failures into a focused regression pack.")
    lines.append("")
    lines.append(f"- Total hard cases: `{total}`")
    lines.append(f"- Current pipeline pass rate: `{current_passed}/{total}`")
    lines.append(f"- Proposed pipeline pass rate: `{proposed_passed}/{total}`")
    lines.append("")
    lines.append("## Family Breakdown")
    lines.append("")
    lines.append("| Failure Family | Cases | Current Passed | Proposed Passed |")
    lines.append("| --- | ---: | ---: | ---: |")
    families = sorted({case.family for case in CASES})
    for family in families:
        current_family = [row for row in current_rows if row["family"] == family]
        proposed_family = [row for row in proposed_rows if row["family"] == family]
        lines.append(
            f"| {family} | {len(current_family)} | "
            f"{sum(1 for row in current_family if row['passed'])} | "
            f"{sum(1 for row in proposed_family if row['passed'])} |"
        )

    lines.append("")
    lines.append("## Per-Test Results")
    lines.append("")
    lines.append("| # | Family | Raw Segments | Expected | Current Pipeline | Proposed Pipeline | Current | Proposed |")
    lines.append("| ---: | --- | --- | --- | --- | --- | --- | --- |")
    for current_row, proposed_row in zip(current_rows, proposed_rows, strict=True):
        lines.append(
            f"| {current_row['id']} | {current_row['family']} | "
            f"{segments_to_markdown(current_row['raw_segments'])} | "
            f"`{markdown_escape(str(current_row['expected']))}` | "
            f"`{markdown_escape(str(current_row['output']))}` | "
            f"`{markdown_escape(str(proposed_row['output']))}` | "
            f"{'PASS' if current_row['passed'] else 'FAIL'} | "
            f"{'PASS' if proposed_row['passed'] else 'FAIL'} |"
        )

    lines.append("")
    lines.append("## Remaining Risk Areas")
    lines.append("")
    lines.append("- Real-user chunk boundaries may still reveal edge cases not covered by the synthetic pack.")
    lines.append("- Minimal-join behavior with cleanup disabled is intentionally not scored here; this pack focuses on cleanup-enabled dictation.")
    lines.append("- If future tuning expands heuristics further, this report should grow before rules get more permissive.")
    lines.append("")
    lines.append("## How To Regenerate")
    lines.append("")
    lines.append("```powershell")
    lines.append("python scripts/evaluate_pipeline_regressions.py")
    lines.append("```")
    lines.append("")
    lines.append("This rewrites `docs/PIPELINE_REGRESSION_PACK.md`.")
    lines.append("")
    return "\n".join(lines)


def main() -> None:
    repo_root = Path(__file__).resolve().parents[1]
    report_path = repo_root / "docs" / "PIPELINE_REGRESSION_PACK.md"
    report_path.write_text(build_report(), encoding="utf-8")
    print(f"Wrote {report_path}")


if __name__ == "__main__":
    main()
