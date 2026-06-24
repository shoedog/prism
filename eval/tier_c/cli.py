"""tier-c entry (spec). Supports --list and (Phase-1b/1c) a `run` subcommand.
Mirrors tier_a/cli.py argument-parsing style."""
from __future__ import annotations
import argparse
import os
from .corpus import load_issues


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="tier-c")
    sub = ap.add_subparsers(dest="cmd")

    # Legacy flat interface (Phase-1): tier-c --issues <f> [--list]
    ap.add_argument("--issues")
    ap.add_argument("--list", action="store_true")

    # Phase-1b/1c: tier-c run --issues <f> [--live] [--bench-root <dir>]
    run_p = sub.add_parser("run", help="run the A/B harness over an issues file")
    run_p.add_argument("--issues", required=True)
    run_p.add_argument("--live", action="store_true",
                       help="execute live model calls (requires corpus + API keys)")
    run_p.add_argument("--bench-root", default="~/code/bench-repos",
                       help="root directory containing cloned benchmark repos (default: ~/code/bench-repos)")

    args = ap.parse_args(argv)

    if args.cmd == "run":
        issues = load_issues(args.issues)
        if not args.live:
            print(
                "live run requires --live + corpus + API keys;\n"
                "pass --live --bench-root <dir> to execute the full 4-variant spec->plan chain."
            )
            return 0

        bench_root = os.path.expanduser(args.bench_root)
        _run_live_cmd(issues, bench_root)
        return 0

    # Legacy flat interface (original cli)
    if args.issues is None:
        ap.error("--issues is required")
    issues = load_issues(args.issues)
    if args.list:
        for i in issues:
            print(f"{i.key}\t{i.language}\t{i.scoped_slice}")
        return 0
    print(f"loaded {len(issues)} issues (run driver lands in Task 13 / Phase-1 live run)")
    return 0


def _run_live_cmd(issues, bench_root: str) -> None:
    """Build LiveComponents and run the full live loop, printing per-cell results."""
    from .model import Variant
    from .arm_runner import ClaudeRunner, CodexRunner, RoutingArmRunner
    from .judges_live import LlmRankJudge, LlmRelevanceJudge, LlmConditionGuesser
    from .llm import live_ask
    from .checkout import Checkout
    from .run import LiveComponents, run_live

    variants = [
        Variant(m, p)
        for m in ("opus-4.8", "gpt-5.5")
        for p in (False, True)
    ]

    runner = RoutingArmRunner(
        claude=ClaudeRunner(),
        codex=CodexRunner(),
    )

    judges = {
        "anthropic": LlmRankJudge(live_ask, "opus-4.8"),
        "openai": LlmRankJudge(live_ask, "gpt-5.5"),
    }
    relevance = LlmRelevanceJudge(live_ask, "opus-4.8")
    guesser = LlmConditionGuesser(live_ask, "opus-4.8")

    def open_checkout(repo, sha):
        path = os.path.join(bench_root, repo)
        return Checkout(path, sha)

    comps = LiveComponents(
        variants=variants,
        runner=runner,
        judges=judges,
        relevance=relevance,
        guesser=guesser,
        plants=[],
        open_checkout=open_checkout,
    )

    print(f"Running live harness: {len(issues)} issues, {len(variants)} variants, 2 stages ...")
    report = run_live(issues, comps)

    # Print per-(stage x language) cells
    print("\n=== Per-(stage x language) Report ===")
    for (stage, language), cell in sorted(report.cells.items()):
        print(f"\n[{stage} / {language}]")
        print(f"  gate: {cell.gate.decision}  — {cell.gate.reason}")
        print(f"  itt_available_rate:      {cell.itt_available_rate:.2f}")
        print(f"  per_protocol_used_rate:  {cell.per_protocol_used_rate:.2f}")
        if cell.prism_precision_delta:
            print(f"  prism_precision_delta:   {cell.prism_precision_delta}")
        if cell.prism_recall_delta:
            print(f"  prism_recall_delta:      {cell.prism_recall_delta}")
        if cell.prism_planted_delta:
            print(f"  prism_planted_delta:     {cell.prism_planted_delta}")

    # Print pooled detectability
    det = report.detectability
    print(f"\n=== Pooled Detectability ===")
    print(f"  correct/n: {det.correct}/{det.n}  pvalue: {det.pvalue:.4f}  detectable: {det.detectable}")
    if det.detectable:
        print("  WARNING: judge prism-delta is INVALID (condition detectable above chance, spec §6b)")
    else:
        print("  OK: judge prism-delta valid (condition not detectable above chance)")
