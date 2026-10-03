"""Bind the post-test rebuilt SUT to complete previously certified public dumps."""
import gzip,hashlib,json,subprocess,sys
from pathlib import Path
head=Path(sys.argv[1]).resolve();old=Path(sys.argv[2]).resolve();out=Path(sys.argv[3]).resolve();out.mkdir(exist_ok=False)
home=Path.home();roots={'X':home/'prism-evidence/inputs/excalidraw-0642e72c/source','installed-X':home/'prism-evidence/inputs/excalidraw-0642e72c-installed/source','R':home/'code/bench-repos/ruff/playground','T':home/'code/bench-repos/TypeScript/src'}
summary={};sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
for name,root in roots.items():
 dest=out/(name+'.jsonl')
 with dest.open('w') as f,(out/(name+'.stderr')).open('w') as e:subprocess.run([str(head),'nav','--no-cache','call-stats','--repo',str(root),'--dump-sites'],stdout=f,stderr=e,check=True)
 prior=old/name/'p2.jsonl.gz';assert prior.exists(),'prior complete stream not yet available'
 assert dest.read_bytes()==gzip.decompress(prior.read_bytes()),'rebuilt output changed'
 for p,h in json.loads((old/name/'fresh-native-inputs.json').read_text()).items():assert sha(root/p)==h,'native input drift'
 r=json.loads((old/name/'comparison.json').read_text());r.update(post_test_rebuild_byte_parity=True,verified_binary_sha256=sha(head),verified_dump_sha256=sha(dest),certification='fresh earlier caller ProjectService certificate, admitted after complete row parity and live native input rehash')
 summary[name]=r
 with gzip.open(str(dest)+'.gz','wb') as f:f.write(dest.read_bytes())
 dest.unlink();(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(name,json.dumps({'changed':r['changed'],'classes':r['classes'],'parity':True}),flush=True)
