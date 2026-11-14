#!/usr/bin/env python3
"""Regenerate obligations.jsonl from the markers in the .cc/.h fixtures and in
src/data_flow/build/cpp_prune_inline_cases.rs (inline cases: macro/census inputs).

Marker (C++ line comment, anywhere on a line):
  // @id:D x               def line of row id, path x
  // @id:U:keep|drop:why   a use line of row id (several allowed)
The owner is the nearest preceding column-0 line holding `name(` (`Cls::name(` gives `name`).
"""
import glob, json, os, re, sys

here = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(here, "../../../.."))
cases = []
ownerless = set()  # fold2/ enumeration fixtures: any owner may keep the pair
for ext in ("cc", "h", "cpp", "hpp"):
    for p in sorted(glob.glob(os.path.join(here, "*." + ext))):
        cases.append((os.path.basename(p), open(p).read()))
    for sub in ("fold2", "fold3", "fold4", "fold5", "fold6", "fold7", "fold8"):
        for p in sorted(glob.glob(os.path.join(here, sub, "*." + ext))):
            cases.append((os.path.basename(p), open(p).read()))
            ownerless.add(os.path.basename(p))
inline = open(os.path.join(root, "src/data_flow/build/cpp_prune_inline_cases.rs")).read()
for m in re.finditer(r'\(\s*"([^"]+\.(?:cc|h|cpp|hpp))",\s*r####"(.*?)"####', inline, re.S):
    cases.append((m.group(1), m.group(2)))
rows = []
for name, src in cases:
    owner = None
    defs, uses = {}, {}
    for n, line in enumerate(src.split("\n"), 1):
        om = re.match(r"^[A-Za-z_].*?\b([A-Za-z_]\w*)\s*\(", line)
        om = re.match(r"^[A-Za-z_].*?\b([A-Za-z_]\w*)\s*\(", re.sub(r"^template\s*<[^>]*>\s*", "", line))
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
            row = {"case": name, "id": rid, "path": path, "def_line": dl,
                   "use_line": ul, "keep": keep, "reason": why}
            if name not in ownerless:
                row["owner"] = downer
            rows.append(row)
with open(os.path.join(here, "obligations.jsonl"), "w") as f:
    for r in rows:
        f.write(json.dumps(r) + "\n")
print(len(rows), "rows")
