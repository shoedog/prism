"""Pinned, source-only counterfactual measurements; never execute corpus code."""
from __future__ import annotations
import argparse
import copy
import difflib
import gzip
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
from .run import (BINARY_SHA, COMPILER, EVIDENCE, canonical, digest, invoke, classify,
                  propose_break, attribute_error, sink_locations, workstream, WEIGHTS)


def pattern_for(row):
    mechanism = row.get('error_mechanism', row.get('first_break', {})).get('mechanism')
    category = row.get('first_break', {}).get('category')
    if mechanism == 'repo_loader_skipped_dist_build':
        return 'admission'
    if mechanism == 'callback_argument_parameter_registration':
        return 'callback_registration'
    if mechanism == 'member_only_parameter_no_def' or category == 'C-member':
        return 'member'
    if mechanism == 'rest_parameter_no_def':
        return 'rest'
    if mechanism == 'legacy_arguments_no_parameter_def':
        return 'arguments'
    if mechanism == 'nested_callback_capture_no_sink_symbol':
        return 'callback_capture'
    return None


def same_control(row, control):
    if row['outcome'] != control['outcome']:
        return False
    if row['outcome'] == 'prism_error':
        return row.get('first_break', {}).get('mechanism') == control.get('first_break', {}).get('mechanism')
    return True


def observe(row, binary, root, out, timeout):
    row = copy.deepcopy(row)
    row.pop('error_mechanism', None)
    source = row['source']
    nav = ['nav', '--cache-dir', str(out / 'cache')]
    common = ['--repo', str(root)]
    source_flags = ['--source', f"{source['file']}:{source['start_line']}"]
    values, records = {}, {}
    for label, args in {
        'witness': nav + ['taint-reaches'] + common + source_flags + sum((['--sink', loc] for loc in sink_locations(row['sink'])), []) + ['--format', 'json'],
        'frontier': nav + ['taint-reaches'] + common + source_flags + ['--format', 'json'],
        'callees': nav + ['callees'] + common + ['--location', f"{source['file']}:{source['start_line']}", '--depth', '8', '--format', 'json'],
    }.items():
        values[label], records[label] = invoke(binary, args, out / 'raw', label, timeout)
    row['invocations'] = records
    if any(v is None for v in values.values()):
        row['outcome'] = 'prism_error'
        row['first_break'] = attribute_error(row, values['witness'], values['frontier'])
    else:
        row['outcome'], row['trace_detail'] = classify(row, values['witness'], values['callees'], values['frontier'])
        row['first_break'] = propose_break(row, values['frontier'], values['witness']) if row['outcome'] != 'traced' else {'category': 'none'}
    shutil.rmtree(out / 'cache', ignore_errors=True)
    return row


def tree_bytes(root):
    return {str(p.relative_to(root)): p.read_bytes() for p in root.rglob('*') if p.is_file()}


def relocate(root, row, compiler=COMPILER):
    """Bijective directory relocation, preserving bytes and local imports.

    AST-plan local import changes before mutation. Refuse observable runtime
    paths or strings rather than changing what the package computes.
    """
    original = tree_bytes(root)
    mapping = lambda name: '/'.join('_secbench_' + p if p in ('dist', 'build') else p for p in name.split('/'))
    if any(mapping(name) != name and mapping(name) in original for name in original):
        raise ValueError('admission semantics: destination collision')
    with tempfile.TemporaryDirectory(prefix='admission-plan-', dir=root.parent) as directory:
        plan = Path(directory) / 'plan.json'
        process = subprocess.run(['node', str(Path(__file__).with_name('rewrite.mjs')), str(compiler), str(root),
                                  '-', 'admission_plan', str(plan)], capture_output=True, text=True)
        if process.returncode:
            raise ValueError('admission semantics: ' + process.stderr[-2000:])
        rewrites = json.loads(plan.read_bytes())
    for name, data in original.items():
        target = root / mapping(name)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    # Keep distributed originals; only indexed copies and references move.
    result = copy.deepcopy(row)
    for name in original:
        target = root / mapping(name)
        if target.suffix in ('.js', '.mjs', '.cjs', '.ts', '.json'):
            text = rewrites.get(name, original[name].decode('utf8'))
            before_text = original[name].decode('utf8')
            opcodes = difflib.SequenceMatcher(None, before_text, text, autojunk=False).get_opcodes()
            def mapped_byte(value):
                position = len(original[name][:value].decode('utf8'))
                for tag, a, b, c, d in opcodes:
                    if a <= position < b or position == b == len(before_text):
                        if tag != 'equal' and a < position < b:
                            raise ValueError('admission edit overlaps endpoint identity')
                        return len(text[:c + min(position - a, d - c)].encode())
                return len(text.encode())
            for side in ('source', 'sink'):
                if row[side]['file'] == name:
                    if 'start_byte' in result[side]:
                        result[side]['start_byte'] = mapped_byte(row[side]['start_byte'])
                        result[side]['end_byte'] = mapped_byte(row[side]['end_byte'])
                    for endpoint in result[side].get('data_parameters', result[side].get('value_occurrences', [])):
                        endpoint['start_byte'] = mapped_byte(endpoint['start_byte'])
                        endpoint['end_byte'] = mapped_byte(endpoint['end_byte'])
            target.write_text(text)
    for key in ('source', 'sink'):
        result[key]['file'] = mapping(result[key]['file'])
    if result.get('syntactic_path'):
        for node in result['syntactic_path']:
            node['file'] = mapping(node['file'])
    return result


def patch_bytes(before, after):
    chunks = []
    for name in sorted(set(before) | set(after)):
        if before.get(name) == after.get(name):
            continue
        old = before.get(name, b'').decode('utf8', errors='replace').splitlines(keepends=True)
        new = after.get(name, b'').decode('utf8', errors='replace').splitlines(keepends=True)
        chunks.extend(difflib.unified_diff(old, new, fromfile='a/' + name, tofile='b/' + name))
    return ''.join(chunks).encode()


def convert(row, binary, packages, out, compiler, timeout):
    case = out / row['class'] / row['entry']
    case.mkdir(parents=True, exist_ok=True)
    result = {'class': row['class'], 'entry': row['entry'], 'baseline': row['outcome'],
              'label': row['first_break'], 'attempts': [], 'workstreams': [],
              'conversion': False, 'status': 'unresolved_hypothesis'}
    first = pattern_for(row)
    if not first:
        return result
    original = packages / row['class'] / row['entry'] / 'src/package'
    with tempfile.TemporaryDirectory(prefix='secbench-rewrite-', dir=out) as directory:
        root = Path(directory) / 'package'
        shutil.copytree(original, root)
        before = tree_bytes(root)
        result['input_tree_sha256'] = digest(canonical({name: digest(data) for name, data in before.items()}))
        control = observe(row, binary, root, case / 'control', timeout)
        (case / 'control.json').write_bytes(canonical(control))
        result['control'] = control['outcome']
        if not same_control(row, control):
            result['status'] = 'control_disagreement'
            return result
        current, seen = row, set()
        # Four distinct capability barriers maximum; never repeat a rewrite.
        for step in range(4):
            pattern = pattern_for(current)
            if not pattern or pattern in seen:
                result['status'] = 'remaining_unresolved_barrier'
                break
            seen.add(pattern)
            attempt = {'pattern': pattern, 'before': current['outcome'], 'before_break': current['first_break']}
            ws = {'admission': 4, 'member': 3, 'rest': 2, 'arguments': 2,
                  'callback_registration': 6, 'callback_capture': 6}[pattern]
            result['workstreams'].append(ws)
            if pattern == 'admission':
                try:
                    current = relocate(root, current, compiler)
                except ValueError as exc:
                    attempt['refusal'] = str(exc)
                    result['attempts'].append(attempt)
                    result['status'] = 'rewrite_not_semantics_certified'
                    break
            else:
                request = case / f'step-{step + 1}.input.json'
                request.write_bytes(canonical(current))
                output = case / f'step-{step + 1}.rewrite.json'
                process = subprocess.run(['node', str(Path(__file__).with_name('rewrite.mjs')), str(compiler), str(root), str(request), pattern, str(output)], capture_output=True, text=True)
                if process.returncode:
                    attempt['refusal'] = process.stderr[-3000:]
                    result['attempts'].append(attempt)
                    result['status'] = 'rewrite_not_semantics_certified'
                    break
                current = json.loads(output.read_bytes())['row']
            patch = case / f'step-{step + 1}.patch'
            patch.write_bytes(patch_bytes(before, tree_bytes(root)))
            attempt['patch'] = str(patch)
            attempt['patch_sha256'] = digest(patch.read_bytes())
            current = observe(current, binary, root, case / f'step-{step + 1}', timeout)
            (case / f'step-{step + 1}.result.json').write_bytes(canonical(current))
            attempt.update(after=current['outcome'], after_break=current['first_break'])
            result['attempts'].append(attempt)
            if current['outcome'] == 'traced':
                result.update(conversion=True, status='demonstrated', final='traced')
                break
    result['workstreams'] = sorted(set(result['workstreams']))
    result['single_workstream'] = result['workstreams'][0] if result['conversion'] and len(result['workstreams']) == 1 else None
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--packages', type=Path, default=EVIDENCE / 'inputs/secbench-pkgs')
    parser.add_argument('--compiler', type=Path, default=COMPILER)
    parser.add_argument('--timeout', type=int, default=120)
    parser.add_argument('--follow', action='store_true', help='consume complete flushed rows while the full measurement finishes')
    parser.add_argument('--inspection', type=Path, help='root-specific reclassification against this byte-bound GT')
    parser.add_argument('--supplement', type=Path, help='one newly collected row replacing an old unavailable row')
    args = parser.parse_args()
    if digest(args.binary.read_bytes()) != BINARY_SHA:
        raise ValueError('binary pin mismatch')
    args.out.mkdir(parents=True, exist_ok=True)
    gt = {(r['class'], r['entry']): r for r in map(json.loads, args.inspection.read_bytes().splitlines())} if args.inspection else None
    supplement = json.loads(args.supplement.read_bytes()) if args.supplement else None
    def rows():
        from .replay import reclassify
        seen = set()
        while True:
            for line in (args.run / 'entries.jsonl').read_bytes().splitlines():
                try:
                    row = json.loads(line)
                except ValueError:
                    if not args.follow:
                        raise
                    continue  # An in-flight partial write is not evidence.
                key = row['class'], row['entry']
                if key in seen:
                    continue
                seen.add(key)
                if supplement and key == (supplement['class'], supplement['entry']):
                    row = supplement
                if gt and row['outcome'] != 'acquisition_excluded':
                    row = reclassify(row, gt[key])
                yield row
            if not args.follow or (args.run / 'summary.json').exists():
                break
            time.sleep(1)
    results = []
    with (args.out / 'entries.jsonl').open('wb') as stream:
        for row in rows():
            if row['outcome'] not in ('traced', 'gt_unavailable', 'acquisition_excluded'):
                result = convert(row, args.binary, args.packages, args.out, args.compiler, args.timeout)
                results.append(result); stream.write(canonical(result)); stream.flush()
                print(f"{row['class']}/{row['entry']}: {result['status']} {result['workstreams']}", flush=True)
    summary = {}
    for ws in (2, 3, 4, 5, 6):
        singles = [r for r in results if r.get('single_workstream') == ws]
        stacks = [r for r in results if r['conversion'] and len(r['workstreams']) > 1 and ws in r['workstreams']]
        needing = [r for r in results if len(r['workstreams']) > 1 and ws in r['workstreams']]
        summary[str(ws)] = {'demonstrated_single_workstream': len(singles),
                            'severity_weighted_single': sum(WEIGHTS[r['class']] for r in singles),
                            'demonstrated_multi_workstream': len(stacks), 'multi_workstream_entries': len(needing),
                            'severity_weighted_multi': sum(WEIGHTS[r['class']] for r in stacks)}
    (args.out / 'summary.json').write_bytes(canonical(summary))
    (args.out / 'binding.json').write_bytes(canonical({'binary_sha256': BINARY_SHA,
        'observations_sha256': digest((args.run / 'entries.jsonl').read_bytes()),
        'inspection_sha256': digest(args.inspection.read_bytes()) if args.inspection else None,
        'supplement_sha256': digest(args.supplement.read_bytes()) if args.supplement else None,
        'rewriter_sha256': digest(Path(__file__).with_name('rewrite.mjs').read_bytes()),
        'runner_sha256': digest(Path(__file__).read_bytes()), 'barrier_cap': 4, 'timeout': args.timeout}))


if __name__ == '__main__':
    main()
