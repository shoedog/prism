"""Check the exact-parent Rust receipt and print its BASELINE.md table."""
import argparse
from hashlib import sha256
import json
from pathlib import Path
import re


MATRIX = Path(__file__).resolve().parent
PARENT = "14c74b4cd39264f6cdff1b3eb07c88b8e234f463"


def render(log):
    rows = {}
    for line in (MATRIX / "rows.jsonl").read_text().splitlines():
        row = json.loads(line)
        if row["group"] == "recall-py":
            contents = (MATRIX / "cases" / row["file"]).read_bytes()
            rows[row["id"]] = sha256(contents).hexdigest()
    builds = re.findall(r"\bT2_PARENT (\S+) build (\S+) (\S+)", log)
    totals = re.findall(r"^T2_PARENT_TOTAL (\d+)$", log, re.MULTILINE)
    if (len(builds) != 1 or builds[0][0] != PARENT
            or builds[0][1].removesuffix("-dirty") != PARENT[:12]
            or totals != [str(len(rows))] or not rows
            or "test result: ok. 1 passed; 0 failed;" not in log):
        raise ValueError("expected one successful exact-parent run over the full recall-py group")
    results = {}
    for ident, digest, reason in re.findall(
        r"^T2_PARENT_RED (\S+) ([0-9a-f]{64}) (.+)$", log, re.MULTILINE
    ):
        if ident in results or rows.get(ident) != digest:
            raise ValueError(f"duplicate, unknown, or stale parent receipt for {ident}")
        results[ident] = reason
    if results.keys() != rows.keys():
        raise ValueError(f"missing parent REDs for {sorted(rows.keys() - results.keys())}")
    lines = [f"Parent `{PARENT}` and library build `{builds[0][2]}`.",
             "", "| Row | Case SHA-256 | Parent RED |", "|---|---|---|"]
    for ident, reason in sorted(results.items()):
        escaped = reason.replace("|", "\\|")
        lines.append(f"| `{ident}` | `{rows[ident]}` | {escaped} |")
    return "\n".join(lines)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log", type=Path)
    args = parser.parse_args()
    try:
        print(render(args.log.read_text()))
    except (OSError, ValueError) as error:
        parser.error(str(error))
