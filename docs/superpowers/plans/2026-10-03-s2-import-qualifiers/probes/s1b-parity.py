"""Current P2/S2-0 base and frozen S2 head: all 411 retained S1b-4 controls."""
import hashlib,json,subprocess,sys
from pathlib import Path
from public import BIN
head=Path(sys.argv[1]).resolve();out=Path(sys.argv[2])
root=Path.home()/'prism-evidence/paths/impl-P1/repair-r2d-target/s1b-fixtures'
fixtures=sorted(p for p in root.iterdir() if p.is_dir() and p.name.startswith('C'))
assert len(fixtures)==411
records=[];sites=0;changes=[]
for fixture in fixtures:
 rec={'case':fixture.name}
 for mode,args in [('sites',['call-stats','--dump-sites']),('functions',['functions'])]:
    rows=[]
    for binary in (BIN,head):
        p=subprocess.run([str(binary),'nav','--no-cache',*args,'--repo',str(fixture)],capture_output=True)
        assert p.returncode==0 and not p.stderr,(fixture.name,mode,p.stderr)
        rows.append(p.stdout)
    if rows[0]!=rows[1]:changes.append([fixture.name,mode])
    rec[mode+'_sha256']=hashlib.sha256(rows[1]).hexdigest()
    if mode=='sites':sites+=sum(json.loads(l).get('record_kind')=='call_site' for l in rows[1].splitlines())
 records.append(rec)
summary={'controls':len(fixtures),'sites':sites,'byte_identical_comparisons':822-len(changes),'changes':changes,'stderr_bytes':0,'records':records}
out.write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k!='records'}))
assert not changes,changes
