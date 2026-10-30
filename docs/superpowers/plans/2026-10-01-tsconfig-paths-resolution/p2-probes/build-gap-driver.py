"""Build a source-bound diagnostic sidecar offline; never patch production files."""
import hashlib, json, os, shutil, subprocess, sys
from pathlib import Path

packet=Path(__file__).resolve().parent
repo=packet.parents[4]
out=Path(sys.argv[1]).resolve(); out.mkdir(parents=True,exist_ok=True)
manifest=json.loads((packet/'gap-source-binding.json').read_text())
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
assert all(sha(repo/p)==h for p,h in manifest['production_inputs'].items()), 'diagnostic source drift'
# Cargo's artifact records select the actual dependency variants, never an arbitrary glob.
with (out/'cargo.jsonl').open('w') as stdout, (out/'cargo.stderr').open('w') as stderr:
    subprocess.run(['cargo','build','--release','--lib','--offline','--locked','--message-format=json'],
                   cwd=repo,stdout=stdout,stderr=stderr,check=True)
libs={}
for line in (out/'cargo.jsonl').read_text().splitlines():
    r=json.loads(line)
    if r.get('reason')=='compiler-artifact':
        candidates=[f for f in r['filenames'] if f.endswith('.rlib')]
        if candidates: libs[r['target']['name']]=candidates[0]
for name in ['js_paths.rs','js_paths_first_pass.rs','js_paths_snapshot.rs','js_paths_syntax.rs',
             'js_paths_boundary.rs','js_paths_case_fold.rs']:
    shutil.copyfile(repo/'src'/name,out/name)
with (out/'js_paths.rs').open('a') as f:f.write('\ninclude!("gap-kernel.rs");\n')
shutil.copyfile(packet/'gap-kernel.rs',out/'gap-kernel.rs')
shutil.copyfile(packet/'gap-driver.rs',out/'main.rs')
deps=Path(libs['serde']).parent
cmd=['rustc','--edition=2021','-Awarnings',str(out/'main.rs'),'-L',f'dependency={deps}']
for name in ['prism','serde','serde_json','sha2','tree_sitter','tree_sitter_typescript','rayon','bincode']:
    cmd+=['--extern',f'{name}={libs[name]}']
cmd+=['-o',str(out/'gap-driver')]
with (out/'rustc.log').open('w') as log:subprocess.run(cmd,stdout=log,stderr=log,check=True)
assert all(sha(repo/p)==h for p,h in manifest['production_inputs'].items()), 'source changed during build'
(out/'binding.json').write_text(json.dumps({'production_manifest_sha256':sha(packet/'gap-source-binding.json'),
    'driver_sha256':sha(out/'gap-driver'),'prism_library_sha256':sha(libs['prism']),
    'explainer_sha256':sha(packet/'gap-kernel.rs'),'driver_source_sha256':sha(packet/'gap-driver.rs')},indent=2)+'\n')
