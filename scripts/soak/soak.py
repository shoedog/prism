#!/usr/bin/env python3
"""Soak instrumentation for the shared prism-mcp daemon (spec section 8 step 2).

    soak.py sample --out FILE.jsonl [--interval 60] [--count N | --duration SECS [--window-file T0]]
                   [--status-bin BIN --runtime-dir BASE]
    soak.py report --phase fanout|round|week --samples FILE.jsonl --out FILE.md [--json FILE.json]
                   [--since EPOCH | --window-file T0] [--until EPOCH] [--claude-logs DIR]
                   [--client-log FILE ...] [--crash-reports DIR] [--round-repo DIR ...]
                   [--min-shims 2] [--min-days 2] [--fix-at RULE=EPOCH ...]
    soak.py probe [--timeout 30] -- COMMAND [ARGS...]

`sample` appends one JSON line per interval: every prism-mcp process (its role, repository, key,
resident size, CPU time, and footprint with whether it was measured, had gone, or could not be measured),
whether the process table could be read, and the host's swap, memory pressure and compressor. It
reads the process table and never connects to a daemon unless --status-bin is given, in which case
it also stores the text of `daemon-status` (a daemon built before S5b restarts its idle time at
each such probe: SI-35). Appending makes it resumable: run it again on the same file. With
--window-file the window starts once, when the file is first written. A duration run includes
the first sample at or beyond --duration seconds after the first observation. A resumed sampler
uses that original observation within the window, rather than starting a new duration.

`report` (soak_judge.py) reads the samples, the clients' standard error (Claude Code's MCP logs,
the files prism-mcp-logged.sh writes) and macOS crash reports, and judges the soak's rules; a rule
whose evidence was not collected is UNKNOWN, and a phase that saw no use is INADMISSIBLE.
The `fanout` phase is a diagnostic report only; its rules and use do not gate latency acceptance.
Sampling also reads newly appended rebuild-log lines passively, when the runtime directory is visible.

`probe` starts a launcher's exact command line, sends `initialize`, and passes only when an answer
comes back: the check, at deployment and at rollback, that a configuration starts a server with
the binary it names (a binary that rejects --shared fails it, with its error shown).

Python 3.9, standard library only. Unit tests:
    python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import argparse
import json
import os
import queue
import re
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

from soak_growth import RebuildLogs
from soak_window import PHASE3_MIN_USAGE_DAYS, fix_at

from soak_judge import (BUILD_CRASH, CLIENT_LINE, CRASH_REPORT, FAIL_CLOSED, GATED, HOLDER_MIN_BYTES,  # noqa: F401
                        LOST, MIB, PRESSURE_CRITICAL, REACHED, RULES, STARTUP_ERROR, built, claude_log_lines,
                        client_log_lines, cmd_report, crash_reports, holder_identity, in_window, is_holder,
                        iso_epoch, judge, load_samples, logs_readable, overall, render, scan_lines,
                        summarize_samples, usage, usage_days)

PS_COMMAND = ['ps', '-A', '-ww', '-o', 'pid=,ppid=,rss=,time=,command=']
FOOTPRINT_COMMAND = 'footprint'
PROC = Path('/proc')
# A prism-mcp binary's file name: `prism-mcp`, or `prism-mcp-<12 hex digits>` as Phase 0 installs a
# build (PR #424 review round 1, sol 2); not prism-mcp-logged.sh, the launcher, or a test binary.
BINARY_NAME = re.compile(r'^prism-mcp(?:-[0-9a-f]{12})?$')
BINARY = re.compile(r'^(.*?(?:^|/)prism-mcp(?:-[0-9a-f]{12})?)(?: (.*))?$')


def capture(command, timeout=30):
    """The stdout of a command as text, or None when it could not run or exited non-zero."""
    try:
        done = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return done.stdout.decode('utf-8', 'replace') if done.returncode == 0 else None


def option(text, name):
    """The value of option `name` in the argument text of a command line: `name value` or
    `name=value`, running to the next ` --` so that a path with spaces in it is kept whole (PR #424
    review round 1, sol 1); None when absent or empty. A value containing ` --` is cut there."""
    found = re.search(r'(?:^|\s)%s(?:=(.*?)|\s+(?!--)(.*?))(?=\s+--|\s*$)' % re.escape(name), text)
    value = found and (found.group(1) if found.group(1) is not None else found.group(2))
    return value or None


def split_command(command):
    """(binary, the text of the other arguments) when the command runs a prism-mcp binary
    (BINARY_NAME), else None. The binary is the first word, or, when its path has a space in it
    (review round 1, Opus S5), the shortest prefix ending in such a name that is an executable file."""
    words = command.split(None, 1)
    if words and BINARY_NAME.match(os.path.basename(words[0])):
        return words[0], words[1] if len(words) > 1 else ''
    found = BINARY.match(command)
    if found and ' ' in found.group(1) and os.path.isfile(found.group(1)) and os.access(found.group(1), os.X_OK):
        return found.group(1), found.group(2) or ''
    return None


def classify(command):
    """Role and configuration of one process from its command line, or None for a process that is
    not prism-mcp. Roles: daemon, shim, standalone, operator. `repo` is as written; the sampler
    resolves a relative one (resolve_repo)."""
    split = split_command(command)
    if split is None:
        return None
    text = split[1]
    tokens = text.split()
    verb = tokens[0] if tokens else ''
    if verb == 'daemon':
        role = 'daemon'
    elif verb in ('daemon-status', 'daemon-shutdown'):
        role = 'operator'
    elif '--shared' in tokens:
        role = 'shim'
    else:
        role = 'standalone'
    return {'role': role, 'repo': option(text, '--repo'), 'key': option(text, '--key'),
            'cache_dir': option(text, '--cache-dir')}


def parse_cputime(text):
    """Seconds of `ps -o time=`: `[DD-]HH:MM:SS` (procps) or `MM:SS.ss` / `HH:MM:SS.ss` (macOS);
    None when it is neither."""
    days, _, clock = text.rpartition('-')
    try:
        parts = [float(part) for part in clock.split(':')]
        whole = int(days) if days else 0
    except ValueError:
        return None
    if not 2 <= len(parts) <= 3:
        return None
    seconds = 0.0
    for part in parts:
        seconds = seconds * 60 + part
    return whole * 86400 + seconds


def parse_ps(text):
    """prism-mcp processes in `ps -A -ww -o pid=,ppid=,rss=,time=,command=` output."""
    processes = []
    for line in text.splitlines():
        fields = line.split(None, 4)
        if len(fields) < 5 or not fields[0].isdigit():
            continue
        found = classify(fields[4])
        if found is not None:
            found.update(pid=int(fields[0]), ppid=int(fields[1]), rss_bytes=int(fields[2]) * 1024,
                         cpu_secs=parse_cputime(fields[3]), command=fields[4])
            processes.append(found)
    return processes


def alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def measure_footprints(pids):
    """({pid: bytes}, {pid: 'ok' | 'gone' | 'failed'}): phys_footprint (macOS `footprint`), or
    resident plus swapped (Linux /proc). A pid without a reading is 'gone' when the process has
    ended and 'failed' while it still runs (review round 1, sol 2)."""
    sizes = {}
    if pids and sys.platform == 'darwin':
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / 'footprint.json'
            capture([FOOTPRINT_COMMAND, '-f', 'bytes', '--noCategories', '-j', str(report)] + [str(p) for p in pids])
            try:
                data = json.loads(report.read_text())
            except (OSError, ValueError):
                data = {}
        sizes = {entry['pid']: entry['auxiliary']['phys_footprint'] for entry in data.get('processes', [])
                 if entry.get('pid') in pids and 'phys_footprint' in entry.get('auxiliary', {})}
    elif pids:
        for pid in pids:
            try:
                status = (PROC / str(pid) / 'status').read_text()
            except OSError:
                continue
            kib = [int(line.split()[1]) for line in status.splitlines() if line.startswith(('VmRSS:', 'VmSwap:'))]
            sizes[pid] = sum(kib) * 1024
    statuses = {pid: 'ok' if pid in sizes else 'failed' if alive(pid) else 'gone' for pid in pids}
    return sizes, statuses


def footprints(pids):
    """phys_footprint in bytes by pid, for the pids that could be measured."""
    return measure_footprints(pids)[0]


def process_cwd(pid):
    """The working directory of a live process (Linux /proc, macOS lsof), or None."""
    if sys.platform.startswith('linux'):
        try:
            return os.readlink(str(PROC / str(pid) / 'cwd'))
        except OSError:
            return None
    listing = capture(['lsof', '-a', '-p', str(pid), '-d', 'cwd', '-Fn']) if sys.platform == 'darwin' else None
    names = [line[1:] for line in (listing or '').splitlines() if line.startswith('n')]
    return names[0] if names else None


def resolve_repo(process, cwd_of=process_cwd):
    """Keep `--repo` as written in `repo_arg` and make `repo` absolute: a relative one is joined to
    the process's working directory (PR #424 review round 1, Opus S3), or None (unresolved) when
    that cannot be read."""
    arg = process.get('repo')
    process['repo_arg'] = arg
    if arg and not os.path.isabs(arg):
        cwd = cwd_of(process['pid'])
        process['repo'] = os.path.join(cwd, arg) if cwd else None


def observe(listing, measure=measure_footprints, cwd_of=process_cwd):
    """The prism-mcp processes of one process table: classified, repository resolved, footprint
    measured, index holders marked."""
    processes = parse_ps(listing or '')
    for process in processes:
        resolve_repo(process, cwd_of)
    sizes, statuses = measure([process['pid'] for process in processes])
    for process in processes:
        process['footprint_bytes'] = sizes.get(process['pid'])
        process['footprint_status'] = statuses[process['pid']]
        process['holder'] = is_holder(process)
    return processes


def parse_swapusage(text):
    """MiB figures of `sysctl -n vm.swapusage`: total, used, free."""
    found = dict(re.findall(r'(total|used|free) = ([0-9.]+)M', text or ''))
    return {name: float(value) for name, value in found.items()}


def parse_compressed(vm_stat):
    """Bytes held by the compressor, from `vm_stat`; None when it does not say."""
    size = re.search(r'page size of (\d+) bytes', vm_stat or '')
    pages = re.search(r'Pages occupied by compressor:\s+(\d+)', vm_stat or '')
    return int(size.group(1)) * int(pages.group(1)) if size and pages else None


def read_pressure():
    """kern.memorystatus_vm_pressure_level (1 normal, 2 warning, 4 critical), or None when it
    cannot be read (not macOS, or the sysctl failed)."""
    if sys.platform != 'darwin':
        return None
    level = capture(['sysctl', '-n', 'kern.memorystatus_vm_pressure_level'])
    return int(level) if level and level.strip().isdigit() else None


def host_memory():
    """Swap, pressure and compressor; a figure that could not be read is None (or {} for swap)."""
    if sys.platform == 'darwin':
        return {'swap_mib': parse_swapusage(capture(['sysctl', '-n', 'vm.swapusage'])),
                'pressure': read_pressure(), 'compressed_bytes': parse_compressed(capture(['vm_stat']))}
    try:
        info = dict(line.split(':', 1) for line in Path('/proc/meminfo').read_text().splitlines() if ':' in line)
    except OSError:
        info = {}
    kib = {name: int(info[name].split()[0]) for name in ('SwapTotal', 'SwapFree') if name in info}
    swap = ({'total': kib['SwapTotal'] / 1024, 'free': kib['SwapFree'] / 1024,
             'used': (kib['SwapTotal'] - kib['SwapFree']) / 1024} if len(kib) == 2 else {})
    return {'swap_mib': swap, 'pressure': None, 'compressed_bytes': None}


def sample_once(status_bin=None, runtime_dir=None):
    listing = capture(PS_COMMAND)
    processes = observe(listing)
    sample = {'t': time.time(), 'ps_ok': listing is not None, 'host': host_memory(), 'processes': processes}
    if status_bin:
        command = [status_bin, 'daemon-status'] + (['--runtime-dir', runtime_dir] if runtime_dir else [])
        sample['daemon_status'] = capture(command, timeout=120)
    return sample


def open_window(path, duration, now=None):
    """(start, end, resumed) of a sampling window kept in `path`: the start is written once, the
    first time, and every later call keeps it, so a resumed sampler and the report agree on the
    window whatever restarts came between (review round 2, sol 1, Opus W-E)."""
    now = time.time() if now is None else now
    path = Path(path)
    text = path.read_text().strip() if path.exists() else ''
    resumed = text.isdigit()
    if not resumed:
        path.write_text('%d\n' % int(now))
    start = int(text) if resumed else int(now)
    return start, (start + duration if duration else None), resumed


def cmd_sample(args):
    rebuild_logs = RebuildLogs(args.runtime_dir)
    first = end = None
    if args.window_file:
        start, end, resumed = open_window(args.window_file, args.duration)
        if args.duration and Path(args.out).exists():
            previous, _ = load_samples(args.out, since=start)
            first = min((s['t'] for s in previous), default=None)
            if first is not None:
                end = first + args.duration
        print('sampler %s at %d: window from %d, %s s left' % (
            'resumed' if resumed else 'started', time.time(), start,
            'unbounded' if end is None else '%d' % max(0, end - time.time())), flush=True)
    taken = 0
    while True:
        sample = sample_once(args.status_bin, args.runtime_dir)
        if first is None:
            first = sample['t']
            end = first + args.duration if args.duration else None
        rebuild_logs.observe(sample['processes'])
        with open(args.out, 'a') as out:
            out.write(json.dumps(sample, sort_keys=True) + '\n')
        taken += 1
        if (args.count and taken >= args.count) or (end is not None and sample['t'] >= end):
            break
        time.sleep(args.interval)
    print('%d samples appended to %s' % (taken, args.out))
    return 0


def cmd_probe(args):
    """Start the command, send `initialize`, wait for an answer; PASS (0) or FAIL (1) with why."""
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    with tempfile.TemporaryFile() as stderr:
        try:
            child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr)
        except OSError as error:
            print('FAIL: cannot start %s: %s' % (command[0], error))
            return 1
        answers = queue.Queue()
        threading.Thread(target=lambda: answers.put(child.stdout.readline()), daemon=True).start()
        hello = {'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {
            'protocolVersion': '2025-11-25', 'capabilities': {}, 'clientInfo': {'name': 'probe', 'version': '1'}}}
        try:
            child.stdin.write((json.dumps(hello) + '\n').encode())
            child.stdin.flush()
        except OSError:
            pass
        try:
            line = answers.get(timeout=args.timeout)
        except queue.Empty:
            line = None
        if line is None:
            child.kill()
        try:
            child.stdin.close()
        except OSError:
            pass
        try:
            status = child.wait(timeout=10)
        except subprocess.TimeoutExpired:
            child.kill()
            status = child.wait()
        child.stdout.close()
        stderr.seek(0)
        err = stderr.read().decode('utf-8', 'replace').strip()
    try:
        answered = line is not None and 'result' in json.loads(line)
    except ValueError:
        answered = False
    if answered:
        print('PASS: %s answered initialize' % command[0])
        return 0
    why = 'no answer to initialize within %ss' % args.timeout if line is None else 'it ended (exit %s)' % status
    print('FAIL: %s: %s; standard error: %s' % (command[0], why, err[-2000:] or '(empty)'))
    return 1


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest='command', required=True)
    sample = commands.add_parser('sample')
    sample.add_argument('--out', required=True)
    sample.add_argument('--interval', type=float, default=60.0)
    sample.add_argument('--count', type=int)
    sample.add_argument('--duration', type=float, help='first-to-last observed seconds; includes a terminal sample; '
                        'with neither this nor --count, forever')
    sample.add_argument('--window-file', help='keeps the window start and original observation across resumes')
    sample.add_argument('--status-bin', help='a prism-mcp binary whose daemon-status text each sample stores')
    sample.add_argument('--runtime-dir', help='runtime base for passive rebuild logs and optional daemon-status')
    sample.set_defaults(handler=cmd_sample)
    report = commands.add_parser('report')
    report.add_argument('--phase', required=True, choices=sorted(GATED))
    report.add_argument('--samples', required=True)
    report.add_argument('--out', required=True)
    report.add_argument('--json')
    report.add_argument('--since', type=float)
    report.add_argument('--window-file', help='the start of the window, as sample --window-file wrote it')
    report.add_argument('--until', type=float)
    report.add_argument('--claude-logs', help='Claude Code MCP log root (~/Library/Caches/claude-cli-nodejs)')
    report.add_argument('--client-log', action='append', help='a file prism-mcp-logged.sh wrote')
    report.add_argument('--crash-reports', default=str(Path.home() / 'Library/Logs/DiagnosticReports'))
    report.add_argument('--round-repo', action='append',
                        help='round: a checkout the round ran in; standalone servers elsewhere are not gated')
    report.add_argument('--min-shims', type=int, default=2, help='round: shims at once that show use')
    report.add_argument('--min-days', type=int, default=PHASE3_MIN_USAGE_DAYS,
                        help='week: usage days that show use (cannot lower the owner floor)')
    report.add_argument('--fix-at', type=fix_at, action='append', metavar='RULE=EPOCH',
                        help='judge only this rule strictly after its repair; repeat per rule (latest wins)')
    report.set_defaults(handler=cmd_report)
    probe = commands.add_parser('probe')
    probe.add_argument('--timeout', type=float, default=30.0)
    probe.add_argument('command', nargs=argparse.REMAINDER)
    probe.set_defaults(handler=cmd_probe)
    args = parser.parse_args(argv)
    if args.handler is cmd_probe and args.command in ([], ['--']):
        probe.error('probe needs the COMMAND to start, after --')
    if args.command == 'report':
        if args.phase == 'week' and args.min_days < PHASE3_MIN_USAGE_DAYS:
            parser.error('--min-days cannot lower the owner usage-day floor')
        if args.phase == 'round' and not args.round_repo:
            parser.error('report --phase round needs --round-repo for each checkout the round ran in')
        if args.window_file:
            if args.since is not None:
                parser.error('--since and --window-file are one or the other')
            args.since = float(Path(args.window_file).read_text().strip())
    return args.handler(args)


if __name__ == '__main__':
    sys.exit(main())
