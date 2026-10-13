#!/usr/bin/env python3
import concurrent.futures,json,subprocess,time
from pathlib import Path
import sys
base=Path(sys.argv[1]);binary=Path(sys.argv[2]);statuses=[json.loads(p.read_text()) for p in base.glob('*.status.json') if '.facts.' not in p.name];statuses=[s for s in statuses if s['exit']==0 and s.get('free_single_exact',0)]
def one(s):
 out=base/(s['name']+'.facts.jsonl')
 if out.exists() and out.stat().st_size and s['name']=='T' and all('form' in f for f in json.loads(out.read_text().splitlines()[0])['forms']):return {'name':s['name'],'status':'EXISTING'}
 start=time.monotonic()
 with out.open('wb') as o,out.with_suffix('.stderr').open('wb') as e:
  try:rc=subprocess.run([str(binary),s['root']],stdout=o,stderr=e,timeout=1500).returncode
  except subprocess.TimeoutExpired:rc=124
 result={'name':s['name'],'exit':rc,'seconds':round(time.monotonic()-start,3)};out.with_suffix('.receipt.json').write_text(json.dumps(result)+'\n');return result
with concurrent.futures.ThreadPoolExecutor(4) as pool:
 for result in pool.map(one,statuses):print(json.dumps(result),flush=True)
