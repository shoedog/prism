import { createHash } from "node:crypto";
import { lstatSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const REQUEST = "prism.native-parameter-request/1";
export const FROZEN_MANIFEST_SHA256 =
  "789352a575d68ef672de7449299676de0c0aab8edd4abd8228e23efda65d326b";
export const LIMITS = Object.freeze({ sourceBytes: 256 * 1024 });
const MEMBER = ["path", "sha256", "bytes", "script_kind"];
const SITE = [
  "path", "start_byte", "end_byte", "compiler_kind", "object_ordinals",
  "later_required_ordinals"
];
const sha = value => createHash("sha256").update(value).digest("hex");
const hex = value => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
const fail = message => { throw Error(message); };

function projection(value, keys, label) {
  if (!value || typeof value !== "object" || Array.isArray(value) ||
    !keys.every(key => Object.hasOwn(value, key))) fail(`invalid ${label} projection`);
  return value;
}

function utf8(bytes, label) {
  try {
    return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
  } catch {
    fail(`invalid UTF-8: ${label}`);
  }
}

function safePath(value) {
  return typeof value === "string" && value && !path.isAbsolute(value) &&
    !value.includes("\\") && !/\p{Cc}/u.test(value) &&
    value.split("/").every(part => part && part !== "." && part !== "..");
}

function compare(left, right) {
  const a = [...left];
  const b = [...right];
  for (let index = 0; index < Math.min(a.length, b.length); index += 1) {
    const difference = a[index].codePointAt(0) - b[index].codePointAt(0);
    if (difference) return difference;
  }
  return a.length - b.length;
}

function parseManifest(bytes) {
  try {
    return JSON.parse(utf8(bytes, "manifest"));
  } catch (error) {
    fail(`invalid manifest JSON: ${error.message}`);
  }
}

function manifestProjection(manifest) {
  projection(manifest, ["members", "sites"], "manifest");
  if (!Array.isArray(manifest.members) || !Array.isArray(manifest.sites)) {
    fail("invalid manifest projection");
  }
  return manifest;
}

export function buildRequest(options) {
  const manifestBytes = readFileSync(options.manifest);
  const manifestSha256 = sha(manifestBytes);
  if (!hex(options.manifestSha256) || manifestSha256 !== options.manifestSha256) {
    fail("manifest SHA mismatch");
  }
  const manifest = parseManifest(manifestBytes);
  const synthetic = typeof manifest?.upstream_repository === "string" &&
    manifest.upstream_repository.startsWith("synthetic://");
  if (!synthetic && manifestSha256 !== FROZEN_MANIFEST_SHA256) {
    fail("frozen manifest SHA mismatch");
  }
  if (!hex(options.nativeSha256)) fail("invalid native SHA-256");
  const { members, sites } = manifestProjection(manifest);
  const byPath = new Map();
  for (const member of members) {
    projection(member, MEMBER, "member");
    if (!safePath(member.path) || byPath.has(member.path)) {
      fail("invalid or duplicate member path");
    }
    byPath.set(member.path, member);
  }
  const grouped = new Map();
  for (const site of sites) {
    projection(site, SITE, "site");
    if (!byPath.has(site.path)) fail("site path is not a member");
    const group = grouped.get(site.path) ?? [];
    group.push(site);
    grouped.set(site.path, group);
  }
  const root = path.resolve(options.root);
  const files = [];
  let total = 0;
  for (const member of [...byPath.values()].sort((left, right) =>
    compare(left.path, right.path))) {
    const target = path.join(root, member.path);
    const stat = lstatSync(target);
    if (!stat.isFile()) fail(`member is not a regular file: ${member.path}`);
    if (stat.size !== member.bytes) fail(`pre-read size mismatch: ${member.path}`);
    if (total + stat.size > LIMITS.sourceBytes) fail("pre-read source limit exceeded");
    total += stat.size;
    const bytes = readFileSync(target);
    if (bytes.length !== member.bytes || sha(bytes) !== member.sha256) {
      fail(`source identity mismatch: ${member.path}`);
    }
    const source = utf8(bytes, member.path);
    const orderedSites = (grouped.get(member.path) ?? []).sort((left, right) =>
      left.start_byte - right.start_byte || left.end_byte - right.end_byte);
    files.push({
      path: member.path,
      sha256: member.sha256,
      bytes: member.bytes,
      script_kind: member.script_kind,
      source,
      sites: orderedSites.map(site => ({
        path: site.path,
        start_byte: site.start_byte,
        end_byte: site.end_byte,
        compiler_kind: site.compiler_kind,
        object_ordinals: site.object_ordinals,
        later_required_ordinals: site.later_required_ordinals
      }))
    });
  }
  return {
    schema: REQUEST,
    input_manifest_sha256: manifestSha256,
    native_binary_sha256: options.nativeSha256,
    files
  };
}

export function parseArgs(argv) {
  const flags = new Map([
    ["--root", "root"],
    ["--manifest", "manifest"],
    ["--manifest-sha256", "manifestSha256"],
    ["--native-sha256", "nativeSha256"]
  ]);
  const options = {};
  for (let index = 0; index < argv.length; index += 2) {
    const flag = argv[index];
    const name = flags.get(flag);
    if (!name) fail(`unknown flag: ${flag}`);
    if (!argv[index + 1] || argv[index + 1].startsWith("--")) {
      fail(`missing value: ${flag}`);
    }
    if (Object.hasOwn(options, name)) fail(`duplicate flag: ${flag}`);
    options[name] = argv[index + 1];
  }
  for (const [flag, name] of flags) {
    if (!Object.hasOwn(options, name)) fail(`missing flag: ${flag}`);
  }
  return options;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    process.stdout.write(`${JSON.stringify(buildRequest(parseArgs(process.argv.slice(2))))}\n`);
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
