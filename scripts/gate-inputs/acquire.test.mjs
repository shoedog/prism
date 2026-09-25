import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {
  cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync,
} from 'node:fs';
import {tmpdir} from 'node:os';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import test from 'node:test';
import {gzipSync} from 'node:zlib';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const candidate = process.env.GATE_INPUTS_MODULE
  ? pathToFileURL(resolve(process.env.GATE_INPUTS_MODULE)).href
  : new URL('./acquire.mjs', import.meta.url);
const {acquire, envLines, inputDirs, treeDigest, verifyInstalled} = await import(candidate);
const sha = value => createHash('sha256').update(value).digest('hex');
const sri = value => `sha512-${createHash('sha512').update(value).digest('base64')}`;
const rootFor = name => join(repo, 'target/gate-input-tests', name);
const sorted = object => Object.entries(object).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0);
const digest = object => {
  return sha(JSON.stringify(sorted(object).map(([name, body]) => [name, sha(body)])));
};
const copy = value => JSON.parse(JSON.stringify(value));

function ustar(entries) {
  const blocks = [];
  for (const entry of entries) {
    const body = Buffer.from(entry.body ?? '');
    const size = entry.size ?? body.length, header = Buffer.alloc(512);
    header.write(entry.name, 0, 'utf8');
    header.write(`${size.toString(8).padStart(11, '0')}\0`, 124, 'ascii');
    header[156] = entry.type ?? 48;
    header.write('ustar\0', 257, 'ascii');
    if (entry.prefix) header.write(entry.prefix, 345, 'utf8');
    if (entry.tail) header.write('00000000001\0', 476, 'ascii');
    blocks.push(header, body);
    if (!entry.truncated) blocks.push(Buffer.alloc((512 - body.length % 512) % 512));
  }
  if (!entries.at(-1)?.truncated) blocks.push(Buffer.alloc(1024));
  return gzipSync(Buffer.concat(blocks));
}
function synthetic() {
  const ts = ustar([
    {name: 'package/lib/', type: 53}, {name: 'package/lib/typescript.js', body: 'compiler'},
    {name: 'package/package.json', body: '{"version":"5.9.3"}'},
  ]);
  const profile = ustar([
    {name: 'react/', type: 53, tail: true}, {name: 'react/a/', type: 53, tail: true},
    {name: 'react/a/b/', type: 53, tail: true},
    {name: 'react/a/b/index.d.ts', body: 'declare x', tail: true},
    {name: 'react/package.json', body: '{"version":"19.0.10"}', tail: true},
  ]);
  const raw = Buffer.from('grammar');
  const profileFiles = {
    'react19/node_modules/@types/react/a/b/index.d.ts': 'declare x',
    'react19/node_modules/@types/react/package.json': '{"version":"19.0.10"}',
  };
  const oldRows = Object.entries(profileFiles)
    .map(([name, body]) => [name.replace('react19/', ''), body]);
  const historical = sha(JSON.stringify(sorted(Object.fromEntries(oldRows))));
  const pins = {schema: 'prism.gate-inputs/1', inputs: {
    typescript: {env: 'PRISM_TYPESCRIPT', env_suffix: 'lib/typescript.js',
      tree_sha256: digest({'lib/typescript.js': 'compiler', 'package.json': '{"version":"5.9.3"}'}),
      file_sha256: {'lib/typescript.js': sha('compiler')},
      artifacts: [{name: 'typescript.tgz', url: 'u://ts', integrity: sri(ts), root: 'package',
        dest: '.', authority: 'test'}]},
    profiles: {env: 'PRISM_CALLABLE_PROFILES', env_suffix: '', tree_sha256: digest(profileFiles),
      profile_hash: {react19: historical},
      artifacts: [{name: 'react.tgz', url: 'u://profile', integrity: sri(profile),
        root: 'react', dest: 'react19/node_modules/@types/react', authority: 'test'}]},
    'grammar-archives': {env: 'PRISM_GRAMMAR_ARCHIVES', env_suffix: '',
      tree_sha256: digest({'grammar.gz': raw}),
      artifacts: [{name: 'grammar.gz', url: 'u://raw', sha256: sha(raw), dest: 'grammar.gz',
        authority: 'test'}]},
  }};
  return {pins, bytes: new Map([['u://ts', ts], ['u://profile', profile], ['u://raw', raw]])};
}
function fetchFrom(bytes, calls = []) {
  return async url => { calls.push(url); return new Response(bytes.get(url)); };
}
function streamError() {
  let yielded = false;
  return new Response(new ReadableStream({pull(controller) {
    if (!yielded) { yielded = true; controller.enqueue(new Uint8Array([1])); }
    else controller.error(new Error('simulated socket reset'));
  }}));
}
function single(pins, input) {
  return {schema: pins.schema, inputs: {[input]: pins.inputs[input]}};
}
function clean(name) {
  const root = rootFor(name); rmSync(root, {recursive: true, force: true}); return root;
}
function stages(root) {
  return existsSync(root) ? readdirSync(root).filter(name => name.startsWith('.stage-')) : [];
}
function absent(root, input, pins) {
  assert.equal(existsSync(join(root, input, pins.inputs[input].tree_sha256)), false);
  assert.deepEqual(stages(root), []);
}

test('A1 installs, verifies, prints env, then reuses immutable inputs', async () => {
  const {pins, bytes} = synthetic(), root = clean('a1'), calls = [];
  const result = await acquire({fetch: fetchFrom(bytes, calls), root, pins});
  assert.deepEqual(result, {ok: true, root, inputs: [
    {input: 'typescript', status: 'installed',
      artifacts: [{name: 'typescript.tgz', files: 2, dirs: 1}]},
    {input: 'profiles', status: 'installed', artifacts: [{name: 'react.tgz', files: 2, dirs: 3}]},
    {input: 'grammar-archives', status: 'installed',
      artifacts: [{name: 'grammar.gz', files: 1, dirs: 0, raw: true}]},
  ]});
  const verified = {root, inputs: Object.entries(pins.inputs).map(([input, pin]) => ({
    input, dir: join(root, input, pin.tree_sha256),
  }))};
  assert.deepEqual(verifyInstalled({root, pins}), verified);
  const lines = Object.entries(inputDirs(root, pins))
    .map(([name, value]) => `export ${name}='${value}'`);
  assert.deepEqual(envLines({root, pins}), lines);
  const again = await acquire({fetch: fetchFrom(bytes, calls), root, pins});
  const reused = Object.keys(pins.inputs).map(input => ({input, status: 'verified'}));
  assert.deepEqual(again, {ok: true, root, inputs: reused});
  assert.equal(calls.length, 3); assert.deepEqual(stages(root), []);
});

test('A2 rejects unauthenticated, unavailable, and over-cap bodies before install', async t => {
  const cases = [
    ['wrong SRI', 'typescript', async (pins, bytes) => bytes.set('u://ts', Buffer.from('not-gzip')),
      'typescript artifact typescript.tgz: byte authority'],
    ['wrong sha256', 'grammar-archives',
      async (pins, bytes) => bytes.set('u://raw', Buffer.from('not-gzip')),
      'grammar-archives artifact grammar.gz: byte authority'],
    ['non-ok', 'typescript', async () => undefined,
      'typescript artifact typescript.tgz: fetch status 503'],
    ['body cap', 'typescript', async () => undefined,
      'typescript artifact typescript.tgz: body cap'],
    ['body read', 'typescript', async () => undefined,
      'typescript artifact typescript.tgz: body read failed'],
  ];
  for (const [name, input, change, message] of cases) await t.test(name, async () => {
    const {pins, bytes} = synthetic(), only = single(copy(pins), input), root = clean(`a2-${name}`);
    await change(only, bytes);
    const fetch = name === 'non-ok' ? async () => ({ok: false, status: 503}) : name === 'body cap'
      ? async () => new Response(new ReadableStream({start(c) {
        c.enqueue(new Uint8Array(64 * 1024 * 1024 + 1)); c.close();
      }}))
      : name === 'body read' ? async () => streamError()
      : fetchFrom(bytes);
    assert.deepEqual(await acquire({fetch, root, pins: only}), {ok: false, root, inputs: [
      {input, status: 'refused', error: message},
    ]});
    absent(root, input, only);
  });
});

test('A3 accepts the census profile and refuses each unsupported tar row', async t => {
  const cases = [
    ['symlink', [{name: 'package/link', type: 50}], 'unsupported tar entry 2 package/link'],
    ['pax', [{name: 'package/x', type: 120}], 'unsupported tar entry x package/x'],
    ['outside root', [{name: 'else/x'}], 'tar root: else/x'],
    ['parent component', [{name: 'package/../x'}], 'tar path: package/../x'],
    ['prefix', [{name: 'package/x', prefix: 'bad'}], 'ustar prefix unsupported'],
    ['nonzero directory', [{name: 'package/a/', type: 53, size: 1, body: 'x'}],
      'directory size: package/a/'],
    ['truncated', [{name: 'package/x', size: 9, body: 'x', truncated: true}], 'truncated'],
    ['duplicate', [{name: 'package/x'}, {name: 'package/x'}], 'duplicate destination x'],
    ['regular trailing slash', [{name: 'package/x/'}],
      'tar path: regular trailing slash package/x/'],
  ];
  for (const [name, entries, rule] of cases) await t.test(name, async () => {
    const {pins, bytes} = synthetic(), only = single(copy(pins), 'typescript');
    const root = clean(`a3-${name}`), archive = ustar(entries);
    only.inputs.typescript.artifacts[0].integrity = sri(archive); bytes.set('u://ts', archive);
    const actual = await acquire({fetch: fetchFrom(bytes), root, pins: only});
    assert.deepEqual(actual, {ok: false, root, inputs: [{input: 'typescript', status: 'refused',
      error: `typescript artifact typescript.tgz: ${rule}`} ]});
    absent(root, 'typescript', only);
  });
});

test('A4 rejects tree and historical identity mismatches', async t => {
  const cases = [
    ['tree', 'typescript', pins => { pins.inputs.typescript.tree_sha256 = '0'.repeat(64); },
      'typescript: tree digest'],
    ['profile', 'profiles', pins => { pins.inputs.profiles.profile_hash.react19 = '0'.repeat(64); },
      'profiles profile hash: react19'],
    ['file', 'typescript', pins => {
      pins.inputs.typescript.file_sha256['lib/typescript.js'] = '0'.repeat(64);
    },
      'typescript file hash: lib/typescript.js'],
  ];
  for (const [name, input, change, message] of cases) await t.test(name, async () => {
    const {pins, bytes} = synthetic(), only = single(copy(pins), input);
    const root = clean(`a4-${name}`); change(only);
    const actual = await acquire({fetch: fetchFrom(bytes), root, pins: only});
    const expected = {ok: false, root, inputs: [{input, status: 'refused', error: message}]};
    assert.deepEqual(actual, expected);
    absent(root, input, only);
  });
});

test('A5 verifies on every use and never auto-repairs a tampered install', async () => {
  const {pins, bytes} = synthetic(), only = single(copy(pins), 'typescript');
  const root = clean('a5'), calls = [];
  await acquire({fetch: fetchFrom(bytes, calls), root, pins: only});
  const dir = join(root, 'typescript', only.inputs.typescript.tree_sha256);
  const message = `typescript: install corrupt: remove ${dir} and re-run`;
  writeFileSync(join(dir, 'lib/typescript.js'), 'tampered');
  assert.throws(() => verifyInstalled({root, pins: only}), {message});
  const actual = await acquire({fetch: fetchFrom(bytes, calls), root, pins: only});
  const expected = {ok: false, root,
    inputs: [{input: 'typescript', status: 'refused', error: message}]};
  assert.deepEqual(actual, expected);
  assert.throws(() => envLines({root, pins: only}), {message});
  assert.equal(calls.length, 1); assert.deepEqual(stages(root), []);
});

test('A6 converges on a winning rename race and preserves a foreign stage', async () => {
  const {pins, bytes} = synthetic(), only = single(copy(pins), 'typescript'), root = clean('a6');
  mkdirSync(root, {recursive: true});
  mkdirSync(join(root, '.stage-foreign'));
  writeFileSync(join(root, '.stage-foreign', 'keep'), 'keep');
  const result = await acquire({fetch: fetchFrom(bytes), root, pins: only,
    onBeforeRename: ({stage, final}) => {
    mkdirSync(dirname(final), {recursive: true}); cpSync(stage, final, {recursive: true});
    }});
  assert.deepEqual(result, {ok: true, root, inputs: [{input: 'typescript', status: 'verified'}]});
  assert.equal(readFileSync(join(root, '.stage-foreign', 'keep'), 'utf8'), 'keep');
  assert.deepEqual(stages(root), ['.stage-foreign']);
});

test('A7 root controls and POSIX apostrophe quoting', async () => {
  const {pins, bytes} = synthetic(), only = single(copy(pins), 'typescript');
  const unsupported = clean('a7-unsupported');
  const unsupportedResult = await acquire({fetch: fetchFrom(bytes), root: unsupported, pins: only,
    platform: 'linux'});
  assert.deepEqual(unsupportedResult, {ok: false, inputs: [], error: 'root: unsupported host'});
  assert.equal(existsSync(unsupported), false);
  const volatile = join(tmpdir(), 'gate-inputs-a7');
  const volatileResult = await acquire({fetch: fetchFrom(bytes), root: volatile, pins: only});
  assert.deepEqual(volatileResult, {ok: false, inputs: [], error: 'root: volatile root'});
  const badRoot = `${rootFor('bad')}\n`;
  const badResult = await acquire({fetch: fetchFrom(bytes), root: badRoot, pins: only});
  assert.deepEqual(badResult, {ok: false, inputs: [], error: 'root: control character'});
  const root = `${rootFor("quote's")}`, saved = process.env.PRISM_GATE_INPUTS_ROOT;
  process.env.PRISM_GATE_INPUTS_ROOT = root;
  try {
    assert.equal((await acquire({fetch: fetchFrom(bytes), pins: only})).root, root);
    const line = envLines({pins: only})[0];
    const shell = spawnSync('sh', ['-c', `${line}; printf '%s' "$PRISM_TYPESCRIPT"`],
      {encoding: 'utf8'});
    assert.deepEqual({status: shell.status, stdout: shell.stdout, stderr: shell.stderr}, {status: 0,
      stdout: join(root, 'typescript', only.inputs.typescript.tree_sha256, 'lib/typescript.js'),
      stderr: ''});
  } finally {
    if (saved === undefined) delete process.env.PRISM_GATE_INPUTS_ROOT;
    else process.env.PRISM_GATE_INPUTS_ROOT = saved;
  }
});

test('CLI rejects invalid commands as a refusal', () => {
  const result = spawnSync(process.execPath, [fileURLToPath(candidate), 'invalid-command'],
    {encoding: 'utf8'});
  const actual = {status: result.status, stdout: result.stdout, stderr: result.stderr};
  const expected = {status: 2, stdout: '', stderr: 'cli: usage acquire|verify|env\n'};
  assert.deepEqual(actual, expected);
});

test('A8 pins the files-only two-file tree-digest formula', () => {
  const root = clean('a8'); mkdirSync(join(root, 'dir'), {recursive: true});
  writeFileSync(join(root, 'a.txt'), 'A'); writeFileSync(join(root, 'dir/b.txt'), 'B');
  const expected = 'f98f2c829c3d3110a55316649c79a43b922b0e48b7bc05b8301dac6a324c6074';
  assert.equal(treeDigest(root), expected);
});

test('A9 keeps real pins tied to all cited local authorities', () => {
  const pins = JSON.parse(readFileSync(join(repo, 'scripts/gate-inputs/pins.json')));
  const registryFile = join(repo, 'docs/superpowers/plans/2026-09-24-gate-input-durability',
    'advisor/registry-integrity.json');
  const registry = JSON.parse(readFileSync(registryFile)).packages;
  const npm = Object.values(pins.inputs).flatMap(pin => pin.artifacts)
    .filter(row => row.integrity).map(row => ({
    name: row.package, version: row.version, tarball: row.url, integrity: row.integrity,
    metadata_sha256: row.metadata_sha256,
  })).sort((a, b) => `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`));
  const expected = registry.map(row => ({
    name: row.name, version: row.version, tarball: row.tarball, integrity: row.integrity,
    metadata_sha256: row.metadata_sha256,
  })).sort((a, b) => `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`));
  assert.deepEqual(npm, expected);
  const names = Object.values(pins.inputs).flatMap(pin => pin.artifacts)
    .map(row => row.name).sort();
  assert.deepEqual(names, [
    '@types/prop-types-15.7.15.tgz', '@types/react-18.3.31.tgz',
    '@types/react-19.0.10.tgz', 'csstype-3.1.3.tgz', 'csstype-3.2.3.tgz',
    'javascript-0.23.1.tgz', 'tree-sitter-macos-arm64.gz',
    'typescript-5.9.3.tgz', 'upstream.tar.gz',
  ]);
  const schema = readFileSync(join(repo, 'scripts/callable-observations/schema.mjs'), 'utf8');
  const compiler = '3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675';
  assert.match(schema, new RegExp(`COMPILER_HASH="${compiler}"`));
  const authorityFile = join(repo, 'docs/eval/receiver-closure/verify-callable-authority.mjs');
  const authority = readFileSync(authorityFile, 'utf8');
  const react19 = '49c6c7a3cde29161a5af224dede5e4442295f9251ef4c694699341ed3682baad';
  const react18 = '7b8bbdc844cd38cbf691987a229858e4006273c3d3b3c4d8c8fef70267859b34';
  assert.match(authority, new RegExp(`react19", "19\\.0\\.10", "3\\.1\\.3", "${react19}`));
  assert.match(authority, new RegExp(`react18", "18\\.3\\.31", "3\\.2\\.3", "${react18}`));
  const grammar = readFileSync(join(repo, 'scripts/verify-typescript-grammar.mjs'), 'utf8');
  for (const row of pins.inputs['grammar-archives'].artifacts) {
    const url = row.url.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    assert.match(grammar, new RegExp(`${url}[\\s\\S]{0,180}${row.sha256}`));
  }
  const grammarRows = pins.inputs['grammar-archives'].artifacts
    .map(row => [row.dest, row.sha256]).sort();
  assert.equal(sha(JSON.stringify(grammarRows)), pins.inputs['grammar-archives'].tree_sha256);
  const treeDigests = Object.fromEntries(Object.entries(pins.inputs)
    .map(([name, pin]) => [name, pin.tree_sha256]));
  assert.deepEqual(treeDigests, {
    typescript: '7e02162c902e5c29ec19fefc573ad55b8ed15fcbda940e737f297c53e9c41f54',
    profiles: 'aa290dd630dcbfea5523bb627e7e3f6008de70c6fa0601c0df82d1d9975ff443',
    'grammar-archives': '35ba6cbb8757dd812ab334163cac59ef2f6bdcccb1588b4d97d941e1cf579e5f',
  });
});
