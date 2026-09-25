import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {chmodSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync,
  writeFileSync} from 'node:fs';
import {dirname, join, resolve} from 'node:path';
import test from 'node:test';
import {fileURLToPath, pathToFileURL} from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const candidate = process.env.GATE_MODULE
  ? pathToFileURL(resolve(process.env.GATE_MODULE)).href : new URL('./gate.mjs', import.meta.url);
const {build, buildEnv, dryRun, gate, population, prepare, repoRoot, resolveTool} =
  await import(candidate);
const sha = value => createHash('sha256').update(value).digest('hex');
const pass = "import test from 'node:test';\ntest('passing', () => {});\n";
const fail = "import test from 'node:test';\ntest('failing', () => { throw new Error('no'); });\n";
const skip = "import test from 'node:test';\ntest('skipped', {skip: 'reason'}, () => {});\n";
const rootFor = name => join(repo, 'target/gate-input-tests', `b-${name}`);
function put(root, path, body) {
  mkdirSync(dirname(join(root, path)), {recursive: true}); writeFileSync(join(root, path), body);
}
function clean(name) {
  const root = rootFor(name); rmSync(root, {recursive: true, force: true}); return root;
}
function git(root, args) {
  const result = spawnSync('git', args, {cwd: root, encoding: 'utf8'});
  assert.deepEqual({status: result.status, stderr: result.stderr}, {status: 0, stderr: ''});
  return result.stdout;
}
function fixture(name, files) {
  const root = clean(name); mkdirSync(root, {recursive: true});
  for (const [path, body] of Object.entries(files)) put(root, path, body);
  git(root, ['init', '-q']);
  git(root, ['config', 'user.email', 'test@example.invalid']);
  git(root, ['config', 'user.name', 'Gate Test']); git(root, ['add', '-A']);
  git(root, ['commit', '-qm', 'fixture']); return root;
}
function fakeCargo(root, failing = false) {
  const bin = join(root, 'cargo-bin'), cargo = join(bin, 'cargo');
  const native = join(bin, 'native'), args = join(bin, 'args');
  mkdirSync(bin, {recursive: true}); writeFileSync(native, 'native');
  const message = JSON.stringify({reason: 'compiler-artifact',
    target: {name: 'project_membership_census'},
    executable: native});
  const body = failing ? '#!/bin/sh\n[ "$1" = --version ] && exit 0\nexit 101\n' :
    `#!/bin/sh\n[ "$1" = --version ] && { echo fake; exit 0; }\nprintf '%s\\n' "$@" > ${args}\n` +
    `printf '%s\\n' '${message}'\n`;
  writeFileSync(cargo, body); chmodSync(cargo, 0o755); return {bin, native, args};
}
const verified = () => ({root: '/inputs', inputs: [
  {input: 'typescript', dir: '/inputs/typescript/ts'},
  {input: 'profiles', dir: '/inputs/profiles/profiles'},
  {input: 'grammar-archives', dir: '/inputs/grammar/grammar'},
]});
const dirs = root => ({PRISM_TYPESCRIPT: `${root}/typescript.js`,
  PRISM_CALLABLE_PROFILES: `${root}/profiles`,
  PRISM_GRAMMAR_ARCHIVES: `${root}/grammar`});
function options(root, cargo, verify = verified) {
  return {root, inherited: {HOME: '/home/test', PATH: `${cargo.bin}:${process.env.PATH}`},
    verify, dirs,
    exclusions: []};
}
function populationOf(root, exclusions = []) {
  const env = buildEnv({inherited: {PATH: process.env.PATH}});
  return population({root, git: resolveTool('git', env.PATH), env, exclusions});
}
function expectedPopulation(active, exclusions = []) {
  return {active, exclusions, digest: sha(JSON.stringify(active))};
}
function fullReceipt(plan, extra = {}) {
  const native = extra.native ?? {path: null, sha256: null};
  const env = native.path ? {...plan.env, PRISM_MEMBERSHIP_NATIVE: native.path} : plan.env;
  return {repo: plan.repo, population: plan.population, tools: plan.tools, runtime: plan.runtime,
    inputs: plan.inputs,
    native, argv: plan.argv, cwd: plan.cwd, env, totals: null, skips: [], status: 'refused',
    stage: 'preflight', exitCode: 2, error: null, ...extra};
}
function blankReceipt(platform, error) {
  return {repo: {head: null, dirty: null}, population: null, tools: null,
    runtime: {node: process.version, platform, arch: process.arch}, inputs: null,
    native: {path: null, sha256: null}, argv: null, cwd: null, env: null, totals: null,
    skips: [],
    status: 'refused', stage: 'preflight', exitCode: 2, error};
}
async function execute(root, cargo, label) {
  const setup = options(root, cargo), plan = prepare(setup);
  const out = join(root, 'target', label);
  return {plan, out, result: await gate({out, ...setup})};
}

test('B1 enumerates committed NUL-safe paths and refuses all visible drift', () => {
  const files = {'a.test.mjs': pass, 'line\nname.test.mjs': pass, 'ignored.test.mjs': pass,
    'info.test.mjs': pass, 'global.test.mjs': pass, 'assumed.test.mjs': pass};
  const root = fixture('b1', files), active = Object.keys(files).sort();
  put(root, '.gitignore', 'target/\nignored.test.mjs\n'); git(root, ['add', '.gitignore']);
  git(root, ['commit', '-qm', 'ignore later']); put(root, '.git/info/exclude', 'info.test.mjs\n');
  const global = join(root, 'global-excludes'); writeFileSync(global, 'global.test.mjs\n');
  git(root, ['config', 'core.excludesFile', global]);
  git(root, ['update-index', '--assume-unchanged', 'assumed.test.mjs']);
  put(root, 'target/c.test.mjs', pass);
  assert.deepEqual(populationOf(root), expectedPopulation(active));
  put(root, 'b.test.mjs', pass);
  assert.throws(() => populationOf(root),
    {message: 'untracked test module: commit or remove b.test.mjs'});
  rmSync(join(root, 'b.test.mjs'));
  assert.throws(() => populationOf(root, [{path: 'stale.test.mjs', reason: 'x'}]),
    {message: 'stale exclusion: stale.test.mjs'});
  git(root, ['rm', '-q', 'a.test.mjs']);
  assert.throws(() => populationOf(root), {message: 'population index drift: a.test.mjs'});
  git(root, ['reset', '--hard', 'HEAD']); put(root, 'added.test.mjs', pass);
  git(root, ['add', 'added.test.mjs']);
  assert.throws(() => populationOf(root), {message: 'population index drift: added.test.mjs'});
  git(root, ['reset', '--hard', 'HEAD']); rmSync(join(root, 'added.test.mjs'), {force: true});
  git(root, ['mv', 'ignored.test.mjs', 'renamed.test.mjs']);
  assert.throws(() => populationOf(root),
    {message: 'population index drift: ignored.test.mjs, renamed.test.mjs'});
});

test('B2 seals the environment from an empty object', () => {
  const inputs = {PRISM_TYPESCRIPT: '/ts', PRISM_CALLABLE_PROFILES: '/profiles',
    PRISM_GRAMMAR_ARCHIVES: '/grammar'};
  const inherited = {HOME: '/home/test', PATH: '/bin', LANG: 'C', LC_ALL: 'C', TMPDIR: '/tmp',
    CARGO_HOME: '/cargo', RUSTUP_HOME: '/rustup', RUSTUP_TOOLCHAIN: 'stable',
    NODE_OPTIONS: '--inspect',
    PRISM_CALLABLE_IMPLEMENTATION: '/wrong', CARGO_TARGET_DIR: '/target', USER: 'wrong'};
  assert.deepEqual(buildEnv({inherited, inputs}), {
    HOME: '/home/test', PATH: `${dirname(process.execPath)}:/bin`,
    LANG: 'C', LC_ALL: 'C', TMPDIR: '/tmp', CARGO_HOME: '/cargo', RUSTUP_HOME: '/rustup',
    RUSTUP_TOOLCHAIN: 'stable', GIT_CONFIG_GLOBAL: '/dev/null',
    GIT_CONFIG_NOSYSTEM: '1', ...inputs});
});

test('B3 records a frozen native build and a build refusal receipt', async () => {
  const root = fixture('b3', {'.gitignore': 'target/\n', 'a.test.mjs': pass});
  const cargo = fakeCargo(root), setup = options(root, cargo), plan = prepare(setup);
  const native = {path: cargo.native, sha256: sha('native')};
  assert.deepEqual(build(plan), native);
  assert.deepEqual(readFileSync(cargo.args, 'utf8').trim().split('\n'),
    ['build', '--frozen', '--offline',
    '--example', 'project_membership_census', '--message-format=json']);
  const bad = fakeCargo(fixture('b3-fail', {'.gitignore': 'target/\n', 'a.test.mjs': pass}),
    true);
  const badRoot = dirname(bad.bin), badSetup = options(badRoot, bad), badPlan = prepare(badSetup);
  const out = join(badRoot, 'target', 'out'), result = await gate({out, ...badSetup});
  const expected = fullReceipt(badPlan, {stage: 'build', error: 'build: cargo failed'});
  assert.deepEqual({exitCode: result.exitCode, receipt: result.receipt},
    {exitCode: 2, receipt: expected});
  assert.deepEqual(JSON.parse(readFileSync(join(out, 'receipt.json'))), expected);
  assert.equal(existsSync(join(out, 'log.txt')), false);
});

test('B4 streams failed TAP output, records totals, and names skips', async () => {
  const root = fixture('b4', {'.gitignore': 'target/\n', 'pass.test.mjs': pass,
    'fail.test.mjs': fail});
  const cargo = fakeCargo(root), failed = await execute(root, cargo, 'failed');
  const native = {path: cargo.native, sha256: sha('native')};
  const expected = fullReceipt(failed.plan, {native,
    totals: {tests: 2, pass: 1, fail: 1, skipped: 0},
    status: 'failed', stage: 'tests', exitCode: 1});
  assert.deepEqual(failed.result.receipt, expected);
  const log = readFileSync(join(failed.out, 'log.txt'), 'utf8');
  assert.match(log, /passing/); assert.match(log, /failing/);
  const skippedRoot = fixture('b4-skip', {'.gitignore': 'target/\n', 'skip.test.mjs': skip});
  const skippedCargo = fakeCargo(skippedRoot);
  const skipped = await execute(skippedRoot, skippedCargo, 'skipped');
  assert.deepEqual(skipped.result.receipt, fullReceipt(skipped.plan, {
    native: {path: skippedCargo.native, sha256: sha('native')},
    totals: {tests: 1, pass: 0, fail: 0, skipped: 1},
    skips: ['skipped'], status: 'passed', stage: 'tests', exitCode: 0,
  }));
});

test('B5 refuses missing tools and a tampered test-root input before tests', async () => {
  const root = fixture('b5', {'.gitignore': 'target/\n', 'a.test.mjs': pass});
  const noGit = join(root, 'none');
  mkdirSync(noGit); const missingOut = join(root, 'target', 'missing');
  const missing = await gate({out: missingOut, root, inherited: {PATH: noGit},
    verify: verified,
    dirs, exclusions: []});
  assert.deepEqual(missing.receipt, blankReceipt(process.platform, 'preflight: git not found'));
  assert.equal(existsSync(join(missingOut, 'log.txt')), false);
  const cargo = fakeCargo(root), input = join(root, 'target', 'input');
  put(input, 'typescript', 'tampered');
  const tampered = () => { if (readFileSync(join(input, 'typescript'), 'utf8') === 'tampered') {
    throw new Error('typescript: install corrupt');
  } return verified(); };
  const tamperedOut = join(root, 'target', 'tampered');
  const result = await gate({out: tamperedOut, ...options(root, cargo, tampered)});
  assert.deepEqual(result.receipt, blankReceipt(process.platform, 'typescript: install corrupt'));
  assert.equal(existsSync(join(tamperedOut, 'log.txt')), false);
});

test('B6 owns a new output leaf first and dry-run writes no receipt', async () => {
  const root = fixture('b6', {'.gitignore': 'target/\n', 'a.test.mjs': pass});
  const existing = join(root, 'target', 'old');
  mkdirSync(existing, {recursive: true}); const refusal = await gate({out: existing});
  assert.equal(refusal.exitCode, 2); assert.match(refusal.error, /^output: EEXIST/);
  assert.deepEqual(readdirSync(existing), []);
  const cargo = fakeCargo(root), setup = options(root, cargo), plan = prepare(setup);
  const out = join(root, 'target', 'new', 'leaf');
  assert.deepEqual(dryRun(setup), {population: plan.population, argv: plan.argv, cwd: plan.cwd,
    env: {...plan.env, PRISM_MEMBERSHIP_NATIVE: '<resolved after build>'}});
  assert.equal(existsSync(out), false); const result = await gate({out, ...setup});
  assert.deepEqual(result.receipt, fullReceipt(plan, {
    native: {path: cargo.native, sha256: sha('native')},
    totals: {tests: 1, pass: 1, fail: 0, skipped: 0}, status: 'passed', stage: 'tests', exitCode: 0,
  }));
});

test('B7 derives the repository root independently of the caller cwd', () => {
  const cwd = rootFor('cwd'); mkdirSync(cwd, {recursive: true});
  const source = `import {repoRoot} from ${JSON.stringify(candidate)};` +
    'process.stdout.write(repoRoot());';
  const result = spawnSync(process.execPath, ['--input-type=module', '--eval', source],
    {cwd, encoding: 'utf8'});
  assert.deepEqual({status: result.status, stdout: result.stdout, stderr: result.stderr},
    {status: 0,
    stdout: git(repo, ['rev-parse', '--show-toplevel']).trim(), stderr: ''});
  assert.equal(repoRoot(), repo);
});

test('B8 protects dash-prefixed modules and refuses a non-darwin host', async () => {
  const root = fixture('b8', {'.gitignore': 'target/\n', '--dash.test.mjs': pass});
  const cargo = fakeCargo(root);
  const dash = await execute(root, cargo, 'dash');
  const native = {path: cargo.native, sha256: sha('native')};
  assert.deepEqual(dash.result.receipt, fullReceipt(dash.plan, {native,
    totals: {tests: 1, pass: 1, fail: 0, skipped: 0},
    status: 'passed', stage: 'tests', exitCode: 0,
  }));
  assert.equal(dash.result.receipt.argv.at(-1), join(root, '--dash.test.mjs'));
  const out = join(root, 'target', 'linux');
  const linux = await gate({out, ...options(root, cargo), platform: 'linux'});
  assert.deepEqual(linux.receipt, blankReceipt('linux', 'unsupported host'));
});
