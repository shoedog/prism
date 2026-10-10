# Lane PKG repair after spec review round 1 (you are the repair engineer, gpt-6.1-sol)

## Where you work
- **Clone:** `/Users/wesleyjinks/code/prism-pkgres`. The checkout is at the committed prototype `92c1d0bc` (branch `proto/workspace-package-resolution`). The plan packet `docs/superpowers/plans/2026-10-04-workspace-package-resolution/` is in the working tree (committed on the plan branch at `c89bc5b7`).
- **Rules:** no git writes, never open private F, keep disk use lean.

## Goal
Bind JS/TS bare specifiers to in-repo workspace package entries **exactly as TS 5.9.3 does** for the writer's own project and resolution mode.
- Positive proof only.
- Zero false bindings, zero false Exact edges, zero lost base edges.

Lane S2 consumes this under owner rule S2-O9: a writer import that neither prism nor TS can resolve is out of model for S2's refusals. So PKG must also distinguish **why** something is unresolved.

## Reviews (both FIX; read both in full)
- `~/prism-evidence/pkgres/review/spec-r1-opus.md`, findings F1–F13
- `~/prism-evidence/pkgres/review/spec-r1-sol.md`, findings W1–W7 and S1–S2 (evidence alongside it)

Every WRONG is a divergence from the TS 5.9.3 resolver, reproduced against the oracle. Fold all material WRONGs:
- extension substitution on explicit TS `exports` targets (Opus F1, sol W1);
- star erasure in literal targets (sol W2);
- a missing `.d.ts` legacy `types` value → the same-name `.ts` (Opus F2);
- ESM legacy `main`/`types` extensionless probing (Opus F4, sol W3);
- the writer file-extension mode for `.mjs`/`.cjs`/`.mts`/`.cts` (Opus F3);
- the package scope walking past the repo root to find `"type"` (Opus F5). Read the outer `package.json` if one exists; if a parent outside the repo is unreadable or ambiguous, decline (positive proof);
- self-name resolution with `resolvePackageJsonExports=false` (Opus F6);
- the Node10-only relative export hop (sol W4);
- `#imports` specifiers on the ordinary package rung (sol W5);
- null or no-result exports treated as opaque winners (sol W6);
- `node:`/`virtual:` specifiers: tsconfig `paths` apply **before** builtin/scheme classification (Opus F7, C28). This is a correctness fix, not an owner question: `node:x` counts as external-builtin only when no paths pattern matches it.

## Required structural changes
1. **A three-way result type** (Opus F8, sol S1):
   - `Bound(module)`;
   - `ProvenUnresolved`: TS semantics give no binding for this input. Examples: a missing `dist/` target in an unbuilt checkout, or a loader scheme with no paths match;
   - `Unsupported(reason)`: prism declines a feature TS supports.

   S2 will treat only `ProvenUnresolved` as out of model (S2-O9). `Unsupported` must fail closed in S2. Document this contract in SPEC.
2. **A differential acceptance gate against the TS 5.9.3 oracle** (Opus F10). Generate a fixture matrix:
   - axes: moduleResolution (node10, node16, nodenext, bundler) × writer extension (.ts, .tsx, .mts, .cts, .js, .mjs, .cjs) × package `"type"` (absent, module, commonjs) × entry shape (exports string, conditions, subpath, pattern, null, array, legacy types/main/module, missing targets, index fallback) × workspace link shape (npm/yarn workspace, pnpm nested link, self-name);
   - it must include every reviewer repro (Opus C1–C30, sol probes).

   Gate: **zero cases where prism says `Bound(m)` and TS binds anything other than m, or nothing.** Report counts of `Unsupported` where TS binds, by feature. That count is the S2-O9 gap the controller needs. Keep the generator and runner in the packet's `probes/` and make the gate rerunnable by the implementer.
3. **Coverage needed before S2** (Opus F9, sol "Unresolved boundary"). Implement, rather than defer, every TS-binds-but-PKG-doesn't feature that the reviewers mark as dangerous for S2 or needed for goal 1:
   - `#` package imports (at minimum classify them as `Unsupported`, never empty);
   - paths on colon specifiers;
   - subpath imports without `exports` through symlinks;
   - the bundler index fallback.

   For the lower-risk ones (C8, C9, C10, C13): implement them if cheap, otherwise return `Unsupported` and list them in OQ.

## Measure
- X, installed X, R and T against main, with every changed row CORRECT under the oracle and ownership agreeing, and zero lost base edges.
- The differential gate totals.
- STOP if any public row becomes a wrong binding. Report it and choose nothing.

## Docs, tests, gates
- **SPEC:** update §0 and the result-type contract.
- **IMPLEMENTOR (Opus F11, sol W7):**
  - dispatch from committed `92c1d0bc` plus the R1 patch;
  - WRONG repairs before any JS-secondary work;
  - the differential gate as acceptance.
- **Tests:** a regression for each reviewer repro (fails on `92c1d0bc`, passes after), plus a persisted-cache invalidation test for `package.json`/symlink/tsconfig edits (sol S2). Add mutants to `mutants/lane-pkg-resolution.json` for each new rule and for the three-way classification.
- **Gates:**
  - one `cargo nextest run --features mcp`, doctests, fmt and clippy;
  - the advisory scoped mutgate;
  - the Tier-A matrix (skip quick);
  - S1b-4 controls byte-identical;
  - lane-P rows unchanged.
  - Rebuild the bins and rebind the BUILD-MANIFEST for `CONTROLLER-pkg.sh`.
- **Patches:** `~/prism-evidence/pkgres/repair-r1/R1-src.patch` (relative to `92c1d0bc`) and `R1-docs.patch` (relative to `c89bc5b7`).

## Final message
- findings and how each was fixed;
- the differential gate totals, including the Unsupported-where-TS-binds table;
- the yield;
- files with commit messages;
- the gates;
- what you did not verify;
- any STOP.
