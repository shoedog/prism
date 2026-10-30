#!/usr/bin/env python3
"""End-to-end tests of `run --kinds` and of the `build` kind against the fake prism-mcp.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import json
import os
import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
import test_memreport_run as base  # noqa: E402
from memreport_fixtures import MIB, canned, fake_prism, stage_named  # noqa: E402

FOUR = ['warm', 'cold', 'workload', 'refresh']


def all_reports(build=True):
    """Fake reports for every kind; a `build` one is what the fake prints for a run into an empty cache."""
    return {kind: canned(kind) for kind in FOUR + (['build'] if build else [])}


class Kinds(base.Harness):
    def prepare(self, build=True, reports=None, **cfg):
        self.corpus('a')
        self.fake(emit={'a': 4}, reports=reports or all_reports(build), **cfg)
        self.gen('a')

    def run_kinds(self, kinds, reps=2):
        argv = self.run_args('a', reps=reps) + ([] if kinds is None else ['--kinds', kinds])
        return base.quiet(memreport.main, argv)

    def raw_names(self):
        return sorted(p.name for p in (self.out / 'raw' / 'a').iterdir())

    def child_calls(self, label='warm'):
        path = self.tools / ('calls-a-' + label)
        return int(path.read_text()) if path.exists() else 0


class SelectionTests(Kinds):
    def test_the_default_runs_todays_four_kinds_and_writes_todays_files(self):
        self.prepare(build=False)
        self.run_kinds(None, reps=1)
        self.assertEqual(self.raw_names(), ['cold-1.json', 'corpus.json', 'prime.json', 'refresh-1.json',
                                            'warm-1.json', 'workload-1.json'])
        self.assertEqual(json.loads((self.out / 'run.json').read_text())['kinds'], FOUR)
        self.assertFalse((self.out / 'cache-build').exists())
        self.assertNotIn('kinds', self.summary()['provenance'])
        self.assertEqual(sorted(self.summary()['corpora']['a']['admissible']), sorted(FOUR))

    def test_naming_all_four_in_any_order_is_the_default(self):
        self.prepare(build=False)
        # The summary records the host's load average before each run; it must not differ between the two runs.
        with mock.patch.object(memreport, 'load_average', return_value=[1.0, 2.0, 3.0]):
            self.run_kinds(None)
            default = (self.out / 'summary.json').read_text()
            self.out = self.out.with_name('out2')
            self.run_kinds('refresh,workload,cold,warm')
        self.assertEqual((self.out / 'summary.json').read_text(), default)
        self.assertEqual(json.loads((self.out / 'run.json').read_text())['kinds'], FOUR)

    def test_every_result_records_the_hosts_load_average_as_three_numbers(self):
        self.prepare(build=False)
        self.run_kinds('warm,cold', reps=1)
        for name in ('warm-1', 'cold-1'):
            load = json.loads((self.out / 'raw' / 'a' / (name + '.json')).read_text())['host_before']['loadavg']
            self.assertEqual(len(load), 3, name)
            self.assertTrue(all(type(value) is float and value >= 0 for value in load), name)
        self.assertEqual(len(self.summary()['corpora']['a']['load_avg_1m']), 2)

    def test_a_host_without_a_load_average_still_runs_and_records_none(self):
        self.prepare(build=False)
        with mock.patch.object(os, 'getloadavg', side_effect=OSError('no load average')):
            self.run_kinds('warm', reps=1)
        stored = json.loads((self.out / 'raw' / 'a' / 'warm-1.json').read_text())
        self.assertIsNone(stored['host_before']['loadavg'])
        self.assertEqual(self.summary()['corpora']['a']['load_avg_1m'], [])
        self.assertEqual(self.summary()['corpora']['a']['admissible']['warm'], 1)

    def test_a_subset_runs_only_those_kinds(self):
        self.prepare()
        self.run_kinds('build,warm', reps=2)
        self.assertEqual(self.raw_names(), ['build-1.json', 'build-2.json', 'corpus.json', 'prime.json',
                                            'warm-1.json', 'warm-2.json'])
        self.assertEqual(json.loads((self.out / 'run.json').read_text())['kinds'], ['warm', 'build'])
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['admissible'], {'warm': 2, 'cold': 0, 'workload': 0, 'refresh': 0, 'build': 2})

    def test_no_priming_run_unless_a_cached_kind_is_selected(self):
        self.prepare()
        self.run_kinds('cold,build', reps=2)
        self.assertEqual(self.raw_names(), ['build-1.json', 'build-2.json', 'cold-1.json', 'cold-2.json',
                                            'corpus.json'])
        self.assertEqual(self.child_calls('warm'), 2)    # the two builds, no priming child
        self.assertFalse((self.out / 'cache').exists())
        self.assertEqual(self.summary()['corpora']['a']['cache_files'],
                         {'cpg_cache_bytes': None, 'call_edge_sidecar_bytes': None})
        self.assertIsNone(self.summary()['corpora']['a']['first_build_peak'])

    def test_each_cached_kind_alone_primes_once(self):
        self.prepare()
        for kinds in ('warm', 'workload', 'refresh'):
            self.tools = self.tmp / ('tools-' + kinds)
            self.tools.mkdir()
            self.fake(emit={'a': 4}, reports=all_reports())
            self.out = self.tmp / ('out-' + kinds)
            self.run_kinds(kinds, reps=1)
            self.assertIn('prime.json', self.raw_names(), kinds)
            self.assertEqual(self.child_calls('warm'), 1 + (kinds == 'warm'), kinds)

    def test_unknown_repeated_or_empty_kinds_are_refused_before_anything_runs(self):
        self.prepare()
        for kinds in ('bogus', 'warm,bogus', 'warm,warm', 'build,cold,build', '', 'warm,,cold', ',', 'Warm',
                      ' warm'):
            with self.assertRaises(SystemExit, msg=repr(kinds)) as caught:
                self.run_kinds(kinds)
            self.assertIn('--kinds', str(caught.exception))
            self.assertFalse(self.out.exists(), kinds)
            self.assertEqual([p.name for p in self.tools.glob('calls-*')], [], kinds)

    def test_the_refusal_names_the_offending_kind(self):
        self.prepare()
        with self.assertRaises(SystemExit) as caught:
            self.run_kinds('warm,bogus')
        self.assertIn("'bogus' is not a kind", str(caught.exception))
        with self.assertRaises(SystemExit) as caught:
            self.run_kinds('cold,cold')
        self.assertIn('cold is listed twice', str(caught.exception))


class RescoreTests(Kinds):
    def test_summarize_rescores_a_subset_run_to_the_summary_the_run_wrote(self):
        self.prepare()
        self.run_kinds('warm,build')
        written = (self.out / 'summary.json').read_text()
        (self.out / 'summary.json').unlink()
        base.quiet(memreport.main, ['summarize', '--run', str(self.out)])
        self.assertEqual((self.out / 'summary.json').read_text(), written)

    def test_a_run_json_from_before_kinds_existed_still_summarizes(self):
        self.prepare(build=False)
        self.run_kinds(None)
        written = (self.out / 'summary.json').read_text()
        settings = json.loads((self.out / 'run.json').read_text())
        del settings['kinds']
        (self.out / 'run.json').write_text(json.dumps(settings))
        (self.out / 'summary.json').unlink()
        base.quiet(memreport.main, ['summarize', '--run', str(self.out)])
        self.assertEqual((self.out / 'summary.json').read_text(), written)

    def test_the_readout_of_a_subset_run_renders_with_n_a_for_what_was_not_run(self):
        self.prepare()
        self.run_kinds('warm,build')
        base.quiet(memreport.main, ['render', '--run', str(self.out), '--out', str(self.tmp / 'readout.md')])
        text = (self.tmp / 'readout.md').read_text()
        # steady state, steady heap, refresh peak and cold peak were not run: n/a; the first-build peak is the
        # priming run's (the fake prints its build report for a run into an empty cache directory)
        self.assertIn('| a | n/a | n/a | 700.0 (700.0–700.0, n=2) | 300.0 (300.0–300.0, n=2) | n/a | n/a | 910.0 |',
                      text)


class BuildKindTests(Kinds):
    def test_every_repetition_has_its_own_empty_cache_directory_and_it_is_gone_afterwards(self):
        self.prepare()
        self.run_kinds('build', reps=3)
        dirs = []
        for rep in (1, 2, 3):
            command = json.loads((self.out / 'raw' / 'a' / ('build-%d.json' % rep)).read_text())['command']
            dirs.append(Path(command[command.index('--cache-dir') + 1]))
        self.assertEqual(dirs, [self.out / 'cache-build' / 'a' / str(rep) for rep in (1, 2, 3)])
        self.assertEqual(len(set(dirs)), 3)
        self.assertEqual([d.exists() for d in dirs], [False, False, False])
        # The fake reports a hit for a second run into a directory, so three admitted builds mean three empty ones.
        self.assertEqual(self.summary()['corpora']['a']['admissible']['build'], 3)

    def test_the_directory_is_deleted_even_when_the_run_raises(self):
        self.prepare()
        with mock.patch.object(memreport, 'run_one', side_effect=RuntimeError('boom')):
            with self.assertRaises(RuntimeError):
                self.run_kinds('build', reps=1)
        self.assertFalse((self.out / 'cache-build' / 'a' / '1').exists())

    def test_the_command_is_the_warm_command(self):
        self.prepare()
        self.run_kinds('build,warm', reps=1)
        build = json.loads((self.out / 'raw' / 'a' / 'build-1.json').read_text())['command']
        warm = json.loads((self.out / 'raw' / 'a' / 'warm-1.json').read_text())['command']
        self.assertEqual([word for word in build if '/cache-build/' not in word],
                         [word for word in warm if '/cache/' not in word])
        self.assertNotIn('--no-cache', build)

    def test_the_recorded_sizes_are_those_of_the_files_the_child_wrote(self):
        self.prepare(cache_bytes_by_call=[111, 222, 333], sidecar_bytes=7)
        self.run_kinds('build', reps=3)
        stored = [json.loads((self.out / 'raw' / 'a' / ('build-%d.json' % rep)).read_text()) for rep in (1, 2, 3)]
        self.assertEqual([item['cache_files'] for item in stored], [
            {'cpg_cache_bytes': size, 'call_edge_sidecar_bytes': 7} for size in (111, 222, 333)])

    def test_a_file_the_child_did_not_write_is_none_not_zero_and_the_run_is_not_a_build(self):
        self.prepare(cache_bytes=5, sidecar_bytes=None)
        self.run_kinds('build', reps=1)
        stored = json.loads((self.out / 'raw' / 'a' / 'build-1.json').read_text())
        self.assertEqual(stored['cache_files'], {'cpg_cache_bytes': 5, 'call_edge_sidecar_bytes': None})
        self.assertEqual((stored['status'], stored['reason']), (
            'inadmissible', 'the run wrote no call-edge sidecar, so this was not a build that writes the cache'))
        self.assertEqual(self.summary()['corpora']['a']['admissible']['build'], 0)

    def test_a_child_that_wrote_no_cache_at_all_gives_no_build_series(self):
        # The real binary catches a failed cache save, prints it and exits 0 with both flags false.
        self.prepare(cache_bytes=None, sidecar_bytes=None)
        self.run_kinds('build', reps=3)
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['admissible']['build'], 0)
        self.assertEqual(len(entry['inadmissible']), 3)
        self.assertNotIn('build_peak', entry['metrics'])
        self.assertIn('wrote no CPG cache file', entry['inadmissible'][0]['reason'])

    def test_an_empty_cache_file_is_not_a_written_cache(self):
        self.prepare(cache_bytes=0)
        self.run_kinds('build', reps=1)
        self.assertEqual(self.summary()['corpora']['a']['admissible']['build'], 0)

    def test_a_save_failure_on_stderr_excludes_the_run_even_if_files_exist(self):
        for text, cause in (('nav cache: failed to save CPG cache in /x: disk full', 'the CPG cache'),
                            ('nav edge cache: failed to save sidecar in /x: disk full', 'the call-edge sidecar')):
            self.tools = self.tmp / ('tools-' + cause.split()[1])
            self.tools.mkdir()
            self.out = self.tmp / ('out-' + cause.split()[1])
            self.corpus('a') if not (self.tmp / 'a').exists() else None
            self.fake(emit={'a': 4}, reports=all_reports(), miss_stderr=[text])
            self.gen('a')
            self.run_kinds('build', reps=1)
            entry = self.summary()['corpora']['a']
            self.assertEqual(entry['admissible']['build'], 0, cause)
            self.assertIn('could not be saved', entry['inadmissible'][0]['reason'])
            self.assertIn(cause, entry['inadmissible'][0]['reason'])

    def test_summarize_applies_the_cache_evidence_to_a_stored_build_result(self):
        self.prepare()
        self.run_kinds('build', reps=3)
        raw = self.out / 'raw' / 'a'
        stored = json.loads((raw / 'build-1.json').read_text())
        stored['cache_files']['cpg_cache_bytes'] = None
        (raw / 'build-1.json').write_text(json.dumps(stored))
        stored = json.loads((raw / 'build-2.json').read_text())
        del stored['cache_files']
        (raw / 'build-2.json').write_text(json.dumps(stored))
        base.quiet(memreport.main, ['summarize', '--run', str(self.out)])
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['admissible']['build'], 1)
        self.assertEqual([(i['rep'], i['status']) for i in entry['inadmissible']],
                         [(1, 'inadmissible'), (2, 'inadmissible')])

    def test_a_repetition_whose_report_says_the_cache_was_hit_is_inadmissible(self):
        hit = canned('build')
        hit['cache'] = {'cpg_hit': True, 'call_edges_loaded': True}
        self.prepare(reports=dict(all_reports(), build=hit), miss_cache=False)
        self.run_kinds('build', reps=2)
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['admissible']['build'], 0)
        self.assertEqual([(i['kind'], i['rep'], i['reason']) for i in entry['inadmissible']],
                         [('build', 1, 'the CPG cache was hit, so this was not a first build'),
                          ('build', 2, 'the CPG cache was hit, so this was not a first build')])
        self.assertNotIn('build_peak', entry['metrics'])

    def test_a_second_run_into_the_same_directory_hits_and_is_inadmissible(self):
        self.prepare()
        cache = self.tmp / 'shared-cache'
        command = memreport.command_for(self.binary, self.tmp / 'a', cache, 'build', self.tmp / 'w.jsonl', 0)
        first = memreport.run_one(command, 'build', 30)
        second = memreport.run_one(command, 'build', 30)
        self.assertEqual(first['status'], 'ok')
        self.assertEqual((second['status'], second['reason']),
                         ('inadmissible', 'the CPG cache was hit, so this was not a first build'))

    def test_build_peak_and_build_ms_reach_the_summary_from_the_right_stages(self):
        self.prepare()
        self.run_kinds('build', reps=2)
        metrics = self.summary()['corpora']['a']['metrics']
        self.assertEqual(metrics['build_peak'], {'min': 910 * MIB, 'median': 910 * MIB, 'max': 910 * MIB, 'n': 2})
        self.assertEqual(metrics['build_ms']['median'], 8000)    # after_index 9000 minus after_load_repo 1000
        self.assertNotIn('peak', metrics)

    def test_warm_runs_report_load_ms(self):
        self.prepare(build=False)
        self.run_kinds('warm', reps=2)
        self.assertEqual(self.summary()['corpora']['a']['metrics']['load_ms']['n'], 2)

    def test_a_build_only_run_still_binds_the_workload_before_it_starts(self):
        self.prepare()
        (self.workloads / 'a.jsonl').write_bytes(b'{"id":1}\n')    # no longer what the manifest hashed
        with self.assertRaises(SystemExit) as caught:
            self.run_kinds('build')
        self.assertIn('manifest', str(caught.exception))
        self.assertFalse((self.out / 'raw').exists())


class CompareEndToEndTests(Kinds):
    """Two `run --kinds warm,build` runs of fake builds, then `compare` over their run directories."""

    def side(self, name, identity, warm_peak, build_peak):
        reports = all_reports()
        for data in reports.values():
            data['build']['build_identity'] = identity
        stage_named(reports['warm'], 'after_index')['peak_bytes'] = warm_peak * MIB
        stage_named(reports['build'], 'after_call_edges')['peak_bytes'] = build_peak * MIB
        tools = self.tmp / ('tools-' + name)
        tools.mkdir()
        self.binary = fake_prism(tools, emit={'a': 4, 'b': 4}, reports=reports, cache_bytes=10)
        self.out = self.tmp / ('run-' + name)
        argv = self.run_args('a', 'b', reps=3) + ['--kinds', 'warm,build']
        base.quiet(memreport.main, argv)
        return self.out

    def test_a_lower_candidate_passes_both_gates_and_the_unmeasured_metric_decides_nothing(self):
        self.corpus('a')
        self.corpus('b')
        self.fake(emit={'a': 4, 'b': 4})
        self.gen('a', 'b')
        baseline = self.side('old', 'old-build', 700, 910)
        candidate = self.side('new', 'new-build', 500, 700)
        base.quiet(memreport.main, ['compare', '--experiment', 'm2', '--baseline', str(baseline),
                                    '--candidate', str(candidate), '--out', str(self.tmp / 'm2.md'),
                                    '--json', str(self.tmp / 'm2.json')])
        verdicts = {m['key']: m['verdict'] for m in json.loads((self.tmp / 'm2.json').read_text())['metrics']}
        self.assertEqual(verdicts, {'peak': 'pass', 'build_peak': 'pass', 'load_ms': 'ok', 'build_ms': 'ok',
                                    'steady_heap': 'not measured'})
        self.assertIn('| a | 700.0 (700.0–700.0, n=3) | 500.0 (500.0–500.0, n=3) | -200.0 (-28.6 %) | pass',
                      (self.tmp / 'm2.md').read_text())
        with self.assertRaises(SystemExit) as caught:    # a run against itself
            base.quiet(memreport.main, ['compare', '--experiment', 'm2', '--baseline', str(baseline),
                                        '--candidate', str(baseline), '--out', str(self.tmp / 'again.md')])
        self.assertIn('same directory', str(caught.exception))


if __name__ == '__main__':
    unittest.main()
