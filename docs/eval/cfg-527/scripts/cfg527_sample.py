"""Deterministic adjudication sample (design §3.3): writes sample.json.

Strata: corpus x class x implied, non-fixture, the 7 real corpora only.
Order inside a stratum: sha256("<seed>:<site_id>") ascending.
"""

import argparse
import collections
import hashlib
import json
import os

REAL = ("prism", "ruff", "ripgrep", "uv", "tokio", "clap", "axum")


def order_key(seed, row):
    return hashlib.sha256(("%s:%s" % (seed, row["site_id"])).encode()).hexdigest()


def allocate(strata, quota, minimum=1):
    """Largest-remainder proportional allocation with a per-stratum minimum."""
    sizes = {k: len(v) for k, v in strata.items() if v}
    total = sum(sizes.values())
    if total <= quota:
        return dict(sizes)
    alloc = {k: min(minimum, n) for k, n in sizes.items()}
    left = quota - sum(alloc.values())
    if left <= 0:
        return alloc
    shares = {k: (n * quota / total) for k, n in sizes.items()}
    extra = {k: max(0.0, shares[k] - alloc[k]) for k in sizes}
    floor = {k: min(int(extra[k]), sizes[k] - alloc[k]) for k in sizes}
    for k in sizes:
        take = min(floor[k], left)
        alloc[k] += take
        left -= take
    for k in sorted(sizes, key=lambda k: (-(extra[k] - int(extra[k])), k)):
        if left <= 0:
            break
        if alloc[k] < sizes[k]:
            alloc[k] += 1
            left -= 1
    return alloc


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", required=True, help="directory holding <corpus>-pairs.jsonl")
    ap.add_argument("--seed", type=int, default=527)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    rows = []
    for c in REAL:
        p = os.path.join(a.dir, "%s-pairs.jsonl" % c)
        if os.path.exists(p):
            with open(p) as f:
                rows += [json.loads(l) for l in f]
    rows = [r for r in rows if not r["fixture"] and r["class"] not in ("none", "unmapped")]

    def strata_of(pred):
        st = collections.defaultdict(list)
        for r in rows:
            if pred(r):
                st[(r["corpus"], r["class"], bool(r["implied"]))].append(r)
        for v in st.values():
            v.sort(key=lambda r: order_key(a.seed, r))
        return st

    plan = [("a_not_implied", lambda r: r["class"] in ("a1", "a2") and not r["implied"], 60),
            ("a_implied_control", lambda r: r["class"] in ("a1", "a2") and r["implied"], 5),
            ("c", lambda r: r["class"] == "c", 10),
            ("b_std", lambda r: r["class"] == "b-std" and not r["implied"], 15),
            ("b_other", lambda r: r["class"] == "b-other" and not r["implied"], 5)]
    picked, report = [], {}
    for name, pred, quota in plan:
        st = strata_of(pred)
        alloc = allocate(st, quota)
        sel = []
        for k in sorted(alloc):
            sel += st[k][:alloc[k]]
        if name == "a_not_implied":
            # At least one per rung present.
            have = {r["rung"] for r in sel}
            pool = sorted((r for v in st.values() for r in v), key=lambda r: order_key(a.seed, r))
            for r in pool:
                if r["rung"] not in have:
                    sel.append(r)
                    have.add(r["rung"])
        for r in sel:
            r = dict(r)
            r["sample_group"] = name
            picked.append(r)
        report[name] = {"population": sum(len(v) for v in st.values()), "quota": quota, "selected": len(sel),
                        "allocation": {"%s|%s|%s" % k: n for k, n in sorted(alloc.items())}}
    with open(a.out, "w") as f:
        json.dump({"seed": a.seed, "groups": report, "sites": picked}, f, indent=1, sort_keys=True)
    print(json.dumps(report, indent=1))


if __name__ == "__main__":
    main()
