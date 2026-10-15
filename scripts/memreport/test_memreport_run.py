#!/usr/bin/env python3
"""End-to-end tests of the `run` and `gen-workload` subcommands against a fake prism-mcp.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import contextlib
import io
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
from memreport_fixtures import MIB, canned, digest, fake_prism, stage_named  # noqa: E402


def make_repo(parent, name):
    path = Path(parent) / name
    path.mkdir()
    git = ['git', '-C', str(path), '-c', 'user.name=t', '-c', 'user.email=t@t']
    subprocess.run(git + ['init', '-q'], check=True)
    subprocess.run(git + ['commit', '-q', '--allow-empty', '-m', 'x'], check=True)
    return path


def quiet(function, *args):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        return function(*args)


class Harness(unittest.TestCase):
    """A temp directory with a fake binary, corpora, a workloads directory and an output directory."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = Path(self._tmp.name)
        self.workloads = self.tmp / 'workloads'
        self.out = self.tmp / 'out'
        self.tools = self.tmp / 'tools'
        self.tools.mkdir()

    def corpus(self, name):
        return make_repo(self.tmp, name)

    def fake(self, **cfg):
        self.binary = fake_prism(self.tools, **cfg)
        return self.binary

    def gen(self, *names, functions=5, timeout=60):
        argv = ['gen-workload', '--bin', str(self.binary), '--workloads', str(self.workloads),
                '--functions', str(functions), '--timeout', str(timeout)]
        for name in names:
            argv += ['--corpus', '%s=%s' % (name, self.tmp / name)]
        return quiet(memreport.main, argv)

    def run_args(self, *names, reps=2):
        argv = ['run', '--bin', str(self.binary), '--workloads', str(self.workloads), '--out', str(self.out),
                '--reps', str(reps), '--settle', '0', '--timeout', '30']
        for name in names:
            argv += ['--corpus', '%s=%s' % (name, self.tmp / name)]
        return argv

    def run_cmd(self, *names, reps=2):
        return quiet(memreport.main, self.run_args(*names, reps=reps))

    def summary(self):
        return json.loads((self.out / 'summary.json').read_text())


class WorkloadBindingTests(Harness):
    def prepare(self, **cfg):
        self.corpus('a')
        self.fake(**cfg)
        self.gen('a')

    def test_every_child_reads_the_snapshot_and_its_digest_is_recorded(self):
        self.prepare(emit={'a': 8})
        self.run_cmd('a')
        snapshot = self.out / 'workloads' / 'a.jsonl'
        self.assertEqual(snapshot.read_bytes(), (self.workloads / 'a.jsonl').read_bytes())
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['workload'], {'sha256': digest(snapshot.read_bytes()), 'requests': 8})
        self.assertEqual(entry['admissible']['workload'], 2)
        child = json.loads((self.out / 'raw' / 'a' / 'workload-1.json').read_text())
        self.assertEqual(child['command'][child['command'].index('--mem-workload') + 1], str(snapshot))

    def test_editing_the_source_file_during_the_run_does_not_reach_later_repetitions(self):
        self.prepare(emit={'a': 8}, mutate_source=str(self.workloads / 'a.jsonl'))
        self.run_cmd('a', reps=3)
        self.assertEqual((self.workloads / 'a.jsonl').read_bytes(), b'{"id":9}\n')  # the child did edit it
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['admissible']['workload'], 3)
        self.assertEqual(entry['workload']['requests'], 8)

    def test_a_child_that_reports_another_digest_than_the_manifest_is_inadmissible(self):
        self.prepare(emit={'a': 8}, workload_sha_by_call=['e' * 64])
        self.run_cmd('a', reps=1)
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['admissible']['workload'], 0)
        self.assertEqual([(i['kind'], i['status']) for i in entry['inadmissible']], [('workload', 'inadmissible')])
        self.assertIn('differs from the manifest', entry['inadmissible'][0]['reason'])
        self.assertNotIn('steady', entry['metrics'])

    def test_a_changed_workload_file_stops_the_run_before_anything_runs(self):
        self.prepare(emit={'a': 8})
        (self.workloads / 'a.jsonl').write_bytes(b'{"id":1}\n')
        with self.assertRaises(SystemExit) as caught:
            self.run_cmd('a')
        self.assertIn('manifest', str(caught.exception))
        self.assertIn('re-run gen-workload', str(caught.exception))
        self.assertFalse((self.out / 'raw').exists())

    def test_a_same_length_edit_is_caught_by_the_digest(self):
        self.prepare(emit={'a': 2})
        (self.workloads / 'a.jsonl').write_bytes(b'{"id":7}\n{"id":7}\n')
        with self.assertRaises(SystemExit) as caught:
            self.run_cmd('a')
        self.assertIn('sha256', str(caught.exception))

    def test_a_manifest_count_mismatch_stops_the_run(self):
        self.prepare(emit={'a': 4})
        manifest = json.loads((self.workloads / 'MANIFEST.json').read_text())
        manifest['a']['requests'] = 5
        (self.workloads / 'MANIFEST.json').write_text(json.dumps(manifest))
        with self.assertRaises(SystemExit) as caught:
            self.run_cmd('a')
        self.assertIn('4 requests', str(caught.exception))

    def test_a_workload_from_another_commit_stops_the_run(self):
        self.prepare(emit={'a': 4})
        git = ['git', '-C', str(self.tmp / 'a'), '-c', 'user.name=t', '-c', 'user.email=t@t']
        subprocess.run(git + ['commit', '-q', '--allow-empty', '-m', 'y'], check=True)
        with self.assertRaises(SystemExit) as caught:
            self.run_cmd('a')
        self.assertIn('generated at commit', str(caught.exception))

    def test_a_corpus_without_a_manifest_entry_or_workload_stops_the_run(self):
        self.prepare(emit={'a': 4})
        self.corpus('b')
        with self.assertRaises(SystemExit) as caught:
            self.run_cmd('a', 'b')
        self.assertIn('no workload', str(caught.exception))
        (self.workloads / 'b.jsonl').write_bytes(b'{"id":1}\n')
        with self.assertRaises(SystemExit) as caught:
            self.run_cmd('a', 'b')
        self.assertIn('no entry for this corpus', str(caught.exception))
        self.assertFalse((self.out / 'raw').exists())

    def test_workloads_directory_is_separate_from_the_corpus(self):
        self.prepare(emit={'a': 4})
        self.assertEqual(sorted(p.name for p in self.workloads.iterdir()), ['MANIFEST.json', 'a.jsonl'])
        self.assertEqual(list((self.tmp / 'a').glob('**/*.jsonl')), [])


class GenWorkloadTests(Harness):
    def manifest(self):
        return json.loads((self.workloads / 'MANIFEST.json').read_text())

    def test_a_timeout_in_the_second_corpus_keeps_the_first_and_exits_nonzero(self):
        self.corpus('a')
        self.corpus('b')
        self.fake(emit={'a': 3}, emit_sleep=['b'])
        with self.assertRaises(SystemExit) as caught:
            # 'b' sleeps 30 s. One second was enough for 'a' on an idle machine only: under load the fake
            # interpreter alone took longer to start, and 'a' timed out too.
            self.gen('a', 'b', timeout=5)
        self.assertIn('b', str(caught.exception))
        self.assertNotIn('a,', str(caught.exception))
        self.assertEqual(sorted(self.manifest()), ['a'])
        self.assertEqual(self.manifest()['a']['requests'], 3)
        self.assertEqual((self.workloads / 'a.jsonl').read_bytes().count(b'\n'), 3)
        self.assertFalse((self.workloads / 'b.jsonl').exists())

    def test_the_manifest_is_on_disk_before_the_next_corpus_starts(self):
        self.corpus('a')
        self.corpus('b')
        self.fake(emit={'a': 3, 'b': 3})
        real = memreport.emit_workload
        seen = {}

        def interrupted_in_b(binary, path, functions, timeout):
            if path.name == 'b':
                seen['manifest'] = sorted(self.manifest())
                raise KeyboardInterrupt
            return real(binary, path, functions, timeout)

        with mock.patch.object(memreport, 'emit_workload', interrupted_in_b):
            with self.assertRaises(KeyboardInterrupt):
                self.gen('a', 'b')
        self.assertEqual(seen['manifest'], ['a'])
        self.assertEqual(sorted(self.manifest()), ['a'])

    def test_a_failure_in_the_first_corpus_does_not_stop_the_second(self):
        self.corpus('a')
        self.corpus('b')
        self.fake(emit={'b': 4}, emit_fail=['a'])
        with self.assertRaises(SystemExit) as caught:
            self.gen('a', 'b')
        self.assertEqual(str(caught.exception), 'workload generation failed for: a')
        self.assertEqual(sorted(self.manifest()), ['b'])

    def test_every_failed_corpus_is_named(self):
        self.corpus('a')
        self.corpus('b')
        self.fake(emit_fail=['a', 'b'])
        with self.assertRaises(SystemExit) as caught:
            self.gen('a', 'b')
        self.assertEqual(str(caught.exception), 'workload generation failed for: a, b')
        self.assertFalse((self.workloads / 'MANIFEST.json').exists())

    def test_an_empty_workload_is_a_failure_not_a_manifest_entry(self):
        self.corpus('a')
        self.fake(emit={'a': 0})
        with self.assertRaises(SystemExit):
            self.gen('a')
        self.assertFalse((self.workloads / 'a.jsonl').exists())

    def test_a_missing_binary_is_a_failure_per_corpus(self):
        self.corpus('a')
        self.fake()
        self.binary = self.tools / 'does-not-exist'
        with self.assertRaises(SystemExit) as caught:
            self.gen('a')
        self.assertIn('a', str(caught.exception))

    def test_existing_manifest_entries_survive_a_later_generation(self):
        self.corpus('a')
        self.corpus('b')
        self.fake(emit={'a': 2, 'b': 5})
        self.gen('a')
        self.gen('b')
        self.assertEqual({k: v['requests'] for k, v in self.manifest().items()}, {'a': 2, 'b': 5})


class CacheFileTests(Harness):
    def test_cache_file_sizes_are_measured_after_the_priming_run(self):
        self.corpus('a')
        self.fake(cache_bytes=1234, sidecar_bytes=56)
        self.gen('a')
        self.run_cmd('a', reps=1)
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['cache_files'], {'cpg_cache_bytes': 1234, 'call_edge_sidecar_bytes': 56})
        row = self.summary()['verdict']['M2']['rows']['a']
        self.assertEqual((row['estimate'], row['estimate_n']), (1234, None))
        self.assertEqual(row['base'], 700 * MIB)    # the canned warm report's after_index peak

    def test_an_absent_cache_file_is_none_and_never_zero(self):
        self.corpus('a')
        self.fake(cache_bytes=None, sidecar_bytes=None)
        self.gen('a')
        self.run_cmd('a', reps=1)
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['cache_files'], {'cpg_cache_bytes': None, 'call_edge_sidecar_bytes': None})
        row = self.summary()['verdict']['M2']['rows']['a']
        self.assertIsNone(row['estimate'])
        self.assertFalse(row['meets'])

    def test_exactly_one_match_is_required(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.assertIsNone(memreport.single_file_size(root, 'cpg-cache-v2.bin'))
            (root / 'x').mkdir()
            (root / 'x' / 'cpg-cache-v2.bin').write_bytes(b'12345')
            self.assertEqual(memreport.single_file_size(root, 'cpg-cache-v2.bin'), 5)
            (root / 'y').mkdir()
            (root / 'y' / 'cpg-cache-v2.bin').write_bytes(b'1')
            self.assertIsNone(memreport.single_file_size(root, 'cpg-cache-v2.bin'))
            self.assertIsNone(memreport.single_file_size(root / 'missing', 'cpg-cache-v2.bin'))

    def test_a_temporary_file_is_not_the_cache(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / 'cpg-cache-v2.bin.tmp').write_bytes(b'1234')
            (Path(tmp) / 'cpg-cache.bin.tmp').write_bytes(b'1234')
            self.assertIsNone(memreport.single_file_size(tmp, 'cpg-cache-v2.bin', 'cpg-cache.bin'))

    def test_the_current_name_wins_and_the_legacy_name_is_the_fallback(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            names = memreport.CACHE_FILES['cpg_cache_bytes']
            self.assertEqual(names, ('cpg-cache-v2.bin', 'cpg-cache.bin'))
            (root / 'cpg-cache.bin').write_bytes(b'123')
            self.assertEqual(memreport.single_file_size(root, *names), 3)
            (root / 'cpg-cache-v2.bin').write_bytes(b'12345')
            self.assertEqual(memreport.single_file_size(root, *names), 5)


class RawResultTests(Harness):
    def test_raw_results_record_kind_and_rep_and_each_corpus_has_a_corpus_json(self):
        self.corpus('a')
        self.fake(emit={'a': 4}, cache_bytes=99)
        self.gen('a')
        self.run_cmd('a', reps=2)
        raw = self.out / 'raw' / 'a'
        self.assertEqual(sorted(p.name for p in raw.iterdir()),
                         ['cold-1.json', 'cold-2.json', 'corpus.json', 'prime.json', 'refresh-1.json',
                          'refresh-2.json', 'warm-1.json', 'warm-2.json', 'workload-1.json', 'workload-2.json'])
        for name in ('warm-2', 'workload-1', 'refresh-2', 'cold-1'):
            stored = json.loads((raw / (name + '.json')).read_text())
            self.assertEqual((stored['kind'], stored['rep']), (name.split('-')[0], int(name.split('-')[1])))
        self.assertEqual(json.loads((raw / 'prime.json').read_text())['kind'], memreport.PRIME)
        corpus = json.loads((raw / 'corpus.json').read_text())
        self.assertEqual(corpus['cache_files']['cpg_cache_bytes'], 99)
        self.assertEqual(corpus['workload']['requests'], 4)
        self.assertEqual(corpus['commit'], memreport.corpus_provenance(self.tmp / 'a')['commit'])

    def test_a_cache_missing_run_is_left_out_of_the_statistics_end_to_end(self):
        self.corpus('a')
        self.fake(emit={'a': 4}, stderr={'workload': ['nav cache: CPG cache miss in /x, rebuilding']})
        self.gen('a')
        self.run_cmd('a', reps=2)
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['admissible']['workload'], 0)
        self.assertEqual(len(entry['inadmissible']), 2)
        self.assertNotIn('steady', entry['metrics'])
        self.assertEqual(entry['admissible']['warm'], 2)

    def test_summary_json_is_rewritten_after_each_corpus(self):
        self.corpus('a')
        self.corpus('b')
        self.fake(emit={'a': 4, 'b': 4})
        self.gen('a', 'b')
        real = memreport.run_one

        def explode_on_b(command, kind, *rest):
            if str((self.tmp / 'b').resolve()) in command:
                raise RuntimeError('simulated crash in corpus b')
            return real(command, kind, *rest)

        with mock.patch.object(memreport, 'run_one', explode_on_b):
            with self.assertRaises(RuntimeError):
                self.run_cmd('a', 'b', reps=1)
        self.assertEqual(sorted(self.summary()['corpora']), ['a'])
        self.assertEqual(self.summary()['corpora']['a']['admissible']['warm'], 1)

    def test_run_json_records_the_settings(self):
        self.corpus('a')
        self.fake(emit={'a': 4})
        self.gen('a')
        self.run_cmd('a', reps=1)
        settings = json.loads((self.out / 'run.json').read_text())
        self.assertEqual((settings['reps'], settings['settle'], settings['bin']), (1, 0, str(self.binary)))


class SummarizeTests(Harness):
    def finished_run(self, **cfg):
        self.corpus('a')
        self.corpus('b')
        self.fake(emit={'a': 4, 'b': 4}, cache_bytes=10, **cfg)
        self.gen('a', 'b')
        self.run_cmd('a', 'b', reps=2)

    def test_summarize_rebuilds_the_same_summary_from_raw(self):
        self.finished_run()
        original = (self.out / 'summary.json').read_text()
        (self.out / 'summary.json').unlink()
        quiet(memreport.main, ['summarize', '--run', str(self.out)])
        self.assertEqual((self.out / 'summary.json').read_text(), original)

    def test_summarize_applies_the_current_rules_to_saved_evidence(self):
        self.finished_run()
        self.assertEqual(self.summary()['corpora']['a']['admissible']['warm'], 2)
        path = self.out / 'raw' / 'a' / 'warm-2.json'
        stored = json.loads(path.read_text())
        stage_named(stored['report'], 'after_index')['peak_bytes'] = None
        path.write_text(json.dumps(stored))
        quiet(memreport.main, ['summarize', '--run', str(self.out)])
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['admissible']['warm'], 1)
        self.assertEqual(entry['inadmissible'],
                         [{'kind': 'warm', 'rep': 2, 'status': 'inadmissible',
                           'reason': 'after_index.peak_bytes is unavailable'}])
        self.assertEqual(self.summary()['corpora']['b']['admissible']['warm'], 2)

    def test_summarize_applies_the_cache_evidence_stored_in_each_report(self):
        self.finished_run()
        for name, flag in (('workload-1', 'call_edges_loaded'), ('warm-2', 'cpg_hit'), ('refresh-1', 'cpg_hit')):
            path = self.out / 'raw' / 'a' / (name + '.json')
            stored = json.loads(path.read_text())
            stored['report']['cache'][flag] = False
            path.write_text(json.dumps(stored))
        quiet(memreport.main, ['summarize', '--run', str(self.out)])
        entry = self.summary()['corpora']['a']
        self.assertEqual((entry['admissible']['workload'], entry['admissible']['warm'],
                          entry['admissible']['refresh']), (1, 1, 1))
        self.assertEqual([(i['kind'], i['rep'], i['reason']) for i in entry['inadmissible']], [
            ('warm', 2, 'the CPG cache was not hit, so this was a cold build'),
            ('workload', 1, 'the call-edge index was computed, not loaded from its sidecar'),
            ('refresh', 1, 'the CPG cache was not hit, so this was a cold build')])
        self.assertEqual(self.summary()['corpora']['b']['admissible']['workload'], 2)

    def test_summarize_does_not_touch_raw_and_needs_no_binary(self):
        self.finished_run()
        before = {p.name: p.read_bytes() for p in (self.out / 'raw' / 'a').iterdir()}
        self.binary.unlink()
        quiet(memreport.main, ['summarize', '--run', str(self.out)])
        self.assertEqual({p.name: p.read_bytes() for p in (self.out / 'raw' / 'a').iterdir()}, before)

    def test_summarize_refuses_a_corpus_without_corpus_json_and_a_result_without_kind(self):
        self.finished_run()
        (self.out / 'raw' / 'a' / 'corpus.json').unlink()
        with self.assertRaises(SystemExit) as caught:
            quiet(memreport.main, ['summarize', '--run', str(self.out)])
        self.assertIn('no corpus.json', str(caught.exception))
        stored = json.loads((self.out / 'raw' / 'b' / 'warm-1.json').read_text())
        del stored['kind']
        (self.out / 'raw' / 'b' / 'warm-1.json').write_text(json.dumps(stored))
        (self.out / 'raw' / 'a' / 'corpus.json').write_text('{}')
        with self.assertRaises(SystemExit) as caught:
            quiet(memreport.main, ['summarize', '--run', str(self.out)])
        self.assertIn('no kind/rep', str(caught.exception))


class EndToEndReadoutTests(Harness):
    """gen-workload, run, summarize and render over two fake corpora, one of them degraded."""

    def scenario(self):
        self.corpus('big')
        self.corpus('small')
        no_tree = canned('workload', tree=None)
        self.fake(emit={'big': 8, 'small': 8}, cache_bytes=4096, sidecar_bytes=10,
                  repo_reports={'small': {'workload': no_tree}},
                  repo_stderr={'small': {'warm': ['nav edge cache: failed to load sidecar in /x: bad; rebuilding']}})
        self.gen('big', 'small')
        self.run_cmd('big', 'small', reps=2)
        quiet(memreport.main, ['render', '--run', str(self.out), '--out', str(self.tmp / 'readout.md')])
        return (self.tmp / 'readout.md').read_text()

    def test_the_degraded_corpus_is_excluded_and_unavailable_values_are_n_a(self):
        text = self.scenario()
        summary = self.summary()
        self.assertEqual(summary['corpora']['small']['admissible']['warm'], 0)
        self.assertEqual(summary['corpora']['big']['admissible']['warm'], 2)
        self.assertIn('| small | warm | 1 | inadmissible | the call-edge sidecar failed to load', text)
        self.assertIn('| M1 | small | n/a |', text)
        self.assertIn('| M1 | big | 300.0 | 1000.0 | 30.0 % | both | 2 / 2 |', text)
        self.assertNotIn('Traceback', text)


if __name__ == '__main__':
    unittest.main()
