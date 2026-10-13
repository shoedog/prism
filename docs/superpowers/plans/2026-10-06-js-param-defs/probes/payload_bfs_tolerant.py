#!/usr/bin/env python3
"""payload_bfs.py (r2-opus-A F5) over callee-TOLERANT traced rows (PR-B probe; repository root).
Recomputes the harness classify() detail with empty callees, then applies the identical
payload-specific BFS. Usage: payload_bfs_tolerant.py RUN_DIR"""
import gzip, json, sys
from collections import deque
from pathlib import Path
sys.path.insert(0, str(Path.cwd()))
from eval.secbench.run import classify  # noqa: E402
def base(p): return p.split('.')[0].split('[')[0]
def main():
    run = Path(sys.argv[1]); out = []
    for line in (run / 'entries.jsonl').read_bytes().splitlines():
        row = json.loads(line)
        if row.get('callee_tolerant_outcome') != 'traced': continue
        raw = run / 'raw' / row['class'] / row['entry']
        witness = json.loads(gzip.open(raw / 'witness.json.gz').read()); frontier = json.loads(gzip.open(raw / 'frontier.json.gz').read())
        outcome, detail = classify(row, witness, {'items': []}, frontier)
        assert outcome == 'traced'
        nodes = witness['graph']['nodes']; succ = {}
        for e in witness['graph']['edges']: succ.setdefault(e['from'], []).append(e['to'])
        params = row['source']['data_parameters']; names = {n for p in params for n in p['names']}; src_lines = {p['line'] for p in params}
        var = lambda i: nodes[i]['symbol'].get('Variable', {})
        starts = [i for i in range(len(nodes)) if var(i).get('access') == 'def' and var(i).get('line') in src_lines and base(var(i).get('path', '')) in names]
        targets = {r['graph_node'] for r in detail['parameter_results'] if r['verdict'] == 'Reached' and r['graph_node'] is not None and base(r['source']['path']) in names}
        seen, q = set(starts), deque(starts)
        while q:
            i = q.popleft()
            for j in succ.get(i, []):
                v = var(j)
                if j in seen or (v.get('line') in src_lines and base(v.get('path', '')) not in names): continue
                seen.add(j); q.append(j)
        owners = sorted({var(i).get('function') for i in starts})
        out.append({'entry': row['entry'], 'payload_defs': len(starts), 'payload_specific': bool(targets & seen), 'source_owners': owners})
    for r in out: print(json.dumps(r, sort_keys=True))
    print(json.dumps({'traced': len(out), 'payload_specific': sum(r['payload_specific'] for r in out),
                      'synthetic_source_owner': sum(any(str(o).startswith('<cb@') for o in r['source_owners']) for r in out)}))
if __name__ == '__main__': main()
