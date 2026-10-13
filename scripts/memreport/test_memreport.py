#!/usr/bin/env python3
"""Unit tests for the memory-report driver.

Run with:
    python3 -m unittest scripts/memreport/test_memreport.py
"""
import json
import stat
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402

MIB = 1024 * 1024


def stage(name, footprint, peak, rss=None):
    return {'name': name, 'elapsed_ms': 0, 'rss_bytes': rss if rss is not None else footprint,
            'phys_footprint_bytes': footprint, 'compressed_bytes': 0, 'peak_bytes': peak}


def report(stages, attribution=None, workload=None):
    return {'prism_mem_report': 1, 'build': {'build_identity': 'b'}, 'platform': {'os': 'macos'},
            'config': {}, 'stages': stages, 'ru_maxrss_bytes': 1,
            'attribution': attribution or {}, 'workload': workload, 'refresh': None}


def fake_binary(directory, body):
    """A stand-in for prism-mcp: a Python script that runs `body`."""
    path = Path(directory) / 'fake-prism-mcp'
    path.write_text('#!/usr/bin/env python3\nimport json, sys, time\n' + body)
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


class ParseTests(unittest.TestCase):
    def test_parse_report_takes_the_single_marked_line(self):
        text = 'nav cache: noise\n{"prism_mem_report":1,"stages":[]}\ntrailing\n'
        self.assertEqual(memreport.parse_report(text)['stages'], [])

    def test_parse_report_rejects_zero_or_two_reports(self):
        with self.assertRaises(ValueError):
            memreport.parse_report('no report here\n')
        with self.assertRaises(ValueError):
            memreport.parse_report('{"prism_mem_report":1}\n{"prism_mem_report":1}\n')

    def test_footprint_falls_back_to_rss(self):
        self.assertEqual(memreport.footprint({'phys_footprint_bytes': 7, 'rss_bytes': 9}), 7)
        self.assertEqual(memreport.footprint({'phys_footprint_bytes': None, 'rss_bytes': 9}), 9)

    def test_summarize_reports_min_median_max(self):
        self.assertEqual(memreport.summarize([3, 1, 2]), {'min': 1, 'median': 2, 'max': 3, 'n': 3})
        self.assertIsNone(memreport.summarize([None, None]))
        self.assertEqual(memreport.summarize([5, None])['n'], 1)


class DeriveTests(unittest.TestCase):
    def test_m2_transient_is_the_load_peak_above_the_built_index(self):
        warm = report([stage('after_load_repo', 100, 120), stage('after_index', 400, 700)])
        self.assertEqual(memreport.derive('warm', warm), {'peak': 700, 'm2_transient': 300})

    def test_m2_transient_is_zero_when_parsing_set_the_peak(self):
        warm = report([stage('after_load_repo', 100, 900), stage('after_index', 400, 900)])
        self.assertEqual(memreport.derive('warm', warm)['m2_transient'], 0)

    def test_workload_run_yields_steady_state_and_the_estimates(self):
        attribution = {'tree_bytes_estimate_low': 8, 'tree_bytes_estimate_high': 88,
                       'node_string_heap_bytes_estimate': 100,
                       'node_unique_string_heap_bytes_estimate': 40}
        run = report([stage('after_settle', 500, 900)], attribution)
        self.assertEqual(memreport.derive('workload', run),
                         {'steady': 500, 'm1_low': 8, 'm1_high': 88, 'm3': 60})

    def test_refresh_and_cold_runs_yield_their_peaks(self):
        self.assertEqual(memreport.derive('refresh', report([stage('after_refresh', 1, 77)])),
                         {'refresh_peak': 77})
        self.assertEqual(memreport.derive('cold', report([stage('after_call_edges', 1, 66)])),
                         {'cold_peak': 66})


class RuleTests(unittest.TestCase):
    def row(self, **kw):
        base = {'steady': 1000 * MIB, 'peak': 1500 * MIB, 'm2_transient': 0, 'm1_high': 0, 'm3': 0}
        base.update(kw)
        return base

    def test_a_candidate_advances_on_two_corpora_by_fraction_or_absolute(self):
        rows = {'a': self.row(m2_transient=230 * MIB),            # 15.3 % of peak
                'b': self.row(m2_transient=200 * MIB, peak=9000 * MIB),  # absolute
                'c': self.row(m2_transient=10 * MIB)}
        verdict = memreport.evaluate(rows)
        self.assertEqual(verdict['M2'], {'corpora': ['a', 'b'], 'advance': True})
        self.assertEqual(verdict['M1'], {'corpora': [], 'advance': False})

    def test_one_corpus_is_not_enough(self):
        rows = {'a': self.row(m3=400 * MIB), 'b': self.row(m3=1 * MIB)}
        self.assertEqual(memreport.evaluate(rows)['M3'], {'corpora': ['a'], 'advance': False})

    def test_a_missing_measurement_never_counts(self):
        rows = {'a': {'steady': None, 'peak': None, 'm2_transient': None, 'm1_high': 900 * MIB, 'm3': None},
                'b': self.row(m1_high=900 * MIB)}
        self.assertEqual(memreport.evaluate(rows)['M1'], {'corpora': ['b'], 'advance': False})


class CommandTests(unittest.TestCase):
    def test_each_kind_gets_its_flags(self):
        base = ['bin', '--repo', 'r', '--eager', '--mem-settle-secs', '5']
        cmd = lambda kind: memreport.command_for('bin', 'r', 'c', kind, 'w', 5)  # noqa: E731
        self.assertEqual(cmd('warm'), base + ['--cache-dir', 'c'])
        self.assertEqual(cmd('cold'), base + ['--no-cache'])
        self.assertEqual(cmd('workload'), base + ['--cache-dir', 'c', '--mem-workload', 'w'])
        self.assertEqual(cmd('refresh'), base + ['--cache-dir', 'c', '--mem-refresh'])


class RunOneTests(unittest.TestCase):
    def run_fake(self, body, kind='warm', timeout=20):
        with tempfile.TemporaryDirectory() as tmp:
            binary = fake_binary(tmp, body)
            return memreport.run_one([str(binary)], kind, timeout)

    def test_a_good_run_is_admissible_and_carries_its_report(self):
        result = self.run_fake(
            "print(json.dumps({'prism_mem_report': 1, 'stages': [], 'workload': None}), file=sys.stderr)\n")
        self.assertEqual(result['status'], 'ok')
        self.assertEqual(result['report']['stages'], [])

    def test_timeout_is_recorded_and_does_not_raise(self):
        result = self.run_fake('time.sleep(30)\n', timeout=1)
        self.assertEqual(result['status'], 'timeout')
        self.assertNotIn('report', result)

    def test_a_nonzero_exit_is_recorded_with_the_stderr_tail(self):
        result = self.run_fake("print('boom', file=sys.stderr)\nsys.exit(3)\n")
        self.assertEqual(result['status'], 'failed')
        self.assertEqual(result['exit'], 3)
        self.assertIn('boom', result['stderr_tail'])

    def test_warm_run_that_missed_the_cache_is_inadmissible(self):
        body = ("print('nav cache: CPG cache miss in /x, rebuilding', file=sys.stderr)\n"
                "print(json.dumps({'prism_mem_report': 1, 'stages': [], 'workload': None}), file=sys.stderr)\n")
        self.assertEqual(self.run_fake(body, kind='warm')['status'], 'inadmissible')
        self.assertIn('cache', self.run_fake(body, kind='warm')['reason'])
        # A cold run is expected to build; the same output is fine there.
        self.assertEqual(self.run_fake(body, kind='cold')['status'], 'ok')

    def test_run_with_workload_errors_is_inadmissible(self):
        body = ("print(json.dumps({'prism_mem_report': 1, 'stages': [], "
                "'workload': {'requests': 4, 'responses': 4, 'errors': 1, 'response_bytes': 9}}), file=sys.stderr)\n")
        result = self.run_fake(body, kind='workload')
        self.assertEqual(result['status'], 'inadmissible')
        self.assertIn('workload', result['reason'])

    def test_a_run_without_a_report_is_inadmissible(self):
        result = self.run_fake("print('nothing useful', file=sys.stderr)\n")
        self.assertEqual(result['status'], 'inadmissible')


class RenderTests(unittest.TestCase):
    def test_render_shows_values_inadmissible_counts_and_the_verdict(self):
        summary = {
            'provenance': {'build': {'build_identity': 'b', 'git_sha': 'abc'}, 'reps': 3, 'settle': 5},
            'corpora': {
                'a': {'commit': '111', 'dirty': 0, 'admissible': {'warm': 3, 'cold': 2, 'workload': 3, 'refresh': 3},
                      'inadmissible': [{'kind': 'cold', 'rep': 2, 'status': 'timeout', 'reason': 'timeout'}],
                      'metrics': {'steady': {'min': 900 * MIB, 'median': 1000 * MIB, 'max': 1100 * MIB, 'n': 3}},
                      'attribution': {'files': 10}},
            },
            'verdict': {'M1': {'corpora': [], 'advance': False},
                        'M2': {'corpora': ['a'], 'advance': False},
                        'M3': {'corpora': [], 'advance': False}},
        }
        text = memreport.render(summary)
        self.assertIn('1000.0', text)       # median steady state in MiB
        self.assertIn('timeout', text)      # the excluded run is named
        self.assertIn('| M2 | a | no |', text)
        self.assertIn('coarse triage', text)


if __name__ == '__main__':
    unittest.main()
