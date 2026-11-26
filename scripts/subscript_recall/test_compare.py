import contextlib
import copy
import hashlib
import io
import json
import os
import shutil
import sys
import tempfile
import tracemalloc
import unittest
from unittest import mock
from pathlib import Path

import compare

SRC = b"def f(d, v):\n    d['k'] = v\n    return d\n"
VERSION = "slicing 4.0.0 (abcdef012345)"
EXPECT = "abcdef0123456789abcdef0123456789abcdef01"
TRACE = compare.TRACE_PREFIX + json.dumps({"kind": "owner", "file": "a.py"})
F = dict(type="function", name="f", file="a.py", start_line=1, end_line=3, start_byte=0, end_byte=40)


def var(owner, line, access, start, end, base="d"):
    return dict(type="variable", file="a.py", owner=owner, owner_start_line=1, line=line, access=access,
                path=dict(base=base, fields=[]), start_byte=start, end_byte=end)


# U_F and U_G are identical on the byte projection and differ only by owner.
D_F, U_F, U_G = var("f", 2, "def", 5, 6), var("f", 3, "use", 30, 31), var("g", 3, "use", 30, 31)
RET = dict(type="return_value", file="a.py", owner="f", owner_start_line=1, line=3, return_start_byte=24,
           return_end_byte=32, child_slot=0, start_byte=31, end_byte=32)
CALLER_A, CALLER_B = var("h", 9, "def", 70, 71, "x"), var("h", 9, "def", 80, 81, "y")
ISO = dict(type="statement", file="a.py", line=2, stmt_kind="other", stmt_detail=None, start_byte=10, end_byte=20)
DF = dict(variant="data_flow", confidence="exact", doubt=None, kill_line=None)
CONTAINS = dict(variant="contains")
NODES = [F, D_F, U_F, U_G, RET, CALLER_A, CALLER_B, ISO]
EDGES = [(D_F, U_F, DF), (RET, CALLER_A, dict(variant="return", confidence="exact")),
         (F, RET, dict(variant="return_flow", suppress_shortcut=False)),
         (F, D_F, dict(variant="call", confidence="exact")),
         (F, U_F, CONTAINS), (F, U_G, CONTAINS), (F, CALLER_A, CONTAINS), (F, CALLER_B, CONTAINS)]
BYTES = [dict({"from": dict(file="a.py", line=2, access="def", start_byte=5, end_byte=6),
               "to": dict(file="a.py", line=3, access="use", start_byte=30 + i, end_byte=31 + i)},
              confidence="exact", doubt=None) for i in (0, 0, 1)]


def render(nodes, edges, version="4.0.0"):
    touched = {json.dumps(e[i], sort_keys=True) for e in edges for i in (0, 1)}
    rows = [dict(kind="meta", schema=compare.READER_SCHEMA, prism_version=version),
            dict(kind="file", key="a.py", sha256=hashlib.sha256(SRC).hexdigest())]
    rows += [dict(kind="node", isolated=json.dumps(n, sort_keys=True) not in touched, **n) for n in nodes]
    rows += [dict(kind="edge", **{"from": a, "to": b, "edge": e}) for a, b, e in edges]
    variants = {}
    for e in edges:
        variants[e[2]["variant"]] = variants.get(e[2]["variant"], 0) + 1
    isolated = sum(r["isolated"] for r in rows if r["kind"] == "node")
    return rows + [dict(kind="summary", nodes=len(nodes), isolated_nodes=isolated, edges=len(edges),
                        edges_by_variant=variants)]


def stats(n, extra=0):
    return dict(dfg_label_exact=n, dfg_label_nameonly_killed=0, dfg_pruned_order=extra,
                dfg_labels=dict(dfg_label_exact=n, dfg_fallback_edges_capped=extra))


PRISM = """#!{py}
import json, os, sys
here = os.path.dirname(os.path.abspath(__file__))
D = json.load(open(os.path.join(here, "data-" + os.environ["FAKE_ARM"] + ".json")))
a = sys.argv[1:]
open(os.path.join(here, "log"), "a").write(json.dumps([a, os.environ.get("PRISM_EDGES_BYTES"), os.environ.get("PRISM_ELEMENT_RECALL_TRACE")]) + "\\n")
if a == ["--version"]:
    print(D["version"]); sys.exit(0)
if D["touch"]:
    open(D["touch"], "a").write("x")
if "call-stats" in a:
    print(json.dumps(D["calls"]))
elif "--edges" in a:
    [print(json.dumps(e)) for e in D["edges"]]
    [print(l, file=sys.stderr) for l in D["edges_err"]]
else:
    print(json.dumps(D["dfg"]))
sys.exit(D["prism_exit"])
# {salt}
"""
READER = """#!{py}
import json, os, sys
here = os.path.dirname(os.path.abspath(__file__))
D = json.load(open(os.path.join(here, "data-" + os.environ["FAKE_ARM"] + ".json")))
[print(json.dumps(r)) for r in D["graph"]]
sys.exit(D["reader_exit"])
"""


class Harness(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        self.repo = self.tmp / "repo"
        self.repo.mkdir()
        (self.repo / "a.py").write_bytes(SRC)

    def tool(self, name, salt="", **per_arm):
        """A fake prism and typed reader; `per_arm` maps an arm to the canned fields it overrides."""
        d = self.tmp / name
        d.mkdir(exist_ok=True)
        for arm in compare.TRACE_BY_ARM:
            data = dict(version=VERSION, edges=BYTES, graph=render(NODES, EDGES), dfg=stats(3), calls=stats(3, 1),
                        edges_err=[TRACE] if arm == "on" else [], prism_exit=0, reader_exit=0, touch=None)
            data.update(per_arm.get(arm, {}))
            (d / f"data-{arm}.json").write_text(json.dumps(data))
        for fname, text in (("prism", PRISM), ("reader", READER)):
            (d / fname).write_text(text.format(py=sys.executable, salt=salt))
            (d / fname).chmod(0o755)
        return d

    def run_arm(self, runs, arm, tool, corpus="C", expect=EXPECT, extra=()):
        err = io.StringIO()
        os.environ["FAKE_ARM"] = arm
        with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
            code = compare.main(["run", "--binary", str(tool / "prism"), "--reader", str(tool / "reader"),
                                 "--expect-version", expect, "--corpus", corpus, "--repo", str(self.repo),
                                 "--trace", str(compare.TRACE_BY_ARM[arm]), "--out", str(Path(runs) / corpus / arm), *extra])
        return code, err.getvalue()

    def three(self, **per_arm):
        self.n = getattr(self, "n", 0) + 1
        runs, tool = self.tmp / f"runs{self.n}", self.tool("bin", **per_arm)
        for arm in compare.TRACE_BY_ARM:
            self.assertEqual(self.run_arm(runs, arm, tool)[0], 0)
        return runs

    def zero(self, runs, corpora=("C",)):
        err = io.StringIO()
        with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
            code = compare.main(["zero", "--runs", str(runs), "--corpora", *corpora])
        return code, err.getvalue()

    def assertRejected(self, needles, **per_arm):
        code, err = self.zero(self.three(**per_arm))
        self.assertEqual(code, 1, err)
        for needle in needles:
            self.assertIn(needle, err)
        return err


class RunTests(Harness):
    def test_run_binds_inputs_and_keeps_trace_records_apart(self):
        runs = self.three()
        m = json.loads((runs / "C" / "on" / "manifest.json").read_text())
        self.assertTrue(m["complete"] and m["trace"] == 1 and m["version"]["identity"] == "abcdef012345")
        self.assertEqual((m["source"]["files"], m["counts"]["edge_lines"], m["counts"]["isolated"]), (1, 3, 1))
        self.assertEqual((runs / "C" / "on" / "trace.txt").read_text(), TRACE + "\n")
        self.assertNotIn("element-recall", (runs / "C" / "on" / "edges.err").read_text())
        self.assertEqual((runs / "C" / "off" / "trace.txt").read_text(), "")
        calls = [json.loads(l) for l in (self.tmp / "bin" / "log").read_text().splitlines()]
        runs_only = [c for c in calls if c[0] != ["--version"]]
        self.assertEqual(sum(c[2] == "1" for c in runs_only), 3)  # the on arm, its three prism commands
        self.assertEqual(sum(c[2] == "0" for c in runs_only), 6)
        self.assertEqual([c[0][3:] for c in runs_only if c[1] == "1"], [["--repo", str(self.repo.resolve()), "--edges"]] * 3)

    def test_run_refuses_partial_runs_even_with_exit_zero(self):
        full = render(NODES, EDGES)
        cases = {
            "reader exit": (dict(reader_exit=3), "exited 3"),
            "no summary": (dict(graph=full[:-1]), "summary tail"),
            "short graph": (dict(graph=full[:-3] + full[-1:]), "disagrees"),
            "prism exit": (dict(prism_exit=1), "exited 1"),
            "empty edge dump": (dict(edges=[]), "empty byte-mode"),
            "label counters": (dict(dfg=stats(2)), "label counters"),
            "trace while off": (dict(edges_err=[TRACE]), "trace off"),
            "dirty": (dict(version="slicing 4.0.0 (abcdef012345-dirty)"), "dirty"),
            "wrong sha": (dict(version="slicing 4.0.0 (123456789abc)"), "does not match"),
            "unknown identity": (dict(version="slicing 4.0.0 (unknown)"), "does not match"),
            "reader semver": (dict(graph=render(NODES, EDGES, version="9.9.9")), "different versions"),
            "source drift": (dict(touch=str(self.repo / "b.txt")), "source tree changed"),
            "bad file hash": (dict(graph=[dict(r, sha256="0" * 64) if r["kind"] == "file" else r for r in full]),
                              "disagrees with the source tree"),
        }
        for name, (over, needle) in cases.items():
            with self.subTest(name):
                slug = name.replace(" ", "_")
                code, err = self.run_arm(self.tmp / slug, "off", self.tool(slug, off=over))
                self.assertEqual(code, 1)
                self.assertIn(needle, err)
                out = self.tmp / slug / "C" / "off"
                self.assertTrue((out / "FAILED").exists() and not (out / "manifest.json").exists())
                (self.repo / "b.txt").unlink(missing_ok=True)
        with self.subTest("short expectation"):
            self.assertEqual(self.run_arm(self.tmp / "s", "off", self.tool("s"), expect="abc")[0], 1)
        with self.subTest("occupied out dir is never overwritten"):
            (self.tmp / "occ" / "C" / "off").mkdir(parents=True)
            (self.tmp / "occ" / "C" / "off" / "x").write_text("keep")
            self.assertEqual(self.run_arm(self.tmp / "occ", "off", self.tool("occ"))[0], 2)
        with self.subTest("allow-dirty binds a dirty build"):
            dirty = dict(version="slicing 4.0.0 (abcdef012345-dirty)")
            code, _ = self.run_arm(self.tmp / "d", "off", self.tool("d", off=dirty), extra=["--allow-dirty"])
            self.assertEqual(code, 0)


class ZeroTests(Harness):
    def test_zero_passes_equal_arms_and_ignores_trace_records(self):
        runs = self.three()
        self.assertEqual(self.zero(runs), (0, ""))
        # The on arm's trace file may hold anything well formed: it never enters a comparison.
        on = runs / "C" / "on"
        m = json.loads((on / "manifest.json").read_text())
        (on / "trace.txt").write_text(TRACE + "\n" + TRACE.replace("owner", "write") + "\n")
        m["artifacts"]["trace.txt"] = compare.sha256_file(on / "trace.txt")
        (on / "manifest.json").write_text(json.dumps(m))
        self.assertEqual(self.zero(runs)[0], 0)

    def test_allowance_is_empty_and_any_stats_delta_is_rejected(self):
        self.assertEqual(compare.STATS_ALLOWANCE, frozenset())
        self.assertRejected(["dfg-stats differ", "dfg_pruned_order"], on=dict(dfg=stats(3, 1)))
        self.assertRejected(["calls-stats differ", "dfg_labels.dfg_fallback_edges_capped"], off=dict(calls=stats(3, 2)))
        self.assertRejected(["dfg_new_counter"], on=dict(dfg=dict(stats(3), dfg_new_counter=0)))

    def test_rejects_owner_redirect_with_identical_byte_dump(self):
        edges = copy.deepcopy(EDGES)
        edges[0] = (D_F, U_G, DF)  # the Exact edge now lands on the same-bytes Use of another owner
        runs = self.three(on=dict(graph=render(NODES, edges)))
        dump = lambda arm: (runs / "C" / arm / "edges.jsonl").read_bytes()
        counts = lambda arm: json.loads((runs / "C" / arm / "manifest.json").read_text())["counts"]
        self.assertEqual(dump("base"), dump("on"))
        self.assertEqual(counts("base"), counts("on"))
        code, err = self.zero(runs)
        self.assertEqual(code, 1)
        self.assertIn("typed edge bag", err)
        self.assertNotIn("byte DataFlow bag", err)
        self.assertNotIn("typed node bag", err)

    def test_rejects_return_redirect_shortcut_confidence_and_isolated_node_mutations(self):
        def set_edge(i, edge):
            return lambda e, n: e.__setitem__(i, edge)
        mutated = {
            "return redirect": (set_edge(1, (RET, CALLER_B, EDGES[1][2])), "typed edge bag"),
            "shortcut bit": (set_edge(2, (F, RET, dict(variant="return_flow", suppress_shortcut=True))), "typed edge bag"),
            "call confidence": (set_edge(3, (F, D_F, dict(variant="call", confidence="nameonly"))), "typed edge bag"),
            "isolated node span": (lambda e, n: n.__setitem__(n.index(ISO), dict(ISO, end_byte=21)), "typed node bag"),
        }
        for name, (mutate, needle) in mutated.items():
            with self.subTest(name):
                edges, nodes = copy.deepcopy(EDGES), copy.deepcopy(NODES)
                mutate(edges, nodes)
                runs = self.tmp / ("m-" + name.replace(" ", "_"))
                tool = self.tool("m-" + name.replace(" ", "_"), on=dict(graph=render(nodes, edges)))
                for arm in compare.TRACE_BY_ARM:
                    self.assertEqual(self.run_arm(runs, arm, tool)[0], 0)
                counts = lambda arm: json.loads((runs / "C" / arm / "manifest.json").read_text())["counts"]
                self.assertEqual(counts("base"), counts("on"))
                code, err = self.zero(runs)
                self.assertEqual(code, 1, err)
                self.assertIn(needle, err)
                self.assertNotIn("byte DataFlow bag", err)

    def test_rejects_duplicate_delta_or_unbound_run(self):
        # A duplicated edge standing in for a removed one keeps every count.
        self.assertRejected(["byte DataFlow bag"], on=dict(edges=[BYTES[0], BYTES[2], BYTES[2]]))
        dup = copy.deepcopy(EDGES)
        dup[2] = dup[0]
        self.assertRejected(["typed edge bag"], on=dict(graph=render(NODES, dup)))
        runs = self.three()
        run = lambda arm: runs / "C" / arm
        with self.subTest("tampered artifact"):
            with open(run("off") / "edges.jsonl", "a") as f:
                f.write(json.dumps(BYTES[0]) + "\n")
            self.assertIn("does not match its manifest", self.zero(runs)[1])
        with self.subTest("copied run shares a run id"):
            shutil.rmtree(run("off"))
            shutil.copytree(run("base"), run("off"))
            code, err = self.zero(runs)
            self.assertEqual(code, 1)
            self.assertIn("duplicate run", err)
        with self.subTest("trace flag does not belong to the arm"):
            shutil.rmtree(run("off"))
            shutil.copytree(run("on"), run("off"))
            self.assertIn("not the C/off run", self.zero(runs)[1])
        with self.subTest("missing manifest"):
            (run("off") / "manifest.json").unlink()
            self.assertIn("no readable manifest", self.zero(runs)[1])
        with self.subTest("missing arm and missing corpus"):
            shutil.rmtree(run("off"))
            self.assertEqual(self.zero(runs)[0], 1)
            self.assertEqual(self.zero(self.three_again(), corpora=("C", "NOPE"))[0], 1)

    def three_again(self):
        return self.three()

    def test_rejects_arms_on_different_binary_or_source(self):
        runs = self.tmp / "runs"
        self.run_arm(runs, "base", self.tool("b"))
        self.run_arm(runs, "off", self.tool("o", salt="other build"))
        (self.repo / "a.py").write_bytes(SRC + b"# edit\n")
        edited = [dict(r, sha256=hashlib.sha256(SRC + b"# edit\n").hexdigest()) if r["kind"] == "file" else r
                  for r in render(NODES, EDGES)]
        self.run_arm(runs, "on", self.tool("n", on=dict(graph=edited)))
        code, err = self.zero(runs)
        self.assertEqual(code, 1)
        self.assertIn("different source trees", err)
        self.assertIn("off and on arms used different binary", err)


class StreamingTests(Harness):
    def test_spilled_bags_keep_multiplicity_owner_and_payload_detection(self):
        # One digest per run file: every bag goes through the on-disk merge.
        with mock.patch.object(compare, "SPILL", 1):
            self.assertEqual(self.zero(self.three()), (0, ""))
            self.assertRejected(["byte DataFlow bag"], on=dict(edges=[BYTES[0], BYTES[2], BYTES[2]]))
            err = self.assertRejected(["typed edge bag", "-1x ", "+1x "],
                                      on=dict(graph=render(NODES, [(D_F, U_G, DF)] + EDGES[1:])))
            self.assertIn('-1x {"edge"', err)  # the sample text is recovered from the artifact, not the digest
            self.assertRejected(["typed edge bag"], on=dict(graph=render(
                NODES, [(D_F, U_F, dict(DF, doubt="alias_unstable"))] + EDGES[1:])))

    def test_graph_bags_cover_every_node_and_hold_digests_not_records(self):
        path = self.tmp / "graph.jsonl"
        path.write_text("".join(json.dumps(r) + "\n" for r in render(NODES, EDGES)))
        with compare.Scratch() as scratch:
            graph = compare.load_graph(path, scratch)
            self.assertEqual((graph["nodes"].total, graph["edges"].total, graph["inputs"].total), (8, 8, 1))
            self.assertEqual(sum(c for _, c in graph["nodes"].runs()), 8)
        with compare.Scratch() as scratch, self.assertRaises(compare.Refused):
            compare.load_graph(path, scratch)  # scratch is gone only after the block; a second use is fine
            raise compare.Refused("expected")

    def test_peak_python_memory_does_not_grow_with_record_count(self):
        n = 20000
        nodes = [dict(F, name=f"f{i}" + "x" * 200, start_byte=i) for i in range(n)]
        edges = [(nodes[i], nodes[i + 1], CONTAINS) for i in range(n - 1)]
        path = self.tmp / "big.jsonl"
        path.write_text("".join(json.dumps(r) + "\n" for r in render(nodes, edges)))
        size = path.stat().st_size
        tracemalloc.start()
        try:
            with compare.Scratch() as scratch, mock.patch.object(compare, "SPILL", 1000):
                compare.load_graph(path, scratch)
            peak = tracemalloc.get_traced_memory()[1]
        finally:
            tracemalloc.stop()
        self.assertLess(peak, size // 2, f"peak {peak} bytes for a {size} byte stream")

    def test_verify_rederives_bags_from_artifact_bytes(self):
        runs = self.three()
        run = lambda arm: ["verify", "--run", str(runs / "C" / arm), "--corpus", "C", "--arm", arm]
        with contextlib.redirect_stderr(io.StringIO()), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(compare.main(run("on")), 0)
            self.assertEqual(compare.main(run("off")[:-1] + ["on"]), 1)  # the off run is not the on arm
            with open(runs / "C" / "on" / "graph.jsonl", "a") as f:
                f.write("\n")
            self.assertEqual(compare.main(run("on")), 1)


if __name__ == "__main__":
    unittest.main()
