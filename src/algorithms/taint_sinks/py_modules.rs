//! Python module identity for review-taint sinks (#637 Slice A2).
//!
//! A closed table per module (paramiko, pexpect, fabric / invoke, jinja2) decides a call as a
//! strong sink with a CWE when the callee is proven by a lexically resolved binding, and
//! otherwise says nothing (positive evidence only: nothing is ever suppressed here).
//! Identity comes from an import of a listed module under any local name (every binding of
//! the spelling is that one import), or one hop from it: a local whose only binding is a call
//! of a listed class, a `with Class(..) as v` target, or the first parameter of a function
//! decorated with a `task` imported from fabric / invoke. A rebinding, a second binding, a
//! shadowing parameter, a field receiver or an annotation proves nothing.

use super::callee::{args, Callee, Src};
use super::py_scope::{local_binding, Bound, Local};
use super::python::Facts;
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

/// `task` decorators of fabric and invoke.
const TASK_PATHS: &[&str] = &[
    "fabric.task",
    "fabric.api.task",
    "fabric.decorators.task",
    "invoke.task",
    "invoke.tasks.task",
];

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

/// The dotted path a name or attribute chain stands for through imports alone: every binding of
/// the root spelling in the file is the same import (`import m [as a]`, `from m import x [as a]`).
fn path_of(p: Src<'_>, f: &Facts, expr: Node<'_>) -> Option<String> {
    match expr.kind() {
        "identifier" => {
            let (module, member) = f.sites.trusted_import(p.node_text(&expr))?;
            Some(if member.is_empty() {
                module.clone()
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

/// The kind of instance a receiver identifier holds, one hop from an import.
fn instance_kind(p: Src<'_>, f: &Facts, recv: Node<'_>) -> Option<Kind> {
    if recv.kind() != "identifier" {
        return None;
    }
    let name = p.node_text(&recv);
    match local_binding(p, &f.sites, recv, name) {
        Local::One(Bound::Assign(v)) => constructed(p, f, v),
        Local::One(Bound::Loop(Some(v)))
            if v.parent().is_some_and(|a| {
                a.kind() == "as_pattern" && a.parent().is_some_and(|w| w.kind() == "with_item")
            }) =>
        {
            constructed(p, f, v)
        }
        Local::One(Bound::Param(_)) => is_task_param(p, f, recv, name).then_some(Kind::Conn),
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
    Some(SinkHit::new(rule, cwe).weak_if(sandboxed))
}
