#!/usr/bin/env python3
"""Tests of the `build` kind and of `load_ms`: admission, derivation, aggregation (no subprocess).

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
from memreport_fixtures import MIB, canned, report, stage, stage_named  # noqa: E402

CORPUS = {'path': '/c', 'commit': 'abc', 'dirty': 0, 'cache_files': {}, 'workload': None}


WROTE = {'cpg_cache_bytes': 1000, 'call_edge_sidecar_bytes': 100}


def ok_build(rep, cache_files=WROTE, **changes):
    data = canned('build')
    for name, fields in changes.items():
        stage_named(data, name).update(fields)
    result = {'kind': 'build', 'rep': rep, 'status': 'ok', 'report': data}
    if cache_files is not None:
        result['cache_files'] = dict(cache_files)
    return result


class BuildAdmissionTests(unittest.TestCase):
    def test_a_first_build_that_missed_both_caches_is_admissible(self):
        self.assertIsNone(memreport.validate('build', canned('build')))

    def test_a_run_that_hit_the_cpg_cache_is_not_a_first_build(self):
        data = canned('build')
        data['cache']['cpg_hit'] = True
        self.assertEqual(memreport.validate('build', data), 'the CPG cache was hit, so this was not a first build')

    def test_a_run_that_loaded_the_call_edge_index_is_not_a_first_build(self):
        data = canned('build')
        data['cache']['call_edges_loaded'] = True
        self.assertEqual(memreport.validate('build', data),
                         'the call-edge index was loaded, so this was not a first build')

    def test_a_report_that_does_not_say_the_cache_missed_is_not_admitted(self):
        for flag in ('cpg_hit', 'call_edges_loaded'):
            data = canned('build')
            data['cache'][flag] = None
            self.assertIn('does not say', memreport.validate('build', data))

    def test_a_missing_stage_or_peak_is_inadmissible(self):
        data = canned('build')
        data['stages'] = [entry for entry in data['stages'] if entry['name'] != 'after_call_edges']
        self.assertEqual(memreport.validate('build', data), 'stage after_call_edges is missing')
        data = canned('build')
        stage_named(data, 'after_call_edges')['peak_bytes'] = None
        self.assertEqual(memreport.validate('build', data), 'after_call_edges.peak_bytes is unavailable')

    def test_the_usual_structural_check_applies(self):
        data = canned('build')
        data['threads'] = 0
        self.assertIn('threads', memreport.validate('build', data))

    def test_a_warm_report_with_a_hit_is_still_a_valid_warm_run_and_not_a_build(self):
        self.assertIsNone(memreport.validate('warm', canned('warm')))
        self.assertIsNotNone(memreport.validate('build', canned('warm')))


class BuildDerivationTests(unittest.TestCase):
    def test_build_peak_is_the_peak_at_after_call_edges_and_build_ms_spans_load_repo_to_index(self):
        data = report([stage('after_load_repo', 1, 2, elapsed=1000), stage('after_index', 3, 4, elapsed=9000),
                       stage('after_call_edges', 5, 777, elapsed=9500)])
        self.assertEqual(memreport.derive('build', data), {'build_peak': 777, 'build_ms': 8000})

    def test_build_ms_is_none_when_either_elapsed_time_is_unavailable(self):
        for loaded, indexed in ((None, 9000), (1000, None), (None, None)):
            data = report([stage('after_load_repo', 1, 2, elapsed=loaded), stage('after_index', 3, 4, elapsed=indexed),
                           stage('after_call_edges', 5, 777)])
            self.assertIsNone(memreport.derive('build', data)['build_ms'])
            self.assertEqual(memreport.derive('build', data)['build_peak'], 777)

    def test_build_ms_is_none_when_a_stage_is_absent(self):
        data = report([stage('after_call_edges', 5, 777)])
        self.assertEqual(memreport.derive('build', data), {'build_peak': 777, 'build_ms': None})

    def test_load_ms_of_a_warm_run_spans_load_repo_to_index(self):
        warm = report([stage('after_load_repo', 100, 120, elapsed=400), stage('after_index', 400, 700, elapsed=1900)])
        self.assertEqual(memreport.derive('warm', warm)['load_ms'], 1500)

    def test_load_ms_is_none_unless_both_elapsed_times_are_reported(self):
        for loaded, indexed in ((None, 1900), (400, None), (None, None)):
            warm = report([stage('after_load_repo', 100, 120, elapsed=loaded),
                           stage('after_index', 400, 700, elapsed=indexed)])
            self.assertIsNone(memreport.derive('warm', warm)['load_ms'])


class BuildCacheEvidenceTests(unittest.TestCase):
    """A build is a build that writes the cache: the files it left behind are part of the evidence."""

    def reasons(self, cache_files):
        entry = memreport.build_entry(CORPUS, [ok_build(1, cache_files=cache_files)])
        return entry['admissible']['build'], [item['reason'] for item in entry['inadmissible']]

    def test_both_files_written_is_admissible(self):
        self.assertEqual(self.reasons(WROTE), (1, []))

    def test_no_cpg_cache_file_is_inadmissible(self):
        self.assertEqual(self.reasons({'cpg_cache_bytes': None, 'call_edge_sidecar_bytes': 100}), (
            0, ['the run wrote no cpg-cache.bin, so this was not a build that writes the cache']))

    def test_no_call_edge_sidecar_is_inadmissible(self):
        self.assertEqual(self.reasons({'cpg_cache_bytes': 1000, 'call_edge_sidecar_bytes': None}), (
            0, ['the run wrote no call-edge sidecar, so this was not a build that writes the cache']))

    def test_an_empty_file_is_not_a_written_cache(self):
        self.assertEqual(self.reasons({'cpg_cache_bytes': 0, 'call_edge_sidecar_bytes': 100})[0], 0)
        self.assertEqual(self.reasons({'cpg_cache_bytes': 1000, 'call_edge_sidecar_bytes': 0})[0], 0)

    def test_a_size_that_is_not_an_integer_is_not_a_written_cache(self):
        for size in (True, 1.5, '1000', -5):
            self.assertEqual(self.reasons({'cpg_cache_bytes': size, 'call_edge_sidecar_bytes': 100})[0], 0, size)

    def test_a_stored_build_result_without_cache_files_is_inadmissible(self):
        absent = ok_build(1, cache_files=None)
        malformed = dict(ok_build(1), cache_files='x')
        for result in (absent, malformed):
            entry = memreport.build_entry(CORPUS, [result])
            self.assertEqual(entry['admissible']['build'], 0, result.get('cache_files'))
            self.assertIn('records no cache file sizes', entry['inadmissible'][0]['reason'])
        self.assertEqual(self.reasons({})[0], 0)

    def test_the_check_is_for_the_build_kind_only(self):
        warm = {'kind': 'warm', 'rep': 1, 'status': 'ok', 'report': canned('warm')}
        self.assertEqual(memreport.build_entry(CORPUS, [warm])['admissible']['warm'], 1)


class BuildEntryTests(unittest.TestCase):
    def test_build_metrics_are_series_over_the_admitted_repetitions(self):
        entry = memreport.build_entry(CORPUS, [ok_build(1), ok_build(2, after_call_edges={'peak_bytes': 920 * MIB}),
                                               ok_build(3, after_call_edges={'peak_bytes': 930 * MIB})])
        self.assertEqual(entry['admissible']['build'], 3)
        self.assertEqual(entry['metrics']['build_peak'],
                         {'min': 910 * MIB, 'median': 920 * MIB, 'max': 930 * MIB, 'n': 3})
        self.assertEqual(entry['metrics']['build_ms']['median'], 8000)

    def test_a_build_repetition_that_hit_the_cache_is_listed_with_its_reason_and_measured_as_nothing(self):
        hit = ok_build(2)
        hit['report']['cache']['cpg_hit'] = True
        entry = memreport.build_entry(CORPUS, [ok_build(1), hit])
        self.assertEqual(entry['admissible']['build'], 1)
        self.assertEqual(entry['metrics']['build_peak']['n'], 1)
        self.assertEqual(entry['inadmissible'], [{'kind': 'build', 'rep': 2, 'status': 'inadmissible',
                                                  'reason': 'the CPG cache was hit, so this was not a first build'}])

    def test_a_build_with_another_identity_than_the_corpus_first_run_is_excluded(self):
        other = ok_build(2)
        other['report']['build']['build_identity'] = 'other'
        entry = memreport.build_entry(CORPUS, [ok_build(1), other])
        self.assertEqual(entry['admissible']['build'], 1)
        self.assertIn('build identity differs', entry['inadmissible'][0]['reason'])

    def test_without_build_results_the_entry_has_exactly_the_four_readout_kinds(self):
        entry = memreport.build_entry(CORPUS, [{'kind': 'warm', 'rep': 1, 'status': 'ok', 'report': canned('warm')}])
        self.assertEqual(entry['admissible'], {'warm': 1, 'cold': 0, 'workload': 0, 'refresh': 0})

    def test_build_is_not_a_readout_kind(self):
        self.assertEqual(memreport.KINDS, ('warm', 'cold', 'workload', 'refresh'))
        self.assertEqual(memreport.RUN_KINDS, ('warm', 'cold', 'workload', 'refresh', 'build'))

    def test_a_warm_series_with_no_elapsed_time_has_no_load_ms_statistic(self):
        warm = canned('warm')
        stage_named(warm, 'after_index')['elapsed_ms'] = None
        entry = memreport.build_entry(CORPUS, [{'kind': 'warm', 'rep': 1, 'status': 'ok', 'report': warm}])
        self.assertIsNone(entry['metrics']['load_ms'])
        self.assertEqual(entry['metrics']['peak']['n'], 1)


if __name__ == '__main__':
    unittest.main()
