#!/usr/bin/env node
// Truth by execution for the prototype-write matrix (DESIGN section 7, stages T and E).
//
//   node run_truth.mjs <cases dir> <rows.jsonl> <families.json> [--out truth.jsonl] [--only id,id]
//
// Only fixture functions written for this lane are executed, each in a fresh contained child
// process: `node --permission` with read access to the generated case file and the generated
// driver only, an empty working directory, an environment holding only PATH, and a time limit.
// Node 24's permission model does not cover the network, so a case file that names a network or
// process-spawning module, calls fetch, or uses a dynamic-code or computed-module escape is
// refused and never run.

import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const MODES = ['cjs', 'esm'];
const DEFAULT_TIMEOUT_MS = 5000;
const BANNED_MODULES = new Set([
  'net', 'http', 'https', 'http2', 'dgram', 'dns', 'tls', 'child_process', 'worker_threads',
]);
const BANNED_IDENTS = new Map([
  ['fetch', 'calls fetch'],
  ['WebSocket', 'uses WebSocket'],
  ['EventSource', 'uses EventSource'],
  ['eval', 'uses eval'],
]);

// ---------------------------------------------------------------------------------------------
// Refusal check: a small tokeniser (comments, strings with escapes, templates, regex literals).
// ---------------------------------------------------------------------------------------------

const REGEX_AFTER_KEYWORD = new Set([
  'return', 'typeof', 'instanceof', 'in', 'of', 'new', 'delete', 'void', 'throw', 'case', 'do',
  'else', 'yield', 'await',
]);

function decodeEscape(src, j) {
  // src[j] is the character after a backslash; returns [text, nextIndex].
  const c = src[j];
  const simple = { n: '\n', r: '\r', t: '\t', b: '\b', f: '\f', v: '\v', 0: '\0' };
  if (c === 'x' && /^[0-9a-fA-F]{2}$/.test(src.slice(j + 1, j + 3))) {
    return [String.fromCharCode(parseInt(src.slice(j + 1, j + 3), 16)), j + 3];
  }
  if (c === 'u') {
    if (src[j + 1] === '{') {
      const e = src.indexOf('}', j + 2);
      const hex = e < 0 ? '' : src.slice(j + 2, e);
      if (/^[0-9a-fA-F]+$/.test(hex) && parseInt(hex, 16) <= 0x10ffff) {
        return [String.fromCodePoint(parseInt(hex, 16)), e + 1];
      }
    } else if (/^[0-9a-fA-F]{4}$/.test(src.slice(j + 1, j + 5))) {
      return [String.fromCharCode(parseInt(src.slice(j + 1, j + 5), 16)), j + 5];
    }
  }
  if (c === '\r' && src[j + 1] === '\n') return ['', j + 2];
  if (c === '\n' || c === '\r') return ['', j + 1];
  if (c === undefined) return ['', j + 1];
  return [Object.hasOwn(simple, c) ? simple[c] : c, j + 1];
}

function lex(src, start = 0, nested = false, toks = []) {
  let i = start;
  let depth = 0;
  const prevAllowsRegex = () => {
    const t = toks[toks.length - 1];
    if (!t) return true;
    if (t.type === 'ident') return REGEX_AFTER_KEYWORD.has(t.value);
    if (t.type === 'punct') return t.value !== ')' && t.value !== ']';
    return false;
  };
  while (i < src.length) {
    const c = src[i];
    if (/\s/.test(c)) { i++; continue; }
    if (c === '/' && src[i + 1] === '/') {
      while (i < src.length && src[i] !== '\n') i++;
      continue;
    }
    if (c === '/' && src[i + 1] === '*') {
      const e = src.indexOf('*/', i + 2);
      i = e < 0 ? src.length : e + 2;
      continue;
    }
    if (c === '"' || c === "'") {
      let j = i + 1;
      let s = '';
      while (j < src.length && src[j] !== c && src[j] !== '\n') {
        if (src[j] === '\\') {
          const [t, nj] = decodeEscape(src, j + 1);
          s += t;
          j = nj;
        } else {
          s += src[j++];
        }
      }
      toks.push({ type: 'string', value: s });
      i = j + 1;
      continue;
    }
    if (c === '`') {
      let j = i + 1;
      let s = '';
      let subst = false;
      while (j < src.length && src[j] !== '`') {
        if (src[j] === '\\') {
          const [t, nj] = decodeEscape(src, j + 1);
          s += t;
          j = nj;
        } else if (src[j] === '$' && src[j + 1] === '{') {
          subst = true;
          j = lex(src, j + 2, true, toks);
        } else {
          s += src[j++];
        }
      }
      toks.push({ type: subst ? 'template' : 'string', value: s });
      i = j + 1;
      continue;
    }
    if (c === '/' && prevAllowsRegex()) {
      let j = i + 1;
      let inClass = false;
      while (j < src.length && src[j] !== '\n') {
        if (src[j] === '\\') { j += 2; continue; }
        if (src[j] === '[') inClass = true;
        else if (src[j] === ']') inClass = false;
        else if (src[j] === '/' && !inClass) break;
        j++;
      }
      j++;
      while (j < src.length && /[a-z]/i.test(src[j])) j++;
      toks.push({ type: 'regex', value: '' });
      i = j;
      continue;
    }
    if (/[A-Za-z_$\u0080-￿]/.test(c)) {
      let j = i + 1;
      while (j < src.length && /[\w$\u0080-￿]/.test(src[j])) j++;
      toks.push({ type: 'ident', value: src.slice(i, j) });
      i = j;
      continue;
    }
    if (/[0-9]/.test(c)) {
      let j = i + 1;
      while (j < src.length && /[\w.]/.test(src[j])) j++;
      toks.push({ type: 'num', value: src.slice(i, j) });
      i = j;
      continue;
    }
    if (c === '{') depth++;
    if (c === '}') {
      if (nested && depth === 0) return i + 1;
      depth--;
    }
    toks.push({ type: 'punct', value: c });
    i++;
  }
  return nested ? i : toks;
}

function bannedSpecifier(spec) {
  const base = spec.replace(/^node:/, '').split('/')[0];
  return BANNED_MODULES.has(base);
}

/** Returns a list of refusal reasons for a case text (empty: acceptable to run). */
export function refusalReasons(text) {
  const reasons = [];
  const toks = lex(text);
  const at = (k) => toks[k] ?? { type: 'eof', value: '' };
  for (let k = 0; k < toks.length; k++) {
    const t = toks[k];
    if (t.type === 'punct' && t.value === '\\') reasons.push('escaped identifier spelling');
    if (t.type === 'string' && /^node:/.test(t.value) && bannedSpecifier(t.value)) {
      reasons.push(`names module ${t.value}`);
    }
    if (t.type !== 'ident') continue;
    if (BANNED_IDENTS.has(t.value)) reasons.push(BANNED_IDENTS.get(t.value));
    if (t.value === 'Function' && (at(k + 1).value === '(' || at(k - 1).value === 'new')) {
      reasons.push('uses the Function constructor');
    }
    if (t.value === 'process' && at(k + 1).value === '.') {
      const m = at(k + 2).value;
      if (m === 'binding' || m === 'dlopen' || m === 'mainModule' || m === '_linkedBinding') {
        reasons.push(`uses process.${m}`);
      }
    }
    if (t.value === 'require') {
      const lit = at(k + 1).value === '(' && at(k + 2).type === 'string' && at(k + 3).value === ')';
      if (!lit) reasons.push('uses require other than require("literal")');
      else if (bannedSpecifier(at(k + 2).value)) reasons.push(`names module ${at(k + 2).value}`);
    }
    if (t.value === 'import') {
      if (at(k + 1).value === '(') {
        if (at(k + 2).type === 'string' && at(k + 3).value === ')') {
          if (bannedSpecifier(at(k + 2).value)) reasons.push(`names module ${at(k + 2).value}`);
        } else {
          reasons.push('uses import() with a computed specifier');
        }
      } else if (at(k + 1).type === 'string') {
        if (bannedSpecifier(at(k + 1).value)) reasons.push(`names module ${at(k + 1).value}`);
      }
    }
    if (t.value === 'from' && at(k + 1).type === 'string' && bannedSpecifier(at(k + 1).value)) {
      reasons.push(`names module ${at(k + 1).value}`);
    }
  }
  return [...new Set(reasons)];
}

// ---------------------------------------------------------------------------------------------
// The driver: its source is generated from this function and written into the run's temp dir.
// It runs inside the contained child.
// ---------------------------------------------------------------------------------------------

async function driverMain() {
  // Intrinsics captured before anything can pollute a prototype.
  const { createRequire } = process.getBuiltinModule('node:module');
  const { pathToFileURL: toUrl } = process.getBuiltinModule('node:url');
  const ownKeys = Reflect.ownKeys;
  const gopd = Reflect.getOwnPropertyDescriptor;
  const hasOwn = Object.hasOwn;
  const apply = Reflect.apply;
  const jsonParse = JSON.parse;
  const jsonStringify = JSON.stringify;
  const symToString = Symbol.prototype.toString;
  const strReplace = String.prototype.replace;
  const strCharCodeAt = String.prototype.charCodeAt;
  const mapGet = Map.prototype.get;
  const mapSet = Map.prototype.set;
  const mapHas = Map.prototype.has;
  const arrPush = Array.prototype.push;
  const arrSlice = Array.prototype.slice;
  const arrJoin = Array.prototype.join;
  const arrMap = Array.prototype.map;
  const NewMap = Map;
  const NewFunction = Function;
  const nullProtoObject = Object.create;
  const stdoutWrite = process.stdout.write.bind(process.stdout);
  const exit = process.exit.bind(process);

  const spec = jsonParse(process.argv[2]);

  // ---- snapshot of the three shared prototype objects
  const TARGETS = [
    ['Object.prototype', Object.prototype],
    ['Array.prototype', Array.prototype],
    ['Function.prototype', Function.prototype],
  ];
  const ids = new NewMap();
  let nextId = 0;
  const idOf = (v) => {
    const isRef = (typeof v === 'object' && v !== null) || typeof v === 'function' || typeof v === 'symbol';
    if (isRef) {
      let i = apply(mapGet, ids, [v]);
      if (i === undefined) { i = ++nextId; apply(mapSet, ids, [v, i]); }
      return 'o' + i;
    }
    return typeof v + ':' + String(v);
  };
  const keyText = (k) => (typeof k === 'symbol' ? 'sym#' + idOf(k) + ':' + apply(symToString, k, []) : 's:' + k);
  const snapshot = () => {
    const out = new NewMap();
    for (const [name, obj] of TARGETS) {
      for (const k of ownKeys(obj)) {
        const d = gopd(obj, k);
        const flags = (d.enumerable ? 'e' : '-') + (d.configurable ? 'c' : '-');
        const text = hasOwn(d, 'value')
          ? 'v:' + idOf(d.value) + ':' + (d.writable ? 'w' : '-') + flags
          : 'a:' + idOf(d.get) + ',' + idOf(d.set) + ':' + flags;
        apply(mapSet, out, [name + '|' + keyText(k), text]);
      }
    }
    return out;
  };
  const diffOf = (before, after) => {
    const changes = [];
    for (const [k, v] of before) {
      if (!apply(mapHas, after, [k])) apply(arrPush, changes, [k + ' removed']);
      else if (apply(mapGet, after, [k]) !== v) apply(arrPush, changes, [k + ' changed']);
    }
    for (const [k] of after) {
      if (!apply(mapHas, before, [k])) apply(arrPush, changes, [k + ' added']);
    }
    return changes;
  };

  // ---- output without any inherited-property or toJSON dependence
  const q = (s) => '"' + apply(strReplace, String(s), [/[^\x20-\x21\x23-\x5b\x5d-\x7e]/g,
    (c) => '\\u' + ('0000' + apply(strCharCodeAt, c, [0]).toString(16)).slice(-4)]) + '"';
  const emit = (r) => {
    const line = '@@RESULT {"threw":' + (r.threw ? 'true' : 'false') +
      ',"load_error":' + (r.loadError ? 'true' : 'false') +
      ',"error":' + (r.errName === null ? 'null' : q(r.errName)) +
      ',"code":' + (r.errCode === null ? 'null' : q(r.errCode)) +
      ',"effect":' + (r.effect === null ? 'null' : r.effect ? 'true' : 'false') +
      ',"diff":[' + apply(arrJoin, apply(arrMap, r.diff, [q]), [',']) + ']}\n';
    stdoutWrite(line, () => exit(0));
  };

  // ---- payload from the row's template
  const joinPath = (t, segs) => {
    if (t.delimiter === '[]') {
      return segs[0] + apply(arrJoin, apply(arrMap, apply(arrSlice, segs, [1]), [(s) => '[' + s + ']']), ['']);
    }
    return (t.leading ?? '') + apply(arrJoin, segs, [t.delimiter ?? '.']);
  };
  const nestedText = (segs, wrapped, value) => {
    let s = jsonStringify(value);
    for (let k = segs.length - 1; k >= 0; k--) {
      const lvl = '{' + jsonStringify(segs[k]) + ':' + s + '}';
      s = wrapped && k > 0 ? '[' + lvl + ']' : lvl;
    }
    return s;
  };
  const buildPayload = (t, segs, wrapped, value) => {
    switch (t.kind) {
      case 'path_string': {
        const s = joinPath(t, segs);
        return wrapped ? [s] : s;
      }
      case 'path_array':
        return wrapped ? apply(arrMap, segs, [(s) => [s]]) : apply(arrSlice, segs, []);
      case 'flat_object': {
        const v = jsonStringify(value);
        return jsonParse('{' + jsonStringify(apply(arrJoin, segs, [t.delimiter ?? '.'])) + ':' + (wrapped ? '[' + v + ']' : v) + '}');
      }
      case 'nested_object':
        return jsonParse(nestedText(segs, wrapped, value));
      case 'text': {
        if (t.syntax === 'argv') {
          const parts = wrapped ? apply(arrMap, segs, [(s) => s + '[]']) : segs;
          return ['--' + apply(arrJoin, parts, ['.']) + '=' + String(value)];
        }
        // ini
        const last = segs[segs.length - 1] + (wrapped ? '[]' : '');
        const sections = apply(arrSlice, segs, [0, -1]);
        const head = sections.length ? '[' + apply(arrJoin, sections, ['.']) + ']\n' : '';
        return head + last + '=' + String(value) + '\n';
      }
      default:
        throw new Error('unknown template kind ' + t.kind);
    }
  };
  const makeRoot = (kind) => {
    switch (kind) {
      case 'plain': return {};
      case 'null_proto': return nullProtoObject(null);
      case 'class_instance': return new (class Root {})();
      case 'object_ctor': return Object;
      case 'global_this': return globalThis;
      case 'array': return [];
      default: throw new Error('unknown root kind ' + kind);
    }
  };

  // ---- run
  const before = snapshot();
  let threw = false;
  let loadError = false;
  let errName = null;
  let errCode = null;
  let effect = null;
  const note = (e) => {
    threw = true;
    try {
      errName = String((e && e.name) || 'Error');
      errCode = e && typeof e.code === 'string' ? e.code : null;
    } catch {
      errName = 'Error';
    }
  };
  try {
    let exp;
    try {
      if (spec.mode === 'cjs') exp = createRequire(import.meta.url)(spec.file);
      else exp = (await import(toUrl(spec.file).href)).default;
    } catch (e) {
      loadError = true;
      throw e;
    }
    let fn;
    let holder;
    if (typeof exp === 'function' && !spec.exportName) {
      fn = exp;
    } else {
      if (exp === null || (typeof exp !== 'object' && typeof exp !== 'function')) {
        loadError = true;
        throw new Error('the case exports no function');
      }
      const names = spec.exportName
        ? [spec.exportName]
        : ownKeys(exp).filter((k) => typeof k === 'string' && typeof exp[k] === 'function');
      if (names.length !== 1 || typeof exp[names[0]] !== 'function') {
        loadError = true;
        throw new Error('the case does not export a single function');
      }
      holder = exp;
      fn = exp[names[0]];
    }
    const call = spec.call;
    const payload = buildPayload(call.template, spec.segments, spec.wrapped, spec.value);
    const given = call.args === undefined || call.args === null ? [] : call.args;
    const args = [];
    if (Array.isArray(given)) {
      for (let k = 0; k < given.length; k++) args[k] = given[k];
    } else {
      for (const k of ownKeys(given)) args[Number(k)] = given[k];
    }
    let root;
    if (call.root) {
      root = makeRoot(spec.rootKind);
      args[call.root.arg] = root;
    }
    args[call.arg] = payload;
    for (let k = 0; k < args.length; k++) if (!(k in args)) args[k] = undefined;
    let ret = apply(fn, holder, args);
    if (ret !== null && (typeof ret === 'object' || typeof ret === 'function') && typeof ret.then === 'function') {
      ret = await ret;
    }
    if (spec.expect !== null) {
      try {
        effect = !!new NewFunction('root', 'ret', 'result', 'return (' + spec.expect + ')')(root, ret, ret);
      } catch {
        effect = false;
      }
    }
  } catch (e) {
    note(e);
  }
  const after = snapshot();
  emit({ threw, loadError, errName, errCode, effect, diff: diffOf(before, after) });
}

// ---------------------------------------------------------------------------------------------
// The runner.
// ---------------------------------------------------------------------------------------------

function readRows(rowsPath) {
  const rows = [];
  const lines = fs.readFileSync(rowsPath, 'utf8').split('\n');
  for (let n = 0; n < lines.length; n++) {
    const line = lines[n].trim();
    if (line === '' ) continue;
    try {
      rows.push(JSON.parse(line));
    } catch (e) {
      throw new Error(`${rowsPath}:${n + 1}: ${e.message}`);
    }
  }
  return rows;
}

function resolveCase(row, casesDir, rowsPath) {
  const tries = [
    path.resolve(casesDir, row.file ?? ''),
    path.resolve(casesDir, path.basename(row.file ?? '')),
    path.resolve(path.dirname(rowsPath), row.file ?? ''),
  ];
  for (const p of tries) {
    try {
      if (fs.statSync(p).isFile()) return p;
    } catch { /* try the next spelling */ }
  }
  return null;
}

function validateRow(row) {
  if (typeof row.id !== 'string' || row.id === '') return 'row has no id';
  const c = row.call;
  if (!c || typeof c !== 'object') return 'row has no call';
  if (row.extra_payloads !== undefined && (!Array.isArray(row.extra_payloads) || row.extra_payloads.some((x) => typeof x?.id !== 'string' || !Array.isArray(x.segments)))) {
    return 'extra_payloads must be a list of {id, segments}';
  }
  if (!Number.isInteger(c.arg) || c.arg < 0) return 'call.arg is not a position';
  const t = c.template;
  if (!t || !['path_string', 'path_array', 'flat_object', 'nested_object', 'text'].includes(t.kind)) {
    return 'call.template.kind is not recognised';
  }
  if (t.kind === 'text' && !['argv', 'ini'].includes(t.syntax)) return 'call.template.syntax is not argv or ini';
  if (c.root) {
    if (!Number.isInteger(c.root.arg) || c.root.arg < 0) return 'call.root.arg is not a position';
    const known = ['plain', 'null_proto', 'class_instance', 'array', 'object_ctor', 'global_this'];
    if (!Array.isArray(c.root.kinds) || c.root.kinds.length === 0 || c.root.kinds.some((k) => !known.includes(k))) {
      return 'call.root.kinds is empty or names an unknown kind';
    }
    if (c.root.kinds.every((k) => k === 'object_ctor' || k === 'global_this')) return 'call.root.kinds has no design root kind';
    if (c.root.arg === c.arg) return 'call.root.arg equals call.arg';
  }
  return null;
}

function defaultExpect(segments, value) {
  const chain = segments.map((s) => `[${JSON.stringify(s)}]`).join('');
  return `String((root ?? ret)${chain}) === ${JSON.stringify(String(value))}`;
}

function runChild({ driverPath, caseFile, spec, tmpRoot, timeoutMs, counters }) {
  return new Promise((resolve) => {
    const cwd = fs.mkdtempSync(path.join(tmpRoot, 'cwd-'));
    counters.spawned++;
    const child = spawn(
      process.execPath,
      [
        '--permission',
        `--allow-fs-read=${caseFile}`,
        `--allow-fs-read=${driverPath}`,
        driverPath,
        JSON.stringify(spec),
      ],
      { cwd, env: { PATH: process.env.PATH ?? '' }, stdio: ['ignore', 'pipe', 'pipe'] },
    );
    let out = '';
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      child.kill('SIGKILL');
    }, timeoutMs);
    child.stdout.on('data', (d) => { if (out.length < 1 << 20) out += d; });
    child.stderr.on('data', () => {});
    const finish = (extra) => {
      clearTimeout(timer);
      fs.rmSync(cwd, { recursive: true, force: true });
      resolve(extra);
    };
    child.on('error', () => finish({ status: 'driver_error' }));
    child.on('close', () => {
      if (timedOut) return finish({ status: 'timeout' });
      const line = out.split('\n').reverse().find((l) => l.startsWith('@@RESULT '));
      if (!line) return finish({ status: 'driver_error' });
      try {
        const r = JSON.parse(line.slice('@@RESULT '.length));
        finish({ status: 'ok', ...r });
      } catch {
        finish({ status: 'driver_error' });
      }
    });
  });
}

async function pool(tasks, width) {
  let next = 0;
  const workers = Array.from({ length: Math.min(width, tasks.length) }, async () => {
    while (next < tasks.length) {
      const k = next++;
      await tasks[k]();
    }
  });
  await Promise.all(workers);
}

function resultRecord(kind, r) {
  return {
    ...kind,
    status: r.status,
    polluted: r.status === 'ok' ? r.diff.length > 0 : null,
    threw: r.status === 'ok' ? r.threw : null,
    load_error: r.status === 'ok' ? r.load_error : null,
    error: r.status === 'ok' ? r.error : null,
    code: r.status === 'ok' ? r.code : null,
    diff: r.status === 'ok' ? r.diff : [],
  };
}

const EXTENDED_ROOTS = new Set(['object_ctor', 'global_this']);

export async function runAll({
  casesDir,
  rowsPath,
  familiesPath,
  only = null,
  timeoutMs = DEFAULT_TIMEOUT_MS,
  width = Math.min(8, os.availableParallelism()),
}) {
  const familiesText = fs.readFileSync(familiesPath);
  const fam = JSON.parse(familiesText);
  const header = {
    node: process.version,
    families_sha256: createHash('sha256').update(familiesText).digest('hex'),
  };
  const attackValue = fam.value ?? 'yes';
  const controlSegs = fam.control?.segments ?? ['a', 'b'];
  const controlValue = fam.control?.value ?? 1;
  let rows = readRows(rowsPath);
  if (only) rows = rows.filter((r) => only.includes(r.id));

  const tmpRoot = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'run-truth-')));
  const counters = { spawned: 0 };
  try {
    const driverPath = path.join(tmpRoot, 'driver.mjs');
    fs.writeFileSync(driverPath, `(${driverMain.toString()})().catch(() => process.exit(3));\n`);

    const records = rows.map(() => null);
    const tasks = [];
    rows.forEach((row, idx) => {
      const base = { id: row.id, file: row.file ?? null };
      const bad = validateRow(row);
      if (bad) {
        records[idx] = { ...base, truth: 'inadmissible', reasons: [`bad row: ${bad}`], polluted_observed: false, results: [], control: [] };
        return;
      }
      const caseFile = resolveCase(row, casesDir, rowsPath);
      if (!caseFile) {
        records[idx] = { ...base, truth: 'inadmissible', reasons: ['case file not found'], polluted_observed: false, results: [], control: [] };
        return;
      }
      const text = fs.readFileSync(caseFile, 'utf8');
      const refusal = refusalReasons(text);
      if (refusal.length > 0) {
        const r = { status: 'refused', reasons: refusal };
        records[idx] = { ...base, truth: 'inadmissible', reasons: refusal.map((x) => `refused: ${x}`), polluted_observed: false, results: [r], control: [] };
        return;
      }
      // Both wrappers are generated from the one case text.
      const isTs = caseFile.endsWith('.ts');
      const rowDir = path.join(tmpRoot, `row-${idx}`);
      fs.mkdirSync(rowDir);
      const files = {
        cjs: path.join(rowDir, isTs ? 'case.cts' : 'case.cjs'),
        esm: path.join(rowDir, isTs ? 'case.mts' : 'case.mjs'),
      };
      fs.writeFileSync(files.cjs, text);
      fs.writeFileSync(
        files.esm,
        [
          "import { createRequire as __createRequire } from 'node:module';",
          'const __require = __createRequire(import.meta.url);',
          'function __load(module, exports, require) {',
          "'use strict';",
          text,
          '\n}',
          'const __module = { exports: {} };',
          '__load(__module, __module.exports, __require);',
          'export default __module.exports;',
          '',
        ].join('\n'),
      );

      const call = row.call;
      const roots = call.root ? call.root.kinds : ['none'];
      const useWrapped = call.array_segments === true;
      const attack = [];
      for (const f of fam.families) {
        attack.push({ family: f.id, wrapped: false, segments: f.segments });
        if (useWrapped && f.array_wrapping === true) attack.push({ family: f.id, wrapped: true, segments: f.segments });
      }
      // Row-specific payloads: segment lists that name something only this row's function has
      // (an application key, a reversed order, an encoded name). Same containment, same detection.
      for (const x of row.extra_payloads ?? []) {
        attack.push({ family: `extra:${x.id}`, wrapped: false, segments: x.segments });
      }
      const cSegs = call.control?.segments ?? controlSegs;
      const cValue = call.control?.value ?? controlValue;
      const cExpect = call.control?.expect ?? defaultExpect(cSegs, cValue);

      const results = [];
      const controls = [];
      for (const rootKind of roots) {
        for (const mode of MODES) {
          for (const a of attack) {
            const slot = results.length;
            results.push(null);
            tasks.push(async () => {
              const spec = {
                mode, file: files[mode], call, segments: a.segments, wrapped: a.wrapped,
                value: attackValue, rootKind, expect: null, exportName: call.export ?? null,
              };
              const r = await runChild({ driverPath, caseFile: files[mode], spec, tmpRoot, timeoutMs, counters });
              results[slot] = resultRecord({ family: a.family, wrapped: a.wrapped, root: rootKind, mode }, r);
            });
          }
          const cslot = controls.length;
          controls.push(null);
          tasks.push(async () => {
            const spec = {
              mode, file: files[mode], call, segments: cSegs, wrapped: false,
              value: cValue, rootKind, expect: cExpect, exportName: call.export ?? null,
            };
            const r = await runChild({ driverPath, caseFile: files[mode], spec, tmpRoot, timeoutMs, counters });
            const rec = resultRecord({ root: rootKind, mode }, r);
            const effect = r.status === 'ok' ? r.effect : null;
            controls[cslot] = {
              ...rec,
              effect,
              ok: r.status === 'ok' && !r.threw && r.diff.length === 0 && effect === true,
            };
          });
        }
      }
      records[idx] = { ...base, truth: null, reasons: [], polluted_observed: false, results, control: controls };
    });

    await pool(tasks, width);

    for (const rec of records) {
      if (rec.truth !== null) continue;
      // `truth` is judged over the design's four root kinds; roots in EXTENDED_ROOTS (a function that
      // is handed a built-in constructor or the global object as its root) only feed `truth_extended`.
      const core = (x) => !EXTENDED_ROOTS.has(x.root);
      const ext = (x) => EXTENDED_ROOTS.has(x.root);
      const judge = (results, control) => {
        const reasons = [];
        for (const c of control.filter((c) => !c.ok)) {
          reasons.push(`control failed (${c.root}, ${c.mode}): ${c.status !== 'ok' ? c.status : c.threw ? 'threw ' + c.error : c.polluted ? 'polluted' : 'effect not observed'}`);
        }
        for (const r of results.filter((r) => r.status !== 'ok')) {
          reasons.push(`payload run ${r.status} (${r.family}${r.wrapped ? '+array' : ''}, ${r.root}, ${r.mode})`);
        }
        const polluted = results.some((r) => r.polluted === true);
        return { reasons, polluted, truth: reasons.length > 0 ? 'inadmissible' : polluted ? 'pollutes' : 'safe' };
      };
      const coreJ = judge(rec.results.filter(core), rec.control.filter(core));
      rec.polluted_observed = rec.results.some((r) => r.polluted === true);
      rec.truth = coreJ.truth;
      rec.reasons = coreJ.reasons;
      if (rec.results.some(ext)) {
        const extJ = judge(rec.results.filter(ext), rec.control.filter(ext));
        rec.truth_extended = coreJ.truth === 'inadmissible' ? 'inadmissible'
          : extJ.truth === 'inadmissible' ? 'inadmissible'
          : coreJ.polluted || extJ.polluted ? 'pollutes' : 'safe';
        if (extJ.reasons.length > 0) rec.reasons_extended = extJ.reasons;
      }
    }
    return { header, records, spawned: counters.spawned };
  } finally {
    fs.rmSync(tmpRoot, { recursive: true, force: true });
  }
}

export function formatJsonl({ header, records }) {
  return [header, ...records].map((r) => JSON.stringify(r)).join('\n') + '\n';
}

async function main(argv) {
  const pos = [];
  let out = null;
  let only = null;
  for (let k = 0; k < argv.length; k++) {
    if (argv[k] === '--out') out = argv[++k];
    else if (argv[k] === '--only') only = argv[++k].split(',').filter(Boolean);
    else pos.push(argv[k]);
  }
  if (pos.length !== 3 || (out === null && argv.includes('--out'))) {
    process.stderr.write('usage: run_truth.mjs <cases dir> <rows.jsonl> <families.json> [--out truth.jsonl] [--only id,id]\n');
    return 2;
  }
  const here = path.dirname(fileURLToPath(import.meta.url));
  const result = await runAll({
    casesDir: path.resolve(pos[0]),
    rowsPath: path.resolve(pos[1]),
    familiesPath: path.resolve(pos[2]),
    only,
  });
  fs.writeFileSync(out ?? path.join(here, 'truth.jsonl'), formatJsonl(result));
  const count = (t) => result.records.filter((r) => r.truth === t).length;
  process.stderr.write(
    `${result.records.length} rows: ${count('pollutes')} pollutes, ${count('safe')} safe, ${count('inadmissible')} inadmissible (${result.spawned} child processes)\n`,
  );
  return count('inadmissible') > 0 ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2)).then((code) => { process.exitCode = code; }, (e) => {
    process.stderr.write(`${e.stack ?? e}\n`);
    process.exitCode = 2;
  });
}
