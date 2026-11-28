"""Run real oracle Python admission/publication with a fake guarded Node worker."""

from __future__ import annotations

from contextlib import contextmanager
from copy import deepcopy
import json
import os
from pathlib import Path
from unittest.mock import patch

from .comparator_fixtures import edge
from .gate_fixtures import canonical, digest, jsonl
from .helpers import MEASURE, executable, sha256_of, write_tree
from .test_matrix_rows import CASES

LABELS = (("binding", "other"), ("binding", "slot"), ("lane.a", "slot.v1"), ("argfollow", "target"))


class OracleGateFixture:
    def __init__(self, bench):
        self.bench = bench
        self.ts = bench.work / "checker/lib/typescript.js"
        self.question = bench.work / "question/grade.cjs"
        write_tree(bench.work, {
            "checker/lib/typescript.js": "module.exports.version = '5.9.3';\n",
            "question/grade.cjs": """
module.exports = {
  relations: ['binding', 'lane.a', 'argfollow'],
  grade(row) {
    const [, property, verdict] = row.fixture_grade;
    return {property, verdict, why: 'fixture'};
  },
};
""",
        })
        case = MEASURE / "oracle/selfcheck/oracle-001"
        sample = json.loads((case / "input.jsonl").read_bytes())
        label = json.loads((case / "expected.jsonl").read_bytes())
        write_tree(self.question.parent / "selfcheck/labels", {
            "input.jsonl": jsonl({**sample, "relation": relation,
                                 "fixture_grade": [relation, prop, "CORRECT"]}
                                for relation, prop in LABELS),
            "expected.jsonl": jsonl({**label, "relation": relation, "property": prop}
                                   for relation, prop in LABELS),
            "repo/cases.ts": (case / "repo/cases.ts").read_bytes(),
        })
        worker = Path(__file__).with_name("gate_oracle_worker.py").resolve()
        executable(bench.work / "node", f"""
import runpy
raise SystemExit(runpy.run_path({str(worker)!r})["main"]())
""")

    def selfcheck(self):
        from measure import oracle

        result = self.bench.local_command(["--typescript", self.ts, "--question", self.question],
                                          command=oracle.selfcheck)
        self.bench.assert_exit(result, 0)

    def step(self, kind, **fields):
        return dict(id="grade", do=f"oracle {kind}", typescript=str(self.ts),
                    question=self.question.relative_to(self.bench.work).as_posix(), **fields)

    def edges(self, grades=(), **options):
        rows = [edge(sb=154 + index, fixture_grade=grade)
                for index, grade in enumerate(grades)]
        binary = self.bench.product("base", edges=rows)
        return self.bench.plan([self.bench.run_step(), self.step("ts-edges", run="base", all=True,
                                                                **options)], base=binary)

    def calls(self, categories=(), **options):
        probe = json.loads((CASES / "oracle-022/probe.jsonl").read_bytes())
        rows = []
        for index, (confidence, kind, label) in enumerate(categories):
            row = deepcopy(probe)
            row["source_span"].update(start_byte=370 + index * 8, end_byte=377 + index * 8)
            row["resolved_targets"][0].update(confidence=confidence, kind=kind)
            row["fixture_grade"] = ("argfollow", "target", label)
            rows.append(row)
        binary = self.bench.product("base", sites=rows)
        grade = self.step("ts-calls", census="base", relation="argfollow",
                          target_confidence=["exact", "name_only"],
                          target_kind=["argument_follow", "local_def"], **options)
        return self.bench.plan([self.bench.run_step(sites=True), grade], base=binary)


@contextmanager
def oracle_fixture(bench, *, selfcheck=True):
    from measure import oracle_inputs

    fixture = OracleGateFixture(bench)
    entry = "lib/typescript.js"
    checker_sha = sha256_of(fixture.ts)
    tree_sha = digest(canonical([[entry, checker_sha]]))
    with patch.object(oracle_inputs, "CHECKER", checker_sha), patch.object(oracle_inputs, "TREE", tree_sha):
        with patch.dict(bench.env, {"PATH": str(bench.work) + os.pathsep + bench.env["PATH"]}):
            if selfcheck:
                fixture.selfcheck()
                bench.guard_events.unlink()
            yield fixture
