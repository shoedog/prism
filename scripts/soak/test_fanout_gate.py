"""Tests of the fan-out gate's admissibility (review round 1: sol 3, sol 4, Opus W1, W8, S6): a
gating run needs exactly one successful refresh, a critical or unreadable pressure reading
invalidates a run, the refresh's overlap with other clients' calls is measured, and a comparison
never crashes on a run without calls.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import io
import json
import os
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import fanout  # noqa: E402
from test_fanout import RunCase, arm, dead_pid, run_info  # noqa: E402


def call(client, start, ms, outcome='ok', refresh=False, seq=0):
    return {'client': client, 'seq': seq, 'tool': 'refresh_index' if refresh else 't', 'refresh': refresh,
            'start': start, 'ms': ms, 'outcome': outcome}


class Refresh(unittest.TestCase):
    def gate(self, records):
        return fanout.summarize(records, run_info(clients=1, requests=2, refresh_at=0.5))

    def test_a_gating_run_needs_exactly_one_successful_refresh(self):
        calls = [call(0, 0.0, 1.0), call(0, 0.1, 1.0, seq=1)]
        self.assertTrue(self.gate(calls + [call(0, 0.05, 5.0, refresh=True)])['admissible'])
        self.assertFalse(self.gate(calls)['admissible'])
        for outcome in ('retryable', 'rpc_error', 'tool_error'):
            summary = self.gate(calls + [call(0, 0.05, 5.0, outcome, refresh=True)])
            self.assertFalse(summary['admissible'], outcome)
            self.assertIn('refresh', ' '.join(summary['problems']))
        two = calls + [call(0, 0.05, 5.0, refresh=True), call(0, 0.07, 5.0, refresh=True)]
        self.assertFalse(self.gate(two)['admissible'])

    def test_a_run_without_a_refresh_is_diagnostic_and_never_compared(self):
        diagnostic = fanout.summarize([call(0, 0.0, 1.0), call(0, 0.1, 1.0, seq=1)], run_info(clients=1))
        self.assertTrue(diagnostic['admissible'])
        self.assertEqual(diagnostic['profile'], 'diagnostic')
        quiet = arm(10.0, 20.0)
        quiet['run']['refresh_at'] = None
        verdict = fanout.judge([arm(10.0, 20.0)], [quiet])
        self.assertFalse(verdict['pass'])
        self.assertIn('diagnostic', ' '.join(verdict['problems']))

    def test_the_calls_that_overlap_the_refresh_are_counted_and_their_longest_reported(self):
        records = [call(0, 1.0, 500.0, refresh=True),
                   call(1, 0.9, 400.0), call(2, 1.2, 50.0), call(3, 1.6, 9.0), call(1, 0.1, 99.0, seq=1),
                   call(0, 0.5, 300.0, seq=1), call(0, 1.1, 50.0, seq=2)]
        overlap = fanout.refresh_overlap(records)
        self.assertEqual(overlap, {'calls': 2, 'max_ms': 400.0, 'refresh_ms': 500.0})
        self.assertEqual(fanout.refresh_overlap([call(1, 0.0, 1.0)]), {'calls': 0, 'max_ms': None,
                                                                         'refresh_ms': None})


class Compare(unittest.TestCase):
    def test_a_run_with_no_calls_fails_the_comparison_instead_of_crashing_it(self):
        empty = arm(None, None, admissible=False)
        verdict = fanout.judge([arm(10.0, 20.0)], [empty])
        self.assertFalse(verdict['pass'])
        self.assertIn('**Verdict: FAIL**', fanout.render([arm(10.0, 20.0)], [empty], verdict))
        three = fanout.judge([arm(10.0, 20.0)] * 3, [arm(10.0, 20.0), empty, empty])
        self.assertFalse(three['pass'])

    def test_a_calibration_passes_only_when_both_directions_pass(self):
        # Review round 2 (Opus W-A): a 30 % spread between two identical arms passes one way only.
        slow, fast = [arm(100.0, 200.0)], [arm(70.0, 140.0)]
        self.assertTrue(fanout.judge(slow, fast)['pass'])
        self.assertFalse(fanout.judge(fast, slow)['pass'])
        self.assertFalse(fanout.calibrate(slow, fast)['pass'])
        self.assertFalse(fanout.calibrate(fast, slow)['pass'])
        self.assertTrue(fanout.calibrate([arm(100.0, 200.0)], [arm(105.0, 210.0)])['pass'])

    def test_runs_of_other_client_counts_or_refresh_points_are_not_compared(self):
        other = arm(10.0, 20.0)
        other['run']['clients'] = 4
        self.assertIn('clients', ' '.join(fanout.judge([arm(10.0, 20.0)], [other])['problems']))
        allowed = fanout.judge([arm(10.0, 20.0)], [other], allow_client_mismatch=True)
        self.assertTrue(allowed['pass'], allowed['problems'])
        later = arm(10.0, 20.0)
        later['run']['refresh_at'] = 0.9
        self.assertFalse(fanout.judge([arm(10.0, 20.0)], [later])['pass'])


class GateRuns(RunCase):
    def test_a_refresh_answered_with_a_tool_error_makes_the_run_inadmissible(self):
        os.environ['FAKE_REFRESH'] = 'tool_error'
        _, _, summary, _ = self.run_fanout('refresh-error', 'standalone', clients=2)
        self.assertFalse(summary['admissible'])

    def test_a_critical_pressure_reading_stops_and_invalidates_the_run(self):
        readings = iter([1, 1, 4])
        fanout.read_pressure = lambda: next(readings, 4)
        saved = fanout.PRESSURE_INTERVAL
        fanout.PRESSURE_INTERVAL = 0.01
        self.addCleanup(lambda: setattr(fanout, 'PRESSURE_INTERVAL', saved))
        os.environ['FAKE_DELAY_MS'] = '20'
        _, _, summary, run = self.run_fanout('critical', 'standalone', clients=2, extra=['--pressure', 'off'])
        self.assertFalse(summary['admissible'])
        self.assertIn('critical memory pressure', ' '.join(summary['problems']))
        self.assertEqual(run['pressure']['max'], 4)
        self.assertTrue(run['failures'], 'the clients were not stopped')
        self.assertLess(summary['calls'], 12)

    def test_an_unavailable_pressure_reading_invalidates_a_run_that_requires_it(self):
        fanout.read_pressure = lambda: None
        _, _, required, _ = self.run_fanout('unread', 'standalone', clients=1, extra=['--pressure', 'required'])
        self.assertFalse(required['admissible'])
        self.assertIn('pressure', ' '.join(required['problems']))
        _, _, off, _ = self.run_fanout('unread-off', 'standalone', clients=1, extra=['--pressure', 'off'])
        self.assertTrue(off['admissible'], off['problems'])

    def test_a_shared_run_keeps_its_daemon_logs(self):
        os.environ['FAKE_DAEMON_PID'] = str(dead_pid())
        out, _, _, _ = self.run_fanout('logs', 'shared', clients=2)
        self.assertEqual((out / 'daemon-logs' / 'fake.log').read_text().count('fake daemon log'), 2)

    def test_every_tool_of_the_workload_is_warmed_before_the_barrier(self):
        out, calls, summary, _ = self.run_fanout('warm', 'standalone', clients=2)
        self.assertTrue(summary['admissible'], summary['problems'])
        tools = len({c['tool'] for c in calls if not c['refresh']})
        timed = len([c for c in calls if c['client'] == 1])
        self.assertIn('fake: %d tool calls' % (tools + timed), (out / 'clients' / '1.stderr').read_text())


if __name__ == '__main__':
    unittest.main()
