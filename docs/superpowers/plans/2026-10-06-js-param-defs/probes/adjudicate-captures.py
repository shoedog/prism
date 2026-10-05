#!/usr/bin/env python3
"""Adjudicate admitted capture pairs independently per corpus/package. Static only.
Failed base/head producers exclude the package on both sides. Prototype cost uses
only successful triples and has its own excluded list; missing prototype rows
are never counted as lost. Evidence stays private; stdout contains counts only.
"""
import argparse, collections, json, subprocess, filecmp
from pathlib import Path
from importlib.machinery import SourceFileLoader
P=Path(__file__).resolve().parent
TS=Path.home()/'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'

def run(cmd):
 r=subprocess.run([str(x) for x in cmd],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 if r.returncode:raise RuntimeError((cmd,r.returncode,r.stderr.decode()[-2000:]))

def good(status):return status and all(v['exit']==0 for v in status['operations'].values())

def main():
 ap=argparse.ArgumentParser();ap.add_argument('capture',type=Path);ap.add_argument('--only',nargs='*');a=ap.parse_args()
 binding=json.loads((a.capture/'binding.json').read_text());roots=binding['roots'];out=a.capture/'adjudication';out.mkdir(exist_ok=True)
 summary={};bad=[];costbad=[];aggregates=collections.defaultdict(collections.Counter)
 for name,root in roots.items():
  if a.only and name not in a.only:continue
  tag=name.replace('/','__');per=a.capture/'per'
  statuses={s:json.loads(f.read_text()) if (f:=per/f'{tag}.{s}.status.json').exists() else None for s in ['base','proto','head']}
  if not all(good(statuses[s]) for s in ['base','head']):bad.append(name);continue
  result={'call_sites_identical':(per/f'{tag}.base.sites.jsonl').read_bytes()==(per/f'{tag}.head.sites.jsonl').read_bytes()}
  for label,left in [('base','base'),('cost','proto')]:
   if not good(statuses[left]):costbad.append(name);continue
   prefix=out/f'{tag}.{label}';diff=Path(str(prefix)+'.diff.json');changed=Path(str(prefix)+'.changed.jsonl');adj=Path(str(prefix)+'.adj.json');details=Path(str(prefix)+'.details.jsonl')
   bfile=per/f'{tag}.{left}.bytes.jsonl';hfile=per/f'{tag}.head.bytes.jsonl'
   if filecmp.cmp(bfile,hfile,shallow=False):
    with bfile.open('rb') as stream:
     count=sum(chunk.count(b'\n') for chunk in iter(lambda:stream.read(1024*1024),b''))
    diff.write_text(json.dumps({'base_rows':count,'head_rows':count,'RE-OWNED':'owner and byte identity included; owner replacements appear as LOST plus ADDED'})+'\n');changed.write_text('')
   else:run(['python3',P/'rowdiff.py',bfile,hfile,diff,'--rows',changed])
   if not changed.stat().st_size or name in ['R_black','G_caddy','RS_prism']:
    verdict={};adj.write_text('{}\n');details.write_text('')
   else:
    run(['node','--max-old-space-size=8000',P/'adjudicate.cjs',TS,root,changed,adj,'--details',details]);verdict=json.loads(adj.read_text())
   d=json.loads(diff.read_text());result[label]={'diff':d,'checker':verdict}
   aggregates[label].update(verdict)
  summary[name]=result
 result={'corpora':summary,'excluded':bad,'cost_excluded':costbad,'aggregate_checker':{k:dict(v) for k,v in aggregates.items()}}
 (out/'summary.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
if __name__=='__main__':main()
