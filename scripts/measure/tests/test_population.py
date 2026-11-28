"""S4 population registration, patch parsing and scratch checkout contracts."""

from __future__ import annotations

from collections import Counter
from contextlib import redirect_stderr, redirect_stdout
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
from unittest.mock import patch

from .helpers import DEFAULT_VERSION, MEASURE, Workbench, executable, fixture_digest, write_tree

BEFORE = "const a = 1;\nconst b = 2;\nconst c = 3;\n"
AFTER = "const a = 1;\nconst b = 2;\nconst c = 30;\nconst d = 4;\n"
EMPTY = {"algorithm": "Taint", "findings": []}
QUIET = {"kind": "host_quiet", "launched": False, "child_exit": None}
METRICS = ("presence_changes", "payload_changes", "ambiguous_bags")
SPEC_RULE = {
    "git_diff": ["-U0", "-M"],
    "extensions": [".cjs", ".cts", ".go", ".js", ".jsx", ".mjs", ".mts", ".py", ".ts", ".tsx"],
    "exclude_test": {
        "basename": r"(\.test\.|\.spec\.|\.e2e-spec\.|_test\.go$|^test_.*\.py$|_test\.py$|^conftest\.py$)",
        "path": r"(^|/)(tests?|__tests__|__mocks__|__snapshots__|testing)(/|$)",
    },
    "modify_type": "Modified",
}
POPULATION_FIELDS = {"ordinal", "repo", "commit", "parent", "tree", "spec_sha256", "n_files", "n_lines"}


def spec_for(files):
    # The expected paths and line numbers are supplied by each test.
    return {"files": [
        {"file_path": path, "modify_type": "Modified", "diff_lines": lines}
        for path, lines in sorted(files.items())
    ]}


STANDARD_SPEC = spec_for({"src/a.ts": [3, 4]})


def envelope_bytes(envelope):
    return json.dumps(envelope).encode("utf-8") + b"\n"


class ReviewFixture(Workbench):
    def make_corpus(self):
        self.clones = self.work / "clones"
        self.clones.mkdir()
        self.source = self.clones / "fixture"
        super().make_corpus(files={"src/a.ts": BEFORE, "views.py": "a = 1\nb = a\nprint(b)\n"})
        self.parent = self.commit
        self.population = self.work / "population.json"

    def make_fake(self):
        self.binaries = {}
        self.make_arm("base")
        self.make_arm("head")

    def make_arm(self, role, envelope=EMPTY, *, exit_code=0, stdout=None,
                 stderr=None, before_read="", after_read=""):
        if stdout is None:
            stdout = envelope_bytes(envelope)
        if stderr is None:
            stderr = (role + " diagnostic\n").encode("ascii")
        self.binaries[role] = executable(self.work / (role + " product"), f"""
import json
import os
from pathlib import Path
import sys

args = sys.argv[1:]
root = None
spec = None
source = None
if args == ["--version"]:
    sys.stdout.write({DEFAULT_VERSION + " fixture " + role + chr(10)!r})
    code = 0
else:
    root = Path(args[args.index("--repo") + 1])
    spec_arg = args[args.index("--diff") + 1]
    spec_path = None if spec_arg.lstrip().startswith("{{") else Path(spec_arg)
    exec({before_read!r})
    spec = spec_arg if spec_path is None else spec_path.read_bytes().decode("utf-8")
    source = (root / "src/a.ts").read_bytes().decode("utf-8")
    output, code = {stdout!r}, {exit_code!r}
    exec({after_read!r})
    sys.stdout.buffer.write(output)
    sys.stderr.buffer.write({stderr!r})
with open({str(self.log)!r}, "a", encoding="utf-8") as stream:
    print(json.dumps(dict(role={role!r}, binary=sys.argv[0], argv=args, env=dict(os.environ),
                          root=str(root) if root is not None else None, spec=spec, source=source)),
          file=stream)
sys.exit(code)
""")
        return self.binaries[role]

    def git_bytes(self, *args):
        return subprocess.check_output(
            ["git", "-C", str(self.source), *args], env=self.env, stderr=subprocess.PIPE)

    def make_population(self, *, spec=STANDARD_SPEC, edits=None, delete=(), ordinals=(1,)):
        for name in delete:
            (self.source / name).unlink()
        self.commit = self.commit_files({"src/a.ts": AFTER} if edits is None else edits)
        self.pinned_digest = fixture_digest(self.source)
        files = spec["files"]
        common = dict(repo=self.source.name, commit=self.commit, parent=self.parent,
                      tree=self.git("rev-parse", "HEAD^{tree}"),
                      spec_sha256=hashlib.sha256(json.dumps(spec).encode("utf-8")).hexdigest(),
                      n_files=len(files), n_lines=sum(len(file["diff_lines"]) for file in files))
        self.registration = dict(
            schema="prism.measure.population/1", name="S4 fixture",
            repos={self.source.name: {"url": "https://example.invalid/fixture",
                                     "selection_head": self.commit}},
            spec_rule=SPEC_RULE, diffs=[dict(common, ordinal=n) for n in ordinals])
        self.write_population()

    def write_population(self):
        self.population.write_bytes(json.dumps(self.registration).encode("utf-8"))

    def prepare_m2(self):
        self.parent = self.commit_files({
            "src/a.test.ts": "old test\n", "tests/helper.ts": "old helper\n",
            "README.md": "heading\nold text\n", "src/deleted.ts": "delete this\n",
        })
        self.make_population(
            edits={"src/a.ts": AFTER, "src/a.test.ts": "new test\n",
                   "tests/helper.ts": "new helper\n", "README.md": "heading\nnew text\n"},
            delete=("src/deleted.ts",))

    def guard_for(self, argv):
        return self.guard

    def review(self, *options, arms=None, expected=0):
        from measure.cli import main

        if arms is None:
            arms = [f"{role}={binary}" for role, binary in self.binaries.items()]
        args = ["--manifest", str(self.manifest), "review-diff", "--population", str(self.population),
                "--clones", str(self.clones), "--out", str(self.out)]
        for arm in arms:
            args.extend(("--arm", arm))
        args.extend(options)

        # Inject the Workbench guard at the existing structured-status launch seam.
        # Keep the real runner, status pipe, and child process.
        real_popen = subprocess.Popen

        def launch(argv, *positional, **kwargs):
            if isinstance(argv, (list, tuple)) and len(argv) > 4 and argv[1] == "--status-fd":
                argv = [str(self.guard_for(argv[4:])), *argv[1:]]
            return real_popen(argv, *positional, **kwargs)

        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.dict(os.environ, self.env, clear=True), patch.object(subprocess, "Popen", launch):
            with redirect_stdout(stdout), redirect_stderr(stderr):
                try:
                    code = main(args)
                except SystemExit as error:
                    code = error.code
        result = subprocess.CompletedProcess(args, code, stdout.getvalue(), stderr.getvalue())
        self.assertNotIn("invalid choice: 'review-diff'", result.stderr,
                         "missing S4 review-diff command")
        if expected is not None:
            self.assert_exit(result, expected)
        else:
            self.assertNotIn("Traceback", result.stderr)
        return result

    def read_json(self, name):
        return json.loads((self.out / name).read_bytes())

    def read_rows(self, name="per-diff.jsonl"):
        return [json.loads(line) for line in (self.out / name).read_bytes().splitlines()]

    def attempts(self):
        return [item for item in self.fake_log() if item["root"] is not None]

    def assert_measured_spec(self, spec):
        attempts = self.attempts()
        self.assertEqual([item["role"] for item in attempts], ["base", "head"])
        for attempt in attempts:
            self.assertEqual(attempt["spec"].encode("utf-8"), json.dumps(spec).encode("utf-8"))

    def assert_row_refused(self):
        row, = self.read_rows()
        self.assertEqual(row["status"], "refused")
        self.assertTrue(row["reason"])
        self.assertEqual(self.read_json("totals.json")["refused_ordinals"], [row["ordinal"]])
        self.assertEqual(self.attempts(), [])
        self.assert_null_metrics(row)

    def assert_null_metrics(self, row):
        for field in METRICS:
            self.assertIsNone(row[field], field)

    def assert_unlaunched(self, arm):
        for field in ("exit", "output", "output_sha256", "stderr", "stderr_sha256",
                      "input_after_sha256"):
            self.assertIsNone(arm[field], field)

    def keep_dirty_source(self):
        self.commit_files({"tip.txt": "source tip after candidate\n"})
        write_tree(self.source, {"src/a.ts": "unstaged source edit\n", "local.txt": "local work\n"})

    def source_state(self):
        return (self.git_bytes("rev-parse", "HEAD"), self.git_bytes("status", "--porcelain"),
                (self.source / "src/a.ts").read_bytes(), (self.source / "local.txt").read_bytes())

    def diff_patch(self):
        return self.git_bytes("-c", "core.quotePath=true", "diff", "--no-color", "-U0", "-M",
                              self.parent, self.commit, "--")

    def replace_git_patch(self, diff):
        real_git = shutil.which("git")
        shim = self.work / "git"
        self.diff_log = self.work / "diff-called"
        executable(shim, f"""
import os
from pathlib import Path
import sys
args = sys.argv[1:]
if "diff" in args and "-U0" in args:
    Path({str(self.diff_log)!r}).touch()
    sys.stdout.buffer.write({diff!r})
else:
    os.execv({real_git!r}, [{real_git!r}, *args])
""")
        self.env["PATH"] = str(shim.parent) + os.pathsep + self.env["PATH"]


class PopulationTests(ReviewFixture):
    def test_spec_matches_m2_rule(self):
        self.prepare_m2()
        self.review()
        self.assert_measured_spec({"files": [
            {"file_path": "src/a.ts", "modify_type": "Modified", "diff_lines": [3, 4]},
        ]})

    def test_spec_sha_mismatch_refuses_row(self):
        self.make_population()
        self.registration["diffs"][0]["spec_sha256"] = "0" * 64
        self.write_population()
        self.review(expected=1)
        self.assert_row_refused()

    def test_spec_counts_are_pins_even_when_sha_matches(self):
        self.make_population()
        row = self.registration["diffs"][0]
        for field in ("n_files", "n_lines"):
            with self.subTest(field=field):
                row[field] += 1
                self.write_population()
                self.out = self.work / field
                self.review(expected=1)
                self.assert_row_refused()
                row[field] -= 1

    def test_source_clone_is_untouched(self):
        self.make_population()
        self.keep_dirty_source()
        before = self.source_state()
        self.review()
        self.assertEqual(self.source_state(), before)
        self.assert_measured_spec(STANDARD_SPEC)
        for attempt in self.attempts():
            self.assertNotEqual(Path(attempt["root"]).resolve(), self.source.resolve())
            self.assertEqual(attempt["source"], AFTER)

    def test_quoted_path_decodes_octets_before_utf8(self):
        """review-016: actual Git headers include octal and C-style escapes."""
        names = ("src/é.ts", 'src/tab\tquote"slash\\.ts', "src/with space.ts")
        self.parent = self.commit_files({name: BEFORE for name in names})
        expected = spec_for({name: [3, 4] for name in names})
        self.make_population(spec=expected, edits={name: AFTER for name in names})
        diff = self.diff_patch()
        self.assertIn(br'\303\251', diff)
        self.assertIn(br'\tquote\"slash\\', diff)
        self.review()
        self.assert_measured_spec(expected)

    def test_malformed_quoted_path_refuses_ordinal(self):
        """review-016: bad suffix, prefix or token closure refuses before review."""
        self.make_population()
        real = self.diff_patch()
        self.assertIn(b"+++ b/src/a.ts\n", real)
        for index, header in enumerate((b'+++ "b/src/a.ts"junk', b'+++ "b/src/a.ts',
                                        b"+++ src/a.ts")):
            with self.subTest(header=header):
                self.out = self.work / f"bad-header-{index}"
                self.replace_git_patch(real.replace(b"+++ b/src/a.ts\n", header + b"\n"))
                self.review(expected=1)
                self.assertTrue(self.diff_log.exists())
                self.diff_log.unlink()
                self.assert_row_refused()

    def test_non_utf8_quoted_path_refuses_ordinal(self):
        name = "src/\ufffd.ts"
        self.parent = self.commit_files({name: BEFORE})
        self.make_population(spec=spec_for({name: [3, 4]}), edits={name: AFTER})
        real = self.diff_patch()
        header = br'+++ "b/src/\357\277\275.ts"'
        self.assertIn(header, real)
        # A decoder using UTF-8 replacement would reproduce the registered spec.
        self.replace_git_patch(real.replace(header, br'+++ "b/src/\377.ts"'))
        self.review(expected=1)
        self.assertTrue(self.diff_log.exists())
        self.assert_row_refused()

    def test_hunk_consumes_header_like_added_line(self):
        edited = AFTER + "++ b/src/phantom.ts\n"
        expected = spec_for({"src/a.ts": [3, 4, 5]})
        self.make_population(spec=expected, edits={"src/a.ts": edited})
        self.assertIn(b"\n+++ b/src/phantom.ts\n", self.diff_patch())
        self.review()
        self.assert_measured_spec(expected)

    def test_bad_hunk_length_refuses_ordinal(self):
        self.make_population()
        diff = self.diff_patch()
        self.assertIn(b"+3,2 @@", diff)
        self.replace_git_patch(diff.replace(b"+3,2 @@", b"+3,3 @@"))
        self.review(expected=1)
        self.assertTrue(self.diff_log.exists())
        self.assert_row_refused()

    def test_added_file_counts_as_modified(self):
        """open-n2-16: an eligible surviving postimage is included."""
        expected = spec_for({"src/a.ts": [3, 4], "src/new.ts": [1, 2]})
        self.make_population(spec=expected, edits={"src/a.ts": AFTER, "src/new.ts": "one\ntwo\n"})
        self.assertIn(b"--- /dev/null\n+++ b/src/new.ts\n", self.diff_patch())
        self.review()
        self.assert_measured_spec(expected)

    def test_missing_newline_marker_does_not_consume_hunk_lines(self):
        """open-n2-17: Git's marker consumes neither old nor new length."""
        self.parent = self.commit_files({"src/a.ts": BEFORE.rstrip("\n")})
        self.make_population(edits={"src/a.ts": AFTER.rstrip("\n")})
        self.assertEqual(self.diff_patch().count(b"\\ No newline at end of file\n"), 2)
        self.review()
        self.assert_measured_spec(STANDARD_SPEC)

    def test_parent_and_tree_are_checked_independent_of_spec_hash(self):
        self.make_population()
        # A different parent object with identical file bytes produces the same patch.
        alternate_parent = self.git("commit-tree", self.git("rev-parse", f"{self.parent}^{{tree}}"),
                                    "-m", "alternate fixture parent", env=dict(
                                        self.env, GIT_AUTHOR_NAME="Test", GIT_COMMITTER_NAME="Test",
                                        GIT_AUTHOR_EMAIL="test@example.invalid",
                                        GIT_COMMITTER_EMAIL="test@example.invalid"))
        alternate_tree = self.git("rev-parse", f"{self.parent}^{{tree}}")
        row = self.registration["diffs"][0]
        for field, replacement in (("parent", alternate_parent), ("tree", alternate_tree)):
            with self.subTest(field=field):
                original = row[field]
                self.assertNotEqual(original, replacement)
                row[field] = replacement
                self.write_population()
                self.out = self.work / field
                self.review(expected=1)
                self.assert_row_refused()
                row[field] = original

    def test_filter_converted_scratch_cannot_earn_coverage(self):
        """open-n4-1: a materialised scratch checkout must satisfy the raw Git pin."""
        self.parent = self.commit_files({".gitattributes": "*.ts text eol=crlf\n"})
        self.make_population()
        expected = self.git_bytes("show", "HEAD:src/a.ts")
        self.assertEqual(expected, AFTER.encode("utf-8"))
        result = self.review(expected=None)
        self.assertIn(result.returncode, (0, 1))
        if result.returncode == 1:
            self.assert_row_refused()
        else:
            self.assert_measured_spec(STANDARD_SPEC)
            for item in self.attempts():
                self.assertEqual(item["source"].encode("utf-8"), expected)

    def test_population_manifest_has_120_rows_and_no_content(self):
        path = MEASURE / "populations/602-m2-120.json"
        self.assertTrue(path.is_file(), "missing S4 602-m2-120.json population manifest")
        manifest = json.loads(path.read_bytes())
        self.assertEqual(set(manifest), {"schema", "name", "repos", "spec_rule", "diffs"})
        self.assertEqual(manifest["schema"], "prism.measure.population/1")
        self.assertEqual(manifest["spec_rule"], SPEC_RULE)
        repos = manifest["repos"]
        self.assertEqual(len(repos), 6)
        self.assertEqual({item["url"].removesuffix(".git").split("github.com/")[-1]
                          for item in repos.values()},
                         {"apache/answer", "apache/superset", "calcom/cal.com",
                          "directus/directus", "miniflux/v2", "saleor/saleor"})
        for item in repos.values():
            self.assertEqual(set(item), {"url", "selection_head"})
            self.assertRegex(item["selection_head"], r"^[0-9a-f]{40}$")

        rows = manifest["diffs"]
        self.assertEqual(sorted(row["ordinal"] for row in rows), list(range(1, 121)))
        self.assertEqual(Counter(row["repo"] for row in rows), {repo: 20 for repo in repos})
        for row in rows:
            self.assertEqual(set(row), POPULATION_FIELDS)
            for field in ("commit", "parent", "tree"):
                self.assertRegex(row[field], r"^[0-9a-f]{40}$")
            self.assertRegex(row["spec_sha256"], r"^[0-9a-f]{64}$")
            for field in ("n_files", "n_lines"):
                self.assertIs(type(row[field]), int)
                self.assertGreaterEqual(row[field], 0)
        self.assertNotIn('"diff_lines"', path.read_text(encoding="utf-8"))
        self.assertNotIn('"files"', path.read_text(encoding="utf-8"))

    def refused_by_replaced_patch(self, real, replacement, name):
        self.out = self.work / name
        self.replace_git_patch(replacement)
        self.review(expected=1)
        self.assertTrue(self.diff_log.exists())
        self.diff_log.unlink()
        self.assert_row_refused()
        self.assertNotEqual(real, replacement)

    def test_added_file_with_deletions_refuses_ordinal(self):
        """review-s4-r1: an addition from /dev/null has no preimage lines to delete."""
        expected = spec_for({"src/a.ts": [3, 4], "src/new.ts": [1, 2]})
        self.make_population(spec=expected, edits={"src/a.ts": AFTER, "src/new.ts": "one\ntwo\n"})
        real = self.diff_patch()
        header = b"--- /dev/null\n+++ b/src/new.ts\n@@ -0,0 +1,2 @@\n+one\n+two\n"
        self.assertIn(header, real)
        forged = b"--- /dev/null\n+++ b/src/new.ts\n@@ -1 +1,2 @@\n-gone\n+one\n+two\n"
        self.refused_by_replaced_patch(real, real.replace(header, forged), "added-with-deletion")

    def test_hunks_with_overlapping_or_backward_new_ranges_refuse_ordinal(self):
        self.make_population()
        real = self.diff_patch()
        hunk = b"@@ -3 +3,2 @@ const b = 2;\n-const c = 3;\n+const c = 30;\n+const d = 4;\n"
        self.assertIn(hunk, real)
        backward = (b"@@ -4,0 +4 @@ const b = 2;\n+const d = 4;\n"
                    b"@@ -3 +3 @@\n-const c = 3;\n+const c = 30;\n")
        for name, replacement in (("overlap", hunk * 2), ("backward", backward)):
            with self.subTest(name):
                self.refused_by_replaced_patch(real, real.replace(hunk, replacement), name)

    def test_git_record_without_names_refuses_ordinal(self):
        self.make_population()
        real = self.diff_patch()
        record = b"diff --git a/src/a.ts b/src/a.ts\n"
        self.assertIn(record, real)
        for name, bare in (("empty", b"diff --git \n"), ("blank", b"diff --git   \n")):
            with self.subTest(name):
                self.refused_by_replaced_patch(real, real.replace(record, bare + record), name)

    def test_metadata_only_records_still_parse(self):
        """Control for the stricter record checks: a pure rename has no ---/+++ headers."""
        self.parent = self.commit_files({"src/old.ts": BEFORE})
        self.make_population(delete=("src/old.ts",), edits={"src/a.ts": AFTER, "src/moved.ts": BEFORE})
        self.assertIn(b"rename from src/old.ts\n", self.diff_patch())
        self.review()
        self.assert_measured_spec(STANDARD_SPEC)

    def test_population_git_ignores_the_user_attributes_file(self):
        self.make_population()
        real_git = shutil.which("git")
        shim = self.work / "git"
        self.git_log = self.work / "git-args.jsonl"
        executable(shim, f"""
import json
import os
import sys
with open({str(self.git_log)!r}, "a", encoding="utf-8") as stream:
    print(json.dumps(sys.argv[1:]), file=stream)
os.execv({real_git!r}, [{real_git!r}, *sys.argv[1:]])
""")
        self.env["PATH"] = str(shim.parent) + os.pathsep + self.env["PATH"]
        self.review()
        calls = [json.loads(line) for line in self.git_log.read_text().splitlines()]
        population_calls = [args for args in calls if "diff" in args or "clone" in args]
        self.assertTrue(any("diff" in args for args in population_calls))
        self.assertTrue(any("clone" in args for args in population_calls))
        for args in population_calls:
            self.assertIn("core.attributesFile=/dev/null", args)

    def admitted_with_replaced_patch(self, replacement, name):
        """The pinned spec is reproduced from a patch that Git itself never produces."""
        self.out = self.work / name
        self.replace_git_patch(replacement)
        self.review()
        self.assertTrue(self.diff_log.exists())
        self.diff_log.unlink()
        self.assert_measured_spec(STANDARD_SPEC)
        self.log.unlink()

    def test_known_limit_preimage_ranges_are_not_checked(self):
        """PROVISIONAL (known-s4-4): only postimage ranges are ordered; a regressing
        preimage range (`-8,2` then `-2`) is admitted with the pinned spec."""
        self.make_population()
        real = self.diff_patch()
        hunk = b"@@ -3 +3,2 @@ const b = 2;\n-const c = 3;\n+const c = 30;\n+const d = 4;\n"
        self.assertIn(hunk, real)
        regress = b"@@ -8,2 +3 @@\n-x\n-y\n+const c = 30;\n@@ -2 +4 @@\n-z\n+const d = 4;\n"
        self.admitted_with_replaced_patch(real.replace(hunk, regress), "preimage-regression")
        # Control: overlapping postimage ranges are caught.
        self.refused_by_replaced_patch(real, real.replace(hunk, hunk * 2), "postimage-overlap")

    def test_known_limit_two_pure_deletions_at_one_point_are_admitted(self):
        """PROVISIONAL (known-s4-4): Git emits one pure deletion per point; two are admitted."""
        self.make_population()
        real = self.diff_patch()
        hunk = b"@@ -3 +3,2 @@ const b = 2;\n-const c = 3;\n+const c = 30;\n+const d = 4;\n"
        self.assertIn(hunk, real)
        twice = b"@@ -2 +2,0 @@\n-x\n@@ -2 +2,0 @@\n-y\n"
        self.admitted_with_replaced_patch(real.replace(hunk, twice + hunk), "two-deletions")
        # Control: a pure deletion inside the postimage range an earlier hunk covers is caught.
        inside = b"@@ -9 +3,0 @@\n-x\n"
        self.refused_by_replaced_patch(real, real.replace(hunk, hunk + inside), "deletion-inside")

    def test_known_limit_record_names_are_not_cross_checked_with_header_names(self):
        """PROVISIONAL (known-s4-4): `diff --git a/q b/r` over `--- a/src/a.ts` is admitted;
        the ---/+++ names decide the file."""
        self.make_population()
        real = self.diff_patch()
        record = b"diff --git a/src/a.ts b/src/a.ts\n"
        self.assertIn(record, real)
        self.admitted_with_replaced_patch(real.replace(record, b"diff --git a/q b/r\n"), "other-names")
        # Control: a record that names no files is caught.
        self.refused_by_replaced_patch(real, real.replace(record, b"diff --git \n"), "nameless")

    def test_known_limit_other_malformed_patch_members_are_admitted(self):
        """PROVISIONAL (known-s4-4): further shapes Git never produces keep the pinned spec.
        Each member is checked on the parser directly against the Git-produced patch."""
        from measure import population
        from measure.inputs import PinError

        self.make_population()
        real = self.diff_patch()
        spec = population.patch_spec(real)
        self.assertEqual(spec, STANDARD_SPEC)
        record = b"diff --git a/src/a.ts b/src/a.ts\n"
        hunk = b"@@ -3 +3,2 @@ const b = 2;\n-const c = 3;\n+const c = 30;\n+const d = 4;\n"
        self.assertIn(record, real)
        self.assertIn(hunk, real)
        # Whole extra records of ineligible files, so the spec cannot change.
        added = (b"diff --git a/README.md b/README.md\n--- /dev/null\n+++ b/README.md\n"
                 b"@@ -5,0 +1,2 @@\n+one\n+two\n")
        members = {
            "empty-preimage-names": real.replace(record, b"diff --git a/ b/\n"),
            "unterminated-record-quote": real.replace(record, b'diff --git "a/src/a.ts b/src/a.ts\n'),
            "arbitrary-metadata": real.replace(record, record + b"anything at all\n", 1),
            "context-under-U0": real.replace(
                hunk, b"@@ -2,2 +2,3 @@\n ctx\n-const c = 3;\n+const c = 30;\n+const d = 4;\n"),
            "premature-no-newline-marker": real.replace(
                hunk, hunk.replace(b"-const c = 3;\n", b"-const c = 3;\n\\ No newline at end of file\n")),
            "absent-side-nonzero-start": real + added,
            "two-absent-sides": real + b"diff --git a/x.md b/x.md\n--- /dev/null\n+++ /dev/null\n",
        }
        for name, patch_bytes in members.items():
            with self.subTest(name):
                self.assertEqual(population.patch_spec(patch_bytes), spec)
        # Controls that ARE caught: an added file with a preimage line, a hunk length
        # mismatch, an unterminated quoted header token.
        controls = {
            "added-file-with-deletion": real + added.replace(b"@@ -5,0 +1,2 @@\n+one\n",
                                                             b"@@ -5 +1,2 @@\n-gone\n+one\n"),
            "hunk-length-mismatch": real.replace(b"+3,2 @@", b"+3,3 @@"),
            "unterminated-header-token": real.replace(b"+++ b/src/a.ts\n", b'+++ "b/src/a.ts\n'),
        }
        for name, patch_bytes in controls.items():
            with self.subTest(name), self.assertRaises(PinError):
                population.patch_spec(patch_bytes)
        # A deleted file whose Git-quoted OLD name is not UTF-8 refuses although its postimage
        # is /dev/null (required: ignore the deletion); a UTF-8 old name is ignored.
        deleted = b'diff --git "a/src/x.ts" "b/src/x.ts"\n--- %s\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n'
        self.assertEqual(population.patch_spec(real + deleted % b'"a/src/x.ts"'), spec)
        with self.assertRaises(PinError):
            population.patch_spec(real + deleted % br'"a/src/\377.ts"')
