#!/usr/bin/env python3
"""Tests of the pure comparison: refusals, and the pinned rule `m2` (gate, not worse, not higher).

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport_compare as mc  # noqa: E402
from memreport_fixtures import MIB, make_summary, mib  # noqa: E402


def compare(baseline, candidate, **kwargs):
    return mc.compare(baseline, candidate, 'm2', **kwargs)


def metric(result, key):
    return next(item for item in result['metrics'] if item['key'] == key)


def row_of(base_values, cand_values, key='peak'):
    """The corpus row of `key` in a one-corpus comparison."""
    result = compare(make_summary({'a': {key: base_values}}), make_summary({'a': {key: cand_values}}, 'cand'))
    return metric(result, key)['corpora']['a']


def verdict_of(key, rows):
    """The metric verdict for {corpus: (baseline values or None, candidate values or None)}."""
    base = make_summary({name: {key: pair[0]} if pair[0] is not None else {} for name, pair in rows.items()})
    cand = make_summary({name: {key: pair[1]} if pair[1] is not None else {} for name, pair in rows.items()}, 'cand')
    return metric(compare(base, cand), key)


class RefusalTests(unittest.TestCase):
    def setUp(self):
        self.series = {'a': {'peak': mib(1000, 1000, 1000)}, 'b': {'peak': mib(1000, 1000, 1000)}}
        self.baseline = make_summary(self.series)

    def refused(self, candidate, baseline=None, **kwargs):
        with self.assertRaises(ValueError) as caught:
            compare(baseline or self.baseline, candidate, **kwargs)
        return str(caught.exception)

    def candidate(self, **kwargs):
        return make_summary(self.series, **dict({'identity': 'cand'}, **kwargs))

    def test_runs_with_the_same_settings_and_different_builds_are_compared(self):
        self.assertEqual(compare(self.baseline, self.candidate())['experiment'], 'm2')

    def test_different_corpus_names(self):
        message = self.refused(make_summary({'a': self.series['a'], 'c': self.series['b']}, 'cand'))
        self.assertIn('corpora differ', message)
        self.assertIn("['a', 'b']", message)
        self.assertIn("['a', 'c']", message)

    def test_a_different_commit(self):
        candidate = self.candidate()
        candidate['corpora']['b']['commit'] = 'd' * 40
        message = self.refused(candidate)
        self.assertIn('corpus b: commit', message)
        self.assertNotIn('corpus a', message)

    def test_a_different_dirty_count(self):
        candidate = self.candidate()
        candidate['corpora']['a']['dirty'] = 1
        self.assertIn('corpus a: dirty file count', self.refused(candidate))

    def test_a_different_workload_sha256(self):
        self.assertIn('corpus a: workload sha256', self.refused(self.candidate(sha='e' * 64)))

    def test_a_workload_recorded_on_one_side_only(self):
        candidate = self.candidate()
        candidate['corpora']['a']['workload'] = None
        self.assertIn('corpus a: workload sha256', self.refused(candidate))

    def test_a_workload_recorded_on_neither_side_is_not_a_difference(self):
        candidate = self.candidate()
        for summary in (self.baseline, candidate):
            summary['corpora']['a']['workload'] = None
        self.assertEqual(compare(self.baseline, candidate)['experiment'], 'm2')

    def test_different_reps(self):
        self.assertIn('reps', self.refused(self.candidate(reps=5)))

    def test_different_settle(self):
        self.assertIn('settle', self.refused(self.candidate(settle=0)))

    def test_different_thread_counts(self):
        self.assertIn('threads', self.refused(self.candidate(threads=8)))

    def test_different_measured_process_environment(self):
        message = self.refused(self.candidate(env={'PRISM_X': '1'}))
        self.assertIn('environment', message)
        self.assertIn('PRISM_X', message)

    def test_different_allocators(self):
        self.assertIn('allocator', self.refused(self.candidate(allocator='jemalloc')))

    def test_different_platforms(self):
        message = self.refused(self.candidate(platform={'os': 'linux', 'arch': 'x86_64'}))
        self.assertIn('platform', message)
        self.assertIn('linux', message)

    def test_a_difference_in_one_corpus_only_is_refused_although_the_headline_matches(self):
        """The headline is the first admitted corpus; the others must match too."""
        for field, value in (('threads', 8), ('env', {'PRISM_X': '1'}), ('allocator', 'jemalloc'),
                             ('platform', {'os': 'linux', 'arch': 'x86_64'})):
            candidate = make_summary(self.series, 'cand', only={field: {'b': value}})
            self.assertEqual(candidate['provenance']['threads'], 4)    # the headline looks the same
            message = self.refused(candidate)
            self.assertIn('corpus b', message, field)
            self.assertNotIn('corpus a', message, field)

    def test_the_same_difference_in_the_baseline_only_is_refused_too(self):
        baseline = make_summary(self.series, only={'threads': {'b': 8}})
        self.assertIn('corpus b: threads', self.refused(self.candidate(), baseline=baseline))

    def test_an_unknown_platform_is_refused(self):
        for platform in (None, {'os': 'macos'}, {}):
            candidate = self.candidate(platform=platform)
            self.assertIn('platform is unknown', self.refused(candidate), platform)
            self.assertIn('platform is unknown', self.refused(self.baseline, baseline=candidate), platform)

    def test_a_corpus_with_no_admitted_run_on_one_side_is_not_checked_for_settings(self):
        candidate = self.candidate()
        entry = candidate['corpora']['b']
        entry.update(build=None, env=None, threads=None, platform=None, metrics={})
        self.assertEqual(compare(self.baseline, candidate)['experiment'], 'm2')

    def test_the_same_build_is_refused(self):
        self.assertIn('same build identity', self.refused(self.candidate(identity='base')))

    def test_the_same_build_is_compared_when_allowed_and_the_result_says_so(self):
        result = compare(self.baseline, self.candidate(identity='base'), allow_same_build=True)
        self.assertTrue(result['same_build'])
        self.assertFalse(compare(self.baseline, self.candidate())['same_build'])

    def test_a_run_with_no_build_identity_is_refused_whichever_side_it_is_on(self):
        candidate = self.candidate()
        candidate['provenance']['build'] = None
        self.assertIn('no build identity', self.refused(candidate))
        baseline = copy.deepcopy(self.baseline)
        baseline['provenance']['build'] = None
        self.assertIn('no build identity', self.refused(self.candidate(), baseline=baseline))

    def test_every_difference_is_named_in_one_message(self):
        message = self.refused(self.candidate(reps=5, threads=8, allocator='jemalloc'))
        for word in ('reps', 'threads', 'allocator'):
            self.assertIn(word, message)

    def test_the_inputs_are_not_modified(self):
        candidate = self.candidate()
        before = copy.deepcopy((self.baseline, candidate))
        compare(self.baseline, candidate)
        self.assertEqual((self.baseline, candidate), before)


class GateTests(unittest.TestCase):
    def test_a_separated_reduction_of_100_mib_passes_by_the_absolute_arm_alone(self):
        row = row_of(mib(4000, 4000, 4000), mib(3900, 3900, 3900))    # 2.5 %, exactly 100 MiB
        self.assertEqual((row['outcome'], row['effect']), ('pass', 'absolute'))

    def test_one_byte_under_100_mib_and_under_5_percent_is_not_an_effect(self):
        row = row_of(mib(4000, 4000, 4000), [3900 * MIB + 1] * 3)
        self.assertEqual((row['outcome'], row['effect']), ('fail', 'none'))
        self.assertTrue(row['separated'])

    def test_a_separated_reduction_of_5_percent_passes_by_the_fraction_arm_alone(self):
        row = row_of(mib(1000, 1000, 1000), mib(950, 950, 950))    # 50 MiB, exactly 5 %
        self.assertEqual((row['outcome'], row['effect']), ('pass', 'fraction'))

    def test_one_byte_under_5_percent_is_not_an_effect(self):
        row = row_of(mib(1000, 1000, 1000), [950 * MIB + 1] * 3)
        self.assertEqual((row['outcome'], row['effect']), ('fail', 'none'))

    def test_both_arms_are_reported_as_both(self):
        self.assertEqual(row_of(mib(1000, 1000, 1000), mib(800, 800, 800))['effect'], 'both')

    def test_ranges_that_touch_or_overlap_by_one_byte_are_not_separated(self):
        base = [900 * MIB, 950 * MIB, 1000 * MIB]
        touching = row_of(base, [500 * MIB, 600 * MIB, 900 * MIB])
        overlapping = row_of(base, [500 * MIB, 600 * MIB, 900 * MIB + 1])
        self.assertEqual((touching['outcome'], touching['separated']), ('fail', False))
        self.assertEqual((overlapping['outcome'], overlapping['separated']), ('fail', False))
        clear = row_of(base, [500 * MIB, 600 * MIB, 900 * MIB - 1])
        self.assertEqual((clear['outcome'], clear['separated']), ('pass', True))

    def test_an_increase_is_a_fail_not_a_negative_pass(self):
        row = row_of(mib(1000, 1000, 1000), mib(1100, 1100, 1100))
        self.assertEqual(row['outcome'], 'fail')
        self.assertEqual(row['reduction'], -100 * MIB)

    def test_two_runs_on_either_side_are_insufficient_not_a_pass_and_not_a_fail(self):
        self.assertEqual(row_of(mib(1000, 1000), mib(500, 500, 500))['outcome'], 'insufficient runs')
        self.assertEqual(row_of(mib(1000, 1000, 1000), mib(500, 500))['outcome'], 'insufficient runs')

    def test_three_runs_are_enough(self):
        self.assertEqual(row_of(mib(1000, 1000, 1000), mib(500, 500, 500))['outcome'], 'pass')

    def test_the_reduction_is_baseline_median_minus_candidate_median_not_minimum_minus_minimum(self):
        row = row_of(mib(1000, 1100, 1300), mib(700, 900, 950))    # medians differ by 200, minima by 300
        self.assertEqual(row['reduction'], 200 * MIB)

    def test_the_effect_is_judged_on_the_medians(self):
        # minima differ by 120 MiB (the absolute arm), medians by 90 MiB of 4000 (neither arm)
        row = row_of(mib(4000, 4000, 4100), mib(3880, 3910, 3920))
        self.assertEqual((row['reduction'], row['effect'], row['outcome']), (90 * MIB, 'none', 'fail'))

    def test_a_zero_baseline_cannot_pass_and_does_not_divide_by_zero(self):
        row = row_of([0, 0, 0], [0, 0, 0])
        self.assertEqual((row['outcome'], row['effect']), ('fail', 'none'))
        self.assertIsNone(row['percent'])

    def gate_verdict(self, rows):
        return verdict_of('peak', {name: (mib(*pair[0]) if pair[0] else pair[0],
                                          mib(*pair[1]) if pair[1] else pair[1])
                                   for name, pair in rows.items()})['verdict']

    def test_a_gate_that_passes_on_one_corpus_and_fails_on_another_fails(self):
        self.assertEqual(self.gate_verdict({'a': ((1000,) * 3, (500,) * 3), 'b': ((1000,) * 3, (1000,) * 3)}), 'fail')

    def test_a_gate_with_one_corpus_only_is_undecided_not_a_fail(self):
        self.assertEqual(self.gate_verdict({'a': ((1000,) * 3, (500,) * 3)}), 'undecided')
        self.assertEqual(self.gate_verdict({'a': ((1000,) * 3, (1000,) * 3)}), 'undecided')

    def test_a_gate_that_passes_on_two_corpora_passes_even_if_a_third_fails(self):
        rows = {'a': ((1000,) * 3, (500,) * 3), 'b': ((1000,) * 3, (500,) * 3), 'c': ((1000,) * 3, (1000,) * 3)}
        self.assertEqual(self.gate_verdict(rows), 'pass')

    def test_one_pass_and_one_corpus_short_of_runs_is_undecided(self):
        rows = {'a': ((1000,) * 3, (500,) * 3), 'b': ((1000,) * 2, (500,) * 3)}
        self.assertEqual(self.gate_verdict(rows), 'undecided')

    def test_one_pass_and_one_corpus_with_a_series_on_one_side_only_is_undecided_not_fail(self):
        self.assertEqual(self.gate_verdict({'a': ((1000,) * 3, (500,) * 3), 'b': ((1000,) * 3, None)}), 'undecided')
        self.assertEqual(self.gate_verdict({'a': ((1000,) * 3, (500,) * 3), 'b': (None, (500,) * 3)}), 'undecided')

    def test_one_pass_and_one_corpus_measured_on_neither_side_is_undecided(self):
        self.assertEqual(self.gate_verdict({'a': ((1000,) * 3, (500,) * 3), 'b': (None, None)}), 'undecided')

    def test_one_fail_and_one_corpus_that_could_still_pass_cannot_reach_two_and_is_a_fail(self):
        self.assertEqual(self.gate_verdict({'a': ((1000,) * 3, (1000,) * 3), 'b': ((1000,) * 3, None)}), 'fail')
        rows = {'a': ((1000,) * 3, (1000,) * 3), 'b': ((1000,) * 2, (500,) * 3), 'c': ((1000,) * 3, (1000,) * 3)}
        self.assertEqual(self.gate_verdict(rows), 'fail')    # at most one corpus can still pass

    def test_short_and_one_sided_corpora_together_can_still_reach_two(self):
        rows = {'a': ((1000,) * 2, (500,) * 3), 'b': ((1000,) * 3, None), 'c': ((1000,) * 3, (1000,) * 3)}
        self.assertEqual(self.gate_verdict(rows), 'undecided')

    def test_a_gate_metric_absent_from_every_corpus_on_both_sides_is_not_measured(self):
        self.assertEqual(self.gate_verdict({'a': (None, None), 'b': (None, None)}), 'not measured')


class NotWorseTests(unittest.TestCase):
    def test_a_candidate_median_of_exactly_1_05_times_the_baseline_is_ok(self):
        self.assertEqual(row_of([1000] * 3, [1050] * 3, 'load_ms')['outcome'], 'ok')

    def test_ok_does_not_depend_on_the_ranges(self):
        # at most 1.05 x: ok even if the candidate's range is far wider than the baseline's
        self.assertEqual(row_of([1000] * 3, [10, 1050, 5000], 'load_ms')['outcome'], 'ok')

    def test_just_above_1_05_with_separate_ranges_is_worse(self):
        self.assertEqual(row_of([1000] * 3, [1051] * 3, 'load_ms')['outcome'], 'worse')
        self.assertEqual(row_of([10 ** 9] * 3, [1050000001] * 3, 'load_ms')['outcome'], 'worse')

    def test_above_1_05_with_overlapping_ranges_is_unresolved_not_ok(self):
        # 1300 > 1.05 x 1100 and 1200 <= 1500: no evidence either way, and not "ok"
        row = row_of([1000, 1100, 1500], [1200, 1300, 1400], 'load_ms')
        self.assertEqual((row['outcome'], row['above_band'], row['ranges_apart']), ('unresolved', True, False))
        row = row_of([1000, 1000, 5000], [3000, 3000, 3000], 'load_ms')    # x3.000 inside a wide baseline range
        self.assertEqual(row['outcome'], 'unresolved')

    def test_ranges_that_just_touch_are_overlapping_and_one_apart_are_not(self):
        self.assertEqual(row_of([1000, 1000, 1200], [1200, 1300, 1300], 'load_ms')['outcome'], 'unresolved')
        self.assertEqual(row_of([1000, 1000, 1200], [1201, 1300, 1300], 'load_ms')['outcome'], 'worse')

    def test_fewer_than_3_runs_on_either_side_is_unresolved_whatever_the_numbers(self):
        for base, cand in (([1000], [1000]), ([1000] * 2, [1000] * 3), ([1000] * 3, [5000] * 2),
                           ([1000], [1060]), ([1000], [10])):
            row = row_of(base, cand, 'load_ms')
            self.assertEqual(row['outcome'], 'unresolved', (base, cand))
            self.assertIn('at least 3', row['detail'])
        self.assertEqual(row_of([1000] * 3, [1000] * 3, 'load_ms')['outcome'], 'ok')    # 3 and 3 is enough

    def test_the_median_not_the_baseline_maximum_is_the_yardstick(self):
        # median 1210 > 1.05 x 1000 and the ranges are apart; against the maximum (1200 x 1.05 = 1260) it would be ok
        self.assertEqual(row_of([1000, 1000, 1200], [1210] * 3, 'load_ms')['outcome'], 'worse')

    def test_the_ratio_is_printed_and_carried_as_a_number_with_the_two_flags(self):
        row = row_of([1000] * 3, [1031] * 3, 'load_ms')
        self.assertEqual((row['detail'], row['above_band'], row['ranges_apart']), ('×1.031', False, True))
        self.assertAlmostEqual(row['ratio'], 1.031)
        self.assertEqual(row_of([1000] * 3, [900] * 3, 'build_ms')['detail'], '×0.900')
        overlap = row_of([1000, 1100, 1500], [1200, 1300, 1400], 'load_ms')
        self.assertEqual(overlap['detail'], '×1.182, ranges overlap')
        self.assertAlmostEqual(overlap['ratio'], 1300 / 1100)
        self.assertEqual(row_of([1000] * 3, [1500] * 3, 'load_ms')['detail'], '×1.500')

    def test_an_improvement_is_ok(self):
        self.assertEqual(row_of([1000] * 3, [10] * 3, 'build_ms')['outcome'], 'ok')

    def test_a_zero_baseline_median_does_not_divide_by_zero(self):
        zero = row_of([0, 0, 0], [0, 0, 0], 'load_ms')
        self.assertEqual((zero['outcome'], zero['ratio']), ('ok', None))    # 0 to 0
        zero_to_positive = row_of([0, 0, 0], [5, 5, 5], 'load_ms')
        self.assertEqual(zero_to_positive['outcome'], 'worse')
        self.assertEqual((zero_to_positive['detail'], zero_to_positive['ratio']), ('×n/a', None))
        self.assertEqual(row_of([0, 0, 10], [5, 5, 5], 'load_ms')['outcome'], 'unresolved')    # ranges overlap

    def test_a_percentage_is_unavailable_when_the_baseline_median_is_zero(self):
        row = row_of([0, 0, 0], [5, 5, 5], 'load_ms')
        self.assertIsNone(row['percent'])
        self.assertEqual(row['difference'], 5)


class NotHigherTests(unittest.TestCase):
    def test_exactly_1_mib_above_is_ok(self):
        self.assertEqual(row_of([100 * MIB] * 3, [101 * MIB] * 3, 'steady_heap')['outcome'], 'ok')

    def test_one_byte_past_1_mib_above_is_higher(self):
        self.assertEqual(row_of([100 * MIB] * 3, [101 * MIB + 1] * 3, 'steady_heap')['outcome'], 'higher')

    def test_a_lower_live_heap_is_ok_however_much_lower(self):
        self.assertEqual(row_of([100 * MIB] * 3, [99 * MIB - 1] * 3, 'steady_heap')['outcome'], 'ok')
        self.assertEqual(row_of([100 * MIB] * 3, [10 * MIB] * 3, 'steady_heap')['outcome'], 'ok')

    def test_the_median_not_the_baseline_maximum_is_the_yardstick(self):
        row = row_of([100 * MIB, 100 * MIB, 200 * MIB], [101 * MIB + 1] * 3, 'steady_heap')
        self.assertEqual(row['outcome'], 'higher')

    def test_the_candidate_median_not_its_minimum_is_judged(self):
        self.assertEqual(row_of([100 * MIB] * 3, [50 * MIB, 150 * MIB, 150 * MIB], 'steady_heap')['outcome'], 'higher')


class MetricVerdictTests(unittest.TestCase):
    def test_the_metric_is_ok_only_when_ok_on_every_corpus_with_both_series(self):
        rows = {'a': ([1000] * 3, [1000] * 3), 'b': ([1000] * 3, [2000] * 3)}
        self.assertEqual(verdict_of('load_ms', rows)['verdict'], 'worse')
        rows['b'] = ([1000] * 3, [1000] * 3)
        self.assertEqual(verdict_of('load_ms', rows)['verdict'], 'ok')

    def test_a_corpus_that_is_higher_vetoes_the_metric_beside_ok_and_missing_corpora(self):
        rows = {'a': ([100 * MIB] * 3, [100 * MIB] * 3), 'b': ([100 * MIB] * 3, [200 * MIB] * 3),
                'c': (None, None)}
        result = verdict_of('steady_heap', rows)
        self.assertEqual((result['verdict'], result['corpora']['c']['outcome']), ('higher', 'not measured'))

    def test_a_corpus_that_is_worse_vetoes_the_metric_beside_ok_and_missing_corpora(self):
        rows = {'a': ([1000] * 3, [1000] * 3), 'b': ([1000] * 3, [2000] * 3), 'c': (None, None)}
        self.assertEqual(verdict_of('load_ms', rows)['verdict'], 'worse')

    def test_the_precedence_is_worse_then_unresolved_then_incomplete_then_ok(self):
        worse, unresolved = ([1000] * 3, [2000] * 3), ([1000, 1100, 1500], [1200, 1300, 1400])
        one_sided, ok = ([1000] * 3, None), ([1000] * 3, [1000] * 3)
        self.assertEqual(verdict_of('load_ms', {'a': worse, 'b': unresolved, 'c': one_sided})['verdict'], 'worse')
        self.assertEqual(verdict_of('load_ms', {'a': unresolved, 'b': one_sided, 'c': ok})['verdict'], 'unresolved')
        self.assertEqual(verdict_of('load_ms', {'a': one_sided, 'b': ok})['verdict'], 'incomplete')
        self.assertEqual(verdict_of('load_ms', {'a': ok, 'b': ok})['verdict'], 'ok')

    def test_a_single_run_pair_gives_unresolved_not_a_verdict(self):
        self.assertEqual(verdict_of('load_ms', {'a': ([1000], [1000])})['verdict'], 'unresolved')

    def test_the_largest_ratio_among_the_corpora_is_carried_for_a_not_worse_metric(self):
        rows = {'a': ([1000] * 3, [1031] * 3), 'b': ([1000] * 3, [1010] * 3), 'c': (None, None)}
        result = verdict_of('load_ms', rows)
        self.assertAlmostEqual(result['largest_ratio'], 1.031)
        self.assertIsNone(verdict_of('peak', {'a': (mib(1000) * 3, mib(900) * 3)})['largest_ratio'])
        self.assertIsNone(verdict_of('load_ms', {'a': ([0] * 3, [0] * 3)})['largest_ratio'])

    def test_worse_and_higher_win_over_incomplete(self):
        rows = {'a': ([1000] * 3, [2000] * 3), 'b': ([1000] * 3, None)}
        self.assertEqual(verdict_of('load_ms', rows)['verdict'], 'worse')
        rows = {'a': ([100 * MIB] * 3, [200 * MIB] * 3), 'b': (None, [100 * MIB] * 3)}
        self.assertEqual(verdict_of('steady_heap', rows)['verdict'], 'higher')

    def test_a_series_on_one_side_only_is_one_sided_and_makes_the_metric_incomplete(self):
        for rows in ({'a': ([1000] * 3, [1000] * 3), 'b': ([1000] * 3, None)},
                     {'a': ([1000] * 3, [1000] * 3), 'b': (None, [1000] * 3)}):
            result = verdict_of('load_ms', rows)
            self.assertEqual((result['verdict'], result['corpora']['b']['outcome']), ('incomplete', 'one-sided'))
        rows = {'a': ([100 * MIB] * 3, [100 * MIB] * 3), 'b': ([100 * MIB] * 3, None)}
        self.assertEqual(verdict_of('steady_heap', rows)['verdict'], 'incomplete')

    def test_a_corpus_with_the_series_on_neither_side_is_not_measured_and_does_not_decide(self):
        rows = {'a': ([1000] * 3, [1000] * 3), 'b': (None, None)}
        result = verdict_of('load_ms', rows)
        self.assertEqual((result['verdict'], result['corpora']['b']['outcome']), ('ok', 'not measured'))

    def test_a_metric_absent_from_both_runs_is_not_measured_and_decides_nothing(self):
        base = make_summary({'a': {'peak': mib(1000) * 3}})
        cand = make_summary({'a': {'peak': mib(500) * 3}}, 'cand')
        result = compare(base, cand)
        for key in ('build_peak', 'load_ms', 'build_ms', 'steady_heap'):
            self.assertEqual(metric(result, key)['verdict'], 'not measured', key)
        self.assertEqual(metric(result, 'peak')['verdict'], 'undecided')    # one corpus: it cannot decide

    def test_a_statistic_that_is_none_or_has_no_runs_counts_as_absent(self):
        none_side = make_summary({'a': {'load_ms': []}})
        cand = make_summary({'a': {'load_ms': [5]}}, 'cand')
        row = metric(compare(none_side, cand), 'load_ms')
        self.assertEqual((row['verdict'], row['corpora']['a']['outcome']), ('incomplete', 'one-sided'))
        both = compare(make_summary({'a': {'load_ms': []}}), make_summary({'a': {'load_ms': []}}, 'cand'))
        self.assertEqual(metric(both, 'load_ms')['verdict'], 'not measured')
        empty = make_summary({'a': {}}, 'cand')
        empty['corpora']['a']['metrics']['load_ms'] = {'min': 1, 'median': 1, 'max': 1, 'n': 0}
        row = metric(compare(make_summary({'a': {'load_ms': [5]}}), empty), 'load_ms')
        self.assertEqual(row['corpora']['a']['outcome'], 'one-sided')

    def test_the_table_has_the_five_metrics_with_their_roles(self):
        result = compare(make_summary({'a': {}}), make_summary({'a': {}}, 'cand'))
        self.assertEqual([(item['key'], item['role']) for item in result['metrics']],
                         [('peak', 'gate'), ('build_peak', 'gate'), ('load_ms', 'not_worse'),
                          ('build_ms', 'not_worse'), ('steady_heap', 'not_higher')])

    def test_an_unknown_experiment_is_refused(self):
        with self.assertRaises(ValueError):
            mc.compare(make_summary({}), make_summary({}, 'cand'), 'm9')

    def test_a_gate_metric_must_be_in_bytes(self):
        with self.assertRaises(ValueError):
            mc.check_experiments({'x': (('k', 'Label', 'ms', mc.GATE),)})
        mc.check_experiments({'x': (('k', 'Label', 'bytes', mc.GATE), ('j', 'Label', 'ms', mc.NOT_WORSE))})
        mc.check_experiments(mc.EXPERIMENTS)    # and the shipped table passes

    def test_difference_is_candidate_minus_baseline_with_a_percentage_of_the_baseline(self):
        row = row_of(mib(1000, 1000, 1000), mib(750, 750, 750))
        self.assertEqual((row['difference'], row['percent']), (-250 * MIB, -25.0))


if __name__ == '__main__':
    unittest.main()
