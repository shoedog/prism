//! JS/TS/TSX sink rules (#622 S1): call position first, receiver second.
//!
//! A flat sink identifier is only a sink when it is the callee of a call (or the
//! target of an assignment for DOM and location sinks). Where the bare name is
//! ambiguous the receiver is checked with the hybrid policy: a receiver that is
//! *proven* a non-sink (a regex literal or a binding initialised with one, a
//! logger, a socket, a lexically declared helper of the same name) suppresses the
//! hit, a receiver proven a sink (a `child_process` binding, a response chain)
//! gives it a precise rule and CWE tag, and an unknown receiver keeps firing for
//! the high-severity names. Bindings are resolved lexically (`js_scope`), so a
//! parameter that shadows a module name never inherits the module's meaning.

use super::callee::{
    args, binding_scope, callee_of, chain_names, chain_tail, count_idents, literal_string, peel,
    walk, Callee, Src,
};
use super::js_scope::{is_reassigned, resolve, Binding};
use super::SinkHit;
use crate::frameworks::CallSite;
use crate::languages::Language;
use tree_sitter::Node;

const CP_MODULES: &[&str] = &["child_process", "node:child_process", "shelljs"];
const CP_FUNCTIONS: &[&str] = &[
    "exec",
    "execSync",
    "execFile",
    "execFileSync",
    "spawn",
    "spawnSync",
    "fork",
];
/// `execa` entry points: distinctive command runners callable under their own name.
const EXECA: &[&str] = &["execa", "execaSync", "execaCommand", "execaCommandSync"];
/// SQL entry points; the receiver is only checked for proven non-sinks.
const SQL_NAMES: &[&str] = &[
    "query",
    "queryAsync",
    "queryRaw",
    "$queryRaw",
    "$queryRawUnsafe",
    "$executeRaw",
    "$executeRawUnsafe",
    "whereRaw",
    "havingRaw",
    "orderByRaw",
    "raw",
    "literal",
    "execute",
    "executeQuery",
    "executeSql",
    "createQueryBuilder",
];
/// Receivers that name the global object (or the document, for `location`).
const GLOBALS: &[&str] = &[
    "window",
    "globalThis",
    "self",
    "global",
    "top",
    "parent",
    "document",
];
/// Receivers that name an HTTP response object.
const RESPONSES: &[&str] = &["res", "response", "reply", "resp"];
/// Receivers that name a database handle (positive evidence for `db.exec(sql)`).
const DB_HANDLES: &[&str] = &[
    "db",
    "database",
    "conn",
    "connection",
    "client",
    "pool",
    "knex",
    "sqlite",
    "tx",
    "trx",
    "sequelize",
    "prisma",
    "stmt",
];
/// Proven non-sink receivers: loggers and consoles (M0: `logger.warn(source)`).
const LOGGERS: &[&str] = &[
    "console", "logger", "log", "logging", "winston", "pino", "bunyan", "consola", "signale",
];
/// Proven non-sink receivers for `send`/`write`: message channels and stdio.
const CHANNELS: &[&str] = &[
    "socket",
    "sockets",
    "sock",
    "ws",
    "wss",
    "websocket",
    "emitter",
    "process",
    "worker",
    "port",
    "channel",
    "child",
    "io",
    "peer",
    "stdout",
    "stderr",
    "buf",
    "buffer",
];
/// Receivers that name the filesystem modules.
const FS_LIKE: &[&str] = &[
    "fs",
    "fse",
    "fsextra",
    "fsp",
    "promises",
    "fspromises",
    "rimraf",
];

fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

fn any_name(names: &[&str], set: &[&str]) -> bool {
    names.iter().any(|n| set.contains(&lower(n).as_str()))
}

/// `require('child_process')` / `import('node:child_process')`.
fn is_module_load(p: Src<'_>, node: Node<'_>) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let loads = node
        .child_by_field_name("function")
        .is_some_and(|f| f.kind() == "import" || p.node_text(&f) == "require");
    loads
        && args(node)
            .first()
            .and_then(|a| literal_string(p, *a))
            .is_some_and(|s| CP_MODULES.contains(&s))
}

/// True when `node` evaluates to the `child_process` module: a load, an
/// identifier whose nearest binding is one (through aliases), or the bare global.
fn is_cp_expr(p: Src<'_>, node: Node<'_>, depth: usize) -> bool {
    if depth > 4 {
        return false;
    }
    let n = peel(node);
    match n.kind() {
        "call_expression" => is_module_load(p, n),
        "identifier" => {
            let name = p.node_text(&n);
            match resolve(p, n, name) {
                Binding::Var { init: Some(i) } => is_cp_expr(p, i, depth + 1),
                Binding::Import {
                    module,
                    imported: None,
                } => literal_string(p, module).is_some_and(|s| CP_MODULES.contains(&s)),
                Binding::Unresolved => name == "child_process",
                _ => false,
            }
        }
        _ => false,
    }
}

fn cp_function_named(name: &str) -> Option<&'static str> {
    CP_FUNCTIONS.iter().copied().find(|f| *f == name)
}

/// The `child_process` function an expression denotes (`cp.exec`, a destructured
/// or imported name, `promisify(cp.exec)`), under whatever local name.
fn cp_function_of(p: Src<'_>, node: Node<'_>, depth: usize) -> Option<&'static str> {
    if depth > 4 {
        return None;
    }
    let n = peel(node);
    match n.kind() {
        "member_expression" => {
            let (obj, prop) = (
                n.child_by_field_name("object")?,
                n.child_by_field_name("property")?,
            );
            if is_cp_expr(p, obj, depth + 1) {
                cp_function_named(p.node_text(&prop))
            } else {
                None
            }
        }
        "identifier" => match resolve(p, n, p.node_text(&n)) {
            Binding::Destructured { init, key } if is_cp_expr(p, init, depth + 1) => {
                cp_function_named(p.node_text(&key?))
            }
            Binding::Var { init: Some(i) } => cp_function_of(p, i, depth + 1),
            Binding::Import {
                module,
                imported: Some(orig),
            } if literal_string(p, module).is_some_and(|s| CP_MODULES.contains(&s)) => {
                cp_function_named(p.node_text(&orig))
            }
            _ => None,
        },
        "call_expression" => {
            let callee = n.child_by_field_name("function")?;
            let t = p.node_text(&callee);
            if t == "promisify" || t.ends_with(".promisify") {
                cp_function_of(p, *args(n).first()?, depth + 1)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// A regex literal, `new RegExp(..)`, `RegExp(..)`, or a binding initialised with
/// one or declared `: RegExp` (M0: `/bytes=(\d*)-(\d*)/.exec(header)`).
fn is_regexish(p: Src<'_>, node: Node<'_>, depth: usize) -> bool {
    let n = peel(node);
    match n.kind() {
        "regex" => true,
        "new_expression" => n
            .child_by_field_name("constructor")
            .is_some_and(|c| p.node_text(&c) == "RegExp"),
        "call_expression" => n
            .child_by_field_name("function")
            .is_some_and(|f| p.node_text(&f) == "RegExp"),
        "identifier" => match resolve(p, n, p.node_text(&n)) {
            Binding::Var { init: Some(i) } => {
                depth < 3 && !is_reassigned(p, n, p.node_text(&n)) && is_regexish(p, i, depth + 1)
            }
            Binding::Param {
                annotation: Some(a),
            } => p.node_text(&a).contains("RegExp") && !is_reassigned(p, n, p.node_text(&n)),
            _ => false,
        },
        _ => false,
    }
}

/// A bare callee that resolves to a function declared in this file.
fn is_local_helper(p: Src<'_>, id: Node<'_>, text: &str) -> bool {
    matches!(resolve(p, id, text), Binding::Function)
}

fn is_response(p: Src<'_>, receiver: Node<'_>) -> bool {
    any_name(&chain_names(p, receiver), RESPONSES)
}

fn is_global(p: Src<'_>, receiver: Node<'_>) -> bool {
    chain_tail(p, receiver).is_some_and(|t| GLOBALS.contains(&t))
}

/// A `js-db-call` hit, weak when the query argument is constant text (#636).
fn db_hit(p: Src<'_>, from: Node<'_>, c: &Callee<'_>) -> Option<SinkHit> {
    // `createQueryBuilder('alias')` takes an entity alias as argument 0, not SQL text.
    let text_arg0 = p.node_text(&from) != "createQueryBuilder";
    let constant = text_arg0
        && args(c.call)
            .first()
            .is_some_and(|q| is_constant_query(p, from, *q));
    Some(SinkHit::new("js-db-call", "CWE-89").weak_if(constant))
}

/// True when `q` is a string literal (a template without `${}` counts) or an identifier whose
/// only binding is a declaration initialised with one and never reassigned.
fn is_constant_query(p: Src<'_>, from: Node<'_>, q: Node<'_>) -> bool {
    if literal_string(p, q).is_some() {
        return true;
    }
    if q.kind() != "identifier" {
        return false;
    }
    let name = p.node_text(&q);
    let Binding::Var { init: Some(i) } = resolve(p, from, name) else {
        return false;
    };
    if literal_string(p, i).is_none() || is_reassigned(p, from, name) {
        return false;
    }
    // Exactly one declarator of the name in scope: a later `var` redeclaration is a write.
    // Parameters count too: a parameter read before a later `var q = 'lit'` is the caller's.
    let mut declarators = 0;
    walk(binding_scope(from, i), &mut |n| match n.kind() {
        "variable_declarator" => {
            if let Some(target) = n.child_by_field_name("name") {
                declarators += count_idents(p, target, name);
            }
        }
        "formal_parameters" => declarators += count_idents(p, n, name),
        "arrow_function" => {
            if let Some(single) = n.child_by_field_name("parameter") {
                declarators += count_idents(p, single, name);
            }
        }
        _ => {}
    });
    declarators == 1
}

fn hit(rule: &'static str, cwe: &'static str) -> Option<SinkHit> {
    Some(SinkHit::new(rule, cwe))
}

pub(super) fn classify(p: Src<'_>, id: Node<'_>, text: &str) -> Option<SinkHit> {
    // A bare callee bound to a child_process function under any local name.
    if let Some(c) = callee_of(id) {
        if c.receiver.is_none() && !c.is_new && cp_function_of(p, id, 0).is_some() {
            return hit("js-child-process", "CWE-78");
        }
    }
    match text {
        "eval" => {
            let c = callee_of(id)?;
            if c.is_new || args(c.call).is_empty() {
                return None;
            }
            match c.receiver {
                None if is_local_helper(p, id, text) => None,
                Some(r) if any_name(&chain_names(p, r), LOGGERS) => None,
                _ => hit("js-eval", "CWE-95"),
            }
        }
        "Function" => {
            let c = callee_of(id)?;
            if args(c.call).is_empty() {
                return None;
            }
            hit("js-function-ctor", "CWE-95")
        }
        t if CP_FUNCTIONS.contains(&t) || EXECA.contains(&t) => {
            let c = callee_of(id)?;
            if c.is_new || args(c.call).is_empty() {
                return None;
            }
            let Some(r) = c.receiver else {
                return if is_local_helper(p, id, t) {
                    None
                } else {
                    hit("js-child-process", "CWE-78")
                };
            };
            if is_cp_expr(p, r, 0) {
                return hit("js-child-process", "CWE-78");
            }
            let names = chain_names(p, r);
            if is_regexish(p, r, 0) || any_name(&names, LOGGERS) {
                return None;
            }
            // better-sqlite3 / sqlite3 `db.exec(sql)`.
            if t == "exec"
                && names
                    .last()
                    .is_some_and(|n| DB_HANDLES.contains(&lower(n).as_str()))
            {
                return db_hit(p, id, &c);
            }
            hit("js-exec-call", "CWE-78")
        }
        t if SQL_NAMES.contains(&t) => {
            let c = callee_of(id)?;
            if c.is_new || (args(c.call).is_empty() && t != "createQueryBuilder") {
                return None;
            }
            match c.receiver {
                None if is_local_helper(p, id, t) => None,
                Some(r) => {
                    let names = chain_names(p, r);
                    let string_raw = t == "raw" && names.last() == Some(&"String");
                    if string_raw || any_name(&names, LOGGERS) {
                        None
                    } else {
                        db_hit(p, id, &c)
                    }
                }
                None => db_hit(p, id, &c),
            }
        }
        "redirect" => {
            let c = callee_of(id)?;
            if c.is_new || args(c.call).is_empty() {
                return None;
            }
            if c.receiver.is_none() && is_local_helper(p, id, text) {
                return None;
            }
            hit("js-redirect", "CWE-601")
        }
        "send" | "respond" => {
            let c = callee_of(id)?;
            if c.is_new || args(c.call).is_empty() {
                return None;
            }
            match c.receiver {
                None if is_local_helper(p, id, text) => None,
                Some(r) => {
                    let names = chain_names(p, r);
                    if any_name(&names, RESPONSES) {
                        hit("js-response", "CWE-79")
                    } else if any_name(&names, CHANNELS) || any_name(&names, LOGGERS) {
                        None
                    } else {
                        hit("js-response", "CWE-79")
                    }
                }
                None => hit("js-response", "CWE-79"),
            }
        }
        "write" | "writeln" => {
            let c = callee_of(id)?;
            if c.is_new || args(c.call).is_empty() {
                return None;
            }
            let Some(r) = c.receiver else {
                return if text == "write" && !is_local_helper(p, id, text) {
                    hit("js-write", "CWE-79")
                } else {
                    None
                };
            };
            let names = chain_names(p, r);
            if any_name(&names, &["document"]) {
                return hit("js-dom-html", "CWE-79");
            }
            if any_name(&names, RESPONSES) {
                return hit("js-response", "CWE-79");
            }
            let tail = names.last().map(|n| lower(n)).unwrap_or_default();
            if ["stream", "disk", "storage", "driver"]
                .iter()
                .any(|s| tail.ends_with(s))
            {
                return hit("js-storage-write", "CWE-22");
            }
            if any_name(&names, CHANNELS) || any_name(&names, LOGGERS) {
                return None;
            }
            hit("js-write", "CWE-79")
        }
        "writeFile" | "writeFileSync" | "appendFile" | "appendFileSync" | "unlink"
        | "unlinkSync" | "rmdir" | "rmdirSync" | "rmSync" => {
            let c = callee_of(id)?;
            (!c.is_new).then(|| SinkHit::new("js-fs", "CWE-22"))
        }
        // Names that are DOM or collection methods elsewhere: only on a file-system receiver.
        "remove" | "removeSync" | "emptyDir" | "emptyDirSync" | "rm" | "open" | "openSync" => {
            let c = callee_of(id)?;
            if c.is_new {
                return None;
            }
            let Some(r) = c.receiver else {
                return (text == "open" && !is_local_helper(p, id, text))
                    .then(|| SinkHit::new("js-fs", "CWE-22"));
            };
            let names = chain_names(p, r);
            if any_name(&names, FS_LIKE) {
                hit("js-fs", "CWE-22")
            } else if text == "open" && is_global(p, r) {
                hit("js-location", "CWE-601")
            } else {
                None
            }
        }
        "innerHTML" | "outerHTML" => {
            let member = id.parent().filter(|m| {
                m.kind() == "member_expression" && m.child_by_field_name("property") == Some(id)
            })?;
            is_assignment_target(member).then(|| SinkHit::new("js-dom-html", "CWE-79"))
        }
        "dangerouslySetInnerHTML" => hit("js-dom-html", "CWE-79"),
        "insertAdjacentHTML" => {
            let c = callee_of(id)?;
            (!c.is_new).then(|| SinkHit::new("js-dom-html", "CWE-79"))
        }
        "location" => location(p, id),
        "assign" | "replace" => {
            let c = callee_of(id)?;
            let r = c.receiver?;
            (chain_tail(p, r) == Some("location") && !c.is_new)
                .then(|| SinkHit::new("js-location", "CWE-601"))
        }
        "setHeader" | "header" | "set" | "append" | "writeHead" => {
            let c = callee_of(id)?;
            let r = c.receiver?;
            if c.is_new {
                return None;
            }
            // `setHeader` / `writeHead` are distinctive; `set` / `append` need a response.
            let distinctive = matches!(text, "setHeader" | "writeHead");
            if !distinctive && !is_response(p, r) {
                return None;
            }
            sets_location_header(p, text, c.call).then(|| SinkHit::new("js-location", "CWE-601"))
        }
        _ => None,
    }
}

fn is_assignment_target(node: Node<'_>) -> bool {
    node.parent().is_some_and(|a| {
        matches!(
            a.kind(),
            "assignment_expression" | "augmented_assignment_expression"
        ) && a.child_by_field_name("left") == Some(node)
    })
}

/// `location = x`, `window.location = x`, `location.href = x`, `res.location(x)`.
fn location(p: Src<'_>, id: Node<'_>) -> Option<SinkHit> {
    if let Some(c) = callee_of(id) {
        // `res.location(x)`
        return (!c.is_new && c.receiver.is_some_and(|r| is_response(p, r)))
            .then(|| SinkHit::new("js-location", "CWE-601"));
    }
    let parent = id.parent()?;
    // `window.location` (property of a global) or bare `location`.
    let mut node = if parent.kind() == "member_expression"
        && parent.child_by_field_name("property") == Some(id)
    {
        if !is_global(p, parent.child_by_field_name("object")?) {
            return None;
        }
        parent
    } else {
        id
    };
    // `location.href`
    if let Some(up) = node.parent() {
        if up.kind() == "member_expression"
            && up.child_by_field_name("object") == Some(node)
            && up.child_by_field_name("property").map(|n| p.node_text(&n)) == Some("href")
        {
            node = up;
        }
    }
    is_assignment_target(node).then(|| SinkHit::new("js-location", "CWE-601"))
}

/// `res.setHeader('Location', x)` and `res.writeHead(302, { Location: x })`.
fn sets_location_header(p: Src<'_>, method: &str, call: Node<'_>) -> bool {
    let list = args(call);
    if method == "writeHead" {
        return list.iter().any(|a| {
            a.kind() == "object" && {
                let mut cursor = a.walk();
                let found = a.named_children(&mut cursor).any(|pair| {
                    pair.kind() == "pair"
                        && pair.child_by_field_name("key").is_some_and(|k| {
                            let t = p.node_text(&k);
                            let t = literal_string(p, k).unwrap_or(t);
                            t.eq_ignore_ascii_case("location")
                        })
                });
                found
            }
        });
    }
    list.first()
        .and_then(|a| literal_string(p, *a))
        .is_some_and(|s| s.eq_ignore_ascii_case("location"))
}

/// Semantic check for the structured JS `exec`/`execSync` sinks, whose call path
/// matches by tail name on any receiver (`/a/.exec(input)` included): the call
/// must be a command run by the same rule the flat list uses (a database-handle
/// `db.exec(sql)` also passes, as before this change).
pub(crate) fn exec_call_is_command(call: &CallSite) -> bool {
    let node = call.call_node;
    let name = match node.child_by_field_name("function") {
        Some(f) if f.kind() == "member_expression" => f.child_by_field_name("property"),
        Some(f) if f.kind() == "identifier" => Some(f),
        // Other callee shapes keep the structured sink's earlier behaviour.
        _ => return true,
    };
    let Some(name) = name else { return true };
    let src = Src::new(call.source, Language::JavaScript);
    classify(src, name, src.node_text(&name)).is_some()
}
