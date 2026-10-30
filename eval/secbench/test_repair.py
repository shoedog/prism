import copy
import gzip
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from .blind import kappa, freeze
from . import conversions
from . import probes
from .report import build
from .conversions import observe, pattern_for, relocate, patch_bytes, same_control
from .run import canonical, digest
from . import test_run


class RepairTests(unittest.TestCase):
    def test_kappa_exact_disagreement_and_invalid_population(self):
        self.assertEqual(kappa(['A', 'B'], ['A', 'B'])['kappa'], 1)
        self.assertEqual(kappa(['A', 'B'], ['B', 'A'])['kappa'], -1)
        self.assertIsNone(kappa(['A'], ['A'])['kappa'])
        with self.assertRaises(ValueError):
            kappa([], [])

    def test_successful_rewrite_drops_stale_error_mechanism(self):
        fixture = test_run.AccountingTests();fixture.setUp()
        row = {'source': fixture.source, 'sink': fixture.sink,
               'error_mechanism': {'mechanism': 'callback_argument_parameter_registration'}}
        row['source']['data_parameters'][0].update(bare_references=0, member_references=1)
        witness = {'reasoning': {'per_sink': []}}
        with tempfile.TemporaryDirectory() as temporary:
            def fake(binary, args, raw, label, timeout):
                return witness if label == 'witness' else {}, {'error': None}
            with patch.object(conversions, 'invoke', side_effect=fake):
                result = observe(row, Path('prism'), Path(temporary), Path(temporary), 1)
        self.assertNotIn('error_mechanism', result)
        self.assertEqual(pattern_for(result), 'member')

    def test_identical_error_outcome_does_not_hide_a_different_control_mechanism(self):
        baseline = {'outcome':'prism_error', 'first_break':{'mechanism':'member_only_parameter_no_def'}}
        control = {'outcome':'prism_error', 'first_break':{'mechanism':'timeout'}}
        self.assertFalse(same_control(baseline, control))
        self.assertTrue(same_control(baseline, copy.deepcopy(baseline)))

    def test_admission_relocation_preserves_original_and_endpoint_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b'const helper=require("./dist/helper");\nfunction f(x){return x}\n'
            (root / 'index.js').write_bytes(source)
            (root / 'dist').mkdir();(root / 'dist/helper.js').write_bytes(b'module.exports={};\n')
            parameter = source.index(b'x)')
            row = {'source': {'file': 'index.js', 'start_byte': source.index(b'function'), 'end_byte': len(source)-1,
                              'data_parameters': [{'start_byte': parameter, 'end_byte': parameter+1}]},
                   'sink': {'file': 'dist/helper.js', 'value_occurrences': []}}
            result = relocate(root, row)
            new = (root / 'index.js').read_bytes();p = result['source']['data_parameters'][0]
            self.assertEqual(new[p['start_byte']:p['end_byte']], b'x')
            self.assertEqual((root / '_secbench_dist/helper.js').read_bytes(), (root / 'dist/helper.js').read_bytes())
            self.assertEqual(result['sink']['file'], '_secbench_dist/helper.js')

    def test_blind_packets_never_include_derived_labels_and_freeze_deterministically(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary);samples = root / 'sample.jsonl';reference = root / 'reference.jsonl'
            rows = [{'class': 'redos', 'entry': f'p_{i}', 'outcome': 'gt_unavailable', 'gt_status': 'gt_unavailable',
                     'first_break': {'category': 'SECRET_LABEL'}, 'invocations': {}} for i in range(12)]
            freeze(rows, root, samples, reference)
            first = samples.read_bytes();freeze(rows, root, samples, reference)
            self.assertEqual(first, samples.read_bytes())
            self.assertNotIn(b'SECRET_LABEL', first)
            self.assertEqual(len(first.splitlines()), 8)

    def test_admission_refuses_observable_paths_and_unrelated_strings(self):
        for source in (b'const path=__dirname;\n', b'module.exports="./dist/result";\n'):
            with self.subTest(source=source), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary);(root/'index.js').write_bytes(source)
                (root/'dist').mkdir();(root/'dist/a.js').write_bytes(b'module.exports={};\n')
                row = {'source': {'file':'index.js','data_parameters':[]},
                       'sink': {'file':'dist/a.js','value_occurrences':[]}}
                with self.assertRaisesRegex(ValueError, 'semantics'):
                    relocate(root, row)
                self.assertEqual((root/'index.js').read_bytes(),source)

    def test_admission_keeps_terminal_line_identity_after_metadata_formatting(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);source=b'function f(x){return x}\n'
            (root/'dist').mkdir();(root/'dist/a.js').write_bytes(source)
            (root/'package.json').write_bytes(b'{"main":"dist/a.js"}\n')
            row={'source':{'file':'dist/a.js','start_byte':0,'end_byte':len(source)-1,
                           'start_line':1,'data_parameters':[{'start_byte':11,'end_byte':12,'line':1}]},
                 'sink':{'file':'dist/a.js','line':1,'value_occurrences':[]}}
            result=relocate(root,row)
            self.assertEqual(result['source']['start_line'],1)
            self.assertEqual((root/'_secbench_dist/a.js').read_bytes(),source)

    def test_existing_fixture_requires_byte_verification_before_query(self):
        with tempfile.TemporaryDirectory() as temporary:
            case=Path(temporary);inspection=case/'inspection.jsonl'
            row={'gt_status':'available','class':'command-injection','entry':'p_1'}
            inspection.write_bytes(canonical(row));before=inspection.read_bytes()
            with patch.object(probes,'verify_inspection') as verify, patch.object(probes,'observe',return_value={'outcome':'traced'}) as query:
                self.assertEqual(probes.probe_fixture(case,Path('prism'),case/'out')['outcome'],'traced')
                verify.assert_called_once_with([row],case/'inputs',case/'packages')
                query.assert_called_once()
            with patch.object(probes,'verify_inspection',side_effect=ValueError('input bytes drift')), patch.object(probes,'observe') as query:
                with self.assertRaisesRegex(ValueError,'bytes drift'):
                    probes.probe_fixture(case,Path('prism'),case/'out')
                query.assert_not_called()
            self.assertEqual(inspection.read_bytes(),before)

    def test_report_retains_errors_exclusions_joint_overlap_and_clone_sensitivity(self):
        rows = [
            {'class':'path-traversal', 'entry':'a', 'outcome':'reached_function_only',
             'first_break':{'category':'C-member'},
             'source':{'resolution_mode':'http_registration', 'normalized_handler_sha256':'clone'}},
            {'class':'path-traversal', 'entry':'b', 'outcome':'prism_error',
             'first_break':{'category':'C-member'}, 'error_mechanism':{'mechanism':'member_only_parameter_no_def'},
             'source':{'resolution_mode':'http_registration', 'normalized_handler_sha256':'clone'}},
            {'class':'command-injection', 'entry':'c', 'outcome':'partial',
             'first_break':{'category':'B-rest-spread'}, 'source':{'resolution_mode':'syntactic_export'}},
            {'class':'redos', 'entry':'d', 'outcome':'reached_function_only',
             'first_break':{'category':'C-member'}, 'source':{'resolution_mode':'typescript_checker'}},
            {'class':'redos', 'entry':'e', 'outcome':'gt_unavailable'},
            {'class':'redos', 'entry':'f', 'outcome':'acquisition_excluded'}]
        baseline = [
            {'class':'path-traversal', 'entry':entry, 'outcome':'gt_unavailable',
             'gt_reason':'no_direct_package_api_call_with_data'} for entry in ('a','b')]
        baseline += [
            {'class':'command-injection', 'entry':'c', 'outcome':'partial'},
            {'class':'redos', 'entry':'d', 'outcome':'gt_unavailable', 'gt_reason':'unresolved_export_or_api_chain'},
            {'class':'redos', 'entry':'e', 'outcome':'gt_unavailable', 'gt_reason':'unresolved_export_or_api_chain'},
            {'class':'redos', 'entry':'f', 'outcome':'acquisition_excluded'}]
        conversions = [
            {'class':'path-traversal', 'entry':entry, 'conversion':True, 'status':'demonstrated',
             'workstreams':[3,6], 'single_workstream':None} for entry in ('a','b')]
        conversions += [
            {'class':'command-injection', 'entry':'c', 'conversion':True, 'status':'demonstrated',
             'workstreams':[2], 'single_workstream':2},
            {'class':'redos', 'entry':'d', 'conversion':True, 'status':'demonstrated',
             'workstreams':[3], 'single_workstream':3}]
        result = build(rows, baseline, conversions)
        self.assertEqual(sum(result['outcomes'].values()),6)
        self.assertEqual(result['outcomes']['acquisition_excluded'],1)
        self.assertEqual(result['error_mechanisms'],{'member_only_parameter_no_def':1})
        self.assertEqual((result['http_raw'],result['http_deduplicated']),(2,1))
        self.assertEqual(result['workstreams']['3']['nonerror_first_break'],2)
        self.assertEqual(result['workstreams']['3']['including_errors'],3)
        self.assertEqual(result['workstreams']['2']['single_weighted'],3)
        self.assertEqual(result['workstreams']['3']['single_weighted'],1)
        for ws in ('3','6'):
            self.assertEqual(result['workstreams'][ws]['joint_conversions'],2)
            self.assertEqual(result['workstreams'][ws]['joint_deduplicated'],1)
            self.assertEqual(result['workstreams'][ws]['joint_weighted'],4)
        gain = result['coverage']['unresolved_export_or_api_chain']
        self.assertEqual((gain['entries'],gain['after_eligible'],gain['typescript_checker']),(2,1,1))
        for category in ('B-destructure','B-default','B-plain-argument'):
            self.assertEqual(result['categories'][category],0)
        # An unresolved conversion receives no credit even after multiple barriers.
        conversions[0].update(conversion=False,status='remaining_unresolved_barrier')
        refused = build(rows,baseline,conversions)
        self.assertEqual(refused['workstreams']['3']['joint_conversions'],1)
        self.assertEqual(refused['workstreams']['3']['multi_barrier_entries'],2)
        rows[1]['source']['normalized_handler_sha256']='different'
        self.assertEqual(build(rows,baseline,[])['http_deduplicated'],2)
        self.assertEqual(build(rows,baseline,[])['workstreams']['3']['single_conversions'],0)


if __name__ == '__main__':
    unittest.main()
