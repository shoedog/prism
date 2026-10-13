//! Slice L (owner direction 2026-10-13): the nearest binding decides, and an alias is followed
//! to what it names. A JS/TS `const` alias whose value is an identifier, a plain namespace
//! member, or a flat object-pattern key over a namespace import is followed through a bounded,
//! cycle-safe chain of unique, unwritten `const` links to one callable, one import local, or one
//! member of a namespace import's module. Anything else keeps the base outcome (`Alias`).
//!
//! The follow is a separate query over `js_ts_site_binding`'s answer. `js_ts_classify` and
//! therefore D4's `js_ts_local_export_target` still see `Alias` for every link (an exported
//! alias stays unproven; following aliases across files is a later slice), and the follow never
//! classifies a declaration itself: each link re-enters `js_ts_site_binding` at a strictly
//! different declaration, so the visited set and the link bound terminate it.
use super::js_binding::{JsBindingCache, JsTerminal};
use super::ParsedFile;
use tree_sitter::Node;


/// What a followed `const` alias chain ends at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsFollow {
    /// A proven callable in this file (its `wrapped` flag included).
    Callable(JsTerminal),
    /// The program-scope import binding `local`; the import rungs decide the target.
    ImportLocal(String),
    /// Member `member` of the namespace import of `module_path`.
    NamespaceMember { module_path: String, member: String },
}

impl ParsedFile {
    /// Test-first stub: follows nothing.
    pub(crate) fn js_ts_follow_alias<'a>(
        &'a self,
        _ident: Node<'a>,
        _name: &str,
        _cache: &mut JsBindingCache<'a>,
    ) -> Option<JsFollow> {
        None
    }
}

#[cfg(test)]
#[path = "js_binding_alias_tests.rs"]
mod tests;
