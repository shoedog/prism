"""Complete P1/P2 row comparison. Private callers print only counts and hashes."""
import collections, hashlib, json, sys
from pathlib import Path

def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()

def key(r):
    c, s = r['caller'], r['source_span']
    return (c['file'], c['name'], c['start_line'], s['file'], s['start_byte'], s['end_byte'], r['callee_text'])

def load(p):
    rows = {}
    for line in Path(p).read_text().splitlines():
        r = json.loads(line)
        if r.get('record_kind') != 'call_site': continue
        k = key(r)
        assert k not in rows, 'duplicate site'
        rows[k] = r
    assert rows, 'zero-site probe inadmissible'
    return rows

def compare(p1, p2, native):
    a, b = load(p1), load(p2)
    assert a.keys() == b.keys(), 'site population changed'
    natives = {tuple(r['key']):r for r in json.loads(Path(native).read_text())}
    classes, reasons, written, ceiling = (collections.Counter() for _ in range(4))
    changed = []
    for k, before in a.items():
        after = b[k]
        meta = lambda r: {n:v for n,v in r.items() if n not in ('drop', 'resolved_targets', 'exact_target')}
        assert meta(before) == meta(after), 'site metadata changed'
        if before == after: continue
        n = natives.get(k)
        targets = after.get('resolved_targets') or []
        correct = bool(n and n['native_callable'] and n['ownership_agrees'] and n['site_import_proof'] == 'import' and len(targets) == 1)
        if correct:
            t, terminal = targets[0], n['terminal']
            correct = t['confidence'] == 'exact' and t['kind'] == 'import_member' and all(t['function_id'][field] == terminal[field] for field in ('file', 'name', 'start_line', 'end_line'))
        cls = 'CORRECT_STATIC_BINDING' if correct else 'UNPROVEN_OR_WRONG'
        classes[cls] += 1
        reason = n['reason'] if n else 'NO_NATIVE_ROW'
        reasons[reason] += 1
        if n and 'TERMINAL_MEMBER_WRITTEN' in n['refusal_detail_codes']: written[reason] += 1
        changed.append({'key': k, 'class': cls, 'reason': reason})
    for n in natives.values():
        if n['native_callable'] and n['ownership_agrees'] and n['allow_js'] and n['js_hop_count'] and not n['nonrelative_hop_count'] and not n['hop_stops']:
            ceiling[n['reason']] += 1
    summary = {'sites':len(a), 'changed':len(changed), 'classes':dict(classes), 'recovered_by_p1_reason':dict(reasons),
               'recovered_member_written_by_reason':dict(written), 'relative_js_resolution_ceiling_by_reason':dict(ceiling),
               'keys_added':0, 'keys_removed':0, 'p1_byte_identical':Path(p1).read_bytes() == Path(p2).read_bytes(),
               'p1_dump_sha256':sha(p1), 'p2_dump_sha256':sha(p2), 'native_rows_sha256':sha(native)}
    return summary, changed

if __name__ == '__main__':
    summary, changed = compare(*sys.argv[1:4])
    out = Path(sys.argv[4])
    out.with_suffix('.json').write_text(json.dumps(summary, indent=2)+'\n')
    out.with_name(out.name+'-changed.json').write_text(json.dumps(changed, indent=2)+'\n')
    print(json.dumps(summary, sort_keys=True))
    assert not summary['classes'].get('UNPROVEN_OR_WRONG'), 'uncertified changed rows'
