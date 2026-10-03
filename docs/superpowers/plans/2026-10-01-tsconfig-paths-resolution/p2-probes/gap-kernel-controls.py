"""Public preservation shapes outside the callable bucket; exact production kernel.

Usage: python3 gap-kernel-controls.py GAP_DRIVER TS_JS NEW_OUT
"""
import json, subprocess, sys
from pathlib import Path
driver,ts=map(lambda p:str(Path(p).resolve()),sys.argv[1:3]);out=Path(sys.argv[3]).resolve()
assert not out.exists();root=out/'source';root.mkdir(parents=True)
requests=[];cases=[]
for grammar in ['jsx','tsx']:
    for shape in ['declaration-priority-negative','unindexed-package-negative','allow-off-negative',
                  'missing-negative','ordered-substitution-negative','empty-cycle-negative']:
        d=root/(grammar+'-'+shape);d.mkdir()
        options={'moduleResolution':'node10','allowJs':shape!='allow-off-negative','paths':{'@lib':['./entry.ts']}}
        if shape=='ordered-substitution-negative':options['paths']['@lib']=['./missing','./entry.ts']
        spec='./real.js';extra={};gate='PASS'
        if shape=='declaration-priority-negative':
            spec='./real.service';extra={'real.service.js':'export function real() { return 1; }\n',
              'real.d.service.ts':'export declare function real(): number;\n'};gate='UNKNOWN_SUFFIX_DECLARATION_COMPETITION'
        if shape=='unindexed-package-negative':
            spec='./node_modules/pkg/real.js';extra={'node_modules/pkg/real.js':'export function real() { return 1; }\n'};gate='PATH_OPAQUE'
        if shape=='missing-negative':spec='./missing';gate='CANDIDATE_ABSENT'
        if shape=='allow-off-negative':gate='ALLOW_JS_OFF'
        if shape=='empty-cycle-negative':spec='./entry.ts'
        payload={'tsconfig.json':json.dumps({'compilerOptions':options,'include':['**/*']})+'\n',
          'entry.ts':f"export {{ real }} from '{spec}';\n",
          'real.js':'export function real() { return 1; }\n',
          'app.'+grammar:"import { real as Real } from '@lib';\nexport function run() { Real(); }\n",**extra}
        for file,text in payload.items():
            p=d/file;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
        file=d.name+'/app.'+grammar
        # The driver uses only file/local/spec for kernel requests. These are
        # deliberately kernel seam keys, not certified call-site coordinates.
        requests.append({'key':[file,'kernel_seam',0,file,0,0,'Real'],'local':'Real','specifier':'@lib'})
        cases.append({'case':d.name,'shape':shape,'gate':gate,'specifier':spec})
(out/'requests.json').write_text(json.dumps(requests))
with (out/'sidecar.json').open('w') as f:
    subprocess.run([driver,str(root),str(out/'requests.json')],stdout=f,stderr=subprocess.DEVNULL,check=True)
side=json.loads((out/'sidecar.json').read_text());checks=[]
for c in cases:
    entry=next(r for r in side['entries'] if r['key'][0].split('/')[0]==c['case'])
    if c['shape']=='ordered-substitution-negative':
        assert entry['explanation']['gate']=='ENTRY_CONFIG_SELECTION'
    else:
        allow=c['shape']!='allow-off-negative'
        hop=next(r for r in side['hops'] if r['from']==c['case']+'/entry.ts' and r['allow_js']==allow)
        assert hop['explanation']['gate']==c['gate'],(c,hop)
    checks.append({'shape':c['shape'],'pass':True})
# Native TS resolver over these simple, independently parsed configs. Output
# contains only categorical outcomes; no synthetic path is printed.
oracle="""
const fs=require('fs'),path=require('path'),ts=require(process.argv[1]),root=process.argv[2];
const cases=JSON.parse(fs.readFileSync(process.argv[3]));let rows=[];
for(const c of cases){const d=path.join(root,c.case),cfg=path.join(d,'tsconfig.json');
 const parsed=ts.getParsedCommandLineOfConfigFile(cfg,{}, {...ts.sys,onUnRecoverableConfigFileDiagnostic(){throw Error('config')}});
 const r=ts.resolveModuleName(c.specifier,path.join(d,'entry.ts'),parsed.options,ts.sys).resolvedModule;
 const outcome=!r?'UNRESOLVED':r.resolvedFileName.endsWith('.d.service.ts')?'DECLARATION':r.resolvedFileName.endsWith('.js')?'JS_SOURCE':'TS_SOURCE';
 rows.push({shape:c.shape,outcome});}
process.stdout.write(JSON.stringify(rows));
"""
(out/'cases.json').write_text(json.dumps(cases))
r=subprocess.run(['node','-e',oracle,ts,str(root),str(out/'cases.json')],capture_output=True,text=True,check=True)
native=json.loads(r.stdout)
for row in native:
    if row['shape']=='declaration-priority-negative':assert row['outcome']=='DECLARATION'
    if row['shape']=='missing-negative':assert row['outcome']=='UNRESOLVED'
    if row['shape']=='unindexed-package-negative':assert row['outcome']=='JS_SOURCE'
summary={'status':'PASS','kernel_preservation_checks':len(checks),'typescript_outcome_checks':len(native),
         'checks':checks,'typescript_outcomes':native,'synthetic_kernel_seams_not_callable_rows':True}
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({'status':'PASS','kernel_preservation_checks':len(checks),'typescript_outcome_checks':len(native)}))
