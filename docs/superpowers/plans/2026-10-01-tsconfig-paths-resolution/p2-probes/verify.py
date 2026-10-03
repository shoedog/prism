"""Full offline suites, lint and Tier-A with the freshly rebuilt prototype."""
import json, os, re, signal, subprocess, time
from pathlib import Path
out=Path('target/p2-plan'); env=os.environ.copy()
env['PRISM_TYPESCRIPT']=str(Path.home()/'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js')
env['PYTHONDONTWRITEBYTECODE']='1'
summary={}
for label,flags in [('default',[]),('mcp',['--features','mcp'])]:
    start=time.monotonic()
    with (out/('p2-'+label+'.log')).open('w') as f:
        p=subprocess.run(['cargo','test','--offline',*flags],stdout=f,stderr=subprocess.STDOUT,env=env)
    log=(out/('p2-'+label+'.log')).read_text()
    groups=re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',log)
    totals={name:sum(int(g[i]) for g in groups) for i,name in enumerate(['passed','failed','ignored','measured','filtered'])}
    summary[label]={'status':p.returncode,'seconds':time.monotonic()-start,'groups':len(groups),'totals':totals}
    (out/'p2-suites.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(label,totals,flush=True)
    assert p.returncode==0 and totals['passed']>4900 and totals['failed']==0
for label,cmd in [('fmt',['cargo','fmt','--check']),('clippy',['cargo','clippy','--offline','--all-targets','--all-features','--message-format=json']),('release',['cargo','build','--offline','--release'])]:
    with (out/('p2-'+label+'.log')).open('w') as f:
        p=subprocess.run(cmd,stdout=f,stderr=subprocess.STDOUT,env=env)
    summary[label]={'status':p.returncode};(out/'p2-suites.json').write_text(json.dumps(summary,indent=2)+'\n')
    assert p.returncode==0,label
python=Path('/Users/wesleyjinks/code/slicing/eval/.venv/bin/python')
for label,flag in [('matrix','--matrix-only'),('quick','--quick')]:
    # Rebuild immediately before each dirty-worktree harness invocation.
    subprocess.run(['cargo','build','--offline','--release'],check=True,env=env,stdout=subprocess.DEVNULL)
    command=[str(python),'-m','tier_a.cli']
    if label=='quick':
        # Preserve the committed reports; only the report destination changes.
        destination=str((out/'quick-reports').resolve())
        runner='import sys; from pathlib import Path; import tier_a.cli as c; w=c.write_reports; c.write_reports=lambda run,out:w(run,Path('+repr(destination)+')); sys.exit(c.main())'
        command=[str(python),'-c',runner]
    command += [flag,'--allow-stale-sut','--sut-bin',str(Path('target/release/prism').resolve())]
    if label=='quick': command += ['--date','2026-10-03-p2-proto']
    start=time.monotonic();timed_out=False
    with (out/('p2-tier-a-'+label+'.log')).open('w') as f:
        p=subprocess.Popen(command,cwd='eval',stdout=f,stderr=subprocess.STDOUT,env=env,start_new_session=True)
        try:p.wait(timeout=1800 if label=='quick' else None)
        except subprocess.TimeoutExpired:
            timed_out=True;os.killpg(p.pid,signal.SIGTERM)
            try:p.wait(timeout=10)
            except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait()
    summary['tier_a_'+label]={'status':p.returncode,'timed_out':timed_out,'seconds':time.monotonic()-start};(out/'p2-suites.json').write_text(json.dumps(summary,indent=2)+'\n')
    print('tier_a_'+label,p.returncode,flush=True)
    if label=='matrix': assert p.returncode==0,label
