// Unit tests of the truth runner itself. Run: node --test tests/fixtures/proto_writes/truth/run_truth.test.mjs
// The cases here are tiny functions made for these tests, written into a temp directory.

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

import { refusalReasons, runAll } from './run_truth.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const familiesPath = path.join(here, 'families.json');
const RUNNER = path.join(here, 'run_truth.mjs');

const PLAIN_PATH_ROW = {
  call: { arg: 1, template: { kind: 'path_string', delimiter: '.', leading: '' }, root: { arg: 0, kinds: ['plain'] } },
};

function workspace(cases, rows) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'run-truth-test-'));
  for (const [name, text] of Object.entries(cases)) fs.writeFileSync(path.join(dir, name), text);
  const rowsPath = path.join(dir, 'rows.jsonl');
  fs.writeFileSync(rowsPath, rows.map((r) => JSON.stringify(r)).join('\n') + '\n');
  return { dir, rowsPath };
}

async function run(cases, rows, extra = {}) {
  const { dir, rowsPath } = workspace(cases, rows);
  try {
    return await runAll({ casesDir: dir, rowsPath, familiesPath, ...extra });
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

const row = (id, file, extra = {}) => ({ id, file, ...PLAIN_PATH_ROW, ...extra });

// Walks the path and writes the value, with no guard at all.
const UNGUARDED = `
module.exports = function set(root, path) {
  const segs = String(path).split('.');
  let o = root;
  for (let i = 0; i < segs.length - 1; i++) {
    if (o[segs[i]] === undefined) o[segs[i]] = {};
    o = o[segs[i]];
  }
  o[segs[segs.length - 1]] = 'yes';
  if (segs.join('.') === 'a.b') o.b = 1;
};
`;

const GUARDED = `
module.exports = function set(root, path) {
  const segs = String(path).split('.');
  if (segs.some((s) => s === '__proto__' || s === 'constructor' || s === 'prototype')) return;
  let o = root;
  for (let i = 0; i < segs.length - 1; i++) {
    if (!Object.hasOwn(o, segs[i])) o[segs[i]] = {};
    o = o[segs[i]];
  }
  o[segs[segs.length - 1]] = 1;
};
`;

test('a function that pollutes through the path walk is pollutes in both modes', async () => {
  const { records } = await run({ 'u.js': UNGUARDED }, [row('u', 'u.js')]);
  const rec = records[0];
  assert.equal(rec.truth, 'pollutes');
  assert.ok(rec.control.every((c) => c.ok), JSON.stringify(rec.control));
  const direct = rec.results.filter((r) => r.family === 'direct');
  assert.deepEqual(direct.map((r) => r.mode).sort(), ['cjs', 'esm']);
  assert.ok(direct.every((r) => r.polluted === true));
  assert.ok(direct[0].diff.some((d) => d.includes('Object.prototype|s:polluted added')));
});

test('a row that pollutes only as sloppy CommonJS is pollutes, with the strict module recorded safe', async () => {
  // `this` is the global object only in a sloppy function called with no receiver.
  const sloppy = `
module.exports = function f(root, path) {
  if (path === '__proto__.polluted') {
    this.constructor.prototype.polluted = 'yes';
    return;
  }
  root.a = { b: 1 };
};
`;
  const { records } = await run({ 's.js': sloppy }, [row('s', 's.js')]);
  const rec = records[0];
  assert.equal(rec.truth, 'pollutes');
  const direct = Object.fromEntries(rec.results.filter((r) => r.family === 'direct').map((r) => [r.mode, r]));
  assert.equal(direct.cjs.polluted, true);
  assert.equal(direct.esm.polluted, false);
  assert.equal(direct.esm.threw, true, 'this is undefined in the strict module');
  assert.ok(rec.control.every((c) => c.ok), JSON.stringify(rec.control));
});

test('a row whose control throws is inadmissible, never safe', async () => {
  const throwing = 'module.exports = function f() { throw new Error("always"); };';
  const { records } = await run({ 't.js': throwing }, [row('t', 't.js')]);
  assert.equal(records[0].truth, 'inadmissible');
  assert.ok(records[0].control.every((c) => c.threw === true && c.ok === false));
  assert.ok(records[0].reasons.some((r) => r.startsWith('control failed')));
});

test('a row whose control runs but lacks its effect is inadmissible', async () => {
  const noop = 'module.exports = function f() {};';
  const { records } = await run({ 'n.js': noop }, [row('n', 'n.js')]);
  assert.equal(records[0].truth, 'inadmissible');
  assert.ok(records[0].control.every((c) => c.threw === false && c.effect === false));
});

test('a case that names http is refused and never executed', async () => {
  const net = 'const http = require("node:http");\nmodule.exports = function f() {};';
  const { records, spawned } = await run({ 'h.js': net }, [row('h', 'h.js')]);
  assert.equal(spawned, 0, 'no child process was started');
  assert.equal(records[0].truth, 'inadmissible');
  assert.equal(records[0].results[0].status, 'refused');
  assert.ok(records[0].reasons.some((r) => r.includes('http')));
});

test('refusal check: modules, fetch and escapes are caught, comments and benign text are not', () => {
  for (const m of ['net', 'http', 'https', 'http2', 'dgram', 'dns', 'tls', 'child_process', 'worker_threads']) {
    assert.ok(refusalReasons(`const m = require('${m}');`).length > 0, m);
    assert.ok(refusalReasons(`const m = require('node:${m}');`).length > 0, `node:${m}`);
    assert.ok(refusalReasons(`import x from "${m}";`).length > 0, `import ${m}`);
    assert.ok(refusalReasons(`const m = await import('${m}');`).length > 0, `import() ${m}`);
  }
  assert.ok(refusalReasons('require("ht\\x74p")').length > 0, 'escaped spelling');
  assert.ok(refusalReasons('require(`http`)').length > 0, 'template literal');
  assert.ok(refusalReasons('fetch("x")').length > 0, 'fetch');
  assert.ok(refusalReasons('globalThis.fetch("x")').length > 0, 'globalThis.fetch');
  assert.ok(refusalReasons('const r = require; r("http")').length > 0, 'aliased require');
  assert.ok(refusalReasons('require("h" + "ttp")').length > 0, 'computed require');
  assert.ok(refusalReasons('eval("1")').length > 0, 'eval');
  assert.ok(refusalReasons('new Function("return 1")').length > 0, 'Function');
  assert.ok(refusalReasons('process.binding("tcp_wrap")').length > 0, 'process.binding');
  assert.deepEqual(refusalReasons('// require("http") and fetch(x)\n/* http */ module.exports = () => 1;'), []);
  assert.deepEqual(refusalReasons('const s = "http fetch net"; module.exports = () => s.length / 2 / 3;'), []);
  assert.deepEqual(refusalReasons('const fs = require("fs"); module.exports = () => /[/"]x/.test("/");'), []);
});

test('a safe function with a passing control is safe, across every root kind', async () => {
  const rows = [
    row('g', 'g.js', {
      call: { ...PLAIN_PATH_ROW.call, root: { arg: 0, kinds: ['plain', 'null_proto', 'class_instance'] } },
    }),
  ];
  const { records } = await run({ 'g.js': GUARDED }, rows);
  const rec = records[0];
  assert.equal(rec.truth, 'safe', JSON.stringify(rec.reasons));
  assert.equal(rec.control.length, 6, '3 root kinds x 2 modes');
  assert.ok(rec.control.every((c) => c.ok));
  assert.ok(rec.results.every((r) => r.polluted === false));
});

test('a function that throws on every pollution payload is safe when its control passed', async () => {
  const throwing = `
module.exports = function set(root, path) {
  if (String(path) !== 'a.b') throw new TypeError('bad path');
  root.a = { b: 1 };
};
`;
  const { records } = await run({ 'x.js': throwing }, [row('x', 'x.js')]);
  assert.equal(records[0].truth, 'safe');
  assert.ok(records[0].results.every((r) => r.threw === true && r.error === 'TypeError'));
});

test('containment: reading a file outside the allowed ones is denied inside the child and recorded as threw', async () => {
  const secret = fs.mkdtempSync(path.join(os.tmpdir(), 'run-truth-secret-'));
  const secretFile = path.join(secret, 'secret.txt');
  fs.writeFileSync(secretFile, 'top secret');
  try {
    const reader = `
const fs = require('fs');
module.exports = function f(root, path) {
  fs.readFileSync(${JSON.stringify(secretFile)}, 'utf8');
};
`;
    const { records } = await run({ 'r.js': reader }, [row('r', 'r.js')]);
    const rec = records[0];
    assert.ok(rec.results.every((r) => r.threw === true && r.code === 'ERR_ACCESS_DENIED'), JSON.stringify(rec.results[0]));
    assert.ok(rec.control.every((c) => c.threw === true && c.code === 'ERR_ACCESS_DENIED'));
    assert.equal(rec.truth, 'inadmissible');
  } finally {
    fs.rmSync(secret, { recursive: true, force: true });
  }
});

test('containment: the child has an empty working directory, a PATH-only environment (macOS adds __CF_USER_TEXT_ENCODING itself) (cases cannot spawn: those modules are refused)', async () => {
  const probe = `
const fs = require('fs');
module.exports = function f(root, path) {
  const leaks = [];
  if (fs.readdirSync('.').length !== 0) leaks.push('cwd not empty');
  if (Object.keys(process.env).some((k) => k !== 'PATH')) leaks.push('env: ' + Object.keys(process.env).join());
  root.leaks = leaks;
  root.a = { b: 1 };
};
`;
  // readdirSync('.') is itself denied by the permission model: that is the strongest form of "empty".
  const { records } = await run({ 'p.js': probe }, [row('p', 'p.js')]);
  assert.ok(records[0].control.every((c) => c.threw === true && c.code === 'ERR_ACCESS_DENIED'));
  const envProbe = `
module.exports = function f(root, path) {
  root.a = { b: Object.keys(process.env).every((k) => k === 'PATH' || k === '__CF_USER_TEXT_ENCODING') ? 1 : 2 };
};
`;
  const env = await run({ 'e.js': envProbe }, [row('e', 'e.js')]);
  assert.equal(env.records[0].truth, 'safe');
});

test('detection covers Array.prototype, Function.prototype, symbol keys and replaced values', async () => {
  const attack = (body) => `
module.exports = function f(root, path) {
  if (String(path) === '__proto__.polluted') { ${body} return; }
  root.a = { b: 1 };
};
`;
  const cases = {
    'arr.js': attack("Array.prototype.polluted = 1;"),
    'fn.js': attack("Function.prototype.polluted = 1;"),
    'sym.js': attack("Object.prototype[Symbol.for('polluted')] = 1;"),
    'repl.js': attack("Array.prototype.indexOf = function indexOf() { return -1; };"),
    'del.js': attack("delete Object.prototype.toString;"),
    'noop.js': attack("Array.prototype.indexOf = Array.prototype.indexOf;"),
  };
  const rows = Object.keys(cases).map((f) => row(f.replace('.js', ''), f));
  const { records } = await run(cases, rows);
  const truth = Object.fromEntries(records.map((r) => [r.id, r.truth]));
  assert.deepEqual(truth, { arr: 'pollutes', fn: 'pollutes', sym: 'pollutes', repl: 'pollutes', del: 'pollutes', noop: 'safe' });
  const diffOf = (id) => records.find((r) => r.id === id).results.find((r) => r.family === 'direct' && r.polluted).diff;
  assert.ok(diffOf('arr').some((d) => d.startsWith('Array.prototype|')));
  assert.ok(diffOf('fn').some((d) => d.startsWith('Function.prototype|')));
  assert.ok(diffOf('sym').some((d) => d.includes('sym#')));
  assert.ok(diffOf('repl').some((d) => d.includes('s:indexOf changed')));
  assert.ok(diffOf('del').some((d) => d.includes('s:toString removed')));
});

test('a row that runs past the limit is recorded as timeout and is inadmissible', async () => {
  const spin = 'module.exports = function f() { for (;;) {} };';
  const { records } = await run({ 'l.js': spin }, [row('l', 'l.js')], { timeoutMs: 600 });
  assert.ok(records[0].results.every((r) => r.status === 'timeout'));
  assert.equal(records[0].truth, 'inadmissible');
});

test('families, array wrapping and templates: other template kinds reach the function as spelled', async () => {
  // The function echoes the shape it received into root so the control's effect can check it.
  const echo = `
module.exports = function f(root, arg) {
  root.seen = JSON.stringify(arg);
  root.a = { b: 1 };
};
`;
  const kinds = [
    { kind: 'path_array' },
    { kind: 'flat_object', delimiter: '.' },
    { kind: 'nested_object' },
    { kind: 'text', syntax: 'argv' },
    { kind: 'text', syntax: 'ini' },
    { kind: 'path_string', delimiter: '/', leading: '/' },
    { kind: 'path_string', delimiter: '[]' },
  ];
  const rows = kinds.map((template, i) => row(`k${i}`, 'k.js', {
    call: { arg: 1, template, root: { arg: 0, kinds: ['plain'] }, array_segments: true },
  }));
  const { records } = await run({ 'k.js': echo }, rows);
  for (const rec of records) {
    assert.equal(rec.truth, 'safe', `${rec.id}: ${JSON.stringify(rec.reasons)}`);
    // every family, plain and array-wrapped (families are data), x 2 modes
    const famCount = JSON.parse(fs.readFileSync(familiesPath, 'utf8')).families.length;
    assert.equal(rec.results.length, famCount * 2 * 2, rec.id);
    assert.ok(rec.results.every((r) => r.status === 'ok' && r.threw === false), rec.id);
  }
});

test('each template spells the control and the direct payload (plain and array-wrapped) as documented', async () => {
  // The function reports the argument it got in a thrown error's name, which the runner records.
  const report = `
module.exports = function f(root, arg) {
  const e = new Error('shape');
  e.name = 'SHAPE ' + JSON.stringify(arg);
  throw e;
};
`;
  const T = (template) => ({ arg: 1, template, root: { arg: 0, kinds: ['plain'] }, array_segments: true });
  const table = [
    [{ kind: 'path_string', delimiter: '.', leading: '' }, '"a.b"', '"__proto__.polluted"', '["__proto__.polluted"]'],
    [{ kind: 'path_string', delimiter: '/', leading: '/' }, '"/a/b"', '"/__proto__/polluted"', '["/__proto__/polluted"]'],
    [{ kind: 'path_string', delimiter: '[]' }, '"a[b]"', '"__proto__[polluted]"', '["__proto__[polluted]"]'],
    [{ kind: 'path_array' }, '["a","b"]', '["__proto__","polluted"]', '[["__proto__"],["polluted"]]'],
    [{ kind: 'flat_object', delimiter: '.' }, '{"a.b":1}', '{"__proto__.polluted":"yes"}', '{"__proto__.polluted":["yes"]}'],
    [{ kind: 'nested_object' }, '{"a":{"b":1}}', '{"__proto__":{"polluted":"yes"}}', '{"__proto__":[{"polluted":"yes"}]}'],
    [{ kind: 'text', syntax: 'argv' }, '["--a.b=1"]', '["--__proto__.polluted=yes"]', '["--__proto__[].polluted[]=yes"]'],
    [{ kind: 'text', syntax: 'ini' }, '"[a]\\nb=1\\n"', '"[__proto__]\\npolluted=yes\\n"', '"[__proto__]\\npolluted[]=yes\\n"'],
  ];
  const rows = table.map(([template], i) => row(`t${i}`, 'rep.js', { call: T(template) }));
  const { records } = await run({ 'rep.js': report }, rows);
  records.forEach((rec, i) => {
    const [, control, direct, wrapped] = table[i];
    const seen = (r) => r.error.replace(/^SHAPE /, '');
    assert.equal(seen(rec.control[0]), control, `${rec.id} control`);
    const d = rec.results.filter((r) => r.family === 'direct' && r.mode === 'cjs');
    assert.equal(seen(d.find((r) => !r.wrapped)), direct, `${rec.id} direct`);
    assert.equal(seen(d.find((r) => r.wrapped)), wrapped, `${rec.id} wrapped`);
  });
});

test('payload spelling: nested objects keep a reserved own key, and each template shapes its argument', async () => {
  // Pollutes only if the template delivered a real own "__proto__" key through JSON.parse.
  const merge = `
function merge(target, source) {
  for (const key of Object.keys(source)) {
    if (source[key] !== null && typeof source[key] === 'object' && !Array.isArray(source[key])) {
      if (typeof target[key] !== 'object' || target[key] === null) target[key] = {};
      merge(target[key], source[key]);
    } else {
      target[key] = source[key];
    }
  }
}
module.exports = function (root, src) { merge(root, src); };
`;
  const nested = row('m', 'm.js', {
    call: { arg: 1, template: { kind: 'nested_object' }, root: { arg: 0, kinds: ['plain', 'null_proto'] }, array_segments: true },
  });
  const { records } = await run({ 'm.js': merge }, [nested]);
  const rec = records[0];
  assert.equal(rec.truth, 'pollutes');
  const polluting = rec.results.filter((r) => r.polluted).map((r) => `${r.family}/${r.root}/${r.wrapped}`);
  assert.ok(polluting.includes('direct/plain/false'), polluting.join());
  assert.ok(!polluting.some((p) => p.includes('null_proto') && p.startsWith('direct')), 'on a null-prototype root __proto__ is an ordinary key, so the direct family cannot pollute');
});

test('a TypeScript case runs with built-in type stripping in both modes', async () => {
  const real = `
module.exports = function set(root: Record<string, any>, path: string): void {
  const segs: string[] = String(path).split('.');
  if (segs.includes('__proto__')) return;
  root.a = { b: 1 };
};
`;
  const { records } = await run({ 'c.ts': real }, [row('c', 'c.ts')]);
  assert.equal(records[0].truth, 'safe', JSON.stringify(records[0].reasons));
});

test('--only restricts the rows and the CLI writes the node version first', async () => {
  const { dir, rowsPath } = workspace(
    { 'u.js': UNGUARDED, 'g.js': GUARDED },
    [row('u', 'u.js'), row('g', 'g.js')],
  );
  try {
    const out = path.join(dir, 'truth.jsonl');
    const r = spawnSync(process.execPath, [RUNNER, dir, rowsPath, familiesPath, '--out', out, '--only', 'g'], { encoding: 'utf8' });
    assert.equal(r.status, 0, r.stderr);
    const lines = fs.readFileSync(out, 'utf8').trim().split('\n').map((l) => JSON.parse(l));
    assert.equal(lines.length, 2);
    assert.equal(lines[0].node, process.version);
    assert.match(lines[0].families_sha256, /^[0-9a-f]{64}$/);
    assert.equal(lines[1].id, 'g');
    assert.equal(lines[1].truth, 'safe');
    // Deterministic: the same inputs give byte-identical output.
    const out2 = path.join(dir, 'truth2.jsonl');
    const r2 = spawnSync(process.execPath, [RUNNER, dir, rowsPath, familiesPath, '--out', out2, '--only', 'g'], { encoding: 'utf8' });
    assert.equal(r2.status, 0, r2.stderr);
    assert.equal(fs.readFileSync(out, 'utf8'), fs.readFileSync(out2, 'utf8'));
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('an extended-root failure cannot make `truth` inadmissible; it only marks truth_extended', async () => {
  const fn = `
module.exports = function f(root, path) {
  if (typeof root === 'function') throw new Error('no constructor roots');
  const s = String(path).split('.');
  if (s.some((x) => x === '__proto__' || x === 'constructor' || x === 'prototype')) return;
  root[s[0]] = root[s[0]] || {};
  root[s[0]][s[1]] = 1;
};
`;
  const r = row('x', 'x.js', { call: { ...PLAIN_PATH_ROW.call, root: { arg: 0, kinds: ['plain', 'object_ctor'] } } });
  const { records } = await run({ 'x.js': fn }, [r]);
  assert.equal(records[0].truth, 'safe');
  assert.equal(records[0].truth_extended, 'inadmissible');
});

test('a row whose roots are all extended kinds is rejected', async () => {
  const r = row('y', 'y.js', { call: { ...PLAIN_PATH_ROW.call, root: { arg: 0, kinds: ['object_ctor'] } } });
  const { records } = await run({ 'y.js': GUARDED }, [r]);
  assert.equal(records[0].truth, 'inadmissible');
  assert.match(JSON.stringify(records[0].reasons), /no design root/);
});
