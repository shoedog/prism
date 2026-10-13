#!/usr/bin/env python3
"""Tests that the build kind deletes only the directory it created, and reports what it observed.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import contextlib
import io
import json
import os
import shutil
import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
import memreport_builddir  # noqa: E402
from memreport_fixtures import canned  # noqa: E402
from test_memreport_kinds import Kinds  # noqa: E402


def ok_run(command, kind, timeout, expected=None):
    """A stand-in for run_one: an admissible-looking result, nothing started, nothing written."""
    return {'kind': kind, 'status': 'ok', 'command': command, 'wall_secs': 0, 'report': canned('build')}


def cache_dir_of(command):
    return Path(command[command.index('--cache-dir') + 1])


class BuildDirectoryOwnershipTests(Kinds):
    def run_build_only(self, run_one, reps=1):
        err = io.StringIO()
        with mock.patch.object(memreport, 'run_one', run_one):
            with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
                memreport.main(self.run_args('a', reps=reps) + ['--kinds', 'build'])
        return err.getvalue()

    def stored(self, rep=1):
        return json.loads((self.out / 'raw' / 'a' / ('build-%d.json' % rep)).read_text())

    def unlock_at_exit(self, path):
        self.addCleanup(lambda: [os.chmod(p, 0o755) for p in (path / 'locked', path) if p.exists()])

    def test_a_directory_that_cannot_be_deleted_is_reported_as_not_removed_with_a_warning(self):
        self.prepare()
        locked_dir = self.out / 'cache-build' / 'a' / '1'
        self.unlock_at_exit(locked_dir)

        def writes_a_locked_directory(command, kind, timeout, expected=None):
            locked = cache_dir_of(command) / 'locked'
            locked.mkdir()
            (locked / 'x.bin').write_bytes(b'x')
            locked.chmod(0o555)    # a file inside cannot be unlinked
            return ok_run(command, kind, timeout)

        warning = self.run_build_only(writes_a_locked_directory)
        stored = self.stored()
        self.assertIs(stored['cache_dir_removed'], False)
        self.assertIn('cache_dir_note', stored)
        self.assertIn(str(locked_dir), warning)
        self.assertTrue((locked_dir / 'locked' / 'x.bin').exists())

    def owned_directory(self, after_mkdir=None):
        root = self.out / 'cache-build'
        created, owned, refusal = memreport.prepare_repetition(root, 'a', 1, after_mkdir)
        if owned is not None:
            self.addCleanup(owned.close)
        return root, created, owned, refusal

    def quiet_removal(self, created, root, owned, at=None):
        with contextlib.redirect_stderr(io.StringIO()) as err:
            removed, note = memreport_builddir.remove_created(created, root, owned, at)
        return removed, note, err.getvalue()

    def test_removed_is_true_only_if_the_path_is_gone_afterwards(self):
        self.prepare()
        root, created, owned, refusal = self.owned_directory()
        self.assertIsNone(refusal)
        (created / 'x').write_bytes(b'x')
        with mock.patch.object(os, 'rmdir', lambda *a, **k: None):    # "succeeds" and removes nothing
            removed, note, _ = self.quiet_removal(created, root, owned)
        self.assertIs(removed, False)
        self.assertIn('still exists', note)
        self.assertEqual(list(created.iterdir()), [])    # its contents went through the descriptor

    def test_an_ordinary_removal_deletes_files_and_nested_directories_and_then_the_name(self):
        self.prepare()
        root, created, owned, _ = self.owned_directory()
        (created / 'cpg-cache.bin').write_bytes(b'x' * 10)
        (created / 'nested' / 'deeper').mkdir(parents=True)
        (created / 'nested' / 'deeper' / 'f').write_text('f')
        removed, note, warning = self.quiet_removal(created, root, owned)
        self.assertEqual((removed, note, warning), (True, None, ''))
        self.assertFalse(root.exists())    # the empty parents went too
        self.assertEqual((owned.fd, owned.parent_fd), (None, None))    # no descriptor is left open

    def test_a_symlink_inside_the_directory_is_unlinked_not_followed(self):
        self.prepare()
        outside = self.tmp / 'outside'
        outside.mkdir()
        (outside / 'precious.txt').write_text('keep')
        root, created, owned, _ = self.owned_directory()
        (created / 'link-to-dir').symlink_to(outside)
        (created / 'link-to-file').symlink_to(outside / 'precious.txt')
        removed, note, _ = self.quiet_removal(created, root, owned)
        self.assertEqual((removed, note), (True, None))
        self.assertEqual((outside / 'precious.txt').read_text(), 'keep')

    def swap_in_evidence(self, created):
        """Rename the created directory away and move a directory holding a file into its path."""
        evidence = self.tmp / 'evidence'
        evidence.mkdir()
        (evidence / 'precious.txt').write_text('keep')
        created.rename(self.tmp / 'moved-away')
        shutil.move(str(evidence), str(created))

    def test_a_replacement_made_just_before_the_contents_are_deleted_is_not_touched(self):
        # The reviewer's sequence: every check has passed, then the path is made to name other evidence.
        self.prepare()
        root, created, owned, _ = self.owned_directory()
        (created / 'cpg-cache.bin').write_bytes(b'x')
        removed, note, warning = self.quiet_removal(
            created, root, owned, lambda step: step == 'before-empty' and self.swap_in_evidence(created))
        self.assertEqual((created / 'precious.txt').read_text(), 'keep')
        self.assertIs(removed, False)
        self.assertIn('no longer the directory this run created', note)
        self.assertIn(str(created), warning)
        # What this run created was emptied where it went; nothing else was.
        self.assertEqual(list((self.tmp / 'moved-away').iterdir()), [])

    def test_a_replacement_made_just_before_the_name_is_removed_is_not_touched(self):
        # After the identity of the name has been confirmed and before the rmdir: rmdir refuses content.
        self.prepare()
        root, created, owned, _ = self.owned_directory()
        removed, note, _ = self.quiet_removal(
            created, root, owned, lambda step: step == 'before-rmdir' and self.swap_in_evidence(created))
        self.assertEqual((created / 'precious.txt').read_text(), 'keep')
        self.assertIs(removed, False)
        self.assertIn('could not be deleted', note)

    def test_a_replacement_made_right_after_the_creation_is_refused_and_never_owned(self):
        # Between the mkdir and the opening: what gets opened is not what was created.
        self.prepare()
        created = self.out / 'cache-build' / 'a' / '1'
        root, directory, owned, refusal = self.owned_directory(lambda: self.swap_in_evidence(created))
        self.assertEqual((directory, owned), (None, None))
        self.assertIn('replaced right after it was created', refusal)
        self.assertEqual((created / 'precious.txt').read_text(), 'keep')

    def test_a_replacement_right_after_the_creation_refuses_the_repetition_and_runs_no_child(self):
        self.prepare()
        created = self.out / 'cache-build' / 'a' / '1'
        real = memreport_builddir.prepare_repetition

        def swapped(root, name, rep):
            return real(root, name, rep, lambda: self.swap_in_evidence(created))

        with mock.patch.object(memreport, 'prepare_repetition', swapped):
            self.run_kinds('build', reps=1)
        self.assertEqual(self.child_calls('warm'), 0)
        self.assertEqual((created / 'precious.txt').read_text(), 'keep')
        self.assertIn('replaced', self.summary()['corpora']['a']['inadmissible'][0]['reason'])

    def test_a_parent_swapped_for_a_symlink_during_the_measurement_loses_nothing(self):
        self.prepare()
        victim = self.tmp / 'victim'
        (victim / '1').mkdir(parents=True)
        (victim / '1' / 'keep.txt').write_text('keep')
        root, created, owned, _ = self.owned_directory()
        (created / 'cpg-cache.bin').write_bytes(b'x')
        (root / 'a').rename(self.tmp / 'parent-moved-away')
        (root / 'a').symlink_to(victim)    # the path cache-build/a/1 now names victim/1
        removed, note, warning = self.quiet_removal(created, root, owned)
        self.assertEqual((victim / '1' / 'keep.txt').read_text(), 'keep')
        self.assertIs(removed, False)    # the path still names something, and it says so
        self.assertIn('still exists', note)
        self.assertIn(str(created), warning)
        self.assertFalse((self.tmp / 'parent-moved-away' / '1').exists())    # what this run created is gone

    def test_a_symlinked_corpus_directory_with_an_empty_target_gets_nothing_created_and_no_child_runs(self):
        self.prepare()
        target = self.tmp / 'target'
        target.mkdir()
        (self.out / 'cache-build').mkdir(parents=True)
        (self.out / 'cache-build' / 'a').symlink_to(target)
        self.run_kinds('build', reps=2)
        self.assertEqual(list(target.iterdir()), [])
        self.assertEqual(self.child_calls('warm'), 0)
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['admissible']['build'], 0)
        self.assertIn('symlink', entry['inadmissible'][0]['reason'])

    def test_a_symlinked_cache_build_root_with_an_empty_target_gets_nothing_created_and_no_child_runs(self):
        self.prepare()
        target = self.tmp / 'target'
        target.mkdir()
        self.out.mkdir(parents=True)
        (self.out / 'cache-build').symlink_to(target)
        # run refuses a pre-existing out only for raw/run.json/summary.json, so the link survives into the run
        self.run_kinds('build', reps=1)
        self.assertEqual(list(target.iterdir()), [])
        self.assertEqual(self.child_calls('warm'), 0)
        self.assertIn('symlink', self.summary()['corpora']['a']['inadmissible'][0]['reason'])

    def test_a_directory_swapped_into_the_path_during_the_measurement_is_never_deleted(self):
        self.prepare()
        evidence = self.tmp / 'evidence'
        evidence.mkdir()
        (evidence / 'precious.txt').write_text('keep')
        moved = self.tmp / 'moved-away'

        def swaps(command, kind, timeout, expected=None):
            cache = cache_dir_of(command)
            cache.rename(moved)    # the directory this run created goes elsewhere ...
            shutil.move(str(evidence), str(cache))    # ... and someone else's directory takes its path
            return ok_run(command, kind, timeout)

        warning = self.run_build_only(swaps)
        cache = self.out / 'cache-build' / 'a' / '1'
        self.assertEqual((cache / 'precious.txt').read_text(), 'keep')
        self.assertTrue(moved.exists())    # the owned directory was not found at its path, so it is left alone
        stored = self.stored()
        self.assertIs(stored['cache_dir_removed'], False)
        self.assertIn('no longer', stored['cache_dir_note'])
        self.assertIn(str(cache), warning)

    def test_a_symlink_swapped_into_the_path_is_not_followed_or_deleted(self):
        self.prepare()
        target = self.tmp / 'target'
        target.mkdir()
        (target / 'precious.txt').write_text('keep')
        moved = self.tmp / 'moved-away'

        def swaps(command, kind, timeout, expected=None):
            cache = cache_dir_of(command)
            cache.rename(moved)
            cache.symlink_to(target)
            return ok_run(command, kind, timeout)

        self.run_build_only(swaps)
        self.assertEqual((target / 'precious.txt').read_text(), 'keep')
        self.assertIs(self.stored()['cache_dir_removed'], False)

    def test_a_regular_file_at_cache_build_is_a_refused_repetition_not_a_traceback(self):
        self.prepare()
        self.out.mkdir(parents=True)
        (self.out / 'cache-build').write_text('in the way')
        self.run_kinds('build', reps=2)
        entry = self.summary()['corpora']['a']    # the run finished and wrote its summary
        self.assertEqual(entry['admissible']['build'], 0)
        self.assertEqual(len(entry['inadmissible']), 2)
        self.assertEqual(self.child_calls('warm'), 0)
        self.assertEqual((self.out / 'cache-build').read_text(), 'in the way')

    def test_a_read_only_corpus_directory_is_a_refused_repetition_not_a_traceback(self):
        self.prepare()
        parent = self.out / 'cache-build' / 'a'
        parent.mkdir(parents=True)
        parent.chmod(0o555)
        self.addCleanup(lambda: parent.chmod(0o755))
        self.run_kinds('build', reps=1)
        entry = self.summary()['corpora']['a']
        self.assertEqual(entry['admissible']['build'], 0)
        self.assertIn('Permission', entry['inadmissible'][0]['reason'])
        self.assertEqual(self.child_calls('warm'), 0)

    def test_an_ordinary_run_still_removes_what_it_created(self):
        self.prepare()
        self.run_kinds('build', reps=2)
        for rep in (1, 2):
            self.assertIs(self.stored(rep)['cache_dir_removed'], True)
            self.assertNotIn('cache_dir_note', self.stored(rep))
        self.assertFalse((self.out / 'cache-build').exists())


if __name__ == '__main__':
    unittest.main()
