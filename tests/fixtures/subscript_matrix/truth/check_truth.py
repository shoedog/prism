"""Check that the truth receipts cover their entire input manifests."""

import argparse
from collections import Counter
from pathlib import Path
import sys

from run_truth import EXECUTED, HERE, MATRIX, load_jsonl, load_rows
from spelling_truth import describe_pair, load_spellings, pair_keys


def check_rows(rows_path, truth_path):
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
        if set(record) != {"id", "truth"} or not isinstance(record["id"], str):
            raise ValueError(f"{truth_path}: malformed row truth {record!r}")
        key = record["id"]
        if key in truth:
            raise ValueError(f"{truth_path}: duplicate truth for {key!r}")
        truth[key] = record["truth"]

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
        if truth[key] not in allowed:
            raise ValueError(f"{truth_path}: {key} has invalid truth {truth[key]!r}")
    print(f"{truth_path}: {len(truth)} row truths")


def check_spellings(directory, truth_path):
    expected = set(pair_keys(load_spellings(directory)))
    totals = Counter(key[0] for key in expected)
    nulls = Counter()
    seen = set()
    failed = False
    for record in load_jsonl(truth_path):
        fields = ("lang", "s1", "s2", "form")
        if (
            set(record) != set(fields) | {"same"}
            or any(not isinstance(record[field], str) for field in fields)
            or (record["same"] is not None and type(record["same"]) is not bool)
        ):
            raise ValueError(f"{truth_path}: malformed spelling truth {record!r}")
        key = tuple(record[field] for field in fields)
        if key not in expected:
            raise ValueError(f"{truth_path}: unexpected spelling pair {describe_pair(key)}")
        if key in seen:
            raise ValueError(f"{truth_path}: duplicate spelling pair {describe_pair(key)}")
        seen.add(key)
        if record["same"] is None:
            lang, left, right, _ = key
            nulls[lang] += 1
            print(f"inadmissible spelling pair {describe_pair(key)}", file=sys.stderr)
            if left == right:
                print(f"{truth_path}: identical spelling pair is inadmissible", file=sys.stderr)
                failed = True
    missing = expected - seen
    if missing:
        example = describe_pair(min(missing))
        raise ValueError(f"{truth_path}: {len(missing)} missing spelling pairs, including {example}")
    for lang in sorted(totals):
        print(f"{lang}: {nulls[lang]}/{totals[lang]} inadmissible spelling pairs", file=sys.stderr)
        if nulls[lang] * 100 > 2 * totals[lang]:
            print(f"{truth_path}: {lang} exceeds 2% inadmissible spelling pairs", file=sys.stderr)
            failed = True
    if failed:
        raise ValueError(f"{truth_path}: spelling truth gate failed")
    print(f"{truth_path}: {len(seen)} spelling truths")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("rows", type=Path, nargs="?", default=MATRIX / "rows.jsonl")
    parser.add_argument("truth", type=Path, nargs="?", default=HERE / "truth.jsonl")
    parser.add_argument("spelling_truth", type=Path, nargs="?", default=HERE / "spelling_truth.jsonl")
    parser.add_argument("--spellings", type=Path, default=MATRIX / "spellings")
    args = parser.parse_args()
    failed = False
    for check, arguments in (
        (check_rows, (args.rows, args.truth)),
        (check_spellings, (args.spellings, args.spelling_truth)),
    ):
        try:
            check(*arguments)
        except (OSError, ValueError) as error:
            print(error, file=sys.stderr)
            failed = True
    return int(failed)


if __name__ == "__main__":
    sys.exit(main())
