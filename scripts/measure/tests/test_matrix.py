"""Execute the enabled cases from the Stage T matrix."""

from __future__ import annotations

from collections import Counter
import gzip
import importlib
import json
import re
import unittest

from .comparator_fixtures import ComparatorFixture, read_rows
from . import test_diff_edges
from .helpers import REPO, Workbench, write_tree
from .test_matrix_rows import CASES, MATRIX, read_jsonl
from .test_syntax import parse_as_python_39

ENABLED_GROUPS = {"run", "comparator", "join"}


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


class ComparatorMatrixTests(ComparatorFixture):
    def check_row(self, row):
        inputs = row["input"]
        ident = row["id"]
        case = CASES / inputs["case"]
        decisions = (REPO / "docs/superpowers/plans/2026-11-20-lanes/lane11-measurement-kit"
                     / "FORMATS-DECISIONS.md").read_text(encoding="utf-8")
        revised = re.search(r"^\| " + ident + r" \| `(\{.*\})` \|$", decisions, re.MULTILINE)
        truth = json.loads(revised[1]) if revised else row["truth"]
        setup = json.loads((case / inputs["setup"]).read_bytes()) if "setup" in inputs else {}
        if "repeat_distinct_keys" in setup:
            test_diff_edges.DiffEdgesTests.large_dump(self, setup["repeat_distinct_keys"], truth["peak_bytes_exclusive"])
            return

        self.classifier = case / inputs["classifier"]
        variation = setup.get("receipt_variations", {})
        edit = setup.get("transport_edit", {})
        for name in ("base", "head"):
            payload = (case / inputs[name]).read_bytes()
            if edit.get("arm") == name and "insert" in edit:
                records = payload.splitlines(keepends=True)
                records.insert(edit["after_line"], edit["insert"].encode())
                payload = b"".join(records)
            transport = ((case / inputs[name + "_transport"]).read_bytes()
                         if name + "_transport" in inputs else gzip.compress(payload))
            truncate = edit.get("arm") == name and edit.get("truncate_gzip_bytes")
            if truncate and not edit["after_receipt"]:
                transport = transport[:-truncate]
            trace = (case / inputs["trace"]).read_bytes() if name == "head" and "trace" in inputs else None
            run = self.arm(name, transport=transport, trace=trace, stats=setup.get(name + "_stats"),
                           options=variation.get(name + "_options"),
                           trace_env=variation.get(name + "_trace_env"),
                           version=variation.get(name + "_binary", "fixture-bin"),
                           source=variation.get(name, "export {};\n"))
            if truncate and edit["after_receipt"]:
                (run / "edges.jsonl.gz").write_bytes(transport[:-truncate])
            if variation.get(name + "_output_modified_after_receipt"):
                (run / "edges.jsonl.gz").write_bytes(gzip.compress(payload + b"\n"))
            if name + "_status" in variation:
                path = run / "receipt.json"
                record = json.loads(path.read_bytes())
                record["status"] = variation[name + "_status"]
                self.save(path, record)
        if ident == "cmp-023":
            # Both arms are individually admissible under their own registered keys.
            manifest = json.loads(self.manifest.read_bytes())
            path = self.work / "head/receipt.json"
            record = json.loads(path.read_bytes())
            record["corpus"]["key"] = "H"
            manifest["corpora"]["H"] = {**manifest["corpora"]["Z"],
                                        "tree_sha256": record["corpus"]["input_sha256"]}
            self.save(path, record)
            self.save(self.manifest, manifest)

        if inputs.get("operation") == "diff-stats":
            allow = self.work / "allow.txt"
            allow.write_text("\n".join(setup["allow"]))
            stdout = self.invoke("diff-stats", "--allow", allow, expected=truth["exit"])
            expected_lines = {f"  {key}: {base} -> {head} ({delta:+})"
                              for key, (base, head, delta) in truth["deltas"].items()}
            self.assertEqual({line for line in stdout.splitlines() if line.startswith("  ")}, expected_lines)
            return

        args = []
        if inputs["key_mode"] != "default":
            args.extend(("--key", inputs["key_mode"]))
        if inputs["key_mode"] == "line":
            args.extend(("--line-reason", "matrix line projection"))
        if "unjoined_tags" in inputs or "unjoined_tags" in truth:
            # Test the actual callback payload along with the certificate's counts.
            source = self.classifier.read_text(encoding="utf-8")
            self.classifier = self.work / "classifier.py"
            tags = truth.get("unjoined_tags", inputs.get("unjoined_tags"))
            self.classifier.write_text(source + f"""
_original_validate = globals().get("validate", lambda bound: [])
def validate(bound):
    records = list(bound.unjoined)
    assert [record["tag"] for record in records] == {tags!r}
    mechanisms = {truth.get("unjoined_mechanisms")!r}
    if mechanisms is not None:
        assert [record["mechanism"] for record in records] == mechanisms
    return _original_validate(bound)
""", encoding="utf-8")
        stdout = self.compare(*args, expected=truth["exit"], diagnostic=inputs.get("validation_message"))
        if truth["exit"] == 2:
            self.assertEqual(stdout, "")
            self.assertFalse((self.out / "certificate.json").exists())
            return
        actual = self.cert()
        for field in ("unattributed", "changed_keys", "collisions", "base_edges", "head_edges",
                      "records", "validation"):
            if field in truth:
                self.assertEqual(actual[field], truth[field], field)
        if "occurrences" in truth:
            expected = truth["occurrences"]
            observed = ({"base": actual["base_residual"], "head": actual["head_residual"]}
                        if isinstance(expected, dict) else actual["occurrences"])
            self.assertEqual(observed, expected)
        if ident == "cmp-039":
            self.assertEqual(actual["key_mode"], "line")
        changes = self.changes()
        if "changes" in truth:
            self.check_changes(changes, truth["changes"])
        if "classifier_error" in inputs:
            self.assertIn(inputs["classifier_error"], changes[0]["error"])
        if "reconstruction" in truth:
            self.check_reconstruction(case, inputs, setup["additional_pairs"])

    def check_changes(self, actual, expected):
        def match(change, truth):
            for field, value in truth.items():
                if field in ("from", "to"):
                    side = "base_bag" if field == "from" else "head_bag"
                    if change[side][0] != value:
                        return False
                elif field in ("base_bag", "head_bag"):
                    if Counter(json.dumps(x, sort_keys=True) for x in value) != Counter(
                            json.dumps(x, sort_keys=True) for x in change[field]):
                        return False
                elif change[field] != value:
                    return False
            return True

        remaining = actual[:]
        for row in expected:
            candidates = [i for i, change in enumerate(remaining) if match(change, row)]
            self.assertTrue(candidates, (row, remaining))
            remaining.pop(candidates[0])
        self.assertEqual(remaining, [])

    def check_reconstruction(self, case, inputs, recipe):
        import random
        from measure.edges import decode_edge, residual_rows

        def emitted(base, head, published=None):
            def counts(rows):
                result = {}
                for raw in rows:
                    key, attr, _ = decode_edge(raw, line_mode=False)
                    result.setdefault(key, Counter())[attr] += 1
                return result

            a, b = counts(base), counts(head)
            for key in a.keys() | b.keys():
                left, right = a.get(key, Counter()), b.get(key, Counter())
                if published is None:
                    unchanged, changes = residual_rows(left, right)
                else:
                    unchanged = left & right
                    changes = [(row["kind"], *(Counter(json.dumps(member, sort_keys=True, separators=(",", ":")).encode()
                                                       for member in row[arm + "_bag"])
                                               for arm in ("base", "head")))
                               for row in published if row["key"] == json.loads(key)]
                rebuilt = [unchanged.copy(), unchanged.copy()]
                for _, lb, rb in changes:
                    rebuilt[0].update(lb)
                    rebuilt[1].update(rb)
                self.assertEqual(rebuilt, [left, right])

        emitted(*(read_rows(case / inputs[name]) for name in ("base", "head")), self.changes())
        rng = random.Random(recipe["seed"])
        for _ in range(recipe["count"]):
            arms = [[], []]
            for arm in arms:
                for wire in recipe["keys"]:
                    for attr in recipe["labels"]:
                        source = {**wire, "from": {**wire["from"], "access": attr["from_access"]},
                                  "to": {**wire["to"], "access": attr["to_access"]}}
                        source.update({key: attr[key] for key in ("confidence", "doubt", "kill_line")})
                        arm.extend([source] * rng.randrange(recipe["max_copies_per_label"] + 1))
            emitted(*arms)


def make_test(row):
    def test(self):
        self.check_row(row)
    return test


for row in read_jsonl(MATRIX / "rows.jsonl"):
    if row["group"] in ENABLED_GROUPS:
        setattr(MatrixTests if row["group"] == "run" else ComparatorMatrixTests,
                "test_" + row["id"].replace("-", "_"), make_test(row))
