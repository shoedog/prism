//! One sample of this process's memory, for the memory report (shared-daemon spec §6.1).
//! macOS: `task_info(TASK_VM_INFO)` for resident, footprint and compressed bytes, and
//! `proc_pid_rusage` for the lifetime peak footprint. Linux: `/proc/self/status`. Elsewhere every
//! field is `None` and the report is still produced.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct MemSample {
    /// Resident set size.
    pub rss_bytes: Option<u64>,
    /// Physical footprint (macOS): what the system charges the process, compressed pages included.
    pub phys_footprint_bytes: Option<u64>,
    /// Bytes held by the compressor on this process's behalf (macOS).
    pub compressed_bytes: Option<u64>,
    /// Process-lifetime peak: footprint on macOS, resident size on Linux. Never decreases.
    pub peak_bytes: Option<u64>,
}

/// Sample this process now.
pub fn sample() -> MemSample {
    imp::sample()
}

/// Peak resident set size from `getrusage`, in bytes (macOS reports bytes, Linux kilobytes).
pub fn ru_maxrss_bytes() -> Option<u64> {
    #[cfg(unix)]
    {
        // SAFETY: `usage` is a writable `rusage`; `getrusage` fills it or returns non-zero.
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } != 0 {
            return None;
        }
        let raw = u64::try_from(usage.ru_maxrss).ok()?;
        Some(if cfg!(target_os = "macos") {
            raw
        } else {
            raw.saturating_mul(1024)
        })
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Parse the two fields the report uses out of `/proc/self/status`.
#[cfg(any(target_os = "linux", test))]
fn parse_proc_status(text: &str) -> MemSample {
    let kilobytes = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|rest| rest.trim().strip_suffix("kB"))
            .and_then(|number| number.trim().parse::<u64>().ok())
            .map(|kb| kb.saturating_mul(1024))
    };
    MemSample {
        rss_bytes: kilobytes("VmRSS:"),
        phys_footprint_bytes: None,
        compressed_bytes: None,
        peak_bytes: kilobytes("VmHWM:"),
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::MemSample;

    /// `task_vm_info` through `phys_footprint`: the rev1 layout in `<mach/task_info.h>`, which is
    /// declared under `#pragma pack(4)`. The `libc` crate has no binding for this struct. The
    /// kernel fills as many fields as the passed count covers.
    #[repr(C, packed(4))]
    #[derive(Default, Clone, Copy)]
    struct TaskVmInfoRev1 {
        virtual_size: u64,
        region_count: i32,
        page_size: i32,
        resident_size: u64,
        resident_size_peak: u64,
        device: u64,
        device_peak: u64,
        internal: u64,
        internal_peak: u64,
        external: u64,
        external_peak: u64,
        reusable: u64,
        reusable_peak: u64,
        purgeable_volatile_pmap: u64,
        purgeable_volatile_resident: u64,
        purgeable_volatile_virtual: u64,
        compressed: u64,
        compressed_peak: u64,
        compressed_lifetime: u64,
        phys_footprint: u64,
    }

    /// `TASK_VM_INFO` in `<mach/task_info.h>`.
    const TASK_VM_INFO: libc::task_flavor_t = 22;

    pub(super) fn sample() -> MemSample {
        let mut out = MemSample::default();

        let mut info = TaskVmInfoRev1::default();
        let mut count = (std::mem::size_of::<TaskVmInfoRev1>()
            / std::mem::size_of::<libc::natural_t>())
            as libc::mach_msg_type_number_t;
        // SAFETY: `info` is a writable buffer of `count` natural_t units whose layout matches the
        // flavor; the kernel writes at most `count` units.
        #[allow(deprecated)]
        // libc points to the `mach2` crate; one call does not justify a dependency.
        let status = unsafe {
            libc::task_info(
                libc::mach_task_self(),
                TASK_VM_INFO,
                (&mut info as *mut TaskVmInfoRev1).cast(),
                &mut count,
            )
        };
        if status == libc::KERN_SUCCESS {
            // Copy out of the packed struct before use; references to packed fields are not allowed.
            let (resident, footprint, compressed) =
                (info.resident_size, info.phys_footprint, info.compressed);
            out.rss_bytes = Some(resident);
            out.phys_footprint_bytes = Some(footprint);
            out.compressed_bytes = Some(compressed);
        }

        // SAFETY: `usage` is a writable `rusage_info_v4`, and the flavor names that layout.
        let mut usage: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
        let status = unsafe {
            libc::proc_pid_rusage(
                libc::getpid(),
                libc::RUSAGE_INFO_V4,
                (&mut usage as *mut libc::rusage_info_v4).cast(),
            )
        };
        if status == 0 {
            out.peak_bytes = Some(usage.ri_lifetime_max_phys_footprint);
        }
        out
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::MemSample;

    pub(super) fn sample() -> MemSample {
        std::fs::read_to_string("/proc/self/status")
            .map(|text| super::parse_proc_status(&text))
            .unwrap_or_default()
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod imp {
    use super::MemSample;

    pub(super) fn sample() -> MemSample {
        MemSample::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proc_status_reads_resident_and_peak() {
        let status = "Name:\tprism-mcp\nVmPeak:\t  999999 kB\nVmHWM:\t    2048 kB\nVmRSS:\t    1024 kB\nThreads:\t4\n";
        assert_eq!(
            parse_proc_status(status),
            MemSample {
                rss_bytes: Some(1024 * 1024),
                phys_footprint_bytes: None,
                compressed_bytes: None,
                peak_bytes: Some(2048 * 1024),
            }
        );
        assert_eq!(parse_proc_status("Name:\tx\n"), MemSample::default());
        assert_eq!(
            parse_proc_status("VmRSS:\tnot-a-number kB\n").rss_bytes,
            None
        );
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn sample_tracks_a_touched_allocation() {
        const SIZE: usize = 64 << 20;
        let before = sample();
        let mut block = vec![0u8; SIZE];
        for offset in (0..SIZE).step_by(4096) {
            block[offset] = (offset % 251) as u8 + 1;
        }
        let after = sample();
        // Keep the block alive and observed past the second sample.
        assert!(
            block
                .iter()
                .step_by(1 << 20)
                .map(|byte| *byte as u64)
                .sum::<u64>()
                > 0
        );

        let grew = after.rss_bytes.expect("resident size") as i64
            - before.rss_bytes.expect("resident size") as i64;
        assert!(
            grew >= (32 << 20),
            "touching 64 MiB should add at least 32 MiB of resident memory, added {grew}"
        );
        let peak = after.peak_bytes.expect("lifetime peak");
        assert!(
            peak >= before.peak_bytes.expect("lifetime peak"),
            "the peak never decreases"
        );
        #[cfg(target_os = "macos")]
        {
            let footprint = after.phys_footprint_bytes.expect("footprint on macOS");
            assert!(footprint >= (32 << 20) as u64);
            assert!(
                peak >= footprint,
                "lifetime peak {peak} is below current footprint {footprint}"
            );
            assert!(after.compressed_bytes.is_some());
        }
    }

    #[cfg(unix)]
    #[test]
    fn ru_maxrss_is_reported_in_bytes() {
        let max = ru_maxrss_bytes().expect("getrusage on unix");
        let now = sample().rss_bytes.unwrap_or(0);
        // A value in kilobytes would be about a thousand times smaller than the resident size.
        assert!(
            max >= now / 2,
            "ru_maxrss {max} is implausibly small against rss {now}"
        );
    }
}
