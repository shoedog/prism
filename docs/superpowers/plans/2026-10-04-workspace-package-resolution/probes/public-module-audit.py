"""Audit complete module proof preservation and fresh native certificates.
Usage: PUBLIC_MEASUREMENT_OUT TS_JS AUDIT_JSON
Main facts are retained in planning/<corpus>/facts.jsonl; source hashes must
match the fresh head facts. Every added proof is checked live with ProjectService.
"""
import hashlib,json,subprocess,sys
from pathlib import Path
out,ts,report=map(Path,sys.argv[1:]);planning=Path('/Users/wesleyjinks/prism-evidence/pkgres/planning')
h=Path.home();roots={'X':h/'prism-evidence/inputs/excalidraw-0642e72c/source','installed-X':h/'prism-evidence/inputs/excalidraw-0642e72c-installed/source','R':h/'code/bench-repos/ruff/playground','T':h/'code/bench-repos/TypeScript/src'}
results=[];wrong=[]
def facts(p):return {r['file']:r for r in map(json.loads,p.read_text().splitlines())}
for corpus,root in roots.items():
 base=facts(planning/corpus/'facts.jsonl');head=facts(out/corpus/'facts.jsonl');assert set(base)==set(head),corpus
 assert all(base[k]['hash']==head[k]['hash'] for k in base),corpus
 proofs=lambda rows:{(f,s):v for f,r in rows.items() for s,v in r['modules'].items()}
 b=proofs(base);n=proofs(head);assert all(n.get(k)==v for k,v in b.items()),('lost main module',corpus)
 additions=[]
 for (writer,spec),proof in sorted(n.items()):
  if (writer,spec) in b:continue
  r=subprocess.run(['node',str(Path(__file__).with_name('native-binding.cjs')),str(ts),str(root),writer,spec],capture_output=True,text=True,check=True,timeout=300)
  native=json.loads(r.stdout);valid=[native['target'],native['owner']]==proof
  additions.append({'writer':writer,'specifier':spec,'proof':proof,'native':native,'CORRECT':valid})
  if not valid:wrong.append({'corpus':corpus,**additions[-1]})
 results.append({'corpus':corpus,'main_modules':len(b),'head_modules':len(n),'lost':0,'added':len(additions),'additions':additions,'rows_byte_identical':(out/corpus/'base-sites.jsonl').read_bytes()==(out/corpus/'head-sites.jsonl').read_bytes()})
 summary={'results':results,'wrong':wrong};report.write_text(json.dumps(summary,indent=2)+'\n')
 assert not wrong,'STOP: public wrong binding; choose nothing'
print(json.dumps({'added_by_corpus':{r['corpus']:r['added'] for r in results},'lost_modules':0,'wrong':0,'all_rows_byte_identical':all(r['rows_byte_identical'] for r in results)}))
