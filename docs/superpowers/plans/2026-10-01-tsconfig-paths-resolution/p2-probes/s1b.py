"""Byte-identity preservation on all retained S1b-4 public controls."""
import hashlib, json, subprocess, sys
from pathlib import Path
head=Path(sys.argv[1]).resolve()
p1=Path('/Users/wesleyjinks/code/prism-paths-impl/target/repair-r5/head/prism')
root=Path('/Users/wesleyjinks/prism-evidence/paths/impl-P1/repair-r2d-target/s1b-fixtures')
fixtures=sorted(p for p in root.iterdir() if p.is_dir() and p.name.startswith('C'))
assert len(fixtures)==411
records=[]; sites=0
for f in fixtures:
    rec={'case':f.name}
    for mode,args in [('sites',['call-stats','--dump-sites']),('functions',['functions'])]:
        outputs=[]
        for binary in [p1,head]:
            p=subprocess.run([str(binary),'nav','--no-cache',*args,'--repo',str(f)],capture_output=True)
            assert p.returncode==0 and p.stderr==b'', (f.name,mode,p.stderr)
            outputs.append(p.stdout)
        assert outputs[0]==outputs[1],(f.name,mode,'changed')
        rec[mode+'_sha256']=hashlib.sha256(outputs[1]).hexdigest()
        if mode=='sites': sites+=sum(json.loads(l).get('record_kind')=='call_site' for l in outputs[1].splitlines())
    records.append(rec)
summary={'controls':len(fixtures),'sites':sites,'byte_identical_comparisons':822,'stderr_bytes':0,'records':records}
out=Path('target/p2-plan/p2-s1b.json');out.write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k!='records'}))
