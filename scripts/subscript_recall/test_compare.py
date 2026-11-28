import contextlib
import copy
import hashlib
import io
import json
import os
import shutil
import subprocess
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
BASE_VERSION = "slicing 4.0.0 (123456789abc)"
BASE_EXPECT = "123456789abcdef0123456789abcdef012345678"
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


def render(nodes, edges, version="4.0.0", build_identity="abcdef012345"):
    touched = {json.dumps(e[i], sort_keys=True) for e in edges for i in (0, 1)}
    meta = dict(kind="meta", schema=compare.READER_SCHEMA, prism_version=version)
    if build_identity is not None:
        meta["build_identity"] = build_identity
    rows = [meta,
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
# {salt}
"""


class Harness(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        # The fake tools are plain directories; each stands for a worktree whose HEAD the test names.
        self.real_worktree_of = compare.worktree_of
        self.worktrees = {}
        patch = mock.patch.object(compare, "worktree_of", lambda p: self.worktrees.get(
            Path(p).resolve().parent, (None, None)))
        patch.start()
        self.addCleanup(patch.stop)
        self.repo = self.tmp / "repo"
        self.repo.mkdir()
        (self.repo / "a.py").write_bytes(SRC)

    def tool(self, name, salt="", head=EXPECT, **per_arm):
        """A fake prism and typed reader in a worktree whose HEAD is `head`; `per_arm` maps an arm to the
        canned fields it overrides."""
        d = self.tmp / name
        d.mkdir(exist_ok=True)
        self.worktrees[d.resolve()] = (str(d.resolve()), head)
        for arm in compare.TRACE_BY_ARM:
            data = dict(version=VERSION, edges=BYTES, graph=None, dfg=stats(3), calls=stats(3, 1),
                        edges_err=[TRACE] if arm == "on" else [], prism_exit=0, reader_exit=0, touch=None)
            data.update(per_arm.get(arm, {}))
            if data["graph"] is None:  # a reader built with its binary reports that binary's identity
                data["graph"] = render(NODES, EDGES, build_identity=compare.VERSION_RE.match(data["version"]).group(2)
                                       + ("-dirty" if "-dirty" in data["version"] else ""))
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

    def zero(self, runs, corpora=("C",), extra=("--allow-same-binary",)):
        """The smoke form by default: the fake arms share one executable."""
        err = io.StringIO()
        with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
            code = compare.main(["zero", "--runs", str(runs), "--corpora", *corpora, *extra])
        return code, err.getvalue()

    def bound_runs(self, base_tool=None, head_tool=None, base_expect=BASE_EXPECT):
        """base from one executable (version BASE_VERSION), off and on from another."""
        self.n = getattr(self, "n", 0) + 1
        runs = self.tmp / f"bound{self.n}"
        base = base_tool or self.tool(f"base{self.n}", salt="base build", head=BASE_EXPECT,
                                      base=dict(version=BASE_VERSION))
        head = head_tool or self.tool(f"head{self.n}", salt="head build")
        self.assertEqual(self.run_arm(runs, "base", base, expect=base_expect)[0], 0)
        for arm in ("off", "on"):
            self.assertEqual(self.run_arm(runs, arm, head)[0], 0)
        return runs

    STRICT = ("--expect-base", BASE_EXPECT, "--expect-head", EXPECT)

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


class BaseBindingTests(Harness):
    def test_distinct_base_and_head_executables_pass_when_bound_to_their_revisions(self):
        runs = self.bound_runs()
        self.assertEqual(self.zero(runs, extra=self.STRICT), (0, ""))
        m = {arm: json.loads((runs / "C" / arm / "manifest.json").read_text()) for arm in compare.TRACE_BY_ARM}
        self.assertNotEqual(m["base"]["reader"]["sha256"], m["on"]["reader"]["sha256"])
        self.assertNotEqual(m["base"]["binary"]["sha256"], m["on"]["binary"]["sha256"])

    def test_three_runs_of_one_executable_are_refused_without_the_smoke_flag(self):
        runs = self.three()
        code, err = self.zero(runs, extra=())
        self.assertEqual(code, 1, err)
        self.assertIn("same executable", err)
        self.assertEqual(self.zero(runs, extra=("--allow-same-binary",)), (0, ""))
        # naming revisions does not make one executable two
        code, err = self.zero(runs, extra=self.STRICT)
        self.assertEqual(code, 1, err)
        self.assertIn("same executable", err)

    def test_base_must_have_a_different_version_identity_from_the_head(self):
        runs = self.tmp / "same-identity"
        base, head = self.tool("b", salt="base build"), self.tool("h", salt="head build")  # distinct hashes
        for arm, tool in (("base", base), ("off", head), ("on", head)):
            self.assertEqual(self.run_arm(runs, arm, tool)[0], 0)  # all report abcdef012345
        code, err = self.zero(runs, extra=("--expect-base", EXPECT, "--expect-head", EXPECT))
        self.assertEqual(code, 1, err)
        self.assertIn("identity", err)
        code, err = self.zero(runs, extra=())  # no expectations at all: refused as well
        self.assertEqual(code, 1, err)

    def test_head_reader_supplied_for_the_base_arm_is_refused(self):
        runs = self.tmp / "reader-swap"
        base = self.tool("b", salt="base build", head=BASE_EXPECT, base=dict(version=BASE_VERSION))
        head = self.tool("h", salt="head build")
        self.assertEqual(self.run_arm(runs, "base", base, expect=BASE_EXPECT)[0], 0)
        for arm in ("off", "on"):
            self.assertEqual(self.run_arm(runs, arm, head)[0], 0)
        # re-capture the base arm with the base binary but the head reader
        shutil.rmtree(runs / "C" / "base")
        mixed = self.tmp / "mixed"
        shutil.copytree(base, mixed)
        self.worktrees[mixed.resolve()] = (str(mixed.resolve()), BASE_EXPECT)
        shutil.copy(head / "reader", mixed / "reader")
        self.assertEqual(self.run_arm(runs, "base", mixed, expect=BASE_EXPECT)[0], 0)
        code, err = self.zero(runs, extra=self.STRICT)
        self.assertEqual(code, 1, err)
        self.assertIn("reader", err)
        self.assertEqual(self.zero(runs, extra=self.STRICT + ("--allow-same-binary",))[0], 0)

    def test_arms_are_bound_to_the_expected_revisions(self):
        runs = self.bound_runs()
        for name, extra in (("wrong base", ("--expect-base", "fedcba987654", "--expect-head", EXPECT)),
                            ("wrong head", ("--expect-base", BASE_EXPECT, "--expect-head", "fedcba987654")),
                            ("base only", ("--expect-base", BASE_EXPECT)),
                            ("head only", ("--expect-head", EXPECT)),
                            ("short", ("--expect-base", "abc", "--expect-head", EXPECT))):
            with self.subTest(name):
                code, err = self.zero(runs, extra=extra)
                self.assertEqual(code, 1, err)
        # swapped roles: the head executable as base is not the expected base
        code, err = self.zero(runs, extra=("--expect-base", EXPECT, "--expect-head", BASE_EXPECT))
        self.assertEqual(code, 1, err)

    def test_manifest_without_reader_hash_is_unbound(self):
        runs = self.bound_runs()
        path = runs / "C" / "base" / "manifest.json"
        m = json.loads(path.read_text())
        del m["reader"]["sha256"]
        path.write_text(json.dumps(m))
        self.assertEqual(self.zero(runs, extra=self.STRICT)[0], 1)

    def reader_tamper(self, runs, arm, **fields):
        path = runs / "C" / arm / "manifest.json"
        m = json.loads(path.read_text())
        for key, value in fields.items():
            if value is None:
                m.pop(key, None)
            else:
                m[key] = value
        path.write_text(json.dumps(m))

    def test_run_captures_the_binary_and_reader_worktree_heads_from_git(self):
        tool = self.tool("g")
        git = lambda *a: subprocess.run(["git", "-C", str(tool), "-c", "user.email=t@t", "-c", "user.name=t", *a],
                                        capture_output=True, text=True, check=True).stdout.strip()
        git("init", "-q")
        git("add", "-A")
        git("commit", "-q", "-m", "x")
        with mock.patch.object(compare, "worktree_of", self.real_worktree_of):
            self.assertEqual(self.run_arm(self.tmp / "g-runs", "off", tool)[0], 0)
        m = json.loads((self.tmp / "g-runs" / "C" / "off" / "manifest.json").read_text())
        head = git("rev-parse", "HEAD")
        self.assertEqual((m["reader_worktree_head"], m["binary_worktree_head"]), (head, head))
        self.assertEqual((m["reader_worktree"], m["binary_worktree"]), (str(tool.resolve()),) * 2)
        # outside any worktree nothing is recorded (and `zero` then refuses)
        plain = self.tool("plain")
        with mock.patch.object(compare, "worktree_of", self.real_worktree_of), mock.patch.dict(
                os.environ, {"GIT_CEILING_DIRECTORIES": str(self.tmp)}):
            self.assertEqual(self.run_arm(self.tmp / "p-runs", "off", plain)[0], 0)
        m = json.loads((self.tmp / "p-runs" / "C" / "off" / "manifest.json").read_text())
        self.assertEqual((m["reader_worktree"], m["reader_worktree_head"]), (None, None))

    def test_a_reader_whose_worktree_head_is_not_recorded_is_refused(self):
        runs = self.bound_runs()
        self.assertEqual(self.zero(runs, extra=self.STRICT), (0, ""))
        for arm in ("base", "off", "on"):
            for field in ("reader_worktree_head", "reader_worktree"):
                with self.subTest(arm=arm, field=field):
                    fresh = self.bound_runs()
                    self.reader_tamper(fresh, arm, **{field: None})
                    code, err = self.zero(fresh, extra=self.STRICT)
                    self.assertEqual(code, 1, err)
                    self.assertIn("reader worktree", err)
        self.reader_tamper(runs, "base", reader_worktree_head="not-a-revision")
        self.assertEqual(self.zero(runs, extra=self.STRICT)[0], 1)

    def test_each_reader_is_bound_to_the_revision_of_its_arm(self):
        base = self.tool("rb", salt="base build", head="fedcba9876543210", base=dict(version=BASE_VERSION))
        runs = self.bound_runs(base_tool=base)
        code, err = self.zero(runs, extra=self.STRICT)
        self.assertEqual(code, 1, err)
        self.assertIn("base arm reader worktree HEAD fedcba9876543210 is not the expected base", err)
        head = self.tool("rh", salt="head build", head="fedcba9876543210")
        runs = self.bound_runs(head_tool=head)
        code, err = self.zero(runs, extra=self.STRICT)
        self.assertEqual(code, 1, err)
        self.assertIn("off arm reader worktree HEAD fedcba9876543210 is not the expected head", err)
        self.assertIn("on arm reader worktree HEAD", err)

    def test_a_different_build_of_the_head_reader_cannot_stand_for_the_base_reader(self):
        """Correct Prism revisions, one package version, a reader digest of its own: still the head's reader."""
        base = self.tool("pb", salt="base build", head=BASE_EXPECT, base=dict(version=BASE_VERSION))
        head = self.tool("ph", salt="head build")
        runs = self.tmp / "swap-runs"
        self.assertEqual(self.run_arm(runs, "base", base, expect=BASE_EXPECT)[0], 0)
        for arm in ("off", "on"):
            self.assertEqual(self.run_arm(runs, arm, head)[0], 0)
        self.assertEqual(self.zero(runs, extra=self.STRICT), (0, ""))
        shutil.rmtree(runs / "C" / "base")
        mixed = self.tmp / "swap-mixed"
        shutil.copytree(base, mixed)
        self.worktrees[mixed.resolve()] = (str(mixed.resolve()), EXPECT)  # built from the head revision
        (mixed / "reader").write_text((head / "reader").read_text().replace("head build", "another head build"))
        (mixed / "reader").chmod(0o755)
        self.assertEqual(self.run_arm(runs, "base", mixed, expect=BASE_EXPECT)[0], 0)
        m = {arm: json.loads((runs / "C" / arm / "manifest.json").read_text()) for arm in compare.TRACE_BY_ARM}
        self.assertNotEqual(m["base"]["reader"]["sha256"], m["off"]["reader"]["sha256"])
        self.assertEqual(m["base"]["version"]["semver"], m["off"]["version"]["semver"])
        code, err = self.zero(runs, extra=self.STRICT)
        self.assertEqual(code, 1, err)
        self.assertIn("base arm reader worktree HEAD", err)
        self.assertIn("same worktree HEAD", err)

    def test_a_head_built_reader_under_the_base_checkout_is_refused(self):
        """Round-3 finding 3: the reader's own build identity, not where it sits, binds it to its arm."""
        head_identity = "abcdef012345"
        base = self.tool("ib", salt="base build", head=BASE_EXPECT,
                         base=dict(version=BASE_VERSION, graph=render(NODES, EDGES, build_identity=head_identity)))
        head = self.tool("ih", salt="head build")
        runs = self.tmp / "ident-runs"
        code, err = self.run_arm(runs, "base", base, expect=BASE_EXPECT)
        self.assertEqual(code, 1, err)
        self.assertIn("build identity", err)
        self.assertTrue((runs / "C" / "base" / "FAILED").exists())

    def test_run_binds_the_reader_to_its_binary_by_build_identity(self):
        for name, graph, needle in (
                ("other revision", render(NODES, EDGES, build_identity="123456789abc"), "build identity"),
                ("unrecorded", render(NODES, EDGES, build_identity=None), "build identity"),
                ("unknown", render(NODES, EDGES, build_identity="unknown"), "build identity"),
                ("dirty reader, clean binary", render(NODES, EDGES, build_identity="abcdef012345-dirty"),
                 "build identity")):
            with self.subTest(name):
                tool = self.tool("r-" + name.replace(" ", "_").replace(",", ""), off=dict(graph=graph))
                code, err = self.run_arm(self.tmp / ("r-runs-" + name.replace(" ", "_").replace(",", "")), "off", tool)
                self.assertEqual(code, 1, err)
                self.assertIn(needle, err)
        tool = self.tool("rd", off=dict(version="slicing 4.0.0 (abcdef012345-dirty)"))
        code, _ = self.run_arm(self.tmp / "rd-runs", "off", tool, extra=["--allow-dirty"])
        self.assertEqual(code, 0)
        m = json.loads((self.tmp / "rd-runs" / "C" / "off" / "manifest.json").read_text())
        self.assertEqual(m["reader"]["build_identity"], "abcdef012345-dirty")

    def rewrite_graph_identity(self, runs, arm, identity):
        """Re-label an arm's captured reader stream as built from `identity`, with every digest updated."""
        d = runs / "C" / arm
        rows = [json.loads(x) for x in (d / "graph.jsonl").read_text().splitlines()]
        rows[0]["build_identity"] = identity
        (d / "graph.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows))
        m = json.loads((d / "manifest.json").read_text())
        m["artifacts"]["graph.jsonl"] = compare.sha256_file(d / "graph.jsonl")
        m["reader"]["build_identity"] = identity
        (d / "manifest.json").write_text(json.dumps(m))

    def test_zero_rebinds_each_reader_to_its_arm_from_the_stream_it_captured(self):
        runs = self.bound_runs()
        self.assertEqual(self.zero(runs, extra=self.STRICT), (0, ""))
        m = {arm: json.loads((runs / "C" / arm / "manifest.json").read_text()) for arm in compare.TRACE_BY_ARM}
        self.assertEqual((m["base"]["reader"]["build_identity"], m["on"]["reader"]["build_identity"]),
                         ("123456789abc", "abcdef012345"))
        # the base reader's stream says it was built from head code (manifest and digests re-written to agree)
        fresh = self.bound_runs()
        self.rewrite_graph_identity(fresh, "base", "abcdef012345")
        code, err = self.zero(fresh, extra=self.STRICT)
        self.assertEqual(code, 1, err)
        self.assertIn("base arm reader build identity abcdef012345 is not the expected base", err)
        # a head arm whose reader was built elsewhere
        fresh = self.bound_runs()
        self.rewrite_graph_identity(fresh, "on", "123456789abc")
        code, err = self.zero(fresh, extra=self.STRICT)
        self.assertEqual(code, 1, err)
        self.assertIn("on arm reader build identity 123456789abc is not the expected head", err)
        # the manifest's recorded identity must be the stream's
        fresh = self.bound_runs()
        self.reader_tamper(fresh, "base", reader=dict(m["base"]["reader"], build_identity="abcdef012345"))
        code, err = self.zero(fresh, extra=self.STRICT)
        self.assertEqual(code, 1, err)
        self.assertIn("build identity", err)
        # a reader that records no identity is unbound
        fresh = self.bound_runs()
        self.rewrite_graph_identity(fresh, "base", None)
        code, err = self.zero(fresh, extra=self.STRICT)
        self.assertEqual(code, 1, err)
        self.assertIn("build identity", err)
        # smoke runs (one executable) stay allowed
        self.assertEqual(self.zero(self.three()), (0, ""))

    def test_the_base_reader_may_not_share_a_worktree_head_with_the_head_arms(self):
        runs = self.bound_runs()
        self.reader_tamper(runs, "base", reader_worktree_head=EXPECT)
        code, err = self.zero(runs, extra=self.STRICT)
        self.assertEqual(code, 1, err)
        self.assertIn("same worktree HEAD", err)

    def test_a_reader_outside_its_recorded_worktree_is_refused(self):
        runs = self.bound_runs()
        self.reader_tamper(runs, "base", reader_worktree=str(self.tmp / "elsewhere"))
        code, err = self.zero(runs, extra=self.STRICT)
        self.assertEqual(code, 1, err)
        self.assertIn("is not under its worktree", err)

    def test_smoke_runs_need_no_worktree_binding(self):
        runs = self.three()
        self.reader_tamper(runs, "base", reader_worktree_head=None, reader_worktree=None)
        self.assertEqual(self.zero(runs), (0, ""))


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



class LabelCounterCoverageTests(unittest.TestCase):
    def test_label_counters_cover_every_dfg_label_counter_the_product_declares(self):
        import re
        build = os.path.join(os.path.dirname(__file__), "..", "..", "src", "cpg", "build.rs")
        with open(build, encoding="utf-8") as f:
            declared = set(re.findall(r"pub (dfg_label_(?:exact|nameonly_\w+)): usize", f.read()))
        self.assertIn("dfg_label_nameonly_builtin_callback", declared)  # control: the parse sees the new counter
        self.assertEqual(declared - {"dfg_label_loop_carried"}, set(compare.LABEL_COUNTERS))

if __name__ == "__main__":
    unittest.main()
