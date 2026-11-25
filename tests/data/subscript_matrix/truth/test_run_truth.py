import contextlib
from hashlib import sha256
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import textwrap
import unittest
from unittest.mock import patch

import run_truth
from containment import Refused, check_source


HERE = Path(__file__).resolve().parent


class TruthTests(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.cases = self.root / "cases"
        self.cases.mkdir()
        self.rows = self.root / "rows.jsonl"
        self.truth = self.root / "truth.jsonl"

    def write_jsonl(self, path, records):
        path.write_text("".join(json.dumps(record) + "\n" for record in records), encoding="utf-8")

    def write_cases(self, *cases):
        rows = []
        for name, source in cases:
            (self.cases / name).write_text(textwrap.dedent(source).lstrip("\n"), encoding="utf-8")
            rows.append({"id": name, "lang": name.rsplit(".", 1)[-1], "group": "control",
                         "file": name})
        self.write_jsonl(self.rows, rows)

    def run_cli(self, *args):
        return subprocess.run(
            [sys.executable, "-B", str(HERE / "run_truth.py"), *map(str, args)],
            capture_output=True, text=True, timeout=120,
        )

    def run_check(self):
        return subprocess.run(
            [str(HERE / "check.sh"), str(self.rows), str(self.truth)],
            capture_output=True, text=True, timeout=15,
        )

    def test_observations_and_deep_traversal(self):
        self.write_cases(
            ("reach.py", "def case():\n    t = src()\n    sink('prefix' + t)"),
            ("exception.py", "def case():\n    t = src()\n    sink(t)\n    raise RuntimeError()"),
            ("double.py", "def case():\n    t = src()\n    sink(t)\n    sink(None)"),
            ("silent.py", "def case():\n    t = src()\n    return t"),
            ("nested.py", """
                class Box:
                    pass

                def case():
                    t = src()
                    o = Box()
                    seq = [o]
                    o.values = [{"key": ({t},)}, seq]
                    sink({"nested": seq})
            """),
            ("clean.py", """
                def case():
                    t = src()
                    d = {}
                    d["cycle"] = [d]
                    sink(d)
            """),
            ("nested.js", """
                function caseFn() {
                    const t = src();
                    const m = new Map();
                    const o = { nested: [m] };
                    m.set("cycle", o);
                    m.set("leaf", new Set([Object.defineProperty({}, "secret", {value: t})]));
                    sink(o);
                }
            """),
            ("clean.js", """
                function caseFn() {
                    const t = src();
                    const list = [];
                    list.push(new Map([["cycle", list]]));
                    sink(list);
                }
            """),
            ("typed.ts", """
                function caseFn() {
                    const t: string = src();
                    const row: {x: string} = {x: t};
                    sink(row);
                }
            """),
            ("nested.go", """
                package main
                type hidden struct { key string }
                func caseFn() {
                    t := src()
                    m := map[string]any{}
                    m["cycle"] = m
                    m["leaf"] = &[]any{[1]hidden{{key: t}}}
                    sink(m)
                }
            """),
            ("clean.go", """
                package main
                func caseFn() {
                    t := src()
                    m := map[string]any{}
                    m["cycle"] = []any{m}
                    m["empty"] = t[:0]
                    sink(m)
                }
            """),
            ("control.c", "/* C controls have no executed truth. */"),
        )
        result = self.run_cli(self.cases, self.rows, "--out", self.truth)
        self.assertEqual(result.returncode, 0, result.stderr)
        records = run_truth.load_jsonl(self.truth)
        self.assertEqual(set(records[0]["versions"]), {"python", "node", "go"})
        self.assertEqual(
            [(record["id"], record["truth"]) for record in records[1:]],
            [("reach.py", "reaches"), ("exception.py", "inadmissible"), ("double.py", "inadmissible"),
             ("silent.py", "inadmissible"), ("nested.py", "reaches"), ("clean.py", "not"),
             ("nested.js", "reaches"), ("clean.js", "not"), ("typed.ts", "reaches"),
             ("nested.go", "reaches"), ("clean.go", "not"), ("control.c", "n/a")],
            result.stderr,
        )

    def assert_refused(self, arguments, name, rule):
        stderr = io.StringIO()
        with patch.object(sys, "argv", ["run_truth.py", *map(str, arguments)]), \
             patch.object(run_truth, "Runner") as runner, contextlib.redirect_stderr(stderr):
            with self.assertRaises(SystemExit) as error:
                run_truth.main()
            self.assertEqual(error.exception.code, 2)
            runner.assert_not_called()
        self.assertIn(name, stderr.getvalue())
        self.assertIn(rule, stderr.getvalue())
        self.assertFalse(self.truth.exists())

    def test_refuses_every_case_before_launching_any(self):
        examples = (
            ("py", "import pathlib", "imports"),
            ("py", "from pathlib import Path", "imports"),
            ("py", "__import__('pathlib')", "__import__"),
            ("py", "open('x')", "open"),
            ("js", 'require /* gap */ ("module")', "require"),
            ("js", 'import("./other.mjs")', "imports"),
            ("js", "const v = process;", "process"),
            ("ts", "const v = fetch;", "fetch"),
            ("go", 'package main\nimport (\n"reflect"\n)', "imports"),
            ("go", 'package main\nimport "reflect"', "imports"),
        )
        for lang, source, rule in examples:
            name = "later." + lang
            with self.subTest(lang=lang, source=source):
                self.write_cases(("first.py", "def case():\n    sink(src())"), (name, source))
                self.assert_refused((self.cases, self.rows, "--out", self.truth), name, rule)

    def test_refuses_modified_preludes_even_when_the_language_has_no_row(self):
        self.write_cases(("one.py", "def case():\n    sink(src())"))
        for lang, name in run_truth.PRELUDES.items():
            (self.root / name).write_bytes((HERE / name).read_bytes())
        go = self.root / run_truth.PRELUDES["go"]
        go.write_text(go.read_text(encoding="utf-8").replace('"strings"', '"os"'), encoding="utf-8")
        with patch.object(run_truth, "HERE", self.root):
            self.assert_refused((self.cases, self.rows, "--out", self.truth), go.name, "import set")

    def test_comments_and_string_tokens_do_not_trigger_containment(self):
        check_source("example.py", "py", textwrap.dedent('''
            # open exec eval compile __import__ __builtins__ breakpoint input from import
            key = "open exec eval compile __import__ __builtins__ breakpoint input from import"
            text = f"{key} open"
        '''))
        check_source("example.js", "js", r'''
            /* require("x") import("x") fetch */
            const key = "process globalThis fetch eval Function import('x')";
            // import file from "x"
            const template = `require("x") process ${key}`;
            const re = /process|fetch/;
        ''')
        check_source("example.go", "go", '''
            package main
            // import "os"
            var key = `import "os"`
        ''')

    def test_template_expressions_are_code(self):
        with self.assertRaisesRegex(Refused, "open"):
            check_source("example.py", "py", """key = f"{open('unused')}" """)
        with self.assertRaisesRegex(Refused, "fetch"):
            check_source("example.ts", "ts", "const key = `literal ${fetch}`;")
        with self.assertRaisesRegex(Refused, "process"):
            check_source("example.js", "js", r"const key = pr\u006fcess;")
        with self.assertRaisesRegex(Refused, "process"):
            check_source("example.js", "js", "const key = object.return / process / 2;")

    def test_timeout_discards_observation(self):
        runner = run_truth.Runner(self.root)
        source = "def case():\n    sink(src())\n    while True:\n        pass\n"
        stderr = io.StringIO()
        with patch.object(run_truth, "TIMEOUT", 0.25), contextlib.redirect_stderr(stderr):
            truth = run_truth.row_truth(runner, run_truth.load_preludes(),
                                        {"id": "timeout.py", "lang": "py"}, source)
        self.assertEqual(truth, "inadmissible")
        self.assertIn("timeout", stderr.getvalue())

    def test_go_version_probe_leaves_no_telemetry_counters(self):
        run = run_truth.run_process
        versions = []

        def inspect(command, directory, env, timeout):
            result = run(command, directory, env, timeout)
            if command[1:] == ["version"] and Path(command[0]).name == "go":
                versions.append(result[1])
                self.assertEqual(list(Path(env["HOME"]).rglob("*.count")), [])
            return result

        with patch.object(run_truth, "run_process", side_effect=inspect):
            runner = run_truth.Runner(self.root)
        self.assertEqual(len(versions), 1)
        self.assertEqual(runner.versions["go"], versions[0].strip())

    def test_checks_missing_duplicate_and_invalid_row_truth(self):
        self.write_cases(("a.py", ""), ("b.rs", ""))
        versions = {"versions": {"python": "x", "node": "y", "go": "z"}}
        first = {"id": "a.py", "truth": "reaches"}
        second = {"id": "b.rs", "truth": "n/a"}
        for records, expected in (
            ([], "empty"),
            ([versions, first], "missing truth"),
            ([versions, first, second, first], "duplicate truth"),
            ([versions, {"id": "a.py"}, second], "malformed"),
            ([versions, dict(first, truth="inadmissible"), second], "inadmissible"),
            ([versions, first, dict(second, truth="not")], "invalid truth"),
            ([versions, second, first], "out of row order"),
        ):
            with self.subTest(expected=expected):
                self.write_jsonl(self.truth, records)
                result = self.run_check()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(expected, result.stderr)
        self.write_jsonl(self.truth, [versions, first, second])
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_rebuilt_truth_is_bound_to_fixture_bytes(self):
        self.write_cases(("case.py", "def case():\n    sink(src())"))
        case = self.cases / "case.py"
        digest = sha256(case.read_bytes()).hexdigest()
        rows = run_truth.load_rows(self.rows)
        rows[0]["case_sha256"] = digest
        self.write_jsonl(self.rows, rows)
        result = self.run_cli(self.cases, self.rows, "--out", self.truth)
        self.assertEqual(result.returncode, 0, result.stderr)
        versions, receipt = run_truth.load_jsonl(self.truth)
        self.assertEqual(receipt, {"id": "case.py", "truth": "reaches", "case_sha256": digest})
        self.assertEqual(self.run_check().returncode, 0)

        self.write_jsonl(self.truth, [versions, dict(receipt, case_sha256="0" * 64)])
        self.assertIn("truth SHA-256 differs", self.run_check().stderr)
        self.write_jsonl(self.truth, [versions, receipt])

        case.write_text("def case():\n    sink(None)\n")
        self.assertIn("case SHA-256 differs", self.run_check().stderr)
        result = self.run_cli(self.cases, self.rows, "--out", self.truth)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("case SHA-256 differs", result.stderr)

    def test_mixed_truth_observes_bits_in_source_order(self):
        examples = (
            ("only-a", "sink([a, None])", [True, False]),
            ("only-b", "sink([b, None])", [False, True]),
            ("both", "d = [b, a]; d.append(d); sink(d)", [True, True]),
            ("neither", "sink({})", [False, False]),
            ("replace-a", "d = [a, b]; d[0] = 'clean'; sink(d)", [False, True]),
        )
        self.write_cases(*(
            (name + ".py", f"def case():\n    a = src_a()\n    b = src_b()\n    {body}\n")
            for name, body, _ in examples
        ))
        self.write_jsonl(self.rows, [dict(row, truth_protocol="ab")
                                    for row in run_truth.load_rows(self.rows)])
        result = self.run_cli(self.cases, self.rows, "--out", self.truth)
        self.assertEqual(result.returncode, 0, result.stderr)
        versions, *records = run_truth.load_jsonl(self.truth)
        self.assertEqual([r["truth"] for r in records], [bits for _, _, bits in examples])
        self.assertEqual(self.run_check().returncode, 0)
        for bad in ([True], [True, False, False], [1, 0], "reaches"):
            self.write_jsonl(self.truth, [versions, dict(records[0], truth=bad), *records[1:]])
            self.assertIn("invalid truth", self.run_check().stderr)

    def test_mixed_truth_refuses_incomplete_or_ambiguous_sources(self):
        for body in (
            "a = src_a()\n    sink(a)",
            "a = src_a()\n    b = src_b()\n    again = src_a()\n    sink(b)",
            "a = src_a(); b = src_b()\n    sink([a, b])",
            "a = src_a()\n    if False:\n        b = src_b()\n    sink(a)",
        ):
            with self.subTest(body=body):
                self.write_cases(("bad.py", "def case():\n    " + body))
                row, = run_truth.load_rows(self.rows)
                self.write_jsonl(self.rows, [dict(row, truth_protocol="ab")])
                result = self.run_cli(self.cases, self.rows, "--out", self.truth)
                self.assertIn("mixed sources", result.stderr)
                self.assertTrue(result.returncode != 0 or
                                run_truth.load_jsonl(self.truth)[1]["truth"] == "inadmissible")

    def test_mixed_truth_refuses_missing_repeated_or_malformed_observation(self):
        for body in ("return b", "sink(a); sink(b)", "print('OBSERVE A 0 B 1')",
                     "sink(a); print('OBSERVE B 1 A 0')",
                     "sink(a); print('OBSERVE A 1 A 0')", "sink(a); print('OBSERVE A 1')"):
            with self.subTest(body=body):
                self.write_cases(("bad.py", f"def case():\n    a = src_a()\n    b = src_b()\n    {body}"))
                row, = run_truth.load_rows(self.rows)
                self.write_jsonl(self.rows, [dict(row, truth_protocol="ab")])
                result = self.run_cli(self.cases, self.rows, "--out", self.truth)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("expected one valid observation", result.stderr)
                self.assertEqual(run_truth.load_jsonl(self.truth)[1]["truth"], "inadmissible")

    def test_tsx_value_case_uses_ts_runtime_and_tsx_graph(self):
        self.write_cases(
            ("value.tsx", "function caseFn() {\n  const t: string = src();\n  sink(t);\n}\n"),
            ("jsx.tsx", "function caseFn() { const t = src(); sink(<div>{t}</div>); }"),
        )
        result = self.run_cli(self.cases, self.rows, "--out", self.truth)
        self.assertEqual(result.returncode, 0, result.stderr)
        versions, value, jsx = run_truth.load_jsonl(self.truth)
        self.assertEqual(value, {"id": "value.tsx", "truth": "reaches",
                                 "graph_mode": "tsx", "runtime_mode": "ts"})
        self.assertEqual(jsx["truth"], "inadmissible")
        self.assertEqual(jsx["graph_mode"], "tsx")
        self.write_jsonl(self.rows, run_truth.load_rows(self.rows)[:1])
        self.write_jsonl(self.truth, [versions, value])
        self.assertEqual(self.run_check().returncode, 0)
        self.write_jsonl(self.truth, [versions, dict(value, runtime_mode="tsx")])
        self.assertIn("invalid graph/runtime modes", self.run_check().stderr)


if __name__ == "__main__":
    unittest.main()
