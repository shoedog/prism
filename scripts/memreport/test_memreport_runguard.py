#!/usr/bin/env python3
"""A run directory holds one run, one build, and corpora that git can identify.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import copy
import json
import subprocess
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
import test_memreport_run as base  # noqa: E402
from memreport_fixtures import MIB, canned, ok_result, prime_result  # noqa: E402

DIGEST = {'sha256': 'f' * 64, 'requests': 8}
CORPUS = {'path': '/c', 'commit': 'abc', 'dirty': 0, 'workload': DIGEST,
          'cache_files': {'cpg_cache_bytes': 7 * MIB, 'call_edge_sidecar_bytes': None}}
REFUSAL = "run directory already has results; use a new --out, or 'summarize' to rescore"


def built_by(result, identity):
    result = copy.deepcopy(result)
    result['report']['build']['build_identity'] = identity
    return result


def canned_by(kind, identity):
    data = canned(kind)
    data['build']['build_identity'] = identity
    return data


def files_under(directory):
    return {str(path.relative_to(directory)): path.read_bytes()
            for path in sorted(Path(directory).rglob('*')) if path.is_file()}


class RefusalTests(base.Harness):
    def finished_run(self):
        self.corpus('a')
        self.fake(emit={'a': 4}, cache_bytes=10)
        self.gen('a')
        self.run_cmd('a', reps=2)

    def test_a_second_run_into_the_same_directory_is_refused_and_nothing_is_overwritten(self):
        self.finished_run()
        before = files_under(self.out)
        calls_before = files_under(self.tools)
        with self.assertRaises(SystemExit) as caught:
            self.run_cmd('a', reps=1)
        self.assertEqual(str(caught.exception), REFUSAL)
        self.assertEqual(files_under(self.out), before)       # raw files, run.json and summary.json, byte for byte
        self.assertEqual(files_under(self.tools), calls_before)   # the binary was not run
        self.assertEqual(self.summary()['corpora']['a']['admissible']['warm'], 2)

    def test_any_one_of_the_three_artifacts_is_enough_to_refuse(self):
        self.corpus('a')
        self.fake(emit={'a': 4})
        self.gen('a')
        for artifact in ('raw', 'run.json', 'summary.json'):
            with self.subTest(artifact):
                self.out = self.tmp / ('out-' + artifact.replace('.', '-'))
                self.out.mkdir()
                if artifact == 'raw':
                    (self.out / 'raw').mkdir()
                else:
                    (self.out / artifact).write_text('{}')
                with self.assertRaises(SystemExit) as caught:
                    self.run_cmd('a', reps=1)
                self.assertEqual(str(caught.exception), REFUSAL)
                self.assertEqual(sorted(path.name for path in self.out.iterdir()), [artifact])

    def test_an_existing_empty_directory_is_fine(self):
        self.corpus('a')
        self.fake(emit={'a': 4})
        self.gen('a')
        self.out.mkdir()
        self.run_cmd('a', reps=1)
        self.assertEqual(self.summary()['corpora']['a']['admissible']['warm'], 1)

    def test_the_refusal_comes_before_the_workloads_are_checked(self):
        self.finished_run()
        (self.workloads / 'a.jsonl').write_bytes(b'changed\n')
        with self.assertRaises(SystemExit) as caught:
            self.run_cmd('a')
        self.assertEqual(str(caught.exception), REFUSAL)

    def test_summarize_still_works_on_a_directory_run_refuses(self):
        self.finished_run()
        base.quiet(memreport.main, ['summarize', '--run', str(self.out)])
        self.assertEqual(self.summary()['corpora']['a']['admissible']['warm'], 2)


class BuildIdentityTests(unittest.TestCase):
    def test_a_run_from_another_build_than_the_corpus_first_admitted_one_is_excluded(self):
        entry = memreport.build_entry(CORPUS, [ok_result('warm', 1), built_by(ok_result('warm', 2), 'other'),
                                               ok_result('warm', 3), built_by(ok_result('cold', 1), 'other')])
        self.assertEqual(entry['admissible'], {'warm': 2, 'cold': 0, 'workload': 0, 'refresh': 0})
        self.assertEqual(entry['inadmissible'], [
            {'kind': 'warm', 'rep': 2, 'status': 'inadmissible',
             'reason': "build identity differs from this corpus's first admitted run"},
            {'kind': 'cold', 'rep': 1, 'status': 'inadmissible',
             'reason': "build identity differs from this corpus's first admitted run"}])
        self.assertEqual(entry['metrics']['peak']['n'], 2)
        self.assertNotIn('cold_peak', entry['metrics'])

    def test_the_reference_is_the_first_admitted_run_not_the_first_listed(self):
        failed = built_by(ok_result('warm', 1), 'broken')
        failed.update(status='inadmissible', reason='workload had 1 errors')
        entry = memreport.build_entry(CORPUS, [failed, ok_result('warm', 2), ok_result('warm', 3)])
        self.assertEqual(entry['admissible']['warm'], 2)
        self.assertEqual([item['reason'] for item in entry['inadmissible']], ['workload had 1 errors'])

    def test_one_build_across_all_runs_changes_nothing(self):
        results = [ok_result(kind, rep) for kind in memreport.KINDS for rep in (1, 2)]
        entry = memreport.build_entry(CORPUS, results)
        self.assertEqual(entry['inadmissible'], [])
        self.assertEqual(sum(entry['admissible'].values()), 8)

    def corpora(self, identities):
        entries = {}
        for name, identity in identities.items():
            results = [built_by(ok_result(kind, rep), identity) for kind in memreport.KINDS for rep in (1, 2)]
            entries[name] = memreport.build_entry(dict(CORPUS, path='/' + name), results, prime_result())
        return entries

    def test_a_corpus_from_another_build_than_the_first_is_excluded_whole(self):
        summary = memreport.assemble_summary({'reps': 2}, self.corpora({'a': 'X', 'b': 'Y', 'c': 'X'}))
        self.assertEqual(summary['provenance']['build']['build_identity'], 'X')
        b = summary['corpora']['b']
        self.assertEqual(b['admissible'], {kind: 0 for kind in memreport.KINDS})
        self.assertEqual(b['metrics'], {})
        self.assertIsNone(b['build'])
        self.assertIsNone(b['first_build_peak'])
        self.assertEqual(b['cache_files'], {'cpg_cache_bytes': None, 'call_edge_sidecar_bytes': None})
        self.assertEqual([(item['kind'], item['rep'], item['reason']) for item in b['inadmissible']],
                         [(kind, 'all 2', "build identity differs from the first corpus's (X)")
                          for kind in memreport.KINDS])
        for candidate in ('M1', 'M2', 'M3'):                  # nothing of b reaches the decision rule
            row = summary['verdict'][candidate]['rows']['b']
            self.assertIsNone(row['estimate'])
            self.assertNotIn('b', summary['verdict'][candidate]['corpora'])
        self.assertEqual(summary['corpora']['a'], self.corpora({'a': 'X'})['a'])
        self.assertEqual(summary['corpora']['c']['admissible']['warm'], 2)

    def test_the_first_corpus_is_the_first_by_name_whatever_the_order_given(self):
        entries = self.corpora({'b': 'Y', 'a': 'X'})
        summary = memreport.assemble_summary({}, entries)
        self.assertEqual(summary['provenance']['build']['build_identity'], 'X')
        self.assertEqual(summary['corpora']['b']['admissible']['warm'], 0)
        self.assertEqual(summary['corpora']['a']['admissible']['warm'], 2)

    def test_a_corpus_with_no_admitted_run_does_not_set_the_reference(self):
        entries = self.corpora({'b': 'Y'})
        entries['a'] = memreport.build_entry(CORPUS, [{'kind': 'cold', 'rep': 1, 'status': 'timeout'}])
        summary = memreport.assemble_summary({}, entries)
        self.assertEqual(summary['provenance']['build']['build_identity'], 'Y')
        self.assertEqual(summary['corpora']['b']['admissible']['warm'], 2)


class RescoreIdentityTests(base.Harness):
    def test_identity_is_checked_again_when_a_saved_run_is_rescored(self):
        for name in ('a', 'b'):
            self.corpus(name)
        self.fake(emit={'a': 4, 'b': 4},
                  repo_reports={'b': {kind: canned_by(kind, 'Y') for kind in memreport.KINDS}})
        self.gen('a', 'b')
        self.run_cmd('a', 'b', reps=1)
        summary = self.summary()
        self.assertEqual(summary['corpora']['b']['admissible']['warm'], 0)
        self.assertEqual(summary['corpora']['a']['admissible']['warm'], 1)
        original = (self.out / 'summary.json').read_text()
        base.quiet(memreport.main, ['summarize', '--run', str(self.out)])
        self.assertEqual((self.out / 'summary.json').read_text(), original)


class NoCommitTests(base.Harness):
    def test_git_that_cannot_answer_gives_none_not_blank_or_zero(self):
        plain = self.tmp / 'plain'
        plain.mkdir()
        self.assertEqual(memreport.corpus_provenance(plain), {'path': str(plain), 'commit': None, 'dirty': None})
        unborn = self.tmp / 'unborn'
        unborn.mkdir()
        subprocess.run(['git', '-C', str(unborn), 'init', '-q'], check=True)
        self.assertEqual(memreport.corpus_provenance(unborn), {'path': str(unborn), 'commit': None, 'dirty': 0})
        repo = self.corpus('real')
        (repo / 'new.txt').write_text('x')
        found = memreport.corpus_provenance(repo)
        self.assertEqual(len(found['commit']), 40)
        self.assertEqual(found['dirty'], 1)

    def test_a_missing_directory_or_git_is_none(self):
        self.assertEqual(memreport.corpus_provenance(self.tmp / 'missing')['commit'], None)
        self.assertIsNone(memreport.capture(['definitely-not-a-program-xyz']))

    def test_bind_workload_refuses_a_corpus_with_no_commit(self):
        self.workloads.mkdir()
        (self.workloads / 'a.jsonl').write_bytes(b'{"id":1}\n')
        manifest = {'a': {'commit': None, 'requests': 1, 'sha256': memreport.hashlib.sha256(b'{"id":1}\n').hexdigest()}}
        (self.workloads / 'MANIFEST.json').write_text(json.dumps(manifest))
        for commit in (None, ''):
            with self.assertRaises(SystemExit) as caught:
                memreport.bind_workload('a', self.workloads, commit)
            self.assertIn('no git commit', str(caught.exception))

    def test_run_stops_before_measuring_a_corpus_that_is_not_a_git_checkout(self):
        (self.tmp / 'plain').mkdir()
        self.fake(emit={'plain': 4})
        self.gen('plain')           # the manifest records commit null, which `run` then refuses
        with self.assertRaises(SystemExit) as caught:
            self.run_cmd('plain')
        self.assertIn('no git commit', str(caught.exception))
        self.assertFalse((self.out / 'raw').exists())


class FirstBuildPeakTests(unittest.TestCase):
    def test_it_is_the_last_stage_peak_of_an_admitted_priming_run(self):
        self.assertEqual(memreport.prime_peak(prime_result()), 710 * MIB)

    def test_it_is_none_when_the_priming_run_is_absent_failed_or_unusable(self):
        broken_version = canned('warm')
        broken_version['prism_mem_report'] = 2
        no_peak = canned('warm')
        no_peak['stages'][-1]['peak_bytes'] = None
        no_stages = canned('warm')
        no_stages['stages'] = []
        for label, prime in (('absent', None), ('failed', {'kind': 'prime', 'status': 'failed'}),
                             ('inadmissible', prime_result(status='inadmissible')),
                             ('wrong version', prime_result(broken_version)), ('no peak', prime_result(no_peak)),
                             ('no stages', prime_result(no_stages)), ('no report', {'status': 'ok'})):
            self.assertIsNone(memreport.prime_peak(prime), label)


class PrimeSummarizeTests(base.Harness):
    def test_summarize_reads_it_from_prime_json_and_a_missing_file_is_n_a(self):
        self.corpus('a')
        self.fake(emit={'a': 4})
        self.gen('a')
        self.run_cmd('a', reps=1)
        self.assertEqual(self.summary()['corpora']['a']['first_build_peak'], 710 * MIB)
        (self.out / 'raw' / 'a' / 'prime.json').unlink()
        base.quiet(memreport.main, ['summarize', '--run', str(self.out)])
        self.assertIsNone(self.summary()['corpora']['a']['first_build_peak'])


if __name__ == '__main__':
    unittest.main()
