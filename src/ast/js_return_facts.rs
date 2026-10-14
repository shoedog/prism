//! Slice V: the returned-value fact of one JS/TS function, computed from syntax. This is the
//! skeleton the tests are written against; it yields no fact.
use super::js_binding::JsBindingCache;
use super::ParsedFile;
use crate::js_returns::JsReturnFacts;
use tree_sitter::Node;

impl ParsedFile {
    pub(crate) fn js_ts_return_facts<'a>(
        &'a self,
        _func: Node<'a>,
        _cache: &mut JsBindingCache<'a>,
    ) -> Option<JsReturnFacts> {
        None
    }
}

#[cfg(test)]
#[path = "js_return_facts_tests.rs"]
mod tests;
