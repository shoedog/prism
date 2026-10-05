import copy
import gzip
import json
from pathlib import Path
import tempfile
import unittest
from .replay import retained, reclassify
from .run import apply_adjudications, canonical, digest


class ReplayTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='secbench-replay-')
        self.addCleanup(self.tmp.cleanup)
        data = canonical({'error': {'SymbolNotFound': {'seed': 'x.js:1'}}})
        raw = Path(self.tmp.name) / 'error.json.gz'
        raw.write_bytes(gzip.compress(data, mtime=0))
        record = {'raw': str(raw), 'stdout_sha256': digest(data),
                  'error': 'Prism invocation exited 3',
                  'argv': ['prism', '--source', 'x.js:1', '--sink', 'x.js:3']}
        self.observation = {'invocations': {k: copy.deepcopy(record) for k in ('witness', 'frontier', 'callees')},
                            'dfg_stats': {}, 'outcome': 'prism_error'}
        self.gt = {'class': 'redos', 'entry': 'p_1', 'gt_status': 'available',
                   'source': {'file': 'x.js', 'data_parameters': [{'line': 1}]},
                   'sink': {'file': 'x.js', 'line': 3}}

    def test_nonzero_stdout_error_is_read_and_preserved(self):
        row = reclassify(self.observation, self.gt)
        self.assertEqual(row['outcome'], 'prism_error')
        self.assertEqual(row['first_break']['output_errors']['witness'], {'SymbolNotFound': {'seed': 'x.js:1'}})

    def test_error_artifact_drift_is_rejected(self):
        self.observation['invocations']['witness']['stdout_sha256'] = 'bad'
        with self.assertRaisesRegex(ValueError, 'raw observation drift'):
            retained(self.observation, 'witness')

    def test_new_source_and_sink_require_new_invocations(self):
        for target in ('source', 'sink'):
            changed = copy.deepcopy(self.gt)
            if target == 'source':
                changed['source']['data_parameters'][0]['line'] = 2
            else:
                changed['sink']['line'] = 4
            with self.subTest(target=target), self.assertRaisesRegex(ValueError, 'new seed/sink'):
                reclassify(self.observation, changed)

    def test_unavailable_gt_is_not_an_error_or_miss(self):
        self.gt.update(gt_status='gt_unavailable', gt_reason='ambiguous_payload_parameter')
        self.assertEqual(reclassify(self.observation, self.gt)['outcome'], 'gt_unavailable')

    def test_adjudication_agreement_keeps_unresolved_separate(self):
        rows = [{'class': 'redos', 'entry': 'p_1', 'outcome': 'partial', 'first_break': {'category': 'unresolved'}}]
        label = {'class': 'redos', 'entry': 'p_1', 'expected_outcome': 'partial', 'category': 'unresolved'}
        agreement = apply_adjudications(rows, [label])
        self.assertEqual(agreement['exact_category_agreements'], 1)
        self.assertEqual(agreement['resolved_categories'], 0)
        self.assertEqual(rows[0]['attribution_status'], 'adjudicated_unresolved')
        label['expected_outcome'] = 'traced'
        with self.assertRaisesRegex(ValueError, 'outcome drift'):
            apply_adjudications(rows, [label])


if __name__ == '__main__':
    unittest.main()
