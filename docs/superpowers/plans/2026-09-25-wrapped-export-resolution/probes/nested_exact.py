"""P5: Exact import_member edges whose target FunctionId is lexically nested inside another function
(candidate F4-class false Exacts via the unverified export-list/default-identifier Local path).
Usage: python3 nested_exact.py <dump-sites.jsonl> <functions.json>"""
import json, sys, collections
dump, fnj = sys.argv[1], sys.argv[2]
fns = json.load(open(fnj))
by_file = collections.defaultdict(list)
for f in fns: by_file[f['file']].append(f)
def nested(fid):
    for g in by_file[fid['file']]:
        if (g['start_line'], g['end_line'], g['name']) == (fid['start_line'], fid['end_line'], fid['name']): continue
        if g['start_line'] <= fid['start_line'] and fid['end_line'] <= g['end_line'] and (g['start_line'], g['end_line']) != (fid['start_line'], fid['end_line']):
            return g
    return None
tot = 0; hits = []
for l in open(dump):
    r = json.loads(l)
    if r.get('record_kind') != 'call_site': continue
    for t in r.get('resolved_targets') or []:
        if t['kind'] == 'import_member' and t['confidence'] == 'exact':
            tot += 1
            g = nested(t['function_id'])
            if g: hits.append((r['source_span']['file'], r['source_span']['line'], r['callee_text'], t['function_id'], g['name'], g['start_line'], g['end_line']))
print('exact import_member edges', tot, 'targets nested in another function', len(hits))
for h in hits[:30]: print(' ', h)
