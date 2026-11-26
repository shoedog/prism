from __future__ import annotations

import gzip
import json
from pathlib import Path
import subprocess
import sys

from .helpers import MEASURE, Workbench, sha256_of, write_tree


QUIET = {"kind": "host_quiet", "launched": False, "child_exit": None}


class RunTests(Workbench):
    def test_run_writes_dumps_and_receipt(self):
        self.assert_exit(self.run_kit("--sites", "--trace-env", "PRISM_TRACE=a=b"), 0)
        receipt = self.receipt()
        snapshot = Path(receipt["corpus"]["root"])
        self.assertNotEqual(snapshot, self.source)
        self.assertEqual((snapshot / "a.ts").read_bytes(), (self.source / "a.ts").read_bytes())
        self.assertEqual(receipt["schema"], "prism.measure.receipt/1")
        self.assertEqual(receipt["status"], "ok")
        self.assertEqual((receipt["binary"]["sha256"], receipt["corpus"]["commit"]),
                         (sha256_of(self.fake), self.commit))
        self.assertEqual([c["id"] for c in receipt["commands"]],
                         ["dfg-stats", "call-stats", "edges", "sites"])
        self.assertEqual(receipt["commands"][2]["env"],
                         {"PRISM_EDGES_BYTES": "1", "PRISM_TRACE": "a=b"})
        self.assertEqual(receipt["version_command"]["stdout_sha256"],
                         sha256_of(self.out / "version.stdout"))
        self.assertIsNone(receipt["gate_sha256"])
        self.assertIsNone(receipt["gate_step_id"])
        self.assertEqual(receipt["corpus"]["input_sha256"], receipt["corpus"]["end_input_sha256"])
        self.assertEqual(set(receipt["artifacts"]), {
            "version.stdout", "version.stderr", "dfg-stats.json", "dfg-stats.stderr",
            "call-stats.json", "call-stats.stderr", "edges.jsonl.gz", "edges.stderr.gz",
            "sites.jsonl.gz", "sites.stderr",
        })
        for name, artifact in receipt["artifacts"].items():
            with self.subTest(artifact=name):
                path = self.out / name
                self.assertEqual(artifact, {"sha256": sha256_of(path), "bytes": path.stat().st_size})
        self.assertEqual(gzip.decompress((self.out / "edges.jsonl.gz").read_bytes()), b'{"count":1}\n')
        self.assertEqual(gzip.decompress((self.out / "edges.stderr.gz").read_bytes()), b"edges\n")

    def test_default_edge_environment_only_sets_bytes(self):
        self.assert_exit(self.run_kit(), 0)
        self.assertEqual(self.receipt()["commands"][2]["env"], {"PRISM_EDGES_BYTES": "1"})
        self.assertIn("edges.stderr", self.receipt()["artifacts"])

    def test_run_refuses_corpus_with_pin_mismatch(self):
        self.commit_files({"new.ts": "extra"})
        self.assert_exit(self.run_kit(), 3)
        self.assertEqual(self.fake_log(), [])
        self.assertIsNone(self.receipt())

    def test_run_refuses_dirty_corpus(self):
        write_tree(self.source, {"new.ts": "extra"})
        self.assert_exit(self.run_kit(), 3)
        self.assertEqual(self.fake_log(), [])

    def test_run_refuses_ignored_file_under_root(self):
        self.entry["commit"] = self.commit_files({".gitignore": "generated.ts\n"})
        self.write_manifest()
        write_tree(self.source, {"generated.ts": "extra"})
        self.assert_exit(self.run_kit(), 3)
        self.assertEqual(self.fake_log(), [])

    def test_input_change_during_run_fails(self):
        self.make_fake({"edges": "(root / 'extra.ts').write_text('extra')"})
        self.assert_exit(self.run_kit(), 4)
        self.assertEqual(self.receipt()["status"], "input_changed")
        self.assertFalse((self.source / "extra.ts").exists())

    def test_directory_creation_invalidates_run(self):
        self.make_fake({"edges": "(root / 'generated').mkdir()"})
        self.assert_exit(self.run_kit(), 4)
        receipt = self.receipt()
        self.assertEqual(receipt["status"], "input_changed")
        self.assertEqual(receipt["corpus"]["input_sha256"], receipt["corpus"]["end_input_sha256"])

    def test_permission_change_invalidates_run(self):
        self.fresh_tree()
        self.make_fake({"edges": "(root / 'a.ts').chmod(0o755)"})
        self.assert_exit(self.run_kit(), 4)
        self.assertEqual(self.receipt()["status"], "input_changed")

    def test_link_resolution_change_invalidates_run(self):
        self.fresh_tree(files={"pkg/brew/input": "a"}, links={
            "HomebrewFormula": "pkg/brew", ".env": "shared/.env", "dynamic": "generated",
        })
        self.assert_exit(self.verify(), 0)
        self.assert_exit(self.run_kit(out=self.work / "control"), 0)
        self.make_fake({"edges": "(root / 'generated').mkdir()"})
        self.assert_exit(self.run_kit(), 4)
        corpus = self.receipt()["corpus"]
        self.assertEqual(self.receipt()["status"], "input_changed")
        self.assertEqual(corpus["input_sha256"], corpus["end_input_sha256"])
        self.assertNotEqual(corpus["input_state_sha256"], corpus["end_input_state_sha256"])

    def test_snapshot_excludes_repository_metadata_and_sibling_sources(self):
        self.entry.update(root="src", commit=self.commit_files({
            "src/a.ts": "import '../shared/helper';\n", "shared/helper.ts": "unbound\n",
        }))
        self.write_manifest()
        self.make_fake({"edges": "assert not (root / '.git/config').exists(); "
                                  "assert not (root.parent / 'shared/helper.ts').exists()"})
        self.assert_exit(self.run_kit(), 0)

    def test_original_change_during_run_fails(self):
        target = self.source / "a.ts"
        self.make_fake({"edges": f"Path({str(target)!r}).write_text('changed')"})
        self.assert_exit(self.run_kit(), 4)
        self.assertEqual(self.receipt()["status"], "input_changed")

    def test_receipt_binds_prediction_before_first_command(self):
        prediction = self.work / "prediction.md"
        prediction.write_bytes(b"original\r\n")
        digest = sha256_of(prediction)
        self.make_fake({"version": f"Path({str(prediction)!r}).write_text('replacement')"})
        self.assert_exit(self.run_kit("--prediction", prediction), 0)
        self.assertEqual(self.receipt()["prediction"]["sha256"], digest)
        self.assertNotEqual(sha256_of(prediction), digest)

    def test_gate_supplies_frozen_custody_on_success_and_failure(self):
        digest, step = "1" * 64, "measure-base"
        prediction = self.work / "gate-prediction.md"
        prediction.write_bytes(b"frozen prediction\n")
        prediction_digest = sha256_of(prediction)
        expression = ("import sys; from measure.cli import main; "
                      f"sys.exit(main(sys.argv[1:], gate_sha256={digest!r}, "
                      f"gate_step_id={step!r}, prediction_sha256={prediction_digest!r}))")
        for failed in (False, True):
            with self.subTest(failed=failed):
                self.out = self.work / f"gate-{failed}"
                self.make_fake({"edges": "sys.exit(7)"} if failed else {})
                result = subprocess.run(
                    [sys.executable, "-c", expression, "--manifest", str(self.manifest),
                     "run", "--binary", str(self.fake), "--corpus", self.key, "--out", str(self.out),
                      "--memguard", str(self.guard), "--corpora-root", str(self.work),
                     "--prediction", str(prediction)],
                    cwd=MEASURE.parent, env=self.env, capture_output=True, text=True, timeout=20,
                )
                self.assert_exit(result, 4 if failed else 0)
                receipt = self.receipt()
                self.assertEqual((receipt["gate_sha256"], receipt["gate_step_id"]),
                                 (digest, step))
                self.assertEqual(receipt["prediction"]["sha256"], prediction_digest)

    def test_trace_env_applies_to_edges_run_only(self):
        self.env.update(PRISM_EDGES_BYTES="1", PRISM_DEBUG="inherited", MEMGUARD_IGNORE_QUIET="1",
                        UNRELATED="private", LC_ALL="en_US.UTF-8")
        self.assert_exit(self.run_kit("--trace-env", "PRISM_RD_CENSUS_TRACE=1", "--no-bytes"), 0)
        attempts = self.fake_log()
        self.assertEqual(["PRISM_RD_CENSUS_TRACE" in c["env"] for c in attempts],
                         [False, False, False, True])
        for attempt in attempts:
            for name in ("PRISM_DEBUG", "PRISM_EDGES_BYTES", "MEMGUARD_IGNORE_QUIET", "UNRELATED"):
                self.assertNotIn(name, attempt["env"])
            self.assertEqual(attempt["env"]["LC_ALL"], "C")

    def test_invalid_trace_overrides_refuse_before_launch(self):
        for flags in (("PRISM_EDGES_BYTES=1",), ("PRISM_TRACE",), ("HOME=value",),
                      ("PRISM_TRACE=1", "PRISM_TRACE=2")):
            with self.subTest(flags=flags):
                args = [value for flag in flags for value in ("--trace-env", flag)]
                self.assert_exit(self.run_kit(*args), 2)
        self.assertEqual(self.fake_log(), [])

    def test_guard_refusal_exits_75(self):
        for command in ("version", "dfg-stats"):
            with self.subTest(command=command):
                self.log.unlink(missing_ok=True)
                self.make_guard({command: (75, QUIET)})
                self.assert_exit(self.run_kit(), 75)
                self.assertIsNone(self.receipt())
                self.assertEqual([c["command"] for c in self.fake_log()],
                                 [] if command == "version" else ["version"])
        self.make_guard()
        self.make_fake({"call-stats": "sys.exit(75)"})
        self.assert_exit(self.run_kit(), 4)
        self.assertEqual(self.receipt()["commands"][1]["guard_status"],
                         {"kind": "child", "launched": True, "child_exit": 75})

    def test_late_quiet_preserves_failed_receipt(self):
        self.make_guard({"call-stats": (75, QUIET)})
        self.assert_exit(self.run_kit(), 75)
        receipt = self.receipt()
        self.assertEqual(receipt["status"], "failed")
        self.assertEqual(receipt["commands"][-1]["guard_status"], QUIET)
        self.assertEqual([c["command"] for c in self.fake_log()], ["version", "dfg-stats"])
        self.assertTrue((self.out / "dfg-stats.json").is_file())

    def test_early_command_or_guard_failure_has_no_receipt(self):
        self.make_fake({"version": "sys.exit(7)"})
        self.assert_exit(self.run_kit(), 4)
        self.assertIsNone(self.receipt())
        self.make_fake()
        for command in ("version", "dfg-stats"):
            for code, kind in ((76, "rss_unavailable"), (1, "guard_error")):
                with self.subTest(command=command, kind=kind):
                    self.make_guard({command: (code, dict(kind=kind, launched=False, child_exit=None))})
                    self.assert_exit(self.run_kit(), 4)
                    self.assertIsNone(self.receipt())

    def test_bad_guard_status_never_yields_quiet(self):
        for status in (None, "broken\n", "{}\n{}\n",
                       '{"kind":"host_quiet","launched":false,"child_exit":null,"kind":"host_quiet"}\n',
                       dict(kind="host_quiet", launched=True, child_exit=75),
                       dict(kind="child", launched=True, child_exit=76)):
            with self.subTest(status=status):
                self.make_guard({"version": (75, status)})
                self.assert_exit(self.run_kit(), 4)
                self.assertIsNone(self.receipt())

    def test_version_mutation_takes_precedence_over_pin_mismatch(self):
        self.make_fake({"version": "with open(sys.argv[0], 'a') as f: f.write('# drift\\n')"})
        self.assert_exit(self.run_kit("--expect-version", "other-version"), 4)
        self.assertEqual([c["command"] for c in self.fake_log()], ["version"])
        self.assertIsNone(self.receipt())

    def test_version_pin_uses_the_commit_token_and_minimum_length(self):
        self.make_fake(version="prism 1.2.3 build abc1234 commit def5678aa")
        for pin in ("", "d", "def567", "abc1234"):
            with self.subTest(pin=pin):
                self.assert_exit(self.run_kit("--expect-version", pin), 3)
                self.assertIsNone(self.receipt())
        self.assert_exit(self.run_kit("--expect-version", "DEF5678"), 0)

    def test_missing_unreadable_or_nonexecutable_binary_exits_four(self):
        self.assert_exit(self.run_kit(binary=self.work / "missing"), 4)
        for mode in (0o644, 0o111):
            with self.subTest(mode=mode):
                self.fake.chmod(mode)
                self.assert_exit(self.run_kit(), 4)
                self.assertIsNone(self.receipt())

    def test_output_inside_source_refuses_before_launch(self):
        self.assert_exit(self.run_kit(out=self.source / "out"), 4)
        self.assertFalse((self.source / "out").exists())
        self.assertEqual(self.fake_log(), [])

    def test_output_parent_components_cannot_alias_query_root(self):
        self.fresh_tree(files={})
        self.assert_exit(self.run_kit(out=self.source / ".." / self.source.name / "out"), 4)
        self.assertFalse((self.source / "out").exists())
        self.assertEqual(self.fake_log(), [])

    def test_receipt_admission_checks_all_artifacts_and_command_roles(self):
        from measure.receipt import admit
        self.assert_exit(self.run_kit("--trace-env", "PRISM_TRACE=1"), 0)
        self.assertEqual(admit(self.out, selected=("edges",))["status"], "ok")
        stderr = self.out / "call-stats.stderr"
        original = stderr.read_bytes()
        stderr.write_bytes(b"other")
        with self.assertRaises(ValueError):
            admit(self.out, selected=("edges",))
        stderr.write_bytes(original)
        receipt = self.receipt()
        receipt["commands"][1]["output"] = "../other"
        (self.out / "receipt.json").write_text(json.dumps(receipt), encoding="utf-8")
        with self.assertRaises(ValueError):
            admit(self.out, selected=("edges",))

    def test_receipt_admission_requires_custody_metadata(self):
        from measure.receipt import admit

        self.assert_exit(self.run_kit(), 0)
        path = self.out / "receipt.json"
        original = self.receipt()
        for key in ("guard", "prediction", "kit_commit", "start", "host"):
            with self.subTest(key=key):
                record = dict(original)
                del record[key]
                path.write_text(json.dumps(record), encoding="utf-8")
                with self.assertRaises(ValueError):
                    admit(self.out)

    def test_jsonl_admission_checks_every_gzip_member_and_strict_json(self):
        from measure.inputs import jsonl_objects
        path = self.work / "objects.jsonl.gz"
        valid = gzip.compress(b'{"x":1}\r\n\t\n') + gzip.compress(b'{"x":2}')
        path.write_bytes(valid)
        self.assertEqual(list(jsonl_objects(path)), [{"x": 1}, {"x": 2}])
        for data in (b"", valid[:-1], gzip.compress(b'{"x":1,"x":2}\n'),
                     gzip.compress(b'{"x":1e999}\n'), gzip.compress(b'[]\n')):
            with self.subTest(data=data):
                path.write_bytes(data)
                with self.assertRaises(ValueError):
                    list(jsonl_objects(path))
