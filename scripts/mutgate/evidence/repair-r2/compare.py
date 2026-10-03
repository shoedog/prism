"""Verify full populations, provenance, admissibility, verdicts and timings."""
import collections
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
provenance = json.loads((HERE / 'provenance.json').read_text())
for file, expected in provenance['files_sha256'].items():
    assert hashlib.sha256((ROOT / file).read_bytes()).hexdigest() == expected, file
for file, expected in provenance['production_sha256'].items():
    assert hashlib.sha256((ROOT / file).read_bytes()).hexdigest() == expected, file
compiler = provenance['compiler']
assert hashlib.sha256(Path(compiler['path']).read_bytes()).hexdigest() == compiler['sha256']
ids = list(json.loads((ROOT / 'mutants/lane-p-tsconfig-paths.json').read_text())['mutations'])
assert len(ids) == 93
receipts = {mode: json.loads((HERE / mode / 'summary.json').read_text()) for mode in ('text', 'schema')}
walls = {}
for mode, receipt in receipts.items():
    assert set(receipt['results']) == set(ids) and receipt['selected'] == 93, mode
    assert receipt['killed'] == sum(r['verdict'] == 'KILLED' for r in receipt['results'].values()), mode
    assert receipt['admissible'] == sum(r['admissible'] for r in receipt['results'].values()), mode
    assert len(receipt['baselines']['source']) == 54, mode
    for baseline in receipt['baselines'].values():
        assert all(r['admissible'] and not r['killed'] and not r['timeout'] for r in baseline.values()), mode
    wall = re.findall(r'^real ([0-9.]+)$', (HERE / (mode + '.log')).read_text(), re.M)
    assert len(wall) == 1, mode
    walls[mode] = float(wall[0])
assert len(receipts['schema']['baselines']['schema']) == 54
assert all(r['mode'] == 'text' for r in receipts['text']['results'].values())
rows = []
for mid in ids:
    text = receipts['text']['results'][mid]; schema = receipts['schema']['results'][mid]
    rows.append({'id': mid, 'text': text['verdict'], 'schema': schema['verdict'],
                 'text_admissible': text['admissible'], 'schema_admissible': schema['admissible'],
                 'schema_execution': schema['mode'],
                 'equal': (text['verdict'], text['admissible']) == (schema['verdict'], schema['admissible'])})
differences = [r for r in rows if not r['equal']]
modes = dict(collections.Counter(r['mode'] for r in receipts['schema']['results'].values()))
scoped = json.loads((HERE / 'scoped-control.json').read_text())
comparison = {'compared': len(rows), 'differences': differences, 'wall_seconds': walls,
              'schema_execution_counts': modes, 'text_confirmations': receipts['schema']['text_confirmations'],
              'demotions': receipts['schema']['static'], 'scoped_control_wall_seconds': scoped['wall_seconds'],
              'source_baseline_selectors': 54, 'schema_baseline_selectors': 54, 'rows': rows}
(HERE / 'comparison.json').write_text(json.dumps(comparison, indent=2) + '\n')
table = ['| Mutant | Text | Schema | Schema execution | Admissible (both) | Compared |',
         '|---|---|---|---|---|---|']
table += [f"| {r['id']} | {r['text']} | {r['schema']} | {r['schema_execution']} | {'yes' if r['text_admissible'] and r['schema_admissible'] else 'no'} | {'identical' if r['equal'] else 'DIFFERENT'} |" for r in rows]
(HERE / 'certification-table.md').write_text('\n'.join(table) + '\n')
demotions = ['| Mutant | Why text mode |', '|---|---|',
            '| I44-no-module-types | serde attribute edit on a struct field, outside a function body |',
            '| I45-no-module-typings | serde attribute edit on a struct field, outside a function body |',
            '| I48-no-module-main | serde attribute edit on a struct field, outside a function body |',
            '| R4-08-dropped-import-decline | extra edit adds a struct field; all edits of the mutant must share text mode |']
(HERE / 'demotion-table.md').write_text('\n'.join(demotions) + '\n')
print(json.dumps({k: v for k, v in comparison.items() if k != 'rows'}, indent=2))
assert not differences, 'ID-bound verdict/admissibility difference'
assert all(r['text'] == r['schema'] == 'KILLED' and r['text_admissible'] and r['schema_admissible'] for r in rows)
assert modes == {'schema': 89, 'text': 4} and receipts['schema']['text_confirmations'] == 0
assert walls['schema'] <= 180 and all(w <= 900 for w in walls.values()) and scoped['wall_seconds'] < 300
