"""S3 admission and behavior through the public entry points."""

from __future__ import annotations

from collections import Counter
from contextlib import redirect_stderr, redirect_stdout
import copy
import hashlib
import io
import json
import os
import platform
from pathlib import Path
import shutil
from unittest.mock import patch

from .helpers import DEFAULT_VERSION, MEASURE, Workbench, executable, write_tree
from .test_matrix_rows import CASES, MATRIX, read_jsonl as iter_jsonl


def read_jsonl(path):
    return list(iter_jsonl(path))


KIT = MEASURE / "oracle"
TREE_PIN = "7e02162c902e5c29ec19fefc573ad55b8ed15fcbda940e737f297c53e9c41f54"
MATRIX_ROWS = {row["id"]: row for row in read_jsonl(MATRIX / "rows.jsonl")
               if row["group"] == "oracle"}
DECISIONS = {"oracle-027": [], "oracle-029": ["WRONG"], "oracle-032": ["WRONG"],
             "oracle-034": ["UNDECIDED"]}


def checker():
    root = Path(os.environ.get("PRISM_GATE_INPUTS_ROOT") or
                Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share"))
                / "prism/gate-inputs")
    resolved = root / "typescript" / TREE_PIN / "lib/typescript.js"
    return Path(os.environ["PRISM_TYPESCRIPT"]) if os.environ.get("PRISM_TYPESCRIPT") else (
        resolved if resolved.is_file() else None)


def node_version():
    import subprocess
    result = subprocess.check_output(["node", "--version"], text=True).strip()
    return result.removeprefix("v")


def json_lines(rows):
    return "".join(json.dumps(row) + "\n" for row in rows)


def synthetic_evidence(binary, form, raw):
    """A builder-shaped evidence record with fabricated digests, for test injection only."""
    def sha(tag):
        return hashlib.sha256(tag.encode()).hexdigest()

    target = raw["resolved_targets"][0]["function_id"]
    point = dict(file=target["file"], name=target["name"], start_byte=0, end_byte=1,
                 line=target["start_line"])
    label = dict(run_index=0, record_ordinal=0, target_index=0, form=form, declaration=point,
                 contradiction_line=target["start_line"] + 1)
    return dict(binary_sha256=binary, form=form, label=label, raw=raw,
                source_tree_sha256=sha("tree"), source_state_sha256=sha("state"),
                source_file_sha256=sha("file"), receipt_sha256=sha("receipt"), sites_sha256=sha("sites"),
                controls={"match": "CORRECT", "contradiction": "WRONG"})


class OracleTests(Workbench):
    def setUp(self):
        super().setUp()
        self.out = self.work / "oracle-out"
        self.env["PRISM_MEASURE_SELFCHECK_DIR"] = str(self.work / "selfcheck")
        self.env["PYTHONDONTWRITEBYTECODE"] = "1"

    def typescript(self):
        path = checker()
        if path is None:
            self.skipTest("pinned TypeScript unavailable via PRISM_TYPESCRIPT or gate-inputs resolver")
        return path

    def make_run(self, directory, rows, *, sites=False, raw=None):
        command = "sites" if sites else "edges"
        text = json_lines(rows) if raw is None else raw
        self.make_fake({command: f"print({text!r}, end='')"})
        data = self.fake.read_text()
        placeholder = f"""print({DEFAULT_VERSION!r} if command == "version" else '{{"count":1}}')"""
        self.assertIn(placeholder, data)
        self.fake.write_text(data.replace(placeholder, f"if command != {command!r}: " + placeholder))
        self.assert_exit(self.run_kit(*(["--sites"] if sites else []), out=directory), 0)
        return directory

    def load_case(self, ident, *, transform=None):
        info = MATRIX_ROWS[ident]["input"]
        case = CASES / ident
        source = {p.relative_to(case / "repo").as_posix(): p.read_bytes()
                  for p in (case / "repo").rglob("*") if p.is_file()}
        setup = json.loads((case / info["setup"]).read_bytes()) if "setup" in info else {}
        original = dict(source)
        if "receipt_source" in setup:
            original["cases.ts"] = setup["receipt_source"]
        self.fresh_tree(files=original)
        rows = read_jsonl(case / info["file"])
        if transform:
            transform(rows)
        is_call = info["question"] == "ts-calls"
        head = self.make_run(self.work / "head", rows, sites=is_call)
        if "external" in info:
            snapshot = Path(json.loads((head / "receipt.json").read_bytes())["corpus"]["root"])
            shutil.copyfile(case / info["external"], snapshot.parent / info["external"])
        if "receipt_source" in setup:
            snapshot = Path(json.loads((head / "receipt.json").read_bytes())["corpus"]["root"])
            write_tree(snapshot, source)
        if "base" in info:
            base = self.make_run(self.work / "base", read_jsonl(case / info["base"]), sites=True)
            self.synthetic_qualification(base, head)
            return ("ts-calls", "--base", base, "--head", head)
        if is_call:
            self.synthetic_qualification(head)
            return ("ts-calls", "--census", head)
        return ("ts-edges", "--all", "--run", head)

    def synthetic_qualification(self, *runs, forms=("named-function",)):
        from measure.edges import canonical
        from measure.oracle_inputs import digest
        raw, = read_jsonl(CASES / "oracle-022/probe.jsonl")
        self.qualification = self.work / "qualification"
        builds = {}
        for index, run in enumerate(runs):
            record = json.loads((run / "receipt.json").read_bytes())
            binary = record["binary"]["sha256"]
            builds[binary] = {}
            for form in forms:
                path, data = f"authored-{index}-{form}.json", canonical(synthetic_evidence(binary, form, raw))
                write_tree(self.qualification, {path: data})
                builds[binary][form] = dict(path=path, sha256=digest(data))
        write_tree(self.qualification, {"bundle.json": canonical(
            dict(schema="prism.measure.qualification/1", builds=builds))})

    def local_command(self, callback, args, *, extra=()):
        from measure import oracle_inputs
        from subprocess import CompletedProcess
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.dict(os.environ, self.env, clear=True), redirect_stdout(stdout), redirect_stderr(stderr):
            with patch.object(oracle_inputs, "INJECTED_QUALIFICATIONS", getattr(self, "qualification", None)):
                try:
                    result = callback([*map(str, extra), *map(str, args)])
                except SystemExit as error:
                    result = error.code
        return CompletedProcess(args, result, stdout.getvalue(), stderr.getvalue())

    def prepare(self, *, question=None):
        path = self.typescript()
        command = ["--typescript", str(path)]
        if question:
            command.extend(("--question", str(question)))
        from measure.oracle import selfcheck
        result = self.local_command(selfcheck, command)
        self.assert_exit(result, 0)
        return path

    def oracle(self, args, *, typescript=None, question=None, out=None, **options):
        command = ["oracle", *args, "--out", out or self.out]
        command.extend(("--typescript", typescript or self.typescript()))
        if question:
            command.extend(("--question", question))
        for key, value in options.items():
            command.extend(("--" + key.replace("_", "-"), value))
        from measure.cli import main
        return self.local_command(main, command, extra=("--manifest", self.manifest))

    def fake_node(self, body="raise SystemExit(99)"):
        launcher = self.work / "node"
        log = self.work / "node.log"
        executable(launcher, f"import os\nfrom pathlib import Path\nPath({str(log)!r}).touch()\n" + body)
        self.env["PATH"] = str(self.work) + os.pathsep + self.env["PATH"]
        return log

    def question(self, result, *, relation="custom", prop="custom", name="lane"):
        lane = self.work / name
        entry = lane / "question.cjs"
        for case in ("oracle-001", "oracle-013", "oracle-003", "oracle-004"):
            source = KIT / "selfcheck" / case
            destination = lane / "selfcheck" / case
            shutil.copytree(source, destination)
            expected = read_jsonl(destination / "expected.jsonl")
            inputs = read_jsonl(destination / "input.jsonl")
            for row in expected:
                row.update(relation=relation, property=prop)
                row.pop("why", None)
            for row, input_row in zip(expected, inputs):
                input_row.update(relation=relation, expected_verdict=row["verdict"])
            (destination / "expected.jsonl").write_text(json_lines(expected))
            (destination / "input.jsonl").write_text(json_lines(inputs))
        entry.write_text("module.exports = {relations:['" + relation + "'], grade(row) { return "
                         "{verdict: row.expected_verdict || " + json.dumps(result["verdict"]) +
                         ", property: row.expected_verdict ? '" + prop + "' : " + json.dumps(result["property"]) +
                         ", why: row.expected_verdict ? 'fixture' : " + json.dumps(result["why"]) + "}; }};")
        return entry

    def test_oracle_refuses_wrong_typescript_hash(self):
        args = self.load_case("oracle-017")
        bad = self.work / "mock/lib/typescript.js"
        write_tree(bad.parent, {bad.name: "module.exports = {version: '5.9.3'};\n"})
        log = self.fake_node()
        self.assert_exit(self.oracle(args, typescript=bad), 3)
        self.assertFalse(log.exists())
        self.assertFalse((self.out / "verdicts.jsonl").exists())

    def fake_passing_receipt(self, question=None):
        from measure.oracle_inputs import cache_path, prepare_inputs
        ts = self.typescript()
        from measure import oracle_inputs
        with patch.dict(os.environ, self.env, clear=True), patch.object(
                oracle_inputs, "INJECTED_QUALIFICATIONS", getattr(self, "qualification", None)):
            request = prepare_inputs(ts, question)
            path = cache_path(request["key"])
            expected = sum(len(read_jsonl(Path(case["root"]) / "expected.jsonl"))
                           for case in request["cases"])
            record = dict(schema="prism.measure.selfcheck/1", key=request["key"], status="ok",
                          components=request["components"], cases=len(request["cases"]), expected=expected,
                          agreed=expected, failures=[],
                          runtime={"node": node_version(), "python": platform.python_version()})
            write_tree(path.parent, {path.name: json.dumps(record)})
        return ts

    def test_oracle_refuses_without_selfcheck_receipt_for_current_digest(self):
        args = self.load_case("oracle-018")
        question = self.question(dict(verdict="CORRECT", property="custom", why="proof"))
        self.fake_passing_receipt(question)
        fixture = question.parent / "selfcheck/oracle-001/repo/cases.ts"
        fixture.write_text(fixture.read_text() + "\n")
        log = self.fake_node()
        self.assert_exit(self.oracle(args, question=question), 3)
        self.assertFalse(log.exists())

    def test_oracle_tuples_include_zero_counts(self):
        def include_unsupported_path(rows):
            row = rows[-1]
            endpoint = dict(row["to"], path=dict(row["to"]["path"], fields=["field"]))
            rows.append(dict(row, to=endpoint))
        args = self.load_case("oracle-019", transform=include_unsupported_path)
        self.prepare()
        self.assert_exit(self.oracle(args), 0)
        tables = json.loads((self.out / "tuples.json").read_bytes())
        self.assertEqual(tables["lexical_binding"]["declaration-and-read"],
                         dict(CORRECT=2, WRONG=0, UNDECIDED=1, INADMISSIBLE=0))

    def test_oracle_rejects_unknown_verdict_from_question_module(self):
        self.check_matrix("oracle-020")

    def test_ts_calls_reports_a_retargeted_site_between_runs(self):
        self.check_matrix("oracle-024")

    def test_ts_edges_source_digest_mismatch_is_inadmissible(self):
        self.check_matrix("oracle-016")

    def test_removed_spread_selects_the_base_bag(self):
        case = CASES / "oracle-004"
        self.fresh_tree(files={"cases.ts": (case / "repo/cases.ts").read_bytes()})
        rows = read_jsonl(case / "input.jsonl")
        original = rows[0]
        other = dict(original["from"], path=dict(original["from"]["path"], base="unchanged"))
        retained = [dict(original, **{"from": other})] * 99
        base = self.make_run(self.work / "base", rows + retained)
        head = self.make_run(self.work / "head", retained)
        classifier = self.work / "classifier.py"
        classifier.write_text("CLASSES = ['lost']\ndef classify(change, context):\n    return 'lost'\n")
        diff = self.work / "diff"
        self.assert_exit(self.kit("diff-edges", base, head, "--classifier", classifier, "--out", diff), 0)
        question = self.question(dict(verdict="CORRECT", property="custom", why="proof"))
        question.write_text(question.read_text().replace("grade(row)", "grade(row, ctx)")
                            .replace("|| \"CORRECT\"", "|| (ctx.change?.kind === 'removed' && "
                                     "ctx.arm === 'base' ? 'CORRECT' : 'WRONG')"))
        self.prepare(question=question)
        args = ("ts-edges", "--changes", diff / "changes.jsonl", "--run", base,
                "--kind", "removed", "--class", "lost")
        self.assert_exit(self.oracle(args, question=question, side="base", relation="custom"), 0)
        verdict, = read_jsonl(self.out / "verdicts.jsonl")
        self.assertEqual((verdict["row"], verdict["arm"], verdict["verdict"]), (rows[0], "base", "CORRECT"))
        denominators = json.loads((self.out / "denominators.json").read_bytes())
        self.assertEqual((denominators["selected"], denominators["graded"], denominators["out_of_scope"]),
                         (1, 1, 0))

    def check_helpers(self, code):
        args = self.load_case("oracle-001")
        question = self.question(dict(verdict="CORRECT", property="custom", why="proof"))
        question.write_text(question.read_text().replace("grade(row) { return",
                                                         "grade(row, ctx) {" + code + "\nreturn"))
        self.prepare(question=question)
        self.assert_exit(self.oracle(args, question=question, relation="custom"), 0)

    def test_enclosing_call_uses_both_byte_bounds(self):
        self.check_helpers("""
if (!row.expected_verdict) {
  const {file, start_byte, end_byte} = row.from;
  if (!ctx.enclosing_call(file, start_byte, end_byte) ||
      ctx.enclosing_call(file, start_byte, end_byte + 2) !== null) throw Error('bounds');
}
""")

    def test_default_lib_program_uses_same_sources_and_its_own_checker(self):
        self.check_helpers("""
if (!row.expected_verdict) {
  const {program, checker} = ctx.program_with_default_lib();
  const {file, start_byte, end_byte} = row.from;
  const primary = ctx.identifier_at_bytes(file, start_byte, end_byte);
  const other = ctx.identifier_at_bytes(file, start_byte, end_byte, program);
  if (!primary || !other || primary === other || checker !== program.getTypeChecker() ||
      checker === ctx.checker || !ctx.declarations(other, program).length ||
      ctx.program_with_default_lib().program !== program) throw Error('program');
  let refused = false;
  try { ctx.declarations(primary, program); } catch { refused = true; }
  if (!refused) throw Error('accepted foreign node');
  refused = false;
  try { ctx.identifier_at_bytes(file, start_byte, end_byte, {}); } catch { refused = true; }
  if (!refused) throw Error('accepted foreign program');
}
""")

    def check_matrix(self, ident):
        info = MATRIX_ROWS[ident]
        if ident in ("oracle-017", "oracle-018"):
            return (self.test_oracle_refuses_wrong_typescript_hash() if ident.endswith("017")
                    else self.test_oracle_refuses_without_selfcheck_receipt_for_current_digest())
        args = self.load_case(ident)
        question = (self.question(dict(verdict="UNKNOWN", property="custom", why="fixture"))
                    if ident == "oracle-020" else None)
        if ident == "oracle-028":
            self.assert_exit(self.oracle(args, typescript=Path("unused")), 2)
            return
        self.prepare(question=question)
        truth = info["truth"]
        expected_exit = 2 if ident == "oracle-028" else truth.get("exit", 0)
        result = self.oracle(args, question=question, **({"relation": "custom"} if question else {}))
        self.assert_exit(result, expected_exit)
        if expected_exit:
            self.assertFalse((self.out / "oracle-receipt.json").exists())
            return
        verdicts = read_jsonl(self.out / "verdicts.jsonl")
        if ident in DECISIONS:
            self.assertEqual([row["verdict"] for row in verdicts], DECISIONS[ident])
            if ident in ("oracle-032", "oracle-034"):
                self.assertEqual(verdicts[0]["key"], truth["verdict_alternatives"][0]["key"])
            if ident == "oracle-029":
                self.assertEqual(verdicts[0]["key"],
                                 {"site": dict(file="cases.ts", line=23, start_byte=463, end_byte=470),
                                  "target": dict(file="cases.ts", name="sameB", start_line=22, end_line=22)})
        else:
            self.assertEqual(Counter(json.dumps((v["key"], v["verdict"]), sort_keys=True)
                                     for v in verdicts),
                             Counter(json.dumps((v["key"], v["verdict"]), sort_keys=True)
                                     for v in truth["verdicts"]))
        self.assertTrue(all(v["why"].strip() for v in verdicts))
        deltas = self.out / "deltas.jsonl"
        if "delta_count" in truth:
            self.assertEqual([row["kind"] for row in read_jsonl(deltas)],
                             [truth["delta"]] * truth["delta_count"])
        if info["input"].get("mode") == "census":
            self.assertFalse(deltas.exists())
        actual = json.loads((self.out / "denominators.json").read_bytes())
        for key, expected in truth.get("denominator", {}).items():
            self.assertEqual(actual["census"][key] if key != "sites" else actual[key], expected)

    def test_drop_reason_substitution_survives_cancellation(self):
        args = self.load_case("oracle-030")
        row, = read_jsonl(CASES / "oracle-030/input.jsonl")
        base = self.make_run(self.work / "base", [dict(row, drop="ArgFanout")], sites=True)
        self.prepare()
        self.assert_exit(self.oracle(("ts-calls", "--base", base, "--head", args[-1])), 0)
        delta, = read_jsonl(self.out / "deltas.jsonl")
        self.assertEqual(delta["kind"], "multi-target")
        self.assertEqual(delta["base"]["drop_reasons"], {"ArgFanout": 1})
        self.assertEqual(delta["head"]["drop_reasons"], {"UnknownName": 1})
        self.assertEqual(delta["lost"], [])
        self.assertEqual(delta["gained"], [])

    def check_contradicted_site(self, field):
        setup = CASES / "oracle-021"
        self.fresh_tree(files={"cases.ts": (setup / "repo/cases.ts").read_bytes()})
        first, second = read_jsonl(setup / "input.jsonl")
        base = self.make_run(self.work / "base", [first], sites=True)
        original = copy.deepcopy(first)
        second["resolved_targets"][0]["function_id"]["name"] = "sameB"
        control_head = self.make_run(self.work / "control-head", [original, second], sites=True)
        if field == "line":
            first["source_span"]["line"] += 1
        else:
            first["caller"]["name"] = "contradicted"
        head = self.make_run(self.work / "head", [first, second], sites=True)
        self.synthetic_qualification(base, head, control_head)
        self.prepare()
        control_out = self.work / "control"
        self.assert_exit(self.oracle(("ts-calls", "--base", base, "--head", control_head),
                                     out=control_out), 0)
        self.assertEqual([v["verdict"] for v in read_jsonl(control_out / "verdicts.jsonl")], ["CORRECT"])
        result = self.oracle(("ts-calls", "--base", base, "--head", head))
        if field == "line":
            self.assert_exit(result, 2)
            self.assertIn("cancelled-site-coordinate", result.stderr)
            self.assertFalse((self.out / "oracle-receipt.json").exists())
            return
        self.assert_exit(result, 0)
        verdicts = read_jsonl(self.out / "verdicts.jsonl")
        self.assertTrue(any(v["verdict"] == "INADMISSIBLE" and v["row"] == first for v in verdicts))

    def test_cancelled_site_has_its_own_coordinate_admission(self):
        self.check_contradicted_site("line")

    def test_contradicted_caller_cannot_create_a_credited_claim(self):
        self.check_contradicted_site("caller")

    def check_runtime_result(self, property_name, why, expected):
        args = self.load_case("oracle-001")
        question = self.question(dict(verdict="CORRECT", property=property_name, why=why))
        self.prepare(question=question)
        self.assert_exit(self.oracle(args, question=question, relation="custom"), expected)
        self.assertEqual((self.out / "oracle-receipt.json").exists(), expected == 0)

    def test_runtime_why_must_be_nonblank(self):
        self.check_runtime_result("custom", " ", 1)
        self.out = self.work / "control"
        # The first workbench retains its fixtures. Use a fresh question directory.
        shutil.rmtree(self.work / "lane")
        shutil.rmtree(self.work / "head")
        self.check_runtime_result("custom", "proof", 0)

    def test_runtime_property_must_be_covered(self):
        self.check_runtime_result("unchecked", "proof", 1)
        self.out = self.work / "control"
        shutil.rmtree(self.work / "lane")
        shutil.rmtree(self.work / "head")
        self.check_runtime_result("custom", "proof", 0)

    def test_missing_recorded_snapshot_refuses(self):
        args = self.load_case("oracle-001")
        run = args[-1]
        root = Path(json.loads((run / "receipt.json").read_bytes())["corpus"]["root"])
        root.rename(root.with_name("moved"))
        self.assert_exit(self.oracle(args, typescript=Path("unused")), 2)

    def test_child_cannot_inherit_node_preload_or_search_path(self):
        args = self.load_case("oracle-001")
        self.prepare()
        preload = self.work / "preload.cjs"
        preload.write_text("throw Error('caller preload was executed');")
        self.env.update(NODE_OPTIONS="--require=" + str(preload), NODE_PATH=str(self.work))
        self.assert_exit(self.oracle(args), 0)

    def test_fabricated_current_selfcheck_receipt_cannot_qualify_a_module(self):
        from measure.oracle_inputs import prepare_inputs, cache_path
        args = self.load_case("oracle-001")
        question = self.question(dict(verdict="CORRECT", property="custom", why="proof"))
        ts = self.fake_passing_receipt(question)
        previous, = (self.work / "selfcheck").glob("selfcheck-*.json")
        forged = json.loads(previous.read_bytes())
        good = question.read_text()
        bad = good.replace('row.expected_verdict || "CORRECT"',
                           '(row.expected_verdict ? "WRONG" : "CORRECT")')
        self.assertNotEqual(good, bad)
        question.write_text(bad)
        with patch.dict(os.environ, self.env, clear=True):
            request = prepare_inputs(ts, question)
            forged.update(key=request["key"], components=request["components"])
            cache_path(request["key"]).write_text(json.dumps(forged))
        result = self.oracle(args, question=question, relation="custom")
        self.assert_exit(result, 1)
        self.assertIn("selfcheck-disagreement", result.stderr)
        self.assertFalse((self.out / "oracle-receipt.json").exists())
        question.write_text(good)
        self.assert_exit(self.oracle(args, question=question, relation="custom"), 0)

    def test_default_lib_checks_the_bytes_it_reads(self):
        original = self.typescript()
        install = self.work / "typescript"
        shutil.copytree(original.parent.parent, install)
        candidate = install / "lib/typescript.js"
        args = self.load_case("oracle-001")
        question = self.question(dict(verdict="CORRECT", property="custom", why="proof"))
        text = question.read_text().replace("grade(row) { return", "grade(row, ctx) { "
                                            "if (!row.expected_verdict) ctx.program_with_default_lib(); return")
        question.write_text(text)
        from measure.oracle import selfcheck
        prepared = self.local_command(selfcheck, ["--typescript", candidate, "--question", question])
        self.assert_exit(prepared, 0)
        self.assert_exit(self.oracle(args, typescript=candidate, question=question, relation="custom",
                                     out=self.work / "matched"), 0)
        lib = candidate.with_name("lib.es2022.full.d.ts")
        real_node = shutil.which("node")
        self.assertIsNotNone(real_node)
        self.fake_node(f"""
import subprocess
import sys
library = Path({str(lib)!r})
before = library.read_bytes()
try:
    library.write_bytes(before + b'\\n// changed before the host read\\n')
    result = subprocess.call([{real_node!r}, *sys.argv[1:]])
finally:
    library.write_bytes(before)
raise SystemExit(result)
""")
        outcome = self.oracle(args, typescript=candidate, question=question, relation="custom")
        self.assert_exit(outcome, 1)
        self.assertIn("bound-read: loaded bytes differ", outcome.stderr)
        self.assertFalse((self.out / "oracle-receipt.json").exists())
        self.fake_node(f"""
import subprocess
import sys
library = Path({str(lib)!r})
saved = library.with_suffix('.saved')
library.rename(saved)
try:
    library.symlink_to(saved)
    result = subprocess.call([{real_node!r}, *sys.argv[1:]])
finally:
    library.unlink()
    saved.rename(library)
raise SystemExit(result)
""")
        outcome = self.oracle(args, typescript=candidate, question=question, relation="custom")
        self.assert_exit(outcome, 1)
        self.assertIn("bound-read: path is no longer a regular closure member", outcome.stderr)
        self.assertFalse((self.out / "oracle-receipt.json").exists())

    def test_selfcheck_supplies_a_declared_change_context(self):
        question = self.question(dict(verdict="CORRECT", property="custom", why="proof"))
        sample = question.parent / "selfcheck/oracle-001/input.jsonl"
        raw = read_jsonl(sample)[0]
        from measure.edges import decode_edge
        key, attribute, _ = decode_edge(raw, line_mode=False)
        change = dict(id=1, key=json.loads(key), kind="added", count=1, base_bag=[],
                      head_bag=[json.loads(attribute)], records=[], error=None, **{"class": "fixture"})
        sample.write_text(json_lines([dict(relation="custom", arm="head", row=raw, change=change)]))
        text = question.read_text().replace("grade(row) { return", "grade(row, ctx) { "
             "if (ctx.arm === 'head' && ctx.change?.kind !== 'added') throw Error('missing change'); return")
        question.write_text(text)
        self.prepare(question=question)
        sample.write_text(json_lines([dict(relation="custom", arm="head", row=raw)]))
        from measure.oracle import selfcheck
        result = self.local_command(selfcheck, ["--typescript", self.typescript(), "--question", question])
        self.assert_exit(result, 1)
        self.assertIn("missing change", result.stderr)

    def test_symlink_alias_is_not_a_program_filename(self):
        def build(alias):
            lib = "export function f(v: number) { return v; }\n"
            main = "import { f } from './" + alias + "/def';\nconst actual = 1; f(actual);\n"
            self.source = self.work / ("alias-source" if alias == "alias" else "regular-source")
            self.make_corpus(git=False, files={"lib/def.ts": lib, "main.ts": main},
                             links={"alias": "lib"}, pin_tree=True)
            def point(file, src, token, access, position):
                return dict(file=file, start_byte=position, end_byte=position + len(token),
                            line=src[:position].count("\n") + 1, access=access,
                            path=dict(base=token, fields=[]))
            row = {"from": point("main.ts", main, "actual", "use", main.rindex("actual")),
                   "to": point("lib/def.ts", lib, "v", "def", lib.index("v:")),
                   "confidence": "exact", "doubt": None}
            return self.make_run(self.work / (alias + "-run"), [row])

        self.prepare()
        for alias, expected in (("lib", "CORRECT"), ("alias", "UNDECIDED")):
            run = build(alias)
            out = self.work / (alias + "-out")
            self.assert_exit(self.oracle(("ts-edges", "--all", "--run", run), out=out), 0)
            grade, = read_jsonl(out / "verdicts.jsonl")
            self.assertEqual(grade["verdict"], expected)

    def test_verdict_order_uses_canonical_utf8_key_bytes(self):
        case = CASES / "oracle-001"
        self.fresh_tree(files={"cases.ts": (case / "repo/cases.ts").read_bytes()})
        first, = read_jsonl(case / "input.jsonl")
        nine = dict(first, **{"from": dict(first["from"], line=9)})
        ten = dict(first, **{"from": dict(first["from"], line=10)})
        run = self.make_run(self.work / "head", [nine, ten, first])
        self.prepare()
        self.assert_exit(self.oracle(("ts-edges", "--all", "--run", run)), 0)
        rows = read_jsonl(self.out / "verdicts.jsonl")
        self.assertEqual([v["key"]["from"]["line"] for v in rows], [10, 3, 9])
        self.assertEqual([v["verdict"] for v in rows], ["INADMISSIBLE", "CORRECT", "INADMISSIBLE"])

    def test_qualification_builder_binds_decorated_method_controls(self):
        from measure.oracle import build_qualification
        case = KIT / "selfcheck/oracle-058-decorated"
        source = (case / "repo/cases.ts").read_bytes()
        rows = read_jsonl(case / "input.jsonl")
        call = next(row for row in rows if row.get("record_kind") == "call_site" and
                    row["resolved_targets"][0]["function_id"]["start_line"] == 3)
        self.source = self.work / "qualified-source"
        self.make_corpus(files={"cases.ts": source})
        run = self.make_run(self.work / "capture", [call], sites=True)
        offset = source.index(b"static m") + len(b"static ")
        label = dict(run_index=0, record_ordinal=0, target_index=0, form="decorated-method",
                     declaration=dict(file="cases.ts", name="m", start_byte=offset, end_byte=offset + 1, line=3),
                     contradiction_line=2)
        output = self.work / "qualification"
        with patch.dict(os.environ, self.env | {"PRISM_TYPESCRIPT": str(self.typescript())}, clear=True):
            expect = build_qualification([run], [label], output)
            with self.assertRaisesRegex(ValueError, "label disagrees"):
                build_qualification([run], [dict(label, declaration=dict(label["declaration"], name="q"))],
                                    self.work / "failed-qualification")
        self.qualification = output
        self.prepare()
        self.assert_exit(self.oracle(("ts-calls", "--census", run)), 0)
        record = json.loads((self.out / "oracle-receipt.json").read_bytes())
        self.assertEqual(record["qualification_sha256"], expect)
        grade, = read_jsonl(self.out / "verdicts.jsonl")
        self.assertEqual(grade["verdict"], "CORRECT")

    def test_lf_only_line_rule_admits_raw_unicode_terminator(self):
        case = KIT / "selfcheck/lf-line-and-utf8"
        self.fresh_tree(files={"cases.ts": (case / "repo/cases.ts").read_bytes()})
        run = self.make_run(self.work / "head", read_jsonl(case / "input.jsonl"))
        self.prepare()
        self.assert_exit(self.oracle(("ts-edges", "--all", "--run", run)), 0)
        actual = read_jsonl(self.out / "verdicts.jsonl")
        self.assertEqual([(v["key"], v["verdict"]) for v in actual],
                         [(v["key"], v["verdict"]) for v in read_jsonl(case / "expected.jsonl")])

    def test_declared_question_data_restricts_ambient_reads(self):
        from measure.oracle import selfcheck
        from measure.oracle_inputs import digest
        question = self.question(dict(verdict="CORRECT", property="custom", why="proof"))
        data = question.parent / "annotation.json"
        data.write_text('{"approved":true}')
        question.write_text("const fs = require('node:fs'); "
                            "if (!JSON.parse(fs.readFileSync('annotation.json', 'utf8')).approved) "
                            "throw Error('fixture annotation');" + question.read_text())
        ts = self.typescript()
        args = ["--typescript", ts, "--question", question]
        result = self.local_command(selfcheck, args)
        self.assert_exit(result, 1)
        self.assertIn("undeclared data read", result.stderr)
        (question.parent / "question-data.json").write_text(json.dumps({data.name: digest(data.read_bytes())}))
        self.assert_exit(self.local_command(selfcheck, args), 0)
