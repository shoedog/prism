#!/usr/bin/env python3
"""Regenerates the generated block of TESTS.md (#829) from the oracle data: no hand-kept totals.

  python3 scripts/dfg_prune/oracle_report.py

Reads `oracle/pair_rows.jsonl`, `oracle/inline_rows.jsonl`, `oracle/receipts.jsonl`,
`oracle/legacy_kinds.json` (the hand kind tags of the first matrix, informational only) and
counts the quarantined tests in the sources. Rewrites the text between the BEGIN/END markers.
"""
import collections
import glob
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "../.."))
ORACLE = os.path.join(ROOT, "tests/fixtures/dfg_prune/oracle")
TESTS_MD = os.path.join(ROOT, "docs/superpowers/plans/2026-11-14-829-macro-brackets/TESTS.md")
BEGIN, END = "<!-- BEGIN GENERATED (oracle_report.py) -->", "<!-- END GENERATED -->"
ORDER = ["kept", "pinned_hole", "dropped", "guide", "no_basis"]
VERDICTS = ["CONTRADICTED", "CONFIRMED", "XOFF", "UNJUDGED"]


def load(name):
    p = os.path.join(ORACLE, name)
    return [json.loads(l) for l in open(p) if l.strip()] if os.path.exists(p) else []


def table(head, rows):
    out = ["| " + " | ".join(head) + " |", "|" + "---|" * len(head)]
    out += ["| " + " | ".join(str(c) for c in r) + " |" for r in rows]
    return out


def counts(rows, key):
    c = collections.Counter(key(r) for r in rows)
    return c


def quarantined():
    out = collections.Counter()
    for p in glob.glob(os.path.join(ROOT, "src/**/*.rs"), recursive=True):
        n = open(p).read().count("#[ignore = \"tentative design")
        if n:
            out[os.path.relpath(p, ROOT)] = n
    return out


def generate():
    pair, inline = load("pair_rows.jsonl"), load("inline_rows.jsonl")
    kinds = json.load(open(os.path.join(ORACLE, "legacy_kinds.json")))
    receipts = load("receipts.jsonl")
    L = ["", "## Generated: oracle results (do not edit; `oracle_report.py`)", ""]
    L.append("Truth is the expansion-arm oracle (`xarm.py label`, clang at authoring time only); "
             "the Rust tests read the frozen data. `assert` is derived: **kept** (must-keep, a "
             "dropped pair is red), **pinned_hole** (BOUNDARY, today's drop is asserted so a fix "
             "flips it), **dropped** (validity control), **guide** (reported, never asserted), "
             "**no_basis** (no valid program or the oracle cannot judge).")
    L.append("")
    groups = [("pair rows (Rust sources, C and C++)", pair),
              ("inline cases, C", [r for r in inline if r["lang"] == "c"]),
              ("inline cases, C++", [r for r in inline if r["lang"] == "cpp"])]
    L.append("### Rows by oracle verdict and derived assertion")
    L.append("")
    rows = []
    for name, rs in groups:
        v, a = counts(rs, lambda r: r["oracle"]), counts(rs, lambda r: r["assert"])
        rows.append([name, len(rs)] + [v.get(x, 0) for x in VERDICTS] + [a.get(x, 0) for x in ORDER])
    allr = pair + inline
    v, a = counts(allr, lambda r: r["oracle"]), counts(allr, lambda r: r["assert"])
    rows.append(["**all**", len(allr)] + [v.get(x, 0) for x in VERDICTS] + [a.get(x, 0) for x in ORDER])
    L += table(["rows", "n"] + VERDICTS + ORDER, rows)
    L.append("")
    L.append("### BOUNDARY rows (computed: the verdict flips under the declared configuration, "
             "or clang opens an in-repo file the census does not read)")
    L.append("")
    b = [r for r in allr if r["boundary"]]
    by = collections.defaultdict(collections.Counter)
    for r in b:
        by[r["assumption"] or "?"][r["assert"]] += 1
    L += table(["assumption", "rows"] + ORDER,
               [[k, sum(c.values())] + [c.get(x, 0) for x in ORDER] for k, c in sorted(by.items())])
    L.append("")
    L.append("Pinned holes: " + ", ".join(sorted(
        "%s:%s" % (r.get("test", "").split("::")[-1] or r["case"] + "/" + r["id"],
                   r["assumption"]) for r in allr if r["assert"] == "pinned_hole")[:40]) + " ...")
    L.append("")
    q = quarantined()
    L.append("### Quarantined tentative-design tests: %d `#[ignore]`d" % sum(q.values()))
    L.append("")
    L += table(["file", "tests"], sorted(q.items()))
    L.append("")
    L.append("### Runtime receipts: %d classes (`xarm_receipts.py run`, needs clang, not run in CI)"
             % len(receipts))
    L.append("")
    L += ["- " + r["class"] for r in receipts]
    L.append("")
    L.append("### Inline rows (cases 20 to 34)")
    L.append("")
    rows = []
    for r in inline:
        rows.append([r["lang"], r["case"], r["id"], kinds.get(r["id"], "") if r["case"].startswith("2") and r["case"][:2] in ("20", "21") else "",
                     r["want"], r["oracle"] + ("" if r["oracle_repo"] == r["oracle"] else " (repo " + r["oracle_repo"] + ")"),
                     r["assumption"] or "", "drops" if r["head_drops"] else "keeps", r["assert"]])
    L += table(["lang", "case", "id", "legacy kind", "hand", "oracle", "assumption", "head", "assert"], rows)
    L.append("")
    L.append("### Pair rows by test")
    L.append("")
    byt = collections.defaultdict(collections.Counter)
    for r in pair:
        byt[r["test"].split("::")[-1]][r["assert"]] += 1
    L += table(["test", "rows"] + ORDER,
               [[t, sum(c.values())] + [c.get(x, 0) for x in ORDER] for t, c in sorted(byt.items())])
    L.append("")
    L.append("### Hand expectation against the oracle")
    L.append("")
    moved = [r for r in allr if r["want"] == "kept" and r["assert"] != "kept"]
    mv = collections.defaultdict(list)
    for r in moved:
        mv[r["assert"]].append(r.get("what") and r["test"].split("::")[-1] or r["lang"] + ":" + r["case"].split("_")[0] + "/" + r["id"])
    for k in ("guide", "no_basis", "pinned_hole", "dropped"):
        names = sorted(set(mv.get(k, [])))
        L.append("- hand `kept`, now **%s**: %d rows" % (k, len(mv.get(k, []))))
        if names and k != "no_basis":
            L.append("  " + ", ".join(names[:60]) + (" ..." if len(names) > 60 else ""))
    ctrl = [r for r in allr if r["want"] == "pruned"]
    cc = counts(ctrl, lambda r: r["assert"])
    L.append("- hand controls (`pruned`): %d rows -> %s" % (len(ctrl), dict(cc)))
    dis = [r for r in allr if r["want"] == "pruned" and r["oracle"] == "CONTRADICTED"]
    L.append("- hand control but the oracle says must-keep: %d rows" % len(dis))
    L.append("")
    return "\n".join(L)


def main():
    text = open(TESTS_MD).read()
    block = BEGIN + "\n" + generate() + "\n" + END
    if BEGIN in text:
        text = re.sub(re.escape(BEGIN) + ".*?" + re.escape(END), lambda m: block, text, flags=re.S)
    else:
        raise SystemExit("TESTS.md has no generated block markers")
    open(TESTS_MD, "w").write(text)
    print("wrote", TESTS_MD)


if __name__ == "__main__":
    main()
