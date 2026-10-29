"""Combine per-corpus verdicts into verdicts.jsonl and adjudication-summary.json (design §3.2/§3.3).

Per site, across configurations (contract reading ratified 2026-10-28: a site wrong under any
reachable configuration in which it compiles is WRONG):
  WRONG             any configuration WRONG (two agreeing probes)
  WRONG_candidate   any configuration with one probe for an alternative, none confirmed
  correct           every configuration in which the site compiled bound the target (>= 1)
  unresolved        compiled somewhere without a marker note (unindexed binding / indistinct)
  dead              not compiled in any adjudicated configuration
  INADMISSIBLE      no green baseline in any configuration
"""

import argparse
import collections
import glob
import json
import math
import os


def wilson(k, n, z=1.96):
    if n == 0:
        return (0.0, 1.0)
    p = k / n
    d = 1 + z * z / n
    c = p + z * z / (2 * n)
    r = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return (max(0.0, (c - r) / d), min(1.0, (c + r) / d))


def overall(vs):
    by = {v["config"]: v["verdict"] for v in vs}
    vals = set(by.values())
    if "WRONG" in vals:
        return "WRONG"
    if "WRONG_candidate" in vals:
        return "WRONG_candidate"
    compiled = [v for v in by.values() if v not in ("dead_in_config", "INADMISSIBLE")]
    if compiled and all(v == "correct_in_config" for v in compiled):
        return "correct"
    if compiled:
        return "unresolved"
    if "dead_in_config" in vals:
        return "dead"
    return "INADMISSIBLE"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--manual", help="JSON {site_id: {verdict, note}} from manual review of unresolved sites")
    a = ap.parse_args()
    rows = []
    for p in sorted(glob.glob(os.path.join(a.dir, "*-verdicts.jsonl"))):
        with open(p) as f:
            rows += [json.loads(l) for l in f]
    manual = {}
    if a.manual and os.path.exists(a.manual):
        with open(a.manual) as f:
            manual = json.load(f)
    by_site = collections.defaultdict(list)
    for r in rows:
        by_site[r["site_id"]].append(r)
    sites = []
    for sid, vs in sorted(by_site.items()):
        v0 = vs[0]
        ov = overall(vs)
        m = manual.get(sid)
        site = {"site_id": sid, "corpus": v0["corpus"], "group": v0["group"], "class": v0["class"],
                "implied": v0["implied"], "rung": v0["rung"], "callee_text": v0["callee_text"],
                "target": v0["target"], "span": v0["span"], "probe_verdict": ov,
                "per_config": {v["config"]: v["verdict"] for v in vs},
                "verdict": m["verdict"] if m else ov, "manual": m}
        sites.append(site)
    with open(os.path.join(a.out, "verdicts.jsonl"), "w") as f:
        for r in sorted(rows, key=lambda r: (r["site_id"], r["config"])):
            f.write(json.dumps(r, sort_keys=True) + "\n")
    with open(os.path.join(a.out, "site-verdicts.jsonl"), "w") as f:
        for s in sites:
            f.write(json.dumps(s, sort_keys=True) + "\n")
    summ = {}
    for g in sorted({s["group"] for s in sites}):
        sel = [s for s in sites if s["group"] == g]
        cnt = collections.Counter(s["verdict"] for s in sel)
        decided = [s for s in sel if s["verdict"] in ("WRONG", "correct", "WRONG_noncfg")]
        k = sum(1 for s in decided if s["verdict"] == "WRONG")
        kc = sum(1 for s in decided if s["verdict"] in ("WRONG", "WRONG_noncfg"))
        summ[g] = {"sites": len(sel), "verdicts": dict(sorted(cnt.items())), "decided": len(decided),
                   "wrong_cfg": k, "wrong_any_cause": kc,
                   "wilson95_cfg_wrong": wilson(k, len(decided)), "wilson95_any_wrong": wilson(kc, len(decided)),
                   "by_corpus": dict(sorted(collections.Counter(s["corpus"] for s in sel).items())),
                   "by_rung": dict(sorted(collections.Counter(s["rung"] for s in sel).items()))}
    with open(os.path.join(a.out, "adjudication-summary.json"), "w") as f:
        json.dump({"groups": summ, "sites": len(sites), "rows": len(rows)}, f, indent=1, sort_keys=True)
    print(json.dumps(summ, indent=1))


if __name__ == "__main__":
    main()
