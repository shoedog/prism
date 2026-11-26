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
    },
  };
}

function generate() {
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
  cells.sort((a, b) => (a.id < b.id ? -1 : Number(a.id > b.id)));
  assert.equal(new Set(cells.map((cell) => cell.id)).size, 576);
  assert.equal(cells.filter((cell) => !cell.excluded).length, 528);
  return Buffer.from(cells.map((cell) => JSON.stringify(cell)).join('\n') + '\n');
}

const output = path.join(__dirname, 'generated-cases.jsonl');
const generated = generate();
const args = process.argv.slice(2);
if (args.length === 1 && args[0] === '--check') {
  assert(fs.readFileSync(output).equals(generated), `${output} differs from TypeScript ${ts.version} regeneration`);
  console.log(`Generator v${generator}, TypeScript ${ts.version}: 576 deterministic cells, 528 applicable, 48 excluded`);
} else {
  assert.deepEqual(args, []);
  fs.writeFileSync(output, generated);
}
