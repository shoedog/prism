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
FIVE = b'def f(d, v):\n' + b"".join(b'    d["%d"] = v\n' % i for i in range(5)) + b"    return d\n"
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
        self.extra = {}
        self.frozen = None
        self.declared = None  # None: the CLI default; else the --full-loader-corpora value

    def function(self, file="a.py", src=SRC, nodes=None, owner="f", line=1, def_line=1):
        """One function owner; its recorded C3 read is `return d` and its W the first store.

        `line` is the line the product keys the owner by (a decorator's), `def_line` the census's."""
        self.files[file] = src
        self.nodes += list(standard_nodes(src, file, owner, line) if nodes is None else nodes)
        first = b'd["k"]' if b'd["k"]' in src else b'd["a"]' if b'd["a"]' in src else b'd["0"]'
        self.frame[(file, f"{owner}@{def_line}")] = [(span(src, first)[0], span(src, b"return d", b"d"), "return")]

    def emit(self, corpus, files, skips, nodes, edges, limits, mutate_ind=None, mutate_trace=None):
        """One corpus's base run, independent rows and `on` trace; returns the independent rows."""
        r = self.root
        run = write_run(r, corpus, files, skips, nodes, edges)
        rows = li.corpus_records(corpus, run, limits)
        records = product_records(rows)
        if mutate_ind:
            mutate_ind(rows)
        if mutate_trace:
            mutate_trace(records)
        on = r / "OUT" / corpus / "on"
        on.mkdir()
        text = "".join(compare.TRACE_PREFIX + json.dumps(x) + "\n" for x in records)
        (on / "trace.txt").write_text(text)
        (on / "manifest.json").write_text(json.dumps(dict(
            complete=True, trace=1, run_id=f"on-{corpus}", source=dict(digest=f"digest-{corpus}"),
            artifacts={"trace.txt": sha(text.encode())})))
        return rows

    def build(self, mutate_ind=None, mutate_trace=None):
        r = self.root
        rows = self.emit("P1", self.files, self.skips, self.nodes, self.edges, self.limits, mutate_ind, mutate_trace)
        for corpus, (files, nodes) in self.extra.items():  # full-loader corpora outside the frame
            rows += self.emit(corpus, files, [], nodes, [], self.limits)
        (r / "ind.jsonl").write_text(dump_lines(rows))
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
        self.frozen_of = lambda: dict(
            sha=draw["families"]["python"]["frame_sha256"], owners=len(keys), corpora=dict(P1=len(keys)),
            rows=sum(map(len, self.frame.values())), census=dict(P1=sha((r / "census" / "P1.jsonl").read_bytes())))
        (r / "maud").mkdir()
        owners = [dict(family="python", corpus="P1", file=f, owner=o, lang="python") for f, o in self.frame]
        (r / "maud" / "frame-counts.json").write_text(json.dumps(dict(owners=owners)))
        (r / "maud" / "sample-inventory.jsonl").write_text(dump_lines([
            dict(owners[0], counts=dict(would_add_at_c3=1))]))

    def argv(self, out="adm", corpora="own"):
        """`corpora`: "own" declares exactly the corpora this world built, None leaves the CLI default."""
        r = self.root
        argv = ["prepare", "--draw", r / "draw.json", "--census-dir", r / "census", "--m-audit", r / "maud",
                "--runs", r / "OUT", "--independent", r / "ind.jsonl", "--out", r / out]
        if corpora == "own":
            corpora = ",".join(["P1", *self.extra])
        return argv + (["--full-loader-corpora", corpora] if corpora else [])

    def prepare(self, **kw):
        self.build(**kw)
        return run_cli(self.argv())

    def precision(self, verdict=None, witnessed=None, which="a-credit", **tamper):
        """The receipt of the derived draw: `draw`, all-valid judgments (`verdict(key, role)` and
        `witnessed(key)` override one), then `precision`; `tamper` rewrites receipt fields afterwards.
        `which` names the scored population (the all-survivors files carry an `-all` suffix)."""
        prep = self.root / "adm"
        tag = "" if which == "a-credit" else "-all"
        draw_path, path = self.root / f"draw{tag}.json", self.root / f"precision{tag}.json"
        pop = ["--population", which]
        assert run_cli(["draw", "--prepared", prep, "--out", draw_path, *pop])[0] == 0
        draw = json.loads(draw_path.read_text())
        meta = json.loads((prep / "prepared.json").read_text())
        rows = []
        for key, role in sorted(draw["keys"].items()):
            v = verdict(key, role) if verdict else "valid"
            rows.append(dict(key=key, verdict=v, prepared_sha256=meta["prepared_sha256"],
                             witnessed=witnessed(key) if witnessed else v == "valid"))
        (self.root / f"judgments{tag}.jsonl").write_text(dump_lines(rows))
        run_cli(["precision", "--prepared", prep, "--draw", draw_path, "--judgments",
                 self.root / f"judgments{tag}.jsonl", "--out", path, *pop])
        if tamper:
            receipt = json.loads(path.read_text())
            receipt.update(tamper)
            path.write_text(json.dumps(receipt))
        return path

    def report(self, witnesses="all", precision="good", approved="a-credit", all_survivors=None):
        """`approved`: the `--approved-population` value (None: the flag is absent); `all_survivors`: the
        all-survivors receipt path, or "good" for a passing one."""
        meta = json.loads((self.root / "adm" / "prepared.json").read_text())
        cands = ad.jsonl(self.root / "adm" / "candidates.jsonl")
        rows = witnesses
        if witnesses == "all":
            rows = [dict(key=c["key"], verdict="feasible", prepared_sha256=meta["prepared_sha256"]) for c in cands]
        path = self.root / "witnesses.jsonl"
        path.write_text(dump_lines(rows or []))
        argv = ["report", "--prepared", self.root / "adm", "--witnesses", path]
        if precision == "good":
            precision = self.precision()
        if precision:
            argv += ["--precision", precision]
        if all_survivors == "good":
            all_survivors = self.precision(which="all-survivors")
        if all_survivors:
            argv += ["--precision-all-survivors", all_survivors]
        if approved:
            argv += ["--approved-population", approved]
        return run_cli(argv)


def run_cli(argv, **kw):
    """The CLI with the test frame (`frozen=None`: only the draw's own consistency is checked)."""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = ad.main([str(a) for a in argv], **dict(dict(frozen=None), **kw))
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
            var("a.py", "f", 1, TWO, "use", b"return d", ("d",), b"d"),
            var("a.py", "f", 1, TWO, "use", b'd["a"] = v', ("v",), b"v"),
            var("a.py", "f", 1, TWO, "use", b'd["b"] = v', ("v",), b"v")])
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
            var("a.py", "f", 1, TWO, "use", b"return d", ("d",), b"d"),
            var("a.py", "f", 1, TWO, "use", b'd["a"] = v', ("v",), b"v"),
            var("a.py", "f", 1, TWO, "use", b'd["b"] = v', ("v",), b"v")])
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
                    other.edges = [tuple(standard_nodes()[:2])]
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
            var("a.py", "f", 1, TWO, "use", b"return d", ("d",), b"d"),
            var("a.py", "f", 1, TWO, "use", b'd["a"] = v', ("v",), b"v"),
            var("a.py", "f", 1, TWO, "use", b'd["b"] = v', ("v",), b"v")])
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
            "feasible": ([dict(key=key, verdict="feasible", prepared_sha256=sha_)], 1),
            "feasible-twice": ([dict(key=key, verdict="feasible", prepared_sha256=sha_)] * 2, 1),
            "unknown": ([dict(key=key, verdict="unknown", prepared_sha256=sha_)], 0),
            "infeasible": ([dict(key=key, verdict="infeasible", prepared_sha256=sha_)], 0),
            "stale-hash": ([dict(key=key, verdict="feasible", prepared_sha256="0" * 64)], 0),
            "stale-verdict": ([dict(key=key, verdict="stale", prepared_sha256=sha_)], 0),
            "conflict": ([dict(key=key, verdict="feasible", prepared_sha256=sha_),
                          dict(key=key, verdict="infeasible", prepared_sha256=sha_)], 0),
            "unknown-key": ([dict(key=key + "x", verdict="feasible", prepared_sha256=sha_)], 0),
            "unbound": ([dict(key=key, verdict="feasible")], 0),
            "one-row-unbound": ([dict(key=key, verdict="feasible", prepared_sha256=sha_),
                                 dict(key=key, verdict="feasible")], 0),
            "none": ([], 0),
        }
        for name, (rows, want) in cases.items():
            self.assertEqual(self.a_of(w.report(rows)[1]), want, name)
        self.assertIn("'unbound': 1", w.report([dict(key=key, verdict="feasible")])[1])
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

    def test_exhausted_parent_route_search_is_unknown_and_earns_no_candidate(self):
        src = b'def f(d, t):\n    d["k"] = t\n    return d\n'
        nodes = [var("a.py", "f", 1, src, "def", b'd["k"]', ("d", "[]")),
                 var("a.py", "f", 1, src, "use", b"return d", ("d",), b"d"),
                 var("a.py", "f", 1, src, "def", b"d, t", ("t",), b"t"),
                 var("a.py", "f", 1, src, "use", b"= t", ("t",), b"t")]
        junk = [dict(nodes[3], start_byte=900 + i, end_byte=901 + i) for i in range(8)]
        self.world.function("a.py", src, nodes=nodes)
        self.world.edges = [(nodes[2], nodes[3])] + [(nodes[2], j) for j in junk]
        with mock.patch.object(li, "ROUTE_BUDGET", 4):
            self.assertEqual(self.world.prepare()[0], 0)
        pair, = [r for r in ad.jsonl(self.world.root / "ind.jsonl") if r["kind"] == "prediction"]
        self.assertEqual((pair["outcome"], pair["parent_route"]), ("predicted", None))
        self.assertEqual(ad.jsonl(self.world.root / "adm" / "candidates.jsonl"), [])
        row, = ad.jsonl(self.world.root / "adm" / "owners.jsonl")
        self.assertEqual(row["status"], "ok")  # unknown is not a reconciliation residual
        with tempfile.TemporaryDirectory() as tmp:  # control: the finished search leaves the candidate
            other = World(tmp)
            other.function("a.py", src, nodes=nodes)
            other.edges = self.world.edges
            self.assertEqual(other.prepare()[0], 0)
            self.assertEqual(len(ad.jsonl(Path(tmp, "adm", "candidates.jsonl"))), 1)

    def test_earlier_w_differing_from_the_recorded_w_earns_the_owner(self):
        w = self.world
        w.function("a.py", TWO, nodes=[
            var("a.py", "f", 1, TWO, "def", b'd["a"]', ("d", "[]")),
            var("a.py", "f", 1, TWO, "def", b'd["b"]', ("d", "[]")),
            var("a.py", "f", 1, TWO, "use", b"sink(d)", ("d",), b"d"),
            var("a.py", "f", 1, TWO, "use", b"return d", ("d",), b"d"),
            var("a.py", "f", 1, TWO, "use", b'd["a"] = v', ("v",), b"v"),
            var("a.py", "f", 1, TWO, "use", b'd["b"] = v', ("v",), b"v")])
        self.assertEqual(w.prepare()[0], 0)
        cands = {c["w"][0]: c for c in ad.jsonl(w.root / "adm" / "candidates.jsonl")}
        early, late = span(TWO, b'd["a"]')[0], span(TWO, b'd["b"]')[0]
        self.assertEqual(cands[early]["recorded_w"], [early])
        sha_ = json.loads((w.root / "adm" / "prepared.json").read_text())["prepared_sha256"]
        witness = [dict(key=cands[late]["key"], verdict="feasible", prepared_sha256=sha_)]
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

    def five_stores(self, world):
        nodes = [var("a.py", "f", 1, FIVE, "def", b'd["%d"]' % i, ("d", "[]")) for i in range(5)]
        nodes.append(var("a.py", "f", 1, FIVE, "use", b"return d", ("d",), b"d"))
        nodes += [var("a.py", "f", 1, FIVE, "use", b'd["%d"] = v' % i, ("v",), b"v") for i in range(5)]
        world.function("a.py", FIVE, nodes=nodes)
        return nodes

    def test_failed_flood_gate_blocks_i2_and_exits_nonzero(self):
        w = self.world
        self.five_stores(w)
        self.assertEqual(w.prepare()[0], 0)
        self.assertFalse(json.loads((w.root / "adm" / "prepared.json").read_text())["flood"]["P1"]["ok"])
        with mock.patch.object(ad, "GATE", 1):
            code, out, _ = w.report()
        self.assertEqual(self.a_of(out), 1, out)
        self.assertIn("5 <= 4: FAIL", out)
        self.assertIn("I2: BLOCKED", out)
        self.assertIn("flood", out.split("I2: BLOCKED")[1])
        self.assertNotEqual(code, 0)
        self.assertNotIn("ADMITTED", out)

    def test_flood_accounting_covers_every_full_loader_corpus_not_only_the_frame(self):
        w = self.world
        w.function()
        other = World(self.world.root / "other")
        (self.world.root / "other").mkdir()
        self.five_stores(other)
        w.extra = {"X1": (other.files, other.nodes)}
        self.assertEqual(w.prepare()[0], 0)
        meta = json.loads((w.root / "adm" / "prepared.json").read_text())
        self.assertEqual({c: f["ok"] for c, f in meta["flood"].items()}, {"P1": True, "X1": False})
        self.assertEqual(meta["frame_by_corpus"], {"P1": 1})  # A still counts the frame corpora only
        self.assertEqual({r["corpus"] for r in ad.jsonl(w.root / "adm" / "candidates.jsonl")}, {"P1"})
        with mock.patch.object(ad, "GATE", 1):
            code, out, _ = w.report()
        self.assertEqual(self.a_of(out), 1, out)
        self.assertIn("flood X1: predicted_edges 5 <= 4: FAIL", out)
        self.assertIn("I2: BLOCKED", out)
        self.assertEqual(code, 4)

    def prepared_population(self, count=130):
        w = self.world
        for i in range(count):
            w.function(f"m{i}.py")
        self.assertEqual(w.prepare()[0], 0)
        return w

    def blocked_by(self, w, receipt, needle):
        with mock.patch.object(ad, "GATE", 1):
            if receipt is None:
                code, out, _ = w.report(precision=None)
            elif isinstance(receipt, str):
                code, out, _ = w.report(precision=receipt)
            else:
                code, out, _ = w.report(precision=w.precision(**receipt))
        self.assertEqual(code, 4, out)
        self.assertIn("I2: BLOCKED", out)
        self.assertIn(needle, out.split("I2: BLOCKED")[1])

    def test_i2_needs_a_passing_prospective_precision_receipt(self):
        w = self.prepared_population()
        first_k = lambda k: lambda key, role: "invalid" if key in k else "valid"
        draw = lambda: json.loads((w.root / "draw.json").read_text())
        w.precision()
        core = sorted(draw()["core"])
        cases = {
            "absent": (None, "no precision receipt"),
            "missing file": (str(w.root / "nope.json"), "precision"),
            "other digest": (dict(prepared_sha256="0" * 64), "prepared"),
            "wilson below .85": (dict(verdict=first_k(set(core[:16]))), "Wilson"),
            "stale in the draw": (dict(verdict=lambda k, r: "stale" if k == core[0] else "valid"), "stale"),
            "unknown in the draw": (dict(verdict=lambda k, r: "unknown" if k == core[0] else "valid"), "unknown"),
            "role short": (dict(witnessed=lambda k: k not in set(core[:11])), "role return"),
            "valid but only 90 of 100 witnessed": (dict(witnessed=lambda k: k not in set(core[:10])), "Wilson"),
            "role counts forged": (dict(roles={"return": dict(n=3)}), "recomputation"),
        }
        for name, (receipt, needle) in cases.items():
            with self.subTest(name):
                self.blocked_by(w, receipt, needle)
        with self.subTest("passes"), mock.patch.object(ad, "GATE", 1):
            for good in (dict(), dict(verdict=first_k(set(core[:6]))), dict(witnessed=lambda k: k not in set(core[:5]))):
                code, out, _ = w.report(precision=w.precision(**good))
                self.assertEqual((code, "I2: ADMITTED" in out), (0, True), out)
        with self.subTest("exhaustive family needs every judgment valid"):
            small = World(w.root / "small")
            (w.root / "small").mkdir()
            for i in range(5):
                small.function(f"m{i}.py")
            self.assertEqual(small.prepare()[0], 0)
            self.blocked_by(small, dict(verdict=lambda k, r: "invalid" if k.endswith("return") and "m0" in k else "valid"),
                            "every one must be")

    def test_a_precision_receipt_is_bound_to_the_derived_draw_and_its_files(self):
        w = self.prepared_population()
        w.precision()
        meta = json.loads((w.root / "adm" / "prepared.json").read_text())
        good = json.loads((w.root / "precision.json").read_text())
        self.assertEqual((good["schema"], good["prepared_sha256"], good["n"]),
                         ("admission_precision/4", meta["prepared_sha256"], 100))
        cases = {
            "other frame": (dict(frame_sha256="0" * 64), "frame"),
            "no frame": (dict(frame_sha256=None), "frame"),
            "old schema": (dict(schema="admission_precision/3"), "schema mismatch"),
            "judgments hash": (dict(judgments_sha256="0" * 64), "judgments_sha256"),
            "draw hash": (dict(draw_sha256="0" * 64), "draw_sha256"),
            "a smaller forged sample": (dict(n=22, valid=22, population=22, roles={
                "return": dict(n=22, witnessed=22)}), "recomputation"),
            "population forged": (dict(population=99), "recomputation"),
            "witnessed forged": (dict(roles={"return": dict(n=100, witnessed=101)}), "recomputation"),
            "draw file gone": (dict(draw_path=str(w.root / "gone.json")), "cannot be recomputed"),
        }
        for name, (receipt, needle) in cases.items():
            with self.subTest(name):
                self.blocked_by(w, receipt, needle)

    def test_precision_judgments_outside_the_draw_are_refused_by_the_cli(self):
        w = self.prepared_population()
        prep, draw_path = w.root / "adm", w.root / "draw.json"
        self.assertEqual(run_cli(["draw", "--prepared", prep, "--out", draw_path])[0], 0)
        draw = json.loads(draw_path.read_text())
        meta = json.loads((prep / "prepared.json").read_text())
        rows = [dict(key=k, verdict="valid", witnessed=True, prepared_sha256=meta["prepared_sha256"])
                for k in sorted(draw["keys"])[:22]]
        path = w.root / "short.jsonl"
        path.write_text(dump_lines(rows))
        code, _, err = run_cli(["precision", "--prepared", prep, "--draw", draw_path, "--judgments", path,
                                "--out", w.root / "short-precision.json"])
        self.assertEqual(code, 3, err)
        self.assertIn("78 missing", err)
        self.assertFalse((w.root / "short-precision.json").exists())

    def method_call_world(self, root, finished_search):
        """One owner whose single predicted pair has an unknown parent route (`None`: the search budget ran
        out, the shape of a method-call RHS) unless `finished_search`; returns the World."""
        src = b'def f(d, t):\n    d["k"] = t\n    return d\n'
        nodes = [var("a.py", "f", 1, src, "def", b'd["k"]', ("d", "[]")),
                 var("a.py", "f", 1, src, "use", b"return d", ("d",), b"d"),
                 var("a.py", "f", 1, src, "def", b"d, t", ("t",), b"t"),
                 var("a.py", "f", 1, src, "use", b"= t", ("t",), b"t")]
        junk = [dict(nodes[3], start_byte=900 + i, end_byte=901 + i) for i in range(8)]
        w = World(root)
        w.function("a.py", src, nodes=nodes)
        w.edges = [(nodes[2], nodes[3])] + ([] if finished_search else [(nodes[2], j) for j in junk])
        with mock.patch.object(li, "ROUTE_BUDGET", 4 if not finished_search else li.ROUTE_BUDGET):
            self.assertEqual(w.prepare()[0], 0)
        return w

    def test_survivors_hold_every_agreed_pair_and_a_credit_is_the_subset_the_head_scores(self):
        with tempfile.TemporaryDirectory() as tmp:
            unknown = self.method_call_world(tmp, finished_search=False)
            meta = json.loads((unknown.root / "adm" / "prepared.json").read_text())
            surv = ad.jsonl(unknown.root / "adm" / "survivors.jsonl")
            self.assertEqual(ad.jsonl(unknown.root / "adm" / "candidates.jsonl"), [])
            self.assertEqual([(r["role"], r["parent_route"], r["credited"]) for r in surv],
                             [("return", None, False)])
            self.assertEqual(meta["survivors_sha256"],
                             ad.digest((unknown.root / "adm" / "survivors.jsonl").read_bytes()))
        with tempfile.TemporaryDirectory() as tmp:  # control: the finished search credits the same pair
            known = self.method_call_world(tmp, finished_search=True)
            cands = ad.jsonl(known.root / "adm" / "candidates.jsonl")
            surv = ad.jsonl(known.root / "adm" / "survivors.jsonl")
            self.assertEqual([c["key"] for c in cands], [r["key"] for r in surv])
            self.assertEqual([r["credited"] for r in surv], [True])

    def test_report_prints_both_populations_side_by_side_and_selects_neither(self):
        w = self.prepared_population()
        with mock.patch.object(ad, "GATE", 1):
            code, out, _ = w.report(approved=None, all_survivors=None)
        self.assertIn("population a-credit", out)
        self.assertIn("population all-survivors: no judgments supplied", out)
        code, out, _ = w.report(approved=None, all_survivors="good")
        for line in (l for l in out.splitlines() if l.startswith("  population ")):
            self.assertNotIn("no judgments supplied", line)
        a, b = [l for l in out.splitlines() if l.startswith("  population ")]
        for line in (a, b):
            for field in ("count 130", "drawn 100", "valid 100", "invalid 0", "unknown 0", "Wilson95 [0.96",
                          "verdict PASS"):
                self.assertIn(field, line)
        with self.subTest("a failing population reads FAIL with its reason, beside a passing one"):
            bad = w.precision(verdict=lambda k, r: "unknown" if "m1" in k else "valid", which="all-survivors")
            _, out, _ = w.report(approved=None, all_survivors=str(bad))
            lines = {l.split()[1].rstrip(":"): l for l in out.splitlines() if l.startswith("  population ")}
            self.assertIn("verdict PASS", lines["a-credit"])
            self.assertIn("verdict FAIL", lines["all-survivors"])
            self.assertIn("unknown", lines["all-survivors"])

    def test_i2_is_blocked_unless_an_approved_population_is_named(self):
        w = self.prepared_population()
        good, good_all = w.precision(), w.precision(which="all-survivors")
        with mock.patch.object(ad, "GATE", 1):
            code, out, _ = w.report(precision=good, approved=None, all_survivors=good_all)
            self.assertEqual(code, 4, out)
            self.assertIn("I2: BLOCKED (precision population not approved)", out)
            self.assertNotIn("ADMITTED", out.replace("NOT ADMITTED", ""))
            for approved, others in (("a-credit", dict(all_survivors=None)), ("all-survivors", dict(all_survivors=good_all)),
                                     ("all-survivors", dict(all_survivors=good_all))):
                code, out, _ = w.report(precision=good, approved=approved, **others)
                self.assertEqual((code, "I2: ADMITTED" in out), (0, True), (approved, out))

    def test_an_approved_population_that_fails_or_has_no_receipt_blocks_i2(self):
        w = self.prepared_population()
        good = w.precision()
        bad_all = w.precision(verdict=lambda k, r: "invalid" if "m1" in k else "valid", which="all-survivors")
        with mock.patch.object(ad, "GATE", 1):
            code, out, _ = w.report(precision=good, approved="all-survivors", all_survivors=str(bad_all))
            self.assertEqual(code, 4, out)
            self.assertIn("I2: BLOCKED by", out)
            self.assertIn("Wilson", out.split("I2: BLOCKED")[1])
            code, out, _ = w.report(precision=good, approved="all-survivors", all_survivors=None)
            self.assertEqual(code, 4, out)
            self.assertIn("no precision receipt for the approved population all-survivors",
                          out.split("I2: BLOCKED")[1])
            # control: the other population's failing receipt does not matter once it is not the approved one
            code, out, _ = w.report(precision=good, approved="a-credit", all_survivors=str(bad_all))
            self.assertEqual((code, "I2: ADMITTED" in out), (0, True), out)

    def test_report_refuses_a_survivors_file_that_is_not_the_recorded_one(self):
        w = self.prepared_population()
        good = w.precision()
        with mock.patch.object(ad, "GATE", 1):
            code, out, _ = w.report(precision=good, approved="a-credit")  # control: the recorded file
            self.assertEqual((code, "I2: ADMITTED" in out), (0, True), out)
            surv = w.root / "adm" / "survivors.jsonl"
            original = surv.read_bytes()
            lines = original.decode().splitlines()
            surv.write_text("\n".join(lines[:-1]) + "\n")  # substitute a different (shorter) population
            for approved in ("a-credit", None):
                code, out, err = w.report(precision=good, approved=approved)
                self.assertNotEqual(code, 0, (approved, out))
                self.assertNotIn("I2: ADMITTED", out)
                self.assertIn("survivors.jsonl does not match prepared.json", out + err)

    def test_report_parses_the_bytes_it_hashed(self):
        w = self.prepared_population()
        good = w.precision()
        surv = w.root / "adm" / "survivors.jsonl"
        original = surv.read_bytes()
        seen, real_line, real_digest = [], ad.population_line, ad.digest

        def swap_after_hash(data):
            out = real_digest(data)
            if data == original:  # a writer replaces the file right after its hash was taken
                surv.write_text("\n".join(original.decode().splitlines()[:-1]) + "\n")
            return out

        def spy(name, rows, *rest):
            seen.append((name, len(rows)))
            return real_line(name, rows, *rest)

        with mock.patch.object(ad, "GATE", 1), mock.patch.object(ad, "digest", swap_after_hash), \
                mock.patch.object(ad, "population_line", spy):
            w.report(precision=good, approved="a-credit")
        expected = len(original.decode().splitlines())
        self.assertIn(("all-survivors", expected), seen)

    def test_other_gates_still_block_an_approved_population(self):
        w = self.prepared_population()
        good = w.precision()
        code, out, _ = w.report(precision=good, approved="a-credit")  # control at the real gate (A=130 >= 100)
        self.assertEqual((code, "I2: ADMITTED" in out), (0, True), out)
        code, out, _ = w.report(witnesses=[], precision=good, approved="a-credit")
        self.assertEqual((code, "I2: ADMITTED" in out), (1, False), out)
        meta_path = w.root / "adm" / "prepared.json"
        meta = json.loads(meta_path.read_text())
        meta["residual_owners"] = 1
        meta_path.write_text(json.dumps(meta))
        code, out, _ = w.report(precision=good, approved="a-credit")
        self.assertEqual(code, 4, out)
        self.assertIn("I2: BLOCKED by unreconciled predictions", out)
        self.assertNotIn("I2: ADMITTED", out)
        with self.subTest("unapproved: the approval line comes first and the other block is named after it"):
            code, out, _ = w.report(precision=good, approved=None)
            self.assertEqual(code, 4, out)
            self.assertIn("I2: BLOCKED (precision population not approved)\n  also blocked by unreconciled predictions",
                          out)

    def test_a_receipt_for_the_other_population_or_a_stale_schema_or_a_lost_file_scores_nothing(self):
        w = self.prepared_population()
        good, good_all = w.precision(), w.precision(which="all-survivors")
        with mock.patch.object(ad, "GATE", 1):
            with self.subTest("a-credit receipt passed as the all-survivors one"):
                code, out, _ = w.report(precision=good, approved="all-survivors", all_survivors=str(good))
                self.assertEqual(code, 4, out)
                self.assertIn("is for population a-credit, not all-survivors", out)
            with self.subTest("an unreadable receipt"):
                code, out, _ = w.report(precision=str(w.root / "nope.json"), approved="a-credit")
                self.assertEqual(code, 4, out)
                self.assertIn("population a-credit: precision receipt unreadable", out)
            with self.subTest("a /3 receipt"):
                old = w.precision(schema="admission_precision/3")
                code, out, _ = w.report(precision=old, approved="a-credit")
                self.assertEqual(code, 4, out)
                self.assertIn("population a-credit: receipt not scored; verdict FAIL (precision receipt schema mismatch",
                              out)
            with self.subTest("survivors.jsonl lost after the receipt was made"):
                (w.root / "adm" / "survivors.jsonl").rename(w.root / "survivors.moved")
                code, out, _ = w.report(precision=good, approved="all-survivors", all_survivors=str(good_all))
                self.assertEqual(code, 4, out)
                self.assertIn("has no survivors.jsonl", out)
                (w.root / "survivors.moved").rename(w.root / "adm" / "survivors.jsonl")
            with self.subTest("control: every receipt in place"):
                code, out, _ = w.report(precision=good, approved="all-survivors", all_survivors=str(good_all))
                self.assertEqual((code, "I2: ADMITTED" in out), (0, True), out)

    def test_approved_population_flag_accepts_only_the_two_populations(self):
        w = self.prepared_population()
        with self.assertRaises(SystemExit):
            with contextlib.redirect_stderr(io.StringIO()):
                w.report(precision=w.precision(), approved="everything")

    def test_i2_needs_flood_evidence_for_every_declared_full_loader_corpus(self):
        w = self.world
        w.function()
        other = World(self.world.root / "other")
        (self.world.root / "other").mkdir()
        self.five_stores(other)
        w.extra = {"X1": (other.files, other.nodes),
                   "Z0": ({"b.py": b"def h():\n    return 1\n"}, [])}  # a corpus with no Python owner
        self.assertEqual(w.prepare()[0], 0)
        meta = json.loads((w.root / "adm" / "prepared.json").read_text())
        self.assertEqual(meta["full_loader_corpora"], ["P1", "X1", "Z0"])
        self.assertEqual(meta["full_loader_missing"], [])
        self.assertEqual(meta["flood"]["Z0"], dict(predicted_edges=0, bound=0, ok=True, reads=0, owners=0))
        # the default declaration names six corpora; the frame's P1 is one, the rest are absent from the stream
        code, _, err = run_cli(w.argv("adm-default", corpora=None))
        self.assertEqual(code, 0, err)
        meta = json.loads((w.root / "adm-default" / "prepared.json").read_text())
        self.assertEqual(meta["full_loader_corpora"], sorted(ad.FULL_LOADER_CORPORA))
        self.assertEqual(meta["full_loader_missing"], ["G1", "G2", "P2", "R", "X"])
        self.assertTrue(meta["valid"])

    def test_a_missing_full_loader_corpus_blocks_i2_but_leaves_the_a_decision(self):
        w = self.world
        w.function()
        w.build()
        self.assertEqual(run_cli(w.argv(corpora=None))[0], 0)
        with mock.patch.object(ad, "GATE", 1):
            code, out, _ = w.report()
        self.assertIn("GATE A >= 1 of 1: PASS (A=1)", out)
        self.assertEqual(code, 4, out)
        blocked = out.split("I2: BLOCKED")[1]
        for name in ("G1", "G2", "P2", "R", "X"):
            self.assertIn(name, blocked)
        self.assertIn("full-loader", blocked)
        self.assertNotIn("ADMITTED", out)

    def test_a_prepared_receipt_that_declares_no_corpus_set_blocks_i2(self):
        w = self.world
        w.function()
        self.assertEqual(w.prepare()[0], 0)
        path = w.root / "adm" / "prepared.json"
        meta = json.loads(path.read_text())
        for field in ("full_loader_corpora", "full_loader_missing"):
            del meta[field]
        path.write_text(json.dumps(meta))
        with mock.patch.object(ad, "GATE", 1):
            code, out, _ = w.report(precision=None)
        self.assertEqual(code, 4, out)
        self.assertIn("declares no full-loader corpus set", out)

    def test_the_production_entry_point_requires_the_six_corpora_and_a_declared_set(self):
        w = self.world
        w.function()
        w.build()
        six = ",".join(ad.FULL_LOADER_CORPORA)
        self.assertEqual(sorted(ad.FULL_LOADER_CORPORA), sorted("P1 P2 X G1 G2 R".split()))
        # a narrowed declaration is refused under a frozen frame, accepted only without one (tests)
        code, _, err = run_cli(w.argv("adm-narrow", corpora="P1,P2"), frozen=w.frozen_of())
        self.assertEqual(code, 3, err)
        self.assertIn("full-loader", err)
        self.assertEqual(run_cli(w.argv("adm-six", corpora=six), frozen=w.frozen_of())[0], 0)
        self.assertEqual(run_cli(w.argv("adm-extra", corpora=six + ",Q"), frozen=w.frozen_of())[0], 0)
        for bad in (",", " , ", "P1,,P2"):
            with self.subTest(bad):
                code, _, err = run_cli(w.argv("adm-bad", corpora=bad))
                self.assertEqual(code, 3, err)

    def test_duplicate_pair_keys_are_residuals_and_invalidate_the_run(self):
        def dup_product(records):
            o = next(r for r in records if r["kind"] == "owner")
            o["pairs"].append(dict(o["pairs"][0]))
            records.append(dict(next(r for r in records if r["kind"] == "prediction")))

        def dup_independent(rows):
            rows.insert(-1, dict(next(r for r in rows if r["kind"] == "prediction")))
            rows[-1]["predictions"] += 1

        for name, hooks in (("product only", dict(mutate_trace=dup_product)),
                            ("independent only", dict(mutate_ind=dup_independent)),
                            ("both sides", dict(mutate_trace=dup_product, mutate_ind=dup_independent))):
            with tempfile.TemporaryDirectory() as tmp:
                w = World(tmp)
                w.function()
                code, _, err = w.prepare(**hooks)
                meta = json.loads(Path(tmp, "adm", "prepared.json").read_text())
                self.assertEqual(code, 1, name)
                self.assertFalse(meta["valid"], name)
                self.assertTrue(any("duplicate pair key" in p for p in meta["problems"]), (name, meta["problems"]))
                if name != "both sides":  # unequal multiplicity is a pair residual, not erased by a set
                    residual, = ad.jsonl(Path(tmp, "adm", "residuals.jsonl"))
                    self.assertEqual(residual["residual"][0]["side"],
                                     "product_only" if name == "product only" else "independent_only")

    def test_production_prepare_requires_the_frozen_frame_and_census_inputs(self):
        w = self.world
        w.function()
        w.build()
        # the CLI's own default: the self-consistent test frame is a substitute and is refused
        code, _, err = run_cli(w.argv("adm-prod"), frozen=ad.FROZEN)
        self.assertEqual(code, 3, err)
        self.assertIn("frozen", err)
        self.assertFalse((w.root / "adm-prod" / "prepared.json").exists())
        out, errs = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(errs):
            self.assertEqual(ad.main([str(a) for a in w.argv("adm-default")]), 3)
        self.assertEqual(ad.FROZEN["sha"], "6c1c0998f4f1afa06b95b2dad84f220e71c784d3d1b8ec82e6606029bb05b818")
        self.assertEqual((ad.FROZEN["owners"], ad.FROZEN["corpora"], ad.FROZEN["rows"]),
                         (787, dict(P1=517, P2=270), 1137))
        self.assertEqual(set(ad.FROZEN["census"]), {"P1", "P2"})
        # an injected frame (test-only parameter) that matches the inputs is accepted ...
        self.assertEqual(run_cli(w.argv("adm-inj", corpora=",".join(ad.FULL_LOADER_CORPORA)), frozen=w.frozen_of())[0], 0)
        # ... and each pinned property refuses on its own
        for name, over in (("sha", dict(sha="0" * 64)), ("owners", dict(owners=2)),
                           ("corpora", dict(corpora=dict(P1=2))), ("rows", dict(rows=9)),
                           ("census", dict(census=dict(P1="0" * 64)))):
            with self.subTest(name):
                code, _, err = run_cli(w.argv("adm-" + name), frozen=dict(w.frozen_of(), **over))
                self.assertEqual(code, 3, err)
                self.assertIn("frozen", err)

    def test_wilson_lower_matches_the_recorded_m_bounds(self):
        self.assertAlmostEqual(ad.wilson_lower(58, 60), 0.8864, places=4)
        self.assertAlmostEqual(ad.wilson_lower(56, 60), 0.8407, places=4)
        self.assertGreaterEqual(int(787 * ad.wilson_lower(13, 60)), 100)


if __name__ == "__main__":
    unittest.main()
