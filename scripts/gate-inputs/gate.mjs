#!/usr/bin/env node
import {createHash} from 'node:crypto';
import {accessSync, createWriteStream, lstatSync, mkdirSync, readFileSync, statSync,
  writeFileSync} from 'node:fs';
import {X_OK} from 'node:constants';
import {basename, dirname, join, resolve} from 'node:path';
import {spawn, spawnSync} from 'node:child_process';
import {once} from 'node:events';
import {fileURLToPath, pathToFileURL} from 'node:url';
import exclusionsFile from './exclusions.json' with {type: 'json'};
import {inputDirs, verifyInstalled} from './acquire.mjs';

const passThrough = ['HOME', 'PATH', 'LANG', 'LC_ALL', 'TMPDIR', 'CARGO_HOME', 'RUSTUP_HOME',
  'RUSTUP_TOOLCHAIN'];
const sha = value => createHash('sha256').update(value).digest('hex');
class Refusal extends Error { constructor(m, s = 'preflight') { super(m); this.stage = s; } }
const refuse = (message, stage) => { throw new Refusal(message, stage); };
const canonical = v => Array.isArray(v) ? v.map(canonical)
  : v && typeof v === 'object'
    ? Object.fromEntries(Object.keys(v).sort().map(k => [k, canonical(v[k])])) : v;
const json = value => `${JSON.stringify(canonical(value), null, 2)}\n`;
const strictUtf8 = new TextDecoder('utf-8', {fatal: true});
const utf8=(v,l)=>{try{return strictUtf8.decode(v);}catch{refuse(`${l}: invalid utf8`);}};
const call = (file, args, cwd, env) => spawnSync(file, args, {cwd, env, encoding: null});

export const repoRoot = () => resolve(dirname(fileURLToPath(import.meta.url)), '../..');
export function buildEnv({inherited = process.env, inputs = {}} = {}) {
  const env = {};
  for (const key of passThrough) if (inherited[key] !== undefined) env[key] = inherited[key];
  env.PATH = `${dirname(process.execPath)}:${inherited.PATH ?? ''}`;
  env.GIT_CONFIG_GLOBAL = '/dev/null'; env.GIT_CONFIG_NOSYSTEM = '1';
  return {...env, ...inputs};
}
export function resolveTool(name, path) {
  for (const dir of path.split(':')) {
    const file = resolve(dir || '.', name);
    try { if (statSync(file).isFile()) { accessSync(file, X_OK); return file; } } catch {}
  }
  refuse(`preflight: ${name} not found`);
}
function commandText(file, args, cwd, env, label) {
  const result = call(file, args, cwd, env);
  if (result.error || result.status !== 0) refuse(`preflight: ${label} failed`);
  return utf8(result.stdout ?? Buffer.alloc(0), label).trim();
}
function nulPaths(bytes, label) {
  const text = utf8(bytes, label);
  if (text && !text.endsWith('\0')) refuse(`${label}: missing NUL terminator`);
  const paths = text ? text.slice(0, -1).split('\0') : [];
  const bad = paths.some(path => !path) || new Set(paths).size !== paths.length;
  if (bad) refuse(`${label}: duplicate or empty path`);
  return paths.filter(path => path.endsWith('.test.mjs'));
}
export function population({root = repoRoot(), git, env, exclusions = exclusionsFile} = {}) {
  const listed = args => {
    const result = call(git, args, root, env);
    if (result.error || result.status !== 0) refuse(`population: git ${args[0]} failed`);
    return nulPaths(result.stdout ?? Buffer.alloc(0), 'population');
  };
  const head = listed(['ls-tree', '-r', '-z', '--name-only', 'HEAD']);
  const cached = listed(['ls-files', '-z', '--cached']);
  const changed = [...head, ...cached].filter(p => head.includes(p) !== cached.includes(p)).sort();
  if (changed.length) refuse(`population index drift: ${changed.join(', ')}`);
  for (const path of head) {
    try { if (!lstatSync(join(root, path)).isFile()) refuse(`population: not regular ${path}`); }
    catch (e) { if (e instanceof Refusal) throw e; refuse(`population: not regular ${path}`); }
  }
  const untracked = listed(['ls-files','-z','--others','--exclude-standard','--','*.test.mjs']);
  if (untracked.length) refuse(`untracked test module: commit or remove ${untracked.join(', ')}`);
  for (const {path} of exclusions) if (!head.includes(path)) refuse(`stale exclusion: ${path}`);
  const excluded = new Set(exclusions.map(row => row.path));
  const active = head.filter(path => !excluded.has(path)).sort();
  return {active, exclusions, digest: sha(JSON.stringify(active))};
}
export function prepare({root = repoRoot(), inherited = process.env, platform = process.platform,
  verify = verifyInstalled, dirs = inputDirs, exclusions = exclusionsFile, inputRoot} = {}) {
  if (platform !== 'darwin') refuse('unsupported host');
  const base = buildEnv({inherited});
  const git = resolveTool('git', base.PATH), cargo = resolveTool('cargo', base.PATH);
  const tools = {git: {path: git, version: commandText(git, ['--version'], root, base, 'git')},
    cargo: {path: cargo, version: commandText(cargo, ['--version'], root, base, 'cargo')},
    node: {path: process.execPath, version: process.version}};
  let verified; try { verified = verify({root: inputRoot}); } catch (e) { refuse(e.message); }
  const inputEnv = dirs(verified.root), env = buildEnv({inherited, inputs: inputEnv});
  const listed = population({root, git, env, exclusions});
  const head = commandText(git, ['rev-parse', 'HEAD'], root, env, 'git rev-parse');
  const dirty = commandText(git, ['status', '--porcelain'], root, env, 'git status') !== '';
  const argv = [process.execPath, '--test', '--test-concurrency=2', '--test-reporter=tap',
    ...listed.active.map(path => resolve(root, path))];
  const inputs = {root: verified.root, directories: verified.inputs.map(({input, dir}) => ({
    input, dir, digest: basename(dir),
  })), env: inputEnv};
  const runtime = {node: process.version, platform, arch: process.arch};
  return {cwd: root, repo: {head, dirty}, tools, runtime, inputs, population: listed, env, argv};
}
export function build(plan) {
  const result = call(plan.tools.cargo.path, ['build', '--frozen', '--offline', '--example',
    'project_membership_census', '--message-format=json'], plan.cwd, plan.env);
  if (result.error || result.status !== 0) refuse('build: cargo failed', 'build');
  let executable;
  for (const line of utf8(result.stdout ?? Buffer.alloc(0), 'build').split('\n')) {
    try {
      const message = JSON.parse(line);
      const name = message.target?.name;
      if (message.reason === 'compiler-artifact' && name === 'project_membership_census'
        && message.executable) executable = message.executable;
    } catch {}
  }
  if (!executable) refuse('build: native executable missing', 'build');
  try { return {path: executable, sha256: sha(readFileSync(executable))}; }
  catch { refuse('build: native executable unreadable', 'build'); }
}
function tap(text) {
  const values = Object.fromEntries(['tests', 'pass', 'fail', 'skipped'].map(key => {
    const match = text.match(new RegExp(`^# ${key} (\\d+)$`, 'm'));
    return [key, match ? Number(match[1]) : null];
  }));
  const totals = Object.values(values).every(value => value !== null) ? values : null;
  const skips = [...text.matchAll(/ - (.*?) # SKIP\b.*$/gm)].map(match => match[1]);
  return {totals, skips};
}
async function runTests(plan, out) {
  const log = createWriteStream(join(out, 'log.txt')); let text = '', code = 1;
  const child = spawn(process.execPath, plan.argv.slice(1), {cwd: plan.cwd, env: plan.env});
  for (const stream of [child.stdout, child.stderr]) {
    stream.on('data', chunk => { text += chunk; log.write(chunk); });
  }
  await new Promise(done => {
    child.on('error', () => done()); child.on('close', value => { code = value ?? 1; done(); });
  });
  log.end(); await once(log, 'finish');
  return {code, ...tap(text)};
}
function claimOutput(out) {
  const leaf = resolve(out); mkdirSync(dirname(leaf), {recursive: true});
  mkdirSync(leaf); return leaf;
}
function emptyReceipt(platform) {
  return {repo: {head: null, dirty: null}, population: null, tools: null,
    runtime: {node: process.version, platform, arch: process.arch}, inputs: null,
    native: {path: null, sha256: null}, argv: null, cwd: null, env: null, totals: null,
    skips: [],
    status: 'refused', stage: 'preflight', exitCode: 2, error: null};
}
export function dryRun(options = {}) {
  const plan = prepare(options);
  return {population: plan.population, argv: plan.argv, cwd: plan.cwd,
    env: {...plan.env, PRISM_MEMBERSHIP_NATIVE: '<resolved after build>'}};
}
export async function gate(options = {}) {
  let out;
  try { out = claimOutput(options.out); }
  catch (error) { return {exitCode: 2, error: `output: ${error.message}`}; }
  const receipt = emptyReceipt(options.platform ?? process.platform);
  try {
    const plan = prepare(options);
    Object.assign(receipt, {repo: plan.repo, population: plan.population, tools: plan.tools,
      runtime: plan.runtime, inputs: plan.inputs, argv: plan.argv, cwd: plan.cwd, env: plan.env});
    const native = build(plan); plan.env.PRISM_MEMBERSHIP_NATIVE = native.path;
    receipt.native = native; receipt.env = plan.env;
    const result = await runTests(plan, out);
    receipt.totals = result.totals; receipt.skips = result.skips;
    receipt.status = result.code === 0 ? 'passed' : 'failed'; receipt.stage = 'tests';
    receipt.exitCode = result.code === 0 ? 0 : 1;
  } catch (error) {
    receipt.error = error.message; receipt.stage = error.stage ?? 'preflight';
  }
  writeFileSync(join(out, 'receipt.json'), json(receipt));
  return {exitCode: receipt.exitCode, out, receipt, error: receipt.error};
}
async function main() {
  const args = process.argv.slice(2);
  if (args.join() === '--dry-run') return process.stdout.write(json(dryRun()));
  if (args.length !== 2 || args[0] !== '--out') refuse('cli: usage --out <new-dir>|--dry-run');
  const result = await gate({out: args[1]});
  if (result.error) process.stderr.write(`${result.error}\n`);
  process.exitCode = result.exitCode;
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main().catch(error => (process.stderr.write(`${error.message}\n`), process.exitCode = 2));
}
