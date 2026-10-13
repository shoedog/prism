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

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
from memreport_fixtures import canned, digest, fake_prism  # noqa: E402


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


if __name__ == '__main__':
    unittest.main()
