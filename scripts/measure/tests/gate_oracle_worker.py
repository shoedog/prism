"""A guarded fake Node executable for gate tests, using the real oracle job protocol."""

from __future__ import annotations

import json
from pathlib import Path
import sys

VERSION = "20.99.0"
VERDICTS = ("CORRECT", "WRONG", "UNDECIDED", "INADMISSIBLE")


def zeros():
    return dict.fromkeys(VERDICTS, 0)


def write(path, value):
    path.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")


def expected_rows(job):
    for case in job["cases"]:
        path = Path(case["root"]) / "expected.jsonl"
        yield from (json.loads(line) for line in path.read_bytes().splitlines() if line.strip())


def grade(job, stage):
    tuples, scopes, verdicts = {}, {}, []
    for row in expected_rows(job):
        if job["relation"] in (None, row["relation"]):
            tuples.setdefault(row["relation"], {}).setdefault(row["property"], zeros())
    denominators = job["denominators"]
    totals = zeros()
    for claim in job["claims"]:
        row, ix = claim["row"], claim["index"]
        relation, prop, label = row["fixture_grade"]
        assert job["relation"] in (None, relation)
        target = row["resolved_targets"][ix] if ix is not None else None
        confidence = target["confidence"] if target else row["confidence"]
        kind = target.get("kind") if target else None
        identity = (relation, prop, confidence, kind)
        scope = scopes.setdefault(identity, dict(relation=relation, property=prop,
                                                 confidence=confidence, kind=kind, graded=0, **zeros()))
        scope["graded"] += 1
        scope[label] += 1
        totals[label] += 1
        tuples[relation][prop][label] += 1
        if target:
            pop = denominators["populations"][confidence]
            bucket = pop["missing_kind"] if kind is None else pop["kinds"][kind]
            bucket["graded"] += 1
            bucket[label] += 1
            key = {"site": row["source_span"], "target": target["function_id"]}
        else:
            key = {}
            for end in ("from", "to"):
                where = row[end]
                key[end] = {name: where[name] for name in ("file", "line", "start_byte", "end_byte")}
                key[end].update(where["path"])
        verdict = dict(key=key, arm=claim["arm"], relation=relation, property=prop,
                       verdict=label, why="fixture", row=row)
        if target:
            verdict["target_index"] = ix
        verdicts.append(verdict)
    graded = len(verdicts)
    denominators.update(graded=graded, ungraded=denominators["selected"] - graded,
                        correct=totals["CORRECT"], wrong=totals["WRONG"],
                        inadmissible=totals["INADMISSIBLE"], decided=totals["CORRECT"] + totals["WRONG"],
                        scopes=list(scopes.values()))
    write(stage / "tuples.json", tuples)
    write(stage / "denominators.json", denominators)
    if not job["selection"].get("aggregate_only"):
        (stage / "verdicts.jsonl").write_text(
            "".join(json.dumps(row, sort_keys=True) + "\n" for row in verdicts), encoding="utf-8")
    return {"node": VERSION}


def main():
    if sys.argv[1:] == ["--version"]:
        print("v" + VERSION)
        return 0
    job = json.loads(Path(sys.argv[2]).read_bytes())
    stage = Path(sys.argv[3])
    if job["operation"] == "selfcheck":
        result = dict(cases=len(job["cases"]), expected=sum(1 for _ in expected_rows(job)), node=VERSION)
    else:
        assert job["operation"] == "grade"
        status = Path(sys.argv[0]).with_name("node-exit")
        if status.exists():
            return int(status.read_text())
        result = grade(job, stage)
    print(json.dumps(result))
    return 0
