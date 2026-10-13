#!/usr/bin/env python3
"""Static producer capture; never execute corpus code. All prism commands bind E5 cache."""
import argparse, concurrent.futures, hashlib, json, os, subprocess, time
from pathlib import Path
SKIP={'.git','target','node_modules','vendor','dist','build'}
def check_root(root,controller_private=False):
    if not controller_private and 'frontend-portal' in Path(root).parts: raise ValueError('forbidden root')
    for folder,dirs,files in os.walk(root,followlinks=False):
        if not controller_private and 'frontend-portal' in dirs: raise ValueError('forbidden directory below root')
        dirs[:]=[d for d in dirs if not d.startswith('.') and d not in SKIP and not Path(folder,d).is_symlink()]
def run(command,out,timeout):
    start=time.monotonic()
    with out.open('wb') as stdout, out.with_suffix(out.suffix+'.stderr').open('wb') as stderr:
        process=subprocess.Popen([str(c) for c in command],stdout=stdout,stderr=stderr,start_new_session=True)
        try: code=process.wait(timeout=timeout); reason=None if code==0 else f'exit {code}'
        except subprocess.TimeoutExpired:
            import signal
            os.killpg(process.pid,signal.SIGKILL); process.wait(); code=124; reason=f'timeout {timeout}s'
    record={'command':[str(c) for c in command],'exit':code,'reason':reason,'seconds':round(time.monotonic()-start,3),'bytes':out.stat().st_size}
    if code==0:
        try:
            count=0; eligible=0
            if '--format' in command and 'json' in command:
                document=json.loads(out.read_text());count=len(document) if isinstance(document,list) else 1
                record['artifact_format']='json_document'
            else:
                for line in out.open():
                    row=json.loads(line); count+=1
                    if row.get('record_kind')=='call_site' and Path(row['caller']['file']).suffix in {'.js','.jsx','.mjs','.cjs','.ts','.tsx'} and any(t['kind']=='free_single' and t['confidence']=='exact' for t in row['resolved_targets']): eligible+=1
                record['artifact_format']='jsonl'
            record.update(rows=count,free_single_exact=eligible)
        except Exception as error: record.update(exit=65,reason=f'invalid JSON: {error}')
    return record
def main():
    ap=argparse.ArgumentParser(); ap.add_argument('binary',type=Path);ap.add_argument('out',type=Path);ap.add_argument('--corpora',nargs='+',default=['X','T','SB']);ap.add_argument('--jobs',type=int,default=4);ap.add_argument('--timeout',type=int,default=1500);ap.add_argument('--paired-base',type=Path);ap.add_argument('--names-file',type=Path);a=ap.parse_args()
    a.out.mkdir(parents=True,exist_ok=True); home=Path.home(); cache=home/'prism-evidence/js-e5/cache'
    roots=[]
    if 'X' in a.corpora: roots.append(('X',home/'prism-evidence/inputs/excalidraw-0642e72c/source'))
    if 'T' in a.corpora: roots.append(('T',home/'code/bench-repos/TypeScript/src'))
    if 'SB' in a.corpora:
        pk=home/'prism-evidence/inputs/secbench-pkgs'; manifest=json.loads((pk/'manifest.json').read_text())
        roots.extend(('SB__'+e['class']+'__'+e['entry'],pk/e['class']/e['entry']/'src/package') for e in manifest['entries'] if e['status']=='ok')
    if a.names_file:
        selected=set(json.loads(a.names_file.read_text()));roots=[p for p in roots if p[0] in selected]
        assert len(roots)==len(selected)
    binding={'binary':str(a.binary.resolve()),'sha256':hashlib.sha256(a.binary.read_bytes()).hexdigest(),'roots':len(roots),'timeout':a.timeout,'cache':str(cache),'selection':str(a.names_file) if a.names_file else None}
    (a.out/'binding.json').write_text(json.dumps(binding,indent=2)+'\n')
    def one(pair):
        name,root=pair; out=a.out/(name+'.sites.jsonl')
        if a.paired_base:
            base=json.loads((a.paired_base/(name+'.status.json')).read_text())
            if base['exit']!=0:
                r={'name':name,'root':str(root),'exit':base['exit'],'reason':'paired exclusion: '+base['reason'],'seconds':0,'base_seconds':base['seconds']}
                (a.out/(name+'.status.json')).write_text(json.dumps(r,indent=2)+'\n');return r
        try:
            check_root(root)
            r=run([a.binary,'nav','--cache-dir',cache,'call-stats','--repo',root,'--dump-sites'],out,a.timeout)
        except Exception as error: r={'exit':65,'reason':str(error),'seconds':0}
        r.update(name=name,root=str(root)); (a.out/(name+'.status.json')).write_text(json.dumps(r,indent=2)+'\n');return r
    statuses=[]
    with concurrent.futures.ThreadPoolExecutor(a.jobs) as pool, (a.out/'status.jsonl').open('w') as stream:
        futures=[pool.submit(one,pair) for pair in roots]
        for f in concurrent.futures.as_completed(futures):
            r=f.result();statuses.append(r);stream.write(json.dumps(r)+'\n');stream.flush()
            print(json.dumps({k:r.get(k) for k in ['name','exit','seconds','free_single_exact','reason']}),flush=True)
    assert hashlib.sha256(a.binary.read_bytes()).hexdigest()==binding['sha256']
    (a.out/'summary.json').write_text(json.dumps({'roots':len(roots),'admitted':sum(r['exit']==0 for r in statuses),'excluded':[r for r in statuses if r['exit']!=0],'free_single_exact':sum(r.get('free_single_exact',0) for r in statuses if r['exit']==0)},indent=2)+'\n')
if __name__=='__main__':main()
