#!/usr/bin/env python3
"""Static base/prototype/R1 capture. No corpus execution. Explicit failure intersection.
Each byte dump uses the CLI's loader/scope inputs/CPG. Wire rows are projected from
those same records; call sites remain actual CLI output. Validate that projection
against dfg-stats --edges on public corpora before admitting the records.
"""
import argparse, concurrent.futures, hashlib, json, subprocess, time, os, shutil
from pathlib import Path

EVIDENCE=Path.home()/'prism-evidence/js-param-defs'
CACHE=EVIDENCE/'cache'
PUBLIC={
 'X':Path.home()/'prism-evidence/inputs/excalidraw-0642e72c/source',
 'Xi':Path.home()/'prism-evidence/inputs/excalidraw-0642e72c-installed/source',
 'T':Path.home()/'code/bench-repos/TypeScript/src',
 'R_black':Path.home()/'code/bench-repos/black',
 'G_caddy':Path.home()/'code/bench-repos/caddy',
 'RS_prism':Path.home()/'.local/share/prism/corpora/prism-20c8490591a3/source',
}

def capture(job):
 name,root,side,binary,dumper,out,public,timeout=job
 tag=name.replace('/','__'); dest=out/'per';dest.mkdir(exist_ok=True)
 status_file=dest/f'{tag}.{side}.status.json'
 status=json.loads(status_file.read_text()) if status_file.exists() else {'name':name,'side':side,'root':str(root),'operations':{}}
 for kind,cmd in [('bytes',[str(dumper),str(root)])]+([] if side=='proto' else [('sites',[str(binary),'nav','--cache-dir',str(CACHE),'call-stats','--repo',str(root),'--dump-sites'])]):
  if status['operations'].get(kind,{}).get('exit') == 0 and (dest/f'{tag}.{side}.{kind}.jsonl').exists():continue
  start=time.monotonic();f=dest/f'{tag}.{side}.{kind}.jsonl';err=dest/f'{tag}.{side}.{kind}.stderr'
  with f.open('wb') as stream,err.open('wb') as errors:
   try: code=subprocess.run(cmd,stdout=stream,stderr=errors,timeout=timeout).returncode
   except subprocess.TimeoutExpired:code=124
  status['operations'][kind]={'exit':code,'seconds':round(time.monotonic()-start,3),'bytes':f.stat().st_size}
 if public and side!='proto' and all(status['operations'].get(kind,{}).get('exit')==0 for kind in ['bytes','sites']) and not (status['operations'].get('wire-cli',{}).get('exit')==0 and (dest/f'{tag}.{side}.wire-cli.jsonl').exists()):
  start=time.monotonic();f=dest/f'{tag}.{side}.wire-cli.jsonl'
  with f.open('wb') as stream,(dest/f'{tag}.{side}.wire-cli.stderr').open('wb') as err:
   try:code=subprocess.run([str(binary),'nav','--cache-dir',str(CACHE),'dfg-stats','--repo',str(root),'--edges'],stdout=stream,stderr=err,timeout=timeout).returncode
   except subprocess.TimeoutExpired:code=124
  status['operations']['wire-cli']={'exit':code,'seconds':round(time.monotonic()-start,3),'bytes':f.stat().st_size}
 (dest/f'{tag}.{side}.status.json').write_text(json.dumps(status,indent=2)+'\n')
 return status

def wire(row):
 result={k:v for k,v in row.items() if k!='kill_line' or v is not None}
 for end in ['from','to']:result[end]={k:v for k,v in row[end].items() if k in ['file','line','path','access']}
 return json.dumps(result,sort_keys=True,separators=(',',':'))

def main():
 p=argparse.ArgumentParser();p.add_argument('mode',choices=['public','secbench']);p.add_argument('--out',type=Path,required=True)
 for side in ['base','head','proto']:
  p.add_argument('--'+side,type=Path,required=True);p.add_argument('--'+side+'-bytes',type=Path,required=True)
 p.add_argument('--timeout',type=int,default=300);p.add_argument('--reuse-from',type=Path);p.add_argument('--jobs',type=int,default=4);a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
 roots=PUBLIC if a.mode=='public' else {f"{e['class']}/{e['entry']}":Path.home()/'prism-evidence/inputs/secbench-pkgs'/e['class']/e['entry']/'src/package' for e in json.loads((Path.home()/'prism-evidence/inputs/secbench-pkgs/manifest.json').read_text())['entries'] if e['status']=='ok'}
 binding={'inputs':{str(v.resolve()):hashlib.sha256(v.read_bytes()).hexdigest() for k,v in vars(a).items() if k in ['base','head','proto','base_bytes','head_bytes','proto_bytes']},'roots':{k:str(v) for k,v in roots.items()},'jobs':a.jobs,'timeout':a.timeout}
 (a.out/'binding.json').write_text(json.dumps(binding,indent=2)+'\n')
 if a.reuse_from:
  old=json.loads((a.reuse_from/'binding.json').read_text())['inputs'];(a.out/'per').mkdir(exist_ok=True)
  for name in roots:
   tag=name.replace('/','__')
   for side in ['base','proto']:
    sha=hashlib.sha256(getattr(a,side+'_bytes').read_bytes()).hexdigest()
    status=a.reuse_from/'per'/f'{tag}.{side}.status.json'
    if sha not in old.values() or not status.exists():continue
    record=json.loads(status.read_text());op=record['operations'].get('bytes',{})
    if op.get('exit')!=0:continue
    source=a.reuse_from/'per'/f'{tag}.{side}.bytes.jsonl';dest=a.out/'per'/source.name
    if dest.exists():continue
    os.link(source,dest)
    record['operations']={'bytes':op};record['reused_from']=str(source)
    (a.out/'per'/status.name).write_text(json.dumps(record,indent=2)+'\n')
 jobs=[(n,r,s,getattr(a,s),getattr(a,s+'_bytes'),a.out,a.mode=='public',a.timeout) for n,r in roots.items() for s in ['base','proto','head']]
 statuses=[]
 with concurrent.futures.ThreadPoolExecutor(a.jobs) as pool:
  for status in pool.map(capture,jobs):
   statuses.append(status)
   with (a.out/'status.jsonl').open('a') as out:out.write(json.dumps(status)+'\n')
 failed={s['name'] for s in statuses if s['side'] in ['base','head'] and any(v['exit'] for v in s['operations'].values())}
 # Both sides excluded for ANY failed required producer, even with partial output.
 (a.out/'excluded.json').write_text(json.dumps([s for s in statuses if s['name'] in failed],indent=2)+'\n')
 summary={'roots':len(roots),'included':len(roots)-len(failed),'excluded':sorted(failed),'wire_projection':{},'call_sites_differing':[]}
 for side in ['base','proto','head']:
  allrows=[];allwire=[]
  for name in roots:
   if name in failed:continue
   tag=name.replace('/','__');f=a.out/'per'/f'{tag}.{side}.bytes.jsonl'
   if side=='proto' and any(v['exit'] for s in statuses if s['name']==name and s['side']==side for v in s['operations'].values()):continue
   rows=[json.loads(l) for l in f.read_text().splitlines() if l.strip()]
   projection=sorted(wire(r) for r in rows)
   if a.mode=='public' and side!='proto':
    actual=sorted(json.dumps(json.loads(l),sort_keys=True,separators=(',',':')) for l in (a.out/'per'/f'{tag}.{side}.wire-cli.jsonl').read_text().splitlines() if l.strip())
    summary['wire_projection'][f'{name}.{side}']=projection==actual
   if a.mode=='secbench':
    for r in rows:
     for end in ['from','to']:r[end]['file']=f"{name}/src/package/{r[end]['file']}"
   for r in rows:r['corpus']=name if a.mode=='public' else 'SecBench'
   allrows.extend(json.dumps(r,sort_keys=True,separators=(',',':')) for r in rows)
   allwire.extend(wire(r) for r in rows)
   if a.mode=='public':
    (a.out/f'{name}.{side}.bytes.jsonl').write_text(''.join(json.dumps(r,sort_keys=True,separators=(',',':'))+'\n' for r in rows))
    (a.out/f'{name}.{side}.wire.jsonl').write_text(''.join(r+'\n' for r in projection))
  if a.mode=='secbench':
   (a.out/f'{side}.bytes.jsonl').write_text(''.join(r+'\n' for r in sorted(allrows)))
   (a.out/f'{side}.wire.jsonl').write_text(''.join(r+'\n' for r in sorted(allwire)))
 for name in roots:
  if name in failed:continue
  tag=name.replace('/','__')
  if (a.out/'per'/f'{tag}.base.sites.jsonl').read_bytes()!=(a.out/'per'/f'{tag}.head.sites.jsonl').read_bytes():summary['call_sites_differing'].append(name)
 (a.out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary))
if __name__=='__main__':main()
