#!/usr/bin/env python3
"""Reviewer oracle/census repros and same-environment E7/E8 controls.
All fixtures are source text only; never executed. Rust guard RED/GREEN is
captured separately by cargo test --lib js_param_defs.
"""
import json,subprocess
from pathlib import Path
ROOT=Path.cwd();P=ROOT/'docs/superpowers/plans/2026-10-06-js-param-defs/probes'
E=Path.home()/'prism-evidence/js-param-defs/repair-r1';TS=Path.home()/'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
B={'base':E/'bin/prism-base-c8de720b','proto':E/'bin/prism-proto-1b2dfdc9','head':E/'bin/prism-head-r1'}
D={'base':E/'bin/prism-base-bytes','proto':E/'bin/prism-proto-bytes','head':E/'bin/prism-head-r1-bytes'}
C=Path.home()/'prism-evidence/js-param-defs/cache';OUT=E/'reviewer-controls';OUT.mkdir(exist_ok=True)

def run(cmd,out):
 with out.open('wb') as stream:
  proc=subprocess.run([str(x) for x in cmd],stdout=stream)
 if proc.returncode:
  # A refused seed is an observed JSON error, not success or absence of evidence.
  data=json.loads(out.read_text())
  if 'error' not in data: raise RuntimeError((cmd,proc.returncode,data))
 return out
oldadj=OUT/'old-adjudicate.cjs';oldadj.write_bytes(subprocess.check_output(['git','show','6b0be4ff:docs/superpowers/plans/2026-10-06-js-param-defs/probes/adjudicate.cjs']))
oldcensus=OUT/'old-census.cjs';oldcensus.write_bytes(subprocess.check_output(['git','show','6b0be4ff:docs/superpowers/plans/2026-10-06-js-param-defs/probes/census.cjs']))
summary={}
for name,source in [('same_line_owners',r'''function outer(...x) { return function (\u0078) {
 sink(x);
}; }'''),('same_line_uses',r'''const outer = x => {
 { let \u0078 = other; sink(x); } use(x);
};''')]:
 repo=OUT/name;repo.mkdir(exist_ok=True);(repo/'case.ts').write_text(source+'\n')
 rows=run([D['proto'],repo],OUT/f'{name}.proto.bytes.jsonl')
 changed=[{'class':'ADDED','row':{k:v for k,v in r.items() if k not in ['confidence','doubt','kill_line']},'head_label':{k:r.get(k) for k in ['confidence','doubt','kill_line']}} for r in map(json.loads,rows.read_text().splitlines()) if r['from']['parameter'] and r['from']['path']['base']=='x']
 f=OUT/f'{name}.changed.jsonl';f.write_text(''.join(json.dumps(r)+'\n' for r in changed))
 assert changed,name
 for label,oracle in [('old',oldadj),('byte',P/'adjudicate.cjs')]:
  run(['node',oracle,TS,repo,f,OUT/f'{name}.{label}.json','--details',OUT/f'{name}.{label}.details.jsonl'],OUT/f'{name}.{label}.stdout')
 old=json.loads((OUT/f'{name}.old.json').read_text());byte=json.loads((OUT/f'{name}.byte.json').read_text())
 assert old.get('ADDED|def->use|CORRECT',0)>0,(name,old)
 assert byte.get('ADDED|def->use|WRONG',0)>0,(name,byte)
 summary[name]={'old':old,'byte':byte}
repo=OUT/'hidden';repo.mkdir(exist_ok=True);(repo/'.sample.js').write_text('const kept = x => use(x);\n')
for label,oracle in [('old',oldcensus),('head',P/'census.cjs')]:run(['node',oracle,TS,repo,OUT/f'hidden.{label}.json'],OUT/f'hidden.{label}.stdout')
a=json.loads((OUT/'hidden.old.json').read_text());b=json.loads((OUT/'hidden.head.json').read_text());assert a['files']==0 and b['files']==1
summary['hidden']={'old_files':a['files'],'head_files':b['files']}
for ext in ['js','ts','tsx']:
 for shape,sig in [('plain','x'),('rest','...x')]:
  repo=OUT/f'unreachable-{ext}-{shape}';repo.mkdir(exist_ok=True);(repo/f'case.{ext}').write_text(f'function outer({sig}) {{\n return;\n sink(x);\n}}\n')
  for side in ['base','head']:
   run([B[side],'nav','--cache-dir',C,'dfg-stats','--repo',repo,'--edges'],OUT/f'unreachable-{ext}-{shape}.{side}.wire.jsonl')
   run([B[side],'nav','--cache-dir',C,'taint-reaches','--repo',repo,'--source',f'case.{ext}:1','--sink',f'case.{ext}:3','--format','json'],OUT/f'unreachable-{ext}-{shape}.{side}.trace.json')
 for key in ['base','head']:
  plain=(OUT/f'unreachable-{ext}-plain.{key}.wire.jsonl').read_bytes();head=(OUT/f'unreachable-{ext}-rest.head.wire.jsonl').read_bytes();assert plain==head,ext
 summary[f'unreachable-{ext}']='head rest wire equals same-environment base/head plain wire; trace artifacts retained'
# E7: checker uses exact key bytes; same wrong key mechanism on plain base.
for shape,sig in [('plain','tokens: number[]'),('rest','...tokens: number[]')]:
 repo=OUT/f'property-key-{shape}';repo.mkdir(exist_ok=True);(repo/'case.ts').write_text(f'function outer({sig}) {{\n return {{tokens: tokens.some(x => x)}};\n}}\n')
 for side in ['base','head']:
  run([B[side],'nav','--cache-dir',C,'dfg-stats','--repo',repo,'--edges'],OUT/f'property-key-{shape}.{side}.wire.jsonl')
for shape,side in [('plain','base'),('rest','head')]:
 repo=OUT/f'property-key-{shape}'
 raw=run([D[side],repo],OUT/f'property-key-{shape}.{side}.bytes.jsonl')
 changed=[{'class':'CONTROL','row':{k:v for k,v in r.items() if k not in ['confidence','doubt','kill_line']},'head_label':{k:r.get(k) for k in ['confidence','doubt','kill_line']}} for r in map(json.loads,raw.read_text().splitlines())]
 f=OUT/f'property-key-{shape}.changed.jsonl';f.write_text(''.join(json.dumps(r)+'\n' for r in changed))
 run(['node',P/'adjudicate.cjs',TS,repo,f,OUT/f'property-key-{shape}.adj.json'],OUT/f'property-key-{shape}.stdout')
 adj=json.loads((OUT/f'property-key-{shape}.adj.json').read_text());assert adj.get('CONTROL|def->use|WRONG')==1,(shape,adj)
assert (OUT/'property-key-plain.base.wire.jsonl').read_bytes()==(OUT/'property-key-rest.head.wire.jsonl').read_bytes()
summary['E7']='head rest property-key wire equals same-environment plain-formal base wire'
(OUT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary))

# Checker branch controls: byte identity, owner uncertainty/mismatch, UTF-8.
repo=OUT/'oracle-branches';repo.mkdir(exist_ok=True)
source='export /* trivia */ function outer(x) {\n const astral = "\U0001f600";\n use(x);\n}\n'
(repo/'case.ts').write_text(source)
raw=run([D['head'],repo],OUT/'oracle-branches.bytes.jsonl')
rows=[r for r in map(json.loads,raw.read_text().splitlines()) if r['from']['parameter'] and r['from']['path']['base']=='x']
assert len(rows)==1,rows
r=rows[0]
import copy
variants=[]
for label in ['correct','ambiguous','wrong_owner','wrong_bytes','missing_bytes','collapsed_unanimous']:
 v=copy.deepcopy(r)
 if label=='ambiguous':v['from']['owner']={'ambiguous':2}
 if label=='wrong_owner':v['from']['owner']['start_byte']+=1
 if label=='wrong_bytes':v['to']['start_byte']+=1;v['to']['end_byte']+=1
 if label=='missing_bytes':del v['to']['start_byte']
 if label=='collapsed_unanimous':v['to']['start_byte']=v['to']['end_byte']=0
 variants.append({'class':label,'row':{k:v for k,v in v.items() if k not in ['confidence','doubt','kill_line']},'head_label':{k:v.get(k) for k in ['confidence','doubt','kill_line']}})
f=OUT/'oracle-branches.changed.jsonl';f.write_text(''.join(json.dumps(v)+'\n' for v in variants))
run(['node',P/'adjudicate.cjs',TS,repo,f,OUT/'oracle-branches.json','--details',OUT/'oracle-branches.details.jsonl'],OUT/'oracle-branches.stdout')
a=json.loads((OUT/'oracle-branches.json').read_text())
for label,verdict in [('correct','CORRECT'),('ambiguous','UNDECIDED'),('wrong_owner','WRONG'),('wrong_bytes','WRONG'),('missing_bytes','UNDECIDED'),('collapsed_unanimous','CORRECT')]:
 assert a.get(f'{label}|def->use|{verdict}')==1,(label,a)
summary['oracle_branches']=a
(OUT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')

repo=OUT/'collapsed-mixed';repo.mkdir(exist_ok=True)
(repo/'case.ts').write_text('function outer(x) {\n { let x=other; use(x); } use(x);\n}\n')
raw=run([D['base'],repo],OUT/'collapsed-mixed.bytes.jsonl')
rows=[r for r in map(json.loads,raw.read_text().splitlines()) if r['from']['parameter'] and r['from']['path']['base']=='x']
assert rows
v=rows[0];v['to']['start_byte']=v['to']['end_byte']=0;v['to']['line']=2
c={'class':'CONTROL','row':{k:v for k,v in v.items() if k not in ['confidence','doubt','kill_line']},'head_label':{k:v.get(k) for k in ['confidence','doubt','kill_line']}}
f=OUT/'collapsed-mixed.changed.jsonl';f.write_text(json.dumps(c)+'\n')
run(['node',P/'adjudicate.cjs',TS,repo,f,OUT/'collapsed-mixed.adj.json','--details',OUT/'collapsed-mixed.details.jsonl'],OUT/'collapsed-mixed.stdout')
a=json.loads((OUT/'collapsed-mixed.adj.json').read_text());assert a.get('CONTROL|def->use|UNDECIDED|collapsed_mixed_bindings')==1,a
summary['collapsed_mixed']=a
(OUT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')

# Resume a genuinely interrupted wire producer: old condition must not pass.
import importlib.util
current=(P/'measure-bytes.py').read_text()
fixed="if public and side!='proto' and all(status['operations'].get(kind,{}).get('exit')==0 for kind in ['bytes','sites']) and not (status['operations'].get('wire-cli',{}).get('exit')==0 and (dest/f'{tag}.{side}.wire-cli.jsonl').exists()):"
old="if public and side!='proto' and all(v['exit']==0 for v in status['operations'].values()):"
assert fixed in current
resume={}
for label,code in [('old',current.replace(fixed,old)),('fixed',current)]:
 module_file=OUT/f'resume-{label}.py';module_file.write_text(code)
 spec=importlib.util.spec_from_file_location(f'resume_{label}',module_file);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
 out=OUT/f'resume-{label}';out.mkdir(exist_ok=True);per=out/'per';per.mkdir(exist_ok=True)
 for kind,source in [('bytes',E/'controller-smoke-positive/base.bytes.jsonl'),('sites',E/'controller-smoke-positive/base.sites.jsonl')]:
  (per/f'fixture.head.{kind}.jsonl').write_bytes(source.read_bytes())
 (per/'fixture.head.wire-cli.jsonl').write_text('')
 (per/'fixture.head.status.json').write_text(json.dumps({'name':'fixture','side':'head','operations':{'bytes':{'exit':0},'sites':{'exit':0},'wire-cli':{'exit':-2}}}))
 result=m.capture(('fixture',OUT/'oracle-branches','head',B['head'],D['head'],out,True,30))
 resume[label]=result['operations']['wire-cli']['exit']
 if label=='fixed':
  assert (per/'fixture.head.wire-cli.jsonl').stat().st_size>0
  projection=sorted(m.wire(json.loads(l)) for l in (per/'fixture.head.bytes.jsonl').read_text().splitlines())
  actual=sorted(json.dumps(json.loads(l),sort_keys=True,separators=(',',':')) for l in (per/'fixture.head.wire-cli.jsonl').read_text().splitlines())
  assert projection==actual
assert resume=={'old':-2,'fixed':0},resume
summary['wire_resume']=resume
(OUT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')

# Physical byte offsets include a UTF-8 BOM that TypeScript strips on read.
repo=E/'bom-control';repo.mkdir(exist_ok=True)
(repo/'case.ts').write_bytes(b'\xef\xbb\xbffunction outer(x) {\n use(x);\n}\n')
raw=run([D['head'],repo],repo/'rows.jsonl');rows=list(map(json.loads,raw.read_text().splitlines()));assert rows
(repo/'changed.jsonl').write_text(''.join(json.dumps({'class':'CONTROL','row':{k:v for k,v in r.items() if k not in ['confidence','doubt','kill_line']},'head_label':{k:r.get(k) for k in ['confidence','doubt','kill_line']}})+'\n' for r in rows))
run(['node',P/'adjudicate.cjs',TS,repo,repo/'changed.jsonl',repo/'post.json'],repo/'post.stdout')
a=json.loads((repo/'post.json').read_text());assert a.get('CONTROL|def->use|CORRECT')==1,a
summary['bom_physical_bytes']=a
(OUT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')

# A controlled stale checker text cannot be judged against physical byte rows.
proxy=repo/'stale-ts.cjs'
proxy.write_text('const ts=require('+json.dumps(str(TS))+');\nmodule.exports={...ts,createProgram(files,options){const host=ts.createCompilerHost(options);const read=host.readFile;host.readFile=f=>{const text=read(f);return f.endsWith("case.ts")&&text?text.replace("use(x)","use(y)"):text;};return ts.createProgram(files,options,host);}};\n')
run(['node',P/'adjudicate.cjs',proxy,repo,repo/'changed.jsonl',repo/'stale.json'],repo/'stale.stdout')
a=json.loads((repo/'stale.json').read_text());assert a.get('CONTROL|def->use|UNDECIDED|physical_source_text_mismatch')==1,a
summary['stale_checker_text']=a
(OUT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
