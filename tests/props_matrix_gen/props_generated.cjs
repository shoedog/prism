'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const ts = require(path.resolve(process.env.PRISM_TS_PATH ||
  '/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'));
assert.equal(ts.version, '5.9.3');

const family = 'pattern-defaults';
const generator = 1;
const terminals = {
  plain: ['{box}', 'seed', 'box'],
  renamed: ['{box: local}', 'seed', 'local'],
  empty: ['{box: {}}', 'seed', null],
  array: ['{box: [item]}', 'seed', 'item'],
  computed: ['{box: {[key]: picked}}', '{item: seed}', 'picked'],
  rest: ['{box: {used, ...rest}}', '{used: seed, residual: extra}', 'rest'],
};
const neighbors = ['fresh', 'duplicate', 'excluded-array', 'rename-key', 'earlier-provider', 'later-binder'];
const defaults = { absent: '', inert: ' = {}', effectful: ' = effect()', identifier: ' = defaults' };

function materialize({ terminal, depth, neighbor, default: defaultKind }) {
  const [shallow, boxValue, designated] = terminals[terminal];
  if (designated === null && ['duplicate', 'excluded-array', 'rename-key'].includes(neighbor)) {
    return null;
  }
  const binding = depth === 1 ? shallow : `{outer: ${shallow}}`;
  const formal = binding + defaults[defaultKind];
  const box = `{box: ${boxValue}}`;
  const nested = depth === 1 ? box : `{outer: ${box}}`;
  const literal = nested.slice(0, -1) + ', keep: spare}';

  // The four competing-formal variants have a fixed earlier provider. The timing
  // variants put the provider immediately before or immediately after the pattern.
  const adjacent = {
    fresh: ['fresh', '0'],
    duplicate: [designated, '0'],
    'excluded-array': [`[${designated}]`, '[]'],
    'rename-key': [`{${designated}: renamedNeighbor}`, '{}'],
  };
  if (neighbor === 'earlier-provider') {
    return { formals: ['fresh', 'defaults', formal], args: ['0', '0', literal], position: 2, designated };
  }
  const [tail, tailArg] = neighbor === 'later-binder' ? ['defaults', '0'] : adjacent[neighbor];
  return {
    formals: [neighbor === 'later-binder' ? 'fresh' : 'defaults', formal, tail],
    args: ['0', literal, tailArg],
    position: 1,
    designated,
  };
}

function inspect(file, source) {
  const options = { allowJs: true, checkJs: true, noLib: true, noResolve: true };
  const tree = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true);
  const program = ts.createProgram([file], options, {
    getSourceFile: (name) => name === file ? tree : undefined,
    getDefaultLibFileName: () => '',
    writeFile: () => assert.fail('the generator must never emit TypeScript output'),
    getCurrentDirectory: () => '',
    getDirectories: () => [],
    fileExists: (name) => name === file,
    readFile: (name) => name === file ? source : undefined,
    getCanonicalFileName: (name) => name,
    useCaseSensitiveFileNames: () => true,
    getNewLine: () => '\n',
  });
  assert.deepEqual(program.getSyntacticDiagnostics(tree), []);
  const checker = program.getTypeChecker();
  const bytes = (offset) => Buffer.byteLength(source.slice(0, offset), 'utf8');
  const span = (node) => [bytes(node.getStart(tree)), bytes(node.end)];
  const line = (node) => tree.getLineAndCharacterOfPosition(node.getStart(tree)).line + 1;
  const functions = tree.statements.filter(ts.isFunctionDeclaration);
  return { checker, span, line, functions };
}

function boundNames(name, array = false) {
  if (ts.isIdentifier(name)) return [{ node: name, array }];
  assert(ts.isObjectBindingPattern(name) || ts.isArrayBindingPattern(name));
  return name.elements.filter(ts.isBindingElement)
    .flatMap((element) => boundNames(element.name, array || ts.isArrayBindingPattern(name)));
}

function patternState(binding, span) {
  const levels = [];
  const elements = [];
  const legacy = new Map();
  function walk(node, path, restLevel = null, legacyPath = path) {
    if (ts.isIdentifier(node)) {
      elements.push({ name: node.text, bytes: span(node), path, rest_level: restLevel });
      legacy.set(node, legacyPath);
      return;
    }
    if (ts.isArrayBindingPattern(node)) {
      for (const { node: local } of boundNames(node)) legacy.set(local, legacyPath);
      return;
    }
    assert(ts.isObjectBindingPattern(node));
    const levelIndex = levels.length;
    const level = { path, bytes: span(node), consumed: [], unknown_sibling: false };
    levels.push(level);
    for (const element of node.elements) {
      if (element.dotDotDotToken) {
        walk(element.name, path, levelIndex, legacyPath);
        continue;
      }
      const key = element.propertyName || element.name;
      if (ts.isIdentifier(key)) {
        level.consumed.push(key.text);
        walk(element.name, path?.concat(key.text) ?? null, null, legacyPath?.concat(key.text) ?? null);
      } else {
        assert(ts.isComputedPropertyName(key));
        level.unknown_sibling = true;
        walk(element.name, null, null, null);
      }
    }
    level.consumed = [...new Set(level.consumed)].sort();
  }
  walk(binding, []);
  return { levels, elements, legacy };
}

function applicableCell(cell, template) {
  const file = `case.${cell.grammar}`;
  let source = `// café\nconst key = 'item';\nfunction D(${template.formals.join(', ')}) {\n  return 0;\n}\n`;
  const parameters = inspect(file, source).functions[0].parameters;
  const names = [...new Set(parameters.flatMap((p) => boundNames(p.name).map(({ node }) => node.text)))];
  const reads = names.map((name) => `  see(${name});\n`).join('');
  source = source.replace('  return 0;', reads + '  return 0;') +
    `function App(seed, extra, spare) {\n  return D(${template.args.join(', ')});\n}\n`;
  const { checker, span, line, functions: [target, caller] } = inspect(file, source);
  const owner = (node) => ({
    name: node.name.text, bytes: span(node), line: line(node), end_line: line(node.body.statements.at(-1)) + 1,
  });
  const endpoint = (ownerNode, node, access, name = node.text, at = line(node)) =>
    ({ file, owner: ownerNode.name.text, owner_line: line(ownerNode), name, line: at, access, bytes: span(node) });
  const def = (node, name = node.text) => endpoint(target, node, 'def', name, line(target));
  const symbolDeclarations = (node) => checker.getSymbolAtLocation(node)?.declarations ?? [];
  const position = template.position;
  const formal = target.parameters[position];
  const wholeList = target.parameters.flatMap((parameter, index) =>
    boundNames(parameter.name).map(({ node, array }) => ({ node, array, index })));
  const bound = wholeList.map(({ node, array, index }) => ({ name: node.text, bytes: span(node), array, position: index }));
  const byName = (name) => wholeList.filter(({ node }) => node.text === name);
  const duplicates = [...new Set(bound.filter(({ name }) => byName(name).length > 1).map(({ name }) => name))].sort();
  assert.equal(duplicates.length > 0, ['duplicate', 'excluded-array'].includes(cell.dimensions.neighbor));
  const selected = wholeList.filter(({ index }) => index === position);
  const designated = selected.find(({ node }) => node.text === template.designated);
  assert.equal(designated === undefined, cell.dimensions.terminal === 'empty');
  const bodyReads = target.body.statements.slice(0, -1).map((stmt) => stmt.expression.arguments[0]);
  const refs = new Map(bodyReads.map((read) => [read.text, read]));
  for (const read of bodyReads) {
    const declarations = byName(read.text);
    assert(declarations.length > 0);
    if (declarations.length === 1) {
      const node = declarations[0].node;
      assert(symbolDeclarations(read).includes(node.parent));
    }
  }

  const init = formal.initializer;
  const provider = init && ts.isIdentifier(init) ? byName(init.text)[0] : null;
  if (provider) {
    assert(symbolDeclarations(init).includes(provider.node.parent));
    assert.notEqual(provider.index, position);
  }
  const alias = Boolean(provider && provider.index < position);
  const inert = !init || (ts.isObjectLiteralExpression(init) && init.properties.length === 0);
  const { levels, elements, legacy } = patternState(formal.name, span);
  const earlyError = duplicates.length !== 0;
  const pattern = {
    position, alias_skipped: alias, levels,
    elements: alias ? [] : elements.map((element) => ({
      ...element, defaulted: Boolean(init), admitted: inert && byName(element.name).length === 1,
    })),
  };
  const defs = [];
  const edges = [];
  for (const { node, array } of selected) {
    if (earlyError || !(alias || (inert && !array))) continue;
    const local = def(node);
    defs.push(local);
    edges.push({ from: local, to: endpoint(target, refs.get(node.text), 'use'), label: 'exact' });
    const legacyPath = legacy.get(node);
    if (alias && legacyPath !== null) {
      defs.push(def(node, [provider.node.text, ...legacyPath].join('.')));
    }
  }

  const call = caller.body.statements[0].expression;
  assert(ts.isCallExpression(call) && symbolDeclarations(call.expression).includes(target));
  const argument = call.arguments[position];
  const sources = [];
  function leaves(object, prefix) {
    assert(ts.isObjectLiteralExpression(object));
    for (const { name, initializer } of object.properties) {
      assert(ts.isIdentifier(name));
      const keyPath = [...prefix, name.text];
      if (ts.isObjectLiteralExpression(initializer)) {
        leaves(initializer, keyPath);
        continue;
      }
      assert(ts.isIdentifier(initializer));
      const binding = caller.parameters.find((p) => p.name.text === initializer.text);
      assert(symbolDeclarations(initializer).includes(binding));
      sources.push({
        path: keyPath,
        full: null,
        base: endpoint(caller, initializer, 'use'),
        incoming: { from: endpoint(caller, binding.name, 'def'), label: 'exact' },
      });
    }
  }
  leaves(argument, []);
  const literal = literalOutcome(
    argument, formal.name, span,
    (node) => endpoint(caller, node, 'use'),
    (name) => defs.find((candidate) => JSON.stringify(candidate.bytes) === JSON.stringify(span(name))),
    alias,
  );
  return {
    ...cell,
    files: { [file]: source },
    required_evidence: {
      parser_recovery: false,
      target: owner(target),
      caller: owner(caller),
      dfg_owner: earlyError ? null : target.name.text,
      duplicates,
      site: { bytes: span(call), argument: span(argument), resolution: 'Exact' },
      formal: { position, bytes: span(formal.name), designated: designated ? span(designated.node) : null },
      initializer: init ? span(init) : null,
      provider: provider ? span(provider.node) : null,
      bound,
      reads: selected.map(({ node }) => endpoint(target, refs.get(node.text), 'use')),
      sources,
    },
    expected_outcome: {
      disposition: earlyError ? 'early_error' : alias ? 'legacy_alias' : defs.length ? 'admitted' : 'no_defs',
      pattern,
      defs,
      edges,
      paths: edges.map(({ from, to }) => ({ from, to })),
      counts: { local_defs: defs.length, edges: edges.length, paths: edges.length, early_error: Number(earlyError) },
      literal,
    },
  };
}

function propsKey(key, span, pattern) {
  // Inspect the source spelling, since TS decodes strings and normalizes numbers.
  const spelling = key.getText();
  assert(span(key)[0] < span(key)[1]);
  if (/[^\x20-\x7e]|\\/.test(spelling) || ts.isComputedPropertyName(key)) return { kind: 'unknown' };
  if (ts.isNumericLiteral(key)) {
    const canonical = /^(0|[1-9]\d*)$/.test(spelling) && Number.isSafeInteger(Number(spelling));
    if (!canonical) return { kind: pattern ? 'unknown' : 'numeric' };
    return { kind: pattern ? 'named' : 'opaque-number', name: spelling };
  }
  return { kind: 'named', name: ts.isStringLiteral(key) ? spelling.slice(1, -1) : spelling };
}

function literalOutcome(argument, formal, span, sourceOf, defOf, alias = false, mutation) {
  const counters = { sites_literal: 1 };
  const add = (key, count = 1) => {
    if (count) counters[key] = (counters[key] || 0) + count;
  };
  const edges = [];
  if (alias) {
    add('refused_alias_pattern');
    return { counters, edges, survivors: [] };
  }
  const targets = [];
  function pattern(node, path) {
    const consumed = new Set();
    let unknown = false;
    const rests = [];
    for (const entry of node.elements) {
      if (entry.dotDotDotToken) {
        rests.push(entry.name);
        continue;
      }
      const key = propsKey(entry.propertyName || entry.name, span, true);
      if (key.kind === 'unknown' && mutation !== 'bind-unknown-target') {
        add('refused_key_unknown');
        unknown = true;
        continue;
      }
      const name = key.name ?? 'a';
      consumed.add(name);
      const local = [...path, name];
      if (ts.isObjectBindingPattern(entry.name)) pattern(entry.name, local);
      if (ts.isIdentifier(entry.name)) targets.push({ path: local, def: defOf(entry.name) });
    }
    for (const rest of rests) {
      if (unknown) add('refused_key_unknown');
      else targets.push({ path, def: defOf(rest), consumed });
    }
  }
  pattern(formal, []);
  function fold(object, path) {
    const current = new Map();
    for (const property of object.properties) {
      const spread = ts.isSpreadAssignment(property);
      if (spread) add('spread_sources_unbound');
      const key = spread ? { kind: 'unknown' } : propsKey(property.name, span, false);
      if (key.kind === 'unknown' || key.kind === 'numeric') {
        const matches = key.kind === 'unknown'
          ? () => mutation !== 'retain-wildcard-loser'
          : (name) => /^[0-9]|^Infinity$/.test(name);
        for (const [name, displaced] of current) {
          if (!matches(name)) continue;
          add('refused_overwritten', displaced.length);
          current.delete(name);
        }
      } else {
        add('refused_overwritten', current.get(key.name)?.length ?? 0);
        const keyPath = [...path, key.name];
        const value = property.initializer;
        let terminals;
        if (key.kind === 'opaque-number') terminals = [{ kind: 'numeric' }];
        else if (value && ts.isObjectLiteralExpression(value)) terminals = fold(value, keyPath);
        else if (value && ts.isIdentifier(value)) terminals = [{ kind: 'leaf', path: keyPath, from: sourceOf(value) }];
        else terminals = [{ kind: 'opaque' }];
        current.set(key.name, terminals);
      }
    }
    return [...current.values()].flat();
  }
  const prefix = (a, b) => a.length <= b.length && a.every((segment, i) => segment === b[i]);
  const survivors = [];
  for (const terminal of fold(argument, [])) {
    // A surviving numeric-key writer terminal supplies no leaf: counted as opaque.
    if (terminal.kind === 'opaque' || terminal.kind === 'numeric') add('refused_value_not_data');
    if (terminal.kind !== 'leaf') continue;
    const { path, from } = terminal;
    survivors.push({ path, from });
    const matching = targets.filter((target) => target.def && (target.consumed
      ? prefix(target.path, path) && target.path.length < path.length && !target.consumed.has(path[target.path.length])
      : prefix(target.path, path) || prefix(path, target.path)));
    if (!matching.length) add('refused_key_not_read');
    for (const target of matching) {
      add(target.consumed ? 'edges_to_rest' : 'edges_to_def');
      if (!target.consumed && target.path.length < path.length) add('edges_to_container');
      edges.push({ from, to: target.def, label: 'exact' });
    }
  }
  return { counters, edges, survivors };
}

const tokens = ['a', "'a'", "'1'", '1', '1e0', "'a.b'", '[k]', "'aria-label'", "'\\u0061'", 'café'];

function literalCell(family, grammar, dimensions, pattern, writes) {
  const file = `case.${grammar}`;
  const source = `const k = "dynamic";\nfunction D(${pattern}) {\n  return use(${dimensions.locals});\n}\n`
    + `function App(x, y, z, w, obj) {\n  return D({${writes}});\n}\n`;
  const { checker, span, line, functions: [target, caller] } = inspect(file, source);
  const formal = target.parameters[0].name;
  const call = caller.body.statements[0].expression;
  const argument = call.arguments[0];
  assert(checker.getSymbolAtLocation(call.expression).declarations.includes(target));
  const owner = (node) => ({
    name: node.name.text, bytes: span(node), line: line(node), end_line: line(node.body.statements.at(-1)) + 1,
  });
  const endpoint = (ownerNode, node, access, at = line(node)) => ({
    file, owner: ownerNode.name.text, owner_line: line(ownerNode), name: node.text, line: at, access, bytes: span(node),
  });
  const defs = boundNames(formal).map(({ node }) => {
    const read = target.body.statements[0].expression.arguments.find((candidate) => candidate.text === node.text);
    assert(checker.getSymbolAtLocation(read).declarations.includes(node.parent));
    return endpoint(target, node, 'def', line(target));
  });
  const sourceOf = (node) => {
    const parameter = caller.parameters.find((candidate) => candidate.name.text === node.text);
    assert(checker.getSymbolAtLocation(node).declarations.includes(parameter));
    return endpoint(caller, node, 'use');
  };
  const outcome = literalOutcome(argument, formal, span, sourceOf,
    (node) => defs.find((def) => def.name === node.text));
  const referenced = new Set();
  function collectReferences(node) {
    if (ts.isIdentifier(node)) {
      for (const parameter of checker.getSymbolAtLocation(node)?.declarations ?? []) {
        if (caller.parameters.includes(parameter)) referenced.add(parameter);
      }
    }
    ts.forEachChild(node, collectReferences);
  }
  collectReferences(argument);
  const sourceDefs = caller.parameters.filter((param) => referenced.has(param))
    .map(({ name }) => endpoint(caller, name, 'def'));
  const { locals, ...idDimensions } = dimensions;
  const id = [family, ...Object.values(idDimensions), grammar].join('/');
  return {
    id, family, grammar, dimensions: idDimensions, generator, typescript: ts.version, files: { [file]: source },
    required_evidence: {
      caller: owner(caller), target: owner(target), formal: span(formal),
      site: { bytes: span(call), argument: span(argument), resolution: 'Exact' }, defs, source_defs: sourceDefs,
    },
    expected_outcome: outcome,
  };
}

function literalCells() {
  const cells = [];
  for (const [s, source] of tokens.entries()) {
    for (const [t, target] of tokens.entries()) {
      for (const [w, writer] of tokens.entries()) {
        for (const order of ['forward', 'reverse']) {
          const writes = [`${source}: x`, `${writer}: y`];
          if (order === 'reverse') writes.reverse();
          for (const grammar of ['js', 'ts']) {
            cells.push(literalCell('literal-collision', grammar, { s, t, w, order, locals: 'v, rest' },
              `{${target}: v, ...rest}`, [...writes, 'keep: z'].join(', ')));
          }
        }
      }
    }
  }
  for (const [writer, replacement] of Object.entries({
    nonempty: 'box: {c: w}', empty: 'box: {}', opaque: 'box: make()', spread: '...obj',
    'inner-spread': '...obj', 'inner-computed': '[k]: w',
  })) {
    for (const order of ['forward', 'reverse']) {
      const inner = writer.startsWith('inner-');
      const writes = [inner ? 'a: x' : 'box: {a: x, b: y}', replacement];
      if (order === 'reverse') writes.reverse();
      if (inner) writes.splice(0, 2, `box: {${writes.join(', ')}, b: y}`);
      for (const grammar of ['jsx', 'tsx']) {
        cells.push(literalCell('literal-replacement', grammar,
          { writer, order, target: 'nested', locals: 'v, r, keep' },
          '{box: {a: v, ...r}, keep}', [...writes, 'keep: z'].join(', ')));
        cells.push(literalCell('literal-replacement', grammar,
          { writer, order, target: 'container', locals: 'v, r' },
          '{box: v, ...r}', [...writes, 'keep: z'].join(', ')));
      }
    }
  }
  assert.equal(cells.length, 4048);
  return cells;
}

function assertMatrixMutations() {
  const rows = fs.readFileSync(path.join(__dirname, '../fixtures/props_matrix/rows.jsonl'), 'utf8')
    .trimEnd().split('\n').map((row) => JSON.parse(row));
  for (const [id, mutation] of [
    ['d2-unknown-writer', 'retain-wildcard-loser'],
    ['d2-unknown-target', 'bind-unknown-target'],
  ]) {
    const row = rows.find((candidate) => candidate.id === id);
    for (const shape of ['escaped', 'nonascii', 'ascii']) {
      const source = row.files[`${shape}/m`];
      const { checker, span, functions: [target, caller] } = inspect('matrix.ts', source);
      const call = caller.body.statements[0].expression;
      assert(checker.getSymbolAtLocation(call.expression).declarations.includes(target));
      const model = (mutant) => literalOutcome(call.arguments[0], target.parameters[0].name,
        span, span, span, false, mutant).edges;
      const expected = model();
      if (shape === 'ascii') {
        assert.deepEqual(model(mutation), expected, `${id}/${shape}`);
        assert.equal(expected.length, 2);
      } else {
        assert.notDeepEqual(model(mutation), expected, `${id}/${shape}`);
        assert.equal(expected.length, id === 'd2-unknown-writer' ? 1 : 0);
      }
    }
  }
}

function generate() {
  assertMatrixMutations();
  const cells = [];
  for (const terminal of Object.keys(terminals)) {
    for (const depth of [1, 2]) {
      for (const neighbor of neighbors) {
        for (const defaultKind of Object.keys(defaults)) {
          for (const grammar of ['js', 'ts']) {
            const id = [family, terminal, depth, neighbor, defaultKind, grammar].join('/');
            const dimensions = { terminal, depth, neighbor, default: defaultKind };
            const cell = { id, family, dimensions, grammar, generator, typescript: ts.version };
            const template = materialize(dimensions);
            cells.push(template ? applicableCell(cell, template) : {
              ...cell,
              files: {},
              required_evidence: null,
              expected_outcome: null,
              excluded: `empty terminal has no designated bound name for ${neighbor}`,
            });
          }
        }
      }
    }
  }
  assert.equal(new Set(cells.map((cell) => cell.id)).size, 576);
  assert.equal(cells.filter((cell) => !cell.excluded).length, 528);
  cells.push(...literalCells());
  cells.sort((a, b) => (a.id < b.id ? -1 : Number(a.id > b.id)));
  assert.equal(new Set(cells.map((cell) => cell.id)).size, 4624);
  return Buffer.from(cells.map((cell) => JSON.stringify(cell)).join('\n') + '\n');
}

const output = path.join(__dirname, 'generated-cases.jsonl');
const generated = generate();
const args = process.argv.slice(2);
if (args.length === 1 && args[0] === '--check') {
  assert(fs.readFileSync(output).equals(generated), `${output} differs from TypeScript ${ts.version} regeneration`);
  console.log(`Generator v${generator}, TypeScript ${ts.version}: 4624 deterministic cells, 4576 applicable, 48 excluded`);
} else {
  assert.deepEqual(args, []);
  fs.writeFileSync(output, generated);
}
