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
        at = {"file": o["file"], "owner": o["name"], "owner_line": o["owner_line"]}
        mine = [r for r in rows if r.get("file") == o["file"] and r.get("owner") == o["owner"]]
        preds = [r for r in mine if r["kind"] == "prediction"]
        out.append(dict(kind="owner", **at, span=list(o["owner_span"]), writes=o["writes"], nodes=o["reads"], rd=o["rd"],
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

    def function(self, file="a.py", src=SRC, nodes=None, owner="f", line=1, def_line=1):
        """One function owner; its recorded C3 read is `return d` and its W the first store.

        `line` is the line the product keys the owner by (a decorator's), `def_line` the census's."""
        self.files[file] = src
        self.nodes += list(standard_nodes(src, file, owner, line) if nodes is None else nodes)
        first = b'd["k"]' if b'd["k"]' in src else b'd["a"]' if b'd["a"]' in src else b'd["0"]'
        self.frame[(file, f"{owner}@{def_line}")] = [(span(src, first)[0], span(src, b"return d", b"d"), "return")]

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
        self.assertEqual(residual["residual"],
                         [dict(field="raw_pairs", product=7, independent=1, independent_max=1)])

    def test_decorated_owner_joins_on_the_product_start_line(self):
        """The product keys a decorated function by its first decorator's line, the census by the def line."""
        w = self.world
        src = b"@deco\n" + SRC
        w.function("a.py", src, owner="f", line=1, def_line=2)
        code, out, err = w.prepare()
        self.assertEqual(code, 0, err)
        row, = ad.jsonl(w.root / "adm" / "owners.jsonl")
        self.assertEqual((row["owner"], row["status"], row["candidates"]), ("f@2", "ok", 1))
        cand, = ad.jsonl(w.root / "adm" / "candidates.jsonl")
        self.assertEqual(cand["key"].split("|")[:3], ["P1", "a.py", "f@2"])
        self.assertEqual(self.a_of(w.report()[1]), 1)

    def test_decorated_owner_with_the_wrong_product_line_or_span_is_refused(self):
        src = b"@deco\n" + SRC

        def line_of_def(records):  # the product keyed it by the def line: not the product's own identity
            for r in records:
                r["owner_line"] = 2 if "owner_line" in r else None

        def span_start_at_def(records):
            next(r for r in records if r["kind"] == "owner")["span"][0] += 6

        for name, hook in (("line", line_of_def), ("span", span_start_at_def)):
            with tempfile.TemporaryDirectory() as tmp:
                w = World(tmp)
                w.function("a.py", src, owner="f", line=1, def_line=2)
                code, _, _ = w.prepare(mutate_trace=hook)
                self.assertEqual(code, 1, name)
                self.assertEqual(ad.jsonl(Path(tmp, "adm", "candidates.jsonl")), [], name)
                row, = ad.jsonl(Path(tmp, "adm", "owners.jsonl"))
                self.assertEqual(row["status"], "missing_trace", name)

    def test_unestablished_decorator_start_joins_nothing(self):
        w = self.world
        src = b"@(  # why\n    deco)\n" + SRC
        w.function("a.py", src, owner="f", line=1, def_line=3)

        def product_knows_its_start(records):  # the product does know where it starts
            for r in records:
                r["owner_line"] = 1
            next(r for r in records if r["kind"] == "owner")["span"] = [0, len(src) - 1]

        code, _, _ = w.prepare(mutate_trace=product_knows_its_start)
        owner, = [r for r in ad.jsonl(w.root / "ind.jsonl") if r["kind"] == "owner"]
        self.assertEqual((owner["line"], owner["owner_line"]), (3, None))
        self.assertEqual(code, 1)
        self.assertEqual(ad.jsonl(w.root / "adm" / "candidates.jsonl"), [])
        row, = ad.jsonl(w.root / "adm" / "owners.jsonl")
        self.assertEqual(row["status"], "missing_trace")

    def test_trailing_comment_may_extend_the_product_span_but_nothing_else(self):
        src = SRC.replace(b"return d\n", b"return d  # note\n")
        for name, extra, want in (("comment", len(b"  # note"), 0), ("code", len(b"  # note") + 1, 1)):
            with tempfile.TemporaryDirectory() as tmp:
                w = World(tmp)
                w.function("a.py", src)

                def stretch(records, extra=extra):
                    next(r for r in records if r["kind"] == "owner")["span"][1] += extra

                self.assertEqual(w.prepare(mutate_trace=stretch)[0], want, name)

    def test_function_without_a_candidate_store_reconciles_when_the_product_predicts_nothing(self):
        """The product lists an owner (refused stores) the independent side found no store in."""
        w = self.world
        w.function()
        hsrc = b"def h(x):\n    y = x\n    return y\n"
        w.files["h.py"] = hsrc
        w.frame[("h.py", "h@1")] = [(span(hsrc, b"y = x")[0], span(hsrc, b"return y", b"y"), "return")]
        owner = dict(kind="owner", file="h.py", owner="h", owner_line=1, span=[0, len(hsrc) - 1], writes=2, nodes=0,
                     rd="available", raw_pairs=0, raw_overflow=False, raw_capped=False, reads_over_fanin=0,
                     omitted_fanin_pairs=0, over_edge_cap=False, omitted_cap_pairs=0,
                     attrition={s: 0 for s in ad.STAGES}, pairs=[])
        writes = [dict(kind="write", file="h.py", owner="h", owner_line=1, bytes=[10, 15]) for _ in range(2)]

        def product_lists_h(records):
            records += [owner, *writes]

        code, _, err = w.prepare(mutate_trace=product_lists_h)
        self.assertEqual(code, 0, err)
        rows = {r["file"]: r for r in ad.jsonl(w.root / "adm" / "owners.jsonl")}
        self.assertEqual((rows["h.py"]["status"], rows["h.py"]["agreed"], rows["h.py"]["candidates"] if "candidates" in rows["h.py"] else 0),
                         ("ok", 0, 0))
        self.assertEqual(rows["a.py"]["status"], "ok")
        self.assertEqual(self.a_of(w.report()[1]), 1)

    def test_function_without_a_candidate_store_but_a_product_prediction_is_unreconciled(self):
        w = self.world
        hsrc = b"def h(x):\n    y = x\n    return y\n"
        w.files["h.py"] = hsrc
        w.frame[("h.py", "h@1")] = [(span(hsrc, b"y = x")[0], span(hsrc, b"return y", b"y"), "return")]
        pair = dict(w=[10, 15], r=[30, 31], role="Return", join="joined")
        owner = dict(kind="owner", file="h.py", owner="h", owner_line=1, span=[0, len(hsrc) - 1], writes=1, nodes=1,
                     rd="available", raw_pairs=1, raw_overflow=False, raw_capped=False, reads_over_fanin=0,
                     omitted_fanin_pairs=0, over_edge_cap=False, omitted_cap_pairs=0,
                     attrition={s: 0 for s in ad.STAGES}, pairs=[pair])
        records = [owner, dict(kind="write", file="h.py", owner="h", owner_line=1, bytes=[10, 15]),
                   dict(kind="node", file="h.py", owner="h", owner_line=1, bytes=[30, 31]),
                   dict(kind="prediction", file="h.py", owner="h", owner_line=1, w=[10, 15], r=[30, 31],
                        role="Return", outcome="predicted")]
        w.function()
        code, _, _ = w.prepare(mutate_trace=lambda rs: rs.extend(records))
        self.assertEqual(code, 0)
        rows = {r["file"]: r for r in ad.jsonl(w.root / "adm" / "owners.jsonl")}
        self.assertEqual(rows["h.py"]["status"], "unreconciled")
        self.assertEqual({c["file"] for c in ad.jsonl(w.root / "adm" / "candidates.jsonl")}, {"a.py"})

    def test_raw_work_may_exceed_the_clean_count_only_up_to_the_join_refused_endpoints(self):
        """The reader shows the graph after first-wins materialization; the product counts raw rows."""
        for name, product_raw, status in (("below", 0, "unreconciled"), ("clean", 1, "ok"),
                                          ("collapsed", 2, "ok"), ("above", 3, "unreconciled")):
            with tempfile.TemporaryDirectory() as tmp:
                w = World(tmp)
                w.function()

                def hooks(rows):
                    owner = next(r for r in rows if r["kind"] == "owner")
                    owner["raw_pairs_max"] = 2

                def skew(records, raw=product_raw):
                    next(r for r in records if r["kind"] == "owner")["raw_pairs"] = raw

                self.assertEqual(w.prepare(mutate_ind=hooks, mutate_trace=skew)[0], 0, name)
                row, = ad.jsonl(Path(tmp, "adm", "owners.jsonl"))
                self.assertEqual(row["status"], status, name)

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
