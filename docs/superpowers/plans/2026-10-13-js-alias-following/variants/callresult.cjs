// Syntactic value-following census for callees bound to a local (hook results, destructures, members).
// For every recorded census site in the `local` group, follow the callee's binding with syntax only:
// through const initialisers, object/array literals, destructuring keys, and one or more "returned
// value" steps into a called function. The called function is found through prism's own Exact
// target for the initialiser call (so the count reflects prism's current import resolution), or
// lexically when it is in the same file. No type information is used.
//   node callresult.cjs <typescript.js> <corpus-root> <sites.jsonl> <functions.json> <out-dir> [--check]
// --check also asks the pinned checker for the resolved declaration and compares positions
// (meaningful on typed corpora only). Prints aggregate counts only; per-site rows go to
// <out-dir>/callresult-rows.jsonl (mode 600).
'use strict';
const fs = require('fs');
const path = require('path');
const args = process.argv.slice(2);
const check = args.includes('--check');
const [tsPath, rootArg, sitesFile, functionsFile, outDir] = args.filter((a) => a !== '--check');
const ts = require(tsPath);
const root = fs.realpathSync(rootArg);
const MAX_HOPS = 4;

const allRows = fs.readFileSync(sitesFile, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
const rowIndex = new Map(); // file -> Map("startByte:name" -> row)
for (const r of allRows) {
  if (!rowIndex.has(r.file)) rowIndex.set(r.file, new Map());
  const m = rowIndex.get(r.file);
  for (const [start] of [r.span, ...(r.join_spans || [])]) m.set(`${start}:${r.name}`, r);
}
const inventory = new Map(); // "file:line" -> 'named' | 'anonymous'
for (const f of JSON.parse(fs.readFileSync(functionsFile, 'utf8'))) {
  const k = `${f.file}:${f.start_line}`;
  if (!inventory.has(k) || f.name) inventory.set(k, f.name ? 'named' : 'anonymous');
}
let deps = new Set();
try {
  const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
  for (const k of ['dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies']) for (const d of Object.keys(pkg[k] || {})) deps.add(d);
} catch (e) { deps = new Set(); }
const pkgName = (spec) => (spec.startsWith('@') ? spec.split('/').slice(0, 2).join('/') : spec.split('/')[0]);

const parsed = new Map();
function load(file) {
  if (parsed.has(file)) return parsed.get(file);
  let rec = null;
  try {
    const text = fs.readFileSync(path.join(root, file), 'utf8');
    const kind = /\.tsx$/.test(file) ? ts.ScriptKind.TSX : /\.ts$/.test(file) ? ts.ScriptKind.TS : ts.ScriptKind.JSX;
    rec = { file, text, sf: ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, kind) };
  } catch (e) { rec = null; }
  parsed.set(file, rec);
  return rec;
}
const lineOf = (rec, node) => rec.sf.getLineAndCharacterOfPosition(node.getStart(rec.sf)).line + 1;
const byteOf = (rec, pos) => Buffer.byteLength(rec.text.slice(0, pos));
const isFn = (n) => ts.isFunctionDeclaration(n) || ts.isFunctionExpression(n) || ts.isArrowFunction(n) || ts.isMethodDeclaration(n) || ts.isConstructorDeclaration(n) || ts.isGetAccessorDeclaration(n) || ts.isSetAccessorDeclaration(n);

function unwrap(e) {
  let awaited = false;
  for (;;) {
    if (!e) break;
    if (ts.isParenthesizedExpression(e) || ts.isAsExpression(e) || ts.isNonNullExpression(e) || ts.isTypeAssertionExpression(e) || (ts.isSatisfiesExpression && ts.isSatisfiesExpression(e))) e = e.expression;
    else if (ts.isAwaitExpression(e)) { awaited = true; e = e.expression; } else break;
  }
  return { e, awaited };
}

// ---- lexical lookup ------------------------------------------------------------------------
function eachBindingName(nameNode, cb) {
  if (ts.isIdentifier(nameNode)) cb(nameNode);
  else if (ts.isObjectBindingPattern(nameNode) || ts.isArrayBindingPattern(nameNode)) {
    for (const el of nameNode.elements) if (ts.isBindingElement(el)) eachBindingName(el.name, cb);
  }
}
function declIn(list, name) {
  for (const d of list) {
    let hit = null;
    eachBindingName(d.name, (id) => { if (!hit && id.text === name) hit = id; });
    if (hit) return hit.parent; // VariableDeclaration, Parameter or BindingElement
  }
  return null;
}
function scanStatements(stmts, name) {
  for (const s of stmts) {
    if (ts.isVariableStatement(s)) { const d = declIn(s.declarationList.declarations, name); if (d) return d; }
    else if ((ts.isFunctionDeclaration(s) || ts.isClassDeclaration(s) || ts.isEnumDeclaration(s)) && s.name && s.name.text === name) return s;
    else if (ts.isImportDeclaration(s) && s.importClause) {
      const c = s.importClause;
      if (c.name && c.name.text === name) return c;
      if (c.namedBindings) {
        if (ts.isNamespaceImport(c.namedBindings) && c.namedBindings.name.text === name) return c.namedBindings;
        if (ts.isNamedImports(c.namedBindings)) for (const el of c.namedBindings.elements) if (el.name.text === name) return el;
      }
    } else if (ts.isImportEqualsDeclaration(s) && s.name.text === name) return s;
  }
  return null;
}
function lookup(name, from) {
  for (let n = from.parent; n; n = n.parent) {
    if (isFn(n)) {
      const d = declIn(n.parameters, name);
      if (d) return d;
      if (ts.isFunctionExpression(n) && n.name && n.name.text === name) return n;
    } else if (ts.isBlock(n) || ts.isSourceFile(n) || ts.isModuleBlock(n)) {
      const d = scanStatements(n.statements, name);
      if (d) return d;
    } else if (ts.isCaseClause(n) || ts.isDefaultClause(n)) {
      const d = scanStatements(n.statements, name);
      if (d) return d;
    } else if (ts.isCatchClause(n) && n.variableDeclaration) {
      const d = declIn([n.variableDeclaration], name);
      if (d) return d;
    } else if ((ts.isForStatement(n) || ts.isForOfStatement(n) || ts.isForInStatement(n)) && n.initializer && ts.isVariableDeclarationList(n.initializer)) {
      const d = declIn(n.initializer.declarations, name);
      if (d) return d;
    }
  }
  return null;
}
// Describe a VariableDeclaration / BindingElement / Parameter binding: root, path to the name, initialiser.
function describe(decl) {
  const pathRev = [];
  let n = decl;
  let hasDefault = false;
  while (ts.isBindingElement(n)) {
    const pat = n.parent;
    if (n.dotDotDotToken) pathRev.push({ t: 'rest' });
    else if (ts.isObjectBindingPattern(pat)) {
      const key = n.propertyName || n.name;
      pathRev.push(ts.isIdentifier(key) || ts.isStringLiteral(key) ? { t: 'key', k: key.text } : { t: 'computed' });
    } else pathRev.push({ t: 'index', i: pat.elements.indexOf(n) });
    if (n.initializer) hasDefault = true;
    n = pat.parent;
  }
  const p = pathRev.reverse();
  if (ts.isParameter(n)) return { root: 'param', path: p, hasDefault: hasDefault || !!n.initializer, node: n };
  if (ts.isVariableDeclaration(n)) {
    const list = n.parent;
    const flags = ts.isVariableDeclarationList(list) ? list.flags : 0;
    const kind = flags & ts.NodeFlags.Const ? 'const' : flags & ts.NodeFlags.Let ? 'let' : 'var';
    const inLoop = ts.isVariableDeclarationList(list) && (ts.isForOfStatement(list.parent) || ts.isForInStatement(list.parent));
    return { root: 'var', kind, path: p, hasDefault, init: n.initializer || null, inLoop, node: n };
  }
  return { root: 'other', path: p, node: n };
}

const REACT = new Set(['useState', 'useReducer', 'useCallback', 'useMemo', 'useRef', 'useContext', 'useTransition', 'useImperativeHandle', 'useEffect', 'useLayoutEffect', 'useId', 'useDeferredValue', 'useSyncExternalStore', 'memo', 'forwardRef', 'lazy', 'createContext', 'createRef']);
function reactName(callee, rec) {
  if (ts.isPropertyAccessExpression(callee) && ts.isIdentifier(callee.expression) && callee.expression.text === 'React' && REACT.has(callee.name.text)) return callee.name.text;
  if (!ts.isIdentifier(callee) || !REACT.has(callee.text)) return null;
  const d = lookup(callee.text, callee);
  if (!d) return callee.text; // unbound: a global React hook in a script, or an auto-import
  if (ts.isImportSpecifier(d)) {
    const src = d.parent.parent.parent.moduleSpecifier.text;
    return src === 'react' || src === 'preact/hooks' || src === 'preact/compat' ? (d.propertyName ? d.propertyName.text : d.name.text) : null;
  }
  return null;
}
const res = (cls, extra = {}) => ({ cls, hops: 0, ...extra });
const fnResult = (rec, node, via) => res('fn', { target: { file: rec.file, line: lineOf(rec, node) }, via });

function returnsOf(fn) {
  if (!fn.body) return [];
  if (!ts.isBlock(fn.body)) return [fn.body];
  const out = [];
  const visit = (n) => {
    if (isFn(n) || ts.isClassLike(n)) return;
    if (ts.isReturnStatement(n) && n.expression) out.push(n.expression);
    ts.forEachChild(n, visit);
  };
  ts.forEachChild(fn.body, visit);
  return out;
}
function merge(results) {
  if (!results.length) return res('callee_returns_nothing');
  const key = (r) => `${r.cls}|${r.target ? `${r.target.file}:${r.target.line}` : ''}`;
  const kinds = new Set(results.map(key));
  if (kinds.size === 1) return results[0];
  const real = results.filter((r) => r.cls !== 'literal_nullish');
  if (real.length && new Set(real.map(key)).size === 1) return real[0];
  return res('divergent_returns');
}
function findFunction(rec, line, name) {
  let best = null;
  const visit = (n) => {
    if (best) return;
    if (isFn(n) && lineOf(rec, n) === line) { best = n; return; }
    if (ts.isVariableDeclaration(n) && ts.isIdentifier(n.name) && n.name.text === name && n.initializer && lineOf(rec, n) === line) {
      const u = unwrap(n.initializer).e;
      if (u && isFn(u)) { best = u; return; }
      if (u && ts.isCallExpression(u) && u.arguments.length && isFn(unwrap(u.arguments[0]).e)) { best = unwrap(u.arguments[0]).e; return; }
    }
    ts.forEachChild(n, visit);
  };
  visit(rec.sf);
  return best;
}

// The value of `expr` after selecting `p` (keys/indices) from it.
function select(expr, p, rec, hops) {
  const { e, awaited } = unwrap(expr);
  if (!e) return res('no_value');
  if (awaited && ts.isCallExpression(e) && e.expression.kind === ts.SyntaxKind.ImportKeyword) return res('dynamic_import');
  if (ts.isObjectLiteralExpression(e)) {
    if (!p.length) return res('object_value');
    const [h, ...rest] = p;
    if (h.t !== 'key') return res(h.t === 'rest' ? 'rest_element' : 'unsupported_path');
    let spread = false;
    for (let i = e.properties.length - 1; i >= 0; i -= 1) {
      const pr = e.properties[i];
      if (ts.isSpreadAssignment(pr)) { spread = true; continue; }
      const nm = pr.name && (ts.isIdentifier(pr.name) || ts.isStringLiteral(pr.name)) ? pr.name.text : null;
      if (nm !== h.k) continue;
      if (spread) return res('key_then_spread');
      if (ts.isPropertyAssignment(pr)) return select(pr.initializer, rest, rec, hops);
      if (ts.isShorthandPropertyAssignment(pr)) return select(pr.name, rest, rec, hops);
      if (ts.isMethodDeclaration(pr)) return rest.length ? res('unsupported_path') : fnResult(rec, pr, 'object_method');
      return res('accessor_property');
    }
    return res(spread ? 'spread_object' : 'key_absent');
  }
  if (ts.isArrayLiteralExpression(e)) {
    if (!p.length) return res('array_value');
    const [h, ...rest] = p;
    if (h.t !== 'index') return res('unsupported_path');
    const el = e.elements[h.i];
    if (!el || ts.isSpreadElement(el) || e.elements.slice(0, h.i).some(ts.isSpreadElement)) return res('index_absent');
    return select(el, rest, rec, hops);
  }
  if (isFn(e)) return p.length ? res('member_of_function') : fnResult(rec, e, 'inline');
  if (ts.isIdentifier(e)) {
    if (e.text === 'undefined') return res('literal_nullish');
    const d = lookup(e.text, e);
    if (!d) return res(p.length ? 'unbound_object' : 'unbound_identifier');
    if (ts.isFunctionDeclaration(d)) return p.length ? res('member_of_function') : fnResult(rec, d, 'declaration');
    if (ts.isClassDeclaration(d)) return res(p.length ? 'class_member' : 'class');
    if (ts.isImportSpecifier(d) || ts.isImportClause(d) || ts.isImportEqualsDeclaration(d)) return res(p.length ? 'imported_object_member' : 'imported_binding');
    if (ts.isNamespaceImport(d)) return res(p.length === 1 ? 'namespace_member' : 'namespace_other');
    if (ts.isFunctionExpression(d)) return fnResult(rec, d, 'inline');
    const b = describe(d);
    if (b.root === 'param') return res(b.path.length + p.length ? 'parameter_member' : 'parameter', { paramDefault: b.hasDefault });
    if (b.root !== 'var') return res('other_binding');
    if (b.inLoop) return res('loop_variable');
    if (!b.init) return res('declared_without_value');
    const r = select(b.init, b.path.concat(p), rec, hops);
    if (b.kind !== 'const') r.mutable = true;
    if (b.hasDefault) r.hasDefault = true;
    return r;
  }
  if (ts.isPropertyAccessExpression(e)) {
    if (e.expression.kind === ts.SyntaxKind.ThisKeyword) return res('this_member');
    return select(e.expression, [{ t: 'key', k: e.name.text }].concat(p), rec, hops);
  }
  if (ts.isElementAccessExpression(e)) {
    const a = e.argumentExpression;
    if (a && ts.isNumericLiteral(a)) return select(e.expression, [{ t: 'index', i: Number(a.text) }].concat(p), rec, hops);
    if (a && ts.isStringLiteral(a)) return select(e.expression, [{ t: 'key', k: a.text }].concat(p), rec, hops);
    return res('computed_member');
  }
  if (ts.isCallExpression(e)) return selectCall(e, p, rec, hops, awaited);
  if (ts.isConditionalExpression(e) || (ts.isBinaryExpression(e) && [ts.SyntaxKind.BarBarToken, ts.SyntaxKind.QuestionQuestionToken, ts.SyntaxKind.AmpersandAmpersandToken].includes(e.operatorToken.kind))) return res('conditional_value');
  if (e.kind === ts.SyntaxKind.NullKeyword) return res('literal_nullish');
  if (ts.isTaggedTemplateExpression(e)) return res('tagged_template');
  if (ts.isNewExpression(e)) return res('new_instance');
  return res('other_value');
}

function selectCall(call, p, rec, hops, awaited) {
  const callee = call.expression;
  const rn = reactName(callee, rec);
  if (rn) {
    const a0 = call.arguments[0];
    if (rn === 'useCallback' || rn === 'memo' || rn === 'forwardRef') {
      if (!a0) return res('react_other');
      const r = select(a0, p, rec, hops);
      r.wrapper = rn;
      return r;
    }
    if (rn === 'useMemo') {
      const f = a0 && unwrap(a0).e;
      if (f && isFn(f)) return merge(returnsOf(f).map((x) => select(x, p, rec, hops)));
      return res('react_other');
    }
    if (rn === 'useState' || rn === 'useReducer' || rn === 'useTransition') {
      if (p.length === 1 && p[0].t === 'index' && p[0].i === 1) return res('react_state_setter');
      return res('react_state_value');
    }
    if (rn === 'useContext') return res('react_context');
    if (rn === 'useRef' || rn === 'createRef') return res('react_ref');
    if (rn === 'lazy') return res('react_lazy');
    return res('react_other');
  }
  if (hops >= MAX_HOPS) return res('hop_limit');
  if (!ts.isIdentifier(callee)) {
    if (ts.isPropertyAccessExpression(callee) && callee.name.text === 'bind') return res('bind_result');
    return res('member_call_result');
  }
  // 1. prism's own Exact target for this call
  const row = (rowIndex.get(rec.file) || new Map()).get(`${byteOf(rec, call.getStart(rec.sf))}:${callee.text}`);
  let target = null;
  let how = null;
  if (row) {
    const exact = (row.prism_targets || []).filter((t) => t.confidence === 'exact');
    if (exact.length === 1) { target = exact[0].target; how = 'prism_exact'; }
  }
  // 2. same-file lexical fallback (covers sites prism has no record for)
  let fnNode = null;
  let trec = rec;
  if (!target) {
    const d = lookup(callee.text, callee);
    if (d && ts.isFunctionDeclaration(d)) { fnNode = d; how = 'same_file'; }
    else if (d && ts.isVariableDeclaration(d) && d.initializer && isFn(unwrap(d.initializer).e)) { fnNode = unwrap(d.initializer).e; how = 'same_file'; }
    else if (d && (ts.isImportSpecifier(d) || ts.isImportClause(d) || ts.isNamespaceImport(d))) {
      let c = d;
      while (c && !ts.isImportDeclaration(c)) c = c.parent;
      const spec = c.moduleSpecifier.text;
      if (spec.startsWith('.')) return res(row && row.joined ? 'callee_unresolved_relative_import' : 'callee_relative_import_no_record');
      return res(deps.has(pkgName(spec)) ? 'callee_external_package' : 'callee_nonrelative_not_a_dependency');
    } else if (d) {
      const b = ts.isVariableDeclaration(d) || ts.isBindingElement(d) || ts.isParameter(d) ? describe(d) : null;
      if (b && b.root === 'param') return res('callee_is_parameter');
      return res('callee_is_local_value');
    } else return res('callee_unbound');
  } else {
    trec = load(target.file);
    if (!trec) return res('callee_file_unreadable');
    fnNode = findFunction(trec, target.start_line, target.name);
    if (!fnNode) return res('callee_node_not_found');
  }
  const r = merge(returnsOf(fnNode).map((x) => select(x, p, trec, hops + 1)));
  const out = { ...r, hops: (r.hops || 0) + 1, calleeVia: r.calleeVia || how };
  return out;
}

// ---- per-site ------------------------------------------------------------------------------
function findDecl(row) {
  const d = (row.declarations || [])[0];
  if (!d) return null;
  const rec = load(d.file);
  if (!rec) return null;
  let hit = null;
  const visit = (n) => {
    if (hit) return;
    if ((ts.isBindingElement(n) || ts.isVariableDeclaration(n)) && ts.isIdentifier(n.name) && n.name.text === row.name && lineOf(rec, n) === d.line) { hit = n; return; }
    ts.forEachChild(n, visit);
  };
  visit(rec.sf);
  return hit ? { rec, node: hit } : null;
}
function finalClass(r) {
  if (r.cls !== 'fn') return r.cls;
  const inv = inventory.get(`${r.target.file}:${r.target.line}`);
  return inv === 'named' ? 'fn_named' : inv === 'anonymous' ? 'fn_anonymous' : 'fn_not_in_inventory';
}

const sites = allRows.filter((r) => r.joined && r.group === 'local');
const out = [];
for (const r of sites) {
  let result;
  const found = findDecl(r);
  if (!found) result = res('declaration_not_located');
  else {
    const b = describe(found.node);
    if (b.root !== 'var') result = res('not_a_variable');
    else if (b.inLoop) result = res('loop_variable');
    else if (!b.init) result = res('declared_without_value');
    else {
      try { result = select(b.init, b.path, found.rec, 0); } catch (e) { result = res('analysis_error'); }
      if (b.kind !== 'const') result.mutable = true;
      if (b.hasDefault) result.hasDefault = true;
    }
  }
  out.push({ file: r.file, line: r.line, name: r.name, binding: r.binding, test: r.test, verdict: r.verdict, cls: finalClass(result), hops: result.hops || 0, via: result.via || null, wrapper: result.wrapper || null, calleeVia: result.calleeVia || null, mutable: !!result.mutable, hasDefault: !!result.hasDefault, target: result.target || null, span: r.join_spans && r.join_spans[0] ? r.join_spans[0] : r.span });
}

// ---- optional checker comparison -------------------------------------------------------------
let checked = null;
if (check) {
  let options = { allowJs: true, checkJs: true, noEmit: true, skipLibCheck: true, types: [], jsx: ts.JsxEmit.Preserve, target: ts.ScriptTarget.ESNext, module: ts.ModuleKind.ESNext, moduleResolution: ts.ModuleResolutionKind.Bundler };
  for (const name of ['tsconfig.json', 'tsconfig.base.json', 'jsconfig.json']) {
    const p = path.join(root, name);
    if (!fs.existsSync(p)) continue;
    const cfg = ts.readConfigFile(p, ts.sys.readFile);
    if (cfg.error) continue;
    const pc = ts.parseJsonConfigFileContent(cfg.config, ts.sys, root);
    if (pc.options.paths || pc.options.baseUrl) { options = { ...options, baseUrl: pc.options.baseUrl || root, paths: pc.options.paths }; break; }
  }
  const withTarget = out.filter((x) => x.target);
  const program = ts.createProgram([...new Set(withTarget.map((x) => path.join(root, x.file)))], options);
  const checker = program.getTypeChecker();
  checked = {};
  const nodesByFile = new Map();
  for (const x of withTarget) {
    const abs = path.join(root, x.file);
    const sf = program.getSourceFile(abs);
    let verdict = 'site_not_located';
    if (sf) {
      if (!nodesByFile.has(abs)) {
        const m = new Map();
        const visit = (n) => {
          let c = null;
          if (ts.isCallExpression(n)) c = n.expression;
          else if (ts.isJsxSelfClosingElement(n) || ts.isJsxOpeningElement(n)) c = n.tagName;
          if (c && ts.isIdentifier(c)) { const k = `${c.getStart(sf)}:${c.text}`; if (!m.has(k)) m.set(k, n); }
          ts.forEachChild(n, visit);
        };
        visit(sf);
        nodesByFile.set(abs, { m, buf: fs.readFileSync(abs) });
      }
      const nf = nodesByFile.get(abs);
      const base = nf.buf.subarray(0, x.span[0]).toString('utf8').length;
      const node = nf.m.get(`${base}:${x.name}`) || nf.m.get(`${base + 1}:${x.name}`);
      if (node) {
        let sig = null;
        try { sig = checker.getResolvedSignature(node); } catch (e) { sig = null; }
        const d = sig && sig.declaration;
        if (!d) verdict = 'checker_no_declaration';
        else {
          const dsf = d.getSourceFile();
          const rel = path.relative(root, dsf.fileName);
          const dl = dsf.getLineAndCharacterOfPosition(d.getStart(dsf)).line + 1;
          const hasBody = isFn(d) && !!d.body;
          if (rel === x.target.file && dl === x.target.line) verdict = 'agree';
          else if (!hasBody) verdict = 'checker_type_only_or_external';
          else verdict = 'disagree';
        }
      }
    }
    const k = `${x.cls}|${verdict}`;
    checked[k] = (checked[k] || 0) + 1;
    x.check = verdict;
  }
}

const tally = {};
for (const x of out) {
  const k = `${x.binding}|${x.test ? 'test' : 'product'}|${x.verdict}|${x.cls}|hops${x.hops}`;
  tally[k] = (tally[k] || 0) + 1;
}
fs.mkdirSync(outDir, { recursive: true, mode: 0o700 });
fs.writeFileSync(path.join(outDir, 'callresult-rows.jsonl'), out.map((x) => JSON.stringify(x)).join('\n') + '\n', { mode: 0o600 });
const summary = { sites: out.length, max_hops: MAX_HOPS, dependencies_known: deps.size, tally, checked };
fs.writeFileSync(path.join(outDir, 'callresult-aggregate.json'), JSON.stringify(summary, null, 2) + '\n', { mode: 0o600 });
console.log(JSON.stringify(summary));
