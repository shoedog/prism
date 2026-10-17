"""Tests of what soak.py does when it could not observe something, and of the use a phase needs
(review round 1: sol 2, Opus W2, W3, S2, S3, S5). A rule that could not be judged is UNKNOWN, never
PASS; a phase that saw no use is INADMISSIBLE.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import io
import json
import os
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
from test_soak import ALL_READ, FAILED, MIB, REACHED, proc, sample  # noqa: E402

CLAP = "error: unexpected argument '--shared' found"
CRASHED = 'prism-mcp: --shared failed closed, no index was built: the last daemon for this key died during its index build'


def dead_pid():
    child = subprocess.Popen(['true'])
    child.wait()
    return child.pid


def verdicts(samples, lines=(), crashes=(), coverage=None):
    summary = soak.summarize_samples(samples)
    return soak.judge(summary, soak.scan_lines(list(lines)), list(crashes), coverage or ALL_READ)


class Unobserved(unittest.TestCase):
    def test_a_failed_process_table_makes_the_process_rules_unknown(self):
        found = verdicts([sample(1, [], ps_ok=False)], [REACHED])
        self.assertEqual((found['one_index_per_key'], found['no_new_standalone_index']), ('UNKNOWN', 'UNKNOWN'))

    def test_a_failed_footprint_of_a_standalone_server_is_unknown_and_a_vanished_one_is_not(self):
        failed = verdicts([sample(1, []), sample(2, [proc('standalone', 7, status='failed')])], [REACHED])
        self.assertEqual(failed['no_new_standalone_index'], 'UNKNOWN')
        gone = verdicts([sample(1, []), sample(2, [proc('standalone', 7, status='gone')])], [REACHED])
        self.assertEqual(gone['no_new_standalone_index'], 'PASS')

    def test_a_missing_pressure_reading_is_unknown_and_a_critical_one_still_fails(self):
        self.assertEqual(verdicts([sample(1, [], pressure=None)], [REACHED])['no_critical_pressure'], 'UNKNOWN')
        both = verdicts([sample(1, [], pressure=None), sample(2, [], pressure=4)], [REACHED])
        self.assertEqual(both['no_critical_pressure'], 'FAIL')

    def test_unread_logs_or_crash_reports_are_unknown_and_what_was_read_still_fails(self):
        unread = verdicts([sample(1, [])], [], coverage={'logs': False, 'crash_reports': False})
        for rule in ('no_fail_closed', 'no_startup_error', 'no_restart_bound', 'no_abort'):
            self.assertEqual(unread[rule], 'UNKNOWN', rule)
        partial = verdicts([sample(1, [])], [FAILED], ['prism-mcp-1.ips'], {'logs': False, 'crash_reports': False})
        self.assertEqual((partial['no_fail_closed'], partial['no_abort']), ('FAIL', 'FAIL'))

    def test_a_vanished_pid_is_gone_and_a_live_one_without_a_reading_failed(self):
        saved = (soak.FOOTPRINT_COMMAND, soak.PROC)
        soak.FOOTPRINT_COMMAND, soak.PROC = 'no-such-footprint-command', Path('/no/such/proc')
        try:
            sizes, statuses = soak.measure_footprints([os.getpid(), dead_pid()])
        finally:
            soak.FOOTPRINT_COMMAND, soak.PROC = saved
        self.assertEqual(sizes, {})
        self.assertEqual(sorted(statuses.values()), ['failed', 'gone'])
        self.assertEqual(statuses[os.getpid()], 'failed')

    def test_a_sample_whose_process_table_could_not_be_read_says_so(self):
        saved = soak.PS_COMMAND
        soak.PS_COMMAND = ['no-such-ps-command']
        try:
            taken = soak.sample_once()
        finally:
            soak.PS_COMMAND = saved
        self.assertEqual((taken['ps_ok'], taken['processes']), (False, []))

    def test_log_sources_are_complete_only_when_every_named_one_exists(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / 'bridge.log'
            log.write_text('')
            self.assertTrue(soak.logs_readable(directory, [str(log)]))
            self.assertFalse(soak.logs_readable(str(Path(directory) / 'missing'), [str(log)]))
            self.assertFalse(soak.logs_readable(directory, [str(Path(directory) / 'missing.log')]))
            self.assertFalse(soak.logs_readable(None, []))


class Use(unittest.TestCase):
    def built(self, t):
        return sample(t, [proc('daemon', 1, 300 * MIB), proc('shim', 2), proc('shim', 3)])

    def test_a_round_needs_shims_a_reached_daemon_and_a_built_index(self):
        def phase(samples, lines):
            summary = soak.summarize_samples(samples)
            events = soak.scan_lines(lines)
            use = soak.usage('round', summary, events, min_shims=2, min_days=5, sources_reached=[bool(events['reached'])])
            return soak.overall('round', soak.judge(summary, events, [], ALL_READ), use)
        self.assertEqual(phase([self.built(1)], [REACHED]), 'PASS')
        self.assertEqual(phase([self.built(1), sample(2, [], ps_ok=False)], [REACHED]), 'INADMISSIBLE')
        self.assertEqual(phase([sample(1, [])], []), 'INADMISSIBLE')
        self.assertEqual(phase([self.built(1)], []), 'INADMISSIBLE')
        idle = sample(1, [proc('daemon', 1, 3 * MIB), proc('shim', 2), proc('shim', 3)])
        self.assertEqual(phase([idle], [REACHED]), 'INADMISSIBLE')
        self.assertEqual(phase([self.built(1), sample(2, [proc('daemon', 1), proc('daemon', 5)])], [REACHED]),
                         'FAIL')

    def test_standalone_holders_already_running_at_the_start_do_not_fail_a_phase(self):
        before = proc('standalone', 7, 500 * MIB, repo='/old')
        summary = soak.summarize_samples([sample(1, [before]), sample(2, [before, proc('standalone', 8, 500 * MIB)])])
        self.assertEqual(summary['preexisting_standalone_holders'], {'/old': [7]})
        self.assertEqual(summary['new_standalone_holders'], {'/r': [8]})

    def test_a_usage_day_is_a_day_a_daemon_worked_not_one_it_was_resident(self):
        # Review round 2 (Opus W-B): one call on day 1, then a window left open all week, is one day.
        day = 86400
        resident = [proc('daemon', 1, 400 * MIB, cpu=3.2), proc('shim', 2), proc('shim', 3)]
        samples = [sample(0, [proc('daemon', 1, 3 * MIB, cpu=0.2), proc('shim', 2)]),
                   sample(600, [proc('daemon', 1, 400 * MIB, cpu=3.2), proc('shim', 2)])]
        samples += [sample(d * day + 600 * k, resident) for d in range(1, 7) for k in range(3)]
        self.assertEqual(soak.summarize_samples(samples)['usage_days'], ['1970-01-01'])
        # PR #424 review round 1 (owner decision 2026-10-16): below 0.05 s in one interval is not use.
        quiet = soak.summarize_samples([sample(0, [proc('daemon', 1, 400 * MIB, cpu=50.0), proc('shim', 2)]),
                                        sample(600, [proc('daemon', 1, 400 * MIB, cpu=50.04), proc('shim', 2)])])
        self.assertEqual(quiet['usage_days'], [])
        new = soak.summarize_samples([sample(0, []), sample(day, [proc('daemon', 5, 400 * MIB, cpu=4.0)])])
        self.assertEqual(new['usage_days'], ['1970-01-02'])

    def test_a_startup_error_and_a_restart_bound_refusal_fail_their_rules(self):
        found = verdicts([sample(1, [])], [CLAP, CRASHED])
        self.assertEqual((found['no_startup_error'], found['no_restart_bound']), ('FAIL', 'FAIL'))
        launcher = verdicts([sample(1, [])], ["prism-mcp-launch.sh: PRISM_MCP_SHARED must be 1 (…), not 'yes'"])
        self.assertEqual(launcher['no_startup_error'], 'FAIL')


class Edges(unittest.TestCase):
    def test_a_binary_path_with_a_space_is_still_prism_mcp(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / 'a b' / 'prism-mcp'
            binary.parent.mkdir()
            binary.write_text('#!/bin/sh\n')
            binary.chmod(0o755)
            found = soak.classify('%s --repo /r --shared' % binary)
            self.assertEqual((found['role'], found['repo']), ('shim', '/r'))
            self.assertIsNone(soak.classify('/usr/bin/vim %s' % binary))
        self.assertIsNone(soak.classify('/usr/bin/vim /r/prism-mcp'))

    def test_a_truncated_last_line_is_skipped_and_counted(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 's.jsonl'
            path.write_text(json.dumps(sample(1, [])) + '\n' + '{"t": 2, "proc')
            samples, skipped = soak.load_samples(path)
            self.assertEqual((len(samples), skipped), (1, 1))

    def test_a_round_report_without_its_log_sources_is_inadmissible(self):
        with tempfile.TemporaryDirectory() as directory:
            samples = Path(directory) / 's.jsonl'
            built = sample(time.time(), [proc('daemon', 1, 300 * MIB), proc('shim', 2), proc('shim', 3)])
            samples.write_text(json.dumps(built) + '\n')
            out = Path(directory) / 'r.md'
            with redirect_stdout(io.StringIO()):
                soak.main(['report', '--phase', 'round', '--round-repo', '/r', '--samples', str(samples), '--claude-logs',
                           str(Path(directory) / 'no-such-root'), '--crash-reports', directory, '--out', str(out)])
            self.assertIn('**Verdict: INADMISSIBLE**', out.read_text())
            self.assertIn('| no_fail_closed | UNKNOWN |', out.read_text())


class Probe(unittest.TestCase):
    def probe(self, command, timeout='20'):
        out = io.StringIO()
        with redirect_stdout(out):
            code = soak.main(['probe', '--timeout', timeout, '--'] + command)
        return code, out.getvalue()

    def test_a_server_that_answers_initialize_passes(self):
        code, text = self.probe([str(HERE / 'fake_prism_mcp.py'), '--repo', '/r', '--shared'])
        self.assertEqual(code, 0, text)
        self.assertIn('PASS', text)

    def test_a_binary_that_rejects_shared_fails_and_says_why(self):
        script = 'import sys; sys.stderr.write("%s\\n"); sys.exit(2)' % CLAP
        code, text = self.probe([sys.executable, '-c', script])
        self.assertEqual(code, 1)
        self.assertIn('FAIL', text)
        self.assertIn(CLAP, text)

    def test_a_server_that_never_answers_fails_at_the_timeout(self):
        code, text = self.probe([sys.executable, '-c', 'import time; time.sleep(60)'], timeout='1')
        self.assertEqual(code, 1)
        self.assertIn('no answer to initialize within 1', text)


if __name__ == '__main__':
    unittest.main()
