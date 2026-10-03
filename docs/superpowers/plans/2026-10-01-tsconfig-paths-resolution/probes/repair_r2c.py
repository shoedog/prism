"""Offline absence/scanning/cache witnesses; public synthetic inputs only.
Usage: repair_r2c.py OUT R2B_BIN HEAD_BIN
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

out, before, head = [Path(p).resolve() for p in sys.argv[1:4]]
out.mkdir(parents=True, exist_ok=True)


def write(root, name, text):
    p = root / name
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text)


def query(binary, root, cache=None):
    proc = subprocess.run([str(binary), "nav", *(["--cache-dir", str(cache)] if cache else ["--no-cache"]),
                           "call-stats", "--repo", str(root), "--dump-sites"], capture_output=True, text=True)
    assert proc.returncode == 0, proc.stderr
    rows = [r for r in map(json.loads, proc.stdout.splitlines()) if r.get("record_kind") == "call_site"]
    assert len(rows) == 1, rows
    return rows, proc.stderr


results = []
for ext in ["jsx", "tsx"]:
    for arm in ["types", "typeRoots", "path", "reference-types"]:
        d = out / (arm + "-" + ext)
        root, cache = d / "repo", d / "cache"
        cfg = {"compilerOptions": {"moduleResolution": "node", "allowJs": True,
                                  "paths": {"@lib": ["lib/real.tsx"]}}, "files": ["app." + ext], "include": []}
        value = {"types": "local", "typeRoots": "./node_modules/local", "path": "./node_modules/local/client.d.ts",
                 "reference-types": "local/client"}[arm]
        source = "import {real as picked} from '@lib'; export function run(){picked();}"
        if arm in ["types", "typeRoots"]:
            cfg["compilerOptions"][arm] = [value]
        else:
            source = f"/// <reference {'path' if arm == 'path' else 'types'}='{value}' />\n" + source
        write(root, "tsconfig.json", json.dumps(cfg))
        write(root, "app." + ext, source)
        write(root, "lib/real.tsx", "export function real(){return 1;}")
        old, _ = query(before, root, cache)
        assert not old[0]["resolved_targets"]
        blobs = list(cache.rglob("cpg-cache.bin"))
        assert len(blobs) == 1
        stamp = lambda: (blobs[0].stat().st_mtime_ns, hashlib.sha256(blobs[0].read_bytes()).hexdigest())
        old_stamp = stamp()
        absent, stderr = query(head, root, cache)
        assert absent[0]["resolved_targets"] and "absent type inputs=1 " in stderr
        assert stamp() != old_stamp and absent == query(head, root)[0]
        new_stamp = stamp()
        assert query(head, root, cache)[0] == absent and stamp() == new_stamp
        write(root, "node_modules/local/package.json", '{"types":"./client.d.ts"}')
        write(root, "node_modules/local/client.d.ts", "declare module '*.svg' {}")
        assert query(head, root, cache)[0] == absent == query(head, root)[0]
        write(root, "node_modules/local/client.d.ts", "declare /* tolerant */ module '@lib' {}")
        shadow, _ = query(head, root, cache)
        assert shadow == old == query(head, root)[0]
        write(root, "node_modules/local/client.d.ts", "declare module '*.svg' {}")
        assert query(head, root, cache)[0] == absent
        write(root, "node_modules/local/client.d.ts", "/// <reference path='../../../outside.d.ts' />")
        outside, warning = query(head, root, cache)
        assert outside == old == query(head, root)[0] and "warning: P1 paths config declined" in warning
        write(root, "node_modules/local/client.d.ts", "/// <reference path='./missing.d.ts' />")
        transitive, stderr = query(head, root, cache)
        assert transitive == absent == query(head, root)[0] and "absent type inputs=1 " in stderr
        (root / "node_modules/local/client.d.ts").unlink()
        missing, stderr = query(head, root, cache)
        assert missing == absent == query(head, root)[0] and "absent type inputs=" in stderr
        write(root, "node_modules/local/client.d.ts", "export interface Empty {}")
        write(out, "external/outside.d.ts", "declare module '@lib' {}")
        write(root, "node_modules/local/package.json", json.dumps({"typings": str(out / "external/outside.d.ts")}))
        metadata, warning = query(head, root, cache)
        # Direct file/subpath references do not consult the parent's type entry.
        assert bool(metadata[0]["resolved_targets"]) == (arm in ["path", "reference-types"])
        assert metadata == query(head, root)[0]
        if arm in ["types", "typeRoots"]:
            assert "warning: P1 paths config declined" in warning
        if arm in ["types", "reference-types"]:
            # Native Node10 secondary lookup remains reachable after custom roots miss.
            if arm == "reference-types":
                write(root, "app." + ext, source.replace("'local/client'", "'local'"))
            for roots in [["./absent-roots"], []]:
                cfg["compilerOptions"]["typeRoots"] = roots
                write(root, "tsconfig.json", json.dumps(cfg))
                secondary, warning = query(head, root, cache)
                assert not secondary[0]["resolved_targets"] and "warning: P1 paths config declined" in warning
                assert secondary == query(head, root)[0]
            write(root, "node_modules/local/package.json", '{"types":"./client.d.ts"}')
            inside, _ = query(head, root, cache)
            assert inside[0]["resolved_targets"] and inside == query(head, root)[0]
        results.append({"arm": arm, "grammar": ext, "same_version_rebuilt": True, "next_hit": True,
                        "absent_counter": 1, "ambient_wildcard_transitive_metadata_parity": True, "versions": "105/61"})
(out / "cache-summary.json").write_text(json.dumps(results, indent=2))
print(json.dumps({"cases": len(results), "all_pass": True, "versions": "105/61"}))
