"""Quiet-residency footprint trends and optional passive rebuild-log observations.

Owner defaults, 2026-10-16. A baseline belongs to one (pid, key), after build and warm-up.
Work may warm caches: only growth within a measured quiet stretch can fail the rule.
"""
import os
from pathlib import Path
import re
import tempfile

from soak_window import night_overlaps

MIB = 1024 * 1024
FOOTPRINT_BASELINE_QUIET_SECS = 10 * 60
FOOTPRINT_QUIET_STRETCH_SECS = 60 * 60
FOOTPRINT_REBUILD_CPU_SECS = 5.0
FOOTPRINT_GROWTH_MIN_BYTES = 200 * MIB
FOOTPRINT_GROWTH_BASELINE_FRACTION = 0.25
# These are the daemon's actual navigation-cache rebuild messages (cache.rs).
REBUILD_LINE = re.compile(rb'^nav cache: (?:CPG cache miss|partial CPG cache hit) in .*, rebuilding$')


class RebuildLogs:
    """Read appended log bytes without connecting to a daemon. The first sighting establishes
    a cursor: old, untimestamped lines cannot be assigned to the current sample interval.
    Missing/unreadable logs are optional; CPU bursts remain the rebuild detector.
    """

    def __init__(self, runtime_dir=None):
        base = runtime_dir or os.environ.get('PRISM_MCP_RUNTIME_DIR') or os.environ.get('XDG_RUNTIME_DIR')
        self.root = Path(base or tempfile.gettempdir()) / 'prism-mcpd'
        self.cursors = {}

    def observe(self, processes):
        active = set()
        for process in processes:
            if process['role'] != 'daemon' or not process.get('key'):
                continue
            identity = (process['pid'], process['key'])
            active.add(identity)
            process['rebuild_observed'] = False
            path = self.root / (process['key'][:16] + '.log')
            try:
                with path.open('rb') as source:
                    stat = os.fstat(source.fileno())
                    previous = self.cursors.get(identity)
                    offset, pending = stat.st_size, b''
                    if previous:
                        offset, pending = (previous[1:] if previous[0] == stat.st_ino
                                           and previous[1] <= stat.st_size else (0, b''))
                    source.seek(offset)
                    data = pending + source.read()
                    lines = data.split(b'\n')
                    self.cursors[identity] = (stat.st_ino, source.tell(), lines.pop())
                    process['rebuild_observed'] = any(REBUILD_LINE.match(line) for line in lines)
            except OSError:
                self.cursors.pop(identity, None)
        self.cursors = {identity: cursor for identity, cursor in self.cursors.items() if identity in active}


class Trend:
    """One daemon identity; historical breaches and measurement holes survive baseline resets."""

    def __init__(self, process, t):
        self.report = dict(pid=process['pid'], key=process.get('key'), first_seen=t, last_seen=t,
                           baseline_bytes=None, baseline_t=None, baselines=[], max_bytes=None,
                           last_bytes=None, growth_bytes_per_hour=None, largest_quiet_growth_bytes=0,
                           largest_quiet_secs=0, quiet_nights=[])
        self.previous = None
        self.first_reading = None
        self.built_seen = False
        self.quiet_t = self.quiet_bytes = None
        self.failed = self.unknown = False

    def observe(self, process, t, index, ps_ok, built_min, usage_cpu, max_interval):
        r = self.report
        fp, cpu = process.get('footprint_bytes'), process.get('cpu_secs')
        r['last_seen'], r['last_bytes'] = t, fp
        if fp is not None:
            r['max_bytes'] = max(r['max_bytes'] or 0, fp)
            if self.first_reading is None:
                self.first_reading = (t, fp)
        if self.first_reading and fp is not None and t > self.first_reading[0]:
            # Observed residency, including work and rebuilds, rather than only quiet time.
            r['growth_bytes_per_hour'] = (fp - self.first_reading[1]) * 3600 / (t - self.first_reading[0])
        else:
            r['growth_bytes_per_hour'] = None

        before = self.previous
        valid = (before is not None and before[0] == index - 1 and ps_ok
                 and 0 < t - before[1] <= max_interval and cpu is not None
                 and before[2] is not None and cpu >= before[2])
        delta = round(cpu - before[2], 6) if valid else None
        rebuild = process.get('rebuild_observed', False) or (valid and delta >= FOOTPRINT_REBUILD_CPU_SECS)
        if rebuild:
            r['baseline_bytes'] = r['baseline_t'] = None
            # Night credit belongs to this baseline epoch; breaches and holes stay sticky.
            r['quiet_nights'] = []
            self.quiet_t = self.quiet_bytes = None
            self.built_seen = False
        self.built_seen |= fp is not None and fp >= built_min
        if fp is None and process.get('footprint_status') != 'gone':
            self.unknown = True
        if before is not None and not valid and self.built_seen:
            self.unknown = True

        quiet = valid and delta < usage_cpu and fp is not None and before[3] is not None
        if not quiet or rebuild:
            self.quiet_t = self.quiet_bytes = None
        if self.built_seen and fp is not None and cpu is not None:
            if self.quiet_t is None:
                self.quiet_t, self.quiet_bytes = t, fp
            if r['baseline_bytes'] is None and t - self.quiet_t >= FOOTPRINT_BASELINE_QUIET_SECS:
                r['baseline_t'], r['baseline_bytes'] = t, fp
                r['baselines'].append({'t': t, 'bytes': fp})
                # Build and warm-up growth are excluded from the first stretch.
                self.quiet_t, self.quiet_bytes = t, fp
            if r['baseline_bytes'] is not None:
                growth = max(0, fp - self.quiet_bytes)
                duration = t - self.quiet_t
                if (growth, duration) > (r['largest_quiet_growth_bytes'], r['largest_quiet_secs']):
                    r['largest_quiet_growth_bytes'], r['largest_quiet_secs'] = growth, duration
                threshold = max(FOOTPRINT_GROWTH_MIN_BYTES,
                                FOOTPRINT_GROWTH_BASELINE_FRACTION * r['baseline_bytes'])
                if duration >= FOOTPRINT_QUIET_STRETCH_SECS and growth > threshold:
                    self.failed = True
                if duration >= FOOTPRINT_QUIET_STRETCH_SECS:
                    nights = {night for night, seconds in night_overlaps(self.quiet_t, t)
                              if seconds >= FOOTPRINT_QUIET_STRETCH_SECS}
                    r['quiet_nights'] = sorted(set(r['quiet_nights']) | nights)
        self.previous = (index, t, cpu, fp)


def footprint_growth(samples, built_min, usage_cpu, max_interval, min_quiet_nights=0):
    """Trends for every daemon, plus FAIL > UNKNOWN > PASS. With min_quiet_nights, each daemon
    needs that many quiet nights after baseline; otherwise no baseline contributes nothing.
    A missing process table cannot establish absence. Holes break quiet continuity rather than
    turning an interval without usage credit into evidence that the daemon was idle.
    """
    trends = {}
    for index, sample in enumerate(samples):
        for process in sample['processes']:
            if process['role'] != 'daemon':
                continue
            identity = (process['pid'], process.get('key'))
            if identity not in trends:
                trends[identity] = Trend(process, sample['t'])
            trends[identity].observe(process, sample['t'], index, sample.get('ps_ok', True),
                                     built_min, usage_cpu, max_interval)
    failed = any(trend.failed for trend in trends.values())
    unknown = any(trend.unknown for trend in trends.values()) or any(not s.get('ps_ok', True) for s in samples)
    if min_quiet_nights:
        unknown |= not trends or any(t.report['baseline_bytes'] is None
                                    or len(t.report['quiet_nights']) < min_quiet_nights
                                    for t in trends.values())
    return {'trends': [trend.report for trend in trends.values()],
            'verdict': 'FAIL' if failed else 'UNKNOWN' if unknown else 'PASS'}
