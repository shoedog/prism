#!/usr/bin/env python3
"""Fan-out latency driver for prism-mcp (shared-daemon spec sections 6.2 and 8 step 2).

    fanout.py run --bin BIN --corpus NAME=PATH --cache-dir DIR --mode shared|standalone --out DIR
                  [--clients 8] [--workloads DIR] [--refresh-at 0.5 | --no-refresh] [--timeout 1800]
    fanout.py compare --baseline DIR [--baseline DIR ...] --candidate DIR [--candidate DIR ...]
                      --out FILE.md [--json FILE.json]

`run` starts CLIENTS prism-mcp processes on one corpus: with --mode shared each is a shim
(`--shared`) of one daemon in a runtime directory of the run's own, which `run` stops at the end
with `daemon-shutdown`; with --mode standalone each holds an index of its own. Every client
initializes and warms up (its first workload call, retried while the index is not ready); then
all start the corpus's pinned workload together, each from its own offset, one call at a time,
and each call is timed from its write to its response. Client 0 also calls refresh_index once,
REFRESH_AT of the way through its calls. The run directory holds run.json (provenance, host,
memory, daemon instances), calls.jsonl (one line per timed call) and summary.json.

`compare` judges candidate runs against baseline runs of the same corpus and workload under the
pinned rule (P95_MAX_RATIO, P99_MAX_RATIO on the medians across runs; every run admissible).

Python 3.9, standard library only. Unit tests:
    python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import argparse
import hashlib
import json
import math
import os
import re
import shutil
import signal
import statistics
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / 'memreport'))
from memreport import WORKLOADS, bind_workload, corpus_provenance, host_state, parse_corpora  # noqa: E402
from soak import PRESSURE_CRITICAL, alive, footprints, read_pressure  # noqa: E402

PROTOCOL = '2025-11-25'
# Settings a client would otherwise inherit from this shell; each run starts from the defaults.
CLEARED_ENV = ('PRISM_MCP_STRUCTURED_CONTENT', 'PRISM_MCP_CONCISE_SHAPE', 'PRISM_MCP_MAX_RESULT_CHARS',
               'PRISM_MCP_SHARED', 'PRISM_MCP_RUNTIME_DIR', 'PRISM_MCP_MEM_REPORT')
WARMUP_RETRY_SECS = 1.0
# How often the run reads the host's memory pressure level (review round 1, sol 4).
PRESSURE_INTERVAL = 1.0
REACHED = re.compile(r'prism-mcp: shared daemon pid (\d+) \(started (\d+),')
# The pinned rule (owner question 7 of the S6 plan): the candidate's median p95 and p99 over its
# runs may exceed the baseline's by at most these factors.
P95_MAX_RATIO = 1.10
P99_MAX_RATIO = 1.25


class ClientGone(Exception):
    pass


def percentile(values, fraction):
    """Nearest rank: the smallest value with at least `fraction` of the values at or below it."""
    if not values:
        return None
    ordered = sorted(values)
    return ordered[max(1, math.ceil(fraction * len(ordered))) - 1]


def outcome(response):
    """ok; tool_error (the tool answered isError); retryable (warming, indexing, build_failed:
    `_meta.prism/retryable`); rpc_error (a JSON-RPC error)."""
    if 'error' in response:
        return 'rpc_error'
    result = response.get('result') or {}
    if (result.get('_meta') or {}).get('prism/retryable') is True:
        return 'retryable'
    return 'tool_error' if result.get('isError') else 'ok'


def rotated(items, client, clients):
    """`items` starting at this client's share of the way through, wrapping around."""
    start = (client * len(items)) // clients
    return items[start:] + items[:start]


class Client:
    """One prism-mcp process with piped standard input and output; one request at a time."""

    def __init__(self, index, argv, env, stderr_path):
        self.index = index
        self.stderr_path = stderr_path
        with open(stderr_path, 'wb') as stderr:
            self.process = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr,
                                            env=env)
        self.next_id = 1
        self.server = None

    def send(self, message):
        self.process.stdin.write((json.dumps(message, separators=(',', ':')) + '\n').encode())
        self.process.stdin.flush()

    def call(self, method, params=None):
        """(response, seconds from the write to the response)."""
        ident = self.next_id
        self.next_id += 1
        message = {'jsonrpc': '2.0', 'id': ident, 'method': method}
        if params is not None:
            message['params'] = params
        started = time.monotonic()
        self.send(message)
        while True:
            raw = self.process.stdout.readline()
            if not raw:
                raise ClientGone('client %d ended its output' % self.index)
            response = json.loads(raw)
            if 'id' not in response:
                continue
            if response['id'] != ident:
                raise ClientGone('client %d answered id %r to request %d' % (self.index, response['id'], ident))
            return response, time.monotonic() - started

    def stderr(self):
        return Path(self.stderr_path).read_text(errors='replace')


def client_argv(binary, repo, cache_dir, mode):
    # Resident-index measurement: a warmed client can wait for slower clients at the barrier.
    argv = [str(binary), '--repo', str(repo), '--cache-dir', str(cache_dir), '--first-call-wait', '600',
            '--index-idle-secs', '0']
    return argv + (['--shared', '--daemon-start-timeout', '120'] if mode == 'shared' else [])


def drive(client, workload, clients, refresh_at, deadline, barrier, clock, records):
    """Initialize, warm up, wait for every client, then run the timed workload."""
    hello, _ = client.call('initialize', {'protocolVersion': PROTOCOL, 'capabilities': {},
                                          'clientInfo': {'name': 'fanout', 'version': '1'}})
    client.server = (hello.get('result') or {}).get('serverInfo')
    client.send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
    while True:
        response, _ = client.call('tools/call', workload[0]['params'])
        if outcome(response) != 'retryable':
            break
        if time.monotonic() > deadline:
            raise ClientGone('client %d: the index was not ready before the timeout' % client.index)
        time.sleep(WARMUP_RETRY_SECS)
    # Every other tool of the workload once, so that no tool's lazily built state is built inside
    # the timed window by each standalone server (review round 1, Opus S6).
    seen = {workload[0]['params'].get('name')}
    for request in workload:
        if request['params'].get('name') not in seen:
            seen.add(request['params'].get('name'))
            client.call('tools/call', request['params'])
    barrier.wait()
    barrier.wait()
    refresh_seq = int(len(workload) * refresh_at) if client.index == 0 and refresh_at is not None else None
    for seq, request in enumerate(rotated(workload, client.index, clients)):
        if seq == refresh_seq:
            start = time.monotonic() - clock[0]
            response, seconds = client.call('tools/call', {'name': 'refresh_index', 'arguments': {}})
            records.append({'client': client.index, 'seq': seq, 'tool': 'refresh_index', 'refresh': True,
                            'start': round(start, 6), 'ms': round(seconds * 1000, 3), 'outcome': outcome(response)})
        start = time.monotonic() - clock[0]
        response, seconds = client.call('tools/call', request['params'])
        records.append({'client': client.index, 'seq': seq, 'tool': request['params'].get('name'),
                        'refresh': False, 'start': round(start, 6), 'ms': round(seconds * 1000, 3),
                        'outcome': outcome(response)})


def memory(pids):
    sizes = footprints(sorted(pids))
    return {'pids': sorted(pids), 'footprint_bytes': sizes, 'total_bytes': sum(sizes.values())}


class PressureWatch:
    """Reads the memory pressure level at start, every PRESSURE_INTERVAL seconds until stop, and
    at stop, keeping every reading; calls `on_critical` once at the first critical reading."""

    def __init__(self, on_critical):
        self.readings, self.fired, self.on_critical = [], False, on_critical
        self.stopped = threading.Event()
        self.thread = threading.Thread(target=self.loop, daemon=True)

    def read(self):
        level = read_pressure()
        self.readings.append([round(time.time(), 3), level])
        if level is not None and level >= PRESSURE_CRITICAL and not self.fired:
            self.fired = True
            self.on_critical()

    def loop(self):
        while not self.stopped.wait(PRESSURE_INTERVAL):
            self.read()

    def start(self):
        self.read()
        self.thread.start()

    def stop(self):
        self.stopped.set()
        self.thread.join()
        self.read()

    def report(self, required):
        levels = [level for _, level in self.readings]
        known = [level for level in levels if level is not None]
        problems = []
        if any(level >= PRESSURE_CRITICAL for level in known):
            problems.append('critical memory pressure (level %d) during the run; its clients were stopped' % max(known))
        if required and len(known) != len(levels):
            problems.append('the memory pressure level could not be read %d of %d times'
                            % (len(levels) - len(known), len(levels)))
        return {'required': required, 'readings': self.readings, 'max': max(known) if known else None,
                'unavailable': len(levels) - len(known), 'problems': problems}


def refresh_overlap(records):
    """The timed calls of other clients whose time overlaps the refresh's: how many, and the
    longest (review round 1, Opus W1: a refresh's stall reaches at most a few calls, which no
    percentile of the whole run shows)."""
    refresh = [r for r in records if r['refresh']]
    if not refresh:
        return {'calls': 0, 'max_ms': None, 'refresh_ms': None}
    first = refresh[0]
    begin, end = first['start'], first['start'] + first['ms'] / 1000.0
    over = [r['ms'] for r in records if not r['refresh'] and r['client'] != first['client']
            and r['start'] < end and r['start'] + r['ms'] / 1000.0 > begin]
    return {'calls': len(over), 'max_ms': max(over) if over else None, 'refresh_ms': first['ms']}


def summarize(records, run):
    timed = [r for r in records if not r['refresh']]
    counts = {}
    for record in timed:
        counts[record['outcome']] = counts.get(record['outcome'], 0) + 1
    ms = [r['ms'] for r in timed]
    refresh = [r for r in records if r['refresh']]
    problems = list(run.get('failures', []))
    if counts.get('retryable') or counts.get('rpc_error'):
        problems.append('timed calls were answered retryable or with a JSON-RPC error: %s' % counts)
    if len(timed) != run['clients'] * run['workload']['requests']:
        problems.append('%d of %d timed calls were made' % (len(timed), run['clients'] * run['workload']['requests']))
    if run['mode'] == 'shared' and len(run.get('daemon_instances', [])) != 1:
        problems.append('the shims reached %d daemon instances, not 1' % len(run.get('daemon_instances', [])))
    profile = 'gate' if run.get('refresh_at') is not None else 'diagnostic'
    if profile == 'gate' and [r['outcome'] for r in refresh] != ['ok']:
        problems.append('a gating run needs exactly one successful refresh_index; it had %s'
                        % ([r['outcome'] for r in refresh] or 'none'))
    problems += (run.get('pressure') or {}).get('problems', [])
    return {'calls': len(timed), 'outcomes': counts, 'profile': profile, 'refresh_overlap': refresh_overlap(records), 'p50_ms': percentile(ms, 0.50), 'p95_ms': percentile(ms, 0.95),
            'p99_ms': percentile(ms, 0.99), 'max_ms': max(ms) if ms else None,
            'mean_ms': round(statistics.mean(ms), 3) if ms else None,
            'refresh_ms': refresh[0]['ms'] if refresh else None,
            'refresh_outcome': refresh[0]['outcome'] if refresh else None,
            'admissible': not problems, 'problems': problems}


def stop_daemon(binary, repo, cache_dir, runtime_base, instances, timeout=60):
    """Ask the run's daemon to drain (it has no client left) and wait until its process is gone."""
    try:
        done = subprocess.run([str(binary), 'daemon-shutdown', '--repo', str(repo), '--cache-dir', str(cache_dir),
                               '--runtime-dir', str(runtime_base), '--timeout', str(timeout)],
                              stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout + 30)
        code, output = done.returncode, done.stdout.decode('utf-8', 'replace')[-2000:]
    except (OSError, subprocess.TimeoutExpired) as error:
        code, output = None, str(error)
    left = []
    for pid, _ in instances:
        give_up = time.monotonic() + timeout
        while time.monotonic() < give_up and alive(pid):
            time.sleep(0.1)
        if alive(pid):
            os.kill(pid, signal.SIGTERM)
            left.append(pid)
    return {'exit': code, 'output': output, 'terminated': left}


def cmd_run(args):
    out = Path(args.out)
    if any((out / name).exists() for name in ('run.json', 'calls.jsonl', 'summary.json')):
        raise SystemExit('run directory already has results; use a new --out')
    corpora = parse_corpora([args.corpus])
    (name, repo), = corpora.items()
    provenance = corpus_provenance(repo)
    data, digest = bind_workload(name, args.workloads, provenance['commit'])
    workload = [json.loads(line) for line in data.decode().splitlines() if line.strip()]
    refresh_at = None if args.no_refresh else args.refresh_at
    (out / 'clients').mkdir(parents=True, exist_ok=True)
    runtime_base = Path(tempfile.mkdtemp(prefix='fanout', dir='/tmp')) if args.mode == 'shared' else None
    env = {key: value for key, value in os.environ.items() if key not in CLEARED_ENV}
    if runtime_base:
        env['PRISM_MCP_RUNTIME_DIR'] = str(runtime_base)
    run = {'mode': args.mode, 'clients': args.clients, 'corpus': dict(provenance, name=name),
           'workload': digest, 'refresh_at': refresh_at, 'bin': str(args.bin),
           'bin_sha256': hashlib.sha256(Path(args.bin).read_bytes()).hexdigest(),
           'argv': client_argv(args.bin, repo, args.cache_dir, args.mode), 'host_before': host_state(),
           'started_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), 'failures': [],
           'profile': 'gate' if refresh_at is not None else 'diagnostic'}
    clients = [Client(i, run['argv'], env, out / 'clients' / ('%d.stderr' % i)) for i in range(args.clients)]
    watch = PressureWatch(lambda: [c.process.kill() for c in clients])
    watch.start()
    deadline = time.monotonic() + args.timeout
    watchdog = threading.Timer(args.timeout, lambda: [c.process.kill() for c in clients])
    watchdog.start()
    barrier = threading.Barrier(args.clients + 1)
    clock = [0.0]
    records = []

    def body(client):
        try:
            drive(client, workload, args.clients, refresh_at, deadline, barrier, clock, records)
        except (ClientGone, OSError, ValueError, threading.BrokenBarrierError) as error:
            run['failures'].append('%s: %s' % (type(error).__name__, error))
            barrier.abort()

    threads = [threading.Thread(target=body, args=(client,)) for client in clients]
    for thread in threads:
        thread.start()
    try:
        # Bounded, so that no failure a client did not report can hold the run past its timeout.
        barrier.wait(timeout=max(1.0, deadline - time.monotonic()))
        instances = sorted({(int(m.group(1)), int(m.group(2))) for c in clients for m in REACHED.finditer(c.stderr())})
        run['memory_after_warmup'] = memory([c.process.pid for c in clients] + [pid for pid, _ in instances])
        clock[0] = time.monotonic()
        barrier.wait(timeout=max(1.0, deadline - time.monotonic()))
    except threading.BrokenBarrierError:
        pass
    for thread in threads:
        thread.join()
    watch.stop()
    run['pressure'] = watch.report(args.pressure == 'required')
    instances = sorted({(int(m.group(1)), int(m.group(2))) for c in clients for m in REACHED.finditer(c.stderr())})
    run['daemon_instances'] = instances
    run['memory_after_workload'] = memory([c.process.pid for c in clients] + [pid for pid, _ in instances])
    run['server'] = next((c.server for c in clients if c.server), None)
    for client in clients:
        try:
            client.process.stdin.close()
        except OSError:
            pass
    run['exits'] = []
    for client in clients:
        try:
            run['exits'].append(client.process.wait(timeout=60))
        except subprocess.TimeoutExpired:
            client.process.kill()
            client.process.wait()
            run['exits'].append('killed')
        client.process.stdout.close()
    watchdog.cancel()
    if runtime_base:
        run['shutdown'] = stop_daemon(args.bin, repo, args.cache_dir, runtime_base, instances)
        # The daemon's own log is the evidence for an inadmissible shared run (review round 1, Opus S6).
        logs = sorted((runtime_base / 'prism-mcpd').glob('*.log'))
        if logs:
            (out / 'daemon-logs').mkdir(exist_ok=True)
            for log in logs:
                shutil.copyfile(log, out / 'daemon-logs' / log.name)
        shutil.rmtree(runtime_base, ignore_errors=True)
    run['host_after'] = host_state()
    run['finished_utc'] = time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())
    records.sort(key=lambda r: (r['client'], r['seq'], not r['refresh']))
    with open(out / 'calls.jsonl', 'w') as calls:
        for record in records:
            calls.write(json.dumps(record, sort_keys=True) + '\n')
    summary = summarize(records, run)
    (out / 'run.json').write_text(json.dumps(run, indent=1, sort_keys=True, default=str))
    (out / 'summary.json').write_text(json.dumps(summary, indent=1, sort_keys=True))
    print('%s %s x%d: p50 %s ms, p95 %s ms, p99 %s ms, %s' % (
        name, args.mode, args.clients, summary['p50_ms'], summary['p95_ms'], summary['p99_ms'],
        'admissible' if summary['admissible'] else 'NOT admissible: %s' % '; '.join(summary['problems'])))


def load_arm(directories):
    arm = []
    for directory in directories:
        run = json.loads((Path(directory) / 'run.json').read_text())
        summary = json.loads((Path(directory) / 'summary.json').read_text())
        arm.append({'dir': str(directory), 'run': run, 'summary': summary})
    return arm


def judge(baseline, candidate, allow_client_mismatch=False):
    """The pinned rule's verdict and the figures it rests on. Never raises on a run without
    percentiles (review round 1, Opus W8): it fails the comparison."""
    entries = baseline + candidate
    problems = ['%s is not admissible: %s' % (entry['dir'], '; '.join(entry['summary']['problems']))
                for entry in entries if not entry['summary']['admissible']]
    problems += ['%s is a diagnostic run (no refresh_index); only gating runs are compared' % entry['dir']
                 for entry in entries if entry['run'].get('refresh_at') is None]
    keys = {(e['run']['corpus'].get('commit'), e['run']['workload']['sha256']) for e in entries}
    if len(keys) != 1:
        problems.append('the runs are not of one corpus commit and workload: %s' % sorted(keys))
    if len({e['run'].get('refresh_at') for e in entries}) > 1:
        problems.append('the runs refresh at different points: %s' % sorted({str(e['run'].get('refresh_at'))
                                                                             for e in entries}))
    if not allow_client_mismatch and len({e['run']['clients'] for e in entries}) > 1:
        problems.append('the runs have different numbers of clients: %s' % sorted({e['run']['clients'] for e in entries}))
    problems += ['%s ran on a corpus with uncommitted changes or unknown cleanliness' % e['dir']
                 for e in entries if e['run']['corpus'].get('dirty') != 0]
    figures = {}
    for name, arm in (('baseline', baseline), ('candidate', candidate)):
        figures[name] = {}
        for stat in ('p50_ms', 'p95_ms', 'p99_ms'):
            values = [e['summary'].get(stat) for e in arm]
            known = all(isinstance(value, (int, float)) for value in values) and values
            figures[name][stat] = statistics.median(values) if known else None
    if None in figures['baseline'].values() or None in figures['candidate'].values():
        problems.append('a run has no percentiles (no timed call was made)')
        p95 = p99 = None
    else:
        p95 = figures['candidate']['p95_ms'] / figures['baseline']['p95_ms']
        p99 = figures['candidate']['p99_ms'] / figures['baseline']['p99_ms']
    passed = not problems and p95 <= P95_MAX_RATIO and p99 <= P99_MAX_RATIO
    return {'pass': passed, 'problems': problems, 'figures': figures, 'p95_ratio': p95, 'p99_ratio': p99,
            'rule': {'p95_max_ratio': P95_MAX_RATIO, 'p99_max_ratio': P99_MAX_RATIO}}


def calibrate(first, second):
    """An A/A calibration: two arms of one configuration pass only when each passes against the
    other, so that their spread is within the bound whichever way it falls (review round 2, Opus
    W-A: a one-way comparison passed a 30 % spread)."""
    forward, backward = judge(first, second), judge(second, first)
    return {'pass': forward['pass'] and backward['pass'], 'forward': forward, 'backward': backward}


def render(baseline, candidate, verdict):
    lines = ['# prism-mcp fan-out latency', '', '| Arm | Run | Mode | Clients | p50 ms | p95 ms | p99 ms | max ms | '
             'refresh ms | calls overlapping the refresh | longest of them, ms | admissible |',
             '|---|---|---|---|---|---|---|---|---|---|---|---|']
    for name, arm in (('baseline', baseline), ('candidate', candidate)):
        for entry in arm:
            s, r = entry['summary'], entry['run']
            overlap = s.get('refresh_overlap') or {}
            lines.append('| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |' % (
                name, entry['dir'], r['mode'], r['clients'], s['p50_ms'], s['p95_ms'], s['p99_ms'], s.get('max_ms'),
                s.get('refresh_ms'), overlap.get('calls'), overlap.get('max_ms'), 'yes' if s['admissible'] else 'no'))
    lines += ['', 'Medians across runs: baseline %s; candidate %s.' % (verdict['figures']['baseline'],
                                                                      verdict['figures']['candidate']),
              'Ratios: p95 %s (at most %.2f), p99 %s (at most %.2f). The refresh overlap is reported, not gated.'
              % (None if verdict['p95_ratio'] is None else round(verdict['p95_ratio'], 4), P95_MAX_RATIO,
                 None if verdict['p99_ratio'] is None else round(verdict['p99_ratio'], 4), P99_MAX_RATIO),
              '', '**Verdict: %s**' % ('PASS' if verdict['pass'] else 'FAIL')]
    lines += ['- %s' % problem for problem in verdict['problems']]
    return '\n'.join(lines) + '\n'


def cmd_compare(args):
    baseline, candidate = load_arm(args.baseline), load_arm(args.candidate)
    if args.both_directions:
        verdict = calibrate(baseline, candidate)
        Path(args.out).write_text('# A/A calibration: both directions must pass\n\n**Verdict: %s**\n\n## Forward\n\n%s\n'
                                  '## Backward\n\n%s' % ('PASS' if verdict['pass'] else 'FAIL',
                                                           render(baseline, candidate, verdict['forward']),
                                                           render(candidate, baseline, verdict['backward'])))
        if args.json:
            Path(args.json).write_text(json.dumps(verdict, indent=1, sort_keys=True))
        print('wrote %s: %s' % (args.out, 'PASS' if verdict['pass'] else 'FAIL'))
        return
    verdict = judge(baseline, candidate, args.allow_client_mismatch)
    Path(args.out).write_text(render(baseline, candidate, verdict))
    if args.json:
        Path(args.json).write_text(json.dumps(verdict, indent=1, sort_keys=True))
    print('wrote %s: %s' % (args.out, 'PASS' if verdict['pass'] else 'FAIL'))


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest='command', required=True)
    run = commands.add_parser('run')
    run.add_argument('--bin', required=True, type=Path)
    run.add_argument('--corpus', required=True, metavar='NAME=PATH')
    run.add_argument('--cache-dir', required=True, type=Path)
    run.add_argument('--mode', required=True, choices=('shared', 'standalone'))
    run.add_argument('--out', required=True)
    run.add_argument('--clients', type=int, default=8)
    run.add_argument('--workloads', type=Path, default=WORKLOADS)
    run.add_argument('--refresh-at', type=float, default=0.5)
    run.add_argument('--no-refresh', action='store_true')
    run.add_argument('--timeout', type=float, default=1800)
    run.add_argument('--pressure', choices=('required', 'off'), default='required' if sys.platform == 'darwin' else 'off',
                     help='whether an unreadable memory pressure level makes the run inadmissible (a critical '
                          'reading always stops it); default: required on macOS')
    run.set_defaults(handler=cmd_run)
    compare = commands.add_parser('compare')
    compare.add_argument('--baseline', action='append', required=True)
    compare.add_argument('--candidate', action='append', required=True)
    compare.add_argument('--out', required=True)
    compare.add_argument('--json')
    compare.add_argument('--both-directions', action='store_true',
                         help='an A/A calibration: pass only when each arm passes against the other')
    compare.add_argument('--allow-client-mismatch', action='store_true',
                         help='compare arms of different client counts (information only, never a gate)')
    compare.set_defaults(handler=cmd_compare)
    args = parser.parse_args(argv)
    if args.command == 'run' and not (args.clients >= 1 and 0.0 <= args.refresh_at < 1.0):
        parser.error('--clients must be at least 1 and --refresh-at in [0, 1)')
    args.handler(args)


if __name__ == '__main__':
    main()
