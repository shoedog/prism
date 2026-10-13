#!/usr/bin/env python3
"""Memory-report driver for prism-mcp (shared-daemon spec section 6.1).

    memreport.py gen-workload --bin BIN --corpus NAME=PATH [--functions 50] [--workloads DIR]
    memreport.py run --bin BIN --corpus NAME=PATH [--corpus ...] --out DIR [--workloads DIR] [--kinds LIST]
    memreport.py summarize --run DIR
    memreport.py render --run DIR --out FILE.md
    memreport.py compare --experiment m2 --baseline DIR --candidate DIR --out FILE.md [--json FILE.json]

Runs are sequential on purpose: one index in memory at a time.
Unit tests: python3 -m unittest scripts/memreport/test_memreport.py
"""
import argparse
import hashlib
import json
import os
import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path

from memreport_compare import EXPERIMENTS, cmd_compare
from memreport_derive import derive, difference, num  # noqa: F401
from memreport_render import KINDS, MIB, render
# The validation names are re-exported: tests and callers use them as `memreport.validate` and so on.
from memreport_validate import (BUILD, CACHE_REASONS, CACHED_KINDS, PRIME, REPORT_VERSION, REQUIRED,  # noqa: F401
                                find_stage, footprint, is_number, validate, well_formed)

MARKER = '{"prism_mem_report":'
# stderr text that means a cache-loaded run did not load everything from cache. The report's `cache` section
# is the evidence that decides admission; these only name the cause more precisely when they are present.
CACHE_MISS_MARKERS = (
    ('CPG cache miss', 'the CPG cache missed, so this was a cold build'),
    ('partial CPG cache hit', 'the CPG cache partially missed, so part of the index was rebuilt'),
    ('nav edge cache: skipping warm sidecar load',
     'the call-edge sidecar was not loaded (binary built from a dirty tree), so call edges were recomputed'),
    ('nav edge cache: failed to load sidecar',
     'the call-edge sidecar failed to load, so call edges were recomputed'),
)
RUN_KINDS = KINDS + (BUILD,)  # every kind `run` accepts; the readout lists only KINDS
RULE_FRACTION = 0.15
RULE_ABSOLUTE = 200 * MIB
# candidate -> (estimate metric, base metric). M1 and M3 are judged against the steady state, M2 against the
# peak through the index load, because that is the footprint each one would lower.
CANDIDATES = (('M1', 'm1', 'steady'), ('M2', 'm2', 'peak'), ('M3', 'm3', 'steady'))
CACHE_FILES = {'cpg_cache_bytes': 'cpg-cache.bin', 'call_edge_sidecar_bytes': 'resolved-call-edge-index.bin'}
HERE = Path(__file__).resolve().parent
WORKLOADS = HERE / 'workloads'


def parse_report(stderr_text):
    """The single report line in a prism-mcp stderr capture, parsed."""
    lines = [line for line in stderr_text.splitlines() if line.startswith(MARKER)]
    if len(lines) != 1:
        raise ValueError('expected exactly one report line, found %d' % len(lines))
    return json.loads(lines[0])


def summarize(values):
    values = [value for value in values if value is not None]
    if not values:
        return None
    return {'min': min(values), 'median': statistics.median(values), 'max': max(values), 'n': len(values)}


def rule_arm(estimate, base):
    """Which arm of the decision rule an estimate meets: absolute, fraction, both, none; None if unjudgeable."""
    if estimate is None or not base:
        return None
    absolute, fraction = estimate >= RULE_ABSOLUTE, estimate / base >= RULE_FRACTION
    if absolute and fraction:
        return 'both'
    return 'absolute' if absolute else 'fraction' if fraction else 'none'


def meets(estimate, base):
    return rule_arm(estimate, base) in ('absolute', 'fraction', 'both')


def decision_points(entry):
    """{metric: (value, n)} the decision rule reads from one corpus entry.

    Per-repetition metrics are medians with the number of runs behind them. m2 is not a series: it is the
    size of the CPG cache file the priming run wrote, one measurement (n is None).
    """
    points = {}
    for metric, stats in (entry.get('metrics') or {}).items():
        points[metric] = (stats['median'], stats['n']) if stats else (None, 0)
    points['m2'] = ((entry.get('cache_files') or {}).get('cpg_cache_bytes'), None)
    return points


def evaluate(entries):
    """entries: {corpus: corpus entry} -> {candidate: {'corpora': [...], 'advance': bool, 'rows': {...}}}"""
    verdict = {}
    for candidate, estimate_key, base_key in CANDIDATES:
        rows = {}
        for name, entry in entries.items():
            points = decision_points(entry)
            (estimate, estimate_n), (base, base_n) = (points.get(estimate_key, (None, 0)),
                                                      points.get(base_key, (None, 0)))
            rows[name] = {'estimate': estimate, 'base': base, 'estimate_n': estimate_n, 'base_n': base_n,
                          'percent': 100.0 * estimate / base if estimate is not None and base else None,
                          'arm': rule_arm(estimate, base), 'meets': meets(estimate, base)}
        hits = sorted(name for name, row in rows.items() if row['meets'])
        verdict[candidate] = {'corpora': hits, 'advance': len(hits) >= 2, 'rows': rows}
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
        return {'kind': kind, 'status': 'timeout', 'reason': 'timeout', 'command': command,
                'wall_secs': timeout}
    result = {'kind': kind, 'command': command, 'wall_secs': round(time.time() - started, 1)}
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
    reason = validate(kind, report, expected)
    # The report's own cache section decides admission. A marker on stderr is a second witness: it can only
    # exclude a run, and it names the cause more precisely than the report's reason does.
    missed = [text for marker, text in CACHE_MISS_MARKERS if marker in stderr] if kind in CACHED_KINDS else []
    if missed:
        reasons = missed + ([reason] if reason and reason not in CACHE_REASONS else [])
        result.update(status='inadmissible', reason='; '.join(reasons))
    elif reason is None:
        result['status'] = 'ok'
    else:
        result.update(status='inadmissible', reason=reason)
    return result


def capture(command):
    """The stripped stdout of a command, or None when it could not run or exited non-zero."""
    try:
        done = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return done.stdout.decode('utf-8', 'replace').strip() if done.returncode == 0 else None


def host_state():
    """Swap and compressor state at the time of a run (spec 6.1)."""
    if sys.platform == 'darwin':
        return {'swapusage': capture(['sysctl', '-n', 'vm.swapusage']), 'vm_stat': capture(['vm_stat'])}
    try:
        return {'meminfo': Path('/proc/meminfo').read_text()[:1500]}
    except OSError:
        return {}


def corpus_provenance(path):
    """Path, commit and count of dirty files; commit and dirty are None when git cannot say (not a checkout)."""
    status = capture(['git', '-C', str(path), 'status', '--porcelain'])
    return {'path': str(path), 'commit': capture(['git', '-C', str(path), 'rev-parse', 'HEAD']) or None,
            'dirty': None if status is None else len([line for line in status.splitlines() if line])}


def parse_kinds(text):
    """The kinds named by `--kinds` in the order `run` executes them; exits on an unknown, repeated or empty name."""
    names = text.split(',')
    problems = ['%r is not a kind' % name for name in names if name not in RUN_KINDS]
    problems += ['%s is listed twice' % name for name in sorted(set(names)) if names.count(name) > 1]
    if problems:
        raise SystemExit('--kinds %r: %s (kinds: %s)' % (text, '; '.join(problems), ','.join(RUN_KINDS)))
    return tuple(kind for kind in RUN_KINDS if kind in names)


def parse_corpora(pairs):
    corpora = {}
    for pair in pairs:
        name, sep, path = pair.partition('=')
        if not sep or not name or not path:
            raise SystemExit('--corpus takes NAME=PATH, got %r' % pair)
        corpora[name] = Path(path).expanduser().resolve()
    return corpora


def write_json_atomically(path, data):
    temporary = path.with_name(path.name + '.tmp')
    temporary.write_text(json.dumps(data, indent=2, sort_keys=True) + '\n')
    os.replace(temporary, path)


def emit_workload(binary, path, functions, timeout):
    """(stdout bytes, None) for a workload the binary emitted, or (None, why not)."""
    command = [binary, '--repo', str(path), '--eager', '--no-cache', '--mem-emit-workload', str(functions)]
    try:
        done = subprocess.run(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE, timeout=timeout,
                              env=dict(os.environ, PRISM_MCP_MEM_REPORT='1'))
    except subprocess.TimeoutExpired:
        return None, 'timed out after %ss' % timeout
    except OSError as error:
        return None, 'could not run %s: %s' % (binary, error)
    if done.returncode != 0:
        return None, 'exit %d:\n%s' % (done.returncode, done.stderr.decode('utf-8', 'replace')[-2000:])
    if done.stdout.count(b'\n') == 0:
        return None, 'emitted no requests'
    return done.stdout, None


def cmd_gen_workload(args):
    """Generate a workload per corpus. Each finished corpus is written (file and manifest) before the next starts."""
    directory = Path(args.workloads)
    directory.mkdir(parents=True, exist_ok=True)
    manifest_path = directory / 'MANIFEST.json'
    manifest = json.loads(manifest_path.read_text()) if manifest_path.exists() else {}
    failed = []
    for name, path in parse_corpora(args.corpus).items():
        data, problem = emit_workload(args.bin, path, args.functions, args.timeout)
        if problem is not None:
            print('%s: workload generation failed: %s' % (name, problem), file=sys.stderr, flush=True)
            failed.append(name)
            continue
        (directory / (name + '.jsonl')).write_bytes(data)
        manifest[name] = dict(corpus_provenance(path), requests=data.count(b'\n'),
                              functions_requested=args.functions, sha256=hashlib.sha256(data).hexdigest())
        write_json_atomically(manifest_path, manifest)
        print('%s: %d requests' % (name, manifest[name]['requests']), flush=True)
    if failed:
        raise SystemExit('workload generation failed for: %s' % ', '.join(failed))


def single_file_size(directory, filename):
    """Size of the one file called `filename` under `directory` (recursive); None unless exactly one."""
    matches = [path for path in Path(directory).rglob(filename) if path.is_file()]
    return matches[0].stat().st_size if len(matches) == 1 else None


def cache_file_sizes(cache_dir):
    return {key: single_file_size(cache_dir, filename) for key, filename in CACHE_FILES.items()}


def bind_workload(name, directory, commit):
    """Read a corpus's workload once and check it against the manifest.

    Returns (bytes, {'sha256', 'requests'}); exits with a message when the file is missing, is not in the
    manifest, differs from it, or was generated at another commit than the corpus is at now.
    """
    if not commit:
        raise SystemExit('%s: the corpus has no git commit (not a git checkout, or no commits), so a workload '
                         'cannot be bound to it' % name)
    path = Path(directory) / (name + '.jsonl')
    if not path.exists():
        raise SystemExit('%s: no workload at %s; run gen-workload first' % (name, path))
    data = path.read_bytes()
    manifest_path = Path(directory) / 'MANIFEST.json'
    manifest = json.loads(manifest_path.read_text()) if manifest_path.exists() else {}
    entry = manifest.get(name)
    if entry is None:
        raise SystemExit('%s: %s has no entry for this corpus; re-run gen-workload' % (name, manifest_path))
    digest = {'sha256': hashlib.sha256(data).hexdigest(), 'requests': data.count(b'\n')}
    if digest['sha256'] != entry.get('sha256') or digest['requests'] != entry.get('requests'):
        raise SystemExit('%s: %s has sha256 %s and %d requests but the manifest has %s and %s; '
                         're-run gen-workload' % (name, path, digest['sha256'], digest['requests'],
                                                  entry.get('sha256'), entry.get('requests')))
    if entry.get('commit') != commit:
        raise SystemExit('%s: the workload was generated at commit %s but the corpus is at %s; '
                         're-run gen-workload' % (name, entry.get('commit'), commit))
    return data, digest


def effective_status(result, expected=None):
    """(status, reason) of a stored result after validating its report again with today's rules.

    The report carries the cache evidence (`cache`), so the cache rules are re-applied too. A result can only
    lose admission here: markers seen on stderr at run time are not stored, so an inadmissible result stays
    inadmissible whatever its report says.
    """
    status = result.get('status')
    if status != 'ok':
        return status or 'inadmissible', result.get('reason') or 'result has no status'
    if not isinstance(result.get('report'), dict):
        return 'inadmissible', 'result has no report'
    reason = validate(result['kind'], result['report'], expected)
    return ('ok', None) if reason is None else ('inadmissible', reason)


def prime_peak(prime):
    """Lifetime peak (bytes) at the last stage of the priming run, the build that writes the cache.

    None when the priming result is absent, was not admitted, reports no peak, or did not build
    the CPG (a cache directory that was already primed makes the "priming" run a warm load, whose
    peak is not a first-build peak).
    """
    if not isinstance(prime, dict) or prime.get('status') != 'ok':
        return None
    report = prime.get('report')
    if not isinstance(report, dict) or validate(PRIME, report) is not None or not report['stages']:
        return None
    if report['cache'].get('cpg_hit') is not False:
        return None
    peak = report['stages'][-1].get('peak_bytes')
    return peak if is_number(peak) else None


def build_entry(corpus, results, prime=None):
    """One corpus's entry of summary.json from its corpus.json content, its raw results and its priming result.

    Pure: no subprocess, no clock, no filesystem. A result that is not admissible contributes to
    `inadmissible` and to nothing else. Every admitted result of a corpus must come from the build the first
    admitted one came from.
    """
    shown = KINDS + ((BUILD,) if any(item['kind'] == BUILD for item in results) else ())
    entry = dict(corpus, admissible={kind: 0 for kind in shown}, inadmissible=[], metrics={},
                 attribution=None, build=None, env=None, threads=None, first_build_peak=prime_peak(prime))
    values, identity = {}, None
    for result in sorted(results, key=lambda item: (RUN_KINDS.index(item['kind']), item['rep'])):
        kind, rep = result['kind'], result['rep']
        status, reason = effective_status(result, corpus.get('workload') if kind == 'workload' else None)
        if status == 'ok':
            this_identity = result['report']['build']['build_identity']
            if identity is None:
                identity = this_identity
            elif this_identity != identity:
                status, reason = 'inadmissible', "build identity differs from this corpus's first admitted run"
        if status != 'ok':
            entry['inadmissible'].append({'kind': kind, 'rep': rep, 'status': status, 'reason': reason})
            continue
        entry['admissible'][kind] += 1
        report = result['report']
        entry['build'], entry['env'], entry['threads'] = report.get('build'), report.get('env'), report.get('threads')
        if kind == 'workload':
            entry['attribution'] = report['attribution']
        for metric, value in derive(kind, report).items():
            values.setdefault(metric, []).append(value)
    entry['metrics'] = {metric: summarize(series) for metric, series in sorted(values.items())}
    return entry


def exclude_entry(entry, reason):
    """A copy of a corpus entry with every admitted run moved to `inadmissible` and nothing left to measure."""
    excluded = dict(entry, admissible={kind: 0 for kind in entry['admissible']}, metrics={}, attribution=None,
                    build=None, env=None, threads=None, first_build_peak=None,
                    cache_files=dict.fromkeys(entry.get('cache_files') or {}))
    excluded['inadmissible'] = list(entry['inadmissible']) + [
        {'kind': kind, 'rep': 'all %d' % count, 'status': 'inadmissible', 'reason': reason}
        for kind, count in entry['admissible'].items() if count]
    return excluded


def assemble_summary(settings, entries):
    """Settings, the corpora and the verdict. Every corpus must come from the build of the first corpus (by
    name) that has an admitted run; one that does not is excluded whole."""
    provenance = dict(settings, build=None, env=None, threads=None)
    checked = {}
    for name in sorted(entries):
        entry = entries[name]
        build = entry.get('build')
        if build is not None and provenance['build'] is None:
            provenance.update(build=build, env=entry.get('env'), threads=entry.get('threads'))
        elif build is not None and build.get('build_identity') != provenance['build'].get('build_identity'):
            entry = exclude_entry(entry, "build identity differs from the first corpus's (%s)" % (
                provenance['build'].get('build_identity'),))
        checked[name] = entry
    return {'provenance': provenance, 'corpora': checked, 'verdict': evaluate(checked)}


def write_summary(out, summary):
    path = Path(out) / 'summary.json'
    write_json_atomically(path, summary)
    return path


def run_build(args, path, out, name, rep, snapshot):
    """One first build into a cache directory of its own; its file sizes are recorded, then it is deleted."""
    cache_dir = out / 'cache-build' / name / str(rep)
    shutil.rmtree(cache_dir, ignore_errors=True)
    cache_dir.mkdir(parents=True)
    try:
        result = run_one(command_for(args.bin, path, cache_dir, BUILD, snapshot, args.settle), BUILD, args.timeout)
        result['cache_files'] = cache_file_sizes(cache_dir)
    finally:
        shutil.rmtree(cache_dir, ignore_errors=True)
    return result


def measure_corpus(args, name, path, provenance, snapshot, digest, kinds=KINDS):
    """Run the chosen kinds for one corpus, saving each raw result as it finishes. Returns (corpus, results, prime)."""
    out = Path(args.out)
    raw = out / 'raw' / name
    raw.mkdir(parents=True, exist_ok=True)
    cache_dir = out / 'cache' / name
    prime = None
    if set(kinds) & set(CACHED_KINDS):
        # Prime the cache with a discarded run so that "warm" means a cache hit.
        prime = run_one(command_for(args.bin, path, cache_dir, 'warm', snapshot, 0), PRIME, args.timeout)
        (raw / 'prime.json').write_text(json.dumps(prime, indent=1))
    corpus = dict(provenance, cache_files=cache_file_sizes(cache_dir), workload=digest)
    (raw / 'corpus.json').write_text(json.dumps(corpus, indent=1, sort_keys=True))
    results = []
    for kind in kinds:
        for rep in range(1, args.reps + 1):
            before = host_state()
            if kind == BUILD:
                result = run_build(args, path, out, name, rep, snapshot)
            else:
                result = run_one(command_for(args.bin, path, cache_dir, kind, snapshot, args.settle),
                                 kind, args.timeout, digest if kind == 'workload' else None)
            result.update(rep=rep, host_before=before)
            (raw / ('%s-%d.json' % (kind, rep))).write_text(json.dumps(result, indent=1))
            print('%s %s #%d: %s (%ss)' % (name, kind, rep, result['status'], result['wall_secs']), flush=True)
            results.append(result)
    return corpus, results, prime


def cmd_run(args):
    kinds = parse_kinds(args.kinds)  # before anything else: a typo must not cost a run
    out = Path(args.out)
    # One run directory holds one run: a second run would overwrite some raw files and leave others, and
    # `summarize` would pool the two.
    if any((out / name).exists() for name in ('raw', 'run.json', 'summary.json')):
        raise SystemExit("run directory already has results; use a new --out, or 'summarize' to rescore")
    settings = {'reps': args.reps, 'settle': args.settle, 'timeout': args.timeout, 'bin': args.bin}
    # Bind every corpus to its workload before any run starts: a mismatch must not cost hours first.
    bound = {}
    for name, path in parse_corpora(args.corpus).items():
        provenance = corpus_provenance(path)
        bound[name] = (path, provenance) + bind_workload(name, args.workloads, provenance['commit'])
    out.mkdir(parents=True, exist_ok=True)
    (out / 'run.json').write_text(json.dumps(dict(settings, kinds=list(kinds)), indent=1, sort_keys=True))
    entries = {}
    for name, (path, provenance, data, digest) in bound.items():
        snapshot = out / 'workloads' / (name + '.jsonl')
        snapshot.parent.mkdir(parents=True, exist_ok=True)
        snapshot.write_bytes(data)
        corpus, results, prime = measure_corpus(args, name, path, provenance, snapshot, digest, kinds)
        entries[name] = build_entry(corpus, results, prime)
        write_summary(out, assemble_summary(settings, entries))  # kept after every corpus, not just at the end
    print('wrote %s' % (out / 'summary.json'))


def cmd_summarize(args):
    """Rebuild summary.json from raw/ alone, validating every stored report with today's rules."""
    out = Path(args.run)
    settings = json.loads((out / 'run.json').read_text())
    settings.pop('kinds', None)  # which kinds ran is what raw/ holds; a run.json from before --kinds has no key
    entries = {}
    for directory in sorted(path for path in (out / 'raw').iterdir() if path.is_dir()):
        corpus_path = directory / 'corpus.json'
        if not corpus_path.exists():
            raise SystemExit('%s: no corpus.json, so this corpus cannot be summarized' % directory)
        results = []
        for path in sorted(directory.glob('*.json')):
            if path.name in ('corpus.json', 'prime.json'):
                continue
            result = json.loads(path.read_text())
            if result.get('kind') not in RUN_KINDS or not isinstance(result.get('rep'), int):
                raise SystemExit('%s: no kind/rep recorded; re-run instead' % path)
            results.append(result)
        prime_path = directory / 'prime.json'
        prime = json.loads(prime_path.read_text()) if prime_path.exists() else None
        entries[directory.name] = build_entry(json.loads(corpus_path.read_text()), results, prime)
    print('wrote %s' % write_summary(out, assemble_summary(settings, entries)))


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
    gen.add_argument('--workloads', type=Path, default=WORKLOADS, metavar='DIR',
                     help='where workloads/*.jsonl and MANIFEST.json live (default: %(default)s)')
    gen.add_argument('--functions', type=int, default=50)
    gen.add_argument('--timeout', type=int, default=3600)
    gen.set_defaults(handler=cmd_gen_workload)
    run = commands.add_parser('run')
    run.add_argument('--bin', required=True)
    run.add_argument('--corpus', action='append', required=True, metavar='NAME=PATH')
    run.add_argument('--out', required=True)
    run.add_argument('--workloads', type=Path, default=WORKLOADS, metavar='DIR',
                     help='where workloads/*.jsonl and MANIFEST.json live (default: %(default)s)')
    run.add_argument('--kinds', default=','.join(KINDS), metavar='LIST',
                     help='comma-separated subset of %s (default: %%(default)s)' % ','.join(RUN_KINDS))
    run.add_argument('--reps', type=int, default=3)
    run.add_argument('--timeout', type=int, default=1800)
    run.add_argument('--settle', type=int, default=5)
    run.set_defaults(handler=cmd_run)
    summarizing = commands.add_parser('summarize', help='rebuild summary.json from raw/ of an earlier run')
    summarizing.add_argument('--run', required=True)
    summarizing.set_defaults(handler=cmd_summarize)
    rendering = commands.add_parser('render')
    rendering.add_argument('--run', required=True)
    rendering.add_argument('--out', required=True)
    rendering.set_defaults(handler=cmd_render)
    comparing = commands.add_parser('compare', help='baseline run against candidate run under a pinned rule')
    comparing.add_argument('--experiment', required=True, choices=sorted(EXPERIMENTS))
    comparing.add_argument('--baseline', required=True, type=Path, metavar='DIR')
    comparing.add_argument('--candidate', required=True, type=Path, metavar='DIR')
    comparing.add_argument('--out', required=True, metavar='FILE.md')
    comparing.add_argument('--json', metavar='FILE.json', help='the same content as data')
    comparing.add_argument('--allow-same-build', action='store_true',
                           help='compare runs of one build identity (a noise check)')
    comparing.set_defaults(handler=cmd_compare)
    args = parser.parse_args(argv)
    args.handler(args)


if __name__ == '__main__':
    main()
