"""PR #424 round 2 and owner decisions of 2026-10-16: measured endpoints, 120 s cap.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import soak  # noqa: E402
from test_soak import sample  # noqa: E402
from test_soak_pr424 import DAY, daemon  # noqa: E402


def report(samples):
    """Read serialized evidence through the CLI and return JSON and Markdown reports."""
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        source, data, markdown = root / 'samples.jsonl', root / 'report.json', root / 'report.md'
        source.write_text(''.join(json.dumps(s) + '\n' for s in samples))
        client = root / 'client.log'
        client.write_text('')
        with redirect_stdout(io.StringIO()):
            soak.main(['report', '--phase', 'week', '--samples', str(source), '--client-log', str(client),
                       '--crash-reports', directory, '--out', str(markdown), '--json', str(data)])
        return json.loads(data.read_text()), markdown.read_text()


def week(first_cpu, later_growth):
    """Seven uninterrupted days; a fresh daemon lives for two samples on five days."""
    samples = []
    for minute in range(7 * DAY // 60 + 1):
        day, within = divmod(minute, DAY // 60)
        processes = []
        if day < 5 and within in (1, 2):
            processes = [daemon(first_cpu + (later_growth if within == 2 else 0.0), pid=100 + day)]
        samples.append(sample(minute * 60, processes))
    return samples


class MeasuredUsage(unittest.TestCase):
    def test_first_sighting_cpu_cannot_pass_a_week(self):
        for growth in (0.0, 0.01):
            with self.subTest(later_growth=growth):
                found, text = report(week(0.06, growth))
                self.assertEqual(found['summary']['usage_days'], [])
                self.assertEqual(found['verdict'], 'INADMISSIBLE')
                self.assertEqual(found['summary']['largest_gap_secs'], 60)
                self.assertEqual(found['summary']['covered_secs'], 7 * DAY)
                self.assertIn('**Verdict: INADMISSIBLE**', text)

    def test_observed_zero_to_point_zero_six_counts_usage_but_short_lived_daemons_need_quiet_nights(self):
        found, text = report(week(0.0, 0.06))
        self.assertEqual(found['summary']['usage_days'], ['1970-01-%02d' % d for d in range(1, 6)])
        self.assertTrue(found['use']['usage_days_at_least_2'])
        self.assertEqual(found['verdicts']['no_footprint_growth'], 'UNKNOWN')
        self.assertEqual(found['verdict'], 'INADMISSIBLE')
        self.assertIn('**Verdict: INADMISSIBLE**', text)

    def test_a_121_second_interval_has_no_credit_and_is_reported(self):
        found, text = report([sample(0, [daemon(0.0)]), sample(121, [daemon(0.6)])])
        self.assertEqual(found['summary']['usage_days'], [])
        self.assertEqual(found['summary']['usage_intervals_excluded'], 1)
        self.assertEqual(found['summary']['largest_gap_secs'], 121)
        self.assertEqual(found['summary']['samples'], 2)
        self.assertIn('| Intervals excluded from usage credit (> 120 s) | 1 |', text)

    def test_a_120_second_interval_counts_and_is_not_excluded(self):
        found, text = report([sample(0, [daemon(0.0)]), sample(120, [daemon(0.06)])])
        self.assertEqual(found['summary']['usage_days'], ['1970-01-01'])
        self.assertEqual(found['summary']['usage_intervals_excluded'], 0)
        self.assertIn('| Intervals excluded from usage credit (> 120 s) | 0 |', text)

    def test_exclusions_count_intervals_even_without_daemons_and_keep_the_next_baseline(self):
        samples = [sample(0, []), sample(121, []), sample(242, [daemon(0.6)]),
                   sample(302, [daemon(0.66)])]
        found, text = report(samples)
        self.assertEqual(found['summary']['usage_days'], ['1970-01-01'])
        self.assertEqual(found['summary']['usage_intervals_excluded'], 2)
        self.assertIn('| Intervals excluded from usage credit (> 120 s) | 2 |', text)

    def test_both_endpoints_need_cpu_readings(self):
        for before, after in ((None, 0.6), (0.0, None)):
            with self.subTest(before=before, after=after):
                self.assertEqual(soak.usage_days([sample(0, [daemon(before)]), sample(60, [daemon(after)])]), [])

    def test_no_intervals_means_no_exclusions(self):
        for samples in ([], [sample(0, [])]):
            with self.subTest(samples=samples):
                self.assertEqual(soak.summarize_samples(samples)['usage_intervals_excluded'], 0)
