"""Tests of the fold of PR #424's review round 1 (gpt-6.1-sol 1-2, Opus W1, W2, S1-S5): a usage day
needs one sample interval of real daemon work (owner decision 2026-10-16), the repository and the
binary are read from every form a process table shows them in, and the wrapper survives a log it
cannot write.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import soak  # noqa: E402
from test_soak import ALL_READ, KEY, MIB, REACHED, proc, sample  # noqa: E402

DAY = 86400
BIN = '/x/target/release/prism-mcp'
USED = ('  101     1   4096   0:09.00 %s daemon --repo /r --cache-dir /c --refresh-policy warn-only --key %s\n'
        '  102    50   3000   0:00.01 %s --repo /r --cache-dir /c --shared\n'
        '  103    50   3000   0:00.01 %s --repo /r --cache-dir /c --shared\n') % (BIN, KEY, BIN, BIN)
SIZES = {101: 300 * MIB, 102: 3 * MIB, 103: 3 * MIB, 109: 500 * MIB, 110: 300 * MIB}


def measured(pids):
    return {pid: SIZES.get(pid, MIB) for pid in pids}, {pid: 'ok' for pid in pids}


def round_report(listings, round_repos, cwd_of=lambda pid: None):
    """The JSON report of a round whose process table read each of `listings` in turn, from the
    parser through the sampler's resolution to `soak.py report`."""
    with tempfile.TemporaryDirectory() as directory:
        samples = Path(directory) / 's.jsonl'
        lines = [json.dumps({'t': 1000.0 + 60 * k, 'ps_ok': True,
                             'host': {'swap_mib': {'used': 0.0}, 'pressure': 1, 'compressed_bytes': 0},
                             'processes': soak.observe(listing, measure=measured, cwd_of=cwd_of)})
                 for k, listing in enumerate(listings)]
        samples.write_text('\n'.join(lines) + '\n')
        client = Path(directory) / 'client.log'
        client.write_text('1970-01-01T00:16:40Z 7 %s\n' % REACHED)
        data = Path(directory) / 'r.json'
        command = ['report', '--phase', 'round', '--samples', str(samples), '--client-log', str(client),
                   '--crash-reports', directory, '--out', str(Path(directory) / 'r.md'), '--json', str(data)]
        for repo in round_repos:
            command += ['--round-repo', repo]
        with redirect_stdout(io.StringIO()):
            soak.main(command)
        return json.loads(data.read_text())


def daemon(cpu, pid=1):
    return proc('daemon', pid, 400 * MIB, cpu=cpu)


class UsageInterval(unittest.TestCase):
    """Owner decision 2026-10-16 (Opus W1): a UTC day counts when some single sample interval
    shows at least 0.05 s of one daemon's CPU growth, not 0.5 s summed over the day."""

    def test_a_resident_idle_daemon_shows_no_usage_day_in_a_week(self):
        # Measured idle cost: about 0.0006 s per minute (0.86 s a day), from the 5 s identity tick.
        samples = [sample(60 * k, [daemon(1.0 + 0.0006 * k), proc('shim', 2)]) for k in range(7 * 1440)]
        self.assertEqual(soak.usage_days(samples), [])

    def test_one_interval_of_work_counts_its_day(self):
        busy = DAY + 3600
        samples = [sample(60 * k, [daemon(1.0 + 0.0006 * k + (0.6 if 60 * k >= busy else 0.0))])
                   for k in range(3 * 1440)]
        self.assertEqual(soak.usage_days(samples), ['1970-01-02'])

    def test_exactly_the_threshold_counts_and_just_below_does_not(self):
        # ps prints hundredths: 0:00.25 then 0:00.30 is 0.05 s, which floating point makes 0.0499...
        at = [sample(0, [daemon(soak.parse_cputime('0:00.25'))]), sample(60, [daemon(soak.parse_cputime('0:00.30'))])]
        self.assertEqual(soak.usage_days(at), ['1970-01-01'])
        below = [sample(0, [daemon(0.25)]), sample(60, [daemon(0.299)])]
        self.assertEqual(soak.usage_days(below), [])

    def test_a_pid_missing_from_a_sample_contributes_nothing_for_that_interval(self):
        gap = [sample(0, [daemon(1.0)]), sample(60, [], ps_ok=False), sample(120, [daemon(1.6)])]
        self.assertEqual(soak.usage_days(gap), [])
        new = [sample(0, []), sample(60, [daemon(0.6, pid=9)])]
        self.assertEqual(soak.usage_days(new), [])
        unseen = [sample(0, []), sample(60, [], ps_ok=False), sample(120, [daemon(0.6, pid=9)])]
        self.assertEqual(soak.usage_days(unseen), [])

    def test_the_report_states_the_interval_rule(self):
        summary = soak.summarize_samples([sample(0, [daemon(1.0)])])
        verdicts = soak.judge(summary, soak.scan_lines([]), [], {'logs': True, 'crash_reports': True})
        use = soak.usage('week', summary, soak.scan_lines([]), 2, 5)
        text = soak.render('week', summary, soak.scan_lines([]), [], verdicts, use, 'INADMISSIBLE', 0)
        self.assertIn('a sample interval with at least 0.05 s of one daemon', text)


class Repository(unittest.TestCase):
    """sol 1, Opus S3: `--repo=value`, a path with a space and a relative path are the repository
    the process serves; one that cannot be resolved never lets the round's rule pass."""

    def test_the_repository_is_read_in_every_form_and_kept_whole(self):
        self.assertEqual(soak.classify('%s --repo=/r --shared' % BIN)['repo'], '/r')
        found = soak.classify('%s --repo /a dir --cache-dir /c  d --shared' % BIN)
        self.assertEqual((found['role'], found['repo'], found['cache_dir']), ('shim', '/a dir', '/c  d'))
        self.assertEqual(soak.classify('%s daemon --repo=/a dir --key=%s' % (BIN, KEY))['key'], KEY)
        self.assertEqual(soak.classify('%s daemon --repo=/a dir --key=%s' % (BIN, KEY))['repo'], '/a dir')
        self.assertIsNone(soak.classify('%s --repo' % BIN)['repo'])

    def standalone(self, args):
        return USED + '  109    50 500000   0:30.00 %s %s\n' % (BIN, args)

    def test_a_new_standalone_holder_named_with_equals_fails_the_round(self):
        report = round_report([USED, self.standalone('--repo=/r')], ['/r'])
        self.assertEqual(report['verdicts']['no_new_standalone_index'], 'FAIL')
        self.assertEqual(report['verdict'], 'FAIL')

    def test_a_new_standalone_holder_in_a_path_with_a_space_fails_the_round(self):
        report = round_report([USED, self.standalone('--repo /a dir --cache-dir /c')], ['/a dir'])
        self.assertEqual(report['verdicts']['no_new_standalone_index'], 'FAIL')
        self.assertEqual(report['summary']['new_standalone_holders'], {'/a dir': [109]})

    def test_a_relative_repository_resolves_against_the_process_s_working_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            checkout = os.path.join(directory, 'checkout')
            os.mkdir(checkout)
            report = round_report([USED, self.standalone('--repo checkout')], [checkout],
                                  cwd_of=lambda pid: directory if pid == 109 else None)
        self.assertEqual(report['verdicts']['no_new_standalone_index'], 'FAIL')

    def test_an_unresolved_repository_makes_the_round_rule_unknown(self):
        report = round_report([USED, self.standalone('--repo checkout')], ['/r'])
        self.assertEqual(report['verdicts']['no_new_standalone_index'], 'UNKNOWN')
        self.assertEqual(report['verdict'], 'INADMISSIBLE')
        self.assertEqual(list(report['summary']['new_standalone_holders']), ['(unresolved) checkout'])
        elsewhere = round_report([USED, self.standalone('--repo=/elsewhere')], ['/r'])
        self.assertEqual((elsewhere['verdicts']['no_new_standalone_index'], elsewhere['verdict']), ('PASS', 'PASS'))

    def test_the_working_directory_of_a_live_process_is_read(self):
        self.assertEqual(os.path.realpath(soak.process_cwd(os.getpid())), os.path.realpath(os.getcwd()))


HASHED = '/e/bin/prism-mcp-0123456789ab'


class HashNamedBinaries(unittest.TestCase):
    """sol 2: Phase 0 installs `prism-mcp-<sha12>`; such a process is prism-mcp, and each process
    counts by its pid whatever its binary is called."""

    def test_prism_mcp_and_prism_mcp_dash_twelve_hex_are_recognized_and_nothing_else(self):
        self.assertEqual(soak.classify('%s --repo /r --shared' % HASHED)['role'], 'shim')
        self.assertEqual(soak.classify('prism-mcp-0123456789ab daemon --repo /r --key %s' % KEY)['key'], KEY)
        for other in ('/x/scripts/soak/prism-mcp-logged.sh /l /x/prism-mcp --repo /r',
                      '/x/prism-mcp-0123456789abc --repo /r', '/x/prism-mcp-0123456789a --repo /r',
                      '/x/prism-mcp-0123456789AB --repo /r', '/x/deps/prism_mcp-0123456789abcdef --repo /r',
                      '/x/prism-mcp-launch.sh /r', '/x/prism-mcp.pre-389 --repo /r'):
            self.assertIsNone(soak.classify(other), other)
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / 'a b' / 'prism-mcp-0123456789ab'
            binary.parent.mkdir()
            binary.write_text('#!/bin/sh\n')
            binary.chmod(0o755)
            self.assertEqual(soak.classify('%s --repo /r --shared' % binary)['role'], 'shim')

    def test_a_healthy_round_of_hash_named_processes_passes(self):
        report = round_report([USED.replace(BIN, HASHED)], ['/r'])
        self.assertEqual(report['verdict'], 'PASS', report['use'])

    def test_two_daemons_of_one_key_under_the_two_names_fail_one_index_per_key(self):
        second = '  110     1   4096   0:09.00 %s daemon --repo /r --cache-dir /c --refresh-policy warn-only --key %s\n'
        report = round_report([USED, USED + second % (HASHED, KEY)], ['/r'])
        self.assertEqual(report['verdicts']['one_index_per_key'], 'FAIL')
        self.assertEqual(report['summary']['duplicated_keys'], {'daemon ' + KEY: [101, 110]})


class WrapperLog(unittest.TestCase):
    """Opus W2: a log the wrapper cannot append to never costs the client a line or the server
    its standard error."""

    def test_an_unwritable_log_passes_every_line_to_the_client_and_keeps_the_server_alive(self):
        code = ('import sys, time; sys.stderr.write("one\\n"); sys.stderr.flush(); time.sleep(0.5); '
                'sys.stderr.write("two\\n"); sys.stderr.flush(); sys.stdout.write(sys.stdin.read()); sys.exit(3)')
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / 'no-such-dir' / 'stderr.log'
            child = subprocess.Popen([str(HERE / 'prism-mcp-logged.sh'), str(log), sys.executable, '-c', code],
                                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            out, err = child.communicate(b'{"jsonrpc":"2.0"}\n', timeout=60)
            self.assertEqual((child.returncode, out, err), (3, b'{"jsonrpc":"2.0"}\n', b'one\ntwo\n'))
            self.assertFalse(log.exists())


class UnmeasuredStandalone(unittest.TestCase):
    """Opus S2: an unreadable footprint makes no_new_standalone_index UNKNOWN only for a server
    that could fail it: new, and in one of the round's checkouts (or of unknown repository)."""

    def rule(self, samples, round_repos=('/r',)):
        summary = soak.summarize_samples(samples)
        return soak.judge(summary, soak.scan_lines([REACHED]), [], ALL_READ, list(round_repos))['no_new_standalone_index']

    def test_only_a_new_unmeasured_standalone_server_in_the_round_is_unknown(self):
        old = proc('standalone', 7, status='failed')
        self.assertEqual(self.rule([sample(1, [old]), sample(2, [old])]), 'PASS')
        elsewhere = proc('standalone', 8, status='failed', repo='/elsewhere')
        self.assertEqual(self.rule([sample(1, []), sample(2, [elsewhere])]), 'PASS')
        inside = proc('standalone', 8, status='failed', repo='/r')
        self.assertEqual(self.rule([sample(1, []), sample(2, [inside])]), 'UNKNOWN')
        unknown = dict(proc('standalone', 8, status='failed', repo=None), repo_arg='checkout')
        self.assertEqual(self.rule([sample(1, []), sample(2, [unknown])]), 'UNKNOWN')


class Robustness(unittest.TestCase):
    """Opus S5: `probe` without a command, and a crash report gone between listing and reading."""

    def test_probe_without_a_command_prints_its_usage_and_exits_2(self):
        for argv in (['probe'], ['probe', '--']):
            err = io.StringIO()
            with redirect_stdout(io.StringIO()), redirect_stderr(err), self.assertRaises(SystemExit) as stop:
                soak.main(argv)
            self.assertEqual(stop.exception.code, 2, argv)
            self.assertIn('usage:', err.getvalue())

    def test_a_crash_report_that_vanished_after_the_listing_is_skipped(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / 'prism-mcp-2026-10-16-101010.ips').write_text('x')
            os.symlink(os.path.join(directory, 'gone'), os.path.join(directory, 'prism-mcp-2026-10-16-101011.ips'))
            self.assertEqual(soak.crash_reports(directory), (['prism-mcp-2026-10-16-101010.ips'], True))


if __name__ == '__main__':
    unittest.main()
