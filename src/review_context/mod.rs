//! Review-time context for taint: which repo files a call leaves the analysed
//! set for (#602). PR-1 holds only the attribution primitive; nothing in the
//! review pipeline calls it yet.

// The consumer (Layer A boundary notes) lands in the next PR.
#[allow(dead_code)]
pub(crate) mod import_targets;

#[cfg(test)]
mod import_targets_py_go_tests;
#[cfg(test)]
mod import_targets_tests;
