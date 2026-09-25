#!/usr/bin/env node
import {createHash} from 'node:crypto';
import {accessSync, createWriteStream, existsSync, lstatSync, mkdirSync, readFileSync,
  realpathSync, statSync, writeFileSync} from 'node:fs';
import {X_OK} from 'node:constants';
import {basename, dirname, join, resolve, sep} from 'node:path';
import {spawn, spawnSync} from 'node:child_process';
import {once} from 'node:events';
import {fileURLToPath, pathToFileURL} from 'node:url';
import exclusionsFile from './exclusions.json' with {type: 'json'};
import nativesFile from './natives.json' with {type: 'json'};
import {inputDirs, inputRoot, verifyInstalled} from './acquire.mjs';

const passThrough = ['HOME', 'PATH', 'LANG', 'LC_ALL', 'TMPDIR', 'CARGO_HOME', 'RUSTUP_HOME',
  'RUSTUP_TOOLCHAIN'];
const sha = value => createHash('sha256').update(value).digest('hex');
const nativeMap = (rows, fn) => Object.fromEntries(rows.map(row => [row.env, fn(row)]).sort());
class Refusal extends Error { constructor(m, s = 'preflight') { super(m); this.stage = s; } }
const refuse = (message, stage) => { throw new Refusal(message, stage); };
const canonical = v => Array.isArray(v) ? v.map(canonical)
  : v && typeof v === 'object'
    ? Object.fromEntries(Object.keys(v).sort().map(k => [k, canonical(v[k])])) : v;
const json = value => `${JSON.stringify(canonical(value), null, 2)}\n`;
const strictUtf8 = new TextDecoder('utf-8', {fatal: true});
const utf8=(v,l)=>{try{return strictUtf8.decode(v);}catch{refuse(`${l}: invalid utf8`);}};
const call = (file, args, cwd, env) => spawnSync(file, args, {cwd, env, encoding: null});
const stderr = result => new TextDecoder().decode(result.stderr ?? Buffer.alloc(0)).slice(-2048)
  .trim() || String(result.error?.message ?? '');

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
  if (result.error || result.status !== 0)
    refuse(`preflight: ${label} failed: ${stderr(result)}`);
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
function nativeRows(rows) {
  for (const key of ['env', 'example']) {
    const values = rows.map(row => row[key]), dup = values.find((v, i) => values.indexOf(v) < i);
    if (dup) refuse(`preflight: duplicate native ${key}: ${dup}`);
  } return rows;
}
export function population({root = repoRoot(), git, env, exclusions = exclusionsFile} = {}) {
  const listed = args => {
    const result = call(git, args, root, env);
    if (result.error || result.status !== 0)
      refuse(`population: git ${args[0]} failed: ${stderr(result)}`);
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
  verify = verifyInstalled, dirs = inputDirs, exclusions = exclusionsFile, inputRoot,
  natives = nativesFile} = {}) {
  if (platform !== 'darwin') refuse('unsupported host');
  const native = nativeRows(natives);
  const base = buildEnv({inherited});
  const git = resolveTool('git', base.PATH), cargo = resolveTool('cargo', base.PATH);
  const tools = {git: {path: git, version: commandText(git, ['--version'], root, base, 'git')},
    cargo: {path: cargo, version: commandText(cargo, ['--version'], root, base, 'cargo')},
    node: {path: process.execPath, version: process.version}};
  let verified; try { verified = verify({root: inputRoot}); } catch (e) { refuse(e.message); }
  const inputEnv = dirs(verified.root), env = buildEnv({inherited, inputs: inputEnv});
  const listed = population({root, git, env, exclusions});
  if (!listed.active.length) refuse('population: no active tests');
  const head = commandText(git, ['rev-parse', 'HEAD'], root, env, 'git rev-parse');
  const dirty = commandText(git, ['status', '--porcelain'], root, env, 'git status') !== '';
  const argv = [process.execPath, '--test', '--test-concurrency=2', '--test-reporter=tap',
    ...listed.active.map(path => resolve(root, path))];
  const inputs = {root: verified.root, directories: verified.inputs.map(({input, dir}) => ({
    input, dir, digest: basename(dir),
  })), env: inputEnv};
  const runtime = {node: process.version, platform, arch: process.arch};
  return {cwd: root, repo: {head, dirty}, tools, runtime, inputs, population: listed, env,
    natives: native, argv};
}
export function build(plan) {
  const args = ['build', '--frozen', '--offline', '--message-format=json',
    ...plan.natives.flatMap(({example}) => ['--example', example])];
  const result = call(plan.tools.cargo.path, args, plan.cwd, plan.env);
  if (result.error || result.status !== 0) refuse(`build: cargo failed: ${stderr(result)}`,'build');
  const executables = new Map();
  for (const line of utf8(result.stdout ?? Buffer.alloc(0), 'build').split('\n')) {
    try {
      const message = JSON.parse(line);
      if (message.reason === 'compiler-artifact' && message.executable
        && plan.natives.some(row => row.example === message.target?.name))
        executables.set(message.target.name, message.executable);
    } catch {}
  }
  return nativeMap(plan.natives, row => {
    const path = executables.get(row.example);
    if (!path) refuse(`build: native executable missing: ${row.example}`, 'build');
    try { return {example: row.example, path, sha256: sha(readFileSync(path))}; }
    catch { refuse(`build: native executable unreadable: ${row.example}`, 'build'); }
  });
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
  for (const s of [child.stdout, child.stderr]) s.on('data', c => { text += c; log.write(c); });
  await new Promise(done => {
    child.on('error', () => done()); child.on('close', value => { code = value ?? 1; done(); });
  });
  log.end(); await once(log, 'finish');
  return {code, ...tap(text)};
}
function canon(p, rest = '') {
  return existsSync(p) ? join(realpathSync(p), rest) : canon(dirname(p), join(basename(p), rest));
}
function claimOutput(out, root) {
  const leaf = resolve(out), input = canon(resolve(root)), target = canon(leaf);
  if (target === input || target.startsWith(input + sep)) refuse('overlaps gate-input root');
  mkdirSync(dirname(leaf), {recursive: true});
  mkdirSync(leaf); return leaf;
}
function emptyReceipt(platform, natives = nativesFile) {
  return {repo: {head: null, dirty: null}, population: null, tools: null,
    runtime: {node: process.version, platform, arch: process.arch}, inputs: null,
    native: nativeMap(natives, ({example}) => ({example, path: null, sha256: null})), argv: null,
    cwd: null, env: null, totals: null,
    skips: [],
    status: 'refused', stage: 'preflight', exitCode: 2, error: null};
}
export function dryRun(options = {}) {
  const plan = prepare(options);
  return {population: plan.population, argv: plan.argv, cwd: plan.cwd,
    env: {...plan.env, ...nativeMap(plan.natives, () => '<resolved after build>')}};
}
export async function gate(options = {}) {
  let out;
  try { out = claimOutput(options.out, inputRoot(options.inputRoot)); }
  catch (error) { return {exitCode: 2, error: `output: ${error.message}`}; }
  const receipt = emptyReceipt(options.platform ?? process.platform, options.natives);
  try {
    const plan = prepare(options);
    Object.assign(receipt, {repo: plan.repo, population: plan.population, tools: plan.tools,
      runtime: plan.runtime, inputs: plan.inputs, argv: plan.argv, cwd: plan.cwd, env: plan.env});
    const native = build(plan);
    for (const [env, row] of Object.entries(native)) plan.env[env] = row.path;
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
