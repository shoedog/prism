//! Test-only observations returned by real jobs to the measuring caller.
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Sites {
    pub pool_files: usize,
    pub caller_files: usize,
    pub pool_pairs: usize,
    pub caller_pairs: usize,
}

thread_local! { static SITES: Cell<Sites> = Cell::new(Sites::default()); }

pub(crate) fn record_file(on_pool: bool) {
    SITES.with(|cell| {
        let mut sites = cell.get();
        if on_pool {
            sites.pool_files += 1;
        } else {
            sites.caller_files += 1;
        }
        cell.set(sites);
    });
}

pub(crate) fn record_pair(on_pool: bool) {
    SITES.with(|cell| {
        let mut sites = cell.get();
        if on_pool {
            sites.pool_pairs += 1;
        } else {
            sites.caller_pairs += 1;
        }
        cell.set(sites);
    });
}

pub(crate) fn measured<R>(op: impl FnOnce() -> R) -> (R, Sites) {
    struct Reset(Sites);
    impl Drop for Reset {
        fn drop(&mut self) {
            SITES.with(|cell| cell.set(self.0));
        }
    }
    let _reset = Reset(SITES.with(|cell| cell.replace(Sites::default())));
    let result = op();
    (result, SITES.with(Cell::get))
}
