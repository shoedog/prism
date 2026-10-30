"""Changed-refresh evidence must bind exact content, identity and restoration."""
import hashlib
import sys
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport
from memreport_fixtures import canned


def changed():
    data = canned('warm')
    data['stages'].append(canned('refresh')['stages'][-1])
    data['refresh'] = dict(status='refreshed')
    edit = dict(repo_root='/private/mirror', commit='a' * 40, path='main.py',
                before='def run():\n    return 1\n', after='def run():\n    return 2\n')
    for key in ('before', 'after'):
        edit[key + '_sha256'] = hashlib.sha256(edit[key].encode()).hexdigest()
    data['changed_refresh'] = dict(edit=edit, before_sha256=edit['before_sha256'],
                                   after_sha256=edit['after_sha256'], restored_sha256=edit['before_sha256'])
    data['refresh'].update(strategy='full', generation=1, fallback_reason=None, indexed_files=1,
                           tracked_paths=1, stale_before_refresh=True, stale_index_total_before_refresh=1,
                           stale_index_paths_before_refresh=['main.py'])
    return data


class ChangedRefresh(unittest.TestCase):
    def test_changed_kind_and_command(self):
        self.assertIn('changed-refresh', memreport.parse_kinds('changed-refresh'))
        command = memreport.command_for('bin', 'repo', 'cache', 'changed-refresh', 'w', 0, 'edit.json')
        self.assertEqual(command[-2:], ['--mem-changed-refresh', 'edit.json'])
        self.assertIsNone(memreport.validate('changed-refresh', changed()))

    def test_validator_rejects_wrong_hash(self):
        for key in ('before_sha256', 'after_sha256', 'restored_sha256'):
            with self.subTest(key=key):
                data = changed()
                data['changed_refresh'][key] = '0' * 64
                self.assertIn('hash', memreport.validate('changed-refresh', data))

    def test_validator_rejects_noop_missing_identity_cache_or_stages(self):
        for section, key, value in [('edit', 'after', ''), ('edit', 'after', '\ud800'), ('edit', 'commit', ''),
                                     ('cache', 'cpg_hit', False), ('refresh', 'indexed_files', -1),
                                     ('refresh', 'stale_before_refresh', False)]:
            data = changed()
            target = data['changed_refresh']['edit'] if section == 'edit' else data[section]
            target[key] = value
            self.assertIsNotNone(memreport.validate('changed-refresh', data))
        data = changed()
        data['stages'] = []
        self.assertIsNotNone(memreport.validate('changed-refresh', data))

    def test_changed_metrics_and_stored_rescore(self):
        data = changed()
        self.assertIn('changed_refresh_peak', memreport.derive('changed-refresh', data))
        result = dict(kind='changed-refresh', rep=1, status='ok', report=data)
        self.assertEqual(memreport.effective_status(result), ('ok', None))
        self.assertIsNotNone(memreport.validate('changed-refresh', data, {}))
        result['report']['changed_refresh']['restored_sha256'] = '0' * 64
        self.assertEqual(memreport.effective_status(result)[0], 'inadmissible')

    def test_failed_child_restored_and_no_next_repetition(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            edit = changed()['changed_refresh']['edit']
            target = root / edit['path']
            target.write_text(edit['before'])
            (root / 'cache' / 'tiny').mkdir(parents=True)
            args = Namespace(out=root, bin='unused', timeout=1, settle=0, reps=2,
                             changed_edits={'tiny': ('manifest.json', edit)})
            def child(command, kind, *unused):
                if kind == 'changed-refresh':
                    target.write_text(edit['after'])
                return dict(kind=kind, status='timeout', wall_secs=1)
            with patch.object(memreport, 'run_one', side_effect=child):
                with self.assertRaises(SystemExit):
                    memreport.measure_corpus(args, 'tiny', root, {}, 'workload', {}, ('changed-refresh',))
            self.assertEqual(target.read_text(), edit['before'])
            self.assertFalse((root / 'raw' / 'tiny' / 'changed-refresh-2.json').exists())
