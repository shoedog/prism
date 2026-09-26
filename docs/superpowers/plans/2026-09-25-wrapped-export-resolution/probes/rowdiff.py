"""P6 row-level diff of call-stats --dump-sites between base and prototype.
Key = (caller fid, source_span file/start_byte/end_byte, callee_text). Reports every changed row."""
import json, sys, collections
def load(p):
    out = {}
    for l in open(p):
        r = json.loads(l)
        if r.get('record_kind') != 'call_site': continue
        c = r['caller']; s = r['source_span']
        k = (c['file'], c['name'], c['start_line'], s['file'], s['start_byte'], s['end_byte'], r['callee_text'])
        out[k] = (r.get('drop'), tuple(sorted((t['function_id']['file'], t['function_id']['name'], t['function_id']['start_line'], t['function_id']['end_line'], t['confidence'], t['kind']) for t in r.get('resolved_targets') or [])))
    return out
a, b = load(sys.argv[1]), load(sys.argv[2])
print('rows base', len(a), 'proto', len(b), 'keys only-in-base', len(set(a) - set(b)), 'only-in-proto', len(set(b) - set(a)))
ch = [(k, a[k], b[k]) for k in sorted(set(a) & set(b)) if a[k] != b[k]]
trans = collections.Counter()
tgt = collections.Counter()
for k, x, y in ch:
    trans[(x[0], tuple((t[4], t[5]) for t in x[1]), y[0], tuple((t[4], t[5]) for t in y[1]))] += 1
    for t in y[1]: tgt[(t[0], t[1], t[2], t[3], t[4])] += 1
print('changed rows', len(ch))
for t, n in trans.most_common(): print(' ', n, t)
print('targets gained:')
for t, n in tgt.most_common(): print('  %4d %s' % (n, t))
json.dump([{'key': list(k), 'base': [x[0], list(x[1])], 'proto': [y[0], list(y[1])]} for k, x, y in ch], open(sys.argv[3], 'w'), indent=0)
