from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Rule:
    find: str
    replace: str


@dataclass(frozen=True)
class Case:
    category: str
    active_pack: str
    spoken: str
    expected: str


USER_RULES: tuple[Rule, ...] = (
    Rule("yolo voice", "YOLO Voice"),
    Rule("parakeet t d t", "Parakeet TDT"),
    Rule("alish term", "AlishTerm"),
    Rule("qyzylorda", "Qyzylorda"),
)

PACK_RULES: dict[str, tuple[Rule, ...]] = {
    "general": (),
    "software_engineering": (
        Rule("super base", "Supabase"),
        Rule("verse cell", "Vercel"),
        Rule("next yes", "Next.js"),
        Rule("cube control", "kubectl"),
        Rule("post gress", "PostgreSQL"),
        Rule("graph QL", "GraphQL"),
        Rule("rest API", "REST API"),
        Rule("cloud flare", "Cloudflare"),
        Rule("get hub", "GitHub"),
        Rule("VS code", "VS Code"),
        Rule("pie torch", "PyTorch"),
        Rule("open AI", "OpenAI"),
        Rule("tory", "Tauri"),
        Rule("lang chain", "LangChain"),
        Rule("hugging face", "Hugging Face"),
        Rule("deep gram", "Deepgram"),
        Rule("type script", "TypeScript"),
        Rule("engine X", "Nginx"),
    ),
    "medical": (
        Rule("a fib", "AFib"),
        Rule("I see you", "ICU"),
        Rule("see BC", "CBC"),
    ),
    "legal": (
        Rule("force major", "force majeure"),
        Rule("sub peena", "subpoena"),
        Rule("star a decisis", "stare decisis"),
    ),
    "finance": (
        Rule("gap", "GAAP"),
        Rule("sox", "SOX"),
        Rule("series a", "Series A"),
        Rule("S and P 500", "S&P 500"),
        Rule("defy", "DeFi"),
    ),
}


CASES: list[Case] = [
    Case("framework names", "software_engineering", "type script", "TypeScript"),
    Case("framework names", "software_engineering", "next yes", "Next.js"),
    Case("framework names", "software_engineering", "lang chain", "LangChain"),
    Case("framework names", "software_engineering", "hugging face", "Hugging Face"),
    Case("framework names", "software_engineering", "open AI", "OpenAI"),
    Case("framework names", "software_engineering", "tory app", "Tauri app"),
    Case("infra + cli", "software_engineering", "super base project", "Supabase project"),
    Case("infra + cli", "software_engineering", "verse cell deploy", "Vercel deploy"),
    Case("infra + cli", "software_engineering", "cube control get pods", "kubectl get pods"),
    Case("infra + cli", "software_engineering", "cloud flare tunnel", "Cloudflare tunnel"),
    Case("infra + cli", "software_engineering", "engine X config", "Nginx config"),
    Case("infra + cli", "software_engineering", "get hub actions", "GitHub actions"),
    Case("acronyms + data", "software_engineering", "graph QL schema", "GraphQL schema"),
    Case("acronyms + data", "software_engineering", "rest API client", "REST API client"),
    Case("acronyms + data", "software_engineering", "VS code settings", "VS Code settings"),
    Case("acronyms + data", "software_engineering", "pie torch model", "PyTorch model"),
    Case("acronyms + data", "software_engineering", "deep gram websocket", "Deepgram websocket"),
    Case("acronyms + data", "software_engineering", "post gress migration", "PostgreSQL migration"),
    Case("repo-specific", "general", "yolo voice release", "YOLO Voice release"),
    Case("repo-specific", "software_engineering", "parakeet t d t model", "Parakeet TDT model"),
    Case("repo-specific", "general", "alish term checklist", "AlishTerm checklist"),
    Case("repo-specific", "general", "qyzylorda build", "Qyzylorda build"),
    Case("cross-domain leakage", "general", "the gap is widening", "the gap is widening"),
    Case("cross-domain leakage", "general", "I see you tomorrow", "I see you tomorrow"),
    Case("cross-domain leakage", "general", "force major issue", "force major issue"),
    Case("cross-domain leakage", "general", "series a of photos", "series a of photos"),
    Case("cross-domain leakage", "general", "defy expectations", "defy expectations"),
    Case("cross-domain leakage", "general", "verse cell choir", "verse cell choir"),
    Case("cross-domain leakage", "general", "engine X noise", "engine X noise"),
    Case("cross-domain leakage", "general", "type script notes", "type script notes"),
    Case("scoped packs", "medical", "a fib episode", "AFib episode"),
    Case("scoped packs", "medical", "transfer to I see you", "transfer to ICU"),
    Case("scoped packs", "legal", "force major clause", "force majeure clause"),
    Case("scoped packs", "legal", "sub peena response", "subpoena response"),
    Case("scoped packs", "finance", "gap revenue policy", "GAAP revenue policy"),
    Case("scoped packs", "finance", "S and P 500 tracker", "S&P 500 tracker"),
    Case("wrong-pack negatives", "software_engineering", "gap analysis", "gap analysis"),
    Case("wrong-pack negatives", "software_engineering", "I see you later", "I see you later"),
    Case("wrong-pack negatives", "software_engineering", "force major risk", "force major risk"),
    Case("wrong-pack negatives", "legal", "type script notes", "type script notes"),
    Case("wrong-pack negatives", "finance", "cube control command", "cube control command"),
    Case("wrong-pack negatives", "medical", "graph QL endpoint", "graph QL endpoint"),
]


def ordered_rules(rules: tuple[Rule, ...] | list[Rule]) -> list[Rule]:
    return sorted(rules, key=lambda rule: (-len(rule.find), rule.find.lower(), rule.replace.lower()))


def apply_rules(text: str, rules: tuple[Rule, ...] | list[Rule]) -> str:
    output = text
    for rule in ordered_rules(rules):
        pattern = re.compile(rf"(?i)\b{re.escape(rule.find)}\b")
        output = pattern.sub(rule.replace, output)
    return output


def current_pipeline(case: Case) -> str:
    merged = list(USER_RULES)
    for pack_rules in PACK_RULES.values():
        merged.extend(pack_rules)
    return apply_rules(case.spoken, merged)


def proposed_pipeline(case: Case) -> str:
    active_rules = list(USER_RULES)
    active_rules.extend(PACK_RULES.get(case.active_pack, ()))
    return apply_rules(case.spoken, active_rules)


def evaluate() -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    current_rows: list[dict[str, object]] = []
    proposed_rows: list[dict[str, object]] = []

    for index, case in enumerate(CASES, start=1):
        current_output = current_pipeline(case)
        proposed_output = proposed_pipeline(case)
        base = {
            "id": index,
            "category": case.category,
            "active_pack": case.active_pack,
            "spoken": case.spoken,
            "expected": case.expected,
        }
        current_rows.append(
            {
                **base,
                "output": current_output,
                "passed": current_output == case.expected,
            }
        )
        proposed_rows.append(
            {
                **base,
                "output": proposed_output,
                "passed": proposed_output == case.expected,
            }
        )

    return current_rows, proposed_rows


def score(rows: list[dict[str, object]]) -> tuple[int, int]:
    passed = sum(1 for row in rows if row["passed"])
    return passed, len(rows)


def markdown_escape(text: str) -> str:
    return text.replace("|", "\\|")


def build_report() -> str:
    current_rows, proposed_rows = evaluate()
    current_passed, total = score(current_rows)
    proposed_passed, _ = score(proposed_rows)

    lines: list[str] = []
    lines.append("# Dictionary Evaluation")
    lines.append("")
    lines.append("## Scope")
    lines.append("")
    lines.append(
        "This report evaluates dictionary behavior for Phase One of the transcription refinement plan."
    )
    lines.append("")
    lines.append(
        "The current model simulates today's merged-pack behavior where all pack rules can remain active at once."
    )
    lines.append(
        "The proposed model simulates scoped runtime resolution: user rules plus the currently active pack only."
    )
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- Total cases: `{total}`")
    lines.append(f"- Current merged-pack behavior: `{current_passed}/{total}`")
    lines.append(f"- Proposed scoped-pack behavior: `{proposed_passed}/{total}`")
    lines.append("")
    lines.append("## Coverage")
    lines.append("")
    lines.append("- Framework names")
    lines.append("- Infra and CLI terminology")
    lines.append("- Acronyms and capitalization")
    lines.append("- Repo-specific user rules")
    lines.append("- Cross-domain leakage")
    lines.append("- Wrong-pack negative cases")
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
    lines.append(
        "| # | Category | Active Pack | Spoken | Expected | Current Output | Proposed Output | Current | Proposed |"
    )
    lines.append("| ---: | --- | --- | --- | --- | --- | --- | --- | --- |")

    for current_row, proposed_row in zip(current_rows, proposed_rows, strict=True):
        lines.append(
            f"| {current_row['id']} | {current_row['category']} | {current_row['active_pack']} | "
            f"`{markdown_escape(str(current_row['spoken']))}` | "
            f"`{markdown_escape(str(current_row['expected']))}` | "
            f"`{markdown_escape(str(current_row['output']))}` | "
            f"`{markdown_escape(str(proposed_row['output']))}` | "
            f"{'PASS' if current_row['passed'] else 'FAIL'} | "
            f"{'PASS' if proposed_row['passed'] else 'FAIL'} |"
        )

    lines.append("")
    lines.append("## Interpretation")
    lines.append("")
    lines.append("- The main improvement is not more replacements; it is stopping wrong-pack rules from leaking into the hot path.")
    lines.append("- Software terminology still works when the software pack is active.")
    lines.append("- User-owned normalization rules still apply across packs.")
    lines.append("- Negative cases improve because ambiguous rules like `gap`, `I see you`, and `force major` stay scoped.")
    lines.append("")
    lines.append("## How To Regenerate")
    lines.append("")
    lines.append("```powershell")
    lines.append("python scripts/evaluate_dictionary_rules.py")
    lines.append("```")
    lines.append("")
    lines.append("This rewrites `docs/DICTIONARY_EVALUATION.md`.")
    lines.append("")
    return "\n".join(lines)


def main() -> None:
    repo_root = Path(__file__).resolve().parents[1]
    report_path = repo_root / "docs" / "DICTIONARY_EVALUATION.md"
    report_path.write_text(build_report(), encoding="utf-8")
    print(f"Wrote {report_path}")


if __name__ == "__main__":
    main()
