import json
import tempfile
import unittest
from pathlib import Path

import loader_inventory as li

SRC = b'def f(d, v):\n    d["k"] = v\n    return d\n'
FLOW = dict(variant="data_flow", confidence="exact", doubt=None, kill_line=None)


def span(src, context, target=None):
    """Byte span of `target` inside the unique `context` (the whole context by default)."""
    assert src.count(context) == 1, context
    at = src.index(context) + (context.index(target) if target else 0)
    return at, at + len(target or context)


def var(file, owner, olines, src, access, context, path, target=None):
    start, end = span(src, context, target)
    return dict(kind="node", type="variable", file=file, owner=owner, owner_start_line=olines,
                line=src[:start].count(b"\n") + 1, access=access,
                path=dict(base=path[0], fields=list(path[1:])), start_byte=start, end_byte=end, isolated=False)


def standard_nodes(src=SRC, file="a.py", owner="f", line=1):
    return (var(file, owner, line, src, "def", b'd["k"]', ("d", "[]")),
            var(file, owner, line, src, "use", b"return d", ("d",), b"d"))


def write_run(root, corpus, files, skips=(), nodes=(), edges=(), complete=True):
    """OUT/<corpus>/base with a manifest, a repo of `files` and a typed reader stream."""
    repo = Path(root, "repo-" + corpus)
    repo.mkdir(parents=True, exist_ok=True)
    rows = [dict(kind="meta", schema="element_recall_graph/1", prism_version="4.0.0")]
    for name, data in files.items():
        (repo / name).parent.mkdir(parents=True, exist_ok=True)
        (repo / name).write_bytes(data)
        if name.endswith((".py", ".txt")):
            import hashlib
            rows.append(dict(kind="file", key=name, sha256=hashlib.sha256(data).hexdigest()))
    rows += [dict(kind="skip", path=p, reason="too_large") for p in skips]
    rows += list(nodes)
    rows += [dict(kind="edge", edge=FLOW, **{"from": a, "to": b}) for a, b in edges]
    run = Path(root, "OUT", corpus, "base")
    run.mkdir(parents=True)
    (run / "graph.jsonl").write_text("".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows))
    (run / "manifest.json").write_text(json.dumps(dict(
        complete=complete, corpus=corpus, repo=str(repo), run_id="run-" + corpus,
        source=dict(digest="digest-" + corpus), version=dict(identity="abc"))))
    return run


def rows_of(run, **kw):
    return li.corpus_records("P1", run, **kw)


def predictions(rows):
    return [r for r in rows if r["kind"] == "prediction"]


class LoaderInventoryTests(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)

    def run_of(self, files=None, **kw):
        return write_run(self.dir.name, "P1", files or {"a.py": SRC}, **kw)

    def test_inventory_uses_full_loader_manifest(self):
        other = b'def g(x, y):\n    x["z"] = y\n    return x\n'
        files = {"a.py": SRC, "pkg/b.py": other, "c.txt": b"d['k'] = 1\n", "e.py": b"def broken(:\n"}
        run = write_run(self.dir.name, "P1", files, skips=["huge.py", "data.json"],
                        nodes=[*standard_nodes(),
                               var("pkg/b.py", "g", 1, other, "def", b'x["z"]', ("x", "[]")),
                               var("pkg/b.py", "g", 1, other, "use", b"return x", ("x",), b"x")])
        rows = rows_of(run)
        owners = [r for r in rows if r["kind"] == "owner"]
        self.assertEqual([(r["file"], r["owner"]) for r in owners], [("a.py", "f@1"), ("pkg/b.py", "g@1")])
        status = {r["file"]: r["status"] for r in rows if r["kind"] == "file"}
        self.assertEqual(status, {"a.py": "parsed", "pkg/b.py": "parsed", "e.py": "parse_error",
                                  "huge.py": "skipped"})
        self.assertEqual(rows[-1], dict(kind="end", corpus="P1", owners=2, predictions=2, files=4))
        self.assertEqual(rows[0]["run_id"], "run-P1")
        self.assertEqual(len([p for p in predictions(rows) if p["outcome"] == "predicted"]), 2)

    def test_full_manifest_not_only_listed_owners(self):
        run = self.run_of({"a.py": SRC, "b.py": b"def h():\n    return 1\n"}, nodes=list(standard_nodes()))
        files = [r for r in rows_of(run) if r["kind"] == "file"]
        self.assertEqual([(r["file"], r["owners"]) for r in files], [("a.py", 1), ("b.py", 1)])

    def test_exact_nodes_predict_with_route_phases(self):
        pair, = predictions(rows_of(self.run_of(nodes=list(standard_nodes()))))
        self.assertEqual((pair["outcome"], pair["role"], pair["occupied"], pair["parent_route"]),
                         ("predicted", "return", False, False))
        self.assertEqual(pair["w"], list(span(SRC, b'd["k"]')))
        self.assertEqual(pair["route"][0], "commit")

    def test_missing_use_node_refuses_the_pair(self):
        run = self.run_of(nodes=[standard_nodes()[0]])
        pair, = predictions(rows_of(run))
        self.assertEqual(pair["outcome"], "refused:join:missing")

    def fresh(self, corpus, *nodes):
        return write_run(self.dir.name, corpus, {"a.py": SRC}, nodes=list(nodes))

    def test_truncated_and_mismatched_def_nodes(self):
        w, r = standard_nodes()
        short = dict(w, end_byte=w["end_byte"] - 1)
        pair, = predictions(rows_of(self.fresh("T1", short, r)))
        self.assertEqual(pair["outcome"], "refused:join:truncated")
        sub = dict(w, start_byte=w["start_byte"] + 1, end_byte=w["end_byte"] - 1)
        pair, = predictions(rows_of(self.fresh("T2", sub, r)))
        self.assertEqual(pair["outcome"], "refused:join:truncated")
        wide = dict(w, start_byte=w["start_byte"] - 1, end_byte=w["end_byte"] + 1)
        pair, = predictions(rows_of(self.fresh("T3", wide, r)))
        self.assertEqual(pair["outcome"], "refused:join:mismatched")

    def test_other_owner_node_does_not_join(self):
        w, r = standard_nodes()
        pair, = predictions(rows_of(self.run_of(nodes=[w, dict(r, owner="other")])))
        self.assertEqual(pair["outcome"], "refused:join:missing")

    def test_parent_edge_occupies_the_pair(self):
        w, r = standard_nodes()
        pair, = predictions(rows_of(self.run_of(nodes=[w, r], edges=[(w, r)])))
        self.assertEqual(pair["outcome"], "refused:occupied")

    def test_parent_value_route_is_annotated_not_refused(self):
        src = b'def f(d, t):\n    d["b"] = t\n    d["a"] = t\n    return d\n'
        w1 = var("a.py", "f", 1, src, "def", b'd["b"]', ("d", "[]"))
        w2 = var("a.py", "f", 1, src, "def", b'd["a"]', ("d", "[]"))
        r = var("a.py", "f", 1, src, "use", b"return d", ("d",), b"d")
        feed = var("a.py", "f", 1, src, "def", b"d, t", ("t",), b"t")
        use = var("a.py", "f", 1, src, "use", b'"a"] = t', ("t",), b"t")
        run = self.run_of({"a.py": src}, nodes=[w1, w2, r, feed, use], edges=[(feed, use), (feed, r)])
        got = {tuple(p["w"]): p["parent_route"] for p in predictions(rows_of(run))
               if p["outcome"] == "predicted"}
        self.assertEqual(got, {span(src, b'd["b"]'): False, span(src, b'd["a"]'): True})

    def test_container_root_uses_are_not_the_stored_value(self):
        w, r = standard_nodes()
        recv = var("a.py", "f", 1, SRC, "use", b'd["k"] = v', ("d",), b"d")
        param = var("a.py", "f", 1, SRC, "def", b"f(d, v)", ("d",), b"d")
        run = self.run_of(nodes=[w, r, recv, param], edges=[(param, recv), (param, r)])
        pair, = predictions(rows_of(run))
        self.assertEqual((pair["outcome"], pair["parent_route"]), ("predicted", False))

    def test_source_hash_must_match_the_loader_manifest(self):
        run = self.run_of(nodes=list(standard_nodes()))
        (Path(self.dir.name) / "repo-P1" / "a.py").write_bytes(SRC + b"# edited\n")
        with self.assertRaises(ValueError):
            rows_of(run)

    def test_incomplete_run_is_refused(self):
        with self.assertRaises(ValueError):
            rows_of(write_run(self.dir.name, "P1", {"a.py": SRC}, complete=False))

    def test_one_line_owner_has_no_cfg_and_no_prediction(self):
        src = b'def f(d, v): d["k"] = v; return d\n'
        w = var("a.py", "f", 1, src, "def", b'd["k"]', ("d", "[]"))
        r = var("a.py", "f", 1, src, "use", b"return d", ("d",), b"d")
        rows = rows_of(self.run_of({"a.py": src}, nodes=[w, r]))
        owner, = [x for x in rows if x["kind"] == "owner"]
        self.assertEqual((owner["rd"], owner["raw_pairs"]), ("no_cfg", None))
        self.assertTrue(all(p["outcome"] != "predicted" for p in predictions(rows)))

    def test_fanin_owner_cap_equality_and_overflow(self):
        src = (b"def f(d, v):\n" + b"".join(b'    d["%d"] = v\n' % i for i in range(3)) + b"    return d\n")
        nodes = [var("a.py", "f", 1, src, "def", b'd["%d"]' % i, ("d", "[]")) for i in range(3)]
        nodes.append(var("a.py", "f", 1, src, "use", b"return d", ("d",), b"d"))
        run = self.run_of({"a.py": src}, nodes=nodes)

        def outcomes(limits):
            rows = rows_of(run, limits=limits)
            return sorted(p["outcome"] for p in predictions(rows)), [r for r in rows if r["kind"] == "owner"][0]

        got, owner = outcomes((1_000_000, 3, 256))
        self.assertEqual(got, ["predicted"] * 3)
        got, owner = outcomes((1_000_000, 2, 256))
        self.assertEqual((got, owner["reads_over_fanin"], owner["omitted_fanin_pairs"]),
                         (["refused:fanin"] * 3, 1, 3))
        got, owner = outcomes((1_000_000, 8, 3))
        self.assertEqual(got, ["predicted"] * 3)
        got, owner = outcomes((1_000_000, 8, 2))
        self.assertEqual((got, owner["over_edge_cap"], owner["omitted_cap_pairs"]),
                         (["refused:ownercap"] * 3, 1, 3))
        got, owner = outcomes((2, 8, 256))
        self.assertEqual((got, owner["raw_capped"], owner["raw_pairs"]), ([], 1, 3))
        got, owner = outcomes((3, 8, 256))
        self.assertEqual((got, owner["raw_capped"]), (["predicted"] * 3, 0))

    def test_cli_writes_one_stream_for_every_corpus_with_a_base_run(self):
        write_run(self.dir.name, "P1", {"a.py": SRC}, nodes=list(standard_nodes()))
        write_run(self.dir.name, "P2", {"a.py": SRC}, nodes=list(standard_nodes()))
        out = Path(self.dir.name, "independent.jsonl")
        self.assertEqual(li.main(["--runs", str(Path(self.dir.name, "OUT")), "--out", str(out)]), 0)
        rows = [json.loads(line) for line in out.read_text().splitlines()]
        self.assertEqual([r["corpus"] for r in rows if r["kind"] == "end"], ["P1", "P2"])


if __name__ == "__main__":
    unittest.main()
