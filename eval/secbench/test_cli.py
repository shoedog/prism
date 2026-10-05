"""Feature-availability regressions also runnable against a pre-change checkout."""
import os
import json
from argparse import Namespace
from pathlib import Path
import subprocess
import sys
import unittest
import tempfile
from unittest.mock import patch
from . import run


class CliTests(unittest.TestCase):
    def command(self, *args):
        root = Path(os.environ.get('PRISM_SECBENCH_TEST_ROOT', Path(__file__).resolve().parents[2]))
        env = {k: v for k, v in os.environ.items() if k != 'PYTHONPATH'}
        env['PYTHONDONTWRITEBYTECODE'] = '1'
        return subprocess.run([sys.executable, '-m', 'eval.secbench', *args], cwd=root,
                              env=env, text=True, capture_output=True, timeout=10)

    def test_module_exposes_the_offline_benchmark_command(self):
        result = self.command('--help')
        self.assertIn('Offline SecBench ground-truth measurement', result.stdout)
        for flag in ('--sut-repo', '--adjudications', '--inspection', '--no-build'):
            self.assertIn(flag, result.stdout)
        self.assertEqual(result.stderr, '')

    def test_unknown_flag_is_rejected_before_any_measurement(self):
        result = self.command('--not-a-secbench-option')
        self.assertIn('unrecognized arguments: --not-a-secbench-option', result.stderr)
        self.assertEqual(result.stdout, '')

    def test_configurable_inputs_and_pinned_binary_guard(self):
        result = self.command('--help')
        for flag in ('--inputs', '--packages', '--compiler', '--binary', '--timeout', '--workers'):
            self.assertIn(flag, result.stdout)
        # Exercise the guard with mocked custody reads; never build or query a package.
        manifest = json.dumps({'secbench_commit':run.SEC_COMMIT,
                               'entries':[{'status':'excluded'} for _ in range(600)]}).encode()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            args = Namespace(out=root, packages=root, inputs=root, sut_repo=root,
                             binary=root/'absent-binary', no_build=False)
            with patch.object(run.argparse.ArgumentParser,'parse_args',return_value=args), \
                 patch.object(Path,'read_bytes',return_value=manifest), \
                 patch.object(run.subprocess,'check_output',side_effect=[run.SUT_COMMIT,run.SEC_COMMIT]), \
                 patch.object(run.subprocess,'run') as command:
                with self.assertRaisesRegex(ValueError,'--binary requires --no-build'):
                    run.main()
                self.assertEqual(command.call_count,1)
                self.assertEqual(command.call_args.args[0][0],'git')

    def test_environment_defaults_can_be_overridden_without_io(self):
        root = Path(os.environ.get('PRISM_SECBENCH_TEST_ROOT', Path(__file__).resolve().parents[2]))
        env = {k: v for k, v in os.environ.items() if k != 'PYTHONPATH'}
        env.update(PYTHONDONTWRITEBYTECODE='1', PRISM_EVIDENCE_ROOT='/private/tmp/secbench-config-only',
                   PRISM_TYPESCRIPT='/private/tmp/secbench-config-only/compiler.js')
        result = subprocess.run([sys.executable, '-c',
            'import json; from eval.secbench import run; print(json.dumps([str(run.EVIDENCE), str(run.COMPILER)]))'],
            cwd=root, env=env, text=True, capture_output=True, timeout=10)
        self.assertEqual(result.stderr, '')
        self.assertEqual(json.loads(result.stdout), [env['PRISM_EVIDENCE_ROOT'], env['PRISM_TYPESCRIPT']])

    def test_report_command_is_available_and_rejects_unknown_flags_before_io(self):
        root = Path(os.environ.get('PRISM_SECBENCH_TEST_ROOT', Path(__file__).resolve().parents[2]))
        env = {k: v for k, v in os.environ.items() if k != 'PYTHONPATH'}
        env['PYTHONDONTWRITEBYTECODE'] = '1'
        def command(*args):
            return subprocess.run([sys.executable, '-m', 'eval.secbench.report', *args],
                                  cwd=root, env=env, text=True, capture_output=True, timeout=10)
        result = command('--help')
        self.assertIn('Source-bound R1 decision tables', result.stdout)
        for flag in ('--run', '--baseline', '--conversions', '--out'):
            self.assertIn(flag, result.stdout)
        result = command('--run', '/private/tmp', '--baseline', '/private/tmp',
                         '--conversions', '/private/tmp', '--out', '/private/tmp', '--unknown-report-option')
        self.assertIn('unrecognized arguments: --unknown-report-option', result.stderr)
        self.assertEqual(result.stdout, '')


if __name__ == '__main__':
    unittest.main()
