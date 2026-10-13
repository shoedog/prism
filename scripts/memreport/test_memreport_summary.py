#!/usr/bin/env python3
"""Tests of the pure aggregation: raw results -> a corpus entry -> a summary.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
from memreport_fixtures import MIB, canned  # noqa: E402

DIGEST = {'sha256': 'f' * 64, 'requests': 8}
CORPUS = {'path': '/c', 'commit': 'abc', 'dirty': 0, 'cache_files': {'cpg_cache_bytes': 7, 'call_edge_sidecar_bytes': None},
          'workload': DIGEST}


def ok(kind, rep, **canned_args):
    return {'kind': kind, 'rep': rep, 'status': 'ok', 'report': canned(kind, **canned_args)}


def bad(kind, rep, status='inadmissible', reason='the CPG cache missed, so this was a cold build', **canned_args):
    """A non-ok result that still carries a perfectly valid report: it must not reach any statistic."""
    return {'kind': kind, 'rep': rep, 'status': status, 'reason': reason, 'report': canned(kind, **canned_args)}


class BuildEntryTests(unittest.TestCase):
    def test_an_inadmissible_result_counts_as_inadmissible_and_as_nothing_else(self):
        entry = memreport.build_entry(CORPUS, [ok('workload', 1, heap=900 * MIB),
                                               bad('workload', 2, heap=100 * MIB, tree=5 * MIB)])
        self.assertEqual(entry['admissible']['workload'], 1)
        self.assertEqual(entry['metrics']['steady_heap']['n'], 1)
        self.assertEqual(entry['metrics']['steady_heap']['median'], 900 * MIB)
        self.assertEqual(entry['metrics']['m1']['max'], 300 * MIB)
        self.assertEqual(entry['inadmissible'], [{'kind': 'workload', 'rep': 2, 'status': 'inadmissible',
                                                  'reason': 'the CPG cache missed, so this was a cold build'}])

    def test_timeouts_and_failures_carry_no_report_and_are_listed(self):
        entry = memreport.build_entry(CORPUS, [
            ok('warm', 1), {'kind': 'warm', 'rep': 2, 'status': 'timeout', 'reason': 'timeout'},
            {'kind': 'cold', 'rep': 1, 'status': 'failed', 'reason': 'exit 3'}])
        self.assertEqual(entry['admissible'], {'warm': 1, 'cold': 0, 'workload': 0, 'refresh': 0})
        self.assertEqual([(i['kind'], i['rep'], i['status']) for i in entry['inadmissible']],
                         [('warm', 2, 'timeout'), ('cold', 1, 'failed')])
        self.assertNotIn('cold_peak', entry['metrics'])

    def test_a_stored_ok_result_is_validated_again(self):
        stored = ok('warm', 1)
        stored['report']['stages'][1]['peak_bytes'] = None
        entry = memreport.build_entry(CORPUS, [stored, ok('warm', 2)])
        self.assertEqual(entry['admissible']['warm'], 1)
        self.assertEqual(entry['inadmissible'][0]['reason'], 'after_index.peak_bytes is unavailable')

    def test_a_stored_workload_run_is_checked_against_the_corpus_digest(self):
        entry = memreport.build_entry(CORPUS, [ok('workload', 1, sha256='f' * 64), ok('workload', 2, sha256='0' * 64)])
        self.assertEqual(entry['admissible']['workload'], 1)
        self.assertIn('differs from the manifest', entry['inadmissible'][0]['reason'])

    def test_an_ok_result_without_a_report_is_inadmissible(self):
        entry = memreport.build_entry(CORPUS, [{'kind': 'cold', 'rep': 1, 'status': 'ok'}])
        self.assertEqual(entry['inadmissible'][0]['reason'], 'result has no report')

    def test_results_are_aggregated_in_kind_and_repetition_order_whatever_the_input_order(self):
        results = [ok('warm', 2), ok('warm', 1), ok('cold', 1)]
        shuffled = memreport.build_entry(CORPUS, results)
        ordered = memreport.build_entry(CORPUS, sorted(results, key=lambda r: (r['kind'], r['rep'])))
        self.assertEqual(shuffled, ordered)

    def test_the_entry_keeps_the_corpus_facts_and_the_build_header(self):
        report = canned('workload')
        report['env'] = {'PRISM_A': '1'}
        entry = memreport.build_entry(CORPUS, [{'kind': 'workload', 'rep': 1, 'status': 'ok', 'report': report}])
        self.assertEqual((entry['commit'], entry['cache_files'], entry['workload']),
                         ('abc', CORPUS['cache_files'], DIGEST))
        self.assertEqual(entry['env'], {'PRISM_A': '1'})
        self.assertEqual(entry['build']['build_identity'], 'b')
        self.assertEqual(entry['attribution']['files'], 10)

    def test_unavailable_optional_numbers_leave_no_metric_rather_than_a_zero(self):
        entry = memreport.build_entry(CORPUS, [ok('workload', 1, heap=None, tree=None)])
        self.assertIsNone(entry['metrics']['m1'])
        self.assertIsNone(entry['metrics']['steady_heap'])
        self.assertEqual(entry['metrics']['steady']['n'], 1)
        self.assertEqual(memreport.decision_points(entry)['m1'], (None, 0))

    def test_build_entry_does_not_modify_its_inputs(self):
        results = [ok('warm', 1)]
        before = repr((CORPUS, results))
        memreport.build_entry(CORPUS, results)
        self.assertEqual(repr((CORPUS, results)), before)


class AssembleTests(unittest.TestCase):
    def test_summary_carries_settings_build_env_and_verdict(self):
        report = canned('workload')
        report['env'] = {'PRISM_A': '1'}
        entry = memreport.build_entry(CORPUS, [{'kind': 'workload', 'rep': 1, 'status': 'ok', 'report': report}])
        summary = memreport.assemble_summary({'reps': 3, 'settle': 5}, {'a': entry})
        self.assertEqual(summary['provenance']['reps'], 3)
        self.assertEqual(summary['provenance']['env'], {'PRISM_A': '1'})
        self.assertEqual(summary['provenance']['build']['build_identity'], 'b')
        self.assertEqual(sorted(summary['verdict']), ['M1', 'M2', 'M3'])

    def test_with_no_admissible_run_the_build_is_none(self):
        entry = memreport.build_entry(CORPUS, [{'kind': 'cold', 'rep': 1, 'status': 'timeout', 'reason': 'timeout'}])
        self.assertIsNone(memreport.assemble_summary({}, {'a': entry})['provenance']['build'])


if __name__ == '__main__':
    unittest.main()
