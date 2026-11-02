//! JS/TS/TSX sink rules (#622 S1): call position and receiver, never a name fragment.
//!
//! Every rule starts from the identifier under test and asks whether it is the
//! callee of a call (or the target of an assignment for DOM/location sinks) and,
//! where the name alone is ambiguous, what the receiver is. A receiver that was
//! bound to `child_process` is found by reading the file's own imports.

use super::callee::{args, callee_of, chain_tail, literal_string, walk, Src};
use super::SinkHit;
use crate::frameworks::CallSite;
use crate::languages::Language;
use std::collections::BTreeMap;
use tree_sitter::Node;

const CP_MODULES: &[&str] = &["child_process", "node:child_process", "shelljs"];
/// Bare calls to these names are command runs even when the import is elsewhere.
const BARE_UNBOUND: &[&str] = &["exec", "execSync", "execFile", "execFileSync", "spawnSync"];
const CP_FUNCTIONS: &[&str] = &[
    "exec",
    "execSync",
    "execFile",
    "execFileSync",
    "spawn",
    "spawnSync",
    "fork",
];
/// Receivers that name the global object.
const GLOBALS: &[&str] = &["window", "globalThis", "self", "global", "top", "parent"];
/// Receivers that name an HTTP response object.
const RESPONSES: &[&str] = &["res", "response", "reply", "resp"];
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

/// Names a file binds to `child_process` or to one of its functions.
#[derive(Default)]
pub(super) struct Facts {
    /// Local names bound to the whole module (`const cp = require(..)`).
    namespaces: Vec<String>,
    /// Local name -> `child_process` function it is bound to.
    functions: BTreeMap<String, String>,
}

impl Facts {
    pub(super) fn collect(p: Src<'_>, root: Node<'_>) -> Self {
        let mut facts = Facts::default();
        let mut promisified: Vec<(String, String)> = Vec::new();
        walk(root, &mut |node| match node.kind() {
            "variable_declarator" => facts.declarator(p, node, &mut promisified),
            "import_statement" => facts.import(p, node),
            _ => {}
        });
        for (local, wrapped) in promisified {
            let orig = match wrapped.rsplit_once('.') {
                // `promisify(cp.exec)`
                Some((ns, name)) => (facts.is_namespace(ns) && CP_FUNCTIONS.contains(&name))
                    .then(|| name.to_string()),
                // `promisify(exec)`
                None => facts.functions.get(&wrapped).cloned(),
            };
            if let Some(orig) = orig {
                facts.functions.insert(local, orig);
            }
        }
        facts
    }

    fn declarator(&mut self, p: Src<'_>, node: Node<'_>, promisified: &mut Vec<(String, String)>) {
        let (Some(name), Some(value)) = (
            node.child_by_field_name("name"),
            node.child_by_field_name("value"),
        ) else {
            return;
        };
        let mut value = value;
        while value.kind() == "await_expression" {
            let Some(inner) = value.named_child(0) else {
                return;
            };
            value = inner;
        }
        if is_require_cp(p, value) {
            match name.kind() {
                "identifier" => self.namespaces.push(p.node_text(&name).to_string()),
                "object_pattern" => self.destructured(p, name),
                _ => {}
            }
            return;
        }
        if name.kind() != "identifier" {
            return;
        }
        let local = p.node_text(&name).to_string();
        // `const exec = require('child_process').exec`
        if value.kind() == "member_expression" {
            let (Some(obj), Some(prop)) = (
                value.child_by_field_name("object"),
                value.child_by_field_name("property"),
            ) else {
                return;
            };
            let prop = p.node_text(&prop);
            if is_require_cp(p, obj) && CP_FUNCTIONS.contains(&prop) {
                self.functions.insert(local, prop.to_string());
            }
            return;
        }
        // `const run = promisify(exec)` / `util.promisify(cp.exec)`
        if value.kind() == "call_expression" {
            let callee = value.child_by_field_name("function");
            let is_promisify = callee.is_some_and(|c| {
                let t = p.node_text(&c);
                t == "promisify" || t.ends_with(".promisify")
            });
            let first = args(value).into_iter().next();
            if let (true, Some(arg)) = (is_promisify, first) {
                // Resolved after every binding is known, in `collect`.
                if matches!(arg.kind(), "identifier" | "member_expression") {
                    promisified.push((local, p.node_text(&arg).to_string()));
                }
            }
        }
    }

    fn destructured(&mut self, p: Src<'_>, pattern: Node<'_>) {
        let mut cursor = pattern.walk();
        for child in pattern.named_children(&mut cursor) {
            let (local, orig) = match child.kind() {
                "shorthand_property_identifier_pattern" => {
                    let t = p.node_text(&child);
                    (t, t)
                }
                "pair_pattern" => {
                    let (Some(key), Some(value)) = (
                        child.child_by_field_name("key"),
                        child.child_by_field_name("value"),
                    ) else {
                        continue;
                    };
                    if value.kind() != "identifier" {
                        continue;
                    }
                    (p.node_text(&value), p.node_text(&key))
                }
                _ => continue,
            };
            if CP_FUNCTIONS.contains(&orig) {
                self.functions.insert(local.to_string(), orig.to_string());
            }
        }
    }

    fn import(&mut self, p: Src<'_>, node: Node<'_>) {
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.named_children(&mut cursor).collect();
        let from_cp = children.iter().any(|c| {
            c.kind() == "string" && literal_string(p, *c).is_some_and(|s| CP_MODULES.contains(&s))
        }) || children.iter().any(|c| {
            c.kind() == "import_require_clause"
                && c.child_by_field_name("source")
                    .or_else(|| c.named_child(1))
                    .and_then(|s| literal_string(p, s))
                    .is_some_and(|s| CP_MODULES.contains(&s))
        });
        if !from_cp {
            return;
        }
        for child in children {
            match child.kind() {
                // `import cp = require('child_process')`
                "import_require_clause" => {
                    if let Some(id) = child.named_child(0) {
                        self.namespaces.push(p.node_text(&id).to_string());
                    }
                }
                "import_clause" => {
                    let mut inner = child.walk();
                    for part in child.named_children(&mut inner) {
                        match part.kind() {
                            "identifier" => self.namespaces.push(p.node_text(&part).to_string()),
                            "namespace_import" => {
                                if let Some(id) = part.named_child(0) {
                                    self.namespaces.push(p.node_text(&id).to_string());
                                }
                            }
                            "named_imports" => self.named_imports(p, part),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn named_imports(&mut self, p: Src<'_>, imports: Node<'_>) {
        let mut cursor = imports.walk();
        for spec in imports.named_children(&mut cursor) {
            if spec.kind() != "import_specifier" {
                continue;
            }
            let Some(name) = spec.child_by_field_name("name") else {
                continue;
            };
            let orig = p.node_text(&name);
            let local = spec
                .child_by_field_name("alias")
                .map(|a| p.node_text(&a))
                .unwrap_or(orig);
            if CP_FUNCTIONS.contains(&orig) {
                self.functions.insert(local.to_string(), orig.to_string());
            }
        }
    }

    fn is_namespace(&self, name: &str) -> bool {
        name == "child_process" || self.namespaces.iter().any(|n| n == name)
    }
}

/// `require('child_process')` / `require('node:child_process')`.
fn is_require_cp(p: Src<'_>, node: Node<'_>) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let is_require = node
        .child_by_field_name("function")
        .is_some_and(|f| p.node_text(&f) == "require");
    is_require
        && args(node)
            .first()
            .and_then(|a| literal_string(p, *a))
            .is_some_and(|s| CP_MODULES.contains(&s))
}

fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

/// Receiver tail names a response object (`res`, `ctx.response`, ...).
fn is_response(p: Src<'_>, receiver: Node<'_>) -> bool {
    chain_tail(p, receiver).is_some_and(|t| RESPONSES.contains(&lower(t).as_str()))
}

fn is_global(p: Src<'_>, receiver: Node<'_>) -> bool {
    chain_tail(p, receiver).is_some_and(|t| GLOBALS.contains(&t))
}

fn receiver_is_cp(p: Src<'_>, facts: &Facts, receiver: Node<'_>) -> bool {
    if is_require_cp(p, receiver) {
        return true;
    }
    receiver.kind() == "identifier" && facts.is_namespace(p.node_text(&receiver))
}

pub(super) fn classify(p: Src<'_>, facts: &Facts, id: Node<'_>, text: &str) -> Option<SinkHit> {
    // A bare callee bound to a child_process function under any local name.
    if let Some(orig) = facts.functions.get(text) {
        if let Some(c) = callee_of(id) {
            if c.receiver.is_none() && !c.is_new && CP_FUNCTIONS.contains(&orig.as_str()) {
                return Some(SinkHit::new("js-child-process", "CWE-78"));
            }
        }
    }
    match text {
        "eval" => {
            let c = callee_of(id)?;
            let ok = !c.is_new && c.receiver.is_none_or(|r| is_global(p, r));
            ok.then(|| SinkHit::new("js-eval", "CWE-95"))
        }
        "Function" => {
            let c = callee_of(id)?;
            c.receiver
                .is_none_or(|r| is_global(p, r))
                .then(|| SinkHit::new("js-function-ctor", "CWE-95"))
        }
        t if CP_FUNCTIONS.contains(&t) => {
            let c = callee_of(id)?;
            if c.is_new {
                return None;
            }
            let Some(r) = c.receiver else {
                return BARE_UNBOUND
                    .contains(&t)
                    .then(|| SinkHit::new("js-child-process", "CWE-78"));
            };
            if receiver_is_cp(p, facts, r) {
                return Some(SinkHit::new("js-child-process", "CWE-78"));
            }
            // better-sqlite3 / sqlite3 `db.exec(sql)`.
            if t == "exec"
                && chain_tail(p, r).is_some_and(|n| DB_HANDLES.contains(&lower(n).as_str()))
            {
                return Some(SinkHit::new("js-db-call", "CWE-89"));
            }
            None
        }
        "query" | "execute" | "raw" | "queryRaw" | "$queryRaw" | "$queryRawUnsafe"
        | "$executeRaw" | "$executeRawUnsafe" | "whereRaw" | "havingRaw" | "orderByRaw"
        | "literal" => {
            let c = callee_of(id)?;
            if c.is_new {
                return None;
            }
            // `String.raw` is a tag helper, not SQL.
            if text == "raw" && c.receiver.and_then(|r| chain_tail(p, r)) == Some("String") {
                return None;
            }
            Some(SinkHit::new("js-db-call", "CWE-89"))
        }
        "redirect" => {
            let c = callee_of(id)?;
            (!c.is_new).then(|| SinkHit::new("js-redirect", "CWE-601"))
        }
        "send" | "respond" => {
            let c = callee_of(id)?;
            let ok = !c.is_new
                && c.receiver
                    .is_some_and(|r| is_response(p, r) || text == "respond");
            ok.then(|| SinkHit::new("js-response", "CWE-79"))
        }
        "write" => {
            let c = callee_of(id)?;
            let r = c.receiver?;
            if is_response(p, r) {
                return Some(SinkHit::new("js-response", "CWE-79"));
            }
            let tail = lower(chain_tail(p, r)?);
            ["stream", "disk", "storage", "driver"]
                .iter()
                .any(|s| tail.ends_with(s))
                .then(|| SinkHit::new("js-storage-write", "CWE-22"))
        }
        "writeFile" | "writeFileSync" | "appendFile" | "appendFileSync" | "unlink"
        | "unlinkSync" | "rmdir" | "rmdirSync" | "rmSync" => {
            let c = callee_of(id)?;
            (!c.is_new).then(|| SinkHit::new("js-fs", "CWE-22"))
        }
        "innerHTML" | "outerHTML" => {
            let member = id.parent().filter(|m| {
                m.kind() == "member_expression" && m.child_by_field_name("property") == Some(id)
            })?;
            is_assignment_target(member).then(|| SinkHit::new("js-dom-html", "CWE-79"))
        }
        "dangerouslySetInnerHTML" => Some(SinkHit::new("js-dom-html", "CWE-79")),
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
            if c.is_new || !is_response(p, r) {
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
/// matches by tail name on any receiver (`re.exec(input)` included): the call
/// must be a command run by the same rule the flat list uses.
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
    let mut root = node;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    let facts = Facts::collect(src, root);
    classify(src, &facts, name, src.node_text(&name)).is_some()
}
