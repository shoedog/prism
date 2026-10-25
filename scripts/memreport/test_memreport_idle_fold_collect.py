"""Round-1 collector regressions, including the README's long resident recipe."""
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import memreport_idle_driver as d
from idle_test_support import NS, resident, simulation
from test_memreport_idle_driver import manifest


class FoldCollect(unittest.TestCase):
    def test_resident_recipe_budget(self):
        with tempfile.TemporaryDirectory() as tmp:
            m = resident(tmp)
            with simulation(m) as (clock, processes, engines):
                from memreport import main
                path = Path(tmp)/'resident.json'; path.write_text(json.dumps(m))
                main(['idle','collect','--manifest',str(path),'--out',str(Path(tmp)/'out')])
                result = json.loads((Path(tmp)/'out'/'run.json').read_text())
            self.assertGreaterEqual(clock.ns,630*NS)
            self.assertEqual(result['rows'][0]['outcome'],'served')
            self.assertIsNotNone(result['rows'][0]['footprint_bytes'])
            self.assertEqual(result['rows'][0]['samples']['idle30']['due_epoch_ns'],630150000000)
            m['timeout_secs'] = 180
            with simulation(m) as (_, processes, _):
                failed = d.collect(m,Path(tmp)/'short')
            self.assertEqual(processes,[])
            self.assertIn('timeout',failed['rows'][0]['failure_text'])

    def test_selected_candidate_and_local_tags(self):
        with tempfile.TemporaryDirectory() as tmp:
            m = resident(tmp,score_cycle=2)
            m['timeout_secs'] = 1000
            with simulation(m):
                result = d.collect(m,Path(tmp)/'out')
            value = result['rows'][0]
            self.assertIn('idle30',value['samples'])
            self.assertEqual(value['samples']['idle30']['due_epoch_ns'],640150000000)
            self.assertEqual(value['samples']['idle30']['attempt'],value['attempt'])
            self.assertEqual(value['resident_pairing']['cycle'],2)
            self.assertEqual(value['resident_pairing']['attempt'],4)
            self.assertEqual(value['resident_pairing']['delta_ns'],610*NS)

    def test_cycle_two_failure_is_local(self):
        m = manifest()
        with tempfile.TemporaryDirectory() as tmp, simulation(m,ValueError('cycle two failed')):
            result = d.collect(m,Path(tmp)/'out')
        first, second = result['rows']
        self.assertEqual(first['outcome'],'served')
        self.assertEqual(first['status'],'ADMITTED')
        self.assertNotIn('collection_failed',first['reasons'])
        self.assertEqual(second['outcome'],'abandoned')
        self.assertIn('cycle two failed',second['failure_text'])

    def test_all_named_exceptions_write_run(self):
        for target in ('Process','bind_binary','rows_for_run'):
            for error in (KeyError('first_request'), StopIteration('no scored row'),
                          TypeError('null disposal_end'), KeyError('STATUS pid')):
                with self.subTest(target=target,error=type(error).__name__), tempfile.TemporaryDirectory() as tmp:
                    m = manifest()
                    with simulation(m), patch.object(d,target,side_effect=error):
                        try:
                            result = d.collect(m,Path(tmp)/'out')
                        except Exception as escaped:
                            self.fail('exception lost run.json: '+repr(escaped))
                    saved = json.loads((Path(tmp)/'out'/'run.json').read_text())
                    self.assertEqual(len(saved['rows']),2 if target=='rows_for_run' else 1)
                    self.assertTrue(all(x['outcome']=='abandoned' for x in saved['rows']))
                    self.assertIn(type(error).__name__,result['rows'][0]['failure_text'])
                    rebuilt = d.rows_for_run(saved['manifest'],saved['events'],saved['journal'],saved['samples'],saved['reasons'])
                    self.assertTrue(all(isinstance(x['samples'],dict) for x in rebuilt))
                    from memreport import main
                    output = Path(tmp)/'readout.json'
                    main(['idle','readout','--run',str(Path(tmp)/'out'/'run.json'),'--out',str(output)])
                    rescored = json.loads(output.read_text())
                    self.assertEqual(len(rescored['rows']),len(saved['rows']))
                    self.assertTrue(all(type(error).__name__ in x['failure_text'] for x in rescored['rows']))

    def test_invalid_pairing_and_manifest_are_durable(self):
        for kind in ('missing','empty','ambiguous','null','negative','first_request'):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as tmp:
                m = resident(tmp)
                pair = Path(m['resident_pair'])
                data = json.loads(pair.read_text())
                if kind == 'missing': del m['resident_pair']
                if kind == 'empty': data['rows'] = []
                if kind == 'ambiguous': data['rows'].append(data['rows'][0])
                if kind == 'null': data['rows'][0]['disposal_end_epoch_ns'] = None
                if kind == 'negative': data['rows'][0]['disposal_end_epoch_ns'] = 0
                if kind == 'first_request': del m['first_request']
                pair.write_text(json.dumps(data))
                with simulation(m) as (_, processes, _):
                    try:
                        result = d.collect(m,Path(tmp)/'out')
                    except Exception as error:
                        self.fail('invalid input lost run: '+repr(error))
                self.assertEqual(processes,[])
                self.assertEqual(result['rows'][0]['outcome'],'abandoned')
                self.assertTrue(result['rows'][0]['failure_text'])

    def test_shared_loaded_completion_schedule(self):
        for delay in (.15,.4):
            m = manifest(); m.update(mode='shared',runtime_dir='/runtime',cycles=1)
            with tempfile.TemporaryDirectory() as tmp, simulation(m,response_delay=delay):
                result = d.collect(m,Path(tmp)/'out')
            sample = result['rows'][0]['samples']['loaded']
            self.assertEqual(sample['due_epoch_ns'],100000000)
            self.assertGreaterEqual(sample['start_epoch_ns'],round(delay*NS))
            self.assertEqual(sample['footprint_bytes'],100 if delay==.15 else None)
            self.assertEqual(result['rows'][0]['status'],'ADMITTED')

    def test_null_manifest_writes_failed_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            try:
                result = d.collect(None,Path(tmp)/'out')
            except Exception as error:
                self.fail('null manifest lost run: '+repr(error))
            self.assertEqual(result['rows'][0]['outcome'],'abandoned')
            self.assertTrue((Path(tmp)/'out'/'run.json').exists())
