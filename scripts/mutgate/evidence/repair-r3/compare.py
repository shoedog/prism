"""Verify source binding and all three full-gate receipts without building."""
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
inputs = json.loads((HERE / 'certification-inputs.json').read_text())
assert subprocess.check_output(['git', '-C', str(ROOT), 'rev-parse', 'HEAD'], text=True).strip() == inputs['head']
assert subprocess.check_output(['git', '-C', str(ROOT), 'diff', 'origin/main', '--', 'src'], text=True) == ''
mismatches = [name for name, digest in inputs['sha256'].items()
              if hashlib.sha256((ROOT / name).read_bytes()).hexdigest() != digest]
assert not mismatches, mismatches
ts = inputs['typescript']
assert hashlib.sha256(Path(ts['path']).read_bytes()).hexdigest() == ts['sha256']
prior_inputs = json.loads((HERE.parent / 'repair-r2/provenance.json').read_text())
assert all(hashlib.sha256((ROOT / name).read_bytes()).hexdigest() == digest
           for name, digest in prior_inputs['production_sha256'].items())
lane = 'mutants/lane-p-tsconfig-paths.json'
assert inputs['sha256'][lane] == prior_inputs['files_sha256'][lane]
previous = json.loads((HERE.parent / 'repair-r2/text/summary.json').read_text())
assert len(previous['results']) == 93
reports = {name: json.loads((HERE / name / 'summary.json').read_text()) for name in ('jobs1', 'jobs2', 'jobs4')}
for name, summary in reports.items():
    assert set(summary['results']) == set(previous['results'])
    assert summary['selected'] == summary['killed'] == summary['admissible'] == 93
    assert summary['authoritative'] and summary['execution_mode'] == 'text'
    assert all(result['verdict'] == 'KILLED' and result['mode'] == 'text'
               and result['admissible'] == previous['results'][mid]['admissible']
               and result['verdict'] == previous['results'][mid]['verdict']
               for mid, result in summary['results'].items())
    assert all(run['admissible'] and not run['killed'] for run in summary['baselines']['source'].values())
    assert len(summary['baselines']['source']) == 54 and summary['text_confirmations'] == 0
    assert len(summary['baselines']['workers']) == summary['resources']['workers']
    assert all('error' not in worker and worker['runs']
               and all(run['admissible'] and not run['killed'] for run in worker['runs'].values())
               for worker in summary['baselines']['workers'].values())
    assert summary['resources']['workers_cleaned']
scoped = json.loads((HERE / 'scoped/summary.json').read_text())
assert scoped['selected'] == scoped['killed'] == scoped['admissible'] == 3
assert scoped['authoritative'] is False
empty = json.loads((HERE / 'scoped-empty/summary.json').read_text())
assert empty['selected'] == 0 and empty['authoritative'] is False
totals = json.loads((HERE / 'suite-totals.json').read_text())
assert totals['failed'] == 0 and totals['passed'] > 0
rows = ['# 93-ID certification against r2 pure text', '', '| ID | r2 text | jobs 1 | jobs 2 | jobs 4 |',
        '|---|---|---|---|---|']
for mid in previous['results']:
    rows.append(f'| {mid} | KILLED | KILLED | KILLED | KILLED |')
(HERE / 'certification-table.md').write_text('\n'.join(rows) + '\n')
receipt = {'bound_files': len(inputs['sha256']), 'hash_mismatches': mismatches,
           'full_gate_ids': 93, 'id_bound_verdict_or_admissibility_differences': 0,
           'jobs': {name: {'timings': summary['timings'], 'resources': summary['resources']}
                    for name, summary in reports.items()}, 'suite_totals': totals,
           'scoped_synthetic_changed_lines': {'src/js_paths_boundary.rs': [169]},
           'scoped_ids': list(scoped['results']), 'real_scope_selected': 0}
(HERE / 'comparison.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps(receipt, indent=2))
