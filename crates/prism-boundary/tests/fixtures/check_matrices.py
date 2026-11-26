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


def read_rows(root, header):
    previous = ""
    with (root / "rows.tsv").open(encoding="utf-8", newline="") as stream:
        reader = csv.reader(stream, delimiter="\t", strict=True)
        if tuple(next(reader, ())) != header:
            raise ValueError(f"{root.name}: wrong header (expected {len(header)} columns)")
        for cells in reader:
            label = f"{root.name}/rows.tsv:{reader.line_num}"
            if len(cells) != len(header):
                raise ValueError(f"{label}: expected {len(header)} columns, got {len(cells)}")
            row = dict(zip(header, cells))
            row_id = row["id"]
            if not row_id or row_id <= previous:
                raise ValueError(f"{label}: empty, duplicate or out-of-order row ID {row_id!r}")
            previous = row_id
            yield label, row


def check_matrix(name, ledger):
    root = ROOT / name
    header = HEADERS[name]
    classes = ledger["classes"]
    actual = {cls: {"rule": 0, "loss": 0} for cls in classes}
    controls = Counter()
    ids = set()
    for label, row in read_rows(root, header):
        row_id, truth, cls = row["id"], row["truth"], row["class"]
        ids.add(row_id)
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


def check_p1b_rows():
    root = ROOT / "p1b_rows"
    expectations = json.loads((root / "expectations.json").read_text(encoding="utf-8"))
    rows = {row["id"]: (label, row) for label, row in read_rows(root, HEADERS["ids_matrix"])}
    if rows.keys() != expectations.keys():
        raise ValueError("p1b_rows: TSV and expectation IDs differ")
    counts = Counter()
    used_controls = set()
    for row_id, (label, row) in rows.items():
        counts[row["class"]] += 1
        main = check_file(root, row["fixture"], label)
        directory = root / "fixtures" / row_id
        entry = expectations[row_id]
        inputs = entry["inputs"]
        if main.parent != directory or not inputs or directory / inputs[0]["fixture"] != main:
            raise ValueError(f"{label}: primary fixture must match the first expectation input")
        seen = set()
        for case in inputs:
            fixture = check_file(directory, case["fixture"], label)
            if fixture in seen:
                raise ValueError(f"{label}: duplicate fixture {fixture.name!r}")
            seen.add(fixture)
            if "media_proof" in case:
                check_file(directory, case["media_proof"], label)
            for reference in case.get("references", []):
                check_file(directory, reference["fixture"], label)
        if row["class"] == "known_limits":
            reason, control = entry["reason"], entry["control"]
            if (not re.fullmatch(r"[a-z][a-z0-9_]*", reason)
                or row["truth"] != f"loss:{reason}"
                or control != f"control-{row_id}"
                or rows[control][1]["class"] != "known_limits_control"):
                raise ValueError(f"{label}: missing reason or supported control")
            if row_id.startswith(("fold2-", "fold3-", "fold4-", "fold6-")) and (
                not entry["case"].startswith("KNOWN-LIMIT ")
                or not isinstance(entry.get("counted", True), bool)
            ):
                raise ValueError(f"{label}: missing KNOWN-LIMIT tag or invalid counted flag")
            used_controls.add(control)
        elif row["truth"] != "rule":
            raise ValueError(f"{label}: expected rule")
    controls = {row_id for row_id, (_, row) in rows.items() if row_id.startswith("control-")}
    if used_controls != controls:
        raise ValueError("p1b_rows: known_limits controls do not match their rows")
    fold = {row_id for row_id in rows if row_id.removeprefix("control-").startswith("fold1-")}
    fold_counts = Counter(rows[row_id][1]["class"] for row_id in fold)
    if fold_counts != {"fold1": 8, "known_limits": 3, "known_limits_control": 3}:
        raise ValueError(f"p1b_rows: fold1 class counts differ: {fold_counts}")
    fold2 = {row_id for row_id in rows if row_id.removeprefix("control-").startswith("fold2-")}
    fold2_counts = Counter(rows[row_id][1]["class"] for row_id in fold2)
    if fold2_counts != {"known_limits": 11, "known_limits_control": 11}:
        raise ValueError(f"p1b_rows: fold2 class counts differ: {fold2_counts}")
    fold3 = {row_id for row_id in rows if row_id.removeprefix("control-").startswith("fold3-")}
    fold3_counts = Counter(rows[row_id][1]["class"] for row_id in fold3)
    if fold3_counts != {"fold3": 7, "known_limits": 1, "known_limits_control": 1}:
        raise ValueError(f"p1b_rows: fold3 class counts differ: {fold3_counts}")
    fold4 = {row_id for row_id in rows if row_id.removeprefix("control-").startswith("fold4-")}
    fold4_counts = Counter(rows[row_id][1]["class"] for row_id in fold4)
    if fold4_counts != {"fold4": 5, "known_limits": 2, "known_limits_control": 2}:
        raise ValueError(f"p1b_rows: fold4 class counts differ: {fold4_counts}")
    fold6 = {row_id for row_id in rows if row_id.removeprefix("control-").startswith("fold6-")}
    fold6_counts = Counter(rows[row_id][1]["class"] for row_id in fold6)
    if fold6_counts != {"known_limits": 1, "known_limits_control": 1}:
        raise ValueError(f"p1b_rows: fold6 class counts differ: {fold6_counts}")
    previous = fold | fold2 | fold3 | fold4 | fold6
    new = len(rows.keys() - previous) - len(controls - previous)
    if new != 76:
        raise ValueError(f"p1b_rows: counted {new} NEW rows, expected 76")
    known_limits = counts["known_limits"] - sum(
        fold_count["known_limits"]
        for fold_count in (fold_counts, fold2_counts, fold3_counts, fold4_counts, fold6_counts)
    )
    print(f"p1b_rows: {new} NEW rows, "
          f"{known_limits} known_limits, "
          f"{len(controls - previous)} supported control rows — OK")
    print(f"p1b_rows: fold1 {fold_counts['fold1']} rule rows and "
          f"{fold_counts['known_limits']} known limits with controls — OK")
    print(f"p1b_rows: fold2 {fold2_counts['known_limits']} known limits with controls — OK")
    print(f"p1b_rows: fold3 {fold3_counts['fold3']} rule rows and "
          f"{fold3_counts['known_limits']} known limit with control — OK")
    print(f"p1b_rows: fold4 {fold4_counts['fold4']} rule rows and "
          f"{fold4_counts['known_limits']} known limits with controls — OK")
    print(f"p1b_rows: fold6 {fold6_counts['known_limits']} known limit with control — OK")


def check_p1c_rows():
    root = ROOT / "p1c_rows"
    fixtures = root / "fixtures"
    expectations = json.loads((root / "expectations.json").read_text(encoding="utf-8"))
    rows = {row["id"]: (label, row) for label, row in read_rows(root, HEADERS["ids_matrix"])}
    if rows.keys() != expectations.keys():
        raise ValueError("p1c_rows: TSV and expectation IDs differ")
    minimum = set("""
        o1 o2 o4 s1 s5 s6 s7 s9 s10 r2 r6 r7 r8 r9 r10
        f2 f4 t1 t2 t3 t11 e2 e3 e5 e6
        u1 u2 u3 u4 c1 c2 c4 c5 c6 c7 c8 c9 d1 d2
        p1 p2 p3 p4 p5 p6 n1 n2 n5 n8
    """.split())
    missing = minimum - rows.keys()
    if missing:
        raise ValueError(f"p1c_rows: missing minimum assertions {sorted(missing)}")
    counts = Counter()
    for row_id, (label, row) in rows.items():
        counts[row["class"]] += 1
        entry = expectations[row_id]
        inputs = entry["inputs"]
        if len(inputs) != 2 or not entry["clause"].startswith("DESIGN §"):
            raise ValueError(f"{label}: comparison requires two sides and a DESIGN clause")
        if check_file(root, row["fixture"], label) != check_file(fixtures, inputs[0]["fixture"], label):
            raise ValueError(f"{label}: primary fixture must match the before input")
        for side in inputs:
            check_file(fixtures, side["fixture"], label)
            if "media_proof" in side:
                check_file(fixtures, side["media_proof"], label)
            if row["class"] == "fold3":
                for reference in side.get("references", []):
                    check_file(fixtures, reference["fixture"], label)
        expected_ids = set()
        for change in entry["changes"]:
            for side in ("before", "after"):
                element, presence = change[side]
                if (element is not None) != (presence == "Present"):
                    raise ValueError(f"{label}: inconsistent {side} presence")
                expected_ids.update(value for value in (element, change.get(f"{side}_declaration"))
                                    if value is not None and not value.startswith("status:"))
                expected_ids.update(field for field, _ in change.get(f"{side}_fields", []))
        for pair in entry.get("pairs", []):
            expected_ids.add(pair["operation"])
            expected_ids.update(value for pair in pair["fields"] for value in pair)
        for limit in entry["limits"]:
            expected_ids.update(value for value, _ in limit.get("presence", [])
                                if value is not None and not value.startswith("status:"))
        if row["expected_ids"] != ";".join(sorted(expected_ids)):
            raise ValueError(f"{label}: TSV and side-local expectation IDs differ")
        reasons = sorted({limit["reason"] for limit in entry["limits"]})
        if row["expected_warnings"] != ";".join(reasons) or row["truth"] != "rule":
            raise ValueError(f"{label}: TSV and comparison expectation disagree")
        controls = entry.get("controls", [])
        if any(control not in rows or control == row_id or (
            not expectations[control]["changes"] and not (
                row["class"] == "fold1" and rows[control][1]["class"] == "fold1"
                and expectations[control]["limits"]
                and not expectations[control].get("subset_limits")
            )
        ) for control in controls):
            raise ValueError(f"{label}: missing independent edit control")
    fold1 = {row_id for row_id, (_, row) in rows.items() if row["class"] == "fold1"}
    if len(fold1) != 15 or any(not row_id.startswith("fold1-") for row_id in fold1):
        raise ValueError(f"p1c_rows: unexpected fold1 rows {sorted(fold1)}")
    fold2 = {row_id for row_id, (_, row) in rows.items() if row["class"] == "fold2"}
    if fold2 != {
        "fold2-item-type-missing-stop", "fold2-json-scalar-to-object",
        "fold2-server-document-checked", "fold2-unproved-scalar-to-object",
    }:
        raise ValueError(f"p1c_rows: unexpected fold2 rows {sorted(fold2)}")
    fold3 = {row_id for row_id, (_, row) in rows.items() if row["class"] == "fold3"}
    if len(fold3) != 70 or any(not row_id.startswith("fold3-") for row_id in fold3):
        raise ValueError(f"p1c_rows: unexpected fold3 rows {sorted(fold3)}")
    for row_id in fold3:
        entry = expectations[row_id]
        if (not entry.get("reverse") or entry.get("subset_limits")
                or (entry["limits"] and not entry.get("controls"))):
            raise ValueError(f"p1c_rows: {row_id} requires exact limits, a reverse run and an edit control for stops")
    variants = (sum(counts.values()) - counts["control"] - len(minimum)
                - len(fold1) - len(fold2) - len(fold3))
    print(f"p1c_rows: {counts['control']} controls, "
          f"{len(minimum)} minimum assertions and {variants} variants — OK")
    print(f"p1c_rows: {len(fold1)} fold1 rows — OK")
    print(f"p1c_rows: {len(fold2)} fold2 rows — OK")
    print(f"p1c_rows: {len(fold3)} fold3 rows — OK")


def check_p1d_rows():
    root = ROOT / "p1d_rows"
    expectations = {}
    for name in ("expected-admission.json", "expectations.json"):
        records = json.loads((root / name).read_text(encoding="utf-8"))
        if expectations.keys() & records.keys():
            raise ValueError(f"{name}: duplicate P1d row IDs")
        expectations.update(records)
    rows = {row["id"]: (label, row) for label, row in read_rows(root, HEADERS["ids_matrix"])}
    pinned_controls = {
        row["id"] for _, row in read_rows(ROOT / "ids_matrix", HEADERS["ids_matrix"])
        if row["id"].startswith("control-document-")
    }
    minimum_ranges = {
        "A": "01-16 18-19 23 27-28 31-36 38 41",
        "B": "01-02 04-08 10 12-13 17 19-20 22-24 27-29 32-33 35-39 41-42 46",
        "C": "01 03 05-18 21-22 24",
        "D": "01-14 16-18",
    }
    minimum = {
        f"{group}{number:02d}" for group, ranges in minimum_ranges.items()
        for token in ranges.split() for bounds in [token.split("-")]
        for number in range(int(bounds[0]), int(bounds[-1]) + 1)
    }
    expected = minimum | pinned_controls
    if rows.keys() != expected or expectations.keys() != expected:
        raise ValueError(f"p1d_rows: missing {sorted(expected - rows.keys())}, "
                         f"unexpected {sorted(rows.keys() - expected)}, "
                         f"JSON difference {sorted(rows.keys() ^ expectations.keys())}")
    consumer_rows = {
        row["id"] for _, row in read_rows(ROOT / "consumer_matrix", HEADERS["consumer_matrix"])
    }

    def check_fixture(name, label):
        if name == "formatted":
            return
        fixture_root = root / "fixtures"
        if name.startswith("p1c:"):
            fixture_root = ROOT / "p1c_rows" / "fixtures"
            name = name.removeprefix("p1c:")
        check_file(fixture_root, name, label)

    for row_id, (label, row) in rows.items():
        entry = expectations[row_id]
        if not entry["clause"].startswith(("DESIGN §", "Design answer")):
            raise ValueError(f"{label}: missing design clause")
        primary = entry["inputs"][0]["fixture"]
        if check_file(root, row["fixture"], label) != check_file(root / "fixtures", primary, label):
            raise ValueError(f"{label}: inconsistent primary fixture")
        if row["class"] != ("control" if row_id in pinned_controls else "minimum"):
            raise ValueError(f"{label}: inconsistent row class")
        if row["truth"] != "rule" or any(
            row[column] != ";".join(sorted(set(entry.get(key, []))))
            for column, key in [("expected_ids", "expected_ids"), ("expected_warnings", "expected_reasons")]
        ):
            raise ValueError(f"{label}: TSV and JSON expectations disagree")
        if any(control not in rows or control == row_id for control in entry.get("controls", [])):
            raise ValueError(f"{label}: invalid row control")
        if not set(entry.get("matrix_controls", ())) <= consumer_rows:
            raise ValueError(f"{label}: unknown consumer control")
        if row_id in pinned_controls and entry["pinned"] != row_id.removeprefix("control-document-"):
            raise ValueError(f"{label}: missing pinned projection reference")
        cases = check_file(root / "fixtures", entry["variants_fixture"], label)
        variants = json.loads(cases.read_text(encoding="utf-8"))[row_id]["variants"]
        if not variants:
            raise ValueError(f"{label}: no executable cases")
        if len({variant.get("name", "default") for variant in variants}) != len(variants):
            raise ValueError(f"{label}: duplicate variant name")
        for variant in variants:
            files = [variant[key] for key in ("fixture", "inventory", "selected") if key in variant]
            files.extend(variant.get("files", {}).values())
            files.extend(variant.get("diff", []))
            for name in files:
                check_fixture(name, label)
    print(f"p1d_rows: {len(pinned_controls)} controls and {len(minimum)} minimum assertions — OK")


def main():
    try:
        ledger = json.loads((ROOT / "ledger.json").read_text(encoding="utf-8"))
        for name in HEADERS:
            check_matrix(name, ledger["matrices"][name])
        check_p1b_rows()
        check_p1c_rows()
        check_p1d_rows()
    except (OSError, ValueError, KeyError, csv.Error) as exc:
        raise SystemExit(str(exc))


if __name__ == "__main__":
    main()
