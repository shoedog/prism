#!/usr/bin/env python3
"""Memory-report driver for prism-mcp (shared-daemon spec section 6.1).

    memreport.py gen-workload --bin BIN --corpus NAME=PATH [--functions 50]
    memreport.py run --bin BIN --corpus NAME=PATH [--corpus ...] --out DIR
    memreport.py render --run DIR --out FILE.md

Runs are sequential on purpose: one index in memory at a time.
Unit tests: python3 -m unittest scripts/memreport/test_memreport.py
"""
import argparse
import hashlib
import json
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

MARKER = '{"prism_mem_report":'
REPORT_VERSION = 2
KINDS = ('warm', 'cold', 'workload', 'refresh')
CACHE_MISS_MARKERS = ('CPG cache miss', 'partial CPG cache hit')
MIB = 1024 * 1024
RULE_FRACTION = 0.15
RULE_ABSOLUTE = 200 * MIB
CANDIDATES = (('M1', 'm1_high', 'steady'), ('M2', 'm2_transient', 'peak'), ('M3', 'm3', 'steady'))
HERE = Path(__file__).resolve().parent
WORKLOADS = HERE / 'workloads'


def parse_report(stderr_text):
    """The single report line in a prism-mcp stderr capture, parsed."""
    lines = [line for line in stderr_text.splitlines() if line.startswith(MARKER)]
    if len(lines) != 1:
        raise ValueError('expected exactly one report line, found %d' % len(lines))
    return json.loads(lines[0])


def stage(report, name):
    for entry in report['stages']:
        if entry['name'] == name:
            return entry
    raise KeyError('report has no stage %r' % name)


def footprint(entry):
    """Physical footprint where the platform reports it, resident size otherwise."""
    value = entry.get('phys_footprint_bytes')
    return value if value is not None else entry.get('rss_bytes')


def is_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def find_stage(report, name):
    """The named stage, or None when the report has no such stage."""
    stages = report.get('stages')
    if not isinstance(stages, list):
        return None
    for entry in stages:
        if isinstance(entry, dict) and entry.get('name') == name:
            return entry
    return None


FOOTPRINT = 'footprint'
# Per kind: the (stage, field) pairs `derive` reads and cannot do without.
REQUIRED = {
    'warm': (('after_load_repo', 'peak_bytes'), ('after_index', 'peak_bytes'), ('after_index', FOOTPRINT)),
    'cold': (('after_call_edges', 'peak_bytes'),),
    'refresh': (('after_refresh', 'peak_bytes'),),
    'workload': (('after_settle', FOOTPRINT),),
}


def validate(kind, report, expected=None):
    """None when the report may be used as evidence for `kind`, else the reason it may not.

    `expected` (workload runs): {'requests': int, 'sha256': hex} of the workload this run was told to use.
    """
    if kind not in REQUIRED:
        return 'unknown kind %r' % kind
    if report.get('prism_mem_report') != REPORT_VERSION:
        return 'report version is %r, expected %d' % (report.get('prism_mem_report'), REPORT_VERSION)
    for stage_name, field in REQUIRED[kind]:
        entry = find_stage(report, stage_name)
        if entry is None:
            return 'stage %s is missing' % stage_name
        if field == FOOTPRINT:
            if not is_number(footprint(entry)):
                return '%s.phys_footprint_bytes and %s.rss_bytes are unavailable' % (stage_name, stage_name)
        elif not is_number(entry.get(field)):
            return '%s.%s is unavailable' % (stage_name, field)
    if kind == 'refresh':
        refresh = report.get('refresh')
        if not isinstance(refresh, dict):
            return 'report has no refresh section'
        if refresh.get('status') != 'refreshed':
            return 'refresh status is %r, not \'refreshed\' (the tree changed during the refresh)' % (
                refresh.get('status'),)
    if kind == 'workload':
        return validate_workload(report, expected)
    return None


def validate_workload(report, expected):
    if not isinstance(report.get('attribution'), dict):
        return 'report has no attribution section'
    workload = report.get('workload')
    if not isinstance(workload, dict):
        return 'report has no workload section'
    requests, responses, errors = (workload.get(key) for key in ('requests', 'responses', 'errors'))
    if not all(is_number(value) for value in (requests, responses, errors)):
        return 'workload requests/responses/errors are unavailable'
    if errors != 0:
        return 'workload had %d errors' % errors
    if requests == 0:
        return 'workload ran no requests'
    if responses != requests:
        return 'workload answered %d of %d requests' % (responses, requests)
    if expected is not None:
        if requests != expected['requests']:
            return 'workload ran %d requests, the manifest has %d' % (requests, expected['requests'])
        if workload.get('sha256') != expected['sha256']:
            return 'workload sha256 %s differs from the manifest %s' % (workload.get('sha256'), expected['sha256'])
    return None


def summarize(values):
    values = [value for value in values if value is not None]
    if not values:
        return None
    return {'min': min(values), 'median': statistics.median(values), 'max': max(values), 'n': len(values)}


def num(mapping, key):
    """mapping[key] when it is a number, else None (absent, null or malformed)."""
    value = mapping.get(key) if isinstance(mapping, dict) else None
    return value if is_number(value) else None


def difference(high, low):
    return None if high is None or low is None else high - low


def derive(kind, report):
    """The per-run quantities the readout is built from (bytes). A value that is unavailable is None.

    Never raises for a report that `validate(kind, report)` accepted.
    """
    if kind == 'warm':
        loaded, built = stage(report, 'after_load_repo'), stage(report, 'after_index')
        transient = built['peak_bytes'] - max(loaded['peak_bytes'], footprint(built))
        return {'peak': built['peak_bytes'], 'm2_transient': max(0, transient)}
    if kind == 'cold':
        return {'cold_peak': stage(report, 'after_call_edges')['peak_bytes']}
    if kind == 'refresh':
        return {'refresh_peak': stage(report, 'after_refresh')['peak_bytes']}
    if kind == 'workload':
        attribution = report['attribution']
        return {
            'steady': footprint(stage(report, 'after_settle')),
            'm1_low': num(attribution, 'tree_bytes_estimate_low'),
            'm1_high': num(attribution, 'tree_bytes_estimate_high'),
            'm3': difference(num(attribution, 'node_string_heap_bytes_estimate'),
                             num(attribution, 'node_unique_string_heap_bytes_estimate')),
        }
    raise ValueError('unknown kind %r' % kind)


def meets(estimate, base):
    if estimate is None or not base:
        return False
    return estimate >= RULE_ABSOLUTE or estimate / base >= RULE_FRACTION


def evaluate(rows):
    """rows: {corpus: {metric: median bytes}} -> {candidate: {'corpora': [...], 'advance': bool}}"""
    verdict = {}
    for candidate, estimate_key, base_key in CANDIDATES:
        hits = sorted(name for name, row in rows.items() if meets(row.get(estimate_key), row.get(base_key)))
        verdict[candidate] = {'corpora': hits, 'advance': len(hits) >= 2}
    return verdict


def command_for(binary, repo, cache_dir, kind, workload, settle):
    command = [str(binary), '--repo', str(repo), '--eager', '--mem-settle-secs', str(settle)]
    command += ['--no-cache'] if kind == 'cold' else ['--cache-dir', str(cache_dir)]
    if kind == 'workload':
        command += ['--mem-workload', str(workload)]
    if kind == 'refresh':
        command += ['--mem-refresh']
    return command


def run_one(command, kind, timeout, expected=None):
    """Run one measurement. Never raises for a bad run: the result says what happened."""
    env = dict(os.environ, PRISM_MCP_MEM_REPORT='1')
    started = time.time()
    try:
        done = subprocess.run(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE, timeout=timeout, env=env)
    except subprocess.TimeoutExpired:
        return {'status': 'timeout', 'reason': 'timeout', 'command': command, 'wall_secs': timeout}
    result = {'command': command, 'wall_secs': round(time.time() - started, 1)}
    stderr = done.stderr.decode('utf-8', 'replace')
    if done.returncode != 0:
        result.update(status='failed', reason='exit %d' % done.returncode, exit=done.returncode,
                      stderr_tail=stderr[-2000:])
        return result
    try:
        report = parse_report(stderr)
    except ValueError as error:
        result.update(status='inadmissible', reason=str(error), stderr_tail=stderr[-2000:])
        return result
    result['report'] = report
    if kind != 'cold' and any(marker in stderr for marker in CACHE_MISS_MARKERS):
        result.update(status='inadmissible', reason='the cache missed, so this was a cold build')
    else:
        reason = validate(kind, report, expected)
        if reason is None:
            result['status'] = 'ok'
        else:
            result.update(status='inadmissible', reason=reason)
    return result


def capture(command):
    try:
        return subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                              timeout=30).stdout.decode('utf-8', 'replace').strip()
    except (OSError, subprocess.TimeoutExpired):
        return None


def host_state():
    """Swap and compressor state at the time of a run (spec 6.1)."""
    if sys.platform == 'darwin':
        return {'swapusage': capture(['sysctl', '-n', 'vm.swapusage']), 'vm_stat': capture(['vm_stat'])}
    try:
        return {'meminfo': Path('/proc/meminfo').read_text()[:1500]}
    except OSError:
        return {}


def corpus_provenance(path):
    status = capture(['git', '-C', str(path), 'status', '--porcelain']) or ''
    return {'path': str(path), 'commit': capture(['git', '-C', str(path), 'rev-parse', 'HEAD']),
            'dirty': len([line for line in status.splitlines() if line])}


def parse_corpora(pairs):
    corpora = {}
    for pair in pairs:
        name, sep, path = pair.partition('=')
        if not sep or not name or not path:
            raise SystemExit('--corpus takes NAME=PATH, got %r' % pair)
        corpora[name] = Path(path).expanduser().resolve()
    return corpora


def cmd_gen_workload(args):
    WORKLOADS.mkdir(parents=True, exist_ok=True)
    manifest_path = WORKLOADS / 'MANIFEST.json'
    manifest = json.loads(manifest_path.read_text()) if manifest_path.exists() else {}
    for name, path in parse_corpora(args.corpus).items():
        command = [args.bin, '--repo', str(path), '--eager', '--no-cache',
                   '--mem-emit-workload', str(args.functions)]
        done = subprocess.run(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE, timeout=args.timeout,
                              env=dict(os.environ, PRISM_MCP_MEM_REPORT='1'))
        if done.returncode != 0:
            raise SystemExit('%s: workload generation failed:\n%s' % (name, done.stderr.decode()[-2000:]))
        (WORKLOADS / (name + '.jsonl')).write_bytes(done.stdout)
        manifest[name] = dict(corpus_provenance(path), requests=done.stdout.count(b'\n'),
                              functions_requested=args.functions,
                              sha256=hashlib.sha256(done.stdout).hexdigest())
        print('%s: %d requests' % (name, manifest[name]['requests']))
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n')


def cmd_run(args):
    out = Path(args.out)
    summary = {'provenance': {'reps': args.reps, 'settle': args.settle, 'timeout': args.timeout,
                              'bin': args.bin, 'build': None},
               'corpora': {}}
    for name, path in parse_corpora(args.corpus).items():
        workload = WORKLOADS / (name + '.jsonl')
        if not workload.exists():
            raise SystemExit('%s: no workload at %s; run gen-workload first' % (name, workload))
        raw = out / 'raw' / name
        raw.mkdir(parents=True, exist_ok=True)
        cache_dir = out / 'cache' / name
        entry = dict(corpus_provenance(path), admissible={kind: 0 for kind in KINDS},
                     inadmissible=[], metrics={}, attribution=None)
        # Prime the cache with a discarded run so that "warm" means a cache hit.
        prime = run_one(command_for(args.bin, path, cache_dir, 'warm', workload, 0), 'cold', args.timeout)
        (raw / 'prime.json').write_text(json.dumps(prime, indent=1))
        values = {}
        for kind in KINDS:
            for rep in range(1, args.reps + 1):
                before = host_state()
                result = run_one(command_for(args.bin, path, cache_dir, kind, workload, args.settle),
                                 kind, args.timeout)
                result['host_before'] = before
                (raw / ('%s-%d.json' % (kind, rep))).write_text(json.dumps(result, indent=1))
                print('%s %s #%d: %s (%ss)' % (name, kind, rep, result['status'], result['wall_secs']),
                      flush=True)
                if result['status'] != 'ok':
                    entry['inadmissible'].append({'kind': kind, 'rep': rep, 'status': result['status'],
                                                  'reason': result['reason']})
                    continue
                entry['admissible'][kind] += 1
                report = result['report']
                summary['provenance']['build'] = report['build']
                if kind == 'workload':
                    entry['attribution'] = report['attribution']
                for metric, value in derive(kind, report).items():
                    values.setdefault(metric, []).append(value)
        entry['metrics'] = {metric: summarize(series) for metric, series in sorted(values.items())}
        summary['corpora'][name] = entry
    rows = {name: {metric: (stats or {}).get('median') for metric, stats in entry['metrics'].items()}
            for name, entry in summary['corpora'].items()}
    summary['verdict'] = evaluate(rows)
    (out / 'summary.json').write_text(json.dumps(summary, indent=2, sort_keys=True) + '\n')
    print('wrote %s' % (out / 'summary.json'))


def mib(stats):
    if not stats:
        return 'n/a'
    return '%.1f (%.1f–%.1f, n=%d)' % (stats['median'] / MIB, stats['min'] / MIB, stats['max'] / MIB, stats['n'])


def render(summary):
    build = summary['provenance'].get('build') or {}
    lines = [
        '# prism-mcp memory readout',
        '',
        'This is **coarse triage**: it ranks reduction candidates for an experiment and selects none.',
        'Byte figures marked "estimate" come from component counts, not from allocation measurement.',
        '',
        '## Provenance',
        '',
        '- Binary: build identity `%s`, git `%s`, CPG cache version %s, allocator %s.' % (
            build.get('build_identity'), build.get('git_sha'), build.get('cpg_cache_version'),
            build.get('allocator')),
        '- %s repetitions per kind, %s s settle. Values are median (min–max, n) in MiB.' % (
            summary['provenance'].get('reps'), summary['provenance'].get('settle')),
        '',
        '| Corpus | Commit | Dirty files | Admissible runs (warm/cold/workload/refresh) |',
        '|---|---|---|---|',
    ]
    for name, entry in sorted(summary['corpora'].items()):
        counts = '/'.join(str(entry['admissible'][kind]) for kind in KINDS)
        lines.append('| %s | `%s` | %s | %s |' % (name, (entry.get('commit') or '')[:12], entry.get('dirty'), counts))
    lines += ['', '## Footprint', '',
              '| Corpus | Steady state | Warm-load peak | Load transient (M2) | Refresh peak | Cold peak |',
              '|---|---|---|---|---|---|']
    for name, entry in sorted(summary['corpora'].items()):
        metrics = entry['metrics']
        lines.append('| %s | %s | %s | %s | %s | %s |' % (
            name, mib(metrics.get('steady')), mib(metrics.get('peak')), mib(metrics.get('m2_transient')),
            mib(metrics.get('refresh_peak')), mib(metrics.get('cold_peak'))))
    lines += ['', '## Attribution (estimates)', '',
              '| Corpus | Files | Source | Tree nodes | Trees, low–high (M1) | CPG nodes / edges | Repeated node strings (M3) | Unique-byte ratio |',
              '|---|---|---|---|---|---|---|---|']
    for name, entry in sorted(summary['corpora'].items()):
        attribution, metrics = entry.get('attribution') or {}, entry['metrics']
        ratio = attribution.get('node_string_unique_ratio')
        lines.append('| %s | %s | %.1f | %s | %s – %s | %s / %s | %s | %s |' % (
            name, attribution.get('files'), (attribution.get('source_bytes') or 0) / MIB,
            attribution.get('tree_nodes'), mib(metrics.get('m1_low')), mib(metrics.get('m1_high')),
            attribution.get('cpg_nodes'), attribution.get('cpg_edges'), mib(metrics.get('m3')),
            'n/a' if ratio is None else '%.3f' % ratio))
    lines += ['', '## Decision rule', '',
              'A candidate advances to its experiment when its estimate is at least 200 MiB or 15 % of the',
              'base (steady state for M1 and M3, warm-load peak for M2) on at least two corpora.', '',
              '| Candidate | Corpora meeting the rule | Advances |', '|---|---|---|']
    for candidate, _, _ in CANDIDATES:
        verdict = summary['verdict'][candidate]
        lines.append('| %s | %s | %s |' % (candidate, ', '.join(verdict['corpora']) or '—',
                                           'yes' if verdict['advance'] else 'no'))
    excluded = [(name, item) for name, entry in sorted(summary['corpora'].items())
                for item in entry['inadmissible']]
    lines += ['', '## Excluded runs', '']
    if excluded:
        lines += ['| Corpus | Kind | Repetition | Status | Reason |', '|---|---|---|---|---|']
        lines += ['| %s | %s | %s | %s | %s |' % (name, item['kind'], item['rep'], item['status'], item['reason'])
                  for name, item in excluded]
    else:
        lines.append('None.')
    return '\n'.join(lines) + '\n'


def cmd_render(args):
    summary = json.loads((Path(args.run) / 'summary.json').read_text())
    Path(args.out).write_text(render(summary))
    print('wrote %s' % args.out)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest='command', required=True)
    gen = commands.add_parser('gen-workload')
    gen.add_argument('--bin', required=True)
    gen.add_argument('--corpus', action='append', required=True, metavar='NAME=PATH')
    gen.add_argument('--functions', type=int, default=50)
    gen.add_argument('--timeout', type=int, default=3600)
    gen.set_defaults(handler=cmd_gen_workload)
    run = commands.add_parser('run')
    run.add_argument('--bin', required=True)
    run.add_argument('--corpus', action='append', required=True, metavar='NAME=PATH')
    run.add_argument('--out', required=True)
    run.add_argument('--reps', type=int, default=3)
    run.add_argument('--timeout', type=int, default=1800)
    run.add_argument('--settle', type=int, default=5)
    run.set_defaults(handler=cmd_run)
    rendering = commands.add_parser('render')
    rendering.add_argument('--run', required=True)
    rendering.add_argument('--out', required=True)
    rendering.set_defaults(handler=cmd_render)
    args = parser.parse_args(argv)
    args.handler(args)


if __name__ == '__main__':
    main()
