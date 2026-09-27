//! S1b (SPEC §3.1): what a JS/TS name statically denotes. RED scaffold: types and
//! signatures only, with no behavior.
#![allow(dead_code)] // S1b-2b and S1b-3 wire these; remove the allow there
use super::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JsTerminal {
    pub(crate) local: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
    pub(crate) wrapped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsBinding {
    Callable(JsTerminal),
    MayCall,
    Refused(&'static str),
    Unchecked(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Strictness {
    Strict,
    Sloppy,
    Unknown,
}

pub(super) type Index<'a> = BTreeMap<String, Vec<Node<'a>>>;

#[derive(Default)]
pub(crate) struct JsBindingCache<'a> {
    pub(super) decls: BTreeMap<usize, (Index<'a>, BTreeSet<String>)>,
}

const CLASSIFIED: &str = "";

impl ParsedFile {
    pub(crate) fn js_ts_module_binding<'a>(
        &'a self,
        _name: &str,
        _site: Node<'a>,
        _cache: &mut JsBindingCache<'a>,
    ) -> JsBinding {
        JsBinding::Refused("unbound")
    }

    pub(super) fn js_ts_strictness(&self, _scope: Node<'_>) -> Strictness {
        Strictness::Unknown
    }
}

#[cfg(test)]
#[path = "js_binding_tests.rs"]
mod tests;
