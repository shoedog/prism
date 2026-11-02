//! #622 S1 variant matrix, JS/TS: review sinks match by call position and receiver.

use super::taint_sink_rules_support::*;
use crate::common::*;

#[rustfmt::skip]
const POS_CODE_EXEC: &[Case] = &[
    Case { label: "direct eval", source: "function h(input) {\n  const x = input.v; // SEED\n  eval(x); // SINK\n}\n" },
    Case { label: "indirect eval (0, eval)", source: "function h(input) {\n  const x = input.v; // SEED\n  (0, eval)('v = ' + x); // SINK\n}\n" },
    Case { label: "window.eval", source: "function h(input) {\n  const x = input.v; // SEED\n  window.eval(x); // SINK\n}\n" },
    Case { label: "globalThis.eval", source: "function h(input) {\n  const x = input.v; // SEED\n  globalThis.eval(x); // SINK\n}\n" },
    Case { label: "new Function", source: "function h(input) {\n  const x = input.v; // SEED\n  const f = new Function('return ' + x); // SINK\n}\n" },
    Case { label: "Function call", source: "function h(input) {\n  const x = input.v; // SEED\n  const f = Function(x); // SINK\n}\n" },
];

#[rustfmt::skip]
const POS_PROCESS: &[Case] = &[
    Case { label: "require alias", source: "const cp = require('child_process');\nfunction h(input) {\n  const x = input.v; // SEED\n  cp.exec(x); // SINK\n}\n" },
    Case { label: "arbitrary alias execSync", source: "const shell = require('child_process');\nfunction h(input) {\n  const x = input.v; // SEED\n  shell.execSync(x); // SINK\n}\n" },
    Case { label: "node: prefix", source: "const tester = require('node:child_process');\nfunction h(input) {\n  const x = input.v; // SEED\n  tester.spawn(x); // SINK\n}\n" },
    Case { label: "destructured exec", source: "const { exec } = require('child_process');\nfunction h(input) {\n  const x = input.v; // SEED\n  exec(x); // SINK\n}\n" },
    Case { label: "destructured rename", source: "const { execFile: run } = require('child_process');\nfunction h(input) {\n  const x = input.v; // SEED\n  run(x); // SINK\n}\n" },
    Case { label: "esm namespace", source: "import * as proc from 'child_process';\nfunction h(input) {\n  const x = input.v; // SEED\n  proc.execFile(x); // SINK\n}\n" },
    Case { label: "esm default", source: "import proc from 'node:child_process';\nfunction h(input) {\n  const x = input.v; // SEED\n  proc.fork(x); // SINK\n}\n" },
    Case { label: "esm named", source: "import { spawnSync } from 'child_process';\nfunction h(input) {\n  const x = input.v; // SEED\n  spawnSync(x); // SINK\n}\n" },
    Case { label: "bare exec wrapper import elsewhere", source: "function h(input) {\n  const x = input.v; // SEED\n  exec(x); // SINK\n}\n" },
    Case { label: "inline require", source: "function h(input) {\n  const x = input.v; // SEED\n  require('child_process').exec(x); // SINK\n}\n" },
    Case { label: "promisified exec", source: "import { exec } from 'child_process';\nimport { promisify } from 'util';\nconst run = promisify(exec);\nasync function h(input) {\n  const x = input.v; // SEED\n  await run(x); // SINK\n}\n" },
];

#[rustfmt::skip]
const POS_DB_RESPONSE: &[Case] = &[
    Case { label: "db.query", source: "function h(db, input) {\n  const x = input.v; // SEED\n  db.query(x); // SINK\n}\n" },
    Case { label: "knex.raw", source: "function h(knex, input) {\n  const x = input.v; // SEED\n  knex.raw(x); // SINK\n}\n" },
    Case { label: "prisma raw unsafe", source: "function h(prisma, input) {\n  const x = input.v; // SEED\n  prisma.$queryRawUnsafe(x); // SINK\n}\n" },
    Case { label: "pool.execute", source: "function h(pool, input) {\n  const x = input.v; // SEED\n  pool.execute(x); // SINK\n}\n" },
    Case { label: "sqlite db.exec", source: "function h(db, input) {\n  const x = input.v; // SEED\n  db.exec(x); // SINK\n}\n" },
    Case { label: "res.send", source: "function h(req, res) {\n  const x = req.query.v; // SEED\n  res.send(x); // SINK\n}\n" },
    Case { label: "res.redirect", source: "function h(req, res) {\n  const x = req.query.v; // SEED\n  res.redirect(x); // SINK\n}\n" },
    Case { label: "bare redirect call", source: "function h(input) {\n  const x = input.v; // SEED\n  return redirect(x); // SINK\n}\n" },
    Case { label: "res.write", source: "function h(req, res) {\n  const x = req.query.v; // SEED\n  res.write(x); // SINK\n}\n" },
    Case { label: "disk.write", source: "async function h(disk, input) {\n  const x = input.name; // SEED\n  await disk.write(x, input.stream); // SINK\n}\n" },
    Case { label: "res.setHeader Location", source: "function h(req, res) {\n  const x = req.query.v; // SEED\n  res.setHeader('Location', x); // SINK\n}\n" },
    Case { label: "res.set lowercase location", source: "function h(req, res) {\n  const x = req.query.v; // SEED\n  res.set('location', x); // SINK\n}\n" },
    Case { label: "res.writeHead Location", source: "function h(req, res) {\n  const x = req.query.v; // SEED\n  res.writeHead(302, { Location: x }); // SINK\n}\n" },
    Case { label: "location.replace", source: "function h(json) {\n  const x = json.next; // SEED\n  window.location.replace(x); // SINK\n}\n" },
    Case { label: "res.location", source: "function h(req, res) {\n  const x = req.query.v; // SEED\n  res.location(x); // SINK\n}\n" },
    Case { label: "window.location.href assignment", source: "function h(json) {\n  const x = json.next; // SEED\n  window.location.href = x; // SINK\n}\n" },
    Case { label: "location assignment", source: "function h(json) {\n  const x = json.next; // SEED\n  location = x; // SINK\n}\n" },
];

#[rustfmt::skip]
const POS_DOM_FS: &[Case] = &[
    Case { label: "innerHTML assignment", source: "function h(el, input) {\n  const x = input.v; // SEED\n  el.innerHTML = x; // SINK\n}\n" },
    Case { label: "insertAdjacentHTML", source: "function h(el, input) {\n  const x = input.v; // SEED\n  el.insertAdjacentHTML('beforeend', x); // SINK\n}\n" },
    Case { label: "appendFileSync", source: "function h(fs, input) {\n  const x = input.v; // SEED\n  fs.appendFileSync('a', x); // SINK\n}\n" },
    Case { label: "writeFileSync path", source: "function h(fs, input) {\n  const x = input.v; // SEED\n  fs.writeFileSync(x, 'a'); // SINK\n}\n" },
    Case { label: "unlinkSync", source: "function h(fs, input) {\n  const x = input.v; // SEED\n  fs.unlinkSync(x); // SINK\n}\n" },
];

#[rustfmt::skip]
const NEG_NAME_FRAGMENTS: &[Case] = &[
    Case { label: "sanitizedQuery name", source: "function h(input) {\n  const x = input.v; // SEED\n  const sanitizedQuery = trim(x); // SINK\n}\n" },
    Case { label: "readByQuery call", source: "function h(input) {\n  const x = input.v; // SEED\n  return readByQuery(x); // SINK\n}\n" },
    Case { label: "InvalidQueryError ctor", source: "function h(input) {\n  const x = input.v; // SEED\n  throw new InvalidQueryError(x); // SINK\n}\n" },
    Case { label: "req.query read", source: "function h(req) {\n  const x = req.body.v; // SEED\n  const t = req.query.term + x; // SINK\n}\n" },
    Case { label: "destructured query binding", source: "function h(req, input) {\n  const x = input.v; // SEED\n  const { query } = x; // SINK\n}\n" },
    Case { label: "revalidate (eval substring)", source: "function h(input) {\n  const x = input.v; // SEED\n  revalidate(x); // SINK\n}\n" },
    Case { label: "evaluate (eval substring)", source: "function h(input) {\n  const x = input.v; // SEED\n  evaluate(x); // SINK\n}\n" },
    Case { label: "Array.find", source: "function h(items, input) {\n  const x = input.v; // SEED\n  const hit = items.find(x); // SINK\n}\n" },
    Case { label: "RegExp.exec (ReDoS is not a taint sink)", source: "function h(re, input) {\n  const x = input.v; // SEED\n  const m = re.exec(x); // SINK\n}\n" },
    Case { label: "regex literal exec", source: "function h(input) {\n  const x = input.v; // SEED\n  const m = /a+/.exec(x); // SINK\n}\n" },
    Case { label: "variable named source", source: "function h(input) {\n  const x = input.v; // SEED\n  const source = x; // SINK\n}\n" },
    Case { label: "variable named raw", source: "function h(input) {\n  const x = input.v; // SEED\n  const raw = x; // SINK\n}\n" },
    Case { label: "variable named run", source: "function h(input) {\n  const x = input.v; // SEED\n  const run = x; // SINK\n}\n" },
    Case { label: "variable named redirect", source: "function h(input) {\n  const x = input.v; // SEED\n  const redirect = x; // SINK\n}\n" },
    Case { label: "redirectUrl variable", source: "function h(input) {\n  const x = input.v; // SEED\n  const redirectUrl = x; // SINK\n}\n" },
    Case { label: "property named redirect (read)", source: "function h(input) {\n  const x = input.v; // SEED\n  const t = x.redirect; // SINK\n}\n" },
    Case { label: "removed flag", source: "function h(input) {\n  const x = input.v; // SEED\n  const removed = x; // SINK\n}\n" },
    Case { label: "list.remove", source: "function h(list, input) {\n  const x = input.v; // SEED\n  list.remove(x); // SINK\n}\n" },
    Case { label: "res.render dropped", source: "function h(res, input) {\n  const x = input.v; // SEED\n  res.render('t', x); // SINK\n}\n" },
    Case { label: "emitter send on a non-response receiver", source: "function h(socket, input) {\n  const x = input.v; // SEED\n  socket.send(x); // SINK\n}\n" },
    Case { label: "sendEvent helper", source: "function h(input) {\n  const x = input.v; // SEED\n  sendEvent(x); // SINK\n}\n" },
    Case { label: "write to a non-sink receiver", source: "function h(logger, input) {\n  const x = input.v; // SEED\n  logger.write(x); // SINK\n}\n" },
    Case { label: "Rewrite / system key", source: "function h(input) {\n  const x = input.v; // SEED\n  const systemFields = rewriteFields(x); // SINK\n}\n" },
    Case { label: "Function in a type position", source: "function h(input, cb) {\n  const x = input.v; // SEED\n  const ok = x instanceof Function; // SINK\n}\n" },
    Case { label: "freeze (free substring)", source: "function h(input) {\n  const x = input.v; // SEED\n  Object.freeze(x); // SINK\n}\n" },
    Case { label: "widgets (gets substring)", source: "function h(input) {\n  const x = input.v; // SEED\n  const widgets = x; // SINK\n}\n" },
    Case { label: "innerHTML read", source: "function h(el, input) {\n  const x = input.v; // SEED\n  const t = el.innerHTML + x; // SINK\n}\n" },
    Case { label: "bare spawn with no child_process binding", source: "function h(input) {\n  const x = input.v; // SEED\n  spawn(x); // SINK\n}\n" },
    Case { label: "bare fork with no child_process binding", source: "function h(input) {\n  const x = input.v; // SEED\n  fork(x); // SINK\n}\n" },
    Case { label: "exec on an unbound receiver", source: "function h(matcher, input) {\n  const x = input.v; // SEED\n  matcher.exec(x); // SINK\n}\n" },
    Case { label: "child_process imported but not used for this name", source: "const cp = require('child_process');\nfunction h(other, input) {\n  const x = input.v; // SEED\n  other.execSync(x); // SINK\n}\n" },
    Case { label: "location read", source: "function h(input) {\n  const x = input.v; // SEED\n  const here = x + window.location.href; // SINK\n}\n" },
    Case { label: "Map.set with a location key", source: "function h(map, input) {\n  const x = input.v; // SEED\n  map.set('location', x); // SINK\n}\n" },
    Case { label: "str.replace", source: "function h(input) {\n  const x = input.v; // SEED\n  const y = 'a'.replace('a', x); // SINK\n}\n" },
    Case { label: "member named location on data object", source: "function h(item, input) {\n  const x = input.v; // SEED\n  item.location = x; // SINK\n}\n" },
    Case { label: "setHeader for a non-Location header", source: "function h(res, input) {\n  const x = input.v; // SEED\n  res.setHeader('X-Trace', x); // SINK\n}\n" },
];

#[test]
fn js_code_execution_shapes_are_sinks() {
    assert_all_sinks("a.js", Language::JavaScript, POS_CODE_EXEC);
    assert_all_sinks("a.ts", Language::TypeScript, POS_CODE_EXEC);
}

#[test]
fn js_child_process_receivers_bound_by_import_are_sinks() {
    assert_all_sinks("a.js", Language::JavaScript, POS_PROCESS);
    assert_all_sinks("a.ts", Language::TypeScript, POS_PROCESS);
}

#[test]
fn js_db_response_and_redirect_calls_are_sinks() {
    assert_all_sinks("a.js", Language::JavaScript, POS_DB_RESPONSE);
    assert_all_sinks("a.ts", Language::TypeScript, POS_DB_RESPONSE);
}

#[test]
fn js_dom_and_fs_calls_are_sinks() {
    assert_all_sinks("a.js", Language::JavaScript, POS_DOM_FS);
}

#[test]
fn js_name_fragments_are_not_sinks() {
    assert_no_sinks("a.js", Language::JavaScript, NEG_NAME_FRAGMENTS);
    assert_no_sinks("a.ts", Language::TypeScript, NEG_NAME_FRAGMENTS);
}
