#!/usr/bin/env python3
"""Complete fixed-checker public audit after a bound measure_b capture. Static only."""
import argparse
from collections import Counter
import json
from pathlib import Path
import subprocess
import time
from measure_b import PUBLIC

P = Path(__file__).resolve().parent
TS = Path.home() / 'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('capture', type=Path)
    args = parser.parse_args()
    deadline = time.monotonic() + 7200
    while not (args.capture / 'status.json').exists():
        if time.monotonic() >= deadline:
            raise TimeoutError('capture status did not complete')
        time.sleep(10)
    status = json.loads((args.capture / 'status.json').read_text())
    summary = {}
    for name, root in PUBLIC.items():
        st = status[name]
        result = summary[name] = {'admitted': st['ok']}
        if not st['ok']:
            result['excluded'] = st
            continue
        result.update({k: st[k] for k in ('sites_identical', 'bytes_identical', 'wire_identical', 'base_projection_equal', 'head_projection_equal')})
        prefix = args.capture / name
        diff, changed = Path(str(prefix) + '.diff.json'), Path(str(prefix) + '.changed.jsonl')
        subprocess.run(['python3', str(P / 'rowdiff.py'), str(args.capture / 'per' / f'{name}.base.bytes.jsonl'),
                        str(args.capture / 'per' / f'{name}.head.bytes.jsonl'), str(diff), '--rows', str(changed)], check=True, stdout=subprocess.DEVNULL)
        details, adj = Path(str(prefix) + '.details.jsonl'), Path(str(prefix) + '.adj.json')
        if changed.stat().st_size:
            subprocess.run(['node', '--max-old-space-size=8000', str(P / 'adjudicate.cjs'), str(TS), str(root),
                            str(changed), str(adj), '--details', str(details)], check=True, stdout=subprocess.DEVNULL)
        else:
            details.write_text('')
            adj.write_text('{}\n')
        counts = Counter()
        for line in details.read_text().splitlines():
            row = json.loads(line)
            counts[row['class'] + '|' + row['verdict']['step1']] += 1
        result.update(diff=json.loads(diff.read_text()), checker=json.loads(adj.read_text()), classes=dict(counts))
        js_suffixes = {'.js', '.jsx', '.ts', '.tsx', '.mjs', '.cjs'}
        non_js_changes = []
        for line in changed.read_text().splitlines():
            row = json.loads(line)
            if any(Path(row['row'][end]['file']).suffix not in js_suffixes for end in ('from', 'to')):
                non_js_changes.append(row)
        result['non_js_changed_rows'] = len(non_js_changes)
        (args.capture / 'adjudication-summary.json').write_text(json.dumps(summary, indent=1) + '\n')
        print(name, result['classes'], flush=True)
        new_wrong = sum(v for k, v in counts.items() if k.endswith('|WRONG') and not k.startswith('LOST|'))
        if counts['LOST|CORRECT'] or new_wrong or not st['sites_identical'] or non_js_changes:
            (args.capture / 'STOP.json').write_text(json.dumps({'name': name, 'result': result}, indent=1) + '\n')
            return


if __name__ == '__main__':
    main()
