//! Go sink rules (#622 S1): call position and receiver, never a name fragment.

use super::callee::{args, callee_of, chain_tail, is_literal_concat, walk, Src};
use super::SinkHit;
use std::collections::BTreeSet;
use tree_sitter::Node;

const FILE_PACKAGES: &[&str] = &["os", "ioutil", "afero", "fs"];

/// Names a file declares with `const`, and the packages it imports: both make a
/// format string constant (`feedFmt`, `constant.Prompt`).
#[derive(Default)]
pub(super) struct Facts {
    consts: BTreeSet<String>,
    packages: BTreeSet<String>,
}

impl Facts {
    pub(super) fn collect(p: Src<'_>, root: Node<'_>) -> Self {
        let mut facts = Facts::default();
        walk(root, &mut |node| match node.kind() {
            "const_spec" => {
                let mut cursor = node.walk();
                for name in node.children_by_field_name("name", &mut cursor) {
                    facts.consts.insert(p.node_text(&name).to_string());
                }
            }
            "import_spec" => facts.import(p, node),
            _ => {}
        });
        facts
    }

    /// `import "a/b"`, `import alias "a/b"`, `import "a/b/v2"` (package `b`).
    fn import(&mut self, p: Src<'_>, spec: Node<'_>) {
        if let Some(alias) = spec.child_by_field_name("name") {
            self.packages.insert(p.node_text(&alias).to_string());
            return;
        }
        let Some(path) = spec.child_by_field_name("path") else {
            return;
        };
        let path = p.node_text(&path).trim_matches(|c| c == '"' || c == '`');
        let mut parts = path.rsplit('/');
        let last = parts.next().unwrap_or("");
        let is_version = last.len() > 1
            && last.starts_with('v')
            && last[1..].chars().all(|c| c.is_ascii_digit());
        let name = if is_version {
            parts.next().unwrap_or(last)
        } else {
            last
        };
        self.packages.insert(name.to_string());
    }
}

/// A receiver tail that names a `database/sql` handle (`db`, `s.db`, `tx`, ...).
fn is_db_handle(tail: &str) -> bool {
    let t = tail.to_ascii_lowercase();
    t.ends_with("db")
        || matches!(
            t.as_str(),
            "tx" | "conn" | "stmt" | "database" | "pool" | "querier" | "execer" | "dbtx" | "trx"
        )
}

/// True when the format argument is a literal, a literal concatenation, a `const`,
/// or a name qualified by an imported package (`constant.Prompt`; a package-level
/// name is a constant in practice, unlike a field read such as `req.Format`).
fn is_constant_format(p: Src<'_>, facts: &Facts, node: Node<'_>) -> bool {
    if is_literal_concat(p, node) {
        return true;
    }
    match node.kind() {
        "identifier" => facts.consts.contains(p.node_text(&node)),
        "selector_expression" => node
            .child_by_field_name("operand")
            .is_some_and(|o| o.kind() == "identifier" && facts.packages.contains(p.node_text(&o))),
        _ => false,
    }
}

/// True when `arg` is a writer: `c.Writer`, or a parameter typed `...Writer`
/// (`http.ResponseWriter`, `io.Writer`) of an enclosing function.
fn is_writer(p: Src<'_>, arg: Node<'_>) -> bool {
    if arg.kind() == "selector_expression" {
        return chain_tail(p, arg) == Some("Writer");
    }
    if arg.kind() != "identifier" {
        return false;
    }
    let name = p.node_text(&arg);
    let mut scope = arg.parent();
    while let Some(node) = scope {
        if matches!(
            node.kind(),
            "function_declaration" | "method_declaration" | "func_literal"
        ) {
            if let Some(params) = node.child_by_field_name("parameters") {
                let mut cursor = params.walk();
                for decl in params.named_children(&mut cursor) {
                    let typed_writer = decl
                        .child_by_field_name("type")
                        .is_some_and(|t| p.node_text(&t).ends_with("Writer"));
                    if !typed_writer {
                        continue;
                    }
                    let mut names = decl.walk();
                    let declares = decl
                        .children_by_field_name("name", &mut names)
                        .any(|n| p.node_text(&n) == name);
                    if declares {
                        return true;
                    }
                }
            }
        }
        scope = node.parent();
    }
    false
}

pub(super) fn classify(p: Src<'_>, facts: &Facts, id: Node<'_>, text: &str) -> Option<SinkHit> {
    let c = callee_of(id)?;
    let tail = c.receiver.and_then(|r| chain_tail(p, r));
    let list = args(c.call);
    match text {
        "Exec" | "Query" | "QueryRow" => tail
            .is_some_and(is_db_handle)
            .then(|| SinkHit::new("go-db-call", "CWE-89")),
        "ExecContext" | "QueryContext" | "QueryRowContext" => {
            c.receiver.map(|_| SinkHit::new("go-db-call", "CWE-89"))
        }
        "Command" | "CommandContext" => {
            (tail == Some("exec")).then(|| SinkHit::new("go-exec-command", "CWE-78"))
        }
        "Sprintf" => {
            let format = list.first()?;
            (tail == Some("fmt") && !is_constant_format(p, facts, *format))
                .then(|| SinkHit::new("go-format-string", "CWE-134"))
        }
        "Fprintf" | "Fprint" | "Fprintln" => {
            if tail != Some("fmt") {
                return None;
            }
            if list.first().is_some_and(|w| is_writer(p, *w)) {
                return Some(SinkHit::new("go-writer-output", "CWE-79"));
            }
            let format = list.get(1)?;
            (text == "Fprintf" && !is_constant_format(p, facts, *format))
                .then(|| SinkHit::new("go-format-string", "CWE-134"))
        }
        "HTML" => (tail == Some("template")).then(|| SinkHit::new("go-template-html", "CWE-79")),
        "WriteFile" | "Rename" | "Chmod" => tail
            .is_some_and(|t| FILE_PACKAGES.contains(&t))
            .then(|| SinkHit::new("go-file-op", "CWE-22")),
        "Redirect" => Some(SinkHit::new("go-redirect", "CWE-601")),
        "redirect" => c
            .receiver
            .is_none()
            .then(|| SinkHit::new("go-redirect", "CWE-601")),
        _ => None,
    }
}
