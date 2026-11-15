//! A detached copy of a snapshot: every value is kept, and the five shared memo
//! cells are replaced by new cells holding clones of the current values, so
//! work done on the copy never reaches the original review's bookkeeping.
use super::JsPathsSnapshot;
use std::sync::{Arc, Mutex};

impl JsPathsSnapshot {
    /// `clone()` shares the five `Arc<Mutex<_>>` cells with the original. This copies
    /// every plain field (including the `#[serde(skip)]` ones a serde round trip loses)
    /// and gives the copy cells of its own with the same contents.
    pub(crate) fn detached(&self) -> Self {
        let mut copy = self.clone();
        copy.probes = Arc::new(Mutex::new(self.probes.lock().unwrap().clone()));
        copy.first_pass_facts = Arc::new(Mutex::new(self.first_pass_facts.lock().unwrap().clone()));
        copy.absent_inputs = Arc::new(Mutex::new(self.absent_inputs.lock().unwrap().clone()));
        copy.unsafe_inputs = Arc::new(Mutex::new(self.unsafe_inputs.lock().unwrap().clone()));
        copy.closure_memo = Arc::new(Mutex::new(self.closure_memo.lock().unwrap().clone()));
        copy
    }
}

/// The five memo cells' current values, for tests outside this module tree that
/// must show a rebase left the original's bookkeeping untouched.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoValues {
    pub(crate) probes: std::collections::BTreeSet<String>,
    pub(crate) first_pass_facts: std::collections::BTreeMap<String, Option<u8>>,
    pub(crate) absent_inputs: std::collections::BTreeSet<(String, String)>,
    pub(crate) unsafe_inputs: std::collections::BTreeSet<(String, String)>,
    pub(crate) closure_memo: std::collections::BTreeMap<(String, Option<Vec<String>>), bool>,
}

#[cfg(test)]
impl JsPathsSnapshot {
    pub(crate) fn memo_values(&self) -> MemoValues {
        MemoValues {
            probes: self.probes.lock().unwrap().clone(),
            first_pass_facts: self.first_pass_facts.lock().unwrap().clone(),
            absent_inputs: self.absent_inputs.lock().unwrap().clone(),
            unsafe_inputs: self.unsafe_inputs.lock().unwrap().clone(),
            closure_memo: self.closure_memo.lock().unwrap().clone(),
        }
    }
}
