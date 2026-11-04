//! #637 Slice A1 variant matrix, JS/TS/TSX: import-resolved module identity for review-taint sinks.
//! Identity is positive evidence only: a hit needs a lexically resolved binding to a listed
//! module, and every other shape keeps exactly the behaviour it had before.

use super::taint_sink_rules_support::*;
use super::taint_sink_strength_test::*;
use crate::common::*;

fn ts(label: &'static str, source: &'static str, severity: &'static str) -> Sev {
    Sev {
        label,
        path: "a.ts",
        lang: Language::TypeScript,
        source,
        severity,
    }
}
fn tsx(label: &'static str, source: &'static str, severity: &'static str) -> Sev {
    Sev {
        label,
        path: "a.tsx",
        lang: Language::Tsx,
        source,
        severity,
    }
}

fn sink_findings(path: &str, lang: Language, source: &str) -> usize {
    let sink = marker_line(source, "SINK");
    findings(path, lang, source)
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("taint_sink") && f.line == sink)
        .count()
}

#[rustfmt::skip]
#[test]
fn fs_module_identity_positive() {
    assert_severities(&[
        js("P7 import * as files from node:fs/promises; files.open", "import * as files from \"node:fs/promises\";\nasync function h(input) {\n  const x = input.path; // SEED\n  await files.open(x, \"w\"); // SINK\n}\n", W),
        js("P7b const files = require('fs-extra'); files.remove", "const files = require(\"fs-extra\");\nfunction h(input) {\n  const x = input.path; // SEED\n  files.remove(x); // SINK\n}\n", W),
        js("P7c const { remove } = require('fs-extra'); remove(x)", "const { remove } = require('fs-extra');\nfunction h(input) {\n  const x = input.path; // SEED\n  remove(x); // SINK\n}\n", W),
        js("P7d import fse from 'fs-extra'; fse.outputFile", "import fse from 'fs-extra';\nfunction h(input) {\n  const x = input.path; // SEED\n  fse.outputFile(x, 'd'); // SINK\n}\n", W),
        js("P7e fs.promises.open through a namespace alias", "import * as f from 'fs';\nasync function h(input) {\n  const x = input.path; // SEED\n  await f.promises.open(x); // SINK\n}\n", W),
        js("P7f named import rm from node:fs/promises", "import { rm } from \"node:fs/promises\";\nasync function h(input) {\n  const x = input.path; // SEED\n  await rm(x); // SINK\n}\n", W),
        js("P7g renamed named import", "import { mkdir as mk } from 'graceful-fs';\nfunction h(input) {\n  const x = input.path; // SEED\n  mk(x); // SINK\n}\n", W),
        js("P7h destructured promises then namespace call", "const { promises: fsp } = require('fs');\nasync function h(input) {\n  const x = input.path; // SEED\n  await fsp.rename(x, 'b'); // SINK\n}\n", W),
        js("P7i import { promises as fsp } from node:fs", "import { promises as fsp } from 'node:fs';\nasync function h(input) {\n  const x = input.path; // SEED\n  await fsp.copyFile(x, 'b'); // SINK\n}\n", W),
        js("P7j require member: require('fs').createReadStream", "function h(input) {\n  const x = input.path; // SEED\n  return require('fs').createReadStream(x); // SINK\n}\n", W),
        js("P7k default import of fs, readdirSync", "import fs from 'fs';\nfunction h(input) {\n  const x = input.path; // SEED\n  return fs.readdirSync(x); // SINK\n}\n", W),
        js("P7l alias const of an fs module", "import * as fs from 'node:fs';\nconst f2 = fs;\nfunction h(input) {\n  const x = input.path; // SEED\n  return f2.mkdirSync(x); // SINK\n}\n", W),
        js("P7m fs-extra copy under an arbitrary name", "const helper = require('fs-extra');\nfunction h(input) {\n  const x = input.path; // SEED\n  helper.copy(x, 'b'); // SINK\n}\n", W),
        js("P7n fs-extra ensureDir named", "import { ensureDir } from 'fs-extra';\nasync function h(input) {\n  const x = input.path; // SEED\n  await ensureDir(x); // SINK\n}\n", W),
        ts("P7o TS file", "import * as files from \"node:fs/promises\";\nasync function h(input: any) {\n  const x = input.path; // SEED\n  await files.open(x, \"w\"); // SINK\n}\n", W),
        tsx("P7p TSX file", "import { rm } from 'fs/promises';\nasync function h(input: any) {\n  const x = input.path; // SEED\n  await rm(x); // SINK\n}\n", W),
        ts("P7q TS import-equals require", "import files = require('fs-extra');\nfunction h(input: any) {\n  const x = input.path; // SEED\n  files.remove(x); // SINK\n}\n", W),
    ]);
}

#[rustfmt::skip]
#[test]
fn fs_module_identity_negative() {
    assert_severities(&[
        js("N8 files.open with a parameter shadowing the fs import", "import * as files from 'fs/promises';\nfunction h(files, input) {\n  const x = input.v; // SEED\n  files.open(x); // SINK\n}\n", NONE),
        js("N8b inner const shadows the fs import", "import * as files from 'fs/promises';\nfunction h(input) {\n  const files = makeThing();\n  const x = input.v; // SEED\n  files.open(x); // SINK\n}\n", NONE),
        js("N8c rebound require alias is no proof", "let files = require('fs-extra');\nfiles = makeFake();\nfunction h(input) {\n  const x = input.v; // SEED\n  files.copy(x); // SINK\n}\n", NONE),
        js("N8d shadowing parameter of a named import", "import { mkdir } from 'fs/promises';\nfunction h(mkdir, input) {\n  const x = input.v; // SEED\n  mkdir(x); // SINK\n}\n", NONE),
        js("N8e named import of a non-path API (watch)", "import { watch } from 'fs';\nfunction h(input) {\n  const x = input.v; // SEED\n  watch(x); // SINK\n}\n", NONE),
        js("N8f non-path API on a namespace (constants)", "import * as files from 'fs';\nfunction h(input) {\n  const x = input.v; // SEED\n  files.constants(x); // SINK\n}\n", NONE),
        js("N8g same-named method on an unrelated receiver, no import", "function h(store, input) {\n  const x = input.v; // SEED\n  store.mkdir(x); // SINK\n}\n", NONE),
        js("N8h module not in the table", "import * as files from 'some-other-fs';\nfunction h(input) {\n  const x = input.v; // SEED\n  files.mkdir(x); // SINK\n}\n", NONE),
        js("N8i fs imported in a field only: property receiver", "import * as fs from 'fs';\nclass P {\n  constructor() { this.files = fs; }\n  h(input) {\n    const x = input.v; // SEED\n    this.files.mkdir(x); // SINK\n  }\n}\n", NONE),
        js("N8j bare mkdir with no import", "function h(input) {\n  const x = input.v; // SEED\n  mkdir(x); // SINK\n}\n", NONE),
        js("N8k named import used as non-callee", "import { mkdir } from 'fs';\nfunction h(input) {\n  const x = input.v; // SEED\n  const m = mkdir; // SINK\n}\n", NONE),
        js("N8l local function named like an fs API shadows nothing imported", "function mkdir(p) { return p; }\nfunction h(input) {\n  const x = input.v; // SEED\n  mkdir(x); // SINK\n}\n", NONE),
        js("N8m namespace .promises on a non-fs module", "import * as other from 'other-lib';\nfunction h(input) {\n  const x = input.v; // SEED\n  other.promises.mkdir(x); // SINK\n}\n", NONE),
    ]);
}

#[rustfmt::skip]
#[test]
fn node_ssh_identity() {
    assert_severities(&[
        js("P4 node-ssh new NodeSSH().execCommand", "import { NodeSSH } from 'node-ssh';\nasync function h(input) {\n  const x = input.v; // SEED\n  const ssh = new NodeSSH();\n  await ssh.execCommand(x); // SINK\n}\n", W),
        js("P4c renamed import and exec", "import { NodeSSH as N } from 'node-ssh';\nasync function h(input) {\n  const x = input.v; // SEED\n  const c = new N();\n  await c.exec(x, []); // SINK\n}\n", W),
        js("P4d require destructure", "const { NodeSSH } = require('node-ssh');\nasync function h(input) {\n  const x = input.v; // SEED\n  const ssh = new NodeSSH();\n  await ssh.execCommand(x); // SINK\n}\n", W),
        js("P4e module-level local used in a function", "import { NodeSSH } from 'node-ssh';\nconst ssh = new NodeSSH();\nasync function h(input) {\n  const x = input.v; // SEED\n  await ssh.execCommand(x); // SINK\n}\n", W),
        ts("P4f TS file", "import { NodeSSH } from 'node-ssh';\nasync function h(input: any) {\n  const x = input.v; // SEED\n  const ssh = new NodeSSH();\n  await ssh.execCommand(x); // SINK\n}\n", W),
        js("P4b execCommand on an unknown receiver: no fallback in this slice", "async function h(ssh, input) {\n  const x = input.v; // SEED\n  await ssh.execCommand(x); // SINK\n}\n", NONE),
        js("N4a NodeSSH not imported from node-ssh", "import { NodeSSH } from './local-ssh';\nasync function h(input) {\n  const x = input.v; // SEED\n  const ssh = new NodeSSH();\n  await ssh.execCommand(x); // SINK\n}\n", NONE),
        js("N4b local rebound after construction", "import { NodeSSH } from 'node-ssh';\nasync function h(input) {\n  let ssh = new NodeSSH();\n  ssh = makeFake();\n  const x = input.v; // SEED\n  await ssh.execCommand(x); // SINK\n}\n", NONE),
        js("N4c second binding of the one-hop local", "import { NodeSSH } from 'node-ssh';\nasync function h(input) {\n  var ssh = new NodeSSH();\n  var ssh = makeFake();\n  const x = input.v; // SEED\n  await ssh.execCommand(x); // SINK\n}\n", NONE),
        js("N4d parameter shadows the local", "import { NodeSSH } from 'node-ssh';\nconst ssh = new NodeSSH();\nasync function h(ssh, input) {\n  const x = input.v; // SEED\n  await ssh.execCommand(x); // SINK\n}\n", NONE),
        js("N4e field receiver is out of scope", "import { NodeSSH } from 'node-ssh';\nclass P {\n  constructor() { this.ssh = new NodeSSH(); }\n  async h(input) {\n    const x = input.v; // SEED\n    await this.ssh.execCommand(x); // SINK\n  }\n}\n", NONE),
        js("N4f NodeSSH method not in the table", "import { NodeSSH } from 'node-ssh';\nasync function h(input) {\n  const x = input.v; // SEED\n  const ssh = new NodeSSH();\n  await ssh.connect(x); // SINK\n}\n", NONE),
        js("N4g shadowing NodeSSH parameter", "import { NodeSSH } from 'node-ssh';\nasync function h(NodeSSH, input) {\n  const x = input.v; // SEED\n  const ssh = new NodeSSH();\n  await ssh.execCommand(x); // SINK\n}\n", NONE),
        js("N4h call without new", "import { NodeSSH } from 'node-ssh';\nasync function h(input) {\n  const x = input.v; // SEED\n  const ssh = NodeSSH();\n  await ssh.execCommand(x); // SINK\n}\n", NONE),
    ]);
}

#[rustfmt::skip]
#[test]
fn document_exec_command() {
    assert_severities(&[
        js("P5 document.execCommand insertHTML (explicit DOM rule)", "function h(input) {\n  const x = input.v; // SEED\n  document.execCommand('insertHTML', false, x); // SINK\n}\n", W),
        js("P5b double-quoted insertHTML", "function h(input) {\n  const x = input.v; // SEED\n  document.execCommand(\"insertHTML\", false, x); // SINK\n}\n", W),
        js("P5c window.document chain", "function h(input) {\n  const x = input.v; // SEED\n  window.document.execCommand('insertHTML', false, x); // SINK\n}\n", W),
        js("P5d globalThis.document chain", "function h(input) {\n  const x = input.v; // SEED\n  globalThis.document.execCommand('insertHTML', false, x); // SINK\n}\n", W),
        js("P5e non-literal command", "function h(input, cmd) {\n  const x = input.v; // SEED\n  document.execCommand(cmd, false, x); // SINK\n}\n", W),
        js("P5f template literal without substitution", "function h(input) {\n  const x = input.v; // SEED\n  document.execCommand(`insertHTML`, false, x); // SINK\n}\n", W),
        js("N12 document.execCommand with a literal non-HTML command", "function h(input) {\n  const x = input.v; // SEED\n  document.execCommand('bold', false, x); // SINK\n}\n", NONE),
        js("N12b shadowing document parameter", "function h(document, input) {\n  const x = input.v; // SEED\n  document.execCommand('insertHTML', false, x); // SINK\n}\n", NONE),
        js("N12c execCommand on an unrelated receiver", "function h(editor, input) {\n  const x = input.v; // SEED\n  editor.execCommand('insertHTML', false, x); // SINK\n}\n", NONE),
        js("N12d no arguments", "function h(input) {\n  const x = input.v; // SEED\n  document.execCommand(); // SINK\n}\n", NONE),
        js("N12e shadowing local document", "function h(input) {\n  const document = makeDoc();\n  const x = input.v; // SEED\n  document.execCommand('insertHTML', false, x); // SINK\n}\n", NONE),
    ]);
}

#[rustfmt::skip]
#[test]
fn unchanged_negatives_and_overlap() {
    assert_severities(&[
        js("N2 Array.prototype.find (shell-only find)", "function h(arr, input) {\n  const x = input.v; // SEED\n  arr.find(x); // SINK\n}\n", NONE),
        js("N3 res.render (deny: render)", "function h(req, res) {\n  const x = req.query.v; // SEED\n  res.render(x); // SINK\n}\n", NONE),
        js("N7 readByQuery declared in the repository", "class S { async readByQuery(q) { return q; } }\nasync function h(s, input) {\n  const x = input.v; // SEED\n  await s.readByQuery(x); // SINK\n}\n", NONE),
        js("N7b useQuery external hook (deny: SQL fragments)", "function h(trpc, input) {\n  const x = input.v; // SEED\n  trpc.report.useQuery(x); // SINK\n}\n", NONE),
        js("N10 handlebars.compile (deny: compile; template-module table is D7)", "const handlebars = require('handlebars');\nfunction h(input) {\n  const x = input.v; // SEED\n  handlebars.compile(x); // SINK\n}\n", NONE),
        js("G1 unknown-receiver execCommand plus a strong rule stays warning", "function h(ssh, res, input) {\n  const x = input.v; // SEED\n  ssh.execCommand(x); res.redirect(x); // SINK\n}\n", W),
        js("G1b main-strong fs name stays warning with an import", "import * as fs from 'fs';\nfunction h(input) {\n  const x = input.v; // SEED\n  fs.writeFile(x, 'a'); // SINK\n}\n", W),
        js("G1c receiver-name fs path unchanged", "function h(input) {\n  const x = input.v; // SEED\n  fs.remove(x); // SINK\n}\n", W),
    ]);
}

#[test]
fn table_hit_on_a_main_strong_line_is_one_finding() {
    let src = "import * as fs from 'fs';\nfunction h(input) {\n  const x = input.v; // SEED\n  fs.writeFile(x, 'a'); // SINK\n}\n";
    assert_eq!(sink_findings("a.js", Language::JavaScript, src), 1);
    let src2 = "import { rm } from 'fs/promises';\nasync function h(input) {\n  const x = input.v; // SEED\n  await rm(x); // SINK\n}\n";
    assert_eq!(sink_findings("a.js", Language::JavaScript, src2), 1);
}

// ---- round 1: lexically correct sole-binding, constructor forms, type-only, case, literal paths

#[rustfmt::skip]
#[test]
fn shadowing_scopes_give_no_identity() {
    assert_severities(&[
        js("W1 for-of head shadows the import", "import * as files from 'fs';\nfunction h(stores, input) {\n  for (const files of stores) {\n    const x = input.v; // SEED\n    files.mkdir(x); // SINK\n  }\n}\n", NONE),
        js("W1b catch parameter shadows the import", "import * as files from 'fs';\nfunction h(input) {\n  try { run(); } catch (files) {\n    const x = input.v; // SEED\n    files.mkdir(x); // SINK\n  }\n}\n", NONE),
        js("W1c destructured default parameter shadows", "import * as files from 'fs';\nfunction h({ files } = {}, input) {\n  const x = input.v; // SEED\n  files.mkdir(x); // SINK\n}\n", NONE),
        js("W1d arrow destructured default value parameter", "import * as files from 'fs';\nconst h = ({ files = fake } = {}, input) => {\n  const x = input.v; // SEED\n  return files.mkdir(x); // SINK\n};\n", NONE),
        js("W1e var in a nested block hoists over the import", "import { mkdir } from 'fs/promises';\nfunction h(c, input) {\n  if (c) { var mkdir = noop; }\n  const x = input.v; // SEED\n  mkdir(x); // SINK\n}\n", NONE),
        js("W1f var in a nested block hoists over a destructured require", "const { ensureDir } = require('fs-extra');\nfunction h(c, input) {\n  if (c) { var ensureDir = noop; }\n  const x = input.v; // SEED\n  ensureDir(x); // SINK\n}\n", NONE),
        js("W1g switch case const shadows the import", "import * as files from 'fs';\nfunction h(k, input) {\n  switch (k) {\n    case 1:\n      const files = makeStore();\n      const x = input.v; // SEED\n      files.mkdir(x); // SINK\n  }\n}\n", NONE),
        js("W1h for-let head shadows the import", "import * as files from 'fs';\nfunction h(stores, input) {\n  for (let files = stores[0]; files; files = null) {\n    const x = input.v; // SEED\n    files.mkdir(x); // SINK\n  }\n}\n", NONE),
        js("W1i for-of class shadows NodeSSH", "import { NodeSSH } from 'node-ssh';\nfunction h(classes, input) {\n  for (const NodeSSH of classes) {\n    const ssh = new NodeSSH();\n    const x = input.v; // SEED\n    ssh.execCommand(x); // SINK\n  }\n}\n", NONE),
        js("W1j for-in var head hoists over the import", "import * as files from 'fs';\nfunction h(o, input) {\n  for (var files in o) {}\n  const x = input.v; // SEED\n  files.mkdir(x); // SINK\n}\n", NONE),
        js("W1k local document shadows the global", "function h(docs, input) {\n  for (const document of docs) {\n    const x = input.v; // SEED\n    document.execCommand('insertHTML', false, x); // SINK\n  }\n}\n", NONE),
        js("W1l function-local const shadows an outer CJS alias", "const files = require('fs');\nfunction h(input) {\n  const files = makeStore();\n  const x = input.v; // SEED\n  files.mkdir(x); // SINK\n}\n", NONE),
        js("W1m increment writes the alias", "let n = require('fs');\nn++;\nfunction h(input) {\n  const x = input.v; // SEED\n  n.mkdir(x); // SINK\n}\n", NONE),
        js("W1n destructuring assignment writes the alias", "let n = require('fs');\n[n] = [fake];\nfunction h(input) {\n  const x = input.v; // SEED\n  n.mkdir(x); // SINK\n}\n", NONE),
    ]);
}

#[rustfmt::skip]
#[test]
fn second_binding_of_destructured_fs() {
    assert_severities(&[
        js("W2 destructured require then var redeclared", "function h(input, fake) {\n  var { mkdir: mk } = require('fs');\n  var mk = fake;\n  const x = input.v; // SEED\n  mk(x); // SINK\n}\n", NONE),
        js("W2b destructured promises then var redeclared", "var { promises: fsp } = require('fs');\nvar fsp = fake;\nfunction h(input) {\n  const x = input.v; // SEED\n  fsp.mkdir(x); // SINK\n}\n", NONE),
        js("W2c single destructured declaration control", "function h(input) {\n  var { mkdir: mk } = require('fs');\n  const x = input.v; // SEED\n  mk(x); // SINK\n}\n", W),
        js("W2d single destructured promises control", "var { promises: fsp } = require('fs');\nfunction h(input) {\n  const x = input.v; // SEED\n  fsp.mkdir(x); // SINK\n}\n", W),
    ]);
}

#[rustfmt::skip]
#[test]
fn unrelated_same_name_scopes_keep_identity() {
    assert_severities(&[
        js("W3 unrelated function parameter of the same name", "const files = require('fs');\nfunction unrelated(files) {}\nfunction h(input) {\n  const x = input.v; // SEED\n  files.mkdir(x); // SINK\n}\n", W),
        js("W3b callback parameter elsewhere", "const files = require('fs');\n[1].map((files) => files);\nfunction h(input) {\n  const x = input.v; // SEED\n  files.mkdir(x); // SINK\n}\n", W),
        js("W3c another function's own local of the same name", "const fse = require('fs-extra');\nfunction other() { const fse = make(); return fse; }\nfunction h(input) {\n  const x = input.v; // SEED\n  fse.remove(x); // SINK\n}\n", W),
        js("W3d module-level NodeSSH local plus an unrelated helper parameter", "import { NodeSSH } from 'node-ssh';\nconst ssh = new NodeSSH();\nfunction close(ssh) {}\nasync function h(input) {\n  const x = input.v; // SEED\n  await ssh.execCommand(x); // SINK\n}\n", W),
        js("W3e two functions each with their own NodeSSH local", "import { NodeSSH } from 'node-ssh';\nfunction a() { const ssh = new NodeSSH(); return ssh; }\nasync function h(input) {\n  const ssh = new NodeSSH();\n  const x = input.v; // SEED\n  await ssh.execCommand(x); // SINK\n}\n", W),
        js("W3f genuine same-scope redeclaration still refuses", "const files = require('fs');\nvar files = fake;\nfunction h(input) {\n  const x = input.v; // SEED\n  files.mkdir(x); // SINK\n}\n", NONE),
    ]);
}

#[rustfmt::skip]
#[test]
fn node_ssh_constructor_forms() {
    assert_severities(&[
        js("W4 member extracted constructor", "const N = require('node-ssh').NodeSSH;\nfunction h(input) {\n  const x = input.v; // SEED\n  const c = new N();\n  c.execCommand(x); // SINK\n}\n", W),
        js("W4b legacy default export require", "const NodeSSH = require('node-ssh');\nfunction h(input) {\n  const x = input.v; // SEED\n  const ssh = new NodeSSH();\n  ssh.execCommand(x); // SINK\n}\n", W),
        js("W4c default import", "import Client from 'node-ssh';\nfunction h(input) {\n  const x = input.v; // SEED\n  const ssh = new Client();\n  ssh.exec(x, []); // SINK\n}\n", W),
        js("W4d namespace member constructor", "import * as m from 'node-ssh';\nfunction h(input) {\n  const x = input.v; // SEED\n  const ssh = new m.NodeSSH();\n  ssh.execCommand(x); // SINK\n}\n", W),
        js("W4e wrong module for the member form", "const N = require('other-ssh').NodeSSH;\nfunction h(input) {\n  const x = input.v; // SEED\n  const c = new N();\n  c.execCommand(x); // SINK\n}\n", NONE),
        js("W4f wrong member", "const N = require('node-ssh').Other;\nfunction h(input) {\n  const x = input.v; // SEED\n  const c = new N();\n  c.execCommand(x); // SINK\n}\n", NONE),
        js("W4g rebound constructor", "let N = require('node-ssh').NodeSSH;\nN = Fake;\nfunction h(input) {\n  const x = input.v; // SEED\n  const c = new N();\n  c.execCommand(x); // SINK\n}\n", NONE),
        js("W4h default require from another module", "const NodeSSH = require('other-ssh');\nfunction h(input) {\n  const x = input.v; // SEED\n  const ssh = new NodeSSH();\n  ssh.execCommand(x); // SINK\n}\n", NONE),
    ]);
}

#[rustfmt::skip]
#[test]
fn type_only_imports_give_no_identity() {
    assert_severities(&[
        ts("W5 import type namespace", "import type * as files from 'fs';\nfunction h(input: any) {\n  const x = input.v; // SEED\n  files.mkdir(x); // SINK\n}\n", NONE),
        ts("W5b import type named", "import type { mkdir } from 'fs';\nfunction h(input: any) {\n  const x = input.v; // SEED\n  mkdir(x); // SINK\n}\n", NONE),
        ts("W5c inline type specifier", "import { type mkdir } from 'fs';\nfunction h(input: any) {\n  const x = input.v; // SEED\n  mkdir(x); // SINK\n}\n", NONE),
        ts("W5d type-only NodeSSH", "import type { NodeSSH } from 'node-ssh';\nfunction h(input: any) {\n  const ssh = new NodeSSH();\n  const x = input.v; // SEED\n  ssh.execCommand(x); // SINK\n}\n", NONE),
        ts("W5e type-only default", "import type files from 'fs-extra';\nfunction h(input: any) {\n  const x = input.v; // SEED\n  files.remove(x); // SINK\n}\n", NONE),
        ts("W5f value import control", "import * as files from 'fs';\nfunction h(input: any) {\n  const x = input.v; // SEED\n  files.mkdir(x); // SINK\n}\n", W),
        ts("W5g value named import control next to a type specifier", "import { type Stats, mkdir } from 'fs';\nfunction h(input: any) {\n  const x = input.v; // SEED\n  mkdir(x); // SINK\n}\n", W),
    ]);
}

#[rustfmt::skip]
#[test]
fn exec_command_case_insensitive() {
    assert_severities(&[
        js("F2 lowercase command id", "function h(input) {\n  const x = input.v; // SEED\n  document.execCommand('inserthtml', false, x); // SINK\n}\n", W),
        js("F2b mixed case command id", "function h(input) {\n  const x = input.v; // SEED\n  document.execCommand('InsertHTML', false, x); // SINK\n}\n", W),
        js("F2c other command stays none", "function h(input) {\n  const x = input.v; // SEED\n  document.execCommand('InsertText', false, x); // SINK\n}\n", NONE),
    ]);
}

#[rustfmt::skip]
#[test]
fn fs_literal_path_with_tainted_data() {
    assert_severities(&[
        js("S1 outputJson fixed path, tainted data", "const fse = require('fs-extra');\nfunction h(req) {\n  const x = req.body; // SEED\n  fse.outputJson('settings.json', x); // SINK\n}\n", NONE),
        js("S1b outputFile fixed path, tainted data", "import fse from 'fs-extra';\nfunction h(req) {\n  const x = req.body; // SEED\n  fse.outputFile('/var/app/message.txt', x); // SINK\n}\n", NONE),
        js("S1c named writeJson fixed path", "import { writeJson } from 'fs-extra';\nfunction h(req) {\n  const x = req.body; // SEED\n  writeJson('db.json', x); // SINK\n}\n", NONE),
        js("S1d chmodSync fixed path, tainted mode", "import * as fs from 'fs';\nfunction h(req) {\n  const x = req.body; // SEED\n  fs.chmodSync('/srv/app.sock', x); // SINK\n}\n", NONE),
        js("S1e template without substitution is literal", "import * as fs from 'fs';\nfunction h(req) {\n  const x = req.body; // SEED\n  fs.mkdirSync(`/srv/app`, x); // SINK\n}\n", NONE),
        js("S1f tainted path stays strong", "const fse = require('fs-extra');\nfunction h(req) {\n  const x = req.body; // SEED\n  fse.outputFile(x, 'data'); // SINK\n}\n", W),
        js("S1g concatenated path stays strong", "const fse = require('fs-extra');\nfunction h(req) {\n  const x = req.body; // SEED\n  fse.outputFile('/srv/' + x, 'data'); // SINK\n}\n", W),
        js("S1h two-path API: tainted destination", "import * as fs from 'fs';\nfunction h(req) {\n  const x = req.body; // SEED\n  fs.copyFileSync('/srv/a', x); // SINK\n}\n", W),
        js("S1i two-path API: both literal", "import * as fs from 'fs';\nfunction h(req) {\n  const x = req.body; // SEED\n  fs.copyFileSync('/srv/a', '/srv/b', x); // SINK\n}\n", NONE),
        js("S1j no arguments", "import * as fs from 'fs';\nfunction h(req) {\n  const x = req.body; // SEED\n  fs.readdirSync(); // SINK\n}\n", NONE),
    ]);
}

// ---- round 2: class-expression name, `with` bodies, impostor declared first

#[rustfmt::skip]
#[test]
fn class_expression_and_with_refuse() {
    assert_severities(&[
        js("R2-W1 class expression's own name shadows the import in its body", "import * as files from 'fs';\nconst C = class files {\n  static run(input) {\n    const x = input.v; // SEED\n    files.mkdir(x); // SINK\n  }\n  static mkdir() {}\n};\n", NONE),
        js("R2-W1b class expression returned from a function", "const fs = require('fs');\nfunction outer() {\n  return class fs {\n    static run(input) {\n      const x = input.v; // SEED\n      fs.mkdir(x); // SINK\n    }\n  };\n}\n", NONE),
        js("R2-W1c control: use outside the class body still fires", "const fs = require('fs');\nconst C = class fs { static run() {} };\nfunction h(input) {\n  const x = input.v; // SEED\n  fs.mkdir(x); // SINK\n}\n", W),
        js("R2-W2 with body makes the alias unprovable", "const fs = require('fs');\nfunction h(o, input) {\n  with (o) {\n    const x = input.v; // SEED\n    fs.mkdir(x); // SINK\n  }\n}\n", NONE),
        js("R2-W2b with body makes document unprovable", "function h(o, input) {\n  with (o) {\n    const x = input.v; // SEED\n    document.execCommand('insertHTML', false, x); // SINK\n  }\n}\n", NONE),
        js("R2-W2c control: just outside the with still fires", "const fs = require('fs');\nfunction h(o, input) {\n  with (o) { run(); }\n  const x = input.v; // SEED\n  fs.mkdir(x); // SINK\n}\n", W),
        js("R2-S1 impostor declared first, module binding last", "var files = fake();\nvar files = require('fs');\nfunction h(input) {\n  const x = input.v; // SEED\n  files.mkdir(x); // SINK\n}\n", NONE),
    ]);
}
