"""O1: anonymous source credit depends on byte-bound taint/frontier results."""
import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from . import conversions, run


class AnonymousLocationTests(unittest.TestCase):
    def fixture(self, root):
        text = '// π\nf(function(input) {\n  use(input);\n});\n'
        data = text.encode()
        (root / 'x.js').write_bytes(data)
        start = data.index(b'function')
        parameter = data.index(b'input')
        use = data.index(b'input', parameter + 1)
        row = {'class': 'command-injection', 'entry': 'fixture', 'gt_status': 'available',
               'source': {'file': 'x.js', 'name': '<anonymous>', 'start_line': 2, 'end_line': 4,
                          'start_byte': start, 'end_byte': len(data) - 3, 'features': [],
                          'callback_expression_statement': True,
                          'data_parameters': [{'line': 2, 'text': 'input', 'names': ['input'],
                                               'start_byte': parameter, 'end_byte': parameter + 5}]},
               'sink': {'file': 'x.js', 'line': 3, 'value_occurrences': [
                   {'name': 'input', 'start_byte': use, 'end_byte': use + 5}]}}
        source = {'Variable': {'file': 'x.js', 'line': 2, 'path': 'input', 'access': 'def', 'start_byte': parameter}}
        sink = {'Variable': {'file': 'x.js', 'line': 3, 'path': 'input', 'access': 'use', 'start_byte': use}}
        witness = {'reasoning': {'per_sink': [{'sink': sink, 'sources': [{'source': source, 'reachability': 'Reached'}]}]}}
        return row, witness

    def test_anonymous_observe_never_requires_callees(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            row, witness = self.fixture(root)
            commands = []
            def invoke(binary, args, raw, label, timeout):
                commands.append((label, args))
                if label == 'callees':
                    return None, {'error': 'LocationOutOfRange'}
                return witness if label == 'witness' else {'items': []}, {'error': None}
            with patch.object(conversions, 'invoke', side_effect=invoke):
                result = conversions.observe(row, root / 'prism', root, root / 'out', 10)
            self.assertEqual(result['outcome'], 'traced')
            self.assertEqual([label for label, _ in commands], ['witness', 'frontier'])
            for _, args in commands:
                self.assertEqual(args[args.index('--source') + 1], 'x.js:2')
            self.assertEqual(result['source_seed_identity']['start_byte'], row['source']['start_byte'])

    def test_anonymous_frontier_error_remains_decisive(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            row, witness = self.fixture(root)
            def invoke(binary, args, raw, label, timeout):
                if label == 'witness': return witness, {'error': None}
                return None, {'error': 'timeout', 'output_error': None}
            with patch.object(conversions, 'invoke', side_effect=invoke):
                result = conversions.observe(row, root / 'prism', root, root / 'out', 10)
            self.assertEqual(result['outcome'], 'prism_error')

    def test_named_source_still_uses_callees(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            row, witness = self.fixture(root)
            row['source']['name'] = 'handler'
            labels = []
            def invoke(binary, args, raw, label, timeout):
                labels.append(label)
                if label == 'callees': return None, {'error': 'LocationOutOfRange'}
                return witness if label == 'witness' else {'items': []}, {'error': None}
            with patch.object(conversions, 'invoke', side_effect=invoke):
                result = conversions.observe(row, root / 'prism', root, root / 'out', 10)
            self.assertIn('callees', labels)
            self.assertEqual(result['outcome'], 'prism_error')

    def test_invalid_byte_span_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            row, _ = self.fixture(root)
            row['source']['start_byte'] = 4  # middle of UTF-8 π
            with patch.object(conversions, 'invoke') as invoke:
                with self.assertRaises(ValueError):
                    conversions.observe(row, root / 'prism', root, root / 'out', 10)
                invoke.assert_not_called()
