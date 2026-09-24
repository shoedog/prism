# Gate-input durability: pinned, re-acquirable Node test-gate inputs (v2)

**Status:** v2 for independent spec review round 2 of 2. v2 folds all 8 WRONG and 4 SMELL findings from sol's spec
round 1 (mapped in §9), plus the parallel review's findings. No implementation is authorized until APPROVE.

**Base:** `origin/main`, now `5501bc0f`: the PR #320 merge, source-equivalent to `30e13053`.

**Owner decision:** 2026-09-23, "Gate-input durability" as the next parallel slice.

**The slice is SPLIT** into two independently reviewed and published implementation slices:

- **Slice A — acquire and install** (§3): make every re-acquirable external input durable and authenticated.
- **Slice B — declared population and sealed runner** (§4): one command runs the declared full Node gate.
  Slice B depends on A's receipt format only.

## 1. Problem and value

The Node test gate's external inputs lived only under `/private/tmp`, and the macOS daily cleaner purged them:

- the TypeScript 5.9.3 package
- the react18 and react19 callable profiles
- the grammar archives

On 2026-09-23 this meant:

- 6 modules could not run: 5 in `scripts/callable-observations/` plus `verify-callable-authority`.
- The grammar tamper test skipped.
- No repository receipt records the exact full-population command or module inventory.

Value: honest, reproducible full-population gates on this host and any host that meets the declared prerequisites
(§4.4).

**Non-goals:**

- No `src/`, Cargo, dependency, or existing-test change.
- No `PRISM_AUDIT_SITES` reconstruction: it has no authority.
- No change to the grammar verifier's Node v26.0.0 / darwin / arm64 host pin.
- No vendored Cargo dependencies.

## 2. Artifacts and logical inputs

Exact enumeration. Counts are only derived from this list; nothing in acceptance relies on a numeric total.

| # | Downloaded artifact | Logical input | Content authority |
|---|---|---|---|
| 1 | `typescript-5.9.3.tgz` | TypeScript | npm SRI from Excalidraw `0642e72c` `yarn.lock` (lock SHA-256 pinned at `docs/eval/post317-input-custody.md:29`); `typescript.js` SHA-256 = `COMPILER_HASH` (`scripts/callable-observations/schema.mjs:18`) |
| 2 | `@types/react-19.0.10.tgz` | profile react19 | yarn.lock SRI, plus the react19 tree hash (`verify-callable-authority.mjs:24`) |
| 3 | `csstype-3.1.3.tgz` | profile react19 | yarn.lock SRI, plus the react19 tree hash |
| 4 | `@types/react-18.3.31.tgz` | profile react18 | react18 tree hash (`verify-callable-authority.mjs:25`) only; §3.3 bootstrap |
| 5 | `csstype-3.2.3.tgz` | profile react18 | same as #4 |
| 6 | `@types/prop-types-15.7.15.tgz` | profile react18 | same as #4; version per `2026-09-06-callable-authority-readout.md:43` |
| 7 | `upstream.tar.gz` | grammar archives | SHA-256 in `scripts/verify-typescript-grammar.mjs:14-26` |
| 8 | `javascript-0.23.1.tgz` | grammar archives | same |
| 9 | `tree-sitter-macos-arm64.gz` | grammar archives | same |

That makes 9 artifacts across 4 logical inputs: TypeScript, react19, react18, and grammar-archives. The native
membership helper is **built**, not acquired (§4).

## 3. Slice A — acquire and install (`scripts/gate-inputs/acquire.mjs`, `pins.json`)

### 3.1 Durable root and generations

- Root selection:
  - `$PRISM_GATE_INPUTS_ROOT`, if set
  - otherwise `$XDG_DATA_HOME/prism/gate-inputs`
  - otherwise `~/.local/share/prism/gate-inputs`
- The tool refuses, with reason `volatile root`, any root whose `realpath` (of the nearest existing ancestor) lies
  under `realpath(os.tmpdir())`, `/private/tmp`, `/tmp`, `/private/var/folders`, or `/var/folders`. There is no
  override.
- The root and every tool-created directory are mode 0700 and must not be symlinks, checked with `lstat` on every
  component below the root.
- An acquisition lock (`<root>/.lock`, exclusive create with the pid inside) serializes acquirers. A stale lock is
  reported and never auto-broken.
- **Immutable generations (W3 fold).** Each logical input installs into `<root>/<input>/gen-<content-id>/`, where
  `<content-id>` is the SHA-256 of that input's canonical raw-byte manifest (§3.4).
  - A generation is written to `<root>/<input>/.staging-<uuid>/`, fully authenticated, `fsync`ed, then renamed to
    its `gen-*` name. Rename to a **new**, nonexistent directory name is atomic.
  - The active pointer is `<root>/current.json`, a file replaced atomically: write a temp file, `fsync`, rename,
    then `fsync` the directory. It maps each input to its generation path and manifest id.
  - Old generations are never modified. `--prune` removes non-current generations only when explicitly requested.
  - Crash recovery: a leftover `.staging-*` is ignored and removed on the next run; `current.json` is always either
    the old or the new version.
- **Retained artifacts:** each generation keeps its source archives under `gen-*/archives/`, so re-verification is
  anchored to the SRI and SHA-256 pins, not to an observed receipt (W8 fold).

### 3.2 Transport (S2 fold)

- Fetch through `fetch` with `redirect:'manual'`, following at most 3 hops by hand. Every hop must be:
  - `https:` on the default port
  - no credentials or userinfo
  - an exact hostname on this allowlist:
    - `registry.npmjs.org`
    - `codeload.github.com`
    - `github.com`
    - `objects.githubusercontent.com`
    - `release-assets.githubusercontent.com`
- Anything else refuses as `redirect host not pinned`.
- `pins.json` records the transport URL separately from the content authority. npm artifacts use
  `registry.npmjs.org` even though the lock authority lists `registry.yarnpkg.com`; content is bound by SRI, not by
  host.
- The body is streamed to a staging file with a running byte count. Refuse above **64 MiB compressed**, and on a
  **120 s** whole-fetch timeout, aborting via `AbortController`.
- `--from <dir>`, the offline mirror:
  - `lstat` the mirror file and refuse symlinks.
  - **Copy** it into private staging first, then hash and consume only the staged copy.

### 3.3 Authentication before use; react18 bootstrap (W1 fold)

- **Artifacts with a byte authority** (#1–3 SRI; #7–9 SHA-256): verify the staged archive bytes **before any
  parsing**.
- **Artifacts #4–6** (react18): the repository holds no archive-byte authority. **The sole exception** to verifying
  before parsing is:
  - The archive is parsed as **hostile input** by the §3.5 reader, with all limits enforced.
  - It is extracted only into its private staging generation.
  - Nothing is installed or executed, and no path outside staging is written, until the **assembled react18 tree**
    matches the pinned tree hash.
  - On success the tool writes each observed archive SRI to `pins.json` as a proposed pin with
    `"authority":"tree-hash-bootstrap"`, recorded in the repository by the controller at acceptance. From then on
    #4–6 have a byte authority and follow the normal path.
- No other path may parse before authentication. The STOP condition wording in §7 reflects this.

### 3.4 Canonical raw-byte manifests (W8 and S1 fold)

For every installed tree the tool computes a canonical manifest, and the SHA-256 of that manifest is the generation
content id. The manifest is: sorted UTF-8 relative paths, each with type (`file` or `dir`), size, and raw-byte
SHA-256. Symlinks, hardlinks, and devices are refused.

Checks on the installed trees:

- **TypeScript:** the manifest is derived from the SRI-authenticated archive's `package/` entries. The installed
  `package/` tree, including all of `lib/`, must equal it exactly.
- **Profiles:**
  - The historical formula `sha256(JSON.stringify([[relpath, utf8text], …]))` must equal the pin.
  - **Additionally**, every file must round-trip exactly through UTF-8 decode and encode. Otherwise refuse
    (`non-utf8 profile file`), because the historical formula alone cannot bind raw bytes.
  - The raw-byte manifest is recorded alongside.
  - `package.json` versions must equal the pins.
- **Grammar archives:** each file's SHA-256 is checked; they are not extracted.

### 3.5 Archive reader (W2 fold)

- An in-Node streaming reader: `zlib.createGunzip()` feeding a minimal ustar/pax parser. It does **not** use the
  system `tar`, which avoids bsdtar versus GNU tar dialect parsing.
- Supported entries: regular file (`0` / NUL) and directory (`5`). Pax `x` headers apply `path` and `size` only.
- Refused before any write:
  - pax `g` headers
  - GNU `L`/`K` long names
  - symlinks, hardlinks, character or block devices, FIFOs
  - sparse files
  - unknown type flags
- Per-archive limits:
  - ≤ 64 MiB compressed
  - ≤ 256 MiB total expanded
  - ≤ 64 MiB per entry
  - ≤ 10,000 entries
  - path ≤ 255 bytes and ≤ 32 components
- Paths must be:
  - under the single `package/` prefix
  - not absolute
  - free of `.`/`..`/empty components, NUL, and backslash
  - valid UTF-8
- **Duplicate destinations and case-folding collisions refuse.** Checksums in ustar headers are verified.
- Write limits are enforced **while streaming**, so an oversized entry refuses before exceeding the limit on disk.

### 3.6 Receipt and env

`<root>/current.json` is canonical JSON with one trailing newline and no timestamps. Per input it records:

- the generation path
- the manifest id
- each artifact's name, transport URL, SHA-256, SRI, and authority kind
- the tool SHA-256

`<root>/env.sh` holds `export` lines for `PRISM_TYPESCRIPT`, `PRISM_CALLABLE_PROFILES`, and
`PRISM_GRAMMAR_ARCHIVES`, pointing into current generations and shell-quoted.

`acquire.mjs --verify` re-authenticates current generations from retained archives and pins without any network.
Slice B's preflight uses it.

### 3.7 Slice A tests (offline; synthetic mirror; injected transport)

**RED first:** a runnable test adapter that installs an assembled tree while omitting one file. The complete
install-record assertion must fail on the concrete manifest id or tree hash. Setup failures are inadmissible as RED.

Required controls, each with a complete expected record:

1. **Happy path.** Synthetic archives and pins go through `--from`: full layout, `current.json`, `env.sh`. A re-run
   yields `verified`, with byte-identical files and no new generation.
2. **Byte authority.** Refuse before parse on:
   - wrong SRI
   - wrong SHA-256
   - truncated gzip
   - an oversized compressed stream (streaming cap)
3. **Bootstrap.** A wrong react18 tree refuses. Assert that nothing is written outside staging and no SRI is
   proposed. A correct tree proposes exactly 3 SRIs.
4. **Reader.** One refusal row per rule in §3.5:
   - each refused type flag
   - pax `g`
   - GNU long name
   - absolute, `..`, and backslash paths
   - outside `package/`
   - duplicate and case-collision destinations
   - bad header checksum
   - an entry count above the limit, and expanded bytes above the per-entry and total limits (a synthetic
     zero-filled entry streamed lazily)
   - non-UTF-8 path
5. **Generations.**
   - Replacing a populated current input with a new generation: `current.json` flips atomically.
   - A crash is simulated by killing the child after staging, before rename; the old `current.json` remains intact
     and staging is cleaned on the next run.
   - A failed input leaves `current.json` unchanged.
   - Tampering with a non-entrypoint TypeScript `lib/*.d.ts` fails `--verify`.
6. **Transport** (injected fetch):
   - an off-allowlist redirect host
   - `http:` hop
   - a non-default port
   - userinfo
   - more than 3 hops
   - timeout abort
   - a symlinked `--from` file
   - mutation of the mirror file after it is staged, which must not affect the consumed bytes
7. **Root.**
   - Refusal under `tmpdir`, `/private/tmp`, `/tmp` via its symlink, and `/var/folders`.
   - A symlinked component under the root refuses.
   - A concurrent acquirer sees the lock.
8. **Drift guards.** Each pin in `pins.json` equals the value parsed from the cited source:
   - the grammar pins in `verify-typescript-grammar.mjs`
   - the profile versions and tree hashes in `verify-callable-authority.mjs`
   - `COMPILER_HASH` in `schema.mjs`
   - the yarn.lock-derived SRIs, which the test checks against a checked-in excerpt of the three lock stanzas plus
     the lock file's pinned SHA-256 citation

**Slice A budget** (honest executable lines, JavaScript ≤ 100 columns):

| Bucket | Cap |
|---|---|
| helper | ≤ 700 |
| tests | ≤ 700 |
| combined | ≤ 1,400 |

Early stop at 95%. The controller forecasts helper about 500–620 and tests about 520–640.

## 4. Slice B — declared population and sealed runner

### 4.1 Declared population (W5 fold)

`scripts/gate-inputs/node-population.json` lists every `*.test.mjs` returned by
`git ls-files --cached --others --exclude-standard -- '*.test.mjs'`, run from the repository root. This is
repository-wide, and it excludes `.gitignore`d paths such as `target/` and `node_modules/`. Each entry is either
`active` or `excluded` with a reason code. The only exclusion today is `inputs-not-reconstructible` for
`docs/eval/receiver-closure/audit-imported-props-source.test.mjs`. Host skips inside active modules stay skips and
are reported.

### 4.2 Sealed execution (W4 fold)

`run-node-gate.mjs --log <new-file>` does the following:

- Derives the repository root from `import.meta.url`, and uses it as the cwd for every child process.
- Builds the child env **from an allowlist**:
  - inherited `PATH`, `HOME`, `LANG`, `TMPDIR`
  - the three inputs from `current.json`
  - `PRISM_MEMBERSHIP_NATIVE`
  - a fixed `CARGO_TARGET_DIR=<repo>/target/gate-inputs-native`
- Clears everything else, explicitly including:
  - `PRISM_CALLABLE_IMPLEMENTATION`
  - `PRISM_CALLABLE_BASELINE`
  - `PRISM_OBSERVER_MODULE`
  - `PRISM_PARAMETER_FREQUENCY_IMPLEMENTATION`
  - `PRISM_PARAMETER_FREQUENCY_TEST_DELAY_MS`
  - `NODE_OPTIONS`, `NODE_PATH`
  - every `PRISM_AUDIT_*`
- Preflights `node`, `git`, `cargo`, and `tar` on the sealed `PATH`, and records their versions.
- Preflights inputs with `acquire.mjs --verify`. It never downloads.
- Builds with `cargo build --frozen --offline --example project_membership_census`.
- Records the native binary's SHA-256.
- Runs `node --test --test-concurrency=2 <active paths, sorted>`.
- Writes the log with new-only publication.
- Writes a gate receipt containing:
  - the population digest
  - tool versions
  - the input manifest ids
  - the native binary hash
  - totals, skips, and exclusions
- Exits nonzero on any failure.

### 4.3 Slice B tests

- **Drift guard:** the declared set must equal the `git ls-files` set. An undeclared new file fails, and so does a
  missing declared file.
- **Env sealing:** a dry-run prints the argv, cwd, and env plan. Each inherited override must be removed, and
  `NODE_OPTIONS` must be removed.
- **Invocation location:** running the tool from another directory still uses the repository cwd.
- **Preflight refusals:** a missing tool, a tampered input, and an unwritable receipt path each refuse.
- **Failure propagation:** a stubbed two-module population where the child fails must produce a nonzero exit.
- **Empty Cargo cache:** simulate it with `CARGO_HOME` pointed at an empty dir. The runner must report
  `host-prerequisite-missing: cargo cache`, distinct from a test failure.

**Slice B budget:**

| Bucket | Cap |
|---|---|
| helper | ≤ 350 |
| tests | ≤ 350 |
| combined | ≤ 700 |

### 4.4 Host prerequisites (W6 fold)

These are declared, not acquired:

- a Rust toolchain compatible with `Cargo.lock`
- a **populated Cargo registry cache** for the locked dependencies, since the build is offline and frozen
- `node` ≥ 24
- `git`
- `tar`, needed by the grammar verifier at test time

The "any machine" claim is replaced by "any host meeting §4.4".

## 5. Acceptance (controller, after each slice's implementation review approves)

**Slice A:**

1. Live acquisition of all 9 artifacts, enumerated by name, into the durable default root. Every authority check
   passes. The 3 react18 SRIs are proposed by the bootstrap and committed to `pins.json` with
   `"authority":"tree-hash-bootstrap"`.
2. A second, cold acquisition into a fresh durable root gives identical manifest ids.
3. `--verify` passes with no network.

**Slice B:**

1. `run-node-gate.mjs` runs the full declared population. It must report 0 failures. The only skips are the
   grammar host-pin skips on the Node v24 host, listed by name.
2. The receipt `docs/eval/gate-input-durability/receipt.md` records the exact module inventory digest and totals.
3. The default Rust suite is 4,559 / 0 / 1, recorded, not re-baselined.

## 6. Review and stop rules

- Each slice gets two implementation review rounds. Findings are tagged WRONG or SMELL, with a concrete input.
- At a cap, the controller classifies convergence before acting.

**STOP conditions:**

- any `src/`, Cargo, or existing-test change
- any `npm install` or lifecycle execution
- parsing before byte authentication, except the §3.3 react18 bootstrap
- installation or execution before tree authentication
- any new pin without cited authority, except the §3.3 bootstrap SRIs
- a budget breach

## 7. Owned paths

**Slice A:**

- `scripts/gate-inputs/{acquire.mjs,pins.json,acquire.test.mjs,README.md}`
- `scripts/gate-inputs/fixtures/yarn-lock-excerpt.txt`

**Slice B:**

- `scripts/gate-inputs/{run-node-gate.mjs,node-population.json,run-node-gate.test.mjs}`
- a README section
- `docs/eval/gate-input-durability/receipt.md`

## 8. Planning review record

- Round 1:
  - sol: FIX, 8 WRONG / 4 SMELL.
  - kimi: first run inadmissible (a read outside its directory was auto-rejected); re-run pending.
- This v2 is round-2 material.

## 9. Round-1 fold map

| Finding | Fold |
|---|---|
| W1 react18 extraction before authentication | §3.3: explicit hostile-parse exception with no install/exec before the tree hash; proposed SRIs become pins |
| W2 unbounded, non-portable tar listing | §3.5: in-Node streaming reader with full limits and refusal rules |
| W3 non-atomic directory replacement | §3.1: immutable generations plus an atomically replaced `current.json` |
| W4 unsealed runner | §4.2: allowlisted env, cleared overrides, fixed cwd and `CARGO_TARGET_DIR`, `--frozen`, tool preflight |
| W5 population scope | §4.1: repository-wide `git ls-files` |
| W6 fresh-machine Cargo | §4.4: declared host prerequisites; distinct `host-prerequisite-missing` |
| W7 count inconsistency | §2: exact 9-artifact / 4-input enumeration; acceptance by name |
| W8 TypeScript `lib/` not authenticated | §3.1 and §3.4: retained archives plus an SRI-anchored raw-byte manifest of the whole `package/` |
| S1 UTF-8-lossy tree formula | §3.4: UTF-8 round-trip requirement plus a raw-byte manifest |
| S2 redirect/TOCTOU/durability | §3.1 and §3.2: host allowlist, manual redirects, staged mirror copy, 0700, lock, fsync |
| S3 insufficient controls | §3.7 and §4.3: expanded per-rule controls |
| S4 budget | split into Slices A and B with separate caps |
