import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

import compare
import review_diff as rd

A_PY = b'def f(t):\n    d = {}\n    d["k"] = t\n    sink(d)\n'
B_PY = b'def g():\n    t = src()\n    d = {}\n    d["k"] = t\n    sink(d)\n'
LISTING = ("Available algorithms:\n\n  Paper:\n" + "".join(
    f"    {n:<16} description\n" for n in rd.GRAPH + rd.CONTROL + tuple(rd.UNREPLAYED)))
FAKE = """#!{py}
import json, os, sys
here = os.path.dirname(os.path.abspath(__file__))
cfg = json.load(open(os.path.join(here, "config.json")))
a = sys.argv[1:]
if a == ["--list-algorithms"]:
    print(cfg["listing"]); sys.exit(0)
algo = a[a.index("--algorithm") + 1]
diff = json.load(open(a[a.index("--diff") + 1]))
open(os.path.join(here, "log"), "a").write(json.dumps([algo, a, os.environ.get("PRISM_ELEMENT_RECALL_TRACE"), diff]) + "\\n")
if cfg.get("touch"):
    open(os.path.join(a[a.index("--repo") + 1], "a.py"), "a").write("# touched\\n")
if algo in cfg.get("fail", []):
    sys.exit(2)
sev = "info" if cfg.get("mutate") or (cfg.get("mutate_on_trace") and os.environ.get("PRISM_ELEMENT_RECALL_TRACE") == "1") else "warning"
print(json.dumps(dict(algorithm=algo, slices=[dict(file=f["file_path"], diff_lines=f["diff_lines"]) for f in diff["files"]],
                      findings=[dict(severity=sev, description="x", rules=[dict(id="r1")])] if algo == "taint" else [])))
"""


class Harness(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        self.repo = self.tmp / "repo"
        self.repo.mkdir()
        (self.repo / "a.py").write_bytes(A_PY)
        (self.repo / "b.py").write_bytes(B_PY)
        self.bins = {name: self.fake(name) for name in ("parent", "head")}
        self.manifests()
        pairs = []
        for f, src in (("a.py", A_PY), ("b.py", B_PY)):
            w, r = src.index(b'd["k"]'), src.rindex(b"d)") 
            pairs.append(dict(key=f"C|{f}|k", corpus="C", file=f, owner="o", w=[w, w + 6], r=[r, r + 1], role="arg"))
        self.pairs(pairs)

    def fake(self, name, **cfg):
        d = self.tmp / name
        d.mkdir(exist_ok=True)
        (d / "config.json").write_text(json.dumps({**dict(listing=LISTING), **cfg}))
        (d / "prism").write_text(FAKE.format(py=sys.executable))
        (d / "prism").chmod(0o755)
        return d / "prism"

    def manifests(self, digest=None, base_digest=None, on_bin=None):
        digest = digest or compare.tree_digest(self.repo)[0]
        for arm, binary in (("base", "parent"), ("off", "head"), ("on", "head")):
            b = on_bin if arm == "on" and on_bin else self.bins[binary]
            m = dict(schema=compare.SCHEMA, complete=True, corpus="C", trace=compare.TRACE_BY_ARM[arm],
                     repo=str(self.repo), version=dict(identity=binary), binary=dict(path=str(b), sha256=compare.sha256_file(b)),
                     source=dict(digest=base_digest if arm == "base" and base_digest else digest))
            d = self.tmp / "runs" / "C" / arm
            d.mkdir(parents=True, exist_ok=True)
            (d / "manifest.json").write_text(json.dumps(m))

    def pairs(self, pairs):
        adm = self.tmp / "adm"
        adm.mkdir(exist_ok=True)
        (adm / "candidates.jsonl").write_text("".join(json.dumps(p) + "\n" for p in pairs))
        (adm / "prepared.json").write_text('{"valid": true}')

    def replay(self, *extra, algos="leftflow,chop,conditioned,taint,delta,originaldiff"):
        out = self.tmp / "out"
        shutil.rmtree(out, ignore_errors=True)
        argv = ["replay", "--runs", str(self.tmp / "runs"), "--admission", str(self.tmp / "adm"), "--out", str(out),
                "--controls", str(self.tmp / "none"), "--algorithms", algos, *extra]
        import contextlib, io
        err = io.StringIO()
        with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
            code = rd.main(argv)
        summary = out / "summary.json"
        return code, err.getvalue(), json.loads(summary.read_text()) if summary.exists() else None

    def calls(self, name="head"):
        return [json.loads(x) for x in (self.tmp / name / "log").read_text().splitlines()]


class ReviewDiffTests(Harness):
    def test_clean_replay_passes_and_runs_every_arm(self):
        code, err, s = self.replay()
        self.assertEqual((code, s["verdict"], s["counts"].get("mismatch", 0)), (0, "PASS", 0), err)
        self.assertEqual(s["seed_classes"], dict(store=2, upstream=2))
        self.assertEqual(len(self.calls("parent")), len(self.calls("head")) // 2)  # head ran off and on
        self.assertEqual({c[2] for c in self.calls("head")}, {None, "1"})
        self.assertTrue(all("--no-cache" in c[1] for c in self.calls("head")))
        self.assertEqual(s["upstream"], dict(pairs=2, same_line=0, missing_or_unparsed=0, ambiguous=0,
                                              distinct_upstream_lines=2))

    def test_review_rejects_missing_reader_or_changed_finding(self):
        miss = LISTING.replace("    taint ", "    taint_x ")
        self.fake("head", listing=miss)
        self.fake("parent", listing=miss)
        code, err, _ = self.replay()
        self.assertEqual(code, 3, "a replayed reader missing from --list-algorithms")
        self.assertIn("missing from --list-algorithms ['taint']", err)
        self.fake("head", listing=LISTING + "    newalgo          description\n")
        self.fake("parent", listing=LISTING + "    newalgo          description\n")
        self.assertEqual(self.replay()[0], 3, "an unaccounted dispatch entry")
        self.fake("head", listing=LISTING)  # parent lists newalgo, head does not
        self.assertEqual(self.replay()[0], 3, "parent and head list different algorithms")
        for name, cfg, text in (("head", dict(mutate=True), "parent vs head default"),
                                ("head", dict(mutate_on_trace=True), "head default vs trace-on")):
            self.bins["head"] = self.fake(name, **cfg)
            self.fake("parent")
            self.manifests()
            code, err, s = self.replay()
            self.assertEqual((code, s["verdict"]), (1, "FAIL"), err)
            self.assertGreater(s["counts"]["mismatch"], 0)
            self.assertTrue(any(text in json.loads(l)["detail"] for l in (self.tmp / "out" / "results.jsonl")
                                .read_text().splitlines() if json.loads(l)["status"] == "mismatch"), text)
            self.assertTrue(list((self.tmp / "out").glob("mismatch-*")))
        self.bins["head"] = self.fake("head", fail=["taint"])
        self.manifests()
        code, _, s = self.replay()
        self.assertEqual((code, s["verdict"]), (1, "FAIL"))
        self.assertGreater(s["counts"]["refused"], 0, "a failed arm supplies no comparison")

    def test_delta_counts_bound_formal_without_preimage(self):
        code, err, s = self.replay()
        self.assertEqual(code, 0, err)
        d = s["delta"]
        self.assertEqual((d["bound"], d["eligible"], d["completed"]), (2, 1, 1))
        self.assertEqual(d["no_preimage:no admitted RHS edit"], 1)
        results = [json.loads(l) for l in (self.tmp / "out" / "results.jsonl").read_text().splitlines()]
        formal = [r for r in results if r["status"] == "no_preimage" and r["variant"] == "delta@t"]
        self.assertEqual(len(formal), 1)
        seeds = {x["digest"]: x for x in map(json.loads, (self.tmp / "out" / "seeds.jsonl").read_text().splitlines())}
        formal_seed = seeds[formal[0]["seed"]]
        self.assertEqual((formal_seed["file"], formal_seed["line"], formal_seed["classes"]), ("a.py", 1, ["upstream"]))
        replayed = {c[0] for c in self.calls() if c[3]["files"][0]["file_path"] == "a.py"
                    and c[3]["files"][0]["diff_lines"] == [1]}
        self.assertTrue({"leftflow", "taint", "originaldiff"} <= replayed)  # other readers still replay the formal
        self.assertNotIn("delta", replayed)
        old_repos = [c[1][c[1].index("--old-repo") + 1] for c in self.calls() if c[0] == "delta"]
        self.assertTrue(old_repos and all(not Path(p).exists() for p in old_repos), "old trees are removed")

    def test_seed_replay_requires_matching_files_and_inputs(self):
        self.manifests(base_digest="0" * 64)
        code, err, _ = self.replay()
        self.assertEqual(code, 3)
        self.assertIn("different source trees", err)
        self.manifests()
        (self.repo / "a.py").write_bytes(A_PY + b"x = 1\n")
        code, err, _ = self.replay()
        self.assertEqual(code, 3)
        self.assertIn("differ from the manifests' source digest", err)
        (self.repo / "a.py").write_bytes(A_PY)
        self.bins["head"].write_text(self.bins["head"].read_text() + "# changed\n")
        self.assertIn("does not match its manifest", self.replay()[1])
        self.manifests()
        self.manifests(on_bin=self.bins["parent"])
        self.assertIn("used different binaries", self.replay()[1])
        self.manifests()
        pair = json.loads((self.tmp / "adm" / "candidates.jsonl").read_text().splitlines()[0])
        self.pairs([dict(pair, w=[0, 10_000])])
        self.assertIn("byte span outside", self.replay()[1])
        self.pairs([dict(pair, file="missing.py")])
        self.assertEqual(self.replay()[0], 3)
        self.bins["head"] = self.fake("head", touch=True)  # a binary that rewrites the tree under replay
        self.manifests()
        self.pairs([pair])
        code, err, _ = self.replay()
        self.assertEqual(code, 3)
        self.assertIn("source tree changed during the replay", err)

    def test_seeds_dedupe_and_hash_order_sampling(self):
        pairs = [json.loads(l) for l in (self.tmp / "adm" / "candidates.jsonl").read_text().splitlines()]
        self.pairs([pairs[0], dict(pairs[0], key="C|a.py|other"), pairs[1]])
        code, _, s = self.replay("--sample", "3")
        self.assertEqual(s["seed_classes"], dict(store=2, upstream=2))  # same store line, one seed
        links = [json.loads(l)["links"] for l in (self.tmp / "out" / "seeds.jsonl").read_text().splitlines()]
        self.assertIn(2, {len(x) for x in links})
        chosen = sorted((p["key"] for p in pairs), key=lambda k: rd.sha(k))
        self.replay("--sample", "1")
        kept = {json.loads(l)["links"][0] for l in (self.tmp / "out" / "seeds.jsonl").read_text().splitlines()}
        self.assertEqual(kept, {chosen[0]})

    def test_upstream_forms(self):
        cases = {b"def f(t):\n    d['k'] = t\n": ("formal", 1),
                 b"def f():\n    t = src()\n    d['k'] = t\n": ("plain_assign", 2),
                 b"def f():\n    for t in x:\n        d['k'] = t\n": ("iteration", 2),
                 b"def f():\n    t = u = 1\n    d['k'] = t\n": ("chained_assign", 2),
                 b"def f():\n    t += 1\n    d['k'] = t\n": ("compound", 2)}
        for src, (form, line) in cases.items():
            up = rd.upstream(src, src.count(b"\n"))
            self.assertEqual((up["form"], up["line"]), (form, line), src)
        self.assertEqual(rd.upstream(b"def f():\n    d['k'] = 1\n", 2)["status"], "missing")
        self.assertEqual(rd.upstream(b"def f(:\n", 1)["status"], "unparsed")
        src = b"def f():\n    t = src()\n    d['k'] = t\n"
        edited, _ = rd.plain_edit(src, "plain_assign", rd.upstream(src, 3)["node"])
        self.assertEqual(edited, b"def f():\n    t = \"CLEAN\"\n    d['k'] = t\n")


if __name__ == "__main__":
    unittest.main()
