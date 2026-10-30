"""Owner request 2026-10-16: quiet residency growth, excluding warm-up and work."""
import io
import json
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

import soak
from test_soak import ALL_READ, KEY, MIB, proc, sample


def history(minutes=370, baseline=500, growth=0, work=False):
    """Ten minutes to baseline, then six hours of measured residency at 60 s cadence."""
    rows = []
    for minute in range(minutes + 1):
        fraction = max(0, minute - 10) / max(1, minutes - 10)
        cpu = max(0, minute - 10) * 0.1 if work else minute * 0.0006
        rows.append(sample(minute * 60, [proc('daemon', 1, int((baseline + growth * fraction) * MIB), cpu=cpu)]))
    return rows


def assess(rows):
    summary = soak.summarize_samples(rows)
    verdict = soak.judge(summary, soak.scan_lines([]), [], ALL_READ)['no_footprint_growth']
    return verdict, summary['footprint_trends'][0]


class FootprintGrowth(unittest.TestCase):
    def test_flat_resident_daemon_passes_with_trend(self):
        rows = history(growth=2)
        verdict, trend = assess(rows)
        self.assertEqual(verdict, 'PASS')
        self.assertEqual(trend['baseline_bytes'], 500 * MIB)
        self.assertEqual(trend['baseline_t'], 600)
        self.assertEqual(trend['max_bytes'], 502 * MIB)
        self.assertEqual(trend['last_bytes'], 502 * MIB)
        self.assertAlmostEqual(trend['growth_bytes_per_hour'], 2 * MIB / (370 / 60))
        self.assertEqual(trend['largest_quiet_growth_bytes'], 2 * MIB)

    def test_300_mib_over_six_quiet_hours_fails(self):
        verdict, trend = assess(history(growth=300))
        self.assertEqual(verdict, 'FAIL')
        self.assertEqual(trend['largest_quiet_growth_bytes'], 300 * MIB)
        self.assertEqual(trend['largest_quiet_secs'], 6 * 3600)

    def test_work_growth_then_flat_passes_and_is_reported(self):
        rows = history(growth=300, work=True)
        for minute in range(371, 432):
            rows.append(sample(minute * 60, [proc('daemon', 1, 800 * MIB, cpu=36.0)]))
        verdict, trend = assess(rows)
        self.assertEqual(verdict, 'PASS')
        self.assertEqual(trend['last_bytes'], 800 * MIB)
        self.assertGreater(trend['growth_bytes_per_hour'], 0)
        self.assertEqual(trend['largest_quiet_growth_bytes'], 0)

    def test_rebuild_resets_baseline_and_requires_ten_quiet_minutes(self):
        rows = history(minutes=100)
        for row in rows[20:]:
            row['processes'][0]['cpu_secs'] += 5 - 0.0006  # Exactly 5 s across the rebuild interval.
            row['processes'][0]['footprint_bytes'] = 900 * MIB
        verdict, trend = assess(rows)
        self.assertEqual(verdict, 'PASS')
        self.assertEqual((trend['baseline_t'], trend['baseline_bytes']), (1800, 900 * MIB))
        self.assertEqual(len(trend['baselines']), 2)

    def test_log_rebuild_resets_even_without_cpu_burst(self):
        rows = history(minutes=100)
        rows[20]['processes'][0]['rebuild_observed'] = True
        for row in rows[20:]:
            row['processes'][0]['footprint_bytes'] = 900 * MIB
        self.assertEqual(assess(rows)[0], 'PASS')
        self.assertEqual(assess(rows)[1]['baseline_t'], 1800)

    def test_just_below_rebuild_cpu_boundary_is_work(self):
        rows = history(minutes=100)
        for row in rows[20:]:
            row['processes'][0]['cpu_secs'] += 4.99
        self.assertEqual(assess(rows)[1]['baseline_t'], 600)

    def test_threshold_is_strict_and_uses_larger_of_absolute_and_relative(self):
        for baseline, threshold in ((500, 200), (1000, 250)):
            for delta, verdict in ((-1, 'PASS'), (0, 'PASS'), (1, 'FAIL')):
                with self.subTest(baseline=baseline, delta=delta):
                    rows = history(minutes=70, baseline=baseline, growth=threshold)
                    rows[-1]['processes'][0]['footprint_bytes'] += delta
                    self.assertEqual(assess(rows)[0], verdict)

    def test_quiet_hour_is_inclusive_and_short_stretch_is_report_only(self):
        rows = history(minutes=69, growth=300)
        self.assertEqual(assess(rows)[0], 'PASS')
        rows.append(sample(4200, [proc('daemon', 1, 800 * MIB, cpu=0.0414)]))
        self.assertEqual(assess(rows)[0], 'FAIL')

    def test_missing_footprints_are_unknown_including_baseline_and_last(self):
        for minute in (0, 10, 40, 70):
            with self.subTest(minute=minute):
                rows = history(minutes=70)
                rows[minute]['processes'][0].update(footprint_bytes=None, footprint_status='failed')
                self.assertEqual(assess(rows)[0], 'UNKNOWN')
        rows = history(minutes=70)
        for row in rows:
            row['processes'][0].update(footprint_bytes=None, footprint_status='failed')
        self.assertEqual(assess(rows)[0], 'UNKNOWN')
        self.assertIsNone(assess(rows)[1]['last_bytes'])

    def test_observed_breach_wins_over_missing_evidence_and_later_rebuild(self):
        rows = history(growth=300)
        rows.append(sample(22260, [proc('daemon', 1, status='failed', cpu=10)]))
        self.assertEqual(assess(rows)[0], 'FAIL')

    def test_no_baseline_contributes_nothing_but_still_reports(self):
        for rows in (history(minutes=9, growth=300), history(work=True, growth=300)):
            # The work fixture first rests for ten minutes; make it busy from the start.
            if len(rows) > 10:
                for i, row in enumerate(rows):
                    row['processes'][0]['cpu_secs'] = i * 0.1
            verdict, trend = assess(rows)
            self.assertEqual(verdict, 'PASS')
            self.assertIsNone(trend['baseline_bytes'])

    def test_unbuilt_daemon_cannot_establish_a_baseline(self):
        verdict, trend = assess(history(baseline=2))
        self.assertEqual(verdict, 'PASS')
        self.assertIsNone(trend['baseline_bytes'])

    def test_warmup_growth_is_excluded_and_quiet_begins_after_build(self):
        rows = history(minutes=70)
        for minute, row in enumerate(rows[:10]):
            row['processes'][0]['footprint_bytes'] = (200 + 30 * minute) * MIB
        self.assertEqual(assess(rows)[0], 'PASS')
        self.assertEqual(assess(rows)[1]['largest_quiet_growth_bytes'], 0)
        rows[0]['processes'][0]['footprint_bytes'] = 2 * MIB
        self.assertEqual(assess(rows)[1]['baseline_t'], 660)

    def test_just_under_usage_boundary_still_counts_as_quiet(self):
        rows = history(minutes=70, growth=300)
        for minute, row in enumerate(rows):
            row['processes'][0]['cpu_secs'] = minute * 0.049
        self.assertEqual(assess(rows)[0], 'FAIL')

    def test_missing_cpu_at_first_sample_cannot_start_warmup(self):
        rows = history(minutes=10)
        rows[0]['processes'][0]['cpu_secs'] = None
        self.assertIsNone(assess(rows)[1]['baseline_bytes'])

    def test_work_at_exact_usage_boundary_breaks_quiet_stretch(self):
        rows = history(minutes=70, growth=300)
        for row in rows[40:]:
            row['processes'][0]['cpu_secs'] += 0.05
        self.assertEqual(assess(rows)[0], 'PASS')

    def test_gaps_missing_cpu_and_disappearing_pid_cannot_certify_quiet(self):
        for mode in ('gap', 'cpu', 'pid', 'ps', 'rollback'):
            with self.subTest(mode=mode):
                rows = history(minutes=70, growth=300)
                if mode == 'gap':
                    rows = rows[:39] + rows[41:]
                elif mode == 'cpu':
                    rows[40]['processes'][0]['cpu_secs'] = None
                elif mode == 'rollback':
                    rows[40]['processes'][0]['cpu_secs'] = -1
                else:
                    rows[40]['processes'] = []
                    rows[40]['ps_ok'] = mode != 'ps'
                verdict, trend = assess(rows)
                self.assertEqual(verdict, 'UNKNOWN')
                self.assertLess(trend['largest_quiet_secs'], 3600)

    def test_pid_and_key_have_separate_baselines(self):
        for field, value in (('pid', 2), ('key', 'cd' * 32)):
            with self.subTest(field=field):
                rows = history(minutes=70)
                for row in rows[35:]:
                    row['processes'][0][field] = value
                    row['processes'][0]['footprint_bytes'] = 900 * MIB
                summary = soak.summarize_samples(rows)
                self.assertEqual(len(summary['footprint_trends']), 2)
                self.assertEqual(soak.judge(summary, soak.scan_lines([]), [], ALL_READ)['no_footprint_growth'], 'PASS')
                self.assertEqual([t['baseline_bytes'] for t in summary['footprint_trends']], [500 * MIB, 900 * MIB])

    def test_phase_two_reports_and_phase_three_gates(self):
        verdicts = {rule: 'PASS' for rule in soak.RULES}
        verdicts['no_footprint_growth'] = 'FAIL'
        self.assertEqual(soak.overall('round', verdicts, {'seen': True}), 'PASS')
        self.assertEqual(soak.overall('week', verdicts, {'seen': True}), 'FAIL')
        verdicts['no_footprint_growth'] = 'UNKNOWN'
        self.assertEqual(soak.overall('week', verdicts, {'seen': True}), 'INADMISSIBLE')

    def test_report_contains_every_daemon_and_phase_one_is_report_only(self):
        rows = history(minutes=70, growth=300)
        rows[-1]['processes'].append(proc('daemon', 2, 2 * MIB, cpu=0))
        with tempfile.TemporaryDirectory() as directory:
            source, md, data = [Path(directory) / name for name in ('samples.jsonl', 'report.md', 'report.json')]
            source.write_text(''.join(json.dumps(row) + '\n' for row in rows))
            with redirect_stdout(io.StringIO()):
                soak.main(['report', '--phase', 'fanout', '--samples', str(source), '--out', str(md),
                           '--json', str(data), '--crash-reports', directory])
            result = json.loads(data.read_text())
            self.assertEqual(result['verdict'], 'PASS')
            self.assertEqual(result['verdicts']['no_footprint_growth'], 'FAIL')
            self.assertEqual(len(result['summary']['footprint_trends']), 2)
            for text in ('no_footprint_growth | FAIL | no', 'Baseline', 'Max', 'Last', 'MiB/h', 'Quiet', KEY):
                self.assertIn(text, md.read_text())

    def test_sampler_reads_only_new_rebuild_lines_from_daemon_log(self):
        with tempfile.TemporaryDirectory() as directory:
            logs = Path(directory) / 'prism-mcpd'
            logs.mkdir()
            log = logs / (KEY[:16] + '.log')
            log.write_text('nav cache: CPG cache miss in /c, rebuilding\n')
            observer = soak.RebuildLogs(directory)
            p = proc('daemon', 1, 500 * MIB, cpu=0)
            observer.observe([p])  # Historical log content is not a new event.
            self.assertFalse(p['rebuild_observed'])
            with log.open('a') as out:
                out.write('nav cache: CPG cache miss in /c, rebuilding\n')
            observer.observe([p])
            self.assertTrue(p['rebuild_observed'])
            observer.observe([p])
            self.assertFalse(p['rebuild_observed'])
            with log.open('a') as out:
                out.write('nav cache: partial CPG cache hit in /c, rebuild')
            observer.observe([p])
            self.assertFalse(p['rebuild_observed'])
            with log.open('a') as out:
                out.write('ing\n')
            observer.observe([p])
            self.assertTrue(p['rebuild_observed'])

    def test_log_rotation_missing_logs_and_non_daemons(self):
        with tempfile.TemporaryDirectory() as directory:
            logs = Path(directory) / 'prism-mcpd'
            logs.mkdir()
            log = logs / (KEY[:16] + '.log')
            p = proc('daemon', 1)
            observer = soak.RebuildLogs(directory)
            observer.observe([p, proc('shim', 2)])
            self.assertFalse(p['rebuild_observed'])
            log.write_text('old\n')
            observer.observe([p])
            log.rename(log.with_suffix('.old'))
            log.write_text('nav cache: CPG cache miss in /c, rebuilding\n')
            observer.observe([p])
            self.assertTrue(p['rebuild_observed'])
            log.write_text('unrelated line\n')
            observer.observe([p])
            self.assertFalse(p['rebuild_observed'])

    def test_sample_command_attaches_log_events_without_status_connection(self):
        rows = history(minutes=0)
        observer = unittest.mock.Mock()
        args = type('Args', (), dict(duration=None, window_file=None, count=1, interval=60,
                                   status_bin=None, runtime_dir='/runtime', out=None))()
        with tempfile.TemporaryDirectory() as directory:
            args.out = str(Path(directory) / 'samples.jsonl')
            with patch.object(soak, 'RebuildLogs', return_value=observer), \
                    patch.object(soak, 'sample_once', return_value=rows[0]), redirect_stdout(io.StringIO()):
                soak.cmd_sample(args)
            observer.observe.assert_called_once_with(rows[0]['processes'])


if __name__ == '__main__':
    unittest.main()
