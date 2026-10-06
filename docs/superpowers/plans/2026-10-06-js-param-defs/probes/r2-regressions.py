#!/usr/bin/env python3
"""R2 source-only call-exposure and same-environment E7/with parity controls."""
import json
import subprocess
from pathlib import Path

P = Path(__file__).resolve().parent
E = Path.home() / 'prism-evidence/js-param-defs'
OUT = E / 'repair-r2/reviewer-controls'
OUT.mkdir(parents=True, exist_ok=True)
CACHE = E / 'cache'
TS = Path.home() / 'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
BINS = {'base': E / 'repair-r1/bin/prism-base-c8de720b',
        'r1': E / 'repair-r1/bin/prism-head-r1',
        'head': E / 'repair-r2/bin/prism-head-r2'}
DUMPERS = {'base': E / 'repair-r1/bin/prism-base-bytes',
           'head': E / 'repair-r2/bin/prism-head-r2-bytes'}


def run(cmd, output):
    with output.open('wb') as stream, Path(str(output) + '.stderr').open('wb') as errors:
        result = subprocess.run([str(x) for x in cmd], stdout=stream, stderr=errors)
    if result.returncode:
        raise RuntimeError((cmd, result.returncode, output.read_text()[-1000:]))
    return output


def nav(side, repo, query, output, *args):
    return run([BINS[side], 'nav', '--cache-dir', CACHE, query, '--repo', repo, *args], output)


summary = {}
for ext in ['js', 'ts', 'tsx']:
    for name, target, call, prefix in [
        ('w2m', 'const obj = { Array(s) {\n sink(s);\n} };\n', 'Array', ''),
        ('w2d', 'export function escape(s) {\n sink(s);\n}\n', 'escape', 'export '),
    ]:
        repo = OUT / f'{name}-{ext}'
        repo.mkdir(exist_ok=True)
        (repo / f'callee.{ext}').write_text(target)
        (repo / f'input.{ext}').write_text(f'{prefix}const entry = input =>\n {call}(input);\n')
        outcomes = {}
        for side in ['base', 'r1', 'head']:
            output = OUT / f'{name}-{ext}.{side}.trace.json'
            nav(side, repo, 'taint-reaches', output, '--source', f'input.{ext}:1',
                '--sink', f'callee.{ext}:2', '--format', 'json')
            outcomes[side] = json.loads(output.read_text())['reasoning']['reachability']
            nav(side, repo, 'call-stats', OUT / f'{name}-{ext}.{side}.sites.jsonl', '--dump-sites')
        assert outcomes == {'base': 'NotReached', 'r1': 'Reached', 'head': 'NotReached'}, outcomes
        assert len({(OUT / f'{name}-{ext}.{side}.sites.jsonl').read_bytes()
                    for side in ['base', 'r1', 'head']}) == 1
        summary[f'{name}-{ext}'] = outcomes

for name, suffix, key in [
    ('jsx', 'q(<C x={1} />, function () {\n return x; });\n', 'x={'),
    ('pair', 'q({ x: 1 }, function () {\n return x; });\n', 'x: '),
]:
    wrong = {}
    for shape, side, formal in [('plain', 'base', '(x)'), ('bare', 'head', 'x')]:
        repo = OUT / f'{name}-{shape}'
        repo.mkdir(exist_ok=True)
        source = f'const g = {formal} => {suffix}'
        (repo / 'case.tsx').write_text(source)
        raw = run([DUMPERS[side], repo], OUT / f'{name}-{shape}.bytes.jsonl')
        key_start = source.index(key)
        rows = [r for r in map(json.loads, raw.read_text().splitlines())
                if r['from']['parameter'] and r['from']['path']['base'] == 'x'
                and (r['to']['start_byte'], r['to']['end_byte']) == (key_start, key_start + 1)]
        assert len(rows) == 1, (name, shape, rows)
        changed = OUT / f'{name}-{shape}.changed.jsonl'
        changed.write_text(''.join(json.dumps({'class': 'CONTROL',
            'row': {k: v for k, v in r.items() if k not in ['confidence', 'doubt', 'kill_line']},
            'head_label': {k: r.get(k) for k in ['confidence', 'doubt', 'kill_line']}}) + '\n' for r in rows))
        adjudicated = OUT / f'{name}-{shape}.adj.json'
        run(['node', P / 'adjudicate.cjs', TS, repo, changed, adjudicated,
             '--details', OUT / f'{name}-{shape}.details.jsonl'], OUT / f'{name}-{shape}.stdout')
        wrong[shape] = json.loads(adjudicated.read_text())
        assert wrong[shape].get('CONTROL|def->use|WRONG|declaration_span_mismatch') == 1
        nav(side, repo, 'dfg-stats', OUT / f'{name}-{shape}.wire.jsonl', '--edges')
    assert (OUT / f'{name}-plain.wire.jsonl').read_bytes() == (OUT / f'{name}-bare.wire.jsonl').read_bytes()
    summary[f'E7-{name}'] = wrong

for shape, side, formal in [('plain', 'base', '(x)'), ('bare', 'head', 'x')]:
    repo = OUT / f'with-{shape}'
    repo.mkdir(exist_ok=True)
    (repo / 'case.js').write_text(f'const g = {formal} => {{ with (obj) {{\n sink(x);\n}} }};\n')
    output = nav(side, repo, 'dfg-stats', OUT / f'with-{shape}.wire.jsonl', '--edges')
    rows = [r for r in map(json.loads, output.read_text().splitlines()) if r['from']['path']['base'] == 'x']
    assert rows and all(r['confidence'] == 'nameonly' for r in rows), rows
assert (OUT / 'with-plain.wire.jsonl').read_bytes() == (OUT / 'with-bare.wire.jsonl').read_bytes()
summary['with'] = 'JS head bare row is NameOnly and identical to same-environment plain-formal base'
(OUT / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary))
