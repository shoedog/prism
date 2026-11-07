//! Test-only selection of the raw lookup for the shared traversal loops.
use std::cell::Cell;
thread_local! {
    static UNCACHED: Cell<bool> = const { Cell::new(false) };
    static DISPATCHES: Cell<usize> = const { Cell::new(0) };
}
pub(super) fn select<'a>(
    module: &'a dyn Fn(&str, &str) -> Option<String>,
    cached: &'a dyn Fn(&str, &str) -> Option<String>,
) -> &'a dyn Fn(&str, &str) -> Option<String> {
    if UNCACHED.get() {
        DISPATCHES.set(DISPATCHES.get() + 1);
        module
    } else {
        cached
    }
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

#[test]
#[should_panic(expected = "oracle dispatch must execute on this thread")]
fn requires_a_dispatch() {
    with_uncached(|| ());
}

#[test]
#[should_panic(expected = "oracle dispatch must execute on this thread")]
fn dispatch_on_another_thread_does_not_satisfy_guard() {
    with_uncached(|| {
        std::thread::spawn(|| super::namespace(&Default::default(), &|_, _| None))
            .join()
            .unwrap()
    });
}
