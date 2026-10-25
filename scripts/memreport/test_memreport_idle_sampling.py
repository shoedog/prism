import ctypes
import errno
import os
import sys
import json
import tempfile
from unittest.mock import Mock
import unittest
from pathlib import Path
import re
import memreport_idle_sampling as s
import memreport_idle_clock as clock

class Sampling(unittest.TestCase):
    def test_rusage_v4_abi_and_unknown(self):
        self.assertEqual(ctypes.sizeof(s.RusageInfoV4), 296)
        self.assertEqual(s.RusageInfoV4.ri_phys_footprint.offset, 72)
        self.assertEqual(s.RusageInfoV4.ri_proc_start_abstime.offset, 80)
        def failure(pid, flavor, pointer):
            pointer._obj.ri_phys_footprint = 900
            ctypes.set_errno(errno.EPERM)
            return -1
        sampler = s.Sampler(failure)
        value = sampler.read(41)
        self.assertIsNone(value['footprint_bytes'])
        self.assertIn('EPERM', value['reason'])
        def ok(pid, flavor, pointer):
            self.assertEqual(flavor, 4)
            pointer._obj.ri_phys_footprint = 17
            pointer._obj.ri_resident_size = 19
            pointer._obj.ri_proc_start_abstime = 123
            return 0
        sampler = s.Sampler(ok)
        self.assertEqual(sampler.read(41)['rss_bytes'], 19)
        self.assertEqual(sampler.read(41, 999)['reason'], 'pid_start_changed')
        def exited(pid, flavor, pointer):
            ok(pid, flavor, pointer); pointer._obj.ri_proc_exit_abstime = 3; return 0
        self.assertEqual(s.Sampler(exited).read(41)['reason'], 'exited_process')

    @unittest.skipUnless(sys.platform == 'darwin', 'live Apple ABI check requires macOS')
    def test_live_abi(self):
        header = Path('/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/sys/resource.h')
        text = header.read_text().split('struct rusage_info_v4 {',1)[1].split('};',1)[0]
        names = re.findall(r'uint(?:8|64)_t\s+(ri_\w+)', text)
        self.assertEqual([n for n,t in s.RusageInfoV4._fields_], names)
        self.assertGreater(s.Sampler().read(os.getpid())['footprint_bytes'], 0)

    def test_pid_start_and_survivor_sum(self):
        def api(pid, flavor, ptr):
            ptr._obj.ri_proc_start_abstime = pid * 10
            ptr._obj.ri_phys_footprint = pid * 100
            ptr._obj.ri_resident_size = pid * 200
            return 0
        members = {1:10, 2:20, 3:30}  # every shim and daemon, once
        sampler = s.Sampler(api)
        result = s.sweep(sampler, members, lambda:dict(members))
        self.assertEqual(result['footprint_bytes'], 600)
        self.assertEqual(result['rss_bytes'], 1200)
        self.assertEqual(len(result['members']), 3)
        bad = s.sweep(sampler, {1:10, 2:99, 3:30}, lambda:members)
        self.assertIsNone(bad['footprint_bytes'])
        self.assertIn('pid_start_changed', bad['reasons'])
        changed = s.sweep(sampler, members, lambda:{1:10})
        self.assertIn('inventory_changed', changed['reasons'])

    def test_event_deadlines_and_lateness(self):
        event = {'epoch_ns':1000000000,'mono_ns':500,'received_epoch_ns':9000000000}
        self.assertEqual(s.deadline(event, 5), 6000000000)
        self.assertIsNone(s.boundary_reason(6000000000, 6000000000, 6250000000))
        self.assertEqual(s.boundary_reason(6000000000, 6000000000, 6250000001), 'late_boundary')
        self.assertEqual(s.boundary_reason(6000000000, 5999999999, 6000000000), 'early_boundary')
        self.assertEqual(s.resident_deadline(900, 100, 400, 5), 5000001200)
        self.assertEqual(s.boundary_reason(0,0,1, crossed=True), 'crossed_lifecycle')

    def test_clock_sleep_exclusion(self):
        def obs(t, offset=0, sleep=0, bracket=0):
            return {'epoch_ns':t+offset,'mono_ns':t,'absolute_ns':t,
                    'continuous_ns':t+sleep,'bracket_ns':bracket}
        guard = clock.Continuity()
        self.assertIsNone(guard.add(obs(0)))
        self.assertIsNone(guard.add(obs(1000000000, 10050000))) # 10ms + 50ppm
        self.assertEqual(guard.add(obs(2000000000, 20100001)), 'clock_step')
        guard = clock.Continuity(); guard.add(obs(0))
        self.assertEqual(guard.add(obs(1, sleep=10000001)), 'sleep_or_unknown_continuity')
        guard = clock.Continuity(); guard.add(obs(0))
        self.assertEqual(guard.add(obs(1, bracket=1000001)), 'clock_bracket')
        self.assertIsNone(guard.add(obs(2))) # rejected bracket never becomes previous
        self.assertIsNone(guard.add(obs(3, offset=1)))
        self.assertEqual(guard.add(None), 'missing_clock')
        self.assertEqual(clock.Continuity().add(obs(0, sleep=0)), None)

    def test_injected_mach_and_bracket_retry(self):
        class Function:
            def __init__(self, fn): self.fn=fn
            def __call__(self,*args): return self.fn(*args)
        class Lib: pass
        lib=Lib()
        def scale(pointer): pointer._obj.numer=125; pointer._obj.denom=3; return 0
        lib.mach_timebase_info=Function(scale)
        lib.mach_absolute_time=Function(lambda:3)
        lib.mach_continuous_time=Function(lambda:6)
        ticks=iter([0,1000001,2000000,2000002])
        clocks=clock.Clocks(lib,epoch=lambda:99,mono=lambda:next(ticks))
        value=clocks.valid_read()
        self.assertEqual(value['absolute_ns'],125)
        self.assertEqual(value['continuous_ns'],250)
        self.assertEqual(value['bracket_ns'],2)
        lib.mach_timebase_info=Function(lambda ptr:-1)
        self.assertIsNone(clock.Clocks(lib).read())

    def test_cli_auxiliary_cross_check(self):
        sampler=Mock(); sampler.read.return_value=dict(os_start=10,footprint_bytes=100)
        def runner(args,**kwargs):
            path=Path(args[args.index('-j')+1])
            path.write_text(json.dumps({'processes':[{'pid':123,'auxiliary':{'phys_footprint':105}}],
                                       'phys_footprint':999999}))
            return Mock(returncode=0,stdout=b'',stderr=b'')
        with tempfile.TemporaryDirectory() as tmp:
            report=s.cross_check(123,Path(tmp),sampler,runner)
            self.assertEqual(len(report.get('per_pid',[])),1)
            self.assertEqual(report['per_pid'][0]['difference_bytes'],5)
            self.assertEqual(report['scored'],False)

if __name__ == '__main__': unittest.main()
