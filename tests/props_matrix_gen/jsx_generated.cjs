'use strict';

const assert = require('node:assert/strict');
const contexts = require('./jsx_contexts.cjs');

function* replacement() {
  const writers = {
    nonempty: 'box: {c: w}', empty: 'box: {}', opaque: 'box: make()', spread: '...obj',
    'inner-spread': '...obj', 'inner-computed': '[k]: w',
  };
  for (const [writer, text] of Object.entries(writers))
    for (const order of ['forward', 'reverse'])
      for (const target of ['nested', 'container'])
        for (const grammar of ['jsx', 'tsx']) {
          const inner = writer.startsWith('inner-');
          const writes = [inner ? 'a: x' : 'box: {a: x, b: y}', text];
          if (order === 'reverse') writes.reverse();
          if (inner) writes.splice(0, 2, `box: {${writes.join(', ')}, b: y}`);
          const pattern = target === 'nested' ? 'box: {a: v, ...r}, keep' : 'box: v, ...r';
          const locals = target === 'nested' ? 'v, r, keep' : 'v, r';
          const source = 'const k = "dynamic";\n'
            + `function D({bag: {${pattern}}}) {\n  return use(${locals});\n}\n`
            + `function App(x, y, z, w, obj) {\n  return <D bag={{${writes.join(', ')}, keep: z}}/>;\n}\n`;
          yield { grammar, dimensions: { writer, order, target }, source, writerTree: true };
        }
}

function* attributeNames() {
  const tokens = ['a', 'data-id', 'xlink:href', 'café', '__proto__', 'key', 'ref', 'Key', 'children', 'spread'];
  for (const token of tokens)
    for (const tag of ['self', 'child'])
      for (const order of ['before', 'after'])
        for (const grammar of ['jsx', 'tsx']) {
          const pair = [token === 'spread' ? '{...o}' : `${token}={x}`, 'a={other}'];
          if (order === 'after') pair.reverse();
          const tail = tag === 'self' ? '/>' : '>{child}</D>';
          const source = "function D({a, 'data-id': hyphen, café: unknownKey, Key, children, keep}) {\n"
            + '  return use(a, hyphen, unknownKey, Key, children, keep);\n}\n'
            + `function App(x, other, z, o, child) {\n  return <D ${pair.join(' ')} keep={z}${tail};\n}\n`;
          yield { grammar, dimensions: { token, tag, order }, source, names: true };
        }
}

function* multiline() {
  const target = contexts.target;
  const sources = {
    full: 'function App(x, y) {\n  const box = make();\n  box.value = x;\n  see(box); return SLOT;\n}\n',
    base: 'function App(x, y) {\n  const box = make();\n  observe(y);\n  see(box); return SLOT;\n}\n',
    shadow: 'function App(y) {\n  const box = make();\n  see(box); const out = SLOT; { const box = fresh(); box.value = y; see(box.value); }\n  return out;\n}\n',
    version: 'function App(y) {\n  const box = make();\n  see(box); const out = SLOT; box.value = y; see(box.value);\n  return out;\n}\n',
  };
  sources['shadow-moved'] = sources.shadow.replace('; { const', ';\n  { const');
  sources['version-moved'] = sources.version.replace('; box.value', ';\n  box.value');
  for (const [program, template] of Object.entries(sources))
    for (const operand of ['two-line', 'three-line'])
      for (const grammar of ['jsx', 'tsx']) {
        const value = operand === 'two-line' ? 'box\n    .value' : 'box\n    .\n    value';
        const source = target + template.replace('SLOT', contexts.element('self', value));
        const conflict = ['shadow', 'version'].includes(program) ? program : null;
        yield { grammar, dimensions: { program, operand }, source, multiline: true, conflict,
          refusal: conflict ? 'refused_source_ambiguous' : null };
      }
}

function generate({ ts, inspect, boundNames, literalOutcome }) {
  const prefix = 'jsx-';
  function cell(familyName, input) {
    const { grammar, dimensions, source, excluded } = input;
    const family = prefix + familyName;
    const id = [family, ...Object.values(dimensions), grammar].join('/');
    const header = { id, family, dimensions, grammar, generator: 1, typescript: ts.version };
    if (excluded) return { ...header, files: {}, required_evidence: null, expected_outcome: null, excluded };

    const file = `case.${grammar}`;
    const { tree, checker, span, line } = inspect(file, source);
    const all = [];
    function visit(node) {
      all.push(node);
      ts.forEachChild(node, visit);
    }
    visit(tree);
    const site = all.find(node => (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node))
      && node.tagName.getText() === 'D');
    assert(site, id);
    const targetBinding = checker.getSymbolAtLocation(site.tagName).valueDeclaration;
    const target = ts.isVariableDeclaration(targetBinding) ? targetBinding.initializer : targetBinding;
    assert(ts.isFunctionLike(target), id);
    const formal = target.parameters[0].name;
    const nameOf = node => {
      if (ts.isConstructorDeclaration(node)) return 'constructor';
      if (node.name) return node.name.getText();
      if (ts.isVariableDeclaration(node.parent) || ts.isPropertyDeclaration(node.parent)) {
        return node.parent.name.getText();
      }
      const start = node.getStart(tree);
      const { character } = tree.getLineAndCharacterOfPosition(start);
      return `<anon@${line(node)}:${character + 1}>`;
    };
    const owner = node => node ? { name: nameOf(node), bytes: span(node), line: line(node),
      end_line: tree.getLineAndCharacterOfPosition(node.end).line + 1 } : null;
    function executingOwner(node) {
      for (let n = node.parent; n; n = n.parent) {
        if (ts.isFunctionLike(n) && !(
          n.name && n.name.getStart(tree) <= node.getStart(tree) && node.end <= n.name.end
        )) return n;
      }
      return null;
    }
    const caller = executingOwner(site);
    const endpoint = (node, parent, access, name = node.getText(), at = line(node)) => ({
      file, owner: nameOf(parent), owner_line: line(parent), name, line: at, access, bytes: span(node),
    });
    const targetDef = node => endpoint(node, target, 'def', node.getText(), line(target));
    const targetEntries = boundNames(formal);
    const targetReads = new Map(targetEntries.map(({ node }) => {
      const read = target.body.statements[0].expression.arguments.find(arg => arg.getText() === node.getText());
      assert(read && checker.getSymbolAtLocation(read) === checker.getSymbolAtLocation(node), id);
      return [node, endpoint(read, target, 'use')];
    }));
    const sourceDef = (value) => {
      const declaration = checker.getSymbolAtLocation(value)?.valueDeclaration;
      assert(declaration?.name && ts.isIdentifier(declaration.name), `${id}: ${value.getText()}`);
      const parent = executingOwner(declaration.name);
      return { declaration, def: parent ? endpoint(declaration.name, parent, 'def') : null };
    };
    const sourceInputs = new Map();
    const operands = [];
    // Named statement and stored concise-arrow producers can materialize a
    // line-start anchor when the operand is a free identifier in the slot.
    const anchored = input.multiline !== true && !input.writerTree && !input.names && (
      (familyName === 'core' && dimensions.owner === 'stored' && dimensions.layout === 'own'
        && !dimensions.route.split('+').includes('call'))
      || (familyName === 'region' && ['generator', 'async-generator', 'declarator', 'field-callable'].includes(dimensions.region)
        && (dimensions.layout === 'own' || dimensions.operand === 'multiline'))
      || (familyName === 't3' && dimensions.layout === 'own' && (
        (dimensions.panel === 'statement' && ['declaration', 'method', 'stored-block'].includes(dimensions.owner)
          && !['return', 'assignment'].includes(dimensions.slot))
        || (dimensions.owner === 'stored-expression' && (dimensions.panel === 'body'
          || (dimensions.panel === 'expression' && !['call', 'optional-call', 'assignment'].includes(dimensions.slot))))
      ))
    );
    function markSource(value, earlier = false) {
      const { declaration, def } = sourceDef(value);
      const read = earlier ? all.find(node => ts.isIdentifier(node) && node.getText() === value.getText()
        && ts.isCallExpression(node.parent) && node.parent.expression.getText() === 'see'
        && checker.getSymbolAtLocation(node) === checker.getSymbolAtLocation(value)
        && node.getStart(tree) < value.getStart(tree) && line(node) === line(value)) : value;
      const use = caller && read ? endpoint(read, caller, 'use') : null;
      if (anchored && use) {
        const { line: row } = tree.getLineAndCharacterOfPosition(read.getStart(tree));
        const start = Buffer.byteLength(source.slice(0, tree.getPositionOfLineAndCharacter(row, 0)), 'utf8');
        use.bytes = [start, start];
      }
      operands.push({ bytes: span(value), binding: span(declaration.name), use });
      if (use && def) sourceInputs.set(JSON.stringify(use), def);
      return use;
    }
    const jsxDef = node => /[^\x20-\x7e]/.test((node.parent.propertyName ?? node).getText())
      ? null : targetDef(node);

    const outcome = { counters: {}, edges: [], paths: [] };
    const registered = input.registered !== false && caller !== null;
    if (registered) outcome.counters.sites_jsx = 1;
    if (registered && input.refusal) outcome.counters[input.refusal] = 1;
    const attrs = site.attributes.properties;
    let replay = null;
    let multilineLookup = null;
    if (input.writerTree) {
      const model = literalOutcome(attrs[0].initializer.expression, formal.elements[0].name,
        span, markSource, targetDef);
      Object.assign(outcome, model);
      delete outcome.counters.sites_literal;
      outcome.counters.sites_jsx = 1;
    } else if (input.names) {
      const { token, tag } = dimensions;
      const winners = new Map();
      const add = count => { outcome.counters[count] = (outcome.counters[count] ?? 0) + 1; };
      const assign = (key, value) => {
        if (winners.has(key)) add('refused_overwritten');
        winners.set(key, value);
      };
      for (const attr of attrs) {
        const key = ts.isJsxSpreadAttribute(attr) ? 'spread' : attr.name.getText();
        if (key === 'key' || key === 'ref') continue;
        if (key === 'spread' || key === 'café') {
          if (key === 'spread') add('spread_sources_unbound');
          for (const _ of winners) add('refused_overwritten');
          winners.clear();
          continue;
        }
        assign(key, ['__proto__', 'xlink:href'].includes(key) ? null : markSource(attr.initializer.expression));
      }
      if (tag === 'child') assign('children', null);
      add('refused_key_unknown'); // The non-ASCII target entry stays unknown even if the source differs.
      for (const [key, from] of winners) {
        const entry = formal.elements.find(node => (node.propertyName ?? node.name).getText().replaceAll("'", '') === key);
        if (!from) add('refused_value_not_data');
        else if (!entry || !jsxDef(entry.name)) add('refused_key_not_read');
        else {
          outcome.edges.push({ from, to: jsxDef(entry.name), label: 'exact' });
          add('edges_to_def');
        }
      }
      if (token === 'café') {
        assert(!outcome.edges.some(edge => edge.to.name === 'unknownKey'), id);
        assert.equal(outcome.edges.some(edge => edge.from.name === 'other'), dimensions.order === 'before', id);
      }
    } else if (input.multiline) {
      const member = attrs[0].initializer.expression;
      const { def, declaration } = sourceDef(member.expression);
      const baseRead = all.find(node => ts.isIdentifier(node) && node.getText() === 'box'
        && ts.isCallExpression(node.parent) && node.parent.expression.getText() === 'see');
      const base = endpoint(baseRead, caller, 'use');
      const fieldRead = all.find(node => ts.isPropertyAccessExpression(node)
        && ts.isCallExpression(node.parent) && node.parent.expression.getText() === 'see');
      const fieldWrite = all.find(node => ts.isBinaryExpression(node) && node.left.getText() === 'box.value');
      const dest = targetDef(formal.elements[0].name);
      const binding = span(declaration.name);
      const { program } = dimensions;
      const rootLine = line(member.expression);
      const lines = Array.from({ length: line(member.name) - rootLine + 1 }, (_, offset) => rootLine + offset);
      const fullKey = endpoint(member, caller, 'use', 'box.value', rootLine);
      multilineLookup = {
        lines, base, full_key: fullKey, full: input.conflict ? endpoint(fieldRead, caller, 'use')
          : ['full', 'base'].includes(program) ? fullKey : null,
        full_disconnected: program === 'base',
      };
      operands.push({ bytes: span(member), binding, use: base });
      if (input.conflict) {
        const { declaration: fullDeclaration } = sourceDef(fieldRead.expression);
        replay = {
          lines, root_line: rootLine, root: span(member.expression), base, def, dest,
          full: endpoint(fieldRead, caller, 'use'), binding, full_binding: span(fullDeclaration.name),
          write: span(fieldWrite), conflict: input.conflict,
        };
        assert.equal(fullDeclaration === declaration, input.conflict === 'version', id);
      } else {
        const from = program === 'full'
          ? fullKey : base;
        sourceInputs.set(JSON.stringify(from), program === 'full'
          ? endpoint(fieldWrite.left, caller, 'def', 'box.value') : def);
        outcome.edges.push({ from, to: dest, label: 'exact' });
        outcome.counters.edges_to_def = 1;
      }
    } else {
      const value = attrs[0].initializer.expression;
      const from = markSource(value, input.earlier);
      if (registered && !input.refusal) {
        assert(from, id);
        outcome.edges.push({ from, to: targetDef(formal.elements[0].name), label: 'exact' });
        outcome.counters.edges_to_def = 1;
      }
    }

    for (const edge of outcome.edges) {
      const def = sourceInputs.get(JSON.stringify(edge.from));
      const local = targetEntries.find(({ node }) => node.getText() === edge.to.name).node;
      assert(def, id);
      outcome.paths.push({ from: def, via: edge.from, to: edge.to, body: targetReads.get(local) });
    }
    const syntax = ts.isJsxOpeningElement(site) ? site.parent : site;
    const incomplete = (familyName === 'region'
        && ['assignment', 'computed-name', 'decorator', 'heritage'].includes(dimensions.region))
      || (familyName === 't3' && dimensions.panel === 'signature' && dimensions.owner.endsWith('-block'));
    const sourceLabels = incomplete ? ['cfg_incomplete'] : ['exact', 'same_line'];
    return { ...header, files: { [file]: source }, required_evidence: {
      site: { bytes: span(site), expression: span(syntax), parent: span(syntax.parent),
        parent_kind: ts.SyntaxKind[syntax.parent.kind], registered },
      caller: owner(caller), target: owner(target), formal: span(formal),
      defs: targetEntries.map(({ node }) => targetDef(node)), operands, replay,
      multiline_lookup: multilineLookup,
      local_links: outcome.paths.map(link => ({ ...link, source_labels: sourceLabels })),
    }, expected_outcome: outcome };
  }

  return [
    ['core', contexts.core(), 3840, 0],
    ['region', contexts.region(), 176, 0],
    ['multiline', multiline(), 24, 0],
    ['replacement', replacement(), 48, 0],
    ['names', attributeNames(), 80, 0],
    ['t3', contexts.t3(), 1044, 268],
    ['region-host', contexts.excludedRegionHosts(), 0, 176],
  ].flatMap(([family, inputs, applicable, excluded]) => {
    const records = [...inputs].map(input => {
      try {
        return cell(family, input);
      } catch (cause) {
        throw new Error(`${family}/${JSON.stringify(input.dimensions)}\n${input.source}`, { cause });
      }
    });
    assert.equal(records.filter(record => record.excluded).length, excluded, family);
    assert.equal(records.length - excluded, applicable, family);
    return records;
  });
}

module.exports = generate;
