"""Re-query existing minimal static fixtures; never create or execute test inputs."""
import argparse
import json
from pathlib import Path
from .conversions import observe
from .run import BINARY_SHA, EVIDENCE, canonical, digest, verify_inspection


def probe_fixture(case, binary, out, timeout=120):
    row = json.loads((case / 'inspection.jsonl').read_bytes())
    if row['gt_status'] != 'available':
        raise ValueError('fixture has no bound source and target')
    verify_inspection([row], case / 'inputs', case / 'packages')
    root = case / 'packages' / row['class'] / row['entry'] / 'src/package'
    return observe(row, binary, root, out, timeout)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixtures', type=Path, default=EVIDENCE / 'meas/secbench/repair-r1/product-probes')
    args = parser.parse_args()
    if digest(args.binary.read_bytes()) != BINARY_SHA:
        raise ValueError('binary drift')
    if args.out.resolve() == args.fixtures.resolve() or args.fixtures.resolve() in args.out.resolve().parents:
        raise ValueError('observations must be outside the read-only fixtures')
    cases = sorted(p for p in args.fixtures.iterdir() if p.is_dir() and (p / 'inspection.jsonl').is_file())
    if not cases:
        raise ValueError('no existing byte-bound fixtures')
    results = []
    for case in cases:
        result = probe_fixture(case, args.binary, args.out / case.name)
        (args.out / case.name / 'result.json').write_bytes(canonical(result))
        results.append({'name': case.name, 'outcome': result['outcome'], 'mechanism': result['first_break'],
                        'inspection_sha256': digest((case / 'inspection.jsonl').read_bytes()),
                        'result': str(args.out / case.name / 'result.json')})
    (args.out / 'results.json').write_bytes(canonical(results))
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    main()
