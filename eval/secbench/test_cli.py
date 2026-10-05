"""Feature-availability regressions also runnable against a pre-change checkout."""
import os
from pathlib import Path
import subprocess
import sys
import unittest


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


if __name__ == '__main__':
    unittest.main()
