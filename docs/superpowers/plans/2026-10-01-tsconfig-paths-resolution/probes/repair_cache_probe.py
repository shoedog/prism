"""Repair occupancy add/remove cache parity in both grammars; no external inputs."""
import json, subprocess, sys, hashlib
from pathlib import Path
binary=str(Path(sys.argv[1]).resolve()); out=Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=True)
results=[]
for ext in ['jsx','tsx']:
 for label,target,blocker,kind,js in [('dotted','lib/user.service','lib/user.service','file',False),('directory','lib/real','lib/real.d.ts','dir',False),('replacement','lib/user.service','lib/user.d.service.ts','file',False),('replacement-dir','lib/user.multi.service','lib/user.multi.d.service.ts','dir',False),('text-target','lib/name.txt','lib/name.txt','file',False),('ambient','lib/real','types/ambient.d.ts','ambient',False),('package','lib/real','node_modules/utils','dir',True),('types-above','lib/real','../node_modules/@types/utils','dir',True)]:
  outer=out/(label+'-'+ext);root=outer/'repo';root.mkdir(parents=True,exist_ok=True);cache=outer/'cache'
  def write(p,s):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(s)
  target_ext='jsx' if js else ext
  write(root/(target+'.'+target_ext),'export function real() { return 1; }\n')
  write(root/('app.'+ext),'import { real as picked } from "utils/format";\nexport function run() { picked(); }\n')
  write(root/'tsconfig.json',json.dumps({'compilerOptions':{'moduleResolution':'node','allowJs':True,'paths':{'utils/format':[target]}},'include':['**/*']}))
  def query(label,cached):
   proc=subprocess.run([binary,'nav',*(['--cache-dir',str(cache)] if cached else ['--no-cache']),'call-stats','--repo',str(root),'--dump-sites'],capture_output=True,text=True)
   (outer/(label+'.jsonl')).write_text(proc.stdout);(outer/(label+'.stderr')).write_text(proc.stderr);assert proc.returncode==0,proc.stderr
   rows=[json.loads(s) for s in proc.stdout.splitlines()];return [r for r in rows if r.get('record_kind')=='call_site']
  cold=query('cold',True);assert cold==query('cold-fresh',False)
  assert any(r['resolved_targets'] for r in cold if r['callee_text']=='picked')
  b=root/blocker
  if kind=='dir':b.mkdir(parents=True)
  else:write(b,'declare module "utils/format" { export function real(): number; }' if kind=='ambient' else 'occupied')
  added=query('added',True);assert added==query('added-fresh',False),(label,ext,'add')
  assert not any(r['resolved_targets'] for r in added if r['callee_text']=='picked'),(label,ext,'refusal')
  # Warm a separate cache with the occupant present, then remove it.
  cache=outer/'reverse-cache';assert query('reverse-cold',True)==added
  if kind=='dir':b.rmdir()
  else:b.unlink()
  removed=query('removed',True);assert removed==query('removed-fresh',False)==cold,(label,ext,'remove')
  blobs=list(cache.rglob('cpg-cache.bin'));assert len(blobs)==1
  blob=blobs[0];stamp=lambda:(blob.stat().st_mtime_ns,hashlib.sha256(blob.read_bytes()).hexdigest())
  before=stamp();write(root/'unrelated.txt','irrelevant');assert query('text-added',True)==cold;assert before==stamp()
  (root/'unrelated.txt').unlink();assert query('text-removed',True)==cold;assert before==stamp()
  results.append({'case':label,'grammar':ext,'add_parity':True,'remove_parity':True,'text_hits':True})
(out/'summary.json').write_text(json.dumps(results,indent=2));print(json.dumps({'cases':len(results),'directions':2*len(results),'all_pass':True}))
