"""Review-r1 regressions: synthetic children and mocked git/build operations only."""
from collections import defaultdict
import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import measure
import reporting
from test_cloud import profile


class RunnerFixTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.fixture = self.root / "fixture"
        self.fixture.mkdir()
        self.binary = self.root / "prism"
        self.out = self.root / "out"
        self.fake_binary()

    def fake_binary(self, body=None):
        if body is None:
            body = ("import os,json\nfrom pathlib import Path\n"
                    f"data={profile([('parse', 'serial_wall', 15)])!r}\n"
                    "Path(os.environ['PRISM_PROFILE']).write_text("
                    "''.join(json.dumps(r)+'\\n' for r in data))\n")
        self.binary.write_text("#!" + sys.executable + "\n" + body)
        self.binary.chmod(0o755)

    def args(self, *extra):
        return ["--refs", "synthetic", "--prism-bin", str(self.binary),
                "--corpora", "S", "--corpus-dir", f"S={self.fixture}",
                "--rounds", "1", "--quiet-threshold", "999999",
                "--quiet-wait", "0", "--out", str(self.out), *extra]

    def report(self):
        return (self.out / "RESULTS.md").read_text()

    def metas(self):
        return [json.loads(p.read_text()) for p in sorted((self.out / "runs").glob("*.meta.json"))]

    def test_all_paths_resolved_in_parser(self):
        args = measure.parse_args(["--out", "out", "--work-dir", "work", "--repo", ".",
                                   "--prism-bin", "bin/prism", "--corpus-dir", "S=fixture",
                                   "--corpus-dir", "U=second"])
        for name, value in [("out", "out"), ("work_dir", "work"), ("repo", "."),
                            ("prism_bin", "bin/prism")]:
            self.assertEqual(getattr(args, name), Path(value).resolve())
        self.assertEqual(args.overrides, {"S": Path("fixture").resolve(),
                                          "U": Path("second").resolve()})
        defaults = measure.parse_args([])
        self.assertTrue(defaults.out.is_absolute())
        self.assertIsNone(defaults.prism_bin)

    def test_relative_out_profile_written_from_child_cwd(self):
        before = Path.cwd()
        try:
            os.chdir(self.root)
            measure.main(self.args("--out", "out"))
        finally:
            os.chdir(before)
        self.assertEqual(len(self.metas()), 1)
        self.assertTrue(Path(self.metas()[0]["command"][0]).is_absolute())
        self.assertIn("unknown (prism-bin)", self.report())

    def test_missing_profile_is_clear_failure_and_partial(self):
        self.fake_binary("print('prism warning: profile unavailable')\n")
        with self.assertRaisesRegex(SystemExit, "missing profile file"):
            measure.main(self.args())
        self.assertIn("PARTIAL", self.report())
        self.assertIn("missing profile file", self.report())
        self.assertEqual(self.metas(), [])

    def test_stale_profile_does_not_mask_missing_output(self):
        runs = self.out / "runs"
        runs.mkdir(parents=True)
        data = "".join(json.dumps(r) + "\n" for r in profile())
        # Cover both the old and indexed artifact names when run against the base.
        for name in ("r0-S-synthetic", "r0-S-synthetic-0"):
            (runs / f"{name}.profile.jsonl").write_text(data)
        self.fake_binary("print('no new profile')\n")
        with self.assertRaisesRegex(SystemExit, "missing profile file"):
            measure.main(self.args())
        self.assertIn("PARTIAL", self.report())
        self.assertEqual(self.metas(), [])

    def test_finished_runs_survive_later_child_failure(self):
        real = measure.run_child
        count = 0

        def child(*args):
            nonlocal count
            count += 1
            if count == 2:
                self.assertIn("measurement in progress", self.report())
                self.assertIn("| 0 | S | synthetic |", self.report())
                raise RuntimeError("synthetic second run failed")
            return real(*args)

        with patch.object(measure, "run_child", side_effect=child):
            with self.assertRaisesRegex(SystemExit, "synthetic second run failed"):
                measure.main(self.args("--rounds", "2"))
        self.assertEqual(len(self.metas()), 1)
        self.assertIn("PARTIAL", self.report())
        self.assertIn("synthetic second run failed", self.report())
        self.assertIn("| 0 | S | synthetic |", self.report())
        self.assertEqual(len(list((self.out / "runs").glob("*.jsonl.gz"))), 1)

    def test_setup_abort_writes_empty_partial(self):
        with self.assertRaisesRegex(SystemExit, "corpus directory missing"):
            measure.main(self.args("--corpus-dir", f"S={self.root / 'absent'}"))
        self.assertIn("PARTIAL", self.report())
        self.assertIn("corpus directory missing", self.report())

    def test_build_abort_preserves_warning_in_partial(self):
        def build(*args, **kwargs):
            measure.resolve_ref(self.root, "main", kwargs["warnings"])
            raise RuntimeError("synthetic build failure")

        def git(repo, *args):
            if args[0] == "symbolic-ref":
                return "refs/heads/feature"
            if args[0] == "merge-base":
                return ""
            return "remote" if args[-1] == "origin/main^{commit}" else "local"

        args = self.args()
        i = args.index("--prism-bin")
        del args[i:i + 2]
        with patch.object(measure, "git", side_effect=git), \
                patch.object(measure, "build_binaries", side_effect=build), \
                contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaisesRegex(SystemExit, "synthetic build failure"):
                measure.main(args)
        self.assertIn("origin/main", self.report())
        self.assertIn("synthetic build failure", self.report())

    def test_interrupt_writes_partial_and_preserves_interrupt(self):
        with patch.object(measure, "run_child", side_effect=KeyboardInterrupt):
            with self.assertRaises(KeyboardInterrupt):
                measure.main(self.args())
        self.assertIn("PARTIAL", self.report())
        self.assertIn("KeyboardInterrupt", self.report())

    def test_malformed_profile_abort_is_partial(self):
        self.fake_binary("import os\nopen(os.environ['PRISM_PROFILE'],'w').write('bad json')\n")
        with self.assertRaises(SystemExit):
            measure.main(self.args())
        self.assertIn("PARTIAL", self.report())
        self.assertEqual(self.metas(), [])

    def test_quiet_timeout_default_records_actual_load_and_continues(self):
        with patch.object(measure.os, "getloadavg", side_effect=[(.123, .4, .5), (9, 0, 0), (.1, 0, 0)]):
            measure.main(self.args("--quiet-threshold", ".5", "--rounds", "2"))
        meta, quiet = self.metas()
        self.assertTrue(meta["gate_failed"])
        self.assertEqual(meta["quiet_load"], 9)
        self.assertFalse(quiet["gate_failed"])
        self.assertIsNone(quiet["gate_failure_reason"])
        self.assertIn("gate_failed", self.report())
        self.assertIn("host not quiet", self.report())
        self.assertIn("loadavg_at_start: 0.12 0.40 0.50", self.report())
        self.assertNotIn("PARTIAL", self.report())

    def test_quiet_timeout_fail_preserves_finished_run(self):
        with patch.object(measure.os, "getloadavg", side_effect=[(.1, 0, 0), (.1, 0, 0), (9, 0, 0)]):
            with self.assertRaisesRegex(SystemExit, "host not quiet"):
                measure.main(self.args("--rounds", "2", "--quiet-threshold", ".5",
                                       "--quiet-on-timeout", "fail"))
        self.assertEqual(len(self.metas()), 1)
        self.assertFalse(self.metas()[0]["gate_failed"])
        self.assertIn("PARTIAL", self.report())
        self.assertIn("9.00", self.report())

    def test_sanitized_ref_collisions_keep_distinct_artifacts(self):
        binaries = [dict(ref=ref, commit=f"commit{i}", path=str(self.binary), sha256="abc")
                    for i, ref in enumerate(["topic/a", "topic_a"])]
        args = self.args("--refs", "topic/a,topic_a")
        i = args.index("--prism-bin")
        del args[i:i + 2]
        with patch.object(measure, "build_binaries", return_value=binaries):
            measure.main(args)
        self.assertEqual(len(self.metas()), 2)
        self.assertEqual(len(list((self.out / "runs").glob("*.jsonl.gz"))), 2)
        self.assertEqual({r["ref"] for r in self.metas()}, {"topic/a", "topic_a"})
        self.assertIn("commit0", self.report())
        self.assertIn("commit1", self.report())

    def test_allow_dirty_cli_controls_build_and_records_opt_in(self):
        target = self.root / "target" / "release" / "prism"
        target.parent.mkdir(parents=True)
        target.write_bytes(self.binary.read_bytes())
        target.chmod(0o755)

        def git(repo, *args):
            return " M tracked.rs" if args[0] == "status" else "head"

        args = self.args("--repo", str(self.root))
        i = args.index("--prism-bin")
        del args[i:i + 2]
        for allow in (False, True):
            with self.subTest(allow=allow), patch.object(measure, "git", side_effect=git), \
                    patch.object(measure, "resolve_ref", return_value="head"), \
                    patch.object(measure.subprocess, "run") as build:
                if not allow:
                    with self.assertRaisesRegex(SystemExit, "dirty tracked tree"):
                        measure.main(args)
                    build.assert_not_called()
                    self.assertIn("PARTIAL", self.report())
                else:
                    measure.main([*args, "--allow-dirty"])
                    build.assert_called_once()
                    self.assertEqual(len(self.metas()), 1)
                    self.assertIn("allow_dirty: True", self.report())
                    self.assertNotIn("PARTIAL", self.report())

    def test_readme_brief_balances_and_refreshes_refs(self):
        text = Path(measure.__file__).with_name("README.md").read_text()
        self.assertIn("--rounds 6", text)
        self.assertLess(text.index("git fetch origin main"), text.index("fetch_corpora.py"))
        self.assertIn("SHAs", text)
        self.assertIn("even", text)


class GitFixTests(unittest.TestCase):
    def test_ref_resolution_prefers_remote_and_warns_only_on_difference(self):
        cases = [("local", "remote", "remote", True), ("same", "same", "same", False),
                 ("local", None, "local", False), (None, "remote", "remote", False),
                 (None, None, None, False)]
        for local, remote, expected, warning in cases:
            with self.subTest(local=local, remote=remote):
                def git(repo, *args):
                    if args[0] == "symbolic-ref":
                        return "refs/heads/feature"
                    if args[0] == "merge-base":
                        return ""
                    value = remote if args[-1] == "origin/main^{commit}" else local
                    if value is None:
                        raise subprocess.CalledProcessError(1, "git")
                    return value

                stderr = io.StringIO()
                with patch.object(measure, "git", side_effect=git), contextlib.redirect_stderr(stderr):
                    if expected is None:
                        with self.assertRaisesRegex(ValueError, "cannot resolve"):
                            measure.resolve_ref("repo", "main")
                    else:
                        self.assertEqual(measure.resolve_ref("repo", "main"), expected)
                self.assertEqual(bool(stderr.getvalue()), warning)
                if warning:
                    self.assertIn("origin/main", stderr.getvalue())
                    self.assertIn("local", stderr.getvalue())
                    self.assertIn("remote", stderr.getvalue())

    def test_reused_tree_head_mismatch_refuses_before_build(self):
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            tree = work / "worktrees" / "main"
            tree.mkdir(parents=True)

            def git(repo, *args):
                return "wrong" if Path(repo) == tree else "head"

            with patch.object(measure, "git", side_effect=git), \
                    patch.object(measure, "resolve_ref", return_value="wanted"), \
                    patch.object(measure, "sha256_file", return_value="digest"), \
                    patch.object(measure.subprocess, "run") as build:
                with self.assertRaisesRegex(ValueError, "HEAD.*wanted"):
                    measure.build_binaries(["main"], work / "repo", work)
                build.assert_not_called()

    def test_dirty_head_requires_opt_in_and_clean_tree_builds(self):
        for dirty, allow in [(True, False), (True, True), (False, False)]:
            with self.subTest(dirty=dirty, allow=allow):
                def git(repo, *args):
                    if args[0] == "status":
                        self.assertIn("--untracked-files=no", args)
                        return " M tracked.rs" if dirty else ""
                    return "head"

                with patch.object(measure, "git", side_effect=git), \
                        patch.object(measure, "resolve_ref", return_value="head"), \
                        patch.object(measure, "sha256_file", return_value="digest"), \
                        patch.object(measure.subprocess, "run") as build:
                    if dirty and not allow:
                        with self.assertRaisesRegex(ValueError, "dirty tracked tree"):
                            measure.build_binaries(["HEAD"], Path("repo"), Path("work"))
                        build.assert_not_called()
                    else:
                        result = measure.build_binaries(["HEAD"], Path("repo"), Path("work"),
                                                        allow_dirty=allow)
                        self.assertEqual(result[0]["commit"], "head")
                        build.assert_called_once()


class ReportFixTests(unittest.TestCase):
    def run_record(self, ref, i, wall, phase=1):
        return dict(round=i, corpus="S", ref=ref, wall_s=wall, peak_rss_bytes=1024,
                    cpu_user_s=0, cpu_system_s=0, quiet_load=.1, phases=[
                        dict(name="parse", time_kind="serial_wall", ns=phase)])

    def test_small_pair_verdicts_and_counts(self):
        for n in (1, 2, 3):
            for before, after, expected in [([10] * n, [9] * n, "holds in every pair"),
                                             ([10] * n, [11] * n, "holds in every pair"),
                                             ([10] * n, [10] * n, "mixed")]:
                verdict = reporting.paired_delta(before, after)[2]
                self.assertEqual(verdict, f"insufficient pairs ({n})" if n < 3
                                 else f"{expected} ({n} pairs)")
        self.assertEqual(reporting.paired_delta([10, 10, 10], [9, 11, 9])[2], "mixed (3 pairs)")

    def test_metric_median_delta_uses_base_median_with_wall_spread_beside_it(self):
        group = defaultdict(dict)
        for i, (before, after) in enumerate(zip([10, 20, 30], [11, 28, 36])):
            group[("S", "a")][i] = self.run_record("a", i, before, before * 1_000_000)
            group[("S", "b")][i] = self.run_record("b", i, after, after * 1_000_000)
        rows = reporting.pair_rows(group, "S", "a", "b", [0, 1, 2])
        for row in rows:
            self.assertIn("+30.0%", row)  # median([1,8,6])/median([10,20,30]); median percent is 20
            self.assertIn("100.00%", row)
            self.assertIn("holds in every pair (3 pairs); 0 of 6 runs gate-failed", row)
        report = reporting.render_results({}, [{"ref": "a"}, {"ref": "b"}],
                                          [r for values in group.values() for r in values.values()])
        self.assertIn("median delta / base median", report)
        self.assertIn("base wall spread", report)

    def test_no_pairs_and_zero_base_are_explicit(self):
        row = reporting.delta_cell([], [])
        self.assertIn("insufficient pairs (0); 0 of 0 runs gate-failed", row)
        self.assertIn("n/a", reporting.delta_cell([0, 0, 0], [1, 1, 1]))

    def test_missing_metric_pairs_are_counted_and_zero_wall_is_defined(self):
        row = reporting.delta_cell([10, None, 30], [9, 20, None])
        self.assertIn("insufficient pairs (1); 0 of 2 runs gate-failed", row)
        self.assertIn("-10.0%", row)
        self.assertEqual(reporting.spread({0: self.run_record("a", 0, 0)}), "n/a")
        self.assertEqual(reporting.spread({}), "unavailable")
        with self.assertRaisesRegex(ValueError, "equal-length"):
            reporting.delta_cell([1], [])

    def test_partial_missing_ref_does_not_drop_other_complete_pairs(self):
        runs = [self.run_record(ref, 0, wall) for ref, wall in [("a", 10), ("b", 9)]]
        report = reporting.render_results({}, [{"ref": x} for x in "abc"], runs,
                                          failure="stopped before c")
        self.assertIn("insufficient pairs (1)", report)
        self.assertIn("insufficient pairs (0)", report)
        self.assertIn("stopped before c", report)

    def test_rss_floor_footnote(self):
        report = reporting.render_results({}, [{"ref": "a"}], [self.run_record("a", 0, 1)])
        for text in ["wait4", "floor", "parent's size"]:
            self.assertIn(text, report)


if __name__ == "__main__":
    unittest.main()
