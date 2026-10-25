import copy
import unittest
import memreport_idle_readout as r

M = 1024 * 1024

def row(arm='A', corpus='py', mode='stdio', rep=1, footprint=100*M):
    return dict(binary_sha256='abc', commit='ae49d937', features=['mcp'], corpus=corpus,
                corpus_pin='pin', workload_hash='work', cache_class='warm', mode=mode, arm=arm,
                requested_idle_secs=600, effective_idle_secs=600, requested_relief=False,
                effective_relief=False, first_call_wait_secs=20, instance=[str(1000+rep)+'-'+arm,'100','key'],
                os_start=99, client_count=1, cycle=rep, attempt=1, generation=2,
                status='ADMITTED', reasons=[], outcome='served', boundary='idle30',
                footprint_bytes=footprint, rss_bytes=footprint, heap_delta_bytes=0,
                pair_id=str(rep), demand_readiness_ns=100000000, build_time_ns=50000000,
                first_served_ns=200000000, cpg='hit', sidecar='hit', first_request='q',
                edges_unused=False, ending_unload=True, first_call_ns=0, samples={}, events=[],
                last_response_epoch_ns=10, disposal_end_epoch_ns=20, failure_text=None,
                journal=[], polling=[], prewarm=[])

class Readout(unittest.TestCase):
    def test_idle_readout_denominators_and_outcomes(self):
        base, candidate = row(footprint=1000*M), row(footprint=100*M)
        self.assertEqual(r.saving(base,candidate)['fraction'], .9)
        candidate['heap_delta_bytes'] = 999999999
        self.assertEqual(r.saving(base,candidate)['bytes'], 900*M)
        candidate['effective_idle_secs'] = 0
        self.assertIn('settings_mismatch', r.admit(candidate)['reasons'])
        candidate=row(); candidate['effective_relief']=True
        self.assertEqual(r.admit(candidate)['status'], 'EXCLUDED')
        missing=row(); missing['footprint_bytes']=None
        self.assertEqual(r.admit(missing)['status'], 'UNKNOWN')
        failed=row(); failed.update(outcome='failed', failure_text='boom')
        abandoned=row(); abandoned.update(outcome='abandoned')
        out=r.readout([base,failed,abandoned,missing])
        self.assertEqual(len(out['rows']),4)
        self.assertEqual(out['outcomes']['failed'],1)
        self.assertEqual(out['outcomes']['abandoned'],1)
        different=row(); different['binary_sha256']='other'
        with self.assertRaisesRegex(ValueError,'mixed'):
            r.readout([base,different])
        rows=[]
        for mode in ('stdio','shared'):
            for corpus in ('py','e','p','ruff'):
                for rep in range(3):
                    rows.extend([row('resident',corpus,mode,rep,1000*M),row('A',corpus,mode,rep,128*M)])
        result=r.readout(rows)
        self.assertTrue(result['memory_gate']) # 128MiB is inclusive; 15% idle is report-only
        rows[-1]['footprint_bytes']=128*M+1
        self.assertFalse(r.readout(rows)['memory_gate'])
        # Two passing corpora establish M, but cap still requires all four and both modes.
        for x in rows:
            if x['corpus'] in ('p','ruff') and x['arm']=='resident': x['footprint_bytes']=129*M
        rows[-1]['footprint_bytes']=128*M
        self.assertTrue(r.readout(rows)['memory_gate'])
        for x in rows:
            if x['corpus']=='py' and x['arm']=='resident': x['footprint_bytes']=129*M
        self.assertFalse(r.readout(rows)['memory_gate'])
        self.assertFalse(r.readout(rows[:6])['memory_gate'])
        self.assertEqual(r.readout(rows)['units'], {'memory':'bytes','duration':'ns'})

    def test_outcomes_and_matched_boundaries(self):
        reload, ref = row(), row()
        self.assertTrue(r.exact_hit(reload, ref))
        for field, value in [('cpg','partial_hit'),('sidecar','miss'),('first_request','other'),
                             ('ending_unload',False),('cache_class','cold'),('binary_sha256','other')]:
            bad=dict(reload); bad[field]=value
            self.assertFalse(r.exact_hit(bad,ref), field)
        reload['sidecar']=ref['sidecar']='not_attempted'
        self.assertFalse(r.exact_hit(reload,ref))
        reload['edges_unused']=ref['edges_unused']=True
        self.assertTrue(r.exact_hit(reload,ref))
        base, candidate=row('resident'),row()
        candidate['boundary']='loaded'
        with self.assertRaisesRegex(ValueError,'boundary'): r.saving(base,candidate)
        candidate=row(); candidate['pair_id']='other'
        with self.assertRaisesRegex(ValueError,'pair'): r.saving(base,candidate)
        self.assertEqual(r.latency([row()], [row()])['status'], 'PASS')
        slow=row(); slow['first_served_ns']=400000000
        self.assertEqual(r.latency([slow],[row()])['status'],'FAIL')
        slow['demand_readiness_ns']=3000000001
        self.assertFalse(r.latency([slow],[row()])['readiness_target'])

    def test_rows_keep_failed_and_abandoned_demands(self):
        meta=row()
        journal=[{'demand_id':'d','kind':'demand','epoch_ns':10,'request_hash':'q'},
                 {'demand_id':'d','kind':'poll','epoch_ns':12,'attempt':1},
                 {'demand_id':'d','kind':'abandoned','epoch_ns':20,'reason':'EOF'}]
        rows=r.rows_for_run(meta,[],journal,[],[])
        self.assertEqual(len(rows),1)
        self.assertEqual(rows[0]['outcome'],'abandoned')
        self.assertEqual(rows[0]['polling'][0]['epoch_ns'],12)
        self.assertIsNone(rows[0]['demand_readiness_ns'])
        self.assertIn('EOF', rows[0]['failure_text'])

    def test_independent_runs_and_excluded_status(self):
        pairs=[(row('resident',rep=i,footprint=500*M),row(rep=i,footprint=100*M)) for i in (1,2,3)]
        for a,b in pairs:
            a['instance']=pairs[0][0]['instance']; b['instance']=pairs[0][1]['instance']
        self.assertFalse(r.matched_m(pairs)['pass'])
        bad=row(); bad['status']='EXCLUDED'
        self.assertEqual(r.admit(bad)['status'],'EXCLUDED')
        pairs=[(row('resident',rep=i,footprint=500*M),row(rep=i,footprint=100*M)) for i in (1,2,3)]
        pairs[0][1]['heap_delta_bytes']=None
        self.assertFalse(r.matched_m(pairs)['pass'])

    def test_served_requires_publication_and_keeps_later_abandonment(self):
        meta=row()
        journal=[dict(kind='demand',demand_id='d',epoch_ns=10,request_hash='q'),
                 dict(kind='response',demand_id='d',epoch_ns=11,outcome='served')]
        value=r.rows_for_run(meta,[],journal,[],[])[0]
        self.assertEqual(value['outcome'],'unpublished')
        self.assertEqual(value['client_outcome'],'served')
        journal.append(dict(kind='abandoned',demand_id='d',epoch_ns=12,reason='timeout before unload'))
        value=r.rows_for_run(meta,[],journal,[],[])[0]
        self.assertEqual(value['outcome'],'abandoned')
        self.assertEqual(value['client_outcome'],'served')
        self.assertIn('timeout before unload',value['failure_text'])

    def test_ranges_and_relief(self):
        pairs=[(row('resident',rep=i,footprint=b*M),row(rep=i,footprint=c*M))
               for i,b,c in [(1,500,100),(2,600,90),(3,700,110)]]
        self.assertTrue(r.matched_m(pairs)['pass'])
        pairs[0][1]['footprint_bytes']=500*M
        self.assertFalse(r.matched_m(pairs)['pass'])
        for heap, ok in [(None,False),(M,True),(-M,True),(M+1,False)]:
            v=row(); v['heap_delta_bytes']=heap
            self.assertEqual(r.relief_ok(v),ok)

if __name__ == '__main__': unittest.main()
