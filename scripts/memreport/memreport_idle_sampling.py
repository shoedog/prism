"""External macOS kernel-ledger sampling; no target calls or RSS substitution."""
import ctypes as C
import errno
import sys
import time


class RusageInfoV4(C.Structure):
    # Complete bsd/sys/resource.h layout, including the unused v4 trailer.
    _fields_ = [('ri_uuid', C.c_uint8 * 16)] + [(name, C.c_uint64) for name in (
        'ri_user_time ri_system_time ri_pkg_idle_wkups ri_interrupt_wkups ri_pageins '
        'ri_wired_size ri_resident_size ri_phys_footprint ri_proc_start_abstime '
        'ri_proc_exit_abstime ri_child_user_time ri_child_system_time '
        'ri_child_pkg_idle_wkups ri_child_interrupt_wkups ri_child_pageins '
        'ri_child_elapsed_abstime ri_diskio_bytesread ri_diskio_byteswritten '
        'ri_cpu_time_qos_default ri_cpu_time_qos_maintenance ri_cpu_time_qos_background '
        'ri_cpu_time_qos_utility ri_cpu_time_qos_legacy ri_cpu_time_qos_user_initiated '
        'ri_cpu_time_qos_user_interactive ri_billed_system_time ri_serviced_system_time '
        'ri_logical_writes ri_lifetime_max_phys_footprint ri_instructions ri_cycles '
        'ri_billed_energy ri_serviced_energy ri_interval_max_phys_footprint ri_runnable_time'
    ).split()]


class Sampler:
    def __init__(self, api=None):
        self.error = None
        if api is None:
            try:
                if sys.platform != 'darwin':
                    raise OSError('unsupported_platform')
                api = C.CDLL('/usr/lib/libproc.dylib', use_errno=True).proc_pid_rusage
                api.argtypes = [C.c_int, C.c_int, C.c_void_p]
                api.restype = C.c_int
            except (OSError, AttributeError) as error:
                self.error = str(error)
        self.api = api

    def read(self, pid, expected_start=None):
        value = dict(pid=pid, os_start=None, footprint_bytes=None, rss_bytes=None, reason=self.error)
        if self.error:
            return value
        buffer = RusageInfoV4()
        C.set_errno(0)
        result = self.api(pid, 4, C.byref(buffer))
        error = C.get_errno()
        if result != 0:
            value['reason'] = 'rusage_' + errno.errorcode.get(error, str(error))
            return value
        if buffer.ri_proc_exit_abstime != 0:
            value['reason'] = 'exited_process'
            return value
        if buffer.ri_proc_start_abstime == 0:
            value['reason'] = 'missing_os_start'
            return value
        if expected_start is not None and buffer.ri_proc_start_abstime != expected_start:
            value['reason'] = 'pid_start_changed'
            return value
        value.update(os_start=buffer.ri_proc_start_abstime, footprint_bytes=buffer.ri_phys_footprint,
                     rss_bytes=buffer.ri_resident_size, reason=None)
        return value


def sweep(sampler, members, inventory, observe=None):
    """members = launch-bound PID -> OS start; inventory rechecks survivors after the read."""
    reasons, readings = [], []
    start = time.time_ns()
    for pid, os_start in members.items():
        clock_before = observe() if observe else None
        before = time.time_ns()
        value = sampler.read(pid, os_start)
        clock_after = observe() if observe else None
        value.update(start_epoch_ns=before, end_epoch_ns=time.time_ns())
        if observe:
            value.update(clock_before=clock_before, clock_after=clock_after)
            if clock_before is None or clock_after is None:
                reasons.append('missing_clock')
        readings.append(value)
        if os_start is None:
            reasons.append('missing_os_start')
        if value['reason']:
            reasons.append(value['reason'])
    if not members:
        reasons.append('missing_members')
    if dict(inventory()) != dict(members):
        reasons.append('inventory_changed')
    return dict(start_epoch_ns=start, end_epoch_ns=time.time_ns(), members=readings, reasons=reasons,
                footprint_bytes=None if reasons else sum(v['footprint_bytes'] for v in readings),
                rss_bytes=None if reasons else sum(v['rss_bytes'] for v in readings))


def deadline(event, delay):
    return event['epoch_ns'] + int(delay * 1000000000)


def resident_deadline(resident_arrival, idle_arrival, disposal_end, delay):
    if disposal_end < idle_arrival:
        raise ValueError('disposal precedes last response')
    return resident_arrival + disposal_end - idle_arrival + int(delay * 1000000000)


def boundary_reason(due, start, end, crossed=False):
    if crossed:
        return 'crossed_lifecycle'
    if start < due:
        return 'early_boundary'
    if end < start or end - due > 250000000:
        return 'late_boundary'
    return None


def cross_check(pid, out, sampler=None, runner=None):
    """One separate, unscored window; compare CLI auxiliary per-PID ledger only."""
    import json
    import subprocess
    from pathlib import Path
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    sampler = sampler if sampler is not None else Sampler()
    runner = runner if runner is not None else subprocess.run
    before_ns = time.time_ns()
    before = sampler.read(pid)
    cli_start = time.time_ns()
    command = ['/usr/bin/footprint','-f','bytes','--noCategories','-j',str(out/'footprint.json'),str(pid)]
    done = runner(command, capture_output=True, timeout=30)
    cli_end = time.time_ns()
    (out/'cli.log').write_bytes(done.stdout+done.stderr)
    after = sampler.read(pid, before.get('os_start'))
    try:
        data = json.loads((out/'footprint.json').read_text())
    except (OSError,ValueError):
        data = {}
    entry = next((x for x in data.get('processes',[]) if x.get('pid')==pid), {})
    auxiliary = entry.get('auxiliary',{}).get('phys_footprint')
    external = before.get('footprint_bytes')
    difference = auxiliary-external if auxiliary is not None and external is not None else None
    result = dict(scored=False, command=command, cli_returncode=done.returncode,
                  before_epoch_ns=before_ns, cli_start_epoch_ns=cli_start, cli_end_epoch_ns=cli_end,
                  after_epoch_ns=time.time_ns(), before=before, after=after,
                  per_pid=[dict(pid=pid,cli_auxiliary_bytes=auxiliary,difference_bytes=difference)],
                  disposition='investigate disagreement; never correct series by assumed overhead')
    (out/'cross-check.json').write_text(json.dumps(result,indent=2)+'\n')
    return result
