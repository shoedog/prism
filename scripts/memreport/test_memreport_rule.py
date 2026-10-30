#!/usr/bin/env python3
"""Tests of the decision rule: 200 MiB or 15 % of the base, on at least two corpora.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
from memreport_fixtures import MIB  # noqa: E402

# (label, estimate, base, arm). Bytes unless written with MIB; arm None means "cannot be judged".
CASES = [
    ('fraction only, well below 200 MiB', 30 * MIB, 150 * MIB, 'fraction'),
    ('fraction only, exactly 15 %', 150 * MIB, 1000 * MIB, 'fraction'),
    ('exactly 15 % in small integers', 3, 20, 'fraction'),
    ('just below 15 %', 149 * MIB, 1000 * MIB, 'none'),
    ('one byte below 15 %', 3 * MIB - 1, 20 * MIB, 'none'),
    ('absolute only, exactly 200 MiB', 200 * MIB, 1400 * MIB, 'absolute'),
    ('absolute only, well above', 250 * MIB, 5000 * MIB, 'absolute'),
    ('just below 200 MiB and below 15 %', 200 * MIB - 1, 1400 * MIB, 'none'),
    ('just below both thresholds', 199 * MIB, 1400 * MIB, 'none'),
    ('both arms', 300 * MIB, 1000 * MIB, 'both'),
    ('a zero estimate', 0, 1000 * MIB, 'none'),
    ('no estimate', None, 1000 * MIB, None),
    ('no base', 900 * MIB, None, None),
    ('a zero base', 900 * MIB, 0, None),
    ('neither', None, None, None),
]


class MeetsTests(unittest.TestCase):
    def test_arm_and_verdict_for_each_case(self):
        for label, estimate, base, arm in CASES:
            with self.subTest(label):
                self.assertEqual(memreport.rule_arm(estimate, base), arm)
                self.assertEqual(memreport.meets(estimate, base), arm in ('absolute', 'fraction', 'both'))

    def test_the_thresholds_are_the_documented_ones(self):
        self.assertEqual(memreport.RULE_ABSOLUTE, 200 * MIB)
        self.assertEqual(memreport.RULE_FRACTION, 0.15)

    def test_a_fraction_only_hit_meets_the_rule(self):
        # 30 MiB of 150 MiB is 20 %: far below 200 MiB, so only the fraction arm can make this true.
        self.assertTrue(memreport.meets(30 * MIB, 150 * MIB))

    def test_an_absolute_only_hit_meets_the_rule(self):
        self.assertTrue(memreport.meets(250 * MIB, 5000 * MIB))   # 5 %: only the absolute arm can make this true


def stats(value):
    return None if value is None else {'min': value, 'median': value, 'max': value, 'n': 3}


def entry(steady=1000 * MIB, peak=1000 * MIB, m1=0, m3=0, m2=0):
    """A corpus entry as build_entry makes it: per-run metrics plus the cache file size M2 is taken from."""
    return {'metrics': {'steady': stats(steady), 'peak': stats(peak), 'm1': stats(m1), 'm3': stats(m3)},
            'cache_files': {'cpg_cache_bytes': m2, 'call_edge_sidecar_bytes': None}}


class EvaluateTests(unittest.TestCase):
    def verdict(self, candidate, entries):
        result = memreport.evaluate(entries)[candidate]
        return result['corpora'], result['advance']

    def test_one_corpus_meeting_the_rule_is_not_enough_and_two_are(self):
        meeting, missing = entry(m1=300 * MIB), entry(m1=1 * MIB)
        self.assertEqual(self.verdict('M1', {'a': meeting, 'b': missing}), (['a'], False))
        self.assertEqual(self.verdict('M1', {'a': meeting, 'b': meeting}), (['a', 'b'], True))
        self.assertEqual(self.verdict('M1', {'a': meeting, 'b': missing, 'c': meeting}), (['a', 'c'], True))
        self.assertEqual(self.verdict('M1', {'a': missing, 'b': missing}), ([], False))

    def test_a_fraction_only_corpus_counts_towards_advancing(self):
        small = entry(steady=150 * MIB, m1=30 * MIB)
        self.assertEqual(self.verdict('M1', {'a': small, 'b': small}), (['a', 'b'], True))

    def test_each_candidate_reads_its_own_estimate(self):
        entries = {name: entry(m1=300 * MIB, m2=0, m3=0) for name in 'ab'}
        verdict = memreport.evaluate(entries)
        self.assertEqual([verdict[c]['advance'] for c in ('M1', 'M2', 'M3')], [True, False, False])
        entries = {name: entry(m1=0, m2=300 * MIB, m3=0) for name in 'ab'}
        verdict = memreport.evaluate(entries)
        self.assertEqual([verdict[c]['advance'] for c in ('M1', 'M2', 'M3')], [False, True, False])
        entries = {name: entry(m1=0, m2=0, m3=300 * MIB) for name in 'ab'}
        verdict = memreport.evaluate(entries)
        self.assertEqual([verdict[c]['advance'] for c in ('M1', 'M2', 'M3')], [False, False, True])

    def test_m2_is_judged_against_the_peak_through_the_index_load(self):
        # 100 MiB is 20 % of a 500 MiB peak but only 10 % of a 1000 MiB steady state.
        low_peak = entry(steady=1000 * MIB, peak=500 * MIB, m2=100 * MIB)
        self.assertEqual(self.verdict('M2', {'a': low_peak, 'b': low_peak}), (['a', 'b'], True))
        # and the reverse: 10 % of the peak, 20 % of the steady state
        high_peak = entry(steady=500 * MIB, peak=1000 * MIB, m2=100 * MIB)
        self.assertEqual(self.verdict('M2', {'a': high_peak, 'b': high_peak}), ([], False))

    def test_m1_and_m3_are_judged_against_the_steady_state(self):
        low_steady = entry(steady=500 * MIB, peak=1000 * MIB, m1=100 * MIB, m3=100 * MIB)   # 20 % of steady
        high_steady = entry(steady=1000 * MIB, peak=500 * MIB, m1=100 * MIB, m3=100 * MIB)  # 10 %; 20 % of peak
        for candidate in ('M1', 'M3'):
            self.assertEqual(self.verdict(candidate, {'a': low_steady, 'b': low_steady}), (['a', 'b'], True))
            self.assertEqual(self.verdict(candidate, {'a': high_steady, 'b': high_steady}), ([], False))

    def test_a_missing_estimate_or_base_never_meets(self):
        huge = 900 * MIB
        no_tree = entry(m1=None)
        no_steady = entry(steady=None, m1=huge, m3=huge)
        no_peak = entry(peak=None, m2=huge)
        no_cache_file = entry(m2=None)
        for candidate, bad in (('M1', no_tree), ('M1', no_steady), ('M3', no_steady), ('M2', no_peak),
                               ('M2', no_cache_file)):
            self.assertEqual(self.verdict(candidate, {'a': bad, 'b': bad}), ([], False), candidate)

    def test_an_entry_without_metrics_or_cache_files_cannot_meet(self):
        bare = {'metrics': {}, 'cache_files': None}
        for candidate in ('M1', 'M2', 'M3'):
            self.assertEqual(self.verdict(candidate, {'a': bare, 'b': {}}), ([], False))

    def test_rows_carry_the_numbers_the_verdict_was_made_from(self):
        row = memreport.evaluate({'a': entry(steady=1000 * MIB, m1=300 * MIB)})['M1']['rows']['a']
        self.assertEqual(row, {'estimate': 300 * MIB, 'base': 1000 * MIB, 'estimate_n': 3, 'base_n': 3,
                               'percent': 30.0, 'arm': 'both', 'meets': True})
        row = memreport.evaluate({'a': entry(peak=700 * MIB, m2=120 * MIB)})['M2']['rows']['a']
        self.assertEqual((row['estimate_n'], row['base_n'], row['arm']), (None, 3, 'fraction'))
        row = memreport.evaluate({'a': entry(m1=None)})['M1']['rows']['a']
        self.assertEqual((row['estimate'], row['estimate_n'], row['percent'], row['arm'], row['meets']),
                         (None, 0, None, None, False))


if __name__ == '__main__':
    unittest.main()
