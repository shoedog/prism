# Gate-input durability: pinned, re-acquirable Node test-gate inputs (v3, simple)

**Status:** v3 re-plan after v2 hit its 2-round cap non-converging (sol r1 8W/4S, r2 8W/3S). Owner decision
2026-09-24: "Re-plan simple v3". No implementation is authorized until APPROVE.

**Provenance:**

- Drafted by a Fable advisor (claude-fable-5-1) at the owner's request. The controller then verified the draft and
  amended it.
- The controller independently reproduced the 6-archive header census (tar roots, type flags, no pax, maximum name
  length 51) and all three derived tree digests.
- The controller amended the RED clauses in §5, which were drafted as missing-module REDs; this repository counts a
  missing module as inadmissible.
- The feasibility evidence, registry metadata, and prior spec reviews are in `advisor/`.

**Base:** `origin/main` `5501bc0f` (the PR #320 merge; its source is equivalent to `30e13053`). The branch rebases onto it before its PR.

**Spec review history:**

- v3 round 1: sol FIX 4 WRONG / 4 SMELL and terra FIX 3 WRONG / 0 SMELL. Both call it converging; every terra finding duplicates a sol finding.
- Round-1 folds are marked `[r1]` below, and §10 maps them.

**What changed from v2, in one sentence:** every one of the 9 artifacts now has a byte authority
(`advisor/registry-integrity.json`, `advisor/FEASIBILITY.md`), so every byte is authenticated *before* it is
decompressed, parsed, or written; that single fact lets v3 delete the react18 bootstrap, the hostile-input tar
reader and its limits, the offline mirror, the redirect allowlist, generations, `current.json`, `env.sh`, the lock,
and `--prune`. What remains is content-addressed immutable install directories and verify-on-use.

**Slices:** two, sequential, each with its own 2-round implementation review cap.

- **Slice A — `acquire.mjs` + `pins.json`**: fetch, authenticate, extract, install, verify, print env.
- **Slice B — `gate.mjs` + `exclusions.json`**: enumerate the population, seal the environment, build the native
  helper, run `node --test`, write log + receipt. B imports two functions from A (`verifyInstalled`, `inputDirs`).

## 1. Problem, value, threat model

The Node gate's external inputs (TypeScript 5.9.3, the react18/react19 callable profiles, the grammar archives)
lived only under `/private/tmp`; the macOS daily cleaner purged them, 6 modules could not run, and no receipt
recorded the full-population command. Value: a durable, authenticated, one-command full-population gate on this host
and any macOS host meeting §4.4, with a receipt naming every module, skip, and exclusion. `[r1]` **Supported host: macOS (darwin) only.** On any other platform both tools refuse at startup with `unsupported host`. Linux and Windows volatile-mount rules, path delimiters, and null devices are out of scope for v3.

**Trust boundary (explicit, so reviewers do not re-litigate it).** The inputs are public and pinned by hash. The
operator's own account on their own machine is the trust boundary.

In scope (each has a mechanism and a control):

1. `/tmp` purge → durable root under the data directory, volatile roots refused.
2. Wrong, corrupted, or substituted download bytes → SRI/SHA-256 checked on the complete body before any use.
3. Extraction bugs or platform nondeterminism → the installed tree's digest must equal a pinned derived digest.
4. Stale, torn, or tampered installs → every use (acquire, `env`, gate preflight) recomputes the tree digest.
5. Ambient environment steering tests → the gate builds the child environment from scratch.
6. Undeclared population drift → the population is enumerated from git, config-neutrally, into the receipt.

Out of scope (justified once, here):

- A hostile process running under the operator's account (symlink races, TOCTOU, 0700 modes, `lstat` walks): if
  that process exists, it can edit the repository and the tests too. No custody machinery defends against it.
- Transport policing (redirect host allowlists, port and userinfo rules): content is bound by hash; a hostile
  transport can only cause a refusal. `fetch` follows redirects with Node's defaults.
- Power-loss durability (`fsync` choreography): a torn install fails verification and is refused, never used.
- Resource exhaustion by the pinned artifacts: their sizes are constants (≤ 4.4 MiB compressed, ≤ 23 MiB expanded).
  Only an *unauthenticated* body is capped (§3.3), and it is capped before hashing.
- `PRISM_AUDIT_*` reconstruction (no authority exists); the grammar verifier's Node v26.0.0/darwin/arm64 pin;
  vendored Cargo dependencies; any `src/`, Cargo, dependency, or existing-test change.

## 2. Artifacts, logical inputs, and `pins.json`

Nine downloaded artifacts assemble three logical inputs. Acceptance enumerates by name; no number is load-bearing.

| # | Artifact | Input | Byte authority (pin) | Tar root |
|---|---|---|---|---|
| 1 | `typescript-5.9.3.tgz` | typescript | SRI = Excalidraw `0642e72c` `yarn.lock` (lock SHA-256 at `docs/eval/post317-input-custody.md:29`) **and** registry metadata `https://registry.npmjs.org/typescript/5.9.3` (captured sha256 `80f0d5d2…43d8`), byte-identical | `package` |
| 2 | `@types/react-19.0.10.tgz` | profiles/react19 | yarn.lock **and** registry metadata (`5afcf1cc…c0bc`) | `react` |
| 3 | `csstype-3.1.3.tgz` | profiles/react19 | yarn.lock **and** registry metadata (`849e6cbb…fe7d`) | `package` |
| 4 | `@types/react-18.3.31.tgz` | profiles/react18 | registry metadata (`3250ec2d…5f87`) | `react v18.3` |
| 5 | `csstype-3.2.3.tgz` | profiles/react18 | registry metadata (`011e326a…c49f`) | `package` |
| 6 | `@types/prop-types-15.7.15.tgz` | profiles/react18 | registry metadata (`aeb1a381…60a4`) | `prop-types` |
| 7 | `upstream.tar.gz` | grammar-archives | SHA-256 at `scripts/verify-typescript-grammar.mjs:14-17` | (not extracted) |
| 8 | `javascript-0.23.1.tgz` | grammar-archives | SHA-256 at `…:18-21` | (not extracted) |
| 9 | `tree-sitter-macos-arm64.gz` | grammar-archives | SHA-256 at `…:22-25` | (not extracted) |

Tar roots come from the controller's header census of the SRI-verified bytes (§7 A-acceptance re-records it). Note
that the three `@types` archives are **not** rooted at `package/`; a fixed `package/` rule (v2 §3.5) would have
rejected real artifacts 2, 4, and 6. v3 therefore pins the root name per artifact.

**Cross-checks that already exist in the repository** (independent of the SRIs, kept as second authorities):

- `lib/typescript.js` SHA-256 = `COMPILER_HASH` (`scripts/callable-observations/schema.mjs:18`).
- react19 / react18 profile hashes by the historical formula (`verify-callable-authority.mjs:15-25`):
  `49c6c7a3…baad` / `7b8bbdc8…9b34`; `package.json` versions 19.0.10/3.1.3 and 18.3.31/3.2.3/15.7.15.

**Derived tree digests (§3.4 formula), the install identity of each logical input.** Computed by the controller from
the feasibility trees (bsdtar extraction of SRI-verified bytes) and to be re-derived by the tool from a cold fetch at
acceptance; equality is the acceptance criterion. They are a cache of a deterministic computation over
authenticated bytes, not a new authority; the install path recomputes them every time.

| Input | Files | `tree_sha256` |
|---|---:|---|
| typescript (package root) | 132 | `7e02162c902e5c29ec19fefc573ad55b8ed15fcbda940e737f297c53e9c41f54` |
| profiles (`react18/…`, `react19/…`) | 53 | `aa290dd630dcbfea5523bb627e7e3f6008de70c6fa0601c0df82d1d9975ff443` |
| grammar-archives (3 files) | 3 | `35ba6cbb8757dd812ab334163cac59ef2f6bdcccb1588b4d97d941e1cf579e5f` |

The grammar-archives digest is computable from the three pinned SHA-256s alone (the formula has no size field),
so it needs no download to verify.

**`pins.json` shape** (`scripts/gate-inputs/pins.json`, `"schema":"prism.gate-inputs/1"`):

```json
"inputs": {
  "typescript": {"env":"PRISM_TYPESCRIPT","env_suffix":"lib/typescript.js","tree_sha256":"7e02…",
    "file_sha256":{"lib/typescript.js":"3ae9…"},
    "artifacts":[{"name":"typescript-5.9.3.tgz","url":"https://registry.npmjs.org/typescript/-/typescript-5.9.3.tgz",
      "integrity":"sha512-jl1v…TgSw==","root":"package","dest":".","authority":"…"}]},
  "profiles": {"env":"PRISM_CALLABLE_PROFILES","env_suffix":"","tree_sha256":"aa29…",
    "profile_hash":{"react18":"7b8b…","react19":"49c6…"},
    "artifacts":[{"name":"@types/react-18.3.31.tgz","url":"…","integrity":"sha512-vfEq…",
      "root":"react v18.3","dest":"react18/node_modules/@types/react","authority":"…"}, …]},
  "grammar-archives": {"env":"PRISM_GRAMMAR_ARCHIVES","env_suffix":"","tree_sha256":"35ba…",
    "artifacts":[{"name":"upstream.tar.gz","url":"…","sha256":"4de2…","dest":"upstream.tar.gz","authority":"…"}, …]}
}
```

`authority` is a free-text citation (file:line, or metadata URL + captured sha256). Each artifact has exactly one
of `integrity` (npm, `sha512-` base64) or `sha256` (raw file). `dest` is relative to the input's install dir.

## 3. Slice A — `scripts/gate-inputs/acquire.mjs`

CLI: `node scripts/gate-inputs/acquire.mjs [acquire|verify|env]` (default `acquire`). Exit 0 on success, 2 on
refusal; every refusal names the input, artifact, and rule.

### 3.1 Root and layout

- Root: `$PRISM_GATE_INPUTS_ROOT`, else `$XDG_DATA_HOME/prism/gate-inputs`, else
  `~/.local/share/prism/gate-inputs`. Created with `mkdir -p`.
- Volatile-root refusal (reason `volatile root`, no override): the `realpath` of the root's longest existing prefix
  is equal to or under `realpath(os.tmpdir())`, `realpath('/tmp')`, or `realpath('/var/folders')`. `[r1]` A deny root that does not exist is skipped, not an error.
- Layout, **derived from pins, no pointer file**: `<root>/<input>/<tree_sha256>/`. Install dirs are immutable and
  content-addressed; the tool never modifies or deletes one. Replacement is impossible by construction: changed pins
  name a different directory. The only thing the tool ever deletes is the `<root>/.stage-<random>/` that **this invocation** created. `[r1]` It never scavenges other stages. A crashed invocation's stage is inert, and the README documents manual cleanup.
- Env values are derived: `PRISM_TYPESCRIPT=<root>/typescript/<digest>/lib/typescript.js`,
  `PRISM_CALLABLE_PROFILES=<root>/profiles/<digest>`, `PRISM_GRAMMAR_ARCHIVES=<root>/grammar-archives/<digest>`.
  `env.sh` does not exist; `acquire.mjs env` verifies and prints the three `export` lines (single-quoted).

### 3.2 Install algorithm (per logical input, independent)

1. If `<root>/<input>/<tree_sha256>` exists: recompute the tree digest (§3.4). Equal → `verified`, no fetch, no
   write. Unequal → refuse `install corrupt: remove <dir> and re-run` (never auto-repaired).
2. Else fetch every artifact (§3.3) and authenticate its complete bytes. Any failure → refuse before any
   decompression or write for this input.
3. Extract each archive (§3.5) into `<root>/.stage-<random>/<dest>`; copy raw files (grammar) to `<dest>`.
4. Run the cross-checks: `file_sha256` entries; `profile_hash` via the historical formula (§3.4); `package.json`
   versions. Then compute the tree digest of the stage and require equality with `tree_sha256`.
5. `mkdir -p <root>/<input>`; `rename(stage, <root>/<input>/<tree_sha256>)`. On `EEXIST`/`ENOTEMPTY` (a concurrent
   acquirer won) go to step 1 for that input. A `rename` into a new name is atomic; no lock is needed because two
   acquirers can only race to create the same content-addressed directory.
6. On any failure: remove own stage, leave everything else untouched, continue to the next input, exit 2 at the end.

Threat 4 (§1) is closed by step 1 on every run plus `verify`, which is step 1 for all inputs and is what the gate
calls. A crash at any point leaves either an inert stage, which is ignored by everyone and never auto-removed (`[r1]`), or a complete
renamed directory.

### 3.3 Fetch and authentication

- `fetch(url, {signal: AbortSignal.timeout(120_000)})`, Node defaults for redirects. Refuse non-`ok` status.
- Read the body as a stream with a running count; refuse above **64 MiB** (the only limit the tool has, and it
  guards unauthenticated bytes only). Buffer the complete body in memory (the pinned artifacts are ≤ 4.4 MiB).
- Compute SHA-512 (npm) or SHA-256 (raw) over the complete buffer; compare to the pin (SRI as base64, exact string
  match after `sha512-`). Mismatch → refuse `byte authority: <artifact>`; the buffer is discarded.
- Nothing is decompressed, parsed, or written before this comparison succeeds. There is no `--from` mirror; tests
  inject `fetch` through the exported `acquire({fetch})` option.

### 3.4 Tree digest formula (v1) and the historical cross-check

- Rows: for every regular file under the directory, recursively, `[relpath, sha256hex(bytes)]`, where `relpath`
  uses `/` and is relative to the input dir. Directories contribute no rows (they are implied by paths; empty
  directories are irrelevant to every consumer). A symlink or any non-regular, non-directory entry refuses.
- `relpath` must match `/^[\x20-\x7e]+$/` and contain no `\`; otherwise refuse `non-ascii path`. (Every pinned
  path is ASCII; this makes sort order unambiguous.)
- Sort rows by `relpath` with JavaScript string comparison (`<`), which is byte order for ASCII.
- `tree_sha256 = sha256hex(utf8(JSON.stringify(rows)))`. The test suite pins one fixed two-file fixture to a fixed
  digest so the formula cannot drift silently (the reviewer recomputes it with any sha256 tool).
- Historical profile cross-check: `sha256(JSON.stringify(readTree(profileDir)))` exactly as
  `verify-callable-authority.mjs:15-22` (sorted `readdirSync`, UTF-8 text). It is lossy for non-UTF-8 bytes, which
  is why it is a cross-check to the historical pin and not the identity; the raw digest above is the identity.

### 3.5 Archive reader — the accepted tar profile is exactly what the six pinned archives use

Because the archive bytes are authenticated constants, the reader is not a hostile-input parser. It accepts exactly
the profile observed in the header census of all six artifacts (`{typeflag 0/NUL/5}`, no pax, no GNU, empty
`prefix`, max name 51 bytes) and refuses everything else. Its correctness on the six real inputs is proven by the
tree digest after extraction, not by the reader's own rules.

- Input: `zlib.gunzipSync(buffer)` (in memory). Truncated gzip refuses.
- Read 512-byte headers. A header of 512 zero bytes ends the archive; nothing after it is read.
- Fields: `name` = bytes 0–99 up to the first NUL; `size` = bytes 124–135, ASCII octal, NUL/space terminated;
  `typeflag` = byte 156; `prefix` = bytes 345–499 must be all NUL, else refuse `ustar prefix unsupported`.
- `typeflag` `'0'` or NUL: regular file; the next `ceil(size/512)` blocks are its content; content past the end of
  the buffer refuses `truncated`. `'5'`: directory; `size` must be 0. Any other typeflag refuses
  `unsupported tar entry <typeflag> <name>` (this covers symlinks, hardlinks, devices, pax `x`/`g`, GNU `L`/`K`).
- Path rule: `name` must equal `<root>`, `<root>/`, or start with `<root>/`; the remainder is split on `/`; every
  component must be non-empty, not `.` or `..`, match `/^[\x20-\x7e]+$/`, and contain no `\`. Duplicates refuse.
- Files are written to `<stage>/<dest>/<remainder>` (parents `mkdir -p`) with default modes (0644/0755); no mode,
  owner, or mtime is preserved. No consumer executes anything from these trees.
- Grammar artifacts are not extracted; the raw verified bytes are written to `<stage>/<dest>`.

**Why not system `tar`?** It would be acceptable in principle (verified bytes in, tree digest out), but it adds a
prerequisite to acquisition, a `--strip-components` rule, a dialect question (bsdtar vs GNU) and would still need
the census above to know each archive's root. The in-Node reader above is ~35 lines, dialect-free, produces the
census for free (`entries: N files, M dirs` in the acquire log), and its only failure mode — a wrong tree — is
caught by the derived digest. A pre-extraction listing check therefore earns nothing beyond the path rule already
in the reader.

## 4. Slice B — `scripts/gate-inputs/gate.mjs`

CLI: `node scripts/gate-inputs/gate.mjs --out <new-dir>` (from any cwd). Exit 0 = all tests passed; 1 = tests
failed; 2 = refused before tests (stage named in the receipt). `--dry-run` prints the plan (population, argv, cwd,
child env) and exits 0 without building or running.

### 4.1 Population (config-neutral, NUL-safe)

- Repo root = `path.resolve(dirname(fileURLToPath(import.meta.url)), '../..')`; every child cwd is the repo root.
- Enumeration, run under the sealed env (§4.2) so `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_NOSYSTEM` apply:
  `git ls-files -z --cached --others --exclude-per-directory=.gitignore -- '*.test.mjs'`.
  `--exclude-per-directory` reads the **working-tree** `.gitignore` files. It does not read `.git/info/exclude` or
  `core.excludesFile`. `[r1]` To bind ignores to committed bytes, the gate first runs
  `git status --porcelain=v1 -z --untracked-files=all --ignored=no -- ':(glob)**/.gitignore'` (sealed env) and
  refuses with `population ignore state` if any `.gitignore` is untracked, modified, or deleted. So the population is
  determined by committed ignore rules only. `-z` emits raw bytes. Decode as UTF-8 with `fatal:true` (refuse otherwise); refuse duplicate
  paths (an unmerged index); refuse a listed path that is not a regular file. Today this yields 48 modules.
- `scripts/gate-inputs/exclusions.json`: `[{"path":…,"reason":…}]`. Today one row:
  `docs/eval/receiver-closure/audit-imported-props-source.test.mjs`, `inputs-not-reconstructible`. An exclusion
  whose path is not in the enumeration refuses (`stale exclusion`). Active = enumeration − exclusions, sorted.
- Population digest = `sha256hex(JSON.stringify(active))`. An undeclared new test is included automatically (it
  cannot be silently omitted); a broken one fails the gate.
- `[r1]` Every active path is passed to Node as `./<path>`, so a filename beginning with `-` can never be parsed as an
  option.

### 4.2 Sealed environment and tool resolution

The child env is built from an empty object. **Pass-through if set:** `HOME`, `PATH`, `LANG`, `LC_ALL`, `TMPDIR`,
`CARGO_HOME`, `RUSTUP_HOME`, `RUSTUP_TOOLCHAIN`. **Set:** `PATH = dirname(process.execPath) + ':' + inherited PATH`
(so a test that spawns literal `node` gets the runner's node); `GIT_CONFIG_GLOBAL=/dev/null`;
`GIT_CONFIG_NOSYSTEM=1`; the three input variables from `inputDirs(root)` after `verifyInstalled` passes;
`PRISM_MEMBERSHIP_NATIVE` after the build. **Everything else is absent**, which covers `NODE_OPTIONS`, `NODE_PATH`,
every `PRISM_*` override (`PRISM_CALLABLE_IMPLEMENTATION`, `PRISM_CALLABLE_BASELINE`, `PRISM_OBSERVER_MODULE`,
`PRISM_PARAMETER_FREQUENCY_*`, `PRISM_AUDIT_*`), `CARGO_TARGET_DIR`, `CARGO_BUILD_*`, `RUSTFLAGS`, `USER`, `SHELL`.

Declared, accepted host authority: the operator's `PATH` (a rogue `git` on it is inside the trust boundary),
`$CARGO_HOME/config.toml`, and the rustup/mise selection those variables and `PATH` express. PATH shims that need
additional locator variables to work are unsupported; they surface as a `build` or `preflight` refusal with the
tool's own stderr.

Preflight (stage `preflight`): resolve `git` and `cargo` to absolute paths by scanning the sealed `PATH` for an
executable; record path and `--version` output; `node` is `process.execPath`. Then `verifyInstalled(root)` from
Slice A (recomputes all three tree digests; never downloads; failure names the input and says to run `acquire`).

### 4.3 Build, run, publish

- Stage `build`: `<cargo> build --frozen --offline --example project_membership_census --message-format=json`
  with cwd = repo, sealed env. The executable path is the `compiler-artifact` message whose `target.name` is
  `project_membership_census` and whose `executable` is non-null; record its SHA-256. Any cargo failure (including
  an unpopulated registry cache, since `--frozen` is offline) is stage `build`, distinct from a test failure; the
  spec does not parse cargo's message text. Prerequisite (§4.4), not acquired.
- Stage `tests`: `node --test --test-concurrency=2 --test-reporter=tap ./<active paths>` with cwd = repo, sealed env,
  stdout+stderr streamed to `<out>/log.txt`. Totals are parsed from the TAP trailer (`# tests/pass/fail/skipped`)
  and skips are collected from `# SKIP` lines by test name; if the trailer is absent, totals are `null` and the
  child's exit status still governs.
- Publication (`[r1]`): resolve `out`, `mkdirSync(dirname(out), {recursive:true})`, then **exclusively** create the leaf
  with `mkdirSync(out)`, which is non-recursive and refuses `EEXIST`. That leaf creation is the first act and happens
  before any other work. If output acquisition fails (existing leaf, or parent not creatable), the gate writes
  nothing, prints the reason to stderr, and exits 2. Once the gate owns the leaf, `log.txt` and `receipt.json` are
  written only inside it, and the receipt is written on **every later** completion path (status `passed`, `failed`,
  or `refused` + stage). The controller copies an approved receipt to
  `docs/eval/gate-input-durability/receipt.md`; the gate never writes under `docs/`.
- `receipt.json` (canonical JSON, no timestamps): repo `HEAD` and dirty flag; population (active list, exclusions,
  digest); tool paths and versions; root and the three verified input dirs with digests; native binary path and
  hash; the exact child argv, cwd, and env (allowlisted keys only, intended for publication); totals; skips by
  name; status and stage; exit code.

### 4.4 Host prerequisites (declared, not acquired)

Rust toolchain compatible with `Cargo.lock` and a Cargo registry cache already satisfying it (any host that has
built this repo once); `node` ≥ 24; `git`; `tar` only for the grammar verifier on a Node v26.0.0/darwin/arm64 host.
macOS only (§1). "Any machine" means "any macOS host meeting this list". The receipt records the runtime tuple
`{node version, platform, arch}` (`[r1]`).

## 5. Tests — fail-first RED and the minimal sufficient controls

Every control asserts the complete expected record (full result object or full error message), and each refusal
control also asserts that nothing was installed for that input and no `.stage-*` remains.

**Slice A (`acquire.test.mjs`; offline; injected `fetch`; synthetic archives built by a ~20-line in-test ustar
writer; synthetic `pins` computed from those archives; test root under `<repo>/target/`, which is durable):**

- **RED (behavioral):** before implementing, a runnable same-signature digest adapter must run the complete A8
  fixed-fixture assertion and fail on the concrete digest value. The adapter deliberately includes directory rows
  in the tree digest, which is exactly the r2 W8 non-canonical defect. A missing module, a compile or setup error,
  or zero selected tests is inadmissible as RED.
- A1 happy path: three inputs installed at `<root>/<input>/<digest>`; `env` prints three exact lines; `verify`
  passes; a second run makes zero `fetch` calls and creates no directory.
- A2 byte authority: wrong SRI (npm) and wrong SHA-256 (raw) refuse before decompression (the fake fetch records
  that no gunzip happened via a corrupt-but-mismatched body); non-`ok` status refuses; body over the cap refuses
  before hashing.
- A3 reader, one row each: symlink typeflag `2`; pax `x`; entry outside the pinned root; `..` component;
  non-empty `prefix`; directory with non-zero size; truncated content; duplicate path. Positive row: an archive
  with directory entries and nested files extracts and digests correctly (this is A1's fixture).
- A4 identity: pins with a wrong `tree_sha256` refuse after extraction, nothing installed; wrong `profile_hash`
  refuses; wrong `file_sha256` refuses.
- A5 verify-on-use: tamper one installed file → `verify` and `acquire` both refuse naming input and path;
  `acquire` makes zero `fetch` calls; `env` refuses.
- A6 rename race (`[r1]`): through a test hook that runs **after** the initial absence check and **before** `rename`,
  create the final directory with correct content. Acquire must hit `EEXIST`/`ENOTEMPTY`, re-verify, report
  `verified`, and remove its own stage. A foreign `.stage-*` directory that exists beforehand survives untouched.
- A7 root: a root under `os.tmpdir()` refuses; `PRISM_GATE_INPUTS_ROOT` is honored.
- A8 formula fixture: fixed two-file tree → fixed digest constant.
- A9 drift guards on the real `pins.json`: `COMPILER_HASH` (regex on `schema.mjs`), profile versions and hashes
  (regex on `verify-callable-authority.mjs:24-25`), grammar URLs and SHA-256s (regex on
  `verify-typescript-grammar.mjs`, which cannot be imported), the grammar-archives `tree_sha256` recomputed from
  the three pins, and the nine artifact names enumerated exactly. `[r1]` A9 also checks that all six npm rows
  (name, version, URL, integrity, metadata SHA-256) equal `advisor/registry-integrity.json`, and that all three
  `tree_sha256` values equal the §2 constants.

**Slice B (`gate.test.mjs`; unit-level via exported functions plus `--dry-run`; never builds the real example):**

- **RED (behavioral):** before implementing, a runnable same-signature env-builder adapter must run the complete B2
  `deepEqual` and fail on the concrete leaked key. The adapter deliberately copies `process.env` and deletes only
  the `PRISM_*` keys, so `NODE_OPTIONS` survives. A missing module or setup error is inadmissible as RED.
- B1 population: temp git repo with tracked `a.test.mjs`, untracked `b.test.mjs` (included), `.gitignore`d
  `target/c.test.mjs` (excluded), a filename containing a newline (listed intact); a global gitignore hiding
  `*.test.mjs` via a `GIT_CONFIG_GLOBAL` file does **not** hide `b` (config-neutral); a stale exclusion refuses.
- B2 sealing: an inherited env containing `NODE_OPTIONS`, `PRISM_CALLABLE_IMPLEMENTATION`, `CARGO_TARGET_DIR`,
  `CARGO_HOME`, `RUSTUP_HOME`, `USER` → `deepEqual` against the exact expected child env (allowlist retained,
  everything else absent, `PATH` prefixed with the runner's node dir, git config isolated).
- B3 build: a fake `cargo` on a temp `PATH` printing one `compiler-artifact` line → executable path and hash are
  recorded, argv contains `--frozen`; a fake `cargo` exiting 101 → stage `build`, no tests run, receipt written.
- B4 run: fixture population of one passing and one failing module → exit 1, totals `{tests:2,pass:1,fail:1}`,
  `log.txt` has both; a module with a `{skip:'reason'}` test → skips list names it.
- B5 preflight: `PATH` without `git` → stage `preflight`; a tampered input in a test root → stage `preflight`
  naming the input.
- B6 publication (`[r1]`): an existing `--out` leaf gives exit 2 with a stderr reason, nothing written anywhere, and no
  receipt. A missing parent directory is created and the run proceeds.
- B8 ignore state and argv (`[r1]`):
  - An untracked `future/.gitignore` containing `*.test.mjs` refuses with `population ignore state`.
  - A tracked `--dash.test.mjs` is passed as `./--dash.test.mjs` and runs as a test.
  - A non-darwin `process.platform` (injected) refuses with `unsupported host`.
- B7 cwd: `repoRoot()` equals `git rev-parse --show-toplevel` from another cwd.

## 6. Budget (honest executable lines: non-blank, non-comment, JavaScript ≤ 100 columns)

| Slice | Helper cap | Test cap | Forecast |
|---|---:|---:|---|
| A (`acquire.mjs`) | 300 `[r1]` | 350 | helper 220–270 (fetch 15, reader 35, digest 15, install 45, CLI/env/root 40); tests 270–330 |
| B (`gate.mjs`) | 190 | 260 | helper 140–180; tests 190–240 |
| **Combined** | **490** | **610** | ~360–450 / ~460–570 |

Early stop at 95% of either bucket; a breach is a stop, never inflation. This is above the ≤ 400 / ≤ 400 target on
the test side by design: the fixture tar writer and synthetic-pins builder (~50), table-driven reader rows (~30),
git fixture repos (~30), and complete-record env/receipt assertions are what make the controls admissible, and
compressing them into dense lines would violate the counting rule. The helper side is comfortably under 400 because
the reader has no limits and the installer has no transactions. If the implementer forecasts a breach, the first
thing to cut is B7 and A6, not any refusal row.

## 7. Acceptance (controller, after each slice's implementation review approves)

**Slice A:**

1. Live: `node scripts/gate-inputs/acquire.mjs` into the default root. The log names all nine artifacts as
   `verified` with their census (`entries: N files, M dirs`; expected 132/0, 24/4, 5/0, 15/2, 5/0, 4/1, and three
   raw files), and the three install dirs are named by the §2 digests.
2. Cold identity: `PRISM_GATE_INPUTS_ROOT=<fresh durable dir>` acquisition yields the same three digests.
3. `acquire.mjs verify` and `env` succeed; `verify` has no fetch code path (reviewer confirms by reading).
4. Default Rust suite unchanged (recorded, not re-baselined).

**Slice B:**

1. `node scripts/gate-inputs/gate.mjs --out target/gate-runs/<date>` runs the full active population: 0 failures;
   skips are host-conditional (`[r1]`), checked against the receipt's runtime tuple. On Node v26.0.0/darwin/arm64 with
   acquired archives there must be **zero** grammar skips. On any other tuple, exactly the grammar verifier's 2 named
   host-pin skips are expected. Exclusions:
   the one `PRISM_AUDIT_*` module, named. Totals recorded from the receipt, not from the terminal.
2. Receipt copied to `docs/eval/gate-input-durability/receipt.md` with the population digest, tool versions, input
   digests, native hash, and the exact command.
3. Default Rust suite 4,559 / 0 / 1, recorded, not re-baselined.

## 8. Review, stop rules, owned paths

Two implementation review rounds per slice; findings tagged WRONG/SMELL with a concrete input; at a cap the
controller classifies convergence before acting. A finding that asks for machinery excluded by §1's threat model is
answered by citing §1, not by adding the machinery; if the reviewer shows the threat is *inside* the boundary, that
is a spec change for the owner, not an implementation fold.

**STOP:** any `src/`, Cargo, or existing-test change; any `npm install` or lifecycle execution; any
decompression, parse, or write of an artifact before its byte authority passes; any install without a tree-digest
match; any pin without a cited authority; deleting anything other than the tool's own `.stage-*`; the gate
downloading anything; a budget breach.

**Owned paths.** A: `scripts/gate-inputs/{acquire.mjs,pins.json,acquire.test.mjs,README.md}`.
B: `scripts/gate-inputs/{gate.mjs,exclusions.json,gate.test.mjs}`, a README section, and (controller, at
acceptance) `docs/eval/gate-input-durability/receipt.md`. Both new test modules join the population (50 total,
49 active) and need no external inputs.

## 9. Disposition of every sol finding (r1 and r2)

Legend: **moot** = the condition no longer exists in v3; **design** = closed by construction; **rule** = closed by
an explicit clause; **out** = explicitly outside §1's trust boundary.

| Finding | Disposition | Where |
|---|---|---|
| r1 W1 / r2 W1 react18 bootstrap; observed SRI promoted to authority | **moot** — all six npm SRIs come from registry metadata (three also from yarn.lock); there is no bootstrap path and no `--from` | §2, §3.3 |
| r1 W2 unbounded, non-portable tar listing | **design** — bytes are authenticated constants before gunzip; in-Node reader; no system tar; no listing step | §3.5 |
| r2 W2 pax/ustar contract undefined; node-tar precedence | **design** — the accepted profile is exactly the census of the six pinned archives (`0`/NUL/`5`, empty prefix, no pax/GNU); everything else refuses; one reader, constant input, tree-digest proof; positive fixture with dir entries; real census re-recorded at acceptance | §3.5, A3, §7 |
| r1 W3 non-atomic directory replacement | **design** — content-addressed immutable dirs; nothing is ever replaced | §3.1 |
| r2 W3 not crash-durable; `env.sh` second pointer | **design** — no pointer files at all (paths derive from pins); `env` is computed after verify; power-loss durability is **out** (torn dir → refusal, never use) | §1, §3.1, §3.2 |
| r2 W4 lock recovery; prune under a running gate | **moot** — no lock, no prune, the tool deletes only its own stage; races converge on `rename` EEXIST; nothing can delete an install under the gate except the operator | §3.2, A6 |
| r1 W4 unsealed runner | **rule** — env from empty; repo cwd from `import.meta.url`; absolute tools; `--frozen` | §4.2, B2 |
| r2 W5 env contract contradictory; CARGO_HOME/RUSTUP_HOME; HOME/gitconfig | **rule** — explicit pass-through list incl. `CARGO_HOME`/`RUSTUP_HOME`/`RUSTUP_TOOLCHAIN`; git config isolated via `GIT_CONFIG_GLOBAL=/dev/null` + `GIT_CONFIG_NOSYSTEM=1`; `HOME` kept for rustup default homes; cargo home config declared accepted authority; `USER`/`SHELL` dropped; no test contradicts the allowlist (B3 uses a fake `cargo` on `PATH`, not `CARGO_HOME`) | §4.2 |
| r1 W5 population scope | **rule** — repo-wide `git ls-files` | §4.1 |
| r2 W6 population host-dependent, not NUL-safe | **rule** — `-z`, `--exclude-per-directory=.gitignore` (no `--exclude-standard`), UTF-8 `fatal`, duplicate refusal, sealed git env; B1 proves a global exclude cannot hide a test | §4.1, B1 |
| r1 W6 fresh machine cannot build offline | **rule** — declared prerequisite; cargo failure is stage `build`, distinct from tests by stage, without parsing cargo text | §4.3, §4.4 |
| r1 W7 counts inconsistent | **design** — `pins.json` is the enumeration; acceptance by name | §2, §7 |
| r1 W8 TypeScript `lib/` unauthenticated | **design** — whole-tree derived digest, recomputed at install and every verify; `COMPILER_HASH` kept as cross-check | §2, §3.4, A5 |
| r2 W8 manifest not canonical (dir sizes, sort order) | **rule** — files-only rows, ASCII-only paths, exact serialization and sort defined, fixed fixture digest | §3.4, A8 |
| r2 W7 mirror `lstat`/copy symlink race | **moot** — no mirror; also **out** (same-account attacker) | §1, §3.3 |
| r1 S1 UTF-8-lossy tree formula | **design** — raw digest is the identity; historical formula is a cross-check only | §3.4 |
| r1 S2 redirects, TOCTOU, 0700, lock, fsync | **out** — content bound by hash; same-account attacker and power loss outside the boundary; no lock needed | §1, §3.2 |
| r1 S3 / r2 S1 controls insufficient; no positive reader test | **rule** — §5 list; A3 positive row; acceptance census | §5, §7 |
| r2 S2 receipt publication underspecified | **rule** — `--out <new-dir>` with `mkdirSync` EEXIST refusal; `log.txt` + `receipt.json` on every completion path; controller copies to `docs/` | §4.3, B6 |
| r1 S4 / r2 S3 budget unrealistic (A forecast 1,600–2,000) | **design** — the forecast collapses because the reader has no hostile-input rules (~35 lines), the installer has no transactions, and there is no transport policy or mirror; new caps and forecast in §6 | §6 |

## 10. v3 round-1 fold map

| Finding | Source | Fold |
|---|---|---|
| Mutable working-tree `.gitignore` hides tests | sol W1, terra W1 | §4.1: refuse unless every `.gitignore` is clean and tracked; B8 |
| Leading-dash filename parsed as a Node option | sol W1 | §4.1 and §4.3: `./` prefix; B8 |
| `--out` parent missing; `EEXIST` receipt contradiction | sol W2, terra W3 | §4.3: create the parent recursively, create the leaf exclusively; receipt only after leaf ownership; B6 |
| Hardcoded grammar skip count; PATH `node` is v26.0.0 here | sol W3, terra W2 | §7: host-conditional skips; runtime tuple in the receipt |
| "Any host" versus the macOS-specific rules | sol W4 | §1 and §4.4: macOS-only; `unsupported host` refusal; missing deny roots skipped |
| Stage scavenging ambiguity; A6 not exercising the rename race | sol S1 | §3.1 and §3.2: no scavenging, manual cleanup; A6 hook between the check and `rename` |
| Slice A forecast crosses its own 95% stop | sol S2 | §6: A helper cap 300; caps rebalanced |
| A9 does not guard the npm pins or derived digests | sol S3 | §5 A9: six-row registry comparison plus the three digest constants |
| Base object unreachable | sol S4 | `5501bc0f` fetched; base line clarified |
