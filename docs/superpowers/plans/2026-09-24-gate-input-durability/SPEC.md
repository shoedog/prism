# Gate-input durability: pinned, re-acquirable Node test-gate inputs

**Status:** DRAFT for independent spec review (review cap: 2 rounds). No implementation is authorized until the
review returns APPROVE, or FIX with the fixes folded in.

**Base:** `origin/main` `30e13053` (the PR #319 merge).

**Owner decision:** on 2026-09-23 the owner chose "Gate-input durability" as the next slice, to run in parallel with
the native positional-gap review.

## 1. Problem and value

The repository's Node test gate depends on external inputs that previous sessions kept only under `/private/tmp`.
The macOS daily `/tmp` cleaner deletes files there that have not been accessed for 3 days. On 2026-09-23 the
following were found **gone**:

- `…/public-inputs/typescript-5.9.3`
- `callable-authority-profiles`
- the grammar archives

As a result:

- 6 modules could not run: 5 in `scripts/callable-observations/` plus `verify-callable-authority`.
- The grammar tamper test skips.
- No receipt in the repository records the **exact full-population command or module inventory**; both lived in
  external orchestration roots.

Every later JS/TS slice currently reports "the largest runnable subset" with exclusions, instead of the full gate.

Value: a single repository-owned tool that re-acquires every re-acquirable input from pinned public sources into a
**durable** location, authenticates each input against the pins the repository already holds, and runs the
**declared** full active Node population with one command. Honest full-population gates become reproducible on any
machine.

**Non-goals:**

- No `src/`, Cargo, or dependency change.
- No new pins invented where the repository has an authority. The one bounded exception is §3.3.
- Do not reconstruct `PRISM_AUDIT_SITES` (§3.5).
- Do not change any existing test's behavior.

## 2. Input contract being made durable

Every pin below already exists in the repository. The implementer must read the current values from the cited
location, not from this table.

| Input (env var) | Required layout | Authority already in the repo |
|---|---|---|
| `PRISM_TYPESCRIPT` | absolute path to `<pkg>/lib/typescript.js` of the extracted npm `typescript@5.9.3` package; the whole `lib/` directory must be present | `typescript.js` SHA-256 `3ae902c9…7675` (`scripts/callable-observations/schema.mjs:18`); version 5.9.3 |
| `PRISM_CALLABLE_PROFILES` | `<root>/react19/node_modules/{@types/react,csstype}/**` and `<root>/react18/node_modules/{@types/react,@types/prop-types,csstype}/**`; complete package contents; no symlinks; no extra files | per-profile tree hashes `49c6c7a3…baad` (react19) and `7b8bbdc8…9b34` (react18); versions from `docs/eval/receiver-closure/verify-callable-authority.mjs:24-25`; tree formula `sha256(JSON.stringify(readTree(profile)))` from the same file, lines 15-22; `@types/prop-types` 15.7.15 per `2026-09-06-callable-authority-readout.md:42-43` |
| `PRISM_GRAMMAR_ARCHIVES` | flat directory holding exactly `upstream.tar.gz`, `javascript-0.23.1.tgz`, `tree-sitter-macos-arm64.gz` | URL and SHA-256 pins in `scripts/verify-typescript-grammar.mjs:14-26` |
| `PRISM_MEMBERSHIP_NATIVE` | absolute path to a freshly built `examples/project_membership_census` | built locally; no pin (its hash is recorded, not pinned) |

## 3. Design

### 3.1 Owned paths (touch nothing else)

- `scripts/gate-inputs/pins.json` — the machine-readable pin set, schema `prism.gate-input-pins/1`. Every entry
  names its repository authority by path and line.
- `scripts/gate-inputs/acquire.mjs` — acquisition, authentication, and durable layout.
- `scripts/gate-inputs/node-population.json` — the declared Node population, schema
  `prism.node-population/1`. It lists **every** `*.test.mjs` in the repository, as either `active` or `excluded`
  with a reason code.
- `scripts/gate-inputs/run-node-gate.mjs` — the one-command full gate runner.
- `scripts/gate-inputs/index.test.mjs` — offline tests.
- `scripts/gate-inputs/README.md` — the runbook.
- `docs/eval/gate-input-durability/receipt.md` — the live acquisition and full-gate receipt, written by the
  controller after approval.

### 3.2 Durable root

- Default root:
  - `$PRISM_GATE_INPUTS_ROOT`, if set
  - otherwise `$XDG_DATA_HOME/prism/gate-inputs`
  - otherwise `~/.local/share/prism/gate-inputs`
- The tool **refuses** any root that resolves (by `realpath` of the nearest existing ancestor) under
  `os.tmpdir()`, `/tmp`, `/private/tmp`, `/var/folders`, or `/private/var/folders`. The refusal reason is
  `volatile root`, and there is no override flag.
- Layout under the root:
  - `typescript-5.9.3/package/**`
  - `profiles/react18/node_modules/**`
  - `profiles/react19/node_modules/**`
  - `grammar-archives/{3 files}`
  - `inputs-receipt.json` (§3.6)
  - `env.sh` (§3.6)

### 3.3 Acquisition and authentication

- Sources are pinned HTTPS URLs only:
  - npm tarballs: `https://registry.npmjs.org/<name>/-/<basename>-<version>.tgz`
  - the three grammar URLs, exactly as pinned
- `--from <dir>` acquires offline from a local mirror directory of the same file names. The tests use it, and it
  lets a machine without network restore from a durable mirror.
- Every tarball is fully downloaded to parent-owned staging **inside the root**, never in `/tmp`. It is
  authenticated **before any extraction**:
  - **TypeScript 5.9.3, `@types/react` 19.0.10, `csstype` 3.1.3:** npm `sha512` SRI must equal the value in
    `pins.json`. Those values are copied from Excalidraw `0642e72c` `yarn.lock`, whose file SHA-256
    `a2a92a77…ecd0` is pinned at `docs/eval/post317-input-custody.md:29`. Each pin cites its lock line.
  - **react18 packages** (`@types/react` 18.3.31, `csstype` 3.2.3, `@types/prop-types` 15.7.15): the repository
    holds **no tarball integrity**; the only authority is the react18 **tree hash**. So the tool downloads, extracts
    into staging, and authenticates the assembled tree against `7b8bbdc8…9b34` before install.
    - The implementer records each tarball's observed `sha512` into `pins.json` with `"authority":
      "tree-hash-bootstrap"`.
    - From then on the tarball SRI is checked as well, and a mismatch refuses.
    - This is the one bounded new-pin exception. It is safe only because the pre-existing tree-hash pin
      authenticates the content.
  - **Grammar archives:** SHA-256 must equal `verify-typescript-grammar.mjs`'s pins. `pins.json` duplicates them,
    because that script cannot be imported: it asserts Node v26 at top level. A drift test (§5) proves equality by
    parsing the script's source text.
- Extraction:
  - No `npm install`, no lifecycle scripts, no package-manager execution.
  - Use the system `tar` (already required by the grammar verifier).
  - Before extracting, list entries with `tar -tvzf`. Refuse, before extraction, any entry that:
    - is absolute
    - has a `..` component
    - is outside the single `package/` prefix
    - is not a regular file or directory (symlink, hardlink, device)
  - Extract into staging, then move `package/` to `node_modules/<name>`.
- After assembling a profile:
  - Its tree hash must equal the pin, using the **exact** `readTree` formula (symlinks refused, sorted, UTF-8
    text).
  - The `package.json` versions must equal the pins.
  - `typescript.js` must equal its SHA-256, and `lib/` must exist.
- Install is atomic per input: build the whole input in staging, then `rename` over the final path, replacing a
  prior copy only after the new one fully authenticates.
  - A failed input leaves the prior installed copy untouched and its staging removed.
  - An already-present input that re-authenticates is left untouched and reported as `verified`.
- Only 5 npm packages and 3 grammar files exist, so every fetch runs sequentially.
- Per-file size cap: 64 MiB. Per-fetch timeout: 120 s. `fetch` has no redirects outside the pinned host families.

### 3.4 Declared Node population and runner

- `node-population.json` enumerates **all** `*.test.mjs` under `scripts/` and `docs/`. Each entry is either:
  - `{path, status:"active", env:[...]}`, or
  - `{path, status:"excluded", reason_code, reason}`.
- Allowed exclusion reason codes:
  - `inputs-not-reconstructible`: `docs/eval/receiver-closure/audit-imported-props-source.test.mjs`, whose
    `PRISM_AUDIT_SITES` is historical Prism output with no hash
  - `host-skip-only`: never used to exclude a module
- Host-dependent **skips** inside active modules stay skips and are reported, not excluded. For example, the grammar
  tamper tests on a Node version other than v26.0.0.
- `run-node-gate.mjs --root <root> --log <new-file>` does the following, in order:
  1. Validates `inputs-receipt.json` and re-authenticates every input. It never downloads.
  2. Builds `project_membership_census` with `cargo build --offline --example project_membership_census` into
     `target/`.
  3. Exports the four env vars.
  4. Runs `node --test --test-concurrency=2 <every active path, sorted>`.
  5. Writes the log with new-only publication.
  6. Prints totals and the exclusion list, and exits nonzero on any failure.
- The runner never modifies inputs.

### 3.5 Explicitly out of scope

- `PRISM_AUDIT_*` reconstruction (`SITES` has no authority).
- Upgrading the grammar verifier's Node v26 host pin.
- Changing which modules exist.
- Any Rust change.
- The native positional-gap observer, which stays on its own lane.

### 3.6 Receipts

- `inputs-receipt.json` records, per input:
  - URL
  - tarball SHA-256 and SRI
  - extracted tree hash, or file SHA-256
  - installed path
  - status: `installed | verified`
- It also records the tool version (`acquire.mjs` SHA-256).
- `env.sh` contains only `export` lines for `PRISM_TYPESCRIPT`, `PRISM_CALLABLE_PROFILES`, and
  `PRISM_GRAMMAR_ARCHIVES`, with shell-quoted absolute paths.
- Canonical JSON, one trailing newline, no timestamps.

## 4. Budget (honest executable lines)

Counting rule: non-blank lines that are not comment-only. JavaScript lines are ≤ 100 columns; the only exception is
an unsplittable single string literal. JSON data files are not counted.

| Bucket | Contents | Cap |
|---|---|---|
| helper | `acquire.mjs` + `run-node-gate.mjs` | ≤ 650 |
| tests | `index.test.mjs` | ≤ 550 |
| combined | helper + tests | ≤ 1,200 |

Early stop at 95% of either bucket's forecast.

The controller's forecast is helper about 420–520 and tests about 380–480. This was calibrated against the observed
27 non-whitespace characters per reflowed JavaScript line. A breach stops and returns to the owner; it is never
inflated silently.

## 5. Fail-first tests (offline; `--from` mirror built in-test)

Behavioral RED first, before implementation: run the complete tree-hash assertion against a runnable adapter that
omits one file from the assembled tree. It must fail on the concrete hash value. Setup and compile failures are
inadmissible as RED.

Required controls, each with a complete expected record:

1. **Happy path from a synthetic mirror.**
   - Tiny synthetic tarballs whose `package/` trees and SRIs are computed in-test, with the pins injected through a
     test-only pins path.
   - Assert the full layout, the receipt record, and `env.sh`.
   - A re-run reports every input as `verified` and leaves the tree byte-identical.
2. **Integrity refusals, each before extraction.**
   - Wrong SRI.
   - Wrong grammar SHA-256.
   - A truncated tarball.
   - In every case the prior installed copy is untouched and staging is removed.
3. **Tree-hash refusals.**
   - An extra file in the tarball, a modified file, and a wrong `package.json` version each refuse install.
   - For the react18 bootstrap path, a wrong tree refuses, and no SRI is recorded.
4. **Archive-entry safety.** Tarballs containing each of the following refuse before extraction, and no file
   appears outside staging:
   - an absolute entry
   - a `..` entry
   - a symlink
   - a hardlink
   - an entry outside `package/`
5. **Volatile root.** Roots under `os.tmpdir()`, under `/private/tmp`, and reached through the `/tmp` symlink all
   refuse. Tests build their durable test roots under a repo-local `target/gate-inputs-test/` (gitignored `target/`).
6. **Drift guards.**
   - `pins.json` grammar entries equal the pins parsed from `scripts/verify-typescript-grammar.mjs`.
   - The profile tree hashes and versions equal those parsed from `verify-callable-authority.mjs`.
   - The TypeScript SHA equals `schema.mjs`'s `COMPILER_HASH`.
   - `node-population.json` lists exactly the set of `*.test.mjs` files on disk.
   - A new, undeclared test file fails this guard, and so does a declared file that is missing.
7. **Runner.**
   - It refuses a missing or tampered receipt input without downloading.
   - It builds its command from `node-population.json` and exports exactly the four env vars.
   - Test via a stubbed `node --test` population of two synthetic modules and a dry-run flag that prints the
     argv/env plan.
   - Exit status is nonzero on a child failure.

## 6. Acceptance (controller, after implementation review approval)

1. Live acquisition from the public sources into the durable default root:
   - all 8 inputs authenticated
   - react18 tarball SRIs recorded via the tree-hash bootstrap
   - `inputs-receipt.json` hashed
2. A second, cold acquisition into a fresh durable root produces an identical `inputs-receipt.json`, apart from the
   root path.
3. The full gate via `run-node-gate.mjs`:
   - every active module runs
   - expected result: 0 fail
   - skips limited to host-pinned grammar cases (Node v24 host), plus any documented baseline skip
   - the observed totals and the exact module inventory are recorded in `docs/eval/gate-input-durability/receipt.md`
4. The default Rust suite is unchanged: 4,559 / 0 / 1. That result is recorded; it is not re-baselined.

## 7. Review and stop rules

Two review rounds each for the spec and for the implementation. Reviewers tag every finding WRONG or SMELL, with a
concrete input and result. At a cap, the controller classifies convergence before acting.

STOP conditions:

- any `src/`, Cargo, or existing-test change
- any `npm install` or lifecycle execution
- extraction before authentication
- any new pin without a cited repository authority, other than the §3.3 bootstrap
- a budget breach
