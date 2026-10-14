#!/usr/bin/env node
'use strict';
// Argument-following census (lane js-arg-follow). Static measurement only: parses the corpus
// with the pinned TypeScript 5.9.3 checker, never executes corpus code.
//   node census.cjs <repo-root> <prism-sites.jsonl> <out-dir> [--aggregate-only]
// Joins every prism `call_site` record (from `prism nav call-stats --dump-sites`) to its
// syntax node by byte span, finds sites whose callee is a parameter of an enclosing function
// (the "holder"), and asks, for each, what the holder's in-repo callers pass at that position.
// --aggregate-only writes aggregate.json only (no file names, no per-site rows): the mode the
// controller uses on private corpora.
const fs = require('fs');
const path = require('path');
const TS_PATH = '/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js';
const ts = require(TS_PATH);
if (ts.version !== '5.9.3') throw Error('pinned checker mismatch');

const [rootArg, sitesFile, outArg, ...flags] = process.argv.slice(2);
const aggregateOnly = flags.includes('--aggregate-only');
// --private-root: the controller's own private corpus; directory names inside it are not filtered
const privateRoot = flags.includes('--private-root');
const root = path.resolve(rootArg);
const out = path.resolve(outArg);
process.umask(0o077);

// ---- file walk ---------------------------------------------------------------------------------
const forbiddenName = (x) => x === 'frontend-portal' || x.startsWith('f-') || x.startsWith('F-private') || x.startsWith('secbench') || x.startsWith('SB__');
const forbidden = (p) => {
  const rel = path.relative(root, path.resolve(p));
  if (rel.startsWith('..')) return path.resolve(p).split(path.sep).some(forbiddenName);
  return !privateRoot && rel.split(path.sep).some(forbiddenName);
};
const skipDir = new Set(['target', 'node_modules', 'vendor', 'dist', 'build']);
const files = [];
const excluded = {};
const isTestPath = (f) => /(^|\/)(tests?|__tests__|__mocks__|testRunner|harness|fixtures?)(\/|$)|\.(test|spec)\.[cm]?[jt]sx?$/.test(f);
const inc = (o, k, n = 1) => { o[k] = (Object.prototype.hasOwnProperty.call(o, k) ? o[k] : 0) + n; };
(function walk(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isSymbolicLink()) { inc(excluded, 'symlink'); continue; }
    if (!privateRoot && forbiddenName(e.name)) { inc(excluded, 'forbidden'); continue; }
    if (e.isDirectory()) {
      if (e.name.startsWith('.') || skipDir.has(e.name)) { inc(excluded, 'directory'); continue; }
      walk(p);
    } else if (/\.(js|jsx|mjs|cjs|ts|tsx|mts|cts)$/.test(e.name) && !/\.d\.[mc]?ts$/.test(e.name)) {
      if (fs.statSync(p).size <= 2 * 1024 * 1024) files.push(p); else inc(excluded, 'over_2_MiB');
    }
  }
})(root);
files.sort();

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
const rel = (n) => path.relative(root, n.getSourceFile().fileName);
const lineOf = (n, pos) => n.getSourceFile().getLineAndCharacterOfPosition(pos === undefined ? n.getStart() : pos).line + 1;
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

// ---- byte map and span index (UTF-8 bytes, as prism reports) ------------------------------------
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
const spanIndex = new Map();
const isCallish = (n) => ts.isCallExpression(n) || ts.isNewExpression(n) || ts.isJsxOpeningElement(n) || ts.isJsxSelfClosingElement(n);
const allCallish = [];
for (const file of files) {
  const sf = program.getSourceFile(file);
  if (!sf) continue;
  const map = byteIndex(sf, fs.readFileSync(file));
  const r = path.relative(root, file);
  (function visit(n) {
    if (isCallish(n)) {
      allCallish.push(n);
      const spans = [[map[n.getStart()], map[n.end]]];
      if (ts.isJsxOpeningElement(n) && ts.isJsxElement(n.parent)) spans.push([map[n.parent.getStart()], map[n.parent.end]]);
      for (const [s, e] of spans) {
        const k = `${r}\0${s}\0${e}`;
        if (!spanIndex.has(k)) spanIndex.set(k, []);
        spanIndex.get(k).push(n);
      }
    }
    ts.forEachChild(n, visit);
  })(sf);
}

// ---- prism records -------------------------------------------------------------------------------
const records = fs.readFileSync(sitesFile, 'utf8').split('\n').filter(Boolean).map(JSON.parse)
  .filter((r) => r.record_kind === 'call_site' && r.origin === 'Source' && /\.(js|jsx|mjs|cjs|ts|tsx|mts|cts)$/.test(r.caller.file));
const recordsBySpan = new Map();
const nodeOf = (r) => {
  const ns = spanIndex.get(`${r.source_span.file || r.caller.file}\0${r.source_span.start_byte}\0${r.source_span.end_byte}`) || [];
  return ns;
};
const join = { source_records: 0, matched: 0, unmatched: 0 };
const joined = [];
for (const r of records) {
  join.source_records++;
  const ns = nodeOf(r);
  if (!ns.length) { join.unmatched++; continue; }
  join.matched++;
  for (const n of ns) {
    if (!recordsBySpan.has(n)) recordsBySpan.set(n, []);
    recordsBySpan.get(n).push(r);
  }
  joined.push([r, ns]);
}
join.unmatched_rate = join.unmatched / (join.source_records || 1);

// ---- callee classification -----------------------------------------------------------------------
const calleeExpr = (n) => peel(ts.isJsxOpeningElement(n) || ts.isJsxSelfClosingElement(n) ? n.tagName : n.expression);
// The parameter (or in-body destructure of one) that `id` names, with the property path into it.
function paramBinding(id) {
  const s = sym(id);
  const d = s && (s.valueDeclaration || (s.declarations || [])[0]);
  if (!d) return null;
  if (ts.isParameter(d) && ts.isIdentifier(d.name)) {
    if (d.name.text === 'this') return null;
    return { param: d, path: [], kind: d.dotDotDotToken ? 'rest' : d.initializer ? 'plain_default' : 'plain', symbol: s, decl: d };
  }
  if (ts.isBindingElement(d)) {
    const pathKeys = [];
    let o = d;
    let hasDefault = !!d.initializer;
    while (ts.isBindingElement(o)) {
      const pat = o.parent;
      if (ts.isObjectBindingPattern(pat)) {
        if (o.dotDotDotToken) return { param: null, kind: 'destructured_rest_object', symbol: s, decl: d };
        const key = o.propertyName ? (ts.isIdentifier(o.propertyName) || ts.isStringLiteral(o.propertyName) ? o.propertyName.text : null) : (ts.isIdentifier(o.name) ? o.name.text : null);
        if (key === null) return { param: null, kind: 'destructured_computed', symbol: s, decl: d };
        pathKeys.unshift({ key });
      } else {
        if (o.dotDotDotToken) return { param: null, kind: 'destructured_rest_array', symbol: s, decl: d };
        pathKeys.unshift({ index: pat.elements.indexOf(o) });
      }
      if (o.initializer) hasDefault = true;
      o = pat.parent;
    }
    if (ts.isParameter(o)) {
      return { param: o, path: pathKeys, kind: (o.dotDotDotToken ? 'rest_destructured' : 'destructured') + (hasDefault ? '_default' : ''), symbol: s, decl: d };
    }
    // `const { onClick } = props` where props is a parameter
    if (ts.isVariableDeclaration(o) && o.initializer && ts.isIdentifier(peel(o.initializer))) {
      const inner = paramBinding(peel(o.initializer));
      if (inner && inner.param && inner.path.length === 0 && inner.kind.startsWith('plain')) {
        return { param: inner.param, path: pathKeys, kind: 'body_destructure_of_param', symbol: s, decl: d };
      }
    }
    return null;
  }
  return null;
}
const writes = new Map();
// writes is a Map keyed by symbol; inc on a Map needs its own helper
function incMap(m, k) { m.set(k, (m.get(k) || 0) + 1); }

// ---- holders --------------------------------------------------------------------------------------
const WRAPPERS = new Set(['memo', 'forwardRef', 'observer', 'React.memo', 'React.forwardRef', 'useCallback', 'React.useCallback']);
function holderInfo(fn) {
  // returns {kind, symbol, exported, nameNode}
  const exportedMods = (node) => (ts.getCombinedModifierFlags(node) & ts.ModifierFlags.Export) !== 0;
  if (ts.isFunctionDeclaration(fn)) {
    return { kind: 'function_declaration', symbol: fn.name ? sym(fn.name) : undefined, exported: exportedMods(fn) || (ts.getCombinedModifierFlags(fn) & ts.ModifierFlags.Default) !== 0, decl: fn };
  }
  if (ts.isMethodDeclaration(fn) || ts.isGetAccessor(fn) || ts.isSetAccessor(fn)) {
    const cls = fn.parent;
    const isClass = ts.isClassDeclaration(cls) || ts.isClassExpression(cls);
    const exported = isClass && ts.isClassDeclaration(cls) && exportedMods(cls);
    return { kind: isClass ? 'class_method' : 'object_literal_method', symbol: sym(fn.name), exported, decl: fn };
  }
  if (ts.isConstructorDeclaration(fn)) {
    const cls = fn.parent;
    return { kind: 'constructor', symbol: cls.name ? sym(cls.name) : undefined, exported: ts.isClassDeclaration(cls) && exportedMods(cls), decl: fn };
  }
  if (ts.isArrowFunction(fn) || ts.isFunctionExpression(fn)) {
    let p = fn.parent;
    while (p && ts.isParenthesizedExpression(p)) p = p.parent;
    if (ts.isVariableDeclaration(p) && ts.isIdentifier(p.name)) {
      const stmt = p.parent && p.parent.parent;
      const isConst = (p.parent.flags & ts.NodeFlags.Const) !== 0;
      return { kind: isConst ? 'const_function' : 'let_var_function', symbol: sym(p.name), exported: !!(stmt && ts.isVariableStatement(stmt) && exportedMods(stmt)), decl: fn, varDecl: p };
    }
    if (ts.isCallExpression(p) && p.arguments[0] === fn) {
      const callee = p.expression.getText();
      let q = p.parent;
      while (q && ts.isParenthesizedExpression(q)) q = q.parent;
      if (WRAPPERS.has(callee) && ts.isVariableDeclaration(q) && ts.isIdentifier(q.name)) {
        const stmt = q.parent && q.parent.parent;
        return { kind: 'wrapped_const', wrapper: callee, symbol: sym(q.name), exported: !!(stmt && ts.isVariableStatement(stmt) && exportedMods(stmt)), decl: fn, varDecl: q };
      }
    }
    if (ts.isCallExpression(p) || ts.isNewExpression(p)) {
      // inline callback handed to another call
      const target = sym(peel(p.expression)) && unalias(sym(peel(p.expression)));
      const d = target && (target.valueDeclaration || (target.declarations || [])[0]);
      return { kind: d && inRepo(d.getSourceFile()) ? 'inline_callback_to_repo_fn' : 'inline_callback_to_other', decl: fn };
    }
    if (ts.isPropertyAssignment(p)) return { kind: 'object_property_function', symbol: sym(p.name), exported: false, decl: fn };
    if (ts.isJsxExpression(p)) return { kind: 'inline_jsx_prop', decl: fn };
    if (ts.isPropertyDeclaration(p)) return { kind: 'class_property_function', symbol: sym(p.name), exported: false, decl: fn };
    if (ts.isBinaryExpression(p)) return { kind: 'assigned_function', decl: fn };
    if (ts.isExportAssignment(p)) return { kind: 'default_export_anon', decl: fn, exported: true };
    return { kind: 'other_anonymous', decl: fn };
  }
  return { kind: 'other_' + ts.SyntaxKind[fn.kind], decl: fn };
}

// ---- references to holders (one pass over identifiers whose text names a holder) ----------------
const holderBySymbol = new Map(); // symbol -> {refs: {calls: [], escapes: n, exportRefs: n, callApply: n}}
function isTypePosition(n) {
  for (let p = n.parent; p; p = p.parent) {
    if (ts.isTypeNode(p) && !ts.isExpressionWithTypeArguments(p)) return true;
    if (ts.isTypeQueryNode && ts.isTypeQueryNode(p)) return true;
    if (ts.isStatement(p) || ts.isExpression(p) && !ts.isIdentifier(p)) return false;
  }
  return false;
}
function classifyRef(id, holderSym, info) {
  // returns ['call', callNode, argOffset] | ['export'] | ['decl'] | ['type'] | ['member_use'] | ['escape', why]
  let e = id;
  const p0 = id.parent;
  if (info.decl && (info.decl.name === id || (info.varDecl && info.varDecl.name === id))) return ['decl'];
  if (ts.isClassDeclaration(p0) && p0.name === id) return ['decl'];
  if (ts.isImportSpecifier(p0) || ts.isImportClause(p0) || ts.isNamespaceImport(p0) || ts.isImportEqualsDeclaration(p0)) return ['decl'];
  if (ts.isExportSpecifier(p0) || ts.isExportAssignment(p0)) return ['export'];
  if (isTypePosition(id)) return ['type'];
  if (ts.isJsxClosingElement(p0)) return ['decl'];
  // method name in `x.run`: e is the property access
  if (ts.isPropertyAccessExpression(p0) && p0.name === id) e = p0;
  // `this.run` etc. are fine; `run.call(...)` / `.apply` / `.bind`
  let q = e.parent;
  while (q && ts.isParenthesizedExpression(q)) { e = q; q = q.parent; }
  if ((ts.isCallExpression(q) || ts.isNewExpression(q)) && q.expression === e) return ['call', q, 0];
  if ((ts.isJsxOpeningElement(q) || ts.isJsxSelfClosingElement(q)) && q.tagName === e) return ['call', q, 0];
  if (ts.isPropertyAccessExpression(q) && q.expression === e) {
    const m = q.name.text;
    const qq = q.parent;
    if (m === 'call' && ts.isCallExpression(qq) && qq.expression === q) return ['call', qq, 1];
    if (m === 'apply' && ts.isCallExpression(qq) && qq.expression === q) return ['escape', 'apply'];
    if (m === 'bind') return ['escape', 'bind'];
    return ['member_use'];
  }
  if (ts.isTypeOfExpression(q)) return ['member_use'];
  if (ts.isBinaryExpression(q) && q.left === e && q.operatorToken.kind === ts.SyntaxKind.EqualsToken) return ['member_use'];
  if (ts.isExportAssignment(q)) return ['export'];
  if (ts.isShorthandPropertyAssignment(q) || ts.isPropertyAssignment(q)) {
    // module.exports = { run } is export
    return ['escape', 'object_property'];
  }
  if (ts.isCallExpression(q) || ts.isNewExpression(q)) return ['escape', 'argument'];
  if (ts.isJsxExpression(q)) return ['escape', 'jsx_prop_value'];
  if (ts.isReturnStatement(q) || ts.isArrowFunction(q)) return ['escape', 'returned'];
  if (ts.isVariableDeclaration(q)) return ['escape', 'stored'];
  if (ts.isArrayLiteralExpression(q)) return ['escape', 'array'];
  if (ts.isBinaryExpression(q)) return ['escape', 'assigned_or_operand'];
  return ['escape', 'other_' + ts.SyntaxKind[q.kind]];
}

// ---- argument values ---------------------------------------------------------------------------
// value(expr) -> {open: [reasons], targets: Map(id -> {followable, via})}
const DEPTH = 4;
function emptyV() { return { open: [], targets: new Map(), nothing: false }; }
function merge(a, b) { a.open.push(...b.open); for (const [k, v] of b.targets) { const o = a.targets.get(k); a.targets.set(k, o ? { followable: o.followable && v.followable, via: o.via } : v); } a.nothing = a.nothing || b.nothing; return a; }
function targetId(fn) {
  const file = rel(fn);
  let name = fn.name && fn.name.getText ? fn.name.getText() : null;
  if (!name && fn.parent && ts.isVariableDeclaration(fn.parent)) name = fn.parent.name.getText();
  return `${file}:${name || '<anon>'}@${lineOf(fn)}`;
}
function target(fn, followable, via) { const v = emptyV(); v.targets.set(targetId(fn), { followable, via }); return v; }
function open(reason) { const v = emptyV(); v.open.push(reason); return v; }
function nothing() { const v = emptyV(); v.nothing = true; return v; }

function valueOfSymbol(s, depth, followable, hops) {
  if (!s) return open('unresolved_symbol');
  let viaImport = false;
  if (s.flags & ts.SymbolFlags.Alias) { s = unalias(s); viaImport = true; if (!s) return open('unresolved_import'); }
  const d = s.valueDeclaration || (s.declarations || [])[0];
  if (!d) return open('no_declaration');
  const sf = d.getSourceFile();
  if (!inRepo(sf)) return open('external');
  if (ts.isFunctionDeclaration(d) || ts.isClassDeclaration(d)) return target(d, followable, viaImport ? 'import' : 'local');
  if (ts.isMethodDeclaration(d)) return target(d, false, 'method');
  if (ts.isVariableDeclaration(d)) {
    const isConst = (d.parent.flags & ts.NodeFlags.Const) !== 0;
    if (!ts.isIdentifier(d.name)) return open('destructured_variable');
    if (!isConst) return writes.has(s) ? open('let_var_reassigned') : open('let_var');
    if (!d.initializer) return open('no_initializer');
    const r = valueOf(d.initializer, depth + 1, followable, hops, null);
    for (const [, x] of r.targets) if (x.via === 'inline') x.via = viaImport ? 'import_const' : 'const';
    return r;
  }
  if (ts.isParameter(d) || ts.isBindingElement(d)) {
    const pb = paramBinding(d.name && ts.isIdentifier(d.name) ? d.name : null);
    if (!pb || !pb.param) return open(ts.isBindingElement(d) ? 'destructured_local' : 'param_unhandled');
    if (hops >= DEPTH) return open('forward_depth');
    const r = paramValue(pb, hops + 1);
    // forwarding keeps followability (the rule may follow forwards transitively)
    for (const [, v] of r.targets) v.followable = v.followable && followable;
    return r;
  }
  if (ts.isPropertyAssignment(d) || ts.isShorthandPropertyAssignment(d)) return open('object_member');
  if (ts.isEnumMember(d)) return nothing();
  return open('decl_' + ts.SyntaxKind[d.kind]);
}
function valueOf(expr, depth, followable, hops, paramDefaultOwner) {
  expr = peel(expr);
  if (!expr) return nothing();
  if (depth > 10) return open('depth');
  if (ts.isArrowFunction(expr) || ts.isFunctionExpression(expr) || ts.isClassExpression(expr)) return target(expr, followable, 'inline');
  if (ts.isIdentifier(expr)) {
    if (expr.text === 'undefined') return nothing();
    return valueOfSymbol(sym(expr), depth, followable, hops);
  }
  if (expr.kind === ts.SyntaxKind.NullKeyword || ts.isLiteralExpression(expr) || ts.isObjectLiteralExpression(expr) || ts.isArrayLiteralExpression(expr) || ts.isTemplateExpression(expr) || expr.kind === ts.SyntaxKind.TrueKeyword || expr.kind === ts.SyntaxKind.FalseKeyword) return nothing();
  if (ts.isPropertyAccessExpression(expr)) {
    const base = peel(expr.expression);
    const bs = ts.isIdentifier(base) ? sym(base) : null;
    if (bs && bs.flags & ts.SymbolFlags.Alias && (bs.declarations || []).some((d) => ts.isNamespaceImport(d))) {
      return valueOfSymbol(sym(expr.name), depth, followable, hops);
    }
    const ms = sym(expr.name);
    const md = ms && (ms.valueDeclaration || (ms.declarations || [])[0]);
    if (md && inRepo(md.getSourceFile()) && (ts.isMethodDeclaration(md) || ts.isFunctionDeclaration(md))) return target(md, false, base.kind === ts.SyntaxKind.ThisKeyword ? 'this_method' : 'member_method');
    if (md && inRepo(md.getSourceFile()) && (ts.isPropertyDeclaration(md) || ts.isPropertyAssignment(md)) && md.initializer && (ts.isArrowFunction(peel(md.initializer)) || ts.isFunctionExpression(peel(md.initializer)))) return target(peel(md.initializer), false, base.kind === ts.SyntaxKind.ThisKeyword ? 'this_property_fn' : 'member_property_fn');
    if (md && !inRepo(md.getSourceFile())) return open('external_member');
    return open('member');
  }
  if (ts.isConditionalExpression(expr)) return merge(valueOf(expr.whenTrue, depth + 1, followable, hops), valueOf(expr.whenFalse, depth + 1, followable, hops));
  if (ts.isBinaryExpression(expr) && [ts.SyntaxKind.BarBarToken, ts.SyntaxKind.QuestionQuestionToken, ts.SyntaxKind.AmpersandAmpersandToken].includes(expr.operatorToken.kind)) {
    const l = expr.operatorToken.kind === ts.SyntaxKind.AmpersandAmpersandToken ? nothing() : valueOf(expr.left, depth + 1, followable, hops);
    return merge(l, valueOf(expr.right, depth + 1, followable, hops));
  }
  if (ts.isCallExpression(expr)) {
    const c = peel(expr.expression).getText();
    if ((c === 'useCallback' || c === 'React.useCallback') && expr.arguments[0]) return valueOf(expr.arguments[0], depth + 1, followable, hops);
    if (ts.isPropertyAccessExpression(expr.expression) && expr.expression.name.text === 'bind') return open('bind_result');
    return open('call_result');
  }
  if (ts.isVoidExpression(expr)) return nothing();
  return open('expr_' + ts.SyntaxKind[expr.kind]);
}

// What arrives at `pb.param` with `pb.path` across the holder's callers.
const holderRefCache = new Map();
function holderRefs(info) {
  if (!info.symbol) return null;
  return holderRefCache.get(info.symbol) || null;
}
function pick(argExpr, pathKeys, depth, hops, defaultInit) {
  // descend into an object/array literal argument along pathKeys
  let e = peel(argExpr);
  for (let i = 0; i < pathKeys.length; i++) {
    const k = pathKeys[i];
    if (!e) return defaultInit ? valueOf(defaultInit, depth, true, hops) : nothing();
    if (ts.isIdentifier(e)) {
      // `run(opts)` where opts is a const object literal: one level, not followable
      const s = sym(e);
      const d = s && (s.valueDeclaration || (s.declarations || [])[0]);
      if (d && ts.isVariableDeclaration(d) && (d.parent.flags & ts.NodeFlags.Const) && d.initializer && (ts.isObjectLiteralExpression(peel(d.initializer)) || ts.isArrayLiteralExpression(peel(d.initializer)))) {
        const r = pick(d.initializer, pathKeys.slice(i), depth, hops, defaultInit);
        for (const [, v] of r.targets) v.followable = false;
        return r;
      }
      // the argument is itself a parameter (props forwarded whole): follow it
      const pb = paramBinding(e);
      if (pb && pb.param && hops < DEPTH) return paramValue({ ...pb, path: pb.path.concat(pathKeys.slice(i)) }, hops + 1);
      return open('object_arg_not_literal');
    }
    if ('key' in k && ts.isObjectLiteralExpression(e)) {
      let found = null; let spread = false;
      for (const p of e.properties) {
        if (ts.isSpreadAssignment(p)) { spread = true; continue; }
        const nm = p.name && (ts.isIdentifier(p.name) || ts.isStringLiteral(p.name)) ? p.name.text : null;
        if (nm === k.key) found = p; // last wins
      }
      if (!found) { if (spread) return open('spread_object'); return defaultInit ? valueOf(defaultInit, depth, true, hops) : nothing(); }
      if (ts.isShorthandPropertyAssignment(found)) {
        if (i < pathKeys.length - 1) return open('nested_shorthand');
        return valueOfSymbol(checker.getShorthandAssignmentValueSymbol(found), depth, true, hops);
      }
      if (ts.isMethodDeclaration(found)) return i === pathKeys.length - 1 ? target(found, true, 'object_method') : open('nested_method');
      e = peel(found.initializer);
      continue;
    }
    if ('index' in k && ts.isArrayLiteralExpression(e)) {
      const el = e.elements[k.index];
      if (e.elements.slice(0, k.index + 1).some(ts.isSpreadElement)) return open('spread_array');
      e = el ? peel(el) : null;
      continue;
    }
    return open('pattern_mismatch');
  }
  if (!e) return defaultInit ? valueOf(defaultInit, depth, true, hops) : nothing();
  return valueOf(e, depth, true, hops);
}
function jsxPick(el, pathKeys, hops, defaultInit) {
  const [first, ...restKeys] = pathKeys;
  if (!first || !('key' in first)) return open('jsx_props_whole');
  if (first.key === 'children') return open('jsx_children');
  let found = null; let spread = false;
  for (const a of el.attributes.properties) {
    if (ts.isJsxSpreadAttribute(a)) { spread = true; found = null; continue; }
    if (a.name.getText() === first.key) { found = a; spread = false; }
  }
  if (!found) return spread ? open('jsx_spread') : defaultInit ? valueOf(defaultInit, 0, true, hops) : nothing();
  if (!found.initializer) return nothing();
  if (ts.isStringLiteral(found.initializer)) return nothing();
  const ex = found.initializer.expression;
  if (!ex) return nothing();
  return restKeys.length ? pick(ex, restKeys, 0, hops, defaultInit) : valueOf(ex, 0, true, hops);
}
const isUndefined = (e) => { e = peel(e); return !!e && ((ts.isIdentifier(e) && e.text === 'undefined') || ts.isVoidExpression(e)); };
const paramValueMemo = new Map();
function paramValue(pb, hops) {
  const key = `${pb.param.pos}:${pb.param.getSourceFile().fileName}:${JSON.stringify(pb.path)}`;
  if (paramValueMemo.has(key)) return cloneV(paramValueMemo.get(key));
  paramValueMemo.set(key, open('cycle'));
  const r = paramValueUncached(pb, hops);
  paramValueMemo.set(key, r);
  return cloneV(r);
}
function cloneV(v) { return { open: v.open.slice(), targets: new Map([...v.targets].map(([k, x]) => [k, { ...x }])), nothing: v.nothing, meta: v.meta }; }
function paramValueUncached(pb, hops) {
  const fn = pb.param.parent;
  const info = holderInfo(fn);
  const r = emptyV();
  r.meta = { holder: info.kind, exported: !!info.exported, escapes: [], calls: 0, callers_prism_covered: 0 };
  const params = fn.parameters.filter((p) => !(ts.isIdentifier(p.name) && p.name.text === 'this'));
  const pos = params.indexOf(pb.param);
  r.meta.prism_targets = [];
  if (pb.param.dotDotDotToken && pb.path.length === 0) { r.open.push('rest_param'); return r; }
  if (pb.symbol && writes.has(pb.symbol)) { r.open.push('param_reassigned'); r.meta.prism_targets = []; return r; }
  const refs = info.symbol ? holderRefCache.get(info.symbol) : null;
  if (!refs) { r.open.push('holder_' + info.kind); return r; }
  const lexicalHolder = ['function_declaration', 'const_function', 'wrapped_const', 'constructor'].includes(info.kind);
  if (!lexicalHolder && refs.calls.length === 0) r.open.push('member_holder_callers_unseen');
  if (info.exported || refs.exported) { r.open.push('exported'); r.meta.exported = true; }
  for (const why of refs.escapes) { r.open.push('escape_' + why); r.meta.escapes.push(why); }
  // default of the parameter / binding element
  let defaultInit = pb.param.initializer && pb.path.length === 0 ? pb.param.initializer : null;
  if (pb.decl && ts.isBindingElement(pb.decl) && pb.decl.initializer) defaultInit = pb.decl.initializer;
  const prismT = new Set();
  for (const [call, offset] of refs.calls) {
    r.meta.calls++;
    if (call.__prismCovered === undefined) call.__prismCovered = prismCovers(call, info);
    if (call.__prismCovered) r.meta.callers_prism_covered++;
    let v;
    if (ts.isJsxOpeningElement(call) || ts.isJsxSelfClosingElement(call)) {
      if (pos !== 0) v = open('jsx_non_props_param');
      else if (pb.path.length === 0) v = open('jsx_props_whole');
      else v = jsxPick(call, pb.path, hops, defaultInit);
    } else {
      const args = call.arguments || [];
      const idx = pos + offset;
      if (args.slice(0, idx + 1).some(ts.isSpreadElement)) v = open('spread_argument');
      else if (pb.path.length === 0) v = args[idx] && !isUndefined(args[idx]) ? valueOf(args[idx], 0, true, hops) : (defaultInit ? valueOf(defaultInit, 0, true, hops) : nothing());
      else v = args[idx] && !isUndefined(args[idx]) ? pick(args[idx], pb.path, 0, hops, defaultInit) : (defaultInit ? valueOf(defaultInit, 0, true, hops) : nothing());
    }
    merge(r, v);
    if (call.__prismCovered) for (const [k, x] of v.targets) if (x.followable) prismT.add(k);
  }
  r.meta.prism_targets = [...prismT];
  if (lexicalHolder && refs.calls.length === 0 && !info.exported && !refs.exported && refs.escapes.length === 0 && r.open.length === 0) r.meta.no_callers = true;
  return r;
}
// Does prism hold a record at this call whose resolved targets include the holder?
function prismCovers(call, info) {
  const recs = recordsBySpan.get(call) || [];
  const d = info.decl;
  const end = lineOf(d, d.end);
  const file = rel(d);
  return recs.some((r) => r.resolved_targets.some((t) => t.function_id.file === file && t.function_id.end_line === end));
}

// ---- main pass --------------------------------------------------------------------------------------
for (const file of files) { const sf = program.getSourceFile(file); if (sf) noteWritesMap(sf); }
function noteWritesMap(sf) {
  (function visit(n) {
    if (ts.isBinaryExpression(n) && ts.isAssignmentOperator(n.operatorToken.kind) && ts.isIdentifier(peel(n.left))) {
      const s = sym(peel(n.left)); if (s) incMap(writes, s);
    }
    if ((ts.isPrefixUnaryExpression(n) || ts.isPostfixUnaryExpression(n)) && ts.isIdentifier(peel(n.operand)) && [ts.SyntaxKind.PlusPlusToken, ts.SyntaxKind.MinusMinusToken].includes(n.operator)) {
      const s = sym(peel(n.operand)); if (s) incMap(writes, s);
    }
    ts.forEachChild(n, visit);
  })(sf);
}

// 1. classify every joined record's callee
const siteRows = [];
const agg = {
  join, excluded, files: files.length,
  records: { source: records.length },
  drops_all: {},
  bare_identifier_drops: 0,
  param_sites: { total: 0, by_drop: {}, by_param_kind: {}, by_holder_kind: {}, reassigned_param: 0, with_targets_today: 0, today_kinds_on_param_sites: {} },
  member_of_param_sites: { total: 0, by_drop: {} },
  element_of_param_or_arguments_sites: { total: 0, by_drop: {} },
};
const todo = [];
for (const [r, ns] of joined) {
  inc(agg.drops_all, r.drop || 'resolved');
  const n = ns[0];
  const c = calleeExpr(n);
  if (!c) continue;
  if (ts.isIdentifier(c)) {
    if (r.drop) agg.bare_identifier_drops++;
    const pb = paramBinding(c);
    if (!pb) continue;
    agg.param_sites.total++;
    inc(agg.param_sites.by_drop, r.drop || 'resolved');
    inc(agg.param_sites.by_param_kind, pb.kind);
    if (writes.has(pb.symbol)) agg.param_sites.reassigned_param++;
    if (!r.drop) { agg.param_sites.with_targets_today++; for (const t of r.resolved_targets) inc(agg.param_sites.today_kinds_on_param_sites, `${t.kind}/${t.confidence}`); }
    if (pb.param) inc(agg.param_sites.by_holder_kind, holderInfo(pb.param.parent).kind);
    todo.push({ r, n, pb, form: 'bare' });
  } else if (ts.isPropertyAccessExpression(c)) {
    const base = peel(c.expression);
    if (ts.isIdentifier(base)) {
      const pb = paramBinding(base);
      if (pb && pb.param && pb.path.length === 0 && pb.kind.startsWith('plain')) {
        const m = c.name.text;
        if (m === 'call' || m === 'apply') {
          // cb.call(...) — the parameter itself is the callee
          agg.param_sites.total++;
          inc(agg.param_sites.by_drop, r.drop || 'resolved');
          inc(agg.param_sites.by_param_kind, 'param_dot_' + m);
          inc(agg.param_sites.by_holder_kind, holderInfo(pb.param.parent).kind);
          if (!r.drop) agg.param_sites.with_targets_today++;
          todo.push({ r, n, pb, form: 'dot_' + m });
        } else {
          agg.member_of_param_sites.total++;
          inc(agg.member_of_param_sites.by_drop, r.drop || 'resolved');
          todo.push({ r, n, pb: { ...pb, path: [{ key: m }], kind: 'member_of_param' }, form: 'member' });
        }
      }
    }
  } else if (ts.isElementAccessExpression(c)) {
    const base = peel(c.expression);
    if (ts.isIdentifier(base) && (base.text === 'arguments' || (paramBinding(base) || {}).kind === 'rest')) {
      agg.element_of_param_or_arguments_sites.total++;
      inc(agg.element_of_param_or_arguments_sites.by_drop, r.drop || 'resolved');
    }
  }
}

// 2. collect references to every holder symbol that appears
const holderSyms = new Map();
for (const t of todo) {
  const info = holderInfo(t.pb.param.parent);
  if (info.symbol) holderSyms.set(info.symbol, info);
}
const holderNames = new Set([...holderSyms.keys()].map((s) => s.escapedName));
for (const [s] of holderSyms) holderRefCache.set(s, { calls: [], escapes: [], exported: false });
// a reachable parameter forwarded into a holder we have not seen is added lazily below
function ensureHolder(info) {
  if (!info.symbol || holderRefCache.has(info.symbol)) return;
  holderRefCache.set(info.symbol, { calls: [], escapes: [], exported: false });
  holderSyms.set(info.symbol, info);
  holderNames.add(info.symbol.escapedName);
  needRescan = true;
}
let needRescan = false;
function scanRefs() {
  for (const [, v] of holderRefCache) { v.calls = []; v.escapes = []; v.exported = false; }
  for (const file of files) {
    const sf = program.getSourceFile(file);
    if (!sf) continue;
    (function visit(n) {
      if (ts.isIdentifier(n) && holderNames.has(n.escapedText)) {
        let s = sym(n);
        if (s && s.flags & ts.SymbolFlags.Alias) s = unalias(s);
        // a shorthand `{ run }` resolves to the property; take the value symbol
        if (n.parent && ts.isShorthandPropertyAssignment(n.parent)) s = checker.getShorthandAssignmentValueSymbol(n.parent) || s;
        if (s && holderRefCache.has(s)) {
          const refs = holderRefCache.get(s);
          const info = holderSyms.get(s);
          const c = classifyRef(n, s, info);
          if (c[0] === 'call') refs.calls.push([c[1], c[2]]);
          else if (c[0] === 'export') refs.exported = true;
          else if (c[0] === 'escape') refs.escapes.push(c[1]);
          // module.exports.x = run / exports.run = run
          if (c[0] === 'escape' && c[1] === 'assigned_or_operand') {
            const q = n.parent; if (ts.isBinaryExpression(q) && /^(module\.)?exports\b/.test(q.left.getText())) refs.exported = true;
          }
        }
      }
      ts.forEachChild(n, visit);
    })(sf);
  }
}
scanRefs();
// forwarded parameters may name holders we did not collect; iterate to a fixed point (bounded)
for (let round = 0; round < 4; round++) {
  needRescan = false;
  for (const t of todo) {
    const fn = t.pb.param.parent;
    // walk callers' forwarded params
    const refs = holderRefCache.get(holderInfo(fn).symbol);
    if (!refs) continue;
    for (const [call] of refs.calls) {
      const args = call.arguments || [];
      const exprs = args.slice();
      if (call.attributes) for (const a of call.attributes.properties) if (ts.isJsxAttribute(a) && a.initializer && a.initializer.expression) exprs.push(a.initializer.expression);
      for (const a of exprs) {
        const e = peel(a);
        if (e && ts.isIdentifier(e)) { const pb = paramBinding(e); if (pb && pb.param) ensureHolder(holderInfo(pb.param.parent)); }
      }
    }
  }
  if (!needRescan) break;
  // newly added holders may themselves forward; collect their forwarded params too
  scanRefs();
  for (const [s, info] of [...holderSyms]) {
    const refs = holderRefCache.get(s);
    for (const [call] of refs.calls) {
      const exprs = (call.arguments || []).slice();
      if (call.attributes) for (const a of call.attributes.properties) if (ts.isJsxAttribute(a) && a.initializer && a.initializer.expression) exprs.push(a.initializer.expression);
      for (const a of exprs) { const e = peel(a); if (e && ts.isIdentifier(e)) { const pb = paramBinding(e); if (pb && pb.param) ensureHolder(holderInfo(pb.param.parent)); } }
    }
  }
  if (needRescan) scanRefs();
}

// 3. per-site follow
const pct = (xs, q) => { if (!xs.length) return null; const s = xs.slice().sort((a, b) => a - b); return s[Math.min(s.length - 1, Math.ceil(q * s.length) - 1)]; };
function bucket(t) {
  const v = paramValue(t.pb, 0);
  const meta = v.meta || {};
  const targets = [...v.targets];
  const followable = targets.filter(([, x]) => x.followable);
  const status = meta.no_callers ? 'no_callers'
    : v.open.length === 0 && targets.length > 0 ? 'closed'
      : v.open.length === 0 ? 'closed_empty'
        : targets.length ? 'open_with_targets' : 'open_no_targets';
  const allFollowable = targets.length > 0 && followable.length === targets.length;
  return { v, meta, targets, followable, status, allFollowable };
}
const S = {
  product: { param_sites: 0, param_sites_dropped: 0, prism_reverse_edge_sites: 0, prism_set_sizes: [], prism_named_target_sites: 0, prism_named_set_sizes: [], closed: 0, closed_prism_covered: 0 },
  param_sites: { by_status: {}, open_reasons: {}, exported_or_escaping: 0, exported: 0, escaping: 0, no_holder_symbol: 0, destructured_props: 0, destructured_props_dropped: 0 },
  closed: { sites: 0, all_targets_followable: 0, prism_covers_every_caller: 0, closed_followable_prism_covered: 0, set_sizes: [] },
  yield: { prism_named_target_sites: 0, prism_named_set_sizes: [], prism_reverse_edge_sites: 0, prism_reverse_edge_set_sizes: [], sites_with_followable_target: 0, sites_with_followable_target_dropped_today: 0, set_sizes: [], sites_with_any_target: 0, any_set_sizes: [], by_via: {} },
  member_of_param: { props_member_sites: 0, props_member_dropped: 0, prism_reverse_edge_sites: 0, prism_reverse_edge_set_sizes: [], by_status: {}, sites_with_followable_target: 0, set_sizes: [], closed_followable: 0 },
};
for (const t of todo) {
  const b = bucket(t);
  const isMember = t.form === 'member';
  if (isMember) {
    // only props-style members: the holder is rendered as a JSX element somewhere in the repo
    const hs = holderInfo(t.pb.param.parent).symbol;
    const jsxHolder = (holderRefCache.get(hs) || { calls: [] }).calls.some(([c]) => ts.isJsxOpeningElement(c) || ts.isJsxSelfClosingElement(c));
    if (!jsxHolder) continue;
    S.member_of_param.props_member_sites++;
    if (t.r.drop) S.member_of_param.props_member_dropped++;
    inc(S.member_of_param.by_status, b.status);
    if (b.followable.length) { S.member_of_param.sites_with_followable_target++; S.member_of_param.set_sizes.push(b.followable.length); }
    if (b.status === 'closed' && b.allFollowable) S.member_of_param.closed_followable++;
    if ((b.meta.prism_targets || []).length) { S.member_of_param.prism_reverse_edge_sites++; S.member_of_param.prism_reverse_edge_set_sizes.push(b.meta.prism_targets.length); }
    if (!aggregateOnly) siteRows.push(row(t, b));
    continue;
  }
  const P = S.param_sites;
  const prod = !isTestPath(t.r.caller.file);
  if (prod) { S.product.param_sites++; if (t.r.drop) S.product.param_sites_dropped++; if ((b.meta.prism_targets || []).length) { S.product.prism_reverse_edge_sites++; S.product.prism_set_sizes.push(b.meta.prism_targets.length); const anonOnly = b.meta.prism_targets.every((k) => /:<anon>@/.test(k)); if (!anonOnly) { S.product.prism_named_target_sites++; S.product.prism_named_set_sizes.push(b.meta.prism_targets.filter((k) => !/:<anon>@/.test(k)).length); } } if (b.status === 'closed') S.product.closed++; if (b.status === 'closed' && b.meta.calls > 0 && b.meta.callers_prism_covered === b.meta.calls) S.product.closed_prism_covered++; }
  { const pt = b.meta.prism_targets || []; const named = pt.filter((k) => !/:<anon>@/.test(k)); if (named.length) { S.yield.prism_named_target_sites++; S.yield.prism_named_set_sizes.push(named.length); } }
  inc(P.by_status, b.status);
  for (const o of new Set(b.v.open)) inc(P.open_reasons, o.replace(/^escape_.*/, (m) => m));
  if (b.meta.exported) P.exported++;
  if (b.meta.escapes && b.meta.escapes.length) P.escaping++;
  if (b.meta.exported || (b.meta.escapes && b.meta.escapes.length)) P.exported_or_escaping++;
  if (!holderInfo(t.pb.param.parent).symbol) P.no_holder_symbol++;
  const isProps = t.pb.kind.startsWith('destructured') || t.pb.kind === 'body_destructure_of_param';
  const jsxCalled = (holderRefCache.get(holderInfo(t.pb.param.parent).symbol) || { calls: [] }).calls.some(([c]) => ts.isJsxOpeningElement(c) || ts.isJsxSelfClosingElement(c));
  if (isProps && jsxCalled) { P.destructured_props++; if (t.r.drop) P.destructured_props_dropped++; }
  if (b.status === 'closed') {
    S.closed.sites++;
    S.closed.set_sizes.push(b.targets.length);
    if (b.allFollowable) S.closed.all_targets_followable++;
    const covered = b.meta.calls > 0 && b.meta.callers_prism_covered === b.meta.calls;
    if (covered) S.closed.prism_covers_every_caller++;
    if (covered && b.allFollowable) S.closed.closed_followable_prism_covered++;
  }
  if (b.targets.length) { S.yield.sites_with_any_target++; S.yield.any_set_sizes.push(b.targets.length); }
  if (b.followable.length) {
    S.yield.sites_with_followable_target++;
    if (t.r.drop) S.yield.sites_with_followable_target_dropped_today++;
    S.yield.set_sizes.push(b.followable.length);
    for (const [, x] of b.followable) inc(S.yield.by_via, x.via);
  }
  const pt = (b.meta.prism_targets || []).length;
  if (pt) { S.yield.prism_reverse_edge_sites++; S.yield.prism_reverse_edge_set_sizes.push(pt); }
  if (!aggregateOnly) siteRows.push(row(t, b));
}
function row(t, b) {
  return { file: t.r.caller.file, line: t.r.source_span.line, callee: t.r.callee_text, drop: t.r.drop, form: t.form, param_kind: t.pb.kind, holder: holderInfo(t.pb.param.parent).kind,
    status: b.status, open: [...new Set(b.v.open)], targets: b.targets.map(([k, x]) => ({ k, followable: x.followable, via: x.via })), calls: b.meta.calls, prism_covered: b.meta.callers_prism_covered, prism_targets: b.meta.prism_targets };
}
const dist = (xs) => ({ n: xs.length, median: pct(xs, 0.5), p90: pct(xs, 0.9), max: xs.length ? Math.max(...xs) : null });
for (const k of ['closed']) S[k].set_sizes = dist(S[k].set_sizes);
S.yield.set_sizes = dist(S.yield.set_sizes);
S.yield.any_set_sizes = dist(S.yield.any_set_sizes);
S.yield.prism_reverse_edge_set_sizes = dist(S.yield.prism_reverse_edge_set_sizes);
S.yield.prism_named_set_sizes = dist(S.yield.prism_named_set_sizes);
S.product.prism_set_sizes = dist(S.product.prism_set_sizes);
S.product.prism_named_set_sizes = dist(S.product.prism_named_set_sizes);
S.member_of_param.set_sizes = dist(S.member_of_param.set_sizes);
S.member_of_param.prism_reverse_edge_set_sizes = dist(S.member_of_param.prism_reverse_edge_set_sizes);

// parameter-callee calls that prism holds no record for (callbacks inside inline arrows etc.)
let unrecordedParamCalls = 0;
for (const n of allCallish) {
  if (recordsBySpan.has(n)) continue;
  const c = calleeExpr(n);
  if (c && ts.isIdentifier(c) && paramBinding(c)) unrecordedParamCalls++;
}
agg.param_callee_calls_without_prism_record = unrecordedParamCalls;
// every call/JSX node with no prism record, by its innermost enclosing callable
const noRecord = { total_call_nodes: allCallish.length, product_call_nodes: allCallish.filter((n) => !isTestPath(path.relative(root, n.getSourceFile().fileName))).length, without_record: 0, by_enclosing: {} };
for (const n of allCallish) {
  if (recordsBySpan.has(n)) continue;
  noRecord.without_record++;
  let f = n.parent;
  while (f && !(ts.isFunctionLike(f) && f.body) && !ts.isSourceFile(f)) f = f.parent;
  let k;
  if (!f || ts.isSourceFile(f)) k = 'module_level';
  else if (ts.isArrowFunction(f) || ts.isFunctionExpression(f)) {
    const h = holderInfo(f);
    k = ['const_function', 'let_var_function', 'wrapped_const', 'object_property_function', 'class_property_function', 'assigned_function'].includes(h.kind) ? 'named_function_expression' : 'anonymous_' + h.kind;
  } else k = 'named_' + ts.SyntaxKind[f.kind];
  inc(noRecord.by_enclosing, k);
  if (!isTestPath(path.relative(root, n.getSourceFile().fileName))) { noRecord.product_without_record = (noRecord.product_without_record || 0) + 1; if (k.startsWith('anonymous_')) noRecord.product_in_anonymous = (noRecord.product_in_anonymous || 0) + 1; }
  else if (k.startsWith('anonymous_')) noRecord.test_in_anonymous = (noRecord.test_in_anonymous || 0) + 1;
}
agg.calls_without_prism_record = noRecord;
agg.follow = S;
fs.mkdirSync(out, { recursive: true, mode: 0o700 });
fs.writeFileSync(path.join(out, 'aggregate.json'), JSON.stringify(agg, null, 2) + '\n', { mode: 0o600 });
if (!aggregateOnly) fs.writeFileSync(path.join(out, 'param-sites.jsonl'), siteRows.map((x) => JSON.stringify(x)).join('\n') + '\n', { mode: 0o600 });
console.log(JSON.stringify(agg, null, 2));
