"""Offline r2b witnesses. Public/finitely generated inputs only; never discovers F.

Usage: repair_r2b.py fixtures OUT OLD_BIN HEAD_BIN
       repair_r2b.py cache OUT OLD_BIN HEAD_BIN
       repair_r2b.py metadata OUT INTERIM_BIN HEAD_BIN
"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

mode = sys.argv[1]
out, old, head = map(lambda p: Path(p).resolve(), sys.argv[2:5])
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


if mode == "fixtures":
    manifest = []
    for ext in ["jsx", "tsx"]:
        for arm in ["typeRoots", "types", "path", "reference-types"]:
            for state in ["covered", "ambient", "missing", "outside", "unread", "empty"]:
                for inherited in ([False, True] if arm in ["typeRoots", "types"] else [False]):
                    name = f"R2b-{arm}-{state}-{'parent' if inherited else 'direct'}-{ext}"
                    root = out / "controls" / name
                    cfg = {"compilerOptions": {"moduleResolution": "node", "allowJs": True,
                                              "paths": {"@lib": ["lib/real.tsx"]}}, "include": ["**/*"]}
                    source = "import {real as picked} from '@lib'; export function run(){picked();}"
                    write(root, "lib/real.tsx", "export function real(){return 1;}")
                    write(root, "node_modules/@types/local/index.d.ts", "declare module '@lib' {}" if state == "ambient" else "export interface Empty {}")
                    write(root, "unread.txt", "declare module '@lib' {}")
                    input_name = {"covered": "local", "ambient": "local", "missing": "missing",
                                  "outside": "../outside.d.ts", "unread": "./unread.txt", "empty": ""}[state]
                    if arm == "typeRoots" and state in ["covered", "ambient"]:
                        input_name = "./node_modules/@types"
                    if arm == "path" and state in ["covered", "ambient"]:
                        input_name = "./node_modules/@types/local/index.d.ts"
                    if arm in ["typeRoots", "types"]:
                        cfg["compilerOptions"][arm] = [] if state == "empty" else [input_name]
                        if inherited:
                            write(root, "tsconfig.parent.json", json.dumps(cfg))
                            cfg = {"extends": "./tsconfig.parent.json"}
                    else:
                        key = "types" if arm == "reference-types" else "path"
                        source = f"/// <reference {key}=\"{input_name}\" />\n" + source
                    write(root, "tsconfig.json", json.dumps(cfg))
                    write(root, f"app.{ext}", source)
                    exact = state == "covered" or (state == "empty" and arm in ["types", "typeRoots"])
                    a, _ = query(old, root)
                    b, _ = query(head, root)
                    assert bool(b[0]["resolved_targets"]) == exact, (name, b)
                    if exact:
                        target = b[0]["resolved_targets"]; assert len(target) == 1
                        assert target[0]["confidence"] == "exact" and target[0]["function_id"]["file"] == "lib/real.tsx"
                        assert a != b, (name, "RED did not discriminate")
                    else:
                        assert a == b, (name, "negative changed")
                    manifest.append({"case": name, "expectation": "gain" if exact else "base",
                                     "files": [str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()],
                                     "arm": arm, "state": state, "grammar": ext, "red_discriminates": exact})
    (out / "witnesses.json").write_text(json.dumps(manifest, indent=2))
    print(json.dumps({"cases": len(manifest), "red_positives": sum(r['red_discriminates'] for r in manifest), "all_pass": True}))

elif mode == "cache":
    results = []
    for ext in ["jsx", "tsx"]:
        for arm in ["typeRoots", "types", "path", "reference-types"]:
            d = out / "cache" / (arm + '-' + ext)
            root, cache = d / "repo", d / "cache"
            cfg = {"compilerOptions": {"moduleResolution": "node", "allowJs": True,
                                      "paths": {"@lib": ["lib/real.tsx"]}}, "files": [f"app.{ext}"], "include": []}
            source = "import {real as picked} from '@lib'; export function run(){picked();}"
            if arm in ["types", "typeRoots"]:
                cfg["compilerOptions"][arm] = ["local" if arm == "types" else "./node_modules/@types"]
            else:
                source = f"/// <reference {'types' if arm == 'reference-types' else 'path'}=\"{'local' if arm == 'reference-types' else './node_modules/@types/local/index.d.ts'}\" />\n" + source
            write(root, "tsconfig.json", json.dumps(cfg))
            write(root, f"app.{ext}", source)
            write(root, "lib/real.tsx", "export function real(){return 1;}")
            write(root, "node_modules/@types/local/index.d.ts", "export interface Empty {}")
            prior, _ = query(old, root, cache)
            blobs = list(cache.rglob('cpg-cache.bin')); assert len(blobs) == 1
            stamp = lambda: (blobs[0].stat().st_mtime_ns, hashlib.sha256(blobs[0].read_bytes()).hexdigest())
            before = stamp(); cold, _ = query(head, root, cache); after = stamp()
            assert prior != cold and cold[0]['resolved_targets'] and before != after
            assert query(head, root)[0] == cold
            assert query(head, root, cache)[0] == cold and stamp() == after
            (root / 'node_modules/@types/local/index.d.ts').unlink()
            shutil.rmtree(root / 'node_modules/@types/local')
            missing, _ = query(head, root, cache)
            # An existing empty typeRoots directory is still covered; named/file inputs are unresolved.
            assert bool(missing[0]['resolved_targets']) == (arm == 'typeRoots')
            assert missing == query(head, root)[0]
            write(root, "node_modules/@types/local/index.d.ts", "declare module '@lib' {}")
            ambient, _ = query(head, root, cache); assert not ambient[0]['resolved_targets']
            assert ambient == query(head, root)[0]
            write(root, "node_modules/@types/local/index.d.ts", "export interface Empty {}")
            write(root, "node_modules/@types/local/package.json", json.dumps({'types':'./index.d.ts'}))
            inside, _ = query(head, root, cache); assert inside[0]['resolved_targets']
            assert inside == query(head, root)[0]
            write(out, "external-inputs/outside.d.ts", "declare module '@lib' {}")
            write(root, "node_modules/@types/local/package.json", json.dumps({'types':str(out/'external-inputs/outside.d.ts')}))
            outside, _ = query(head, root, cache)
            assert bool(outside[0]['resolved_targets']) == (arm == 'path')
            assert outside == query(head, root)[0]
            (root / 'node_modules/@types/local/package.json').unlink()
            assert query(head, root, cache)[0] == inside
            results.append({'arm': arm, 'grammar': ext, 'same_version_rebuilt': True,
                            'next_hit': True, 'missing_and_ambient_parity': True, 'metadata_parity': True, 'versions': '105/61'})
    (out / 'cache-summary.json').write_text(json.dumps(results, indent=2))
    print(json.dumps({'cases': len(results), 'all_pass': True}))
elif mode == "metadata":
    base = Path(sys.argv[5]).resolve()
    results = []
    write(out, "metadata-external/outside.d.ts", "declare module '@lib' { export function real(): void; }")
    external = str(out / 'metadata-external/outside.d.ts')
    for ext in ['jsx', 'tsx']:
        for arm in ['typeRoots', 'types', 'reference-types']:
            for key in ['types', 'typings', 'typesVersions']:
                root = out / 'metadata-controls' / (arm + '-' + key + '-' + ext)
                cfg = {'compilerOptions': {'moduleResolution': 'node', 'allowJs': True, 'paths': {'@lib': ['lib/real.tsx']}}, 'include': ['**/*']}
                source = "import {real as picked} from '@lib'; export function run(){picked();}"
                if arm == 'reference-types': source = "/// <reference types='local' />\n" + source
                else: cfg['compilerOptions'][arm] = ['local' if arm == 'types' else './node_modules/@types']
                write(root, 'tsconfig.json', json.dumps(cfg)); write(root, 'app.' + ext, source)
                write(root, 'lib/real.tsx', 'export function real(){return 1;}')
                write(root, 'node_modules/@types/local/index.d.ts', 'export interface Empty {}')
                value = {'*': {'*': [external]}} if key == 'typesVersions' else external
                write(root, 'node_modules/@types/local/package.json', json.dumps({key: value}))
                before, _ = query(old, root); after, _ = query(head, root); ref, _ = query(base, root)
                assert before[0]['resolved_targets'] and after == ref and before != after, (arm, key, ext)
                results.append({'arm': arm, 'key': key, 'grammar': ext, 'interim_red': True, 'final_equals_base': True})
    (out / 'metadata-summary.json').write_text(json.dumps(results, indent=2))
    print(json.dumps({'cases': len(results), 'all_pass': True}))
else:
    raise SystemExit('unknown mode')
