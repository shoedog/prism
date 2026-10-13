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
    /// The kernel's count of this task's memory currently held by the compressor, as the task
    /// ledger accounts it (macOS): the size of those pages before compression, not the space the
    /// compressor uses to store them.
    pub compressed_bytes: Option<u64>,
    /// Process-lifetime peak: footprint on macOS, resident size on Linux. Never decreases.
    pub peak_bytes: Option<u64>,
    /// Bytes the allocator reports as in use at this instant (macOS: `size_in_use` summed over all
    /// malloc zones; Linux glibc: `uordblks + hblkhd`). Unlike the footprint, it falls when memory
    /// is freed. It is not resident memory: it leaves out mappings that do not come from malloc
    /// (thread stacks, mapped files). On glibc it is not "handed out and not freed": a block freed
    /// into a thread cache (tcache) or fastbin still counts as in use, and the allocator's own
    /// bookkeeping is included, so it can sit above the program's live allocations. `None` where
    /// there is no source (non-glibc Linux, other platforms, glibc older than 2.33).
    pub heap_in_use_bytes: Option<u64>,
}

/// Sample this process now.
pub fn sample() -> MemSample {
    let mut out = imp::sample();
    out.heap_in_use_bytes = heap_in_use_bytes();
    out
}

/// Bytes the allocator currently reports as in use; see [`MemSample::heap_in_use_bytes`].
fn heap_in_use_bytes() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        let mut stats = libc::malloc_statistics_t {
            blocks_in_use: 0,
            size_in_use: 0,
            max_size_in_use: 0,
            size_allocated: 0,
        };
        // SAFETY: a null zone asks for the totals over every zone, and `stats` is a writable
        // `malloc_statistics_t` that the call fills.
        unsafe { libc::malloc_zone_statistics(std::ptr::null_mut(), &mut stats) };
        Some(stats.size_in_use as u64)
    }
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        // `mallinfo2` exists only in glibc 2.33 and later. Referencing it directly would make the
        // whole `mcp` feature fail to link on an older glibc, so it is looked up at run time and
        // the figure is `None` when the symbol is absent.
        type Mallinfo2 = unsafe extern "C" fn() -> libc::mallinfo2;
        // SAFETY: `dlsym` is given the default search scope and a NUL-terminated name; it returns a
        // pointer or null and touches no memory of ours.
        let symbol = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"mallinfo2".as_ptr()) };
        if symbol.is_null() {
            return None;
        }
        // SAFETY: a non-null `mallinfo2` symbol in glibc is `struct mallinfo2 mallinfo2(void)`,
        // which is `Mallinfo2`; function and data pointers have the same size on this target.
        let mallinfo2: Mallinfo2 =
            unsafe { std::mem::transmute::<*mut libc::c_void, Mallinfo2>(symbol) };
        // SAFETY: the function takes no arguments and returns a plain struct by value.
        let info = unsafe { mallinfo2() };
        Some((info.uordblks as u64).saturating_add(info.hblkhd as u64))
    }
    #[cfg(not(any(target_os = "macos", all(target_os = "linux", target_env = "gnu"))))]
    {
        None
    }
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
        heap_in_use_bytes: None,
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

    /// The tests below allocate and free blocks of 64 to 256 MiB and compare process-wide
    /// samples before and after. Run concurrently they would see each other's blocks, so each
    /// holds this lock for its whole body.
    static LARGE_BLOCKS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn large_blocks_lock() -> std::sync::MutexGuard<'static, ()> {
        LARGE_BLOCKS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

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
                heap_in_use_bytes: None,
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
        let _serial = large_blocks_lock();
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

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn heap_in_use_rises_with_a_live_allocation_and_falls_when_it_is_freed() {
        let _serial = large_blocks_lock();
        const SIZE: usize = 256 << 20;
        const MIN_CHANGE: u64 = 192 << 20;
        let before = sample();
        let mut block = vec![0u8; SIZE];
        for offset in (0..SIZE).step_by(4096) {
            block[offset] = (offset % 251) as u8 + 1;
        }
        let high = sample();
        // Keep the block alive and observed past the second sample.
        assert!(
            block
                .iter()
                .step_by(1 << 20)
                .map(|byte| *byte as u64)
                .sum::<u64>()
                > 0
        );
        drop(block);
        let low = sample();

        let (Some(before), Some(high), Some(low)) = (
            before.heap_in_use_bytes,
            high.heap_in_use_bytes,
            low.heap_in_use_bytes,
        ) else {
            // The figure exists on macOS and on Linux with glibc 2.33 or later; a missing sample
            // there is a defect (or a glibc too old for `mallinfo2`), not a reason to skip.
            if cfg!(any(
                target_os = "macos",
                all(target_os = "linux", target_env = "gnu")
            )) {
                panic!(
                    "heap_in_use_bytes is unavailable on a platform that must report it \
                     (glibc older than 2.33 has no mallinfo2): {before:?} {high:?} {low:?}"
                );
            }
            return; // musl and other Linux libcs have no source for the figure
        };
        assert!(
            high >= before + MIN_CHANGE,
            "a live 256 MiB block should add at least 192 MiB in use: {before} -> {high}"
        );
        assert!(
            low + MIN_CHANGE <= high,
            "freeing the block should drop at least 192 MiB in use: {high} -> {low}"
        );
    }

    /// `task_info(TASK_VM_INFO)` is read through a hand-written struct layout. Cross-check the
    /// fields the report uses against `proc_pid_rusage`, which has a binding in `libc`.
    #[cfg(target_os = "macos")]
    #[test]
    fn task_info_fields_agree_with_proc_pid_rusage() {
        const TOLERANCE: u64 = 16 << 20;
        let _serial = large_blocks_lock();
        let from_task_info = sample();
        // SAFETY: `usage` is a writable `rusage_info_v4`, and the flavor names that layout.
        let mut usage: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
        let status = unsafe {
            libc::proc_pid_rusage(
                libc::getpid(),
                libc::RUSAGE_INFO_V4,
                (&mut usage as *mut libc::rusage_info_v4).cast(),
            )
        };
        assert_eq!(status, 0, "proc_pid_rusage failed");

        let close = |name: &str, ours: u64, reference: u64| {
            assert!(
                ours.abs_diff(reference) <= TOLERANCE,
                "{name}: task_info says {ours}, proc_pid_rusage says {reference}"
            );
        };
        close(
            "physical footprint",
            from_task_info.phys_footprint_bytes.expect("footprint"),
            usage.ri_phys_footprint,
        );
        close(
            "resident size",
            from_task_info.rss_bytes.expect("resident size"),
            usage.ri_resident_size,
        );
    }

    #[cfg(unix)]
    #[test]
    fn ru_maxrss_is_reported_in_bytes() {
        let _serial = large_blocks_lock();
        const SIZE: usize = 128 << 20;
        let mut block = vec![0u8; SIZE];
        for offset in (0..SIZE).step_by(4096) {
            block[offset] = (offset % 251) as u8 + 1;
        }
        assert!(
            block
                .iter()
                .step_by(1 << 20)
                .map(|byte| *byte as u64)
                .sum::<u64>()
                > 0
        );

        let max = ru_maxrss_bytes().expect("getrusage on unix");
        // SAFETY: `sysconf` takes a name and returns a value; it touches no memory of ours.
        let physical = unsafe {
            libc::sysconf(libc::_SC_PHYS_PAGES) as u64 * libc::sysconf(libc::_SC_PAGESIZE) as u64
        };
        assert!(physical > 0, "sysconf reported no physical memory");
        // The block above is resident, so the lifetime peak is at least its size. A value in
        // kilobytes would be about a thousand times too small; a bytes value scaled up by 1024
        // again would exceed the machine's memory.
        assert!(
            max >= SIZE as u64,
            "ru_maxrss {max} is below the 128 MiB just touched"
        );
        assert!(
            max <= physical,
            "ru_maxrss {max} exceeds physical memory {physical}"
        );
        drop(block);
    }
}
