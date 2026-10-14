#!/usr/bin/env node
'use strict';
// Lane A2 step 0: census of calls inside anonymous JS/TS callables. Static measurement only:
// parses the corpus with the pinned TypeScript 5.9.3 checker, never executes corpus code.
//   node anon-census.cjs <repo-root> <base-sites.jsonl> <proto-sites.jsonl|-> <out-dir> [--aggregate-only] [--private-root]
// base-sites: `prism nav --no-cache call-stats --dump-sites` from main.
// proto-sites: the same from the step-0 prototype (anonymous callables own their calls), or `-`.
// --aggregate-only writes aggregate.json only (no file names, no per-site rows).
const fs = require('fs');
const path = require('path');
const TS_PATH = '/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js';
const ts = require(TS_PATH);
if (ts.version !== '5.9.3') throw Error('pinned checker mismatch');

const [rootArg, baseFile, protoFile, outArg, ...flags] = process.argv.slice(2);
const aggregateOnly = flags.includes('--aggregate-only');
const privateRoot = flags.includes('--private-root');
const root = path.resolve(rootArg);
const out = path.resolve(outArg);
process.umask(0o077);

// ---- file walk (same filters as js-arg-follow census.cjs) ---------------------------------------
const forbiddenName = (x) => x === 'frontend-portal' || x.startsWith('f-') || x.startsWith('F-private') || x.startsWith('secbench') || x.startsWith('SB__');
const forbidden = (p) => {
  const rel = path.relative(root, path.resolve(p));
  if (rel.startsWith('..')) return path.resolve(p).split(path.sep).some(forbiddenName);
  return !privateRoot && rel.split(path.sep).some(forbiddenName);
};
const skipDir = new Set(['target', 'node_modules', 'vendor', 'dist', 'build']);
const files = [];
(function walk(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isSymbolicLink()) continue;
    if (!privateRoot && forbiddenName(e.name)) continue;
    if (e.isDirectory()) {
      if (e.name.startsWith('.') || skipDir.has(e.name)) continue;
      walk(p);
    } else if (/\.(js|jsx|mjs|cjs|ts|tsx|mts|cts)$/.test(e.name) && !/\.d\.[mc]?ts$/.test(e.name)) {
      if (fs.statSync(p).size <= 2 * 1024 * 1024) files.push(p);
    }
  }
})(root);
files.sort();
const isTestPath = (f) => /(^|\/)(tests?|__tests__|__mocks__|testRunner|harness|fixtures?)(\/|$)|\.(test|spec)\.[cm]?[jt]sx?$/.test(f);
const inc = (o, k, n = 1) => { o[k] = (Object.prototype.hasOwnProperty.call(o, k) ? o[k] : 0) + n; };

const compilerOptions = {
  allowJs: true, checkJs: true, types: [], jsx: ts.JsxEmit.Preserve, target: ts.ScriptTarget.ESNext,
  module: ts.ModuleKind.ESNext, moduleResolution: ts.ModuleResolutionKind.Bundler, skipLibCheck: true, noEmit: true,
};
const host = ts.createCompilerHost(compilerOptions);
for (const key of ['readFile', 'fileExists', 'directoryExists', 'getDirectories', 'realpath']) {
  const orig = host[key];
  if (orig) host[key] = function (p, ...a) {
    if (forbidden(p)) return key === 'getDirectories' ? [] : ['fileExists', 'directoryExists'].includes(key) ? false : undefined;
    return orig.call(host, p, ...a);
  };
}
const program = ts.createProgram(files, compilerOptions, host);
const checker = program.getTypeChecker();
const inRepo = (sf) => !sf.isDeclarationFile && !path.relative(root, path.resolve(sf.fileName)).startsWith('..') && !/[\\/]node_modules[\\/]/.test(sf.fileName);
const sym = (n) => { try { return checker.getSymbolAtLocation(n); } catch { return undefined; } };
function unalias(s) {
  const seen = new Set();
  while (s && s.flags & ts.SymbolFlags.Alias) {
    if (seen.has(s)) return undefined;
    seen.add(s);
    let t; try { t = checker.getAliasedSymbol(s); } catch { return undefined; }
    if (!t || t === s) return undefined;
    s = t;
  }
  return s;
}
const peel = (n) => { while (n && (ts.isParenthesizedExpression(n) || ts.isAsExpression(n) || ts.isTypeAssertionExpression(n) || ts.isNonNullExpression(n) || ts.isSatisfiesExpression(n))) n = n.expression; return n; };
const upParens = (n) => { let p = n.parent; while (p && ts.isParenthesizedExpression(p)) { n = p; p = p.parent; } return [n, p]; };

function byteIndex(sf, raw) {
  const enc = Buffer.from(sf.text);
  const bom = raw.length === enc.length + 3 && raw[0] === 0xef && raw[1] === 0xbb && raw[2] === 0xbf;
  const a = new Uint32Array(sf.text.length + 1);
  let b = bom ? 3 : 0;
  for (let i = 0; i < sf.text.length; i++) {
    a[i] = b;
    const cp = sf.text.codePointAt(i);
    if (cp > 65535) { a[++i] = b; b += 4; } else b += cp < 128 ? 1 : cp < 2048 ? 2 : 3;
  }
  a[sf.text.length] = b;
  return a;
}

// ---- prism's naming rule (languages/mod.rs::function_name, JS/TS arms), mirrored on the TS AST ----
// tree-sitter keeps parentheses as a node, so a parenthesized callable has no name-giving parent.
function prismNamed(fn) {
  if (ts.isFunctionExpression(fn) && fn.name) return true;
  const p = fn.parent;
  if (!p) return false;
  if (ts.isVariableDeclaration(p) && p.initializer === fn) return true; // P1
  if (ts.isPropertyAssignment(p) && p.initializer === fn) return true; // P2
  if (ts.isCallExpression(p) && p.arguments.includes(fn) && p.parent && ts.isVariableDeclaration(p.parent) && p.parent.initializer === p) return true; // P3
  if (ts.isPropertyDeclaration(p) && p.initializer === fn) return true; // P4
  if (ts.isBinaryExpression(p) && p.right === fn && p.operatorToken.kind === ts.SyntaxKind.EqualsToken
    && (ts.isPropertyAccessExpression(p.left) || ts.isIdentifier(p.left))) return true; // P5
  return false;
}
const isAnonCallable = (n) => (ts.isArrowFunction(n) || ts.isFunctionExpression(n)) && !prismNamed(n);
const isCallableWithBody = (n) => ts.isFunctionLike(n) && !!n.body;

const HOOKS = /^(React\.)?(useEffect|useLayoutEffect|useInsertionEffect|useCallback|useMemo|useState|useReducer|useImperativeHandle|useSyncExternalStore)$/;
const PROMISE = new Set(['then', 'catch', 'finally']);
const ARRAY = new Set(['map', 'forEach', 'filter', 'reduce', 'reduceRight', 'some', 'every', 'find', 'findIndex', 'findLast', 'findLastIndex', 'flatMap', 'sort', 'toSorted']);
const TIMER = /^(window\.)?(setTimeout|setInterval|requestAnimationFrame|requestIdleCallback|queueMicrotask|addEventListener|removeEventListener|on|once|subscribe)$|\.(addEventListener|on|once|subscribe)$/;
const TESTFN = /^(describe|it|test|beforeEach|afterEach|beforeAll|afterAll)(\.\w+)?$/;
const calleeText = (call) => { const e = peel(call.expression); return e ? e.getText() : ''; };
const calleeLast = (call) => { const e = peel(call.expression); return ts.isPropertyAccessExpression(e) ? e.name.text : ts.isIdentifier(e) ? e.text : ''; };

// The syntactic role of an anonymous callable (what creates or receives it).
function kindOf(fn) {
  const [self, p] = upParens(fn);
  if (!p) return 'other';
  if (ts.isCallExpression(p) && peel(p.expression) === fn) return 'iife';
  if ((ts.isCallExpression(p) || ts.isNewExpression(p)) && (p.arguments || []).includes(self)) {
    const t = calleeText(p);
    const last = calleeLast(p);
    if (HOOKS.test(t)) return 'react_hook:' + t.replace(/^React\./, '');
    if (ts.isNewExpression(p)) return /Promise$/.test(t) ? 'new_promise_executor' : 'new_arg';
    if (PROMISE.has(last) && ts.isPropertyAccessExpression(peel(p.expression))) return 'promise_then';
    if (ARRAY.has(last) && ts.isPropertyAccessExpression(peel(p.expression))) return 'array_method';
    if (TIMER.test(t)) return 'timer_or_listener';
    if (TESTFN.test(t)) return 'test_framework';
    const s = sym(peel(p.expression));
    const target = s && unalias(s);
    const d = target && (target.valueDeclaration || (target.declarations || [])[0]);
    return d && inRepo(d.getSourceFile()) ? 'arg_to_repo_fn' : 'arg_to_other';
  }
  if (ts.isJsxExpression(p)) return p.parent && ts.isJsxAttribute(p.parent) ? 'jsx_prop' : 'jsx_child';
  if (ts.isParameter(p) && p.initializer === self) return 'default_param';
  if (ts.isBindingElement(p) && p.initializer === self) return 'default_binding';
  if (ts.isExportAssignment(p)) return 'export_default';
  if (ts.isReturnStatement(p)) return 'returned';
  if (ts.isArrowFunction(p) && p.body === self) return 'returned_from_arrow';
  if (ts.isPropertyAssignment(p) || ts.isShorthandPropertyAssignment(p)) return 'object_property_wrapped';
  if (ts.isPropertyDeclaration(p)) return 'class_field_wrapped';
  if (ts.isVariableDeclaration(p)) return 'variable_wrapped';
  if (ts.isBinaryExpression(p)) return p.operatorToken.kind === ts.SyntaxKind.EqualsToken ? 'assigned_other' : 'operand';
  if (ts.isConditionalExpression(p)) return 'conditional';
  if (ts.isArrayLiteralExpression(p)) return 'array_element';
  if (ts.isAsExpression(p) || ts.isSatisfiesExpression(p) || ts.isNonNullExpression(p) || ts.isTypeAssertionExpression(p)) return 'type_wrapped';
  return 'other_' + ts.SyntaxKind[p.kind];
}
// Nearest enclosing callable with a body (named or not), or the source file.
function enclosingCallable(n) {
  let f = n.parent;
  while (f && !isCallableWithBody(f) && !ts.isSourceFile(f)) f = f.parent;
  return f;
}
// Anonymous nesting depth: anonymous callables from fn up to the nearest named callable or the file.
function anonDepth(fn) {
  let d = 0;
  for (let f = fn; f && !ts.isSourceFile(f); f = enclosingCallable(f)) { if (isAnonCallable(f)) d++; else break; }
  return d;
}
const inClass = (n) => { for (let p = n.parent; p; p = p.parent) if (ts.isClassLike(p)) return true; return false; };
// Mirrors census.cjs (js-arg-follow) classification for the reproduction line.
const NAMED_FE_KINDS = (fn) => {
  let p = fn.parent; while (p && ts.isParenthesizedExpression(p)) p = p.parent;
  if (ts.isVariableDeclaration(p) && ts.isIdentifier(p.name)) return true;
  if (ts.isPropertyAssignment(p) || ts.isPropertyDeclaration(p)) return true;
  if (ts.isBinaryExpression(p)) return true;
  if (ts.isCallExpression(p) && p.arguments[0] === fn) {
    let q = p.parent; while (q && ts.isParenthesizedExpression(q)) q = q.parent;
    const W = new Set(['memo', 'forwardRef', 'observer', 'React.memo', 'React.forwardRef', 'useCallback', 'React.useCallback']);
    if (W.has(p.expression.getText()) && ts.isVariableDeclaration(q) && ts.isIdentifier(q.name)) return true;
  }
  return false;
};

// ---- walk ---------------------------------------------------------------------------------------
const isCallish = (n) => ts.isCallExpression(n) || ts.isNewExpression(n) || ts.isJsxOpeningElement(n) || ts.isJsxSelfClosingElement(n);
const spanIndex = new Map(); // `${rel}\0${s}\0${e}` -> [node]
const callish = []; // {node, rel, prod}
const anonFns = []; // {node, rel, prod, line, col}
const synthByKey = new Map(); // `${rel}\0<cb@L:C>` -> fn
for (const file of files) {
  const sf = program.getSourceFile(file);
  if (!sf) continue;
  const map = byteIndex(sf, fs.readFileSync(file));
  const r = path.relative(root, file);
  const prod = !isTestPath(r);
  // UTF-8 byte column of a position (prism's column is the tree-sitter byte column)
  const lineStarts = sf.getLineStarts();
  (function visit(n) {
    if (isCallish(n)) {
      callish.push({ node: n, rel: r, prod });
      const spans = [[map[n.getStart()], map[n.end]]];
      if (ts.isJsxOpeningElement(n) && ts.isJsxElement(n.parent)) spans.push([map[n.parent.getStart()], map[n.parent.end]]);
      for (const [s, e] of spans) {
        const k = `${r}\0${s}\0${e}`;
        if (!spanIndex.has(k)) spanIndex.set(k, []);
        spanIndex.get(k).push(n);
      }
    }
    if (isAnonCallable(n)) {
      const pos = n.getStart();
      const lc = sf.getLineAndCharacterOfPosition(pos);
      const col = map[pos] - map[lineStarts[lc.line]] + 1;
      const name = `<cb@${lc.line + 1}:${col}>`;
      anonFns.push({ node: n, rel: r, prod, name });
      const key = `${r}\0${name}`;
      synthByKey.set(key, n);
    }
    ts.forEachChild(n, visit);
  })(sf);
}

// ---- prism records --------------------------------------------------------------------------------
function load(file) {
  if (!file || file === '-') return null;
  return fs.readFileSync(file, 'utf8').split('\n').filter(Boolean).map(JSON.parse)
    .filter((r) => r.record_kind === 'call_site' && r.origin === 'Source' && /\.(js|jsx|mjs|cjs|ts|tsx|mts|cts)$/.test(r.caller.file));
}
function bySpan(records) {
  const m = new Map();
  let unmatched = 0;
  for (const r of records) {
    const ns = spanIndex.get(`${r.source_span.file || r.caller.file}\0${r.source_span.start_byte}\0${r.source_span.end_byte}`) || [];
    if (!ns.length) { unmatched++; continue; }
    for (const n of ns) { if (!m.has(n)) m.set(n, []); m.get(n).push(r); }
  }
  return { m, unmatched, total: records.length };
}
function grade(recs) {
  // one grade per call node: exact if any record has an exact target, else nameonly, else drop:<reason>
  let exact = false, nameonly = false; const drops = new Set();
  for (const r of recs) {
    for (const t of r.resolved_targets || []) { if (t.confidence === 'exact') exact = true; else nameonly = true; }
    if (r.drop) drops.add(r.drop);
  }
  if (exact) return 'exact';
  if (nameonly) return 'nameonly';
  return 'drop:' + ([...drops].sort().join('+') || 'none');
}
const targetKinds = (recs) => { const s = new Set(); for (const r of recs) for (const t of r.resolved_targets || []) s.add(t.confidence + ':' + t.kind); return [...s].sort(); };

// Does the pinned checker's declaration of the callee match every exact target (file + start line)?
function checkerAgrees(n, recs) {
  let e = ts.isJsxOpeningElement(n) || ts.isJsxSelfClosingElement(n) ? n.tagName : peel(n.expression);
  if (e && ts.isPropertyAccessExpression(e)) e = e.name;
  const s0 = e && sym(e);
  const s = s0 && (s0.flags & ts.SymbolFlags.Alias ? unalias(s0) : s0);
  const d = s && (s.valueDeclaration || (s.declarations || [])[0]);
  if (!d) return 'checker_unresolved';
  const sf = d.getSourceFile();
  if (!inRepo(sf)) return 'checker_external';
  let fn = d;
  if (ts.isVariableDeclaration(d) && d.initializer) fn = peel(d.initializer);
  const want = [path.relative(root, sf.fileName), new Set([sf.getLineAndCharacterOfPosition(d.getStart()).line + 1, sf.getLineAndCharacterOfPosition(fn.getStart()).line + 1])];
  const exact = recs.flatMap((r) => (r.resolved_targets || []).filter((t) => t.confidence === 'exact'));
  return exact.every((t) => t.function_id.file === want[0] && want[1].has(t.function_id.start_line)) ? 'agree' : 'disagree';
}
const samples = [];
const base = bySpan(load(baseFile));
const protoRecords = load(protoFile);
const proto = protoRecords ? bySpan(protoRecords) : null;

const A = { base_exact_vs_checker: {}, corpus: aggregateOnly ? '(private)' : path.basename(root), files: files.length, join: { base_records: base.total, base_unmatched: base.unmatched } };
if (proto) Object.assign(A.join, { proto_records: proto.total, proto_unmatched: proto.unmatched });

// reproduction of the js-arg-follow census line (same classification)
const repro = { product_call_nodes: 0, product_without_record: 0, product_in_anonymous: 0, test_in_anonymous: 0 };
// this lane's view: innermost callable is anonymous under prism's own naming rule
const S = {
  product_unrecorded_in_anon: 0, test_unrecorded_in_anon: 0,
  product_unrecorded_in_anon_by_kind: {}, product_unrecorded_in_anon_by_depth: {},
  product_unrecorded_in_anon_by_outer: {}, // what is above the anon chain: named fn / module / class context
  product_unrecorded_not_in_anon_by_enclosing: {},
};
const P = { // prototype outcome on base-unrecorded product calls inside anonymous callables
  now_recorded: 0, still_unrecorded: 0, by_grade: {}, by_grade_and_kind: {}, by_target_kind: {},
  caller_is_synthetic: 0, caller_is_named: 0, exact_vs_checker: {},
};
const REG = { // regression check: base-recorded call nodes whose records changed in the prototype
  base_recorded_nodes: 0, unchanged: 0, record_count_grew: 0, caller_changed: 0, grade_changed: {}, lost_record: 0,
};
for (const { node: n, rel, prod } of callish) {
  const f = enclosingCallable(n);
  const anonInner = f && !ts.isSourceFile(f) && isAnonCallable(f);
  // repro
  const baseRecs = base.m.get(n);
  if (prod) repro.product_call_nodes++;
  if (!baseRecs) {
    let anonCensus = false;
    if (f && !ts.isSourceFile(f) && (ts.isArrowFunction(f) || ts.isFunctionExpression(f))) anonCensus = !NAMED_FE_KINDS(f);
    if (prod) { repro.product_without_record++; if (anonCensus) repro.product_in_anonymous++; } else if (anonCensus) repro.test_in_anonymous++;
  }
  if (!baseRecs) {
    if (anonInner) {
      if (!prod) { S.test_unrecorded_in_anon++; continue; }
      S.product_unrecorded_in_anon++;
      const k = kindOf(f);
      inc(S.product_unrecorded_in_anon_by_kind, k);
      const d = anonDepth(f);
      inc(S.product_unrecorded_in_anon_by_depth, d >= 4 ? '4+' : String(d));
      // what holds the outermost anonymous callable of the chain
      let top = f; while (true) { const e = enclosingCallable(top); if (e && !ts.isSourceFile(e) && isAnonCallable(e)) top = e; else { inc(S.product_unrecorded_in_anon_by_outer, !e || ts.isSourceFile(e) ? (inClass(top) ? 'module_level_in_class' : 'module_level') : (inClass(top) ? 'named_in_class' : 'named_function')); break; } }
      if (proto) {
        const pr = proto.m.get(n);
        if (!pr) { P.still_unrecorded++; samples.push({ what: 'still_unrecorded', kind: k, file: rel, line: n.getSourceFile().getLineAndCharacterOfPosition(n.getStart()).line + 1, text: n.getText().slice(0, 80) }); continue; }
        P.now_recorded++;
        const g = grade(pr);
        inc(P.by_grade, g);
        inc(P.by_grade_and_kind, k + ' | ' + g.replace(/^drop:.*/, 'drop'));
        for (const tk of targetKinds(pr)) inc(P.by_target_kind, tk);
        if (pr.every((r) => r.caller.name.startsWith('<cb@'))) P.caller_is_synthetic++; else P.caller_is_named++;
        if (g === 'exact') { const v = checkerAgrees(n, pr); inc(P.exact_vs_checker, v); if (v === 'disagree') samples.push({ what: 'new_exact_disagree', file: rel, line: n.getSourceFile().getLineAndCharacterOfPosition(n.getStart()).line + 1, text: n.getText().slice(0, 80), targets: pr.flatMap((r) => r.resolved_targets) }); }
      }
    } else if (prod) {
      inc(S.product_unrecorded_not_in_anon_by_enclosing, !f || ts.isSourceFile(f) ? 'module_level' : 'named_' + ts.SyntaxKind[f.kind]);
    }
  } else if (proto) {
    if (prod && grade(baseRecs) === 'exact') inc(A.base_exact_vs_checker, checkerAgrees(n, baseRecs));
    REG.base_recorded_nodes++;
    const pr = proto.m.get(n) || [];
    if (!pr.length) { REG.lost_record++; continue; }
    const key = (rs) => rs.map((r) => JSON.stringify([r.caller, r.resolved_targets, r.drop])).sort().join('\n');
    if (key(pr) === key(baseRecs)) { REG.unchanged++; continue; }
    if (pr.length > baseRecs.length) REG.record_count_grew++;
    const callers = (rs) => rs.map((r) => JSON.stringify(r.caller)).sort().join('|');
    if (callers(pr) !== callers(baseRecs)) REG.caller_changed++;
    const g0 = grade(baseRecs), g1 = grade(pr);
    if (g0 !== g1) inc(REG.grade_changed, g0 + ' -> ' + g1);
  }
}
A.reproduction_js_arg_follow = repro;
A.unrecorded = S;
if (proto) { A.prototype_outcome = P; A.prototype_regression_check = REG; }

// ---- nodes and edges --------------------------------------------------------------------------------
const N = { anon_callables: 0, product_anon_callables: 0, product_by_kind: {}, product_with_own_calls: 0, product_without_own_calls: 0,
  product_enclosing_edges: 0, product_module_level: 0, product_in_class_context: 0, product_same_line_siblings: 0, synthetic_name_collisions: 0 };
const ownCalls = new Map();
for (const { node: n } of callish) { const f = enclosingCallable(n); if (f && isAnonCallable(f)) ownCalls.set(f, (ownCalls.get(f) || 0) + 1); }
const perLine = new Map();
const seen = new Set();
for (const a of anonFns) {
  N.anon_callables++;
  const k = `${a.rel}\0${a.name}`;
  if (seen.has(k)) N.synthetic_name_collisions++; seen.add(k);
  if (!a.prod) continue;
  N.product_anon_callables++;
  inc(N.product_by_kind, kindOf(a.node).replace(/^react_hook:.*/, 'react_hook'));
  if (ownCalls.get(a.node)) N.product_with_own_calls++; else N.product_without_own_calls++;
  const e = enclosingCallable(a.node);
  if (!e || ts.isSourceFile(e)) N.product_module_level++; else N.product_enclosing_edges++;
  if (inClass(a.node)) N.product_in_class_context++;
  const lk = `${a.rel}\0${a.name.split(':')[0]}`;
  perLine.set(lk, (perLine.get(lk) || 0) + 1);
}
for (const v of perLine.values()) if (v > 1) N.product_same_line_siblings += v;
A.nodes = N;

// prototype: synthetic callers that the TS mirror does not know (naming-rule drift check)
if (proto) {
  const drift = { synthetic_callers: 0, unknown_to_mirror: 0 };
  const callers = new Set();
  for (const r of protoRecords) if (r.caller.name.startsWith('<cb@')) callers.add(`${r.caller.file}\0${r.caller.name}`);
  drift.synthetic_callers = callers.size;
  for (const c of callers) if (!synthByKey.has(c)) drift.unknown_to_mirror++;
  // duplicate records: one call node recorded under more than one caller
  let multi = 0, multiProd = 0;
  for (const [n, rs] of proto.m) { const cs = new Set(rs.map((r) => JSON.stringify(r.caller))); if (cs.size > 1) { multi++; if (!isTestPath(path.relative(root, n.getSourceFile().fileName))) multiProd++; } }
  drift.nodes_with_more_than_one_caller = multi; drift.product_nodes_with_more_than_one_caller = multiProd;
  let bmulti = 0;
  for (const [, rs] of base.m) { const cs = new Set(rs.map((r) => JSON.stringify(r.caller))); if (cs.size > 1) bmulti++; }
  drift.base_nodes_with_more_than_one_caller = bmulti;
  A.prototype_consistency = drift;
}

fs.mkdirSync(out, { recursive: true, mode: 0o700 });
fs.writeFileSync(path.join(out, 'aggregate.json'), JSON.stringify(A, null, 2) + '\n', { mode: 0o600 });
if (!aggregateOnly) fs.writeFileSync(path.join(out, 'samples.jsonl'), samples.map((x) => JSON.stringify(x)).join('\n') + '\n', { mode: 0o600 });
console.log(JSON.stringify(A, null, 2));
