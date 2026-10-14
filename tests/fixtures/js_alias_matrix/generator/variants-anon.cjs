// Lane A2 (anonymous callables): calls made INSIDE inline arrows and function expressions.
// `step` is 'anon'. The site's callee truth is ordinary (`f` in callee.*, `F` for JSX, or
// none/may); what these rows pin is whether prism records the site at all. Each fixture also
// carries an authored `caller` truth (which callable a correct call graph credits the site to),
// read by the A2 slices' caller check; today's test ignores it.
//   caller: {kind:'anon', marker:'/*C*/'}  the anonymous callable starting right after the marker
//           {kind:'named', name}           a named callable (prism's name inference applies)
// `contract` explains rows whose caller truth is a design decision of the A2 plan.
'use strict';
const { ext, ty, calleeSrc, otherSrc } = require('./variants-common.cjs');

function anonVariants() {
  const out = [];
  const langs = ['ts', 'js'];
  const anon = { kind: 'anon', marker: '/*C*/' };
  const named = (name) => ({ kind: 'named', name });
  for (const lang of langs) {
    const e = ext(lang, 'call');
    const ex = ext(lang, 'jsx');
    const a = ty(lang, ': any');
    const n = ty(lang, ': number');
    const cf = `callee.${e}`;
    const one = { t: 'one', target: `${cf}:f` };
    const oneJsx = { t: 'one', target: `${cf}:F` };
    const IMP = `import { f, F } from './callee';`;
    // add(shape, src, {truth, caller, jsx, files, contract, imports})
    const add = (shape, src, o) => {
      const jsx = !!o.jsx;
      const siteFile = `caller.${jsx ? ex : e}`;
      const files = { [cf]: calleeSrc(lang), ...(o.files || {}) };
      files[siteFile] = `${o.imports === undefined ? IMP : o.imports}\n${src}`;
      out.push({
        id: `anon.${shape}.${jsx ? 'jsx' : 'call'}.${lang}`, step: 'anon', shape, scope: o.scope || 'fn', form: jsx ? 'jsx' : 'call',
        naming: 'same', lang, files, siteFile, truth: o.truth || one, caller: o.caller || anon, contract: o.contract || null,
      });
    };

    // --- callable kinds and bodies
    add('arrow-expression-body', `export function A() {\n  return [1].map(/*C*/(x${n}) =>\n    f(x)); // SITE\n}\n`, {});
    add('arrow-block-body', `export function A() {\n  [1].forEach(/*C*/(x${n}) => {\n    f(x); // SITE\n  });\n}\n`, {});
    add('async-arrow', `export function A() {\n  setTimeout(/*C*/async () => {\n    await f(1); // SITE\n  }, 0);\n}\n`, {});
    add('function-expression', `export function A() {\n  setTimeout(/*C*/function () {\n    f(1); // SITE\n  }, 0);\n}\n`, {});
    add('named-function-expression', `export function A() {\n  setTimeout(function tick() {\n    f(1); // SITE\n  }, 0);\n}\n`, { caller: named('tick'), contract: 'control: a function expression with its own name is a named callable today' });
    add('generator-function-expression', `export function A() {\n  return run(/*C*/function* () {\n    yield f(1); // SITE\n  });\n}\nfunction run(g${a}) { return g; }\n`, { contract: 'anonymous generator expressions have no synthetic name today (PR-B covers arrow_function and function_expression only)' });
    // --- nesting
    add('nested-2', `export function A() {\n  return [1].map(() => [2].map(/*C*/(y${n}) =>\n    f(y))); // SITE\n}\n`, { contract: 'caller is the innermost anonymous callable; its enclosing chain is anon -> anon -> A' });
    add('nested-3', `export function A() {\n  [1].forEach(() => {\n    [2].forEach(() => {\n      [3].forEach(/*C*/() => {\n        f(1); // SITE\n      });\n    });\n  });\n}\n`, {});
    add('nested-in-named-local', `export function A() {\n  function inner() {\n    [1].forEach(/*C*/() => {\n      f(1); // SITE\n    });\n  }\n  return inner;\n}\n`, {});
    // --- several callables on one line (column disambiguates)
    add('siblings-one-line', `export function A() {\n  run(() => { g2(); }, /*C*/() => {\n    f(1); // SITE\n  });\n}\nfunction run(a${a}, b${a}) { a(); b(); }\nfunction g2() {}\n`, { contract: 'two anonymous callables start on one line; only the column tells them apart' });
    add('minified-one-line', `export function A(){return [1].map(()=>[2].map(/*C*/()=>f(1)))} // SITE\n`, { contract: 'minified: every callable on one line; the row also carries the external .map records' });
    // --- IIFE
    add('iife-arrow', `export function A() {\n  (/*C*/() => {\n    f(1); // SITE\n  })();\n}\n`, {});
    add('iife-function', `export function A() {\n  (/*C*/function () {\n    f(1); // SITE\n  })();\n}\n`, {});
    add('iife-callee', `export function A() {\n  return (() => 1)(); // SITE // ANON\n}\n`, { truth: { t: 'anon', file: `caller.${e}` }, caller: named('A'), contract: 'the IIFE call itself: callee is the anonymous callable, caller is A' });
    // --- callbacks passed to in-repo vs external functions
    add('arg-to-repo-fn', `import { each } from './lib';\nexport function A() {\n  each(/*C*/() => {\n    f(1); // SITE\n  });\n}\n`, { files: { [`lib.${e}`]: `export function each(cb${a}) { cb(); }\n` } });
    add('arg-to-external-fn', `import { debounce } from 'lodash';\nexport function A() {\n  return debounce(/*C*/() => {\n    f(1); // SITE\n  }, 10);\n}\n`, {});
    add('promise-then', `export function A(p${ty(lang, ': Promise<number>')}) {\n  return p.then(/*C*/(r${n}) => {\n    f(r); // SITE\n  });\n}\n`, {});
    add('new-promise-executor', `export function A() {\n  return new Promise(/*C*/(resolve${a}) => {\n    resolve(f(1)); // SITE\n  });\n}\n`, { contract: 'the row also carries the parameter call resolve(...)' });
    // --- React
    const R = (names) => `import { ${names} } from 'react';\n${IMP}`;
    add('jsx-prop-handler', `export function A() {\n  return <button onClick={/*C*/() => {\n    f(1); // SITE\n  }} />;\n}\n`, { jsx: true });
    add('jsx-element-in-map', `export function A(props${a}) {\n  return <div>{props.items.map(/*C*/(i${a}) => (\n    <F key={i} /> // SITE\n  ))}</div>;\n}\n`, { jsx: true, truth: oneJsx });
    add('use-effect', `export function A() {\n  useEffect(/*C*/() => {\n    f(1); // SITE\n  }, []);\n  return null;\n}\n`, { imports: R('useEffect') });
    add('use-effect-cleanup', `export function A() {\n  useEffect(() => /*C*/() => {\n    f(1); // SITE\n  }, []);\n  return null;\n}\n`, { imports: R('useEffect'), contract: 'the cleanup arrow is returned by the effect arrow: depth 2' });
    add('use-callback-assigned', `export function A() {\n  const h = useCallback(() => {\n    f(1); // SITE\n  }, []);\n  return h;\n}\n`, { imports: R('useCallback'), caller: named('h'), contract: 'control: name inference (const X = call(() => ...)) names the arrow h today' });
    add('use-callback-unassigned', `export function A() {\n  return useCallback(/*C*/() => {\n    f(1); // SITE\n  }, []);\n}\n`, { imports: R('useCallback') });
    add('use-memo-assigned', `export function A() {\n  const v = useMemo(() => {\n    return f(1); // SITE\n  }, []);\n  return v;\n}\n`, { imports: R('useMemo'), caller: named('v'), contract: 'today the factory arrow is named v (the memoized VALUE), by the same name-inference pattern; whether that is right is an owner question' });
    // --- class, object
    add('class-field-arrow', `export class C {\n  handle = () => {\n    f(1); // SITE\n  };\n}\n`, { caller: named('handle'), scope: 'class', contract: 'control: class field arrows are named by inference' });
    add('class-field-wrapped', `import { debounce } from 'lodash';\nexport class C {\n  handle = debounce(/*C*/() => {\n    f(1); // SITE\n  }, 10);\n}\n`, { scope: 'class', contract: 'field initializer: no enclosing callable (enclosing is the class)' });
    add('class-method-callback', `export class C {\n  run() {\n    [1].forEach(/*C*/() => {\n      f(1); // SITE\n    });\n  }\n}\n`, { scope: 'class', contract: 'recorded today with caller run (class bodies keep the legacy call inventory); under B the caller moves to the anonymous callable' });
    add('object-method', `export const api = {\n  run() {\n    [1].forEach(/*C*/() => {\n      f(1); // SITE\n    });\n  },\n};\n`, { scope: 'object' });
    add('object-arrow-property', `export const api = {\n  run: () => {\n    f(1); // SITE\n  },\n};\n`, { caller: named('run'), scope: 'object', contract: 'control: pair key names the arrow' });
    add('object-wrapped-property', `import { debounce } from 'lodash';\nexport const api = {\n  run: debounce(/*C*/() => {\n    f(1); // SITE\n  }, 10),\n};\n`, { scope: 'object' });
    add('default-param-arrow', `export function A(cb${a} = /*C*/() => {\n  f(1); // SITE\n}) {\n  return cb();\n}\n`, { scope: 'param', contract: "the default-parameter arrow is created in A's parameter scope" });
    add('export-default-arrow', `export default /*C*/() => {\n  f(1); // SITE\n};\n`, { scope: 'module' });
    add('module-level-callback', `[1].forEach(/*C*/() => {\n  f(1); // SITE\n});\nexport const k = 1;\n`, { scope: 'module', contract: 'no enclosing callable: the enclosing edge comes from the module' });
    add('parenthesized-const', `export const h = (/*C*/() => {\n  f(1); // SITE\n});\n`, { scope: 'module', contract: 'prism names const X = () => ... but not const X = (() => ...): tree-sitter keeps the parentheses' });
    // --- binding rules inside callbacks (#372 aliases, #382 returned values, shadowing)
    add('alias-outside-callback', `export function A() {\n  const h = f;\n  [1].forEach(/*C*/() => {\n    h(1); // SITE\n  });\n}\n`, {});
    add('alias-inside-callback', `export function A() {\n  [1].forEach(/*C*/() => {\n    const h = f;\n    h(1); // SITE\n  });\n}\n`, {});
    add('returned-value-in-callback', `import { useThing } from './hooks';\nexport function A() {\n  const { doIt } = useThing();\n  [1].forEach(/*C*/() => {\n    doIt(1); // SITE\n  });\n}\n`, {
      imports: '', truth: { t: 'one', target: `hooks.${e}:doIt` },
      files: { [`hooks.${e}`]: `export function useThing() {\n  function doIt(x${n}) { return x; }\n  return { doIt };\n}\n` },
    });
    add('callback-param-shadows-import', `export function A(xs${a}) {\n  xs.forEach(/*C*/(f${a}) => {\n    f(1); // SITE\n  });\n}\n`, { truth: { t: 'may', targets: [] }, contract: 'the callback parameter f shadows the import f' });
    add('enclosing-local-shadows-decoy', `export function A(useAtom${a}) {\n  const [v, decoy] = useAtom();\n  [v].forEach(/*C*/() => {\n    decoy(1); // SITE\n  });\n}\n`, {
      imports: '', truth: { t: 'may', targets: [] }, files: { [`other.${e}`]: otherSrc(lang) },
      contract: 'decoy is a local of A; an unrelated file exports a function named decoy (prototype minted a free_single Exact here)',
    });
  }
  return out;
}
module.exports = { anonVariants };
