# usage: cmp3.py CORPUS_DIR OUTDIR  (expects OUTDIR/{main,5bc,fold,nocap,nocensus,nooff}.jsonl)
import sys, json, collections, random, re, os
corpus, out = sys.argv[1:3]
LAB = ('confidence', 'doubt', 'kill_line', 'loop_carried')
def load(name):
    d = collections.defaultdict(list)
    p = f'{out}/{name}.jsonl'
    if not os.path.exists(p): return None
    with open(p, 'rb') as f:
        for l in f:
            e = json.loads(l); lab = '/'.join(f'{k}={e.pop(k)}' for k in LAB if k in e)
            d[json.dumps(e, sort_keys=True)].append(lab)
    return d
def cnt(d): return {k: len(v) for k, v in d.items()}
M, B, F = load('main'), load('5bc3fdb3'), load('fold3')
abl = {n: load(n) for n in ('nocap', 'nocensus', 'nooff')}
cm, cb, cf = cnt(M), cnt(B), cnt(F)
def diff(a, b):  # multiset a - b
    r = {}
    for k, n in a.items():
        m = n - b.get(k, 0)
        if m > 0: r[k] = m
    return r
restored = diff(cf, cb); newly_removed = diff(cb, cf)
removed_vs_main = diff(cm, cf); added_vs_main = diff(cf, cm)
lines = {}
def src(file, n):
    if file not in lines:
        try: lines[file] = open(os.path.join(corpus, file), errors='replace').read().split('\n')
        except Exception: lines[file] = []
    L = lines[file]
    return L[n - 1].strip() if 0 < n <= len(L) else ''
def row(k, extra=None):
    e = json.loads(k); f, t = e['from'], e['to']
    r = {'file': f['file'], 'path': f['path']['base'] + ''.join('.' + x for x in f['path']['fields']),
         'def_line': f['line'], 'use_line': t['line'], 'def_src': src(f['file'], f['line']), 'use_src': src(f['file'], t['line'])}
    if extra: r.update(extra)
    return r
cap = re.compile(r'\{(%s)(:[^}]*)?\}')
res = {'main_edges': sum(cm.values()), '5bc_edges': sum(cb.values()), 'fold_edges': sum(cf.values()),
       'vs_main_removed': sum(removed_vs_main.values()), 'vs_main_added': sum(added_vs_main.values()),
       'vs_5bc_restored': sum(restored.values()), 'vs_5bc_newly_removed': sum(newly_removed.values())}
# labels on common keys vs main (kept-label transitions)
trans = 0
for k in cf:
    if k in cm and cf[k] == cm[k] and sorted(F[k]) != sorted(M[k]): trans += cf[k]
res['vs_main_kept_label_transitions'] = trans
# cause attribution of restored edges
by_cause = collections.Counter(); rows = []
for k, n in restored.items():
    causes = []
    for name, d in abl.items():
        if d is None: continue
        if k not in d or d[k].__len__() < cf[k]: causes.append({'nocap': 'format_capture', 'nocensus': 'census_refusal', 'nooff': 'owner_off_belt'}[name])
    r = row(k, {'causes': causes, 'n': n})
    base = r['path'].split('.')[0]
    window = '\n'.join(src(r['file'], i) for i in range(r['use_line'], r['use_line'] + 4))
    r['capture_text_match'] = bool(re.search(cap.pattern % re.escape(base), window))
    rows.append(r); by_cause[','.join(causes) or 'unattributed'] += n
res['restored_by_cause'] = dict(by_cause)
res['restored_with_capture_text_of_def_name'] = sum(r['n'] for r in rows if r['capture_text_match'])
res['restored_capture_edges_attributed_to_capture'] = sum(r['n'] for r in rows if 'format_capture' in r['causes'])
json.dump(res, open(f'{out}/summary.json', 'w'), indent=1)
rng = random.Random(7943)
def sample(d, n, extra):
    ks = sorted(d); picked = rng.sample(ks, min(n, len(ks)))
    return [row(k, extra) for k in picked]
with open(f'{out}/samples-fold3.jsonl', 'w') as f:
    for r in sample(removed_vs_main, 20, {'set': 'removed_vs_main'}): f.write(json.dumps(r) + '\n')
with open(f'{out}/samples-newly-removed-vs-5bc.jsonl', 'w') as f:
    for r in sample(newly_removed, 20, {'set': 'newly_removed_vs_5bc'}): f.write(json.dumps(r) + '\n')
with open(f'{out}/restored-vs-5bc.jsonl', 'w') as f:
    for r in rows: f.write(json.dumps(r) + '\n')
with open(f'{out}/newly-removed-vs-5bc-all.jsonl', 'w') as f:
    for k in sorted(newly_removed): f.write(json.dumps(row(k)) + '\n')
print(json.dumps(res, indent=1))
