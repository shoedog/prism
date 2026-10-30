"""Launcher/scheduler admission tests; no MCP sampling traffic."""
import copy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock
import memreport_idle_driver as d
import memreport_idle_schedule as schedule
from test_memreport_idle import fixture, instance
from memreport_idle import parse_line


def manifest():
    return dict(binary='/binary', repo='/repo', cache_dir='/cache', mode='stdio', arm='A',
                requested_idle_secs=600, requested_relief=False, first_call_wait_secs=20, platform='darwin',
                first_request={'name':'nav_callees','arguments':{'seed':{'kind':'symbol','name':'f'}}},
                commit='pin', features=['mcp'], corpus='py', corpus_pin='pin', workload_hash='hash',
                cache_class='warm', pair_id='1', cycles=2)

class Driver(unittest.TestCase):
    def test_lazy_command_and_manifest(self):
        m=manifest()
        command=d.command(m)
        self.assertIn('--index-idle-secs',command)
        self.assertNotIn('--eager',command)
        self.assertNotIn('--warm-at-startup',command)
        m.update(mode='shared',runtime_dir='/private/runtime',requested_relief=True,arm='A+C')
        self.assertIn('--shared',d.command(m))
        self.assertIn('--index-idle-pressure-relief',d.command(m))
        with self.assertRaisesRegex(ValueError,'runtime'): d.command(dict(m,runtime_dir=None))
        with self.assertRaisesRegex(ValueError,'600'): d.validate_manifest(dict(m,requested_idle_secs=2))
        self.assertIn('smoke_only',d.validate_manifest(dict(m,smoke_only=True,requested_idle_secs=2)))
        self.assertIn('missing_prerequisite', d.validate_manifest(m))
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'out'; d.external_output(p)
            self.assertTrue(p.exists())
        with self.assertRaisesRegex(ValueError,'checkout'):
            d.external_output(Path(__file__).resolve().parent/'raw')

    def test_production_schedule_and_slow_boundary(self):
        clocks=Mock()
        clocks.valid_read.side_effect=lambda:dict(epoch_ns=clock[0],mono_ns=clock[0],
            absolute_ns=clock[0],continuous_ns=clock[0],bracket_ns=0)
        clock=[100]
        sampler=Mock()
        sampler.read.return_value=dict(pid=1,os_start=10,footprint_bytes=15,rss_bytes=20,reason=None)
        engine=schedule.Engine(sampler,{1:10},lambda:{1:10},clocks)
        event=dict(event='disposal_end',epoch_ns=100,mono_ns=0,attempt=1,generation=0,outcome='complete')
        engine.observe([event]); engine.tick()
        self.assertEqual(engine.samples,[])
        clock[0]=5000000100; engine.tick()
        self.assertEqual(len(engine.samples),1)
        self.assertEqual(engine.samples[0]['due_epoch_ns'],5000000100)
        self.assertEqual(engine.samples[0]['footprint_bytes'],15)
        clock[0]=30250000101; engine.tick()
        self.assertIsNone(engine.samples[-1]['footprint_bytes'])
        self.assertIn('late_boundary',engine.samples[-1]['reasons'])
        self.assertEqual(sampler.read.call_count,1) # no catch-up reads

    def test_real_rows_sidecar_binding(self):
        import memreport_idle_readout as r
        events=[parse_line(line)[0] for line in fixture('stdio').splitlines(keepends=True)]
        events=[e for e in events if e]
        pub=next(e for e in events if e['event']=='publication')
        m=manifest(); m.update(binary_sha256='hash',instance=list(instance('stdio').values()),
                               os_start=10,client_count=1)
        journal=[dict(kind='demand',demand_id='x',epoch_ns=pub['epoch_ns']-1,request_hash='q'),
                 dict(kind='attempt',demand_id='x',epoch_ns=pub['epoch_ns'],attempt=pub['attempt']),
                 dict(kind='response',demand_id='x',epoch_ns=pub['epoch_ns']+1,outcome='served')]
        row=r.rows_for_run(m,events,journal,[],[])[0]
        self.assertTrue(row['ending_unload'])
        self.assertEqual(row['sidecar'],'not_attempted')
        changed=copy.deepcopy(events)
        for e in changed:
            if e['event']=='unload': e['generation']+=1
        row=r.rows_for_run(m,changed,journal,[],[])[0]
        self.assertIsNone(row['sidecar'])
        self.assertFalse(row['ending_unload'])
        self.assertIn('missing_sample_idle30',row['reasons'])

    def test_crossed_boundary(self):
        engine=schedule.Engine(Mock(),{},lambda:{})
        engine.samples=[dict(boundary='idle30',anchor_epoch_ns=10,end_epoch_ns=30,
            attempt=1,footprint_bytes=1,rss_bytes=2,reasons=[])]
        engine.finish([dict(event='build_start',epoch_ns=20,attempt=2,outcome='complete')])
        self.assertIsNone(engine.samples[0]['footprint_bytes'])
        self.assertIn('crossed_lifecycle',engine.samples[0]['reasons'])
        engine.samples[0].update(boundary='loaded',footprint_bytes=1,reasons=[])
        engine.finish([dict(event='refresh',epoch_ns=20,attempt=1,outcome='complete')])
        self.assertIsNone(engine.samples[0]['footprint_bytes'])

    def test_bad_clock_outside_boundary_is_reread(self):
        clocks=Mock()
        value=dict(epoch_ns=0,mono_ns=0,absolute_ns=0,continuous_ns=0,bracket_ns=0)
        clocks.valid_read.return_value=value
        engine=schedule.Engine(Mock(),{1:10},lambda:{1:10},clocks)
        engine.enqueue('idle5',5000000000,dict(epoch_ns=0,attempt=1,generation=0))
        clocks.valid_read.return_value=None
        engine.tick()
        self.assertEqual(engine.samples,[])
        self.assertEqual(engine.reasons,[])

    def test_pre_index_unknown_and_negative_delta(self):
        import memreport_idle_readout as r
        events=[parse_line(x)[0] for x in fixture('stdio').splitlines(keepends=True)]
        events=[e for e in events if e]
        pub=next(e for e in events if e['event']=='publication')
        start=next(e['epoch_ns'] for e in events if e['event']=='build_start')
        meta=manifest(); meta.update(binary_sha256='hash',requested_idle_secs=2,
            instance=list(instance('stdio').values()),os_start=10,client_count=1)
        journal=[dict(kind='demand',demand_id='d',epoch_ns=start-1,request_hash='q'),
                 dict(kind='attempt',demand_id='d',epoch_ns=start,attempt=1),
                 dict(kind='response',demand_id='d',epoch_ns=pub['epoch_ns']+1,outcome='served')]
        samples=[dict(boundary=b,attempt=1,generation=pub['generation'],
            footprint_bytes=90,rss_bytes=91,start_epoch_ns=start-3,end_epoch_ns=start-2,reasons=[])
            for b in ('pre_index','loaded','idle5','idle30')]
        samples[0]['footprint_bytes']=100
        value=r.rows_for_run(meta,events,journal,samples,[])[0]
        self.assertEqual(r.admit(value)['status'],'ADMITTED')
        self.assertEqual(value['pre_index_deltas']['footprint_bytes'],-10)
        samples[0].update(end_epoch_ns=None,reasons=['missing_clock'])
        try:
            value=r.rows_for_run(meta,events,journal,samples,[])[0]
        except TypeError as error:
            self.fail('missing clock must preserve a row: '+str(error))
        self.assertIn('missing_clock',value['samples']['pre_index']['reasons'])
        samples[0].update(end_epoch_ns=start+1,reasons=[])
        value=r.rows_for_run(meta,events,journal,samples,[])[0]
        self.assertIn('pre_index_after_build',value['samples']['pre_index']['reasons'])
        self.assertIn('pre_index_after_call',value['samples']['pre_index']['reasons'])

    def test_observed_settings_and_binary_pin(self):
        status=dict(index_idle_timeout_secs=0,index_idle_pressure_relief=False,admitted=2)
        fields,reasons=d.observed_settings(status)
        self.assertEqual(fields.get('effective_idle_secs'),0)
        self.assertEqual(fields['effective_relief'],False)
        self.assertIn('unknown_shim_membership',reasons)
        status['admitted']=1
        self.assertEqual(d.observed_settings(status)[1],[])
        with tempfile.TemporaryDirectory() as tmp:
            binary=Path(tmp)/'binary'; binary.write_bytes(b'actual')
            meta=dict(binary=str(binary),binary_sha256='wrong')
            observed,reasons=d.bind_binary(meta)
            self.assertIn('binary_pin_mismatch',reasons)
            self.assertEqual(observed['declared_binary_sha256'],'wrong')
            self.assertNotEqual(observed['binary_sha256'],'wrong')
        m=manifest(); m['startup']='background'
        self.assertIn('--warm-at-startup',d.command(m))
        for field,value in [('cycles',0),('max_polls',0),('startup','eager')]:
            with self.assertRaises(ValueError): d.validate_manifest(dict(m,**{field:value}))

    def test_prelaunch_failure_keeps_row(self):
        m=manifest(); m.update(binary='/does-not-exist-i0c-binary',cycles=1)
        with tempfile.TemporaryDirectory() as tmp:
            try:
                result=d.collect(m,Path(tmp)/'out')
            except OSError as error:
                self.fail('failed launch must preserve rows: '+str(error))
            self.assertEqual(len(result['rows']),1)
            self.assertEqual(result['rows'][0]['outcome'],'abandoned')
            self.assertIsNone(result['rows'][0]['footprint_bytes'])
            self.assertIn('does-not-exist-i0c-binary',result['rows'][0]['failure_text'])

    def test_runtime_outside_checkout(self):
        m=manifest(); m.update(mode='shared',runtime_dir=str(Path(__file__).resolve().parent/'raw'))
        with self.assertRaisesRegex(ValueError,'checkout'): d.validate_manifest(m)

    def test_shutdown_checks_os_identity(self):
        from memreport_idle_process import Process
        process=object.__new__(Process)
        process.child=Mock(); process.owned_daemon=(Path('/not-a-real-socket'),dict(identity='key',pid=2,start_time=3))
        process.members={2:99}; process.sampler=Mock()
        process.sampler.read.return_value={'reason':'rusage_ESRCH'}
        process.control=Mock(); process.reader=Mock(); process.stdout=Mock(); process.stderr=Mock()
        process.close()
        self.assertIs(getattr(process,'cleanup_confirmed',None),True)
        process.sampler.read.return_value={'reason':None}
        with unittest.mock.patch('memreport_idle_process.time.monotonic',side_effect=[0,6]):
            process.close()
        self.assertFalse(process.cleanup_confirmed)

if __name__=='__main__': unittest.main()
