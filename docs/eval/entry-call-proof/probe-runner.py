import json,subprocess,hashlib,sys,os
B='/Users/wesleyjinks/code/prism-plan-entry-call/target/release/prism'
ROOT='/Users/wesleyjinks/prism-evidence/inputs/excalidraw-0642e72c/source'
T=json.load(open('/Users/wesleyjinks/code/prism-plan-entry-call/docs/superpowers/plans/2026-09-25-bounded-entry-call-proof/targets.json'))['targets']
n=sys.argv[1]; out=os.path.join(os.path.dirname(os.path.abspath(__file__)),f'run-{n}'); os.makedirs(out,exist_ok=True)
cmds=[('call-stats-dump-sites.jsonl',[B,'nav','--no-cache','call-stats','--repo',ROOT,'--dump-sites']),
      ('dfg-stats-edges.jsonl',[B,'nav','--no-cache','dfg-stats','--repo',ROOT,'--edges'])]
for i,t in enumerate(T):
    src=open(os.path.join(ROOT,t['path']),'rb').read()
    line=src[:t['callable']['start_byte']].count(b'\n')+1
    tag=f"{i:02d}-{t['callable']['name']}"
    cmds.append((f'nodes-at-{tag}.json',[B,'nav','--no-cache','nodes-at','--repo',ROOT,'--location',f"{t['path']}:{line}",'--format','json']))
    cmds.append((f'callers-{tag}.json',[B,'nav','--no-cache','callers','--repo',ROOT,'--symbol',t['callable']['name'],'--file',t['path'],'--format','json']))
manifest=[]
for name,argv in cmds:
    r=subprocess.run(argv,capture_output=True)
    open(os.path.join(out,name),'wb').write(r.stdout)
    manifest.append({'file':name,'argv':[a.replace(ROOT,'<root>').replace(B,'prism') for a in argv],'exit':r.returncode,'sha256':hashlib.sha256(r.stdout).hexdigest(),'stderr_tail':r.stderr.decode(errors='replace')[-300:]})
json.dump(manifest,open(os.path.join(out,'MANIFEST.json'),'w'),indent=1)
print(n,'commands',len(manifest),'nonzero exits',[m['file'] for m in manifest if m['exit']])
