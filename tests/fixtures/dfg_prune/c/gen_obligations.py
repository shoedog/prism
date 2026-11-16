#!/usr/bin/env python3
"""Regenerate obligations.jsonl from the markers in the .c fixtures and in
src/data_flow/build/c_prune_inline_cases.rs (inline cases: macro/census inputs).

Marker (C line comment, anywhere on a line):
  // @id:D x               def line of row id, path x
  // @id:U:keep|drop:why   a use line of row id (several allowed)
The owner is the nearest preceding column-0 line holding `name(`.
"""
import glob, json, os, re, sys

here = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(here, "../../../.."))
cases = []
for p in sorted(glob.glob(os.path.join(here, "*.c"))):
    cases.append((os.path.basename(p)[:-2], open(p).read()))
inline = open(os.path.join(root, "src/data_flow/build/c_prune_inline_cases.rs")).read()
for m in re.finditer(r'\(\s*"([^"]+)\.c",\s*r####"(.*?)"####', inline, re.S):
    cases.append((m.group(1), m.group(2)))
rows = []
for name, src in cases:
    owner = None
    defs, uses = {}, {}
    for n, line in enumerate(src.split("\n"), 1):
        om = re.match(r"^[A-Za-z_].*?\b([A-Za-z_]\w*)\s*\(", line)
        if om and not line.startswith("#"):
            owner = om.group(1)
        for m in re.finditer(r"@(\w+):D\s+(\w+(?:\.\w+)*)", line):
            defs[m.group(1)] = (n, m.group(2), owner)
        for m in re.finditer(r"@(\w+):U:(keep|drop):(\w+)", line):
            uses.setdefault(m.group(1), []).append((n, m.group(2) == "keep", m.group(3), owner))
    for rid, us in uses.items():
        if rid not in defs:
            sys.exit(f"{name}: row {rid} has uses but no def marker")
        dl, path, downer = defs[rid]
        for ul, keep, why, _ in us:
            rows.append({"case": name, "id": rid, "path": path, "def_line": dl,
                         "use_line": ul, "keep": keep, "reason": why, "owner": downer})
# Oracle fields (xarm.py label, #829): oracle verdicts, config, assumption, boundary and the
# derived `assert` of the rows of the 2x/3x inline cases. A row without data keeps its old meaning.
oracle = {}
op = os.path.join(here, "..", "oracle", "inline_rows.jsonl")
if os.path.exists(op):
    for line in open(op):
        o = json.loads(line)
        if o["lang"] == "c":
            oracle[(o["case"], o["id"], o["def_line"], o["use_line"])] = o
KEEP = ("oracle", "oracle_repo", "oracle_declared", "config", "assumption", "boundary",
        "inputs_gap", "head_drops", "basis", "assert", "validity_control", "seen_input")
for r in rows:
    o = oracle.get((r["case"], r["id"], r["def_line"], r["use_line"]))
    if o:
        r.update({k: o[k] for k in KEEP if k in o})
with open(os.path.join(here, "obligations.jsonl"), "w") as f:
    for r in rows:
        f.write(json.dumps(r) + "\n")
print(len(rows), "rows")
