"""Offline certification: getrusage peak child RSS and sampled target disk use."""
import argparse
import json
import os
from pathlib import Path
import re
import resource
import subprocess
import sys
import time
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(ROOT / 'scripts/mutgate'))
import mutgate


def measure(name, command):
    target = ROOT / 'target'
    start = time.monotonic()
    peak_disk = samples = 0
    errors = set()
    usage = HERE / (name + '-time.txt')
    with (HERE / (name + '.log')).open('w') as log:
        process = subprocess.Popen(['/usr/bin/time', '-p', '-o', str(usage), *command],
                                   cwd=ROOT, env=mutgate.cargo_env(target), stdout=log, stderr=log)
        while process.poll() is None:
            disk = subprocess.run(['du', '-sk', str(target)], capture_output=True, text=True)
            if disk.returncode == 0:
                peak_disk = max(peak_disk, int(disk.stdout.split()[0]) * 1024)
            elif target.exists():
                errors.add('du: ' + disk.stderr.strip())
            samples += 1
            time.sleep(0.5)
        status = process.wait()
    timing = usage.read_text()
    receipt = {'command': command, 'exit': status, 'wall_seconds': float(re.search(r'^real ([\d.]+)', timing, re.M)[1]),
               'monitor_wall_seconds': round(time.monotonic() - start, 2),
               'peak_rusage_rss_bytes': resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss,
               'peak_target_disk_bytes': peak_disk,
               'samples': samples, 'sample_interval_seconds': 0.5,
               'measurement': 'macOS getrusage(RUSAGE_CHILDREN) maximum RSS in bytes, including nested waited children (not aggregate concurrent RSS); sampled du -sk target blocks (APFS shared extents can be counted repeatedly)',
               'measurement_errors': sorted(errors)}
    (HERE / (name + '-resources.json')).write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt), flush=True)
    return status


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('run', choices=['warm', 'jobs1', 'jobs2', 'jobs4', 'scoped', 'scoped-child', 'suite'])
    args = parser.parse_args()
    target = ROOT / 'target'
    tree = target / 'mutgate/tree'
    if args.run == 'scoped-child':
        argv = ['mutgate', '--since', 'origin/main', '--scope', 'fn', '--out', str(HERE / 'scoped')]
        with patch.object(mutgate, 'changed_lines', return_value={'src/js_paths_boundary.rs': {169}}), \
                patch.object(sys, 'argv', argv):
            status = mutgate.main()
        summary = json.loads((HERE / 'scoped/summary.json').read_text())
        expected = {'I17-no-project-reference-cut', 'I73-declare-prefilter', 'R4-08-dropped-import-decline'}
        assert set(summary['results']) == expected and summary['selected'] == summary['killed'] == summary['admissible'] == 3
        assert summary['authoritative'] is False
        return status
    if args.run in ('warm', 'suite'):
        mutgate.sync_tree(tree, {})
        manifest = tree / 'Cargo.toml' if args.run == 'warm' else ROOT / 'Cargo.toml'
        command = ['cargo', 'test', '--offline', '--manifest-path', str(manifest)]
        command += ['--lib', '--test', 'integration', '--no-run'] if args.run == 'warm' else ['--all-features', '--no-fail-fast']
    elif args.run == 'scoped':
        command = [sys.executable, str(Path(__file__).resolve()), 'scoped-child']
    else:
        command = [sys.executable, str(ROOT / 'scripts/mutgate/mutgate.py'), '--jobs', args.run[-1],
                   '--out', str(HERE / args.run)]
    status = measure(args.run, command)
    if args.run.startswith('jobs'):
        summary = json.loads((HERE / args.run / 'summary.json').read_text())
        previous = json.loads((HERE.parent / 'repair-r2/text/summary.json').read_text())
        differences = {mid: (previous['results'].get(mid, {}).get('verdict'), result['verdict'])
                       for mid, result in summary['results'].items()
                       if result['verdict'] != previous['results'].get(mid, {}).get('verdict')}
        assert set(summary['results']) == set(previous['results']) and len(summary['results']) == 93
        assert not differences and summary['selected'] == summary['killed'] == summary['admissible'] == 93
        assert summary['authoritative'] and all(r['mode'] == 'text' for r in summary['results'].values())
        assert summary['resources']['workers_cleaned']
        print('93/93 text KILLED; zero ID-bound differences from r2; workers removed', flush=True)
    if args.run == 'suite':
        output = (HERE / 'suite.log').read_text()
        blocks = re.findall(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', output)
        totals = {'result_blocks': len(blocks), 'passed': sum(int(x[1]) for x in blocks),
                  'failed': sum(int(x[2]) for x in blocks), 'ignored': sum(int(x[3]) for x in blocks)}
        (HERE / 'suite-totals.json').write_text(json.dumps(totals, indent=2) + '\n')
        print(json.dumps(totals), flush=True)
    return status


if __name__ == '__main__':
    sys.exit(main())
