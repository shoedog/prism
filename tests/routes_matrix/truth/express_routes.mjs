/** Observe Express 4 registration calls and its native router stack. */
import { readFileSync, readdirSync, realpathSync } from 'node:fs';
import { createRequire, isBuiltin, registerHooks, stripTypeScriptTypes } from 'node:module';
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = realpathSync(process.argv[2]);
const dependencies = join(dirname(root), 'node_modules');
const require = createRequire(import.meta.url);
const extensions = new Set(['.js', '.mjs', '.cjs', '.ts', '.mts', '.cts']);
const stringifyFunction = Function.prototype.toString;
const protocol = console.log.bind(console);
const report = console.error.bind(console);
console.log = console.info = console.debug = report;

class HandRequired extends Error {}
class EnvironmentError extends Error {}

function within(path, base) {
  const name = relative(base, path);
  return name !== '..' && !name.startsWith('..' + sep) && !isAbsolute(name);
}

function dependencyName(specifier) {
  const parts = specifier.split('/');
  return parts.slice(0, specifier.startsWith('@') ? 2 : 1).join('/');
}

function verifySpecifier(specifier, file, packages) {
  if (typeof specifier !== 'string') {
    throw new Error(`${file}: import or require needs a string literal`);
  }
  if (specifier === '.' || specifier === '..' || specifier.startsWith('./') || specifier.startsWith('../')) {
    if (!within(resolve(dirname(file), specifier), root)) {
      throw new Error(`${file}: relative import escapes row: ${specifier}`);
    }
  } else if (!isBuiltin(specifier) && !packages.has(dependencyName(specifier))) {
    throw new Error(`${file}: undeclared JS import: ${specifier}`);
  }
}

function readSources() {
  // Node 24 ships the parser used for its own module detection. The pinned
  // Express dependency tree has no JavaScript parser to add to the harness.
  const { parse } = require('internal/deps/acorn/acorn/dist/acorn');
  const { fullAncestor } = require('internal/deps/acorn/acorn-walk/dist/walk');
  const files = new Map();
  const functions = new Map();
  const packages = new Set(readdirSync(dependencies).filter(name => !name.startsWith('.')));
  for (const scope of [...packages].filter(name => name.startsWith('@'))) {
    packages.delete(scope);
    for (const child of readdirSync(join(dependencies, scope))) packages.add(`${scope}/${child}`);
  }
  for (const name of readdirSync(root, { recursive: true })) {
    const absolute = join(root, name);
    const ext = extname(absolute);
    if (!extensions.has(ext)) continue;
    const source = readFileSync(absolute, 'utf8');
    const parsed = ext.endsWith('ts') ? stripTypeScriptTypes(source) : source;
    const options = { ecmaVersion: 'latest', locations: true, allowHashBang: true };
    let tree;
    try {
      tree = parse(parsed, { ...options, sourceType: 'module' });
    } catch {
      // CommonJS Node wraps files in a function, where top-level return is legal.
      tree = parse(parsed, { ...options, sourceType: 'script', allowReturnOutsideFunction: true });
    }
    const calls = [];
    files.set(absolute, { source, calls });
    fullAncestor(tree, (node, _state, ancestors) => {
      const parent = ancestors.at(-2);
      const staticSpecifier = ['ImportDeclaration', 'ExportNamedDeclaration', 'ExportAllDeclaration'];
      if (staticSpecifier.includes(node.type) && node.source) {
        verifySpecifier(node.source.value, absolute, packages);
      } else if (node.type === 'ImportExpression') {
        verifySpecifier(node.source.value, absolute, packages);
      } else if (node.type === 'CallExpression') {
        calls.push(node);
        const callee = node.callee;
        const directRequire = callee.type === 'Identifier' && callee.name === 'require';
        const moduleRequire = callee.type === 'MemberExpression' && callee.object.name === 'module' &&
          (callee.property.name === 'require' || callee.property.value === 'require');
        if (directRequire || moduleRequire) {
          verifySpecifier(node.arguments[0]?.value, absolute, packages);
        }
      }
      if (!['FunctionDeclaration', 'FunctionExpression', 'ArrowFunctionExpression'].includes(node.type)) return;
      const method = node === parent?.value && (parent.method || parent.type === 'MethodDefinition');
      const span = method ? parent : node;
      const name = method ? parent.key.name ?? parent.key.value : node.id?.name;
      const { line, column } = span.loc.start;
      const byteColumn = Buffer.byteLength(source.slice(span.start - column, span.start)) + 1;
      const identity = {
        kind: 'javascript', file: nameFor(absolute), name: name == null ? `<anon@${line}:${byteColumn}>` : String(name),
        first_line: line, column: byteColumn, end_line: span.loc.end.line, anonymous: name == null,
      };
      for (const text of new Set([source.slice(span.start, span.end), parsed.slice(span.start, span.end)])) {
        if (!functions.has(text)) functions.set(text, []);
        functions.get(text).push(identity);
      }
    });
  }
  return { files, functions, packages };
}

function nameFor(path) {
  return relative(root, path);
}

function installResolutionGuard(packages) {
  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (context.parentURL?.startsWith('file:')) {
        const parent = fileURLToPath(context.parentURL);
        if (within(parent, root)) verifySpecifier(specifier, parent, packages);
      }
      const resolution = nextResolve(specifier, context);
      if (resolution.url.startsWith('node:')) return resolution;
      if (resolution.url.startsWith('file:')) {
        const target = realpathSync(fileURLToPath(resolution.url));
        if (within(target, root) || within(target, dependencies)) return resolution;
      }
      throw new Error(`JS import resolved outside row or pinned dependencies: ${specifier}`);
    },
  });
}

function encodePath(path) {
  if (path instanceof RegExp) return { regexp: path.source, flags: path.flags };
  if (Array.isArray(path)) return path.map(encodePath);
  return path;
}

function literalPrefix(layer, input) {
  const { regexp, keys } = layer;
  if (typeof input !== 'string' || keys.length || regexp.fast_star || !['', 'i'].includes(regexp.flags)) {
    throw new HandRequired('Express mount prefix is outside the literal-prefix pattern');
  }
  if (regexp.fast_slash && regexp.source === String.raw`^\/?(?=\/|$)`) return '';
  // path-to-regexp 0.1.x compiles non-ending, non-strict literal paths to
  // ^\/prefix\/?(?=\/|$). Dots and separators are escaped in RegExp.source.
  const ending = String.raw`\/?(?=\/|$)`;
  const source = regexp.source;
  const prefix = source.endsWith(ending) ? source.slice(1, -ending.length) : '';
  if (!source.startsWith('^') || !/^(?:\\\/)(?:[a-zA-Z0-9_~%/@-]|\\[./])+$/.test(prefix)) {
    throw new HandRequired('Express mount prefix is outside the literal-prefix pattern');
  }
  return prefix.replace(/\\([./])/g, '$1');
}

function installExpress(express, sources) {
  const httpMethods = require('methods');
  const calls = [];
  const ordinals = new Map();
  const routeLayers = new WeakMap();
  const builders = new WeakMap();
  const handlerLayers = new WeakMap();
  const useLayers = new Map();
  const bootstrapLayers = new WeakSet();
  const routerPrototype = express.Router;
  const appPrototype = express.application;
  let active = null;
  let initialization = 0;

  function site() {
    const prepare = Error.prepareStackTrace;
    const limit = Error.stackTraceLimit;
    try {
      Error.stackTraceLimit = Infinity;
      Error.prepareStackTrace = (_error, frames) => frames;
      for (const frame of new Error().stack) {
        let file = frame.getFileName();
        if (file?.startsWith('file:')) file = fileURLToPath(file);
        if (sources.files.has(file)) {
          return { file: nameFor(file), line: frame.getLineNumber(), column: frame.getColumnNumber() };
        }
      }
    } finally {
      Error.prepareStackTrace = prepare;
      Error.stackTraceLimit = limit;
    }
    throw new Error('registration has no fixture stack frame');
  }

  function record(kind, args, callbacks, frame, pathNode) {
    const key = `${frame.file}:${frame.line}`;
    const ordinal = (ordinals.get(key) ?? 0) + 1;
    ordinals.set(key, ordinal);
    const call = {
      id: calls.length + 1, kind, site: { ...frame, ordinal }, args, mounted: false, callbacks,
    };
    if (pathNode && pathNode.type !== 'SpreadElement') {
      const raw = sources.files.get(join(root, frame.file)).source.slice(pathNode.start, pathNode.end);
      const value = pathNode.type === 'TemplateLiteral' && pathNode.expressions.length === 0
        ? pathNode.quasis[0].value.cooked : pathNode.value;
      const literal = typeof value === 'string' && value === args.path && !raw.includes('\\') ? value : null;
      call.path = { raw, literal };
    }
    calls.push(call);
    return call;
  }

  function pathArgument(frame, method) {
    const nodes = sources.files.get(join(root, frame.file)).calls;
    const relevant = nodes.filter(node => {
      const callee = node.callee;
      if (callee.type !== 'MemberExpression' ||
          (callee.property.name ?? callee.property.value) !== method) return false;
      const { start, end } = callee.property.loc;
      return start.line === frame.line && start.column + 1 <= frame.column && frame.column <= end.column + 1;
    });
    return relevant.length === 1 ? relevant[0].arguments[0] : undefined;
  }

  function withinCall(call, state, invoke) {
    const previous = active;
    active = { call, ...state };
    try {
      return invoke();
    } finally {
      active = previous;
    }
  }

  function activeAt(kind, frame) {
    const previous = active?.call.site;
    return active?.call.kind === kind && previous.file === frame.file &&
      previous.line === frame.line && previous.column === frame.column;
  }

  function isRouter(handler) {
    return typeof handler === 'function' && routerPrototype.isPrototypeOf(handler);
  }

  function isApp(handler) {
    return typeof handler === 'function' && typeof handler.lazyrouter === 'function' &&
      typeof handler.handle === 'function' && typeof handler.set === 'function';
  }

  function identify(fn, allowExternal = false) {
    const source = stringifyFunction.call(fn);
    const matches = sources.functions.get(source) ?? [];
    if (matches.length === 1) return { ...matches[0], source };
    if (allowExternal) {
      return { kind: isApp(fn) || isRouter(fn) ? 'router' : 'unlocated', name: fn.name, source };
    }
    throw new HandRequired(matches.length ? 'ambiguous Express callback source' : 'unlocated Express callback source');
  }

  const lazyrouter = appPrototype.lazyrouter;
  appPrototype.lazyrouter = function (...args) {
    initialization++;
    try {
      return lazyrouter.apply(this, args);
    } finally {
      initialization--;
    }
  };

  appPrototype.listen = function () {
    throw new Error('Express truth fixtures must not listen');
  };

  const routerRoute = routerPrototype.route;
  routerPrototype.route = function (path) {
    const frame = site();
    const builder = activeAt('verb', frame) ? null
      : record('route', { path: encodePath(path) }, [], frame, pathArgument(frame, 'route'));
    const route = routerRoute.call(this, path);
    const layer = this.stack.at(-1);
    if (layer.route !== route) throw new Error('expected a native Express route layer');
    routeLayers.set(layer, route);
    if (builder) builders.set(route, builder);
    return route;
  };

  for (const method of [...httpMethods, 'all']) {
    for (const prototype of [appPrototype, routerPrototype]) {
      const original = prototype[method];
      prototype[method] = function (...args) {
        if (prototype === appPrototype && method === 'get' && args.length === 1) {
          return original.apply(this, args);
        }
        const frame = site();
        const call = record('verb', {
          path: encodePath(args[0]), methods: method === 'all' ? 'ANY' : [method.toUpperCase()],
        }, args.slice(1).flat(Infinity).map(fn => identify(fn)), frame, pathArgument(frame, method));
        return withinCall(call, {}, () => original.apply(this, args));
      };
    }
    const original = express.Route.prototype[method];
    express.Route.prototype[method] = function (...args) {
      const frame = site();
      const delegated = activeAt('verb', frame);
      const call = delegated ? active.call : record('verb', {
        path: encodePath(this.path), methods: method === 'all' ? 'ANY' : [method.toUpperCase()],
      }, args.flat(Infinity).map(fn => identify(fn)), frame);
      const builder = builders.get(this);
      if (!delegated && builder?.path) call.path = builder.path;
      const before = this.stack.length;
      const result = original.apply(this, args);
      for (const layer of this.stack.slice(before)) {
        handlerLayers.set(layer, { id: call.id, callback: layer.handle, method: layer.method });
      }
      return result;
    };
  }

  // Express 4's deprecation wrapper closes over the original delete method.
  appPrototype.del = appPrototype.delete;

  function useArgs(args) {
    let first = args[0];
    while (Array.isArray(first) && first.length !== 0) first = first[0];
    const hasPath = typeof first !== 'function';
    return {
      path: hasPath ? args[0] : '/',
      callbacks: args.slice(Number(hasPath)).flat(Infinity),
      hasPath,
    };
  }

  function useCall(args, frame) {
    const { path, callbacks, hasPath } = useArgs(args);
    return record('use', { path: encodePath(path), has_path_arg: hasPath },
      callbacks.map(fn => identify(fn, true)), frame, hasPath ? pathArgument(frame, 'use') : undefined);
  }

  const routerUse = routerPrototype.use;
  routerPrototype.use = function (...args) {
    const before = this.stack.length;
    if (initialization) {
      const result = routerUse.apply(this, args);
      for (const layer of this.stack.slice(before)) bootstrapLayers.add(layer);
      return result;
    }
    const frame = site();
    const delegated = activeAt('use', frame) && active.receiver._router === this;
    const call = delegated ? active.call : useCall(args, frame);
    const { path, callbacks } = useArgs(args);
    const result = routerUse.apply(this, args);
    const layers = this.stack.slice(before);
    if (layers.length !== callbacks.length) throw new Error('unexpected Express use layer count');
    for (const [index, layer] of layers.entries()) {
      const handler = delegated ? active.callbacks[active.index++] : callbacks[index];
      const child = isApp(handler) || isRouter(handler) ? handler : null;
      if (child) call.mount = true;
      useLayers.set(layer, { call, path, child, callback: layer.handle });
    }
    return result;
  };

  const appUse = appPrototype.use;
  appPrototype.use = function (...args) {
    const call = useCall(args, site());
    const { callbacks } = useArgs(args);
    return withinCall(call, { receiver: this, callbacks, index: 0 }, () => {
      const result = appUse.apply(this, args);
      if (active.index !== callbacks.length) throw new Error('missing Express application use layer');
      return result;
    });
  };

  function served(app) {
    const observations = [];
    for (const [layer, mount] of useLayers) {
      if (mount.child) {
        mount.prefix = literalPrefix(layer, mount.path);
        mount.call.mount_prefix = mount.prefix;
      }
    }
    function walk(router, prefix, mounts, ancestors) {
      if (!router) return;
      if (ancestors.includes(router)) throw new HandRequired('cyclic Express mount');
      for (const layer of router.stack) {
        if (layer.route) {
          const route = layer.route;
          if (routeLayers.get(layer) !== route) throw new Error('untraced Express route layer');
          const registrations = [];
          for (const handler of route.stack) {
            const registration = handlerLayers.get(handler);
            if (!registration || registration.callback !== handler.handle || registration.method !== handler.method) {
              throw new Error('untraced Express route handler');
            }
            if (!registrations.includes(registration.id)) registrations.push(registration.id);
          }
          if (!registrations.length) {
            if (Object.keys(route.methods).length) throw new Error('untraced Express route methods');
            continue;
          }
          const declared = registrations.map(id => calls[id - 1].args.methods);
          const effective = route.methods._all ? 'ANY' : [...new Set([...route._options(), 'OPTIONS'])].sort();
          const builder = builders.get(route);
          for (const path of Array.isArray(route.path) ? route.path.flat(Infinity) : [route.path]) {
            const observation = {
              kind: 'http', path: typeof path === 'string' ? prefix + path : encodePath(path),
              declared_methods: declared.includes('ANY') ? 'ANY' : [...new Set(declared.flat())].sort(),
              effective_methods: effective,
              call_ids: [...registrations, ...(builder ? [builder.id] : [])],
              mount_call_ids: mounts,
            };
            if (path instanceof RegExp) observation.mount_prefix = prefix;
            observations.push(observation);
          }
        } else if (!bootstrapLayers.has(layer)) {
          const mount = useLayers.get(layer);
          if (!mount || mount.callback !== layer.handle) throw new Error('untraced Express middleware/mount layer');
          if (mount.child) {
            const child = isApp(mount.child) ? mount.child._router : mount.child;
            walk(child, prefix + mount.prefix, [...mounts, mount.call.id], [...ancestors, router]);
          }
        }
      }
    }
    walk(app._router, '', [], []);
    return observations;
  }

  return { calls, served, isApp };
}

try {
  if (process.versions.node.split('.')[0] !== '24') {
    throw new EnvironmentError(`expected Node 24, found ${process.versions.node}`);
  }
  const pin = require('express/package.json').version;
  if (pin !== '4.22.3') throw new EnvironmentError(`expected Express 4.22.3, found ${pin}`);
  const sources = readSources();
  installResolutionGuard(sources.packages);
  const express = require('express');
  const trace = installExpress(express, sources);
  const entry = pathToFileURL(join(root, process.argv[3]));
  const originalStdout = process.stdout.write;
  let output;
  try {
    process.stdout.write = process.stderr.write.bind(process.stderr);
    const exported = await import(entry.href);
    const app = [exported.default, exported.app, exported.default?.app].find(trace.isApp);
    if (!app) throw new Error('entry must export an Express app as default or app');
    output = { status: 'executed', framework: 'express', calls: trace.calls, served: trace.served(app) };
  } finally {
    process.stdout.write = originalStdout;
  }
  protocol(JSON.stringify(output));
} catch (error) {
  const status = error instanceof HandRequired ? 'hand_required'
    : error instanceof EnvironmentError ? 'environment_error' : 'error';
  protocol(JSON.stringify({ status, error: error.message }));
}
