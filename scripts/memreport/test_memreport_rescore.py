#!/usr/bin/env python3
"""Tests that rescoring (summarize, and compare which uses it) fails closed on a raw directory it cannot trust.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import contextlib
import io
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
import memreport_compare as mc  # noqa: E402
from memreport_fixtures import write_run  # noqa: E402


class RawDirectoryTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = Path(self._tmp.name)
        self.base = write_run(self.tmp / 'base', {'a': [1000, 1000, 1000]}, identity='base', reps=3)
        self.raw = self.base / 'raw' / 'a'

    def refused(self, run=None):
        with self.assertRaises(SystemExit) as caught:
            memreport.rescore(run or self.base)
        return str(caught.exception)

    def summarize_refused(self):
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(SystemExit) as caught:
            memreport.main(['summarize', '--run', str(self.base)])
        return str(caught.exception)

    def test_a_clean_directory_is_rescored(self):
        self.assertEqual(memreport.rescore(self.base)['corpora']['a']['admissible']['warm'], 3)

    def test_a_copied_result_file_is_refused_and_named(self):
        shutil.copy(self.raw / 'warm-1.json', self.raw / 'warm-1 copy.json')
        self.assertIn('warm-1 copy.json', self.refused())
        self.assertIn('warm-1 copy.json', self.summarize_refused())

    def test_a_result_filed_under_another_kind_is_refused_and_named(self):
        shutil.copy(self.raw / 'warm-1.json', self.raw / 'cold-1.json')    # a warm result named cold-1
        self.assertIn('cold-1.json', self.refused())

    def test_a_repetition_above_the_runs_reps_is_refused(self):
        stored = json.loads((self.raw / 'warm-1.json').read_text())
        stored['rep'] = 4
        (self.raw / 'warm-4.json').write_text(json.dumps(stored))
        message = self.refused()
        self.assertIn('warm-4.json', message)
        self.assertIn('1..3', message)

    def test_repetition_zero_negative_and_non_integers_are_refused(self):
        for rep in (0, -1, 1.0, True, '1'):
            run = write_run(self.tmp / ('run-%r' % (rep,)).replace('/', '_'), {'a': [1000, 1000, 1000]}, reps=3)
            stored = json.loads((run / 'raw' / 'a' / 'warm-1.json').read_text())
            stored['rep'] = rep
            (run / 'raw' / 'a' / 'warm-1.json').write_text(json.dumps(stored))
            with self.assertRaises(SystemExit, msg=repr(rep)):
                memreport.rescore(run)

    def test_a_file_named_for_the_right_result_in_another_spelling_is_refused(self):
        shutil.copy(self.raw / 'warm-1.json', self.raw / 'warm-01.json')
        self.assertIn('warm-01.json', self.refused())

    def test_a_result_file_that_is_not_a_json_object_is_refused_with_a_message(self):
        for text in ('[]', '3', 'null', 'not json', ''):
            (self.raw / 'warm-1.json').write_text(text)
            message = self.refused()
            self.assertIn('warm-1.json', message, text)
            self.assertNotIn('Traceback', message)

    def test_run_json_corpus_json_and_prime_json_that_are_not_objects_are_refused_with_a_message(self):
        original = (self.base / 'run.json').read_text()
        for text in ('[]', 'null', '{"kinds": ["warm"]}', '{"reps": "3"}', '{"reps": 0}', '{bad'):
            (self.base / 'run.json').write_text(text)
            self.assertIn('run.json', self.refused(), text)
        (self.base / 'run.json').write_text(original)
        for name in ('corpus.json', 'prime.json'):
            (self.raw / name).write_text('[]')
            self.assertIn(name, self.refused(), name)
            (self.raw / name).unlink()
            (self.raw / 'corpus.json').write_text(json.dumps({'path': '/c', 'commit': 'c', 'dirty': 0}))

    def test_a_directory_with_run_json_and_no_raw_is_refused_cleanly(self):
        shutil.rmtree(self.base / 'raw')
        message = self.refused()
        self.assertIn('raw/', message)
        self.assertIn(str(self.base), message)

    def test_a_directory_with_raw_and_no_run_json_is_refused_cleanly(self):
        (self.base / 'run.json').unlink()
        self.assertIn('run.json', self.refused())

    def test_a_gate_that_is_undecided_at_two_runs_is_not_lifted_to_a_pass_by_copied_files(self):
        base = write_run(self.tmp / 'two-base', {'a': [1000, 1000], 'b': [1000, 1000]}, identity='x', reps=2)
        cand = write_run(self.tmp / 'two-cand', {'a': [500, 500], 'b': [500, 500]}, identity='y', reps=2)
        before = mc.compare(memreport.rescore(base), memreport.rescore(cand), 'm2')
        self.assertEqual(next(m for m in before['metrics'] if m['key'] == 'peak')['verdict'], 'undecided')
        for run in (base, cand):
            for corpus in ('a', 'b'):
                shutil.copy(run / 'raw' / corpus / 'warm-1.json', run / 'raw' / corpus / 'warm-1 backup.json')
        with self.assertRaises(SystemExit):
            memreport.rescore(base)
        with self.assertRaises(SystemExit):
            with contextlib.redirect_stdout(io.StringIO()):
                memreport.main(['compare', '--experiment', 'm2', '--baseline', str(base), '--candidate', str(cand),
                                '--out', str(self.tmp / 'out.md')])
        self.assertFalse((self.tmp / 'out.md').exists())


if __name__ == '__main__':
    unittest.main()
