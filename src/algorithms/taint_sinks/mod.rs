//! Review-taint flat sink matching by call position, with a hybrid receiver policy (#622 S1).
//!
//! The flat `SINK_PATTERNS` list in `taint.rs` matched substrings of any
//! identifier on a line (`Query` in `sanitizedQuery`, `eval` in `revalidate`,
//! `get_user` in `get_user_id()`). For JS/TS/TSX, Python and Go the decision now
//! comes from the AST: the identifier must be the callee of a call (or the target
//! of an assignment for DOM and location sinks). Where the name is ambiguous the
//! receiver decides under the hybrid policy: an UNKNOWN receiver keeps firing for
//! the high-severity names, only a PROVEN non-sink suppresses (a regex, a logger,
//! a message channel, a collection or file object, a same-named function declared
//! in the file, a zero-argument call, a shadowed binding), and the old receiver
//! allowlists (response chains, database handles, `child_process` bindings,
//! import-path-resolved Go packages) are positive evidence only: they choose the
//! rule and CWE tag. Bindings resolve lexically (`js_scope`, `py_scope`,
//! `go_scope`). Other languages keep the legacy substring list (`legacy.rs`),
//! except for shell-only command names (Bash only) and `get_user` (exact, C/C++).
//!
//! Structured sinks (`GO_CWE78_SINKS`, `JS_CWE22_SINKS`, ...) are untouched and
//! consulted independently by the caller.

mod callee;
mod go;
mod go_scope;
mod js;
mod js_scope;
mod legacy;
mod py_scope;
mod python;

pub(crate) use js::exec_call_is_command as js_exec_call_is_command;

use self::callee::Src;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::cell::{OnceCell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use tree_sitter::Node;

/// A matched sink rule and its weakness class.
///
/// The finding structure has no slot for the tag (`category` stays `taint_sink`
/// and the description is pinned by consumers), so the tag lives here for the
/// unit tests and the SARIF/targets follow-ups (#590) to consume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct SinkHit {
    /// Rule name (`js-eval`, `go-db-call`, `legacy`, ...).
    pub rule: &'static str,
    /// CWE tag, absent for the legacy substring list.
    pub cwe: Option<&'static str>,
}

impl SinkHit {
    const fn new(rule: &'static str, cwe: &'static str) -> Self {
        Self {
            rule,
            cwe: Some(cwe),
        }
    }
}

#[derive(Default)]
struct FileFacts {
    py: OnceCell<python::Facts>,
    go: OnceCell<go::Facts>,
}

/// Per-run classifier: caches the import/const facts each rule reads, per file.
#[derive(Default)]
pub(crate) struct SinkClassifier {
    facts: RefCell<BTreeMap<String, Rc<FileFacts>>>,
}

impl SinkClassifier {
    /// Decide whether identifier `id` (text `text`) in `file` is a flat sink.
    /// `legacy` is the substring list, consulted only for languages without rules.
    pub(crate) fn classify(
        &self,
        file: &str,
        parsed: &ParsedFile,
        id: Node<'_>,
        text: &str,
        legacy: &[&str],
    ) -> Option<SinkHit> {
        let language = parsed.language;
        if !(language.is_js_family() || matches!(language, Language::Python | Language::Go)) {
            return legacy::legacy_hit(language, text, legacy);
        }
        let facts = {
            let mut map = self.facts.borrow_mut();
            match map.get(file) {
                Some(facts) => facts.clone(),
                None => map.entry(file.to_string()).or_default().clone(),
            }
        };
        let src = Src::of(parsed);
        let root = parsed.tree.root_node();
        match language {
            Language::Python => {
                let f = facts.py.get_or_init(|| python::Facts::collect(src, root));
                python::classify(src, f, id, text)
            }
            Language::Go => {
                let f = facts.go.get_or_init(|| go::Facts::collect(src, root));
                go::classify(src, f, id, text)
            }
            _ => js::classify(src, id, text),
        }
    }
}

#[cfg(test)]
mod tests;
