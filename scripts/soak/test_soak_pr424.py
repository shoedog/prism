"""Tests of the fold of PR #424's review round 1 (gpt-6.1-sol 1-2, Opus W1, W2, S1-S5): a usage day
needs one sample interval of real daemon work (owner decision 2026-10-16), the repository and the
binary are read from every form a process table shows them in, and the wrapper survives a log it
cannot write.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import soak  # noqa: E402
from test_soak import MIB, proc, sample  # noqa: E402

DAY = 86400


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
        self.assertEqual(soak.usage_days(new), ['1970-01-01'])
        unseen = [sample(0, []), sample(60, [], ps_ok=False), sample(120, [daemon(0.6, pid=9)])]
        self.assertEqual(soak.usage_days(unseen), [])

    def test_the_report_states_the_interval_rule(self):
        summary = soak.summarize_samples([sample(0, [daemon(1.0)])])
        verdicts = soak.judge(summary, soak.scan_lines([]), [], {'logs': True, 'crash_reports': True})
        use = soak.usage('week', summary, soak.scan_lines([]), 2, 5)
        text = soak.render('week', summary, soak.scan_lines([]), [], verdicts, use, 'INADMISSIBLE', 0)
        self.assertIn('a sample interval with at least 0.05 s of one daemon', text)


if __name__ == '__main__':
    unittest.main()
