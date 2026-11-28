'use strict';

const grammars = ['jsx', 'tsx'];
const tags = ['self', 'paired'];

function element(tag, value) {
  return `<D a={${value}}${tag === 'self' ? '/>' : '></D>'}`;
}

const coreOwners = {
  declaration: 'function Factory(outer) {\n  function App(x) {\n    return E;\n  }\n}',
  stored: 'function Factory(outer) {\n  const App = (x) => (\n    E\n  );\n}',
  'map-expression': 'function App(outer) {\n  return items.map(x => (\n    E\n  ));\n}',
  'map-block': 'function App(outer) {\n  return items.map(x => {\n    return E;\n  });\n}',
  'attribute-expression': 'function App(outer) {\n  return <host f={x => (\n    E\n  )}/>;\n}',
  'attribute-block': 'function App(outer) {\n  return <host f={x => {\n    return E;\n  }}/>;\n}',
};
const slots = {
  call: 'wrap(E)', object: '({slot: E})', child: '<host>{E}</host>', conditional: 'flag ? E : null',
};
const coreTarget = 'const D = ({a}) => {\n  return use(a);\n};\n';

function* core() {
  const names = Object.keys(slots);
  const routes = [...names.map(a => [a]), ...names.flatMap(a => names.map(b => [a, b]))];
  for (const route of routes)
    for (const [owner, template] of Object.entries(coreOwners))
      for (const tag of tags)
        for (const layout of ['own', 'capture', 'earlier', 'write'])
          for (const grammar of grammars)
            for (const order of ['before', 'after']) {
              const name = layout === 'capture' ? 'outer' : 'x';
              let expr = element(tag, name);
              if (layout === 'earlier') expr = `(see(x), ${expr})`;
              if (layout === 'write') expr = `(see(x), x = fresh(), ${expr})`;
              for (const slot of route) expr = slots[slot].replace('E', `(${expr})`);
              const host = template.replace('E', expr) + '\n';
              const timing = order === 'after' && ['declaration', 'map-expression', 'map-block'].includes(owner);
              const concise = owner.endsWith('-expression') || owner === 'stored';
              const hasCall = route.includes('call');
              const refusal = timing ? 'refused_pre_initializer_unproved' :
                layout === 'write' ? 'refused_source_ambiguous' :
                layout === 'capture' ? (concise && !hasCall
                  ? 'refused_source_in_enclosing_owner' : 'refused_source_unconnected') : null;
              yield {
                grammar, dimensions: { route: route.join('+'), owner, tag, layout, order },
                source: order === 'before' ? coreTarget + host : host + coreTarget,
                refusal, earlier: ['earlier', 'write'].includes(layout),
                refusedLookup: layout === 'capture' && concise && hasCall
                  && refusal === 'refused_source_unconnected' ? 'disconnected' : null,
              };
            }
}

const target = 'function D({a}) {\n  return use(a);\n}\n';
const regions = {
  heritage: 'class C extends base(E) { m(x) { return x; } }',
  method: 'class C { m(x) {\n    return E;\n  } }',
  'computed-name': 'class C { [key(E)](x) { return x; } }',
  decorator: '@dec(E) class C { m(x) { return x; } }',
  'field-initializer': 'class C { slot = E; m(x) { return x; } }',
  'field-callable': 'class C { slot = x => (\n    E\n  ); }',
  'static-field': 'class C { static slot = E; m(x) { return x; } }',
  'static-block': 'class C { static { keep(E); } m(x) { return x; } }',
  'formal-default': 'function F(x, y = E) { keep(y); }',
  'formal-body': 'function F(x) {\n    return E;\n  }',
  'catch-default': 'try {} catch ({x, y = E}) { keep(y); }',
  'catch-body': 'try { throw err; } catch (err) {\n    const x = err;\n    keep(E);\n  }',
  getter: '({get v(){const x = fresh();\n    return E;}});',
  setter: '({set v(x){\n    keep(E);\n  }});',
  generator: 'function* F(x){\n    yield E;\n  }',
  'async-generator': 'async function* F(x){\n    yield E;\n  }',
  module: 'const x = fresh(); const slot = E;',
  constructor: 'class C { constructor(x){\n    keep(E);\n  } }',
  declarator: 'const slot = E; return slot;',
  assignment: 'let slot; slot = E; return slot;',
  switch: 'switch(x){case 1: keep(E);}',
  finally: 'try {} finally {keep(E);}',
};

function* region() {
  for (const [name, template] of Object.entries(regions))
    for (const layout of ['own', 'earlier'])
      for (const operand of ['compact', 'multiline'])
        for (const tag of tags) {
          const value = operand === 'compact' ? 'x' : '\n  x\n';
          let expr = element(tag, value);
          if (layout === 'earlier') expr = `(see(x), ${expr})`;
          const inner = template.replace('E', expr);
          const module = name === 'module';
          const field = name === 'field-initializer' || name === 'static-field';
          const compact = operand === 'compact';
          const conflict = field && compact && layout === 'own';
          const disconnected = name === 'catch-default' && compact && layout === 'earlier';
          const missing = name === 'catch-default' || (name === 'formal-default' && compact);
          const refusal = conflict || disconnected ? 'refused_source_ambiguous' :
            missing ? 'refused_source_not_found' : null;
          yield {
            grammar: 'tsx', dimensions: { region: name, layout, operand, tag },
            source: target + (module ? inner : `function App(x) {\n  ${inner}\n}\n`),
            registered: !module, refusal, earlier: layout === 'earlier' && compact,
            refusedLookup: conflict ? 'method-read' : disconnected ? 'disconnected' : null,
          };
        }
}

const direct = {
  call: 'keep(E)', 'optional-call': 'keep?.(E)', object: '({a: E})',
  conditional: 'flag ? E : null', logical: 'flag && E', child: '<host>{E}</host>',
  'attribute-braced': '<host a={E}/>', 'attribute-unbraced': '<host a=E/>',
  'optional-index': 'root?.[E]', template: '`prefix ${E}`', array: '[E]',
  sequence: '(0, E)', unary: 'void E', 'object-key': '({[E]: 1})',
  constructor: 'new Target(E)', await: 'await E', assignment: '(holder = E)',
};
const statements = {
  return: 'return E;', declarator: 'const held = E;', assignment: 'holder = E;',
  expression: 'E;', throw: 'throw E;', 'object-default': 'const {y = E} = obj;',
  'array-default': 'const [y = E] = list;', 'for-init': 'for (let y = E; flag;) {}',
  'for-test': 'for (; E;) {}', 'for-update': 'for (;flag; E) {}',
  'for-of': 'for (const y of E) {}', while: 'while (E) {}', 'loop-body': 'while (flag) { E; }',
  'switch-discriminant': 'switch (E) { default: break; }',
  'switch-case': 'switch (flag) { case E: break; }',
  'switch-arm': 'switch (flag) { default: E; }', try: 'try { E; } catch {}',
  catch: 'try {} catch { E; }', finally: 'try {} finally { E; }',
};
const host = {
  declaration: 'ASYNCfunction App(P) {\n  B\n}',
  'stored-block': 'const App = ASYNC(P) => {\n  B\n};',
  method: 'const obj = { ASYNCview(P) {\n  B\n} };',
  'call-block': 'function App() { return map(ASYNC(P) => {\n  B\n}); }',
  'jsx-block': 'function App() { return <host f={ASYNC(P) => {\n  B\n}}/>; }',
  'stored-expression': 'const App = ASYNC(P) => (\n  B\n);',
  'call-expression': 'function App() { return map(ASYNC(P) => (\n  B\n)); }',
  'jsx-expression': 'function App() { return <host f={ASYNC(P) => (\n  B\n)}/>; }',
};
const signatures = {
  'whole-default': 'p = E', 'binding-default': '{p = E} = {}', 'computed-binding-key': '{[E]: p} = {}',
};

function* t3() {
  const panels = { expression: direct, statement: statements,
    body: { compact: 'E', multiline: '(\n  E\n)' }, signature: signatures };
  for (const [panel, slots] of Object.entries(panels))
    for (const [slot, syntax] of Object.entries(slots))
      for (const [owner, template] of Object.entries(host))
        for (const layout of ['own', 'earlier'])
          for (const grammar of grammars) {
            const dimensions = { panel, slot, owner, layout };
            const concise = owner.endsWith('-expression');
            const excluded = panel === 'statement' && concise ? 'statement requires a block body'
              : panel === 'body' && !concise ? 'direct expression body requires a concise arrow' : null;
            if (excluded) {
              yield { grammar, dimensions, excluded };
              continue;
            }
            const earlier = layout === 'earlier';
            const elementText = element('self', 'x');
            const signature = panel === 'signature';
            const statement = panel === 'statement';
            let params = 'x';
            let body = syntax.replace('E', elementText);
            if (signature) {
              params += (earlier ? ', before = see(x), ' : ', ') + body;
              body = concise ? 'null' : 'return null;';
            } else if (statement) {
              if (earlier) body = 'see(x); ' + body;
            } else {
              if (earlier) body = panel === 'body' && slot === 'multiline'
                ? body.replace(elementText, '(see(x), ' + elementText + ')')
                : `(see(x), ${body})`;
              if (!concise) body = `return ${body};`;
            }
            const source = target + template.replace('ASYNC', slot === 'await' ? 'async ' : '')
              .replace('P', () => params).replace('B', () => body) + '\n';
            const refusal = signature && !(owner.startsWith('call-') || owner.startsWith('jsx-'))
              ? 'refused_source_not_found' : null;
            yield { grammar, dimensions, source, refusal, earlier };
          }
}

function* excludedRegionHosts() {
  for (const region of Object.keys(regions))
    for (const owner of Object.keys(host))
      yield {
        grammar: 'tsx', dimensions: { region, owner },
        excluded: 'additional region-host embedding outside the direct grid; standalone region is tested separately',
      };
}

module.exports = { core, region, t3, excludedRegionHosts, element, target };
