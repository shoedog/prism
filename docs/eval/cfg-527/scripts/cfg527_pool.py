"""Pool per-corpus census summaries into pooled-summary.json and summary.md (design §2.1.2)."""

import argparse
import glob
import json
import os

REAL = ("prism", "ruff", "ripgrep", "uv", "tokio", "clap", "axum")
SEPARATE = ("prism-quick",)
CLASSES = ("none", "a1", "a2", "b-std", "b-other", "c", "unmapped")
SUMMED = ("E", "G", "A", "A_syntactic", "F1_cost_syntactic", "F3_not_implied", "F1_cost", "F2_cost", "F2_cost_with_c", "F3_cost", "F3_false", "F3_unknown",
          "statically_false", "statically_false_always")


def add_block(acc, b):
    for k in SUMMED:
        acc[k] = acc.get(k, 0) + b.get(k, 0)
    cl = acc.setdefault("classes", {})
    for c in CLASSES:
        d = cl.setdefault(c, {"implied": 0, "not_implied": 0})
        src = b["classes"].get(c, {"implied": 0, "not_implied": 0})
        d["implied"] += src["implied"]
        d["not_implied"] += src["not_implied"]
    for h in ("origin_hist", "atom_family_hist"):
        dst = acc.setdefault(h, {})
        for k, v in b.get(h, {}).items():
            dst[k] = dst.get(k, 0) + v


def pct(n, d):
    return "%.2f%%" % (100.0 * n / d) if d else "n/a"


def cls_cell(b, c):
    d = b["classes"][c]
    return "%d / %d" % (d["not_implied"], d["implied"])


def table_classes(rows):
    out = ["| corpus | E | G | A | (a1) ¬impl / impl | (a2) | (b-std) | (b-other) | (c) | stat-false | map rate |",
           "|---|---:|---:|---:|---|---|---|---|---:|---:|---:|"]
    for name, b, s in rows:
        out.append("| %s | %d | %d | **%d** | %s | %s | %s | %s | %d | %d | %s |" % (
            name, b["E"], b["G"], b["A"], cls_cell(b, "a1"), cls_cell(b, "a2"), cls_cell(b, "b-std"),
            cls_cell(b, "b-other"), b["classes"]["c"]["not_implied"] + b["classes"]["c"]["implied"],
            b["statically_false"], ("%.4f" % s["map_rate"]) if s else "—"))
    return out


def table_costs(rows):
    out = ["| corpus | E | F1 strict | F2 alt-aware | F2 + (c) | F3 (false + unknown) |",
           "|---|---:|---|---|---|---|"]
    for name, b, _ in rows:
        out.append("| %s | %d | %d (%s) | %d (%s) | %d (%s) | %d (%s) = %d + %d |" % (
            name, b["E"], b["F1_cost"], pct(b["F1_cost"], b["E"]), b["F2_cost"], pct(b["F2_cost"], b["E"]),
            b["F2_cost_with_c"], pct(b["F2_cost_with_c"], b["E"]), b["F3_cost"], pct(b["F3_cost"], b["E"]),
            b["F3_false"], b["F3_unknown"]))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", required=True)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    sums = {}
    for p in sorted(glob.glob(os.path.join(a.dir, "*-summary.json"))):
        with open(p) as f:
            s = json.load(f)
        sums[s["corpus"]] = s
    pooled, pooled_rung, pooled_fix = {}, {}, {}
    for c in REAL:
        if c not in sums:
            continue
        add_block(pooled, sums[c]["nonfixture"])
        add_block(pooled_fix, sums[c]["fixture"])
        for r, b in sums[c]["by_rung_nonfixture"].items():
            add_block(pooled_rung.setdefault(r, {}), b)
    res = {"real_corpora": [c for c in REAL if c in sums], "missing": [c for c in REAL if c not in sums],
           "pooled_nonfixture": pooled, "pooled_fixture": pooled_fix, "pooled_by_rung": pooled_rung,
           "separate": {c: sums[c]["nonfixture"] for c in SEPARATE if c in sums},
           "per_corpus": {c: {k: sums[c][k] for k in ("rust_exact_pairs", "map_rate", "admissible",
                                                        "parent_unknown_files", "parent_unknown_pairs",
                                                        "unparsed_files", "cfg_if_files", "attr_parse_errors",
                                                        "files_parsed", "packages")}
                          for c in sums}}
    with open(os.path.join(a.out, "pooled-summary.json"), "w") as f:
        json.dump(res, f, indent=1, sort_keys=True)

    L = ["# #527 cfg census — summary", "",
         "Unit: one (call site, Exact target) pair, target file `.rs`, non-fixture. "
         "Class cells are `not implied / implied`. **A** = (a) ∧ ¬implied.", "",
         "## Table A — per corpus and pooled (7 real corpora)", ""]
    rows = [(c, sums[c]["nonfixture"], sums[c]) for c in REAL if c in sums]
    rows.append(("**pooled**", pooled, None))
    rows += [(c + " (separate)", sums[c]["nonfixture"], sums[c]) for c in SEPARATE if c in sums]
    L += table_classes(rows)
    L += ["", "## Table B — per rung, pooled", ""]
    L += table_classes([(r, b, None) for r, b in sorted(pooled_rung.items())])
    L += ["", "## Table C — fence cost (Exact pairs demoted), share of E", ""]
    L += table_costs(rows)
    L += ["", "## T origin histogram (pairs with that origin among T's conjuncts, pooled)", "",
          "| origin | pairs |", "|---|---:|"]
    L += ["| %s | %d |" % kv for kv in sorted(pooled.get("origin_hist", {}).items())]
    L += ["", "## Atom-family histogram (pairs whose T mentions the family, pooled)", "",
          "| family | pairs |", "|---|---:|"]
    L += ["| %s | %d |" % kv for kv in sorted(pooled.get("atom_family_hist", {}).items())]
    L += ["", "## Fixture pairs (excluded above)", "",
          "Pooled fixture-path pairs: E=%d, G=%d, A=%d." % (pooled_fix.get("E", 0), pooled_fix.get("G", 0),
                                                             pooled_fix.get("A", 0)), "",
          "## Coverage", "",
          "| corpus | files parsed | packages | parse-error files | parent_unknown files (pairs) | cfg_if! files | attr parse errors |",
          "|---|---:|---:|---:|---|---:|---:|"]
    for c in list(REAL) + list(SEPARATE):
        if c in sums:
            s = sums[c]
            L.append("| %s | %d | %d | %d | %d (%d) | %d | %d |" % (
                c, s["files_parsed"], s["packages"], s["unparsed_files"], s["parent_unknown_files"],
                s["parent_unknown_pairs"], s["cfg_if_files"], s["attr_parse_errors"]))
    extra = os.path.join(a.dir, "summary-extra.md")
    if os.path.exists(extra):
        with open(extra) as f:
            L += ["", f.read().rstrip()]
    with open(os.path.join(a.out, "summary.md"), "w") as f:
        f.write("\n".join(L) + "\n")


if __name__ == "__main__":
    main()
