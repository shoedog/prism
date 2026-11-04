"""Extract the gold file list per trial question into goldfiles.json.

Needs the PRIVATE tracing-trial store (questions and gold v2 are not published):
  PRISM_TRIAL_DIR  tracing-trial directory (questions.jsonl, gold/v2/)
  PRISM_GOLD_DIR   optional, default $PRISM_TRIAL_DIR/gold/v2
Usage: python3 goldfiles.py <out-dir>
Writes <out-dir>/goldfiles.json. Keep that file private (it lists gold files) and point
PRISM_GOLDFILES at it for rank*.py / atbudget.py. Never write it into this repository.
"""
import json, re, sys, os, collections
if len(sys.argv) != 2:
    sys.exit('usage: python3 goldfiles.py <out-dir>   (outside the repository; needs PRISM_TRIAL_DIR)')
T = os.environ.get('PRISM_TRIAL_DIR')
if not T:
    sys.exit('PRISM_TRIAL_DIR is not set (private tracing-trial directory with questions.jsonl and gold/v2/)')
G = os.environ.get('PRISM_GOLD_DIR') or os.path.join(T, 'gold', 'v2')
if not os.path.isfile(os.path.join(T, 'questions.jsonl')):
    sys.exit('questions.jsonl not found in PRISM_TRIAL_DIR=' + T)
if not os.path.isdir(G):
    sys.exit('gold directory not found: ' + G)
os.makedirs(sys.argv[1], exist_ok=True)
pat=re.compile(r'([A-Za-z0-9_./@-]+\.(?:py|rs|go|js|jsx|ts|tsx|mjs|cjs)):(\d+)')
out={}
for l in open(os.path.join(T, 'questions.jsonl')):
    q=json.loads(l)
    g=json.load(open(os.path.join(G, f"{q['id']}.json")))
    s=json.dumps(g)
    files=collections.Counter(m.group(1) for m in pat.finditer(s))
    out[q['id']]={'corpus':q['corpus'],'format':q['format'],'files':sorted(files)}
json.dump(out,open(os.path.join(sys.argv[1], 'goldfiles.json'),'w'),indent=1)
for k,v in out.items(): print(k,v['corpus'],v['format'],len(v['files']),v['files'][:8])
