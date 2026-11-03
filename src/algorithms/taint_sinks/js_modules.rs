//! Import-resolved module identity for JS/TS/TSX review sinks (#637 Slice A1).
//!
//! Three closed tables, positive evidence only. A call is a hit when its callee resolves, through
//! the lexically correct sole-binding lookup (`js_binding`), to a listed module: an import or
//! `require` of the module under any local name, a named or destructured import (renames
//! included), or one hop through a single-binding local (an alias `const f = fs`,
//! `const ssh = new NodeSSH()`). A shadowing parameter, loop, catch or hoisted `var`, a rebinding,
//! a second binding or a type-only import proves nothing, and fields are not followed. A hit fires
//! whatever the name-based rules said; no hit leaves the name-based outcome untouched.
//!
//! * the fs family (`fs`, `fs/promises`, `fs-extra`, `graceful-fs`, `node:` forms, `.promises`)
//!   and its path-taking API: `js-fs`, CWE-22. It fires only when a path-position argument is not a
//!   string literal, so tainted data written to a fixed path is not reported;
//! * `node-ssh`: `execCommand` / `exec` on a `new NodeSSH()` local: `js-exec-call`, CWE-78;
//! * `document.execCommand(cmd, ..)` on the unshadowed global document, firing for a literal
//!   `insertHTML` (any case) or a non-literal command: `js-dom-html`, CWE-79.

use super::callee::{args, callee_of, chain_names, literal_string, peel, Src};
use super::js_binding::{is_type_only, lookup, sole, Lookup};
use super::js_scope::Binding;
use super::SinkHit;
use tree_sitter::Node;

const FS_MODULES: &[&str] = &[
    "fs",
    "node:fs",
    "fs/promises",
    "node:fs/promises",
    "fs-extra",
    "graceful-fs",
];
const NODE_SSH: &[&str] = &["node-ssh"];

/// Path-taking `fs` / `fs/promises` API (async and `*Sync`) plus the `fs-extra` additions.
const FS_API: &[&str] = &[
    "access",
    "accessSync",
    "appendFile",
    "appendFileSync",
    "chmod",
    "chmodSync",
    "chown",
    "chownSync",
    "copyFile",
    "copyFileSync",
    "cp",
    "cpSync",
    "createReadStream",
    "createWriteStream",
    "exists",
    "existsSync",
    "glob",
    "globSync",
    "lchmod",
    "lchmodSync",
    "lchown",
    "lchownSync",
    "link",
    "linkSync",
    "lstat",
    "lstatSync",
    "lutimes",
    "lutimesSync",
    "mkdir",
    "mkdirSync",
    "mkdtemp",
    "mkdtempSync",
    "open",
    "openAsBlob",
    "openSync",
    "opendir",
    "opendirSync",
    "readFile",
    "readFileSync",
    "readdir",
    "readdirSync",
    "readlink",
    "readlinkSync",
    "realpath",
    "realpathSync",
    "rename",
    "renameSync",
    "rm",
    "rmSync",
    "rmdir",
    "rmdirSync",
    "stat",
    "statSync",
    "statfs",
    "statfsSync",
    "symlink",
    "symlinkSync",
    "truncate",
    "truncateSync",
    "unlink",
    "unlinkSync",
    "utimes",
    "utimesSync",
    "writeFile",
    "writeFileSync",
    // fs-extra additions
    "copy",
    "copySync",
    "emptyDir",
    "emptyDirSync",
    "emptydir",
    "emptydirSync",
    "ensureDir",
    "ensureDirSync",
    "ensureFile",
    "ensureFileSync",
    "ensureLink",
    "ensureLinkSync",
    "ensureSymlink",
    "ensureSymlinkSync",
    "mkdirp",
    "mkdirpSync",
    "mkdirs",
    "mkdirsSync",
    "move",
    "moveSync",
    "outputFile",
    "outputFileSync",
    "outputJSON",
    "outputJSONSync",
    "outputJson",
    "outputJsonSync",
    "pathExists",
    "pathExistsSync",
    "readJSON",
    "readJSONSync",
    "readJson",
    "readJsonSync",
    "remove",
    "removeSync",
    "writeJSON",
    "writeJSONSync",
    "writeJson",
    "writeJsonSync",
];
const SSH_EXEC: &[&str] = &["execCommand", "exec"];
/// APIs whose first two arguments are both paths.
const TWO_PATH_API: &[&str] = &[
    "copy",
    "copySync",
    "copyFile",
    "copyFileSync",
    "cp",
    "cpSync",
    "move",
    "moveSync",
    "rename",
    "renameSync",
    "link",
    "linkSync",
    "symlink",
    "symlinkSync",
    "ensureLink",
    "ensureLinkSync",
    "ensureSymlink",
    "ensureSymlinkSync",
    "createSymlink",
];
const MAX_DEPTH: usize = 4;

pub(super) fn classify(p: Src<'_>, id: Node<'_>, text: &str) -> Option<SinkHit> {
    // A bare callee may carry any local name (`import { mkdir as mk }`); a member needs an API name.
    if let Some(hit) = fs_hit(p, id, text) {
        return Some(hit);
    }
    match text {
        "execCommand" => document_exec(p, id).or_else(|| ssh_exec(p, id)),
        "exec" => ssh_exec(p, id),
        _ => None,
    }
}

/// The module specifier a `require('m')` / `import('m')` expression loads (an unshadowed `require`).
fn load_of<'p>(p: Src<'p>, node: Node<'_>) -> Option<&'p str> {
    let n = peel(node);
    if n.kind() != "call_expression" {
        return None;
    }
    let f = n.child_by_field_name("function")?;
    let loads = f.kind() == "import"
        || (f.kind() == "identifier"
            && p.node_text(&f) == "require"
            && matches!(lookup(p, f, "require"), Lookup::Unbound));
    if !loads {
        return None;
    }
    literal_string(p, *args(n).first()?)
}

fn in_mods(p: Src<'_>, module: Node<'_>, mods: &[&str]) -> bool {
    literal_string(p, module).is_some_and(|m| mods.contains(&m))
}

/// A single-binding, never-written local initialised with an expression.
fn one_hop<'t>(p: Src<'_>, id: Node<'t>, name: &str) -> Option<Node<'t>> {
    match sole(p, id, name)? {
        Binding::Var { init: Some(i) } => Some(i),
        _ => None,
    }
}

/// True when `node` evaluates to a module of `mods` (or, for the fs family, its `promises`).
fn is_module(p: Src<'_>, node: Node<'_>, mods: &[&str], depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    let n = peel(node);
    match n.kind() {
        "call_expression" => load_of(p, n).is_some_and(|m| mods.contains(&m)),
        "identifier" => match sole(p, n, p.node_text(&n)) {
            Some(Binding::Import {
                module,
                imported: None,
            }) => !is_type_only(module, None) && in_mods(p, module, mods),
            // `import { promises as fsp } from 'fs'`
            Some(Binding::Import {
                module,
                imported: Some(orig),
            }) => {
                p.node_text(&orig) == "promises"
                    && !is_type_only(module, Some(orig))
                    && in_mods(p, module, mods)
            }
            // `const { promises: fsp } = require('fs')`
            Some(Binding::Destructured { init, key }) => {
                key.is_some_and(|k| p.node_text(&k) == "promises")
                    && is_module(p, init, mods, depth + 1)
            }
            Some(Binding::Var { init: Some(i) }) => is_module(p, i, mods, depth + 1),
            _ => false,
        },
        // `fs.promises`
        "member_expression" => {
            n.child_by_field_name("property")
                .is_some_and(|prop| p.node_text(&prop) == "promises")
                && n.child_by_field_name("object")
                    .is_some_and(|o| is_module(p, o, mods, depth + 1))
        }
        _ => false,
    }
}

fn fs_hit(p: Src<'_>, id: Node<'_>, text: &str) -> Option<SinkHit> {
    let c = callee_of(p, id)?;
    if c.is_new {
        return None;
    }
    let api = match c.receiver {
        Some(r) => (FS_API.contains(&text) && is_module(p, r, FS_MODULES, 0)).then_some(text),
        None => bare_fs_function(p, id, text),
    }?;
    path_is_not_literal(p, c.call, api).then(|| SinkHit::new("js-fs", "CWE-22"))
}

/// A path-position argument is not a string literal (a fixed path with tainted data is no path flow).
fn path_is_not_literal(p: Src<'_>, call: Node<'_>, api: &str) -> bool {
    let n = if TWO_PATH_API.contains(&api) { 2 } else { 1 };
    args(call)
        .iter()
        .take(n)
        .any(|a| literal_string(p, *a).is_none())
}

/// The fs API name a bare callee is bound to: `import { mkdir as mk } from 'fs/promises'`,
/// `const { remove } = require('fs-extra')`, `const rm = require('fs').rm`.
fn bare_fs_function<'p>(p: Src<'p>, id: Node<'_>, local: &str) -> Option<&'p str> {
    let api = match sole(p, id, local)? {
        Binding::Import {
            module,
            imported: Some(orig),
        } => (!is_type_only(module, Some(orig)) && in_mods(p, module, FS_MODULES))
            .then(|| p.node_text(&orig))?,
        Binding::Destructured { init, key } => {
            if !is_module(p, init, FS_MODULES, 0) {
                return None;
            }
            p.node_text(&key?)
        }
        Binding::Var { init: Some(i) } => {
            let i = peel(i);
            if i.kind() != "member_expression"
                || !i
                    .child_by_field_name("object")
                    .is_some_and(|o| is_module(p, o, FS_MODULES, 0))
            {
                return None;
            }
            p.node_text(&i.child_by_field_name("property")?)
        }
        _ => return None,
    };
    FS_API.contains(&api).then_some(api)
}

/// `ssh.execCommand(..)` / `ssh.exec(..)` where `ssh` is a single-binding `new NodeSSH()` local.
fn ssh_exec(p: Src<'_>, id: Node<'_>) -> Option<SinkHit> {
    let c = callee_of(p, id)?;
    let r = peel(c.receiver?);
    if c.is_new || r.kind() != "identifier" || !SSH_EXEC.contains(&p.node_text(&id)) {
        return None;
    }
    let init = peel(one_hop(p, r, p.node_text(&r))?);
    if init.kind() != "new_expression" {
        return None;
    }
    let ctor = peel(init.child_by_field_name("constructor")?);
    is_node_ssh_class(p, ctor, 0).then(|| SinkHit::new("js-exec-call", "CWE-78"))
}

/// True when `node` is the `NodeSSH` class of `node-ssh`: a named import or destructured require
/// (renames included), `m.NodeSSH`, a local initialised with one of those or with the module
/// itself (the legacy default export).
fn is_node_ssh_class(p: Src<'_>, node: Node<'_>, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    let member_of_module = |n: Node<'_>| {
        n.kind() == "member_expression"
            && n.child_by_field_name("property")
                .is_some_and(|prop| p.node_text(&prop) == "NodeSSH")
            && n.child_by_field_name("object")
                .is_some_and(|o| is_module(p, o, NODE_SSH, 0))
    };
    let n = peel(node);
    match n.kind() {
        "identifier" => match sole(p, n, p.node_text(&n)) {
            Some(Binding::Import { module, imported }) => {
                imported.is_none_or(|o| p.node_text(&o) == "NodeSSH")
                    && !is_type_only(module, imported)
                    && in_mods(p, module, NODE_SSH)
            }
            Some(Binding::Destructured { init, key }) => {
                key.is_some_and(|k| p.node_text(&k) == "NodeSSH") && is_module(p, init, NODE_SSH, 0)
            }
            Some(Binding::Var { init: Some(i) }) => {
                let i = peel(i);
                load_of(p, i).is_some_and(|m| NODE_SSH.contains(&m))
                    || member_of_module(i)
                    || (i.kind() == "identifier" && is_node_ssh_class(p, i, depth + 1))
            }
            _ => false,
        },
        _ => member_of_module(n),
    }
}

/// `document.execCommand(cmd, ..)` on the unshadowed global document.
fn document_exec(p: Src<'_>, id: Node<'_>) -> Option<SinkHit> {
    let c = callee_of(p, id)?;
    let r = c.receiver?;
    if c.is_new {
        return None;
    }
    let names = chain_names(p, r);
    if !matches!(
        names.as_slice(),
        ["document"] | ["window" | "globalThis", "document"]
    ) {
        return None;
    }
    // The chain root must be the global, not a parameter, local or import of that name.
    let mut root = peel(r);
    while root.kind() == "member_expression" {
        root = peel(root.child_by_field_name("object")?);
    }
    if root.kind() != "identifier"
        || !matches!(lookup(p, root, p.node_text(&root)), Lookup::Unbound)
    {
        return None;
    }
    let cmd = *args(c.call).first()?;
    let fires = literal_string(p, cmd).is_none_or(|s| s.eq_ignore_ascii_case("insertHTML"));
    fires.then(|| SinkHit::new("js-dom-html", "CWE-79"))
}
