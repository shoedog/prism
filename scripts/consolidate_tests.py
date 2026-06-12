#!/usr/bin/env python3
"""Per-directory test consolidation (WP2, spec §3.1). Usage:
   python3 scripts/consolidate_tests.py tests/ast ast
   python3 scripts/consolidate_tests.py tests/mcp mcp --required-features mcp
Idempotence guard: refuses to run if <dir>/main.rs already exists.
"""
import re, sys, pathlib

d = pathlib.Path(sys.argv[1])
target = sys.argv[2]
req_feat = sys.argv[4] if "--required-features" in sys.argv else None

assert not (d / "main.rs").exists(), f"{d} already consolidated"
files = sorted(p for p in d.glob("*.rs") if p.name != "main.rs")
assert files, f"no test files under {d}"

header = re.compile(r'#\[path = "[^"]*common/mod\.rs"\]\s*\nmod common;\n')
uses_common = False
for p in files:
    s = p.read_text()
    s2 = header.sub("", s)
    s2 = s2.replace("use common::", "use crate::common::")
    if s2 != s:
        uses_common = True
        p.write_text(s2)

rel = "../" * (len(d.parts) - 1) + "common/mod.rs"
lines = []
if uses_common:
    lines.append(f'#[allow(dead_code)]\n#[path = "{rel}"]\nmod common;\n')
lines += [f"mod {p.stem};\n" for p in files]
(d / "main.rs").write_text("".join(lines))

c = pathlib.Path("Cargo.toml").read_text()
block = re.compile(
    r'\[\[test\]\]\nname = "[^"]+"\npath = "' + re.escape(str(d))
    + r'/[^"]+"\n(required-features = \[[^\]]*\]\n)?\n?'
)
n_removed = len(block.findall(c))
c = block.sub("", c)
entry = f'\n[[test]]\nname = "{target}"\npath = "{d}/main.rs"\n'
if req_feat:
    entry += f'required-features = ["{req_feat}"]\n'
pathlib.Path("Cargo.toml").write_text(c.rstrip() + "\n" + entry)
print(f"{target}: {len(files)} files, removed {n_removed} old [[test]] blocks")
