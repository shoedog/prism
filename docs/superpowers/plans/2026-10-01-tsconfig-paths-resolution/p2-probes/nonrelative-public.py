"""Same-environment complete base/P1/P2 public streams; certify every new row."""
import gzip, hashlib, json, subprocess, sys
from pathlib import Path
from compare import load, compare
packet=Path(__file__).resolve().parent; repo=packet.parents[4]; out=Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=False)
head=Path(sys.argv[1]).resolve();home=Path.home()
p1=home/'code/prism-paths-impl/target/repair-r5/head/prism';base=home/'prism-evidence/paths/reviews/evidence-P1-r4-sol61/r4/base-prism'
ts=home/'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
oldfacts=home/'code/prism-paths-plan/target/paths-plan/extension-priority/evidence/public/base'
roots={'X':home/'prism-evidence/inputs/excalidraw-0642e72c/source','installed-X':home/'prism-evidence/inputs/excalidraw-0642e72c-installed/source','R':home/'code/bench-repos/ruff/playground','T':home/'code/bench-repos/TypeScript/src'}
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest(); summary={}
for name,root in roots.items():
 d=out/name;d.mkdir();paths={}
 for label,binary in [('base',base),('p1',p1),('p2',head)]:
  dest=d/(label+'.jsonl');paths[label]=dest
  with dest.open('w') as f,(d/(label+'.stderr')).open('w') as err:subprocess.run([str(binary),'nav','--no-cache','call-stats','--repo',str(root),'--dump-sites'],stdout=f,stderr=err,check=True)
 rows={k:load(v) for k,v in paths.items()};assert rows['base'].keys()==rows['p1'].keys()==rows['p2'].keys()
 retained=repo/'target/p2-plan/public'/name
 assert gzip.decompress((retained/'p1-sites.jsonl.gz').read_bytes())==paths['p1'].read_bytes(),'P1 population drift'
 facts=oldfacts/(('X' if name=='installed-X' else name)+'-import-facts.jsonl');fr=[json.loads(l) for l in facts.read_text().splitlines()]
 for f in fr:
  h=bytes(f['hash']).hex() if isinstance(f['hash'],list) else f['hash'];assert sha(root/f['file'])==h,'fact input drift'
 aliases=d/'aliases.json';aliases.write_bytes(gzip.decompress((retained/'oracle-alias-sites.json.gz').read_bytes()))
 with (d/'native.log').open('w') as f:subprocess.run(['node',str(packet/'native.cjs'),str(ts),str(root),str(aliases),str(facts),str(d/'fresh')],stdout=f,stderr=subprocess.STDOUT,check=True)
 r,changed=compare(paths['p1'],paths['p2'],d/'fresh-native-rows.json');assert not r['classes'].get('UNPROVEN_OR_WRONG'),r
 for p,h in json.loads((d/'fresh-native-inputs.json').read_text()).items():assert sha(root/p)==h,'native input drift'
 # Retained P1 gains are unchanged; every additional difference is certified above.
 oldkeys={k for k in rows['base'] if rows['base'][k]!=rows['p1'][k]};assert len(oldkeys)==(3121 if name in ['X','installed-X'] else 0)
 assert all(rows['p1'][k]==rows['p2'][k] for k in oldkeys),'P1 gain changed'
 r.update(vs_base=sum(rows['base'][k]!=rows['p2'][k] for k in rows['base']),facts_rehashed=len(fr),native_inputs_rehashed=len(json.loads((d/'fresh-native-inputs.json').read_text())),binary_sha256={'base':sha(base),'p1':sha(p1),'p2':sha(head)})
 summary[name]=r;(d/'comparison.json').write_text(json.dumps(r,indent=2)+'\n');(d/'changed.json').write_text(json.dumps(changed,indent=2)+'\n')
 for p in paths.values():
  with gzip.open(str(p)+'.gz','wb') as f:f.write(p.read_bytes())
  p.unlink()
 (out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(name,json.dumps(r),flush=True)
