'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { createHash } = require('node:crypto');

const sourceExtension = /\.(?:ts|tsx|mts|cts|js|jsx|mjs|cjs)$/;
const utf8 = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true });
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const has = (object, key) => Object.hasOwn(object, key);

function readBound(root, key, digest) {
  const file = path.join(root, key);
  if (fs.realpathSync(file) !== file || !fs.lstatSync(file).isFile()) {
    throw Error('bound-read: path is no longer a regular closure member');
  }
  const bytes = fs.readFileSync(file);
  if (sha256(bytes) !== digest) throw Error('bound-read: loaded bytes differ from admitted digest');
  return bytes;
}

function byteMap(text) {
  const bytes = new Int32Array(Buffer.byteLength(text) + 1).fill(-1);
  const units = new Int32Array(text.length + 1).fill(-1);
  const lines = [0];
  let byte = 0;
  let unit = 0;
  for (const ch of text) {
    bytes[byte] = unit;
    units[unit] = byte;
    byte += Buffer.byteLength(ch);
    unit += ch.length;
    if (ch === '\n') lines.push(unit);
  }
  bytes[byte] = unit;
  units[unit] = byte;
  return {
    position: offset => bytes[offset],
    byte: offset => units[offset],
    line(position) {
      let lo = 0;
      let hi = lines.length;
      while (lo + 1 < hi) {
        const mid = lo + ((hi - lo) >> 1);
        if (lines[mid] > position) hi = mid;
        else lo = mid;
      }
      return lo + 1;
    },
  };
}

function isIdentifier(ts, node) {
  return node && (ts.isIdentifier(node) || ts.isPrivateIdentifier(node));
}

function isCall(ts, node) {
  return ts.isCallExpression(node) || ts.isNewExpression(node);
}

function headerStart(ts, node) {
  const decorators = ts.canHaveDecorators(node) && ts.getDecorators(node);
  if (!decorators?.length) return node.getStart();
  const scanner = ts.createScanner(ts.ScriptTarget.ES2022, true, ts.LanguageVariant.Standard,
    node.getSourceFile().text, undefined, decorators[decorators.length - 1].end);
  scanner.scan();
  return scanner.getTokenPos();
}

function inside(root, file) {
  const relative = path.relative(root, file);
  return relative && relative !== '..' && !relative.startsWith('../') && !path.isAbsolute(relative)
    ? relative : null;
}

function buildProgram(ts, source, pins, noLib, readSource) {
  const options = {
    target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS,
    moduleResolution: ts.ModuleResolutionKind.Node10, jsx: ts.JsxEmit.Preserve,
    allowJs: true, checkJs: true, noEmit: true, skipLibCheck: true, strict: true, noLib, types: [],
  };
  const names = Object.keys(source.files).filter(file => sourceExtension.test(file))
    .sort((a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b)));
  const exists = new Set(names);
  const directories = new Set([source.root]);
  for (const file of names) {
    let parent = path.dirname(path.join(source.root, file));
    while (!directories.has(parent)) {
      directories.add(parent);
      parent = path.dirname(parent);
    }
  }
  const libRoot = path.join(pins.root, 'lib');
  const lib = file => {
    const key = inside(pins.root, file);
    return !noLib && key && /^lib\/lib(?:\.[^/]+)?\.d\.ts$/.test(key) && has(pins.files, key) ? key : null;
  };
  const local = file => inside(source.root, file);
  const host = {
    useCaseSensitiveFileNames: () => true, getCanonicalFileName: name => name,
    getCurrentDirectory: () => source.root, getNewLine: () => '\n',
    getDefaultLibLocation: () => libRoot,
    getDefaultLibFileName: opts => path.join(libRoot, ts.getDefaultLibFileName(opts)),
    directoryExists: directory => directories.has(directory) || !noLib && directory === libRoot,
    fileExists: file => exists.has(local(file)) || Boolean(lib(file)),
    readFile(file) {
      const localName = local(file);
      const libName = lib(file);
      if (exists.has(localName)) return readSource(localName);
      if (libName) return utf8.decode(readBound(pins.root, libName, pins.files[libName]));
      return undefined;
    },
    getSourceFile(file, languageVersion) {
      const text = host.readFile(file);
      return text === undefined ? undefined : ts.createSourceFile(file, text, languageVersion, true);
    },
    resolveModuleNames(specifiers, containingFile) {
      return specifiers.map(specifier => {
        if (!specifier.startsWith('./') && !specifier.startsWith('../')) return undefined;
        return ts.resolveModuleName(specifier, containingFile, options, host).resolvedModule;
      });
    },
  };
  const program = ts.createProgram(names.map(file => path.join(source.root, file)), options, host);
  return { program, checker: program.getTypeChecker() };
}

function buildContext(ts, source, pins) {
  const maps = new Map();
  const texts = new Map();
  const readSource = file => {
    if (!texts.has(file)) {
      const bytes = readBound(source.root, file, source.files[file]);
      try {
        texts.set(file, utf8.decode(bytes));
      } catch (error) {
        if (!(error instanceof TypeError)) throw error;
        texts.set(file, undefined);
      }
    }
    return texts.get(file);
  };
  const primary = buildProgram(ts, source, pins, true, readSource);
  let withLib;
  const defaultLib = () => (withLib ??= buildProgram(ts, source, pins, false, readSource));
  function graph(program) {
    if (program === primary.program) return primary;
    if (program === withLib?.program) return withLib;
    throw Error('program-identity: foreign program');
  }
  function sourceFile(program, file) {
    graph(program);
    return has(source.files, file) ? program.getSourceFile(path.join(source.root, file)) : undefined;
  }
  function map(file) {
    if (!maps.has(file)) {
      const text = readSource(file);
      maps.set(file, text === undefined ? null : byteMap(text));
    }
    return maps.get(file);
  }
  function interval(file, start, end) {
    if (!has(source.files, file) || !Number.isSafeInteger(start) || !Number.isSafeInteger(end)
        || start < 0 || end < start) return null;
    const positions = map(file);
    const a = positions?.position(start);
    const b = positions?.position(end);
    return a >= 0 && b >= 0 ? [a, b] : null;
  }
  function lookup(program, file, start, end, match, exact) {
    const sf = sourceFile(program, file);
    const range = interval(file, start, end);
    if (!sf || !range) return null;
    const [a, b] = range;
    let found = null;
    let depthFound = -1;
    let ambiguous = false;
    function visit(node, depth) {
      const lo = node.getStart(sf);
      if (lo > a || node.end < b) return;
      if (match(node) && (!exact || lo === a && node.end === b)) {
        if (depth > depthFound) {
          found = node;
          depthFound = depth;
          ambiguous = false;
        } else if (depth === depthFound) {
          ambiguous = true;
        }
      }
      ts.forEachChild(node, child => { visit(child, depth + 1); });
    }
    visit(sf, 0);
    return ambiguous ? null : found;
  }
  function identifier(file, start, end, program = primary.program) {
    return lookup(program, file, start, end, node => isIdentifier(ts, node), true);
  }
  function nodeAt(program, file, start, end) {
    return lookup(program, file, start, end, () => true, true);
  }
  function enclosingCall(file, start, end, program = primary.program) {
    return lookup(program, file, start, end, node => isCall(ts, node), true)
      ?? lookup(program, file, start, end, node => isCall(ts, node), false);
  }
  function declarations(node, program = primary.program) {
    const { checker } = graph(program);
    if (!node || program.getSourceFile(node.getSourceFile().fileName) !== node.getSourceFile()) {
      throw Error('program-identity: node belongs to a different program');
    }
    const result = new Set(checker.getSymbolAtLocation(node)?.declarations ?? []);
    return [...result].sort((a, b) => Buffer.compare(Buffer.from(a.getSourceFile().fileName),
      Buffer.from(b.getSourceFile().fileName)) || a.getStart() - b.getStart() || a.end - b.end);
  }
  function span(node) {
    const file = inside(source.root, node.getSourceFile().fileName);
    if (!file || !has(source.files, file)) return null;
    const positions = map(file);
    return { file, start_byte: positions.byte(node.getStart()), end_byte: positions.byte(node.end) };
  }
  function line(node, position = node.getStart()) {
    const file = inside(source.root, node.getSourceFile().fileName);
    return map(file).line(position);
  }
  function coordinate(point, formal = false) {
    const { file, start_byte: start, end_byte: end } = point;
    const range = interval(file, start, end);
    if (!range) return 'invalid-byte-span';
    if (point.line === map(file).line(range[0])) return null;
    if (formal) {
      const id = identifier(file, start, end);
      if (id && ts.isParameter(id.parent) && id.parent.name === id
          && point.line === line(id, headerStart(ts, id.parent.parent))) return null;
    }
    return 'source-line-mismatch';
  }
  return {
    public: { ts, ...primary, identifier_at_bytes: identifier, declarations, enclosing_call: enclosingCall,
      node_at_bytes: nodeAt, program_with_default_lib: defaultLib },
    sourceFile: file => sourceFile(primary.program, file),
    span, line, coordinate, interval,
  };
}

module.exports = { buildContext, byteMap, headerStart, inside, isCall, isIdentifier, readBound,
  sha256, sourceExtension, utf8 };
