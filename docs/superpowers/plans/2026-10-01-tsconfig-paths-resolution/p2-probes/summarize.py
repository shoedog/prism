"""READ: Bind complete public bucket counts and actual ProjectService terminals.
Usage: python3 summarize.py
"""
import gzip, hashlib, json, subprocess
from collections import Counter
from pathlib import Path

PROBES=Path(__file__).resolve().parent
HERE=PROBES.parents[4]
OUT=HERE/'target/p2-plan'
def load(p):
    if p.exists(): return json.loads(p.read_bytes())
    return json.loads(gzip.decompress(Path(str(p)+'.gz').read_bytes()))
summary={'claim':'MEASURED','corpora':{},'bucket_rows':{},'public_callable_sites':{}}
roots={'X':Path.home()/'prism-evidence/inputs/excalidraw-0642e72c/source',
       'installed-X':Path.home()/'prism-evidence/inputs/excalidraw-0642e72c-installed/source',
       'R':Path.home()/'code/bench-repos/ruff/playground','T':Path.home()/'code/bench-repos/TypeScript/src'}
for corpus,root in roots.items():
    d=OUT/'public'/corpus
    s=json.load(open(d/'summary.json')); receipt=json.load(open(d/'receipt.json'))
    rows=load(d/'oracle-native-rows.json')
    aliases=load(d/'oracle-alias-sites.json')
    configurations=json.load(open(d/'oracle-P0.json'))['configurations']
    ts=Path.home()/'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
    parser="""const fs=require('fs'),p=require('path'),ts=require(process.argv[1]);
const configs=JSON.parse(fs.readFileSync(0,'utf8')),root=process.argv[2],out={};
for(const file of configs){const f=p.resolve(root,file),raw=ts.parseConfigFileTextToJson(f,ts.sys.readFile(f)||'').config||{};
const e=raw.extends;out[file]=Array.isArray(e)?'array':typeof e==='string'&&!e.startsWith('.')?'package':'local_or_none';}
process.stdout.write(JSON.stringify(out));"""
    extends=json.loads(subprocess.check_output(['node','-e',parser,str(ts),str(root)],input=json.dumps([c['path'] for c in configurations]),text=True))
    inputs=json.load(open(d/'oracle-native-inputs.json'))
    assert s['probe_sha256']==hashlib.sha256((PROBES/'native.cjs').read_bytes()).hexdigest(), 'stale native instrument'
    mismatches=[p for p,h in inputs.items() if not (root/p).is_file() or hashlib.sha256((root/p).read_bytes()).hexdigest()!=h]
    assert not mismatches, (corpus,mismatches)
    assert len(rows)==s['associated_low_sites']
    assert sum(s['all_low_refusal_histogram'].values())==len(rows)
    assert sum(s['refusal_histogram'].values())==s['remaining_p1_callable']
    assert s['old_recoverable_native_terminal_disagreements']==0
    assert s['old_recoverable_ownership_disagreements']==0
    callable_rows=[r for r in rows if r['old_recoverable']]
    summary['public_callable_sites'][corpus]=[{k:r[k] for k in ['key','reason','owner_config','terminal','terminal_agrees','ownership_agrees']} for r in callable_rows]
    by_key={tuple(a['key']):a for a in aliases}
    traits={'ordered_substitutions':0,'tied_pattern':0,'empty_capture':0,'explicit_substitution_extension':0,
            'node_next_or_bundler':0,'package_or_array_extends':0}
    for r in rows:
        a=by_key[tuple(r['key'])];opts=a.get('options') or {}
        traits['ordered_substitutions']+=any(len(v)>1 for v in opts.get('paths',{}).values())
        traits['tied_pattern']+=a['refusal_reason']=='TIED_PATTERN'
        traits['empty_capture']+=a['refusal_reason']=='EMPTY_CAPTURE'
        traits['explicit_substitution_extension']+=a['refusal_reason']=='EXPLICIT_EXTENSION_OUTSIDE_P1'
        traits['node_next_or_bundler']+=r.get('compiler_mode') in [3,99,100]
        config=a.get('config')
        if config:
            for cf in [config]+a.get('chain',[]):
                if extends.get(cf) in ['array','package']:
                    traits['package_or_array_extends']+=1;break
    s.update(receipt=receipt,native_inputs_rehashed=len(inputs),parked_trait_rows=traits)
    summary['corpora'][corpus]=s
    for reason,count in s['all_low_refusal_histogram'].items():
        summary['bucket_rows'].setdefault(reason,{})[corpus]=count
    s['bucket_native_terminals']={reason:dict(Counter(r['terminal']['class'] for r in rows if r['reason']==reason)) for reason in s['all_low_refusal_histogram']}
summary['public_distinct_callable_keys']=len({tuple(r['key']) for rs in summary['public_callable_sites'].values() for r in rs})
summary['public_distinct_callable_terminals']=len({(r['terminal']['file'],r['terminal']['name'],r['terminal']['start_line'],r['terminal']['end_line']) for rs in summary['public_callable_sites'].values() for r in rs})
(OUT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
(OUT/'public-callable-sites.json').write_text(json.dumps(summary['public_callable_sites'],indent=2)+'\n')
print(json.dumps({'claim':'MEASURED','sites':{c:s['total_sites'] for c,s in summary['corpora'].items()},
                  'remaining_callable':{c:s['remaining_p1_callable'] for c,s in summary['corpora'].items()},
                  'distinct_keys':summary['public_distinct_callable_keys'],'distinct_terminals':summary['public_distinct_callable_terminals']}))
