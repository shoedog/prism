"""Four scoped W1/W1b mutants, one shared build cache, serial execution."""
import hashlib, json, os, re, shutil, subprocess, sys
from pathlib import Path
source,out=Path(sys.argv[1]).resolve(),Path(sys.argv[2]).resolve()
out.mkdir(parents=True,exist_ok=False);work=out/'work'
shutil.copytree(source,work,ignore=shutil.ignore_patterns('target','.git'))
file='src/js_exports.rs';original=(source/file).read_text()
arm="""                    if nonrelative_forward {
                        return ExportLookup::BlockedClaim;
                    }
"""
predicate="""                let nonrelative_forward = matches!(target, JsExportTarget::ImportForward { .. })
                    && !module_path.starts_with('.');"""
depth_arm="""                    // Preserve the claim even when the next hop exceeds the bound.
"""+arm
assert original.count(arm)==2 and original.count(predicate)==1 and original.count(depth_arm)==1
mutants=[
 ('W01-drop-blocked-forward',arm,''),
 ('W02-block-relative-forward',predicate,predicate.replace("\n                    && !module_path.starts_with('.')",'')),
 ('W03-block-unresolved-reexport',predicate,predicate.replace('matches!(target, JsExportTarget::ImportForward { .. })','true')),
 ('W04-depth-before-blocked-forward',depth_arm,''),
]
(out/'manifest.json').write_text(json.dumps(mutants,indent=2)+'\n')
env=dict(os.environ,CARGO_TARGET_DIR=str(source/'target'))
records=[]
for name,mutation in [('reference',None)]+[(m[0],m) for m in mutants]:
    text=original if not mutation else original.replace(mutation[1],mutation[2])
    (work/file).write_text(text)
    with (out/(name+'.log')).open('w') as log:
        p=subprocess.run(['cargo','test','--offline','--features','mcp','--manifest-path',str(work/'Cargo.toml'),'--test','integration','js_paths_p2_test'],stdout=log,stderr=subprocess.STDOUT,env=env)
    log=(out/(name+'.log')).read_text()
    groups=re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed;',log)
    selected=sum(sum(map(int,g)) for g in groups);failed=sum(int(g[1]) for g in groups)
    admissible=selected==12 and not re.search(r'error\[E\d+\]',log)
    if not mutation: assert p.returncode==0 and admissible and failed==0,log[-5000:]
    else:
        expected='legacy_star_nonrelative_forwards_preserve_complete_main_rows' if name.startswith(('W01','W04')) else 'legacy_star_other_unresolved_claims_keep_existing_behavior'
        record={'mutant':name,'admissible':admissible,'selected':selected,'failed':failed,'killed':admissible and failed>0 and expected+' ... FAILED' in log,'expected_regression':expected,'mutated_sha256':hashlib.sha256(text.encode()).hexdigest()}
        if name.startswith('W04'):
            record['layout_mismatches']={k:log.count('"'+k+'"') for k in ['A1/A4','K1','K2']}
            record['killed'] &= record['layout_mismatches']=={'A1/A4':0,'K1':8,'K2':8}
        records.append(record);print(json.dumps(record),flush=True)
        (out/'results.json').write_text(json.dumps(records,indent=2)+'\n')
(work/file).write_text(original)
assert all(r['killed'] for r in records),records
# Remove only this runner's reproducible source copy; keep manifest/logs/results.
shutil.rmtree(work)
