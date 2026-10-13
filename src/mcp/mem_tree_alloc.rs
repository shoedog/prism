//! Live bytes allocated by tree-sitter, for the memory report (shared-daemon spec §6.1).
//!
//! Tree-sitter allocates its syntax trees through four function pointers that
//! `tree_sitter::set_allocator` can replace. [`install`] replaces them with wrappers around the
//! same libc allocator that adds each allocation's usable size to a counter and subtracts it on
//! free, so [`live_bytes`] is every live tree-sitter allocation: the retained syntax trees (hidden
//! nodes and allocator rounding included) plus prism's compiled query cache, which is about 54 KB
//! and fixed per process. It is an upper bound on what candidate M1 (drop the retained trees)
//! would release, and trees that are re-parsed later would come back; counting the nodes a
//! traversal visits cannot give it.
//!
//! Only the `prism-mcp` binary in memory-report mode installs the hooks. Library code and
//! library tests must not: tests run concurrently in one process, and swapping the allocator
//! pointers under a parse in flight is a data race.

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

/// Live bytes by usable size. Signed so that an allocation made before the hooks were installed
/// and freed after cannot wrap it; reads clamp at zero.
static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);

/// Live bytes of tree-sitter allocations made since [`install`] (the retained trees plus prism's
/// compiled query cache, about 54 KB fixed per process), or `None` when the hooks are not
/// installed (never called, or the platform has no usable-size call).
pub fn live_bytes() -> Option<u64> {
    INSTALLED
        .load(Ordering::Acquire)
        .then(|| clamp(LIVE_BYTES.load(Ordering::Relaxed)))
}

fn clamp(live: i64) -> u64 {
    live.max(0) as u64
}

/// Route tree-sitter's allocations through the counting hooks. Idempotent.
///
/// Call it once, in the binary, before anything parses; allocations made earlier are not counted
/// (their later frees are clamped, see [`LIVE_BYTES`]).
///
/// # Safety
///
/// `tree_sitter::set_allocator` overwrites global function pointers, which tree-sitter reads
/// without synchronization. The caller must guarantee that no other thread is parsing, or
/// otherwise inside tree-sitter, while this runs, and must call it once, before anything in the
/// process parses (a second call returns without doing anything, but a first call made after
/// parsing began races with the parse). The hooks themselves forward to the libc
/// `malloc`/`calloc`/`realloc`/`free` that tree-sitter uses by default, so a pointer allocated
/// before installation can be freed by the hooks and the reverse, and they are never
/// uninstalled, so no allocation can outlive the hooks that must free it.
pub unsafe fn install() {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        if INSTALLED.swap(true, Ordering::AcqRel) {
            return;
        }
        // SAFETY: the caller upholds the contract under `# Safety`; the hooks are libc-compatible.
        unsafe {
            tree_sitter::set_allocator(
                Some(hook_malloc),
                Some(hook_calloc),
                Some(hook_realloc),
                Some(hook_free),
            );
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
use hooks::{hook_calloc, hook_free, hook_malloc, hook_realloc};

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod hooks {
    use super::LIVE_BYTES;
    use std::ffi::c_void;
    use std::sync::atomic::Ordering;

    /// Bytes the allocator really reserved for `ptr`.
    unsafe fn usable_size(ptr: *mut c_void) -> i64 {
        if ptr.is_null() {
            return 0;
        }
        #[cfg(target_os = "macos")]
        let size = libc::malloc_size(ptr as *const c_void);
        #[cfg(target_os = "linux")]
        let size = libc::malloc_usable_size(ptr);
        size as i64
    }

    fn out_of_memory(size: usize) -> ! {
        eprintln!("tree-sitter failed to allocate {size} bytes");
        std::process::abort()
    }

    pub(super) unsafe extern "C" fn hook_malloc(size: usize) -> *mut c_void {
        let ptr = libc::malloc(size);
        if ptr.is_null() && size != 0 {
            out_of_memory(size);
        }
        LIVE_BYTES.fetch_add(usable_size(ptr), Ordering::Relaxed);
        ptr
    }

    pub(super) unsafe extern "C" fn hook_calloc(count: usize, size: usize) -> *mut c_void {
        let ptr = libc::calloc(count, size);
        if ptr.is_null() && count != 0 && size != 0 {
            out_of_memory(count.saturating_mul(size));
        }
        LIVE_BYTES.fetch_add(usable_size(ptr), Ordering::Relaxed);
        ptr
    }

    pub(super) unsafe extern "C" fn hook_realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
        // A null `ptr` has size 0 here, which makes this a malloc.
        let old = usable_size(ptr);
        let new_ptr = libc::realloc(ptr, size);
        if new_ptr.is_null() && size != 0 {
            out_of_memory(size);
        }
        if new_ptr.is_null() {
            // realloc(ptr, 0) may free `ptr` and return null.
            LIVE_BYTES.fetch_sub(old, Ordering::Relaxed);
        } else {
            LIVE_BYTES.fetch_add(usable_size(new_ptr) - old, Ordering::Relaxed);
        }
        new_ptr
    }

    pub(super) unsafe extern "C" fn hook_free(ptr: *mut c_void) {
        // Read the size before the pointer is released; free(NULL) is a no-op.
        LIVE_BYTES.fetch_sub(usable_size(ptr), Ordering::Relaxed);
        libc::free(ptr);
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests {
    use super::*;
    use std::ffi::c_void;

    /// The raw counter. Nothing else in the library test process installs the hooks, so this
    /// test owns it and exact equality on return-to-start is safe.
    fn raw() -> i64 {
        LIVE_BYTES.load(Ordering::Relaxed)
    }

    #[test]
    fn the_hooks_track_malloc_calloc_realloc_and_free() {
        let start = raw();

        // SAFETY: each pointer comes from a hook and is released by the matching hook once.
        unsafe {
            let block = hook_malloc(100);
            assert!(raw() - start >= 100, "malloc(100) added {}", raw() - start);

            let block = hook_realloc(block, 1000);
            assert!(
                raw() - start >= 1000,
                "realloc to 1000 left {} live",
                raw() - start
            );

            hook_free(block);
            assert_eq!(raw(), start, "free returns the counter to its start");

            let from_null = hook_realloc(std::ptr::null_mut::<c_void>(), 64);
            assert!(raw() - start >= 64, "realloc(NULL, 64) acts as malloc");
            hook_free(from_null);
            assert_eq!(raw(), start);

            let zeroed = hook_calloc(10, 10);
            assert!(
                raw() - start >= 100,
                "calloc(10, 10) added {}",
                raw() - start
            );
            assert!((0..100).all(|offset| *(zeroed as *const u8).add(offset) == 0));
            hook_free(zeroed);
            assert_eq!(raw(), start);

            hook_free(std::ptr::null_mut());
            assert_eq!(raw(), start, "free(NULL) changes nothing");
        }
    }

    #[test]
    fn live_bytes_is_absent_until_installed_and_never_negative() {
        // The hooks are not installed in this process, so the figure is absent.
        assert_eq!(live_bytes(), None);
        // A pre-installation allocation freed afterwards drives the raw count below zero.
        assert_eq!(clamp(-4096), 0);
        assert_eq!(clamp(0), 0);
        assert_eq!(clamp(4096), 4096);
    }
}
