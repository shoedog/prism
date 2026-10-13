"""Bound public-only reproduction, or retained synthetic controls.
measure.py BASE_BIN HEAD_BIN HEAD_FACTS TS_JS OUT --public
measure.py BASE_BIN HEAD_BIN HEAD_FACTS TS_JS OUT --controls MANIFEST
"""
import argparse, hashlib, json, shutil, subprocess
from pathlib import Path

parser=argparse.ArgumentParser(description=__doc__)
for name in ('base','head','facts','ts','out'):parser.add_argument(name,type=Path)
g=parser.add_mutually_exclusive_group(required=True);g.add_argument('--public',action='store_true');g.add_argument('--controls',type=Path)
parser.add_argument('--reuse-base',type=Path,help='public-only retained main streams; rehash original census reads before reuse')
a=parser.parse_args();p=Path(__file__).resolve().parent;a.out.mkdir(parents=True,exist_ok=False)
if a.reuse_base and not a.public:parser.error('--reuse-base is public-only')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
binding={k:{'path':str(getattr(a,k).resolve()),'sha256':sha(getattr(a,k))} for k in ('base','head','facts','ts')}
if a.public:
 h=Path.home();cases=[{'name':n,'root':str(r)} for n,r in {'X':h/'prism-evidence/inputs/excalidraw-0642e72c/source','installed-X':h/'prism-evidence/inputs/excalidraw-0642e72c-installed/source','R':h/'code/bench-repos/ruff/playground','T':h/'code/bench-repos/TypeScript/src'}.items()]
else:cases=json.loads(a.controls.read_text())
results=[]
for c in cases:
 out=a.out/c['name'];out.mkdir();root=c['root']
 def run(args,file):
  with (out/file).open('w') as o,(out/(file+'.stderr')).open('w') as err:subprocess.run([str(x) for x in args],stdout=o,stderr=err,check=True,timeout=1800)
 if a.reuse_base:
  original=a.reuse_base/c['name']/'base-sites.jsonl'
  census=json.loads((a.reuse_base.parent/c['name']/'census.json').read_text())
  for f,digest in census['inputs'].items():assert sha(Path(f))==digest, 'retained base native/source inputs drift'
  shutil.copy2(original,out/'base-sites.jsonl')
  (out/'base-reuse.json').write_text(json.dumps({'source':str(original.resolve()),'sha256':sha(original),'native_read_hashes_rechecked':len(census['inputs'])},indent=2)+'\n')
 else:run([a.base,'nav','--no-cache','call-stats','--repo',root,'--dump-sites'],'base-sites.jsonl')
 run([a.head,'nav','--no-cache','call-stats','--repo',root,'--dump-sites'],'head-sites.jsonl')
 run([a.facts,root],'facts.jsonl')
 run(['node',p/'compare.cjs',a.ts,root,out/'base-sites.jsonl',out/'head-sites.jsonl',out/'facts.jsonl',out],'compare.log')
 summary=json.loads((out/'comparison.json').read_text())['summary']
 if not a.public:
  run(['node',p/'census.cjs',a.ts,root,out/'base-sites.jsonl',out/'facts.jsonl',out],'census.log')
  d=json.loads((out/'census.json').read_text());r=next(r for r in d['records'] if r['writer']==c['writer'] and r['specifier']==c['specifier'])
  assert r['canonical']==c['native_target'],(c,r['canonical'])
  assert bool(summary['changed'])==c['admit'],(c,summary)
 results.append({'corpus':c['name'],**summary});print(c['name'],json.dumps(summary),flush=True)
 assert all(sha(Path(v['path']))==v['sha256'] for v in binding.values()), 'binary/oracle drift'
 (a.out/'summary.json').write_text(json.dumps({'binding':binding,'results':results},indent=2)+'\n')
