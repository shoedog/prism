"""Feature-availability regressions also runnable against a pre-change checkout."""
import contextlib
import io
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
                             binary=root/'absent-binary', no_build=False,
                             class_=None, binary_sha=None, sut_commit=None)
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


class MainDriver(unittest.TestCase):
    """Drives main() end to end on a tiny synthetic population: real argument parsing and
    real hashing, with git, the binary, measurement and the committed input pins faked."""
    CLASSES = ('alpha', 'beta')

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name).resolve()
        self.packages, self.inputs, self.out = self.root/'pkgs', self.root/'inputs', self.root/'out'
        self.packages.mkdir()
        self.inputs.mkdir()
        self.compiler = self.root/'compiler.js'
        self.compiler.write_bytes(b'compiler')
        self.binary = self.root/'prism'
        self.binary.write_bytes(b'fake-binary')
        self.sut_repo = self.root/'sut'
        self.sut_repo.mkdir()
        entries = []
        for i in range(600):
            cls = self.CLASSES[i % 2]
            if i < 6:   # three ok entries per class
                tarball = self.packages/cls/f'e{i}'/'x.tgz'
                tarball.parent.mkdir(parents=True, exist_ok=True)
                tarball.write_bytes(b'tar%d' % i)
                entries.append({'class': cls, 'entry': f'e{i}', 'status': 'ok', 'tarball': 'x.tgz',
                                'sha256': run.digest(tarball.read_bytes())})
            else:
                entries.append({'class': cls, 'entry': f'x{i}', 'status': 'unavailable'})
        self.manifest = {'secbench_commit': run.SEC_COMMIT, 'entries': entries}
        (self.packages/'manifest.json').write_bytes(json.dumps(self.manifest).encode())
        self.inspection = self.root/'inspection.jsonl'
        self.inspection.write_bytes(b''.join(json.dumps({'class': e['class'], 'entry': e['entry']}).encode() + b'\n'
                                             for e in entries if e['status'] == 'ok'))
        self.excluded = {c: sum(e['class'] == c and e['status'] != 'ok' for e in entries) for c in self.CLASSES}
        self.git, self.version = [], f'prism 1.0 ({run.SUT_COMMIT[:12]})'

    def check_output(self, command, **kwargs):
        if command[0] == str(self.binary):
            return self.version + '\n'
        if command[:2] == ['git', 'rev-parse']:
            return run.SUT_COMMIT + '\n'
        self.assertEqual(command[:2], ['git', '-C'])
        return run.SEC_COMMIT + '\n'

    def call(self, *argv, binary_sha=None, sut_commit=None):
        """Run main(); the pinned constants are patched to sentinels unless a test overrides them."""
        base = ['--inputs', str(self.inputs), '--packages', str(self.packages), '--sut-repo', str(self.sut_repo),
                '--compiler', str(self.compiler), '--out', str(self.out), '--inspection', str(self.inspection),
                '--binary', str(self.binary), '--no-build']
        self.measured, self.verified = [], []
        real_read = Path.read_bytes
        pins = None

        def read_bytes(path):
            if path.name == 'input-pins.json':
                return run.canonical({'schema': 'prism.secbench-input-pins/1',
                                      'manifest_sha256': run.digest((self.packages/'manifest.json').read_bytes()),
                                      'compiler_sha256': run.digest(self.compiler.read_bytes()), 'packages': []})
            return real_read(path)

        def measure(row, binary, packages, output, timeout):
            self.measured.append((row['class'], row['entry']))
            return {'class': row['class'], 'entry': row['entry'], 'outcome': 'traced'}

        def verify(rows, inputs, packages):
            self.verified.append(len(rows))
            return []

        def fake_run(command, **kwargs):
            self.git.append(command)

        stdout = io.StringIO()
        with patch.object(run, 'SUT_COMMIT', run.SUT_COMMIT), \
             patch.object(run, 'BINARY_SHA', run.digest(b'fake-binary') if binary_sha is None else binary_sha), \
             patch.object(sys, 'argv', ['run', *base, *argv]), patch.object(Path, 'read_bytes', read_bytes), \
             patch.object(run.subprocess, 'check_output', self.check_output), \
             patch.object(run.subprocess, 'run', fake_run), patch.object(run, 'measure', measure), \
             patch.object(run, 'verify_inspection', verify), contextlib.redirect_stdout(stdout):
            run.main()
        self.stdout = stdout.getvalue()
        return json.loads(real_read(self.out/'summary.json')), json.loads(real_read(self.out/'binding.json'))

    def entry_rows(self):
        return [json.loads(line) for line in (self.out/'entries.jsonl').read_text().splitlines()]


class ClassFilterAndPinOverrideTests(MainDriver):
    def test_default_run_is_unchanged(self):
        summary, binding = self.call()
        self.assertEqual(len(self.measured), 6)
        self.assertEqual(len(self.entry_rows()), 600)
        self.assertEqual(self.verified, [6])
        self.assertNotIn('class_filter', summary)
        self.assertNotIn('pins_overridden', summary)
        self.assertNotIn('class_filter', binding)
        self.assertNotIn('pins_overridden', binding)
        self.assertEqual(binding['sut_head'], run.SUT_COMMIT)
        self.assertEqual(self.git[0][:3], ['git', 'diff', '--exit-code'])
        self.assertEqual(self.git[0][3], run.SUT_COMMIT)
        self.assertEqual(sorted(summary['by_class']), ['alpha', 'beta'])
        self.assertIn('6/6 ', self.stdout)

    def test_class_filter_measures_only_that_class(self):
        summary, binding = self.call('--class', 'beta')
        self.assertEqual({c for c, _ in self.measured}, {'beta'})
        self.assertEqual(len(self.measured), 3)
        rows = self.entry_rows()
        self.assertEqual({r['class'] for r in rows}, {'beta'})
        self.assertEqual(len(rows), 3 + self.excluded['beta'])
        self.assertEqual(sorted(summary['by_class']), ['beta'])
        self.assertEqual(summary['class_filter'], ['beta'])
        self.assertEqual(binding['class_filter'], ['beta'])
        self.assertNotIn('pins_overridden', summary)
        # Verification still covers the whole acquired population.
        self.assertEqual(self.verified, [6])
        self.assertIn('3/3 beta/', self.stdout)
        self.assertNotIn('alpha/', self.stdout)

    def test_class_filter_is_repeatable_and_rejects_unknown_classes(self):
        summary, _ = self.call('--class', 'beta', '--class', 'alpha', '--class', 'beta')
        self.assertEqual(len(self.measured), 6)
        self.assertEqual(summary['class_filter'], ['alpha', 'beta'])
        with self.assertRaisesRegex(ValueError, 'unknown --class'):
            self.call('--class', 'gamma')

    def test_class_filter_restricts_adjudication_labels_to_measured_classes(self):
        labels = self.root/'labels.json'
        labels.write_bytes(json.dumps({'sut_commit': run.SUT_COMMIT, 'secbench_commit': run.SEC_COMMIT,
                                       'entries': [{'class': 'alpha', 'entry': 'e0', 'expected_outcome': 'traced'},
                                                   {'class': 'beta', 'entry': 'e1', 'expected_outcome': 'traced'}]}).encode())
        summary, _ = self.call('--class', 'beta', '--adjudications', str(labels))
        self.assertEqual(summary['adjudication']['sample_entries'], 1)

    def test_pin_overrides_replace_both_checks_and_are_recorded(self):
        new_commit, new_sha = 'b' * 40, run.digest(b'fake-binary')
        self.version = f'prism 1.0 ({new_commit[:12]})'
        summary, binding = self.call('--sut-commit', new_commit, '--binary-sha', new_sha, binary_sha='0' * 64)
        # the diff check, the version check and the recorded head all use the override
        self.assertEqual(self.git[0][3], new_commit)
        self.assertNotIn(run.SUT_COMMIT, self.git[0])
        self.assertEqual(binding['sut_head'], new_commit)
        self.assertEqual(binding['binary_sha256'], new_sha)
        self.assertEqual(summary['pins_overridden'], {'binary_sha': new_sha, 'sut_commit': new_commit})
        self.assertEqual(binding['pins_overridden'], summary['pins_overridden'])
        self.assertNotIn('class_filter', summary)
        self.assertEqual(len(self.measured), 6)
        # the built-in pins no longer apply: each override replaces its own check only
        self.version = f'prism 1.0 ({run.SUT_COMMIT[:12]})'
        with self.assertRaisesRegex(ValueError, 'binary identity mismatch'):
            self.call('--sut-commit', new_commit, '--binary-sha', new_sha)
        self.version = f'prism 1.0 ({new_commit[:12]})'
        with self.assertRaisesRegex(ValueError, 'binary SHA256 pin mismatch'):
            self.call('--sut-commit', new_commit, '--binary-sha', '1' * 64)
        with self.assertRaisesRegex(ValueError, 'binary SHA256 pin mismatch'):
            self.call('--sut-commit', new_commit, binary_sha='0' * 64)   # an sut-commit override leaves the SHA pin in force
        # a single override records the other as null
        self.version = f'prism 1.0 ({run.SUT_COMMIT[:12]})'
        summary, _ = self.call('--binary-sha', run.digest(b'fake-binary'), binary_sha='0' * 64)
        self.assertEqual(summary['pins_overridden'], {'binary_sha': run.digest(b'fake-binary'), 'sut_commit': None})

    def test_override_commit_must_be_a_full_sha_and_labels_use_the_effective_commit(self):
        with self.assertRaisesRegex(ValueError, '40 lowercase hex'):
            self.call('--sut-commit', 'abc123')
        new_commit = 'c' * 40
        self.version = f'prism 1.0 ({new_commit[:12]})'
        labels = self.root/'labels.json'
        body = {'sut_commit': new_commit, 'secbench_commit': run.SEC_COMMIT, 'entries': []}
        labels.write_bytes(json.dumps(body).encode())
        self.call('--sut-commit', new_commit, '--adjudications', str(labels))
        body['sut_commit'] = run.SUT_COMMIT
        labels.write_bytes(json.dumps(body).encode())
        with self.assertRaisesRegex(ValueError, 'adjudication revision mismatch'):
            self.call('--sut-commit', new_commit, '--adjudications', str(labels))


if __name__ == '__main__':
    unittest.main()
