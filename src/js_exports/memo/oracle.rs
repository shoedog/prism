//! Test-only dispatch to the unchanged pre-memoization loops.
use std::cell::Cell;
thread_local! {
    static UNCACHED: Cell<bool> = const { Cell::new(false) };
    static DISPATCHES: Cell<usize> = const { Cell::new(0) };
}
pub(super) fn enabled() -> bool {
    let enabled = UNCACHED.get();
    if enabled {
        DISPATCHES.set(DISPATCHES.get() + 1);
    }
    enabled
}
pub(super) fn with_uncached<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            UNCACHED.set(self.0);
        }
    }
    let _restore = Restore(UNCACHED.replace(true));
    let before = DISPATCHES.get();
    let result = f();
    assert!(
        DISPATCHES.get() > before,
        "oracle dispatch must execute on this thread"
    );
    result
}
