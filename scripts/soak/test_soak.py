"""Tests of soak.py and prism-mcp-logged.sh.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import calendar
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import soak  # noqa: E402

MIB = 1024 * 1024
KEY = 'ab' * 32
PS = """  101     1   4096   1:02.50 /x/target/release/prism-mcp daemon --repo /r --cache-dir /c --refresh-policy warn-only --key %s
  102    50   3000   0:00.01 /x/target/release/prism-mcp --repo /r --cache-dir /c --shared
  103    50 900000 01:02:03 /x/prism-mcp --repo /other
  104    50   2000   0:00.00 /x/prism-mcp daemon-status --runtime-dir /tmp
  105    50   1000   0:00.00 /usr/bin/vim prism-mcp
  106    50   1000   0:00.00 /x/prism-mcp.pre-389 --repo /r
""" % KEY


def proc(role, pid, footprint=0, key=KEY, repo='/r', status='ok', cache='/c', cpu=None):
    return {'role': role, 'pid': pid, 'key': key if role == 'daemon' else None, 'repo': repo, 'cache_dir': cache,
            'cpu_secs': cpu,
            'footprint_bytes': footprint if status == 'ok' else None, 'footprint_status': status,
            'holder': soak.is_holder({'role': role, 'footprint_bytes': footprint if status == 'ok' else None})}


def sample(t, processes, swap_used=0.0, pressure=1, compressed=0, ps_ok=True):
    return {'t': t, 'processes': processes, 'ps_ok': ps_ok,
            'host': {'swap_mib': {'used': swap_used}, 'pressure': pressure, 'compressed_bytes': compressed}}


ALL_READ = {'logs': True, 'crash_reports': True}


class Processes(unittest.TestCase):
    def test_roles_come_from_the_command_line(self):
        found = {p['pid']: p for p in soak.parse_ps(PS)}
        self.assertEqual(sorted(found), [101, 102, 103, 104])
        self.assertEqual((found[101]['role'], found[101]['key'], found[101]['repo']), ('daemon', KEY, '/r'))
        self.assertEqual(found[102]['role'], 'shim')
        self.assertEqual((found[103]['role'], found[103]['rss_bytes']), ('standalone', 900000 * 1024))
        self.assertEqual((found[101]['cpu_secs'], found[103]['cpu_secs']), (62.5, 3723.0))
        self.assertEqual(found[104]['role'], 'operator')

    def test_an_index_holder_is_a_daemon_or_a_standalone_server_past_100_mib(self):
        self.assertTrue(soak.is_holder({'role': 'daemon', 'footprint_bytes': 0}))
        self.assertTrue(soak.is_holder({'role': 'standalone', 'footprint_bytes': 100 * MIB}))
        self.assertFalse(soak.is_holder({'role': 'standalone', 'footprint_bytes': 100 * MIB - 1}))
        self.assertFalse(soak.is_holder({'role': 'standalone', 'footprint_bytes': None}))
        self.assertFalse(soak.is_holder({'role': 'shim', 'footprint_bytes': 900 * MIB}))

    def test_host_figures_are_parsed(self):
        self.assertEqual(soak.parse_swapusage('total = 3072.00M  used = 1887.25M  free = 1184.75M  (encrypted)'),
                         {'total': 3072.0, 'used': 1887.25, 'free': 1184.75})
        vm_stat = 'Mach Virtual Memory Statistics: (page size of 16384 bytes)\nPages occupied by compressor:   10.\n'
        self.assertEqual(soak.parse_compressed(vm_stat), 163840)
        self.assertIsNone(soak.parse_compressed('nothing'))

    def test_a_live_sample_has_the_host_and_a_process_list(self):
        taken = soak.sample_once()
        self.assertIsInstance(taken['processes'], list)
        self.assertIn('t', taken)
        self.assertIsInstance(taken['host'], dict)


class Samples(unittest.TestCase):
    def test_two_daemons_of_one_key_at_once_are_found(self):
        summary = soak.summarize_samples([
            sample(1, [proc('daemon', 1), proc('shim', 2)]),
            sample(2, [proc('daemon', 1), proc('daemon', 3), proc('shim', 2)])])
        self.assertEqual(summary['max_daemons_per_key'], 2)
        self.assertEqual(list(summary['duplicated_keys'].values()), [[1, 3]])
        self.assertEqual(len(summary['daemon_instances']), 2)

    def test_daemons_of_different_keys_and_one_after_another_are_not_duplicates(self):
        summary = soak.summarize_samples([
            sample(1, [proc('daemon', 1), proc('daemon', 2, key='cd' * 32)]),
            sample(2, [proc('daemon', 3)])])
        self.assertEqual(summary['max_daemons_per_key'], 1)
        self.assertEqual(summary['peaks']['holders'], 2)
        self.assertEqual(summary['max_daemons_per_repo'], 2)
        self.assertEqual(summary['repos_with_several_daemons'], {'/r': [1, 2]})
        other = soak.summarize_samples([sample(1, [proc('daemon', 1), proc('daemon', 2, key='cd' * 32, repo='/s'),
                                                   proc('shim', 5, repo='/s')])])
        self.assertEqual((other['max_daemons_per_repo'], other['repos_with_several_daemons']), (1, {}))

    def test_standalone_holders_and_peaks_are_reported(self):
        summary = soak.summarize_samples([
            sample(1, [proc('standalone', 7, 200 * MIB, repo='/a'), proc('standalone', 8, MIB, repo='/b')],
                   swap_used=10.0, pressure=2, compressed=5),
            sample(2, [proc('shim', 9, MIB)], swap_used=4.0, pressure=1, compressed=9)])
        self.assertEqual(summary['new_standalone_holders'], {})
        self.assertEqual(summary['preexisting_standalone_holders'], {'/a': [7]})
        self.assertEqual(summary['peaks'], {'holders': 1, 'shims': 1, 'prism_footprint_bytes': 201 * MIB,
                                            'swap_used_mib': 10.0, 'pressure': 2, 'compressed_bytes': 9,
                                            'built_daemons': 0})



FAILED = ('prism-mcp: --shared failed closed, no index was built: no daemon answered READY before the deadline; '
          'key k; runtime dir /tmp/prism-mcpd')
CRASHED = ('prism-mcp: --shared failed closed, no index was built: the last daemon for this key, pid 9, died '
           'during its index build; the key is not restarted')
REACHED = 'prism-mcp: shared daemon pid 41 (started 1700000000, generation 0) at /tmp/prism-mcpd/x.sock'
LOST = 'prism-mcp: the shared daemon closed the connection (shared daemon pid 41); this session cannot continue'


class Events(unittest.TestCase):
    def test_client_lines_are_counted(self):
        events = soak.scan_lines([FAILED, CRASHED, REACHED, REACHED, LOST, 'nav cache: CPG cache miss'])
        self.assertEqual(len(events['fail_closed']), 2)
        self.assertEqual(events['build_crash_refusals'], 1)
        self.assertEqual(events['reached'], {(41, 1700000000)})
        self.assertEqual(events['lost'], 1)

    def test_claude_code_logs_are_read_for_prism_servers_in_the_window(self):
        with tempfile.TemporaryDirectory() as root:
            logs = Path(root) / '-Users-me-repo' / 'mcp-logs-plugin-prism-prism'
            other = Path(root) / '-Users-me-repo' / 'mcp-logs-plugin-slack'
            logs.mkdir(parents=True)
            other.mkdir(parents=True)
            entries = [{'error': 'Server stderr: %s\n' % FAILED, 'timestamp': '2026-10-16T10:00:00.000Z'},
                       {'error': 'Server stderr: %s\n' % REACHED, 'timestamp': '2026-10-16T12:00:00.000Z'},
                       {'debug': 'Starting connection', 'timestamp': '2026-10-16T10:00:00.000Z'}]
            (logs / 'a.jsonl').write_text('\n'.join(json.dumps(e) for e in entries) + '\nnot json\n')
            (other / 'a.jsonl').write_text(json.dumps(entries[0]) + '\n')
            since = soak.iso_epoch('2026-10-16T09:00:00')
            until = soak.iso_epoch('2026-10-16T11:00:00')
            self.assertEqual(list(soak.claude_log_lines(root, since, until)), [FAILED])
            self.assertEqual(len(list(soak.claude_log_lines(root))), 2)

    def test_iso_times_are_utc_in_any_local_zone_winter_and_summer(self):
        # Review round 1 (sol S7, Opus S7): run under explicit zones, so that local time read as UTC
        # fails everywhere, and in July, where `mktime(...) - time.timezone` is an hour out.
        stamps = {'2026-01-15T12:00:00Z': calendar.timegm((2026, 1, 15, 12, 0, 0)),
                  '2026-07-15T12:00:00Z': calendar.timegm((2026, 7, 15, 12, 0, 0))}
        code = 'import sys, soak; print(soak.iso_epoch(sys.argv[1]))'
        for zone in ('UTC0', 'MST7MDT,M3.2.0,M11.1.0'):
            for stamp, expected in stamps.items():
                done = subprocess.run([sys.executable, '-c', code, stamp], cwd=str(HERE), capture_output=True,
                                      text=True, env=dict(os.environ, TZ=zone))
                self.assertEqual(done.stdout.strip(), str(expected), (zone, stamp, done.stderr))

    def test_crash_reports_of_prism_mcp_in_the_window_are_counted(self):
        with tempfile.TemporaryDirectory() as directory:
            for name in ('prism-mcp-2026-10-16-101010.ips', 'prism-mcp_2026-10-16-101011_host.crash',
                         'Safari-2026-10-16-101010.ips'):
                (Path(directory) / name).write_text('x')
            old = Path(directory) / 'prism-mcp-2026-01-01-000000.ips'
            old.write_text('x')
            os.utime(old, (100, 100))
            self.assertEqual(soak.crash_reports(directory, since=1000),
                             (['prism-mcp-2026-10-16-101010.ips', 'prism-mcp_2026-10-16-101011_host.crash'], True))
            self.assertEqual(soak.crash_reports(str(Path(directory) / 'missing')), ([], False))


class Report(unittest.TestCase):
    def test_the_rules_pass_on_a_clean_soak_and_each_fails_on_its_own_breach(self):
        def failing(summary, events, crashes=()):
            return [r for r, v in soak.judge(summary, events, list(crashes), ALL_READ).items() if v != 'PASS']
        clean = soak.summarize_samples([sample(1, [proc('daemon', 1), proc('shim', 2)])])
        quiet = soak.scan_lines([REACHED])
        self.assertEqual(failing(clean, quiet), [])
        two = soak.summarize_samples([sample(1, [proc('daemon', 1), proc('daemon', 2)])])
        self.assertEqual(failing(two, quiet), ['one_index_per_key'])
        alone = soak.summarize_samples([sample(1, []), sample(2, [proc('standalone', 3, 500 * MIB)])])
        self.assertEqual(failing(alone, quiet), ['no_new_standalone_index'])
        self.assertEqual(failing(clean, soak.scan_lines([FAILED])), ['no_fail_closed'])
        self.assertEqual(failing(clean, quiet, ['prism-mcp-x.ips']), ['no_abort'])
        critical = soak.summarize_samples([sample(1, [], pressure=4)])
        self.assertEqual(failing(critical, quiet), ['no_critical_pressure'])

    def test_report_writes_the_table_and_the_verdicts(self):
        with tempfile.TemporaryDirectory() as directory:
            samples = Path(directory) / 's.jsonl'
            samples.write_text(json.dumps(sample(time.time(), [proc('daemon', 1)])) + '\n')
            client = Path(directory) / 'client.log'
            client.write_text('%s 77 %s\n' % (time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), FAILED))
            out, data = Path(directory) / 'r.md', Path(directory) / 'r.json'
            with redirect_stdout(io.StringIO()):
                soak.main(['report', '--phase', 'week', '--samples', str(samples), '--client-log', str(client),
                           '--out', str(out), '--json', str(data), '--crash-reports', directory])
            self.assertIn('| no_fail_closed | FAIL |', out.read_text())
            self.assertIn('**Verdict: FAIL**', out.read_text())
            self.assertEqual(json.loads(data.read_text())['verdicts']['one_index_per_key'], 'PASS')

    def test_sample_appends_one_line_per_sample(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / 's.jsonl'
            with redirect_stdout(io.StringIO()):
                soak.main(['sample', '--out', str(out), '--count', '2', '--interval', '0'])
                soak.main(['sample', '--out', str(out), '--count', '1', '--interval', '0'])
            self.assertEqual(soak.load_samples(out), (soak.load_samples(out)[0], 0))
            self.assertEqual(len(soak.load_samples(out)[0]), 3)


class Wrapper(unittest.TestCase):
    def test_standard_error_is_logged_with_time_and_pid_and_nothing_else_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / 'stderr.log'
            code = ('import sys; sys.stdout.write(sys.stdin.read()); sys.stderr.write("one\\ntwo"); '
                    'sys.exit(3)')
            child = subprocess.Popen([str(HERE / 'prism-mcp-logged.sh'), str(log), sys.executable, '-c', code],
                                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            out, err = child.communicate(b'{"jsonrpc":"2.0"}\n')
            self.assertEqual((child.returncode, out, err), (3, b'{"jsonrpc":"2.0"}\n', b'one\ntwo\n'))
            lines = log.read_text().splitlines()
            self.assertEqual([soak.CLIENT_LINE.match(line).group(3) for line in lines], ['one', 'two'])
            self.assertEqual({soak.CLIENT_LINE.match(line).group(2) for line in lines}, {str(child.pid)})
            self.assertEqual(list(soak.client_log_lines(log)), ['one', 'two'])
            self.assertTrue(all(re.match(r'\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ ', line) for line in lines))


if __name__ == '__main__':
    unittest.main()
