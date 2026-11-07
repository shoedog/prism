# usage: add.py MAIN HEAD OUT.json -- addition-only check, added-edge classes and labels, kept-label transitions
import sys, json, collections, random
a, b, out = sys.argv[1:4]
LAB = ('confidence', 'doubt', 'kill_line', 'loop_carried')
def load(p):
    d = collections.defaultdict(list)
    with open(p, 'rb') as f:
        for l in f:
            e = json.loads(l); lab = '/'.join(f'{k}={e.pop(k)}' for k in LAB if k in e)
            d[json.dumps(e, sort_keys=True)].append(lab)
    return d
ma, hb = load(a), load(b)
main_only = 0; added = []; trans = collections.Counter(); tex = []; cls = collections.Counter(); labs = collections.Counter()
def prefix(e):
    f, t = e['from']['path'], e['to']['path']
    return f['base'] == t['base'] and len(t['fields']) < len(f['fields']) and f['fields'][:len(t['fields'])] == t['fields'] \
        and e['from']['file'] == e['to']['file']
for k in set(ma) | set(hb):
    x, y = collections.Counter(ma.get(k, [])), collections.Counter(hb.get(k, []))
    nx, ny = sum(x.values()), sum(y.values())
    if nx > ny: main_only += nx - ny
    if ny > nx:
        e = json.loads(k); c = 'prefix' if prefix(e) else 'other'
        for lab in sorted((y - x).elements())[: ny - nx]:
            added.append((c, lab, k)); cls[c] += 1; labs[lab.split('/kill_line')[0]] += 1
    elif nx == ny and x != y:
        for p, q in zip(sorted((x - y).elements()), sorted((y - x).elements())):
            trans[f'{p} -> {q}'] += 1; tex.append({'main': p, 'head': q, 'edge': json.loads(k)})
res = {'main_edges': sum(map(len, ma.values())), 'head_edges': sum(map(len, hb.values())), 'main_only': main_only,
       'added': len(added), 'added_classes': dict(cls), 'added_labels': labs.most_common(),
       'kept_label_transitions': trans.most_common(50), 'kept_transition_edges': tex[:100],
       'added_top_files': collections.Counter(json.loads(k)['from']['file'] for _, _, k in added).most_common(20)}
json.dump(res, open(out, 'w'), indent=1)
rng = random.Random(639); added.sort(key=lambda r: r[2])
with open(out.replace('-add.json', '-added-sample.jsonl'), 'w') as f:
    for c, lab, k in rng.sample(added, min(40, len(added))): f.write(json.dumps({'class': c, 'label': lab, 'edge': json.loads(k)}) + '\n')
    for c, lab, k in [r for r in added if r[0] == 'other'][:20]: f.write(json.dumps({'class': c, 'label': lab, 'edge': json.loads(k)}) + '\n')
import gzip
with gzip.open(out.replace('-add.json', '-added-all.jsonl.gz'), 'wt') as f:
    for c, lab, k in added: f.write(json.dumps({'class': c, 'label': lab, 'edge': json.loads(k)}) + '\n')
print(out, 'main_only', main_only, 'added', dict(cls), 'kept transitions', sum(trans.values()))
