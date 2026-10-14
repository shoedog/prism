//! Slice V: a `const` binding that takes its value from a call to a bare identifier `H`. This is
//! the skeleton the tests are written against; it follows nothing.
use super::js_binding::JsBindingCache;
use super::ParsedFile;
use crate::ast::JsFollow;
use crate::js_returns::JsReturnPath;
use tree_sitter::Node;

impl ParsedFile {
    pub(crate) fn js_ts_follow_returned<'a>(
        &'a self,
        _ident: Node<'a>,
        _name: &str,
        _cache: &mut JsBindingCache<'a>,
    ) -> Option<(JsFollow, JsReturnPath)> {
        None
    }
}

#[cfg(test)]
#[path = "js_binding_returned_tests.rs"]
mod tests;
