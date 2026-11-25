"""Run with python3 -m unittest discover -s tests/routes_matrix/truth."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent


class RunnerTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.work = Path(temporary.name)
        self.root = self.work / "selfcheck"
        shutil.copytree(HERE / "selfcheck", self.root)
        self.rows = self.root / "rows.jsonl"
        self.output = self.work / "truth.jsonl"

    def run_rows(self, *ids, extra_env=None, verify_sources=False):
        command = [
            sys.executable, "-B", str(HERE / "run_truth.py"), "--root", str(self.root),
            "--rows", str(self.rows), "--out", str(self.output),
        ]
        if ids:
            command.extend(["--only", *ids])
        if verify_sources:
            command.append("--verify-sources")
        result = subprocess.run(
            command, env={**os.environ, **(extra_env or {})},
            capture_output=True, text=True, timeout=40,
        )
        rows = [json.loads(line) for line in self.output.read_text().splitlines()]
        self.assertEqual(len(rows), len({row["id"] for row in rows}))
        return result, {row["id"]: row for row in rows}

    def assert_subset(self, expected, actual):
        if isinstance(expected, dict):
            for key, value in expected.items():
                self.assertIn(key, actual)
                self.assert_subset(value, actual[key])
        elif isinstance(expected, list):
            self.assertEqual(len(expected), len(actual))
            for left, right in zip(expected, actual):
                self.assert_subset(left, right)
        else:
            self.assertEqual(expected, actual)

    def assert_expected(self, row):
        metadata = next(
            value for line in self.rows.read_text().splitlines()
            if (value := json.loads(line))["id"] == row["id"]
        )
        for expected in metadata["expect"]:
            candidates = []
            for projected in row["projection"]:
                actual = dict(projected)
                if "paths" in expected:
                    actual["paths"] = []
                    for pin in expected["paths"]:
                        for path in projected["paths"]:
                            if pin["status"] == "full" and path["value"] == pin["value"]:
                                actual["paths"].append(path)
                                break
                            if pin["status"] == "suffix" and path["value"].endswith(pin["value"]):
                                actual["paths"].append(pin)
                                break
                try:
                    self.assert_subset(expected, actual)
                except AssertionError:
                    continue
                candidates.append(projected)
            self.assertEqual(len(candidates), 1, (expected, row["projection"]))

    def test_flask_blueprint_factory_decorators_and_ordinals(self):
        result, rows = self.run_rows("flask")
        self.assertEqual(result.returncode, 0, result.stderr)
        row = rows["flask"]
        self.assert_expected(row)
        original = [call for call in row["calls"] if not call["mounted"]]
        self.assertEqual([(call["site"]["line"], call["site"]["ordinal"]) for call in original],
                         [(7, 1), (6, 1), (12, 1), (12, 2)])
        self.assertEqual(len({json.dumps(call["callbacks"]) for call in original}), 1)
        self.assertEqual(original[2]["args"]["methods"], ["put", "put"])
        self.assertEqual(len(row["served"]), 8)
        for route in row["served"]:
            source, copy = (row["calls"][i - 1] for i in route["call_ids"])
            self.assertEqual(copy["origin_call_id"], source["id"])
            self.assertEqual(copy["site"]["file"], "app.py")
            self.assertEqual(copy["args"]["rule"], route["path"])
            self.assertEqual(source["callbacks"], copy["callbacks"])
        one = next(route for route in row["served"] if route["path"] == "/v1/one")
        self.assertEqual(one["declared_methods"], ["GET"])
        self.assertEqual(one["effective_methods"], ["GET", "HEAD", "OPTIONS"])

    def test_fastapi_includes_prefix_and_websocket(self):
        result, rows = self.run_rows("fastapi")
        self.assertEqual(result.returncode, 0, result.stderr)
        row = rows["fastapi"]
        self.assert_expected(row)
        self.assertEqual(len(row["served"]), 2)
        http, ws = row["served"]
        self.assertEqual(http["effective_methods"], ["GET"])
        self.assertEqual(ws["kind"], "websocket")
        for route in row["served"]:
            source, copy = (row["calls"][i - 1] for i in route["call_ids"])
            self.assertEqual(copy["origin_call_id"], source["id"])
            self.assertEqual(copy["site"]["file"], "app.py")

    def test_express_mount_and_swapped_anonymous_callbacks(self):
        result, rows = self.run_rows("express", "swapped")
        self.assertEqual(result.returncode, 0, result.stderr)
        for row in rows.values():
            self.assert_expected(row)
            route = next(route for route in row["served"] if route["path"] == "/v1/a")
            self.assertEqual(route["effective_methods"], ["GET", "HEAD", "OPTIONS"])
            mount, = (row["calls"][i - 1] for i in route["mount_call_ids"])
            self.assertEqual(mount["kind"], "use")
            self.assertEqual(mount["args"]["path"], "/v1")
            self.assertEqual(mount["mount_prefix"], "/v1")
        self.assertNotEqual(rows["express"]["projection"], rows["swapped"]["projection"])

    def test_express_builder_explicit_head_and_all(self):
        result, rows = self.run_rows("builder")
        self.assertEqual(result.returncode, 0, result.stderr)
        row = rows["builder"]
        self.assert_expected(row)
        self.assertEqual([call["kind"] for call in row["calls"]],
                         ["route", "verb", "verb", "verb", "route", "verb", "verb"])
        head = [route for route in row["served"] if route["path"] == "/head"]
        self.assertEqual([route["declared_methods"] for route in head], [["GET", "HEAD"]])
        self.assertEqual(head[0]["effective_methods"], ["GET", "HEAD", "OPTIONS"])

    def test_express_subapp_and_fast_slash(self):
        result, rows = self.run_rows("nested")
        self.assertEqual(result.returncode, 0, result.stderr)
        row = rows["nested"]
        self.assert_expected(row)
        self.assertEqual([route["mount_call_ids"] for route in row["served"]], [[3, 2], [4, 2]])

    def test_invalid_rows_refuse_with_the_expected_reasons(self):
        (self.root / "bad_link" / "entry.py").symlink_to("target.py")
        decoy = self.work / "decoy"
        decoy.mkdir()
        (decoy / "truth_runner_ambient_decoy.py").write_text("loaded = True\n")
        (decoy / "external_truth_helper.py").write_text("loaded = True\n")
        origin = self.root / "origin_escape" / "app.py"
        origin.write_text(origin.read_text().replace("EXTERNAL_DIRECTORY", repr(str(decoy))))
        reasons = {
            "outside": "invalid row directory",
            "link": "symlink",
            "helper": "undeclared Python import: undeclared_routes_dependency",
            "ambient": "No module named 'truth_runner_ambient_decoy'",
            "untraced": "untraced FastAPI route: /openapi.json",
            "untraced-js": "untraced Express route layer",
            "js-escape": "relative import escapes row",
            "relative": "relative import escapes row package",
            "origin": "Python module outside row, venv or stdlib: external_truth_helper",
        }
        result, rows = self.run_rows(*reasons, extra_env={
            "PYTHONPATH": str(decoy), "NODE_PATH": str(decoy), "GOPATH": str(decoy),
        })
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(set(rows), set(reasons))
        for row_id, reason in reasons.items():
            with self.subTest(row=row_id):
                self.assertEqual(rows[row_id]["status"], "error")
                self.assertIn(reason, rows[row_id]["error"])

    def test_hardlink_refused_even_when_both_names_are_inside_row(self):
        source = self.root / "flask_multi" / "app.py"
        os.link(source, source.with_name("alias.py"))
        result, rows = self.run_rows("flask")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("hardlink", rows["flask"]["error"])

    def test_provenance_gate(self):
        result, rows = self.run_rows("regex", "go", verify_sources=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(rows, {})
        lines = [json.loads(line) for line in self.rows.read_text().splitlines()]
        regex = next(row for row in lines if row["id"] == "regex")
        flask = next(row for row in lines if row["id"] == "flask")
        regex["truth_source"] = "executed"
        flask["truth_source"] = "hand"
        self.rows.write_text("".join(json.dumps(row) + "\n" for row in lines))
        result, rows = self.run_rows("regex", "flask", verify_sources=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(rows["regex"]["status"], "hand_required")
        self.assertIn("Express mount prefix", rows["regex"]["error"])
        self.assertIn("hand row can execute", result.stderr)

    def test_shell_gate_diffs_the_truth(self):
        rows = [json.loads(line) for line in self.rows.read_text().splitlines()]
        selected = {"empty", "flask", "express", "regex", "go"}
        self.rows.write_text("".join(json.dumps(row) + "\n" for row in rows if row["id"] in selected))
        generated, observed = self.run_rows()
        self.assertEqual(generated.returncode, 0, generated.stderr)
        self.assertEqual(set(observed), {"empty", "flask", "express"})
        self.assertEqual(observed["empty"]["calls"], [])
        self.assertEqual(observed["empty"]["served"], [])
        command = [str(HERE / "check.sh"), str(self.root), str(self.output)]
        clean = subprocess.run(command, capture_output=True, text=True, timeout=40)
        self.assertEqual(clean.returncode, 0, clean.stderr)
        self.output.write_text(self.output.read_text().replace("/v1/a", "/v1/changed"))
        dirty = subprocess.run(command, capture_output=True, text=True, timeout=40)
        self.assertEqual(dirty.returncode, 1, dirty.stderr)
        self.assertIn("/v1/changed", dirty.stdout)

    def test_timeout_refuses_the_row(self):
        result, rows = self.run_rows("timeout")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(rows["timeout"]["status"], "environment_error")
        self.assertIn("child exceeded 10 s", rows["timeout"]["error"])


class SourceGateTests(unittest.TestCase):
    def test_only_the_truncation_row_may_execute_with_a_hand_label(self):
        from unittest.mock import patch
        import run_truth

        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            rows = root / "rows.jsonl"
            output = root / "truth.jsonl"
            rows.write_text("".join(json.dumps({
                "id": row_id, "truth_source": "hand", **metadata,
            }) + "\n" for row_id, metadata in [
                ("es2-34", {"hand_reason": "64-exposure cap"}),
                ("other", {"hand_reason": "free text cannot exempt this row"}),
                ("another", {}),
            ]))
            with (patch.object(sys, "argv", ["run_truth", "--rows", str(rows), "--root",
                                             str(root), "--out", str(output), "--verify-sources"]),
                  patch.object(run_truth, "execute", return_value={"status": "executed"}),
                  patch.object(sys, "stderr") as stderr):
                self.assertEqual(run_truth.main(), 1)
            self.assertEqual("".join(call.args[0] for call in stderr.write.call_args_list),
                             "other: hand row can execute\nanother: hand row can execute\n")
            self.assertEqual(output.read_text(), "")


if __name__ == "__main__":
    unittest.main()
