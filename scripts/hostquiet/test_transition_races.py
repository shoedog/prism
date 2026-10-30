"""Deterministic transition interleavings, also runnable against the pre-fix module."""

import json
import os
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

import host_quiet as quiet


class TransitionRaceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.lock = self.directory / 'lock'
        # The nested contender must give up while the paused transition owns
        # the guard. The old module ignores this constant, so its race still runs.
        timeout = patch.object(quiet, 'GUARD_WAIT_SECS', 0, create=True)
        timeout.start()
        self.addCleanup(timeout.stop)

    def take(self, owner):
        return quiet.acquire(owner, 'sample', 1200, directory=self.directory)

    def stale_directory(self):
        self.lock.mkdir()
        old = time.time() - 61
        os.utime(self.lock, (old, old))

    def info(self):
        return json.loads((self.lock / 'info.json').read_text())

    def test_takeover_rename_cannot_replace_completed_takeover(self):
        self.stale_directory()
        rename = quiet.os.rename
        competitor = []

        def before_rename(src, dst):
            if Path(src) == self.lock and not competitor:
                # Pause A after its identity check. B completes acquisition on
                # the old code; then A resumes and incorrectly replaces B.
                with patch.object(quiet.os, 'rename', rename):
                    competitor.append(self.take('B'))
            return rename(src, dst)

        with patch.object(quiet.os, 'rename', side_effect=before_rename):
            acquired = self.take('A')
        self.assertEqual(competitor, [False], 'B acquired during A takeover')
        self.assertTrue(acquired)
        self.assertEqual(self.info()['owner'], 'A')

    def test_release_cannot_remove_concurrent_takeover(self):
        self.assertTrue(self.take('A'))
        info = self.info()
        info['expected_end_epoch'] = time.time() - 1801
        (self.lock / 'info.json').write_text(json.dumps(info))
        read_info = quiet.read_info
        competitor = []

        def after_read(lock):
            observed = read_info(lock)
            with patch.object(quiet, 'read_info', read_info):
                competitor.append(quiet.acquire('B', 'sample', 1200,
                                               pid=os.getppid(), directory=self.directory))
            return observed

        with patch.object(quiet, 'read_info', side_effect=after_read):
            released = quiet.release(directory=self.directory)
        self.assertEqual(competitor, [False], 'B acquired after A ownership read')
        self.assertTrue(released)
        self.assertFalse(self.lock.exists())
        self.assertTrue(quiet.acquire('B', 'sample', 1200,
                                      pid=os.getppid(), directory=self.directory))
        self.assertFalse(quiet.release(directory=self.directory))
        self.assertEqual(self.info()['owner'], 'B')

    def test_delayed_initial_publisher_cannot_overwrite_later_takeover(self):
        temporary_file = quiet.tempfile.NamedTemporaryFile
        competitor = []

        def before_publish(*args, **kwargs):
            # A has created lock but has not opened its metadata file. Make
            # that unpublished directory old enough for B's stale recovery.
            old = time.time() - 61
            os.utime(self.lock, (old, old))
            with patch.object(quiet.tempfile, 'NamedTemporaryFile', temporary_file):
                competitor.append(self.take('B'))
            return temporary_file(*args, **kwargs)

        with patch.object(quiet.tempfile, 'NamedTemporaryFile', side_effect=before_publish):
            acquired = self.take('A')
        self.assertEqual(competitor, [False], 'B acquired before A publication')
        self.assertTrue(acquired)
        self.assertEqual(self.info()['owner'], 'A')


if __name__ == '__main__':
    unittest.main()
