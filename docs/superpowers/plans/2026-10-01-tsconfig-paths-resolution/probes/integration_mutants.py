from pathlib import Path
import subprocess,json,os,sys,hashlib,shutil,time,re
root=Path(sys.argv[1]).resolve();out=Path(sys.argv[2]).resolve();cargo=Path(sys.argv[3]).resolve();out.mkdir(parents=True,exist_ok=True);repo=out/'mutation-repo';packet=Path(__file__).resolve().parent
population=json.loads((packet/'adopted-mutants.json').read_text());m=population['mutations'];extras=population['extra'];records=[]
if len(sys.argv)>4:
 selected=sys.argv[4];assert selected in m,selected;m={selected:m[selected]}
for name in ['src','tests','vendor','scripts/callable-observations','docs/eval/receiver-closure']:shutil.copytree(root/name,repo/name,dirs_exist_ok=True)
for name in ['Cargo.toml','Cargo.lock','build.rs']:shutil.copy2(root/name,repo/name)
for p in repo.rglob('*.rs'):os.utime(p,None)
original={str(p.relative_to(repo)):p.read_bytes() for p in (repo/'src').rglob('*.rs')};env=os.environ.copy();env.update(CARGO_TARGET_DIR=str(cargo),CARGO_INCREMENTAL='1',CARGO_PROFILE_DEV_DEBUG='0',PRISM_TYPESCRIPT='/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js');available={};bins={};defects=[]
for mode in ['lib','integration']:
 args=['cargo','test','--offline','--manifest-path',str(repo/'Cargo.toml'),*(['--lib'] if mode=='lib' else ['--test','integration']),'--no-run','--message-format=json'];p=subprocess.run(args,capture_output=True,text=True,env=env);(out/f'mutation-preflight-{mode}.log').write_text(p.stderr)
 assert p.returncode==0,(mode,p.stderr[-2000:])
 for l in p.stdout.splitlines():
  a=json.loads(l)
  if a.get('reason')=='compiler-artifact' and a.get('profile',{}).get('test') and a.get('executable') and (('lib' in a['target']['kind']) if mode=='lib' else a['target']['name']=='integration'):bins[mode]=a['executable']
 available[mode]=subprocess.check_output([bins[mode],'--list'],text=True)
for label,(file,a,b,test) in m.items():
 mode='lib' if test.startswith(('js_paths_snapshot::','js_paths_first_pass::','repo_loader::')) else 'integration'
 if test+': test' not in available[mode] or a not in (repo/file).read_text():defects.append(label)
assert not defects,defects
baseline=[]
for test in sorted({v[3] for v in m.values()}):
 mode='lib' if test.startswith(('js_paths_snapshot::','js_paths_first_pass::','repo_loader::')) else 'integration';p=subprocess.run([bins[mode],test,'--exact'],capture_output=True,text=True,env=env);log=p.stdout+p.stderr;baseline.append({'test':test,'pass':'running 1 test' in log and 'test result: ok.' in log,'status':p.returncode})
 if not baseline[-1]['pass']:(out/f'baseline-{test.split("::")[-1]}.log').write_text(log)
(out/'mutant-baseline.json').write_text(json.dumps(baseline,indent=2));assert all(r['pass'] for r in baseline),baseline
print('PRECHECK',len(m),'mutants,',len(baseline),'baseline selectors green',flush=True)
for label,(file,a,b,test) in m.items():
 changed={}
 def replace(file,a,b):
  p=repo/file;s=p.read_text();assert a in s,(label,file,a);changed.setdefault(file,p.read_bytes());p.write_text(s.replace(a,b))
 try:
  replace(file,a,b)
  for f,a,b in extras.get(label,[]):replace(f,a,b)
  if label=='I56-no-paths-first-pass':replace('src/js_paths.rs','if self.snapshot.kind(&q).is_some() {','if (q.ends_with(".js") || q.ends_with(".jsx")) && self.snapshot.kind(&q).is_some() {')
  mode='lib' if test.startswith(('js_paths_snapshot::','js_paths_first_pass::','repo_loader::')) else 'integration';command=['cargo','test','--offline','--manifest-path',str(repo/'Cargo.toml'),*(['--lib'] if mode=='lib' else ['--test','integration']),test,'--','--exact'];t=time.monotonic();p=subprocess.run(command,capture_output=True,text=True,env=env);log=p.stdout+p.stderr;d=out/'integration-mutants'/label;d.mkdir(parents=True,exist_ok=True);(d/'test.log').write_text(log);killed='running 1 test' in log and 'panicked at' in log and 'test result: FAILED.' in log;admissible=killed or ('running 1 test' in log and 'test result: ok.' in log)
  rec={'mutant':label,'test':test,'killed':killed,'admissible':admissible,'status':p.returncode,'seconds':time.monotonic()-t,'mutated_source_hashes':{f:hashlib.sha256((repo/f).read_bytes()).hexdigest() for f in changed}}
 except Exception as e:rec={'mutant':label,'test':test,'killed':False,'admissible':False,'error':str(e)}
 finally:
  for f,data in changed.items():(repo/f).write_bytes(data)
  assert all((repo/f).read_bytes()==data for f,data in original.items()),'restore drift'
 records.append(rec);(out/'integration-mutants-summary.json').write_text(json.dumps(records,indent=2));print(label,'KILLED' if rec['killed'] else 'SURVIVED' if rec['admissible'] else 'INADMISSIBLE',flush=True)
print('TOTAL',sum(r['killed'] for r in records),'/',len(records),'admissible',sum(r['admissible'] for r in records),flush=True)
