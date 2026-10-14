// Argument following: the callee of the marked site is a parameter (plain, defaulted, rest,
// destructured, a props object) of the function that holds it, so what it calls is whatever the
// callers of that function pass at that position. `step` is 'arg'.
// Truths are authored from what the code statically allows:
//   `one`/`set`/`anon`: every caller of the parameter's function is visible in the fixture and
//     the function does not escape (not exported, not passed or stored as a value);
//   `may`: the function is exported or escapes, so invisible callers may pass anything;
//   `none`: the visible callers pass nothing callable that lives in the repo.
'use strict';
const { ext, ty, calleeSrc, otherSrc } = require('./variants-common.cjs');

function argVariants() {
  const out = [];
  for (const lang of ['ts', 'js']) {
    const e = ext(lang, 'call');
    const ex = ext(lang, 'jsx');
    const n = ty(lang, ': number');
    const a = ty(lang, ': any');
    const b = ty(lang, ': boolean');
    const cf = `callee.${e}`;
    const IMP = `import { f as imp, g as imp2 } from './callee';`;
    const one = (t) => ({ t: 'one', target: t });
    const set = (...ts) => ({ t: 'set', targets: ts });
    const may = (...ts) => ({ t: 'may', targets: ts });
    // add(shape, {src, truth, form, jsx, naming, contract, files})
    const add = (shape, o) => {
      const jsx = !!o.jsx;
      const siteFile = `caller.${jsx ? ex : e}`;
      const files = { [cf]: calleeSrc(lang), ...(o.files || {}) };
      if (o.naming === 'trap') files[`other.${e}`] = otherSrc(lang);
      files[siteFile] = o.src;
      out.push({
        id: `arg.${shape}.${o.form || 'call'}.${lang}`, step: 'arg', shape, scope: 'param', form: o.form || 'call',
        naming: o.naming || 'fresh', lang, files, siteFile, truth: o.truth, contract: o.contract || null,
      });
    };
    const caller = `caller.${e}`;
    const callerX = `caller.${ex}`;

    // ---- what one in-repo caller passes -------------------------------------------------------
    add('named-fn-same-file', {
      src: `function save(x${n}) { return x; }\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(save); }\n`,
      truth: one(`${caller}:save`),
    });
    add('const-arrow-same-file', {
      src: `const save = (x${n}) => x;\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(save); }\n`,
      truth: one(`${caller}:save`),
    });
    add('inline-arrow', {
      src: `function run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() {\n  return run((x${n}) => x); // ANON\n}\n`,
      truth: { t: 'anon', file: caller },
    });
    add('inline-function-expression', {
      src: `function run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() {\n  return run(function (x${n}) { return x; }); // ANON\n}\n`,
      truth: { t: 'anon', file: caller },
    });
    add('imported-fn', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(imp); }\n`,
      truth: one(`${cf}:f`),
    });
    add('const-alias-of-import', {
      src: `${IMP}\nconst h = imp;\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(h); }\n`,
      truth: one(`${cf}:f`),
    });
    add('namespace-member', {
      src: `import * as N from './callee';\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(N.f); }\n`,
      truth: one(`${cf}:f`),
    });
    add('second-position', {
      src: `${IMP}\nfunction run(x${a}, cb${a}) {\n  return cb(x); // SITE\n}\nexport function driver() { return run(imp2, imp); }\n`,
      truth: one(`${cf}:f`), naming: 'trap', contract: 'the argument at the parameter\'s own position, not another one',
    });
    add('param-shadows-import-name', {
      src: `import { f, g as imp2 } from './callee';\nexport const keep = f;\nfunction run(f${a}) {\n  return f(1); // SITE\n}\nexport function driver() { return run(imp2); }\n`,
      truth: one(`${cf}:g`), naming: 'trap',
    });
    add('non-callable-arg', {
      src: `function run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(42); }\n`,
      truth: { t: 'none' },
    });
    add('external-arg', {
      src: `import { ext } from 'ext-pkg';\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(ext); }\n`,
      truth: { t: 'none' },
    });
    add('no-callers', {
      src: `function run(cb${a}) {\n  return cb(1); // SITE\n}\nexport const unused = 1;\n`,
      truth: { t: 'none' }, contract: 'a function nothing calls or references: its parameter receives nothing',
    });
    add('new-through-param', {
      src: `import { K } from './callee';\nfunction make(Ctor${a}) {\n  return new Ctor(1); // SITE\n}\nexport function driver() { return make(K); }\n`,
      truth: one(`${cf}:K`), form: 'new',
    });

    // ---- several callers -----------------------------------------------------------------------
    add('two-callers-two-fns', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function d1() { return run(imp); }\nexport function d2() { return run(imp2); }\n`,
      truth: set(`${cf}:f`, `${cf}:g`),
    });
    add('two-callers-same-fn', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function d1() { return run(imp); }\nexport function d2() { return run(imp); }\n`,
      truth: one(`${cf}:f`),
    });
    add('callers-mixed-external', {
      src: `${IMP}\nimport { ext } from 'ext-pkg';\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function d1() { return run(imp); }\nexport function d2() { return run(ext); }\n`,
      truth: may(`${cf}:f`), contract: 'one caller passes a function outside the repo',
    });
    add('caller-passes-own-param', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function d1(h${a}) { return run(h); }\nexport function d2() { return run(imp); }\n`,
      truth: may(`${cf}:f`), contract: 'a caller forwards its own parameter, and that caller is exported',
    });
    add('conditional-arg', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver(c${b}) { return run(c ? imp : imp2); }\n`,
      truth: set(`${cf}:f`, `${cf}:g`), contract: 'the argument is a conditional of two callables',
    });

    // ---- exported or escaping holder ------------------------------------------------------------
    add('exported-holder', {
      src: `${IMP}\nexport function run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(imp); }\n`,
      truth: may(`${cf}:f`), contract: 'exported callable: callers may be invisible',
    });
    add('exported-via-list', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(imp); }\nexport { run };\n`,
      truth: may(`${cf}:f`), contract: 'exported callable: callers may be invisible',
    });
    add('default-exported-holder', {
      src: `${IMP}\nexport default function run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(imp); }\n`,
      truth: may(`${cf}:f`), contract: 'exported callable: callers may be invisible',
    });
    add('escapes-to-external', {
      src: `${IMP}\nimport { register } from 'ext-pkg';\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nregister(run);\nexport function driver() { return run(imp); }\n`,
      truth: may(`${cf}:f`), contract: 'the holder is passed to code outside the repo',
    });
    add('escapes-to-local-table', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nconst table = [run];\nexport function driver() { table[0](imp2); return run(imp); }\n`,
      truth: set(`${cf}:f`, `${cf}:g`), contract: 'the holder is stored and called through a table: a reverse-edge rule sees only the direct caller',
    });
    add('holder-passed-itself', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb(imp); // SITE\n}\nexport function driver() { return run(run); }\n`,
      truth: one(`caller.${e}:run`), contract: 'the holder is passed as its own argument (escapes into itself)',
    });

    // ---- the parameter's value inside the holder ----------------------------------------------
    add('reassigned-before-call', {
      src: `${IMP}\nfunction run(cb${a}) {\n  cb = imp2;\n  return cb(1); // SITE\n}\nexport function driver() { return run(imp); }\n`,
      truth: one(`${cf}:g`), contract: 'the parameter is overwritten before the call',
    });
    add('reassigned-conditionally', {
      src: `${IMP}\nfunction run(c${b}, cb${a}) {\n  if (c) { cb = imp2; }\n  return cb(1); // SITE\n}\nexport function driver(c${b}) { return run(c, imp); }\n`,
      truth: set(`${cf}:f`, `${cf}:g`),
    });
    add('inner-block-shadow', {
      src: `${IMP}\nfunction run(cb${a}) {\n  {\n    const cb = imp2;\n    void cb;\n  }\n  return cb(1); // SITE\n}\nexport function driver() { return run(imp); }\n`,
      truth: one(`${cf}:f`), naming: 'trap',
    });
    add('closure-in-holder', {
      src: `${IMP}\nfunction run(cb${a}) {\n  const later = () => {\n    return cb(1); // SITE\n  };\n  return later;\n}\nexport function driver() { return run(imp); }\n`,
      truth: one(`${cf}:f`),
    });
    add('optional-call', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb?.(1); // SITE\n}\nexport function driver() { return run(imp); }\n`,
      truth: one(`${cf}:f`), form: 'opt',
    });
    add('guarded-call', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb && cb(1); // SITE\n}\nexport function d1() { return run(imp); }\nexport function d2() { return run(undefined); }\n`,
      truth: one(`${cf}:f`), contract: 'one caller passes undefined (no callee)',
    });

    // ---- defaults --------------------------------------------------------------------------------
    add('default-unused', {
      src: `${IMP}\nfunction run(cb${a} = imp) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(); }\n`,
      truth: one(`${cf}:f`), contract: 'the default applies at the only call',
    });
    add('default-overridden', {
      src: `${IMP}\nfunction run(cb${a} = imp) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(imp2); }\n`,
      truth: one(`${cf}:g`), naming: 'trap',
    });
    add('default-mixed', {
      src: `${IMP}\nfunction run(cb${a} = imp) {\n  return cb(1); // SITE\n}\nexport function d1() { return run(); }\nexport function d2() { return run(imp2); }\n`,
      truth: set(`${cf}:f`, `${cf}:g`),
    });
    add('default-undefined-arg', {
      src: `${IMP}\nfunction run(cb${a} = imp) {\n  return cb(1); // SITE\n}\nexport function driver() { return run(undefined); }\n`,
      truth: one(`${cf}:f`), contract: 'an explicit undefined argument takes the default',
    });

    // ---- rest parameters and `arguments` ---------------------------------------------------------
    add('rest-index', {
      src: `${IMP}\nfunction run(...cbs${a}) {\n  return cbs[0](1); // SITE\n}\nexport function driver() { return run(imp, imp2); }\n`,
      truth: one(`${cf}:f`), contract: 'element access on a rest parameter: not a bare-name callee',
    });
    add('rest-destructured', {
      src: `${IMP}\nfunction run(...args${a}) {\n  const [x, cb] = args;\n  void x;\n  return cb(1); // SITE\n}\nexport function driver() { return run(imp2, imp); }\n`,
      truth: one(`${cf}:f`), naming: 'trap',
    });
    add('arguments-object', {
      src: `${IMP}\nfunction run(${lang === 'ts' ? '..._: any[]' : ''}) {\n  return arguments[0](1); // SITE\n}\nexport function driver() { return run(imp); }\n`,
      truth: one(`${cf}:f`), contract: '`arguments` element access: not a bare-name callee',
    });
    add('spread-at-caller', {
      src: `${IMP}\nfunction run(x${a}, cb${a}) {\n  void x;\n  return cb(1); // SITE\n}\nexport function driver() {\n  const args${ty(lang, ': [number, any]')} = [1, imp];\n  return run(...args);\n}\n`,
      truth: one(`${cf}:f`), contract: 'a spread argument hides positions',
    });

    // ---- destructured parameters (non-JSX) ---------------------------------------------------------
    add('destructured-object-param', {
      src: `${IMP}\nfunction run({ onDone }${a}) {\n  return onDone(1); // SITE\n}\nexport function driver() { return run({ onDone: imp }); }\n`,
      truth: one(`${cf}:f`),
    });
    add('destructured-object-shorthand-arg', {
      src: `${IMP}\nfunction run({ onDone }${a}) {\n  return onDone(1); // SITE\n}\nexport function driver() {\n  const onDone = imp2;\n  return run({ onDone });\n}\n`,
      truth: one(`${cf}:g`),
    });
    add('destructured-array-param', {
      src: `${IMP}\nfunction run([cb]${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run([imp]); }\n`,
      truth: one(`${cf}:f`),
    });
    add('destructured-arg-is-variable', {
      src: `${IMP}\nfunction run({ onDone }${a}) {\n  return onDone(1); // SITE\n}\nexport function driver() {\n  const opts = { onDone: imp };\n  return run(opts);\n}\n`,
      truth: one(`${cf}:f`), contract: 'the argument is a const bound to an object literal',
    });

    // ---- forwarding ------------------------------------------------------------------------------
    add('forwarded-two-levels', {
      src: `${IMP}\nfunction inner(cb${a}) {\n  return cb(1); // SITE\n}\nfunction mid(h${a}) { return inner(h); }\nfunction outer(k${a}) { return mid(k); }\nexport function driver() { return outer(imp); }\n`,
      truth: one(`${cf}:f`),
    });
    add('forwarded-two-levels-two-roots', {
      src: `${IMP}\nfunction inner(cb${a}) {\n  return cb(1); // SITE\n}\nfunction mid(h${a}) { return inner(h); }\nexport function d1() { return mid(imp); }\nexport function d2() { return mid(imp2); }\n`,
      truth: set(`${cf}:f`, `${cf}:g`),
    });

    // ---- .call / .apply / .bind --------------------------------------------------------------------
    add('holder-via-call', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run.call(null, imp); }\n`,
      truth: one(`${cf}:f`), contract: '`.call` shifts positions by one',
    });
    add('holder-via-apply', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() { return run.apply(null, [imp]); }\n`,
      truth: one(`${cf}:f`), contract: '`.apply` passes an array',
    });
    add('holder-via-bind', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb(1); // SITE\n}\nexport function driver() {\n  const bound = run.bind(null, imp);\n  return bound();\n}\n`,
      truth: one(`${cf}:f`), contract: '`.bind` pre-fills positions',
    });
    add('site-is-param-dot-call', {
      src: `${IMP}\nfunction run(cb${a}) {\n  return cb.call(null, 1); // SITE\n}\nexport function driver() { return run(imp); }\n`,
      truth: one(`${cf}:f`), contract: 'the site calls the parameter through `.call`: a member callee',
    });
    if (lang === 'ts') {
      add('ts-this-param', {
        src: `${IMP}\nfunction run(this: any, cb: any) {\n  return cb(1); // SITE\n}\nexport function driver() { return run.call(null, imp); }\n`,
        truth: one(`${cf}:f`), naming: 'trap', contract: 'a TypeScript `this` parameter takes no argument position',
      });
    }

    // ---- methods -----------------------------------------------------------------------------------
    add('class-method-this-call', {
      src: `${IMP}\nclass Runner {\n  run(cb${a}) {\n    return cb(1); // SITE\n  }\n  go() { return this.run(imp); }\n}\nexport function driver() { return new Runner().go(); }\n`,
      truth: one(`${cf}:f`),
    });
    add('exported-class-method', {
      src: `${IMP}\nexport class Runner {\n  run(cb${a}) {\n    return cb(1); // SITE\n  }\n  go() { return this.run(imp); }\n}\n`,
      truth: may(`${cf}:f`), contract: 'a method of an exported class: callers may be invisible',
    });
    add('object-literal-method', {
      src: `${IMP}\nconst api = {\n  run(cb${a}) {\n    return cb(1); // SITE\n  },\n};\nexport function driver() { return api.run(imp); }\n`,
      truth: one(`${cf}:f`),
    });

    // ---- inline callbacks handed to other code -------------------------------------------------------
    add('param-of-callback-to-external', {
      src: `${IMP}\nexport function driver(items${a}) {\n  return items.map((fn${a}) => {\n    return fn(1); // SITE\n  });\n}\n`,
      truth: may(), contract: 'the holder is an inline callback given to unknown code',
    });
    add('param-of-callback-to-local-hof', {
      src: `${IMP}\nfunction each(xs${a}, visit${a}) { for (const x of xs) visit(x); }\nexport function driver() {\n  return each([imp], (fn${a}) => {\n    return fn(1); // SITE\n  });\n}\n`,
      truth: one(`${cf}:f`), contract: 'the holder is an inline callback given to an in-repo function (two hops)',
    });

    // ---- React props (JSX callers) ------------------------------------------------------------------
    const jx = (shape, src, truth, extra = {}) => add(shape, { src, truth, jsx: true, ...extra });
    jx('props-destructured', `${IMP}\nfunction Btn({ onClick }${a}) {\n  return onClick(1); // SITE\n}\nexport function App() { return <Btn onClick={imp} />; }\n`, one(`${cf}:f`));
    jx('props-destructured-arrow-component', `${IMP}\nconst Btn = ({ onClick }${a}) => {\n  return onClick(1); // SITE\n};\nexport function App() { return <Btn onClick={imp} />; }\n`, one(`${cf}:f`));
    jx('props-member', `${IMP}\nfunction Btn(props${a}) {\n  return props.onClick(1); // SITE\n}\nexport function App() { return <Btn onClick={imp} />; }\n`, one(`${cf}:f`), { contract: 'a member callee on the props parameter' });
    jx('props-two-elements', `${IMP}\nfunction Btn({ onClick }${a}) {\n  return onClick(1); // SITE\n}\nexport function App() { return <div><Btn onClick={imp} /><Btn onClick={imp2} /></div>; }\n`, set(`${cf}:f`, `${cf}:g`));
    jx('props-inline-arrow', `${IMP}\nfunction Btn({ onClick }${a}) {\n  return onClick(1); // SITE\n}\nexport function App() {\n  return <Btn onClick={(x${n}) => imp(x)} />; // ANON\n}\n`, { t: 'anon', file: callerX });
    jx('props-local-handler', `function Btn({ onClick }${a}) {\n  return onClick(1); // SITE\n}\nexport function App() {\n  const save = (x${n}) => x;\n  return <Btn onClick={save} />;\n}\n`, one(`${callerX}:save`));
    jx('props-missing-at-one-caller', `${IMP}\nfunction Btn({ onClick }${a}) {\n  return onClick?.(1); // SITE\n}\nexport function App() { return <div><Btn /><Btn onClick={imp} /></div>; }\n`, one(`${cf}:f`), { form: 'opt' });
    jx('props-spread-caller', `${IMP}\nfunction Btn({ onClick }${a}) {\n  return onClick(1); // SITE\n}\nexport function App(p${a}) { return <Btn {...p} />; }\n`, may(), { contract: 'a spread attribute hides the prop' });
    jx('props-in-effect-closure', `${IMP}\nimport { useEffect } from 'react';\nfunction Btn({ onDone }${a}) {\n  useEffect(() => {\n    onDone(1); // SITE\n  }, [onDone]);\n  return null;\n}\nexport function App() { return <Btn onDone={imp} />; }\n`, one(`${cf}:f`));
    jx('props-forwarded-two-levels', `${IMP}\nfunction Inner({ onClick }${a}) {\n  return onClick(1); // SITE\n}\nfunction Mid({ onSave }${a}) { return <Inner onClick={onSave} />; }\nfunction Outer({ handler }${a}) { return <Mid onSave={handler} />; }\nexport function App() { return <Outer handler={imp} />; }\n`, one(`${cf}:f`));
    jx('props-memo-wrapped', `${IMP}\nimport { memo } from 'react';\nconst Btn = memo(({ onClick }${a}) => {\n  return onClick(1); // SITE\n});\nexport function App() { return <Btn onClick={imp} />; }\n`, one(`${cf}:f`), { contract: 'the component is a memo() of an inline function' });
    jx('props-exported-component', `${IMP}\nexport function Btn({ onClick }${a}) {\n  return onClick(1); // SITE\n}\nexport function App() { return <Btn onClick={imp} />; }\n`, may(`${cf}:f`), { contract: 'exported component: callers may be invisible' });
    jx('props-component-slot', `import { F } from './callee';\nfunction Slot({ Icon }${a}) {\n  return <Icon />; // SITE\n}\nexport function App() { return <Slot Icon={F} />; }\n`, one(`${cf}:F`), { form: 'jsx' });
    jx('props-name-trap', `${IMP}\nimport { decoy } from './other';\nexport const keep = decoy;\nfunction Btn({ decoy }${a}) {\n  return decoy(1); // SITE\n}\nexport function App() { return <Btn decoy={imp} />; }\n`, one(`${cf}:f`), { naming: 'trap' });
  }
  return out;
}

module.exports = { argVariants };
