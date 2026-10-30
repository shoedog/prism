"""Public-only P0/base-head reproduction; builds/install/Git writes are external.
Usage: reproduce_public.py BASE_BIN HEAD_BIN BASE_FACTS_BIN TS_JS OUT
"""
import json,subprocess,sys
from pathlib import Path
base,head,facts,ts=map(lambda p:str(Path(p).resolve()),sys.argv[1:5]);out=Path(sys.argv[5]).resolve();out.mkdir(parents=True,exist_ok=True)
probes=Path(__file__).resolve().parent
roots={'X':Path.home()/'prism-evidence/inputs/excalidraw-0642e72c/source','R':Path.home()/'code/bench-repos/ruff/playground','T':Path.home()/'code/bench-repos/TypeScript/src'}
for corpus,root in roots.items():
 for label,binary in [('base',base),('head',head)]:
  d=out/label;d.mkdir(exist_ok=True)
  with (d/f'{corpus}-dump-sites.jsonl').open('w') as stdout,(d/f'{corpus}-dump-sites.stderr').open('w') as stderr:
   subprocess.run([binary,'nav','--no-cache','call-stats','--repo',str(root),'--dump-sites'],stdout=stdout,stderr=stderr,check=True)
 facts_path=out/'base'/f'{corpus}-import-facts.jsonl'
 with facts_path.open('w') as stdout:subprocess.run([facts,str(root)],stdout=stdout,check=True)
 changes=out/f'{corpus}-changes.json'
 with (out/f'{corpus}-rowdiff.log').open('w') as stdout:
  subprocess.run(['python3',str(probes/'rowdiff.py'),str(out/'base'/f'{corpus}-dump-sites.jsonl'),str(out/'head'/f'{corpus}-dump-sites.jsonl'),str(changes)],stdout=stdout,check=True)
 with (out/f'{corpus}-oracle.log').open('w') as stdout:
  subprocess.run(['node',str(probes/'oracle.cjs'),ts,str(root),str(out/'base'/f'{corpus}-dump-sites.jsonl'),str(facts_path),str(out/corpus),str(changes)],stdout=stdout,stderr=subprocess.STDOUT,check=True)
 r=json.loads((out/f'{corpus}-P0.json').read_text())
 print(corpus,json.dumps({'claim':'MEASURED','counts':r['counts'],'classes':r['classes'],'changed_rows':r['changed_rows']}))
