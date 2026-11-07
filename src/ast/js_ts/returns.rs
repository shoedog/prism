//! Slice V: returned-value facts for JS/TS. A function whose value-returning statements all
//! hand back the same object literal, array literal or callable yields, per key or slot, one
//! callable value in the function's own file. The facts are computed per file from syntax at
//! build time (`ParsedFile::js_ts_return_facts`), stored per function on the call graph, and
//! read by the resolver when a site's callee is a value another in-repo function returned.
use crate::ast::JsFollow;
use std::collections::BTreeMap;

/// Which part of a function's returned value a binding takes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum JsReturnPath {
    /// `const x = H(...)`: the returned value itself.
    Whole,
    /// `const { k: x } = H(...)`: property `k` of an object literal.
    Key(String),
    /// `const [a, x] = H(...)`: element `i` of an array literal.
    Slot(usize),
}

/// What one function returns, as callable values in the function's file. A key or slot is
/// present only when every value-returning statement of the function selects the same
/// callable for it; `Whole` only when every one returns that same callable itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JsReturnFacts {
    pub whole: Option<JsFollow>,
    pub keys: BTreeMap<String, JsFollow>,
    pub slots: BTreeMap<usize, JsFollow>,
}

impl JsReturnFacts {
    pub fn is_empty(&self) -> bool {
        self.whole.is_none() && self.keys.is_empty() && self.slots.is_empty()
    }

    /// The callable value this function returns at `path`, when a fact exists.
    pub fn get(&self, path: &JsReturnPath) -> Option<&JsFollow> {
        match path {
            JsReturnPath::Whole => self.whole.as_ref(),
            JsReturnPath::Key(k) => self.keys.get(k),
            JsReturnPath::Slot(i) => self.slots.get(i),
        }
    }
}
