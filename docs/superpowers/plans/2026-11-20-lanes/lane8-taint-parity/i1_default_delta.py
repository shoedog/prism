"""Compare default line and marked-operand verdicts with the main-head binary."""

import argparse
import json
import os
from pathlib import Path
import subprocess


VERDICT_ORDER = ("Reached", "BoundaryExited", "Sanitized", "NotReached")
SEED_ERRORS = {
    "AmbiguousSymbol",
    "LocationOutOfRange",
    "NoEnclosingCallable",
    "SymbolNotFound",
    "UnsupportedFile",
}


def verdict_at_operand(summary, row):
    src = row["src"]
    start, end = row["sink"]["operand"]
    terminals = []
    for sink in summary["per_sink"]:
        operand = sink["sink"]["Variable"]
        left, right = operand["start_byte"], operand["end_byte"]
        overlaps = left == start if left == right else left < end and start < right
        if operand["file"] != row["sink"]["file"] or not overlaps:
            continue
        if sink["sources_omitted"]:
            raise ValueError("CLI omitted source results")
        for result in sink["sources"]:
            root = result["source"]["Variable"]
            if root["file"] == src["file"] and [root["start_byte"], root["end_byte"]] == src["bytes"]:
                terminals.append((result["reachability"], left, right, result.get("reason")))
    if not terminals:
        return "NotReached", []
    verdict, left, right, _ = min(terminals, key=lambda x: (VERDICT_ORDER.index(x[0]), x[1:3]))
    reasons = sorted({reason for status, start, end, reason in terminals
                      if (status, start, end) == (verdict, left, right) and reason})
    return verdict, reasons


def observe(binary, repo, row):
    src, sink = row["src"], row["sink"]
    env = {key: value for key, value in os.environ.items() if not key.startswith("PRISM_TAINT_")}
    proc = subprocess.run(
        [binary, "nav", "taint-reaches", "--repo", repo,
         "--source", f'{src["file"]}:{src["line"]}', "--sink", f'{sink["file"]}:{sink["line"]}',
         "--format", "json"],
        env=env, capture_output=True, text=True, timeout=30,
    )
    payload = json.loads(proc.stdout)
    if "error" in payload:
        error = payload["error"]
        if len(error) != 1 or not error.keys() <= SEED_ERRORS:
            raise ValueError(f"unexpected CLI error {error}")
        return {"line": "SeedError", "operand": "SeedError", "error": error, "reasons": [],
                "line_reasons": []}
    if proc.returncode:
        raise ValueError(f"CLI exited {proc.returncode}: {proc.stderr}")
    summary = payload["reasoning"]
    if summary["sinks_omitted"] or any(s["sources_omitted"] for s in summary["per_sink"]):
        raise ValueError("CLI clipped the witness results")
    verdict, reasons = verdict_at_operand(summary, row)
    line = summary["reachability"]
    line_reasons = set()
    for operand in summary["per_sink"]:
        for pair in operand["sources"]:
            if reason := pair.get("reason"):
                if pair["reachability"] != "BoundaryExited":
                    raise ValueError(f"unexpected boundary reason {reason} on {pair['reachability']}")
                if line == "BoundaryExited":
                    line_reasons.add(reason)
    boundaries = []
    for warning in payload["warnings"]:
        kind = warning["kind"]
        if isinstance(kind, dict) and "InterproceduralBoundary" in kind.get("Reasoning", {}):
            boundaries.append({"location": warning["location"], "message": warning["message"]})
    return {"line": line, "operand": verdict, "reasons": reasons,
            "line_reasons": sorted(line_reasons), "boundary_warnings": boundaries}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-root", type=Path, default=Path("tests/taint_parity"))
    parser.add_argument("--base", type=Path, required=True)
    parser.add_argument("--head", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    root = args.fixture_root.resolve(strict=True)
    base, head = args.base.resolve(strict=True), args.head.resolve(strict=True)
    rows = [json.loads(line) for shard in sorted(root.glob("rows*.jsonl"))
            for line in shard.read_text().splitlines() if line.strip()]
    if not rows or len({row["id"] for row in rows}) != len(rows):
        raise ValueError("empty or duplicate row population")
    ledger = []
    for row in rows:
        fixture = root / row["fixture"]
        if (fixture.parent != root or fixture.resolve(strict=True).parent != root
                or fixture.name.startswith("PRIVATE")
                or any(p.name.startswith("PRIVATE") or p.is_symlink() for p in fixture.rglob("*"))):
            raise ValueError(f"fixture outside the admitted corpus: {fixture}")
        try:
            before, after = observe(base, fixture, row), observe(head, fixture, row)
        except (ValueError, KeyError, subprocess.TimeoutExpired) as exc:
            raise RuntimeError(f'{row["id"]}: {exc}') from exc
        ledger.append({"id": row["id"], "group": row["group"], "base": before, "head": after})

    args.out.write_text("".join(json.dumps(entry, sort_keys=True) + "\n" for entry in ledger))
    print(f"{len(ledger)} paired rows")
    differences = 0
    for entry in ledger:
        before, after = entry["base"], entry["head"]
        for scope, reasons in [("operand", "reasons"), ("line", "line_reasons")]:
            if before[scope] != after[scope]:
                differences += 1
                print(entry["id"], scope, before[scope], after[scope],
                      ",".join(after[reasons]) or "–", sep="\t")
        for field in ("error", "boundary_warnings"):
            if before.get(field) != after.get(field):
                differences += 1
                print(entry["id"], field, before.get(field), after.get(field), sep="\t")
    return int(differences != 0)


if __name__ == "__main__":
    raise SystemExit(main())
