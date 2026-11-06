//! SCRATCH (measure/mem-census): live-byte allocator counter + CPG census. Never merged.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

pub static LIVE: AtomicUsize = AtomicUsize::new(0);
pub static PEAK: AtomicUsize = AtomicUsize::new(0);

pub struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            let n = LIVE.fetch_add(l.size(), Relaxed) + l.size();
            PEAK.fetch_max(n, Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l);
        LIVE.fetch_sub(l.size(), Relaxed);
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(l);
        if !p.is_null() {
            let n = LIVE.fetch_add(l.size(), Relaxed) + l.size();
            PEAK.fetch_max(n, Relaxed);
        }
        p
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        let q = System.realloc(p, l, new);
        if !q.is_null() {
            if new >= l.size() {
                let n = LIVE.fetch_add(new - l.size(), Relaxed) + (new - l.size());
                PEAK.fetch_max(n, Relaxed);
            } else {
                LIVE.fetch_sub(l.size() - new, Relaxed);
            }
        }
        q
    }
}

pub fn enabled() -> bool {
    std::env::var_os("PRISM_MEM_CENSUS").is_some()
}
pub fn live() -> usize {
    LIVE.load(Relaxed)
}
/// Print live bytes and peak since last mark; reset peak.
pub fn mark(name: &str) {
    if !enabled() {
        return;
    }
    let l = live();
    let p = PEAK.swap(l, Relaxed);
    eprintln!(
        "CENSUS_MARK {{\"mark\":\"{name}\",\"live\":{l},\"peak_since_last\":{p},\"rss\":{}}}",
        rss()
    );
}

/// VmRSS in bytes (0 if unreadable).
pub fn rss() -> usize {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<usize>().ok())
        })
        .map(|kb| kb * 1024)
        .unwrap_or(0)
}

/// Destructive census: drop `v`, print how many live bytes (and RSS) that freed.
pub fn drop_measure<T>(name: &str, v: T) {
    let (l0, r0) = (live(), rss());
    drop(v);
    let (l1, r1) = (live(), rss());
    eprintln!(
        "CENSUS_DROP {{\"item\":\"{name}\",\"freed_live\":{},\"live_after\":{l1},\"rss_freed\":{},\"rss_after\":{r1}}}",
        l0 - l1,
        r0 as i64 - r1 as i64
    );
}

/// `PRISM_CENSUS_DROP=<point>` selects the destructive point of this run.
pub fn drop_point() -> Option<String> {
    std::env::var("PRISM_CENSUS_DROP").ok()
}


/// Measure bytes retained by a clone of `v` (dense rebuild; includes tree overhead).
pub fn clone_bytes<T: Clone>(v: &T) -> usize {
    let before = live();
    let c = v.clone();
    let after = live();
    drop(c);
    after - before
}
