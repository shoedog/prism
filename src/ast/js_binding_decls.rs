//! S1b (SPEC §3.1 D, P rows): the declarations a scope's statement list holds. RED scaffold.
#![allow(dead_code)] // S1b-2b and S1b-3 wire these; remove the allow there
use super::js_binding::{Index, Strictness};
use super::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    pub(super) fn js_ts_declare_walk<'a>(
        &'a self,
        _node: Node<'a>,
        _mode: (bool, bool, Strictness),
        _taint: &BTreeSet<String>,
        _out: &mut Index<'a>,
        _annex: &mut BTreeSet<String>,
    ) {
    }
}
