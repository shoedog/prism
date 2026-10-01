"""Bound Tier-A quick with its usual queries and an explicit writable nav cache.
Usage (installed Python 3.12): run_quick_s1b4.py PROTO_ROOT OUT_DIR [SECONDS]
Only the cache directory changes; no oracle/query/grade/baseline override.
"""
import json,os,signal,subprocess,sys,time
from pathlib import Path
root=Path(sys.argv[1]).resolve();out=Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=True);cap=int(sys.argv[3]) if len(sys.argv)>3 else 300
cache=out/'nav-cache'
code="""import sys
from tier_a.sut import PrismCli
from tier_a.cli import main
original=PrismCli._run
cache_path=sys.argv[1]
def local_cache(self,args):
    if getattr(self,"no_cache",False):
        return original(self,args)
    return original(self,["--cache-dir",cache_path,*args])
PrismCli._run=local_cache
sys.argv=[sys.argv[0],"--quick","--allow-stale-sut"]
raise SystemExit(main())
"""
env=dict(os.environ,PYTHONPATH=str(root/'eval'));start=time.monotonic()
with (out/'quick.log').open('w') as log:
 p=subprocess.Popen([sys.executable,'-u','-c',code,str(cache)],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
 try:
  rc=p.wait(timeout=cap);status='completed'
 except subprocess.TimeoutExpired:
  os.killpg(p.pid,signal.SIGINT)
  try:rc=p.wait(timeout=10)
  except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);rc=p.wait()
  status='deadline_no_complete_result'
report=dict(status=status,returncode=rc,elapsed_s=round(time.monotonic()-start,2),cap_s=cap,cache_dir=str(cache),query_and_oracle_overrides=False)
(out/'receipt.json').write_text(json.dumps(report,indent=1));print(json.dumps(report))
raise SystemExit(0 if status=='completed' and rc==0 else 1)
