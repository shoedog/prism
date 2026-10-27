"""Guard lifetime, bounded contention and read-only observer regressions."""

import fcntl
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import host_quiet as quiet


class TransitionGuardTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.guard = self.directory / 'transition.guard'

    def take(self):
        return quiet.acquire('measurement', 'sample', 1200, directory=self.directory)

    def test_interrupted_and_busy_attempts_retry_then_acquire(self):
        flock = fcntl.flock
        errors = [InterruptedError, BlockingIOError]
        ticks = [0.0]
        sleeps = []

        def attempt(fd, mode):
            if mode & fcntl.LOCK_NB and errors:
                raise errors.pop(0)()
            return flock(fd, mode)

        def sleep(seconds):
            sleeps.append(seconds)
            ticks[0] += seconds

        with patch.object(fcntl, 'flock', side_effect=attempt), \
                patch.object(quiet.time, 'monotonic', side_effect=lambda: ticks[0]), \
                patch.object(quiet.time, 'sleep', side_effect=sleep):
            self.assertTrue(self.take())
        self.assertEqual(sleeps, [0.05, 0.05])
        self.assertTrue(quiet.release(directory=self.directory))

    def test_stalled_guard_bounds_acquire_and_release_without_mutation(self):
        for existing in (False, True):
            with self.subTest(existing=existing):
                if existing:
                    self.assertTrue(self.take())
                lock = self.directory / 'lock'
                original = (lock / 'info.json').read_bytes() if existing else None
                with self.guard.open('a') as held:
                    fcntl.flock(held, fcntl.LOCK_EX)
                    for operation in (self.take, lambda: quiet.release(directory=self.directory)):
                        ticks = [0.0]
                        sleeps = []

                        def sleep(seconds):
                            sleeps.append(seconds)
                            ticks[0] += seconds

                        with patch.object(quiet.time, 'monotonic', side_effect=lambda: ticks[0]), \
                                patch.object(quiet.time, 'sleep', side_effect=sleep):
                            self.assertFalse(operation())
                        self.assertAlmostEqual(ticks[0], 10.0)
                        self.assertTrue(all(0 < delay <= 0.05 for delay in sleeps))
                        self.assertEqual(lock.exists(), existing)
                        if existing:
                            self.assertEqual((lock / 'info.json').read_bytes(), original)
                if existing:
                    self.assertTrue(quiet.release(directory=self.directory))

    def test_observers_never_take_or_create_guard(self):
        with patch.object(fcntl, 'flock', side_effect=AssertionError('observer took guard')):
            self.assertIsNone(quiet.inspect_lock(self.directory))
            self.assertTrue(quiet.wait_until_free(self.fail, directory=self.directory))
        self.assertFalse(self.guard.exists())
        self.assertTrue(self.take())
        with self.guard.open('r') as held:
            fcntl.flock(held, fcntl.LOCK_EX)
            with patch.object(fcntl, 'flock', side_effect=AssertionError('observer took guard')):
                self.assertFalse(quiet.inspect_lock(self.directory)[1])
                self.assertFalse(quiet.wait_until_free(lambda _: None, timeout=0,
                                                       directory=self.directory))

    def test_guard_inode_survives_release_takeover_and_publish_failure(self):
        self.assertTrue(self.take())
        identity = self.guard.stat().st_ino
        self.assertTrue(quiet.release(directory=self.directory))
        self.assertEqual(self.guard.stat().st_ino, identity)
        with patch.object(quiet.os, 'rename', side_effect=OSError('publication failed')):
            with self.assertRaisesRegex(OSError, 'publication failed'):
                self.take()
        self.assertFalse((self.directory / 'lock').exists())
        self.assertEqual(self.guard.stat().st_ino, identity)
        self.assertTrue(self.take())
        with patch.object(quiet.os, 'kill', side_effect=ProcessLookupError):
            self.assertTrue(self.take())
        self.assertEqual(self.guard.stat().st_ino, identity)

    def test_kernel_releases_guard_when_holder_dies(self):
        program = (
            'import fcntl, signal, sys\n'
            'with open(sys.argv[1], "a") as guard:\n'
            '    fcntl.flock(guard, fcntl.LOCK_EX)\n'
            '    print("held", flush=True)\n'
            '    signal.pause()\n'
        )
        with subprocess.Popen([sys.executable, '-c', program, str(self.guard)],
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) as holder:
            try:
                self.assertTrue(select.select([holder.stdout], [], [], 5)[0], 'holder never became ready')
                self.assertEqual(holder.stdout.readline().strip(), 'held')
                with patch.object(quiet, 'GUARD_WAIT_SECS', 0):
                    self.assertFalse(self.take())
                holder.kill()
                holder.wait(timeout=5)
                self.assertTrue(self.take())
            finally:
                if holder.poll() is None:
                    holder.kill()
                    holder.wait(timeout=5)


if __name__ == '__main__':
    unittest.main()
