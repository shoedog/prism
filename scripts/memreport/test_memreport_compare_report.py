#!/usr/bin/env python3
"""Tests of the comparison report (Markdown and JSON) and of the `compare` command on run directories.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import contextlib
import copy
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
import memreport_compare as mc  # noqa: E402
from memreport_fixtures import MIB, make_summary, mib, write_run  # noqa: E402


def compare(baseline, candidate, **kwargs):
    return mc.compare(baseline, candidate, 'm2', **kwargs)


def metric(result, key):
    return next(item for item in result['metrics'] if item['key'] == key)


def section(text, heading):
    return text.split('## ' + heading)[1].split('\n## ')[0]


class MarkdownTests(unittest.TestCase):
    def setUp(self):
        def series(scale, extra):
            return {'a': {'peak': mib(1000 - scale, 1010 - scale, 1020 - scale),
                          'build_peak': mib(2000 - scale, 2010 - scale, 2020 - scale),
                          'load_ms': [1000 + extra, 1100 + extra, 1200 + extra],
                          'build_ms': [20000, 21000, 22000],
                          'steady_heap': mib(300, 300, 300)},
                    'b': {'peak': mib(500 - scale, 500 - scale, 500 - scale)}}

        self.baseline = make_summary(series(0, 0), env={'PRISM_Y': '2'}, commit='1' * 40, loads={'a': [2.0, 4.0]})
        self.candidate = make_summary(series(200, 10), 'cand', env={'PRISM_Y': '2'}, commit='1' * 40,
                                      loads={'a': [30.0, 50.5], 'b': [40.0]})
        self.same_build = make_summary(series(200, 10), 'base', env={'PRISM_Y': '2'}, commit='1' * 40)
        self.result = compare(self.baseline, self.candidate)
        self.result['sources'] = {'baseline': '/runs/base', 'candidate': '/runs/cand'}
        self.text = mc.to_markdown(self.result)

    def test_both_runs_provenance_is_shown(self):
        for word in ('`base`', '`gbase`', '`cand`', '`gcand`', 'allocator system', 'threads 4', '`PRISM_Y=2`',
                     '(base)', '(cand)', '3 repetitions', '5 s settle', '`' + '1' * 12 + '`',
                     'binary_input_dirty false', 'platform macos/aarch64'):
            self.assertIn(word, self.text)

    def test_run_directories_are_named_by_their_last_component_only(self):
        self.assertNotIn('/runs', self.text)
        self.assertEqual(self.result['sources'], {'baseline': '/runs/base', 'candidate': '/runs/cand'})    # data

    def test_host_load_is_shown_per_side_with_median_and_maximum(self):
        self.assertIn('host 1-minute load average median 3.0, max 4.0 over 2 runs', self.text)
        self.assertIn('median 40.0, max 50.5 over 3 runs', self.text)
        plain = mc.to_markdown(compare(self.baseline, self.same_build, allow_same_build=True))
        self.assertIn('host 1-minute load average n/a', plain)

    def test_the_rule_is_stated_including_the_gate_verdict_words_and_the_overlap_clause(self):
        for word in ('100 MiB', '5 %', 'at least 3 runs', 'at least 2 corpora', '1.05', '1 MiB', 'undecided',
                     'one-sided', 'incomplete', 'ranges do not overlap', 'unresolved', 'fewer than 3 runs'):
            self.assertIn(word, self.text)

    def test_the_rule_is_carried_in_the_json_as_data(self):
        rule = self.result['rule']
        self.assertEqual(rule['constants']['gate_absolute_bytes'], 100 * MIB)
        self.assertEqual(rule['constants']['not_worse_ratio'], 1.05)
        self.assertEqual(rule['constants']['gate_min_runs'], 3)
        self.assertTrue(any('undecided' in line for line in rule['lines']))
        self.assertTrue(any('unresolved' in line and 'above the band' in line for line in rule['lines']))
        json.dumps(self.result)

    def test_a_metric_table_row_shows_medians_ranges_n_the_difference_and_the_outcome(self):
        row = [line for line in self.text.splitlines()
               if line.startswith('| a |') and '1010.0 (1000.0–1020.0, n=3)' in line]
        self.assertEqual(row, ['| a | 1010.0 (1000.0–1020.0, n=3) | 810.0 (800.0–820.0, n=3) | -200.0 (-19.8 %) | '
                               'pass (separated, reduction 200.0 MiB, effect both) |'])

    def test_milliseconds_are_integers_and_the_ratio_is_printed(self):
        row = [line for line in self.text.splitlines() if '1100 (1000–1200, n=3)' in line]
        self.assertEqual(len(row), 1)
        self.assertIn('1110 (1010–1210, n=3) | +10 (+0.9 %) | ok (×1.009) |', row[0])

    def test_an_unavailable_value_is_n_a_and_never_zero(self):
        for heading in ('First-build peak', 'Index load time', 'Index build time', 'Steady live heap'):
            self.assertIn('| b | n/a | n/a | n/a | not measured |', section(self.text, heading), heading)
        half = copy.deepcopy(self.candidate)
        half['corpora']['a']['metrics']['load_ms'] = None
        table = section(mc.to_markdown(compare(self.baseline, half)), 'Index load time')
        self.assertIn('| a | 1100 (1000–1200, n=3) | n/a | n/a | one-sided (series on the baseline side only) |',
                      table)

    def test_the_verdict_table_has_one_line_per_metric(self):
        rows = [line for line in section(self.text, 'Verdict').splitlines()
                if line.startswith('| ') and not line.startswith('| Metric')]
        rows = [line for line in rows if '---' not in line]
        self.assertEqual(len(rows), 5)
        self.assertEqual(rows[0], '| Load peak (cache-loaded, through the index load) | gate | pass | pass: a, b |')
        self.assertEqual(rows[2], '| Index load time | not worse | ok (largest ratio ×1.009) | '
                                  'not measured: b; ok: a |')
        self.assertEqual(rows[3], '| Index build time | not worse | ok (largest ratio ×1.000) | '
                                  'not measured: b; ok: a |')
        self.assertTrue(rows[4].startswith('| Steady live heap | not higher | ok |'))

    def test_the_same_build_is_flagged(self):
        result = compare(self.baseline, self.same_build, allow_same_build=True)
        self.assertIn('same build', mc.to_markdown(result))
        self.assertNotIn('same build', self.text)


class ExcludedAndWarningTests(unittest.TestCase):
    def setUp(self):
        series = {'a': {'peak': mib(1000, 1000, 1000)}, 'b': {'peak': mib(1000, 1000, 1000)}}
        self.lost = [{'kind': 'warm', 'rep': 2, 'status': 'timeout', 'reason': 'timeout'}]
        self.baseline = make_summary(series)
        self.candidate = make_summary(series, 'cand', excluded={'b': self.lost})

    def test_the_excluded_runs_of_each_side_are_listed_with_corpus_kind_repetition_and_reason(self):
        result = compare(self.baseline, self.candidate)
        self.assertEqual(result['excluded'], {'baseline': [], 'candidate': [
            {'corpus': 'b', 'kind': 'warm', 'rep': 2, 'reason': 'timeout'}]})
        text = mc.to_markdown(result)
        excluded = section(text, 'Excluded runs')
        self.assertIn('### Baseline\n\nNone.', excluded)
        self.assertIn('| b | warm | 2 | timeout |', excluded.split('### Candidate')[1])

    def test_a_corpus_excluded_whole_for_another_build_is_explained(self):
        reason = "build identity differs from the first corpus's (cand)"
        candidate = make_summary({'a': {'peak': mib(500, 500, 500)}, 'b': {}}, 'cand',
                                 excluded={'b': [{'kind': 'warm', 'rep': 'all 3', 'reason': reason}]})
        text = mc.to_markdown(compare(self.baseline, candidate))
        self.assertIn('| b | warm | all 3 | %s |' % reason, text)

    def test_a_dirty_binary_gets_a_bold_warning_on_either_side(self):
        for kwargs in ({'binary_dirty': True}, {}):
            base = make_summary({'a': {}}, **kwargs)
            cand = make_summary({'a': {}}, 'cand', **({} if kwargs else {'binary_dirty': True}))
            text = mc.to_markdown(compare(base, cand))
            self.assertIn('binary_input_dirty true', text)
            self.assertRegex(text, r'\*\*The (baseline|candidate) binary was built from a tree with uncommitted')
        self.assertNotIn('uncommitted', mc.to_markdown(compare(self.baseline, self.candidate)))

    def test_a_corpus_with_uncommitted_files_gets_a_bold_warning_even_when_the_counts_are_equal(self):
        series = {'a': {}, 'b': {}}
        result = compare(make_summary(series, dirty=2), make_summary(series, 'cand', dirty=2))
        self.assertIn('**Corpus a has 2 uncommitted files: equal counts are not equal trees.**',
                      mc.to_markdown(result))
        self.assertIn('Corpus b has 2 uncommitted files', ' '.join(result['warnings']))


class MoreWarningTests(unittest.TestCase):
    series = {'a': {'peak': mib(1000, 1000, 1000)}, 'b': {'peak': mib(1000, 1000, 1000)}}

    def test_a_build_directory_left_behind_on_either_side_gets_a_bold_warning(self):
        for side in ('baseline', 'candidate'):
            left = {'b': [2, 3]}
            base = make_summary(self.series, dirs_left=left if side == 'baseline' else None)
            cand = make_summary(self.series, 'cand', dirs_left=left if side == 'candidate' else None)
            result = compare(base, cand)
            text = mc.to_markdown(result)
            self.assertIn('**The %s run left the cache directory of build repetitions 2, 3 of corpus b behind '
                          '(cache_dir_removed false).**' % side, text)
            self.assertTrue(any('corpus b' in line for line in result['warnings']))
        self.assertNotIn('cache_dir_removed', mc.to_markdown(compare(make_summary(self.series),
                                                                      make_summary(self.series, 'cand'))))

    def test_one_corpus_given_under_two_names_gets_a_bold_warning(self):
        both = make_summary(self.series, commit='1' * 40, paths={'a': '/c/same', 'b': '/c/same'})
        text = mc.to_markdown(compare(both, make_summary(self.series, 'cand', commit='1' * 40,
                                                         paths={'a': '/c/same', 'b': '/c/same'})))
        self.assertIn('**Corpora a and b of the baseline are the same repository (same path and commit): they '
                      'count as two corpora in the gate.**', text)
        self.assertIn('of the candidate are the same repository', text)

    def test_two_corpora_with_the_same_commit_at_different_paths_are_not_flagged(self):
        both = make_summary(self.series, commit='1' * 40)
        self.assertNotIn('same repository', mc.to_markdown(compare(both, make_summary(self.series, 'cand',
                                                                                      commit='1' * 40))))

    def test_run_directories_with_the_same_last_component_are_shown_with_their_parent(self):
        result = compare(make_summary(self.series), make_summary(self.series, 'cand'))
        result['sources'] = {'baseline': '/x/old/run-1', 'candidate': '/x/new/run-1'}
        text = mc.to_markdown(result)
        self.assertIn('Baseline (old/run-1)', text)
        self.assertIn('Candidate (new/run-1)', text)
        self.assertNotIn('/x/', text)
        result['sources'] = {'baseline': '/x/old/run-1', 'candidate': '/x/new/run-2'}
        text = mc.to_markdown(result)
        self.assertIn('Baseline (run-1)', text)
        self.assertIn('Candidate (run-2)', text)


class RunDirectoryTests(unittest.TestCase):
    """`compare` on run directories: it rescoring them from raw/, never reading summary.json."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = Path(self._tmp.name)

    def run_dirs(self, **kwargs):
        series = {'a': [1000, 1000, 1000], 'b': [1000, 1000, 1000]}
        write_run(self.tmp / 'base', series, **dict(kwargs, identity='base'))
        write_run(self.tmp / 'cand', {n: [500, 500, 500] for n in series}, **dict(kwargs, identity='cand'))

    def argv(self, *extra, baseline='base', candidate='cand'):
        return ['compare', '--experiment', 'm2', '--baseline', str(self.tmp / baseline),
                '--candidate', str(self.tmp / candidate), '--out', str(self.tmp / 'out.md')] + list(extra)

    def run_main(self, argv):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            memreport.main(argv)

    def test_it_writes_the_markdown_and_the_json(self):
        self.run_dirs(loads={'a': [1.0] * 3, 'b': [2.0] * 3})
        self.run_main(self.argv('--json', str(self.tmp / 'out.json')))
        text = (self.tmp / 'out.md').read_text()
        data = json.loads((self.tmp / 'out.json').read_text())
        self.assertIn('| Load peak (cache-loaded, through the index load) | gate | pass |', text)
        self.assertEqual(metric(data, 'peak')['verdict'], 'pass')
        self.assertEqual(data['sources'], {'baseline': str(self.tmp / 'base'), 'candidate': str(self.tmp / 'cand')})
        self.assertNotIn(str(self.tmp), text)
        self.assertIn('median 1.5, max 2.0 over 6 runs', text)    # the recorded host load of the admitted runs

    def test_a_stale_summary_json_is_not_trusted_and_none_is_needed(self):
        self.run_dirs()
        write_run(self.tmp / 'stale-cand', {'a': [500] * 3, 'b': [500] * 3}, identity='cand', with_summary=True)
        self.run_main(self.argv(candidate='stale-cand'))
        self.assertIn('pass', (self.tmp / 'out.md').read_text())

    def test_todays_rules_are_applied_to_the_saved_evidence(self):
        self.run_dirs()
        write_run(self.tmp / 'bad-cand', {'a': [500] * 3, 'b': [500] * 3}, identity='cand',
                  broken={'a': (1, 2, 3)})    # every warm repetition of a says the cache missed
        self.run_main(self.argv('--json', str(self.tmp / 'out.json'), candidate='bad-cand'))
        data = json.loads((self.tmp / 'out.json').read_text())
        self.assertEqual(metric(data, 'peak')['verdict'], 'undecided')    # only b still passes
        self.assertEqual(len(data['excluded']['candidate']), 3)

    def test_a_directory_without_raw_or_run_json_is_refused_and_named(self):
        self.run_dirs()
        (self.tmp / 'empty').mkdir()
        (self.tmp / 'cand-summary-only').mkdir()
        (self.tmp / 'cand-summary-only' / 'summary.json').write_text('{}')
        for name in ('empty', 'cand-summary-only', 'missing'):
            with self.assertRaises(SystemExit) as caught:
                self.run_main(self.argv(candidate=name))
            self.assertIn(name, str(caught.exception))
        self.assertFalse((self.tmp / 'out.md').exists())

    def test_a_refusal_exits_non_zero_names_the_difference_and_writes_nothing(self):
        self.run_dirs()
        write_run(self.tmp / 'other', {'a': [500] * 3}, identity='cand')
        with self.assertRaises(SystemExit) as caught:
            self.run_main(self.argv('--json', str(self.tmp / 'out.json'), candidate='other'))
        self.assertNotEqual(caught.exception.code, 0)
        self.assertIn('corpora differ', str(caught.exception))
        self.assertFalse((self.tmp / 'out.md').exists())
        self.assertFalse((self.tmp / 'out.json').exists())

    def test_the_same_build_needs_the_flag(self):
        write_run(self.tmp / 'base', {'a': [1000] * 3}, identity='x')
        write_run(self.tmp / 'again', {'a': [1000] * 3}, identity='x')
        with self.assertRaises(SystemExit) as caught:
            self.run_main(self.argv(candidate='again'))
        self.assertIn('same build identity', str(caught.exception))
        self.run_main(self.argv('--allow-same-build', candidate='again'))
        self.assertTrue((self.tmp / 'out.md').exists())

    def test_one_directory_against_itself_is_refused_with_or_without_the_flag(self):
        self.run_dirs()
        (self.tmp / 'link').symlink_to(self.tmp / 'base')
        for candidate in ('base', 'link', 'base/../base'):
            for extra in ((), ('--allow-same-build',)):
                with self.assertRaises(SystemExit) as caught:
                    self.run_main(self.argv(*extra, candidate=candidate))
                self.assertIn('same directory', str(caught.exception))
        self.assertFalse((self.tmp / 'out.md').exists())

    def test_an_unknown_experiment_is_refused_by_the_parser(self):
        self.run_dirs()
        argv = self.argv()
        argv[argv.index('m2')] = 'm9'
        with self.assertRaises(SystemExit) as caught:
            self.run_main(argv)
        self.assertNotEqual(caught.exception.code, 0)


if __name__ == '__main__':
    unittest.main()
