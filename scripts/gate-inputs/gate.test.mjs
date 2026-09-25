import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {chmodSync, existsSync, symlinkSync, mkdirSync, readFileSync, readdirSync, rmSync,
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
const membership = 'project_membership_census', parameter = 'parameter_slot_characterization';
const nativeRows = [['PRISM_MEMBERSHIP_NATIVE', membership],
  ['PRISM_NATIVE_PARAMETER_EXAMPLE', parameter]];
const natives = nativeRows.map(([env, example]) => ({env, example}));
const native = cargo => Object.fromEntries(nativeRows.map(([env, example]) => [env,
  {example, path: cargo?.native(example) ?? null, sha256: cargo ? sha('native') : null}]));
const nativeEnv = rows => Object.fromEntries(Object.entries(rows).filter(([, row]) => row.path)
  .map(([env, row]) => [env, row.path]));
const rootFor = name => join(repo, 'target/gate-input-tests', `b-${name}`);
function put(root, path, body) { mkdirSync(dirname(join(root, path)), {recursive: true});
  writeFileSync(join(root, path), body);
}
function clean(name){const r=rootFor(name);rmSync(r,{recursive:true,force:true});return r;}
function git(root, args) { const result = spawnSync('git', args, {cwd: root, encoding: 'utf8'});
  assert.deepEqual({status: result.status, stderr: result.stderr}, {status: 0, stderr: ''});
  return result.stdout;
}
function fixture(name, files) { const root = clean(name); mkdirSync(root, {recursive: true});
  for (const [path, body] of Object.entries(files)) put(root, path, body);
  git(root, ['init', '-q']); git(root, ['config', 'user.email', 'test@example.invalid']);
  git(root, ['config', 'user.name', 'Gate Test']); git(root, ['add', '-A']);
  git(root, ['commit', '-qm', 'fixture']); return root;
}
function fakeCargo(root, failing = false, omit = '') {
  const bin = join(root, 'cargo-bin'), cargo = join(bin, 'cargo');
  const native = example => join(bin, `native-${example}`), args = join(bin, 'args');
  const omitted = JSON.stringify(omit);
  mkdirSync(bin, {recursive: true});
  const body = failing === 'preflight' ? '#!/bin/sh\necho preflight fault >&2\nexit 101\n' :
    typeof failing === 'string' ? failing :
    failing ? '#!/bin/sh\n[ "$1" = --version ] && exit 0\necho build fault >&2\nexit 101\n' :
    `#!/bin/sh\n[ "$1" = --version ] && { echo fake; exit 0; }\nprintf '%s\\n' "$@" > ${args}\n` +
    `while [ "$#" -gt 0 ]; do\n  if [ "$1" = --example ] && [ "$2" != ${omitted} ]; then\n` +
    `    shift; native=${bin}/native-$1; printf native > "$native"\n` +
    `    printf '%s\\n' "{\\"reason\\":\\"compiler-artifact\\",` +
    `\\"target\\":{\\"name\\":\\"$1\\"},\\"executable\\":\\"$native\\"}"\n  fi\n  shift\ndone\n`;
  // rustup installs cargo as a symlink proxy; resolution must follow it.
  writeFileSync(`${cargo}-real`, body); chmodSync(`${cargo}-real`, 0o755);
  symlinkSync(`${cargo}-real`, cargo); return {bin, native, args};
}
const verified = () => ({root: '/inputs', inputs: [
  ['typescript', '/inputs/typescript/ts'], ['profiles', '/inputs/profiles/profiles'],
  ['grammar-archives', '/inputs/grammar/grammar'],
].map(([input, dir]) => ({input, dir}))});
const dirs = root => ({PRISM_TYPESCRIPT: `${root}/typescript.js`,
  PRISM_CALLABLE_PROFILES: `${root}/profiles`, PRISM_GRAMMAR_ARCHIVES: `${root}/grammar`});
function options(root, cargo, verify = verified) {
  const inputRoot = join(root, 'input-root'); mkdirSync(inputRoot, {recursive: true});
  return {root, inherited: {HOME: '/home/test', PATH: `${cargo.bin}:${process.env.PATH}`},
    verify, dirs, inputRoot, exclusions: [], natives};
}
function populationOf(root, exclusions = []) {
  const env = buildEnv({inherited: {PATH: process.env.PATH}});
  return population({root, git: resolveTool('git', env.PATH), env, exclusions});
}
const expected=(active,exclusions=[])=>({active,exclusions,digest:sha(JSON.stringify(active))});
const throws = (fn, message) => assert.throws(fn, {message});
function fullReceipt(plan, extra = {}) {
  const rows = extra.native ?? native(), env = {...plan.env, ...nativeEnv(rows)};
  return {repo: plan.repo, population: plan.population, tools: plan.tools, runtime: plan.runtime,
    inputs: plan.inputs, native: rows, argv: plan.argv, cwd: plan.cwd, env, totals: null,
    skips: [], status: 'refused', stage: 'preflight', exitCode: 2, error: null, ...extra};
}
function blankReceipt(platform, error) {
  return {repo: {head: null, dirty: null}, population: null, tools: null,
    runtime: {node: process.version, platform, arch: process.arch}, inputs: null,
    native: native(), argv: null, cwd: null, env: null, totals: null, skips: [],
    status: 'refused', stage: 'preflight', exitCode: 2, error};
}
async function execute(root,cargo,label) { const setup=options(root,cargo),plan=prepare(setup),
  out = join(root, 'target', label);
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
  assert.deepEqual(populationOf(root), expected(active));
  put(root, 'b.test.mjs', pass);
  throws(() => populationOf(root), 'untracked test module: commit or remove b.test.mjs');
  rmSync(join(root, 'b.test.mjs'));
  throws(() => populationOf(root, [{path: 'stale.test.mjs', reason: 'x'}]),
    'stale exclusion: stale.test.mjs');
  git(root, ['rm', '-q', 'a.test.mjs']);
  assert.throws(() => populationOf(root), {message: 'population index drift: a.test.mjs'});
  git(root, ['reset', '--hard', 'HEAD']); put(root, 'added.test.mjs', pass);
  git(root, ['add', 'added.test.mjs']);
  assert.throws(() => populationOf(root), {message: 'population index drift: added.test.mjs'});
  git(root, ['reset', '--hard', 'HEAD']); rmSync(join(root, 'added.test.mjs'), {force: true});
  git(root, ['mv', 'ignored.test.mjs', 'renamed.test.mjs']);
  throws(() => populationOf(root), 'population index drift: ignored.test.mjs, renamed.test.mjs');
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
  const built = build(plan);
  assert.ok(built.PRISM_NATIVE_PARAMETER_EXAMPLE, 'missing PRISM_NATIVE_PARAMETER_EXAMPLE');
  assert.deepEqual(built, native(cargo));
  assert.deepEqual(readFileSync(cargo.args, 'utf8').trim().split('\n'),
    ['build', '--frozen', '--offline', '--message-format=json', '--example', membership,
      '--example', parameter]);
  const bad = fakeCargo(fixture('b3-fail', {'.gitignore': 'target/\n', 'a.test.mjs': pass}), true);
  const badRoot = dirname(bad.bin), badSetup = options(badRoot, bad), badPlan = prepare(badSetup);
  const out = join(badRoot, 'target', 'out'), result = await gate({out, ...badSetup});
  const expected = fullReceipt(badPlan,
    {stage: 'build', error: 'build: cargo failed: build fault'});
  assert.deepEqual({exitCode: result.exitCode, receipt: result.receipt},
    {exitCode: 2, receipt: expected});
  assert.deepEqual(JSON.parse(readFileSync(join(out, 'receipt.json'))), expected);
  assert.equal(existsSync(join(out, 'log.txt')), false);
  const missingRoot = fixture('b3-missing', {'.gitignore': 'target/\n', 'a.test.mjs': pass});
  const missingCargo = fakeCargo(missingRoot, false, parameter);
  const missing = (await execute(missingRoot, missingCargo, 'out')).result;
  assert.deepEqual([missing.receipt.stage, missing.receipt.error],
    ['build', `build: native executable missing: ${parameter}`]);
  const duplicate = (env, example, message) => assert.throws(
    () => prepare({...setup,natives:[{env:'x',example:membership},{env,example}]}), {message});
  duplicate('x', parameter, 'preflight: duplicate native env: x');
  duplicate('y', membership, `preflight: duplicate native example: ${membership}`);
});

test('B4 streams failed TAP output, records totals, and names skips', async () => {
  const root = fixture('b4', {'.gitignore': 'target/\n', 'pass.test.mjs': pass,
    'fail.test.mjs': fail});
  const cargo = fakeCargo(root), failed = await execute(root, cargo, 'failed');
  assert.deepEqual(failed.result.receipt, fullReceipt(failed.plan, {native: native(cargo),
    totals: {tests: 2, pass: 1, fail: 1, skipped: 0},
    status: 'failed', stage: 'tests', exitCode: 1}));
  const log = readFileSync(join(failed.out, 'log.txt'), 'utf8');
  assert.match(log, /passing/); assert.match(log, /failing/);
  const skippedRoot = fixture('b4-skip', {'.gitignore': 'target/\n', 'skip.test.mjs': skip});
  const skippedC=fakeCargo(skippedRoot), skipped=await execute(skippedRoot,skippedC,'skipped');
  assert.deepEqual(skipped.result.receipt, fullReceipt(skipped.plan, {
    native: native(skippedC),
    totals: {tests: 1, pass: 0, fail: 0, skipped: 1},
    skips: ['skipped'], status: 'passed', stage: 'tests', exitCode: 0,
  }));
});

test('B5 refuses missing tools and a tampered test-root input before tests', async () => {
  const root = fixture('b5', {'.gitignore': 'target/\n', 'a.test.mjs': pass});
  const noGit=join(root,'none'); mkdirSync(noGit); const missingOut=join(root,'target','missing');
  const missing = await gate({out: missingOut, root, inherited: {PATH: noGit},
    verify: verified, dirs, exclusions: [], natives});
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

test('B9 preserves preflight stderr and refuses an empty population before build', async () => {
  const root=fixture('b9',{'.gitignore':'target/\n','a.test.mjs':pass}),cargo=fakeCargo(root);
  const preRoot = fixture('b9-pre', {'.gitignore': 'target/\n', 'a.test.mjs': pass});
  const pre = fakeCargo(preRoot, 'preflight');
  const preflight = await gate({out: join(preRoot, 'target', 'out'), ...options(preRoot, pre)});
  const empty = await gate({out: join(root, 'target', 'empty'), ...options(root, cargo),
    exclusions: [{path: 'a.test.mjs', reason: 'excluded'}]});
  assert.deepEqual([preflight.receipt.error, empty.receipt.error, existsSync(cargo.args)],
    ['preflight: cargo failed: preflight fault', 'population: no active tests', false]);
  const wideRoot = fixture('b9-wide', {'.gitignore': 'target/\n', 'a.test.mjs': pass});
  const wide = fakeCargo(wideRoot, '#!/bin/sh\n[ "$1" = --version ] && exit 0\n' +
    `printf '\\377${'\u00e9'.repeat(1100)}END' >&2\nexit 101\n`);
  const w = (await gate({out: join(wideRoot, 'target', 'o'), ...options(wideRoot, wide)})).receipt;
  assert.deepEqual([w.stage, /^build: cargo failed: .*\u00e9END$/s.test(w.error)], ['build', true]);
  const gitRoot = fixture('b9-git', {'.gitignore': 'target/\n', 'a.test.mjs': pass});
  const gc = fakeCargo(gitRoot), shim = '#!/bin/sh\n[ "$1" = --version ] && echo git && exit 0\n';
  put(gc.bin, 'git', `${shim}echo locator missing >&2\nexit 1\n`);
  chmodSync(join(gc.bin, 'git'), 0o755);
  const g = await gate({out: join(gitRoot, 'target', 'o'), ...options(gitRoot, gc)});
  assert.equal(g.receipt.error, 'population: git ls-tree failed: locator missing');
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
    env: {...plan.env, PRISM_MEMBERSHIP_NATIVE: '<resolved after build>',
      PRISM_NATIVE_PARAMETER_EXAMPLE: '<resolved after build>'}});
  assert.equal(existsSync(out), false); const result = await gate({out, ...setup});
  assert.deepEqual(result.receipt, fullReceipt(plan, {
    native: native(cargo),
    totals: {tests: 1, pass: 1, fail: 0, skipped: 0}, status: 'passed', stage: 'tests', exitCode: 0,
  }));
  put(setup.inputRoot, 'sealed', 'input');
  const digest = sha(JSON.stringify(readdirSync(setup.inputRoot)));
  const overlap = join(setup.inputRoot, 'out'), refused = await gate({out: overlap, ...setup});
  assert.deepEqual(refused, {exitCode: 2, error: 'output: overlaps gate-input root'});
  assert.equal(existsSync(overlap), false);
  assert.equal(sha(JSON.stringify(readdirSync(setup.inputRoot))), digest);
  const absent = join(root, 'target', 'absent-root');
  assert.deepEqual(await gate({out: absent, ...setup, inputRoot: absent}),
    {exitCode: 2, error: 'output: overlaps gate-input root'});
  assert.equal(existsSync(absent), false);
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
  const cargo = fakeCargo(root), dash = await execute(root, cargo, 'dash');
  assert.deepEqual(dash.result.receipt, fullReceipt(dash.plan, {native: native(cargo),
    totals: {tests: 1, pass: 1, fail: 0, skipped: 0},
    status: 'passed', stage: 'tests', exitCode: 0,
  }));
  assert.equal(dash.result.receipt.argv.at(-1), join(root, '--dash.test.mjs'));
  const out = join(root, 'target', 'linux');
  const linux = await gate({out, ...options(root, cargo), platform: 'linux'});
  assert.deepEqual(linux.receipt, blankReceipt('linux', 'unsupported host'));
});
