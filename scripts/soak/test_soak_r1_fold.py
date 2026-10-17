"""PR #440 round 1: rebuild epochs, provenance, duration and exact latency bounds."""
import io
import json
import os
from pathlib import Path
import tempfile
import time
import unittest
from contextlib import redirect_stdout
from unittest.mock import patch

import fanout
import soak
from test_fanout import arm
from test_soak import MIB, proc, sample
from test_soak_owner_decisions import DAY, history, report

EPOCH = 1791849600  # 2026-10-13 00:00 UTC


class RebuildEpoch(unittest.TestCase):
    def setUp(self):
        self.timezone = patch.dict(os.environ, {'TZ': 'UTC'})
        self.timezone.start()
        time.tzset()
        self.addCleanup(time.tzset)
        self.addCleanup(self.timezone.stop)

    def rows(self, days=3, mode=None):
        rows = history(days)
        for row in rows:
            if mode and row['t'] >= 3 * DAY:
                row['processes'][0]['footprint_bytes'] = 900 * MIB
                if mode == 'cpu':
                    row['processes'][0]['cpu_secs'] += 5
                else:
                    row['processes'][0]['rebuild_observed'] = row['t'] == 3 * DAY
            row['t'] += EPOCH
        return rows

    def test_late_cpu_rebuild_needs_a_current_baseline(self):
        found, _ = report(self.rows(mode='cpu'))
        trend = found['summary']['footprint_trends'][0]
        self.assertIsNone(trend['baseline_bytes'])
        self.assertEqual(found['verdicts']['no_footprint_growth'], 'UNKNOWN')
        self.assertEqual(found['verdict'], 'INADMISSIBLE')

    def test_late_log_rebuild_needs_a_current_baseline(self):
        found, _ = report(self.rows(mode='log'))
        self.assertIsNone(found['summary']['footprint_trends'][0]['baseline_bytes'])
        self.assertEqual(found['verdicts']['no_footprint_growth'], 'UNKNOWN')
        self.assertEqual(found['verdict'], 'INADMISSIBLE')

    def test_new_baseline_cannot_borrow_earlier_nights(self):
        for mode in ('cpu', 'log'):
            with self.subTest(mode=mode):
                rows = self.rows(mode=mode)
                last, cpu = rows[-1]['t'], rows[-1]['processes'][0]['cpu_secs']
                for minute in range(1, 11):
                    rows.append(sample(last + minute * 60, [proc('daemon', 1, 900 * MIB, cpu=cpu)]))
                found, _ = report(rows)
                trend = found['summary']['footprint_trends'][0]
                self.assertEqual(trend['baseline_t'], last + 600)
                self.assertEqual(trend['quiet_nights'], [])
                self.assertEqual(len(trend['baselines']), 2)
                self.assertEqual(found['verdicts']['no_footprint_growth'], 'UNKNOWN')

    def test_two_nights_after_new_baseline_pass(self):
        self.assertEqual(report(self.rows())[0]['verdict'], 'PASS')
        for mode in ('cpu', 'log'):
            with self.subTest(mode=mode):
                found, _ = report(self.rows(5, mode))
                self.assertEqual(found['verdict'], 'PASS')
                self.assertEqual(found['summary']['footprint_trends'][0]['quiet_nights'],
                                 ['2026-10-15', '2026-10-16', '2026-10-17'])

    def test_historical_breach_survives_rebuild_and_fresh_nights(self):
        rows = self.rows(5, 'log')
        for row in rows[11:3 * DAY // 60]:
            row['processes'][0]['footprint_bytes'] = 800 * MIB
        found, _ = report(rows)
        self.assertEqual(found['verdicts']['no_footprint_growth'], 'FAIL')

    def test_measurement_hole_survives_rebuild_and_fresh_nights(self):
        rows = self.rows(5, 'log')
        rows[30]['processes'][0].update(footprint_bytes=None, footprint_status='failed')
        found, _ = report(rows)
        self.assertEqual(found['verdicts']['no_footprint_growth'], 'UNKNOWN')


class CleanCorpus(unittest.TestCase):
    def test_compare_requires_confirmed_cleanliness_on_both_arms(self):
        for side in ('baseline', 'candidate'):
            for dirty in (0, 1, None, 'missing'):
                with self.subTest(side=side, dirty=dirty):
                    baseline, candidate = [arm(10, 20)], [arm(10, 20)]
                    corpus = (baseline if side == 'baseline' else candidate)[0]['run']['corpus']
                    if dirty == 'missing':
                        del corpus['dirty']
                    else:
                        corpus['dirty'] = dirty
                    found = fanout.judge(baseline, candidate)
                    self.assertEqual(found['pass'], dirty == 0)
                    if dirty != 0:
                        self.assertIn('corpus', ' '.join(found['problems']))

    def test_failed_git_status_provenance_cannot_pass_comparison(self):
        from memreport import corpus_provenance
        with patch('memreport.capture', side_effect=[None, 'a' * 40]):
            provenance = corpus_provenance('/corpus')
        self.assertIsNone(provenance['dirty'])
        baseline, candidate = arm(10, 20), arm(10, 20)
        baseline['run']['corpus']['commit'] = provenance['commit']
        candidate['run']['corpus'].update(provenance)
        self.assertFalse(fanout.judge([baseline], [candidate])['pass'])


class SamplingSpan(unittest.TestCase):
    def setUp(self):
        self.timezone = patch.dict(os.environ, {'TZ': 'UTC'})
        self.timezone.start()
        time.tzset()
        self.addCleanup(time.tzset)
        self.addCleanup(self.timezone.stop)

    def test_documented_sample_command_supplies_a_full_phase_three_report(self):
        for window in (False, True):
            with self.subTest(window=window), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                source, data, log = [root / name for name in ('samples', 'report.json', 'client.log')]
                log.touch()
                clock = [EPOCH + 0.25]

                def acquire(*args):
                    t = clock[0]
                    elapsed = t - EPOCH
                    cpu = 0.05 * (elapsed >= 43200) + 0.05 * (elapsed >= DAY + 43200)
                    row = sample(t, [proc('daemon', 1, 500 * MIB, cpu=cpu)])
                    clock[0] += 0.125  # Acquisition overhead is outside time.sleep.
                    return row

                def sleep(seconds):
                    clock[0] += seconds

                window_args = ['--window-file', str(root / 'T0')] if window else []
                with patch.object(soak.time, 'time', side_effect=lambda: clock[0]), \
                        patch.object(soak.time, 'sleep', side_effect=sleep), \
                        patch.object(soak, 'sample_once', side_effect=acquire), \
                        redirect_stdout(io.StringIO()):
                    soak.main(['sample', '--out', str(source), '--interval', '60',
                               '--duration', '259200'] + window_args)
                rows, skipped = soak.load_samples(source)
                self.assertEqual(skipped, 0)
                with redirect_stdout(io.StringIO()):
                    soak.main(['report', '--phase', 'week', '--samples', str(source),
                               '--out', str(root / 'report.md'), '--json', str(data),
                               '--client-log', str(log), '--crash-reports', directory,
                               '--until', str(clock[0])] + window_args)
                found = json.loads(data.read_text())
                self.assertTrue(all(v == 'PASS' for v in found['verdicts'].values()),
                                found['verdicts'])
                self.assertTrue(all(found['use'].values()), found['use'])
                self.assertEqual(found['verdict'], 'PASS')
                self.assertGreaterEqual(rows[-1]['t'] - rows[0]['t'], 3 * DAY)
                self.assertLess(rows[-2]['t'] - rows[0]['t'], 3 * DAY)

    def test_resume_uses_the_original_first_observation_and_keeps_t0(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, window = root / 'samples', root / 'T0'
            window.write_text('1000\n')
            # Pre-window samples and interrupted JSON must not choose the span anchor.
            source.write_text(json.dumps(sample(0, [])) + '\n{broken\n' +
                              json.dumps(sample(1000.25, [])) + '\n')
            clock = [1060.25]

            def acquire(*args):
                row = sample(clock[0], [])
                clock[0] += 0.25
                return row

            def sleep(seconds):
                clock[0] += seconds

            with patch.object(soak.time, 'time', side_effect=lambda: clock[0]), \
                    patch.object(soak.time, 'sleep', side_effect=sleep), \
                    patch.object(soak, 'sample_once', side_effect=acquire), redirect_stdout(io.StringIO()):
                soak.main(['sample', '--out', str(source), '--window-file', str(window),
                           '--duration', '120', '--interval', '60'])
            rows, skipped = soak.load_samples(source, since=1000)
            self.assertEqual(skipped, 1)
            self.assertEqual([row['t'] for row in rows], [1000.25, 1060.25, 1120.5])
            self.assertEqual(window.read_text(), '1000\n')

    def test_terminal_sample_uses_observed_time_including_empty_resume(self):
        for window in (False, True):
            for duration, overhead in ((60.2, 0.125), (120, 0)):
                with self.subTest(window=window, duration=duration), tempfile.TemporaryDirectory() as directory:
                    root = Path(directory)
                    source = root / 'samples'
                    source.touch()
                    (root / 'T0').write_text('900\n')
                    clock = [1000.25]

                    def acquire(*args):
                        row = sample(clock[0], [])
                        clock[0] += overhead
                        return row

                    def sleep(seconds):
                        clock[0] += seconds

                    with patch.object(soak.time, 'time', side_effect=lambda: clock[0]), \
                            patch.object(soak.time, 'sleep', side_effect=sleep), \
                            patch.object(soak, 'sample_once', side_effect=acquire), \
                            redirect_stdout(io.StringIO()):
                        soak.main(['sample', '--out', str(source), '--interval', '60',
                                   '--duration', str(duration)] +
                                  (['--window-file', str(root / 'T0')] if window else []))
                    rows, _ = soak.load_samples(source)
                    self.assertEqual(len(rows), 3)
                    self.assertGreaterEqual(rows[-1]['t'] - rows[0]['t'], duration)
                    self.assertLess(rows[-2]['t'] - rows[0]['t'], duration)


class ExactRatios(unittest.TestCase):
    def test_comparison_uses_unrounded_ratios_at_each_bound(self):
        for p95, p99, expected in ((110, 250, True), (110.001, 250, False),
                                   (110, 250.001, False), (110.001, 250.001, False)):
            with self.subTest(p95=p95, p99=p99):
                found = fanout.judge([arm(100, 200)], [arm(p95, p99)])
                self.assertEqual(found['problems'], [])
                self.assertEqual(found['pass'], expected)
                self.assertEqual(found['p95_ratio'], p95 / 100)
                self.assertEqual(found['p99_ratio'], p99 / 200)
                rendered = fanout.render([arm(100, 200)], [arm(p95, p99)], found)
                self.assertIn('Ratios: p95 1.1 (at most 1.10), p99 1.25 (at most 1.25)', rendered)


if __name__ == '__main__':
    unittest.main()
