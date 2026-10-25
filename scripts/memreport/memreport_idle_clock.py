"""Bracketed cross-process epoch joins and consecutive valid clock continuity."""
import ctypes as C
import sys
import time


class Timebase(C.Structure):
    _fields_ = [('numer', C.c_uint32), ('denom', C.c_uint32)]


class Clocks:
    def __init__(self, library=None, epoch=time.time_ns, mono=time.monotonic_ns):
        self.epoch, self.mono, self.error = epoch, mono, None
        try:
            if library is None:
                if sys.platform != 'darwin':
                    raise OSError('unsupported_platform')
                library = C.CDLL('/usr/lib/libSystem.B.dylib', use_errno=True)
            self.absolute = library.mach_absolute_time
            self.continuous = library.mach_continuous_time
            self.absolute.argtypes = self.continuous.argtypes = []
            self.absolute.restype = self.continuous.restype = C.c_uint64
            info = library.mach_timebase_info
            info.argtypes, info.restype = [C.POINTER(Timebase)], C.c_int
            scale = Timebase()
            C.set_errno(0)
            if info(C.byref(scale)) != 0 or not scale.denom or not scale.numer:
                raise OSError('mach_timebase_failed errno=%s' % C.get_errno())
            self.numer, self.denom = scale.numer, scale.denom
        except (OSError, AttributeError) as error:
            self.error = str(error)

    def read(self):
        if self.error:
            return None
        before = self.mono()
        epoch = self.epoch()
        absolute = self.absolute() * self.numer // self.denom
        continuous = self.continuous() * self.numer // self.denom
        after = self.mono()
        return dict(epoch_ns=epoch, mono_ns=(before + after)//2, bracket_ns=after-before,
                    absolute_ns=absolute, continuous_ns=continuous)

    def valid_read(self, retries=3):
        # A slow bracket is re-read, never promoted to the continuity reference.
        for _ in range(retries):
            value = self.read()
            if value is None or 0 <= value['bracket_ns'] <= 1000000:
                return value
        return None


class Continuity:
    def __init__(self):
        self.previous, self.origin_offset = None, None
        self.reasons, self.cumulative_drift_ns = [], 0

    def add(self, value):
        if value is None:
            return 'missing_clock'
        if not 0 <= value['bracket_ns'] <= 1000000:
            return 'clock_bracket'
        reason = None
        offset = value['epoch_ns'] - value['mono_ns']
        if self.previous is not None:
            old = self.previous
            gap = value['mono_ns'] - old['mono_ns']
            differential = ((value['continuous_ns'] - value['absolute_ns']) -
                            (old['continuous_ns'] - old['absolute_ns']))
            if gap < 0 or abs(differential) > 10000000:
                reason = 'sleep_or_unknown_continuity'
            elif abs(offset - (old['epoch_ns'] - old['mono_ns'])) > 10000000 + gap * .00005:
                reason = 'clock_step'
        if self.origin_offset is None:
            self.origin_offset = offset
        self.cumulative_drift_ns = offset - self.origin_offset
        self.previous = value
        if reason:
            self.reasons.append(reason)
        return reason
