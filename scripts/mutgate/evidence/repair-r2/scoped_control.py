"""Nonempty fn-scope control: synthetic changed lines, real builds and tests."""
import json
from pathlib import Path
import sys
import time
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(ROOT / 'scripts/mutgate'))
import mutgate

changed = {'src/js_paths_boundary.rs': {169}}
args = ['mutgate', '--since', 'origin/main', '--scope', 'fn', '--mode', 'schema',
        '--out', str(HERE / 'scoped')]
t = time.monotonic()
with patch.object(mutgate, 'changed_lines', return_value=changed), patch.object(sys, 'argv', args):
    status = mutgate.main()
wall = time.monotonic() - t
summary = json.loads((HERE / 'scoped/summary.json').read_text())
ids = ['I17-no-project-reference-cut', 'I73-declare-prefilter', 'R4-08-dropped-import-decline']
assert set(summary['results']) == set(ids) and summary['selected'] == summary['killed'] == summary['admissible'] == 3
assert status == 0 and wall < 300
receipt = {'exit': status, 'wall_seconds': wall, 'synthetic_changed_lines': {f: sorted(ls) for f, ls in changed.items()},
           'ids': ids, 'production_diff': 'empty: this is a synthetic selector control, not an actual source diff'}
(HERE / 'scoped-control.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps(receipt))
