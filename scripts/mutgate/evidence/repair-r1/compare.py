#!/usr/bin/env python3
"""Replay the round-1 certification comparison; run from the repository root."""
import hashlib
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
lane = json.loads(Path('mutants/lane-p-tsconfig-paths.json').read_text())
ids = list(lane['mutations'])
assert len(ids) == 93
receipts = {mode: json.loads((HERE / mode / 'summary.json').read_text())
            for mode in ('text', 'schema')}
provenance = json.loads((HERE / 'provenance.json').read_text())
for file, expected in provenance['files_sha256'].items():
    assert hashlib.sha256(Path(file).read_bytes()).hexdigest() == expected, file
for file, expected in provenance['production_sha256'].items():
    assert hashlib.sha256(Path(file).read_bytes()).hexdigest() == expected, file
for mode, receipt in receipts.items():
    assert receipt['selected'] == 93, mode
    assert set(receipt['results']) == set(ids), mode
    assert receipt['killed'] == sum(r['verdict'] == 'KILLED' for r in receipt['results'].values()), mode
    assert receipt['admissible'] == sum(r['admissible'] for r in receipt['results'].values()), mode
    assert receipt['baselines']['source'], mode
    for baseline in receipt['baselines'].values():
        assert all(r['admissible'] and not r['killed'] and not r['timeout'] for r in baseline.values()), mode
assert receipts['schema']['baselines']['schema']

rows = []
differences = []
for mid in ids:
    text = receipts['text']['results'][mid]
    schema = receipts['schema']['results'][mid]
    equal = (text['verdict'], text['admissible']) == (schema['verdict'], schema['admissible'])
    rows.append({'id': mid, 'text': text['verdict'], 'schema': schema['verdict'],
                 'text_admissible': text['admissible'], 'schema_admissible': schema['admissible'],
                 'schema_execution': schema['mode'], 'equal': equal})
    if not equal:
        differences.append(rows[-1])
walls = {}
for mode in receipts:
    matches = re.findall(r'^real ([0-9.]+)$', (HERE / (mode + '.log')).read_text(), re.M)
    assert len(matches) == 1, mode
    walls[mode] = float(matches[0])
comparison = {'rows': rows, 'compared': len(rows), 'differences': differences, 'wall_seconds': walls,
              'source_baseline_selectors': {m: len(r['baselines']['source']) for m, r in receipts.items()},
              'schema_baseline_selectors': len(receipts['schema']['baselines']['schema']),
              'schema_candidates': sum(r['mode'] == 'schema+text' for r in receipts['schema']['results'].values())}
(HERE / 'comparison.json').write_text(json.dumps(comparison, indent=2))
table = ['| Mutant | Text | Schema | Admissible (both) | Compared |',
         '|---|---|---|---|---|']
table += [f"| {r['id']} | {r['text']} | {r['schema']} | {'yes' if r['text_admissible'] and r['schema_admissible'] else 'no'} | {'identical' if r['equal'] else 'DIFFERENT'} |"
          for r in rows]
(HERE / 'certification-table.md').write_text('\n'.join(table) + '\n')
print(json.dumps({k: v for k, v in comparison.items() if k != 'rows'}, indent=2))
assert not differences, 'verdict/admissibility differences are defects'
assert all(r['text'] == r['schema'] == 'KILLED' and r['text_admissible'] and r['schema_admissible'] for r in rows)
assert all(w <= 900 for w in walls.values()), 'full gate exceeded 15 minutes'
