//! Admission boundary for JS/TS-specific binding queries.
use super::ParsedFile;

impl ParsedFile {
    #[inline]
    pub(crate) fn js_ts_binding_enabled(&self) -> bool {
        let enabled = self.language.is_js_family();
        #[cfg(test)]
        let enabled = enabled || LEGACY.with(|flag| flag.get());
        #[cfg(test)]
        if enabled {
            ADMISSIONS.with(|n| n.set(n.get() + 1));
        }
        enabled
    }
}

// Bypass only the newly added guards. Existing guards stay active, preserving
// the exact pre-gate behavior. Run complete DFG comparisons on one owned worker
// via build_on_current_pool so this flag cannot silently miss the file job.
#[cfg(test)]
thread_local! {
    static LEGACY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ADMISSIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn legacy<R>(run: impl FnOnce() -> R) -> R {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            LEGACY.with(|flag| flag.set(self.0));
        }
    }
    let _restore = Restore(LEGACY.with(|flag| flag.replace(true)));
    run()
}

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;
