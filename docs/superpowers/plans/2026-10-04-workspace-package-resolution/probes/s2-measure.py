"""Public-only S2 scratch measurement against retained, source-bound main rows.
Usage: s2-measure.py MAIN_ROWS CENSUS_ROOT S2_BASE S2_PKG BASE_FACTS PKG_FACTS TS_JS NEW_OUT
The supplied +132 reference weights are inherited; this script does not rebuild
the original pre-R3b gain candidate or adopt its refusal rule.
"""
import hashlib, importlib.util, json, os, re, shutil, subprocess, sys
from collections import Counter
from pathlib import Path

rows,census,base,pkg,basefacts,pkgfacts,ts,out=map(Path,sys.argv[1:])
out.mkdir(parents=True,exist_ok=False);here=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('s2_compare',here/'s2-compare.py')
compare=importlib.util.module_from_spec(spec);spec.loader.exec_module(compare)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
binaries={k:{'path':str(p.resolve()),'sha256':sha(p)} for k,p in dict(base=base,pkg=pkg,basefacts=basefacts,pkgfacts=pkgfacts,ts=ts).items()}
roots={'X':Path.home()/'prism-evidence/inputs/excalidraw-0642e72c/source','installed-X':Path.home()/'prism-evidence/inputs/excalidraw-0642e72c-installed/source'}
gains={('packages/excalidraw/data/EditorLocalStorage.ts','EditorLocalStorage'):3,('packages/excalidraw/tests/helpers/api.ts','API'):52,('packages/excalidraw/tests/helpers/ui.ts','Keyboard'):34,('packages/excalidraw/tests/helpers/ui.ts','UI'):43}
summaries={}
for variant,bin,facts in [('s2-base',base,basefacts),('s2-pkg',pkg,pkgfacts)]:
 summaries[variant]={}
 for name,root in roots.items():
  d=out/variant/name;d.mkdir(parents=True)
  original=rows/name/'base-sites.jsonl';shutil.copy2(original,d/'base-sites.jsonl')
  native=json.loads((census/name/'census.json').read_text())
  for p,h in native['inputs'].items():assert sha(Path(p))==h,'retained base native/source input drift'
  categories={(r['writer'],r['specifier']):r['category'] for r in native['records']}
  def run(args,file):
   with (d/file).open('w') as o,(d/(file+'.stderr')).open('w') as e:subprocess.run([str(x) for x in args],env=dict(os.environ,PRISM_S2_REFUSAL_DIAGNOSTICS='1'),stdout=o,stderr=e,check=True,timeout=1800)
  run([bin,'nav','--no-cache','call-stats','--repo',root,'--dump-sites'],'head-sites.jsonl')
  run([facts,root],'facts.jsonl')
  run(['node',here/'s2-census.cjs',ts,root,d/'base-sites.jsonl',d/'facts.jsonl',d],'oracle.log')
  result=compare.compare(d)
  joins={}
  for line in (d/'head-sites.jsonl.stderr').read_text().splitlines():
   m=re.fullmatch(r'S2 unavailable refusal join: writer=(.*?) spec=(.*?) member=(.*?) possible=(.*)',line)
   if not m:continue
   writer,imp,member,possible=m.groups();k=(writer,imp,member)
   identities={(json.loads(a),json.loads(b)) for a,b in re.findall(r'\(("(?:[^"\\]|\\.)*"), ("(?:[^"\\]|\\.)*")\)',possible)}
   affected=identities & gains.keys()
   cat='relative' if imp.startswith('.') else 'node_builtin' if imp.startswith('node:') else 'other_scheme' if ':' in imp else 'workspace' if categories.get((writer,imp))=='workspace' else 'bare_or_alias'
   r=joins.setdefault(k,dict(writer=writer,specifier=imp,member=member,events=0,gain_classes=[],observed_union=[],observed_states=[],category=cat))
   current=[dict(file=f,local=l,rows=gains[f,l]) for f,l in sorted(affected)]
   union={(v['file'],v['local']) for v in r['observed_union']} | affected
   r['observed_union']=[dict(file=f,local=l,rows=gains[f,l]) for f,l in sorted(union)]
   if current not in r['observed_states']:r['observed_states'].append(current)
   r['gain_classes']=current
   r['events']+=1
  assert joins,'empty refusal diagnostic population'
  totals={}
  for cat in ['bare_or_alias','relative','workspace','other_scheme','node_builtin']:
   records=[r for r in joins.values() if r['category']==cat];affected={(c['file'],c['local']) for r in records for c in r['gain_classes']}
   union={(c['file'],c['local']) for r in records for c in r['observed_union']}
   totals[cat]=dict(unique_joins=len(records),gain_revoking_joins=sum(bool(r['gain_classes']) for r in records),gain_rows_at_risk=sum(gains[k] for k in affected),observed_union_rows_at_risk=sum(gains[k] for k in union))
  residual=dict(comparison=result,categories=totals,joins=list(joins.values()),trace_semantics='gain_classes is last observed emission; observed_union conservatively combines all graph-build emissions. No graph id or isolated causal recovery credit is inferred.')
  (d/'residuals.json').write_text(json.dumps(residual,indent=2)+'\n')
  (d/'binary-binding.json').write_text(json.dumps(dict(binaries=binaries,retained_main_stream_sha256=sha(original)),indent=2)+'\n')
  summaries[variant][name]=dict(comparison=result,categories=totals)
  assert all(sha(Path(v['path']))==v['sha256'] for v in binaries.values()),'binary drift'
  print(variant,name,json.dumps(result),flush=True)
report=dict(reference_gain=132,reference_provenance='owner brief and supplied R3b report; original 75a35 reference not rebuilt in PKG lane',recovered_gain=max(s['comparison']['correct'] for v in summaries.values() for s in v.values()),variants=summaries)
(out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
