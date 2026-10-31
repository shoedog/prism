"""Complete S2 comparison extracted from the owner S2 packet; no private-root defaults."""
import gzip,json
from pathlib import Path
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
