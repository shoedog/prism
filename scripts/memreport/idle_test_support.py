"""Deterministic collector transport and clocks; never launches a server."""
import copy
import json
from contextlib import ExitStack, contextmanager
from pathlib import Path
from unittest.mock import Mock, patch

import memreport_idle_driver as d
from memreport_idle import parse_line
from memreport_idle_schedule import Engine
from test_memreport_idle import fixture
from test_memreport_idle_driver import manifest
from test_memreport_idle_readout import row

NS = 1000000000


class Clock:
    def __init__(self):
        self.ns = 0

    def read(self):
        return dict(epoch_ns=self.ns, mono_ns=self.ns, absolute_ns=self.ns,
                    continuous_ns=self.ns, bracket_ns=0)

    def sleep(self, seconds):
        self.ns += round(seconds * NS)


@contextmanager
def simulation(meta, fail=None, response_delay=.15, build_delay=0):
    meta.setdefault('prerequisites',dict.fromkeys(d.PREREQUISITES,True))
    meta['prerequisites']['free_bytes'] = 50*1024**3
    clock, scheduled, instances = Clock(), [], []
    templates = [parse_line(x)[0] for x in fixture('stdio').splitlines(keepends=True)]
    templates = {e['event']: e for e in templates if e}
    sampler = Mock()
    sampler.read.return_value = dict(pid=1, os_start=99, footprint_bytes=100,
                                    rss_bytes=110, reason=None)

    class Transport:
        def __init__(self, *args):
            self.child = Mock(returncode=0)
            self.child.poll.return_value = None
            self.members, self.status = {templates['publication']['pid']:99}, None
            self.cleanup_confirmed, self.cycle, self.queue = True, 0, []
            instances.append(self)

        def send(self, method, params=None, notify=False):
            if notify:
                return None
            if method == 'tools/call':
                self.cycle += 1
                if fail and self.cycle == 2:
                    raise fail
                for name, offset in [('build_start',0), ('publication',.1),
                                     ('unload',600.15), ('disposal_end',600.15)]:
                    if meta['arm'] == 'resident' and name in ('unload','disposal_end'):
                        continue
                    e = copy.deepcopy(templates[name])
                    e.update(epoch_ns=clock.ns+round((offset+build_delay)*NS), attempt=self.cycle,
                             generation=self.cycle, configured_idle_secs=meta['requested_idle_secs'])
                    scheduled.append(e)
                self.queue.append((self.cycle+1,clock.ns+round(response_delay*NS)))
                return self.cycle+1
            self.queue.append((1,clock.ns))
            return 1

        def responses(self):
            ready = [x for x in self.queue if x[1] <= clock.ns]
            self.queue = [x for x in self.queue if x not in ready]
            return [(dict(id=i,result={}), t) for i,t in ready]

        def discover(self):
            return []

        def inventory(self):
            return dict(self.members)

        def close(self):
            pass

    class Events:
        def __init__(self, streams):
            self.events, self.reasons, self.raw = [], [], []

        def poll(self):
            ready = [e for e in scheduled if e['epoch_ns'] <= clock.ns]
            self.events.extend(ready)
            for e in ready:
                scheduled.remove(e)

        def finish(self):
            pass

    engines = []
    def engine(*args):
        value = Engine(*args, clocks=Mock(valid_read=clock.read))
        engines.append(value)
        return value

    with ExitStack() as stack:
        for target, value in [('Process',Transport), ('Collector',Events), ('Sampler',lambda:sampler),
                              ('Engine',engine), ('bind_binary',lambda m:(dict(m,binary_sha256='abc'),[]))]:
            stack.enter_context(patch.object(d,target,value))
        stack.enter_context(patch.object(d.time,'time_ns',lambda:clock.ns))
        stack.enter_context(patch.object(d.time,'monotonic',lambda:clock.ns/NS))
        stack.enter_context(patch.object(d.time,'sleep',clock.sleep))
        yield clock, instances, engines


def resident(tmp, score_cycle=1):
    m = manifest()
    m.update(arm='resident', requested_idle_secs=0, cycles=1, commit='ae49d937',
             workload_hash='work', binary_sha256='abc', resident_pair=str(Path(tmp)/'pair.json'))
    candidates = []
    for cycle, delta in [(1,600), (2,610)]:
        x = row(rep=1)
        x.update(cycle=cycle, attempt=cycle+2, generation=cycle+4,
                 score_memory=cycle == score_cycle, last_response_epoch_ns=NS,
                 disposal_end_epoch_ns=(delta+1)*NS)
        candidates.append(x)
    Path(m['resident_pair']).write_text(json.dumps(dict(rows=candidates)))
    return m
