#!/usr/bin/env node
// Offline reproduction in fresh scratch space; never modifies shipped vendor bytes.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { chmodSync, copyFileSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { gunzipSync } from 'node:zlib';

const pins = {
  'tree-sitter-go-0.25.0.crate': 'c8560a4d2f835cc0d4d2c2e03cbd0dde2f6114b43bc491164238d333e28b16ea',
  'upstream-0.25.0-1547678a.tar.gz': '0b088a561b847c99224ea42374d2c7a619fb0831aaa83f295bfbf886309aaedd',
  'pr193-5a6af13a.tar.gz': '746d8ed954b645c4bad1d2f222614da2ed4105462aa371b8b87b97b402139b34',
  'pr193.diff': 'dd257adf7806a97525b6d418b1d980ac243cc4dd7136f737ee9296326ae83980',
  'tree-sitter-macos-arm64.gz': 'ae3bbba3ba68e759a949e7591a42100a12d660cae165837aba48cae76a599e64',
};
const args = process.argv.slice(2);
assert(args.length === 1, 'usage: node scripts/verify-go-grammar.mjs <pinned-input-directory>');
assert.equal(process.version, 'v26.0.0', 'verified JavaScript runtime');
assert.equal(process.platform, 'darwin', 'regeneration pins the macOS arm64 CLI; ordinary Cargo builds are portable');
assert.equal(process.arch, 'arm64');
const vendor = resolve(dirname(fileURLToPath(import.meta.url)), '../vendor/tree-sitter-go');
const inputs = resolve(args[0]);
const scratch = mkdtempSync(join(tmpdir(), 'prism-go-grammar-'));
console.log(`Evidence directory (retained): ${scratch}`);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
function run(command, argv, cwd = scratch) {
  const result = spawnSync(command, argv, { cwd, encoding: 'utf8', timeout: 180_000, maxBuffer: 16 * 1024 * 1024 });
  assert.equal(result.error, undefined, String(result.error));
  assert.equal(result.status, 0, `${command}: ${result.stdout}\n${result.stderr}`);
  return result.stdout;
}
function inventory(root, rel = '') {
  return Object.fromEntries(readdirSync(join(root, rel), { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name)).flatMap(entry => {
    const name = join(rel, entry.name);
    assert(!entry.isSymbolicLink(), `unexpected symlink ${name}`);
    return entry.isDirectory() ? Object.entries(inventory(root, name)) : [[name, sha(readFileSync(join(root, name)))]];
  }));
}
for (const [name, pin] of Object.entries(pins)) {
  assert.equal(sha(readFileSync(join(inputs, name))), pin, `archive pin: ${name}`);
}
for (const name of ['tree-sitter-go-0.25.0.crate', 'upstream-0.25.0-1547678a.tar.gz', 'pr193-5a6af13a.tar.gz']) {
  run('tar', ['-xzf', join(inputs, name), '-C', scratch]);
}
const baseline = join(scratch, 'tree-sitter-go-0.25.0');
const upstream = join(scratch, 'tree-sitter-tree-sitter-go-1547678');
const pr = join(scratch, 'nvlbg-tree-sitter-go-5a6af13');
const cli = join(scratch, 'tree-sitter');
writeFileSync(cli, gunzipSync(readFileSync(join(inputs, 'tree-sitter-macos-arm64.gz'))));
chmodSync(cli, 0o700);
const generator = run(cli, ['--version']).trim();
assert.equal(generator, 'tree-sitter 0.25.8 (f2f197b6b27ce75c280c20f131d4f71e906b86f7)');
assert.equal(sha(readFileSync(join(baseline, 'grammar.js'))), sha(readFileSync(join(upstream, 'grammar.js'))));
const original = inventory(join(baseline, 'src'));
run(cli, ['generate', '--abi', '15', '--js-runtime', process.execPath], baseline);
assert.deepEqual(inventory(join(baseline, 'src')), original, 'unchanged published src/ generation drift');
assert.equal(sha(readFileSync(join(vendor, 'grammar.js'))), sha(readFileSync(join(pr, 'grammar.js'))), 'authored grammar must be exactly PR #193');
copyFileSync(join(vendor, 'grammar.js'), join(baseline, 'grammar.js'));
run(cli, ['generate', '--abi', '15', '--js-runtime', process.execPath], baseline);
const patched = inventory(join(baseline, 'src'));
assert.deepEqual(patched, inventory(join(vendor, 'src')), 'shipped generated src/ differs');
for (const rel of Object.keys(inventory(baseline))) {
  if (rel.startsWith('src/') || ['grammar.js', 'bindings/rust/build.rs'].includes(rel)) continue;
  assert.equal(sha(readFileSync(join(vendor, rel))), sha(readFileSync(join(baseline, rel))), `unexpected upstream change: ${rel}`);
}
const receipt = { pins, generator, node: process.version, original, patched, pr_parser_equal: patched['parser.c'] === sha(readFileSync(join(pr, 'src/parser.c'))) };
writeFileSync(join(scratch, 'receipt.json'), JSON.stringify(receipt, null, 2) + '\n');
console.log('PASS: unchanged published src/ and shipped patched src/ reproduce byte-for-byte.');
console.log(`Patched parser equals PR generated parser: ${receipt.pr_parser_equal}`);
