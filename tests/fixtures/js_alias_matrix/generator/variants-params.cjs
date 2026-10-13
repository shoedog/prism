// Parameter and props shapes.
'use strict';
const { FORMS, ext, ty, calleeSrc, otherSrc, baseFiles } = require('./variants-common.cjs');

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
      truth: { t: 'may', targets: [] }, contract: null,
    });
    out.push({
      id: `local.inner-shadows-import.fn.call.same.${lang}`, step: 'local', shape: 'inner-shadows-import', scope: 'fn', form: 'call',
      naming: 'same', lang, siteFile: `caller.${e}`,
      files: { [cf]: calleeSrc(lang), [`caller.${e}`]: `import { f } from './callee';\nimport * as N from './callee';\nexport const keep = f;\nexport function caller() {\n  const f = N.g;\n  return f(1); // SITE\n}\n` },
      truth: { t: 'one', target: `${cf}:g` }, contract: null,
    });
    out.push({
      id: `param.ident-rhs-is-outer-param.call.fresh.${lang}`, step: 'param-lookup', shape: 'ident-rhs-is-outer-param', scope: 'param', form: 'call',
      naming: 'fresh', lang, siteFile: `caller.${e}`,
      files: { [cf]: calleeSrc(lang), [`caller.${e}`]: `import { f } from './callee';\nexport const keep = f;\nexport function outer(f${a}) {\n  function caller() {\n    const h = f;\n    return h(1); // SITE\n  }\n  return caller;\n}\n` },
      truth: { t: 'may', targets: [] }, contract: null,
    });
    out.push({
      id: `param.ident-rhs-is-own-param.call.fresh.${lang}`, step: 'param-lookup', shape: 'ident-rhs-is-own-param', scope: 'param', form: 'call',
      naming: 'fresh', lang, siteFile: `caller.${e}`,
      files: { [cf]: calleeSrc(lang), [`caller.${e}`]: `import { f } from './callee';\nexport const keep = f;\nexport function caller(f${a}) {\n  const h = f;\n  return h(1); // SITE\n}\n` },
      truth: { t: 'may', targets: [] }, contract: null,
    });
    out.push({
      id: `param.ns-receiver-is-parameter.call.same.${lang}`, step: 'param-lookup', shape: 'ns-receiver-is-parameter', scope: 'param', form: 'call',
      naming: 'same', lang, siteFile: `caller.${e}`,
      files: { [cf]: calleeSrc(lang), [`caller.${e}`]: `import * as N from './callee';\nexport const keep = N.g;\nexport function caller(N${a}) {\n  const f = N.f;\n  return f(1); // SITE\n}\n` },
      truth: { t: 'may', targets: [] }, contract: null,
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

module.exports = { paramVariants };
