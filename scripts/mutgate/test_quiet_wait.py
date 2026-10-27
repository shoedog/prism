"""Pre-start quiet wait decisions, without executing a mutation gate."""

import argparse
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import mutgate


class QuietWaitTests(unittest.TestCase):
    def test_skip_switches(self):
        for plan, disabled, env in ((True, False, {}), (False, True, {}),
                                    (False, False, {'HOST_QUIET_IGNORE': '1'})):
            with self.subTest(plan=plan, disabled=disabled, env=env):
                waiter = Mock()
                mutgate.quiet_wait(argparse.Namespace(plan_only=plan, no_quiet_wait=disabled),
                                   env, Mock(), waiter)
                waiter.assert_not_called()

    def test_wait_otherwise_without_timeout(self):
        for env in ({}, {'HOST_QUIET_IGNORE': '0'}, {'HOST_QUIET_IGNORE': 'true'}):
            with self.subTest(env=env):
                waiter, log = Mock(), Mock()
                mutgate.quiet_wait(argparse.Namespace(plan_only=False, no_quiet_wait=False),
                                   env, log, waiter)
                waiter.assert_called_once_with(log, timeout=None)

    def test_main_wait_precedes_population_and_output_creation(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'report'

            def waited(args, env, log):
                self.assertFalse(out.exists())
                log('test wait')

            class StopPlanning(Exception):
                pass

            with patch.object(sys, 'argv', ['mutgate', '--out', str(out)]), \
                    patch.object(mutgate, 'quiet_wait', side_effect=waited) as wait, \
                    patch.object(mutgate, 'load_population', side_effect=StopPlanning):
                with self.assertRaises(StopPlanning):
                    mutgate.main()
                wait.assert_called_once()
                self.assertTrue(out.exists())

    def test_invalid_arguments_do_not_wait(self):
        for args in (['--jobs', '0'], ['--mode', 'schema']):
            with self.subTest(args=args), patch.object(sys, 'argv', ['mutgate', *args]), \
                    patch.object(mutgate, 'quiet_wait') as wait:
                with self.assertRaises(SystemExit) as error:
                    mutgate.main()
                self.assertEqual(error.exception.code, 2)
                wait.assert_not_called()

    def test_wait_changes_only_total_in_summary(self):
        summaries = []
        for ignored in (False, True):
            with tempfile.TemporaryDirectory() as tmp:
                out = Path(tmp) / 'report'
                ticks = [0.0]
                original = mutgate.quiet_wait

                def waiting(args, env, log):
                    def waiter(log, timeout):
                        self.assertIsNone(timeout)
                        ticks[0] += 25.0
                        return True
                    return original(args, env, log, waiter)

                flags = ['--no-quiet-wait'] if ignored else []
                with patch.object(sys, 'argv', ['mutgate', '--out', str(out), *flags]), \
                        patch.object(mutgate, 'ROOT', Path(tmp)), \
                        patch.object(mutgate, 'load_population', return_value={}), \
                        patch.object(mutgate, 'quiet_wait', side_effect=waiting), \
                        patch.object(mutgate.time, 'monotonic', side_effect=lambda: ticks[0]), \
                        patch.dict(os.environ, {'HOST_QUIET_IGNORE': '0'}), \
                        contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(mutgate.main(), 0)
                summaries.append(json.loads((out / 'summary.json').read_text()))
        self.assertEqual(summaries[0].pop('timings'), {'total': 25.0})
        self.assertEqual(summaries[1].pop('timings'), {'total': 0.0})
        self.assertEqual(summaries[0], summaries[1])

    def test_import_independent_of_working_directory_and_cargo_target(self):
        path = Path(mutgate.__file__).resolve()
        with tempfile.TemporaryDirectory() as tmp, \
                patch.dict(os.environ, CARGO_TARGET_DIR=tmp), \
                patch.object(sys, 'path', [p for p in sys.path if 'hostquiet' not in p]):
            previous = Path.cwd()
            try:
                os.chdir(tmp)
                spec = importlib.util.spec_from_file_location('mutgate_import_probe', path)
                module = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(module)
                self.assertEqual(Path(sys.path[0]), path.parents[1] / 'hostquiet')
                self.assertTrue(callable(module.wait_until_free))
            finally:
                os.chdir(previous)


if __name__ == '__main__':
    unittest.main()
