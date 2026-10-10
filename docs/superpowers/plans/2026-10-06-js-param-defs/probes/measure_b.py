#!/usr/bin/env python3
"""PR-B static base/head capture with build time and peak RSS. No corpus execution.
Per corpus and side: byte rows (dumper = full CPG build, timed with direct wait4),
CLI wire rows (`nav --cache-dir <lane> dfg-stats --edges`, timed) and call sites (`call-stats --dump-sites`).
Usage: measure_b.py OUT --base BIN --base-bytes BIN --head BIN --head-bytes BIN [--only NAME ...] [--secbench]
A failed producer on either side excludes that corpus from both aggregates.
"""
import argparse, hashlib, json, re, subprocess, time
from pathlib import Path
H=Path.home()
from timed_process import timed_process
PUBLIC={'X':H/'prism-evidence/inputs/excalidraw-0642e72c/source','Xi':H/'prism-evidence/inputs/excalidraw-0642e72c-installed/source',
 'T':H/'code/bench-repos/TypeScript/src','R_black':H/'code/bench-repos/black','G_caddy':H/'code/bench-repos/caddy',
 'RS_prism':H/'.local/share/prism/corpora/prism-20c8490591a3/source'}
def timed(cmd,out,err,timeout):
  with open(out,'wb') as o,open(err,'wb') as e:
    return timed_process(cmd,o,e,timeout)
def main():
  ap=argparse.ArgumentParser();ap.add_argument('out',type=Path)
  for s in ['base','head']:ap.add_argument('--'+s,type=Path,required=True);ap.add_argument('--'+s+'-bytes',type=Path,required=True)
  ap.add_argument('--only',nargs='*');ap.add_argument('--secbench',action='store_true');ap.add_argument('--timeout',type=int,default=600)
  ap.add_argument('--jobs',type=int,default=1)
  ap.add_argument('--reuse-base',type=Path,help='earlier run dir: hardlink its successful base outputs (same base binaries, checked by sha)');a=ap.parse_args()
  roots=dict(PUBLIC)
  if a.secbench:
    m=json.loads((H/'prism-evidence/inputs/secbench-pkgs/manifest.json').read_text())
    roots={f"{e['class']}/{e['entry']}":H/'prism-evidence/inputs/secbench-pkgs'/e['class']/e['entry']/'src/package' for e in m['entries'] if e['status']=='ok'}
  if a.only: roots={k:v for k,v in roots.items() if k in a.only}
  a.out.mkdir(parents=True,exist_ok=True);(a.out/'per').mkdir(exist_ok=True)
  sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
  (a.out/'binding.json').write_text(json.dumps({k:{'path':str(getattr(a,k)),'sha256':sha(getattr(a,k))} for k in ['base','base_bytes','head','head_bytes']},indent=1)+'\n')
  status={}
  def one(item):
    name,root=item
    tag=name.replace('/','__');st={}
    for side in ['base','head']:
      binp,dump=getattr(a,side),getattr(a,side+'_bytes');p=a.out/'per'/f'{tag}.{side}'
      if side=='base' and a.reuse_base:
        old=json.loads((a.reuse_base/'binding.json').read_text())
        same=all(old[k]['sha256']==sha(getattr(a,k)) for k in ['base','base_bytes'])
        prev={}
        for l in (a.reuse_base/'status.jsonl').read_text().splitlines():
          r=json.loads(l)
          if r['name']==name:prev=r
        kinds=['bytes','sites']+([] if a.secbench else ['wire'])
        if same and prev and all(prev['base'].get(k,{}).get('exit')==0 for k in kinds):
          for k in kinds:
            import os
            src=a.reuse_base/'per'/f'{tag}.base.{k}.jsonl'
            os.link(src,f'{p}.{k}.jsonl')
          st['base']={**prev['base'],'reused_from':str(a.reuse_base)}
          continue
      st[side]={'bytes':timed([dump,root],f'{p}.bytes.raw',f'{p}.bytes.err',a.timeout)}
      if not a.secbench:
        st[side]['wire']=timed([binp,'nav','--cache-dir',str(H/'prism-evidence/js-param-defs/cache'),'dfg-stats','--repo',root,'--edges'],f'{p}.wire.raw',f'{p}.wire.err',a.timeout)
      st[side]['sites']=timed([binp,'nav','--cache-dir',str(H/'prism-evidence/js-param-defs/cache'),'call-stats','--repo',root,'--dump-sites'],f'{p}.sites.jsonl',f'{p}.sites.err',a.timeout)
      for k in ['bytes','wire']:
        f=Path(f'{p}.{k}.raw')
        if f.exists():
          rows=sorted(l for l in f.read_text().splitlines() if l.strip());Path(f'{p}.{k}.jsonl').write_text(''.join(r+'\n' for r in rows));f.unlink()
    st['ok']=all(v['exit']==0 for s in ['base','head'] for v in st[s].values() if isinstance(v,dict))
    if st['ok']:
      st['sites_identical']=(a.out/'per'/f'{tag}.base.sites.jsonl').read_bytes()==(a.out/'per'/f'{tag}.head.sites.jsonl').read_bytes()
      st['bytes_identical']=(a.out/'per'/f'{tag}.base.bytes.jsonl').read_bytes()==(a.out/'per'/f'{tag}.head.bytes.jsonl').read_bytes()
      if not a.secbench:
        st['wire_identical']=(a.out/'per'/f'{tag}.base.wire.jsonl').read_bytes()==(a.out/'per'/f'{tag}.head.wire.jsonl').read_bytes()
        # projection check: byte rows projected to wire == CLI rows
        for side in ['base','head']:
          proj=[]
          for l in (a.out/'per'/f'{tag}.{side}.bytes.jsonl').read_text().splitlines():
            r=json.loads(l)
            if r.get('kill_line') is None:r.pop('kill_line',None)
            for e in ['from','to']:r[e]={k:v for k,v in r[e].items() if k in ('file','line','path','access')}
            proj.append(json.dumps(r,sort_keys=True))
          cli=[json.dumps(json.loads(l),sort_keys=True) for l in (a.out/'per'/f'{tag}.{side}.wire.jsonl').read_text().splitlines()]
          st[f'{side}_projection_equal']=sorted(proj)==sorted(cli)
    return name,st
  import concurrent.futures
  with concurrent.futures.ThreadPoolExecutor(a.jobs) as pool:
    for name,st in pool.map(one,roots.items()):
      status[name]=st
      with open(a.out/'status.jsonl','a') as f:f.write(json.dumps({'name':name,**st})+'\n')
  (a.out/'status.json').write_text(json.dumps(status,indent=1)+'\n')
  print(json.dumps({k:{'ok':v['ok'],'sites_identical':v.get('sites_identical'),'bytes_identical':v.get('bytes_identical')} for k,v in status.items()}) if not a.secbench else json.dumps({'roots':len(status),'failed':[k for k,v in status.items() if not v['ok']],'sites_differ':[k for k,v in status.items() if v['ok'] and not v['sites_identical']]}))
if __name__=='__main__':main()
