import gzip
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
from . import run


class InvocationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='secbench-invoke-')
        self.addCleanup(self.tmp.cleanup)
        self.raw = Path(self.tmp.name)

    def invoke(self, code, **options):
        return run.invoke(Path(sys.executable), ['-c', code], self.raw,
                          'query', options.pop('timeout', 5), **options)

    def test_json_and_jsonl_success_preserve_authenticated_output(self):
        value, rec = self.invoke('print(\'{"items":[]}\')')
        self.assertEqual(value, {'items': []})
        self.assertIsNone(rec['error'])
        self.assertEqual(run.digest(gzip.open(rec['raw'], 'rb').read()), rec['stdout_sha256'])
        value, rec = self.invoke('print(\'{"item":1}\\n{"item":2}\')', jsonl=True)
        self.assertEqual(value, [{'item': 1}, {'item': 2}])
        self.assertFalse((self.raw / 'query.stdout').exists())
        self.assertFalse((self.raw / 'query.stderr').exists())

    def test_nonzero_json_error_is_preserved_not_a_negative_result(self):
        value, rec = self.invoke('import sys; print(\'{"error":{"SymbolNotFound":{"seed":"x.js:1"}}}\'); sys.exit(3)')
        self.assertIsNone(value)
        self.assertEqual(rec['output_error'], {'SymbolNotFound': {'seed': 'x.js:1'}})
        self.assertIn('exited 3', rec['error'])

    def test_invalid_json_and_timeout_are_errors_with_retained_bytes(self):
        value, rec = self.invoke('print("not JSON")')
        self.assertIsNone(value)
        self.assertIn('invalid output', rec['error'])
        self.assertEqual(gzip.open(rec['raw'], 'rb').read(), b'not JSON\n')
        value, rec = self.invoke('import time; time.sleep(5)', timeout=0.05)
        self.assertIsNone(value)
        self.assertIn('timeout', rec['error'])
        self.assertTrue(Path(rec['raw']).is_file())


class InputPinTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='secbench-pins-')
        self.addCleanup(self.tmp.cleanup)
        base = Path(self.tmp.name)
        self.inputs, self.packages = base / 'inputs', base / 'packages'
        self.source = self.packages / 'redos/p_1/src/package/index.js'
        self.metadata = self.inputs / 'redos/p_1/package.json'
        self.exploit = self.inputs / 'redos/p_1/p.test.js'
        self.source.parent.mkdir(parents=True)
        self.metadata.parent.mkdir(parents=True)
        self.source.write_bytes(b'function p(x) { return x; }')
        self.metadata.write_bytes(b'{}')
        self.exploit.write_bytes(b'require("p")("payload")')
        self.row = {'class': 'redos', 'entry': 'p_1',
                    'identities': [{'path': 'index.js', 'bytes': self.source.stat().st_size,
                                    'sha256': run.digest(self.source.read_bytes())}],
                    'exploits': [{'path': 'redos/p_1/p.test.js', 'sha256': run.digest(self.exploit.read_bytes())}],
                    'metadata_sha256': run.digest(self.metadata.read_bytes())}

    def test_unchanged_population_produces_stable_pins(self):
        pins = run.verify_inspection([self.row], self.inputs, self.packages)
        self.assertEqual(pins, run.verify_inspection([self.row], self.inputs, self.packages))
        self.assertEqual(pins[0]['tree_sha256'], run.digest(run.canonical(self.row['identities'])))

    def test_source_exploit_and_metadata_drift_are_rejected(self):
        for file, message in ((self.source, 'input bytes drift'),
                              (self.exploit, 'exploit bytes drift'),
                              (self.metadata, 'metadata drift')):
            original = file.read_bytes()
            with self.subTest(file=file.name):
                file.write_bytes(original + b'changed')
                with self.assertRaisesRegex(ValueError, message):
                    run.verify_inspection([self.row], self.inputs, self.packages)
            file.write_bytes(original)

    def test_added_file_and_symlink_are_rejected(self):
        extra = self.source.parent / 'extra.js'
        extra.write_bytes(b'extra')
        with self.assertRaisesRegex(ValueError, 'input population drift'):
            run.verify_inspection([self.row], self.inputs, self.packages)
        extra.unlink()
        external = Path(self.tmp.name) / 'external.js'
        external.write_bytes(self.source.read_bytes())
        self.source.unlink()
        self.source.symlink_to(external)
        with self.assertRaisesRegex(ValueError, 'input bytes drift'):
            run.verify_inspection([self.row], self.inputs, self.packages)


class MeasurementErrorTests(unittest.TestCase):
    def row(self):
        return {'class': 'redos', 'entry': 'p_1', 'gt_status': 'available',
                'source': {'file': 'x.js', 'start_line': 1, 'end_line': 4, 'features': [],
                           'data_parameters': [{'line': 1, 'names': ['input'], 'start_byte': 11, 'end_byte': 16,
                                                'rest': False, 'destructure': False, 'default': False}]},
                'sink': {'file': 'x.js', 'line': 3, 'value_occurrences': [{'name': 'input', 'start_byte': 31}]}}

    def test_decisive_failures_and_bad_schema_are_prism_error(self):
        for kind in ('invocation', 'schema'):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory(prefix='secbench-measure-') as directory:
                def fake(binary, args, raw, label, timeout):
                    raw.mkdir(parents=True, exist_ok=True)
                    return (None if label == 'witness' and kind == 'invocation' else {}), {
                        'error': 'invocation failure' if label == 'witness' and kind == 'invocation' else None}
                with patch.object(run, 'invoke', side_effect=fake):
                    row = run.measure(self.row(), Path('prism'), Path(directory), Path(directory), 1)
                self.assertEqual(row['outcome'], 'prism_error')
                self.assertIn('invocation failure' if kind == 'invocation' else 'output contract', row['first_break']['reason'])

    def test_comparison_only_error_does_not_replace_a_reached_witness(self):
        witness = {'reasoning': {'per_sink': [{'sink': {'Variable': {'file': 'x.js', 'line': 3, 'path': 'input', 'start_byte': 31}},
                                              'sources': [{'source': {'Variable': {'file': 'x.js', 'line': 1, 'path': 'input', 'access': 'def', 'start_byte': 11}},
                                                           'reachability': 'Reached'}]}]}}
        with tempfile.TemporaryDirectory(prefix='secbench-measure-') as directory:
            def fake(binary, args, raw, label, timeout):
                raw.mkdir(parents=True, exist_ok=True)
                return (witness if label == 'witness' else None if label == 'taint' else {}), {
                    'error': 'comparison failure' if label == 'taint' else None}
            with patch.object(run, 'invoke', side_effect=fake):
                row = run.measure(self.row(), Path('prism'), Path(directory), Path(directory), 1)
            self.assertEqual(row['outcome'], 'traced')
            self.assertEqual(row['invocations']['taint']['error'], 'comparison failure')
            seed = json.loads((Path(directory) / 'raw/redos/p_1/seed.json').read_bytes())
            self.assertEqual(seed['files'][0]['diff_lines'], [1])

    def test_shared_nav_cache_is_removed_after_failure_or_unavailable_gt(self):
        for available in (True,False):
            with self.subTest(available=available), tempfile.TemporaryDirectory(prefix='secbench-cache-') as directory:
                output = Path(directory)
                cache = output/'cache/redos/p_1'
                row = self.row()
                if not available:
                    row.update(gt_status='gt_unavailable',gt_reason='ambiguous_payload_parameter')
                calls = []
                def fake(binary,args,raw,label,timeout):
                    calls.append(args)
                    raw.mkdir(parents=True,exist_ok=True)
                    cache.mkdir(parents=True,exist_ok=True)
                    failed = label=='witness'
                    return (None if failed else {}), {'error':'invocation failure' if failed else None}
                with patch.object(run,'invoke',side_effect=fake):
                    result = run.measure(row,Path('prism'),output,output,1)
                nav_calls = [args for args in calls if args[0]=='nav']
                self.assertEqual(len(nav_calls),6 if available else 1)
                for args in nav_calls:
                    self.assertIn('--cache-dir',args)
                    self.assertEqual(args[args.index('--cache-dir')+1],str(cache))
                    self.assertNotIn('--no-cache',args)
                self.assertFalse(cache.exists())
                self.assertEqual(result['outcome'],'prism_error' if available else 'gt_unavailable')


if __name__ == '__main__':
    unittest.main()
