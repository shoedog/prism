#!/usr/bin/env node
// EVALUATION.md §3.5 steps 1-2 for changed DFG rows (planner probe; aggregate stdout).
// Usage: node adjudicate.cjs <typescript.js> <corpus root> <changed.jsonl> <out.json> [--details <file>]
//   changed.jsonl: rowdiff.py --rows output ({class, row, base_label?, head_label?}).
// Step 1 (binding identity, pinned TS 5.9.3 checker):
//   def->use row: every identifier spelled like the row's base on the Use line is resolved;
//     CORRECT  some occurrence binds to a parameter declaration whose owning callable starts on
//              the Def line (prism pins parameter Defs to the callable start line);
//     WRONG    occurrences exist and none binds to such a parameter;
//     UNDECIDED no occurrence on the line (synthetic/zero-width Use) or no symbol.
//   use->def row (Step 5b argument->formal): a call on the Use line whose argument at index k is
//     the row's identifier must resolve (checker signature or callee symbol) to a callable that
//     starts on the Def line and whose parameter k is the row's formal (never a rest element).
// Step 2 (Exact labels on CORRECT def->use rows): any write to the formal's symbol textually
//   before the Use (outside the Use's own assignment right-hand side), or inside a loop that also
//   contains the Use, is reported as EXACT_PRIOR_WRITE (strict rule; parity control decides).
//   CFG reachability is NOT checked here (disclosed).
'use strict';
const fs = require('fs');
const path = require('path');
const [tsPath, root, changedFile, outFile, ...rest] = process.argv.slice(2);
const detailsFile = rest[0] === '--details' ? rest[1] : null;
const ts = require(tsPath);
const S = ts.SyntaxKind;

const changed = fs.readFileSync(changedFile, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
const files = [...new Set(changed.flatMap((c) => [c.row.from.file, c.row.to.file]))].map((f) => path.resolve(root, f));
const program = ts.createProgram(files, { allowJs: true, noLib: true, types: [], jsx: ts.JsxEmit.Preserve,
  target: ts.ScriptTarget.ESNext, module: ts.ModuleKind.ESNext, moduleResolution: ts.ModuleResolutionKind.Bundler, skipLibCheck: true, noEmit: true });
const checker = program.getTypeChecker();
const sfCache = new Map();
const sf = (rel) => { const p = path.resolve(root, rel); if (!sfCache.has(p)) sfCache.set(p, program.getSourceFile(p)); return sfCache.get(p); };
const lineOf = (s, pos) => s.getLineAndCharacterOfPosition(pos).line + 1;

// Uses and declaration-name occurrences of `text` on `line`. Binding names are not Uses (e.g. a
// function-type annotation's `(x: T) => U`); a row whose only occurrence is another binding's name
// (a nested formal re-declaring the name) is WRONG.
function identsOnLine(s, line, text) {
  const out = []; const decls = [];
  const visit = (n) => {
    if (n.end < s.getPositionOfLineAndCharacter(line - 1, 0)) return;
    if (ts.isIdentifier(n) && n.text === text && lineOf(s, n.getStart(s)) === line) {
      const isDeclName = n.parent && (ts.isParameter(n.parent) || ts.isVariableDeclaration(n.parent) || ts.isBindingElement(n.parent)) && n.parent.name === n;
      (isDeclName ? decls : out).push(n);
    }
    ts.forEachChild(n, visit);
  };
  visit(s);
  out.decls = decls;
  return out;
}
const isCallable = (n) => ts.isFunctionLike(n) && !!n.body;
function symbolOf(id) {
  if (ts.isShorthandPropertyAssignment(id.parent) && id.parent.name === id) return checker.getShorthandAssignmentValueSymbol(id.parent);
  return checker.getSymbolAtLocation(id);
}
function paramDecl(sym) {
  const d = sym && (sym.valueDeclaration || (sym.declarations || [])[0]);
  return d && ts.isParameter(d) && ts.isIdentifier(d.name) ? d : null;
}
function inLoopWith(node, a, b) {
  for (let p = node.parent; p; p = p.parent) {
    if ((ts.isForStatement(p) || ts.isForInStatement(p) || ts.isForOfStatement(p) || ts.isWhileStatement(p) || ts.isDoStatement(p)) && p.pos <= a && b <= p.end) return true;
    if (isCallable(p)) return false;
  }
  return false;
}
function writesBefore(decl, use) {
  const fn = decl.parent; const sym = checker.getSymbolAtLocation(decl.name);
  let bad = false;
  const visit = (n) => {
    if (bad) return;
    if (ts.isIdentifier(n) && n !== decl.name && checker.getSymbolAtLocation(n) === sym) {
      const p = n.parent;
      const write = (ts.isBinaryExpression(p) && p.left === n && p.operatorToken.kind >= S.FirstAssignment && p.operatorToken.kind <= S.LastAssignment) ||
        ((ts.isPrefixUnaryExpression(p) || ts.isPostfixUnaryExpression(p)) && (p.operator === S.PlusPlusToken || p.operator === S.MinusMinusToken)) ||
        (ts.isArrayLiteralExpression(p) || ts.isShorthandPropertyAssignment(p)) && ts.isBinaryExpression(p.parent) && p.parent.left === p;
      // `p = f(p)`: the right-hand side is evaluated before the write, so a Use inside the
      // assignment's own right operand is not preceded by that write.
      let assign = p;
      while (assign && !(ts.isBinaryExpression(assign) && assign.operatorToken.kind >= S.FirstAssignment && assign.operatorToken.kind <= S.LastAssignment)) assign = assign.parent;
      const selfRhs = assign && use.pos >= assign.right.pos && use.end <= assign.right.end;
      if (write && !selfRhs && (n.getStart() < use.getStart() || inLoopWith(n, n.pos, use.end) && inLoopWith(use, n.pos, use.end))) bad = true;
    }
    ts.forEachChild(n, visit);
  };
  if (fn.body) visit(fn.body);
  return bad;
}

function defUse(c) {
  const r = c.row; const s = sf(r.to.file);
  if (!s) return { step1: 'UNDECIDED', why: 'no_source' };
  const ids = identsOnLine(s, r.to.line, r.from.path.base);
  if (!ids.length) return ids.decls.length ? { step1: 'WRONG', why: 'only_other_binding_names' } : { step1: 'UNDECIDED', why: 'no_occurrence' };
  let any = null; const other = [];
  for (const id of ids) {
    const d = paramDecl(symbolOf(id));
    if (d && isCallable(d.parent) && lineOf(s, d.parent.getStart(s)) === r.from.line) { any = { id, d }; break; }
    other.push(d ? 'param_other_callable' : 'non_parameter');
  }
  if (!any) return { step1: 'WRONG', why: other.join(',') };
  const shape = any.d.dotDotDotToken ? 'rest' : (ts.isArrowFunction(any.d.parent) && !any.d.parent.getChildren(s).some((x) => x.kind === S.OpenParenToken) ? 'bare_arrow' : 'other_formal');
  const label = (c.head_label || c.base_label || {}).confidence;
  let step2 = 'n/a';
  // EXACT_PRIOR_WRITE: some write to the formal precedes the Use on some path (EVALUATION §3.5
  // step 2, strict). Prism's existing Exact means "reaches on an unflagged CFG route", so these
  // need the same-shape parity control before they count as a PR regression.
  if (label === 'exact') step2 = writesBefore(any.d, any.id) ? 'EXACT_PRIOR_WRITE' : 'EXACT_OK';
  return { step1: 'CORRECT', shape, step2 };
}

function calleeCallable(call) {
  const sig = checker.getResolvedSignature(call);
  let d = sig && sig.declaration;
  if (!d || !isCallable(d)) {
    let sym = checker.getSymbolAtLocation(call.expression);
    if (sym && sym.flags & ts.SymbolFlags.Alias) sym = checker.getAliasedSymbol(sym);
    const vd = sym && (sym.valueDeclaration || (sym.declarations || [])[0]);
    if (vd && isCallable(vd)) d = vd;
    else if (vd && (ts.isVariableDeclaration(vd) || ts.isPropertyAssignment(vd)) && vd.initializer && isCallable(vd.initializer)) d = vd.initializer;
    else if (vd && ts.isBinaryExpression(vd) && isCallable(vd.right)) d = vd.right;
    else d = null;
  }
  return d;
}

function useDef(c) {
  const r = c.row; const s = sf(r.from.file);
  if (!s) return { step1: 'UNDECIDED', why: 'no_source' };
  const calls = [];
  const visit = (n) => { if (ts.isCallExpression(n) && n.arguments.some((a) => ts.isIdentifier(a) && a.text === r.from.path.base && lineOf(s, a.getStart(s)) === r.from.line)) calls.push(n); ts.forEachChild(n, visit); };
  visit(s);
  if (!calls.length) return { step1: 'UNDECIDED', why: 'no_call' };
  let unresolved = 0;
  for (const call of calls) {
    const d = calleeCallable(call);
    if (!d) { unresolved++; continue; }
    const ds = d.getSourceFile();
    if (path.relative(root, ds.fileName) !== r.to.file || lineOf(ds, d.getStart(ds)) !== r.to.line) continue;
    for (let k = 0; k < call.arguments.length; k++) {
      const a = call.arguments[k];
      const p = d.parameters[k];
      if (ts.isIdentifier(a) && a.text === r.from.path.base && p && !p.dotDotDotToken && ts.isIdentifier(p.name) && p.name.text === r.to.path.base) return { step1: 'CORRECT' };
    }
    return { step1: 'WRONG', why: 'position_or_name' };
  }
  return unresolved ? { step1: 'UNDECIDED', why: 'callee_unresolved' } : { step1: 'WRONG', why: 'callee_elsewhere' };
}

const agg = {};
const inc = (k) => { agg[k] = (agg[k] || 0) + 1; };
const details = [];
for (const c of changed) {
  const kind = c.row.from.access + '->' + c.row.to.access;
  const v = kind === 'def->use' ? defUse(c) : kind === 'use->def' ? useDef(c) : { step1: 'UNDECIDED', why: 'kind' };
  const label = c.head_label ? c.head_label.confidence + ':' + c.head_label.doubt : '';
  inc([c.class, kind, v.step1].join('|'));
  if (v.shape) inc([c.class, 'shape', v.shape].join('|'));
  if (v.step2 && v.step2 !== 'n/a') inc([c.class, 'step2', v.step2].join('|'));
  if (v.why) inc([c.class, kind, v.step1, v.why].join('|'));
  details.push({ ...c, verdict: v, label });
}
fs.writeFileSync(outFile, JSON.stringify(agg, null, 1) + '\n');
if (detailsFile) fs.writeFileSync(detailsFile, details.map((d) => JSON.stringify(d)).join('\n') + '\n');
process.stdout.write(JSON.stringify(agg) + '\n');
