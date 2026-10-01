"""Read-only Tier-A quick control against the real Git-bound base corpus.
The Git-ignored prototype copy has no tracked universe. This is a control,
not the prototype worktree's own quick acceptance run. No baseline is edited.
Usage: python3.12 quick_control.py REAL_REPO BIN OUT_JSON
"""
import argparse,json,sys
from pathlib import Path
repo=Path(sys.argv[1]).resolve();sys.path.insert(0,str(repo/'eval'))
from tier_a import cli
cfg=cli.load_corpora();corpus=dict(cfg['corpus']['prism']);corpus['path']=str(repo)
args=argparse.Namespace(sut_bin=str(Path(sys.argv[2]).resolve()),allow_stale_sut=True,allow_drift=True,quick=True,date='2026-10-01-paths-quick-control',oracle=None)
result=cli.run_corpus('prism',corpus,cfg['defaults'],args)
Path(sys.argv[3]).write_text(json.dumps(result,indent=2,sort_keys=True,default=str))
print(json.dumps({'meta':result['meta'],'probe_count':len(result.get('probes',{}))},default=str))
