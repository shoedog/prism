import contextlib
import hashlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import admission as ad
import compare
import loader_inventory as li
from test_loader_inventory import SRC, span, standard_nodes, var, write_run

TAG = {"arg": "Argument", "return": "Return", "assign_rhs": "AssignmentRhs"}
TWO = b'def f(d, v):\n    d["a"] = v\n    d["b"] = v\n    sink(d)\n    return d\n'
CAPS = b'def f(d, v):\n' + b"".join(b'    d["%d"] = v\n' % i for i in range(3)) + b"    return d\n"
sha = lambda data: hashlib.sha256(data).hexdigest()


def dump_lines(rows):
    return "".join(json.dumps(r, sort_keys=True) + "\n" for r in rows)


def product_records(rows):
    """Product-format trace records equal to the independent stream's own pairs (mutated by tests)."""
    out = []
    for o in (r for r in rows if r["kind"] == "owner"):
        at = {"file": o["file"], "owner": o["name"], "owner_line": o["line"]}
        mine = [r for r in rows if r.get("file") == o["file"] and r.get("owner") == o["owner"]]
        preds = [r for r in mine if r["kind"] == "prediction"]
        out.append(dict(kind="owner", **at, span=o["span"], writes=o["writes"], nodes=o["reads"], rd=o["rd"],
                        raw_pairs=o["raw_pairs"], raw_overflow=bool(o["raw_overflow"]),
                        raw_capped=bool(o["raw_capped"]), reads_over_fanin=o["reads_over_fanin"],
                        omitted_fanin_pairs=o["omitted_fanin_pairs"], over_edge_cap=bool(o["over_edge_cap"]),
                        omitted_cap_pairs=o["omitted_cap_pairs"], attrition={s: 0 for s in ad.STAGES},
                        pairs=[dict(w=p["w"], r=p["r"], role=TAG[p["role"]], join="joined") for p in preds]))
        out += [dict(kind="write", **at, bytes=w["span"]) for w in mine if w["kind"] == "write"]
        out += [dict(kind="node", **at, bytes=n["span"]) for n in mine if n["kind"] == "node"]
        out += [dict(kind="prediction", **at, w=p["w"], r=p["r"], role=TAG[p["role"]],
                     outcome="predicted" if p["outcome"] == "predicted" else p["outcome"]) for p in preds]
    return out


class World:
    def __init__(self, root):
        self.root = Path(root)
        self.files, self.nodes, self.edges, self.skips, self.frame = {}, [], [], [], {}
        self.limits = li.LIMITS
        self.witness_all = True

    def function(self, file="a.py", src=SRC, nodes=None, owner="f"):
        """One function owner on line 1; its recorded C3 read is `return d` and its W the first store."""
        self.files[file] = src
        self.nodes += list(standard_nodes(src, file, owner) if nodes is None else nodes)
        first = b'd["k"]' if b'd["k"]' in src else b'd["a"]' if b'd["a"]' in src else b'd["0"]'
        self.frame[(file, f"{owner}@1")] = [(span(src, first)[0], span(src, b"return d", b"d"), "return")]

    def build(self, mutate_ind=None, mutate_trace=None):
        r, files = self.root, {}
        run = write_run(r, "P1", self.files, self.skips, self.nodes, self.edges)
        rows = li.corpus_records("P1", run, self.limits)
        records = product_records(rows)
        if mutate_ind:
            mutate_ind(rows)
        if mutate_trace:
            mutate_trace(records)
        (r / "ind.jsonl").write_text(dump_lines(rows))
        on = r / "OUT" / "P1" / "on"
        on.mkdir()
        text = "".join(compare.TRACE_PREFIX + json.dumps(x) + "\n" for x in records)
        (on / "trace.txt").write_text(text)
        (on / "manifest.json").write_text(json.dumps(dict(
            complete=True, trace=1, run_id="on-P1", source=dict(digest="digest-P1"),
            artifacts={"trace.txt": sha(text.encode())})))
        census = [dict(kind="owner", corpus="P1", file=f, owner=o, lang="python") for (f, o) in self.frame]
        for (f, o), items in self.frame.items():
            census += [dict(kind="whole", corpus="P1", file=f, owner=o, lang="python", consumer=role,
                            r_start=rr[0], r_end=rr[1], w_start=ws, base_text="d") for ws, rr, role in items]
        (r / "census").mkdir()
        (r / "census" / "P1.jsonl").write_text(dump_lines(census))
        keys = sorted(("P1", f, o, "python") for f, o in self.frame)
        sample = [list(keys[0])]
        draw = dict(python_version="3.12", families=dict(python=dict(
            frame_size=len(keys), frame_sha256=sha(json.dumps(keys).encode()), by_corpus=dict(P1=len(keys)),
            by_lang=dict(python=len(keys)), sample=sample, sample_sha256=sha(json.dumps(sample).encode()))))
        (r / "draw.json").write_text(json.dumps(draw))
        (r / "maud").mkdir()
        owners = [dict(family="python", corpus="P1", file=f, owner=o, lang="python") for f, o in self.frame]
        (r / "maud" / "frame-counts.json").write_text(json.dumps(dict(owners=owners)))
        (r / "maud" / "sample-inventory.jsonl").write_text(dump_lines([
            dict(owners[0], counts=dict(would_add_at_c3=1))]))

    def prepare(self, **kw):
        self.build(**kw)
        r = self.root
        return run_cli(["prepare", "--draw", r / "draw.json", "--census-dir", r / "census",
                        "--m-audit", r / "maud", "--runs", r / "OUT", "--independent", r / "ind.jsonl",
                        "--out", r / "adm"])

    def report(self, witnesses="all"):
        meta = json.loads((self.root / "adm" / "prepared.json").read_text())
        cands = ad.jsonl(self.root / "adm" / "candidates.jsonl")
        rows = witnesses
        if witnesses == "all":
            rows = [dict(key=c["key"], verdict="feasible", prepared_sha256=meta["prepared_sha256"]) for c in cands]
        path = self.root / "witnesses.jsonl"
        path.write_text(dump_lines(rows or []))
        return run_cli(["report", "--prepared", self.root / "adm", "--witnesses", path])


def run_cli(argv):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = ad.main([str(a) for a in argv])
    return code, out.getvalue(), err.getvalue()


class AdmissionTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.world = World(tmp.name)

    def a_of(self, out):
        return int(out.split("A=")[1].split(" ")[0])

    def test_two_pairs_count_one_owner_and_non_c3_does_not_credit(self):
        w = self.world
        w.function("a.py", TWO, nodes=[
            var("a.py", "f", 1, TWO, "def", b'd["a"]', ("d", "[]")),
            var("a.py", "f", 1, TWO, "def", b'd["b"]', ("d", "[]")),
            var("a.py", "f", 1, TWO, "use", b"sink(d)", ("d",), b"d"),
            var("a.py", "f", 1, TWO, "use", b"return d", ("d",), b"d")])
        # g: its recorded read has no Use node, so only the non-C3 `sink(d)` survives.
        g = TWO.replace(b"def f", b"def g")
        w.function("b.py", g, owner="g", nodes=[
            var("b.py", "g", 1, g, "def", b'd["a"]', ("d", "[]")),
            var("b.py", "g", 1, g, "use", b"sink(d)", ("d",), b"d")])
        code, out, err = w.prepare()
        self.assertEqual(code, 0, err)
        cands = ad.jsonl(w.root / "adm" / "candidates.jsonl")
        self.assertEqual(sorted({(c["file"], c["w"][0]) for c in cands}),
                         sorted({("a.py", span(TWO, b'd["a"]')[0]), ("a.py", span(TWO, b'd["b"]')[0])}))
        self.assertEqual(len(cands), 2)
        code, out, _ = w.report()
        self.assertEqual((code, self.a_of(out)), (1, 1), out)
        self.assertIn("GATE A >= 100 of 2: STOP (A=1)", out)

    def test_non_c3_reads_can_cap_a_c3_owner(self):
        w = self.world
        w.function("a.py", TWO, nodes=[
            var("a.py", "f", 1, TWO, "def", b'd["a"]', ("d", "[]")),
            var("a.py", "f", 1, TWO, "def", b'd["b"]', ("d", "[]")),
            var("a.py", "f", 1, TWO, "use", b"sink(d)", ("d",), b"d"),
            var("a.py", "f", 1, TWO, "use", b"return d", ("d",), b"d")])
        w.limits = (1_000_000, 8, 3)  # two C3 pairs plus two non-C3 pairs: over the owner cap
        self.assertEqual(w.prepare()[0], 0)
        self.assertEqual(ad.jsonl(w.root / "adm" / "candidates.jsonl"), [])
        self.assertEqual(self.a_of(w.report()[1]), 0)
        for tmp in (tempfile.TemporaryDirectory(),):  # equality passes the owner limit
            self.addCleanup(tmp.cleanup)
            eq = World(tmp.name)
            eq.files, eq.nodes, eq.frame = w.files, w.nodes, w.frame
            eq.limits = (1_000_000, 8, 4)
            self.assertEqual(eq.prepare()[0], 0)
            code, out, _ = eq.report()
            self.assertEqual(self.a_of(out), 1, out)

    def test_capped_missing_and_occupied_pairs_earn_zero(self):
        w = self.world
        nodes = [var("a.py", "f", 1, CAPS, "def", b'd["%d"]' % i, ("d", "[]")) for i in range(3)]
        nodes.append(var("a.py", "f", 1, CAPS, "use", b"return d", ("d",), b"d"))
        w.function("a.py", CAPS, nodes=nodes)
        w.limits = (1_000_000, 2, 256)  # three sources feed the read: fan-in refuses all of them
        self.assertEqual(w.prepare()[0], 0)
        self.assertEqual(ad.jsonl(w.root / "adm" / "candidates.jsonl"), [])
        stale = [dict(key="P1|a.py|f@1|0:0|0:0|return", verdict="feasible")]
        code, out, _ = w.report(stale)
        self.assertEqual((code, self.a_of(out)), (1, 0))
        # a missing Use node and a parent edge each leave no candidate either
        for name, nodes_, edges in (("missing", [n for n in standard_nodes() if n["access"] == "def"], []),
                                    ("occupied", list(standard_nodes()), "edge")):
            with tempfile.TemporaryDirectory() as tmp:
                other = World(tmp)
                other.function("a.py", SRC, nodes=nodes_)
                if edges:
                    other.edges = [tuple(standard_nodes())]
                self.assertEqual(other.prepare()[0], 0, name)
                self.assertEqual(ad.jsonl(Path(tmp, "adm", "candidates.jsonl")), [], name)

    def test_99_stops_100_passes(self):
        w = self.world
        for i in range(100):
            w.function(f"m{i}.py")
        self.assertEqual(w.prepare()[0], 0)
        cands = ad.jsonl(w.root / "adm" / "candidates.jsonl")
        self.assertEqual(len(cands), 100)
        meta = json.loads((w.root / "adm" / "prepared.json").read_text())
        make = lambda n: [dict(key=c["key"], verdict="feasible", prepared_sha256=meta["prepared_sha256"])
                          for c in cands[:n]]
        code, out, _ = w.report(make(99))
        self.assertEqual((code, self.a_of(out)), (1, 99), out)
        self.assertIn("GATE A >= 100 of 100: STOP (A=99)", out)
        code, out, _ = w.report(make(100))
        self.assertEqual((code, self.a_of(out)), (0, 100), out)
        self.assertIn("GATE A >= 100 of 100: PASS (A=100)", out)
        self.assertIn("P1: A=100 of 100", out)

    def test_rejects_incomplete_trace_or_unreconciled_predictions(self):
        def drop_prediction(records):
            records.remove(next(r for r in records if r["kind"] == "prediction"))

        def drop_owner(records):
            records[:] = [r for r in records if r["kind"] == "write"]

        def duplicate_owner(records):
            records.append(dict(records[0]))

        for name, hook in (("prediction", drop_prediction), ("owner", drop_owner), ("duplicate", duplicate_owner)):
            with tempfile.TemporaryDirectory() as tmp:
                w = World(tmp)
                w.function()
                code, _, err = w.prepare(mutate_trace=hook)
                self.assertEqual(code, 1, name)
                self.assertFalse(json.loads(Path(tmp, "adm", "prepared.json").read_text())["valid"])
                code, out, _ = w.report()
                self.assertEqual(code, 2, name)
                self.assertIn("GATE A >= 100 of 1: INVALID", out)
        with tempfile.TemporaryDirectory() as tmp:  # the independent stream lost its end record
            w = World(tmp)
            w.function()
            code, _, _ = w.prepare(mutate_ind=lambda rows: rows.pop())
            self.assertEqual(code, 1)
        with tempfile.TemporaryDirectory() as tmp:  # a product-only prediction is unreconciled
            w = World(tmp)
            w.function()
            extra = lambda records: records.append(dict(next(r for r in records if r["kind"] == "prediction"),
                                                        w=[1, 2]))
            self.assertEqual(w.prepare(mutate_trace=extra)[0], 1)

    def test_disagreeing_predictions_are_a_reconciliation_residual(self):
        w = self.world
        w.function()

        def product_refuses(records):
            for r in records:
                if r["kind"] == "prediction":
                    r["outcome"] = "refused:occupied"

        code, out, err = w.prepare(mutate_trace=product_refuses)
        self.assertEqual(code, 0, err)
        residual, = ad.jsonl(w.root / "adm" / "residuals.jsonl")
        self.assertEqual((residual["file"], residual["owner"]), ("a.py", "f@1"))
        self.assertEqual([x["side"] for x in residual["residual"]], ["independent_only"])
        row, = ad.jsonl(w.root / "adm" / "owners.jsonl")
        self.assertEqual((row["status"], row["product_survivors"], row["independent_survivors"]),
                         ("unreconciled", 0, 1))
        self.assertEqual(ad.jsonl(w.root / "adm" / "candidates.jsonl"), [])
        code, out, _ = w.report()
        self.assertEqual(self.a_of(out), 0)
        self.assertIn("1 residual owners", out)

    def test_any_residual_in_an_owner_zeroes_its_credit_and_blocks_i2(self):
        w = self.world
        w.function("a.py", TWO, nodes=[
            var("a.py", "f", 1, TWO, "def", b'd["a"]', ("d", "[]")),
            var("a.py", "f", 1, TWO, "use", b"sink(d)", ("d",), b"d"),
            var("a.py", "f", 1, TWO, "use", b"return d", ("d",), b"d")])
        w.function("b.py")

        def drop_non_c3(records):
            for r in records:
                if r["kind"] == "prediction" and r["file"] == "a.py" and r["role"] == "Argument":
                    r["outcome"] = "refused:occupied"

        self.assertEqual(w.prepare(mutate_trace=drop_non_c3)[0], 0)
        self.assertEqual({c["file"] for c in ad.jsonl(w.root / "adm" / "candidates.jsonl")}, {"b.py"})
        with mock.patch.object(ad, "GATE", 1):
            code, out, _ = w.report()
        self.assertEqual((code, self.a_of(out)), (4, 1), out)
        self.assertIn("I2: BLOCKED", out)

    def test_work_totals_disagreement_is_a_residual(self):
        w = self.world
        w.function()

        def skew(records):
            next(r for r in records if r["kind"] == "owner")["raw_pairs"] = 7

        self.assertEqual(w.prepare(mutate_trace=skew)[0], 0)
        residual, = ad.jsonl(w.root / "adm" / "residuals.jsonl")
        self.assertEqual(residual["residual"], [dict(field="raw_pairs", product=7, independent=1)])

    def test_loader_skip_keeps_its_status_row(self):
        w = self.world
        w.function()
        w.skips = ["big.py"]
        w.frame[("big.py", "h@9")] = [(5, (7, 9), "arg")]
        code, _, err = w.prepare()
        self.assertEqual(code, 0, err)
        rows = ad.jsonl(w.root / "adm" / "owners.jsonl")
        self.assertEqual({r["file"]: r["status"] for r in rows}, {"a.py": "ok", "big.py": "loader_skip"})

    def test_frame_owner_without_trace_or_skip_invalidates(self):
        w = self.world
        w.function()
        w.frame[("gone.py", "h@9")] = [(5, (7, 9), "arg")]
        code, _, err = w.prepare()
        self.assertEqual(code, 1)
        self.assertEqual({r["file"]: r["status"] for r in ad.jsonl(w.root / "adm" / "owners.jsonl")},
                         {"a.py": "ok", "gone.py": "missing_trace"})

    def test_witness_must_be_feasible_fresh_and_unanimous(self):
        w = self.world
        w.function()
        self.assertEqual(w.prepare()[0], 0)
        key = ad.jsonl(w.root / "adm" / "candidates.jsonl")[0]["key"]
        sha_ = json.loads((w.root / "adm" / "prepared.json").read_text())["prepared_sha256"]
        cases = {
            "feasible": ([dict(key=key, verdict="feasible")], 1),
            "feasible-twice": ([dict(key=key, verdict="feasible"), dict(key=key, verdict="feasible")], 1),
            "unknown": ([dict(key=key, verdict="unknown")], 0),
            "infeasible": ([dict(key=key, verdict="infeasible")], 0),
            "stale-hash": ([dict(key=key, verdict="feasible", prepared_sha256="0" * 64)], 0),
            "stale-verdict": ([dict(key=key, verdict="stale")], 0),
            "conflict": ([dict(key=key, verdict="feasible"), dict(key=key, verdict="infeasible")], 0),
            "unknown-key": ([dict(key=key + "x", verdict="feasible")], 0),
            "none": ([], 0),
        }
        for name, (rows, want) in cases.items():
            self.assertEqual(self.a_of(w.report(rows)[1]), want, name)
        out = w.report([dict(key=key, verdict="feasible", prepared_sha256=sha_, public_stop=True)])[1]
        self.assertIn("A=1 of 1", out)
        self.assertIn("zero public recovery: 1", out)

    def test_parent_value_route_earns_no_candidate(self):
        w = self.world
        src = b'def f(d, t):\n    d["k"] = t\n    return d\n'
        nodes = [var("a.py", "f", 1, src, "def", b'd["k"]', ("d", "[]")),
                 var("a.py", "f", 1, src, "use", b"return d", ("d",), b"d"),
                 var("a.py", "f", 1, src, "def", b"d, t", ("t",), b"t"),
                 var("a.py", "f", 1, src, "use", b"= t", ("t",), b"t")]
        w.function("a.py", src, nodes=nodes)
        w.edges = [(nodes[2], nodes[3]), (nodes[2], nodes[1])]
        self.assertEqual(w.prepare()[0], 0)
        self.assertEqual(ad.jsonl(w.root / "adm" / "candidates.jsonl"), [])

    def test_earlier_w_differing_from_the_recorded_w_earns_the_owner(self):
        w = self.world
        w.function("a.py", TWO, nodes=[
            var("a.py", "f", 1, TWO, "def", b'd["a"]', ("d", "[]")),
            var("a.py", "f", 1, TWO, "def", b'd["b"]', ("d", "[]")),
            var("a.py", "f", 1, TWO, "use", b"sink(d)", ("d",), b"d"),
            var("a.py", "f", 1, TWO, "use", b"return d", ("d",), b"d")])
        self.assertEqual(w.prepare()[0], 0)
        cands = {c["w"][0]: c for c in ad.jsonl(w.root / "adm" / "candidates.jsonl")}
        early, late = span(TWO, b'd["a"]')[0], span(TWO, b'd["b"]')[0]
        self.assertEqual(cands[early]["recorded_w"], [early])
        witness = [dict(key=cands[late]["key"], verdict="feasible")]
        self.assertEqual(self.a_of(w.report(witness)[1]), 1)

    def test_frame_or_m_audit_disagreement_is_refused(self):
        w = self.world
        w.function()
        w.build()
        owners = json.loads((w.root / "maud" / "frame-counts.json").read_text())
        owners["owners"].append(dict(family="python", corpus="P1", file="z.py", owner="z@1", lang="python"))
        (w.root / "maud" / "frame-counts.json").write_text(json.dumps(owners))
        r = w.root
        argv = ["prepare", "--draw", r / "draw.json", "--census-dir", r / "census", "--m-audit", r / "maud",
                "--runs", r / "OUT", "--independent", r / "ind.jsonl", "--out", r / "adm2"]
        self.assertEqual(run_cli(argv)[0], 3)

    def test_wilson_lower_matches_the_recorded_m_bounds(self):
        self.assertAlmostEqual(ad.wilson_lower(58, 60), 0.8864, places=4)
        self.assertAlmostEqual(ad.wilson_lower(56, 60), 0.8407, places=4)
        self.assertGreaterEqual(int(787 * ad.wilson_lower(13, 60)), 100)


if __name__ == "__main__":
    unittest.main()
