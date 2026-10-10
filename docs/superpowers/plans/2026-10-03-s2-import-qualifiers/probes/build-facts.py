"""READ: build an offline facts driver against this exact checkout; no Git writes.
Usage: python3 build-facts.py --head
"""
import json
import shutil
import subprocess
import argparse
from pathlib import Path

here = Path(__file__).resolve().parent
repo = here.parents[4]
out = repo/'target/s2-plan'
parser = argparse.ArgumentParser()
parser.add_argument('--head', action='store_true', required=True, help='preserve immutable S2-0 base binaries')
parser.parse_args()
scratch = out/'facts-driver-source'
scratch.mkdir(parents=True, exist_ok=True)
(scratch/'Cargo.toml').write_text(f'''[package]
name = "s2-dump-imports"
version = "0.0.0"
edition = "2021"
[dependencies]
prism = {{ path = {json.dumps(str(repo))} }}
serde_json = "1"
[[bin]]
name = "s2-dump-imports"
path = {json.dumps(str(here/'dump_imports.rs'))}
''')
# READ: preserve the checkout's dependency versions, not newer cached releases.
shutil.copy2(repo/'Cargo.lock',scratch/'Cargo.lock')
subprocess.run(['cargo','build','--offline','--release','--manifest-path',str(scratch/'Cargo.toml'),
                '--target-dir',str(repo/'target')],check=True,cwd=repo)
def packages(manifest):
    metadata = json.loads(subprocess.check_output(['cargo','metadata','--offline','--locked',
        '--format-version','1','--manifest-path',str(manifest)],cwd=repo))
    return {(p['name'],p['version'],p.get('source')) for p in metadata['packages']}
assert packages(scratch/'Cargo.toml') - packages(repo/'Cargo.toml') == {('s2-dump-imports','0.0.0',None)}, 'dependency drift'
(out/'bin').mkdir(exist_ok=True)
shutil.copy2(repo/'target/release/s2-dump-imports',out/'bin/head-dump_imports')
shutil.copy2(repo/'target/release/prism',out/'bin/head-prism')
print(json.dumps({'claim':'MEASURED','status':'BUILT','driver':'target/s2-plan/bin/head-dump_imports'}))
