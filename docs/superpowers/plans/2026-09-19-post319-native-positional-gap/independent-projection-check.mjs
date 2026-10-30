import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const packetPath = "/private/tmp/prism-post317-parameter-frequency-verification/final-f79bb954/cold.json";
const parentManifestPath = "/private/tmp/prism-post317-planning/p2/input-hash-manifest.json";
const siteManifestPath = "/private/tmp/prism-post319-successor-planning/SITE-MANIFEST.json";
const packetBytes = readFileSync(packetPath);
const parentBytes = readFileSync(parentManifestPath);
const packet = JSON.parse(packetBytes);
const parent = JSON.parse(parentBytes);
const actual = JSON.parse(readFileSync(siteManifestPath));
const sha = bytes => createHash("sha256").update(bytes).digest("hex");

const sites = [];
for (const file of packet.files) for (const callable of file.callables) {
  const objectOrdinals = callable.parameters.filter(parameter => parameter.pattern === "object").map(parameter => parameter.ordinal);
  const laterRequiredOrdinals = callable.parameters.filter(parameter =>
    parameter.pattern === "identifier" && !parameter.optional && !parameter.rest && !parameter.outer_initializer
    && objectOrdinals.some(ordinal => ordinal < parameter.ordinal)).map(parameter => parameter.ordinal);
  if (objectOrdinals.length && laterRequiredOrdinals.length) sites.push({
    path: file.path,
    start_byte: callable.token_start,
    end_byte: callable.end,
    compiler_kind: callable.syntax_kind,
    object_ordinals: objectOrdinals.filter(ordinal => laterRequiredOrdinals.some(later => ordinal < later)),
    later_required_ordinals: laterRequiredOrdinals,
  });
}
const selectedPaths = new Set(sites.map(site => site.path));
const members = parent.members.filter(member => selectedPaths.has(member.path));
const packetFiles = new Map(packet.files.map(file => [file.path, file]));
const projection = {
  packetSha: sha(packetBytes),
  parentSha: sha(parentBytes),
  sites,
  members: members.map(({ path, sha256, bytes, declaration_only, test_path }) => ({
    path, sha256, bytes, script_kind: packetFiles.get(path).script_kind, declaration_only, test_path,
  })),
  sourceBytes: members.reduce((sum, member) => sum + member.bytes, 0),
};
const expected = {
  packetSha: actual.parent_packet_sha256,
  parentSha: actual.parent_manifest_sha256,
  sites: actual.sites,
  members: actual.members,
  sourceBytes: actual.source_bytes,
};
console.log(JSON.stringify({ equal: JSON.stringify(projection) === JSON.stringify(expected), siteCount: sites.length, memberCount: members.length, sourceBytes: projection.sourceBytes }));
if (JSON.stringify(projection) !== JSON.stringify(expected)) process.exitCode = 1;
