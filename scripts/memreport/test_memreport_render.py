#!/usr/bin/env python3
"""Tests of the Markdown readout.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
import memreport_render  # noqa: E402
from memreport_fixtures import MIB, sample_summary  # noqa: E402


def table_row(text, *cells):
    """The one wide table line of `text` that starts with `| cells[0] | cells[1] ...`."""
    prefix = '| ' + ' | '.join(cells) + ' |'
    rows = [line for line in text.splitlines() if line.startswith(prefix) and line.count('|') > 5]
    assert len(rows) == 1, (prefix, rows)
    return rows[0]


class DecisionTableTests(unittest.TestCase):
    def setUp(self):
        self.text = memreport_render.render(sample_summary())

    def test_every_candidate_and_corpus_row_shows_estimate_base_percentage_arm_and_n(self):
        self.assertEqual(table_row(self.text, 'M1', 'big'),
                         '| M1 | big | 300.0 | 1000.0 | 30.0 % | both | 3 / 3 |  |')
        self.assertEqual(table_row(self.text, 'M2', 'big'),
                         '| M2 | big | 120.0 | 700.0 | 17.1 % | fraction | file / 3 |  |')
        self.assertEqual(table_row(self.text, 'M3', 'big'),
                         '| M3 | big | 60.0 | 1000.0 | 6.0 % | — | 3 / 3 |  |')

    def test_an_unavailable_estimate_is_n_a_in_every_cell_never_zero(self):
        row = table_row(self.text, 'M1', 'small')
        self.assertTrue(row.startswith('| M1 | small | n/a | 1000.0 | n/a | n/a | 0 / 1 |'), row)
        row = table_row(self.text, 'M2', 'small')
        self.assertTrue(row.startswith('| M2 | small | n/a | 700.0 | n/a | n/a | file / 1 |'), row)
        self.assertNotIn('| 0.0 |', row)

    def test_a_row_with_fewer_runs_than_requested_is_marked(self):
        self.assertIn('low n (3 repetitions requested)', table_row(self.text, 'M3', 'small'))
        self.assertNotIn('low n', table_row(self.text, 'M3', 'big'))
        self.assertNotIn('low n', table_row(self.text, 'M2', 'big'))   # one file measurement is not a series

    def test_the_mark_follows_the_requested_repetitions(self):
        text = memreport_render.render(sample_summary(reps=2))
        self.assertNotIn('low n', table_row(text, 'M3', 'big'))
        self.assertNotIn('low n', table_row(text, 'M1', 'big'))   # 2 runs requested, 2 made
        self.assertIn('low n (2 repetitions requested)', table_row(text, 'M3', 'small'))

    def test_the_per_candidate_line_and_the_definitions_follow(self):
        self.assertIn('| M1 | big | no |', self.text)
        self.assertIn('| M3 | — | no |', self.text)
        self.assertIn('measured live tree-sitter heap after the workload and settle', self.text)
        self.assertIn('size of the CPG cache file, an upper bound', self.text)
        self.assertIn('estimated heap of repeated node strings', self.text)

    def test_two_corpora_meeting_the_rule_advance(self):
        summary = sample_summary()
        summary['corpora']['small']['metrics']['m1'] = {'min': 250 * MIB, 'median': 250 * MIB, 'max': 250 * MIB, 'n': 3}
        summary['verdict'] = memreport.evaluate(summary['corpora'])
        text = memreport_render.render(summary)
        self.assertIn('| M1 | big, small | yes |', text)
        self.assertIn('| 250.0 | 1000.0 | 25.0 % | both |', table_row(text, 'M1', 'small'))

    def test_the_arms_are_named(self):
        summary = sample_summary()
        big = summary['corpora']['big']
        big['metrics']['m1'] = {'min': 250 * MIB, 'median': 250 * MIB, 'max': 250 * MIB, 'n': 3}
        big['metrics']['steady']['median'] = 5000 * MIB            # 5 %: absolute arm only
        summary['verdict'] = memreport.evaluate(summary['corpora'])
        self.assertIn('| 250.0 | 5000.0 | 5.0 % | absolute | 3 / 3 |',
                      table_row(memreport_render.render(summary), 'M1', 'big'))


class TableTests(unittest.TestCase):
    def setUp(self):
        self.text = memreport_render.render(sample_summary())

    def test_footprint_columns_are_renamed_and_extended(self):
        self.assertNotIn('Warm-load peak', self.text)
        self.assertIn('Peak through index load', self.text)
        self.assertIn('Steady live heap', self.text)
        self.assertIn('Observed load overshoot (lower bound; 0 is uninformative)', self.text)
        row = table_row(self.text, 'big', '1000.0 (1000.0–1000.0, n=3)', '900.0 (900.0–900.0, n=3)')
        self.assertIn('700.0 (700.0–700.0, n=3)', row)
        self.assertTrue(row.endswith('| 800.0 (800.0–800.0, n=3) | 710.0 |'), row)   # cold peak, first-build peak

    def test_a_kind_with_no_admissible_run_is_n_a_not_zero(self):
        row = table_row(self.text, 'small', '1000.0 (1000.0–1000.0, n=1)')
        self.assertTrue(row.endswith('| n/a | n/a |'), row)

    def test_attribution_shows_measured_tree_heap_and_what_is_not_attributed(self):
        self.assertNotIn('low–high', self.text)
        self.assertIn('Tree heap, measured (M1)', self.text)
        big = table_row(self.text, 'big', '10', '4.0', '1000')
        # M1 | CPG nodes / edges | M3 | ratio | index heap | attributed (20 + 100) | remainder (300 - 120)
        self.assertEqual(big, '| big | 10 | 4.0 | 1000 | 300.0 (300.0–300.0, n=3) | 500 / 900 | '
                              '60.0 (60.0–60.0, n=3) | 0.400 | 300.0 (300.0–300.0, n=3) | 120.0 | 180.0 |')
        small = table_row(self.text, 'small', '10', '4.0', '1000')
        self.assertIn('| n/a | 500 / 900 |', small)       # tree heap unavailable

    def test_missing_attribution_fields_render_as_n_a(self):
        summary = sample_summary()
        summary['corpora']['small']['attribution'] = {}
        summary['corpora']['big']['attribution'] = None
        text = memreport_render.render(summary)
        row = table_row(text, 'small', 'n/a', 'n/a', 'n/a')
        self.assertTrue(row.endswith('| n/a | n/a |'), row)     # attributed and not attributed
        self.assertIn('| n/a | n/a | n/a | n/a | n/a / n/a |', row)
        self.assertNotIn('| 0.0 |', text)

    def test_unavailable_index_heap_leaves_the_remainder_n_a(self):
        summary = sample_summary()
        summary['corpora']['big']['metrics']['index_heap'] = None
        text = memreport_render.render(summary)
        row = table_row(text, 'big', '10', '4.0', '1000')
        self.assertTrue(row.endswith('| n/a | 120.0 | n/a |'), row)

    def test_excluded_runs_are_listed_with_their_reason(self):
        self.assertIn('| small | workload | 2 | inadmissible | workload had 1 errors |', self.text)
        clean = copy.deepcopy(sample_summary())
        for entry in clean['corpora'].values():
            entry['inadmissible'] = []
        self.assertTrue(memreport_render.render(clean).rstrip().endswith('None.'))

    def test_the_header_says_what_is_measured_and_what_is_estimated(self):
        self.assertIn('coarse triage', self.text)
        self.assertIn('tree-sitter tree heap (M1) are measured', self.text)
        self.assertIn('string figures (M3, "attributed") are estimates', self.text)
        self.assertNotIn('low-high', self.text)

    def test_render_is_reachable_from_the_driver_module(self):
        self.assertIs(memreport.render, memreport_render.render)


class LabelTests(unittest.TestCase):
    def setUp(self):
        self.text = memreport_render.render(sample_summary())

    def test_the_footprint_columns_say_what_each_run_did_with_the_cache(self):
        self.assertIn('| Corpus | Steady state (cache-loaded) |', self.text)
        self.assertIn('| Cold peak (no cache write) |', self.text)
        self.assertIn('| First-build peak (writes the cache, n=1) |', self.text)
        self.assertNotIn('| Steady state |', self.text)
        self.assertNotIn('| Cold peak |', self.text)

    def test_the_first_build_peak_is_the_priming_runs_last_stage_peak_or_n_a(self):
        big = table_row(self.text, 'big', '1000.0 (1000.0–1000.0, n=3)')
        self.assertTrue(big.endswith('| 710.0 |'), big)       # the canned priming run's after_settle peak
        small = table_row(self.text, 'small', '1000.0 (1000.0–1000.0, n=1)')
        self.assertTrue(small.endswith('| n/a | n/a |'), small)   # cold peak and first-build peak

    def test_the_m1_definition_calls_it_an_upper_bound_and_names_the_query_cache(self):
        self.assertIn('an upper bound on what dropping the trees frees (it includes a fixed ~54 KB query cache, '
                      'and trees that are re-parsed later would come back)', self.text)


class UnknownCommitTests(unittest.TestCase):
    def test_a_commit_or_dirty_count_git_could_not_give_is_n_a_not_blank_or_zero(self):
        summary = sample_summary()
        summary['corpora']['small'].update(commit=None, dirty=None)
        text = memreport_render.render(summary)
        self.assertIn('| small | n/a | n/a | 1/0/1/0 |', text)
        self.assertIn('| big | `aaaaaaaaaaaa` | 0 | 3/3/3/3 |', text)


class ProvenanceTests(unittest.TestCase):
    def summary(self, build, env):
        summary = sample_summary()
        summary['provenance'].update(build=build, env=env)
        return summary

    def test_provenance_shows_the_dirty_flag_and_the_environment(self):
        text = memreport_render.render(self.summary(
            {'build_identity': 'b', 'binary_input_dirty': True},
            {'PRISM_NAV_EDGE_CACHE_LOAD_DIRTY': '1', 'PRISM_A': 'x'}))
        self.assertIn('binary_input_dirty true', text)
        self.assertIn('`PRISM_A=x`, `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY=1`', text)

    def test_provenance_prints_the_thread_count_of_the_measured_process(self):
        summary = sample_summary()
        self.assertEqual(summary['provenance']['threads'], 4)
        self.assertIn('- Worker threads (rayon pool) of the measured process: 4.', memreport_render.render(summary))
        summary['provenance']['threads'] = None
        self.assertIn('measured process: n/a.', memreport_render.render(summary))

    def test_empty_and_unknown_values_are_distinguished(self):
        text = memreport_render.render(self.summary({'build_identity': 'b', 'binary_input_dirty': False}, {}))
        self.assertIn('binary_input_dirty false', text)
        self.assertIn('Environment of the measured process: none.', text)
        text = memreport_render.render(self.summary({}, None))
        self.assertIn('binary_input_dirty n/a', text)
        self.assertIn('process: n/a.', text)
        self.assertIn('build identity `n/a`, git `n/a`, CPG cache version n/a, allocator n/a', text)


if __name__ == '__main__':
    unittest.main()
