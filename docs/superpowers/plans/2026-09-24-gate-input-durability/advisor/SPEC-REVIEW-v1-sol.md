# Independent SPEC review — round 1 of 2

Review target: `feat/gate-input-durability` at `5c0325fb`, based on `origin/main` `30e13053`. The worktree was clean and the spec is the only branch change.

The report was not written because the review is read-only and `target/` does not exist in the read-only sandbox.

## Repository and pin verification

All existing authority paths cited by the spec exist and contain the claimed values:

- `scripts/callable-observations/schema.mjs:18` has `COMPILER_HASH` = `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.
- `verify-callable-authority.mjs:15-22` implements the claimed sorted `[[relativePath, UTF8 text], ...]` formula. Lines 24–25 contain the exact React 19/18 versions, csstype versions, and tree hashes claimed.
- `2026-09-06-callable-authority-readout.md:43` names `@types/prop-types` 15.7.15.
- `verify-typescript-grammar.mjs:14-26` contains all three claimed URLs and SHA-256 pins. Its top-level guards require Node `v26.0.0`, macOS, and arm64—not merely Node v26.
- `post317-input-custody.md:29` pins the Excalidraw `yarn.lock` SHA-256 as `a2a92a778255e83a576290948a23db0d05ec4ee994dd43c1d368a3979f16ecd0`.
- The local Excalidraw clone contains commit `0642e72cfa2d9a71198200e52f37399384610ee3`; that commit’s `yarn.lock` has the same SHA-256 and contains the exact SRIs:
  - TypeScript 5.9.3: `sha512-jl1v...TgSw==`
  - `@types/react` 19.0.10: `sha512-JuRQ...Qj2g==`
  - csstype 3.1.3: `sha512-M1uQ...ADRw==`
- `examples/project_membership_census.rs` exists.
- The seven planned implementation/receipt paths do not exist, as expected for a design-only branch.

## Node population and environment inventory

There are currently 48 tracked and on-disk `*.test.mjs` modules. All presently reside under `scripts/` or `docs/`. The proposed exclusion leaves 47 active modules. Adding `scripts/gate-inputs/index.test.mjs` would make 49 total and 48 active.

`PRISM_TYPESCRIPT` is read by 33 modules:

- `docs/eval/receiver-closure/{audit-callable-source.test.mjs,verify-callable-authority.test.mjs}`
- `docs/eval/receiver-closure/parameter-sites/source-proof.test.mjs`
- `scripts/parameter-frequency/index.test.mjs`
- `scripts/callable-observations/{closure-classification,closure-policy-proof,config-provenance-integration,config-provenance,direct-owner,entries,entry-obligations-integration,exact-ambient,identity-domains,index,lib-search-cases,lib-search,lookup,membership,merged-wildcard-proof,merged-wildcard,module-search-cases,module-search,nested,props-class,required-paths,semantic-closure-integration,semantic-closure-v2-integration,semantic-closure-v3-integration,type-lib,type-search-cases,type-search-identity,umd-qualifier,wildcard}.test.mjs`

Other required inputs:

- `PRISM_CALLABLE_PROFILES`: six modules—`verify-callable-authority`, `index`, `module-search`, `nested`, `props-class`, and `umd-qualifier`.
- `PRISM_GRAMMAR_ARCHIVES`: `scripts/verify-typescript-grammar.test.mjs`.
- `PRISM_MEMBERSHIP_NATIVE`: `scripts/callable-observations/membership.test.mjs`.
- `PRISM_AUDIT_{TYPESCRIPT,UPSTREAM,SLICE,SITES,SOURCE_REPO}`: only `audit-imported-props-source.test.mjs`; its exclusion is justified by unavailable `SITES` authority.

The population also reads optional override variables not covered by the proposed four exports: `PRISM_CALLABLE_IMPLEMENTATION`, `PRISM_CALLABLE_BASELINE`, `PRISM_OBSERVER_MODULE`, `PRISM_PARAMETER_FREQUENCY_IMPLEMENTATION`, and `PRISM_PARAMETER_FREQUENCY_TEST_DELAY_MS`.

## WRONG findings

### WRONG W1 — The react18 bootstrap cannot satisfy the spec’s pre-authentication invariant

On the first acquisition, the three react18 archives have no authoritative archive digest. The tool must parse and extract them before it can calculate the assembled profile tree hash. That directly contradicts both “authenticated before any extraction” in §3.3 and the STOP condition “extraction before authentication.”

Concrete scenario: a clean installation reaches `@types/react@18.3.31`; there is no authoritative SRI yet. Either acquisition stops, making bootstrap impossible, or it extracts unauthenticated bytes in violation of the contract.

Bounded fix: explicitly redefine the bootstrap rule as hostile-input parsing with no installation or execution before tree authentication, and make it the sole exception to pre-extraction archive authentication. Alternatively, owner-promote independently custodied raw archive digests before implementation.

### WRONG W2 — Archive validation permits resource exhaustion and relies on non-portable text parsing

The checks reject paths and entry types but impose no entry-count, path-length, depth, per-entry expanded-size, or aggregate expanded-size limit.

Concrete scenario: a react18 archive smaller than 64 MiB contains a regular `package/index.d.ts` entry that expands to hundreds of gigabytes. It passes every listed check and can exhaust disk before the tree hash refuses it.

Additionally, `tar -tvzf` is not a stable machine-readable interface across bsdtar and GNU tar. Filenames containing whitespace, newlines, link-like text, or PAX/GNU metadata make line/column parsing ambiguous.

Bounded fix: specify one robust archive-reading mechanism and reject unsupported extensions, duplicate destinations, case-colliding paths, and malformed names. Enforce compressed bytes while streaming plus entry count, path bytes/depth, per-entry expanded bytes, and total expanded bytes before filesystem extraction. Test captured BSD/GNU cases if both are supported.

### WRONG W3 — Atomic replacement of a populated directory is not implementable as specified

POSIX `rename` cannot replace an existing nonempty directory with another nonempty directory.

Concrete scenario: `profiles/react18` exists but fails reauthentication. A valid replacement is staged. `rename(staging, final)` fails with `ENOTEMPTY`/`EEXIST`; deleting or renaming the old directory first introduces a crash window in which the final path is missing.

Bounded fix: install immutable generation directories and atomically replace a file/symlink manifest pointing to the active generation. Otherwise specify a recoverable two-rename transaction and stop calling replacement atomic.

### WRONG W4 — The runner does not seal its execution context

The proposed runner exports four values but does not require clearing inherited override variables or establish subprocess cwd and host-tool policy.

Concrete scenarios:

- `PRISM_CALLABLE_IMPLEMENTATION=/old/checkout` causes multiple active tests to test that checkout rather than the reviewed repository.
- `PRISM_OBSERVER_MODULE` or `PRISM_PARAMETER_FREQUENCY_IMPLEMENTATION` redirects other tests.
- `NODE_OPTIONS=--require ...` changes every test process.
- Inherited `CARGO_TARGET_DIR` puts the native binary somewhere other than the path the runner expects.
- Invoking the runner outside the repository makes relative test paths, the Cargo build, and `membership.test.mjs`’s relative CLI path resolve incorrectly.
- `audit-callable-source.test.mjs` needs `git` on `PATH`; `source-proof.test.mjs` spawns literal `node`.
- `cargo build --offline` lacks `--locked`/`--frozen` and may update `Cargo.lock` when it is stale.

Bounded fix: derive the repository root from `import.meta.url`, set every child cwd to it, construct a documented child environment, clear all optional Prism overrides plus `NODE_OPTIONS`/`NODE_PATH`, set a fixed `CARGO_TARGET_DIR`, preflight `node`, `git`, and `cargo`, and use `cargo build --frozen`. Record the native binary hash in the gate-run receipt.

### WRONG W5 — The population guard does not enforce “every test in the repository”

The declared population and drift guard search only `scripts/` and `docs/`, while the contract repeatedly says every repository `*.test.mjs`.

Concrete scenario: a future tracked `tools/check.test.mjs` is added. It is outside both search roots, so the drift guard passes and the “full” gate silently omits it.

Bounded fix: enumerate from repository root with explicit exclusions for `.git`, `target`, dependency/generated directories, and the durable input root. Alternatively, narrow every completeness claim to `scripts/` and `docs/`.

### WRONG W6 — A fresh machine cannot run the promised offline one-command gate

`Cargo.lock` contains 138 packages. `cargo build --offline` requires their sources to already exist in the local Cargo cache, but that cache is neither acquired nor declared as a prerequisite.

Concrete scenario: a fresh clone on a machine with an empty Cargo cache has every new durable Node input, yet the runner stops before executing Node tests because offline Cargo dependency resolution fails.

Bounded fix: either define the exact Rust toolchain and populated locked Cargo cache as host prerequisites and remove the “any machine” claim, or add authenticated vendored Cargo dependency custody as a separately reviewed scope.

### WRONG W7 — The acquisition and acceptance counts are inconsistent

The design names six npm archives:

1. TypeScript 5.9.3
2. `@types/react` 19.0.10
3. csstype 3.1.3
4. `@types/react` 18.3.31
5. csstype 3.2.3
6. `@types/prop-types` 15.7.15

Together with three grammar files, there are nine downloaded artifacts, not “5 npm packages and 3 grammar files” or “all 8 inputs.”

Concrete scenario: acceptance relies on the count eight and reports completion while one named artifact was never reconciled.

Bounded fix: define whether counts refer to downloaded artifacts, installed logical inputs, or environment variables, and make acceptance enumerate the exact names rather than relying on a numeric total.

### WRONG W8 — The specified TypeScript reauthentication does not cover the required `lib/` contents

The initial npm SRI authenticates the downloaded package, but the mandatory installed checks only hash `typescript.js` and check that `lib/` exists. Active tests copy and use other files from that directory.

Concrete scenario: `lib/lib.dom.d.ts` is modified while `lib/typescript.js` remains unchanged. The explicitly required post-assembly checks accept the input, so the runner may claim it reauthenticated TypeScript while using altered compiler-library bytes.

Bounded fix: derive and pin a raw-byte manifest/tree hash for the complete required TypeScript tree from the SRI-authenticated archive, then require exact path/type/content equality during acquisition and every runner preflight. An unanchored observed receipt hash is not a substitute for an authority pin.

## SMELL findings

### SMELL S1 — The react profile tree hash is not independently binary-safe

`readFileSync(file, "utf8")` is lossy for invalid UTF-8; different byte sequences can decode to the same JavaScript string. No current pinned-file collision was demonstrated, so this is not a WRONG, but the formula alone cannot establish exact bytes.

Bounded improvement: require every file to round-trip exactly through UTF-8 decoding/encoding before applying the historical formula, reject every nonregular file, and record a derived raw-byte manifest after successful authentication.

### SMELL S2 — Redirect, mirror, TOCTOU, ownership, and durability rules remain underspecified

“Pinned host families” is not an executable rule, especially because GitHub release downloads redirect to asset hosts. The Excalidraw authority uses `registry.yarnpkg.com`, while acquisition selects `registry.npmjs.org`; the SRI protects content, but the spec should distinguish transport from authority.

The spec also does not require:

- Manual validation of every redirect hop, exact allowed hostnames, HTTPS/default port, credentials, and hop count.
- Copying `--from` files into private staging before hashing and consuming only that staged copy.
- A private mode-0700, non-symlink staging root and acquisition lock.
- Durability barriers for installed generations, receipts, and parent directories.

### SMELL S3 — RED and the seven control groups are insufficient

The proposed missing-file tree-hash adapter proves the old verifier notices a missing file; it does not necessarily prove the public acquisition boundary fails before implementation.

Missing controls include:

- Replacement of an existing populated input and crash recovery.
- Compressed/expanded size, entry-count, duplicate, PAX, and parser-dialect cases.
- Redirect-host refusal, hop limits, streaming size enforcement, and timeout abort.
- TypeScript non-entrypoint library tampering.
- Mirror symlink/mutation and same-staged-byte consumption.
- Runner cwd, environment sanitation, `PATH` preflight, `NODE_OPTIONS`, and inherited Cargo variables.
- Empty Cargo cache.
- Concurrent acquisition/runner behavior and atomic receipt/log publication.

The synthetic acquisition controls can remain offline, but network-policy logic needs pure/injected transport tests. Live acceptance alone covers only successful fetching.

### SMELL S4 — The line budget is unlikely to hold for the actual contract

My forecast after the required fixes is:

- Helpers: approximately 700–900 honest lines.
- Tests: approximately 600–800 honest lines.
- Combined: approximately 1,300–1,700 honest lines.

The secure downloader/archive validator, transactional installer, receipt/env generator, population validator, Cargo builder, environment-sealed runner, log publisher, and cross-platform negative fixtures are too much for 650/550/1,200 without compressing logic or omitting controls.

Bounded improvement: split acquisition/authentication/installation from population/runner orchestration, or revise the caps before implementation. The chosen external-input scope itself is coherent; the mismatch is between that scope and the budget.

No tests or acquisition commands were run during this read-only design review.

VERDICT: FIX (8 WRONG / 4 SMELL)