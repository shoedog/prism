// Values returned by an in-repo function: `const { k } = useThing()`, `const [a, b] = useThing()`,
// `const h = getHandler()`. The callee of the marked site is whatever the called function returns
// under that key or slot. `step` is 'returned'.
'use strict';
const { ext, ty, calleeSrc, otherSrc } = require('./variants-common.cjs');

function returnVariants() {
  const out = [];
  for (const lang of ['ts', 'js']) {
    const e = ext(lang, 'call');
    const ex = ext(lang, 'jsx');
    const n = ty(lang, ': number');
    const a = ty(lang, ': any');
    const b = ty(lang, ': boolean');
    const cf = `callee.${e}`;
    const hf = `hooks.${e}`;
    const one = (t) => ({ t: 'one', target: t });
    // body(scope) wraps the declaration and the site
    const wrap = (scope, imports, decl, site, isAsync) => (scope === 'fn'
      ? `${imports}\nexport ${isAsync ? 'async ' : ''}function caller(c${b}, opts${a}) {\n  ${decl}\n  ${site}\n}\n`
      : `${imports}\nexport function outer(c${b}, opts${a}) {\n  ${decl}\n  function caller() {\n    ${site}\n  }\n  return caller;\n}\n`);
    // add(shape, {hooks, extra files, imports, decl, callee name at site, truth, contract, scopes, form})
    const add = (shape, o) => {
      for (const scope of o.scopes || ['fn', 'outer']) {
        const jsx = o.form === 'jsx';
        const siteFile = `caller.${jsx ? ex : e}`;
        const site = jsx ? `return <${o.call} />; // SITE` : `return ${o.call}(1); // SITE`;
        const files = { [cf]: calleeSrc(lang), ...(o.hooks ? { [hf]: o.hooks } : {}), ...(o.files || {}) };
        files[siteFile] = wrap(scope, o.imports === undefined ? `import { useThing } from './hooks';` : o.imports, o.decl, site, o.async);
        out.push({
          id: `returned.${shape}.${scope}.${o.form || 'call'}.${lang}`, step: o.step || 'returned', shape, scope, form: o.form || 'call',
          naming: o.naming || 'same', lang, files, siteFile, truth: o.truth, contract: o.contract || null,
        });
      }
    };
    const IMP = `import { f as imp, g as imp2 } from './callee';`;
    const doIt = `function doIt(x${n}) { return x; }`;

    add('object-shorthand-local-fn', { hooks: `export function useThing() {\n  ${doIt}\n  return { doIt };\n}\n`, decl: `const { doIt } = useThing();`, call: 'doIt', truth: one(`${hf}:doIt`) });
    add('object-renamed-key-decoy', {
      hooks: `export function useThing() {\n  ${doIt}\n  return { decoy: doIt };\n}\n`, files: { [`other.${e}`]: otherSrc(lang) },
      decl: `const { decoy } = useThing();`, call: 'decoy', truth: one(`${hf}:doIt`), naming: 'trap',
    });
    add('object-const-arrow', { hooks: `export function useThing() {\n  const doIt = (x${n}) => x;\n  return { doIt };\n}\n`, decl: `const { doIt } = useThing();`, call: 'doIt', truth: one(`${hf}:doIt`) });
    add('object-usecallback', {
      hooks: `import { useCallback } from 'react';\nexport function useThing() {\n  const doIt = useCallback((x${n}) => x, []);\n  return { doIt };\n}\n`,
      decl: `const { doIt } = useThing();`, call: 'doIt', truth: one(`${hf}:doIt`), contract: 'the returned value is a useCallback of an inline function',
    });
    add('object-import-value', { hooks: `${IMP}\nexport function useThing() {\n  return { first: imp2, second: imp };\n}\n`, decl: `const { second } = useThing();`, call: 'second', truth: one(`${cf}:f`), naming: 'fresh' });
    add('object-inline-arrow-prop', { hooks: `export function useThing() {\n  return {\n    doIt: (x${n}) => x, // TARGET:doIt\n  };\n}\n`, decl: `const { doIt } = useThing();`, call: 'doIt', truth: one(`${hf}:doIt`) });
    add('object-method', { hooks: `export function useThing() {\n  return {\n    doIt(x${n}) { return x; }, // TARGET:doIt\n  };\n}\n`, decl: `const { doIt } = useThing();`, call: 'doIt', truth: one(`${hf}:doIt`) });
    add('object-renamed-binding', { hooks: `export function useThing() {\n  ${doIt}\n  return { doIt };\n}\n`, decl: `const { doIt: run } = useThing();`, call: 'run', truth: one(`${hf}:doIt`), naming: 'fresh' });
    add('array-slot', { hooks: `${IMP}\nexport function useThing() {\n  ${doIt}\n  return [imp2, doIt]${ty(lang, ' as const')};\n}\n`, decl: `const [first, run] = useThing();\n  void first;`, call: 'run', truth: one(`${hf}:doIt`), naming: 'fresh' });
    add('value-is-function', { hooks: `export function useThing() {\n  function handler(x${n}) { return x; }\n  return handler;\n}\n`, decl: `const h = useThing();`, call: 'h', truth: one(`${hf}:handler`), naming: 'fresh' });
    add('ident-object', { hooks: `export function useThing() {\n  ${doIt}\n  const api = { doIt };\n  return api;\n}\n`, decl: `const { doIt } = useThing();`, call: 'doIt', truth: one(`${hf}:doIt`), contract: 'the function returns a const identifier bound to an object literal' });
    add('usememo-object', {
      hooks: `import { useMemo } from 'react';\nexport function useThing() {\n  ${doIt}\n  return useMemo(() => ({ doIt }), []);\n}\n`,
      decl: `const { doIt } = useThing();`, call: 'doIt', truth: one(`${hf}:doIt`), contract: 'the function returns useMemo(() => ({…}))',
    });
    add('arrow-expression-body', { hooks: `${doIt}\nexport const useThing = () => ({ doIt });\n`, decl: `const { doIt } = useThing();`, call: 'doIt', truth: one(`${hf}:doIt`) });
    add('default-export-hook', { hooks: `export default function useThing() {\n  ${doIt}\n  return { doIt };\n}\n`, imports: `import useThing from './hooks';`, decl: `const { doIt } = useThing();`, call: 'doIt', truth: one(`${hf}:doIt`) });
    add('same-file-hook', { imports: `function useThing() {\n  ${doIt}\n  return { doIt };\n}`, decl: `const { doIt } = useThing();`, call: 'doIt', truth: one(`caller.${e}:doIt`) });
    add('jsx-component', {
      hooks: `export function useThing() {\n  function Panel(props${a}) { return null; }\n  return { Panel };\n}\n`,
      decl: `const { Panel } = useThing();`, call: 'Panel', form: 'jsx', truth: one(`${hf}:Panel`),
    });
    add('spread-before-key', { hooks: `export function useThing(base${a}) {\n  ${doIt}\n  return { ...base, doIt };\n}\n`, decl: `const { doIt } = useThing(opts);`, call: 'doIt', truth: one(`${hf}:doIt`), contract: 'a spread precedes the key: the later key wins' });
    add('spread-after-key', { hooks: `export function useThing(rest${a}) {\n  ${doIt}\n  return { doIt, ...rest };\n}\n`, decl: `const { doIt } = useThing(opts);`, call: 'doIt', truth: { t: 'may', targets: [`${hf}:doIt`] }, contract: 'a spread follows the key and may replace it' });
    add('divergent-returns', {
      hooks: `${IMP}\nexport function useThing(c${b}) {\n  if (c) { return { run: imp }; }\n  return { run: imp2 };\n}\n`,
      decl: `const { run } = useThing(c);`, call: 'run', truth: { t: 'set', targets: [`${cf}:f`, `${cf}:g`] }, naming: 'fresh',
    });
    add('early-null-return', {
      hooks: `export function useThing(c${b})${ty(lang, ': any')} {\n  ${doIt}\n  if (!c) { return null; }\n  return { doIt };\n}\n`,
      decl: `const { doIt } = useThing(c);`, call: 'doIt', truth: one(`${hf}:doIt`), contract: 'one return is null; the other returns the object',
    });
    add('param-value', { hooks: `export function useThing(cb${a}) {\n  return { run: cb };\n}\n`, imports: `import { useThing } from './hooks';\n${IMP}`, decl: `const { run } = useThing(imp);`, call: 'run', truth: one(`${cf}:f`), naming: 'fresh', contract: 'the returned value is the called function\'s parameter (needs argument following)' });
    add('nested-hook', {
      hooks: `function useInner() {\n  ${doIt}\n  return { doIt };\n}\nexport function useThing() {\n  const { doIt } = useInner();\n  return { doIt };\n}\n`,
      decl: `const { doIt } = useThing();`, call: 'doIt', truth: one(`${hf}:doIt`), contract: 'two returned-value steps',
    });
    add('await-async', { hooks: `export async function useThing() {\n  ${doIt}\n  return { doIt };\n}\n`, decl: `const { doIt } = await useThing();`, call: 'doIt', truth: one(`${hf}:doIt`), async: true, scopes: ['fn'], contract: 'awaited result of an async function' });
    add('external-hook-trap', {
      imports: `import { useThing } from 'ext-hooks';`, files: { [`other.${e}`]: otherSrc(lang) },
      decl: `const { decoy } = useThing();`, call: 'decoy', truth: { t: 'none' }, naming: 'trap',
    });
    add('hook-is-parameter', { imports: '', files: { [`other.${e}`]: otherSrc(lang) }, decl: `const { decoy } = opts.useThing();`, call: 'decoy', truth: { t: 'may', targets: [] }, naming: 'trap', contract: 'the called function is a member of a parameter' });
    add('let-binding', { hooks: `export function useThing() {\n  ${doIt}\n  return { doIt };\n}\n`, decl: `let { doIt } = useThing();`, call: 'doIt', truth: one(`${hf}:doIt`), contract: 'a let binding (not followed by contract)' });
    add('reassigned-binding', {
      hooks: `${IMP}\nexport function useThing() {\n  return { run: imp };\n}\n`, imports: `import { useThing } from './hooks';\nimport * as N from './callee';`,
      decl: `let { run } = useThing();\n  if (c) { run = N.g; }`, call: 'run', truth: { t: 'set', targets: [`${cf}:f`, `${cf}:g`] }, naming: 'fresh',
    });
    // member call on the call result: a qualified callee, outside the bare-name lookup
    out.push({
      id: `returned.member-call-on-result.fn.call.${lang}`, step: 'none', shape: 'member-call-on-result', scope: 'fn', form: 'call', naming: 'same', lang, siteFile: `caller.${e}`,
      files: { [cf]: calleeSrc(lang), [hf]: `export function useThing() {\n  ${doIt}\n  return { doIt };\n}\n`, [`caller.${e}`]: `import { useThing } from './hooks';\nexport function caller() {\n  const api = useThing();\n  return api.doIt(1); // SITE\n}\n` },
      truth: one(`${hf}:doIt`), contract: 'member call on a call result: outside the bare-name lookup',
    });
  }
  return out;
}

module.exports = { returnVariants };
