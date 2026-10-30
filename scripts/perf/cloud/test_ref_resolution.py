"""Review-r2 regressions: real local Git repositories and synthetic measurements."""
import contextlib
import io
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import measure
import reporting
import test_measure_contracts as fixtures


class RefResolutionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.remote = self.root / "origin.git"
        self.command("init", "-q", "-b", "main", str(self.repo))
        self.command("init", "-q", "--bare", "-b", "main", str(self.remote))
        self.git("config", "user.name", "Synthetic fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        self.base = self.commit("base")
        self.git("remote", "add", "origin", str(self.remote))
        self.git("push", "-q", "origin", "main")  # Local file transport only.
        self.git("remote", "set-head", "origin", "main")
        self.git("checkout", "-q", "-b", "feature")
        self.feature = self.commit("feature")

    def command(self, *args):
        return subprocess.run(["git", *args], check=True, text=True,
                              capture_output=True).stdout.strip()

    def git(self, *args):
        return self.command("-C", str(self.repo), *args)

    def commit(self, name):
        self.git("-c", "commit.gpgsign=false", "commit", "-q", "--allow-empty", "-m", name)
        return self.git("rev-parse", "HEAD")

    def resolve(self, ref, expected, warning=False):
        warnings, stderr = [], io.StringIO()
        with contextlib.redirect_stderr(stderr):
            actual = measure.resolve_ref(self.repo, ref, warnings)
        self.assertEqual(actual, expected)
        self.assertEqual(bool(warnings), warning)
        self.assertEqual(bool(stderr.getvalue()), warning)
        for message in warnings:
            self.assertIn(message, stderr.getvalue())
            report = reporting.render_results({}, [], [], failure="fixture", warnings=warnings)
            self.assertIn(message, report)
        return warnings

    def test_head_on_feature_does_not_follow_origin_head(self):
        self.assertEqual(self.git("rev-parse", "origin/HEAD"), self.base)
        self.resolve("HEAD", self.feature)

    def test_full_and_short_sha_ignore_origin_shadow(self):
        for ref in (self.feature, self.feature[:7]):
            with self.subTest(ref=ref):
                self.git("update-ref", f"refs/remotes/origin/{ref}", self.base)
                self.resolve(ref, self.feature)

    def test_explicit_origin_and_refs_ignore_origin_shadow(self):
        cases = [("origin/main", self.base), ("refs/heads/feature", self.feature)]
        for ref, expected in cases:
            with self.subTest(ref=ref):
                shadow = self.feature if expected == self.base else self.base
                self.git("update-ref", f"refs/remotes/origin/{ref}", shadow)
                self.resolve(ref, expected)

    def test_explicit_ref_and_sha_never_fall_back_to_origin(self):
        for ref in ("origin/missing", "refs/heads/missing", "abcdef0"):
            with self.subTest(ref=ref):
                self.git("update-ref", f"refs/remotes/origin/{ref}", self.base)
                with self.assertRaisesRegex(ValueError, "cannot resolve"):
                    measure.resolve_ref(self.repo, ref)

    def test_main_only_at_origin_resolves_without_warning(self):
        self.git("branch", "-D", "main")
        self.resolve("main", self.base)

    def test_stale_local_main_uses_origin_and_warns(self):
        self.git("push", "-q", "origin", "feature:main")
        warnings = self.resolve("main", self.feature, warning=True)
        for value in (self.base, self.feature, "stale", "using origin/main"):
            self.assertIn(value, warnings[0])

    def assert_ambiguous(self):
        local, remote = self.git("rev-parse", "main"), self.git("rev-parse", "origin/main")
        with self.assertRaises(ValueError) as error:
            measure.resolve_ref(self.repo, "main")
        for value in (local, remote, "origin/main", "SHA"):
            self.assertIn(value, str(error.exception))

    def test_local_ahead_on_non_current_branch_refuses(self):
        self.git("branch", "-f", "main", self.feature)
        self.assert_ambiguous()

    def test_current_branch_ahead_uses_local_and_warns(self):
        self.git("checkout", "-q", "main")
        local = self.commit("local ahead")
        warnings = self.resolve("main", local, warning=True)
        for value in (local, self.base, "checked-out", "using main"):
            self.assertIn(value, warnings[0])

    def test_current_branch_stale_or_diverged_still_uses_local(self):
        self.git("push", "-q", "origin", "feature:main")
        self.git("checkout", "-q", "main")
        with self.subTest(state="stale"):
            self.resolve("main", self.base, warning=True)
        local = self.commit("current branch diverged")
        with self.subTest(state="diverged"):
            self.resolve("main", local, warning=True)

    def test_diverged_non_current_branch_refuses(self):
        self.git("checkout", "-q", "main")
        self.commit("local divergence")
        self.git("checkout", "-q", "feature")
        self.git("push", "-q", "origin", "feature:main")
        self.assert_ambiguous()

    def test_detached_head_at_local_branch_commit_still_refuses(self):
        self.git("branch", "-f", "main", self.feature)
        self.git("checkout", "-q", "--detach", self.feature)
        self.assert_ambiguous()

    def test_equal_refs_and_local_only_do_not_warn(self):
        self.resolve("main", self.base)
        self.resolve("feature", self.feature)

    def test_all_refs_resolved_before_any_build(self):
        self.git("branch", "-f", "main", self.feature)
        real_run, builds = subprocess.run, []

        def run(command, *args, **kwargs):
            if command[0] != "git":
                builds.append(command)
                raise AssertionError("must refuse before any build")
            return real_run(command, *args, **kwargs)

        # Git calls are real; intercept non-Git commands so no cargo can execute.
        with patch.object(measure.subprocess, "run", side_effect=run):
            with self.assertRaisesRegex(ValueError, "origin/main"):
                measure.build_binaries(["HEAD", "main"], self.repo, self.root / "work")
        self.assertEqual(builds, [])


class GateReportTests(unittest.TestCase):
    run_record = fixtures.ReportFixTests.run_record

    def report(self, runs, refs="ab"):
        return reporting.render_results({}, [{"ref": ref} for ref in refs], runs)

    def test_header_and_each_metric_count_only_its_complete_pairs(self):
        runs = [self.run_record(ref, i, 10 if ref == "a" else 9)
                for ref in "ab" for i in range(3)]
        runs[0]["gate_failed"] = True
        runs[4]["gate_failed"] = True
        runs[4]["phases"] = []  # Exclude both sides of this pair from the phase verdict.
        orphan = self.run_record("a", 99, 1)
        orphan["gate_failed"] = True
        report = self.report(runs + [orphan])
        self.assertIn("## Paired deltas (2 of 6 runs gate-failed)", report)
        self.assertIn("### S: a → b (2 of 6 runs gate-failed)", report)
        wall = next(line for line in report.splitlines() if line.startswith("| command wall"))
        phase = next(line for line in report.splitlines() if line.startswith("| parse ("))
        self.assertIn("holds in every pair (3 pairs); 2 of 6 runs gate-failed", wall)
        self.assertIn("insufficient pairs (2); 1 of 4 runs gate-failed", phase)

    def test_header_counts_unique_runs_across_multiple_ref_pairs(self):
        runs = [self.run_record(ref, i, 10) for ref in "abc" for i in range(3)]
        runs[0]["gate_failed"] = True
        report = self.report(runs, "abc")
        self.assertIn("## Paired deltas (1 of 9 runs gate-failed)", report)
        self.assertIn("### S: b → c (0 of 6 runs gate-failed)", report)
        self.assertIn("mixed (3 pairs); 0 of 6 runs gate-failed", report)

    def test_no_shared_rounds_and_no_metric_pairs_have_zero_runs(self):
        runs = [self.run_record("a", 0, 10), self.run_record("b", 1, 9)]
        runs[0]["gate_failed"] = True
        report = self.report(runs)
        self.assertIn("## Paired deltas (0 of 0 runs gate-failed)", report)
        self.assertIn("insufficient pairs (0); 0 of 0 runs gate-failed", report)
        runs[1]["round"] = 0
        runs[1]["phases"] = []
        report = self.report(runs)
        self.assertIn("insufficient pairs (1); 1 of 2 runs gate-failed", report)
        self.assertIn("insufficient pairs (0); 0 of 0 runs gate-failed", report)
        with self.assertRaisesRegex(ValueError, "equal-length"):
            reporting.delta_cell([10], [9], gate_failures=[])
        runs[1]["gate_failed"] = True
        self.assertIn("insufficient pairs (1); 2 of 2 runs gate-failed", self.report(runs))


class GateWaitTests(unittest.TestCase):
    setUp = fixtures.RunnerFixTests.setUp
    fake_binary = fixtures.RunnerFixTests.fake_binary
    args = fixtures.RunnerFixTests.args
    metas = fixtures.RunnerFixTests.metas
    report = fixtures.RunnerFixTests.report

    def test_timeout_caps_all_later_gates_and_records_effective_wait(self):
        effects = [.1, measure.QuietTimeout(9, .5, 300), .1,
                   measure.QuietTimeout(8, .5, 60)]
        with patch.object(measure, "wait_quiet", side_effect=effects) as gate:
            measure.main(self.args("--rounds", "4", "--quiet-wait", "300"))
        self.assertEqual([c.args[1] for c in gate.call_args_list], [300, 300, 60, 60])
        metas = self.metas()
        self.assertEqual([r["quiet_wait_s"] for r in metas], [300, 300, 60, 60])
        self.assertEqual([r["quiet_wait_reduced"] for r in metas], [False, False, True, True])
        self.assertEqual([r["gate_failed"] for r in metas], [False, True, False, True])
        for text in ("gate wait limit s", "gate wait reduced", "later quiet gates", "60"):
            self.assertIn(text, self.report())

    def test_short_wait_never_increases_after_timeout(self):
        for wait in (0, 10, 60):
            with self.subTest(wait=wait), patch.object(measure, "wait_quiet", side_effect=[
                    measure.QuietTimeout(9, .5, wait), .1]) as gate:
                measure.main(self.args("--rounds", "2", "--quiet-wait", str(wait)))
                self.assertEqual([c.args[1] for c in gate.call_args_list], [wait, wait])
                self.assertEqual([r["quiet_wait_s"] for r in self.metas()], [wait, wait])
                self.assertFalse(any(r["quiet_wait_reduced"] for r in self.metas()))

    def test_no_timeout_keeps_configured_wait(self):
        with patch.object(measure, "wait_quiet", return_value=.1) as gate:
            measure.main(self.args("--rounds", "2", "--quiet-wait", "300"))
        self.assertEqual([c.args[1] for c in gate.call_args_list], [300, 300])
        self.assertEqual([r["quiet_wait_s"] for r in self.metas()], [300, 300])
        self.assertFalse(any(r["quiet_wait_reduced"] for r in self.metas()))

    def test_wait_cap_resets_for_next_measurement(self):
        with patch.object(measure, "wait_quiet", side_effect=[
                measure.QuietTimeout(9, .5, 300), .1, .1]) as gate:
            measure.main(self.args("--rounds", "2", "--quiet-wait", "300"))
            measure.main(self.args("--quiet-wait", "300", "--out", str(self.root / "next")))
        self.assertEqual([c.args[1] for c in gate.call_args_list], [300, 60, 300])


if __name__ == "__main__":
    unittest.main()
