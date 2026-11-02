//! Go sink rules (#622 S1): call position first, receiver second.
//!
//! Same hybrid policy as the JS and Python rules. A receiver that is an imported
//! package is checked by import path (so `import osexec "os/exec"` works and
//! `cobra.Command(..)` does not); any other receiver is unknown and keeps firing
//! for the high-severity names, minus proven non-sinks (a call with no argument
//! such as `u.Query()`, a receiver declared as an HTTP request or framework
//! context, a container receiver for `Remove`, a lexically shadowed binding).

use super::callee::{args, callee_of, chain_names, chain_tail, is_literal_concat, peel, walk, Src};
use super::go_scope::{resolve, GoBind};
use super::SinkHit;
use std::collections::BTreeMap;
use tree_sitter::Node;

const SQL_NAMES: &[&str] = &[
    "Exec",
    "ExecContext",
    "Query",
    "QueryContext",
    "QueryRow",
    "QueryRowContext",
    "Queryx",
    "QueryxContext",
    "QueryRowx",
    "QueryRowxContext",
    "MustExec",
    "MustExecContext",
    "NamedExec",
    "NamedExecContext",
    "NamedQuery",
    "NamedQueryContext",
];
const EXEC_PATHS: &[&str] = &["os/exec", "golang.org/x/sys/execabs"];
const SYSCALL_PATHS: &[&str] = &["syscall", "golang.org/x/sys/unix"];
const FILE_PATHS: &[&str] = &["os", "io/ioutil", "io/fs", "github.com/spf13/afero"];
const FILE_NAMES: &[&str] = &["os", "ioutil", "afero", "fs"];
/// Declared receiver types that read a request (`ctx.Query("k")` is a source).
const REQUEST_TYPES: &[&str] = &[
    "gin.Context",
    "echo.Context",
    "fiber.Ctx",
    "iris.Context",
    "http.Request",
    "url.URL",
    "url.Values",
    "RequestCtx",
];
/// Container receivers whose `Remove(e)` is not a file removal.
const CONTAINERS: &[&str] = &[
    "l", "ll", "list", "lst", "heap", "cache", "lru", "set", "queue", "stack", "items",
];

/// Import paths by local package name.
#[derive(Default)]
pub(super) struct Facts {
    imports: BTreeMap<String, String>,
}

impl Facts {
    pub(super) fn collect(p: Src<'_>, root: Node<'_>) -> Self {
        let mut facts = Facts::default();
        walk(root, &mut |node| {
            if node.kind() == "import_spec" {
                facts.import(p, node);
            }
        });
        facts
    }

    /// `import "a/b"`, `import alias "a/b"`, `import "a/b/v2"` (package `b`).
    fn import(&mut self, p: Src<'_>, spec: Node<'_>) {
        let Some(path) = spec.child_by_field_name("path") else {
            return;
        };
        let path = p
            .node_text(&path)
            .trim_matches(|c| c == '"' || c == '`')
            .to_string();
        if let Some(alias) = spec.child_by_field_name("name") {
            let alias = p.node_text(&alias);
            if alias != "_" && alias != "." {
                self.imports.insert(alias.to_string(), path);
            }
            return;
        }
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
        self.imports.insert(name.to_string(), path);
    }
}

/// What a call's receiver is.
enum Recv<'a> {
    /// No receiver: a bare call.
    Bare,
    /// An imported package, with its import path.
    Package(&'a str),
    /// An identifier that names a package nobody imported in this file (a
    /// fixture without imports, or a dot import): its own spelling.
    Named(&'a str),
    /// A variable, field, call result: unknown.
    Value,
}

fn receiver<'a>(p: Src<'a>, facts: &'a Facts, recv: Option<Node<'_>>) -> Recv<'a> {
    let Some(r) = recv else { return Recv::Bare };
    let r = peel(r);
    if r.kind() != "identifier" {
        return Recv::Value;
    }
    let name = p.node_text(&r);
    if !matches!(resolve(p, r, name), GoBind::Unresolved) {
        return Recv::Value;
    }
    match facts.imports.get(name) {
        Some(path) => Recv::Package(path.as_str()),
        None => Recv::Named(name),
    }
}

/// Declared type text of an identifier receiver (`ctx *gin.Context`).
fn declared_type<'a>(p: Src<'a>, r: Node<'_>) -> Option<&'a str> {
    let r = peel(r);
    if r.kind() != "identifier" {
        return None;
    }
    match resolve(p, r, p.node_text(&r)) {
        GoBind::Param { ty: Some(t) } => Some(p.node_text(&t)),
        _ => None,
    }
}

/// True when the format argument is a literal, a literal concatenation, a `const`
/// that is not shadowed, or a name qualified by an imported package
/// (`constant.Prompt`; a package-level name is a constant in practice, unlike a
/// field read such as `req.Format`).
fn is_constant_format(p: Src<'_>, facts: &Facts, node: Node<'_>) -> bool {
    if is_literal_concat(p, node) {
        return true;
    }
    match node.kind() {
        "identifier" => matches!(resolve(p, node, p.node_text(&node)), GoBind::Const),
        "selector_expression" => node.child_by_field_name("operand").is_some_and(|o| {
            let name = p.node_text(&o);
            o.kind() == "identifier"
                && facts.imports.contains_key(name)
                && matches!(resolve(p, o, name), GoBind::Unresolved)
        }),
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
    matches!(
        resolve(p, arg, p.node_text(&arg)),
        GoBind::Param { ty: Some(t) } if p.node_text(&t).ends_with("Writer")
    )
}

fn hit(rule: &'static str, cwe: &'static str) -> Option<SinkHit> {
    Some(SinkHit::new(rule, cwe))
}

pub(super) fn classify(p: Src<'_>, facts: &Facts, id: Node<'_>, text: &str) -> Option<SinkHit> {
    let c = callee_of(id)?;
    // `query := db.Query; query(x)`: a bare callee bound to a method value.
    let (name, recv_node) = match (c.receiver, resolve(p, id, text)) {
        (None, GoBind::Var { init: Some(i) }) if i.kind() == "selector_expression" => (
            i.child_by_field_name("field")
                .map_or(text, |f| p.node_text(&f)),
            i.child_by_field_name("operand"),
        ),
        (r, _) => (text, r),
    };
    let list = args(c.call);
    let recv = receiver(p, facts, recv_node);
    let names = recv_node.map(|r| chain_names(p, r)).unwrap_or_default();
    let local_func = matches!(recv, Recv::Bare) && matches!(resolve(p, id, text), GoBind::Func);
    match name {
        "Exec" if matches!(recv, Recv::Package(path) if SYSCALL_PATHS.contains(&path)) => {
            hit("go-exec-command", "CWE-78")
        }
        n if SQL_NAMES.contains(&n) => {
            let request = recv_node
                .and_then(|r| declared_type(p, r))
                .is_some_and(|t| REQUEST_TYPES.iter().any(|q| t.contains(q)));
            (!list.is_empty() && !request && !local_func)
                .then(|| SinkHit::new("go-db-call", "CWE-89"))
        }
        "Command" | "CommandContext" => {
            let ok = match recv {
                Recv::Package(path) => EXEC_PATHS.contains(&path),
                Recv::Bare => !local_func,
                _ => true,
            };
            (ok && !list.is_empty()).then(|| SinkHit::new("go-exec-command", "CWE-78"))
        }
        "Sprintf" => {
            let format = list.first()?;
            let fmt = matches!(recv, Recv::Package("fmt") | Recv::Named("fmt"));
            (fmt && !is_constant_format(p, facts, *format))
                .then(|| SinkHit::new("go-format-string", "CWE-134"))
        }
        "Fprintf" | "Fprint" | "Fprintln" => {
            if !matches!(recv, Recv::Package("fmt") | Recv::Named("fmt")) {
                return None;
            }
            if list.first().is_some_and(|w| is_writer(p, *w)) {
                return hit("go-writer-output", "CWE-79");
            }
            let format = list.get(1)?;
            (name == "Fprintf" && !is_constant_format(p, facts, *format))
                .then(|| SinkHit::new("go-format-string", "CWE-134"))
        }
        "HTML" => {
            // `ctx.HTML(code, tpl, data)` on a framework context renders a template; any
            // other receiver (an imported helper such as `response.HTML`) keeps firing.
            let renders = recv_node
                .and_then(|r| declared_type(p, r))
                .is_some_and(|t| REQUEST_TYPES.iter().any(|q| t.contains(q)));
            let ok = !matches!(recv, Recv::Bare) && !renders;
            (ok && !list.is_empty()).then(|| SinkHit::new("go-template-html", "CWE-79"))
        }
        "WriteFile" | "Rename" | "Chmod" | "Chown" | "Remove" | "RemoveAll" => {
            let ok = match recv {
                Recv::Package(path) => FILE_PATHS.contains(&path),
                Recv::Bare => false,
                Recv::Named(n) => FILE_NAMES.contains(&n) || !is_container(&names),
                Recv::Value => !is_container(&names),
            };
            (ok && !list.is_empty()).then(|| SinkHit::new("go-file-op", "CWE-22"))
        }
        "Redirect" => hit("go-redirect", "CWE-601"),
        "redirect" => matches!(recv, Recv::Bare).then(|| SinkHit::new("go-redirect", "CWE-601")),
        _ => None,
    }
}

fn is_container(names: &[&str]) -> bool {
    names
        .last()
        .is_some_and(|n| CONTAINERS.contains(&n.to_ascii_lowercase().as_str()))
}
