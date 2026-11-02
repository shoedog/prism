//! Review-taint flat sink matching by call position and receiver (#622 S1).
//!
//! The flat `SINK_PATTERNS` list in `taint.rs` matched substrings of any
//! identifier on a line (`Query` in `sanitizedQuery`, `eval` in `revalidate`,
//! `get_user` in `get_user_id()`). For JS/TS/TSX, Python and Go the decision now
//! comes from the AST: the identifier must be the callee of a call (or the target
//! of an assignment for DOM and location sinks), and where the bare name is
//! ambiguous the receiver is checked (a database handle, a response object, a
//! module bound to `child_process`, a writer parameter). Every rule carries a CWE
//! tag. Other languages keep the legacy substring list, except that Bash-only
//! command names stop firing outside Bash and `get_user` is exact and C/C++ only.
//!
//! Structured sinks (`GO_CWE78_SINKS`, `JS_CWE22_SINKS`, ...) are untouched and
//! consulted independently by the caller.

mod callee;
mod go;
mod js;
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

/// Bash command names in the flat list: shell semantics, so only for Bash files.
const BASH_ONLY: &[&str] = &[
    "=eval",
    "=source",
    "xargs",
    "=su",
    "=sudo",
    "=chmod",
    "=chown",
    "sqlite3",
    "=curl",
    "=wget",
    "=exec",
    "=awk",
    "=sed",
    "=find",
    "=mtd",
    "=fw_setenv",
    "=fw_printenv",
    "=uci",
    "=iptables",
    "=ip6tables",
    "=ifconfig",
    "=ip",
    "=brctl",
    "=bridge",
    "=vconfig",
    "=swconfig",
    "=insmod",
    "=modprobe",
    "=rmmod",
    "procd_set_param",
];

#[derive(Default)]
struct FileFacts {
    js: OnceCell<js::Facts>,
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
            return legacy_hit(language, text, legacy);
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
            _ => {
                let f = facts.js.get_or_init(|| js::Facts::collect(src, root));
                js::classify(src, f, id, text)
            }
        }
    }
}

/// The legacy list for languages that have no rules yet.
fn legacy_hit(language: Language, text: &str, patterns: &[&str]) -> Option<SinkHit> {
    let c_family = matches!(language, Language::C | Language::Cpp);
    patterns
        .iter()
        .filter(|p| language == Language::Bash || !BASH_ONLY.contains(p))
        .any(|p| match *p {
            // Kernel `get_user`: an exact C/C++ name, not `get_user_id`.
            "get_user" => c_family && text == "get_user",
            p => super::taint::matches_sink(text, p),
        })
        .then_some(SinkHit {
            rule: "legacy",
            cwe: None,
        })
}

#[cfg(test)]
mod tests;
