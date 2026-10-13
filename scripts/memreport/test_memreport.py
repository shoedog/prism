#!/usr/bin/env python3
"""Unit tests for the memory-report driver: parsing, validation, derivation, one run.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
from memreport_fixtures import MIB, canned, emit, fake_binary, noise, report, stage, workload_section  # noqa: E402


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
    def test_warm_run_yields_the_peak_the_observed_overshoot_and_the_index_heap(self):
        warm = report([stage('after_load_repo', 100, 120, heap=30), stage('after_index', 400, 700, heap=350)])
        self.assertEqual(memreport.derive('warm', warm),
                         {'peak': 700, 'm2_observed_overshoot': 300, 'index_heap': 320})

    def test_observed_overshoot_is_zero_when_the_footprint_is_the_peak(self):
        warm = report([stage('after_load_repo', 100, 900), stage('after_index', 900, 900)])
        self.assertEqual(memreport.derive('warm', warm)['m2_observed_overshoot'], 0)

    def test_index_heap_is_none_unless_both_stages_report_a_heap(self):
        for loaded, built in ((None, 350), (30, None), (None, None)):
            warm = report([stage('after_load_repo', 100, 120, heap=loaded),
                           stage('after_index', 400, 700, heap=built)])
            self.assertIsNone(memreport.derive('warm', warm)['index_heap'])

    def test_workload_run_yields_steady_state_measured_tree_heap_and_the_string_estimate(self):
        attribution = {'node_string_heap_bytes_estimate': 100, 'node_unique_string_heap_bytes_estimate': 40}
        run = report([stage('after_settle', 500, 900, heap=420, tree=77)], attribution)
        self.assertEqual(memreport.derive('workload', run),
                         {'steady': 500, 'steady_heap': 420, 'm1': 77, 'm3': 60})

    def test_a_workload_run_without_tree_heap_or_heap_in_use_reports_none_not_zero(self):
        run = report([stage('after_settle', 500, 900)], {})
        self.assertEqual(memreport.derive('workload', run),
                         {'steady': 500, 'steady_heap': None, 'm1': None, 'm3': None})
        self.assertIsNone(memreport.summarize([memreport.derive('workload', run)['m1']]))

    def test_a_zero_tree_heap_is_a_measurement_not_an_absence(self):
        run = report([stage('after_settle', 500, 900, tree=0)], {})
        self.assertEqual(memreport.derive('workload', run)['m1'], 0)

    def test_refresh_and_cold_runs_yield_their_peaks(self):
        self.assertEqual(memreport.derive('refresh', report([stage('after_refresh', 1, 77)])),
                         {'refresh_peak': 77})
        self.assertEqual(memreport.derive('cold', report([stage('after_call_edges', 1, 66)])),
                         {'cold_peak': 66})


class RuleTests(unittest.TestCase):
    def entry(self, **kw):
        stats = lambda value: None if value is None else {'min': value, 'median': value, 'max': value, 'n': 3}  # noqa: E731
        metrics = {'steady': 1000 * MIB, 'peak': 1500 * MIB, 'm1': 0, 'm3': 0}
        metrics.update(kw)
        return {'metrics': {key: stats(value) for key, value in metrics.items()},
                'cache_files': {'cpg_cache_bytes': kw.get('m2', 0)}}

    def test_a_candidate_advances_on_two_corpora_by_fraction_or_absolute(self):
        entries = {'a': self.entry(m2=230 * MIB),                      # 15.3 % of peak
                   'b': self.entry(m2=200 * MIB, peak=9000 * MIB),     # absolute
                   'c': self.entry(m2=10 * MIB)}
        verdict = memreport.evaluate(entries)
        self.assertEqual(verdict['M2']['corpora'], ['a', 'b'])
        self.assertTrue(verdict['M2']['advance'])
        self.assertEqual((verdict['M1']['corpora'], verdict['M1']['advance']), ([], False))

    def test_one_corpus_is_not_enough(self):
        entries = {'a': self.entry(m3=400 * MIB), 'b': self.entry(m3=1 * MIB)}
        self.assertEqual((memreport.evaluate(entries)['M3']['corpora'], memreport.evaluate(entries)['M3']['advance']),
                         (['a'], False))


class ValidateTests(unittest.TestCase):
    def mutate(self, kind, stage_name, **fields):
        data = canned(kind)
        for entry in data['stages']:
            if entry['name'] == stage_name:
                entry.update(fields)
        return data

    def test_a_canned_report_of_each_kind_is_accepted(self):
        for kind in memreport.KINDS:
            self.assertIsNone(memreport.validate(kind, canned(kind)), kind)

    def test_other_report_versions_are_rejected(self):
        for version in (1, 3, None):
            data = canned('cold')
            data['prism_mem_report'] = version
            self.assertIn('report version', memreport.validate('cold', data))

    def test_an_unavailable_number_a_kind_needs_names_the_stage_and_field(self):
        cases = [
            ('warm', 'after_index', {'peak_bytes': None}, 'after_index.peak_bytes is unavailable'),
            ('warm', 'after_load_repo', {'peak_bytes': None}, 'after_load_repo.peak_bytes is unavailable'),
            ('cold', 'after_call_edges', {'peak_bytes': None}, 'after_call_edges.peak_bytes is unavailable'),
            ('refresh', 'after_refresh', {'peak_bytes': None}, 'after_refresh.peak_bytes is unavailable'),
            ('workload', 'after_settle', {'phys_footprint_bytes': None, 'rss_bytes': None},
             'after_settle.phys_footprint_bytes and after_settle.rss_bytes are unavailable'),
            ('warm', 'after_index', {'phys_footprint_bytes': None, 'rss_bytes': None},
             'after_index.phys_footprint_bytes and after_index.rss_bytes are unavailable'),
        ]
        for kind, stage_name, fields, reason in cases:
            self.assertEqual(memreport.validate(kind, self.mutate(kind, stage_name, **fields)), reason)

    def test_a_missing_footprint_falls_back_to_rss_and_is_accepted(self):
        data = self.mutate('workload', 'after_settle', phys_footprint_bytes=None, rss_bytes=5)
        self.assertIsNone(memreport.validate('workload', data))

    def test_a_missing_stage_is_named(self):
        data = canned('warm')
        data['stages'] = [entry for entry in data['stages'] if entry['name'] != 'after_index']
        self.assertEqual(memreport.validate('warm', data), 'stage after_index is missing')
        data['stages'] = None
        self.assertEqual(memreport.validate('warm', data), 'stage after_load_repo is missing')

    def test_optional_numbers_may_be_null_and_derive_returns_none_for_them(self):
        data = self.mutate('workload', 'after_settle', heap_in_use_bytes=None, tree_sitter_heap_bytes=None)
        self.assertIsNone(memreport.validate('workload', data))
        derived = memreport.derive('workload', data)
        self.assertEqual(derived['steady'], 1000 * MIB)
        data['attribution'] = {}
        self.assertIsNone(memreport.validate('workload', data))
        self.assertIsNone(memreport.derive('workload', data)['m3'])

    def test_a_refresh_that_raced_is_rejected(self):
        data = canned('refresh')
        data['refresh'] = {'status': 'raced_stale'}
        self.assertIn('raced_stale', memreport.validate('refresh', data))
        data['refresh'] = None
        self.assertEqual(memreport.validate('refresh', data), 'report has no refresh section')

    def test_workload_section_rules(self):
        cases = [
            (None, 'report has no workload section'),
            (workload_section(errors=2), 'workload had 2 errors'),
            (workload_section(requests=8, responses=7), 'workload answered 7 of 8 requests'),
            (workload_section(requests=0), 'workload ran no requests'),
            ({'requests': None, 'responses': 1, 'errors': 0}, 'workload requests/responses/errors are unavailable'),
        ]
        for section, reason in cases:
            data = canned('workload')
            data['workload'] = section
            self.assertEqual(memreport.validate('workload', data), reason)

    def test_workload_must_match_the_expected_count_and_digest(self):
        data = canned('workload', sha256='a' * 64, requests=8)
        self.assertIsNone(memreport.validate('workload', data, {'requests': 8, 'sha256': 'a' * 64}))
        self.assertIn('differs', memreport.validate('workload', data, {'requests': 8, 'sha256': 'b' * 64}))
        self.assertIn('ran 8 requests, the manifest has 9',
                      memreport.validate('workload', data, {'requests': 9, 'sha256': 'a' * 64}))


class AdmissionTests(unittest.TestCase):
    """validate is wired into the run path: a null measurement keeps a run out of the statistics."""

    def run_fake(self, body, kind, expected=None):
        with tempfile.TemporaryDirectory() as tmp:
            return memreport.run_one([str(fake_binary(tmp, body))], kind, 20, expected)

    def test_a_warm_report_with_a_null_peak_is_inadmissible_not_an_exception(self):
        data = canned('warm')
        data['stages'][1]['peak_bytes'] = None
        result = self.run_fake(emit(data), 'warm')
        self.assertEqual(result['status'], 'inadmissible')
        self.assertEqual(result['reason'], 'after_index.peak_bytes is unavailable')
        self.assertEqual(result['report'], data)

    def test_a_report_of_the_wrong_version_is_inadmissible(self):
        data = canned('cold')
        data['prism_mem_report'] = 1
        self.assertEqual(self.run_fake(emit(data), 'cold')['status'], 'inadmissible')

    def test_a_raced_refresh_is_inadmissible(self):
        data = canned('refresh')
        data['refresh'] = {'status': 'raced_stale'}
        self.assertEqual(self.run_fake(emit(data), 'refresh')['status'], 'inadmissible')

    def test_expected_workload_digest_is_enforced_by_the_run_path(self):
        data = canned('workload', sha256='a' * 64)
        expected = {'requests': 8, 'sha256': 'b' * 64}
        self.assertEqual(self.run_fake(emit(data), 'workload', expected)['status'], 'inadmissible')
        expected['sha256'] = 'a' * 64
        self.assertEqual(self.run_fake(emit(data), 'workload', expected)['status'], 'ok')


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
        result = self.run_fake(emit(canned('warm')))
        self.assertEqual(result['status'], 'ok')
        self.assertEqual(result['report']['stages'][1]['name'], 'after_index')

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
        miss = noise('nav cache: CPG cache miss in /x, rebuilding')
        warm = self.run_fake(emit(canned('warm'), miss), kind='warm')
        self.assertEqual(warm['status'], 'inadmissible')
        self.assertIn('cache', warm['reason'])
        # A cold run is expected to build; the same output is fine there.
        self.assertEqual(self.run_fake(emit(canned('cold'), miss), kind='cold')['status'], 'ok')

    def test_warm_run_that_recomputed_call_edges_is_inadmissible_naming_the_sidecar(self):
        for text in ('nav edge cache: skipping warm sidecar load because prism was built from dirty binary '
                     'inputs; recomputing resolved call edges',
                     'nav edge cache: failed to load sidecar in /x: bad magic; rebuilding'):
            for kind in ('warm', 'workload', 'refresh'):
                result = self.run_fake(emit(canned(kind), noise(text)), kind=kind)
                self.assertEqual(result['status'], 'inadmissible', (kind, text))
                self.assertIn('call-edge sidecar', result['reason'])
            # a cold run builds everything anyway
            self.assertEqual(self.run_fake(emit(canned('cold'), noise(text)), kind='cold')['status'], 'ok')

    def test_both_cache_misses_are_named_in_the_reason(self):
        text = noise('nav cache: CPG cache miss in /x') + noise('nav edge cache: failed to load sidecar in /x')
        reason = self.run_fake(emit(canned('warm'), text), kind='warm')['reason']
        self.assertIn('CPG cache', reason)
        self.assertIn('call-edge sidecar', reason)

    def test_the_override_warning_alone_does_not_exclude_a_run(self):
        text = noise('nav edge cache: PRISM_NAV_EDGE_CACHE_LOAD_DIRTY=1 set; loading sidecar from dirty '
                     'binary-input build may serve stale resolved edges')
        self.assertEqual(self.run_fake(emit(canned('warm'), text), kind='warm')['status'], 'ok')

    def test_run_with_workload_errors_is_inadmissible(self):
        bad = canned('workload')
        bad['workload'] = workload_section(requests=4, errors=1)
        result = self.run_fake(emit(bad), kind='workload')
        self.assertEqual(result['status'], 'inadmissible')
        self.assertIn('workload had 1 errors', result['reason'])

    def test_a_run_without_a_report_is_inadmissible(self):
        result = self.run_fake("print('nothing useful', file=sys.stderr)\n")
        self.assertEqual(result['status'], 'inadmissible')


if __name__ == '__main__':
    unittest.main()
