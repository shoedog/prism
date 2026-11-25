#!/usr/bin/env python3
"""Compare pinned route expectations with saved executed truth. Read-only.

Compare declared methods per registration. Effective methods describe a served
route and can combine several builder legs. An omitted methods pin accepts any
observation. A runtime declaration of null is compatible with a missing pin or
Any, and disagrees with a Known pin. Explicit app.all also matches Any.
Unknown methods describe a static expression, leaving the runtime set unpinned.

Full and suffix pins constrain linked served paths, with exact and endswith
matching respectively. Extra exposures are unpinned. Dynamic classifications
and unserved suffixes with a static reachability reason are static only.
Lexical receiver identities, source span metadata, callback resolution status
and delegates, and the full mount listing also belong to the static oracle.

Pair by site before comparing other fields. Same-line route registrations
require the runner's raw call ordinal, which may count a builder's .route()
call. A source-text occurrence is a different field. A unique declared method
pin can identify a legacy same-line record for reporting its missing ordinal.
"""
import argparse
from collections import Counter, defaultdict
from dataclasses import dataclass, field
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
MISSING = object()
CALLBACK_FIELDS = ("position", "file", "name", "start_line")


@dataclass
class Difference:
    record: int
    field: str
    expected: object
    observed: object


@dataclass
class Comparison:
    differences: list = field(default_factory=list)
    checked: set = field(default_factory=set)
    static_only: set = field(default_factory=set)

    def check(self, record, field_name, expected, observed):
        self.checked.add(field_name)
        if expected != observed:
            self.differences.append(Difference(record, field_name, expected, observed))

    def pins(self, record, prefix, expected, observed, keys):
        for key in keys:
            if key in expected:
                self.check(record, f"{prefix}.{key}", expected[key], observed.get(key, MISSING))

    def static_pins(self, prefix, expected, keys):
        self.static_only.update(f"{prefix}.{key}" for key in expected if key not in keys)


def site_key(site):
    return site.get("file"), site.get("line")


def method_pin(value):
    if value == "any" or value == {"any": True} or value == {"status": "any"}:
        return {"any": True}
    return value


def pair_records(expected, observed, calls):
    """Use each observed registration at most once, in site order of priority."""
    pairs = {}
    remaining = set(range(len(observed)))

    def candidates(pin, exact):
        site = pin.get("site", {})
        for index in sorted(remaining):
            record = observed[index]
            actual = record["site"]
            if any(actual.get(key) != site[key] for key in ("file", "line") if key in site):
                continue
            ordinal = calls[record["call_id"]]["site"]["ordinal"]
            if exact and "ordinal" in site and site["ordinal"] != ordinal:
                continue
            yield index

    for exact in (True, False):
        for index, pin in enumerate(expected):
            if index in pairs:
                continue
            matches = list(candidates(pin, exact))
            if len(matches) > 1 and not exact and "methods" in pin:
                declared = method_pin(pin["methods"])
                matches = [m for m in matches if observed[m].get("methods") == declared]
            if len(matches) == 1:
                pairs[index] = matches[0]
                remaining.remove(matches[0])

    # One remaining pair can report bad site pins without guessing its identity.
    unmatched = set(range(len(expected))) - pairs.keys()
    if len(unmatched) == len(remaining) == 1:
        pairs[unmatched.pop()] = remaining.pop()
    return pairs, remaining


def compare_paths(result, index, pins, projected, served):
    normalized = {path["value"]: path.get("normalized") for path in projected}
    actual = list(dict.fromkeys(route["path"] for route in served if isinstance(route["path"], str)))
    for position, pin in enumerate(pins):
        prefix = f"paths[{position}]"
        status = pin["status"]
        if status == "dynamic":
            result.static_pins(prefix, pin, ())
            continue
        result.static_pins(prefix, pin, ("status", "value", "normalized"))
        if not actual and status == "suffix" and pin.get("reason") in {
            "no_known_mount", "mount_cycle", "expansion_truncated",
        }:
            result.static_pins(prefix, pin, ())
            continue
        if "value" not in pin:
            continue
        value = pin["value"]
        if status == "suffix":
            matches = [path for path in actual if path.endswith(value)]
        else:
            matches = [path for path in actual if path == value]
        result.check(index, f"{prefix}.value", value, value if matches else actual)
        if matches and "normalized" in pin:
            keys = [normalized[path] for path in matches if path in normalized]
            key = pin["normalized"]
            agrees = key in keys or (
                status == "suffix" and isinstance(key, str)
                and any(item is not None and item.endswith(key) for item in keys)
            )
            result.check(index, f"{prefix}.normalized", key, key if agrees else keys)


def compare_callbacks(result, index, pins, projected, callbacks):
    observed = {callback["position"]: callback for callback in projected}
    result.check(index, "callbacks.length", len(pins), len(callbacks))
    for position, pin in enumerate(pins):
        prefix = f"callbacks[{position}]"
        result.static_pins(prefix, pin, CALLBACK_FIELDS)
        actual = {}
        if position < len(callbacks):
            raw = callbacks[position]
            actual = {
                "position": position,
                **{key: raw[source] for key, source in
                   (("file", "file"), ("name", "name"), ("start_line", "first_line")) if source in raw},
                **observed.get(position, {}),
            }
        result.pins(index, prefix, pin, actual, CALLBACK_FIELDS)


def compare_row(row, truth):
    if truth["status"] != "executed":
        raise ValueError(f"{row['id']}: truth is {truth['status']}")
    result = Comparison()
    expected, observed = row["expect"], truth["projection"]
    calls = {call["id"]: call for call in truth["calls"]}
    served = defaultdict(list)
    for route in truth["served"]:
        for call_id in route["call_ids"]:
            served[call_id].append(route)
    pairs, extra = pair_records(expected, observed, calls)
    per_line = Counter(site_key(record["site"]) for record in observed)
    expected_per_line = Counter(site_key(pin.get("site", {})) for pin in expected)
    if row.get("expect_mounts"):
        result.static_only.add("expect_mounts")
    if "expect_unlisted" in row:
        result.static_only.add("expect_unlisted")
    result.checked.add("registrations")
    for index, pin in enumerate(expected):
        if index not in pairs:
            if pin.get("receiver", {}).get("status") == "unproven" and not pin.get("paths"):
                result.static_only.update(pin.keys() | {"registrations"})
            else:
                result.check(index, "registrations", pin.get("site", "registration"), MISSING)
            continue
        actual = observed[pairs[index]]
        call_id = actual["call_id"]
        call = calls[call_id]
        if "receiver" in pin:
            result.static_only.add("receiver")
        if "framework" in pin:
            result.check(index, "framework", pin["framework"], actual["framework"])
        if "site" in pin:
            site = pin["site"]
            result.static_pins("site", site, ("file", "line", "ordinal"))
            result.pins(index, "site", site, actual["site"], ("file", "line"))
            if ("ordinal" in site or per_line[site_key(actual["site"])] > 1
                    or expected_per_line[site_key(site)] > 1):
                result.check(index, "site.ordinal", site.get("ordinal", MISSING), call["site"]["ordinal"])
        if "methods" in pin:
            declared = method_pin(pin["methods"])
            if isinstance(declared, dict) and declared.get("status") == "unknown":
                result.static_only.add("methods")
            else:
                runtime = actual.get("methods")
                if runtime is None and declared == {"any": True}:
                    runtime = declared
                result.check(index, "methods", declared, runtime)
        if "path" in pin:
            result.pins(index, "path", pin["path"], actual.get("path", {}), ("raw", "literal"))
        if "paths" in pin:
            if not pin["paths"] and pin.get("receiver", {}).get("status") == "unproven":
                result.static_only.add("paths")
            compare_paths(result, index, pin["paths"], actual["paths"], served[call_id])
        if "callbacks" in pin:
            compare_callbacks(result, index, pin["callbacks"], actual.get("callbacks", []), call["callbacks"])
    for index in sorted(extra):
        actual = observed[index]
        result.check(None, "registrations", MISSING, calls[actual["call_id"]]["site"])
    return result


def read_jsonl(path):
    records = {}
    with path.open(encoding="utf-8") as stream:
        for line_number, line in enumerate(stream, 1):
            try:
                record = json.loads(line)
                row_id = record["id"]
                if not isinstance(row_id, str) or not row_id or row_id in records:
                    raise ValueError(f"missing or duplicate ID {row_id!r}")
                records[row_id] = line_number, record
            except (ValueError, KeyError, TypeError) as exc:
                raise ValueError(f"{path}:{line_number}: {exc}") from exc
    return records


def display(value):
    return "<absent>" if value is MISSING else json.dumps(value, ensure_ascii=False)


def group(field_name):
    return field_name.split("[", 1)[0].split(".", 1)[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rows", type=Path, default=HERE.parent / "rows.jsonl")
    parser.add_argument("--truth", type=Path, default=HERE / "truth.jsonl")
    args = parser.parse_args()
    try:
        rows, truth = read_jsonl(args.rows), read_jsonl(args.truth)
        if truth.keys() - rows.keys():
            raise ValueError(f"unknown IDs in truth: {', '.join(sorted(truth.keys() - rows.keys()))}")
        return report(rows, truth)
    except (OSError, ValueError, KeyError, TypeError) as exc:
        parser.error(str(exc))


def report(rows, truth):
    totals, checked, changed = Counter(), Counter(), Counter()
    for row_id, (_, row) in rows.items():
        line_number, entry = truth.get(row_id, (None, {}))
        status = entry.get("status")
        if row["truth_source"] == "hand":
            totals["hand"] += 1
            if entry and status != "hand_required":
                print(f"{row_id}: unexpected truth for hand row at line {line_number} ({status})")
                totals["unavailable"] += 1
            elif entry:
                print(f"{row_id}: hand, truth line {line_number} refused: {entry['error']}")
            continue
        if row["truth_source"] != "executed":
            raise ValueError(f"{row_id}: invalid truth_source {row['truth_source']!r}")
        if status != "executed":
            problem = f"truth line {line_number}, {status}: {entry.get('error')}" if entry else "missing truth"
            print(f"{row_id}: {problem}")
            totals["unavailable"] += 1
            continue
        result = compare_row(row, entry)
        disposition = "differs" if result.differences else "agrees"
        totals[disposition] += 1
        print(f"{row_id} (truth line {line_number}): {disposition}")
        for diff in result.differences:
            record = f"expect[{diff.record}]." if diff.record is not None else ""
            print(f"  {record}{diff.field}: {display(diff.expected)} -> {display(diff.observed)}")
        print(f"  static only: {', '.join(sorted(result.static_only)) or 'none'}")
        checked.update({group(name) for name in result.checked})
        changed.update({group(diff.field) for diff in result.differences})
    print(f"Rows: {totals['agrees']} agree, {totals['differs']} differ, "
          f"{totals['unavailable']} unavailable, {totals['hand']} hand")
    print("Field                    Agree  Differ")
    for name in sorted(checked):
        print(f"{name:24} {checked[name] - changed[name]:5} {changed[name]:7}")
    return int(bool(totals["differs"] or totals["unavailable"]))


if __name__ == "__main__":
    raise SystemExit(main())
