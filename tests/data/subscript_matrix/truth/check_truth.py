"""Check that the truth receipts cover their entire input manifests."""

import argparse
from pathlib import Path

from run_truth import EXECUTED, HERE, MATRIX, case_path, check_case_digest, load_jsonl, load_rows


def check_rows(rows_path, truth_path, cases_path):
    rows = load_rows(rows_path)
    records = load_jsonl(truth_path)
    versions = records[0].get("versions")
    if (
        set(records[0]) != {"versions"} or not isinstance(versions, dict)
        or set(versions) != {"python", "node", "go"}
        or any(not isinstance(value, str) or not value.strip() for value in versions.values())
    ):
        raise ValueError(f"{truth_path}: missing or invalid interpreter versions")

    truth = {}
    for record in records[1:]:
        if (not {"id", "truth"} <= set(record)
                or set(record) - {"id", "truth", "case_sha256", "graph_mode", "runtime_mode"}
                or not isinstance(record["id"], str)):
            raise ValueError(f"{truth_path}: malformed row truth {record!r}")
        key = record["id"]
        if key in truth:
            raise ValueError(f"{truth_path}: duplicate truth for {key!r}")
        truth[key] = record

    expected = [row["id"] for row in rows]
    missing = set(expected) - truth.keys()
    extra = truth.keys() - set(expected)
    if missing or extra:
        raise ValueError(f"{truth_path}: missing truth for {sorted(missing)}, extra IDs {sorted(extra)}")
    if list(truth) != expected:
        raise ValueError(f"{truth_path}: truth records are out of row order")
    for row in rows:
        key = row["id"]
        allowed = ("reaches", "not") if row["lang"] in EXECUTED else ("n/a",)
        record = truth[key]
        modes = {name: record[name] for name in ("graph_mode", "runtime_mode") if name in record}
        expected_modes = {"graph_mode": "tsx", "runtime_mode": "ts"} if row["lang"] == "tsx" else {}
        if modes != expected_modes:
            raise ValueError(f"{truth_path}: {key} has invalid graph/runtime modes {modes!r}")
        observed = record["truth"]
        if row.get("truth_protocol") == "ab":
            valid = isinstance(observed, list) and len(observed) == 2 and all(
                isinstance(bit, bool) for bit in observed
            )
        else:
            valid = observed in allowed
        if not valid:
            raise ValueError(f"{truth_path}: {key} has invalid truth {observed!r}")
        if "markers" in row:
            markers = row["markers"]
            expected_markers = ["A", "B"] if row.get("truth_protocol") == "ab" else ["A"]
            if (not isinstance(markers, list) or any(not isinstance(m, dict) for m in markers)
                    or [m.get("marker") for m in markers] != expected_markers
                    or any(not isinstance(m.get("present"), bool) for m in markers)):
                raise ValueError(f"{rows_path}: {key} has invalid marker expectations")
            bits = observed if isinstance(observed, list) else [observed == "reaches"]
            if bits != [marker["present"] for marker in markers]:
                raise ValueError(f"{truth_path}: {key} observed {bits}, expected marker bits differ")
            if (row["lang"] != "py" or not versions["python"].startswith("3.12.")
                    or "case_sha256" not in row):
                raise ValueError(f"{truth_path}: {key} requires Python 3.12 and a case SHA-256")
        if ("case_sha256" in row) != ("case_sha256" in record) or (
            row.get("case_sha256") != record.get("case_sha256")
        ):
            raise ValueError(f"{truth_path}: {key} truth SHA-256 differs from the row")
        if "case_sha256" in row:
            path = case_path(cases_path.resolve(strict=True), row)
            check_case_digest(row, path.read_bytes())
    print(f"{truth_path}: {len(truth)} row truths")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("rows", type=Path, nargs="?", default=MATRIX / "rows.jsonl")
    parser.add_argument("truth", type=Path, nargs="?", default=HERE / "truth.jsonl")
    parser.add_argument("--cases", type=Path, help="defaults to cases/ beside the row index")
    args = parser.parse_args()
    try:
        check_rows(args.rows, args.truth, args.cases or args.rows.parent / "cases")
    except (OSError, ValueError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
