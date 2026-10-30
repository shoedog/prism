"""Same-environment base/old-head/repair behavioral regressions; no corpus reads.
Usage: repair_s1b4_r1.py BASE HEAD OUT [OLD_HEAD]
Outputs fixtures, complete dumps, explicit expected/actual rows and RED/GREEN.
"""
from pathlib import Path
import hashlib,json,subprocess,sys
base,head,out=Path(sys.argv[1]),Path(sys.argv[2]),Path(sys.argv[3]).resolve()
old=Path(sys.argv[4]) if len(sys.argv)>4 else None
out.mkdir(parents=True,exist_ok=True)
APP="import * as ns from './lib'; export function run(){return ns.f();}"
LIB="function h(){\n function f(){}\n}\nexport function f(){}"
cases={}
def add(name,files,member='f',mode='base'):
    cases[name]=(files,member,mode)
for i,value in enumerate(['memo(Button)','withRouter(Button)','connect(null)(Button)','(function Button(){})','make()']):
    add(f'opus_w1a_{i}',{'components/index':"export {default as Button} from './Button';",'components/Button':f'function Button(){{return <div/>;}} export default {value};','app':"import * as UI from './components'; export function run(){return <UI.Button/>;}"},'Button')
for i,value in enumerate(['(function f(){})','g=function f(){}','(function f(){}) as any']):
    add(f'opus_w1b_{i}',{'lib':f'function h(){{return function f(){{}};}} export const f={value};','app':APP})
for i,barrel in enumerate(["export {default as f} from './impl';","import f from './impl'; export {f};"]):
    add(f'opus_w1a_forward_{i}',{'lib/index':barrel,'lib/impl':'function f(){} export default make();','app':APP})
add('opus_w1c_import_equals',{'lib/index':"import f=require('./impl'); export {f};",'lib/impl':'function f(){} export = f;','app':APP})
add('opus_w2_revisited_depth',{'lib/index':"export {g as f} from './a'; export {f as h} from './c';",'lib/a':"export {h as g} from './index';",'lib/c':'export function f(){}','app':APP})
for i,app in enumerate(["import * as ns from './lib'; export function run(a=ns.f()){var ns;return a;}",r"import * as ns from './l\u0069b'; export function run(){return ns.f();}"]):
    add(f'opus_w3_{i}',{'lib/index':"export {f} from './impl';",'lib/impl':'function make(){return function f(){};} export const f=make();','lib/other':'export function f(){}','app':app})
add('opus_w4_e7_sibling',{'lib/index':"export {g as f} from './helpers';",'lib/helpers':'function mk(){return function f(){};} export const g=mk();','app':APP.replace("'./lib'","'lib'")},mode='e7')
for name,decl in [('enum','function h(){function f(){}} export enum f {A}; f=o;'),('namespace','export namespace f {export function f(){return 7;}} f=f.f;'),('ambient','function h(){function f(){}} export declare let f:any; f=o;')]:
    add('sol_w3_'+name,{'lib':decl,'app':APP})
for name,other in [('number','export const f=0;'),('namespace',"export * as f from './a';"),('depth',"export * from './c';")]:
    files={'lib':"export * from './a'; export * from './other';",'a':'export function f(){}','other':other,'c':"export * from './d';",'d':'export function f(){}','app':APP}
    add('sol_w1w2_'+name,files)
add('sol_w4_class_cycle',{'lib':"function f(){return 17;} export {f as rootFn}; export {default as f} from './impl';",'impl':"import {rootFn} from './lib'; class g{} g=rootFn; export default g;",'app':APP})
add('sol_w5_namespace_cycle',{'lib':"function f(){return 23;} export namespace N {export const value=f;} export {g as f} from './impl';",'impl':"import {N} from './lib'; export const g=N.value;",'app':"import './impl'; "+APP})
add('self_initializer',{'lib':'function h(){function f(){}} export const f=f(()=>1);','app':APP})
add('mutual_initializer',{'lib':'function h(){function f(){}} const a=b(()=>1),b=a(()=>2); export {a as f};','app':APP})
add('positive_direct',{'lib':LIB,'app':APP},mode='filter')
for name,barrel in [('named',"export {f} from './impl';"),('star',"export * from './impl';"),('forward',"import {f as alias} from './impl'; export {alias as f};")]:
    add('positive_'+name,{'lib/index':barrel,'lib/impl':LIB,'lib/other':'export function f(){}','app':APP},mode='filter')
add('rule4_outside_candidates',{'lib':"function h(){function f(){}} export {f} from './impl';",'impl':'export function f(){}','app':APP})
add('audit_rename_pin',{'origin':'function origin(){} export {origin as item,origin as default};','decoy':'function item(){}','app':"import * as ns from './origin'; function run(){ns.item();}"},'item')
report=[]
def selected(text,member):
    data=[json.loads(line) for line in text.splitlines()]
    rows=[r for r in data if r.get('record_kind')=='call_site' and r['caller']['name']=='run' and r['callee_text']==member]
    assert rows,'empty selection'
    return [{'drop':r.get('drop'),'targets':r['resolved_targets']} for r in rows]
for name,(files,member,mode) in cases.items():
    for ext in ['jsx','tsx']:
        fixture=out/'fixtures'/f'{name}_{ext}';fixture.mkdir(parents=True,exist_ok=True)
        for file,src in files.items():
            p=fixture/(file+'.'+ext);p.parent.mkdir(parents=True,exist_ok=True);p.write_text(src)
        item={'case':name,'grammar':ext,'mode':mode}
        for label,binary in [('base',base),('head',head)]+([('old',old)] if old else []):
            cmd=[str(binary),'nav','--no-cache','call-stats','--repo',str(fixture),'--dump-sites']
            result=subprocess.run(cmd,capture_output=True,text=True,timeout=120)
            (out/(name+'_'+ext+'.'+label+'.jsonl')).write_text(result.stdout)
            (out/(name+'_'+ext+'.'+label+'.stderr')).write_text(result.stderr)
            assert result.returncode==0 and not result.stderr,(name,ext,label,result.stderr)
            item[label]=selected(result.stdout,member)
        expected=json.loads(json.dumps(item['base']))
        if mode=='e7':
            for row in expected:
                for target in row['targets']:target['confidence']='name_only'
        if mode=='filter':
            terminal='lib.'+ext if name=='positive_direct' else 'lib/impl.'+ext
            for row in expected:
                row['targets']=[t for t in row['targets'] if t['function_id']['file']==terminal and t['function_id']['start_line']==4]
                assert len(row['targets'])==1
        item['expected']=expected;item['green']=item['head']==expected
        item['old_red']=item.get('old',expected)!=expected
        report.append(item)
assert all(r['green'] for r in report),[(r['case'],r['grammar'],r['head'],r['expected']) for r in report if not r['green']]
summary={'repositories':len(report),'green':sum(r['green'] for r in report),'old_red':sum(r['old_red'] for r in report),'binaries':{label:hashlib.sha256(binary.read_bytes()).hexdigest() for label,binary in [('base',base),('head',head)]+([('old',old)] if old else [])},'rows':report}
(out/'results.json').write_text(json.dumps(summary,indent=2))
print(json.dumps({k:v for k,v in summary.items() if k!='rows'},sort_keys=True))
