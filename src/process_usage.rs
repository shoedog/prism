//! Shared process resource counters. RSS is a lifetime high-water mark, never a phase delta.

/// Peak resident set size in bytes (macOS reports bytes; Linux reports KiB).
pub fn ru_maxrss_bytes() -> Option<u64> {
    usage().and_then(|u| {
        let raw = u64::try_from(u.ru_maxrss).ok()?;
        Some(if cfg!(target_os = "macos") {
            raw
        } else {
            raw.saturating_mul(1024)
        })
    })
}

#[cfg(unix)]
fn usage() -> Option<libc::rusage> {
    // SAFETY: getrusage writes to a correctly sized, initialized rusage.
    let mut out = unsafe { std::mem::zeroed() };
    (unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut out) } == 0).then_some(out)
}

#[cfg(not(unix))]
fn usage() -> Option<Usage> {
    None
}

#[cfg(not(unix))]
struct Usage {
    ru_maxrss: i64,
}

pub(crate) fn cpu_ns(thread: bool) -> Option<u64> {
    #[cfg(unix)]
    {
        // Thread clocks keep parallel file work from charging other Rayon workers' CPU.
        let clock = if thread {
            libc::CLOCK_THREAD_CPUTIME_ID
        } else {
            libc::CLOCK_PROCESS_CPUTIME_ID
        };
        let mut time: libc::timespec = unsafe { std::mem::zeroed() };
        // SAFETY: time points to writable timespec storage; clock IDs are platform constants.
        if unsafe { libc::clock_gettime(clock, &mut time) } != 0 {
            return None;
        }
        Some(
            (time.tv_sec as u64)
                .saturating_mul(1_000_000_000)
                .saturating_add(time.tv_nsec as u64),
        )
    }
    #[cfg(not(unix))]
    {
        let _ = thread;
        None
    }
}
