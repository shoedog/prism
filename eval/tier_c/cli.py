"""tier-c entry (spec). Phase 1 supports --list and (later) a fakes/live run driver.
Mirrors tier_a/cli.py argument-parsing style."""
from __future__ import annotations
import argparse
from .corpus import load_issues

def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="tier-c")
    ap.add_argument("--issues", required=True)
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args(argv)
    issues = load_issues(args.issues)
    if args.list:
        for i in issues:
            print(f"{i.key}\t{i.language}\t{i.scoped_slice}")
        return 0
    print(f"loaded {len(issues)} issues (run driver lands in Task 13 / Phase-1 live run)")
    return 0
