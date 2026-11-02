//! Python sink rules (#622 S1): call position and receiver, never a name fragment.

use super::callee::{args, callee_of, chain_tail, literal_string, walk, Src};
use super::SinkHit;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Names a file binds to `subprocess`, `sqlalchemy` and their `run` / `text`.
#[derive(Default)]
pub(super) struct Facts {
    /// Local names of the `subprocess` module.
    subprocess: BTreeSet<String>,
    /// Local names bound to `subprocess.run`.
    run: BTreeSet<String>,
    /// Local names of the `sqlalchemy` module.
    sqlalchemy: BTreeSet<String>,
    /// Local names bound to `sqlalchemy.text`.
    text: BTreeSet<String>,
}

impl Facts {
    pub(super) fn collect(p: Src<'_>, root: Node<'_>) -> Self {
        let mut facts = Facts::default();
        walk(root, &mut |node| match node.kind() {
            "import_statement" => facts.import(p, node),
            "import_from_statement" => facts.import_from(p, node),
            _ => {}
        });
        facts
    }

    /// `import subprocess [as sp]`
    fn import(&mut self, p: Src<'_>, node: Node<'_>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            let (module, local) = module_and_local(p, child);
            match module {
                "subprocess" => self.subprocess.insert(local.to_string()),
                "sqlalchemy" => self.sqlalchemy.insert(local.to_string()),
                _ => false,
            };
        }
    }

    /// `from subprocess import run [as r]`
    fn import_from(&mut self, p: Src<'_>, node: Node<'_>) {
        let module = node
            .child_by_field_name("module_name")
            .map(|m| p.node_text(&m))
            .unwrap_or("");
        let mut cursor = node.walk();
        for name in node.children_by_field_name("name", &mut cursor) {
            let (orig, local) = module_and_local(p, name);
            match (module, orig) {
                ("subprocess", "run") => self.run.insert(local.to_string()),
                ("sqlalchemy", "text") => self.text.insert(local.to_string()),
                _ => false,
            };
        }
    }
}

/// `(name, local alias)` of a `dotted_name` or `aliased_import`.
fn module_and_local<'p>(p: Src<'p>, node: Node<'_>) -> (&'p str, &'p str) {
    if node.kind() == "aliased_import" {
        let name = node.child_by_field_name("name").map(|n| p.node_text(&n));
        let alias = node.child_by_field_name("alias").map(|n| p.node_text(&n));
        if let (Some(name), Some(alias)) = (name, alias) {
            return (name, alias);
        }
    }
    let t = p.node_text(&node);
    (t, t)
}

fn is_literal(p: Src<'_>, node: Node<'_>) -> bool {
    match node.kind() {
        "string" => literal_string(p, node).is_some(),
        "concatenated_string" => {
            let mut cursor = node.walk();
            let all = node
                .named_children(&mut cursor)
                .all(|c| c.kind() == "comment" || is_literal(p, c));
            all
        }
        _ => false,
    }
}

pub(super) fn classify(p: Src<'_>, facts: &Facts, id: Node<'_>, text: &str) -> Option<SinkHit> {
    // Bare callees bound by `from subprocess import run [as r]` /
    // `from sqlalchemy import text [as t]`.
    if facts.run.contains(text) || facts.text.contains(text) {
        let c = callee_of(id)?;
        if c.receiver.is_none() {
            if facts.run.contains(text) {
                return Some(SinkHit::new("py-subprocess", "CWE-78"));
            }
            if args(c.call).first().is_some_and(|a| !is_literal(p, *a)) {
                return Some(SinkHit::new("py-sql-clause", "CWE-89"));
            }
        }
    }
    let tail = |c: &super::callee::Callee<'_>| c.receiver.and_then(|r| chain_tail(p, r));
    match text {
        "eval" | "exec" => {
            let c = callee_of(id)?;
            (c.receiver.is_none() || tail(&c) == Some("builtins"))
                .then(|| SinkHit::new("py-eval", "CWE-95"))
        }
        "system" | "popen" => {
            let c = callee_of(id)?;
            (tail(&c) == Some("os") || (c.receiver.is_none() && text == "system"))
                .then(|| SinkHit::new("py-os-command", "CWE-78"))
        }
        "Popen" | "check_call" | "check_output" => {
            callee_of(id).map(|_| SinkHit::new("py-subprocess", "CWE-78"))
        }
        "run" => {
            let c = callee_of(id)?;
            let t = tail(&c)?;
            (t == "subprocess" || facts.subprocess.contains(t))
                .then(|| SinkHit::new("py-subprocess", "CWE-78"))
        }
        "execute" | "executemany" | "raw" => {
            let c = callee_of(id)?;
            c.receiver.map(|_| SinkHit::new("py-db-call", "CWE-89"))
        }
        // ORM clause entry points: the clause argument is where raw SQL can enter.
        // Keyword-only calls (`Model.objects.filter(pk=x)`, `filter_by(id=x)`) are
        // parameterised lookups, not clauses.
        "where" | "filter" | "filter_by" => {
            let c = callee_of(id)?;
            let clause = args(c.call)
                .iter()
                .any(|a| !matches!(a.kind(), "keyword_argument" | "dictionary_splat"));
            (c.receiver.is_some() && clause).then(|| SinkHit::new("py-sql-clause", "CWE-89"))
        }
        "text" => {
            let c = callee_of(id)?;
            let t = tail(&c)?;
            let module = t == "sqlalchemy" || facts.sqlalchemy.contains(t);
            (module && args(c.call).first().is_some_and(|a| !is_literal(p, *a)))
                .then(|| SinkHit::new("py-sql-clause", "CWE-89"))
        }
        "mark_safe" | "Markup" => callee_of(id).map(|_| SinkHit::new("py-html", "CWE-79")),
        "render_template_string" => callee_of(id).map(|_| SinkHit::new("py-template", "CWE-1336")),
        "getattr" | "setattr" => {
            let c = callee_of(id)?;
            let name_arg = args(c.call).into_iter().nth(1)?;
            (c.receiver.is_none() && !is_literal(p, name_arg))
                .then(|| SinkHit::new("py-dynamic-attr", "CWE-470"))
        }
        "redirect" | "HttpResponseRedirect" => {
            callee_of(id).map(|_| SinkHit::new("py-redirect", "CWE-601"))
        }
        "open" => {
            let c = callee_of(id)?;
            (c.receiver.is_none() || matches!(tail(&c), Some("io" | "os")))
                .then(|| SinkHit::new("py-file-open", "CWE-22"))
        }
        "remove" | "unlink" | "rmdir" | "removedirs" => {
            let c = callee_of(id)?;
            (tail(&c) == Some("os")).then(|| SinkHit::new("py-file-remove", "CWE-22"))
        }
        _ => None,
    }
}
