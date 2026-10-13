// Expands the local alias shapes over scope, invocation form, naming and language; plus CommonJS shapes.
'use strict';
const { FORMS, ext, ty, baseFiles } = require('./variants-common.cjs');
const { LOCAL_SHAPES } = require('./variants-local-shapes.cjs');

const ALL_SCOPES = ['fn', 'module', 'outer'];
const SCOPE_RICH = new Set(['ident', 'ns-member', 'ns-destructure']);
const FORM_RICH = new Set(['ident', 'ns-member', 'ns-destructure', 'conditional', 'wrapper-call', 'local-arrow']);
const OPT_FORM = new Set(['ident', 'ns-member']);
// Shapes that also get the `trap2` naming: two unrelated files export the alias's name.
const TRAP2 = new Set(['ident', 'ns-member', 'let-reassigned', 'object-param-destructure', 'hook-result-destructure', 'alias-of-param']);

function truthOf(kind, c) {
  const cf = `callee.${ext(c.lang, 'call')}`;
  if (kind === 'T') return { t: 'one', target: `${cf}:${c.T}` };
  if (kind === 'SET') return { t: 'set', targets: [`${cf}:${c.T}`, `${cf}:${c.ALT}`] };
  if (kind === 'MAY') return { t: 'may', targets: [`${cf}:${c.T}`] };
  if (kind === 'LOCAL') return { t: 'one', target: `${c.callerFile}:${c.L}` };
  if (kind === 'LOCAL1') return { t: 'one', target: `${c.callerFile}:local1` };
  if (kind === 'ALT') return { t: 'one', target: `${cf}:${c.ALT}` };
  if (kind === 'ANON') return { t: 'anon', file: c.callerFile };
  if (kind === 'DFLT') return { t: 'one', target: `dflt.${ext(c.lang, 'call')}:${c.T}` };
  if (kind === 'UNKNOWN') return { t: 'may', targets: [] };
  if (kind === 'NONE') return { t: 'none' };
  throw new Error(kind);
}


function localVariants() {
  const out = [];
  for (const [shape, fn] of Object.entries(LOCAL_SHAPES)) {
    for (const lang of ['ts', 'js']) {
      const forms = FORM_RICH.has(shape) ? ['call', 'jsx', 'new'] : ['call', 'jsx'];
      if (OPT_FORM.has(shape)) forms.push('opt');
      for (const form of forms) {
        for (const naming of ['same', 'trap', 'fresh', 'trap2']) {
          if (naming === 'trap2' && !(TRAP2.has(shape) && form === 'call')) continue;
          const F = FORMS[form];
          const L = naming === 'same' ? F.T : naming === 'fresh' ? F.fresh : F.trap;
          if (form === 'jsx' && !FORM_RICH.has(shape) && !(fn({ lang, form, L: 'X', T: 'X', ALT: 'Y' }).forms || []).includes('jsx')) continue;
          const callerFile = `caller.${ext(lang, form)}`;
          const c = { lang, form, naming, L, T: F.T, ALT: F.ALT, callerFile };
          const s = fn(c);
          if (s.langs && !s.langs.includes(lang)) continue;
          if (s.forms && !s.forms.includes(form)) continue;
          if (s.namings && !s.namings.includes(naming === 'trap2' ? 'trap' : naming)) continue;
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

module.exports = { localVariants, cjsVariants };
