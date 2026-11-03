//! Go sink rules (#622 S1): call position first, receiver second.
//!
//! Same hybrid policy as the JS and Python rules. A receiver that is an imported
//! package is checked by import path (so `import osexec "os/exec"` works and
//! `cobra.Command(..)` does not); any other receiver is unknown and keeps firing
//! for the high-severity names, minus proven non-sinks (a call with no argument
//! such as `u.Query()`, a receiver declared as an HTTP request or framework
//! context, a container receiver for `Remove`, a lexically shadowed binding).

use super::callee::{
    args, binding_scope, callee_of, chain_names, chain_tail, count_idents, is_literal_concat, peel,
    walk, Src,
};
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

/// Declared type fragments that make a receiver a container.
const CONTAINER_TYPES: &[&str] = &[
    "list", "lru", "cache", "heap", "set", "queue", "stack", "ring", "items",
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
        GoBind::Param { ty: Some(t) } | GoBind::Var { ty: Some(t), .. } => Some(p.node_text(&t)),
        _ => None,
    }
}

/// True when a declared type is exactly `gin.Context` or `*gin.Context` (not `plugin.Context`).
fn is_gin_context(ty: &str) -> bool {
    ty.trim().trim_start_matches('*').trim() == "gin.Context"
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

/// True when a SQL query argument is constant text: a literal (or literal concatenation) or a
/// `const` that is not shadowed. Taint then reaches the call only as a bound parameter (#636).
fn is_constant_query(p: Src<'_>, node: Node<'_>) -> bool {
    if is_literal_concat(p, node) {
        return true;
    }
    if node.kind() != "identifier" {
        return false;
    }
    let name = p.node_text(&node);
    match resolve(p, node, name) {
        GoBind::Const => true,
        // `query := "..."` / `var query = "..."`, never written again in the file.
        GoBind::Var { init: Some(i), .. } => {
            is_literal_concat(p, i) && binding_count(p, binding_scope(node, i), name) == 1
        }
        _ => false,
    }
}

/// Number of bindings of `name` in `scope` (declarations of every form, parameters, and writes
/// of every form). A query is constant only when its one declaration is the whole story.
fn binding_count(p: Src<'_>, scope: Node<'_>, name: &str) -> usize {
    let mut count = 0;
    walk(scope, &mut |n| {
        let targets: Vec<Node<'_>> = match n.kind() {
            "short_var_declaration" | "assignment_statement" | "range_clause" => {
                n.child_by_field_name("left").into_iter().collect()
            }
            "inc_dec_statement" => n.named_child(0).into_iter().collect(),
            "var_spec" | "parameter_declaration" | "variadic_parameter_declaration" => {
                let mut c = n.walk();
                n.children_by_field_name("name", &mut c).collect()
            }
            _ => Vec::new(),
        };
        for t in targets {
            count += count_idents(p, t, name);
        }
    });
    count
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
        (None, GoBind::Var { init: Some(i), .. }) if i.kind() == "selector_expression" => (
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
            (!list.is_empty() && !request && !local_func).then(|| {
                // The query text sits after the context argument of the `...Context` forms.
                let query = list.get(usize::from(n.ends_with("Context")));
                SinkHit::new("go-db-call", "CWE-89")
                    .weak_if(query.is_some_and(|q| is_constant_query(p, *q)))
            })
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
            // Framework contexts fire too: echo `c.HTML(code, s)` and iris `ctx.HTML(s)` write a
            // raw string. A 3-argument call on a value receiver, not a package (`c.HTML(code, name, data)`, gin)
            // renders an auto-escaping template: tagged `go-html-render` and graded weak (#636). Everything else keeps `go-template-html`.
            let ok = !matches!(recv, Recv::Bare) && !list.is_empty();
            let render = list.len() == 3 && matches!(recv, Recv::Value);
            // Weak only for the gin shape (#636): a receiver declared exactly `gin.Context` or
            // `*gin.Context` (package-qualified identity). A status code argument is no proof of
            // an escaping renderer, and iris `HTML(format, a, b)` writes raw formatted HTML.
            let gin = recv_node
                .and_then(|r| declared_type(p, r))
                .is_some_and(is_gin_context);
            ok.then(|| {
                SinkHit::new(
                    if render {
                        "go-html-render"
                    } else {
                        "go-template-html"
                    },
                    "CWE-79",
                )
                .weak_if(render && gin)
            })
        }
        "WriteFile" | "Rename" | "Chmod" | "Chown" | "Remove" | "RemoveAll" => {
            let ok = match recv {
                Recv::Package(path) => FILE_PATHS.contains(&path),
                Recv::Bare => false,
                Recv::Named(n) => {
                    FILE_NAMES.contains(&n) || !is_container(p, facts, recv_node, &names)
                }
                Recv::Value => !is_container(p, facts, recv_node, &names),
            };
            (ok && !list.is_empty()).then(|| SinkHit::new("go-file-op", "CWE-22"))
        }
        "Redirect" => hit("go-redirect", "CWE-601"),
        "redirect" => matches!(recv, Recv::Bare).then(|| SinkHit::new("go-redirect", "CWE-601")),
        _ => None,
    }
}

/// Container receivers whose `Remove(e)` is not a file removal. The name alone is a hint
/// only when the receiver has no declared type; a declared type must itself be a container
/// type, and a file-system type (`afero.Fs`, `*os.File`, `fs.FS`) never is.
fn is_container(p: Src<'_>, facts: &Facts, recv: Option<Node<'_>>, names: &[&str]) -> bool {
    let named = names
        .last()
        .is_some_and(|n| CONTAINERS.contains(&n.to_ascii_lowercase().as_str()));
    if !named {
        return false;
    }
    if recv.is_some_and(|r| is_file_constructed(p, facts, r)) {
        return false;
    }
    match recv.and_then(|r| declared_type(p, r)) {
        None => true,
        Some(t) => {
            let t = t.to_ascii_lowercase();
            let file_like = ["fs", "afero", "file"].iter().any(|w| t.contains(w));
            !file_like && CONTAINER_TYPES.iter().any(|w| t.contains(w))
        }
    }
}

/// True when the receiver is a local initialised by a call into a file package
/// (`cache := afero.NewOsFs()`, `l, _ := os.Open(..)`): a file value, not a container.
fn is_file_constructed(p: Src<'_>, facts: &Facts, r: Node<'_>) -> bool {
    let r = peel(r);
    if r.kind() != "identifier" {
        return false;
    }
    let GoBind::Var {
        init: Some(init), ..
    } = resolve(p, r, p.node_text(&r))
    else {
        return false;
    };
    if init.kind() != "call_expression" {
        return false;
    }
    let Some(f) = init.child_by_field_name("function") else {
        return false;
    };
    let Some(op) = (f.kind() == "selector_expression")
        .then(|| f.child_by_field_name("operand"))
        .flatten()
        .filter(|o| o.kind() == "identifier")
    else {
        return false;
    };
    let name = p.node_text(&op);
    match facts.imports.get(name) {
        Some(path) => FILE_PATHS.contains(&path.as_str()),
        None => FILE_NAMES.contains(&name),
    }
}
