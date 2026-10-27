"""Hermetic marker protocol and CLI regressions; no real sleeping."""

import concurrent.futures
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import host_quiet as quiet


class MarkerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.env = patch.dict(os.environ, HOST_QUIET_DIR=self.temp.name)
        self.env.start()
        self.addCleanup(self.env.stop)
        self.pid = os.getpid()
        self.lock = self.directory / 'lock'

    def take(self, **kwargs):
        return quiet.acquire('measurement', 'sample', 1200, **kwargs)

    def info(self):
        return json.loads((self.lock / 'info.json').read_text())

    def rewrite(self, **fields):
        info = self.info()
        info.update(fields)
        (self.lock / 'info.json').write_text(json.dumps(info))

    def cli(self, *args):
        return subprocess.run([sys.executable, str(Path(quiet.__file__)), *args],
                              capture_output=True, text=True, env=os.environ.copy())

    def test_acquire_and_second_acquire_refused(self):
        self.assertTrue(self.take())
        info = self.info()
        self.assertEqual(set(info), {'owner', 'step', 'pid', 'started_epoch', 'expected_end_epoch'})
        self.assertEqual(info['pid'], self.pid)
        self.assertIsInstance(info['started_epoch'], float)
        self.assertEqual(info['expected_end_epoch'] - info['started_epoch'], 1200)
        self.assertFalse(quiet.acquire('other', 'step', 10))
        self.assertEqual(self.info(), info)
        self.assertEqual(list(self.lock.iterdir()), [self.lock / 'info.json'])

    def test_holder_only_release_and_missing_lock(self):
        self.assertFalse(quiet.release())
        self.assertTrue(self.take())
        self.assertFalse(quiet.release(self.pid + 1))
        self.assertTrue(self.lock.exists())
        self.assertTrue(quiet.release(self.pid))
        self.assertFalse(self.lock.exists())
        self.assertFalse(quiet.release(self.pid))

    def test_dead_pid_stale_takeover_uses_rename(self):
        self.take()
        with patch.object(quiet.os, 'kill', side_effect=ProcessLookupError), \
                patch.object(quiet.os, 'rename', wraps=os.rename) as rename:
            self.assertTrue(quiet.acquire('replacement', 'next', 10))
        self.assertEqual(self.info()['owner'], 'replacement')
        self.assertEqual(rename.call_args_list[0].args[0], self.lock)
        self.assertTrue(rename.call_args_list[0].args[1].name.startswith('lock.stale-'))
        self.assertEqual(list(self.directory.iterdir()), [self.lock])

    def test_overdue_live_pid_and_exact_grace_boundary(self):
        self.take()
        now = time.time()
        self.rewrite(expected_end_epoch=now - 1800)
        self.assertFalse(quiet.inspect_lock(now=now)[1])
        self.assertTrue(quiet.inspect_lock(now=now + 0.01)[1])
        self.rewrite(expected_end_epoch=now - 1801)
        self.assertTrue(quiet.acquire('replacement', 'next', 1))

    def test_permission_error_means_alive(self):
        self.take()
        with patch.object(quiet.os, 'kill', side_effect=PermissionError):
            self.assertFalse(quiet.inspect_lock()[1])
            self.assertFalse(self.take())

    def test_missing_info_young_live_old_stale(self):
        self.lock.mkdir()
        now = time.time()
        os.utime(self.lock, (now, now))
        self.assertFalse(quiet.inspect_lock(now=now + 60)[1])
        self.assertFalse(self.take())
        self.assertFalse(quiet.release())
        os.utime(self.lock, (now - 61, now - 61))
        self.assertTrue(quiet.inspect_lock(now=now)[1])
        self.assertTrue(self.take())

    def test_malformed_info_uses_missing_info_grace(self):
        self.lock.mkdir()
        for content in ('{', '[]', '{}', '{"pid": true}',
                        json.dumps(dict(owner='a', step='b', pid=-1,
                                        started_epoch=0, expected_end_epoch=0)),
                        json.dumps(dict(owner='a', step='b', pid=self.pid,
                                        started_epoch=0, expected_end_epoch=float('nan')))):
            with self.subTest(content=content):
                (self.lock / 'info.json').write_text(content)
                now = time.time()
                os.utime(self.lock, (now, now))
                self.assertFalse(quiet.inspect_lock(now=now)[1])
                self.assertTrue(quiet.inspect_lock(now=now + 61)[1])
        with patch.object(Path, 'read_text', side_effect=PermissionError):
            self.assertIsNone(quiet.read_info(self.lock))

    def test_live_in_time_respected_and_free_does_not_create_directory(self):
        self.assertIsNone(quiet.inspect_lock())
        self.assertTrue(quiet.wait_until_free(self.fail, directory=self.directory / 'absent'))
        self.assertFalse((self.directory / 'absent').exists())
        self.take()
        self.assertFalse(quiet.inspect_lock()[1])
        self.assertFalse(self.take())

    def test_wait_release_with_injected_sleep_logs_once(self):
        self.take()
        now = [time.time()]
        calls = []
        logs = []

        def sleep(seconds):
            calls.append(seconds)
            now[0] += seconds
            if len(calls) == 2:
                quiet.release()

        self.assertTrue(quiet.wait_until_free(logs.append, poll=2,
                                             clock=lambda: now[0], sleep=sleep))
        self.assertEqual(calls, [2, 2])
        self.assertEqual(len(logs), 1)
        self.assertIn('owner=measurement, step=sample, expected end=', logs[0])
        self.assertIn(time.strftime('%Y-%m-%d', time.localtime(now[0])), logs[0])

    def test_wait_logs_holder_change(self):
        self.take()
        now = [time.time()]
        logs = []

        def sleep(seconds):
            now[0] += seconds
            quiet.release()
            if len(logs) == 1:
                quiet.acquire('second', 'next', 1200)

        self.assertTrue(quiet.wait_until_free(logs.append, clock=lambda: now[0], sleep=sleep))
        self.assertEqual(len(logs), 2)
        self.assertIn('owner=second', logs[1])

    def test_wait_timeout_caps_last_sleep(self):
        self.take()
        now = [time.time()]
        sleeps = []

        def sleep(seconds):
            sleeps.append(seconds)
            now[0] += seconds

        self.assertFalse(quiet.wait_until_free(lambda _: None, timeout=3, poll=2,
                                              clock=lambda: now[0], sleep=sleep))
        self.assertEqual(sleeps, [2, 1])
        self.assertTrue(self.lock.exists())

    def test_wait_stale_returns_without_sleeping(self):
        self.lock.mkdir()
        now = time.time()
        os.utime(self.lock, (now - 61, now - 61))
        self.assertTrue(quiet.wait_until_free(self.fail, clock=lambda: now, sleep=self.fail))
        self.assertTrue(self.lock.exists())

    def test_wait_unknown_holder_times_out_without_sleep(self):
        self.lock.mkdir()
        logs = []
        self.assertFalse(quiet.wait_until_free(logs.append, timeout=0, sleep=self.fail))
        self.assertEqual(len(logs), 1)
        self.assertIn('owner unknown', logs[0])

    def test_validation_rejects_bad_durations_and_pid(self):
        for value in (-1, float('nan'), float('inf')):
            with self.subTest(value=value), self.assertRaises(ValueError):
                quiet.acquire('a', 'b', value)
            with self.subTest(value=value), self.assertRaises(ValueError):
                quiet.wait_until_free(self.fail, timeout=value)
        for value in (0, -1, float('nan'), float('inf')):
            with self.subTest(poll=value), self.assertRaises(ValueError):
                quiet.wait_until_free(self.fail, poll=value)
        for pid in (0, -1, True):
            with self.subTest(pid=pid), self.assertRaises(ValueError):
                self.take(pid=pid)

    def test_cli_codes_holder_json_and_parent_pid_default(self):
        self.assertEqual(self.cli('status').stdout.strip(), 'free')
        self.assertEqual(self.cli('wait', '--timeout', '0').returncode, 0)
        acquired = self.cli('acquire', '--owner', 'cli', '--step', 'sample', '--expected-secs', '1200')
        self.assertEqual(acquired.returncode, 0, acquired.stderr)
        self.assertEqual(self.info()['pid'], self.pid)
        refused = self.cli('acquire', '--owner', 'other', '--step', 'sample', '--expected-secs', '1')
        self.assertEqual(refused.returncode, 1)
        self.assertEqual(json.loads(refused.stderr), self.info())
        status = self.cli('status')
        self.assertEqual(status.returncode, 0)
        self.assertFalse(json.loads(status.stdout)['stale'])
        timed_out = self.cli('wait', '--timeout', '0')
        self.assertEqual(timed_out.returncode, 3)
        self.assertIn('owner=cli', timed_out.stderr)
        wrong = self.cli('release', '--pid', str(self.pid + 1))
        self.assertEqual(wrong.returncode, 0)
        self.assertEqual(wrong.stdout.strip(), 'not released')
        released = self.cli('release')
        self.assertEqual(released.returncode, 0)
        self.assertEqual(released.stdout.strip(), 'released')
        self.assertEqual(self.cli('release').returncode, 0)

    def test_cli_explicit_pid_stale_status_and_invalid_arguments(self):
        result = self.cli('acquire', '--owner', 'cli', '--step', 'step',
                          '--expected-secs', '1', '--pid', str(self.pid))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.rewrite(expected_end_epoch=time.time() - 1801)
        self.assertTrue(json.loads(self.cli('status').stdout)['stale'])
        self.assertEqual(self.cli('wait', '--timeout', '0').returncode, 0)
        for args in (('wait', '--poll', '0'), ('wait', '--timeout', '-1'),
                     ('wait', '--poll', 'nan'), ('release', '--pid', '0'),
                     ('acquire', '--owner', 'x', '--step', 'y', '--expected-secs', 'inf')):
            with self.subTest(args=args):
                self.assertEqual(self.cli(*args).returncode, 2)

    def test_directory_override_and_expanduser(self):
        with patch.object(quiet.Path, 'expanduser', return_value=self.directory / 'marker'):
            self.assertEqual(quiet.marker_directory('~/marker'), self.directory / 'marker')
        with patch.dict(os.environ, {}, clear=True), \
                patch.object(quiet.Path, 'expanduser', autospec=True, return_value=self.directory) as expand:
            self.assertEqual(quiet.marker_directory(), self.directory)
            expand.assert_called_once_with(Path('~/.local/state/host-quiet'))
        other = self.directory / 'explicit'
        self.assertTrue(self.take(directory=other))
        self.assertFalse(self.lock.exists())
        self.assertTrue(quiet.release(directory=other))

    def test_stale_observation_cannot_remove_replaced_directory(self):
        self.lock.mkdir()
        now = time.time()
        os.utime(self.lock, (now - 61, now - 61))
        stale = quiet.inspect_lock()
        os.rename(self.lock, self.directory / 'old')
        self.take()
        original = self.info()
        with patch.object(quiet, 'inspect_lock', return_value=stale):
            self.assertFalse(self.take())
        self.assertEqual(self.info(), original)

    def test_losing_stale_rename_and_mkdir_races(self):
        self.lock.mkdir()
        now = time.time()
        os.utime(self.lock, (now - 61, now - 61))
        with patch.object(quiet.os, 'rename', side_effect=FileNotFoundError):
            self.assertFalse(self.take())
        original = quiet.shutil.rmtree

        def competing_acquirer(path):
            original(path)
            self.assertTrue(quiet.acquire('winner', 'next', 1200))

        with patch.object(quiet.shutil, 'rmtree', side_effect=competing_acquirer):
            self.assertFalse(self.take())
        self.assertEqual(self.info()['owner'], 'winner')

    def test_info_publish_failure_cleans_up_own_unpublished_lock(self):
        with patch.object(quiet.os, 'rename', side_effect=OSError('publish failed')):
            with self.assertRaisesRegex(OSError, 'publish failed'):
                self.take()
        self.assertFalse(self.lock.exists())
        self.assertTrue(self.take())

    def test_two_concurrent_acquirers_exactly_one_wins(self):
        barrier = threading.Barrier(2)

        def take(index):
            barrier.wait()
            return quiet.acquire(str(index), 'sample', 1200)

        with concurrent.futures.ThreadPoolExecutor(2) as pool:
            results = list(pool.map(take, range(2)))
        self.assertEqual(sorted(results), [False, True])
        self.assertIn(self.info()['owner'], ('0', '1'))


if __name__ == '__main__':
    unittest.main()
