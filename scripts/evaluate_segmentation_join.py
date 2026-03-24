from __future__ import annotations

import re
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable


@dataclass(frozen=True)
class Case:
    category: str
    segments: tuple[str, ...]
    expected: str


CASES: list[Case] = [
    Case("mid-sentence continuation", ("I think we should", "Go tomorrow because the API is down"), "I think we should go tomorrow because the API is down"),
    Case("mid-sentence continuation", ("Open the settings menu", "And then click advanced"), "Open the settings menu and then click advanced"),
    Case("mid-sentence continuation", ("This is probably fine", "If we add a retry"), "This is probably fine if we add a retry"),
    Case("mid-sentence continuation", ("I left it disabled", "Because staging was unstable"), "I left it disabled because staging was unstable"),
    Case("mid-sentence continuation", ("The issue started yesterday", "When the cron job retried"), "The issue started yesterday when the cron job retried"),
    Case("mid-sentence continuation", ("This is the feature", "That users asked for"), "This is the feature that users asked for"),
    Case("mid-sentence continuation", ("Please open", "The latest report"), "Please open the latest report"),
    Case("mid-sentence continuation", ("The thing that worries me is", "The rollback path"), "The thing that worries me is the rollback path"),
    Case("multi-segment sentence", ("We need to update", "The documentation", "Before the release"), "We need to update the documentation before the release"),
    Case("multi-segment sentence", ("Can you check", "Whether the migration ran", "On staging"), "Can you check whether the migration ran on staging"),
    Case("multi-segment sentence", ("Open the dashboard", "And check the alerts", "Then ping me"), "Open the dashboard and check the alerts then ping me"),
    Case("explicit sentence boundary", ("We shipped the fix", "Please verify production"), "We shipped the fix. Please verify production"),
    Case("explicit sentence boundary", ("The server restarted", "It came back cleanly"), "The server restarted. It came back cleanly"),
    Case("explicit sentence boundary", ("I reviewed the logs", "Nothing looked suspicious"), "I reviewed the logs. Nothing looked suspicious"),
    Case("punctuation already present", ("We shipped the fix.", "Please verify production"), "We shipped the fix. Please verify production"),
    Case("punctuation already present", ("What changed?", "Can you summarize the rollout"), "What changed? Can you summarize the rollout"),
    Case("comma continuation", ("For the rollout,", "We should notify support"), "For the rollout, we should notify support"),
    Case("comma continuation", ("If this fails,", "We can revert quickly"), "If this fails, we can revert quickly"),
    Case("short discourse lead-in", ("Well", "I guess that works"), "Well, I guess that works"),
    Case("short discourse lead-in", ("So", "What do we change next"), "So, what do we change next"),
    Case("affirmation", ("Yes", "That matches my logs"), "Yes that matches my logs"),
    Case("affirmation", ("No", "That is not the right file"), "No that is not the right file"),
    Case("mixed realistic", ("We should probably", "Wait until tomorrow", "Because support is offline"), "We should probably wait until tomorrow because support is offline"),
    Case("mixed realistic", ("Can you open", "The billing page", "And check the failed invoices"), "Can you open the billing page and check the failed invoices"),
    Case("mixed realistic", ("The update is live", "Can you smoke test the login flow"), "The update is live. Can you smoke test the login flow"),
    Case("mixed realistic", ("Actually", "I think the first version was better"), "Actually, I think the first version was better"),
    Case("mixed realistic", ("Basically", "We just need one more approval"), "Basically, we just need one more approval"),
    Case("mixed realistic", ("I pushed the fix", "To the release branch"), "I pushed the fix to the release branch"),
]


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

CHUNK_SIZE = 512
VAD_SAMPLE_RATE = 16000
STOP_DRAIN_MS = 150


def tokenize(text: str) -> list[str]:
    return [part for part in text.strip().split() if part]


def normalize_token(token: str) -> str:
    return re.sub(r"(^[^A-Za-z']+|[^A-Za-z']+$)", "", token).lower()


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


def capitalize_first(text: str) -> str:
    return text[:1].upper() + text[1:] if text else ""


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


def current_join(segments: tuple[str, ...]) -> str:
    cleaned = [seg for seg in segments if seg.strip()]
    if not cleaned:
        return ""
    if len(cleaned) == 1:
        return cleaned[0]

    result = cleaned[0]
    for seg in cleaned[1:]:
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


def space_only_join(segments: tuple[str, ...]) -> str:
    return " ".join(seg.strip() for seg in segments if seg.strip())


def ends_with_meaningful_repetition(tokens: list[str]) -> bool:
    if len(tokens) < 2:
        return False
    return normalize_token(tokens[-1]) == normalize_token(tokens[-2]) != ""


def is_short_affirmation(prev_tokens: list[str], first: str) -> bool:
    if len(prev_tokens) != 1:
        return False
    return normalize_token(prev_tokens[-1]) in AFFIRMATIONS and first != "" and first != "can"


def heuristic_join(segments: tuple[str, ...]) -> str:
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


def evaluate(joiner: Callable[[tuple[str, ...]], str]) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for index, case in enumerate(CASES, start=1):
        output = joiner(case.segments)
        rows.append(
            {
                "id": index,
                "category": case.category,
                "segments": case.segments,
                "expected": case.expected,
                "output": output,
                "passed": output == case.expected,
            }
        )
    return rows


def score(rows: list[dict[str, object]]) -> tuple[int, int]:
    passed = sum(1 for row in rows if row["passed"])
    return passed, len(rows)


def benchmark(joiner: Callable[[tuple[str, ...]], str], rounds: int = 5000) -> float:
    start = time.perf_counter()
    checksum = 0
    for _ in range(rounds):
        for case in CASES:
            checksum += len(joiner(case.segments))
    elapsed = time.perf_counter() - start
    if checksum == -1:
        raise RuntimeError("unreachable")
    return elapsed / (len(CASES) * rounds) * 1_000_000


def effective_endpoint_ms(target_silence_ms: int) -> int:
    chunk_ms = int(CHUNK_SIZE / VAD_SAMPLE_RATE * 1000)
    silence_chunks = target_silence_ms // max(chunk_ms, 1)
    return silence_chunks * chunk_ms


def markdown_escape(text: str) -> str:
    return text.replace("|", "\\|")


def segments_to_markdown(segments: tuple[str, ...]) -> str:
    return "<br>".join(f"`{markdown_escape(segment)}`" for segment in segments)


def build_report() -> str:
    current_rows = evaluate(current_join)
    space_rows = evaluate(space_only_join)
    heuristic_rows = evaluate(heuristic_join)

    current_passed, total = score(current_rows)
    space_passed, _ = score(space_rows)
    heuristic_passed, _ = score(heuristic_rows)

    current_us = benchmark(current_join)
    space_us = benchmark(space_only_join)
    heuristic_us = benchmark(heuristic_join)

    lines: list[str] = []
    lines.append("# Segmentation And Join Evaluation")
    lines.append("")
    lines.append("## Scope")
    lines.append("")
    lines.append("This report evaluates how chunked transcript segments are recombined into final text. It focuses on deterministic join behavior, not model inference.")
    lines.append("")
    lines.append("The examples are synthetic but modeled after VAD-caused sentence splits that commonly happen in dictation when the speaker pauses mid-thought.")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- Total cases: `{total}`")
    lines.append(f"- Current join pass rate: `{current_passed}/{total}`")
    lines.append(f"- Space-only join pass rate: `{space_passed}/{total}`")
    lines.append(f"- Heuristic join pass rate: `{heuristic_passed}/{total}`")
    lines.append("")
    lines.append("## What The Current Code Did")
    lines.append("")
    lines.append("- Legacy `smart_join` added a period between segments whenever the previous segment did not end in `.`, `!`, `?`, or `,`.")
    lines.append("- It also capitalized the next segment unconditionally.")
    lines.append("- This meant many VAD boundaries became sentence boundaries.")
    lines.append("")
    lines.append("## What The Heuristic Variant Does")
    lines.append("")
    lines.append("- Preserve real sentence boundaries when punctuation is already present.")
    lines.append("- Join obvious continuations with a space instead of forcing a period.")
    lines.append("- Lowercase function-word continuations like `And`, `If`, `Because`, `The`, and `Whether` when they were capitalized only because a new chunk started.")
    lines.append("- Turn short lead-ins like `Well`, `So`, `Actually`, and `Basically` into comma continuations.")
    lines.append("- Treat short affirmations like `Yes` and `No` as continuations instead of sentence starts when the next chunk clearly continues the same thought.")
    lines.append("")
    lines.append("## Category Breakdown")
    lines.append("")
    lines.append("| Category | Cases | Current Passed | Space-Only Passed | Heuristic Passed |")
    lines.append("| --- | ---: | ---: | ---: | ---: |")

    categories = sorted({case.category for case in CASES})
    for category in categories:
        current_in_category = [row for row in current_rows if row["category"] == category]
        space_in_category = [row for row in space_rows if row["category"] == category]
        heuristic_in_category = [row for row in heuristic_rows if row["category"] == category]
        lines.append(
            f"| {category} | {len(current_in_category)} | "
            f"{sum(1 for row in current_in_category if row['passed'])} | "
            f"{sum(1 for row in space_in_category if row['passed'])} | "
            f"{sum(1 for row in heuristic_in_category if row['passed'])} |"
        )

    lines.append("")
    lines.append("## Per-Test Results")
    lines.append("")
    lines.append("| # | Category | Segments | Expected | Current | Space-Only | Heuristic | Current | Space | Heuristic |")
    lines.append("| ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- |")

    for current_row, space_row, heuristic_row in zip(current_rows, space_rows, heuristic_rows, strict=True):
        lines.append(
            f"| {current_row['id']} | {current_row['category']} | "
            f"{segments_to_markdown(current_row['segments'])} | "
            f"`{markdown_escape(str(current_row['expected']))}` | "
            f"`{markdown_escape(str(current_row['output']))}` | "
            f"`{markdown_escape(str(space_row['output']))}` | "
            f"`{markdown_escape(str(heuristic_row['output']))}` | "
            f"{'PASS' if current_row['passed'] else 'FAIL'} | "
            f"{'PASS' if space_row['passed'] else 'FAIL'} | "
            f"{'PASS' if heuristic_row['passed'] else 'FAIL'} |"
        )

    lines.append("")
    lines.append("## CPU Cost Of Join Strategies")
    lines.append("")
    lines.append("| Strategy | Approx microseconds per join call | Relative to Current |")
    lines.append("| --- | ---: | ---: |")
    lines.append(f"| Current | {current_us:.3f} | 1.00x |")
    lines.append(f"| Space-only | {space_us:.3f} | {space_us / current_us:.2f}x |")
    lines.append(f"| Heuristic | {heuristic_us:.3f} | {heuristic_us / current_us:.2f}x |")
    lines.append("")
    lines.append("These costs are far below transcription cost. In practice, join logic itself is unlikely to be user-visible.")
    lines.append("")
    lines.append("## Endpointing Latency Context")
    lines.append("")
    lines.append("User-visible latency is dominated by VAD endpointing and the stop-time drain wait, not by string joining.")
    lines.append("")
    lines.append("| Configured silence threshold | Effective silence before endpoint | Plus stop drain wait | Approx total before finalization |")
    lines.append("| ---: | ---: | ---: | ---: |")
    for threshold in (500, 700, 900):
        effective = effective_endpoint_ms(threshold)
        lines.append(f"| {threshold} ms | {effective} ms | {STOP_DRAIN_MS} ms | {effective + STOP_DRAIN_MS} ms |")

    lines.append("")
    lines.append("## Recommended Interpretation")
    lines.append("")
    lines.append("- Changing only the join logic should not create a noticeable slowdown.")
    lines.append("- Increasing the VAD silence threshold can improve sentence boundaries, but it adds real waiting time.")
    lines.append("- Best first experiment: keep the current endpointing threshold and replace the hard period-insertion join logic.")
    lines.append("")
    lines.append("## How To Regenerate")
    lines.append("")
    lines.append("Run:")
    lines.append("")
    lines.append("```powershell")
    lines.append("python scripts/evaluate_segmentation_join.py")
    lines.append("```")
    lines.append("")
    lines.append("This rewrites `docs/SEGMENTATION_JOIN_EVALUATION.md`.")
    lines.append("")
    return "\n".join(lines)


def main() -> None:
    repo_root = Path(__file__).resolve().parents[1]
    report_path = repo_root / "docs" / "SEGMENTATION_JOIN_EVALUATION.md"
    report_path.write_text(build_report(), encoding="utf-8")
    print(f"Wrote {report_path}")


if __name__ == "__main__":
    main()
