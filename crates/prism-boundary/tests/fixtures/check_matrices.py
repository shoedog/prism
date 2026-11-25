#!/usr/bin/env python3
"""Check matrix references and the pinned stage D class counts."""

import csv
import json
import re
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent
HEADERS = {
    "ids_matrix": ("id", "fixture", "expected_ids", "expected_warnings", "truth", "class"),
    "consumer_matrix": (
        "id", "fixture", "config", "expected_grade", "expected_ids",
        "expected_reason", "truth", "class",
    ),
}


def check_file(root, reference, label):
    path_text, _, line = reference.partition(":")
    relative = Path(path_text)
    path = root / relative
    if not path_text or relative.is_absolute() or ".." in relative.parts or not path.is_file():
        raise ValueError(f"{label}: missing or invalid file {reference!r}")
    if ":" in reference:
        with path.open(encoding="utf-8") as stream:
            line_count = sum(1 for _ in stream)
        if not line.isdecimal() or not 1 <= int(line) <= line_count:
            raise ValueError(f"{label}: invalid source line {reference!r}")
    return path


def check_matrix(name, ledger):
    root = ROOT / name
    header = HEADERS[name]
    classes = ledger["classes"]
    actual = {cls: {"rule": 0, "loss": 0} for cls in classes}
    controls = Counter()
    ids = set()
    previous = ""
    with (root / "rows.tsv").open(encoding="utf-8", newline="") as stream:
        reader = csv.reader(stream, delimiter="\t", strict=True)
        if tuple(next(reader, ())) != header:
            raise ValueError(f"{name}: wrong header (expected {len(header)} columns)")
        for cells in reader:
            label = f"{name}/rows.tsv:{reader.line_num}"
            if len(cells) != len(header):
                raise ValueError(f"{label}: expected {len(header)} columns, got {len(cells)}")
            row = dict(zip(header, cells))
            row_id, truth, cls = row["id"], row["truth"], row["class"]
            if not row_id or row_id in ids:
                raise ValueError(f"{label}: empty or duplicate row ID {row_id!r}")
            if row_id < previous:
                raise ValueError(f"{label}: {row_id!r} must sort before {previous!r}")
            ids.add(row_id)
            previous = row_id

            fixture_root = root / "fixtures"
            path_root = root if name == "ids_matrix" else fixture_root
            fixture = check_file(path_root, row["fixture"], label)
            config = check_file(path_root, row["config"], label) if "config" in row else None
            if cls not in classes:
                raise ValueError(f"{label}: unknown ledger class {cls!r}")
            if truth != "rule" and not re.fullmatch(r"loss:[a-z][a-z0-9_]*", truth):
                raise ValueError(f"{label}: invalid truth {truth!r}")
            if row_id.startswith("control-"):
                directory = fixture_root / row_id
                if truth != "rule" or directory not in fixture.parents or (
                    config is not None and config.parent != directory
                ):
                    raise ValueError(f"{label}: control must be a rule with files under {directory.name}/")
                controls[cls] += 1
            else:
                actual[cls][truth.split(":")[0]] += 1

    mismatches = [
        f"{cls}: got {actual[cls]}, expected {counts}"
        for cls, counts in classes.items()
        if actual[cls] != counts
    ]
    if mismatches:
        raise ValueError(f"{name}: ledger mismatch\n" + "\n".join(mismatches))
    totals = {kind: sum(counts[kind] for counts in actual.values()) for kind in ("rule", "loss")}
    if totals != ledger["totals"]:
        raise ValueError(f"{name}: class counts sum to {totals}, expected {ledger['totals']}")
    print(
        f"{name}: {len(ids)} rows, {len(classes)} classes, "
        f"{totals['rule']} rule / {totals['loss']} loss pinned, "
        f"{sum(controls.values())} controls — OK"
    )


def main():
    try:
        ledger = json.loads((ROOT / "ledger.json").read_text(encoding="utf-8"))
        for name in HEADERS:
            check_matrix(name, ledger["matrices"][name])
    except (OSError, ValueError, KeyError, csv.Error) as exc:
        raise SystemExit(str(exc))


if __name__ == "__main__":
    main()
