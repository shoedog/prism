"""Executable RED/GREEN warning probe for the 200,000-entry P1 budget."""
import json,subprocess,sys
from pathlib import Path
base,head=map(lambda x:str(Path(x).resolve()),sys.argv[1:3]);out=Path(sys.argv[3]).resolve();root=out/'repo';root.mkdir(parents=True,exist_ok=True)
(root/'tsconfig.json').write_text(json.dumps({'compilerOptions':{'moduleResolution':'node','allowJs':True,'paths':{'@lib':['lib/real']}},'include':['**/*']}))
(root/'lib').mkdir(exist_ok=True);(root/'lib/real.tsx').write_text('export function real() { return 1; }')
for e in ['jsx','tsx']:(root/('app.'+e)).write_text('import { real as picked } from "@lib"; export function run() { picked(); }')
fill=root/'fillers';fill.mkdir(exist_ok=True)
for n in range(200_001):(fill/(str(n)+'.txt')).touch()
results={}
for label,binary in [('base',base),('candidate',head)]:
 p=subprocess.run([binary,'nav','--no-cache','call-stats','--repo',str(root),'--dump-sites'],capture_output=True,text=True);(out/(label+'.jsonl')).write_text(p.stdout);(out/(label+'.stderr')).write_text(p.stderr);assert p.returncode==0,p.stderr
 results[label]=[json.loads(s) for s in p.stdout.splitlines() if json.loads(s).get('record_kind')=='call_site']
assert results['candidate']==results['base']
report={'sites':len(results['base']),'complete_base_parity':True,'warning_seen':'P1 paths disabled: P1 snapshot budget' in (out/'candidate.stderr').read_text()}
(out/'summary.json').write_text(json.dumps(report,indent=2));print(json.dumps(report));assert report['warning_seen'],'missing observable budget refusal'
