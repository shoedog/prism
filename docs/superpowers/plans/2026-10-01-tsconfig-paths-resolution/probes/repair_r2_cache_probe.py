"""Repository-wide scanner dependencies: cold/hit/add/edit/remove parity."""
import hashlib,json,subprocess,sys
from pathlib import Path
binary=Path(sys.argv[1]).resolve();out=Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=True);results=[]
for ext in ['jsx','tsx']:
 for folder in ['vendor','build','.hidden','node_modules/@types/legacy']:
  d=out/(folder.replace('/','_')+'-'+ext);root=d/'repo';cache=d/'cache';root.mkdir(parents=True,exist_ok=True)
  def write(name,text):p=root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
  write('tsconfig.json',json.dumps({'compilerOptions':{'moduleResolution':'node','allowJs':True,'paths':{'@lib':['lib/real.tsx']}},'files':['app.'+ext],'include':[]}))
  write('app.'+ext,'import {real as picked} from "@lib"; export function run(){picked();}')
  write('lib/real.tsx','export function real(){return 1;}')
  path=folder+'/ambient.d.ts';write(path,'export interface Empty {}')
  def query(label,cached):
   p=subprocess.run([str(binary),'nav',*(['--cache-dir',str(cache)] if cached else ['--no-cache']),'call-stats','--repo',str(root),'--dump-sites'],capture_output=True,text=True);assert p.returncode==0,p.stderr
   if cached:(d/(label+'.jsonl')).write_text(p.stdout);(d/(label+'.stderr')).write_text(p.stderr)
   return [json.loads(line) for line in p.stdout.splitlines() if json.loads(line).get('record_kind')=='call_site']
  def state(label,hit):
   cached=query(label,True);fresh=query(label,False);assert cached==fresh,(folder,ext,label)
   picked=[r for r in cached if r['callee_text']=='picked'];assert len(picked)==1
   assert bool(picked[0]['resolved_targets'])==hit,(label,picked)
   return cached
  cold=state('cold',True);assert state('hit',True)==cold
  write(path,'declare module "@lib" {export function real(): number;}');state('added',False)
  blobs=list(cache.rglob('cpg-cache.bin'));assert len(blobs)==1;blob=blobs[0]
  stamp=lambda:(blob.stat().st_mtime_ns,hashlib.sha256(blob.read_bytes()).hexdigest())
  before=stamp();write(path,'declare module "@lib" {export function real(): string;}');state('same-pattern-edit',False);assert stamp()!=before,'ambient bytes must invalidate even when names are identical'
  write(path,'declare module "@other" {}');state('nonmatch-edit',True)
  (root/path).unlink();assert state('removed',True)==cold
  before=stamp();write(path,'export interface Empty {}');assert state('read-occupancy-added',True)==cold;assert stamp()!=before,'every read must be a dependency'
  (root/path).unlink();assert state('read-occupancy-removed',True)==cold
  results.append({'folder':folder,'grammar':ext,'states':8,'parity':True,'same_pattern_hash_invalidates':True,'read_occupancy_invalidates':True})
(out/'summary.json').write_text(json.dumps(results,indent=2));print(json.dumps({'cases':len(results),'states':8*len(results),'all_pass':True}))
