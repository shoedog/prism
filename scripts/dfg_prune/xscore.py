#!/usr/bin/env python3
"""L4 design scoring for #829 (TEST-FORM section 4, L4 / option E2). Python 3.9, stdlib only.

  xscore.py run [--pops l1,l2,l3] [--corpora redis,...] [--out DIR] [--candidates FILE.py]
                [--l2dirs DIR ...] [--l1-export DIR] [--l3-dir DIR] [--only NAME,...]
      scores every candidate (an owner-refusal predicate, `xscore_candidates.py`) without running
      prism: an owner the predicate REFUSES gets the Tb baseline arm's pairs (nothing dropped), an
      owner it ADMITS keeps the T arm's pairs. The oracle verdicts and the dropped pairs come
      from saved runs; the predicate sees only the owner's tokens, the file-scope context and the
      macro definitions of the census inputs, never the compiler's expansion.
  xscore.py report DIR          markdown tables of a `run`
  xscore.py selftest            lexer, fact automaton, locator and candidate unit checks

Populations: L1 (the frozen oracle rows: pair rows, inline rows, generator rows), L2 (the bound-3
generator's judged programs, `.scratch829/gen/k3`), L3 (the six corpora of the saved base sweep,
`.scratch829/l3/<corpus>/base`; the sources are re-read from the corpus checkouts).

Classes of a pair the head (T arm) drops:
  MUSTKEEP     the oracle says CONTRADICTED (or the row asserts `kept`): dropping it is a violation
  MUSTKEEP831  the same, for a generator program whose macro body has a top-level `;` (issue #831,
               not this lane's gate)
  PINNED       a pinned hole row (A-inputs and the like): today's drop is asserted, a design may close it
  BOUNDARY     a guide row whose truth depends on the declared configuration (A-ext, A-config):
               reported, never a violation
  CONFIRMED    the oracle says the drop is sound: refusing it is recall cost
  UNJUDGED     no truth basis (XOFF, UNJUDGED): refusing it is reported apart from recall
See XARM.md, "L4 scoring", for what the emulation cannot model.
"""
import argparse
import collections
import glob
import gzip
import importlib.util
import json
import os
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import xarm_core as core  # noqa: E402
import xscore_lex as L  # noqa: E402

ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
FIX = os.path.join(ROOT, "tests", "fixtures", "dfg_prune", "oracle")
SCR = os.path.join(ROOT, ".scratch829")
CLASSES = ["MUSTKEEP", "MUSTKEEP831", "PINNED", "BOUNDARY", "CONFIRMED", "UNJUDGED"]
VIOL_CLASSES = ("MUSTKEEP", "MUSTKEEP831", "PINNED", "BOUNDARY")
MAX_GROUPS = 400


def load_candidates(path):
    if not path:
        import xscore_candidates as m
        return m.CANDIDATES
    spec = importlib.util.spec_from_file_location("xscore_user_candidates", path)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m.CANDIDATES


class Scorer:
    def __init__(self, cands):
        self.cands = cands
        self.pops = collections.defaultdict(lambda: {c: {k: {"n": 0, "refused": 0,
                                                             "reasons": collections.Counter()}
                                                         for k in CLASSES} for c in cands})
        self.viol = collections.defaultdict(lambda: {c: collections.defaultdict(dict) for c in cands})
        self.extra = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))

    def decide(self, owner, db):
        """{candidate: refusal reason or None}"""
        return {c: fn(owner, db) for c, (fn, _) in self.cands.items()}

    def record(self, pop, cls, group, label, dec, example=None):
        for c, reason in dec.items():
            cell = self.pops[pop][c][cls]
            cell["n"] += 1
            if reason:
                cell["refused"] += 1
                cell["reasons"][reason.split(":")[0]] += 1
            elif cls in VIOL_CLASSES:
                g = self.viol[pop][c][cls]
                if group in g:
                    g[group]["n"] += 1
                elif len(g) < MAX_GROUPS:
                    g[group] = {"n": 1, "label": label, "example": example}

    def dump(self):
        return {"pops": {p: {c: {k: {"n": v["n"], "refused": v["refused"],
                                     "reasons": dict(v["reasons"])} for k, v in d.items()}
                             for c, d in cs.items()} for p, cs in self.pops.items()},
                "viol": {p: {c: {k: v for k, v in d.items()} for c, d in cs.items()}
                         for p, cs in self.viol.items()},
                "extra": {p: {k: dict(v) for k, v in d.items()} for p, d in self.extra.items()},
                "candidates": {c: d for c, (_, d) in self.cands.items()}}


def census_db(files, links=None):
    """MacroDb over the emulated census inputs of a fixture ({rel: bytes})."""
    files = dict(files)
    for rel, target in (links or {}).items():
        t = os.path.normpath(os.path.join(os.path.dirname(rel), target))
        if t in files and rel not in files:
            files[rel] = files[t]
    inputs = core.census_inputs(files)
    return L.MacroDb((rel, L.decode(files[rel])) for rel in sorted(inputs)), files


# ---------------------------------------------------------------- L1
def l1_class(r):
    """-> class of a frozen row, or None when head already keeps the pair."""
    if not r["head_drops"]:
        return None
    a, o = r["assert"], r["oracle"]
    if a == "kept":
        return "MUSTKEEP"
    if a == "pinned_hole":
        return "PINNED"
    if r.get("boundary") and o == "CONTRADICTED":
        return "BOUNDARY"
    if o == "CONFIRMED":
        return "CONFIRMED"
    return "UNJUDGED"


def run_l1(sc, export, only=None):
    unlocated = []
    # pair rows
    pr = [json.loads(l) for l in open(os.path.join(FIX, "pair_rows.jsonl"))]
    for r in pr:
        cls = l1_class(r)
        if cls is None:
            sc.extra["L1-pair"]["head_keeps"]["rows"] += 1
            continue
        d = os.path.join(export, "rows", r["key"])
        row = json.load(open(os.path.join(d, "row.json")))
        files = {}
        for dp, _, fs in os.walk(os.path.join(d, "f")):
            for f in fs:
                p = os.path.join(dp, f)
                files[os.path.relpath(p, os.path.join(d, "f"))] = open(p, "rb").read()
        db, files = census_db(files, row.get("links"))
        fx = L.FileIndex(L.decode(files[r["target"]]))
        fn = fx.owner_at(min(r["from"], r["to"]))
        if fn is None:
            unlocated.append(("pair", r["key"], r["test"]))
            sc.extra["L1-pair"]["unlocated"][cls] += 1
            continue
        dec = sc.decide(L.Owner(fx, fn, r["target"]), db)
        test = r["test"].split("::")[-1]
        sc.record("L1-pair", cls, r["key"], "%s [%s] %s" % (test, r["key"], r["what"]), dec)
    # inline rows
    ir = [json.loads(l) for l in open(os.path.join(FIX, "inline_rows.jsonl"))]
    texts = {}
    for r in ir:
        cls = l1_class(r)
        if cls is None:
            sc.extra["L1-inline"]["head_keeps"]["rows"] += 1
            continue
        name = r["case"] + ("" if r["lang"] == "cpp" else ".c")
        p = os.path.join(export, "inline", r["lang"], name)
        if p not in texts:
            data = open(p, "rb").read()
            db, _ = census_db({name: data})
            texts[p] = (db, L.FileIndex(L.decode(data)))
        db, fx = texts[p]
        fn = fx.owner_at(min(r["def_line"], r["use_line"]))
        if fn is None:
            unlocated.append(("inline", r["case"], r["id"]))
            sc.extra["L1-inline"]["unlocated"][cls] += 1
            continue
        dec = sc.decide(L.Owner(fx, fn, name), db)
        sc.record("L1-inline", cls, r["case"] + ":" + r["id"], "%s:%s (%s)" % (r["case"], r["id"], r["owner"]), dec)
    # generator rows
    gr = [json.loads(l) for l in open(os.path.join(FIX, "generated_rows.jsonl"))]
    import xgen
    for r in gr:
        cls = l1_class(r)
        if cls is None:
            continue
        files = {k: v.encode("latin-1", "replace") if isinstance(v, str) else v
                 for k, v in r["files"].items()}
        db, files = census_db(files)
        fx = L.FileIndex(L.decode(files[r["target"]]))
        fn = fx.owner_at(min(r["from"], r["to"]))
        if fn is None:
            unlocated.append(("generated", r["key"]))
            sc.extra["L1-gen"]["unlocated"][cls] += 1
            continue
        dec = sc.decide(L.Owner(fx, fn, r["target"]), db)
        sc.record("L1-gen", cls, r["key"], r["key"], dec, example=r["files"][r["target"]])
    return unlocated


# ---------------------------------------------------------------- L2
def run_l2(sc, dirs, limit=None):
    import xgen
    n = 0
    t0 = time.time()
    for d in dirs:
        pop = "L2"
        for line in open(os.path.join(d, "results.jsonl")):
            r = json.loads(line)
            v = r["verdict"]
            if v in ("NODROP", "INCOMPLETE"):
                continue
            spec = r["spec"]
            rd = xgen.Prog(spec).render(None)
            if rd is None:
                continue
            files = {k: x.encode("latin-1", "replace") for k, x in rd["files"].items()}
            db, files = census_db(files)
            fx = L.FileIndex(L.decode(files[rd["target"]]))
            fn = fx.owner_at(min(rd["from"], rd["to"]))
            eff = rd["effects"]  # the current classification (enclosing macros: no SEMI0)
            key = xgen.key_of(spec, eff)
            variant = xgen.vkey(spec)
            if fn is None:
                sc.extra[pop]["unlocated"][v] += 1
                continue
            if v == "CONTRADICTED":
                cls = "MUSTKEEP831" if "SEMI0" in eff else "MUSTKEEP"
            elif v == "CONFIRMED":
                cls = "CONFIRMED"
            else:
                cls = "UNJUDGED"
            dec = sc.decide(L.Owner(fx, fn, rd["target"]), db)
            sc.record(pop, cls, key, key, dec, example=rd["files"][rd["target"]])
            # the same program counted by lexical variant / channel for the audit of the emulation
            sc.extra[pop]["variant:" + variant][cls] += 1
            for c, reason in dec.items():
                if reason and cls in ("MUSTKEEP", "MUSTKEEP831"):
                    sc.extra[pop]["caught_variant:%s:%s" % (c, variant)][cls] += 1
            n += 1
            if limit and n >= limit:
                return n
    sc.extra["L2"]["seconds"]["total"] = int(time.time() - t0)
    return n


# ---------------------------------------------------------------- L3
def load_run(path):
    with gzip.open(path, "rt") as f:
        return json.load(f)


def run_l3(sc, l3dir, names):
    import xarm_l3
    cfgs = xarm_l3.corpora()
    for name in names:
        t0 = time.time()
        cfg = cfgs[name]
        root = xarm_l3.expand_path(cfg["root"])
        files = xarm_l3.collect(root)
        mains = set(xarm_l3.mains_of(files, cfg["scope"]))
        db = L.MacroDb((rel, L.decode(b)) for rel, b in sorted(files.items()))
        base = os.path.join(l3dir, name, "base")
        rows = collections.defaultdict(list)
        for line in open(os.path.join(base, "rows.tsv")):
            p = line.rstrip("\n").split("\t")
            v, reason, rule, f, path, a, b, macros = p[:8]
            if v == "OUT_OF_SCOPE":
                continue
            rows[f].append((v, reason, rule, path, int(a), int(b), macros))
        towners = collections.defaultdict(list)
        for f, s, st in load_run(os.path.join(base, "T.json.gz"))["owners"]:
            if f in mains and st.startswith("on"):
                towners[f].append(s)
        pop = "L3-" + name
        ex = sc.extra[pop]
        for f in sorted(set(rows) | set(towners)):
            if f not in files:
                continue
            fx = L.FileIndex(L.decode(files[f]))
            memo = {}

            def dec_for(fn):
                if fn.open not in memo:
                    memo[fn.open] = sc.decide(L.Owner(fx, fn, f), db)
                return memo[fn.open]

            for v, reason, rule, path, a, b, macros in rows.get(f, ()):
                fn = fx.owner_at(min(a, b))
                if fn is None:
                    ex["unlocated"][v] += 1
                    continue
                dec = dec_for(fn)
                if v == "CONTRADICTED":
                    cls = "MUSTKEEP"
                elif v == "CONFIRMED":
                    cls = "CONFIRMED"
                else:
                    cls = "UNJUDGED"
                label = "%s:%d-%d %s [%s]" % (f, a, b, path, reason.split(";")[-1].strip() or v)
                sc.record(pop, cls, reason if v == "CONTRADICTED" else label, label, dec)
                ex["pairs_" + cls]["n"] += 1
            # owners: every prism-on owner of a scoped main, matched to a located function
            seen = set()
            for s in towners.get(f, ()):
                fn = fx.owner_for_start(s)
                if fn is None:
                    ex["owners"]["unmatched"] += 1
                    continue
                if fn.open in seen:
                    continue
                seen.add(fn.open)
                ex["owners"]["on"] += 1
                for c, reason in dec_for(fn).items():
                    if reason:
                        ex["owners_refused:" + c][reason.split(":")[0]] += 1
                        ex["owners_refused:" + c]["total"] += 1
            # owners with at least one dropped pair
            dropfn = {}
            for v, reason, rule, path, a, b, macros in rows.get(f, ()):
                fn = fx.owner_at(min(a, b))
                if fn is not None:
                    dropfn[fn.open] = fn
            for o, fn in dropfn.items():
                ex["owners_with_drops"]["n"] += 1
                for c, reason in dec_for(fn).items():
                    if reason:
                        ex["owners_with_drops_refused:" + c]["total"] += 1
            del fx
        ex["seconds"]["total"] = int(time.time() - t0)
        print("L3", name, "done in", int(time.time() - t0), "s", flush=True)


# ---------------------------------------------------------------- report
def pct(a, b):
    return "%.1f%%" % (100.0 * a / b) if b else "-"


def cell(res, pop, c, k):
    return res["pops"][pop][c][k]


def md_table(rows, header):
    out = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    for r in rows:
        out.append("| " + " | ".join(str(x) for x in r) + " |")
    return "\n".join(out)


def report(res):
    cands = list(res["candidates"])
    pops = list(res["pops"])
    out = []
    for pop in pops:
        out.append("\n#### %s\n" % pop)
        rows = []
        for c in cands:
            cc = res["pops"][pop][c]
            viol = sum(cc[k]["n"] - cc[k]["refused"] for k in ("MUSTKEEP",))
            v831 = cc["MUSTKEEP831"]["n"] - cc["MUSTKEEP831"]["refused"]
            pin = cc["PINNED"]["n"] - cc["PINNED"]["refused"]
            bnd = cc["BOUNDARY"]["n"] - cc["BOUNDARY"]["refused"]
            conf = cc["CONFIRMED"]
            unj = cc["UNJUDGED"]
            rows.append([c, "%d / %d" % (viol, cc["MUSTKEEP"]["n"]),
                         "%d / %d" % (v831, cc["MUSTKEEP831"]["n"]),
                         "%d / %d" % (pin, cc["PINNED"]["n"]), "%d / %d" % (bnd, cc["BOUNDARY"]["n"]),
                         "%d / %d (%s)" % (conf["refused"], conf["n"], pct(conf["refused"], conf["n"])),
                         "%d / %d" % (unj["refused"], unj["n"])])
        out.append(md_table(rows, ["candidate", "must-keep dropped / total", "#831 must-keep dropped / total",
                                   "pinned open / total", "boundary open / total",
                                   "CONFIRMED lost / total", "UNJUDGED lost / total"]))
    return "\n".join(out)


CORP = ["snappy", "leveldb", "re2", "redis", "cpython", "protobuf"]


def sumcells(res, pops, c, k):
    n = r = 0
    for p in pops:
        if p in res["pops"]:
            cc = res["pops"][p][c][k]
            n += cc["n"]
            r += cc["refused"]
    return n, r


def tables(res):
    """The tables of L4-SCORES.md (markdown)."""
    cands = list(res["candidates"])
    out = []
    l1 = ["L1-pair", "L1-inline", "L1-gen"]
    l3 = ["L3-" + c for c in CORP]
    out.append("### Summary (must-keep pairs the design still drops, out of the pairs head drops; "
               "CONFIRMED drops lost)\n")
    rows = []
    for c in cands:
        def mk(pops, k="MUSTKEEP"):
            n, r = sumcells(res, pops, c, k)
            return "%d / %d" % (n - r, n)
        cn, cr = sumcells(res, l3, c, "CONFIRMED")
        ln, lr = sumcells(res, ["L2"], c, "CONFIRMED")
        pn, pr = sumcells(res, l1, c, "PINNED")
        bn, br = sumcells(res, l1, c, "BOUNDARY")
        rows.append([c, mk(["L1-pair"]), mk(["L1-inline"]), mk(["L1-gen"]), mk(["L2"]), mk(l3),
                     "%d / %d" % (pn - pr, pn), "%d / %d" % (bn - br, bn),
                     "%s (%d / %d)" % (pct(cr, cn), cr, cn)])
    out.append(md_table(rows, ["candidate", "L1 pair rows", "L1 inline rows", "L1 generator rows",
                               "L2 bound 3", "L3 corpora (6)", "pinned holes open (L1)",
                               "boundary rows open (L1)", "L3 CONFIRMED lost"]))
    out.append("\n### L3 recall cost per corpus: CONFIRMED drops lost / total (UNJUDGED lost / total)\n")
    rows = []
    for c in cands:
        row = [c]
        for k in CORP:
            p = "L3-" + k
            if p not in res["pops"]:
                row.append("-")
                continue
            a = res["pops"][p][c]["CONFIRMED"]
            u = res["pops"][p][c]["UNJUDGED"]
            row.append("%d / %d (%s) (%d / %d)" % (a["refused"], a["n"], pct(a["refused"], a["n"]),
                                                  u["refused"], u["n"]))
        rows.append(row)
    out.append(md_table(rows, ["candidate"] + CORP))
    out.append("\n### L3 owners refused / owners `on` on the head (pairs aside; owners with a dropped pair in brackets)\n")
    rows = []
    for c in cands:
        row = [c]
        for k in CORP:
            e = res["extra"].get("L3-" + k)
            if not e:
                row.append("-")
                continue
            on = e["owners"].get("on", 0)
            ref = e.get("owners_refused:" + c, {}).get("total", 0)
            wd = e["owners_with_drops"].get("n", 0)
            wr = e.get("owners_with_drops_refused:" + c, {}).get("total", 0)
            row.append("%d / %d (%s) [%d / %d]" % (ref, on, pct(ref, on), wr, wd))
        rows.append(row)
    out.append(md_table(rows, ["candidate"] + CORP))
    out.append("\n### Gate reasons of the refused CONFIRMED drops (L3, six corpora, pairs)\n")
    rows = []
    for c in cands:
        tot = collections.Counter()
        for p in l3:
            if p in res["pops"]:
                tot.update(res["pops"][p][c]["CONFIRMED"]["reasons"])
        rows.append([c, ", ".join("%s %d" % (k, v) for k, v in tot.most_common()) or "-"])
    out.append(md_table(rows, ["candidate", "reason: pairs"]))
    out.append("\n### Gate reasons of the refused CONFIRMED drops per corpus (pairs)\n")
    rows = []
    for c in cands:
        for k in CORP:
            p = "L3-" + k
            if p in res["pops"]:
                rs = res["pops"][p][c]["CONFIRMED"]["reasons"]
                if rs:
                    rows.append([c, k, ", ".join("%s %d" % (a, b) for a, b in
                                                  sorted(rs.items(), key=lambda x: -x[1]))])
    out.append(md_table(rows, ["candidate", "corpus", "reason: pairs"]))
    return "\n".join(out)


def violations(res, pops=None, classes=("MUSTKEEP",), limit=40):
    out = []
    for c in res["candidates"]:
        for p in (pops or res["viol"]):
            for k in classes:
                g = res["viol"].get(p, {}).get(c, {}).get(k, {})
                if g:
                    out.append("%s / %s / %s: %d group(s), %d pair(s)" % (c, p, k, len(g),
                                                                       sum(v["n"] for v in g.values())))
                    for gid, v in list(sorted(g.items()))[:limit]:
                        out.append("    %4d  %s" % (v["n"], v["label"]))
    return "\n".join(out)


def cmd_run(a):
    cands = load_candidates(a.candidates)
    if a.only:
        keep = a.only.split(",")
        cands = collections.OrderedDict((k, v) for k, v in cands.items() if k in keep)
    sc = Scorer(cands)
    os.makedirs(a.out, exist_ok=True)
    pops = a.pops.split(",")
    meta = {}
    if "l1" in pops:
        t = time.time()
        meta["l1_unlocated"] = run_l1(sc, a.l1_export)
        print("L1 done", int(time.time() - t), "s; unlocated", meta["l1_unlocated"], flush=True)
    if "l2" in pops:
        t = time.time()
        meta["l2_programs"] = run_l2(sc, a.l2dirs, a.l2_limit)
        print("L2 done", int(time.time() - t), "s;", meta["l2_programs"], "programs", flush=True)
    if "l3" in pops:
        run_l3(sc, a.l3_dir, a.corpora.split(","))
    res = sc.dump()
    res["meta"] = meta
    with open(os.path.join(a.out, "scores.json"), "w") as f:
        json.dump(res, f, indent=1, sort_keys=True)
    print(report(res))


def cmd_report(a):
    res = json.load(open(os.path.join(a.dir, "scores.json")))
    if a.md:
        print(tables(res))
    elif a.violations:
        print(violations(res, a.pops.split(",") if a.pops else None, tuple(a.classes.split(","))))
    else:
        print(report(res))


def selftest():
    src = ("#define OPEN {\n#define PAIR(a, b) a, b\n#define TIGHT(a) ((a) + 1)\n"
           "#define BOTH(s) OPEN s CLOSE\n#define CLOSE }\n"
           "int f(int c) {\n    int x = 1;\n    OPEN x = 2; CLOSE\n    return TIGHT(x);\n}\n"
           "int g(int c) { return PAIR({1, 2}); }\n"
           "int h(int c) { return c; }\n")
    db = L.MacroDb([("a.c", src)])
    assert db.own("OPEN").unbalanced and db.own("CLOSE").unbalanced
    assert not db.own("TIGHT").unbalanced and not db.own("TIGHT").loose_comma
    assert db.own("PAIR").loose_comma and not db.own("PAIR").unbalanced
    assert db.closed("BOTH").unbalanced and not db.own("BOTH").unbalanced
    fx = L.FileIndex(src)
    names = [fx.code[f.name_idx].text for f in fx.fns]
    assert names == ["f", "g", "h"], names
    import xscore_candidates as C
    of, og, oh = (L.Owner(fx, fn) for fn in fx.fns)
    assert C.cand_tentative(of, db).startswith("unbalanced")
    assert C.cand_tentative(og, db).startswith(("loose_comma", "arg_split"))
    assert C.cand_tentative(oh, db) is None
    assert C.cand_any_macro(oh, db) is None and C.cand_any_macro(of, db)
    assert C.cand_certificate(oh, db) is None
    # digraphs and comments in bodies
    db2 = L.MacroDb([("b.h", "#define B <%\n#define C /* { */ 1 /* } */\n#define D(x) __VA_OPT__(,) x\n")])
    assert db2.own("B").unbalanced and not db2.own("C").unbalanced
    # enclosing group
    enc = "#define M(p) p\nM(static int d;\nint probe(int c) { return c; }\n)\n"
    fx2 = L.FileIndex(enc)
    o2 = L.Owner(fx2, fx2.owner_at(3))
    assert o2.enclosing_open and C.cand_tentative_enclosing(o2, L.MacroDb([("a.c", enc)])) == "enclosing_group"
    assert not C.cand_tentative(o2, L.MacroDb([("a.c", enc)]))
    print("selftest ok")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    pr = sub.add_parser("run")
    pr.add_argument("--pops", default="l1,l2,l3")
    pr.add_argument("--corpora", default="snappy,leveldb,re2,redis,cpython,protobuf")
    pr.add_argument("--out", default=os.path.join(SCR, "l4"))
    pr.add_argument("--candidates")
    pr.add_argument("--only")
    pr.add_argument("--l2dirs", nargs="+", default=[os.path.join(SCR, "gen", "k3")])
    pr.add_argument("--l2-limit", type=int)
    pr.add_argument("--l1-export", default=os.path.join(SCR, "exp2"))
    pr.add_argument("--l3-dir", default=os.path.join(SCR, "l3"))
    pr.set_defaults(fn=cmd_run)
    pp = sub.add_parser("report")
    pp.add_argument("dir")
    pp.add_argument("--md", action="store_true", help="the tables of L4-SCORES.md")
    pp.add_argument("--violations", action="store_true", help="admitted must-keep groups")
    pp.add_argument("--pops")
    pp.add_argument("--classes", default="MUSTKEEP")
    pp.set_defaults(fn=cmd_report)
    st = sub.add_parser("selftest")
    st.set_defaults(fn=lambda a: selftest())
    a = ap.parse_args()
    a.fn(a)


if __name__ == "__main__":
    main()
