"""Owner's Phase 3 floors and local-night accounting (2026-10-16).

A local night is 22:00–06:00, labelled by its starting date. A night needs an
hour of sampled intervals; a gap above the usage cadence cap earns no time.
The reporting host's timezone applies, including its daylight-saving rules.
"""
from datetime import datetime, time, timedelta
from functools import lru_cache
import math
import os
import time as clock

PHASE3_WINDOW_SECS = 72 * 60 * 60
PHASE3_MIN_NIGHTS = 2
PHASE3_MIN_USAGE_DAYS = 2
NIGHT_OBSERVED_SECS = 60 * 60


@lru_cache(maxsize=32768)
def local_date(epoch, timezone):
    return datetime.fromtimestamp(epoch).date()


@lru_cache(maxsize=256)
def night_bounds(date, timezone):
    return (datetime.combine(date, time(22)).timestamp(),
            datetime.combine(date + timedelta(days=1), time(6)).timestamp())


def night_overlaps(start, end):
    """Seconds of overlap with each local night, without assuming 24-hour local days."""
    timezone = (os.environ.get('TZ'), clock.tzname)
    date = local_date(start, timezone) - timedelta(days=1)
    last = local_date(end, timezone)
    while date <= last:
        left, right = night_bounds(date, timezone)
        overlap = min(end, right) - max(start, left)
        if overlap > 0:
            yield date.isoformat(), overlap
        date += timedelta(days=1)


def observed_nights(samples, max_interval):
    seconds = {}
    for before, after in zip(samples, samples[1:]):
        if (before.get('ps_ok', True) and after.get('ps_ok', True)
                and 0 < after['t'] - before['t'] <= max_interval):
            for night, duration in night_overlaps(before['t'], after['t']):
                seconds[night] = seconds.get(night, 0) + duration
    return sorted(night for night, duration in seconds.items() if duration >= NIGHT_OBSERVED_SECS)


def fix_at(text):
    """argparse value: RULE=nonnegative finite epoch seconds."""
    from argparse import ArgumentTypeError
    from soak_judge import RULES
    try:
        rule, stamp = text.split('=', 1)
        epoch = float(stamp)
        if rule not in RULES or not math.isfinite(epoch) or epoch < 0:
            raise ValueError
    except ValueError:
        raise ArgumentTypeError('fix-at needs a known RULE=finite nonnegative EPOCH') from None
    return rule, epoch
