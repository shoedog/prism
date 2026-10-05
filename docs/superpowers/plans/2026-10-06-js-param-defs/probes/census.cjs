#!/usr/bin/env node
// Step-0 census for lane js-param-defs (all four gaps), syntax + TS-checker symbol identity.
// Usage: node census.cjs <typescript.js> <root> <out.json> [--multi]
//   --multi: <root> holds <class>/<entry>/src package roots (SecBench layout); one program per root.
// Aggregate-only: out.json holds counts, never source text or paths outside the summary keys.
// Mirrors prism's file walk (src/repo_loader.rs: skip hidden, .git/target/node_modules/vendor/dist/build,
// files > 2 MiB, symlinks; extensions js/mjs/cjs/jsx/ts/tsx) and prism's callable naming
// (src/languages/mod.rs::function_name ECMAScript name-inference patterns 1-5).
'use strict';
const fs = require('fs');
const path = require('path');
const [tsPath, root, outFile, ...flags] = process.argv.slice(2);
const ts = require(tsPath);
const multi = flags.includes('--multi');
const SKIP = new Set(['.git', 'target', 'node_modules', 'vendor', 'dist', 'build']);
const EXT = new Set(['.js', '.mjs', '.cjs', '.jsx', '.ts', '.tsx']);

function walk(dir, out) {
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    if (ent.name.startsWith('.')) continue;
    const p = path.join(dir, ent.name);
    if (ent.isSymbolicLink()) continue;
    if (ent.isDirectory()) { if (!SKIP.has(ent.name)) walk(p, out); continue; }
    if (!ent.isFile() || !EXT.has(path.extname(ent.name))) continue;
    if (fs.statSync(p).size > 2 * 1024 * 1024) continue;
    out.push(p);
  }
  return out;
}

const inc = (o, k, n = 1) => { o[k] = (o[k] || 0) + n; };
const S = ts.SyntaxKind;

function unwrapParentKind(n) { return n.parent; }

// prism function nodes: function_declaration, method_definition, arrow_function,
// function_expression (non-generator), generator_function_declaration; body required.
function isPrismCallable(n) {
  if (ts.isArrowFunction(n)) return true;
  if (ts.isFunctionExpression(n)) return !n.asteriskToken;
  if (ts.isFunctionDeclaration(n)) return !!n.body;
  if (ts.isMethodDeclaration(n) || ts.isConstructorDeclaration(n) || ts.isGetAccessorDeclaration(n) || ts.isSetAccessorDeclaration(n)) return !!n.body;
  return false;
}

// Mirror of Language::function_name for JS/TS (direct tree-sitter parent; parentheses are a parent).
function prismNamed(n) {
  if (ts.isArrowFunction(n) || (ts.isFunctionExpression(n) && !n.name)) {
    const p = n.parent;
    if (ts.isVariableDeclaration(p) && p.initializer === n) return true;
    if (ts.isPropertyAssignment(p) && p.initializer === n) return true;
    if (ts.isCallExpression(p) && p.arguments.includes(n) && ts.isVariableDeclaration(p.parent) && p.parent.initializer === p) return true;
    if (ts.isPropertyDeclaration(p) && p.initializer === n) return true;
    if (ts.isBinaryExpression(p) && p.operatorToken.kind === S.EqualsToken && p.right === n &&
        (ts.isPropertyAccessExpression(p.left) || ts.isIdentifier(p.left))) return true;
    return false;
  }
  if (ts.isFunctionDeclaration(n)) return !!n.name;
  return true; // methods, named function expressions
}

function anonParentKind(n) {
  const p = n.parent;
  if (ts.isCallExpression(p) && p.arguments.includes(n)) return 'call_argument';
  if (ts.isNewExpression(p) && (p.arguments || []).includes(n)) return 'new_argument';
  if (ts.isJsxExpression(p)) return 'jsx_expression';
  if (ts.isReturnStatement(p)) return 'return';
  if (ts.isParenthesizedExpression(p)) return ts.isCallExpression(p.parent) && p.parent.expression === p ? 'iife' : 'parenthesized_other';
  if (ts.isExportAssignment(p)) return 'export_default';
  if (ts.isArrayLiteralExpression(p)) return 'array_element';
  if (ts.isConditionalExpression(p)) return 'conditional';
  if (ts.isBinaryExpression(p)) return p.operatorToken.kind === S.EqualsToken ? 'assignment_other_lhs' : 'binary_or_logical';
  if (ts.isParameter(p)) return 'default_parameter';
  if (ts.isArrowFunction(p)) return 'arrow_body (curried)';
  if (ts.isFunctionDeclaration(n) && ts.isSourceFile(p)) return 'export_default';
  if (ts.isAsExpression(p) || ts.isSatisfiesExpression?.(p) || ts.isNonNullExpression(p) || ts.isTypeAssertionExpression?.(p)) return 'type_wrapper';
  if (ts.isShorthandPropertyAssignment(p) || ts.isPropertyAssignment(p)) return 'property_other';
  if (ts.isAwaitExpression(p) || ts.isYieldExpression(p)) return 'await_yield';
  if (ts.isTaggedTemplateExpression(p) || ts.isTemplateSpan(p)) return 'template';
  if (ts.isVariableDeclaration(p)) return 'declarator_pattern';
  return 'other:' + S[p.kind];
}

function isBareArrow(n, sf) {
  if (!ts.isArrowFunction(n) || n.parameters.length !== 1) return false;
  return !n.getChildren(sf).some((c) => c.kind === S.OpenParenToken);
}

// Classify the uses of a parameter symbol inside its owner callable via checker identity.
function useProfile(checker, fn, sym, sf) {
  const prof = { bare: 0, member: 0, memberPaths: new Set(), bareLines: new Set(), write: 0 };
  const visit = (node) => {
    if (ts.isIdentifier(node) && !(ts.isParameter(node.parent) && node.parent.name === node)) {
      let s = checker.getSymbolAtLocation(node);
      if (ts.isShorthandPropertyAssignment(node.parent) && node.parent.name === node) s = checker.getShorthandAssignmentValueSymbol(node.parent);
      if (s && s === sym) {
        const line = sf.getLineAndCharacterOfPosition(node.getStart(sf)).line + 1;
        const par = node.parent;
        if ((ts.isPropertyAccessExpression(par)) && par.expression === node) {
          prof.member++;
          let top = par; while (ts.isPropertyAccessExpression(top.parent) && top.parent.expression === top) top = top.parent;
          prof.memberPaths.add(line + ':' + top.getText(sf).replace(/\s+/g, ''));
        } else {
          prof.bare++; prof.bareLines.add(line);
          if (ts.isBinaryExpression(par) && par.left === node && par.operatorToken.kind >= S.FirstAssignment && par.operatorToken.kind <= S.LastAssignment) prof.write++;
          if ((ts.isPrefixUnaryExpression(par) || ts.isPostfixUnaryExpression(par)) && (par.operator === S.PlusPlusToken || par.operator === S.MinusMinusToken)) prof.write++;
        }
      }
    }
    ts.forEachChild(node, visit);
  };
  if (fn.body) visit(fn.body);
  return prof;
}

function paramShape(p) {
  if (p.name.kind === S.Identifier && p.name.text === 'this') return 'this';
  if (p.dotDotDotToken) return p.name.kind === S.Identifier ? 'rest_ident' : 'rest_pattern';
  if (p.name.kind !== S.Identifier) return 'destructured';
  if (p.initializer) return 'default';
  return 'plain';
}

function newSummary() {
  return {
    files: 0, parse_diagnostics_files: 0, callables: 0, named_callables: 0, anonymous_callables: 0,
    anonymous_by_parent: {}, anonymous_with_params: 0,
    params_by_shape: {},
    gap1_callback: { anon_params_plain_bare_used: 0, anon_params_plain_member_only: 0, projected_def_rows: 0, projected_use_rows: 0, anon_by_parent_with_bare_param: {} },
    gap2_member_only: { named_owner_plain: 0, anon_owner_plain: 0, member_use_paths_named: 0, member_use_paths_anon: 0 },
    gap3_bare_arrow: { total: 0, named_owner: 0, anon_owner: 0, named_bare_used: 0, named_member_only: 0, named_unused: 0, anon_bare_used: 0, projected_prA_def_rows: 0, projected_prA_use_rows: 0, async: 0, curried_inner: 0 },
    gap4_rest: { ident_total: 0, pattern_total: 0, named_owner: 0, anon_owner: 0, named_bare_used: 0, named_member_only: 0, named_unused: 0, anon_bare_used: 0, with_type_annotation: 0, projected_prA_def_rows: 0, projected_prA_use_rows: 0, written: 0 },
  };
}

function censusProgram(files, sum) {
  const program = ts.createProgram(files, { allowJs: true, noResolve: true, noLib: true, types: [], jsx: ts.JsxEmit.Preserve, target: ts.ScriptTarget.ESNext, skipLibCheck: true });
  const checker = program.getTypeChecker();
  for (const sf of program.getSourceFiles()) {
    if (!files.includes(sf.fileName) && !files.includes(path.resolve(sf.fileName))) continue;
    sum.files++;
    if ((sf.parseDiagnostics || []).length) sum.parse_diagnostics_files++;
    const visit = (n) => {
      if (isPrismCallable(n)) {
        sum.callables++;
        const named = prismNamed(n);
        if (named) sum.named_callables++; else { sum.anonymous_callables++; inc(sum.anonymous_by_parent, anonParentKind(n)); if (n.parameters.length) sum.anonymous_with_params++; }
        const bareArrow = isBareArrow(n, sf);
        if (bareArrow) {
          const g = sum.gap3_bare_arrow; g.total++;
          if (named) g.named_owner++; else g.anon_owner++;
          if (n.modifiers && n.modifiers.some((m) => m.kind === S.AsyncKeyword)) g.async++;
          if (ts.isArrowFunction(n.parent)) g.curried_inner++;
        }
        let anonHasBare = false;
        for (const p of n.parameters) {
          const shape = paramShape(p);
          inc(sum.params_by_shape, (named ? 'named:' : 'anon:') + shape);
          if (!['plain', 'rest_ident', 'default'].includes(shape)) { if (shape === 'rest_pattern') sum.gap4_rest.pattern_total++; continue; }
          const sym = checker.getSymbolAtLocation(p.name);
          if (!sym) continue;
          const prof = useProfile(checker, n, sym, sf);
          const cls = prof.bare > 0 ? 'bare' : prof.member > 0 ? 'member_only' : 'unused';
          if (bareArrow) {
            const g = sum.gap3_bare_arrow;
            if (named) { inc(g, 'named_' + (cls === 'bare' ? 'bare_used' : cls)); if (cls === 'bare') { g.projected_prA_def_rows++; g.projected_prA_use_rows += prof.bareLines.size; } }
            else if (cls === 'bare') g.anon_bare_used++;
          }
          if (shape === 'rest_ident') {
            const g = sum.gap4_rest; g.ident_total++;
            if (p.type) g.with_type_annotation++;
            if (prof.write) g.written++;
            if (named) { g.named_owner++; inc(g, 'named_' + (cls === 'bare' ? 'bare_used' : cls)); if (cls === 'bare') { g.projected_prA_def_rows++; g.projected_prA_use_rows += prof.bareLines.size; } }
            else { g.anon_owner++; if (cls === 'bare') g.anon_bare_used++; }
          }
          if (shape === 'plain' && cls === 'member_only') {
            const g = sum.gap2_member_only;
            if (named) { g.named_owner_plain++; g.member_use_paths_named += prof.memberPaths.size; }
            else { g.anon_owner_plain++; g.member_use_paths_anon += prof.memberPaths.size; }
          }
          if (!named && (shape === 'plain' || shape === 'rest_ident' || bareArrow)) {
            const g = sum.gap1_callback;
            if (cls === 'bare') { g.anon_params_plain_bare_used++; g.projected_def_rows++; g.projected_use_rows += prof.bareLines.size; anonHasBare = true; }
            else if (cls === 'member_only') g.anon_params_plain_member_only++;
          }
        }
        if (anonHasBare) inc(sum.gap1_callback.anon_by_parent_with_bare_param, anonParentKind(n));
      }
      ts.forEachChild(n, visit);
    };
    visit(sf);
  }
}

const sum = newSummary();
let roots = [root];
if (multi) {
  roots = [];
  const mf = path.join(root, 'manifest.json');
  const ok = fs.existsSync(mf) ? new Set(JSON.parse(fs.readFileSync(mf, 'utf8')).entries.filter((e) => e.status === 'ok').map((e) => e.class + '/' + e.entry)) : null;
  for (const cls of fs.readdirSync(root).sort()) {
    const cdir = path.join(root, cls);
    if (!fs.statSync(cdir).isDirectory()) continue;
    for (const entry of fs.readdirSync(cdir).sort()) {
      const src = path.join(cdir, entry, 'src');
      if (fs.existsSync(src) && (!ok || ok.has(cls + '/' + entry))) roots.push(src);
    }
  }
}
for (const r of roots) {
  const files = walk(path.resolve(r), []).sort();
  if (files.length) censusProgram(files, sum);
}
sum.roots = roots.length;
fs.writeFileSync(outFile, JSON.stringify(sum, null, 1) + '\n');
process.stdout.write(JSON.stringify({ roots: sum.roots, files: sum.files, anonymous_callables: sum.anonymous_callables, gap3: sum.gap3_bare_arrow.total, gap4: sum.gap4_rest.ident_total }) + '\n');
