"""Round-2 failure custody and exhausted warming schedule regressions."""
import json
import tempfile
import unittest
from pathlib import Path
import sys
from unittest.mock import patch
import memreport_idle_driver as d
from idle_test_support import resident, simulation
from test_memreport_idle_driver import manifest


class RoundTwoCollect(unittest.TestCase):
    def test_collector_records_platform_and_applicability(self):
        for platform,idle in (('darwin',0),('linux',600),('darwin',600)):
            with self.subTest(platform=platform,idle=idle), tempfile.TemporaryDirectory() as tmp:
                m = resident(tmp) if idle==0 else manifest()
                m.update(requested_relief=True,arm='resident' if idle==0 else 'A+C',platform='manifest-host')
                with simulation(m), patch.object(sys,'platform',platform):
                    result = d.collect(m,Path(tmp)/'out')
                applicable = platform=='darwin' and idle>0
                self.assertEqual(result['manifest']['platform'],platform)
                self.assertIs(result['manifest']['applicable_requested_relief'],applicable)
                for row in result['rows']:
                    self.assertEqual(row['platform'],platform)
                    self.assertTrue(row['requested_relief'])
                    self.assertIs(row['applicable_requested_relief'],applicable)
                    self.assertEqual('settings_mismatch' in row['reasons'],applicable)

    def test_two_demand_recovery_and_cli(self):
        from memreport import main
        for target in ('rows_for_run','engine_finish','visibility_write','clocks_write','run_write','cleanup_then_render'):
            with self.subTest(target=target), tempfile.TemporaryDirectory() as tmp:
                m = manifest()
                original_write = d.write
                tripped = []
                def write(path,value):
                    filename = {'visibility_write':'visibility.json','clocks_write':'clocks.json',
                                'run_write':'run.json'}.get(target)
                    if path.name==filename and not tripped:
                        tripped.append(True)
                        raise OSError('final write failed')
                    original_write(path,value)
                with simulation(m) as (_,_,engines):
                    from memreport_idle_schedule import Engine
                    fault = (patch.object(d,'rows_for_run',side_effect=TypeError('render failed')) if target in ('rows_for_run','cleanup_then_render')
                             else patch.object(Engine,'finish',side_effect=KeyError('finish failed')) if target=='engine_finish'
                             else patch.object(d,'write',side_effect=write))
                    with fault, patch.object(d.Process,'close',lambda p:setattr(p,'cleanup_confirmed',target!='cleanup_then_render')):
                        result = d.collect(m,Path(tmp)/'out')
                saved = json.loads((Path(tmp)/'out'/'run.json').read_text())
                self.assertEqual(len(saved['rows']),2)
                self.assertEqual(sum(x['kind']=='demand' for x in saved['journal']),2)
                self.assertGreater(len(saved['events']),0)
                self.assertEqual(saved['samples'],engines[0].samples)
                self.assertGreater(len(saved['samples']),0)
                self.assertEqual(result,saved)
                if target=='cleanup_then_render':
                    self.assertIn('shutdown_unconfirmed',saved['reasons'])
                for x in saved['rows']:
                    self.assertEqual(x['outcome'],'abandoned')
                    self.assertIn('failed',x['failure_text'])
                    self.assertIn('KeyError' if target=='engine_finish' else 'TypeError' if target in (
                        'rows_for_run','cleanup_then_render') else 'OSError',x['failure_text'])
                    self.assertFalse(x['score_memory'])
                    self.assertFalse(x['m_eligible'])
                    self.assertFalse(x['cap_eligible'])
                output = Path(tmp)/'readout.json'
                with patch('sys.platform','scorer-host'):
                    main(['idle','readout','--run',str(Path(tmp)/'out'/'run.json'),'--out',str(output)])
                rescored = json.loads(output.read_text())
                self.assertEqual(len(rescored['rows']),2)
                self.assertEqual(rescored['outcomes'],{'abandoned':2})
                self.assertTrue(all('failed' in x['failure_text'] for x in rescored['rows']))

    def test_warming_exhaustion_collects_next_cycle(self):
        for delay,build_delay in ((.05,0),(.15,0),(.05,.5)):
            with self.subTest(delay=delay,build_delay=build_delay), tempfile.TemporaryDirectory() as tmp:
                m = manifest(); m.update(max_polls=2)
                with simulation(m,response_delay=delay,build_delay=build_delay) as (clock,processes,engines):
                    original = d.Process.responses
                    def responses(process):
                        values = original(process)
                        for response,_ in values:
                            if response['id'] in (2,3):
                                response['result'] = {'_meta':{'prism/index_state':'warming'}}
                        return values
                    with patch.object(d.Process,'responses',responses):
                        result = d.collect(m,Path(tmp)/'out')
                self.assertGreaterEqual(sum(e['event']=='disposal_end' for e in result['events']),2)
                self.assertEqual(processes[0].cycle,3)
                self.assertEqual(len(result['rows']),2)
                first,second = result['rows']
                self.assertEqual(first['outcome'],'warming')
                self.assertEqual(len(first['polling']),1)
                self.assertIsNone(first['samples']['loaded']['footprint_bytes'])
                self.assertIn('unserved_query',first['samples']['loaded']['reasons'])
                self.assertEqual(second['outcome'],'served')
                self.assertEqual(second['status'],'ADMITTED')
                self.assertEqual(engines[0].pending,[])
                self.assertLess(clock.ns/1000000000,1300)
