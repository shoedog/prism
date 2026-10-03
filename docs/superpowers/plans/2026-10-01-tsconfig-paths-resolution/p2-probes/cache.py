"""P1-cache -> P2 rebuild, warm hits and JS-hop candidate edit parity."""
import hashlib, json, subprocess, sys
from pathlib import Path
head=Path(sys.argv[1]).resolve();p1=Path.home()/'code/prism-paths-impl/target/repair-r5/head/prism'
out=Path('target/p2-plan/p2-cache').resolve();records=[]
for grammar in ['jsx','tsx']:
 for candidate in ['real.ts','real.d.ts','real.js.ts','middle.d.ts']:
    d=out/(grammar+'-'+candidate);root=d/'source';root.mkdir(parents=True,exist_ok=True);cache=d/'cache'
    def write(name,text):
        p=root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
    write('tsconfig.json',json.dumps({'compilerOptions':{'moduleResolution':'node10','allowJs':True,'paths':{'@lib':['entry.ts']}},'include':['**/*']}))
    write('app.'+grammar,"import {real as picked} from '@lib'; export function run(){picked();}\n")
    write('entry.ts',"export {real} from './lib/middle.js';\n")
    write('lib/middle.js',"export {real} from './real.js';\n")
    write('lib/real.js',"export function real(){return 1;}\nreal.displayName='Real';\n")
    def query(binary,cached):
        p=subprocess.run([str(binary),'nav',*(['--cache-dir',str(cache)] if cached else ['--no-cache']),'call-stats','--repo',str(root),'--dump-sites'],capture_output=True)
        assert p.returncode==0,p.stderr;return p.stdout
    def state(exact):
        a=query(head,True);assert a==query(head,False)
        sites=[json.loads(l) for l in a.splitlines() if json.loads(l).get('record_kind')=='call_site']
        assert len(sites)==1 and bool(sites[0]['resolved_targets'])==exact,sites
        return a
    old=query(p1,True);start=state(True);assert start!=old
    blob,=cache.rglob('cpg-cache.bin')
    stamp=lambda:(blob.stat().st_mtime_ns,hashlib.sha256(blob.read_bytes()).hexdigest())
    before=stamp();assert state(True)==start and stamp()==before
    write('note.txt','unrelated');assert state(True)==start and stamp()==before
    write('lib/'+candidate,'export declare function real(): number;\n' if '.d.' in candidate else 'export function real(){return 2;}\n')
    assert state(False)==old and stamp()!=before
    (root/('lib/'+candidate)).unlink();assert state(True)==start
    records.append({'case':grammar+'-'+candidate,'states':5,'old_cache_rebuilt':True,'warm_and_unrelated_hits':True,'candidate_add_remove_parity':True})
summary={'cases':len(records),'states':len(records)*5,'versions':[106,62],'records':records}
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps({k:v for k,v in summary.items() if k!='records'}))
