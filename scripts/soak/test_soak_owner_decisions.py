"""2026-10-16: three days, two nights, and per-rule repair without phase restart."""
import io
import json
import os
from pathlib import Path
import tempfile
import time
import unittest
from contextlib import redirect_stderr, redirect_stdout
from unittest.mock import patch

import soak
from soak_window import night_overlaps, observed_nights
from test_soak import MIB, REACHED, proc, sample

DAY = 86400


def history(days=3, used=(0, 1)):
    rows = []
    cpu = 0.0
    for minute in range(days * 1440 + 1):
        day, within = divmod(minute, 1440)
        if day in used and within == 720:
            cpu += 0.05
        rows.append(sample(minute * 60, [proc('daemon', 1, 500 * MIB, cpu=cpu),
                                        proc('shim', 2), proc('shim', 3)]))
    return rows


def two_nights():
    """72 h with exactly two sampled/quiet nights; daemon retires before later gaps."""
    rows = history()
    for s in rows:
        s['processes'][0]['cpu_secs'] = 0.05 * (s['t'] >= 43200) + 0.05 * (s['t'] >= DAY + 18000)
        if s['t'] > DAY + 21600:
            s['processes'] = s['processes'][1:]
    return [s for s in rows if (s['t'] <= DAY + 21600 or s['t'] == 3 * DAY
            or 6 <= time.localtime(s['t']).tm_hour < 22)]


def report(rows, fixes=(), phase='week', lines=(), crashes=(), extra=(), claude_lines=()):
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        source, md, data, log = [root / n for n in ('samples', 'report.md', 'report.json', 'client.log')]
        source.write_text(''.join(json.dumps(s) + '\n' for s in rows))
        log.write_text('\n'.join('%s 99 %s' % (time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(t)), line)
                                 for t, line in lines))
        for t in crashes:
            crash = root / ('prism-mcp-%s.ips' % t)
            crash.touch()
            os.utime(crash, (t, t))
        args = ['report', '--phase', phase, '--samples', str(source), '--out', str(md),
                '--json', str(data), '--client-log', str(log), '--crash-reports', directory]
        if claude_lines:
            claude = root / 'claude'
            path = claude / 'project/mcp-logs-prism/events.jsonl'
            path.parent.mkdir(parents=True)
            path.write_text(''.join(json.dumps({
                'timestamp': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(t)),
                'error': 'Server stderr: ' + line}) + '\n' for t, line in claude_lines))
            args += ['--claude-logs', str(claude)]
        if phase == 'round':
            args += ['--round-repo', '/r']
        for fix in fixes:
            args += ['--fix-at', fix]
        args += list(extra)
        with redirect_stdout(io.StringIO()):
            soak.main(args)
        return json.loads(data.read_text()), md.read_text()


class OwnerDecisions(unittest.TestCase):
    def setUp(self):
        self.timezone = patch.dict(os.environ, {'TZ': 'UTC'})
        self.timezone.start()
        time.tzset()

    def tearDown(self):
        self.timezone.stop()
        time.tzset()

    def test_three_days_two_usage_days_are_admissible(self):
        found, md = report(two_nights())
        self.assertEqual(found['verdict'], 'PASS')
        self.assertEqual(found['summary']['covered_secs'], 72 * 3600)
        self.assertEqual(len(found['summary']['local_nights']), 2)
        self.assertEqual(len(found['summary']['footprint_trends'][0]['quiet_nights']), 2)
        self.assertIn('Local nights', md)
        self.assertEqual(found['summary']['largest_gap_secs'], 8 * 3600 + 60)

    def test_less_than_72_hours_is_inadmissible(self):
        self.assertEqual(report(history()[:-1])[0]['verdict'], 'INADMISSIBLE')

    def test_only_one_observed_night_is_inadmissible(self):
        rows = [s for s in history() if (DAY - 7200 <= s['t'] <= DAY + 6 * 3600
                or 6 <= time.localtime(s['t']).tm_hour < 22 or s['t'] in (0, 3 * DAY))]
        found, _ = report(rows)
        self.assertEqual(found['verdict'], 'INADMISSIBLE')
        self.assertFalse(all(found['use'].values()))

    def test_one_usage_day_is_inadmissible(self):
        self.assertEqual(report(history(used=(0,)))[0]['verdict'], 'INADMISSIBLE')

    def test_only_one_quiet_stretch_is_unknown(self):
        rows = history()
        for s in rows:
            if s['t'] >= 100 * 60:
                s['processes'][0]['cpu_secs'] = s['t'] / 60 * 0.1
        self.assertEqual(report(rows)[0]['verdicts']['no_footprint_growth'], 'UNKNOWN')

    def test_daytime_quiet_stretches_do_not_count_as_nights(self):
        rows = history()
        cpu = 0.0
        for s in rows:
            if time.localtime(s['t']).tm_hour >= 22 or time.localtime(s['t']).tm_hour < 6:
                cpu += 0.1
            s['processes'][0]['cpu_secs'] = cpu
        self.assertEqual(report(rows)[0]['verdicts']['no_footprint_growth'], 'UNKNOWN')

    def test_each_daemon_needs_two_quiet_nights(self):
        rows = history()
        for s in rows[:100]:
            s['processes'].append(proc('daemon', 7, 500 * MIB, key='other', cpu=0))
        self.assertEqual(report(rows)[0]['verdicts']['no_footprint_growth'], 'UNKNOWN')

    def test_no_daemon_cannot_certify_quiet_nights(self):
        rows = history()
        for row in rows:
            row['processes'] = []
        self.assertEqual(report(rows)[0]['verdicts']['no_footprint_growth'], 'UNKNOWN')

    def test_fixed_rule_passes_with_post_fix_usage_and_keeps_history(self):
        rows = history(4, used=(1, 2))
        rows[10]['processes'].append(proc('daemon', 7, 500 * MIB, cpu=0))
        found, md = report(rows, ['one_index_per_key=86400'])
        self.assertEqual(found['verdicts']['one_index_per_key'], 'PASS')
        self.assertEqual(found['summary']['max_daemons_per_key'], 2)
        self.assertIn('one_index_per_key=86400', md)

    def test_too_little_post_fix_usage_is_unknown(self):
        found, _ = report(history(), ['one_index_per_key=86400'])
        self.assertEqual(found['verdicts']['one_index_per_key'], 'UNKNOWN')
        self.assertEqual(found['verdict'], 'INADMISSIBLE')

    def test_other_rules_earlier_evidence_survives(self):
        found, _ = report(history(4, used=(1, 2)), ['one_index_per_key=86400'],
                          lines=[(60, 'error: old startup failure')])
        self.assertEqual(found['verdicts']['no_startup_error'], 'FAIL')
        self.assertEqual(found['verdicts']['one_index_per_key'], 'PASS')

    def test_log_and_crash_evidence_are_filtered_only_for_fixed_rule(self):
        for rule, lines, crashes in (
                ('no_startup_error', [(60, 'error: broken')], ()),
                ('no_fail_closed', [(60, 'prism-mcp: --shared failed closed')], ()),
                ('no_abort', [], (60,))):
            with self.subTest(rule=rule):
                found, _ = report(history(4, used=(1, 2)), [rule + '=86400'], lines=lines, crashes=crashes)
                self.assertEqual(found['verdicts'][rule], 'PASS')

    def test_post_fix_breach_still_fails_even_without_usage(self):
        found, _ = report(history(), ['no_startup_error=172800'], lines=[(172860, 'error: still broken')])
        self.assertEqual(found['verdicts']['no_startup_error'], 'FAIL')

    def test_claude_log_cutoff_excludes_old_failures_but_keeps_new_ones(self):
        for stamp, expected in ((60, 'PASS'), (DAY + 60, 'FAIL')):
            with self.subTest(stamp=stamp):
                found, _ = report(history(used=(1, 2)), ['no_startup_error=86400'],
                                  claude_lines=[(stamp, 'error: broken')])
                self.assertEqual(found['verdicts']['no_startup_error'], expected)
                self.assertEqual(found['events']['startup_errors'], ['error: broken'])

    def test_growth_fix_needs_two_post_fix_nights(self):
        for fix, expected in ((DAY, 'PASS'), (2 * DAY, 'UNKNOWN')):
            with self.subTest(fix=fix):
                found, _ = report(history(used=(1, 2)), ['no_footprint_growth=%s' % fix])
                self.assertEqual(found['verdicts']['no_footprint_growth'], expected)

    def test_growth_failure_before_fix_is_cleared_only_with_new_nights(self):
        rows = history(used=(1, 2))
        for s in rows[:DAY // 60]:
            s['processes'][0]['footprint_bytes'] += min(300, max(0, s['t'] // 60 - 10) * 5) * MIB
        self.assertEqual(report(rows)[0]['verdicts']['no_footprint_growth'], 'FAIL')
        found, _ = report(rows, ['no_footprint_growth=86400'])
        self.assertEqual(found['verdict'], 'PASS')
        self.assertEqual(len(found['fix_windows']['no_footprint_growth']['summary']['usage_days']), 2)

    def test_two_usage_days_cannot_replace_two_post_fix_quiet_nights(self):
        rows = history(used=(1, 2))
        for s in rows:
            if s['t'] >= DAY + 6 * 3600:
                s['processes'][0]['cpu_secs'] = s['t'] / 600
        found, _ = report(rows, ['no_footprint_growth=86400'])
        self.assertGreaterEqual(len(found['fix_windows']['no_footprint_growth']['summary']['usage_days']), 2)
        self.assertEqual(found['verdicts']['no_footprint_growth'], 'UNKNOWN')

    def test_repeated_rule_uses_latest_fix_and_boundary_events_are_excluded(self):
        found, _ = report(history(used=(1, 2)), ['no_startup_error=86400', 'no_startup_error=0'],
                          lines=[(DAY, 'error: at boundary')])
        self.assertEqual(found['verdicts']['no_startup_error'], 'PASS')
        self.assertEqual(found['fix_windows']['no_startup_error']['fix_at'], DAY)

    def test_post_fix_usage_cannot_borrow_pre_fix_endpoint(self):
        found, _ = report(history(used=(1, 2)), ['one_index_per_key=%d' % (DAY + 43200 - 30)])
        self.assertEqual(found['verdicts']['one_index_per_key'], 'UNKNOWN')

    def test_standalone_new_in_phase_does_not_become_preexisting_at_fix(self):
        rows = history(days=0) + history(days=0)
        rows[1]['t'] = 60
        rows[1]['processes'].append(proc('standalone', 7, 500 * MIB))
        found, _ = report(rows, ['no_new_standalone_index=30'], phase='round', lines=[(60, REACHED)])
        self.assertEqual(found['verdicts']['no_new_standalone_index'], 'FAIL')

    def test_round_fixed_rule_can_pass_using_fresh_round_evidence(self):
        rows = history(days=0) + [sample(60, history(days=0)[0]['processes'])]
        found, _ = report(rows, ['no_startup_error=30'], phase='round',
                          lines=[(0, 'error: broken'), (60, REACHED)])
        self.assertEqual(found['verdict'], 'PASS')

    def test_night_sampling_needs_an_hour_and_does_not_credit_gaps(self):
        rows = [sample(t, []) for t in range(0, 3601, 60)]
        self.assertEqual(len(observed_nights(rows, 120)), 1)
        self.assertEqual(observed_nights(rows[:-1], 120), [])
        self.assertEqual(observed_nights([rows[0], rows[-1]], 120), [])
        rows[20]['ps_ok'] = False
        self.assertEqual(observed_nights(rows, 120), [])

    def test_quiet_night_hour_is_inclusive_and_distinct_from_daytime(self):
        rows = history()
        cpu = 0.0
        for s in rows:
            hour_minute = s['t'] % DAY // 60
            if not 61 <= hour_minute <= 120:
                cpu += 0.1
            s['processes'][0]['cpu_secs'] = cpu
        # The warm-up takes the first ten minutes on day zero; later nights have the baseline.
        self.assertEqual(report(rows)[0]['verdicts']['no_footprint_growth'], 'PASS')
        for s in rows:
            if s['t'] % DAY == 120 * 60:
                s['processes'][0]['cpu_secs'] += 0.05
        self.assertEqual(report(rows)[0]['verdicts']['no_footprint_growth'], 'UNKNOWN')

    def test_local_timezone_and_dst_define_nights(self):
        with patch.dict(os.environ, {'TZ': 'America/Denver'}):
            time.tzset()
            start = time.mktime((2026, 10, 31, 22, 0, 0, 0, 0, -1))
            end = time.mktime((2026, 11, 1, 6, 0, 0, 0, 0, -1))
            self.assertEqual(list(night_overlaps(start, end)), [('2026-10-31', 9 * 3600)])
        time.tzset()

    def test_round_fix_requires_round_use_again(self):
        rows = history(days=0) + [sample(60, history(days=0)[0]['processes'])]
        found, _ = report(rows, ['no_startup_error=30'], phase='round', lines=[(0, REACHED)])
        self.assertEqual(found['verdicts']['no_startup_error'], 'UNKNOWN')

    def test_no_post_fix_samples_is_unknown(self):
        found, _ = report(history(), ['one_index_per_key=999999'])
        self.assertEqual(found['verdicts']['one_index_per_key'], 'UNKNOWN')

    def test_fix_input_is_validated(self):
        for fix in ('bogus=1', 'no_abort=nan', 'no_abort=inf', 'no_abort=-1', 'no_abort=bad', 'no_abort'):
            with self.subTest(fix=fix), redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                report(history(days=0), [fix])

    def test_usage_day_override_can_raise_but_cannot_lower_phase_three_floor(self):
        with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            report(history(days=0), extra=['--min-days', '1'])
        self.assertEqual(report(history(), extra=['--min-days', '3'])[0]['verdict'], 'INADMISSIBLE')
        found, _ = report(history(days=0), phase='round', lines=[(0, REACHED)], extra=['--min-days', '1'])
        self.assertEqual(found['verdict'], 'PASS')


if __name__ == '__main__':
    unittest.main()
