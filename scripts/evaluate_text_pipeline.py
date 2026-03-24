from __future__ import annotations

import re
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable


@dataclass(frozen=True)
class Case:
    category: str
    raw_segments: tuple[str, ...]
    expected: str


CASES: list[Case] = [
    Case("filler + continuation", ("um I think we should", "go tomorrow because the API is down"), "I think we should go tomorrow because the API is down"),
    Case("filler + continuation", ("uh open the settings menu", "and then click advanced"), "Open the settings menu and then click advanced"),
    Case("discourse lead-in", ("well", "I guess that works"), "Well, I guess that works"),
    Case("discourse lead-in", ("so", "what do we change next"), "So, what do we change next"),
    Case("discourse marker + hedge", ("you know, I I think", "this is basically okay"), "I think this is basically okay"),
    Case("discourse marker + filler", ("I mean, uh, we can", "probably merge this"), "We can probably merge this"),
    Case("stutter + continuation", ("the the API endpoint", "is down"), "The API endpoint is down"),
    Case("stutter + continuation", ("we we were going to ship", "today"), "We were going to ship today"),
    Case("soft semantics", ("it's kind of", "working"), "It's kind of working"),
    Case("soft semantics", ("I literally", "saw it happen"), "I literally saw it happen"),
    Case("soft semantics", ("this is sort of", "fragile"), "This is sort of fragile"),
    Case("meaningful repetition", ("this is very very", "important"), "This is very very important"),
    Case("meaningful repetition", ("I had had", "enough"), "I had had enough"),
    Case("comma continuation", ("for the rollout,", "we should notify support"), "For the rollout, we should notify support"),
    Case("explicit sentence boundary", ("we shipped the fix", "please verify production"), "We shipped the fix. Please verify production"),
    Case("explicit sentence boundary", ("what changed?", "can you summarize the rollout"), "What changed? Can you summarize the rollout"),
    Case("multi-segment continuation", ("um the the thing", "that worries me is", "the rollback path"), "The thing that worries me is the rollback path"),
    Case("multi-segment continuation", ("can you check", "whether the migration ran", "on staging"), "Can you check whether the migration ran on staging"),
    Case("mixed realistic", ("we should probably", "wait until tomorrow", "because support is offline"), "We should probably wait until tomorrow because support is offline"),
    Case("mixed realistic", ("can you open", "the billing page", "and check the failed invoices"), "Can you open the billing page and check the failed invoices"),
    Case("short discourse lead-in", ("actually", "I want to keep that"), "Actually, I want to keep that"),
    Case("short discourse lead-in", ("basically", "we just need one more approval"), "Basically, we just need one more approval"),
    Case("continuation with infinitive", ("I pushed the fix", "to the release branch"), "I pushed the fix to the release branch"),
    Case("continuation with article", ("please open", "the latest report"), "Please open the latest report"),
    Case("mixed filler + hedge", ("um actually I think", "it's fine"), "Actually I think it's fine"),
    Case("affirmation", ("uh-huh yes", "that's correct"), "Yes that's correct"),
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


def remove_stutters_current(text: str) -> str:
    words = text.split()
    if not words:
        return ""
    result = [words[0]]
    for word in words[1:]:
        if result[-1].lower() != word.lower():
            result.append(word)
    return " ".join(result)


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


def clean_current(text: str) -> str:
    if not text.strip():
        return ""
    text = FILLER_SINGLE_CURRENT.sub("", text)
    text = FILLER_PHRASE_CURRENT.sub("", text)
    text = remove_stutters_current(text)
    return finish(text)


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
    if not tokens:
        return ""
    return normalize_token(tokens[0])


def last_word_lower(text: str) -> str:
    tokens = tokenize(text)
    if not tokens:
        return ""
    return normalize_token(tokens[-1])


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


def smart_join_current(segments: list[str]) -> str:
    if not segments:
        return ""
    if len(segments) == 1:
        return segments[0]

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


def smart_join_heuristic(segments: list[str]) -> str:
    cleaned = [seg.strip() for seg in segments if seg.strip()]
    if not cleaned:
        return ""
    if len(cleaned) == 1:
        return cleaned[0]

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
    cleaned_segments = [clean_current(segment) for segment in raw_segments]
    cleaned_segments = [segment for segment in cleaned_segments if segment]
    joined = smart_join_current(cleaned_segments)
    return clean_current(joined)


def proposed_pipeline(raw_segments: tuple[str, ...]) -> str:
    cleaned_segments = [clean_refined_segment(segment) for segment in raw_segments]
    cleaned_segments = [segment for segment in cleaned_segments if segment]
    joined = smart_join_heuristic(cleaned_segments)
    return clean_refined_final(joined)


def evaluate(pipeline: Callable[[tuple[str, ...]], str]) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for index, case in enumerate(CASES, start=1):
        output = pipeline(case.raw_segments)
        rows.append(
            {
                "id": index,
                "category": case.category,
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


def benchmark(pipeline: Callable[[tuple[str, ...]], str], rounds: int = 4000) -> float:
    start = time.perf_counter()
    checksum = 0
    for _ in range(rounds):
        for case in CASES:
            checksum += len(pipeline(case.raw_segments))
    elapsed = time.perf_counter() - start
    if checksum == -1:
        raise RuntimeError("unreachable")
    return elapsed / (len(CASES) * rounds) * 1_000_000


def markdown_escape(text: str) -> str:
    return text.replace("|", "\\|")


def segments_to_markdown(segments: tuple[str, ...]) -> str:
    return "<br>".join(f"`{markdown_escape(segment)}`" for segment in segments)


def build_report() -> str:
    current_rows = evaluate(current_pipeline)
    proposed_rows = evaluate(proposed_pipeline)

    current_passed, total = score(current_rows)
    proposed_passed, _ = score(proposed_rows)

    current_us = benchmark(current_pipeline)
    proposed_us = benchmark(proposed_pipeline)

    lines: list[str] = []
    lines.append("# End-To-End Text Pipeline Evaluation")
    lines.append("")
    lines.append("## Scope")
    lines.append("")
    lines.append("This report simulates the deterministic text pipeline for VAD-style chunked transcripts:")
    lines.append("")
    lines.append("1. Per-segment cleanup")
    lines.append("2. Segment recombination")
    lines.append("3. Final cleanup pass")
    lines.append("")
    lines.append("It compares the legacy pipeline to the Phase 2 deterministic pipeline. No LLM is involved.")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- Total cases: `{total}`")
    lines.append(f"- Current pipeline pass rate: `{current_passed}/{total}`")
    lines.append(f"- Proposed pipeline pass rate: `{proposed_passed}/{total}`")
    lines.append("")
    lines.append("## Proposed Pipeline Behavior")
    lines.append("")
    lines.append("- Use lightweight per-segment cleanup instead of full destructive cleanup on every chunk.")
    lines.append("- Use heuristic segment joining instead of forcing periods at most segment boundaries.")
    lines.append("- Run one stronger final cleanup pass after joining.")
    lines.append("")
    lines.append("## Category Breakdown")
    lines.append("")
    lines.append("| Category | Cases | Current Passed | Proposed Passed |")
    lines.append("| --- | ---: | ---: | ---: |")

    categories = sorted({case.category for case in CASES})
    for category in categories:
        current_in_category = [row for row in current_rows if row["category"] == category]
        proposed_in_category = [row for row in proposed_rows if row["category"] == category]
        lines.append(
            f"| {category} | {len(current_in_category)} | "
            f"{sum(1 for row in current_in_category if row['passed'])} | "
            f"{sum(1 for row in proposed_in_category if row['passed'])} |"
        )

    lines.append("")
    lines.append("## Per-Test Results")
    lines.append("")
    lines.append("| # | Category | Raw Segments | Expected | Current Pipeline | Proposed Pipeline | Current | Proposed |")
    lines.append("| ---: | --- | --- | --- | --- | --- | --- | --- |")

    for current_row, proposed_row in zip(current_rows, proposed_rows, strict=True):
        lines.append(
            f"| {current_row['id']} | {current_row['category']} | "
            f"{segments_to_markdown(current_row['raw_segments'])} | "
            f"`{markdown_escape(str(current_row['expected']))}` | "
            f"`{markdown_escape(str(current_row['output']))}` | "
            f"`{markdown_escape(str(proposed_row['output']))}` | "
            f"{'PASS' if current_row['passed'] else 'FAIL'} | "
            f"{'PASS' if proposed_row['passed'] else 'FAIL'} |"
        )

    lines.append("")
    lines.append("## Estimated CPU Cost")
    lines.append("")
    lines.append("| Pipeline | Approx microseconds per case | Relative to Current |")
    lines.append("| --- | ---: | ---: |")
    lines.append(f"| Current deterministic pipeline | {current_us:.3f} | 1.00x |")
    lines.append(f"| Proposed deterministic pipeline | {proposed_us:.3f} | {proposed_us / current_us:.2f}x |")
    lines.append("")
    lines.append("Even when the proposed pipeline is slower in relative terms, the absolute cost remains tiny compared with audio transcription.")
    lines.append("")
    lines.append("## Interpretation")
    lines.append("")
    lines.append("- The legacy pipeline mostly fails by over-removing words and over-inserting sentence boundaries.")
    lines.append("- The Phase 2 pipeline improves lexical cleanup and sentence reconstruction together.")
    lines.append("- This report is still synthetic, so the next upgrade after Phase 2 would be to add anonymized real chunk outputs from the app.")
    lines.append("")
    lines.append("## How To Regenerate")
    lines.append("")
    lines.append("Run:")
    lines.append("")
    lines.append("```powershell")
    lines.append("python scripts/evaluate_text_pipeline.py")
    lines.append("```")
    lines.append("")
    lines.append("This rewrites `docs/TEXT_PIPELINE_EVALUATION.md`.")
    lines.append("")
    return "\n".join(lines)


def main() -> None:
    repo_root = Path(__file__).resolve().parents[1]
    report_path = repo_root / "docs" / "TEXT_PIPELINE_EVALUATION.md"
    report_path.write_text(build_report(), encoding="utf-8")
    print(f"Wrote {report_path}")


if __name__ == "__main__":
    main()
