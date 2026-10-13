#!/usr/bin/env python3
"""Tests of the baseline-against-candidate comparison: refusals, the pinned rule `m2`, the Markdown, the CLI.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
import memreport_compare as mc  # noqa: E402
from memreport_fixtures import MIB  # noqa: E402


def make_summary(series, identity='base', reps=3, settle=5, threads=4, env=None, allocator='system',
                 commit=None, dirty=0, sha='f' * 64):
    """A summary.json shaped like the driver's: `series` is {corpus: {metric: [values]}}."""
    corpora = {name: {'commit': commit or ('c' * 8 + name), 'dirty': dirty, 'workload': {'sha256': sha, 'requests': 8},
                      'metrics': {metric: memreport.summarize(values) for metric, values in metrics.items()}}
               for name, metrics in series.items()}
    build = {'build_identity': identity, 'git_sha': 'g' + identity, 'cpg_cache_version': 7, 'allocator': allocator}
    return {'provenance': {'reps': reps, 'settle': settle, 'timeout': 1800, 'bin': 'x', 'build': build,
                           'env': env or {}, 'threads': threads},
            'corpora': corpora, 'verdict': {}}


def mib(*values):
    return [value * MIB for value in values]


def compare(baseline, candidate, **kwargs):
    return mc.compare(baseline, candidate, 'm2', **kwargs)


def metric(result, key):
    return next(item for item in result['metrics'] if item['key'] == key)


def gate(base_values, cand_values, key='peak'):
    """The `peak` metric of a one-corpus comparison and its corpus row."""
    result = compare(make_summary({'a': {key: base_values}}), make_summary({'a': {key: cand_values}}, 'cand'))
    return metric(result, key)['corpora']['a']


class RefusalTests(unittest.TestCase):
    def setUp(self):
        self.series = {'a': {'peak': mib(1000, 1000, 1000)}, 'b': {'peak': mib(1000, 1000, 1000)}}
        self.baseline = make_summary(self.series)

    def refused(self, candidate, **kwargs):
        with self.assertRaises(ValueError) as caught:
            compare(self.baseline, candidate, **kwargs)
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
        message = self.refused(self.candidate(sha='e' * 64))
        self.assertIn('corpus a: workload sha256', message)

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

    def test_the_same_build_is_refused(self):
        self.assertIn('same build identity', self.refused(self.candidate(identity='base')))

    def test_the_same_build_is_compared_when_allowed_and_the_result_says_so(self):
        result = compare(self.baseline, self.candidate(identity='base'), allow_same_build=True)
        self.assertTrue(result['same_build'])
        self.assertFalse(compare(self.baseline, self.candidate())['same_build'])

    def test_a_run_with_no_build_identity_is_refused(self):
        candidate = self.candidate()
        candidate['provenance']['build'] = None
        self.assertIn('no build identity', self.refused(candidate))

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
        row = gate(mib(4000, 4000, 4000), mib(3900, 3900, 3900))    # 2.5 %, exactly 100 MiB
        self.assertEqual(row['outcome'], 'pass')
        self.assertEqual(row['effect'], 'absolute')

    def test_one_byte_under_100_mib_and_under_5_percent_is_not_an_effect(self):
        row = gate(mib(4000, 4000, 4000), [3900 * MIB + 1] * 3)
        self.assertEqual((row['outcome'], row['effect']), ('fail', 'none'))
        self.assertTrue(row['separated'])

    def test_a_separated_reduction_of_5_percent_passes_by_the_fraction_arm_alone(self):
        row = gate(mib(1000, 1000, 1000), mib(950, 950, 950))    # 50 MiB, exactly 5 %
        self.assertEqual((row['outcome'], row['effect']), ('pass', 'fraction'))

    def test_one_byte_under_5_percent_is_not_an_effect(self):
        row = gate(mib(1000, 1000, 1000), [950 * MIB + 1] * 3)
        self.assertEqual((row['outcome'], row['effect']), ('fail', 'none'))

    def test_both_arms_are_reported_as_both(self):
        self.assertEqual(gate(mib(1000, 1000, 1000), mib(800, 800, 800))['effect'], 'both')

    def test_ranges_that_touch_or_overlap_by_one_byte_are_not_separated(self):
        base = [900 * MIB, 950 * MIB, 1000 * MIB]
        touching = gate(base, [500 * MIB, 600 * MIB, 900 * MIB])
        overlapping = gate(base, [500 * MIB, 600 * MIB, 900 * MIB + 1])
        self.assertEqual((touching['outcome'], touching['separated']), ('fail', False))
        self.assertEqual((overlapping['outcome'], overlapping['separated']), ('fail', False))
        clear = gate(base, [500 * MIB, 600 * MIB, 900 * MIB - 1])
        self.assertEqual((clear['outcome'], clear['separated']), ('pass', True))

    def test_an_increase_is_a_fail_not_a_negative_pass(self):
        row = gate(mib(1000, 1000, 1000), mib(1100, 1100, 1100))
        self.assertEqual(row['outcome'], 'fail')
        self.assertEqual(row['reduction'], -100 * MIB)

    def test_two_runs_on_either_side_are_insufficient_not_a_pass_and_not_a_fail(self):
        self.assertEqual(gate(mib(1000, 1000), mib(500, 500, 500))['outcome'], 'insufficient runs')
        self.assertEqual(gate(mib(1000, 1000, 1000), mib(500, 500))['outcome'], 'insufficient runs')

    def test_three_runs_are_enough(self):
        self.assertEqual(gate(mib(1000, 1000, 1000), mib(500, 500, 500))['outcome'], 'pass')

    def test_the_reduction_is_baseline_median_minus_candidate_median(self):
        row = gate(mib(1000, 1100, 1200), mib(700, 800, 850))
        self.assertEqual(row['reduction'], 300 * MIB)

    def gate_verdict(self, rows):
        base = make_summary({name: {'peak': mib(*base_values)} for name, (base_values, _) in rows.items()})
        cand = make_summary({name: {'peak': mib(*cand_values)} for name, (_, cand_values) in rows.items()}, 'cand')
        return metric(compare(base, cand), 'peak')['verdict']

    def test_a_gate_that_passes_on_one_corpus_only_does_not_pass(self):
        self.assertEqual(self.gate_verdict({'a': ((1000,) * 3, (500,) * 3), 'b': ((1000,) * 3, (1000,) * 3)}), 'fail')
        self.assertEqual(self.gate_verdict({'a': ((1000,) * 3, (500,) * 3)}), 'fail')

    def test_a_gate_that_passes_on_two_corpora_passes_even_if_a_third_fails(self):
        rows = {'a': ((1000,) * 3, (500,) * 3), 'b': ((1000,) * 3, (500,) * 3), 'c': ((1000,) * 3, (1000,) * 3)}
        self.assertEqual(self.gate_verdict(rows), 'pass')

    def test_one_pass_and_one_corpus_short_of_runs_is_insufficient_runs_not_fail(self):
        rows = {'a': ((1000,) * 3, (500,) * 3), 'b': ((1000,) * 2, (500,) * 3)}
        self.assertEqual(self.gate_verdict(rows), 'insufficient runs')

    def test_one_pass_and_one_short_corpus_with_a_third_that_failed_can_still_not_reach_two(self):
        rows = {'a': ((1000,) * 3, (500,) * 3), 'b': ((1000,) * 2, (500,) * 3), 'c': ((1000,) * 3, (1000,) * 3)}
        self.assertEqual(self.gate_verdict(rows), 'insufficient runs')    # a + b could still make two
        rows['d'] = ((1000,) * 3, (1000,) * 3)
        self.assertEqual(self.gate_verdict(rows), 'insufficient runs')
        rows = {'a': ((1000,) * 3, (1000,) * 3), 'b': ((1000,) * 2, (500,) * 3), 'c': ((1000,) * 3, (1000,) * 3)}
        self.assertEqual(self.gate_verdict(rows), 'fail')    # at most one corpus can still pass


class NotWorseAndUnchangedTests(unittest.TestCase):
    def row(self, key, base_values, cand_values):
        return gate(base_values, cand_values, key=key)

    def test_a_candidate_median_of_exactly_1_05_times_the_baseline_is_ok(self):
        self.assertEqual(self.row('load_ms', [1000] * 3, [1050] * 3)['outcome'], 'ok')

    def test_just_above_1_05_is_worse(self):
        self.assertEqual(self.row('load_ms', [1000] * 3, [1050, 1051, 1051])['outcome'], 'worse')
        self.assertEqual(self.row('load_ms', [10**9] * 3, [1050000001] * 3)['outcome'], 'worse')

    def test_an_improvement_is_ok(self):
        self.assertEqual(self.row('build_ms', [1000] * 3, [10] * 3)['outcome'], 'ok')

    def test_not_worse_needs_no_particular_number_of_runs(self):
        self.assertEqual(self.row('load_ms', [1000], [1000])['outcome'], 'ok')

    def test_unchanged_at_exactly_1_mib_in_either_direction_is_ok(self):
        self.assertEqual(self.row('steady_heap', [100 * MIB] * 3, [101 * MIB] * 3)['outcome'], 'ok')
        self.assertEqual(self.row('steady_heap', [100 * MIB] * 3, [99 * MIB] * 3)['outcome'], 'ok')

    def test_unchanged_one_byte_past_1_mib_is_changed(self):
        self.assertEqual(self.row('steady_heap', [100 * MIB] * 3, [101 * MIB + 1] * 3)['outcome'], 'changed')
        self.assertEqual(self.row('steady_heap', [100 * MIB] * 3, [99 * MIB - 1] * 3)['outcome'], 'changed')

    def verdict(self, key, rows):
        base = make_summary({name: {key: values[0]} if values[0] is not None else {} for name, values in rows.items()})
        cand = make_summary({name: {key: values[1]} if values[1] is not None else {} for name, values in rows.items()},
                            'cand')
        return metric(compare(base, cand), key)

    def test_the_metric_is_ok_only_when_ok_on_every_corpus_with_both_series(self):
        rows = {'a': ([1000] * 3, [1000] * 3), 'b': ([1000] * 3, [2000] * 3)}
        self.assertEqual(self.verdict('load_ms', rows)['verdict'], 'worse')
        rows['b'] = ([1000] * 3, [1000] * 3)
        self.assertEqual(self.verdict('load_ms', rows)['verdict'], 'ok')

    def test_a_corpus_with_a_missing_series_is_listed_as_not_measured_and_does_not_decide(self):
        rows = {'a': ([1000] * 3, [1000] * 3), 'b': ([1000] * 3, None)}
        result = self.verdict('load_ms', rows)
        self.assertEqual(result['verdict'], 'ok')
        self.assertEqual(result['corpora']['b']['outcome'], 'not measured')
        self.assertEqual(result['corpora']['a']['outcome'], 'ok')

    def test_a_metric_absent_from_both_runs_is_not_measured_and_decides_nothing(self):
        base = make_summary({'a': {'peak': mib(1000) * 3}})
        cand = make_summary({'a': {'peak': mib(500) * 3}}, 'cand')
        result = compare(base, cand)
        for key in ('build_peak', 'load_ms', 'build_ms', 'steady_heap'):
            self.assertEqual(metric(result, key)['verdict'], 'not measured', key)
        self.assertEqual(metric(result, 'peak')['verdict'], 'fail')    # one corpus only: decided on its own terms

    def test_a_statistic_that_is_none_in_the_summary_counts_as_absent(self):
        base = make_summary({'a': {'load_ms': []}})
        cand = make_summary({'a': {'load_ms': [5]}}, 'cand')
        row = metric(compare(base, cand), 'load_ms')
        self.assertEqual((row['verdict'], row['corpora']['a']['outcome']), ('not measured', 'not measured'))

    def test_the_table_has_the_five_metrics_with_their_roles(self):
        result = compare(make_summary({'a': {}}), make_summary({'a': {}}, 'cand'))
        self.assertEqual([(item['key'], item['role']) for item in result['metrics']],
                         [('peak', 'gate'), ('build_peak', 'gate'), ('load_ms', 'not_worse'),
                          ('build_ms', 'not_worse'), ('steady_heap', 'unchanged')])

    def test_an_unknown_experiment_is_refused(self):
        with self.assertRaises(ValueError):
            mc.compare(make_summary({}), make_summary({}, 'cand'), 'm9')


class DifferenceTests(unittest.TestCase):
    def test_difference_is_candidate_minus_baseline_with_a_percentage_of_the_baseline(self):
        row = gate(mib(1000, 1000, 1000), mib(750, 750, 750))
        self.assertEqual(row['difference'], -250 * MIB)
        self.assertEqual(row['percent'], -25.0)

    def test_the_percentage_is_none_when_the_baseline_median_is_zero(self):
        row = gate([0, 0, 0], [5, 5, 5], key='load_ms')
        self.assertIsNone(row['percent'])
        self.assertEqual(row['difference'], 5)


class MarkdownTests(unittest.TestCase):
    def setUp(self):
        def series(scale, extra):
            return {'a': {'peak': mib(1000 - scale, 1010 - scale, 1020 - scale),
                          'build_peak': mib(2000 - scale, 2010 - scale, 2020 - scale),
                          'load_ms': [1000 + extra, 1100 + extra, 1200 + extra],
                          'build_ms': [20000, 21000, 22000],
                          'steady_heap': mib(300, 300, 300)},
                    'b': {'peak': mib(500 - scale, 500 - scale, 500 - scale)}}

        self.baseline = make_summary(series(0, 0), env={'PRISM_Y': '2'}, commit='1' * 40)
        self.candidate = make_summary(series(200, 10), 'cand', env={'PRISM_Y': '2'}, commit='1' * 40)
        self.same_build = make_summary(series(200, 10), 'base', env={'PRISM_Y': '2'}, commit='1' * 40)
        self.result = compare(self.baseline, self.candidate)
        self.result['sources'] = {'baseline': '/runs/base', 'candidate': '/runs/cand'}
        self.text = mc.to_markdown(self.result)

    def test_both_runs_provenance_is_shown(self):
        for word in ('`base`', '`gbase`', '`cand`', '`gcand`', 'allocator system', 'threads 4', '`PRISM_Y=2`',
                     '/runs/base', '/runs/cand', '3 repetitions', '5 s settle', '`' + '1' * 12 + '`'):
            self.assertIn(word, self.text)

    def test_the_rule_is_stated(self):
        for word in ('100 MiB', '5 %', 'at least 3 runs', 'at least 2 corpora', '1.05', '1 MiB'):
            self.assertIn(word, self.text)

    def test_a_metric_table_row_shows_medians_ranges_n_the_difference_and_the_outcome(self):
        row = [line for line in self.text.splitlines()
               if line.startswith('| a |') and '1010.0 (1000.0–1020.0, n=3)' in line]
        self.assertEqual(row, ['| a | 1010.0 (1000.0–1020.0, n=3) | 810.0 (800.0–820.0, n=3) | -200.0 (-19.8 %) | '
                               'pass (separated, reduction 200.0 MiB, effect both) |'])

    def test_milliseconds_are_integers(self):
        row = [line for line in self.text.splitlines() if '1100 (1000–1200, n=3)' in line]
        self.assertEqual(len(row), 1)
        self.assertIn('1110 (1010–1210, n=3) | +10 (+0.9 %) | ok |', row[0])

    def test_an_unavailable_value_is_n_a_and_never_zero(self):
        # corpus b has no first-build, load, build or heap series on either side
        for heading in ('First-build peak', 'Index load time', 'Index build time', 'Steady live heap'):
            table = self.text.split('## ' + heading)[1].split('\n## ')[0]
            self.assertIn('| b | n/a | n/a | n/a | not measured |', table, heading)
        # a missing candidate series against a present baseline one: n/a, not 0
        half = copy.deepcopy(self.candidate)
        half['corpora']['a']['metrics']['load_ms'] = None
        table = mc.to_markdown(compare(self.baseline, half)).split('## Index load time')[1].split('\n## ')[0]
        self.assertIn('| a | 1100 (1000–1200, n=3) | n/a | n/a | not measured |', table)

    def test_the_verdict_table_has_one_line_per_metric(self):
        verdict = self.text.split('## Verdict')[1].splitlines()
        rows = [line for line in verdict if line.startswith('| ') and not line.startswith('| Metric')]
        self.assertEqual(len(rows), 5)
        self.assertEqual(rows[0], '| Load peak (cache-loaded, through the index load) | gate | pass | pass: a, b |')
        self.assertEqual(rows[2], '| Index load time | not worse | ok | not measured: b; ok: a |')
        self.assertTrue(rows[4].startswith('| Steady live heap | unchanged | ok |'))

    def test_the_same_build_is_flagged(self):
        result = compare(self.baseline, self.same_build, allow_same_build=True)
        self.assertIn('same build', mc.to_markdown(result))
        self.assertNotIn('same build', self.text)


class CommandTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = Path(self._tmp.name)
        series = {'a': {'peak': mib(1000, 1000, 1000)}, 'b': {'peak': mib(1000, 1000, 1000)}}
        self.write('base', make_summary(series))
        self.write('cand', make_summary({n: {'peak': mib(500, 500, 500)} for n in series}, 'cand'))

    def write(self, name, summary):
        (self.tmp / name).mkdir(exist_ok=True)
        (self.tmp / name / 'summary.json').write_text(json.dumps(summary))

    def argv(self, *extra, baseline='base', candidate='cand'):
        return ['compare', '--experiment', 'm2', '--baseline', str(self.tmp / baseline),
                '--candidate', str(self.tmp / candidate), '--out', str(self.tmp / 'out.md')] + list(extra)

    def run_main(self, argv):
        import io
        import contextlib
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            memreport.main(argv)

    def test_it_writes_the_markdown_and_the_json(self):
        self.run_main(self.argv('--json', str(self.tmp / 'out.json')))
        text = (self.tmp / 'out.md').read_text()
        data = json.loads((self.tmp / 'out.json').read_text())
        self.assertIn('| Load peak (cache-loaded, through the index load) | gate | pass |', text)
        self.assertEqual(metric(data, 'peak')['verdict'], 'pass')
        self.assertEqual(data['sources'], {'baseline': str(self.tmp / 'base'), 'candidate': str(self.tmp / 'cand')})
        self.assertIn(str(self.tmp / 'base'), text)

    def test_a_refusal_exits_non_zero_names_the_difference_and_writes_nothing(self):
        self.write('other', make_summary({'a': {}}, 'cand'))
        with self.assertRaises(SystemExit) as caught:
            self.run_main(self.argv('--json', str(self.tmp / 'out.json'), candidate='other'))
        self.assertNotEqual(caught.exception.code, 0)
        self.assertIn('corpora differ', str(caught.exception))
        self.assertFalse((self.tmp / 'out.md').exists())
        self.assertFalse((self.tmp / 'out.json').exists())

    def test_the_same_build_needs_the_flag(self):
        self.write('same', make_summary({'a': {'peak': mib(1, 1, 1)}, 'b': {}}, 'base'))
        self.write('base2', make_summary({'a': {'peak': mib(1, 1, 1)}, 'b': {}}, 'base'))
        with self.assertRaises(SystemExit) as caught:
            self.run_main(self.argv(baseline='base2', candidate='same'))
        self.assertIn('same build identity', str(caught.exception))
        self.run_main(self.argv('--allow-same-build', baseline='base2', candidate='same'))
        self.assertTrue((self.tmp / 'out.md').exists())

    def test_a_run_directory_without_a_summary_is_named(self):
        (self.tmp / 'empty').mkdir()
        with self.assertRaises(SystemExit) as caught:
            self.run_main(self.argv(candidate='empty'))
        self.assertIn('empty', str(caught.exception))
        self.assertIn('summary.json', str(caught.exception))

    def test_an_unknown_experiment_is_refused_by_the_parser(self):
        argv = self.argv()
        argv[argv.index('m2')] = 'm9'
        with self.assertRaises(SystemExit) as caught:
            self.run_main(argv)
        self.assertNotEqual(caught.exception.code, 0)


if __name__ == '__main__':
    unittest.main()
