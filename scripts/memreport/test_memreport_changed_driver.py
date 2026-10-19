"""Custody and cache isolation for the changed-refresh driver (synthetic fixtures only)."""
import copy
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport
from test_memreport_changed import changed


class ChangedDriver(unittest.TestCase):
    def fixture(self, root):
        edit = copy.deepcopy(changed()['changed_refresh']['edit'])
        edit['repo_root'] = str(root)
        target = root / edit['path']
        target.write_bytes(edit['before'].encode())
        cache = root / 'cache' / 'tiny'
        cache.mkdir(parents=True)
        (cache / 'seed').write_bytes(b'pristine prime')
        args = Namespace(out=root, bin='unused', timeout=1, settle=0, reps=2,
                         changed_edits={'tiny': ('manifest.json', edit)})
        return edit, target, cache, args

    def measure(self, root, args):
        with patch.object(memreport, 'host_state', return_value={}):
            return memreport.measure_corpus(args, 'tiny', root, {}, 'workload', {}, ('changed-refresh',))

    def test_wrong_starting_manifest_refused_without_child_or_recovery_write(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            edit, target, _, args = self.fixture(root)
            original = target.read_bytes()
            edit['before'] = 'self-consistent but wrong starting content'
            edit['before_sha256'] = hashlib.sha256(edit['before'].encode()).hexdigest()
            self.assertIsNone(memreport.edit_reason(edit))
            # If invoked, this child rejects without touching the mirror.
            with patch.object(memreport, 'run_one', return_value=dict(kind='changed-refresh', status='failed',
                              reason='wrong starting hash', wall_secs=0)) as child, \
                    patch.object(Path, 'write_bytes', wraps=target.write_bytes) as recovery:
                with self.assertRaises(SystemExit):
                    self.measure(root, args)
                self.assertEqual(target.read_bytes(), original)
                recovery.assert_not_called()
                child.assert_not_called()
            self.assertFalse((root / 'raw' / 'tiny' / 'changed-refresh-2.json').exists())

    def test_missing_target_refused_without_child_or_write(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            _, target, _, args = self.fixture(root)
            target.unlink()
            with patch.object(memreport, 'run_one', return_value=dict(kind='changed-refresh', status='failed',
                              reason='missing target', wall_secs=0)) as child, \
                    patch.object(Path, 'write_bytes') as recovery:
                with self.assertRaises(SystemExit):
                    self.measure(root, args)
                child.assert_not_called()
                recovery.assert_not_called()
            self.assertFalse(target.exists())

    def test_interrupt_and_exception_recover_verified_bytes_and_remove_cache(self):
        for exception in (KeyboardInterrupt, subprocess.TimeoutExpired('child', 1), OSError('child failed')):
            with self.subTest(exception=exception), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp).resolve()
                edit, target, _, args = self.fixture(root)
                calls = []
                def child(command, kind, *unused):
                    calls.append(kind)
                    if kind == 'changed-refresh':
                        target.write_text(edit['after'])
                        raise exception
                    return dict(kind=kind, status='ok', wall_secs=0)
                with patch.object(memreport, 'run_one', side_effect=child):
                    with self.assertRaises(type(exception) if isinstance(exception, BaseException) else exception):
                        self.measure(root, args)
                self.assertEqual(target.read_bytes(), edit['before'].encode())
                self.assertEqual(calls, [memreport.PRIME, 'changed-refresh'])
                self.assertFalse((root / 'cache-changed' / 'tiny' / '1').exists())
                self.assertFalse((root / 'raw' / 'tiny' / 'changed-refresh-2.json').exists())

    def test_two_successful_repetitions_use_fresh_copies_and_remove_them(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            _, _, cache, args = self.fixture(root)
            copies = []
            def child(command, kind, *unused):
                if kind == 'changed-refresh':
                    current = Path(command[command.index('--cache-dir') + 1])
                    self.assertNotEqual(current, cache)
                    self.assertEqual((current / 'seed').read_bytes(), b'pristine prime')
                    copies.append(current)
                    (current / 'seed').write_bytes(b'refreshed cache')
                return dict(kind=kind, status='ok', wall_secs=0)
            with patch.object(memreport, 'run_one', side_effect=child):
                _, results, _ = self.measure(root, args)
            self.assertEqual(len(set(copies)), 2)
            self.assertTrue(all(not item.exists() for item in copies))
            self.assertEqual((cache / 'seed').read_bytes(), b'pristine prime')
            self.assertTrue(all(result['cache_dir_removed'] for result in results))
            for rep in (1, 2):
                stored = json.loads((root / 'raw' / 'tiny' / ('changed-refresh-%d.json' % rep)).read_text())
                self.assertTrue(stored['cache_dir_removed'])

    def test_changed_target_rechecked_before_repetition_child(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            _, target, _, args = self.fixture(root)
            def prime(*unused):
                target.write_bytes(b'changed after preflight')
                return dict(kind=memreport.PRIME, status='ok', wall_secs=0)
            with patch.object(memreport, 'run_one', side_effect=prime) as child:
                with self.assertRaisesRegex(SystemExit, 'wrong starting content'):
                    self.measure(root, args)
                self.assertEqual(child.call_count, 1)
            self.assertEqual(target.read_bytes(), b'changed after preflight')

    def test_existing_cache_copy_refused_and_retained(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            edit, _, cache, args = self.fixture(root)
            existing = root / 'cache-changed' / 'tiny' / '1'
            existing.mkdir(parents=True)
            (existing / 'foreign').write_bytes(b'keep')
            with patch.object(memreport, 'run_one') as child:
                result = memreport.run_changed(args, root, root, 'tiny', 1, 'workload', cache, 'manifest', edit)
                child.assert_not_called()
            self.assertEqual(result['status'], 'inadmissible')
            self.assertEqual((existing / 'foreign').read_bytes(), b'keep')

    def test_failed_recovery_is_fatal_and_cache_removed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            edit, target, cache, args = self.fixture(root)
            def child(*unused):
                target.unlink()
                target.mkdir()
                raise KeyboardInterrupt
            with patch.object(memreport, 'run_one', side_effect=child):
                with self.assertRaisesRegex(RuntimeError, 'restore failed'):
                    memreport.run_changed(args, root, root, 'tiny', 1, 'workload', cache, 'manifest', edit)
            self.assertTrue(target.is_dir())
            self.assertFalse((root / 'cache-changed' / 'tiny' / '1').exists())


if __name__ == '__main__':
    unittest.main()
