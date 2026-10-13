// Alias-following variant matrix: enumerated fixtures with ground truth.
// Each variant is a tiny repo. The marked site (line ending in "// SITE") is inside a named
// callable so prism has a caller for it. Ground truth is what the site statically calls:
//   {t:'one', target:'file:name'}      exactly one in-repo callable
//   {t:'set', targets:[...]}           a closed set, all in-repo
//   {t:'may', targets:[...]}           possibly these, not provable from visible code
//   {t:'none'}                         no in-repo callable is the callee
//   {t:'anon', file}                   the anonymous callable on the line marked "// ANON"
// `step` says which lookup step of the owner's order would have to resolve it:
//   local | param-lookup | param-args | import | same-file | global | none
// `contract` marks variants whose desired grade is a contract decision, not a fact.
'use strict';

const FORMS = {
  call: { T: 'f', ALT: 'g', trap: 'decoy', fresh: 'h', site: (L) => `return ${L}(1); // SITE` },
  jsx: { T: 'F', ALT: 'G', trap: 'Decoy', fresh: 'H', site: (L) => `return <${L} />; // SITE` },
  opt: { T: 'f', ALT: 'g', trap: 'decoy', fresh: 'h', site: (L) => `return ${L}?.(1); // SITE` },
  new: { T: 'K', ALT: 'K2', trap: 'DecoyK', fresh: 'HK', site: (L) => `return new ${L}(1); // SITE` },
};

function ext(lang, form) {
  if (lang === 'ts') return form === 'jsx' ? 'tsx' : 'ts';
  return form === 'jsx' ? 'jsx' : 'js';
}
const ty = (lang, s) => (lang === 'ts' ? s : '');

function calleeSrc(lang) {
  const n = ty(lang, ': number');
  const a = ty(lang, ': any');
  return [
    `export function f(x${n}) { return x; }`,
    `export function g(x${n}) { return x + 1; }`,
    `export function F(props${a}) { return null; }`,
    `export function G(props${a}) { return null; }`,
    `export class K { constructor(x${n}) { this.v = x; } ${ty(lang, 'v: number;')} }`,
    `export class K2 { constructor(x${n}) { this.v = x + 1; } ${ty(lang, 'v: number;')} }`,
    `export const renamed = f;`,
    '',
  ].join('\n');
}
function otherSrc(lang) {
  const n = ty(lang, ': number');
  const a = ty(lang, ': any');
  return [
    `export function decoy(x${n}) { return -x; }`,
    `export function Decoy(props${a}) { return null; }`,
    `export class DecoyK { constructor(x${n}) { this.v = -x; } ${ty(lang, 'v: number;')} }`,
    '',
  ].join('\n');
}

// ---- local alias shapes -------------------------------------------------------------------
// Each returns {imports, pre (module level), decl (the alias declaration), truth, contract?}
const NS = `import * as N from './callee';`;
const LOCAL_SHAPES = {
  'ident': (c) => ({ imports: `import { ${c.T} as imp } from './callee';`, decl: `const ${c.L} = imp;`, truth: 'T' }),
  'ns-member': (c) => ({ imports: NS, decl: `const ${c.L} = N.${c.T};`, truth: 'T' }),
  'ns-destructure': (c) => ({
    imports: NS,
    decl: c.L === c.T ? `const { ${c.T} } = N;` : `const { ${c.T}: ${c.L} } = N;`,
    truth: 'T',
  }),
  'obj-member': (c) => ({
    imports: `import { ${c.T} as imp } from './callee';`,
    decl: `const o = { ${c.T}: imp }; const ${c.L} = o.${c.T};`,
    truth: 'T',
  }),
  'chain2': (c) => ({ imports: NS, decl: `const a0 = N.${c.T}; const ${c.L} = a0;`, truth: 'T' }),
  'let-stable': (c) => ({ imports: NS, decl: `let ${c.L} = N.${c.T};`, truth: 'T' }),
  'let-reassigned': (c) => ({
    imports: NS,
    decl: `let ${c.L}${ty(c.lang, ': any')} = N.${c.T}; if (c) { ${c.L} = N.${c.ALT}; }`,
    truth: 'SET',
    scopes: ['fn', 'outer'],
  }),
  'var': (c) => ({ imports: NS, decl: `var ${c.L} = N.${c.T};`, truth: 'T' }),
  'conditional': (c) => ({ imports: NS, decl: `const ${c.L} = c ? N.${c.T} : N.${c.ALT};`, truth: 'SET', scopes: ['fn', 'outer'] }),
  'wrapper-call': (c) => ({
    imports: `import { ${c.T} as imp } from './callee';`,
    pre: `function wrap(fn${ty(c.lang, ': any')})${ty(c.lang, ': any')} { return fn; }`,
    decl: `const ${c.L} = wrap(imp);`,
    truth: 'T',
    contract: 'wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee?',
  }),
  'bind': (c) => ({
    imports: NS,
    decl: `const ${c.L} = N.${c.T}.bind(null);`,
    truth: 'T',
    forms: ['call'],
    contract: '.bind result',
  }),
  'array-destructure': (c) => ({ imports: `import { ${c.T} as imp } from './callee';`, decl: `const [${c.L}] = [imp];`, truth: 'T' }),
  'default-destructure': (c) => ({
    imports: `import { ${c.T} as imp } from './callee';`,
    decl: `const { cb: ${c.L} = imp } = opts;`,
    truth: 'MAY',
    scopes: ['fn', 'outer'],
  }),
  'fallback-or': (c) => ({
    imports: `import { ${c.T} as imp } from './callee';`,
    decl: `const ${c.L} = opts.cb || imp;`,
    truth: 'MAY',
    scopes: ['fn', 'outer'],
  }),
  'ns-member-paren': (c) => ({ imports: NS, decl: `const ${c.L} = (N.${c.T});`, truth: 'T' }),
  'ns-member-as': (c) => ({ imports: NS, decl: `const ${c.L} = N.${c.T} as typeof N.${c.T};`, truth: 'T', langs: ['ts'] }),
  'ns-member-nonnull': (c) => ({ imports: NS, decl: `const ${c.L} = N.${c.T}!;`, truth: 'T', langs: ['ts'] }),
  'block-shadow': (c) => ({
    imports: NS,
    decl: `const ${c.L} = N.${c.ALT};\n  if (c) {\n    const ${c.L} = N.${c.T};\n    ${FORMS[c.form].site(c.L)}\n  }`,
    noSite: true,
    truth: 'T',
    scopes: ['fn'],
    forms: ['call'],
  }),
  'react-memo': (c) => ({
    imports: `import { memo } from 'react';\nimport { ${c.T} as imp } from './callee';`,
    decl: `const ${c.L} = memo(imp);`,
    truth: 'T',
    forms: ['call', 'jsx'],
    scopes: ['module', 'fn'],
    contract: 'library wrapper (memo, forwardRef, observer): is the wrapped callable the callee?',
  }),
  'react-usecallback-ident': (c) => ({
    imports: `import { useCallback } from 'react';\nimport { ${c.T} as imp } from './callee';`,
    decl: `const ${c.L} = useCallback(imp, []);`,
    truth: 'T',
    forms: ['call'],
    scopes: ['fn'],
    contract: 'library wrapper (useCallback, useMemo): is the wrapped callable the callee?',
  }),
  'react-usecallback-inline': (c) => ({
    imports: `import { useCallback } from 'react';`,
    decl: `const ${c.L} = useCallback((x${ty(c.lang, ': any')}) => x, []); // ANON`,
    truth: 'ANON',
    forms: ['call'],
    scopes: ['fn'],
    contract: 'library wrapper around an inline callable',
  }),
  'react-usestate-setter': (c) => ({
    imports: `import { useState } from 'react';`,
    decl: `const [value, ${c.L}] = useState(0);`,
    truth: 'NONE',
    forms: ['call'],
    namings: ['trap', 'fresh'],
    scopes: ['fn', 'outer'],
  }),
  'react-usecontext-destructure': (c) => ({
    imports: `import { createContext, useContext } from 'react';\nimport { ${c.T} as imp } from './callee';`,
    pre: `const Ctx = createContext({ ${c.T}: imp });`,
    decl: c.L === c.T ? `const { ${c.T} } = useContext(Ctx);` : `const { ${c.T}: ${c.L} } = useContext(Ctx);`,
    truth: 'MAY',
    forms: ['call'],
    scopes: ['fn', 'outer'],
    contract: 'context value: the default is visible, providers may not be',
  }),
  'custom-hook-array-result': (c) => ({
    imports: `import { ${c.T} as imp } from './callee';`,
    pre: `function useThing() { return [1, imp]${ty(c.lang, ' as const')}; }`,
    decl: `const [value, ${c.L}] = useThing();`,
    truth: 'T',
    forms: ['call'],
    scopes: ['fn', 'outer'],
    contract: 'value returned by a same-file function (hook result)',
  }),
  'styled-template': (c) => ({
    imports: `import styled from 'styled-components';`,
    decl: `const ${c.L} = styled.div\`color: red;\`;`,
    truth: 'NONE',
    forms: ['jsx'],
    namings: ['trap', 'fresh'],
    scopes: ['module', 'fn'],
  }),
  'react-lazy': (c) => ({
    imports: `import { lazy } from 'react';`,
    decl: `const ${c.L} = lazy(() => import('./dflt'));`,
    truth: 'DFLT',
    forms: ['jsx'],
    scopes: ['module'],
    contract: 'lazy(() => import(...)): is the default export the callee?',
  }),
  'element-member': (c) => ({
    imports: `import { ${c.T} as imp } from './callee';`,
    decl: `const table = [imp]; const ${c.L} = table[0];`,
    truth: 'T',
    forms: ['call'],
    contract: 'element access into a local array literal',
  }),
  'deep-member': (c) => ({
    imports: `import { ${c.T} as imp } from './callee';`,
    decl: `const o = { inner: { ${c.T}: imp } }; const ${c.L} = o.inner.${c.T};`,
    truth: 'T',
    forms: ['call'],
  }),
  'declared-then-assigned': (c) => ({
    imports: NS,
    decl: `let ${c.L}${ty(c.lang, ': any')}; ${c.L} = N.${c.T};`,
    truth: 'T',
    forms: ['call'],
    scopes: ['fn', 'outer'],
    contract: 'declared without a value, assigned once',
  }),
  'for-of-element': (c) => ({
    imports: `import { ${c.T} as imp } from './callee';`,
    decl: `for (const ${c.L} of [imp]) {\n    ${FORMS[c.form].site(c.L)}\n  }`,
    noSite: true,
    truth: 'T',
    forms: ['call'],
    scopes: ['fn'],
    contract: 'loop variable over a local array literal',
  }),
  'object-param-destructure': (c) => ({
    imports: '',
    decl: `const { ${c.L} } = opts;`,
    truth: 'UNKNOWN',
    forms: ['call'],
    namings: ['trap', 'fresh'],
    scopes: ['fn', 'outer'],
  }),
  'local-arrow': (c) => ({
    imports: '',
    decl: c.form === 'new' ? `class ${c.L} { constructor(x${ty(c.lang, ': number')}) {} }` : `const ${c.L} = (x${ty(c.lang, ': any')}) => null;`,
    truth: 'LOCAL',
  }),
  'ts-import-equals-member': (c) => ({
    imports: NS,
    decl: `import ${c.L} = N.${c.T};`,
    truth: 'T',
    langs: ['ts'],
    scopes: ['module'],
  }),
  'await-import-destructure': (c) => ({
    imports: '',
    decl: c.L === c.T ? `const { ${c.T} } = await import('./callee');` : `const { ${c.T}: ${c.L} } = await import('./callee');`,
    truth: 'T',
    scopes: ['fn'],
    async: true,
  }),
  'hook-result-destructure': (c) => ({
    imports: `import { ${c.T} as imp } from './callee';`,
    pre: `function useThing() { return { ${c.T}: imp }; }`,
    decl: c.L === c.T ? `const { ${c.T} } = useThing();` : `const { ${c.T}: ${c.L} } = useThing();`,
    truth: 'T',
    scopes: ['fn', 'outer'],
    contract: 'value returned by a same-file function (hook result)',
  }),
  'await-import-member': (c) => ({
    imports: '',
    decl: `const ${c.L} = (await import('./callee')).${c.T};`,
    truth: 'T',
    forms: ['call'],
    scopes: ['fn'],
    async: true,
  }),
  'await-import-default': (c) => ({
    imports: '',
    decl: `const ${c.L} = (await import('./dflt')).default;`,
    truth: 'DFLT',
    forms: ['call'],
    scopes: ['fn'],
    async: true,
    dflt: true,
  }),
  'ns-member-satisfies': (c) => ({ imports: NS, decl: `const ${c.L} = N.${c.T} satisfies typeof N.${c.T};`, truth: 'T', langs: ['ts'], forms: ['call'] }),
  'exported-ns-member': (c) => ({ imports: NS, decl: `export const ${c.L} = N.${c.T};`, truth: 'T', forms: ['call'], scopes: ['module'] }),
  'ident-of-reassigned-let': (c) => ({
    imports: NS,
    decl: `let base${ty(c.lang, ': any')} = N.${c.T}; if (c) { base = N.${c.ALT}; }\n  const ${c.L} = base;`,
    truth: 'SET',
    forms: ['call'],
    scopes: ['fn', 'outer'],
  }),
  'chain8': (c) => ({
    imports: `import { ${c.T} as imp } from './callee';`,
    decl: `const a1 = imp; const a2 = a1; const a3 = a2; const a4 = a3; const a5 = a4; const a6 = a5; const a7 = a6; const ${c.L} = a7;`,
    truth: 'T',
    forms: ['call'],
    scopes: ['fn'],
  }),
  'chain9': (c) => ({
    imports: `import { ${c.T} as imp } from './callee';`,
    decl: `const a1 = imp; const a2 = a1; const a3 = a2; const a4 = a3; const a5 = a4; const a6 = a5; const a7 = a6; const a8 = a7; const ${c.L} = a8;`,
    truth: 'T',
    forms: ['call'],
    scopes: ['fn'],
    contract: 'nine alias hops: one past the proposed eight-hop bound',
  }),
  'alias-of-param': (c) => ({
    imports: '',
    decl: `const ${c.L} = opts;`,
    truth: 'UNKNOWN',
    forms: ['call'],
    namings: ['trap', 'fresh'],
    scopes: ['fn', 'outer'],
  }),
  'catch-binding': (c) => ({
    imports: NS,
    decl: `const ${c.L} = N.${c.ALT};\n  try { opts(); } catch (${c.L}${ty(c.lang, ': any')}) {\n    ${FORMS[c.form].site(c.L)}\n  }`,
    noSite: true,
    truth: 'UNKNOWN',
    forms: ['call'],
    namings: ['trap', 'fresh'],
    scopes: ['fn'],
  }),
  'ns-member-exported-alias': (c) => ({
    imports: NS,
    decl: `const ${c.L} = N.renamed;`,
    truth: 'T',
    forms: ['call'],
    namings: ['trap', 'fresh'],
  }),
};
const ALL_SCOPES = ['fn', 'module', 'outer'];
const SCOPE_RICH = new Set(['ident', 'ns-member', 'ns-destructure']);
const FORM_RICH = new Set(['ident', 'ns-member', 'ns-destructure', 'conditional', 'wrapper-call', 'local-arrow']);
const OPT_FORM = new Set(['ident', 'ns-member']);

function truthOf(kind, c) {
  const cf = `callee.${ext(c.lang, 'call')}`;
  if (kind === 'T') return { t: 'one', target: `${cf}:${c.T}` };
  if (kind === 'SET') return { t: 'set', targets: [`${cf}:${c.T}`, `${cf}:${c.ALT}`] };
  if (kind === 'MAY') return { t: 'may', targets: [`${cf}:${c.T}`] };
  if (kind === 'LOCAL') return { t: 'one', target: `${c.callerFile}:${c.L}` };
  if (kind === 'ALT') return { t: 'one', target: `${cf}:${c.ALT}` };
  if (kind === 'ANON') return { t: 'anon', file: c.callerFile };
  if (kind === 'DFLT') return { t: 'one', target: `dflt.${ext(c.lang, 'call')}:${c.T}` };
  if (kind === 'UNKNOWN') return { t: 'may', targets: [] };
  if (kind === 'NONE') return { t: 'none' };
  throw new Error(kind);
}

function baseFiles(lang, naming) {
  const e = ext(lang, 'call');
  const files = { [`callee.${e}`]: calleeSrc(lang) };
  if (naming === 'trap') files[`other.${e}`] = otherSrc(lang);
  return files;
}

function localVariants() {
  const out = [];
  for (const [shape, fn] of Object.entries(LOCAL_SHAPES)) {
    for (const lang of ['ts', 'js']) {
      const forms = FORM_RICH.has(shape) ? ['call', 'jsx', 'new'] : ['call', 'jsx'];
      if (OPT_FORM.has(shape)) forms.push('opt');
      for (const form of forms) {
        for (const naming of ['same', 'trap', 'fresh']) {
          const F = FORMS[form];
          const L = naming === 'same' ? F.T : naming === 'trap' ? F.trap : F.fresh;
          if (form === 'jsx' && !FORM_RICH.has(shape) && !(fn({ lang, form, L: 'X', T: 'X', ALT: 'Y' }).forms || []).includes('jsx')) continue;
          const callerFile = `caller.${ext(lang, form)}`;
          const c = { lang, form, naming, L, T: F.T, ALT: F.ALT, callerFile };
          const s = fn(c);
          if (s.langs && !s.langs.includes(lang)) continue;
          if (s.forms && !s.forms.includes(form)) continue;
          if (s.namings && !s.namings.includes(naming)) continue;
          if (shape === 'local-arrow' && naming === 'fresh') continue; // nothing to confuse it with
          let scopes = s.scopes || ALL_SCOPES;
          if (form === 'new' || form === 'opt') scopes = scopes.filter((x) => x === 'fn');
          if (form === 'jsx' && !s.scopes) scopes = scopes.filter((x) => x !== 'module');
          if (SCOPE_RICH.has(shape) && form === 'call') scopes = [...scopes, 'method', 'arrow'];
          for (const scope of scopes) {
            const params = `c${ty(lang, ': boolean')}, opts${ty(lang, ': any')}`;
            const asyncKw = s.async ? 'async ' : '';
            const siteLine = s.noSite ? '' : F.site(L);
            let body;
            if (scope === 'fn') {
              body = `export ${asyncKw}function caller(${params}) {\n  ${s.decl}\n  ${siteLine}\n}\n`;
            } else if (scope === 'method') {
              body = `export class Box {\n  caller(${params}) {\n    ${s.decl}\n    ${siteLine}\n  }\n}\n`;
            } else if (scope === 'arrow') {
              body = `export const caller = (${params}) => {\n  ${s.decl}\n  ${siteLine}\n};\n`;
            } else if (scope === 'module') {
              body = `${s.decl}\nexport function caller(${params}) {\n  ${siteLine}\n}\n`;
            } else {
              body = `export function outer(${params}) {\n  ${s.decl}\n  function caller() {\n    ${siteLine}\n  }\n  return caller;\n}\n`;
            }
            const src = [s.imports, s.pre || '', body].filter(Boolean).join('\n');
            const files = baseFiles(lang, naming);
            files[callerFile] = src;
            if (shape === 'react-lazy' || s.dflt) files[`dflt.${ext(lang, 'call')}`] = `export default function ${F.T}(props${ty(lang, ': any')}) { return null; }\n`;
            out.push({
              id: `local.${shape}.${scope}.${form}.${naming}.${lang}`,
              step: shape === 'local-arrow' ? 'same-file' : 'local',
              shape, scope, form, naming, lang, files, siteFile: callerFile,
              truth: truthOf(s.truth, c), contract: s.contract || null,
            });
          }
        }
      }
    }
  }
  return out;
}

// ---- CommonJS local shapes ------------------------------------------------------------------
function cjsVariants() {
  const out = [];
  const callee = `function f(x) { return x; }\nfunction g(x) { return x + 1; }\nmodule.exports = { f, g };\n`;
  const other = `function decoy(x) { return -x; }\nmodule.exports = { decoy };\n`;
  const shapes = {
    'require-member': (L) => `const m = require('./callee');\n  const ${L} = m.f;`,
    'require-destructure': (L) => (L === 'f' ? `const { f } = require('./callee');` : `const { f: ${L} } = require('./callee');`),
    'require-inline-member': (L) => `const ${L} = require('./callee').f;`,
    'require-assign': (L) => `let ${L};\n  ${L} = require('./callee').f;`,
    'require-destructure-parens': (L) => (L === 'f' ? `const { f } = (require('./callee'));` : `const { f: ${L} } = (require('./callee'));`),
    'require-assign-destructure': (L) => `let ${L};\n  ({ f: ${L} } = require('./callee'));`,
  };
  for (const [shape, decl] of Object.entries(shapes)) {
    for (const naming of ['same', 'trap', 'fresh']) {
      for (const scope of ['fn', 'module']) {
        const L = naming === 'same' ? 'f' : naming === 'trap' ? 'decoy' : 'h';
        const d = decl(L);
        const src = scope === 'fn'
          ? `function caller() {\n  ${d}\n  return ${L}(1); // SITE\n}\nmodule.exports = { caller };\n`
          : `${d.replace(/\n  /g, '\n')}\nfunction caller() {\n  return ${L}(1); // SITE\n}\nmodule.exports = { caller };\n`;
        const files = { 'callee.js': callee, 'caller.js': src };
        if (naming === 'trap') files['other.js'] = other;
        out.push({
          id: `local.${shape}.${scope}.call.${naming}.cjs`,
          step: 'local', shape, scope, form: 'call', naming, lang: 'cjs', files, siteFile: 'caller.js',
          truth: { t: 'one', target: 'callee.js:f' }, contract: null,
        });
      }
    }
  }
  return out;
}

// ---- parameter shapes -----------------------------------------------------------------------
function paramVariants() {
  const out = [];
  for (const lang of ['ts', 'js']) {
    const a = ty(lang, ': any');
    for (const naming of ['same', 'trap', 'fresh']) {
      const add = (shape, form, mk) => {
        const F = FORMS[form];
        const L = naming === 'same' ? F.T : naming === 'trap' ? F.trap : F.fresh;
        const useJsx = form === 'jsx' || shape.includes('props');
        const e = useJsx ? ext(lang, 'jsx') : ext(lang, 'call');
        const siteFile = `caller.${e}`;
        const cf = `callee.${ext(lang, 'call')}`;
        const c = { lang, form, naming, L, T: F.T, ALT: F.ALT, a, site: F.site(L), cf, siteFile };
        const v = mk(c);
        if (!v) return;
        const files = baseFiles(lang, naming);
        files[siteFile] = v.src;
        out.push({
          id: `param.${shape}.${form}.${naming}.${lang}`,
          step: v.step || 'param-args', shape, scope: 'param', form, naming, lang, files, siteFile,
          truth: v.truth, contract: v.contract || null,
        });
      };
      const imp = (c) => `import { ${c.T} as imp, ${c.ALT} as imp2 } from './callee';\nimport * as N from './callee';`;
      for (const form of ['call', 'jsx']) {
        add('plain-one-caller', form, (c) => ({
          src: `${imp(c)}\nfunction outer(${c.L}${a}) {\n  ${c.site}\n}\nexport function driver() { return outer(imp); }\n`,
          truth: { t: 'one', target: `${c.cf}:${c.T}` },
        }));
        add('plain-exported', form, (c) => ({
          src: `${imp(c)}\nexport function outer(${c.L}${a}) {\n  ${c.site}\n}\nexport function driver() { return outer(imp); }\n`,
          truth: { t: 'may', targets: [`${c.cf}:${c.T}`] },
          contract: 'exported callable: callers may be invisible',
        }));
        add('plain-two-callers', form, (c) => ({
          src: `${imp(c)}\nfunction outer(${c.L}${a}) {\n  ${c.site}\n}\nexport function driver() { return outer(imp); }\nexport function driver2() { return outer(imp2); }\n`,
          truth: { t: 'set', targets: [`${c.cf}:${c.T}`, `${c.cf}:${c.ALT}`] },
        }));
        add('plain-member-arg', form, (c) => ({
          src: `${imp(c)}\nfunction outer(${c.L}${a}) {\n  ${c.site}\n}\nexport function driver() { return outer(N.${c.T}); }\n`,
          truth: { t: 'one', target: `${c.cf}:${c.T}` },
        }));
        add('forwarded-twice', form, (c) => ({
          src: `${imp(c)}\nfunction inner(${c.L}${a}) {\n  ${c.site}\n}\nfunction outer(cb${a}) { return inner(cb); }\nexport function driver() { return outer(imp); }\n`,
          truth: { t: 'one', target: `${c.cf}:${c.T}` },
        }));
        add('default-value', form, (c) => ({
          src: `${imp(c)}\nfunction outer(${c.L}${a} = imp) {\n  ${c.site}\n}\nexport function driver() { return outer(); }\n`,
          truth: { t: 'one', target: `${c.cf}:${c.T}` },
          contract: 'parameter default used at the only visible call site',
        }));
        add('destructured-props', form, (c) => ({
          src: `${imp(c)}\nfunction C({ ${c.L} }${a}) {\n  ${c.site}\n}\nexport function App() { return <C ${c.L}={imp} />; }\n`,
          truth: { t: 'one', target: `${c.cf}:${c.T}` },
        }));
        add('destructured-props-renamed', form, (c) => ({
          src: `${imp(c)}\nfunction C({ cb: ${c.L} }${a}) {\n  ${c.site}\n}\nexport function App() { return <C cb={imp} />; }\n`,
          truth: { t: 'one', target: `${c.cf}:${c.T}` },
        }));
        add('destructured-props-default-unused', form, (c) => ({
          src: `${imp(c)}\nfunction C({ ${c.L} = imp }${a}) {\n  ${c.site}\n}\nexport function App() { return <C />; }\n`,
          truth: { t: 'one', target: `${c.cf}:${c.T}` },
          contract: 'destructured parameter default used at the only visible call site',
        }));
        add('destructured-props-default-overridden', form, (c) => ({
          src: `${imp(c)}\nfunction C({ ${c.L} = imp }${a}) {\n  ${c.site}\n}\nexport function App() { return <C ${c.L}={imp2} />; }\n`,
          truth: { t: 'one', target: `${c.cf}:${c.ALT}` },
        }));
        add('closure-param', form, (c) => ({
          src: `${imp(c)}\nfunction outer(${c.L}${a}) {\n  function caller() {\n    ${c.site}\n  }\n  return caller;\n}\nexport function driver() { return outer(imp); }\n`,
          truth: { t: 'one', target: `${c.cf}:${c.T}` },
        }));
        add('props-destructure-in-body', form, (c) => ({
          src: `${imp(c)}\nfunction C(props${a}) {\n  const { ${c.L} } = props;\n  ${c.site}\n}\nexport function App() { return <C ${c.L}={imp} />; }\n`,
          truth: { t: 'one', target: `${c.cf}:${c.T}` },
        }));
        add('props-object-member', form, (c) => (c.form === 'jsx' ? null : {
          src: `${imp(c)}\nfunction C(props${a}) {\n  const ${c.L} = props.cb;\n  ${c.site}\n}\nexport function App() { return <C cb={imp} />; }\n`,
          truth: { t: 'one', target: `${c.cf}:${c.T}` },
        }));
      }
      add('inline-arrow-arg', 'call', (c) => ({
        src: `function outer(${c.L}${a}) {\n  ${c.site}\n}\nexport function driver() {\n  return outer((x${a}) => x); // ANON\n}\n`,
        truth: { t: 'anon', file: c.siteFile },
      }));
    }
    // shadowing: one naming only
    const e = ext(lang, 'call');
    const cf = `callee.${e}`;
    out.push({
      id: `param.shadows-import.call.same.${lang}`, step: 'param-lookup', shape: 'shadows-import', scope: 'param', form: 'call',
      naming: 'same', lang, siteFile: `caller.${e}`,
      files: { [cf]: calleeSrc(lang), [`caller.${e}`]: `import { f } from './callee';\nexport const keep = f;\nexport function caller(f${a}) {\n  return f(1); // SITE\n}\n` },
      truth: { t: 'none' }, contract: null,
    });
    out.push({
      id: `local.inner-shadows-import.fn.call.same.${lang}`, step: 'local', shape: 'inner-shadows-import', scope: 'fn', form: 'call',
      naming: 'same', lang, siteFile: `caller.${e}`,
      files: { [cf]: calleeSrc(lang), [`caller.${e}`]: `import { f } from './callee';\nimport * as N from './callee';\nexport const keep = f;\nexport function caller() {\n  const f = N.g;\n  return f(1); // SITE\n}\n` },
      truth: { t: 'one', target: `${cf}:g` }, contract: null,
    });
    out.push({
      id: `param.inner-param-shadows-alias.call.trap.${lang}`, step: 'param-lookup', shape: 'inner-param-shadows-alias', scope: 'param', form: 'call',
      naming: 'trap', lang, siteFile: `caller.${e}`,
      files: { [cf]: calleeSrc(lang), [`other.${e}`]: otherSrc(lang), [`caller.${e}`]: `import * as N from './callee';\nexport function outer() {\n  const decoy = N.f;\n  function caller(decoy${a}) {\n    return decoy(1); // SITE\n  }\n  return [caller, decoy];\n}\n` },
      truth: { t: 'may', targets: [] }, contract: null,
    });
  }
  return out;
}

// ---- import, same-file and global shapes ----------------------------------------------------
function importVariants() {
  const out = [];
  for (const lang of ['ts', 'js']) {
    const e = ext(lang, 'call');
    const ex = ext(lang, 'jsx');
    const cf = `callee.${e}`;
    const n = ty(lang, ': number');
    const a = ty(lang, ': any');
    const add = (shape, files, siteFile, truth, extra = {}) => out.push({
      id: `${extra.group || 'import'}.${shape}.${extra.form || 'call'}.${lang}`, step: extra.step || 'import', shape, scope: 'module',
      form: extra.form || 'call', naming: extra.naming || 'n/a', lang, files, siteFile, truth, contract: extra.contract || null,
    });
    const callee = calleeSrc(lang);
    const one = (t) => ({ t: 'one', target: t });
    add('named', { [cf]: callee, [`caller.${e}`]: `import { f } from './callee';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`));
    add('renamed', { [cf]: callee, [`caller.${e}`]: `import { f as q } from './callee';\nexport function caller() {\n  return q(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`));
    add('renamed-to-decoy', { [cf]: callee, [`other.${e}`]: otherSrc(lang), [`caller.${e}`]: `import { f as decoy } from './callee';\nexport function caller() {\n  return decoy(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`), { naming: 'trap' });
    add('default', { [`dflt.${e}`]: `export default function f(x${n}) { return x; }\n`, [`caller.${e}`]: `import d from './dflt';\nexport function caller() {\n  return d(1); // SITE\n}\n` }, `caller.${e}`, one(`dflt.${e}:f`));
    add('ns-qualified', { [cf]: callee, [`caller.${e}`]: `import * as N from './callee';\nexport function caller() {\n  return N.f(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`));
    add('barrel-named', { [cf]: callee, [`barrel.${e}`]: `export { f } from './callee';\n`, [`caller.${e}`]: `import { f } from './barrel';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`));
    add('barrel-star', { [cf]: callee, [`barrel.${e}`]: `export * from './callee';\n`, [`caller.${e}`]: `import { f } from './barrel';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`));
    add('barrel-renamed', { [cf]: callee, [`barrel.${e}`]: `export { f as q } from './callee';\n`, [`caller.${e}`]: `import { q } from './barrel';\nexport function caller() {\n  return q(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`));
    add('named-exported-alias', { [cf]: callee, [`caller.${e}`]: `import { renamed } from './callee';\nexport function caller() {\n  return renamed(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`));
    add('object-literal-member-call', { [cf]: callee, [`caller.${e}`]: `import { f as imp } from './callee';\nconst api = { run: imp };\nexport function caller() {\n  return api.run(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`), { group: 'member-alias', step: 'none', contract: 'qualified callee through an object literal: outside the bare-name lookup' });
    add('object-shorthand-member-call', { [cf]: callee, [`caller.${e}`]: `import { f } from './callee';\nconst api = { f };\nexport function caller() {\n  return api.f(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`), { group: 'member-alias', step: 'none', contract: 'qualified callee through an object literal: outside the bare-name lookup' });
    add('props-member-call', { [cf]: callee, [`caller.${ex}`]: `import { f as imp } from './callee';\nfunction C(props${a}) {\n  return props.cb(1); // SITE\n}\nexport function App() { return <C cb={imp} />; }\n` }, `caller.${ex}`, one(`${cf}:f`), { group: 'member-alias', step: 'none', contract: 'qualified callee through props: outside the bare-name lookup' });
    add('external-named-trap', { [cf]: callee, [`caller.${e}`]: `import { f } from 'ext-pkg';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, { t: 'none' }, { naming: 'trap' });
    add('external-default-trap', { [cf]: callee, [`caller.${e}`]: `import f from 'ext-pkg';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, { t: 'none' }, { naming: 'trap' });
    add('external-ns-member-alias-trap', { [cf]: callee, [`caller.${e}`]: `import * as X from 'ext-pkg';\nexport function caller() {\n  const f = X.f;\n  return f(1); // SITE\n}\n` }, `caller.${e}`, { t: 'none' }, { naming: 'trap', step: 'local' });
    add('named-jsx', { [cf]: callee, [`caller.${ex}`]: `import { F } from './callee';\nexport function App() {\n  return <F />; // SITE\n}\n` }, `caller.${ex}`, one(`${cf}:F`), { form: 'jsx' });
    add('ns-qualified-jsx', { [cf]: callee, [`caller.${ex}`]: `import * as N from './callee';\nexport function App() {\n  return <N.F />; // SITE\n}\n` }, `caller.${ex}`, one(`${cf}:F`), { form: 'jsx' });
    add('external-jsx-trap', { [cf]: callee, [`caller.${ex}`]: `import { F } from 'ext-ui';\nexport function App() {\n  return <F />; // SITE\n}\n` }, `caller.${ex}`, { t: 'none' }, { form: 'jsx', naming: 'trap' });
    // module path shapes: explicit .js suffix (a TypeScript file on disk when lang is ts), dot specifiers, redundant segments
    add('named-js-suffix', { [cf]: callee, [`caller.${e}`]: `import { f } from './callee.js';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`), { step: 'import-path' });
    add('renamed-to-decoy-js-suffix', { [cf]: callee, [`other.${e}`]: otherSrc(lang), [`caller.${e}`]: `import { f as decoy } from './callee.js';\nexport function caller() {\n  return decoy(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`), { step: 'import-path', naming: 'trap' });
    add('default-js-suffix', { [`dflt.${e}`]: `export default function f(x${n}) { return x; }\n`, [`caller.${e}`]: `import d from './dflt.js';\nexport function caller() {\n  return d(1); // SITE\n}\n` }, `caller.${e}`, one(`dflt.${e}:f`), { step: 'import-path' });
    add('ns-qualified-js-suffix', { [cf]: callee, [`caller.${e}`]: `import * as N from './callee.js';\nexport function caller() {\n  return N.f(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`), { step: 'import-path' });
    add('barrel-named-js-suffix', { [cf]: callee, [`barrel.${e}`]: `export { f } from './callee.js';\n`, [`caller.${e}`]: `import { f } from './barrel.js';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`), { step: 'import-path' });
    add('barrel-star-js-suffix', { [cf]: callee, [`barrel.${e}`]: `export * from './callee.js';\n`, [`caller.${e}`]: `import { f } from './barrel.js';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`), { step: 'import-path' });
    add('named-jsx-js-suffix', { [cf]: callee, [`caller.${ex}`]: `import { F } from './callee.js';\nexport function App() {\n  return <F />; // SITE\n}\n` }, `caller.${ex}`, one(`${cf}:F`), { step: 'import-path', form: 'jsx' });
    add('ns-member-alias-js-suffix', { [cf]: callee, [`caller.${e}`]: `import * as N from './callee.js';\nexport function caller() {\n  const h = N.f;\n  return h(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`), { step: 'local', naming: 'fresh' });
    add('missing-export-js-suffix-trap', { [cf]: callee, [`other.${e}`]: otherSrc(lang), [`caller.${e}`]: `// @ts-ignore\nimport { decoy } from './callee.js';\nexport function caller() {\n  return decoy(1); // SITE\n}\n` }, `caller.${e}`, { t: 'none' }, { step: 'import-path', naming: 'trap' });
    add('dot-index', { [`pkg/index.${e}`]: callee, [`pkg/app.${e}`]: `import { f } from '.';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `pkg/app.${e}`, one(`pkg/index.${e}:f`), { step: 'import-path' });
    add('dotdot-index', { [`pkg/index.${e}`]: callee, [`pkg/sub/app.${e}`]: `import { f } from '..';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `pkg/sub/app.${e}`, one(`pkg/index.${e}:f`), { step: 'import-path' });
    add('redundant-slash', { [`lib/callee.${e}`]: callee, [`caller.${e}`]: `import { f } from './lib//callee';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`lib/callee.${e}:f`), { step: 'import-path' });
    add('dot-segment', { [`lib/callee.${e}`]: callee, [`caller.${e}`]: `import { f } from './lib/./callee';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`lib/callee.${e}:f`), { step: 'import-path' });
    add('mid-dotdot-segment', { [cf]: callee, [`lib/x.${e}`]: `export const z = 1;\n`, [`caller.${e}`]: `import { f } from './lib/../callee';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`${cf}:f`), { step: 'import-path' });
    // same-file
    add('hoisted-function', { [cf]: callee, [`caller.${e}`]: `export function caller() {\n  return f(1); // SITE\n}\nfunction f(x${n}) { return x * 2; }\n` }, `caller.${e}`, one(`caller.${e}:f`), { group: 'same-file', step: 'same-file', naming: 'trap' });
    add('nested-shadows-import', { [cf]: callee, [`caller.${e}`]: `import { f } from './callee';\nexport const keep = f;\nexport function caller() {\n  function f(x${n}) { return x * 2; }\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`caller.${e}:f`), { group: 'same-file', step: 'same-file', naming: 'trap' });
    // globals and unbound
    add('module-unbound-module-target', { [cf]: callee, [`caller.${e}`]: `export function caller() {\n  // @ts-ignore\n  return f(1); // SITE\n}\n` }, `caller.${e}`, { t: 'none' }, { group: 'global', step: 'none', naming: 'trap' });
    add('lib-global-trap', { [`callee.${e}`]: `export function isNaN(x${a}) { return false; }\n`, [`caller.${e}`]: `export function caller() {\n  return isNaN(1); // SITE\n}\n` }, `caller.${e}`, { t: 'none' }, { group: 'global', step: 'global', naming: 'trap' });
    add('module-to-script-global', { [`glob.${e}`]: `function f(x${n}) { return x; }\n`, [`caller.${e}`]: `export function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`glob.${e}:f`), { group: 'global', step: 'global' });
    add('script-to-script-global', { [`glob.${e}`]: `function f(x${n}) { return x; }\n`, [`caller.${e}`]: `function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`glob.${e}:f`), { group: 'global', step: 'global' });
  }
  // TypeScript-only shapes from the two E5 review rounds
  const callee = calleeSrc('ts');
  const mk = (shape, files, siteFile, truth, extra = {}) => out.push({
    id: `${extra.group || 'import'}.${shape}.${extra.form || 'call'}.ts`, step: extra.step || 'import', shape, scope: 'module', form: extra.form || 'call',
    naming: extra.naming || 'n/a', lang: 'ts', files, siteFile, truth, contract: extra.contract || null,
  });
  mk('import-equals-require', { 'callee.ts': callee, 'caller.ts': `import q = require('./callee');\nexport function caller() {\n  return q.f(1); // SITE\n}\n` }, 'caller.ts', { t: 'one', target: 'callee.ts:f' });
  mk('import-equals-exported-alias', { 'callee.ts': callee, 'caller.ts': `import * as N from './callee';\nimport f = N.renamed;\nexport function caller() {\n  return f(1); // SITE\n}\n` }, 'caller.ts', { t: 'one', target: 'callee.ts:f' }, { step: 'local' });
  mk('namespace-body-script-global', { 'glob.ts': `function f(x: number) { return x; }\n`, 'caller.ts': `export namespace M { export const tag = 1; }\nexport namespace M {\n  export function caller() {\n    return f(1); // SITE\n  }\n}\n` }, 'caller.ts', { t: 'one', target: 'glob.ts:f' }, { group: 'global', step: 'global' });
  mk('super-extends-member', { 'base.ts': `export class Base { constructor(public x: number) {} }\nexport const bag = { super: Base };\n`, 'caller.ts': `import { bag } from './base';\nexport class Child extends bag.super {\n  constructor(x: number) {\n    super(x); // SITE\n  }\n}\n` }, 'caller.ts', { t: 'one', target: 'base.ts:Base' }, { step: 'import', contract: 'super(...) through an imported member' });
  const G = `function f(x: number) { return x; }\n`;
  const one = (t) => ({ t: 'one', target: t });
  mk('named-js-suffix-tsx-target', { 'comp.tsx': `export function F(props: any) { return null; }\n`, 'caller.tsx': `import { F } from './comp.js';\nexport function App() {\n  return <F />; // SITE\n}\n` }, 'caller.tsx', one('comp.tsx:F'), { step: 'import-path', form: 'jsx' });
  mk('named-jsx-suffix-tsx-target', { 'comp.tsx': `export function F(props: any) { return null; }\n`, 'caller.tsx': `import { F } from './comp.jsx';\nexport function App() {\n  return <F />; // SITE\n}\n` }, 'caller.tsx', one('comp.tsx:F'), { step: 'import-path', form: 'jsx', contract: 'explicit .jsx suffix naming a .tsx file: not modeled by the import-suffix slice' });
  mk('named-js-suffix-literal-and-ts', { 'callee.js': `export function f(x) { return x; }\n`, 'callee.ts': `export function f(x: number) { return x; }\n`, 'caller.ts': `import { f } from './callee.js';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, 'caller.ts', { t: 'set', targets: ['callee.js:f', 'callee.ts:f'] }, { step: 'import-path', contract: 'both x.js and x.ts exist: prism keeps the literal file, TypeScript prefers x.ts' });
  mk('named-js-suffix-dts-only', { 'callee.d.ts': `export declare function f(x: number): number;\n`, 'other.ts': `export function f(x: number) { return -x; }\n`, 'caller.ts': `import { f } from './callee.js';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, 'caller.ts', { t: 'none' }, { step: 'import-path', naming: 'trap' });
  mk('enum-merged-ns-script-global', { 'glob.ts': G, 'caller.ts': `export enum Color { Red = 1 }\nexport namespace Color {\n  export function caller(x: number) {\n    return f(x); // SITE\n  }\n}\n` }, 'caller.ts', one('glob.ts:f'), { group: 'global', step: 'global' });
  mk('nonexported-merged-ns-plus-import-script-global', { 'glob.ts': G, 'dep.ts': `export const v = 1;\n`, 'caller.ts': `import { v } from './dep';\nexport const keep = v;\nnamespace A { export const a = 1; }\nnamespace A {\n  export function caller(x: number) {\n    return f(x); // SITE\n  }\n}\n` }, 'caller.ts', one('glob.ts:f'), { group: 'global', step: 'global' });
  mk('decorated-class-expr-script-global', { 'glob.ts': `function f(...args: any[]): any { return () => {}; }\n`, 'caller.ts': `export function caller(x: any) {\n  const K = @f() class Named {}; // SITE\n  return K;\n}\n` }, 'caller.ts', one('glob.ts:f'), { group: 'global', step: 'global' });
  mk('single-ns-script-global', { 'glob.ts': G, 'caller.ts': `export namespace A {\n  export function caller(x: number) {\n    return f(x); // SITE\n  }\n}\n` }, 'caller.ts', one('glob.ts:f'), { group: 'global', step: 'global' });
  mk('merged-ns-module-target-trap', { 'callee.ts': `export function f(x: number) { return x; }\n`, 'caller.ts': `export namespace A { export const a = 1; }\nexport namespace A {\n  export function caller(x: number) {\n    // @ts-ignore\n    return f(x); // SITE\n  }\n}\n` }, 'caller.ts', { t: 'none' }, { group: 'global', step: 'none', naming: 'trap' });
  mk('merged-ns-local-alias-unknown', { 'glob.ts': G, 'caller.ts': `export namespace A { export const a = 1; }\nexport namespace A {\n  export function caller(o: any) {\n    const f = o.cb;\n    return f(1); // SITE\n  }\n}\n` }, 'caller.ts', { t: 'may', targets: [] }, { group: 'global', step: 'local', naming: 'trap' });
  // indirect records (a static property assigned in another file): outside the bare-name lookup, must not change
  out.push({
    id: 'member-alias.static-property-jsx.jsx.js', step: 'none', shape: 'static-property-jsx', scope: 'module', form: 'jsx', naming: 'n/a', lang: 'js', siteFile: 'App.jsx',
    files: {
      'App.jsx': `import { Menu } from "./Menu";\nexport function caller() {\n  return <Menu.Item />; // SITE\n}\n`,
      'Menu.jsx': `import { MenuItem } from "./MenuItem";\nexport function Menu() { return null; }\nMenu.Item = MenuItem;\n`,
      'MenuItem.jsx': `export function MenuItem() { return null; }\n`,
    },
    truth: { t: 'one', target: 'MenuItem.jsx:MenuItem' }, contract: 'indirect record through a static property (E5 review round 1): outside the bare-name lookup',
  });
  out.push({
    id: 'member-alias.static-property-call.call.js', step: 'none', shape: 'static-property-call', scope: 'module', form: 'call', naming: 'n/a', lang: 'js', siteFile: 'caller.js',
    files: {
      'api.js': `import { helper } from "./helper.js";\nexport function api() {}\napi.run = helper;\n`,
      'caller.js': `import { api } from "./api.js";\nexport function caller(x) {\n  return api.run(x); // SITE\n}\n`,
      'helper.js': `export function helper(x) { return x; }\n`,
    },
    truth: { t: 'one', target: 'helper.js:helper' }, contract: 'indirect record through a static property (E5 review round 1): outside the bare-name lookup',
  });
  out.push({
    id: 'import.super-extends-member-named-super.call.js', step: 'import', shape: 'super-extends-member-named-super', scope: 'module', form: 'call',
    naming: 'same', lang: 'js', siteFile: 'caller.js',
    files: {
      'callee.js': `export const Base = { super: function(x) { this.value = x; } };\n`,
      'caller.js': `import { Base } from './callee';\nexport class Child extends Base.super {\n  constructor(x) {\n    super(x); // SITE\n  }\n}\n`,
    },
    truth: { t: 'one', target: 'callee.js:super' }, contract: 'super(...) through an imported member (E5 review round 2, constructed)',
  });
  return out;
}

function all() {
  return [...localVariants(), ...cjsVariants(), ...paramVariants(), ...importVariants()];
}
module.exports = { all };
