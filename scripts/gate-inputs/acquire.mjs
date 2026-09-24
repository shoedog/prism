#!/usr/bin/env node
import {createHash, randomUUID} from 'node:crypto';
import {
  existsSync, lstatSync, mkdirSync, readdirSync, readFileSync, realpathSync,
  renameSync, rmSync, writeFileSync,
} from 'node:fs';
import {homedir, tmpdir} from 'node:os';
import {dirname, join, resolve, sep} from 'node:path';
import {pathToFileURL} from 'node:url';
import {gunzipSync} from 'node:zlib';
import pinsFile from './pins.json' with {type: 'json'};

const LIMIT = 64 * 1024 * 1024;
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const sri = bytes => `sha512-${createHash('sha512').update(bytes).digest('base64')}`;
class Refusal extends Error {}
const refuse = message => { throw new Refusal(message); };
const order = (a, b) => a < b ? -1 : a > b ? 1 : 0;
const pathRule = value => /^[\x20-\x7e]+$/.test(value) && !value.includes('\\');

function rootValue(root) {
  if (root !== undefined) return root;
  if (process.env.PRISM_GATE_INPUTS_ROOT) return process.env.PRISM_GATE_INPUTS_ROOT;
  if (process.env.XDG_DATA_HOME) return join(process.env.XDG_DATA_HOME, 'prism/gate-inputs');
  return join(homedir(), '.local/share/prism/gate-inputs');
}
function existingPrefix(root) {
  let prefix = root;
  while (!existsSync(prefix)) {
    const parent = dirname(prefix);
    if (parent === prefix) refuse(`root: inaccessible ${root}`);
    prefix = parent;
  }
  return realpathSync(prefix);
}
function isUnder(child, parent) { return child === parent || child.startsWith(parent + sep); }
export function resolveRoot(root, platform = process.platform) {
  if (platform !== 'darwin') refuse('root: unsupported host');
  const value = rootValue(root);
  if (typeof value !== 'string' || /[\x00-\x1f\x7f]/.test(value)) refuse('root: control character');
  const absolute = resolve(value), prefix = existingPrefix(absolute);
  for (const denied of [tmpdir(), '/tmp', '/var/folders']) {
    if (existsSync(denied) && isUnder(prefix, realpathSync(denied))) refuse('root: volatile root');
  }
  mkdirSync(absolute, {recursive: true});
  return absolute;
}
function safeRelative(value, label) {
  const valid = typeof value === 'string'
    && value.split('/').every(p => p && p !== '.' && p !== '..' && pathRule(p));
  if (!valid) {
    refuse(`pin: ${label}`);
  }
  return value;
}
function destinationPath(value) { return value === '.' ? '' : safeRelative(value, 'dest'); }
function installed(root, input, pin) { return join(root, input, pin.tree_sha256); }
export function inputDirs(root, pins = pinsFile) {
  return Object.fromEntries(Object.entries(pins.inputs).map(([input, pin]) =>
    [pin.env, join(installed(root, input, pin), pin.env_suffix)]));
}
export function treeDigest(root) {
  const rows = [];
  function walk(dir, prefix = '') {
    const entries = readdirSync(dir, {withFileTypes: true}).sort((a, b) => order(a.name, b.name));
    for (const entry of entries) {
      const rel = prefix + entry.name, file = join(dir, entry.name), stat = lstatSync(file);
      const bad = stat.isSymbolicLink() || (!stat.isDirectory() && !stat.isFile());
      if (bad) refuse(`tree: unsupported ${rel}`);
      if (stat.isDirectory()) walk(file, rel + '/');
      else {
        if (!pathRule(rel)) refuse(`tree: non-ascii path ${rel}`);
        rows.push([rel, sha(readFileSync(file))]);
      }
    }
  }
  walk(root);
  rows.sort((a, b) => order(a[0], b[0]));
  return sha(JSON.stringify(rows));
}
function historicalTree(root, prefix = '') {
  return readdirSync(root).sort(order).flatMap(name => {
    const file = join(root, name), rel = prefix + name, stat = lstatSync(file);
    const bad = stat.isSymbolicLink() || (!stat.isDirectory() && !stat.isFile());
    if (bad) refuse(`profile: unsupported ${rel}`);
    return stat.isDirectory() ? historicalTree(file, rel + '/')
      : [[rel, readFileSync(file, 'utf8')]];
  });
}
function verifyCrossChecks(dir, pin) {
  for (const [name, expected] of Object.entries(pin.file_sha256 ?? {})) {
    const actual = sha(readFileSync(join(dir, safeRelative(name, 'file_sha256'))));
    if (actual !== expected) refuse(`file hash: ${name}`);
  }
  for (const [profile, expected] of Object.entries(pin.profile_hash ?? {})) {
    const profileDir = join(dir, safeRelative(profile, 'profile'));
    const actual = sha(JSON.stringify(historicalTree(profileDir)));
    if (actual !== expected) refuse(`profile hash: ${profile}`);
    const versions = pin.versions?.[profile] ?? {};
    for (const [pkg, version] of Object.entries(versions)) {
      const file = pkg === 'react' ? 'node_modules/@types/react/package.json'
        : pkg === 'prop-types' ? 'node_modules/@types/prop-types/package.json'
          : `node_modules/${pkg}/package.json`;
      if (JSON.parse(readFileSync(join(profileDir, file), 'utf8')).version !== version) {
        refuse(`profile version: ${profile}/${pkg}`);
      }
    }
  }
}
function verifiedInput(root, input, pin) {
  const dir = installed(root, input, pin);
  if (!existsSync(dir)) refuse(`${input}: install missing: ${dir}; run acquire`);
  const actual = treeDigest(dir);
  if (actual !== pin.tree_sha256) refuse(`${input}: install corrupt: remove ${dir} and re-run`);
  return dir;
}
export function verifyInstalled({root, pins = pinsFile, platform = process.platform} = {}) {
  const resolved = resolveRoot(root, platform);
  const inputs = Object.entries(pins.inputs).map(([input, pin]) =>
    ({input, dir: verifiedInput(resolved, input, pin)}));
  return {root: resolved, inputs};
}
async function bodyBytes(fetch, artifact) {
  let response;
  try { response = await fetch(artifact.url, {signal: AbortSignal.timeout(120_000)}); }
  catch { refuse(`artifact ${artifact.name}: fetch failed`); }
  const status = response?.status ?? 'unknown';
  if (!response?.ok) refuse(`artifact ${artifact.name}: fetch status ${status}`);
  if (!response.body) refuse(`artifact ${artifact.name}: empty body`);
  const chunks = []; let total = 0;
  for await (const chunk of response.body) {
    total += chunk.length;
    if (total > LIMIT) refuse(`artifact ${artifact.name}: body cap`);
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks);
}
function authenticate(artifact, bytes) {
  const actual = artifact.integrity ? sri(bytes) : sha(bytes);
  const expected = artifact.integrity ?? artifact.sha256;
  if (actual !== expected) refuse(`artifact ${artifact.name}: byte authority`);
}
function cString(buffer, start, end) {
  const zero = buffer.subarray(start, end).indexOf(0);
  return buffer.subarray(start, zero < 0 ? end : start + zero).toString('utf8');
}
function octal(header) {
  const value = cString(header, 124, 136).trim();
  if (!/^[0-7]+$/.test(value)) refuse('tar: invalid size');
  return Number.parseInt(value, 8);
}
function tarPath(name, root, type) {
  const normalized = type === 53 && name.endsWith('/') ? name.slice(0, -1) : name;
  if (type !== 53 && name.endsWith('/')) refuse(`tar path: regular trailing slash ${name}`);
  if (!(normalized === root || normalized.startsWith(root + '/'))) refuse(`tar root: ${name}`);
  const rest = normalized.slice(root.length).replace(/^\//, '');
  if (!rest) return '';
  const valid = rest.split('/').every(p => p && p !== '.' && p !== '..' && pathRule(p));
  if (!valid) refuse(`tar path: ${name}`);
  return rest;
}
function extractTar(bytes, artifact, stage) {
  let tar; try { tar = gunzipSync(bytes); } catch { refuse(`artifact ${artifact.name}: gunzip`); }
  const seen = new Set(); let offset = 0, files = 0, dirs = 0;
  while (true) {
    if (offset + 512 > tar.length) refuse(`artifact ${artifact.name}: truncated`);
    const header = tar.subarray(offset, offset + 512);
    if (header.every(byte => byte === 0)) return {files, dirs};
    if (header.subarray(257, 263).toString('ascii') !== 'ustar\0') {
      refuse(`artifact ${artifact.name}: ustar`);
    }
    const name = cString(header, 0, 100), type = header[156], size = octal(header);
    if (Buffer.byteLength(name) > 51) refuse(`artifact ${artifact.name}: tar name`);
    if (header[345] !== 0) refuse(`artifact ${artifact.name}: ustar prefix unsupported`);
    if (![0, 48, 53].includes(type)) {
      const kind = String.fromCharCode(type);
      refuse(`artifact ${artifact.name}: unsupported tar entry ${kind} ${name}`);
    }
    if (type === 53 && size !== 0) refuse(`artifact ${artifact.name}: directory size: ${name}`);
    let rest;
    try { rest = tarPath(name, artifact.root, type); }
    catch (error) { refuse(`artifact ${artifact.name}: ${error.message}`); }
    const destination = join(stage, destinationPath(artifact.dest), rest);
    if (rest && seen.has(rest)) {
      refuse(`artifact ${artifact.name}: duplicate destination ${rest}`);
    }
    if (rest) seen.add(rest);
    const dataStart = offset + 512, dataEnd = dataStart + size;
    if (dataEnd > tar.length) refuse(`artifact ${artifact.name}: truncated`);
    if (type === 53) { if (rest) mkdirSync(destination, {recursive: true}); dirs++; }
    else {
      if (!rest) refuse(`artifact ${artifact.name}: tar path: ${name}`);
      mkdirSync(dirname(destination), {recursive: true});
      writeFileSync(destination, tar.subarray(dataStart, dataEnd));
      files++;
    }
    offset = dataStart + Math.ceil(size / 512) * 512;
  }
}
function rawPath(stage, artifact) { return join(stage, destinationPath(artifact.dest)); }
async function installInput({fetch, root, input, pin, onBeforeRename}) {
  const final = installed(root, input, pin);
  if (existsSync(final)) { verifiedInput(root, input, pin); return {input, status: 'verified'}; }
  const downloads = [];
  for (const artifact of pin.artifacts) {
    try {
      const bytes = await bodyBytes(fetch, artifact);
      authenticate(artifact, bytes);
      downloads.push([artifact, bytes]);
    }
    catch (error) { refuse(`${input} ${error.message}`); }
  }
  let stage = join(root, `.stage-${randomUUID()}`);
  try {
    mkdirSync(stage); const artifacts = [];
    for (const [artifact, bytes] of downloads) {
      try {
        const entry = artifact.root ? extractTar(bytes, artifact, stage)
          : (mkdirSync(dirname(rawPath(stage, artifact)), {recursive: true}),
            writeFileSync(rawPath(stage, artifact), bytes),
            {files: 1, dirs: 0, raw: true});
        artifacts.push({name: artifact.name, ...entry});
      } catch (error) { refuse(`${input} ${error.message}`); }
    }
    try { verifyCrossChecks(stage, pin); } catch (error) { refuse(`${input} ${error.message}`); }
    if (treeDigest(stage) !== pin.tree_sha256) refuse(`${input}: tree digest`);
    mkdirSync(dirname(final), {recursive: true});
    await onBeforeRename?.({input, stage, final});
    try {
      renameSync(stage, final);
      stage = undefined;
      return {input, status: 'installed', artifacts};
    }
    catch (error) {
      if (!['EEXIST', 'ENOTEMPTY'].includes(error.code)) throw error;
      rmSync(stage, {recursive: true, force: true}); stage = undefined;
      verifiedInput(root, input, pin); return {input, status: 'verified'};
    }
  } finally { if (stage) rmSync(stage, {recursive: true, force: true}); }
}
export async function acquire({
  fetch = globalThis.fetch, root, pins = pinsFile, platform = process.platform, onBeforeRename,
} = {}) {
  let resolved;
  try { resolved = resolveRoot(root, platform); }
  catch (error) { return {ok: false, inputs: [], error: error.message}; }
  const inputs = [];
  for (const [input, pin] of Object.entries(pins.inputs)) {
    try { inputs.push(await installInput({fetch, root: resolved, input, pin, onBeforeRename})); }
    catch (error) { inputs.push({input, status: 'refused', error: error.message}); }
  }
  return {ok: inputs.every(entry => entry.status !== 'refused'), root: resolved, inputs};
}
export function envLines({root, pins = pinsFile, platform = process.platform} = {}) {
  const verified = verifyInstalled({root, pins, platform});
  return Object.entries(inputDirs(verified.root, pins)).map(([name, value]) => {
    return `export ${name}='${value.replaceAll("'", "'\\''")}'`;
  });
}
function print(result) {
  for (const entry of result.inputs) {
    if (entry.error) console.error(entry.error);
    else if (entry.artifacts) for (const artifact of entry.artifacts) {
      const label = `${entry.input} ${artifact.name}`;
      console.log(`${label}: verified entries: ${artifact.files} files, ${artifact.dirs} dirs`);
    } else console.log(`${entry.input}: ${entry.status}`);
  }
  if (result.error) console.error(result.error);
}
async function main() {
  const command = process.argv[2] ?? 'acquire';
  if (!['acquire', 'verify', 'env'].includes(command)) refuse(`cli: usage acquire|verify|env`);
  if (command === 'acquire') {
    const result = await acquire();
    print(result);
    process.exitCode = result.ok ? 0 : 2;
    return;
  }
  try {
    if (command === 'verify') {
      const verified = verifyInstalled();
      print({inputs: verified.inputs.map(({input}) => ({input, status: 'verified'}))});
    } else for (const line of envLines()) console.log(line);
  } catch (error) { console.error(error.message); process.exitCode = 2; }
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) main();
