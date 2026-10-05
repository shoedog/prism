#!/usr/bin/env python3
"""Offline SecBench ground-truth measurement. No package/exploit execution."""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

REPO = Path(__file__).resolve().parents[2]
EVIDENCE = Path('/Users/wesleyjinks/prism-evidence')
SEC_COMMIT = '5d362353550a8baa42bba34edd26e5fb86d41b60'
SUT_COMMIT = '4e592daa7858a195eb3a9eb77c83dfbc763b49fa'
COMPILER = EVIDENCE / 'inputs/excalidraw-0642e72c-installed/source/node_modules/typescript/lib/typescript.js'
OUTCOMES = ('traced', 'reached_function_only', 'partial', 'not_reached', 'prism_error')
WEIGHTS = {'code-injection': 3, 'command-injection': 3, 'prototype-pollution': 2,
           'path-traversal': 2, 'redos': 1}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(value) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()


def workstream(category: str):
    return {'A': 3, 'B': 2, 'C': 3, 'D': 6, 'E': 4, 'F': 5, 'G': 6, 'H': 6}.get(category.split('-')[0])


def variable(symbol):
    return symbol.get('Variable', {}) if isinstance(symbol, dict) else {}


def base_name(value: str) -> str:
    return re.split(r'[.\[]', value)[0]


def data_root(v: dict, source: dict) -> bool:
    if v.get('file') != source['file'] or v.get('access') != 'def':
        return False
    for p in source['data_parameters']:
        if base_name(v.get('path', '')) in p['names'] and v.get('line') == p['line']:
            # Byte identity binds the input parameter rather than another def on
            # a minified line. Synthetic/missing byte ranges receive no credit.
            if p['start_byte'] <= v.get('start_byte', -1) < p['end_byte']:
                return True
    return False


def sink_value(v: dict, sink: dict) -> bool:
    return v.get('file') == sink['file'] and v.get('line') == sink['line'] \
        and any(v.get('path', '') == occurrence.get('path', occurrence['name'])
                and (v.get('start_byte') == occurrence['start_byte']
                     or v.get('start_byte') == v.get('end_byte')
                     and occurrence.get('line_occurrences') == 1)
                for occurrence in sink['value_occurrences'])


def classify(row: dict, witness: dict, callees: dict, frontier: dict) -> tuple[str, dict]:
    if not isinstance(witness.get('reasoning'), dict) or not isinstance(witness['reasoning'].get('per_sink'), list):
        raise ValueError('missing taint witness schema')
    accepted, rejected = [], []
    for s in witness['reasoning']['per_sink']:
        target = variable(s.get('sink'))
        if not sink_value(target, row['sink']):
            continue
        for src in s.get('sources', []):
            v = variable(src.get('source'))
            if data_root(v, row['source']):
                accepted.append({'source': v, 'sink': target, 'verdict': src['reachability'],
                                 'graph_node': src.get('graph_node')})
            elif src.get('reachability') == 'Reached':
                rejected.append({'source': v, 'sink': target, 'reason': 'not_a_test_fed_data_parameter'})
    detail = {'parameter_results': accepted, 'unrelated_reached_roots': rejected,
              'warnings': witness.get('warnings', []),
              'aggregate_reachability': witness['reasoning'].get('reachability')}
    if any(s['verdict'] == 'Reached' for s in accepted):
        return 'traced', detail
    functions = [(i.get('symbol') or {}).get('Function', {}) for i in callees.get('items', [])]
    own = row['source']['file'] == row['sink']['file'] and row['source']['start_line'] <= row['sink']['line'] <= row['source']['end_line']
    reached = own or any(f.get('file') == row['sink']['file'] and f.get('start_line', 0) <= row['sink']['line'] <= f.get('end_line', -1) for f in functions)
    # These are call-graph enclosing spans, explicitly weaker than a value path.
    if reached:
        return 'reached_function_only', detail
    if any(s['verdict'] in ('BoundaryExited', 'Sanitized') for s in accepted):
        return 'partial', detail
    if any(variable(i.get('symbol')).get('line', 0) > row['source']['start_line']
           for i in frontier.get('items', [])
           if variable(i.get('symbol')).get('file') == row['source']['file']):
        return 'partial', detail
    return 'not_reached', detail


def propose_break(row: dict, frontier: dict) -> dict:
    """First missing path segment heuristic, kept separate from adjudication."""
    source, sink = row['source'], row['sink']
    for p in source['data_parameters']:
        if p['destructure'] or p['rest'] or p['default']:
            category = 'B-destructure' if p['destructure'] else 'B-rest-spread' if p['rest'] else 'B-default'
            return {'category': category, 'file': source['file'], 'line': p['line'],
                    'mechanism': 'complex source binding precedes the missing value path', 'text': p['text']}
    chain = row.get('syntactic_path')
    variables = [variable(i.get('symbol')) for i in frontier.get('items', [])]
    locations = {(v.get('file'), v.get('line')) for v in variables}
    names = {n for p in source['data_parameters'] for n in p['names']}
    # Only consider constructs referring to an actual data parameter at an
    # unreached line of the source callable; no whole-package keyword guessing.
    candidates = [f for f in source['features'] if (source['file'], f['line']) not in locations
                  and any(re.search(r'\b' + re.escape(n) + r'\b', f['text']) for n in names)]
    order = {'C-merge': 0, 'B-destructure': 1, 'B-rest-spread': 2, 'C-dynamic-key': 3,
             'C-member': 4, 'A-loop': 5, 'H-callback/promise/event': 6}
    if candidates:
        first = sorted(candidates, key=lambda f: (f['line'], order.get(f['category'], 10)))[0]
        return {**first, 'file': source['file'], 'mechanism': 'first syntactic data-parameter construct outside observed frontier'}
    if chain:
        for prev, nxt in zip(chain, chain[1:]):
            if not any(v.get('file') == nxt['file'] and nxt['start_line'] <= v.get('line', 0) <= nxt['end_line'] for v in variables):
                category = 'D-cjs' if nxt['file'] != prev['file'] else 'H-callback/promise/event' if nxt['name'] == '<anonymous>' else 'B-argument'
                return {'category': category, 'file': nxt['file'], 'line': nxt['start_line'],
                        'mechanism': 'first syntactic call-chain callable outside observed frontier', 'text': nxt['name']}
    return {'category': 'unresolved', 'file': source['file'], 'line': source['start_line'],
            'mechanism': 'no discriminating first-break evidence; manual investigation required'}


def invoke(binary: Path, args: list[str], raw: Path, label: str, timeout: int, jsonl=False) -> tuple[dict | list | None, dict]:
    command = [str(binary), *args]
    raw.mkdir(parents=True, exist_ok=True)
    stdout = raw / (label + '.stdout')
    stderr = raw / (label + '.stderr')
    error = None
    with stdout.open('wb') as out, stderr.open('wb') as err:
        try:
            process = subprocess.run(command, stdout=out, stderr=err, timeout=timeout,
                                     env={**os.environ, 'RAYON_NUM_THREADS': '2'})
            code = process.returncode
        except subprocess.TimeoutExpired:
            code = None
            error = f'timeout after {timeout}s'
    data, errors = stdout.read_bytes(), stderr.read_bytes()
    record = {'argv': command, 'returncode': code, 'stdout_sha256': digest(data),
              'stderr_sha256': digest(errors), 'stderr': errors.decode('utf-8', errors='replace')[-2000:],
              'raw': str(raw / (label + '.json.gz')), 'error': error}
    with gzip.GzipFile(record['raw'], 'wb', mtime=0) as zipped:
        zipped.write(data)
    stdout.unlink()
    stderr.unlink()
    if code != 0:
        record['error'] = error or f'Prism invocation exited {code}'
        try:
            record['output_error'] = json.loads(data).get('error')
        except (ValueError, UnicodeDecodeError, AttributeError):
            record['output_error'] = None
        return None, record
    try:
        parsed = [json.loads(line) for line in data.splitlines() if line.strip()] if jsonl else json.loads(data)
    except (ValueError, UnicodeDecodeError) as exc:
        record['error'] = f'invalid output schema/JSON: {exc}'
        return None, record
    return parsed, record


def measure(row, binary, packages, output, timeout):
    row = {k: v for k, v in row.items() if k != 'identities'}
    root = packages / row['class'] / row['entry'] / 'src/package'
    raw = output / 'raw' / row['class'] / row['entry']
    nav = ['nav', '--no-cache']
    common = ['--repo', str(root)]
    dfg, rec = invoke(binary, nav + ['dfg-stats'] + common, raw, 'dfg-stats', timeout)
    row['invocations'] = {'dfg-stats': rec}
    row['dfg_stats'] = dfg
    if row['gt_status'] != 'available':
        row['outcome'] = 'gt_unavailable'
        row['first_break'] = {'category': 'unavailable_ground_truth', 'reason': row['gt_reason']}
        return row
    source, sink = row['source'], row['sink']
    sources = sorted(set(f"{source['file']}:{p['line']}" for p in source['data_parameters']))
    source_flags = sum((['--source', s] for s in sources), [])
    loc = f"{source['file']}:{source['start_line']}"
    queries = {
        'witness': nav + ['taint-reaches'] + common + source_flags + ['--sink', f"{sink['file']}:{sink['line']}", '--format', 'json'],
        'frontier': nav + ['taint-reaches'] + common + source_flags + ['--format', 'json'],
        'callees': nav + ['callees'] + common + ['--location', loc, '--depth', '8', '--format', 'json'],
        'callers': nav + ['callers'] + common + ['--location', f"{sink['file']}:{sink['line']}", '--depth', '8', '--format', 'json'],
        'ego': nav + ['ego'] + common + ['--location', loc, '--hops', '2', '--format', 'json'],
    }
    parsed = {}
    for name, command in queries.items():
        parsed[name], row['invocations'][name] = invoke(binary, command, raw, name, timeout)
    # Diff-only APIs are measured as comparison channels. The synthetic diff
    # touches only source parameter declaration lines; source files stay intact.
    diff = raw / 'seed.json'
    diff.write_bytes(canonical({'files': [{'file_path': source['file'], 'modify_type': 'Modified',
                                        'diff_lines': [p['line'] for p in source['data_parameters']]}]}))
    base = ['--repo', str(root), '--diff', str(diff), '--format', 'json']
    for name, command in {
        'taint': base + ['--algorithm', 'taint', '--taint-return-flow'] + sum((['--taint-source', s] for s in sources), []),
        'chop': base + ['--algorithm', 'chop', '--chop-source', sources[0], '--chop-sink', f"{sink['file']}:{sink['line']}"],
    }.items():
        _, row['invocations'][name] = invoke(binary, command, raw, name, timeout)
    decisive = ('witness', 'frontier', 'callees')
    if any(parsed[k] is None for k in decisive):
        row['outcome'] = 'prism_error'
        row['first_break'] = {'category': 'prism_error', 'reason': '; '.join(row['invocations'][k]['error'] or '' for k in decisive if parsed[k] is None)}
    else:
        try:
            row['outcome'], row['trace_detail'] = classify(row, parsed['witness'], parsed['callees'], parsed['frontier'])
            row['heuristic_break'] = propose_break(row, parsed['frontier']) if row['outcome'] != 'traced' else None
            row['first_break'] = row['heuristic_break'] or {'category': 'none'}
            row['attribution_status'] = 'heuristic_unadjudicated' if row['heuristic_break'] else 'not_applicable'
        except (KeyError, ValueError, TypeError) as exc:
            row['outcome'] = 'prism_error'
            row['first_break'] = {'category': 'prism_error', 'reason': f'output contract: {exc}'}
    return row


def verify_inspection(rows, inputs, packages):
    pins = []
    for row in rows:
        root = packages / row['class'] / row['entry'] / 'src/package'
        actual = sorted(str(p.relative_to(root)) for p in root.rglob('*') if p.is_file())
        expected = sorted(f['path'] for f in row['identities'])
        if actual != expected:
            raise ValueError(f'input population drift: {root}')
        for f in row['identities']:
            file = root / f['path']
            if file.is_symlink() or digest(file.read_bytes()) != f['sha256']:
                raise ValueError(f'input bytes drift: {file}')
        for f in row['exploits']:
            if digest((inputs / f['path']).read_bytes()) != f['sha256']:
                raise ValueError(f'exploit bytes drift: {f["path"]}')
        if digest((inputs / row['class'] / row['entry'] / 'package.json').read_bytes()) != row['metadata_sha256']:
            raise ValueError('metadata drift')
        pins.append({'class': row['class'], 'entry': row['entry'], 'tree_sha256': digest(canonical(row['identities'])),
                     'metadata_sha256': row['metadata_sha256'], 'exploits': row['exploits']})
    return pins


def summarize(rows):
    by_class = {}
    for cls in sorted({r['class'] for r in rows}):
        group = [r for r in rows if r['class'] == cls]
        outcomes = Counter(r['outcome'] for r in group)
        breaks = Counter(r.get('first_break', {}).get('category', 'unresolved') for r in group if r['outcome'] in OUTCOMES and r['outcome'] != 'traced')
        confirmed = Counter(r['first_break']['category'] for r in group if r.get('attribution_status') == 'adjudicated')
        syntax = Counter()
        for r in group:
            c = r.get('census', {})
            syntax.update({k: int(v) for k, v in c.items() if isinstance(v, (int, bool))})
            syntax.update({'syntax_' + k: v for k, v in c.get('counts', {}).items()})
            syntax['unparsed_files'] += len(c.get('unparsed_files', []))
        paths = [r['syntactic_path'] for r in group if r.get('syntactic_path')]
        path_destructure = sum(any(f['category'] == 'B-destructure' for n in p for f in n['features']) for p in paths)
        path_rest = sum(any(f['category'] == 'B-rest-spread' for n in p for f in n['features']) for p in paths)
        manual_paths = [r['adjudication'] for r in group if r.get('adjudication', {}).get('path_destructure') is not None
                        and r['outcome'] in OUTCOMES]
        payload_paths = {axis: Counter() for axis in ('destructure', 'rest_spread')}
        for r in group:
            if r['outcome'] not in OUTCOMES:
                continue
            c = r.get('census', {})
            for axis, values in payload_paths.items():
                flag = r.get('adjudication', {}).get('path_' + axis)
                # The payload enters through this binding, so its syntax is
                # on the path even when Prism cannot register the parameter.
                parameter_flag = 'rest' if axis == 'rest_spread' else 'destructure'
                if any(p.get(parameter_flag) for p in r.get('source', {}).get('data_parameters', [])):
                    flag = True
                if flag is not None:
                    values['present' if flag else 'absent'] += 1
                elif c and not c.get('diagnostic_files') and not c.get('unparsed_files') and not c.get('package_' + axis):
                    values['absent'] += 1
                else:
                    values['unknown'] += 1
        by_class[cls] = {'entries': len(group), 'outcomes': dict(outcomes), 'breaks': dict(breaks),
                         'confirmed_breaks': dict(confirmed), 'syntax': dict(syntax),
                         'candidate_paths': len(paths), 'candidate_path_destructure': path_destructure,
                         'candidate_path_rest_spread': path_rest,
                         'manual_paths': len(manual_paths),
                         'manual_path_destructure': sum(p['path_destructure'] for p in manual_paths),
                         'manual_path_rest_spread': sum(p['path_rest_spread'] for p in manual_paths),
                         'payload_path_syntax': {k: dict(v) for k, v in payload_paths.items()}}
    ws, weighted, confirmed_ws = Counter(), Counter(), Counter()
    for r in rows:
        if r['outcome'] not in OUTCOMES or r['outcome'] == 'traced':
            continue
        w = workstream(r['first_break']['category'])
        if w:
            ws[w] += 1
            weighted[w] += WEIGHTS[r['class']]
            if r.get('attribution_status') == 'adjudicated':
                confirmed_ws[w] += 1
    return {'by_class': by_class, 'outcomes': dict(Counter(r['outcome'] for r in rows)),
            'first_break_opportunities': dict(ws), 'weighted_opportunities': dict(weighted),
            'adjudicated_opportunities': dict(confirmed_ws), 'severity_weights': WEIGHTS,
            'conversion_lower_bound': {str(w): 0 for w in (2, 3, 4, 5, 6)},
            'warning': 'First-break opportunities are not demonstrated single-workstream conversions.'}


def apply_adjudications(rows, adjudications):
    by_key = {(r['class'], r['entry']): r for r in rows}
    compared, agreements, resolved, resolved_agreements = 0, 0, 0, 0
    for item in adjudications:
        row = by_key[(item['class'], item['entry'])]
        if row['outcome'] != item['expected_outcome']:
            raise ValueError(f'adjudication outcome drift: {item["entry"]}')
        row['adjudication'] = item
        if row['outcome'] in OUTCOMES and row['outcome'] != 'traced' and item.get('category'):
            predicted = row['first_break']['category']
            compared += 1
            agrees = int(predicted == item['category'])
            agreements += agrees
            if item['category'] not in ('unresolved', 'prism_error'):
                resolved += 1
                resolved_agreements += agrees
            row['first_break'] = {k: v for k, v in item.items() if k not in ('class', 'entry', 'expected_outcome')}
            row['attribution_status'] = 'adjudicated' if item['category'] not in ('unresolved', 'prism_error') else 'adjudicated_unresolved'
    return {'sample_entries': len(adjudications), 'non_traced_compared': compared,
            'exact_category_agreements': agreements, 'agreement': agreements / compared if compared else None,
            'resolved_categories': resolved, 'resolved_category_agreements': resolved_agreements,
            'resolved_agreement': resolved_agreements / resolved if resolved else None,
            'independent': False, 'method': 'same-worker source and retained-output adjudication; frozen hash-stratified selection'}


def markdown(summary):
    lines = ['# SecBench measurement', '', 'Raw JSON is authoritative; exclusions remain in the 600-entry denominator.', '',
             '| Class | Total | Traced | Function only | Partial | Not reached | Prism error | GT unavailable | Acquisition excluded |',
             '|---|---:|---:|---:|---:|---:|---:|---:|---:|']
    for cls, v in summary['by_class'].items():
        c = v['outcomes']
        lines.append('| ' + ' | '.join(map(str, [cls, v['entries'], *(c.get(o, 0) for o in OUTCOMES), c.get('gt_unavailable', 0), c.get('acquisition_excluded', 0)])) + ' |')
    lines += ['', '## First-break attribution (manual labels replace sampled heuristics)', '',
              '| Category | Workstream | Code | Command | Path | Pollution | ReDoS | Overall | Share of  eligible non-traced |',
              '|---|---:|---:|---:|---:|---:|---:|---:|---:|']
    total = sum(v for k, v in summary['outcomes'].items() if k in OUTCOMES and k != 'traced')
    counts = Counter()
    for v in summary['by_class'].values():
        counts.update(v['breaks'])
    for cat, count in sorted(counts.items()):
        cells = [f'{v["breaks"].get(cat, 0)} ({v["breaks"].get(cat, 0) / sum(v["breaks"].values()):.1%})'
                 if v['breaks'] else '0 (n/a)' for v in summary['by_class'].values()]
        lines.append('| ' + ' | '.join([cat, str(workstream(cat) or 'unassigned'), *cells, str(count), f'{count / total:.1%}' if total else 'n/a']) + ' |')
    lines += ['', '## Syntax census and candidate path prevalence', '',
              '| Class | Acquired packages | Packages destructuring | Packages rest/spread or Object.assign | Candidate paths | Path destructuring | Path rest/spread |',
              '|---|---:|---:|---:|---:|---:|---:|']
    for cls, v in summary['by_class'].items():
        s = v['syntax']
        lines.append(f'| {cls} | {v["entries"] - v["outcomes"].get("acquisition_excluded", 0)} | {s.get("package_destructure", 0)} | {s.get("package_rest_spread", 0)} | {v["candidate_paths"]} | {v["candidate_path_destructure"]} | {v["candidate_path_rest_spread"]} |')
    lines += ['', 'Path rows concern features anywhere in candidate callable spans, not proved value dependence. Unknown paths are excluded only from this clearly stated sub-denominator. Package syntax includes shipped tests, demos, declarations and vendored sources; it does not imply server-runtime use.', '',
              '## Manually inspected payload paths', '',
              '| Class | Inspected eligible paths | Destructuring | Rest/spread or Object.assign |', '|---|---:|---:|---:|']
    for cls, v in summary['by_class'].items():
        lines.append(f'| {cls} | {v["manual_paths"]} | {v["manual_path_destructure"]} | {v["manual_path_rest_spread"]} |')
    lines += ['', '## Payload-path syntax bounds (eligible cases)', '',
              '| Class | Destructure present / absent / unknown | Rest-spread present / absent / unknown |', '|---|---:|---:|']
    for cls, v in summary['by_class'].items():
        cells = [' / '.join(str(v['payload_path_syntax'][axis].get(k, 0)) for k in ('present', 'absent', 'unknown'))
                 for axis in ('destructure', 'rest_spread')]
        lines.append('| ' + ' | '.join([cls, *cells]) + ' |')
    lines += ['', 'Present is established by a test-fed entry parameter binding or manual payload-path inspection. Absent is established manually or by whole-package syntax absence with zero diagnostics/unparsed files. Syntax presence elsewhere cannot establish path presence. Bounds concern constructs inside the acquired package, excluding unpacked external dependencies.', '']
    if summary.get('adjudication'):
        lines += ['', 'Adjudication: ' + json.dumps(summary['adjudication'], sort_keys=True), '']
    lines += ['',
              '## Workstream opportunity sensitivity', '', '| Workstream | First-break opportunities | Severity-weighted score | Adjudicated entries | Proven conversions |', '|---|---:|---:|---:|---:|']
    for w in (2, 3, 4, 5, 6):
        lines.append(f'| {w} | {summary["first_break_opportunities"].get(w, 0)} | {summary["weighted_opportunities"].get(w, 0)} | {summary["adjudicated_opportunities"].get(w, 0)} | 0 |')
    lines += ['', 'Weights (scenario, not CVSS): command/code injection 3, prototype pollution/path traversal 2, ReDoS 1. Equal weighting is available in the raw opportunity counts.', '', summary['warning'], '']
    return '\n'.join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inputs', type=Path, default=EVIDENCE / 'inputs/secbench-js')
    parser.add_argument('--packages', type=Path, default=EVIDENCE / 'inputs/secbench-pkgs')
    parser.add_argument('--sut-repo', type=Path, default=REPO, help='exact pinned SUT checkout; separate from harness after controller commit')
    parser.add_argument('--compiler', type=Path, default=COMPILER)
    parser.add_argument('--out', type=Path, default=EVIDENCE / 'meas/secbench/rerun')
    parser.add_argument('--adjudications', type=Path, default=Path(__file__).with_name('adjudications.json'))
    parser.add_argument('--inspection', type=Path, help='reuse a complete inspection, after rehashing every input')
    parser.add_argument('--no-build', action='store_true', help='explicitly use the already built, hash-recorded binary')
    parser.add_argument('--workers', type=int, default=2)
    parser.add_argument('--timeout', type=int, default=30)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    manifest_file = args.packages / 'manifest.json'
    manifest = json.loads(manifest_file.read_bytes())
    if manifest['secbench_commit'] != SEC_COMMIT or len(manifest['entries']) != 600:
        raise ValueError('SecBench manifest pin/count mismatch')
    sut_head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=args.sut_repo, text=True).strip()
    sec_head = subprocess.check_output(['git', '-C', str(args.inputs), 'rev-parse', 'HEAD'], text=True).strip()
    if sut_head != SUT_COMMIT or sec_head != SEC_COMMIT:
        raise ValueError('checkout revision mismatch')
    subprocess.run(['git', 'diff', '--exit-code', SUT_COMMIT, '--', 'src', 'Cargo.toml', 'Cargo.lock', 'build.rs', 'vendor'],
                   cwd=args.sut_repo, check=True, stdout=subprocess.DEVNULL)
    for e in manifest['entries']:
        if e['status'] == 'ok' and digest((args.packages / e['class'] / e['entry'] / e['tarball']).read_bytes()) != e['sha256']:
            raise ValueError('tarball hash mismatch')
    if not args.no_build:
        with (args.out / 'build.log').open('wb') as log:
            subprocess.run(['cargo', 'build', '--release', '--offline'], cwd=args.sut_repo, stdout=log, stderr=log, check=True)
    binary = args.sut_repo / 'target/release/prism'
    version = subprocess.check_output([str(binary), '--version'], text=True).strip()
    if SUT_COMMIT[:12] not in version:
        raise ValueError('binary identity mismatch')
    inspection = args.inspection or args.out / 'inspection.jsonl'
    if not args.inspection:
        with (args.out / 'inspection.log').open('wb') as log:
            subprocess.run(['node', '--max-old-space-size=2048', str(Path(__file__).with_name('inspect.mjs')),
                            str(args.compiler), str(manifest_file), str(args.inputs), str(args.packages), str(inspection)],
                           stderr=log, stdout=log, check=True)
    rows = [json.loads(line) for line in inspection.read_bytes().splitlines()]
    expected = {(e['class'], e['entry']) for e in manifest['entries'] if e['status'] == 'ok'}
    if len(rows) != len(expected) or {(r['class'], r['entry']) for r in rows} != expected:
        raise ValueError('incomplete/duplicate inspection population')
    pins = {'schema': 'prism.secbench-input-pins/1', 'manifest_sha256': digest(manifest_file.read_bytes()),
            'compiler_sha256': digest(args.compiler.read_bytes()), 'packages': verify_inspection(rows, args.inputs, args.packages)}
    if pins != json.loads(Path(__file__).with_name('input-pins.json').read_bytes()):
        raise ValueError('input bytes differ from the committed benchmark pins')
    pin_file = args.out / 'pins.json'
    if pin_file.exists() and pin_file.read_bytes() != canonical(pins):
        raise ValueError('input pins changed; use a new output directory')
    pin_file.write_bytes(canonical(pins))
    binding = {'sut_head': sut_head, 'sut_repo': str(args.sut_repo), 'secbench_head': sec_head, 'binary_sha256': digest(binary.read_bytes()),
               'version': version, 'inspection_sha256': digest(inspection.read_bytes()),
               'pins_sha256': digest(pin_file.read_bytes()), 'workers': args.workers, 'timeout_seconds': args.timeout,
               'adjudications_sha256': digest(args.adjudications.read_bytes()),
               'harness_sha256': {p.name: digest(p.read_bytes()) for p in Path(__file__).parent.glob('*') if p.suffix in ('.py', '.mjs')}}
    (args.out / 'binding.json').write_bytes(canonical(binding))
    measured = []
    with (args.out / 'entries.jsonl').open('wb') as stream:
        with ThreadPoolExecutor(max_workers=args.workers) as pool:
            for row in pool.map(lambda r: measure(r, binary, args.packages, args.out, args.timeout), rows):
                stream.write(canonical(row)); stream.flush(); measured.append(row)
                print(f'{len(measured)}/583 {row["class"]}/{row["entry"]}: {row["outcome"]}', flush=True)
        for e in manifest['entries']:
            if e['status'] != 'ok':
                row = {**e, 'outcome': 'acquisition_excluded', 'exclusion_reason': e['status']}
                measured.append(row); stream.write(canonical(row))
    labels = json.loads(args.adjudications.read_bytes())
    if labels['sut_commit'] != sut_head or labels['secbench_commit'] != sec_head:
        raise ValueError('adjudication revision mismatch')
    agreement = apply_adjudications(measured, labels['entries'])
    (args.out / 'entries.jsonl').write_bytes(b''.join(canonical(r) for r in measured))
    summary = summarize(measured)
    summary['adjudication'] = agreement
    (args.out / 'summary.json').write_bytes(canonical(summary))
    (args.out / 'summary.md').write_text(markdown(summary))
    print(json.dumps(summary['outcomes'], sort_keys=True))


if __name__ == '__main__':
    main()
