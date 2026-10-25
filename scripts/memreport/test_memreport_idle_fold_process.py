"""Inventory and pre-discovery custody tests with sequenced kernel responses."""
import signal
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch
from memreport_idle_process import Process
from memreport_idle_sampling import sweep


def process(tmp):
    p = object.__new__(Process)
    p.meta = dict(mode='shared',runtime_dir=tmp)
    p.members, p.status, p.owned_daemon = {1:10}, None, None
    p.cleanup_daemons = {}
    p.child = Mock(pid=1,returncode=0)
    p.child.poll.return_value = 0
    p.reader, p.stdout, p.stderr = Mock(), Mock(), Mock()
    p.sampler, p.control = Mock(), Mock(side_effect=OSError('control failed'))
    p.sampler.read.side_effect = lambda pid,*args: dict(pid=pid,os_start=pid*10,
        footprint_bytes=50,rss_bytes=60,reason=None)
    return p


class FoldProcess(unittest.TestCase):
    def test_inventory_recheck_failure_is_unknown(self):
        for reason in ('rusage_ESRCH','pid_start_changed','rusage_EPERM','rusage_EINVAL',
                       'missing_os_start','exited_process',None):
            with self.subTest(reason=reason), tempfile.TemporaryDirectory() as tmp:
                p = process(tmp)
                p.child.poll.return_value = None
                first = dict(pid=1,os_start=10,footprint_bytes=50,rss_bytes=60,reason=None)
                second = dict(first,reason=reason,os_start=10 if reason is None else None)
                p.sampler.read.side_effect = [first,second]
                result = sweep(p.sampler,p.members,p.inventory)
                self.assertEqual(result['footprint_bytes'],50 if reason is None else None)
                if reason:
                    self.assertIn('inventory_changed',result['reasons'])
                    self.assertTrue(any(reason in str(x) for x in result['reasons']))
                else:
                    self.assertEqual(result['reasons'],[])

    def test_pre_discovery_failure_has_custody(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = process(tmp)
            runtime = Path(tmp)/'prism-mcpd'; runtime.mkdir()
            (runtime/'key.pid').write_text('2\n3\n')
            with patch('memreport_idle_process.os.kill') as kill, patch('memreport_idle_process.time.sleep'), \
                 patch('memreport_idle_process.time.monotonic',side_effect=[0,6]):
                p.close()
            self.assertIn(2,p.members)
            self.assertFalse(p.cleanup_confirmed)
            kill.assert_called_once_with(2,signal.SIGTERM)
            p.sampler.read.return_value = dict(reason='rusage_ESRCH',os_start=None)
            p.sampler.read.side_effect = None
            with patch('memreport_idle_process.os.kill') as kill:
                p.close()
            self.assertFalse(kill.called)
            self.assertTrue(p.cleanup_confirmed)

    def test_shutdown_errors_and_unknown_identity(self):
        for error in (OSError('shutdown'),TimeoutError('shutdown')):
            with self.subTest(error=error), tempfile.TemporaryDirectory() as tmp:
                p = process(tmp)
                p.members[2] = 20
                p.owned_daemon = (Path(tmp)/'socket',dict(pid=2,start_time=3,identity='key'))
                p.control.side_effect = error
                with patch('memreport_idle_process.os.kill') as kill:
                    p.close()
                self.assertFalse(p.cleanup_confirmed)
                self.assertFalse(kill.called)
                p.sampler.read.side_effect = None
                p.sampler.read.return_value = dict(reason='missing_os_start',os_start=None)
                p.control.reset_mock()
                with patch('memreport_idle_process.os.kill') as kill:
                    p.close()
                self.assertFalse(p.cleanup_confirmed)
                self.assertFalse(kill.called)
                self.assertFalse(p.control.called)

    def test_status_missing_keys_still_binds_custody(self):
        for status in ('STATUS {}', 'STATUS {"pid":2}', 'invalid', 'STATUS {'):
            with self.subTest(status=status), tempfile.TemporaryDirectory() as tmp:
                p = process(tmp)
                p.stderr_path = Path(tmp)/'stderr'
                runtime = Path(tmp)/'prism-mcpd'; runtime.mkdir()
                (runtime/'key.pid').write_text('2\n3\n')
                p.control.side_effect = None
                p.control.return_value = status
                try:
                    p.discover()
                except Exception as error:
                    self.assertIsInstance(error,ValueError)
                else:
                    self.fail('invalid STATUS was accepted')
                self.assertEqual(p.members[2],20)
                self.assertIn(2,p.cleanup_daemons)
                self.assertEqual(p.cleanup_daemons[2],20)
                p.sampler.read.side_effect = lambda pid,*args: dict(reason='rusage_ESRCH',os_start=None)
                with patch('memreport_idle_process.os.kill') as kill:
                    p.close()
                self.assertTrue(p.cleanup_confirmed)
                self.assertFalse(kill.called)
