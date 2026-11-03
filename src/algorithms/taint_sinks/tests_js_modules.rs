//! Module-identity gates for JS/TS (#637 A1), read straight off the classifier: each test pins one
//! family of gates so a mutant of the gate is killed by a named test.

use super::tests::{hit, js_hit, tag, FS_CWE};
use super::*;

fn ts_hit(source: &str, name: &str) -> Option<(&'static str, &'static str)> {
    let line = source.lines().position(|l| l.contains("// SINK")).unwrap() + 1;
    tag(hit(Language::TypeScript, source, line, name))
}

#[test]
fn js_binding_scopes_shadow_the_module() {
    let cases = [
        "for (const f of s) {\n  f.mkdir(x); // SINK\n}\n",
        "try { r(); } catch (f) {\n  f.mkdir(x); // SINK\n}\n",
        "function h({ f } = {}) {\n  f.mkdir(x); // SINK\n}\n",
        "function h({ f = z } = {}) {\n  f.mkdir(x); // SINK\n}\n",
        "function h(c) {\n  if (c) { var f = z; }\n  f.mkdir(x); // SINK\n}\n",
        "function h(k) {\n  switch (k) {\n    case 1:\n      const f = z;\n      f.mkdir(x); // SINK\n  }\n}\n",
        "for (let f = z; f; f = 0) {\n  f.mkdir(x); // SINK\n}\n",
        "function h(o) {\n  for (var f in o) {}\n  f.mkdir(x); // SINK\n}\n",
    ];
    for body in cases {
        let src = format!("import * as f from 'fs';\n{body}");
        assert_eq!(js_hit(&src, "mkdir"), None, "{body}");
    }
    // the same bodies without the shadowing binding keep the module identity
    let ok = "import * as f from 'fs';\nfunction h() {\n  f.mkdir(x); // SINK\n}\n";
    assert_eq!(js_hit(ok, "mkdir"), FS_CWE);
}

#[test]
fn js_binding_sole_binding_and_writes() {
    let twice = "var { mkdir: mk } = require('fs');\nvar mk = z;\nmk(x); // SINK\n";
    assert_eq!(js_hit(twice, "mk"), None);
    let once = "var { mkdir: mk } = require('fs');\nmk(x); // SINK\n";
    assert_eq!(js_hit(once, "mk"), FS_CWE);
    let inc = "let f = require('fs');\nf++;\nf.mkdir(x); // SINK\n";
    assert_eq!(js_hit(inc, "mkdir"), None);
    let destructuring_write = "let f = require('fs');\n[f] = [z];\nf.mkdir(x); // SINK\n";
    assert_eq!(js_hit(destructuring_write, "mkdir"), None);
    let for_of_write = "let f = require('fs');\nfor (f of z) {}\nf.mkdir(x); // SINK\n";
    assert_eq!(js_hit(for_of_write, "mkdir"), None);
    let compound = "let f = require('fs');\nf ||= z;\nf.mkdir(x); // SINK\n";
    assert_eq!(js_hit(compound, "mkdir"), None);
    // a nested scope's own same-named local, written there, does not touch the module binding
    let nested =
        "const f = require('fs');\nfunction o() { let f = 1; f = 2; }\nf.mkdir(x); // SINK\n";
    assert_eq!(js_hit(nested, "mkdir"), FS_CWE);
    let unrelated_param = "const f = require('fs');\nfunction o(f) {}\nf.mkdir(x); // SINK\n";
    assert_eq!(js_hit(unrelated_param, "mkdir"), FS_CWE);
    let promises_twice = "var { promises: p } = require('fs');\nvar p = z;\np.mkdir(x); // SINK\n";
    assert_eq!(js_hit(promises_twice, "mkdir"), None);
}

#[test]
fn js_binding_type_only_imports() {
    for src in [
        "import type * as f from 'fs';\nf.mkdir(x); // SINK\n",
        "import type f from 'fs-extra';\nf.remove(x); // SINK\n",
    ] {
        assert_eq!(
            ts_hit(
                src,
                if src.contains("mkdir") {
                    "mkdir"
                } else {
                    "remove"
                }
            ),
            None
        );
    }
    assert_eq!(
        ts_hit(
            "import type { mkdir } from 'fs';\nmkdir(x); // SINK\n",
            "mkdir"
        ),
        None
    );
    assert_eq!(
        ts_hit(
            "import { type mkdir } from 'fs';\nmkdir(x); // SINK\n",
            "mkdir"
        ),
        None
    );
    assert_eq!(
        ts_hit("import type { NodeSSH } from 'node-ssh';\nconst s = new NodeSSH();\ns.execCommand(x); // SINK\n", "execCommand"),
        None
    );
    assert_eq!(
        ts_hit("import * as f from 'fs';\nf.mkdir(x); // SINK\n", "mkdir"),
        FS_CWE
    );
    assert_eq!(
        ts_hit(
            "import { type Stats, mkdir } from 'fs';\nmkdir(x); // SINK\n",
            "mkdir"
        ),
        FS_CWE
    );
}

#[test]
fn js_module_promises_branches() {
    let named = "import { promises as p } from 'fs';\np.mkdir(x); // SINK\n";
    assert_eq!(js_hit(named, "mkdir"), FS_CWE);
    let not_promises = "import { mkdir as m } from 'fs';\nm.open(x); // SINK\n";
    assert_eq!(js_hit(not_promises, "open"), None);
    let destructured = "const { promises: p } = require('fs');\np.mkdir(x); // SINK\n";
    assert_eq!(js_hit(destructured, "mkdir"), FS_CWE);
    let other_key = "const { mkdir: p } = require('fs');\np.open(x); // SINK\n";
    assert_eq!(js_hit(other_key, "open"), None);
    let member = "const f = require('fs');\nf.promises.mkdir(x); // SINK\n";
    assert_eq!(js_hit(member, "mkdir"), FS_CWE);
    let not_member = "const f = require('fs');\nf.other.mkdir(x); // SINK\n";
    assert_eq!(js_hit(not_member, "mkdir"), None);
}

#[test]
fn js_module_fs_call_shape_gates() {
    // `new` is never a call of the API
    assert_eq!(
        js_hit(
            "import * as f from 'fs';\nnew f.mkdir(x); // SINK\n",
            "mkdir"
        ),
        None
    );
    // a literal path-position argument is no path flow; a computed one is
    let lit = "import * as f from 'fs';\nf.chmodSync('/srv/a', x); // SINK\n";
    assert_eq!(js_hit(lit, "chmodSync"), None);
    let tpl = "import * as f from 'fs';\nf.chmodSync(`/srv/a`, x); // SINK\n";
    assert_eq!(js_hit(tpl, "chmodSync"), None);
    let concat = "import * as f from 'fs';\nf.chmodSync('/srv/' + x, 1); // SINK\n";
    assert_eq!(js_hit(concat, "chmodSync"), FS_CWE);
    // two-path APIs check both path positions
    let second = "import * as f from 'fs';\nf.copyFileSync('/srv/a', x); // SINK\n";
    assert_eq!(js_hit(second, "copyFileSync"), FS_CWE);
    let both_literal = "import * as f from 'fs';\nf.copyFileSync('/a', '/b', x); // SINK\n";
    assert_eq!(js_hit(both_literal, "copyFileSync"), None);
    // a single-path API ignores its second argument
    let single = "import * as f from 'fs';\nf.mkdirSync('/a', x); // SINK\n";
    assert_eq!(js_hit(single, "mkdirSync"), None);
}

#[test]
fn js_module_node_ssh_constructors() {
    let ssh = Some(("js-exec-call", "CWE-78"));
    let member =
        "const N = require('node-ssh').NodeSSH;\nconst s = new N();\ns.execCommand(x); // SINK\n";
    assert_eq!(js_hit(member, "execCommand"), ssh);
    let default_require =
        "const N = require('node-ssh');\nconst s = new N();\ns.execCommand(x); // SINK\n";
    assert_eq!(js_hit(default_require, "execCommand"), ssh);
    let default_import =
        "import N from 'node-ssh';\nconst s = new N();\ns.execCommand(x); // SINK\n";
    assert_eq!(js_hit(default_import, "execCommand"), ssh);
    let ns_member =
        "import * as m from 'node-ssh';\nconst s = new m.NodeSSH();\ns.execCommand(x); // SINK\n";
    assert_eq!(js_hit(ns_member, "execCommand"), ssh);
    let alias = "import { NodeSSH } from 'node-ssh';\nconst N = NodeSSH;\nconst s = new N();\ns.execCommand(x); // SINK\n";
    assert_eq!(js_hit(alias, "execCommand"), ssh);
    let wrong_module =
        "const N = require('other').NodeSSH;\nconst s = new N();\ns.execCommand(x); // SINK\n";
    assert_eq!(js_hit(wrong_module, "execCommand"), None);
    let wrong_member =
        "const N = require('node-ssh').Other;\nconst s = new N();\ns.execCommand(x); // SINK\n";
    assert_eq!(js_hit(wrong_member, "execCommand"), None);
    let wrong_default =
        "const N = require('other');\nconst s = new N();\ns.execCommand(x); // SINK\n";
    assert_eq!(js_hit(wrong_default, "execCommand"), None);
    let rebound =
        "let N = require('node-ssh').NodeSSH;\nN = z;\nconst s = new N();\ns.execCommand(x); // SINK\n";
    assert_eq!(js_hit(rebound, "execCommand"), None);
    let destructured_twice = "var { NodeSSH } = require('node-ssh');\nvar NodeSSH = z;\nconst s = new NodeSSH();\ns.execCommand(x); // SINK\n";
    assert_eq!(js_hit(destructured_twice, "execCommand"), None);
    let destructured_other =
        "var { Other } = require('node-ssh');\nconst s = new Other();\ns.execCommand(x); // SINK\n";
    assert_eq!(js_hit(destructured_other, "execCommand"), None);
    let destructured_wrong_module =
        "var { NodeSSH } = require('other');\nconst s = new NodeSSH();\ns.execCommand(x); // SINK\n";
    assert_eq!(js_hit(destructured_wrong_module, "execCommand"), None);
}

#[test]
fn js_module_document_chain_and_case() {
    let dom = Some(("js-dom-html", "CWE-79"));
    assert_eq!(
        js_hit(
            "document.execCommand('inserthtml', 0, x); // SINK\n",
            "execCommand"
        ),
        dom
    );
    assert_eq!(
        js_hit(
            "window.document.execCommand('InsertHTML', 0, x); // SINK\n",
            "execCommand"
        ),
        dom
    );
    assert_eq!(
        js_hit(
            "foo.document.execCommand('insertHTML', 0, x); // SINK\n",
            "execCommand"
        ),
        None
    );
    assert_eq!(
        js_hit(
            "document.execCommand('insertText', 0, x); // SINK\n",
            "execCommand"
        ),
        None
    );
    assert_eq!(
        js_hit(
            "new document.execCommand('insertHTML', 0, x); // SINK\n",
            "execCommand"
        ),
        None
    );
    let loop_shadow =
        "for (const document of ds) {\n  document.execCommand('insertHTML', 0, x); // SINK\n}\n";
    assert_eq!(js_hit(loop_shadow, "execCommand"), None);
}
