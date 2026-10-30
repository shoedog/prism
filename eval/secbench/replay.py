#!/usr/bin/env python3
"""Reclassify retained Prism observations after bounded ground-truth corrections."""
import argparse
import gzip
import json
from pathlib import Path
try:
    from .run import (SUT_COMMIT, EVIDENCE, attribute_error, sink_locations, apply_adjudications, canonical, classify, digest, markdown,
                      propose_break, summarize, verify_inspection)
except ImportError:
    from run import (SUT_COMMIT, EVIDENCE, attribute_error, sink_locations, apply_adjudications, canonical, classify, digest, markdown,
                     propose_break, summarize, verify_inspection)


def retained(row, name):
    record = row['invocations'][name]
    data = gzip.open(record['raw'], 'rb').read()
    if digest(data) != record['stdout_sha256']:
        raise ValueError(f'raw observation drift: {record["raw"]}')
    if record['error']:
        try:
            record['output_error'] = json.loads(data).get('error')
        except (ValueError, UnicodeDecodeError, AttributeError):
            record['output_error'] = None
        return None
    return json.loads(data)


def reclassify(observation, ground_truth):
    row = {k: v for k, v in ground_truth.items() if k != 'identities'}
    row['invocations'] = observation['invocations']
    row['dfg_stats'] = observation['dfg_stats']
    row['previous_outcome'] = observation['outcome']
    if ground_truth['gt_status'] != 'available':
        row['outcome'] = 'gt_unavailable'
        row['first_break'] = {'category': 'unavailable_ground_truth', 'reason': row['gt_reason']}
        return row
    # Only narrower sources and refined terminal occurrences are admissible:
    # the raw per-root query must contain every requested seed line and use
    # the same package and sink line. Never invent a new observation by replay.
    expected_lines = {f"{row['source']['file']}:{row['source'].get('start_line', p['line'])}" for p in row['source']['data_parameters']}
    argv = row['invocations']['witness']['argv']
    actual_lines = {argv[i + 1] for i, arg in enumerate(argv) if arg == '--source'}
    expected_sinks = set(sink_locations(row['sink']))
    actual_sinks = {argv[i + 1] for i, arg in enumerate(argv) if arg == '--sink'}
    if not expected_lines <= actual_lines or expected_sinks != actual_sinks:
        raise ValueError('replay needs new seed/sink invocations')
    values = {name: retained(observation, name) for name in ('witness', 'callees', 'frontier')}
    if any(v is None for v in values.values()):
        row['outcome'] = 'prism_error'
        row['first_break'] = {**attribute_error(row, values.get('witness'), values.get('frontier')), 'reason': '; '.join(row['invocations'][k]['error'] or '' for k, v in values.items() if v is None),
                              'output_errors': {k: row['invocations'][k].get('output_error') for k, v in values.items() if v is None}}
        return row
    row['outcome'], row['trace_detail'] = classify(row, values['witness'], values['callees'], values['frontier'])
    row['heuristic_break'] = propose_break(row, values['frontier'], values['witness']) if row['outcome'] != 'traced' else None
    row['first_break'] = row['heuristic_break'] or {'category': 'none'}
    row['attribution_status'] = 'heuristic_unadjudicated' if row['heuristic_break'] else 'not_applicable'
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', type=Path, required=True)
    parser.add_argument('--inspection', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--adjudications', type=Path)
    parser.add_argument('--inputs', type=Path, default=EVIDENCE / 'inputs/secbench-js')
    parser.add_argument('--packages', type=Path, default=EVIDENCE / 'inputs/secbench-pkgs')
    parser.add_argument('--provisional', action='store_true')
    args = parser.parse_args()
    observations = [json.loads(l) for l in (args.run / 'entries.jsonl').read_bytes().splitlines()]
    gt = {(r['class'], r['entry']): r for r in (json.loads(l) for l in args.inspection.read_bytes().splitlines())}
    if len(observations) != 600 and not args.provisional:
        raise ValueError('incomplete measurement: need 600 entry rows')
    binding = json.loads((args.run / 'binding.json').read_bytes())
    if binding['sut_head'] != SUT_COMMIT:
        raise ValueError('SUT pin mismatch')
    pins = verify_inspection(list(gt.values()), args.inputs, args.packages)
    previous = json.loads((args.run / 'pins.json').read_bytes())['packages']
    if canonical(pins) != canonical(previous):
        raise ValueError('ground-truth correction changed input bytes')
    rows = [reclassify(r, gt[(r['class'], r['entry'])]) if r['outcome'] != 'acquisition_excluded' else r for r in observations]
    labels = json.loads(args.adjudications.read_bytes()) if args.adjudications else None
    if labels and (labels['sut_commit'] != binding['sut_head'] or labels['secbench_commit'] != binding['secbench_head']):
        raise ValueError('adjudication revision mismatch')
    adjudications = labels['entries'] if labels else []
    agreement = apply_adjudications(rows, adjudications)
    summary = summarize(rows)
    summary['adjudication'] = agreement
    summary['classification_changes'] = [
        {'class': r['class'], 'entry': r['entry'], 'before': r['previous_outcome'], 'after': r['outcome']}
        for r in rows if r.get('previous_outcome') != r['outcome'] and 'previous_outcome' in r]
    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / 'entries.jsonl').write_bytes(b''.join(canonical(r) for r in rows))
    (args.out / 'summary.json').write_bytes(canonical(summary))
    (args.out / 'summary.md').write_text(markdown(summary))
    (args.out / 'binding.json').write_bytes(canonical({
        'schema': 'prism.secbench-replay/1', 'observations_binding': binding,
        'observations_sha256': digest((args.run / 'entries.jsonl').read_bytes()),
        'inspection_sha256': digest(args.inspection.read_bytes()),
        'adjudications_sha256': digest(args.adjudications.read_bytes()) if args.adjudications else None,
        'classifier_sha256': digest(Path(__file__).with_name('run.py').read_bytes()),
        'inspector_sha256': digest(Path(__file__).with_name('inspect.mjs').read_bytes()),
        'bindings_sha256': digest(Path(__file__).with_name('bindings.mjs').read_bytes()),
        'replay_sha256': digest(Path(__file__).read_bytes()), 'provisional': args.provisional,
    }))
    print(json.dumps(summary['outcomes'], sort_keys=True))


if __name__ == '__main__':
    main()
