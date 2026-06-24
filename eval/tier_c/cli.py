"""tier-c entry (spec). Supports --list and (Phase-1b) a `run` subcommand.
Mirrors tier_a/cli.py argument-parsing style."""
from __future__ import annotations
import argparse
from .corpus import load_issues


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="tier-c")
    sub = ap.add_subparsers(dest="cmd")

    # Legacy flat interface (Phase-1): tier-c --issues <f> [--list]
    ap.add_argument("--issues")
    ap.add_argument("--list", action="store_true")

    # Phase-1b: tier-c run --issues <f> [--live]
    run_p = sub.add_parser("run", help="run the A/B harness over an issues file")
    run_p.add_argument("--issues", required=True)
    run_p.add_argument("--live", action="store_true",
                       help="execute live model calls (requires corpus + API keys)")

    args = ap.parse_args(argv)

    if args.cmd == "run":
        issues = load_issues(args.issues)
        if not args.live:
            print(
                "live run requires --live + corpus + API; "
                "pass --live to execute (Phase-1b plan Task 7 Step 5 TODO)"
            )
            return 0
        # TODO (Phase-1b plan Task 7 Step 5): wire the live execution loop here.
        # Steps:
        #   1. Construct ClaudeRunner / CodexRunner from config / env.
        #   2. Build variants (on+off for each model).
        #   3. For each issue: checkout.Checkout(issue.repo, issue.sha),
        #      call run_issue(issue, ...), collect ChainResult.
        #   4. Aggregate StageMetrics across issues/languages.
        #   5. Call assemble_cell per (stage x language) and print GO/NO-GO table.
        # This is intentionally deferred to the first-real-run session so the live
        # runner seam (ClaudeRunner/CodexRunner) can be verified against real output.
        print(f"live run over {len(issues)} issues — wiring TODO (see plan Task 7 Step 5)")
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
