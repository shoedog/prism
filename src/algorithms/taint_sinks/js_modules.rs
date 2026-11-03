//! Import-resolved module identity for JS/TS/TSX review sinks (#637 Slice A1).
//!
//! Three closed tables, positive evidence only. A call is a hit when its callee resolves, through
//! a lexical binding (`js_scope`), to a listed module: an import or `require` of the module under
//! any local name, a named or destructured import (renames included), or one hop through a
//! single-binding local (an alias `const f = fs`, `const ssh = new NodeSSH()`). A shadowing
//! parameter, a rebinding or a second binding proves nothing, and fields are not followed. A hit
//! fires whatever the name-based rules said; no hit leaves the name-based outcome untouched.
//!
//! * the fs family (`fs`, `fs/promises`, `fs-extra`, `graceful-fs`, `node:` forms, `.promises`)
//!   and its path-taking API: `js-fs`, CWE-22;
//! * `node-ssh`: `execCommand` / `exec` on a `new NodeSSH()` local: `js-exec-call`, CWE-78;
//! * `document.execCommand(cmd, ..)` on the global document, firing for a literal `'insertHTML'`
//!   or a non-literal command: `js-dom-html`, CWE-79.

use super::callee::{args, callee_of, chain_names, literal_string, peel, Src};
use super::js_scope::{declarator_count, is_reassigned, resolve, Binding};
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
            && matches!(resolve(p, f, "require"), Binding::Unresolved));
    if !loads {
        return None;
    }
    literal_string(p, *args(n).first()?)
}

/// A single-binding, never-reassigned local initialised with `init`.
fn one_hop<'t>(p: Src<'_>, id: Node<'t>, name: &str) -> Option<Node<'t>> {
    match resolve(p, id, name) {
        Binding::Var { init: Some(i) }
            if !is_reassigned(p, id, name) && declarator_count(p, id, i, name) == 1 =>
        {
            Some(i)
        }
        _ => None,
    }
}

/// True when `node` evaluates to a module of `mods` (or, with `promises`, its promise namespace).
fn is_module(p: Src<'_>, node: Node<'_>, mods: &[&str], depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    let n = peel(node);
    match n.kind() {
        "call_expression" => load_of(p, n).is_some_and(|m| mods.contains(&m)),
        "identifier" => {
            let name = p.node_text(&n);
            match resolve(p, n, name) {
                Binding::Import {
                    module,
                    imported: None,
                } => literal_string(p, module).is_some_and(|m| mods.contains(&m)),
                // `import { promises as fsp } from 'fs'`
                Binding::Import {
                    module,
                    imported: Some(orig),
                } => {
                    p.node_text(&orig) == "promises"
                        && literal_string(p, module).is_some_and(|m| mods.contains(&m))
                }
                // `const { promises: fsp } = require('fs')`
                Binding::Destructured { init, key } => {
                    key.is_some_and(|k| p.node_text(&k) == "promises")
                        && !is_reassigned(p, n, name)
                        && is_module(p, init, mods, depth + 1)
                }
                _ => one_hop(p, n, name).is_some_and(|i| is_module(p, i, mods, depth + 1)),
            }
        }
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
    let proven = match c.receiver {
        Some(r) => FS_API.contains(&text) && is_module(p, r, FS_MODULES, 0),
        None => bare_fs_function(p, id, text),
    };
    proven.then(|| SinkHit::new("js-fs", "CWE-22"))
}

/// A bare callee bound to an fs API function: `import { rm as r } from 'fs/promises'`,
/// `const { remove } = require('fs-extra')`, `const rm = require('fs').rm`.
fn bare_fs_function(p: Src<'_>, id: Node<'_>, local: &str) -> bool {
    match resolve(p, id, local) {
        Binding::Import {
            module,
            imported: Some(orig),
        } => {
            FS_API.contains(&p.node_text(&orig))
                && literal_string(p, module).is_some_and(|m| FS_MODULES.contains(&m))
        }
        Binding::Destructured { init, key } => {
            key.is_some_and(|k| FS_API.contains(&p.node_text(&k)))
                && !is_reassigned(p, id, local)
                && is_module(p, init, FS_MODULES, 0)
        }
        _ => one_hop(p, id, local).is_some_and(|i| {
            let i = peel(i);
            i.kind() == "member_expression"
                && i.child_by_field_name("property")
                    .is_some_and(|prop| FS_API.contains(&p.node_text(&prop)))
                && i.child_by_field_name("object")
                    .is_some_and(|o| is_module(p, o, FS_MODULES, 0))
        }),
    }
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
    let is_class = match ctor.kind() {
        "identifier" => {
            let name = p.node_text(&ctor);
            match resolve(p, ctor, name) {
                Binding::Import {
                    module,
                    imported: Some(orig),
                } => {
                    p.node_text(&orig) == "NodeSSH"
                        && literal_string(p, module).is_some_and(|m| NODE_SSH.contains(&m))
                }
                Binding::Destructured { init, key } => {
                    key.is_some_and(|k| p.node_text(&k) == "NodeSSH")
                        && !is_reassigned(p, ctor, name)
                        && declarator_count(p, ctor, init, name) == 1
                        && load_of(p, init).is_some_and(|m| NODE_SSH.contains(&m))
                }
                _ => false,
            }
        }
        // `new ssh.NodeSSH()` through a namespace import
        "member_expression" => {
            ctor.child_by_field_name("property")
                .is_some_and(|prop| p.node_text(&prop) == "NodeSSH")
                && ctor
                    .child_by_field_name("object")
                    .is_some_and(|o| is_module(p, o, NODE_SSH, 0))
        }
        _ => false,
    };
    is_class.then(|| SinkHit::new("js-exec-call", "CWE-78"))
}

/// `document.execCommand(cmd, ..)` on the unshadowed global document.
fn document_exec(p: Src<'_>, id: Node<'_>) -> Option<SinkHit> {
    let c = callee_of(p, id)?;
    let r = c.receiver?;
    if c.is_new {
        return None;
    }
    let names = chain_names(p, r);
    let global_document = match names.as_slice() {
        ["document"] => true,
        ["window" | "globalThis", "document"] => true,
        _ => false,
    };
    if !global_document {
        return None;
    }
    // The chain root must be the global, not a parameter, local or import of that name.
    let mut root = peel(r);
    while root.kind() == "member_expression" {
        root = peel(root.child_by_field_name("object")?);
    }
    if root.kind() != "identifier"
        || !matches!(resolve(p, root, p.node_text(&root)), Binding::Unresolved)
    {
        return None;
    }
    let cmd = *args(c.call).first()?;
    let fires = literal_string(p, cmd).is_none_or(|s| s == "insertHTML");
    fires.then(|| SinkHit::new("js-dom-html", "CWE-79"))
}
