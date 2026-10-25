"""Boundary and decision regressions; all numbers are synthetic bytes and ns."""
import copy
import unittest
from unittest.mock import Mock
import memreport_idle_readout as r
from memreport_idle_schedule import Engine
from idle_test_support import Clock, NS
from test_memreport_idle_readout import row, M


def batch():
    return [row(arm,corpus,mode,rep,size*M)
            for mode in ('stdio','shared') for corpus in ('py','e','p','ruff')
            for rep in (1,2,3) for arm,size in [('resident',1000),('A',100)]]


class FoldRules(unittest.TestCase):
    def test_report_boundaries_do_not_gate(self):
        from memreport_idle_rows import rows_for_run
        from memreport_idle import parse_line
        from test_memreport_idle import fixture
        events = [parse_line(x)[0] for x in fixture('stdio').splitlines(keepends=True)]
        events = [e for e in events if e]
        pub = next(e for e in events if e['event']=='publication')
        start = next(e['epoch_ns'] for e in events if e['event']=='build_start')
        journal = [dict(kind='demand',demand_id='d',epoch_ns=start-1,request_hash='q'),
                   dict(kind='attempt',demand_id='d',epoch_ns=start,attempt=pub['attempt']),
                   dict(kind='response',demand_id='d',epoch_ns=pub['epoch_ns']+1,outcome='served')]
        for boundary in ('pre_index','loaded','idle5','idle30'):
            samples = [dict(boundary=b,attempt=pub['attempt'],generation=pub['generation'],
                       footprint_bytes=100*M,rss_bytes=100*M,start_epoch_ns=start-3,
                       end_epoch_ns=start-2,reasons=['late_boundary'] if b==boundary else [])
                       for b in ('pre_index','loaded','idle5','idle30')]
            m = row(); m.update(requested_idle_secs=2, instance=None)
            value = rows_for_run(m,events,journal,samples,[])[0]
            self.assertIsNone(value['samples'][boundary]['footprint_bytes'])
            self.assertEqual(value['samples'][boundary]['status'],'UNKNOWN')
            if boundary == 'idle30':
                self.assertNotEqual(r.admit(value)['status'],'ADMITTED')
            else:
                self.assertEqual(r.admit(value)['status'],'ADMITTED')
                rows = batch()
                for x in rows:
                    if x['arm']=='A':
                        x.update(samples=value['samples'],reasons=value['reasons'])
                self.assertTrue(r.readout(rows)['memory_gate']['A']['pass'])
                self.assertEqual(r.latency([dict(value,cpg='hit',sidecar='hit')],
                                          [dict(value,cpg='hit',sidecar='hit')])['status'],'PASS')
        missing = rows_for_run(m,events,journal,[],[])[0]
        self.assertIn('missing_sample_loaded',missing['samples']['loaded']['reasons'])
        visible = rows_for_run(m,events,journal,samples,['sequence_gap'])[0]
        self.assertEqual(r.admit(visible)['status'],'EXCLUDED')

    def test_latency_independent_of_idle_memory(self):
        a,b = row(),row()
        a.update(footprint_bytes=None,status='UNKNOWN',reasons=['missing_sample_idle30'],
                 samples={'idle30':{'reasons':['missing_sample_idle30']}})
        self.assertEqual(r.latency([a],[b])['status'],'PASS')
        a['reasons'].append('sequence_gap')
        self.assertEqual(r.latency([a],[b])['status'],'UNKNOWN')

    def test_loaded_waits_for_served_query(self):
        for delay in (.1,.3):
            c = Clock(); sampler = Mock()
            sampler.read.return_value = dict(pid=1,os_start=99,footprint_bytes=99,rss_bytes=100,reason=None)
            e = Engine(sampler,{1:99},lambda:{1:99},Mock(valid_read=c.read))
            event = dict(event='publication',outcome='complete',epoch_ns=0,attempt=1,generation=2)
            e.observe([event]); e.tick()
            self.assertEqual(e.samples,[], 'publication alone cannot witness query allocations')
            c.sleep(delay)
            sampler.read.return_value['footprint_bytes'] = 200
            e.served({1},c.ns); e.tick()
            sample = e.samples[0]
            self.assertEqual(sample['due_epoch_ns'],0)
            self.assertEqual(sample['footprint_bytes'],200 if delay==.1 else None)
            self.assertEqual('late_boundary' in sample['reasons'],delay==.3)

    def test_failed_binary_keeps_denominator(self):
        failed = row(); failed.update(binary_sha256=None,outcome='abandoned',failure_text='launch failed')
        try:
            result = r.readout([row(),failed])
        except (TypeError,ValueError) as error:
            self.fail('failed hash is not a mixed binary: '+str(error))
        self.assertEqual(len(result['rows']),2)
        self.assertEqual(result['outcomes']['abandoned'],1)
        self.assertIn('binary_unavailable',result['rows'][1]['reasons'])
        with self.assertRaisesRegex(ValueError,'mixed'):
            r.readout([row()],binary_sha256='other')
        with self.assertRaisesRegex(ValueError,'mixed'):
            r.readout([row(),dict(row(),binary_sha256='other')])

    def test_relief_is_independent_and_per_arm(self):
        rows = batch()
        rows[1]['heap_delta_bytes'] = None
        rows[3]['heap_delta_bytes'] = M+1
        out = r.readout(rows)
        self.assertTrue(out['matched']['stdio/A/py']['pass'])
        relief = out['relief']['stdio/A']
        self.assertEqual(relief['counts']['UNKNOWN'],1)
        self.assertEqual(relief['counts']['FAIL'],1)
        self.assertEqual(relief['status'],'FAIL')
        rows[3]['heap_delta_bytes'] = -M
        self.assertEqual(r.readout(rows)['relief']['stdio/A']['status'],'UNKNOWN')
        rows[5]['reasons'] = ['sequence_gap']
        out = r.readout(rows)
        self.assertEqual(out['relief']['A']['counts']['EXCLUDED'],1)
        self.assertEqual(out['relief']['A']['status'],'UNKNOWN')

    def test_per_arm_cap_and_coverage(self):
        rows = batch()
        for x in list(rows):
            if x['arm']=='A':
                rows.append(dict(x,arm='A+C',pair_id='C'+x['pair_id'],footprint_bytes=129*M))
        out = r.readout(rows)
        self.assertIsInstance(out['memory_gate'],dict)
        self.assertTrue(out['memory_gate']['A']['pass'])
        self.assertFalse(out['memory_gate']['A+C']['pass'])
        self.assertEqual(out['memory_gate']['A+C']['modes']['stdio']['cap']['status'],'FAIL')
        rows.append(dict(row(rep=4),outcome='failed',reasons=['collection_failed']))
        out = r.readout(rows)
        self.assertIsInstance(out['memory_gate'],dict)
        self.assertTrue(out['memory_gate']['A']['pass'])
        self.assertEqual(out['outcomes']['failed'],1)
        partial = [x for x in rows if not (x['arm']=='A' and x['mode']=='shared' and x['corpus']=='ruff')]
        cap = r.readout(partial)['memory_gate']['A']['modes']['shared']['cap']
        self.assertEqual(cap['status'],'UNKNOWN')
        self.assertIn('ruff',cap['missing_cells'])

    def test_service_counts_and_proportions(self):
        a = dict(row(),kind='reload',cpg='hit',journal=[{'outcome':'warming'}],polling=[{}])
        b = dict(row(corpus='e',mode='shared'),kind='reload',cpg='miss',changed_snapshot=True,outcome='abandoned')
        c = dict(row(corpus='e',mode='shared'),kind='reload',cpg='miss',changed_snapshot=True,outcome='failed')
        out = r.readout([a,b,c])
        self.assertIn('service_groups',out)
        groups = out['service_groups']
        self.assertEqual(groups['pooled']['counts']['abandonment'],1)
        self.assertEqual(groups['pooled']['proportions']['failure'],1/3)
        self.assertIn('corpus/py',groups)
        self.assertEqual(groups['corpus/py']['counts']['warming'],1)
        self.assertEqual(groups['mode/shared']['counts']['failure'],1)
        self.assertEqual(groups['reload/exact_hit']['counts']['served_within_wait'],1)
        self.assertEqual(groups['reload/changed_snapshot']['counts']['abandonment'],1)
        self.assertEqual(groups['cell/e/shared/A/changed_snapshot']['total'],2)
        a['sidecar'] = None
        out = r.readout([a])
        self.assertNotIn('reload/exact_hit',out['service_groups'])
        self.assertEqual(out['service_groups']['reload/unknown']['total'],1)

    def test_pair_provenance_checked_when_scoring(self):
        a,b = row('resident'),row()
        self.assertEqual(r.saving(a,b)['bytes'],0)
        for field in ('arm','instance','cycle','attempt','generation','delta_ns'):
            changed = copy.deepcopy(a)
            changed['resident_pairing'][field] = 'other'
            with self.subTest(field=field), self.assertRaisesRegex(ValueError,'pairing'):
                r.saving(changed,b)

    def test_cap_uses_admitted_rows_without_M_sample_minimum(self):
        rows = [row(corpus=c,mode=m) for c in ('py','e','p','ruff') for m in ('stdio','shared')]
        out = r.readout(rows)
        self.assertIsInstance(out['memory_gate'],dict)
        for mode in ('stdio','shared'):
            self.assertEqual(out['memory_gate']['A']['modes'][mode]['cap']['status'],'PASS')
            self.assertEqual(out['memory_gate']['A']['modes'][mode]['M']['status'],'UNKNOWN')
