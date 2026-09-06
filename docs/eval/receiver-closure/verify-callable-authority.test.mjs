// PRISM_TYPESCRIPT=/absolute/typescript.js PRISM_CALLABLE_PROFILES=/absolute/profiles node --test <this-file>
import test from "node:test";
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, appendFileSync, rmSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

test("changed declarations cannot retain a pinned profile by keeping its package version", () => {
  assert(process.env.PRISM_TYPESCRIPT, "PRISM_TYPESCRIPT is required");
  assert(process.env.PRISM_CALLABLE_PROFILES, "PRISM_CALLABLE_PROFILES is required");
  const root = mkdtempSync(path.join(tmpdir(), "prism-callable-pin-"));
  try {
    cpSync(process.env.PRISM_CALLABLE_PROFILES, root, {recursive:true});
    appendFileSync(path.join(root, "react19/node_modules/@types/react/index.d.ts"), "\n// changed declaration bytes\n");
    const result = spawnSync(process.execPath, [
      fileURLToPath(new URL("./verify-callable-authority.mjs", import.meta.url)),
      process.env.PRISM_TYPESCRIPT, root,
    ], {encoding:"utf8",maxBuffer:4*1024*1024});
    assert.ifError(result.error);
    assert.notEqual(result.status,0,"version-only identity accepted modified dependency bytes");
    assert.match(result.stderr,/pinned declaration tree changed/);
  } finally { rmSync(root,{recursive:true,force:true}); }
});
