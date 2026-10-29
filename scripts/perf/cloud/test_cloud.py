import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import fetch_corpora as fetch
import measure
import reporting
from summarize_profile import load_records


def profile(phases=()):
    return [dict(record="header", schema_version=1),
            dict(record="command", wall_ns=100),
            *[dict(record="phase", name=name, time_kind=kind, wall_ns=value,
                   status="measured") for name, kind, value in phases],
            dict(record="summary")]


class CorpusTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def registry(self, data):
        path = self.root / "registry.json"
        path.write_text(json.dumps(data))
        return fetch.load_registry(path)

    def test_registry_and_default(self):
        self.assertEqual(set(fetch.load_registry()), {"T", "X", "D", "M"})
        self.assertEqual(self.registry({"S": {"asset": "s.tar.gz", "root": "s/src"}})
                         ["S"]["root"], "s/src")

    def test_bad_registry(self):
        for data in [{}, [], {"S": {}}, {"S": {"asset": "../x", "root": "s"}},
                     {"S": {"asset": "s", "root": "../s"}},
                     {"S": {"asset": "s", "root": "/s"}},
                     {"S": {"asset": "s", "root": "s", "manifest_sha256": "bad"}},
                     {"../S": {"asset": "s", "root": "s"}}]:
            with self.subTest(data=data), self.assertRaises(ValueError):
                self.registry(data)

    def test_checksum_refusal(self):
        asset = self.root / "a.tar.gz"
        asset.write_bytes(b"archive")
        digest = hashlib.sha256(b"archive").hexdigest()
        sums = fetch.parse_checksums(f"{digest}  a.tar.gz\n")
        fetch.verify_checksum(asset, sums)
        asset.write_bytes(b"tampered")
        with self.assertRaisesRegex(ValueError, "checksum"):
            fetch.verify_checksum(asset, sums)
        with self.assertRaisesRegex(ValueError, "missing"):
            fetch.verify_checksum(asset, {})
        for text in ["broken", f"{digest}  a\n{digest}  a", f"{digest}  ../a"]:
            with self.assertRaises(ValueError):
                fetch.parse_checksums(text)

    def test_manifest_and_bytewise_order(self):
        (self.root / "b").write_bytes(b"B")
        (self.root / "a").write_bytes(b"A")
        (self.root / "link").symlink_to("a")
        expected = "".join(hashlib.sha256(data).hexdigest() + "  " + name + "\n"
                           for name, data in [("a", b"A"), ("b", b"B")])
        digest = hashlib.sha256(expected.encode()).hexdigest()
        self.assertEqual(fetch.manifest_hash(self.root), digest)
        fetch.verify_manifest(self.root, digest)
        with self.assertRaisesRegex(ValueError, "manifest"):
            fetch.verify_manifest(self.root, "0" * 64)

    def archive(self, members):
        archive = self.root / "archive.tar.gz"
        with tarfile.open(archive, "w:gz") as tar:
            for name, kind, content in members:
                member = tarfile.TarInfo(name)
                if kind == "link":
                    member.type = tarfile.SYMTYPE
                    member.linkname = content
                    tar.addfile(member)
                else:
                    member.size = len(content)
                    tar.addfile(member, io.BytesIO(content))
        return archive

    def test_extract_safe_and_reject_escape(self):
        archive = self.archive([("s/a", "file", b"A"), ("s/link", "link", "a")])
        fetch.extract_archive(archive, self.root / "ok")
        self.assertEqual((self.root / "ok/s/link").read_bytes(), b"A")
        for members in [[("../escape", "file", b"x")],
                        [("/escape", "file", b"x")],
                        [("s/link", "link", "../../escape")],
                        [("s/link/a", "file", b"x"), ("s/link", "link", "a")]]:
            with self.subTest(members=members), self.assertRaises(ValueError):
                fetch.extract_archive(self.archive(members), self.root / "bad")
            if (self.root / "bad").exists():
                import shutil
                shutil.rmtree(self.root / "bad")

    def test_fetch_uses_rest_and_verifies_before_publication(self):
        archive = self.archive([("s/a", "file", b"A")]).read_bytes()
        registry = {"S": {"asset": "s.tar.gz", "root": "s"}}
        sums = hashlib.sha256(archive).hexdigest() + "  s.tar.gz\n"
        assets = [dict(id=i, name=n) for i, n in enumerate(
            ["s.tar.gz", "SHA256SUMS", "CORPORA.md"], 1)]
        calls = []

        def api(endpoint, output=None):
            calls.append(endpoint)
            if endpoint.endswith("/releases"):
                return [dict(id=7, name="corpora-v1")]
            if endpoint.endswith("/7/assets"):
                return assets
            content = {1: archive, 2: sums.encode(), 3: b"public corpus"}
            output.write_bytes(content[int(endpoint.rsplit("/", 1)[1])])

        with patch.object(fetch, "gh_api", side_effect=api):
            roots = fetch.fetch_corpora(registry, ["S"], self.root / "work")
        self.assertEqual((roots["S"] / "a").read_bytes(), b"A")
        self.assertTrue(all(call.startswith("repos/shoedog/prism/") for call in calls))
        registry["S"]["manifest_sha256"] = "0" * 64
        with patch.object(fetch, "gh_api", side_effect=api), self.assertRaises(ValueError):
            fetch.fetch_corpora(registry, ["S"], self.root / "refused")
        self.assertFalse((self.root / "refused/corpora/S").exists())

    def test_release_missing_or_ambiguous(self):
        for releases in [[], [dict(id=1, name="corpora-v1")] * 2]:
            with patch.object(fetch, "gh_api", return_value=releases), self.assertRaises(ValueError):
                fetch.fetch_corpora({}, [], self.root / "work")


class MetricsTests(unittest.TestCase):
    def test_rotation(self):
        self.assertEqual(list(measure.run_order(["a", "b", "c"], ["T", "X"], 3)),
                         [(r, c, ref) for r, refs in enumerate(
                             [["a", "b", "c"], ["b", "c", "a"], ["c", "a", "b"]])
                          for c in ["T", "X"] for ref in refs])
        self.assertEqual(list(measure.run_order(["a"], ["S"], 1)), [(0, "S", "a")])

    def test_delta_signs_zero_and_missing(self):
        for before, after, verdict in [([10, 20, 30], [9, 18, 27], "holds in every pair (3 pairs)"),
                                        ([10, 20, 30], [11, 22, 33], "holds in every pair (3 pairs)"),
                                        ([10, 20, 30], [9, 22, 27], "mixed (3 pairs)"),
                                        ([10, 20, 30], [10, 20, 30], "mixed (3 pairs)"),
                                        ([10, 20, 30], [9, 20, 27], "mixed (3 pairs)")]:
            values, percents, actual = reporting.paired_delta(before, after)
            self.assertEqual(actual, verdict)
            self.assertEqual(values, [b - a for a, b in zip(before, after)])
        self.assertEqual(reporting.paired_delta([0], [1])[1], [None])
        for before, after in [([], []), ([1], []), ([None], [1])]:
            with self.assertRaises(ValueError):
                reporting.paired_delta(before, after)

    def test_phase_kinds_instances_and_unavailable(self):
        records = profile([("parse", "serial_wall", 10), ("parse", "serial_wall", 20),
                           ("parse", "parallel_work", 90)])
        values = reporting.phase_totals(records)
        self.assertEqual(values[("parse", "serial_wall")], 30)
        self.assertEqual(values[("parse", "parallel_work")], 90)
        records.insert(-1, dict(record="phase", name="rd", time_kind="parallel_work",
                                work_ns=7, status="measured"))
        self.assertEqual(reporting.phase_totals(records)[("rd", "parallel_work")], 7)
        for field, value in [("wall_ns", None), ("wall_ns", -1), ("time_kind", "future")]:
            bad = profile([("bad", "serial_wall", 1)])
            bad[2][field] = value
            with self.assertRaises(ValueError):
                reporting.phase_totals(bad)
        records.insert(-1, dict(record="phase", name="unused", time_kind="serial_wall",
                                wall_ns=0, status="not_run"))
        self.assertNotIn(("unused", "serial_wall"), reporting.phase_totals(records))

    def test_rusage_units(self):
        self.assertEqual(measure.rss_bytes(123, "linux"), 123 * 1024)
        self.assertEqual(measure.rss_bytes(123, "darwin"), 123)
        with self.assertRaises(ValueError):
            measure.rss_bytes(1, "unknown")

    def test_steal_parser(self):
        self.assertEqual(measure.parse_steal("cpu  1 2 3 4 5 6 7 88 9 10\ncpu0 0"), 88)
        for text in ["cpu0 1 2", "cpu 1 2"]:
            with self.assertRaises(ValueError):
                measure.parse_steal(text)

    def test_steal_unavailable_without_proc(self):
        self.assertIsNone(measure.read_steal("/nonexistent/proc/stat"))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "stat"
            path.write_text("cpu  1 2 3 4 5 6 7 9 0 0\n")
            self.assertEqual(measure.read_steal(path), 9)
            path.write_text("garbage")
            self.assertIsNone(measure.read_steal(path))

    def test_quiet_gate_and_timeout(self):
        with patch.object(measure.os, "getloadavg", side_effect=[(1, 0, 0), (.2, 0, 0)]), \
                patch.object(measure.time, "sleep"):
            self.assertEqual(measure.wait_quiet(.5, 10), .2)
        with patch.object(measure.os, "getloadavg", return_value=(1, 0, 0)):
            with self.assertRaisesRegex(RuntimeError, "quiet"):
                measure.wait_quiet(.5, 0)
        with patch.object(measure.os, "getloadavg", return_value=(1, 0, 0)):
            self.assertEqual(measure.wait_quiet(2, 0), 1)

    def test_loader_plain_gzip_and_refusal(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "profile.jsonl"
            data = "\n".join(json.dumps(r) for r in profile()) + "\n"
            path.write_text(data)
            self.assertEqual(load_records(path), profile())
            gz = path.with_suffix(".gz")
            with gzip.open(gz, "wt") as stream:
                stream.write(data)
            self.assertEqual(load_records(gz), profile())
            for content in ["", "bad json", '{"record":"header","schema_version":9}',
                            json.dumps(profile()[0]), '[]']:
                path.write_text(content)
                with self.assertRaises(ValueError):
                    load_records(path)

    def test_results_all_pairs_and_missing_phase(self):
        runs = []
        for round_id in range(2):
            for ref, wall, ns in [("a", 10, 100), ("b", 9, 90), ("c", 11, 110)]:
                runs.append(dict(round=round_id, corpus="S", ref=ref, wall_s=wall,
                                 peak_rss_bytes=1024, cpu_user_s=.1, cpu_system_s=.2,
                                 quiet_load=.1, steal_before=5, steal_after=7,
                                 phases=[dict(name="parse", time_kind="serial_wall", ns=ns)]))
        report = reporting.render_results(dict(os="test"), [{"ref": x, "sha256": "abc"}
                                                          for x in ["a", "b", "c"]], runs)
        for text in ["a → b", "a → c", "b → c", "insufficient pairs (2)", "serial_wall",
                     "Cloud VM wall times are not comparable with local history.", "Noise", "0.00%"]:
            self.assertIn(text, report)
        runs[1]["phases"] = []
        report = reporting.render_results({}, [{"ref": x} for x in ["a", "b", "c"]], runs)
        self.assertIn("unavailable", report)
        with self.assertRaises(ValueError):
            reporting.render_results({}, [], [])


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def child(self, code):
        path = self.root / "child.py"
        path.write_text(code)
        return [sys.executable, str(path)]

    def test_child_resources_and_failure_tail(self):
        command = self.child("print('synthetic stdout')\n")
        usage = measure.run_child(command, self.root, os.environ.copy(),
                                  self.root / "stdout", self.root / "stderr")
        self.assertEqual(usage["exit_code"], 0)
        self.assertGreater(usage["peak_rss_bytes"], 0)
        self.assertGreaterEqual(usage["cpu_user_s"], 0)
        self.assertEqual((self.root / "stdout").read_text(), "synthetic stdout\n")
        command = self.child("import sys\nprint('synthetic failure', file=sys.stderr)\nsys.exit(3)")
        with self.assertRaisesRegex(RuntimeError, "synthetic failure"):
            measure.run_child(command, self.root, os.environ.copy(),
                              self.root / "stdout", self.root / "stderr")

    def test_cli_refuses_invalid_inputs_before_build(self):
        for args in [["--rounds", "0"], ["--threads", "0"], ["--refs", "HEAD,HEAD"],
                     ["--refs", "HEAD,main", "--prism-bin", sys.executable],
                     ["--quiet-threshold", "nan"], ["--query", ""],
                     ["--corpus-dir", "broken"]]:
            result = subprocess.run([sys.executable, str(Path(measure.__file__)), *args],
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn("Compiling", result.stderr)

    def test_override_never_builds_and_preserves_artifacts(self):
        fixture = self.root / "fixture"
        fixture.mkdir()
        binary = self.root / "prism"
        binary.write_text("#!" + sys.executable + "\nimport os,json\n"
                          "data=" + repr(profile([("parse", "serial_wall", 15)])) + "\n"
                          "with open(os.environ['PRISM_PROFILE'],'w') as f:\n"
                          " for r in data: f.write(json.dumps(r)+'\\n')\nprint('DFG synthetic')\n")
        binary.chmod(0o755)
        out = self.root / "out"
        with patch.object(measure, "build_binaries", side_effect=AssertionError("must skip build")):
            measure.main(["--refs", "HEAD", "--prism-bin", str(binary),
                          "--corpora", "S", "--corpus-dir", f"S={fixture}", "--rounds", "1",
                          "--threads", "1", "--quiet-threshold", "999999", "--out", str(out)])
        self.assertIn("DFG synthetic", next((out / "runs").glob("*.stdout")).read_text())
        self.assertEqual(len(list((out / "runs").glob("*.jsonl.gz"))), 1)
        meta = json.loads(next((out / "runs").glob("*.meta.json")).read_text())
        self.assertEqual(meta["threads"], 1)
        self.assertIn("--repo", meta["command"])
        self.assertEqual(meta["phases"][0]["ns"], 15)
        self.assertTrue((out / "RESULTS.md").exists())


if __name__ == "__main__":
    unittest.main()
