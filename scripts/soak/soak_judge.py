"""The soak report's reading and judging (soak.py report; spec section 8 step 2).

A rule is PASS, FAIL or UNKNOWN. UNKNOWN means the evidence the rule needs was not collected (the
process table could not be read, a live process's footprint could not be measured, the pressure
level was unavailable, a named log source or the crash-report directory was missing): a rule never
passes on what it could not see (review round 1, sol 2). A phase also needs evidence of use; a
phase whose rules all pass but that saw no use is INADMISSIBLE (Opus W2, S3).
"""
import calendar
import json
import os
import re
import time
from pathlib import Path

MIB = 1024 * 1024
# A standalone prism-mcp that has not built an index is a few MiB; one that has holds hundreds.
HOLDER_MIN_BYTES = 100 * MIB
# kern.memorystatus_vm_pressure_level: 1 normal, 2 warning, 4 critical.
PRESSURE_CRITICAL = 4
FAIL_CLOSED = 'prism-mcp: --shared failed closed'
REACHED = re.compile(r'prism-mcp: shared daemon pid (\d+) \(started (\d+),')
LOST = re.compile(r'prism-mcp: (the shared daemon closed the connection|the shared daemon\'s connection '
                  r'failed)')
BUILD_CRASH = 'died during its index build'
# A server that never started: clap's refusal (an old binary given --shared), the launcher's own
# refusals (review round 1, Opus S2).
STARTUP_ERROR = re.compile(r'^(error: |prism-mcp-launch\.sh: |prism-mcp not found)')
CLIENT_LINE = re.compile(r'(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ) (\d+) (.*)')
CRASH_REPORT = re.compile(r'prism-mcp[-_].*\.(ips|crash)$')
RULES = ('one_index_per_key', 'no_new_standalone_index', 'no_fail_closed', 'no_startup_error', 'no_restart_bound',
         'no_abort', 'no_critical_pressure')
# The rules each phase gates on; the others are reported.
GATED = {'round': RULES,
         'week': ('one_index_per_key', 'no_fail_closed', 'no_startup_error', 'no_restart_bound', 'no_abort')}


def is_holder(process):
    """Whether a process holds an index: every daemon, and a standalone server whose measured
    footprint is past the size of one that has not built one. A shim never does."""
    if process['role'] == 'daemon':
        return True
    return process['role'] == 'standalone' and (process.get('footprint_bytes') or 0) >= HOLDER_MIN_BYTES


def built(process):
    """A daemon whose measured footprint shows an index was built."""
    return process['role'] == 'daemon' and (process.get('footprint_bytes') or 0) >= HOLDER_MIN_BYTES


def load_samples(path, since=None, until=None):
    """(samples in the window, lines that could not be read: a sampler killed mid-write)."""
    samples, skipped = [], 0
    with open(path) as source:
        for line in source:
            if not line.strip():
                continue
            try:
                sample = json.loads(line)
            except ValueError:
                skipped += 1
                continue
            if (since is None or sample['t'] >= since) and (until is None or sample['t'] <= until):
                samples.append(sample)
    return samples, skipped


def repo_label(process):
    """A process's repository as an absolute path, or `(unresolved) <as written>` when the sampler
    could not make it absolute (PR #424 review round 1, sol 1)."""
    repo = process.get('repo')
    if repo and os.path.isabs(repo):
        return repo
    return '(unresolved) %s' % (process.get('repo_arg') or repo or 'no --repo')


def holder_identity(process):
    """What one index is for: a daemon's key, or a standalone server's repository."""
    if process['role'] == 'daemon':
        return 'daemon ' + (process.get('key') or '?')
    return 'standalone ' + (process.get('repo') or '?')


# CPU seconds one daemon must spend between two consecutive samples for that interval's UTC day
# to show use (owner decision 2026-10-16, PR #424 review round 1, Opus W1; replaces 0.5 s summed
# over the day). An idle daemon with a session attached spends about 0.0006 s a minute on its 5 s
# identity tick (0.86 s a day, measured at nanosecond precision), about 80 times below this in a
# 60 s interval; a tool call on a warm index costs tens of milliseconds or more.
USAGE_INTERVAL_CPU_SECS = 0.05


def usage_days(samples):
    """The UTC days of the sample intervals in which one daemon's CPU time grew by at least
    USAGE_INTERVAL_CPU_SECS: growth from the previous sample, or, for a daemon never seen before
    and not in the first sample, its whole CPU time (it started in that interval). A pid missing
    from either end of an interval contributes nothing for it, and a daemon that is resident, or a
    shim that is attached, is not use. Growth is rounded to microseconds: `ps` prints hundredths,
    and 0.30 - 0.25 is 0.0499... in floating point."""
    days, seen = set(), set()
    previous = None
    for index, sample in enumerate(samples):
        now = {p['pid']: p.get('cpu_secs') for p in sample['processes'] if p['role'] == 'daemon'}
        for pid, cpu in now.items():
            if cpu is None or index == 0:
                continue
            if previous is not None and pid in previous:
                before = previous[pid]
            elif pid not in seen and samples[index - 1].get('ps_ok', True):
                before = 0.0
            else:
                before = None
            if before is not None and round(cpu - before, 6) >= USAGE_INTERVAL_CPU_SECS:
                days.add(time.strftime('%Y-%m-%d', time.gmtime(sample['t'])))
        seen.update(now)
        previous = {pid: cpu for pid, cpu in now.items() if cpu is not None}
    return sorted(days)


def summarize_samples(samples):
    worst_per_key, worst_per_repo, new_standalone, old_standalone, daemons = {}, {}, {}, {}, {}
    unmeasured_new = {}
    missing = {'ps': 0, 'pressure': 0, 'footprint': 0}
    first_pids = {p['pid'] for p in samples[0]['processes']} if samples else set()
    peaks = {'holders': 0, 'shims': 0, 'prism_footprint_bytes': 0, 'swap_used_mib': 0.0, 'pressure': 0,
             'compressed_bytes': 0, 'built_daemons': 0}
    for sample in samples:
        missing['ps'] += not sample.get('ps_ok', True)
        holders = {}
        for process in sample['processes']:
            if process.get('footprint_status') == 'failed':
                missing['footprint'] += 1
                if process['role'] == 'standalone' and process['pid'] not in first_pids:
                    unmeasured_new.setdefault(repo_label(process), set()).add(process['pid'])
            if process.get('holder'):
                holders.setdefault(holder_identity(process), []).append(process['pid'])
            if process['role'] == 'standalone' and process.get('holder'):
                found = old_standalone if process['pid'] in first_pids else new_standalone
                found.setdefault(repo_label(process), set()).add(process['pid'])
            if process['role'] == 'daemon':
                daemons.setdefault((process['pid'], process.get('key')), [sample['t'], sample['t']])[1] = sample['t']
        for identity, pids in holders.items():
            if identity.startswith('daemon ') and len(pids) > len(worst_per_key.get(identity, [])):
                worst_per_key[identity] = pids
        repos = {}
        for process in sample['processes']:
            if process['role'] == 'daemon':
                repos.setdefault(process.get('repo') or '?', []).append(process['pid'])
        for repo, pids in repos.items():
            if len(pids) > len(worst_per_repo.get(repo, [])):
                worst_per_repo[repo] = pids
        host = sample.get('host') or {}
        missing['pressure'] += host.get('pressure') is None
        peaks['holders'] = max(peaks['holders'], sum(len(p) for p in holders.values()))
        peaks['shims'] = max(peaks['shims'], sum(1 for p in sample['processes'] if p['role'] == 'shim'))
        peaks['built_daemons'] = max(peaks['built_daemons'], sum(1 for p in sample['processes'] if built(p)))
        peaks['prism_footprint_bytes'] = max(peaks['prism_footprint_bytes'], sum(
            p.get('footprint_bytes') or 0 for p in sample['processes']))
        peaks['swap_used_mib'] = max(peaks['swap_used_mib'], (host.get('swap_mib') or {}).get('used', 0.0))
        peaks['pressure'] = max(peaks['pressure'], host.get('pressure') or 0)
        peaks['compressed_bytes'] = max(peaks['compressed_bytes'], host.get('compressed_bytes') or 0)
    return {'samples': len(samples), 'first': samples[0]['t'] if samples else None,
            'last': samples[-1]['t'] if samples else None, 'peaks': peaks, 'missing': missing,
            'max_daemons_per_key': max([len(p) for p in worst_per_key.values()] or [0]),
            'duplicated_keys': {k: v for k, v in worst_per_key.items() if len(v) > 1},
            'max_daemons_per_repo': max([len(p) for p in worst_per_repo.values()] or [0]),
            'usage_days': usage_days(samples),
            'largest_gap_secs': max([b['t'] - a['t'] for a, b in zip(samples, samples[1:])] or [0]),
            'covered_secs': samples[-1]['t'] - samples[0]['t'] if samples else 0,
            'repos_with_several_daemons': {k: v for k, v in worst_per_repo.items() if len(v) > 1},
            'new_standalone_holders': {repo: sorted(pids) for repo, pids in new_standalone.items()},
            'unmeasured_new_standalone': {repo: sorted(pids) for repo, pids in unmeasured_new.items()},
            'preexisting_standalone_holders': {repo: sorted(pids) for repo, pids in old_standalone.items()},
            'daemon_instances': [{'pid': pid, 'key': key, 'first_seen': span[0], 'last_seen': span[1]}
                                 for (pid, key), span in sorted(daemons.items(), key=lambda item: item[1][0])]}


def scan_lines(lines):
    """Client events in shim standard-error lines."""
    events = {'fail_closed': [], 'reached': set(), 'lost': 0, 'build_crash_refusals': 0, 'startup_errors': []}
    for line in lines:
        if FAIL_CLOSED in line:
            events['fail_closed'].append(line.strip())
            if BUILD_CRASH in line:
                events['build_crash_refusals'] += 1
        if STARTUP_ERROR.match(line):
            events['startup_errors'].append(line.strip())
        found = REACHED.search(line)
        if found:
            events['reached'].add((int(found.group(1)), int(found.group(2))))
        if LOST.search(line):
            events['lost'] += 1
    return events


def iso_epoch(text):
    """Epoch seconds of a UTC time written `YYYY-MM-DDTHH:MM:SS…`."""
    return calendar.timegm(time.strptime(text[:19], '%Y-%m-%dT%H:%M:%S'))


def in_window(epoch, since, until):
    return (since is None or epoch >= since) and (until is None or epoch <= until)


def logs_readable(claude_root, client_logs):
    """Whether every named log source exists: the Claude Code root a directory, every client log a
    file. No source at all is not coverage."""
    if not claude_root and not client_logs:
        return False
    if claude_root and not Path(claude_root).is_dir():
        return False
    return all(Path(path).is_file() for path in client_logs)


def claude_log_lines(root, since=None, until=None):
    """Standard-error lines of prism servers in Claude Code's MCP logs under `root`
    (`<project>/mcp-logs-<server>/*.jsonl`, server names containing `prism`)."""
    for path in sorted(Path(root).glob('*/mcp-logs-*prism*/*.jsonl')):
        with open(path, errors='replace') as source:
            for raw in source:
                try:
                    entry = json.loads(raw)
                except ValueError:
                    continue
                text = entry.get('error') if isinstance(entry, dict) else None
                stamp = entry.get('timestamp') if isinstance(entry, dict) else None
                if not isinstance(text, str) or not text.startswith('Server stderr: ') or not stamp:
                    continue
                if in_window(iso_epoch(stamp), since, until):
                    for line in text[len('Server stderr: '):].splitlines():
                        yield line


def client_log_lines(path, since=None, until=None):
    """Lines of a log prism-mcp-logged.sh wrote (`<UTC time> <pid> <line>`), in the window."""
    with open(path, errors='replace') as source:
        for raw in source:
            found = CLIENT_LINE.match(raw.rstrip('\n'))
            if found and in_window(iso_epoch(found.group(1)), since, until):
                yield found.group(3)


def crash_reports(directory, since=None, until=None):
    """(prism-mcp crash reports in `directory` modified in the window, whether it could be read)."""
    try:
        paths = sorted(Path(directory).iterdir()) if directory else None
    except OSError:
        paths = None
    if paths is None:
        return [], False
    return [p.name for p in paths if CRASH_REPORT.match(p.name) and in_window(p.stat().st_mtime, since, until)], True


def in_round(holders, round_repos):
    """The standalone holders whose repository is one of the round's checkouts (paths resolved),
    or all of them when no checkouts are named."""
    if round_repos is None:
        return holders
    checkouts = {os.path.realpath(repo) for repo in round_repos}
    return {repo: pids for repo, pids in holders.items()
            if os.path.isabs(repo) and os.path.realpath(repo) in checkouts}


def unresolved(holders, round_repos):
    """With checkouts named, the holders whose repository is not an absolute path: whether they
    are in the round cannot be told (PR #424 review round 1, sol 1)."""
    if round_repos is None:
        return {}
    return {repo: pids for repo, pids in holders.items() if not os.path.isabs(repo)}


def judge(summary, events, crashes, coverage, round_repos=None):
    """Each rule's verdict: FAIL on a breach that was seen; else UNKNOWN when the evidence it
    needs was not collected; else PASS. `coverage`: {'logs': bool, 'crash_reports': bool}. With
    `round_repos`, a new standalone holder fails the round only in one of those checkouts (review
    round 2, Opus W-D): a launcher the soak does not edit, elsewhere, is reported; one whose repository
    could not be resolved makes the rule UNKNOWN, as does a new standalone server there (or of unknown
    repository) whose footprint could not be measured (PR #424 review round 1, Opus S2)."""
    missing = summary['missing']
    new, unmeasured = summary['new_standalone_holders'], summary['unmeasured_new_standalone']

    def verdict(breach, unseen):
        return 'FAIL' if breach else 'UNKNOWN' if unseen else 'PASS'
    logs = not coverage['logs']
    return {'one_index_per_key': verdict(summary['max_daemons_per_key'] > 1, missing['ps']),
            'no_new_standalone_index': verdict(in_round(new, round_repos),
                                               missing['ps'] or in_round(unmeasured, round_repos)
                                               or unresolved(new, round_repos) or unresolved(unmeasured, round_repos)),
            'no_fail_closed': verdict(events['fail_closed'], logs),
            'no_startup_error': verdict(events['startup_errors'], logs),
            'no_restart_bound': verdict(events['build_crash_refusals'], logs),
            'no_abort': verdict(crashes, not coverage['crash_reports']),
            'no_critical_pressure': verdict(summary['peaks']['pressure'] >= PRESSURE_CRITICAL, missing['pressure'])}


def usage(phase, summary, events, min_shims, min_days, sources_reached=None):
    """The use a phase must have seen, check by check (True when seen). `sources_reached`: for
    each --client-log, whether it holds a reached daemon in the window (review round 2, Opus S-A:
    the bridge's own log must show the bridge's agents reached one)."""
    if phase == 'round':
        return {'shims_at_once_at_least_%d' % min_shims: summary['peaks']['shims'] >= min_shims,
                'a_daemon_reached_in_every_client_log': bool(sources_reached) and all(sources_reached),
                'an_index_built': summary['peaks']['built_daemons'] >= 1}
    return {'usage_days_at_least_%d' % min_days: len(summary['usage_days']) >= min_days}


def overall(phase, verdicts, use):
    """FAIL on any gated rule's FAIL; INADMISSIBLE when a gated rule is UNKNOWN or the use was not
    seen; else PASS."""
    gated = [verdicts[rule] for rule in GATED[phase]]
    if 'FAIL' in gated:
        return 'FAIL'
    return 'INADMISSIBLE' if 'UNKNOWN' in gated or not all(use.values()) else 'PASS'


def render(phase, summary, events, crashes, verdicts, use, result, skipped):
    peaks = summary['peaks']
    lines = ['# prism-mcp soak report (%s)' % phase, '',
             'Samples: %d, from %s to %s (epoch seconds); unreadable sample lines skipped: %d.'
             % (summary['samples'], summary['first'], summary['last'], skipped),
             '', '**Verdict: %s**' % result, '', '| Rule | Verdict | Gated |', '|---|---|---|']
    lines += ['| %s | %s | %s |' % (rule, verdicts[rule], 'yes' if rule in GATED[phase] else 'no') for rule in RULES]
    lines += ['', '| Use | Seen |', '|---|---|'] + ['| %s | %s |' % (k, 'yes' if v else 'NO') for k, v in use.items()]
    lines += ['', '| Measure | Value |', '|---|---|',
              '| Most index holders at once | %d |' % peaks['holders'],
              '| Most daemons for one key at once | %d |' % summary['max_daemons_per_key'],
              '| Most daemons for one repository at once (any configuration or build) | %d |'
              % summary['max_daemons_per_repo'],
              '| Most shims at once | %d |' % peaks['shims'],
              '| Most daemons with a built index at once | %d |' % peaks['built_daemons'],
              "| Usage days (UTC; a sample interval with at least %.2f s of one daemon's CPU) | %d |"
              % (USAGE_INTERVAL_CPU_SECS, len(summary['usage_days'])),
              '| Time covered by the samples / largest gap between two (s) | %d / %d |'
              % (summary['covered_secs'], summary['largest_gap_secs']),
              '| Peak footprint of all prism-mcp processes (MiB) | %.1f |' % (peaks['prism_footprint_bytes'] / MIB),
              '| Peak swap used (MiB) | %.1f |' % peaks['swap_used_mib'],
              '| Peak compressor (MiB) | %.1f |' % (peaks['compressed_bytes'] / MIB),
              '| Highest memory pressure level | %d |' % peaks['pressure'],
              '| Samples without a process table / without pressure | %d / %d |'
              % (summary['missing']['ps'], summary['missing']['pressure']),
              '| Live processes whose footprint could not be measured | %d |' % summary['missing']['footprint'],
              '| Daemon instances seen in samples | %d |' % len(summary['daemon_instances']),
              '| Daemon instances clients reached | %d |' % len(events['reached']),
              '| Fail-closed lines | %d |' % len(events['fail_closed']),
              '| Of which a build that died (restart bound) | %d |' % events['build_crash_refusals'],
              '| Startup errors (a server that never started) | %d |' % len(events['startup_errors']),
              '| Sessions ended by their daemon | %d |' % events['lost'],
              '| prism-mcp crash reports | %d |' % len(crashes)]
    for title, key in (('Keys with more than one daemon at once', 'duplicated_keys'),
                       ('Repositories with several daemons at once (other cache directories, refresh policies or '
                        'builds)', 'repos_with_several_daemons'),
                       ('Standalone index holders first seen after the start (a round gates only those in its '
                        'checkouts)', 'new_standalone_holders'),
                       ('Standalone index holders already running at the start (not counted)',
                        'preexisting_standalone_holders'),
                       ('Standalone servers first seen after the start whose footprint could not be measured',
                        'unmeasured_new_standalone')):
        if summary[key]:
            lines += ['', '%s: %s' % (title, json.dumps(summary[key]))]
    lines += ['', 'Fail-closed lines:'] + ['- `%s`' % line for line in events['fail_closed'][:50]]
    lines += ['', 'Startup errors:'] + ['- `%s`' % line for line in events['startup_errors'][:50]]
    lines += ['', 'Crash reports:'] + ['- `%s`' % name for name in crashes]
    return '\n'.join(lines) + '\n'


def cmd_report(args):
    samples, skipped = load_samples(args.samples, args.since, args.until)
    if not samples:
        raise SystemExit('no samples in %s for the window' % args.samples)
    client_logs = args.client_log or []
    lines, sources_reached = [], []
    if args.claude_logs and Path(args.claude_logs).is_dir():
        lines += list(claude_log_lines(args.claude_logs, args.since, args.until))
    for path in client_logs:
        found = list(client_log_lines(path, args.since, args.until)) if os.path.isfile(path) else []
        sources_reached.append(bool(scan_lines(found)['reached']))
        lines += found
    summary = summarize_samples(samples)
    events = scan_lines(lines)
    crashes, crash_ok = crash_reports(args.crash_reports, args.since, args.until)
    coverage = {'logs': logs_readable(args.claude_logs, client_logs), 'crash_reports': crash_ok}
    verdicts = judge(summary, events, crashes, coverage, args.round_repo)
    use = usage(args.phase, summary, events, args.min_shims, args.min_days, sources_reached)
    result = overall(args.phase, verdicts, use)
    Path(args.out).write_text(render(args.phase, summary, events, crashes, verdicts, use, result, skipped))
    if args.json:
        Path(args.json).write_text(json.dumps(
            {'phase': args.phase, 'verdict': result, 'summary': summary, 'coverage': coverage, 'use': use,
             'events': dict(events, reached=sorted(events['reached'])), 'crash_reports': crashes,
             'verdicts': verdicts, 'skipped_lines': skipped}, indent=1, sort_keys=True))
    print('wrote %s: %s' % (args.out, result))
    return 0
