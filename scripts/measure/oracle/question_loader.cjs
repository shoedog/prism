'use strict';

const path = require('node:path');
const Module = require('node:module');
const vm = require('node:vm');
const { readBound, utf8 } = require('./bytes.cjs');

const deterministic = new Set(['assert', 'assert/strict', 'buffer', 'path', 'path/posix',
  'path/win32', 'querystring', 'string_decoder', 'url', 'util']);

function loadQuestion(spec, ts, checkerPath) {
  if (!spec) return null;
  const cache = new Map();
  // This loader enforces the reviewed CommonJS and declared-data interface.
  // The module and its helpers remain trusted code.
  function local(name, directory) {
    const filename = path.resolve(directory, name);
    const key = path.relative(spec.root, filename).split(path.sep).join('/');
    if (key.startsWith('../') || key === '..' || path.isAbsolute(key)) throw Error('question-closure: path escape');
    return key;
  }
  function fileAPI(directory) {
    return Object.freeze({
      readFileSync(name, encoding) {
        if (typeof name !== 'string') throw Error('question-closure: expected a data path');
        const key = local(name, directory);
        if (!Object.hasOwn(spec.data, key)) throw Error('question-closure: undeclared data read');
        const bytes = readBound(spec.root, key, spec.data[key]);
        return encoding == null ? bytes : bytes.toString(encoding);
      },
    });
  }
  function load(key) {
    if (!Object.hasOwn(spec.files, key) || !/\.(?:cjs|js|json)$/.test(key)) {
      throw Error('question-closure: unadmitted CommonJS member');
    }
    const filename = path.join(spec.root, key);
    const text = utf8.decode(readBound(spec.root, key, spec.files[key]));
    if (cache.has(key)) return cache.get(key).exports;
    const module = { exports: {} };
    cache.set(key, module);
    if (key.endsWith('.json')) {
      module.exports = JSON.parse(text);
      return module.exports;
    }
    const directory = path.dirname(filename);
    function boundRequire(request) {
      if (request === 'typescript' || request === checkerPath) return ts;
      if (typeof request !== 'string') throw Error('question-closure: expected a module name');
      const bare = request.replace(/^node:/, '');
      if (bare === 'fs') return fileAPI(directory);
      if (deterministic.has(bare)) return require('node:' + bare);
      if (!request.startsWith('./') && !request.startsWith('../') && !path.isAbsolute(request)) {
        throw Error('question-closure: external or ambient module');
      }
      // Resolve only over the admitted table, with no package or filesystem fallback.
      const candidate = local(request, directory);
      const match = [candidate, ...['.js', '.cjs', '.json'].map(ext => candidate + ext),
        ...['index.js', 'index.cjs', 'index.json'].map(file => candidate + '/' + file)]
        .find(name => Object.hasOwn(spec.files, name));
      if (!match) throw Error('question-closure: module is outside admitted file set');
      return load(match);
    }
    const wrapped = new vm.Script(Module.wrap(text.replace(/^#![^\n]*/, '')), { filename });
    const fn = wrapped.runInNewContext({ TextDecoder, TextEncoder, Buffer });
    fn.call(module.exports, module.exports, boundRequire, module, filename, directory);
    return module.exports;
  }
  const question = load(spec.entry);
  if (!question || !Array.isArray(question.relations) || !question.relations.length
      || new Set(question.relations).size !== question.relations.length
      || question.relations.some(rel => typeof rel !== 'string' || !rel.trim())
      || typeof question.grade !== 'function' ||
      Object.keys(question).some(key => !['relations', 'grade'].includes(key))) {
    throw Error('question-interface: expected unique relations and grade');
  }
  return question;
}

module.exports = { loadQuestion };
