"""S2 complete-stream comparison; every changed row needs a fresh native certificate."""
import argparse, concurrent.futures, gzip, hashlib, json, subprocess, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
sys.path.insert(0, str(HERE))
from public import ROOTS, TS, BIN, FACTS

def read(p):
    data = gzip.decompress(p.read_bytes()) if p.suffix == '.gz' else p.read_bytes()
    return [json.loads(l) for l in data.splitlines() if l]

def key(r):
    c,s = r['caller'],r['source_span']
    return (c['file'],c['name'],c['start_line'],s['file'],s['start_byte'],s['end_byte'],r['callee_text'])

def compare(d):
    rows = []
    for name in ('base-sites.jsonl','head-sites.jsonl'):
        stream = [r for r in read(d/name) if r.get('record_kind')=='call_site']
        keyed = {key(r):r for r in stream}
        assert len(keyed) == len(stream), f'duplicate site keys: {name}'
        rows.append(keyed)
    b,h = rows
    assert b.keys() == h.keys(), 'site population changed'
    candidates = json.loads((d/'candidates.json').read_text())
    native = {tuple(r['key']):r for r in candidates}
    assert len(native) == len(candidates), 'duplicate native keys'
    changed = []
    summary = dict(total_sites=len(b), base_bound_sites=sum(bool(r['resolved_targets']) for r in b.values()),
                   changed=0, correct=0, lost_base_edges=0, unproven=0, mechanisms={}, classes={})
    for k, before in b.items():
        after = h[k]
        meta = lambda r:{x:y for x,y in r.items() if x not in ('resolved_targets','exact_target','drop')}
        assert meta(before)==meta(after), 'site metadata changed'
        if before==after: continue
        summary['changed'] += 1
        summary['lost_base_edges'] += sum(t not in after['resolved_targets'] for t in before['resolved_targets'])
        n = native.get(k)
        ts = after['resolved_targets']
        correct = bool(not before['resolved_targets'] and len(ts)==1 and ts[0]['confidence']=='exact' and n and
                       n['callable'] and not n['binding']['type_only'] and n['prism_module_proof'] and
                       n['native_module']==n['prism_module_proof']['module'] and n['owner']==n['prism_module_proof']['owner'] and
                       len(n['qualifier_declarations'])==1 and
                       all(ts[0]['function_id'][x]==n['terminal'][x] for x in ('file','name','start_line','end_line')))
        label = 'CORRECT_STATIC_BINDING' if correct else 'UNPROVEN_OR_WRONG'
        summary['correct' if correct else 'unproven'] += 1
        summary['classes'][label] = summary['classes'].get(label,0)+1
        mech = n['mechanism'] if n else 'UNJOINABLE'
        summary['mechanisms'][mech] = summary['mechanisms'].get(mech,0)+1
        changed.append(dict(key=k,classification=label,native=n,before=before,after=after))
    (d/'changed.json').write_text(json.dumps(changed,indent=2)+'\n')
    (d/'comparison.json').write_text(json.dumps(summary,indent=2)+'\n')
    assert summary['lost_base_edges']==summary['unproven']==0, summary
    return summary

def run(args, out):
    with out.open('w') as f, Path(str(out)+'.stderr').open('w') as err:
        subprocess.run([str(x) for x in args],stdout=f,stderr=err,check=True,timeout=1200)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('head', type=Path)
    parser.add_argument('headfacts', type=Path)
    parser.add_argument('out', type=Path)
    parser.add_argument('--reuse-base', type=Path, help='reuse source-bound base streams; fresh facts/oracle inputs must match')
    parser.add_argument('--jobs', type=int, default=1, choices=range(1,5))
    a = parser.parse_args()
    head,headfacts,out = a.head,a.headfacts,a.out
    out.mkdir(parents=True,exist_ok=False)
    binaries = {'base': BIN, 'head': head, 'headfacts': headfacts}
    hashes = {role:hashlib.sha256(p.read_bytes()).hexdigest() for role,p in binaries.items()}
    manifest = json.loads((HERE/'reference-binaries.json').read_text())
    assert hashes['base']==manifest['binaries']['base']['sha256'], 'base binary drift'
    def corpus(name):
        root = ROOTS[name]
        d=out/name;d.mkdir()
        if a.reuse_base:
            prior = a.reuse_base/name
            prior_binding = json.loads((prior/'binary-binding.json').read_text())
            assert prior_binding['base']==hashes['base'], 'reused base binary drift'
            stream = prior/'base-sites.jsonl'
            if not stream.exists(): stream = Path(str(stream)+'.gz')
            data = gzip.decompress(stream.read_bytes()) if stream.suffix=='.gz' else stream.read_bytes()
            (d/'base-sites.jsonl').write_bytes(data)
        else:
            run([BIN,'nav','--no-cache','call-stats','--repo',root,'--dump-sites'],d/'base-sites.jsonl')
        retained = REPO/'target/s2-plan/current-main'/name/'base-sites.jsonl.gz'
        assert gzip.decompress(retained.read_bytes())==(d/'base-sites.jsonl').read_bytes(), 'P2/S2-0 base drift'
        run([head,'nav','--no-cache','call-stats','--repo',root,'--dump-sites'],d/'head-sites.jsonl')
        run([headfacts,root],d/'facts.jsonl')
        run(['node',HERE/'census.cjs',TS,root,d/'base-sites.jsonl',d/'facts.jsonl',d],d/'oracle.log')
        if a.reuse_base:
            for file in ('receipt.json','oracle-inputs.json'):
                previous = json.loads((prior/file).read_text())
                current = json.loads((d/file).read_text())
                if file=='receipt.json': previous,current = previous['source_hashes'],current['source_hashes']
                assert previous==current, 'reused base source/config input drift'
        summary=compare(d)
        assert hashes=={role:hashlib.sha256(p.read_bytes()).hexdigest() for role,p in binaries.items()}, 'binary changed during measurement'
        (d/'binary-binding.json').write_text(json.dumps(hashes,indent=2)+'\n')
        print(name,json.dumps(summary),flush=True)
        return name,summary
    with concurrent.futures.ThreadPoolExecutor(max_workers=a.jobs) as pool:
        summaries = dict(pool.map(corpus,ROOTS))
    (out/'summary.json').write_text(json.dumps(summaries,indent=2)+'\n')

if __name__=='__main__': main()
