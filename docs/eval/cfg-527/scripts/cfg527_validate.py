"""Probe validation (design §2.4): run prism + census on each fixture crate and on the
matrix corpus, and compare with fixtures/expected.json. Exit 1 on any mismatch."""

import argparse
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import cfg527_census as census  # noqa: E402


def dump(prism, root, path):
    with open(path, "w") as f:
        subprocess.run([prism, "nav", "--no-cache", "call-stats", "--repo", root, "--dump-sites"],
                       stdout=f, check=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--prism", required=True)
    ap.add_argument("--fixtures", required=True)
    ap.add_argument("--matrix", required=True)
    ap.add_argument("--host-cfg", required=True)
    ap.add_argument("--work", required=True)
    a = ap.parse_args()
    with open(os.path.join(a.fixtures, "expected.json")) as f:
        expected = json.load(f)
    os.makedirs(a.work, exist_ok=True)
    report, bad = {}, 0
    for name in sorted(k for k in expected if not k.startswith("_")):
        root = os.path.join(a.fixtures, name)
        d = os.path.join(a.work, name + ".jsonl")
        dump(a.prism, root, d)
        census.run(argparse.Namespace(corpus=name, root=root, dump=d, out=a.work,
                                      host_cfg=a.host_cfg, identity=None))
        with open(os.path.join(a.work, name + "-pairs.jsonl")) as f:
            rows = [json.loads(l) for l in f]
        got = {}
        for callee, exp in expected[name].items():
            sel = [r for r in rows if r["callee_text"] == callee]
            if exp == "no_exact":
                ok = not sel
                got[callee] = {"ok": ok, "rows": len(sel)}
            else:
                ok = len(sel) == 1
                obs = {}
                if ok:
                    r = sel[0]
                    obs = {"class": r["class"], "implied": r["implied"], "rung": r["rung"],
                           "statically_false": r.get("statically_false"),
                           "origins": sorted({e["origin"] for e in r.get("T") or ()})}
                    ok = all(obs.get(k) == v for k, v in exp.items())
                got[callee] = {"ok": ok, "observed": obs, "expected": exp}
            bad += 0 if ok else 1
        report[name] = got
    # Matrix: probe-admissibility only (map rate, no unmapped, every class reported).
    md = os.path.join(a.work, "matrix.jsonl")
    dump(a.prism, a.matrix, md)
    s = census.run(argparse.Namespace(corpus="matrix", root=a.matrix, dump=md, out=a.work,
                                      host_cfg=a.host_cfg, identity=None))
    report["matrix"] = {"E": s["rust_exact_pairs"], "map_rate": s["map_rate"], "admissible": s["admissible"],
                        "classes": {k: v for k, v in s["fixture"]["classes"].items()
                                    if v["implied"] or v["not_implied"]},
                        "nonfixture_classes": {k: v for k, v in s["nonfixture"]["classes"].items()
                                               if v["implied"] or v["not_implied"]}}
    if not s["admissible"]:
        bad += 1
    report["mismatches"] = bad
    print(json.dumps(report, indent=1, sort_keys=True))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
