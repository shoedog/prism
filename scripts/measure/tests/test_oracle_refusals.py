"""Paired controls for S3 closure, selector, and receipt refusals."""

from __future__ import annotations

import copy
import json
import os
import platform
import shutil
import unittest
from collections import Counter
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

from measure import oracle, oracle_inputs, oracle_selection
from measure.corpora import load_manifest
from measure.edges import canonical, decode_edge

from .helpers import MEASURE, write_tree
from . import test_oracle as fixtures
from .test_oracle_guards import argparse_namespace


class BindingRefusals(unittest.TestCase):
    def setUp(self):
        work = TemporaryDirectory(prefix=".s3-bindings-", dir=MEASURE.parents[1])
        self.addCleanup(work.cleanup)
        self.root = Path(work.name)

    def test_qualification_bundle_requires_bound_form_evidence(self):
        binary = "f" * 64
        raw, = fixtures.read_jsonl(fixtures.CASES / "oracle-022/probe.jsonl")
        point = fixtures.synthetic_evidence(binary, "named-function", raw)
        blob = canonical(point)
        bundle = dict(schema="prism.measure.qualification/1", builds={
            binary: {"named-function": dict(path="evidence.json", sha256=oracle_inputs.digest(blob))}
        })

        def check(value, data=blob):
            write_tree(self.root, {"bundle.json": canonical(value), "evidence.json": data})
            return oracle_inputs.qualifications(self.root)[0]

        self.assertEqual(oracle_inputs.qualifications(self.root / "absent"), ({}, None, {}))
        self.assertEqual(check(bundle), {binary: ["named-function"]})
        for reason, change in (
            ("invalid qualification bundle", lambda v: v.update(extra=True)),
            ("invalid qualification build or form", lambda v: v["builds"].update(bad={})),
            ("unbound qualification evidence",
             lambda v: v["builds"][binary]["named-function"].update(path="../evidence.json")),
            ("unbound qualification evidence",
             lambda v: v["builds"][binary]["named-function"].update(sha256="0" * 64)),
        ):
            with self.subTest(reason=reason):
                bad = copy.deepcopy(bundle)
                change(bad)
                with self.assertRaisesRegex(ValueError, reason):
                    check(bad)
        changed = canonical(dict(point, form="named-method"))
        bad = copy.deepcopy(bundle)
        bad["builds"][binary]["named-function"]["sha256"] = oracle_inputs.digest(changed)
        with self.assertRaisesRegex(ValueError, "does not establish the form"):
            check(bad, changed)
        (self.root / "foreign.json").write_text("{}")
        with self.assertRaisesRegex(ValueError, "unlisted qualification evidence"):
            check(bundle)

    def test_change_wire_refuses_each_invalid_nested_record(self):
        row, = fixtures.read_jsonl(fixtures.CASES / "oracle-001/input.jsonl")
        key, attrs, _ = decode_edge(row, line_mode=False)
        change = dict(id=1, key=json.loads(key), kind="added", count=1, base_bag=[],
                      head_bag=[json.loads(attrs)], records=[], error=None, **{"class": "known"})
        self.assertEqual(oracle_selection.validate_change(change)[1]["head"], Counter({attrs: 1}))
        for reason, mutate in (
            ("invalid change", lambda v: v.update(count=2)),
            ("bag incompatible", lambda v: v.update(kind="removed")),
            ("invalid key", lambda v: v["key"].update(extra={})),
            ("invalid projected endpoint", lambda v: v["key"]["from"].update(start_byte=-1)),
            ("invalid bag attribute", lambda v: v["head_bag"][0].update(from_access="read")),
        ):
            with self.subTest(reason=reason):
                bad = copy.deepcopy(change)
                mutate(bad)
                with self.assertRaisesRegex(ValueError, reason):
                    oracle_selection.validate_change(bad)

    def test_selfcheck_fixture_set_requires_regular_cases_and_wire_rows(self):
        case = fixtures.KIT / "selfcheck/oracle-001"
        data = {f"case/{file}": (case / file).read_bytes()
                for file in ("input.jsonl", "expected.jsonl", "repo/cases.ts")}
        write_tree(self.root, data)
        parse = oracle_inputs.fixture_set
        self.assertEqual([entry["id"] for entry in parse(self.root, data, "lane")], ["lane/case"])
        for bad, reason in (
            ({}, "has no cases"),
            (dict(data, orphan=b"TRUE"), "inside a case directory"),
            ({key: value for key, value in data.items() if not key.endswith("expected.jsonl")},
             "requires input and expected"),
            (dict(data, **{"case/input.jsonl": b'{"relation":"x","arm":"census","row":false}\n'}),
             "expected wire object"),
            (dict(data, **{"case/input.jsonl": b'{"record_kind":"go_level3_b1_candidate"}\n'}),
             "expected call_site or edge"),
            (dict(data, **{"case/input.jsonl": b'{"from":null,"from":null}\n'}),
             "duplicate JSON field"),
        ):
            with self.subTest(reason=reason), self.assertRaisesRegex(ValueError, reason):
                parse(self.root, bad, "lane")
        (self.root / "case/repo").rename(self.root / "moved")
        with self.assertRaisesRegex(ValueError, "requires repo directory"):
            parse(self.root, data, "lane")

    def test_question_tree_refuses_invalid_root_member_and_directory(self):
        root = self.root / "closure"
        write_tree(root, {"file.cjs": "module.exports = {};"})
        self.assertEqual(oracle_inputs.file_tree(root), {"file.cjs": b"module.exports = {};"})
        with self.assertRaisesRegex(ValueError, "closure root must be a regular directory"):
            oracle_inputs.file_tree(root / "file.cjs")
        alias = root / "alias"
        alias.symlink_to(self.root, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "linked or special directory"):
            oracle_inputs.file_tree(root)
        alias.unlink()
        (root / ".git").write_bytes(b"")
        with self.assertRaisesRegex(ValueError, "unsupported closure path"):
            oracle_inputs.file_tree(root)


class OracleRefusals(unittest.TestCase):
    def setUp(self):
        self.h = fixtures.OracleTests()
        self.addCleanup(self.h.doCleanups)
        self.h.setUp()

    def check_selfcheck(self, question, expected, reason=None):
        result = self.h.local_command(oracle.selfcheck, ["--typescript", self.h.typescript(),
                                                         "--question", question])
        self.h.assert_exit(result, expected)
        if reason:
            self.assertIn(reason, result.stderr)

    def test_selfcheck_refuses_mismatched_cardinality_coverage_and_shapes(self):
        question = self.h.question(dict(verdict="CORRECT", property="custom", why="proof"))
        case = question.parent / "selfcheck/oracle-001"
        golden = case / "expected.jsonl"
        input_file = case / "input.jsonl"
        expected, = fixtures.read_jsonl(golden)
        original = golden.read_bytes()
        raw, = fixtures.read_jsonl(input_file)
        original_input = input_file.read_bytes()
        self.check_selfcheck(question, 0)
        for rows, reason in (
            ([], "selfcheck-cardinality"),
            ([dict(expected, extra=True)], "expected row"),
            ([dict(expected, relation="undeclared")], "undeclared relation"),
            ([dict(expected, property="unchecked")], "need all verdicts"),
        ):
            with self.subTest(reason=reason):
                golden.write_text(fixtures.json_lines(rows))
                self.check_selfcheck(question, 1, reason)
        golden.write_bytes(original)
        text = question.read_text()
        question.write_text(text.replace("['custom']", "['custom','never']"))
        self.check_selfcheck(question, 1, "untested relation")
        question.write_text(text)
        envelope = dict(relation="custom", arm="census", row=raw)
        key, attrs, _ = decode_edge(raw, line_mode=False)
        change = dict(id=1, key=json.loads(key), kind="added", count=1, base_bag=[],
                      head_bag=[json.loads(attrs)], records=[], error=None, **{"class": None})
        for row, reason in (
            (dict(envelope, extra=True), "invalid input wrapper"),
            (dict(envelope, arm="third"), "invalid input wrapper"),
            (dict(envelope, target_index=0), "target index"),
            (dict(envelope, change=change), "change context"),
        ):
            with self.subTest(reason=reason):
                input_file.write_text(fixtures.json_lines([row]))
                self.check_selfcheck(question, 1, reason)
        input_file.write_bytes(original_input)
        self.check_selfcheck(question, 0)

    def test_question_requires_only_bound_commonjs_members_and_closed_exports(self):
        question = self.h.question(dict(verdict="CORRECT", property="custom", why="proof"))
        original = question.read_text()
        write_tree(question.parent, {"helper.cjs": "module.exports = true;", "other.mjs": "export {};"})
        for request, reason in (
            ("../helper.cjs", "path escape"),
            ("external-package", "external or ambient module"),
            ("./missing", "outside admitted file set"),
            ("./other.mjs", "unadmitted CommonJS member"),
            (4, "expected a module name"),
        ):
            with self.subTest(request=request):
                question.write_text(f"require({json.dumps(request)}); " + original)
                self.check_selfcheck(question, 1, reason)
        question.write_text("require('node:fs').readFileSync(4); " + original)
        self.check_selfcheck(question, 1, "expected a data path")
        question.write_text(original.replace("['custom']", "['custom','custom']"))
        self.check_selfcheck(question, 1, "question-interface")
        question.write_text("require('./' + 'helper'); " + original)
        self.check_selfcheck(question, 0)

    def test_question_paths_and_publication_recheck_are_bound(self):
        args = self.h.load_case("oracle-001")
        question = self.h.question(dict(verdict="CORRECT", property="custom", why="proof"))
        ts = self.h.prepare(question=question)
        with patch.dict(os.environ, self.h.env, clear=True):
            before = oracle_inputs.prepare_inputs(ts, question)
            oracle.recheck(before, ts, question)
            alternate = question.with_suffix(".mjs")
            alternate.write_bytes(question.read_bytes())
            with self.assertRaisesRegex(oracle_inputs.OracleError, "regular CommonJS entry"):
                oracle_inputs.prepare_inputs(ts, alternate)
            alternate.unlink()
            with patch.dict(os.environ, PRISM_MEASURE_SELFCHECK_DIR=str(question.parent / "cache")):
                with self.assertRaisesRegex(oracle_inputs.OracleError, "cache must be outside"):
                    oracle_inputs.prepare_inputs(ts, question)
            alternate = question.parent / "unused.cjs"
            alternate.write_text("module.exports = {};")
            with self.assertRaisesRegex(oracle_inputs.OracleError, "input bytes changed"):
                oracle.recheck(before, ts, question)
            alternate.unlink()
            alternate.symlink_to(question)
            with self.assertRaisesRegex(oracle_inputs.OracleError, "checker-or-question-drift: selfcheck-binding"):
                oracle.recheck(before, ts, question)
            alternate.unlink()
            oracle.recheck(before, ts, question)
        for flags, reason in (
            ({"out": question.parent / "output"}, "output must be outside"),
            ({"relation": "undeclared"}, "unknown relation"),
        ):
            result = self.h.oracle(args, question=question, **flags)
            self.h.assert_exit(result, 2)
            self.assertIn(reason, result.stderr)
        self.h.assert_exit(self.h.oracle(args, question=question, relation="custom"), 0)

    def test_missing_or_unbound_snapshot_refuses_with_restored_control(self):
        args = self.h.load_case("oracle-001")
        directory = args[-1]
        capture = json.loads((directory / "receipt.json").read_bytes())
        root = Path(capture["corpus"]["root"])
        moved = root.with_name("moved-source")
        root.rename(moved)
        result = self.h.oracle(args, typescript=Path("unused"))
        self.h.assert_exit(result, 2)
        self.assertIn("snapshot unavailable", result.stderr)
        moved.rename(root)
        link = root / "outside"
        link.symlink_to("../../../outside")
        result = self.h.oracle(args, typescript=Path("unused"))
        self.h.assert_exit(result, 2)
        self.assertIn("unbound-link", result.stderr)
        link.unlink()
        self.h.prepare()
        self.h.assert_exit(self.h.oracle(args), 0)

    def test_recorded_runtime_and_row_count_are_required(self):
        args = self.h.load_case("oracle-001")
        self.h.prepare()
        path, = (self.h.work / "selfcheck").glob("selfcheck-*.json")
        receipt = json.loads(path.read_bytes())
        control = self.h.oracle(args, out=self.h.work / "control")
        self.h.assert_exit(control, 0)
        for changes, exit_code, reason in (
            ({"runtime": dict(receipt["runtime"], python="wrong")}, 3, "Python runtime"),
            ({"runtime": dict(receipt["runtime"], node="wrong")}, 1, "Node runtime"),
            ({"expected": receipt["expected"] + 1, "agreed": receipt["agreed"] + 1}, 1, "row count"),
            ({"runtime": dict(receipt["runtime"], extra="unknown")}, 3, "missing or stale"),
        ):
            with self.subTest(reason=reason):
                self.assertEqual(receipt["runtime"]["python"], platform.python_version())
                path.write_bytes(canonical(dict(receipt, **changes)))
                result = self.h.oracle(args)
                self.h.assert_exit(result, exit_code)
                self.assertIn(reason, result.stderr)
                self.assertEqual(list(self.h.out.iterdir()), [])

    def test_selector_and_output_refusals_precede_grading(self):
        edge_args = self.h.load_case("oracle-001")
        edge = edge_args[-1]
        occupied = self.h.work / "occupied"
        write_tree(occupied, {"keep": "sentinel"})
        for args, flags, reason in (
            (edge_args, {"side": "head"}, "require --changes"),
            (edge_args, {"relation": "unknown"}, "unknown built-in relation"),
            (edge_args, {"relation": " "}, "invalid relation"),
            (edge_args, {"out": occupied}, "not empty"),
            (("ts-calls", "--base", edge), {}, "census or both arms"),
            (("ts-calls", "--census", edge, "--target-kind", " "), {}, "invalid target kind"),
            (("ts-calls", "--census", edge, "--target-confidence", "name_only"), {}, "requires question"),
            (("ts-calls", "--census", edge), {}, "missing sites artifact"),
        ):
            with self.subTest(reason=reason):
                result = self.h.oracle(args, typescript=Path("unused"), **flags)
                self.h.assert_exit(result, 2)
                self.assertIn(reason, result.stderr)
        self.assertEqual((occupied / "keep").read_text(), "sentinel")
        self.h.prepare()
        self.h.assert_exit(self.h.oracle(edge_args), 0)
        call_rows = fixtures.read_jsonl(fixtures.CASES / "oracle-022/input.jsonl")
        sites = self.h.make_run(self.h.work / "sites", call_rows, sites=True)
        entries = load_manifest(self.h.manifest)
        args = argparse_namespace(("ts-calls", "--census", sites))
        self.assertEqual(len(oracle_selection.call_population(args, entries)[2]), 1)

    def test_certificate_and_residual_selectors_refuse_changed_bindings(self):
        source = fixtures.CASES / "oracle-001"
        self.h.fresh_tree(files={"cases.ts": (source / "repo/cases.ts").read_bytes()})
        row, = fixtures.read_jsonl(source / "input.jsonl")
        left = self.h.make_run(self.h.work / "left", [])
        right = self.h.make_run(self.h.work / "right", [row])
        classifier = self.h.work / "classifier.py"
        classifier.write_text("CLASSES = ['bound']\ndef classify(change, context): return 'bound'\n")
        out = self.h.work / "diff"
        self.h.assert_exit(self.h.kit("diff-edges", left, right, "--classifier", classifier, "--out", out), 0)
        stream = out / "changes.jsonl"
        path = out / "certificate.json"
        original = stream.read_bytes()
        certificate = json.loads(path.read_bytes())
        args = ("ts-edges", "--run", right, "--changes", stream)
        entries = load_manifest(self.h.manifest)

        def select(options=args, gate=None):
            return oracle_selection.edge_population(argparse_namespace(options), entries, gate)[3]

        self.assertEqual([claim["row"] for claim in select()], [row])
        for options, gate, reason in (
            ((*args, "--side", "base"), None, "selected arm"),
            ((*args, "--class", "unknown"), None, "unknown class"),
            (args, "0" * 64, "gate binding differs"),
        ):
            with self.subTest(reason=reason), self.assertRaisesRegex(ValueError, reason):
                select(options, gate)
        for reason, mutate in (
            ("invalid shape", lambda v: v.update(extra=True)),
            ("invalid run reference", lambda v: v["inputs"]["head"].update(run=str(right))),
            ("receipt digest differs", lambda v: v["inputs"]["head"].update(receipt_sha256="0" * 64)),
        ):
            with self.subTest(reason=reason):
                changed = copy.deepcopy(certificate)
                mutate(changed)
                path.write_bytes(canonical(changed))
                with self.assertRaisesRegex(ValueError, reason):
                    select()
        path.write_bytes(canonical(certificate))
        stream.write_bytes(original + b"\n")
        with self.assertRaisesRegex(ValueError, "changes digest differs"):
            select()
        change = json.loads(original)
        for variant, reason in (
            (dict(change, id=2), "invalid change id"),
            (dict(change, count=2, head_bag=change["head_bag"] * 2), "residual requests absent"),
        ):
            data = canonical(variant) + b"\n"
            stream.write_bytes(data)
            path.write_bytes(canonical(dict(certificate, changes_sha256=oracle_inputs.digest(data))))
            with self.assertRaisesRegex(ValueError, reason):
                select()
        stream.write_bytes(original)
        path.write_bytes(canonical(certificate))
        self.assertEqual([claim["row"] for claim in select()], [row])

    def test_realuse_authored_claims_grade_against_source(self):
        root = fixtures.KIT / "realuse"
        expected = fixtures.read_jsonl(root / "truth.jsonl")
        self.assertGreaterEqual(sum(row["verdict"] == "CORRECT" for row in expected), 10)
        self.h.fresh_tree(files={"shapes.ts": (root / "repo/shapes.ts").read_bytes()})

        def endpoint(point, access):
            return {key: point[key] for key in ("file", "line", "start_byte", "end_byte")} | {
                "access": access, "path": {"base": point["base"], "fields": point["fields"]}
            }

        rows = []
        for item in expected:
            origin, destination = ("use", "def") if item["relation"] == "argument_binding" else ("def", "use")
            rows.append({"from": endpoint(item["key"]["from"], origin),
                         "to": endpoint(item["key"]["to"], destination),
                         "confidence": "exact", "doubt": None})
        run = self.h.make_run(self.h.work / "realuse", rows)
        self.h.prepare()
        self.h.assert_exit(self.h.oracle(("ts-edges", "--all", "--run", run)), 0)
        actual = fixtures.read_jsonl(self.h.out / "verdicts.jsonl")
        fields = ("key", "relation", "property", "verdict")
        self.assertEqual(Counter(canonical({key: row[key] for key in fields}) for row in actual),
                         Counter(canonical(row) for row in expected))

    def test_checker_entry_pin_and_tree_pin_are_independent(self):
        original = self.h.typescript()
        copied = self.h.work / "typescript"
        shutil.copytree(original.parent.parent, copied)
        entry = copied / "lib/typescript.js"
        control = oracle_inputs.checker_files(entry)
        self.assertEqual(control["files"]["lib/typescript.js"], oracle_inputs.CHECKER)
        with self.assertRaisesRegex(oracle_inputs.OracleError, "expected lib/typescript.js"):
            oracle_inputs.checker_files(entry.with_suffix(".cjs"))
        alias = self.h.work / "alias"
        alias.symlink_to(copied, target_is_directory=True)
        with self.assertRaisesRegex(oracle_inputs.OracleError, "checker path traverses an alias"):
            oracle_inputs.checker_files(alias / "lib/typescript.js")
        extra = copied / "entrée.txt"
        extra.write_text("added")
        with self.assertRaisesRegex(oracle_inputs.OracleError, "unsupported checker path"):
            oracle_inputs.checker_files(entry)
        extra.unlink()
        extra = copied / "unbound.txt"
        extra.write_text("added")
        with self.assertRaisesRegex(oracle_inputs.OracleError, "tree digest differs"):
            oracle_inputs.checker_files(entry)
        extra.unlink()
        self.assertEqual(oracle_inputs.checker_files(entry), control)

    def test_qualification_requires_indexed_site_and_matching_controls(self):
        from measure import oracle_qualification

        case = fixtures.KIT / "selfcheck/oracle-058-decorated"
        data = (case / "repo/cases.ts").read_bytes()
        sites = [row for row in fixtures.read_jsonl(case / "input.jsonl") if row.get("record_kind") == "call_site"]
        good = next(row for row in sites if row["resolved_targets"][0]["function_id"]["start_line"] == 3)
        wrong = next(row for row in sites if row["resolved_targets"][0]["function_id"]["start_line"] == 2)
        self.h.source = self.h.work / "qualification-source"
        self.h.make_corpus(files={"cases.ts": data})
        run = self.h.make_run(self.h.work / "site-capture",
                              [good, wrong, {"record_kind": "go_level3_b1_candidate"}], sites=True)
        capture = json.loads((run / "receipt.json").read_bytes())
        source_file = Path(capture["corpus"]["root"]) / "cases.ts"
        start = data.index(b"static m") + len(b"static ")
        label = dict(run_index=0, record_ordinal=0, target_index=0, form="decorated-method",
                     declaration=dict(file="cases.ts", name="m", start_byte=start, end_byte=start + 1, line=3),
                     contradiction_line=2)
        out = self.h.work / "qualifications"
        ts = self.h.typescript()
        with patch.dict(os.environ, self.h.env | {"PRISM_TYPESCRIPT": str(ts)}, clear=True):
            for bad, reason in (
                ([], "no controls"),
                ([dict(label, run_index=1)], "invalid index or form"),
                ([dict(label, contradiction_line=3)], "invalid source label"),
                ([dict(label, record_ordinal=2)], "expected a call_site"),
                ([dict(label, record_ordinal=3)], "record index unavailable"),
                ([dict(label, target_index=1)], "expected exact indexed target"),
                ([label, label], "duplicate build/form"),
                ([dict(label, declaration=dict(label["declaration"], file="other.ts"))], "label source missing"),
                ([dict(label, record_ordinal=1)], "serializer claim or changed-line control disagrees"),
            ):
                with self.subTest(reason=reason), self.assertRaisesRegex(ValueError, reason):
                    oracle.build_qualification([run], bad, out)
                self.assertFalse((out / "bundle.json").exists())
            source_file.write_bytes(data + b"\n")
            with self.assertRaisesRegex(ValueError, "qualification-source: snapshot differs"):
                oracle.build_qualification([run], [label], out)
            source_file.write_bytes(data)
            with patch.object(oracle_qualification, "regular", return_value=data + b"\n"):
                with self.assertRaisesRegex(ValueError, "label source drift"):
                    oracle.build_qualification([run], [label], out)
            with patch.object(oracle_qualification, "launch", return_value={"qualified": 0}):
                with self.assertRaisesRegex(ValueError, "incomplete controls"):
                    oracle.build_qualification([run], [label], out)

            def change_snapshot_after_controls(request, stage):
                result = oracle.launch(request, stage)
                source_file.write_bytes(data + b"\n")
                return result

            with patch.object(oracle_qualification, "launch", side_effect=change_snapshot_after_controls):
                with self.assertRaisesRegex(ValueError, "qualification source changed"):
                    oracle.build_qualification([run], [label], out)
            source_file.write_bytes(data)
            self.assertEqual(list(out.iterdir()), [])
            digest = oracle.build_qualification([run], [label], out)
        builds, digest_read, evidence = oracle_inputs.qualifications(out)
        self.assertEqual(builds, {capture["binary"]["sha256"]: ["decorated-method"]})
        self.assertEqual(digest_read, digest)
        bound = json.loads(evidence["evidence-1.json"])
        self.assertEqual(bound["label"], label)
        self.assertEqual(bound["raw"], good)
        self.assertEqual(bound["source_file_sha256"], oracle_inputs.digest(data))
        self.assertEqual(bound["sites_sha256"], capture["artifacts"]["sites.jsonl.gz"]["sha256"])
