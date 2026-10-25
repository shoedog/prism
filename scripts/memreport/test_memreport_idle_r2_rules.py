"""Bounded round-2 population, applicability, retry and relief regressions."""
import copy
import unittest
from unittest.mock import patch
import memreport_idle_readout as r
from test_memreport_idle_fold_rules import batch
from test_memreport_idle_readout import row, M


class RoundTwoRules(unittest.TestCase):
    def test_cap_all_cycles(self):
        rows = batch()
        extra = dict(row(rep=9),cycle=2,score_memory=False,footprint_bytes=129*M)
        out = r.readout(rows+[extra])
        self.assertEqual(out['memory_gate']['A']['modes']['stdio']['cap']['status'],'FAIL')
        self.assertTrue(out['matched']['stdio/A/py']['pass'])
        self.assertTrue(out['rows'][-1]['cap_eligible'])
        self.assertFalse(out['rows'][-1]['m_eligible'])
        partial = [x for x in rows if not (x['arm']=='A' and x['mode']=='stdio' and x['corpus']=='ruff')]
        coverage = dict(extra,corpus='ruff',footprint_bytes=128*M)
        cap = r.readout(partial+[coverage])['memory_gate']['A']['modes']['stdio']['cap']
        self.assertEqual(cap['status'],'PASS')
        extra.update(status='EXCLUDED',reasons=['sequence_gap'])
        out = r.readout(rows+[extra])
        self.assertTrue(out['memory_gate']['A']['pass'])
        self.assertFalse(out['rows'][-1]['cap_eligible'])
        self.assertEqual(len(out['memory_gate']['A']['modes']['stdio']['cap']['excluded']),1)

    def test_relief_applicability(self):
        for mode in ('stdio','shared'):
            for platform,idle,raw,effective,mismatch in (
                    ('darwin',0,True,False,False), ('linux',600,True,False,False),
                    ('linux',0,True,False,False), ('darwin',600,True,False,True),
                    ('darwin',600,False,True,True), ('darwin',600,True,True,False)):
                with self.subTest(mode=mode,platform=platform,idle=idle,raw=raw,effective=effective):
                    x = dict(row(),mode=mode,platform=platform,requested_idle_secs=idle,
                             effective_idle_secs=idle,requested_relief=raw,effective_relief=effective)
                    with patch('sys.platform','scorer-host'):
                        admitted = r.admit(x)
                    self.assertEqual('settings_mismatch' in admitted['reasons'],mismatch)
                    self.assertEqual(admitted['requested_relief'],raw)
                    self.assertEqual(admitted['platform'],platform)
                    self.assertIs(admitted.get('applicable_requested_relief'),raw and idle>0 and platform=='darwin')
                    self.assertEqual(r.service_admitted(x),not mismatch)
        missing = dict(row(),platform=None,requested_relief=True)
        self.assertEqual(r.admit(missing)['status'],'EXCLUDED')

    def test_failed_retry_keeps_unique_admitted_pair(self):
        for arm in ('resident','A'):
            for paired in (False,True):
                for last in (False,True):
                    with self.subTest(arm=arm,paired=paired,last=last):
                        failed = dict(row(arm),outcome='abandoned',status='EXCLUDED',
                                      reasons=['collection_failed'],resident_pairing=row('resident')['resident_pairing'] if paired else None)
                        rows = batch()
                        rows = rows+[failed] if last else [failed]+rows
                        try:
                            out = r.readout(rows)
                        except Exception as error:
                            self.fail('kept failed retry crashed: '+repr(error))
                        self.assertFalse(out['duplicate_pairs'])
                        self.assertTrue(out['memory_gate']['A']['pass'])
                        self.assertEqual(out['outcomes']['abandoned'],1)
                        value = next(x for x in out['rows'] if x['outcome']=='abandoned')
                        self.assertFalse(value['m_eligible'])
                        self.assertFalse(value['cap_eligible'])
        for arm in ('resident','A'):
            rows = batch()+[copy.deepcopy(row(arm))]
            self.assertTrue(r.readout(rows)['duplicate_pairs'])
        from memreport_idle_rows import rows_for_run
        failed = rows_for_run(row('resident'),[],[],[],['launch_failed'])[0]
        self.assertIs(failed.get('score_memory'),False)
        self.assertIs(failed.get('m_eligible'),False)
        self.assertIs(failed.get('cap_eligible'),False)
        self.assertIsInstance(failed['samples'],dict)

    def test_relief_remaining_comparisons(self):
        good = row()
        missing = dict(row(rep=2),heap_delta_bytes=None)
        excluded = dict(row(rep=3),reasons=['sequence_gap'],heap_delta_bytes=99*M)
        out = r.readout([good,missing,excluded])
        self.assertEqual(out['relief']['A']['status'],'PASS')
        self.assertEqual(out['relief']['stdio/A']['counts'],{'PASS':1,'EXCLUDED':2})
        self.assertEqual(len(out['relief']['A']['comparisons']),3)
        self.assertIn('missing_heap',out['relief']['A']['comparisons'][1]['reasons'])
        bad = dict(row(mode='shared'),heap_delta_bytes=M+1)
        out = r.readout([good,missing,excluded,bad])
        self.assertEqual(out['relief']['A']['status'],'FAIL')
        self.assertEqual(out['relief']['stdio/A']['status'],'PASS')
        self.assertEqual(out['relief']['shared/A']['status'],'FAIL')
        out = r.readout([missing,excluded])
        self.assertEqual(out['relief']['A']['status'],'UNKNOWN')
