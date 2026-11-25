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
        if (set(record) not in ({"id", "truth"}, {"id", "truth", "case_sha256"})
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
        if record["truth"] not in allowed:
            raise ValueError(f"{truth_path}: {key} has invalid truth {record['truth']!r}")
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
