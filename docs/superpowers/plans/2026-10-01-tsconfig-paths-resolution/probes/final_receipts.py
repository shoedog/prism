"""Finalize public X/R/T expected rows and bind input bytes; never discovers F.
Usage: final_receipts.py PUBLIC_OUT BASE_BIN HEAD_BIN BASE_FACTS_BIN
"""
import json,sys,hashlib
from collections import Counter
from pathlib import Path
from rowdiff import load,raw_load
out=Path(sys.argv[1]);binary_paths=list(map(Path,sys.argv[2:5]))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
roots={'X':Path.home()/'prism-evidence/inputs/excalidraw-0642e72c/source','R':Path.home()/'code/bench-repos/ruff/playground','T':Path.home()/'code/bench-repos/TypeScript/src'}
summary={'claim':'MEASURED','binaries':{str(p.resolve()):sha(p) for p in binary_paths},'corpora':{}};inputs={}
for corpus,root in roots.items():
 p0=json.loads((out/f'{corpus}-P0.json').read_text());classes=json.loads((out/f'{corpus}-classified.json').read_text())
 base=load(out/'base'/f'{corpus}-dump-sites.jsonl');head=load(out/'head'/f'{corpus}-dump-sites.jsonl');assert base.keys()==head.keys()
 assert len(classes)==p0['changed_rows'];assert all(c['class']=='CORRECT_STATIC_BINDING' for c in classes)
 expected=[]
 for c in classes:
  t=c['oracle']['terminal'];target=[t['file'],t['name'],t['start_line'],t['end_line'],'exact','import_member']
  assert c['proto']==[None,[target]]
  expected.append({'key':c['key'],'expected':[None,[target]],'class':c['class'],'basis':'independent TypeScript import/module/export/name/span proof'})
 (out/f'{corpus}-expected.json').write_text(json.dumps(expected,indent=2)+'\n')
 facts=[json.loads(x) for x in (out/'base'/f'{corpus}-import-facts.jsonl').read_text().splitlines()];names={f['file'] for f in facts}
 for name,h in p0['input_hashes'].items():
  path=root/name
  if path.is_file():assert sha(path)==h,(corpus,name);names.add(name)
 inputs[corpus]={name:sha(root/name) for name in sorted(names)}
 aliases=json.loads((out/f'{corpus}-alias-sites.json').read_text());retained=[x for x in aliases if x['recoverable'] and base[tuple(x['key'])]==head[tuple(x['key'])]]
 if corpus=='X':
  unrelated=[x for x in retained if x['member'] in ['Footer','WelcomeScreen']]
  assert {x['member'] for x in unrelated}=={'Footer','WelcomeScreen'}, 'name-directed histogram controls missing'
  assert all(x['refusal_reason']=='UNCLASSIFIED_P1_PROOF' for x in unrelated), 'unrelated named exports must not explain the requested member'
  related=[x for x in retained if x['member']=='getSceneVersion']
  assert related and all(x['refusal_reason']=='NONRELATIVE_EXPORT_HOP' for x in related), 'actual nonrelative member forwarding must retain its reason'
 (out/f'{corpus}-retained-callable-candidates.json').write_text(json.dumps(retained,indent=2)+'\n')
 raw_base=raw_load(out/'base'/f'{corpus}-dump-sites.jsonl');raw_head=raw_load(out/'head'/f'{corpus}-dump-sites.jsonl')
 metadata=lambda row:{k:v for k,v in row.items() if k not in ('drop','resolved_targets')}
 assert all(metadata(raw_base[k])==metadata(raw_head[k]) for k in raw_base)
 summary['corpora'][corpus]={'counts':p0['counts'],'classes':p0['classes'],'changed_rows':len(classes),'keys_added':0,'keys_removed':0,'metadata_changes':0,'unchanged_recoverable':len(retained),'refusal_reason_histogram':dict(sorted(Counter(x['refusal_reason'] for x in retained).items())),'input_files':len(inputs[corpus]),'expected_sha256':sha(out/f'{corpus}-expected.json')}
(out/'source-input-hashes.json').write_text(json.dumps(inputs,indent=2)+'\n');(out/'FINAL-SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:{'changed':v['changed_rows'],'classes':v['classes'],'input_files':v['input_files'],'retained':v['unchanged_recoverable'],'refusals':v['refusal_reason_histogram']} for k,v in summary['corpora'].items()}))
