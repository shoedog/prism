// Import, module-path, same-file, script-global and out-of-scope control shapes.
'use strict';
const { ext, ty, calleeSrc, otherSrc } = require('./variants-common.cjs');

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
    add('same-stem-two-dirs', { [`a/callee.${e}`]: callee, [`b/callee.${e}`]: callee, [`caller.${e}`]: `import { f } from './a/callee';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`a/callee.${e}:f`), { step: 'import-path', naming: 'trap' });
    add('same-stem-two-dirs-js-suffix', { [`a/callee.${e}`]: callee, [`b/callee.${e}`]: callee, [`caller.${e}`]: `import { f } from './b/callee.js';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, `caller.${e}`, one(`b/callee.${e}:f`), { step: 'import-path', naming: 'trap' });
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
  mk('named-js-suffix-literal-and-ts', { 'callee.js': `export function f(x) { return x; }\n`, 'callee.ts': `export function f(x: number) { return x; }\n`, 'caller.ts': `import { f } from './callee.js';\nexport function caller() {\n  return f(1); // SITE\n}\n` }, 'caller.ts', { t: 'one', target: 'callee.js:f' }, { step: 'import-path', contract: 'policy row: both x.js and x.ts exist; prism keeps the literal file (as main does), TypeScript itself would bind x.ts' });
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

module.exports = { importVariants };
