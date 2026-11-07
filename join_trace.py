#!/usr/bin/env python3
"""Join every moved edge of a base/head `dfg-stats --edges` pair to its PRISM_RD_CENSUS_TRACE mechanism.

usage: join_trace.py BASE.edges.jsonl[.gz] HEAD.edges.jsonl[.gz] HEAD.edges.err [--sample N] [--list]
Moved edge  = (from, to) identity whose label multiset changed between the arms (the key compare_edges.py uses;
              the dump has no bytes or owner). The moves come from the dumps, never from the trace.
Move kind   = old label -> new label per changed row: exact->cfg_incomplete, sameline->cfg_incomplete,
              nameonly/<doubt>->cfg_incomplete (killed, alias_unstable, call_nameonly), anything else is OTHER.
Trace line  = rd-census<TAB>mechanism<TAB>file<TAB>owner<TAB>from path@line:byte<TAB>to path@line:byte
              joined on (file, from path, from line, to path, to line).
Prints moved keys per mechanism x move kind; every moved edge with NO mechanism, every PROMOTION (a new row
that is exact where the old row was not) and every OTHER move are listed in full and fail the run (exit 1).
The trace is a superset of the moves (it also fires where the label was already cfg_incomplete): trace-only keys
are reported as a count plus a small sample, never as moves.
S1b-1 mechanisms (PRISM_RD_CENSUS_TRACE, a census-free control pass names every edge the census moved):
  kill:<form>            form in update | augmented | delete | pattern | for-head | for-increment | assign | other;
                         explains moves INTO nameonly/killed only
  owner-refuse:escape|parse|target:<kind>|one-node|def-cap   whole owner refused (one-node/def-cap: the completed
                         def list would trip the one-node belt or the 2,048 cap; RD ran on recorded defs); explains moves into nameonly/cfg_incomplete only
  site-unmapped          a base with an eager site that maps to no statement node; cfg_incomplete only
Move kinds accepted: anything -> nameonly/cfg_incomplete or -> nameonly/killed (a move into exact is a PROMOTION and
fails). A mechanism that cannot explain the move kind it is joined to is a MISMATCH and fails. Every site-unmapped
demotion is listed in full ("SITE-UNMAPPED"), every d3-loop/d3-unknown demotion too ("D3-FULL"; d3-* explains exact->sameline only, S1b-2) and the kill:for-increment count is printed (DESIGN 10, 7.3).
S1c mechanism (D2-c): d2c explains exact->cfg_incomplete and sameline->cfg_incomplete only, never a move into exact or
into killed; a killed or already-incomplete label is kept by the solver, so such a move with only d2c is a MISMATCH.
S1a mechanisms (capture-*) stay in the trace but base already contains S1a, so they are trace-only keys here.
"""
import collections, gzip, json, random, sys


def opener(path):
    return gzip.open(path, "rt") if path.endswith(".gz") else open(path)


def pstr(p):
    return p["base"] + "".join("[]" if f == "[]" else "." + f for f in p["fields"])


def ident(j):
    f, t = j["from"], j["to"]
    return (f["file"], pstr(f["path"]), f["line"], pstr(t["path"]), t["line"], t["file"])


def load_edges(path):
    m = collections.defaultdict(list)
    for line in opener(path):
        j = json.loads(line)
        m[ident(j)].append((j["confidence"], j.get("doubt") or "-"))
    return m


def load_trace(path):
    t = collections.defaultdict(set)
    for line in opener(path):
        if not line.startswith("rd-census\t"):
            continue
        f = line.rstrip("\n").split("\t")
        if len(f) != 6:
            continue
        _, mech, file, _owner, frm, to = f
        fp, fl = frm.rsplit("@", 1)[0], int(frm.rsplit("@", 1)[1].split(":")[0])
        tp, tl = to.rsplit("@", 1)[0], int(to.rsplit("@", 1)[1].split(":")[0])
        t[(file, fp, fl, tp, tl, file)].add(mech)
    return t


def kind(old, new):
    """Move kind of one (old row -> new row) pair, and whether it is a promotion."""
    promotion = new[0] == "exact" and old[0] != "exact"
    if new not in (("nameonly", "cfg_incomplete"), ("nameonly", "killed"), ("nameonly", "sameline")) or promotion:
        return "OTHER %s/%s->%s/%s" % (old + new), promotion
    if old == new:
        return "OTHER %s/%s->%s/%s" % (old + new), promotion
    target = new[1]
    if old[0] == "exact":
        return "exact->" + target, False
    if old[1] == "sameline":
        return "sameline->" + target, False
    return "nameonly/%s->%s" % (old[1], target), False


def explains(mech, kd):
    """Whether mechanism `mech` can account for a move of kind `kd`."""
    mech = mech.replace("derived:", "")
    if mech.startswith("kill:"):
        return kd.endswith("->killed")
    if mech.startswith("owner-refuse:") or mech == "site-unmapped":
        return kd.endswith("->cfg_incomplete")
    if mech.startswith("d3-"):
        # S1b-2 (#660 D3): only rewrites labels that are already exact, into NameOnly(SameLine)
        return kd == "exact->sameline"
    if mech == "d2c":
        # S1c (#660 D2-c): a write in a region refuses `exact` and `sameline` edges only
        return kd in ("exact->cfg_incomplete", "sameline->cfg_incomplete")
    return True


def main():
    a = sys.argv[1:]
    base, head, trace = load_edges(a[0]), load_edges(a[1]), load_trace(a[2])
    n = int(a[a.index("--sample") + 1]) if "--sample" in a else 5
    moved = [k for k in base if k in head and sorted(base[k]) != sorted(head[k])]
    table = collections.defaultdict(collections.Counter)  # mechanism -> move kind -> keys
    kinds = collections.Counter()
    untraced, promotions, other, rows = [], [], [], []
    mismatches, unmapped, d3full = [], [], []
    traced_by_to = collections.defaultdict(list)
    for k in moved:
        if trace.get(k):
            traced_by_to[(k[5], k[3], k[4])].append((k, sorted(trace[k])))
    for k in moved:
        old, new = collections.Counter(base[k]), collections.Counter(head[k])
        gone, came = list((old - new).elements()), list((new - old).elements())
        pairs = list(zip(sorted(gone), sorted(came))) or [(gone[0] if gone else ("-", "-"), came[0] if came else ("-", "-"))]
        ks = []
        for o, w in pairs:
            kd, promo = kind(o, w)
            ks.append(kd)
            if promo:
                promotions.append((k, o, w))
            if kd.startswith("OTHER"):
                other.append((k, o, w))
        mechs = sorted(trace.get(k, ()))
        if not mechs:
            # Derived edges (#594 object-literal arguments) carry the label of the callee edge into the same
            # `to` endpoint; attribute them to that edge's mechanism when it moved and was traced.
            src = sorted({m for (tk, ms) in traced_by_to.get((k[5], k[3], k[4]), []) if tk != k for m in ms})
            mechs = ["derived:" + m for m in src]
        if not mechs:
            untraced.append(k)
        elif not all(any(explains(m, kd) for m in mechs) for kd in ks):
            mismatches.append((k, mechs, ks))
        if "site-unmapped" in mechs:
            unmapped.append((k, ks))
        if "d3-loop" in mechs or "d3-unknown" in mechs:
            d3full.append((k, mechs, ks))
        for kd in ks:
            kinds[kd] += 1
            for m in mechs or ["(none)"]:
                table[m][kd] += 1
        rows.append((k, mechs, ks))
    print(f"moved edge keys: {len(moved)}  UNTRACED: {len(untraced)}  PROMOTIONS: {len(promotions)}  OTHER: {len(other)}  MISMATCH: {len(mismatches)}")
    fi = sum(1 for _k, mechs, _ks in rows if "kill:for-increment" in mechs)
    print(f"kill:for-increment moved keys: {fi}   site-unmapped moved keys: {len(unmapped)}")
    print(f"d3-loop/d3-unknown moved keys (listed in full below as D3-FULL): {len(d3full)}")
    print("by move kind:", dict(kinds))
    for m in sorted(table):
        print(f"  {m}: {sum(table[m].values())}")
        for kd, c in sorted(table[m].items(), key=lambda x: -x[1]):
            print(f"      {kd}: {c}")
    moved_set = set(moved)
    trace_only = [k for k in trace if k not in moved_set]
    print(f"trace-only keys (trace fired, label did not move; NOT moves): {len(trace_only)}")
    random.seed(660)
    for k in random.sample(trace_only, min(n, len(trace_only))):
        print("  trace-only sample:", k, sorted(trace[k]))
    for k, ks in unmapped:
        print("SITE-UNMAPPED demotion:", k, ks)
    for k, mechs, ks in sorted(d3full):
        print("D3-FULL demotion:", mechs, ks, k)
    for k, mechs, ks in mismatches:
        print("MISMATCH:", k, mechs, ks)
    for k in untraced:
        print("UNTRACED moved edge:", k, base[k], "->", head[k])
    for k, o, w in promotions:
        print("PROMOTION:", k, o, "->", w)
    for k, o, w in other:
        print("OTHER move:", k, o, "->", w)
    if "--list" in a:
        for k, mechs, ks in sorted(rows):
            print("moved", mechs, ks, k)
    elif n and rows:
        for k, mechs, ks in random.sample(rows, min(n, len(rows))):
            print("sample", mechs, ks, k)
    bad = untraced or promotions or other or mismatches
    print("PASS: every moved edge carries a mechanism, no promotion" if not bad else "FAIL: untraced, promoted, OTHER or mismatched moves")
    return 1 if bad else 0


sys.exit(main())
