#!/usr/bin/env python3
"""r2-opus-A F5 payload-specific check for traced SecBench rows (planner probe).

For every `traced` entry in RUN/entries.jsonl, BFS the raw witness graph from each payload
formal Def node, never entering a node on the source signature line whose path base is a
different identifier (a sibling formal). The credit is payload-specific when a Reached
parameter_result's graph_node is reachable that way.
Usage: payload_bfs.py RUN_DIR
"""
import gzip
import json
import sys
from collections import deque
from pathlib import Path


def base(path):
    return path.split('.')[0].split('[')[0]


def main():
    run = Path(sys.argv[1])
    out = []
    for line in (run / 'entries.jsonl').read_bytes().splitlines():
        row = json.loads(line)
        if row.get('outcome') != 'traced':
            continue
        witness = json.loads(gzip.open(run / 'raw' / row['class'] / row['entry'] / 'witness.json.gz').read())
        nodes = witness['graph']['nodes']
        succ = {}
        for e in witness['graph']['edges']:
            succ.setdefault(e['from'], []).append(e['to'])
        params = row['source']['data_parameters']
        names = {n for p in params for n in p['names']}
        src_lines = {p['line'] for p in params}

        def var(i):
            return nodes[i]['symbol'].get('Variable', {})

        starts = [i for i in range(len(nodes)) if var(i).get('access') == 'def' and var(i).get('line') in src_lines
                  and base(var(i).get('path', '')) in names]
        targets = {r['graph_node'] for r in row['trace_detail']['parameter_results']
                   if r['verdict'] == 'Reached' and r['graph_node'] is not None and base(r['source']['path']) in names}
        seen, queue = set(starts), deque(starts)
        while queue:
            i = queue.popleft()
            for j in succ.get(i, []):
                v = var(j)
                if j in seen or (v.get('line') in src_lines and base(v.get('path', '')) not in names):
                    continue
                seen.add(j)
                queue.append(j)
        out.append({'entry': row['entry'], 'payload_defs': len(starts), 'payload_specific': bool(targets & seen)})
    for r in out:
        print(json.dumps(r, sort_keys=True))
    print(json.dumps({'traced': len(out), 'payload_specific': sum(r['payload_specific'] for r in out)}))


if __name__ == '__main__':
    main()
