//! Python module identity for review-taint sinks (#637 Slice A2).
//!
//! A closed table per module (paramiko, pexpect, fabric / invoke, jinja2) decides a call as a
//! strong sink with a CWE when the callee is proven by a lexically resolved binding, and
//! otherwise says nothing (positive evidence only: nothing is ever suppressed here).
//! Identity comes from an import of a listed module under any local name (every binding of
//! the spelling in the scope the use resolves to is that one import, and no star import can
//! replace it), or one hop from it: a local whose only binding is a call
//! of a listed class, a `with Class(..) as v` target, or the first parameter of a function
//! decorated with a `task` imported from fabric / invoke. A rebinding, a second binding, a
//! shadowing parameter, a field receiver or an annotation proves nothing.

use super::callee::{args, Callee, Src};
use super::py_scope::{scope_bindings, Bound, Scoped};
use super::python::{is_literal, Facts};
use super::SinkHit;
use tree_sitter::Node;

/// A class whose instances carry a method table.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ssh,
    Conn,
    Jinja,
    Sandbox,
}

/// Fully qualified class paths per kind.
fn class_kind(path: &str) -> Option<Kind> {
    match path {
        "paramiko.SSHClient" | "paramiko.client.SSHClient" => Some(Kind::Ssh),
        "fabric.Connection"
        | "fabric.connection.Connection"
        | "invoke.Context"
        | "invoke.context.Context" => Some(Kind::Conn),
        "jinja2.Environment" | "jinja2.environment.Environment" => Some(Kind::Jinja),
        "jinja2.sandbox.SandboxedEnvironment" => Some(Kind::Sandbox),
        _ => None,
    }
}

/// `task` decorators whose first parameter is a connection or context (Fabric 2 and invoke;
/// Fabric 1 tasks take command-line strings).
const TASK_PATHS: &[&str] = &["fabric.task", "invoke.task", "invoke.tasks.task"];

const COMMAND: (&str, &str) = ("py-module-command", "CWE-78");
const TEMPLATE: (&str, &str) = ("py-module-template", "CWE-1336");

/// Module-level callables (and the `PopenSpawn` constructor).
fn path_rule(path: &str) -> Option<(&'static str, &'static str)> {
    let (module, name) = path.rsplit_once('.')?;
    let hit = match (module, name) {
        ("pexpect", "spawn" | "spawnu" | "run" | "runu") => true,
        ("pexpect.popen_spawn", "PopenSpawn") => true,
        ("fabric" | "fabric.api" | "invoke", "run" | "sudo" | "local") => true,
        _ => false,
    };
    hit.then_some(COMMAND)
}

/// Methods of a proven instance; `true` marks the sandboxed (defused) grade.
fn method_rule(kind: Kind, method: &str) -> Option<((&'static str, &'static str), bool)> {
    match (kind, method) {
        (Kind::Ssh, "exec_command") => Some((COMMAND, false)),
        (Kind::Conn, "run" | "sudo" | "local") => Some((COMMAND, false)),
        (Kind::Jinja, "from_string") => Some((TEMPLATE, false)),
        (Kind::Sandbox, "from_string") => Some((TEMPLATE, true)),
        _ => None,
    }
}

/// The `(module, member)` import a bare name stands for at its use: every binding in the
/// nearest scope that binds it is the same import, and no star import can replace it.
fn import_at(p: Src<'_>, f: &Facts, ident: Node<'_>) -> Option<(String, String)> {
    let Scoped::Found(scope, bound) = scope_bindings(p, &f.sites, ident, p.node_text(&ident))
    else {
        return None;
    };
    let mut first: Option<(&String, &String)> = None;
    for b in &bound {
        let Bound::Import(m, x) = b else { return None };
        if first.is_some_and(|(fm, fx)| (fm, fx) != (m, x)) {
            return None;
        }
        first = Some((m, x));
    }
    let (m, x) = first?;
    if scope.kind() == "module" && f.wildcards.iter().any(|w| w != m) {
        return None;
    }
    Some((m.clone(), x.clone()))
}

/// The dotted path a name or attribute chain stands for through imports alone.
fn path_of(p: Src<'_>, f: &Facts, expr: Node<'_>) -> Option<String> {
    match expr.kind() {
        "identifier" => {
            let (module, member) = import_at(p, f, expr)?;
            Some(if member.is_empty() {
                module
            } else {
                format!("{module}.{member}")
            })
        }
        "attribute" => {
            let object = expr.child_by_field_name("object")?;
            let attr = expr.child_by_field_name("attribute")?;
            Some(format!("{}.{}", path_of(p, f, object)?, p.node_text(&attr)))
        }
        _ => None,
    }
}

/// The kind of a call of a listed class (`paramiko.SSHClient()`).
fn constructed(p: Src<'_>, f: &Facts, value: Node<'_>) -> Option<Kind> {
    if value.kind() != "call" {
        return None;
    }
    class_kind(&path_of(p, f, p.call_function_node(&value)?)?)
}

/// The kind of instance a receiver holds: a call of a listed class, or one hop from an import.
fn instance_kind(p: Src<'_>, f: &Facts, recv: Node<'_>) -> Option<Kind> {
    if recv.kind() == "call" {
        return constructed(p, f, recv);
    }
    if recv.kind() != "identifier" {
        return None;
    }
    let name = p.node_text(&recv);
    let Scoped::Found(scope, bound) = scope_bindings(p, &f.sites, recv, name) else {
        return None;
    };
    // A star import can replace a module-level name.
    if bound.len() != 1 || (scope.kind() == "module" && !f.wildcards.is_empty()) {
        return None;
    }
    match &bound[0] {
        Bound::Assign(v) => constructed(p, f, *v),
        Bound::Loop(Some(v))
            if v.parent().is_some_and(|a| {
                a.kind() == "as_pattern" && a.parent().is_some_and(|w| w.kind() == "with_item")
            }) =>
        {
            constructed(p, f, *v)
        }
        Bound::Param(_) => is_task_param(p, f, recv, name).then_some(Kind::Conn),
        _ => None,
    }
}

/// Parameter names in order; a non-name parameter (`*args`, `/`) holds its slot as "".
fn param_names<'a>(p: Src<'a>, params: Node<'_>) -> Vec<&'a str> {
    let mut cursor = params.walk();
    params
        .named_children(&mut cursor)
        .map(|q| match q.kind() {
            "identifier" => p.node_text(&q),
            "typed_parameter" => q
                .named_child(0)
                .filter(|n| n.kind() == "identifier")
                .map_or("", |n| p.node_text(&n)),
            "default_parameter" | "typed_default_parameter" => q
                .child_by_field_name("name")
                .map_or("", |n| p.node_text(&n)),
            _ => "",
        })
        .collect()
}

/// `name` is the first parameter of the nearest function that binds it, and that function is
/// decorated with a `task` of fabric / invoke.
fn is_task_param(p: Src<'_>, f: &Facts, at: Node<'_>, name: &str) -> bool {
    let mut cur = at.parent();
    while let Some(n) = cur {
        if matches!(n.kind(), "lambda" | "function_definition") {
            if let Some(params) = n.child_by_field_name("parameters") {
                let names = param_names(p, params);
                if names.contains(&name) {
                    return n.kind() == "function_definition"
                        && names.first() == Some(&name)
                        && has_task_decorator(p, f, n);
                }
            }
        }
        cur = n.parent();
    }
    false
}

fn has_task_decorator(p: Src<'_>, f: &Facts, func: Node<'_>) -> bool {
    let Some(decorated) = func.parent().filter(|d| d.kind() == "decorated_definition") else {
        return false;
    };
    let mut found = false;
    let mut cursor = decorated.walk();
    for d in decorated.named_children(&mut cursor) {
        if d.kind() != "decorator" {
            continue;
        }
        let Some(mut e) = d.named_child(0) else {
            continue;
        };
        if e.kind() == "call" {
            match e.child_by_field_name("function") {
                Some(inner) => e = inner,
                None => continue,
            }
        }
        found |= path_of(p, f, e).is_some_and(|path| TASK_PATHS.contains(&path.as_str()));
    }
    found
}

/// The module-identity hit for the call `c`, when its callee is proven.
pub(super) fn classify(p: Src<'_>, f: &Facts, c: &Callee<'_>) -> Option<SinkHit> {
    if args(c.call).is_empty() {
        return None;
    }
    let func = p.call_function_node(&c.call)?;
    if let Some((rule, cwe)) = path_of(p, f, func).as_deref().and_then(path_rule) {
        return Some(SinkHit::new(rule, cwe));
    }
    if func.kind() != "attribute" {
        return None;
    }
    let attr = func.child_by_field_name("attribute")?;
    let kind = instance_kind(p, f, func.child_by_field_name("object")?)?;
    let ((rule, cwe), sandboxed) = method_rule(kind, p.node_text(&attr))?;
    // Like main's `Template(..)` rule, a constant template text is no injection.
    if (rule, cwe) == TEMPLATE && !template_source_is_dynamic(p, c) {
        return None;
    }
    Some(SinkHit::new(rule, cwe).weak_if(sandboxed))
}

/// The template source (first positional argument, or `source=`) is not a string literal.
fn template_source_is_dynamic(p: Src<'_>, c: &Callee<'_>) -> bool {
    let all = args(c.call);
    let keyword = all.iter().find_map(|a| {
        (a.kind() == "keyword_argument"
            && a.child_by_field_name("name")
                .is_some_and(|n| p.node_text(&n) == "source"))
        .then(|| a.child_by_field_name("value"))
        .flatten()
    });
    let positional = all.iter().copied().find(|a| a.kind() != "keyword_argument");
    keyword
        .or(positional)
        .is_some_and(|src| !is_literal(p, src))
}
