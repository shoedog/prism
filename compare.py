#!/usr/bin/env python3
import sys, json, collections, os, re
c = sys.argv[1]
def load(p):
    cnt = collections.Counter()
    for l in open(p):
        e = json.loads(l)
        k = (e["from"]["file"], e["from"]["line"], json.dumps(e["from"]["path"], sort_keys=True), e["from"]["access"],
             e["to"]["file"], e["to"]["line"], json.dumps(e["to"]["path"], sort_keys=True), e["to"]["access"],
             e["confidence"], e["doubt"], e.get("kill_line"))
        cnt[k] += 1
    return cnt
b = load(f"/tmp/out/{c}.base.edges"); h = load(f"/tmp/out/{c}.head.edges")
removed = b - h; added = h - b
def kind(f):
    lo = f.lower()
    if lo.endswith(".c"): return "c_source"
    if lo.endswith(".h"): return "dot_h"
    if lo.endswith((".hpp", ".hh", ".hxx", ".h++", ".inl", ".ipp", ".tcc", ".txx")): return "cpp_header"
    if lo.endswith((".cc", ".cpp", ".cxx", ".c++", ".cu")): return "cpp_source"
    return "other:" + os.path.splitext(f)[1]
drops = {}
for l in open(f"/tmp/out/{c}.head.trace", errors="replace"):
    if l.startswith("c-prune-drop\t"):
        _, rule, f, line, base = l.rstrip("\n").split("\t")
        # file path in trace is absolute-ish; match by suffix
        drops.setdefault((f, int(line), base), set()).add(rule)
def rule_of(e):
    f = e[4]; line = e[5]; base = json.loads(e[6])["base"]
    rs = set()
    for (tf, tl, tb), r in drops.items():
        if tl == line and tb == base and tf.endswith(f):
            rs |= r
    return "U" if "U" in rs and "D" not in rs else ("D" if rs == {"D"} else ("U+D" if rs else "?"))
led = collections.Counter()
labels = collections.Counter()
for e, n in removed.items():
    led[(rule_of(e), kind(e[4]))] += n
    labels[(e[8], e[9])] += n
res = {"corpus": c, "base_edges": sum(b.values()), "head_edges": sum(h.values()),
       "removed": sum(removed.values()), "added": sum(added.values()),
       "removed_outside_cpp_or_h": sum(n for e, n in removed.items() if kind(e[4]) in ("c_source",) or kind(e[4]).startswith("other")),
       "ledger_rule_kind": {f"{r}/{k}": n for (r, k), n in sorted(led.items())},
       "removed_by_label": {f"{a}/{d}": n for (a, d), n in sorted(labels.items(), key=lambda x: str(x))}}
print(json.dumps(res, indent=1))
with open(f"/tmp/out/{c}.removed.jsonl", "w") as f:
    for e, n in sorted(removed.items(), key=lambda x: (x[0][0], x[0][1], x[0][5])):
        for _ in range(n):
            f.write(json.dumps({"from_file": e[0], "from_line": e[1], "path": json.loads(e[2])["base"], "to_file": e[4], "to_line": e[5], "confidence": e[8], "doubt": e[9], "kill_line": e[10], "rule": rule_of(e), "kind": kind(e[0])}) + "\n")
