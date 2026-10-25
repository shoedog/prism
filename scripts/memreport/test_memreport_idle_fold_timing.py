"""Original demand clocks remain separate from attempt-local build clocks."""
import copy
import unittest
from memreport_idle import parse_line
from memreport_idle_rows import rows_for_run
from test_memreport_idle import fixture
from test_memreport_idle_readout import row
from idle_test_support import Clock, NS


class FoldTiming(unittest.TestCase):
    def test_readiness_build_poll_retry_and_abandonment(self):
        clock = Clock()
        template = [parse_line(x)[0] for x in fixture('stdio').splitlines(keepends=True)]
        template = {e['event']:e for e in template if e}
        for prewarm, retry in ((False,False),(True,False),(True,True)):
            clock.ns = 0
            journal = [dict(kind='demand',demand_id='d',epoch_ns=clock.ns,
                            prewarm=prewarm,request_hash='q')]
            clock.sleep(1)
            journal.append(dict(kind='response',demand_id='d',epoch_ns=clock.ns,outcome='warming'))
            if retry:
                journal.append(dict(kind='attempt',demand_id='d',epoch_ns=clock.ns,attempt=1))
            clock.sleep(3)
            journal += [dict(kind='poll',demand_id='d',epoch_ns=clock.ns),
                        dict(kind='attempt',demand_id='d',epoch_ns=clock.ns,attempt=2)]
            events = []
            for name,t in [('disposal_start',0),('disposal_end',4)]:
                e = copy.deepcopy(template[name]); e.update(epoch_ns=t*NS,attempt=99)
                events.append(e)
            self.assertGreater(events[1]['epoch_ns']-events[0]['epoch_ns'],3*NS)
            for name,t in [('build_start',4),('publication',5)]:
                e = copy.deepcopy(template[name]); e.update(epoch_ns=t*NS,attempt=2)
                events.append(e)
            clock.sleep(1.1)
            journal.append(dict(kind='response',demand_id='d',epoch_ns=clock.ns,outcome='served'))
            value = rows_for_run(dict(row(),instance=None),events,journal,[],[])[0]
            self.assertEqual(value['demand_readiness_ns'],5*NS)
            self.assertEqual(value['build_time_ns'],NS)
            self.assertEqual(value['first_served_ns'],5100000000)
            self.assertEqual(value['attempt'],2)
            self.assertEqual(bool(value['prewarm']),prewarm)
            self.assertGreater(value['demand_readiness_ns'],3*NS)
            self.assertLess(value['build_time_ns'],3*NS)
        journal[-1]['epoch_ns'] = 9100000000
        journal.append(dict(kind='poll',demand_id='d',epoch_ns=9*NS))
        delayed = rows_for_run(dict(row(),instance=None),events,journal,[],[])[0]
        self.assertEqual(delayed['demand_readiness_ns'],5*NS)
        self.assertEqual(delayed['first_served_ns'],9100000000)
        journal.append(dict(kind='abandoned',demand_id='d',epoch_ns=10*NS,reason='no publication'))
        value = rows_for_run(row(),[],journal,[],[])[0]
        self.assertEqual(value['outcome'],'abandoned')
        self.assertIsNone(value['demand_readiness_ns'])
        self.assertIsNone(value['build_time_ns'])
