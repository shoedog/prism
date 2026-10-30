"""Freeze label-free strata and score later independent labels with Cohen's kappa."""
import argparse
from collections import Counter, defaultdict
import gzip
import json
from pathlib import Path
from .run import EVIDENCE, canonical, digest


def kappa(left, right):
    if len(left) != len(right) or not left:
        raise ValueError('labels must have equal nonzero length')
    n = len(left)
    a, b = Counter(left), Counter(right)
    observed = sum(x == y for x, y in zip(left, right)) / n
    expected = sum(a[key] * b[key] for key in set(a) | set(b)) / n ** 2
    return {'n': n, 'observed_agreement': observed, 'chance_agreement': expected,
            'kappa': (observed - expected) / (1 - expected) if expected != 1 else None,
            'left_counts': dict(a), 'right_counts': dict(b)}


def freeze(rows, packages, sample, reference):
    strata = defaultdict(list)
    for row in rows:
        if row['outcome'] != 'acquisition_excluded':
            strata[row['class']].append(row)
    selected = []
    for cls, group in sorted(strata.items()):
        ordered = sorted(group, key=lambda r: digest(f"R1-blind/{cls}/{r['entry']}".encode()))
        # Eight per class, enriched with failures/exclusions; never prevalence.
        keys = set()
        for outcomes, quota in (({'prism_error'}, 2), ({'partial', 'not_reached', 'reached_function_only'}, 3),
                                ({'gt_unavailable'}, 2), ({'traced'}, 1)):
            matches = [r for r in ordered if r['outcome'] in outcomes][:quota]
            selected.extend(matches);keys.update(r['entry'] for r in matches)
        missing = 8 - len(keys)
        selected.extend([r for r in ordered if r['entry'] not in keys][:missing])
    packets, labels = [], []
    for row in selected:
        root = packages / row['class'] / row['entry'] / 'src/package'
        packet = {'class': row['class'], 'entry': row['entry'], 'source': None, 'sink': None, 'prism_output_excerpt': {}}
        for side in ('source', 'sink'):
            bound = row.get(side)
            if bound:
                file = root / bound['file']
                lines = file.read_text(errors='replace').splitlines()
                start, end = max(1, bound.get('start_line', bound.get('line', 1)) - 1), min(len(lines), bound.get('end_line', bound.get('line', 1)) + 1)
                # Limit long wrappers, but include exact endpoint and formal bindings.
                indices = sorted(set(range(start, min(end, start + 35) + 1)) | set(range(max(1, bound.get('line', start)-2), min(len(lines), bound.get('line', start)+2)+1)))
                packet[side] = {'file': bound['file'], 'start_byte': bound.get('start_byte'), 'end_byte': bound.get('end_byte'),
                                'line': bound.get('line', bound.get('start_line')), 'excerpt': '\n'.join(f'{i}: {lines[i-1]}' for i in indices)}
                if side == 'source':
                    packet[side]['parameters'] = [{k:p.get(k) for k in ('ordinal','text','names','line','start_byte','end_byte')} for p in bound.get('data_parameters', [])]
                else:
                    packet[side]['occurrences'] = bound.get('value_occurrences', [])
        for name in ('witness', 'frontier'):
            record = row.get('invocations', {}).get(name)
            if record:
                data = gzip.open(record['raw'], 'rb').read()
                if digest(data) != record['stdout_sha256']:
                    raise ValueError('raw drift')
                value = json.loads(data) if data else {'no_output': True}
                # Raw verdicts are evidence; derived outcome/mechanism labels are absent.
                if name == 'frontier':
                    value = {'items': value.get('items', [])[:12], 'error': value.get('error')}
                elif 'reasoning' in value:
                    value = {'reasoning': value['reasoning'], 'warnings': value.get('warnings', [])}
                packet['prism_output_excerpt'][name] = value
        packets.append(packet)
        labels.append({'class': row['class'], 'entry': row['entry'], 'gt': row.get('gt_status'),
                       'outcome': row['outcome'], 'mechanism': row.get('first_break', {}).get('category', 'unavailable_ground_truth')})
    sample.parent.mkdir(parents=True, exist_ok=True)
    sample.write_bytes(b''.join(canonical(r) for r in packets))
    reference.write_bytes(b''.join(canonical(r) for r in labels))
    sample.with_suffix('.binding.json').write_bytes(canonical({'schema': 'prism.secbench-blind/1',
        'sample_sha256': digest(sample.read_bytes()), 'reference_sha256': digest(reference.read_bytes()),
        'entries': len(packets), 'selection': 'SHA256 R1-blind/class/entry; 8/class, 2 errors + 3 nontraces + 2 unavailable + 1 trace, deterministic fill',
        'warning': 'Failure-enriched sample; agreement is not a prevalence estimate. Independent labels pending.'}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', type=Path)
    parser.add_argument('--sample', type=Path, default=EVIDENCE / 'meas/secbench/blind-sample.jsonl')
    parser.add_argument('--reference', type=Path, required=True)
    parser.add_argument('--labels', type=Path)
    parser.add_argument('--packages', type=Path, default=EVIDENCE / 'inputs/secbench-pkgs')
    args = parser.parse_args()
    if args.labels:
        def rows(p):
            values = [json.loads(l) for l in p.read_bytes().splitlines()]
            result = {(r['class'], r['entry']): r for r in values}
            if len(result) != len(values):
                raise ValueError('duplicate labels')
            return result
        left, right = rows(args.reference), rows(args.labels)
        if set(left) != set(right):
            raise ValueError('independent label population differs')
        result = {stage: kappa([left[k][stage] for k in sorted(left)], [right[k][stage] for k in sorted(left)]) for stage in ('gt', 'outcome', 'mechanism')}
        print(json.dumps(result, sort_keys=True, indent=2))
    elif args.run:
        freeze([json.loads(l) for l in (args.run / 'entries.jsonl').read_bytes().splitlines()], args.packages, args.sample, args.reference)
    else:
        parser.error('supply --run to freeze or --labels to score')


if __name__ == '__main__':
    main()
