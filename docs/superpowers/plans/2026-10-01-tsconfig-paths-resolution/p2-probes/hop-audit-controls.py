"""Explicit public partition-seam tests; not asserted to be F's old rows."""
import json,subprocess,sys
from pathlib import Path
packet=Path(__file__).resolve().parent;controls=Path(sys.argv[1]).resolve();evidence=controls/'evidence';prior=controls/'audit-seam';prior.mkdir(exist_ok=False)
(prior/'p1-sites.jsonl').write_bytes((evidence/'p1-sites.jsonl').read_bytes())
(prior/'F-native-inputs.json').write_bytes((evidence/'F-native-inputs.json').read_bytes())
rows=json.loads((evidence/'F-native-rows.json').read_text());selected=[]
for n in rows:
 name=n['key'][0].split('/')[0]
 if '-empty-star-' not in name and '-missing-star-' not in name:continue
 missing='-missing-star-' in name
 selected.append({'key':n['key'],'class':'nonrelative_hop','event':{'file':name+'/entry.ts','spec':'@/empty','member':'real'},'typescript':{'export_symbol':'UNRESOLVED_MODULE' if missing else 'ABSENT','resolution':'UNRESOLVED' if missing else 'JS_SECONDARY_PASS'}})
assert len(selected)==16
(prior/'gap-private-rows.json').write_text(json.dumps(selected))
ts=Path.home()/'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
p=subprocess.run(['node',str(packet/'hop-audit.cjs'),str(ts),str(controls/'source'),str(evidence),str(prior)],capture_output=True,text=True,check=True)
r=json.loads(p.stdout);assert r['absent_explanations']=={'NONCONTRIBUTING_STAR_BRANCH_FULL_BARREL_BINDS_TERMINAL':8},r
assert r['unresolved']=={'rows':8,'stayed_at_p1':8},r
assert r['full_chain_span_agreement']=={'AGREES':16},r
r.update(control_origin='EXPLICIT_PUBLIC_DIAGNOSTIC_PARTITION_SEAM',private_F_claim=False)
(controls/'audit-seam-summary.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r))
