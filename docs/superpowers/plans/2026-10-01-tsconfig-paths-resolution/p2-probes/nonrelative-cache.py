"""H1: real CPG cache follows barrel-ancestor declaration and caller paths edits."""
import hashlib, json, subprocess, sys
from pathlib import Path
head, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
out.mkdir(parents=True, exist_ok=False)
records=[]
for grammar in ['jsx','tsx']:
    d=out/grammar; root=d/'source'; cache=d/'cache'
    def write(name,text):
        p=root/name; p.parent.mkdir(parents=True,exist_ok=True); p.write_text(text)
    def config(target):
        return json.dumps({'compilerOptions':{'moduleResolution':'node10','allowJs':True,'baseUrl':'.','paths':{'@lib':['../lib/barrel.ts'],'@x/f':['../src/'+target+'/f']}},'include':['**/*']})
    write('app/tsconfig.json',config('x'))
    write('app/app.'+grammar,"import {real as Real} from '@lib';\nexport function run(){ Real(); return <Real />; }\n")
    write('lib/barrel.ts',"export {real} from '@x/f';\n")
    for target in ['x','y']:
        write('src/'+target+'/f.js','export function real(){ return null; }\n')
    states=[]
    def query(cached):
        p=subprocess.run([str(head),'nav',*(['--cache-dir',str(cache)] if cached else ['--no-cache']),'call-stats','--repo',str(root),'--dump-sites'],capture_output=True,check=True)
        with (d/'queries.stderr').open('ab') as log: log.write(p.stderr)
        return p.stdout
    def state(name,target):
        a=query(True); assert a==query(False),'cached/fresh mismatch: '+name
        rows=[json.loads(l) for l in a.splitlines() if json.loads(l).get('record_kind')=='call_site']
        assert len(rows)==2,rows
        for r in rows:
            ts=r['resolved_targets']
            if target:
                assert len(ts)==1 and ts[0]['function_id']=={'file':'src/'+target+'/f.js','name':'real','start_line':1,'end_line':1},r
                assert ts[0]['confidence']=='exact' and ts[0]['kind']=='import_member',r
            else:
                assert r['drop']=='UnknownName' and not ts,r
        blob,=cache.rglob('cpg-cache.bin')
        stamp=(blob.stat().st_mtime_ns,hashlib.sha256(blob.read_bytes()).hexdigest())
        states.append({'name':name,'target':target,'dump_sha256':hashlib.sha256(a).hexdigest(),'cache_stamp':stamp})
        return a,stamp
    cold,stamp=state('cold','x')
    warm,warm_stamp=state('warm','x'); assert warm==cold and warm_stamp==stamp,'warm cache rewritten'
    declaration='lib/node_modules/@x/f/index.d.ts'
    write(declaration,'export declare function real(): unknown;\n')
    blocked,blocked_stamp=state('barrel-ancestor-declaration-added',None)
    assert blocked!=cold and blocked_stamp!=stamp,'absence dependency not invalidated'
    (root/declaration).unlink()
    restored,restored_stamp=state('declaration-removed','x'); assert restored==cold and restored_stamp!=blocked_stamp
    write('app/tsconfig.json',config('y'))
    moved,moved_stamp=state('caller-paths-edited','y'); assert moved!=restored and moved_stamp!=restored_stamp
    records.append({'grammar':grammar,'states':states,'cached_fresh_parity':True,'warm_hit':True,'declaration_add_remove_invalidated':True,'config_edit_invalidated':True})
summary={'cases':2,'states':10,'sites_per_state':2,'binary_sha256':hashlib.sha256(head.read_bytes()).hexdigest(),'records':records}
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k!='records'}))
