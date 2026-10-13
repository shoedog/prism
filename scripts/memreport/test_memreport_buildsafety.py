#!/usr/bin/env python3
"""Tests of what the driver may write and delete: corpus names, and the build kind's cache directories.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import contextlib
import io
import json
import shutil
import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
from test_memreport_kinds import Kinds  # noqa: E402

BAD_NAMES = ('/abs/elsewhere', '..', '../victim', 'a/b', '.hidden', '-x', 'a b', 'a\\b', '', 'é')


class CorpusNameTests(Kinds):
    def test_a_name_that_is_not_a_plain_token_is_refused_before_anything_is_written(self):
        self.prepare()
        for name in BAD_NAMES:
            with self.assertRaises(SystemExit, msg=repr(name)) as caught:
                memreport.parse_corpora(['%s=%s' % (name, self.tmp / 'a')])
            self.assertIn('NAME', str(caught.exception))
        for name in ('a', 'Big-1', 'x.y_z', '0'):
            self.assertEqual(list(memreport.parse_corpora(['%s=%s' % (name, self.tmp / 'a')])), [name])

    def test_two_names_for_one_path_are_refused_before_anything_is_written(self):
        self.prepare()
        (self.tmp / 'alias').symlink_to(self.tmp / 'a')
        for second in (self.tmp / 'a', self.tmp / 'alias', self.tmp / 'a' / '..' / 'a'):
            with self.assertRaises(SystemExit) as caught:
                memreport.parse_corpora(['x=%s' % (self.tmp / 'a'), 'y=%s' % second])
            self.assertIn('x', str(caught.exception))
            self.assertIn('y', str(caught.exception))
            self.assertIn('same', str(caught.exception))
        with self.assertRaises(SystemExit):
            memreport.main(self.run_args()[:self.run_args().index('--reps')]
                           + ['--corpus', 'x=%s' % (self.tmp / 'a'), '--corpus', 'y=%s' % (self.tmp / 'alias')])
        self.assertFalse(self.out.exists())
        self.assertEqual(len(memreport.parse_corpora(['x=%s' % (self.tmp / 'a'), 'y=%s' % (self.tmp / 'b')])), 2)

    def test_run_and_gen_workload_both_refuse_an_absolute_name_and_touch_nothing(self):
        self.prepare()
        victim = self.tmp / 'elsewhere'
        (victim / '1').mkdir(parents=True)
        (victim / '1' / 'evidence.txt').write_text('keep')
        argv = ['--corpus', '%s=%s' % (victim, self.tmp / 'a')]
        with self.assertRaises(SystemExit):
            base_args = self.run_args()[:self.run_args().index('--reps')]
            memreport.main(base_args + ['--kinds', 'build'] + argv)
        with self.assertRaises(SystemExit):
            memreport.main(['gen-workload', '--bin', str(self.binary), '--workloads', str(self.tmp / 'w2')] + argv)
        self.assertEqual((victim / '1' / 'evidence.txt').read_text(), 'keep')
        self.assertFalse(self.out.exists())
        self.assertFalse((self.tmp / 'w2').exists())


class BuildDirectoryTests(Kinds):
    def calls(self):
        return self.child_calls('warm')

    def test_a_repetition_directory_that_already_exists_is_refused_and_left_alone(self):
        self.prepare()
        existing = self.out / 'cache-build' / 'a' / '1'
        existing.mkdir(parents=True)
        (existing / 'old.bin').write_bytes(b'x')
        self.run_kinds('build', reps=2)
        self.assertEqual((existing / 'old.bin').read_bytes(), b'x')
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['admissible']['build'], 1)
        self.assertEqual([(i['rep'], i['reason']) for i in entry['inadmissible']],
                         [(1, 'the cache directory already existed')])
        self.assertEqual(self.calls(), 1)    # no child ran for the refused repetition

    def test_a_symlinked_repetition_path_is_refused_and_its_target_untouched(self):
        self.prepare()
        target = self.tmp / 'target'
        target.mkdir()
        (target / 'keep.txt').write_text('keep')
        (self.out / 'cache-build' / 'a').mkdir(parents=True)
        (self.out / 'cache-build' / 'a' / '1').symlink_to(target)
        self.run_kinds('build', reps=1)
        self.assertEqual((target / 'keep.txt').read_text(), 'keep')
        self.assertEqual(self.summary()['corpora']['a']['admissible']['build'], 0)

    def test_a_symlinked_corpus_directory_is_refused_and_its_target_untouched(self):
        self.prepare()
        target = self.tmp / 'target'
        (target / '1').mkdir(parents=True)
        (target / '1' / 'keep.txt').write_text('keep')
        (self.out / 'cache-build').mkdir(parents=True)
        (self.out / 'cache-build' / 'a').symlink_to(target)
        self.run_kinds('build', reps=1)
        self.assertEqual((target / '1' / 'keep.txt').read_text(), 'keep')
        self.assertEqual(self.calls(), 0)
        self.assertEqual(self.summary()['corpora']['a']['admissible']['build'], 0)

    def test_a_removal_that_fails_is_recorded_and_warned_about_not_hidden(self):
        self.prepare()
        err = io.StringIO()
        with mock.patch.object(shutil, 'rmtree', side_effect=OSError('busy')):
            with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
                memreport.main(self.run_args('a', reps=1) + ['--kinds', 'build'])
        stored = json.loads((self.out / 'raw' / 'a' / 'build-1.json').read_text())
        self.assertIs(stored['cache_dir_removed'], False)
        self.assertIn(str(self.out / 'cache-build' / 'a' / '1'), err.getvalue())
        self.assertTrue((self.out / 'cache-build' / 'a' / '1').exists())    # the parents stay too

    def test_a_removal_that_works_is_recorded_and_leaves_no_empty_directories(self):
        self.prepare()
        self.run_kinds('build', reps=2)
        for rep in (1, 2):
            stored = json.loads((self.out / 'raw' / 'a' / ('build-%d.json' % rep)).read_text())
            self.assertIs(stored['cache_dir_removed'], True)
        self.assertFalse((self.out / 'cache-build').exists())

    def test_nothing_outside_the_run_directory_is_ever_deleted(self):
        self.prepare()
        deleted = []
        real = shutil.rmtree
        with mock.patch.object(shutil, 'rmtree', lambda path, *a, **k: (deleted.append(Path(path)), real(path))[1]):
            self.run_kinds('build', reps=2)
        self.assertEqual(len(deleted), 2)
        for path in deleted:
            self.assertIn(self.out.resolve(), path.resolve().parents)


if __name__ == '__main__':
    unittest.main()
