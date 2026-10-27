"""Fork custody, absent release and post-publication exception regressions."""

import fcntl
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

import host_quiet as quiet


# Keep the fork and crash in an isolated interpreter. Its child lives until the
# test closes a pipe, independently of the crashed writer's lifetime.
FORKED_WRITER = r'''
import os
from pathlib import Path
import signal
import sys
import threading
from unittest.mock import patch

sys.path.insert(0, sys.argv[1])
import host_quiet as quiet

directory = Path(sys.argv[2])
stop_fd = int(sys.argv[3])
phase = sys.argv[4]
reply_fd = int(sys.argv[5])
opened = threading.Event()
resume_open = threading.Event()
publishing = threading.Event()
forking = threading.Event()
original_open = quiet.os.open

def pause_open(path, *args, **kwargs):
    fd = original_open(path, *args, **kwargs)
    if Path(path) == directory / 'transition.guard' and phase == 'open':
        opened.set()
        if not resume_open.wait(5):
            raise RuntimeError('fork never started')
    return fd

def pause_publication(*args, **kwargs):
    publishing.set()
    threading.Event().wait()

def write():
    with patch.object(quiet.os, 'open', side_effect=pause_open), \
            patch.object(quiet.tempfile, 'NamedTemporaryFile', side_effect=pause_publication):
        quiet.acquire('writer', 'sample', 1200, directory=directory)

threading.Thread(target=write, daemon=True).start()
if not (opened if phase == 'open' else publishing).wait(5):
    raise RuntimeError('writer never paused')
if phase == 'open':
    # before callbacks run in reverse registration order: notify the releaser
    # before the module's fork callback waits for the in-progress open.
    os.register_at_fork(before=forking.set)
    def resume():
        if forking.wait(5):
            resume_open.set()
    threading.Thread(target=resume, daemon=True).start()
child = os.fork()
if child == 0:
    os.close(1)
    os.close(2)
    while os.read(stop_fd, 1):
        os.write(reply_fd, b'alive')
    os._exit(0)
if not publishing.wait(5):
    os.kill(child, signal.SIGKILL)
    raise RuntimeError('writer never reached publication')
print(child, flush=True)
signal.pause()
'''


class RoundTwoRegressions(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.lock = self.directory / 'lock'

    def take(self):
        return quiet.acquire('recovery', 'sample', 1200, directory=self.directory)

    def test_forked_child_cannot_keep_crashed_writers_guard(self):
        for phase in ('publication', 'open'):
            with self.subTest(phase=phase):
                stop_read, stop_write = os.pipe()
                reply_read, reply_write = os.pipe()
                with subprocess.Popen(
                        [sys.executable, '-c', FORKED_WRITER,
                         str(Path(quiet.__file__).resolve().parent),
                         str(self.directory), str(stop_read), phase, str(reply_write)],
                        pass_fds=(stop_read, reply_write), stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE, text=True) as writer:
                    os.close(stop_read)
                    os.close(reply_write)
                    try:
                        self.assertTrue(select.select([writer.stdout], [], [], 8)[0],
                                        'writer never reported its forked child')
                        line = writer.stdout.readline().strip()
                        self.assertTrue(line.isdigit(), 'writer failed before reporting child')
                        child = int(line)
                        old = time.time() - 61
                        os.utime(self.lock, (old, old))
                        self.assertTrue(quiet.inspect_lock(self.directory)[1])
                        with patch.object(quiet, 'GUARD_WAIT_SECS', 0.1):
                            self.assertFalse(self.take(), 'child unlocked the live writer')
                            writer.kill()
                            writer.wait(timeout=5)
                            os.write(stop_write, b'?')
                            self.assertTrue(select.select([reply_read], [], [], 5)[0],
                                            'child stopped executing after writer death')
                            self.assertEqual(os.read(reply_read, 5), b'alive')
                            self.assertTrue(self.take(), 'child retained the crashed writer guard')
                        self.assertTrue(quiet.release(directory=self.directory))
                    finally:
                        os.close(stop_write)  # Let the child exit even on assertion failure.
                        os.close(reply_read)
                        if writer.poll() is None:
                            writer.kill()
                            writer.wait(timeout=5)
                        if self.lock.exists():
                            # Base-control subtests must start with independent markers.
                            import shutil
                            shutil.rmtree(self.lock)

        # Forking inside the guard also leaves a context in the child. Unwinding
        # it must preserve a newly opened fd that reused the inherited fd number.
        context = quiet.transition_guard(self.directory)
        output = self.directory / 'child-output'
        with context as held:
            self.assertTrue(held)
            child = os.fork()
            if child == 0:
                try:
                    replacement = os.open(output, os.O_CREAT | os.O_WRONLY, 0o600)
                    context.__exit__(None, None, None)
                    os.write(replacement, b'child descriptor still open')
                    os.close(replacement)
                except BaseException:
                    os._exit(1)
                os._exit(0)
            os.waitpid(child, 0)
            self.assertEqual(output.read_bytes(), b'child descriptor still open')
            with patch.object(quiet, 'GUARD_WAIT_SECS', 0):
                with quiet.transition_guard(self.directory) as contender:
                    self.assertFalse(contender, 'child context unlocked parent guard')

        # A retry sleeper must not keep the module lock across its sleep: another
        # thread must be able to fork without waiting for the ten-second budget.
        sleeping = threading.Event()
        resume = threading.Event()
        errors = []

        def sleep(_):
            sleeping.set()
            resume.wait(2)

        def wait_for_guard():
            try:
                with quiet.transition_guard(self.directory) as held:
                    self.assertFalse(held)
            except BaseException as error:
                errors.append(error)

        with (self.directory / 'transition.guard').open('r') as guard:
            fcntl.flock(guard, fcntl.LOCK_EX)
            with patch.object(quiet.time, 'sleep', side_effect=sleep), \
                    patch.object(quiet, 'GUARD_WAIT_SECS', 0.1):
                waiter = threading.Thread(target=wait_for_guard)
                waiter.start()
                try:
                    self.assertTrue(sleeping.wait(5), 'acquirer never reached retry sleep')
                    started = time.monotonic()
                    child = os.fork()
                    if child == 0:
                        os._exit(0)
                    elapsed = time.monotonic() - started
                    os.waitpid(child, 0)
                    self.assertLess(elapsed, 1, 'retry sleep blocked another thread fork')
                finally:
                    resume.set()
                    waiter.join(timeout=5)
                self.assertFalse(waiter.is_alive())
                self.assertEqual(errors, [])

    def test_absent_release_does_not_touch_guard(self):
        for directory in (self.directory, self.directory / 'absent'):
            with self.subTest(directory=directory.name):
                with patch.object(quiet.os, 'open', side_effect=PermissionError('read only')) as opened:
                    self.assertFalse(quiet.release(directory=directory))
                opened.assert_not_called()
        self.assertFalse((self.directory / 'transition.guard').exists())
        self.assertFalse(quiet.release(directory=self.directory))
        self.assertFalse((self.directory / 'transition.guard').exists())

    def test_post_rename_exception_preserves_publication_and_original_error(self):
        rename = quiet.os.rename
        original = RuntimeError('interrupted after publication')

        def publish_then_raise(source, destination):
            rename(source, destination)
            raise original

        with patch.object(quiet.os, 'rename', side_effect=publish_then_raise):
            with self.assertRaises(RuntimeError) as raised:
                self.take()
        self.assertIs(raised.exception, original)
        info = json.loads((self.lock / 'info.json').read_text())
        self.assertEqual(info['owner'], 'recovery')
        self.assertFalse(self.take())
        self.assertTrue(quiet.release(directory=self.directory))
        # The guard was released despite the publication exception.
        self.assertTrue(self.take())


if __name__ == '__main__':
    unittest.main()
