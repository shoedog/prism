#!/usr/bin/env node
// Reproduce baseline and patched outputs in a fresh temporary directory. Never
// install packages, execute npm lifecycle scripts, or modify the vendor tree.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { gunzipSync } from 'node:zlib';

const pins = {
  'upstream.tar.gz': {
    url: 'https://codeload.github.com/tree-sitter/tree-sitter-typescript/tar.gz/f975a621f4e7f532fe322e13c4f79495e0a7b2e7',
    sha256: '4de2e82e557810eecb93cb3b31fbb3bd28ba4c91ba9a6c6164bcaa074125b7b5',
  },
  'javascript-0.23.1.tgz': {
    url: 'https://registry.npmjs.org/tree-sitter-javascript/-/tree-sitter-javascript-0.23.1.tgz',
    sha256: '90e80b25a67517a4daf6ad751557bee21efbda7b7a5a554897933245d1734398',
  },
  'tree-sitter-macos-arm64.gz': {
    url: 'https://github.com/tree-sitter/tree-sitter/releases/download/v0.24.4/tree-sitter-macos-arm64.gz',
    sha256: '7bee5649df5ae3965e132493c597f9e51772c3812c7751ba862fed5662bd106c',
  },
};
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const vendor = join(repo, 'vendor/tree-sitter-typescript');
const args = process.argv.slice(2);
assert(args.length === 1, 'usage: node scripts/verify-typescript-grammar.mjs <pinned-archive-directory|--download>');
assert.equal(process.version, 'v26.0.0', 'verified generation runtime is Node v26.0.0');
assert.equal(process.platform, 'darwin', 'this bootstrap pins the macOS arm64 generator; other platforms need a separately verified binary pin');
assert.equal(process.arch, 'arm64');
const scratch = mkdtempSync(join(tmpdir(), 'prism-ts-grammar-'));
console.log(`Evidence directory (retained): ${scratch}`);
const inputs = args[0] === '--download' ? scratch : resolve(args[0]);
function run(command, argv, cwd = scratch) {
  const r = spawnSync(command, argv, { cwd, encoding: 'utf8', timeout: 180_000, maxBuffer: 16 * 1024 * 1024 });
  assert.equal(r.error, undefined, String(r.error));
  assert.equal(r.status, 0, `${command}: ${r.stdout}\n${r.stderr}`);
  return r.stdout;
}
for (const [name, pin] of Object.entries(pins)) {
  const file = join(inputs, name);
  if (args[0] === '--download') {
    run('curl', ['--fail', '--location', '--proto', '=https', '--tlsv1.2', '--connect-timeout', '20', '--max-time', '120', pin.url, '-o', file]);
  }
  assert(existsSync(file), `missing pinned archive ${file}`);
  assert.equal(sha(readFileSync(file)), pin.sha256, `archive pin: ${name}`);
}
run('tar', ['-xzf', join(inputs, 'upstream.tar.gz'), '-C', scratch]);
const upstream = join(scratch, 'tree-sitter-typescript-f975a621f4e7f532fe322e13c4f79495e0a7b2e7');
const js = join(scratch, 'node_modules/tree-sitter-javascript');
mkdirSync(js, { recursive: true });
run('tar', ['-xzf', join(inputs, 'javascript-0.23.1.tgz'), '--strip-components=1', '-C', js]);
const lock = JSON.parse(readFileSync(join(upstream, 'package-lock.json')));
assert.equal(lock.packages['node_modules/tree-sitter-cli'].version, '0.24.4');
assert.equal(lock.packages['node_modules/tree-sitter-javascript'].version, '0.23.1');
assert.equal('sha512-' + createHash('sha512').update(readFileSync(join(inputs, 'javascript-0.23.1.tgz'))).digest('base64'), lock.packages['node_modules/tree-sitter-javascript'].integrity);
const cli = join(scratch, 'tree-sitter');
writeFileSync(cli, gunzipSync(readFileSync(join(inputs, 'tree-sitter-macos-arm64.gz'))));
chmodSync(cli, 0o700);
assert.equal(run(cli, ['--version']).trim(), 'tree-sitter 0.24.4 (fc8c1863e2e5724a0c40bb6e6cfc8631bfe5908b)');
function inventory(root, rel = '') {
  return Object.fromEntries(readdirSync(join(root, rel), { withFileTypes: true }).sort((a,b) => a.name.localeCompare(b.name)).flatMap(entry => {
    const name = join(rel, entry.name);
    assert(!entry.isSymbolicLink(), `unexpected generated symlink ${name}`);
    return entry.isDirectory() ? Object.entries(inventory(root, name)) : [[name, sha(readFileSync(join(root, name)))]];
  }));
}
const receipt = { pins, node: process.version, generator_sha256: sha(readFileSync(cli)), dialects: {} };
for (const rel of ['common/scanner.h', 'common/common.mak', 'bindings/rust/lib.rs', 'Cargo.toml', 'LICENSE', 'README.md', 'tree-sitter.json', 'queries/highlights.scm', 'queries/locals.scm', 'queries/tags.scm']) {
  assert.equal(sha(readFileSync(join(vendor, rel))), sha(readFileSync(join(upstream, rel))), `unexpected non-generated upstream change: ${rel}`);
}
for (const dialect of ['typescript', 'tsx']) {
  const dir = join(upstream, dialect);
  const baseline = inventory(join(dir, 'src'));
  run(cli, ['generate', '--abi', '14', '--js-runtime', process.execPath], dir);
  assert.deepEqual(inventory(join(dir, 'src')), baseline, `${dialect}: unchanged baseline generation drift`);
  receipt.dialects[dialect] = { baseline };
}
receipt.authored_grammar_sha256 = sha(readFileSync(join(vendor, 'common/define-grammar.js')));
copyFileSync(join(vendor, 'common/define-grammar.js'), join(upstream, 'common/define-grammar.js'));
for (const dialect of ['typescript', 'tsx']) {
  const dir = join(upstream, dialect);
  assert.equal(sha(readFileSync(join(dir, 'grammar.js'))), sha(readFileSync(join(vendor, dialect, 'grammar.js'))));
  run(cli, ['generate', '--abi', '14', '--js-runtime', process.execPath], dir);
  const patched = inventory(join(dir, 'src'));
  assert.deepEqual(patched, inventory(join(vendor, dialect, 'src')), `${dialect}: shipped generated bytes differ`);
  receipt.dialects[dialect].patched = patched;
}
writeFileSync(join(scratch, 'receipt.json'), JSON.stringify(receipt, null, 2) + '\n');
console.log('PASS: both unchanged baselines and both shipped patched parser trees reproduce byte-for-byte.');
