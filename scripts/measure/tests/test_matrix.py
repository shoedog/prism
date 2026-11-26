"""Execute the frozen S1 cases from the Stage T matrix."""

from __future__ import annotations

import importlib
import json
import re
import unittest

from .helpers import REPO, Workbench, write_tree
from .test_matrix_rows import CASES, MATRIX, read_jsonl
from .test_syntax import parse_as_python_39

ENABLED_GROUPS = {"run"}


# Ledger "Materialize" IDs of the run group that the enabled matrix does not
# carry as rows, mapped to the named executable tests of this slice.
S1_TESTS = {
    "run-019": [("test_run", "RunTests", "test_guard_refusal_exits_75"),
                ("test_run", "RunTests", "test_late_quiet_preserves_failed_receipt"),
                ("test_run", "RunTests", "test_bad_guard_status_never_yields_quiet")],
    "run-020": [("test_corpora", "CorporaTests",
                 "test_tree_pin_detects_link_changes_with_same_perf_manifest")],
    "run-021": [("test_corpora", "CorporaTests", "test_links_into_git_refuse_and_tracked_target_admits"),
                ("test_corpora", "CorporaTests", "test_git_blob_binding_ignores_index_flags"),
                ("test_run", "RunTests", "test_snapshot_excludes_repository_metadata_and_sibling_sources")],
    "run-022": [("test_run", "RunTests", "test_link_resolution_change_invalidates_run"),
                ("test_corpora", "CorporaTests", "test_link_hops_refuse_even_when_dangling"),
                ("test_run", "IsolatedLinkTargetTests",
                 "test_admitted_and_digest_binds_link_text_and_target_files"),
                ("test_run", "IsolatedLinkTargetTests", "test_target_file_edit_changes_the_digest"),
                ("test_run", "IsolatedLinkTargetTests", "test_target_file_edit_during_a_run_exits_four"),
                ("test_run", "IsolatedLinkTargetTests",
                 "test_dangling_link_text_change_changes_the_digest"),
                ("test_run", "IsolatedLinkTargetTests",
                 "test_dangling_link_retarget_during_a_run_exits_four"),
                ("test_run", "IsolatedLinkTargetTests",
                 "test_escaping_dangling_text_refuses_before_version")],
    "run-023": [("test_corpora", "CorporaTests", "test_link_cycles_and_ancestor_directories_refuse"),
                ("test_corpora", "CorporaTests", "test_link_hops_refuse_even_when_dangling")],
}
# Round-four IDs owned by later slices, with their owning group.
LATER_SLICE_IDS = {"gate-026": "gate", "oracle-065": "oracle", "oracle-066": "oracle",
                   "cmp-061": "comparator", "review-019": "review"}


class MatrixLedgerTests(unittest.TestCase):
    def ledger(self):
        return (REPO / "docs/superpowers/plans/2026-11-20-lanes/lane11-measurement-kit"
                / "FORMATS-LEDGER.md").read_text(encoding="utf-8")

    def test_round_four_handoff_covers_all_eight_fixture_ids(self):
        section = self.ledger().split("Materialise the following eight new IDs", 1)[1]
        table, handoff = section.split("**Existing controls to extend.**", 1)
        ids = re.findall(r"^\| ([a-z]+-[0-9]+) \| Materialize", table, flags=re.MULTILINE)
        self.assertEqual(len(set(ids)), 8)
        self.assertIn("adds all eight IDs", handoff.split("**Handoff.**", 1)[1])
        # Every ID is executed here or assigned to a later slice, none dropped.
        self.assertEqual(set(ids), {i for i in S1_TESTS if i >= "run-021"} | set(LATER_SLICE_IDS))
        self.assertTrue(all(S1_TESTS[i] for i in S1_TESTS), "every S1 ID needs a nonempty mapping")

    def test_every_run_materialize_id_is_a_matrix_row_or_a_named_test(self):
        ledger = self.ledger()
        wanted = set(re.findall(r"^\| (run-[0-9]+) \| Materialize", ledger, flags=re.MULTILINE))
        rows = {row["id"] for row in read_jsonl(MATRIX / "rows.jsonl") if row["group"] == "run"}
        self.assertEqual(wanted - rows, set(S1_TESTS))
        for ident, tests in S1_TESTS.items():
            # An empty mapping would leave the row without executable coverage.
            self.assertTrue(tests, f"{ident} maps to no executable test")
            for module, name, method in tests:
                with self.subTest(id=ident, test=method):
                    owner = getattr(importlib.import_module(f"measure.tests.{module}"), name)
                    self.assertTrue(callable(getattr(owner, method, None)))


class MatrixTests(Workbench):
    def check_row(self, row):
        setup = json.loads((CASES / row["id"] / row["input"]["setup"]).read_bytes())
        truth = row["truth"]
        env = setup.get("environment", {})
        if setup["command"] == "python syntax":
            try:
                parse_as_python_39(env["module_text"])
            except SyntaxError:
                actual = False
            else:
                actual = True
            self.assertEqual(actual, truth["parses_as_python_39"])
            return

        corpus = setup["corpus"]
        self.source = self.work / "matrix-corpus"
        self.key = corpus.get("key", "Z")
        self.make_corpus(git=corpus["git"], files=corpus["tracked"], links=corpus.get("symlinks"),
                         ignore=corpus.get("tracked_ignore"), pin_tree=corpus["pin"] == "matching-archive")
        if corpus.get("second_commit"):
            self.commit_files(corpus["second_commit"])
        for field in ("untracked", "ignored", "tracked_edits"):
            write_tree(self.source, corpus.get(field, {}))

        binary = setup["fake_binary"]
        cases = {}
        if "mutate" in binary:
            mutation = binary["mutate"]
            base = f"Path({str(self.work)!r})" if mutation.get("outside_query_root") else "root"
            cases[mutation["command"]] = (
                f"path = {base} / {mutation['file']!r}; "
                f"path.parent.mkdir(parents=True, exist_ok=True); path.write_text({mutation['contents']!r})"
            )
        if "retarget_symlink" in binary:
            change = binary["retarget_symlink"]
            cases[change["command"]] = (f"path = root / {change['file']!r}; path.unlink(); "
                                        f"path.symlink_to({change['target']!r})")
        if "fail" in binary:
            cases[binary["fail"]["command"]] = f"sys.exit({binary['fail']['exit']!r})"
        self.make_fake(cases, binary["version"], binary["commands_exit"])
        if "guard_exit" in env:
            self.assertEqual(env["guard_exit"], 75)
            self.make_guard({"version": (75, dict(kind="host_quiet", launched=False, child_exit=None))})
        args = []
        if "prediction" in env:
            prediction = env["prediction"]
            write_tree(self.work, {prediction["file"]: prediction["contents"]})
            args.extend(["--prediction", self.work / prediction["file"]])
        for name, value in env.get("trace_env", {}).items():
            args.extend(["--trace-env", f"{name}={value}"])
        if "expect_version" in env:
            args.extend(["--expect-version", env["expect_version"]])
        write_tree(self.out, setup.get("out_files", {}))

        if "manifest_sha256" in truth:
            # §8 replaces Stage T's archive-only success with two distinct checks.
            from measure.inputs import manifest_hash
            from .helpers import perf_manifest_hash as _load
            perf_hash = _load()

            self.entry["manifest_sha256"] = truth["manifest_sha256"]
            self.write_manifest()
            self.assert_exit(self.verify(), 3)
            self.entry["tree_sha256"] = "b6bffcac5874e0ee6175e89da1d91450d4ee15634e82572f368c2158b74d14b1"
            self.write_manifest()
            self.assertEqual(manifest_hash(self.source), truth["manifest_sha256"])
            self.assertEqual(manifest_hash(self.source) == perf_hash(self.source),
                             truth["manifest_hash_matches_perf_cloud"])

        result = self.verify() if setup["command"] == "corpora verify" else self.run_kit(*args)
        self.assert_exit(result, truth["exit"])
        if setup["command"] == "corpora verify":
            self.assertIn(f"{self.key}\t{'ok' if truth['exit'] == 0 else 'refused'}\t", result.stdout)
        if "binary_ran" in truth:
            self.assertEqual(bool(self.fake_log()), truth["binary_ran"])
        receipt = self.receipt()
        if "receipt_status" in truth:
            self.assertEqual(receipt["status"] if receipt else None, truth["receipt_status"])
        if "receipt_prediction_sha256" in truth:
            self.assertEqual(receipt["prediction"]["sha256"], truth["receipt_prediction_sha256"])
        if "trace_env_by_command" in truth:
            labels = {"edges": "dfg-stats --edges"}
            trace = {labels.get(c["id"], c["id"]): {k: v for k, v in c["env"].items()
                                                    if k in env["trace_env"]}
                     for c in receipt["commands"]}
            self.assertEqual(trace, truth["trace_env_by_command"])
        if "call_stats_cache_option" in truth:
            self.assertIn(truth["call_stats_cache_option"], receipt["commands"][1]["argv"])
        if row["id"] == "run-012":
            self.assertEqual([c["id"] for c in receipt["commands"]], ["dfg-stats", "call-stats"])
            self.assertEqual(receipt["commands"][-1]["guard_status"]["child_exit"], 7)
        if row["id"] == "run-017":
            self.assertEqual([c["command"] for c in self.fake_log()], ["version"])
        for name, contents in setup.get("out_files", {}).items():
            self.assertEqual((self.out / name).read_text(), contents)


def make_test(row):
    def test(self):
        self.check_row(row)
    return test


for row in read_jsonl(MATRIX / "rows.jsonl"):
    if row["group"] in ENABLED_GROUPS:
        setattr(MatrixTests, "test_" + row["id"].replace("-", "_"), make_test(row))
