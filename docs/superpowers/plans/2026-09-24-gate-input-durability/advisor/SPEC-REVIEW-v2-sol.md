# Independent SPEC review — round 2 of 2

Reviewed branch `feat/gate-input-durability` at `1b5f7c486dd423d97d606ac4b9e86215f8a46843`; worktree was clean. This was read-only. No implementation or test suite exists to run.

## Round-1 closure verdicts

| Finding | Verdict | Evidence |
|---|---|---|
| W1 — react18 extraction before authentication | **NOT CLOSED** | [§3.3](/Users/wesleyjinks/code/prism-gate-inputs/docs/superpowers/plans/2026-09-24-gate-input-durability/SPEC.md:107) bounds hostile extraction and prevents installation before the tree hash, which addresses filesystem safety. But the resulting observed SRI is promoted to byte “authority” even though the tree hash authenticates extracted files, not the archive encoding. WRONG 1 below gives a constructible failure. |
| W2 — unbounded/non-portable tar listing | **NOT CLOSED** | [§3.5](/Users/wesleyjinks/code/prism-gate-inputs/docs/superpowers/plans/2026-09-24-gate-input-durability/SPEC.md:140) removes system-tar dependence and adds useful limits, but the pax contract contradicts node-tar’s metadata layout and omits framing/precedence rules needed to avoid parser differentials. See WRONG 2. |
| W3 — non-atomic directory replacement | **NOT CLOSED** | Immutable generations plus one atomic `current.json` rename solve replacement atomicity, but not crash durability: the generation-parent directory is not explicitly synced before publishing the pointer, and `env.sh` is a second unsynchronized pointer. See WRONG 3. |
| W4 — unsealed runner | **NOT CLOSED** | [§4.2](/Users/wesleyjinks/code/prism-gate-inputs/docs/superpowers/plans/2026-09-24-gate-input-durability/SPEC.md:254) clears Prism and Node overrides, but drops required non-default Rust/tool-manager locators while still inheriting ambient configuration through `HOME`. Its own `CARGO_HOME` test conflicts with the allowlist. See WRONG 5. |
| W5 — population scope | **NOT CLOSED** | Repository scope is corrected, and the current command returns the 48 expected modules. However, `--exclude-standard` is affected by user-global excludes, and non-NUL output is not a canonical filename protocol. See WRONG 6. |
| W6 — fresh-machine Cargo | **NOT CLOSED** | Declaring an offline cache prerequisite is directionally correct, but a host with a populated custom `CARGO_HOME`/`RUSTUP_HOME` can meet [§4.4](/Users/wesleyjinks/code/prism-gate-inputs/docs/superpowers/plans/2026-09-24-gate-input-durability/SPEC.md:306) and still fail because the runner clears those locators. |
| W7 — count inconsistency | **CLOSED** | [§2](/Users/wesleyjinks/code/prism-gate-inputs/docs/superpowers/plans/2026-09-24-gate-input-durability/SPEC.md:42) explicitly enumerates 9 downloaded artifacts and 4 logical inputs; acceptance is by name rather than a bare count. |
| W8 — TypeScript `lib/` not authenticated | **CLOSED, narrowly** | The retained SRI-authenticated archive and whole-`package/` comparison cover every `lib/*` file, including non-entrypoints. The canonical-manifest encoding has a separate new defect, WRONG 8. |
| S1 — UTF-8-lossy tree formula | **CLOSED** | [§3.4](/Users/wesleyjinks/code/prism-gate-inputs/docs/superpowers/plans/2026-09-24-gate-input-durability/SPEC.md:122) requires exact UTF-8 round-trip and records raw-byte hashes in addition to preserving the historical formula. |
| S2 — redirect/TOCTOU/durability | **NOT CLOSED** | Manual redirects and staged copying improve the design, but `lstat` followed by pathname-based copy remains a symlink race, and the generation durability barriers remain incomplete. See WRONG 3 and WRONG 7. |
| S3 — insufficient controls | **NOT CLOSED** | Refusal coverage is much broader, but there is no positive ustar-prefix/pax test, no real-artifact compatibility fixture, no crash point after generation rename or pointer rename, and no verify/prune race test. See SMELL 1. |
| S4 — budget | **NOT CLOSED** | Splitting A/B helps, but Slice A still combines a security-sensitive streaming tar implementation, acquisition, durable transactions, manifests, and extensive adversarial tests inside 700/700 lines. See SMELL 3. |

## Current WRONG findings

### WRONG 1 — The react18 bootstrap does not establish archive-byte authority

Concrete scenario: an offline mirror supplies repacked react18 archives with exactly the pinned extracted tree but different gzip metadata, entry order, or harmless pax metadata. The tree hash passes, so [§3.3](/Users/wesleyjinks/code/prism-gate-inputs/docs/superpowers/plans/2026-09-24-gate-input-durability/SPEC.md:111) proposes those new SRIs as `"tree-hash-bootstrap"` authority. A later download of the genuine registry tarballs has the same authenticated tree but different bytes and is rejected. The tool has converted an arbitrary representation into an authority and lost re-acquirability.

The historical custody record says these packages were already checked against exact-version registry metadata, not merely observed after extraction: [callable-authority readout](/Users/wesleyjinks/code/prism-gate-inputs/docs/eval/receiver-closure/2026-09-06-callable-authority-readout.md:34).

Bounded spec fix: obtain and commit the three exact `dist.integrity` values from the cited npm registry metadata before implementation, with a captured metadata hash/source citation. Verify SRI before parsing and retain the react18 tree hash as an independent extracted-content check. If bootstrap remains, prohibit `--from` and require the proposed SRI to equal separately fetched registry metadata.

### WRONG 2 — The ustar/pax contract rejects a valid node-tar pax archive and leaves parser precedence undefined

Concrete scenario: node-tar packs `package/<name-over-100-bytes>`. It emits an `x` header whose own tar pathname is `PaxHeader/<basename>`, followed by the real entry. Section 3.5 says every path must be under `package/`, so a literal implementation rejects the pax metadata header before it can apply `path`. Exempting it informally leaves unspecified which validations apply to metadata headers.

The contract also does not define:

- ustar `prefix + name` reconstruction;
- exact pax length-record parsing and UTF-8 validation;
- whether duplicate `path` or `size` records refuse;
- one-entry lifetime and reset of an `x` header;
- whether unknown pax keys refuse or are ignored;
- whether metadata headers count toward entry/expanded limits;
- header-size versus pax-size precedence;
- truncated header/body/padding handling;
- required end-of-archive blocks and rejection of nonzero trailing records;
- synthesis of parent directories absent from the tar.

These are security-relevant distinctions: node-tar has had a concrete pax-size interpretation differential capable of file smuggling. See the official [pax implementation](https://github.com/isaacs/node-tar/blob/main/src/pax.ts) and [pax-size advisory](https://github.com/isaacs/node-tar/security/advisories/GHSA-vmf3-w455-68vh).

Bounded spec fix: define the complete accepted tar profile, including metadata-header path exemption, prefix handling, canonical numeric encodings, single-use pax state, duplicate/unknown-key policy, exact framing/EOF rules, and synthesized parent directories. Add positive fixtures for ustar prefix and node-tar pax `path`/`size`.

Artifact impact:

- Only the six npm artifacts use this reader; the grammar archives remain hash-only.
- The locally cached, SRI-matching TypeScript 5.9.3 tarball has 132 regular-file entries, no directory/pax/GNU entries, and a maximum 51-byte pathname.
- The published React 18/19 inventories are shallow—root files plus `ts5.0/*`; the longest evident archive path is far below 100 bytes. The [React 19.0.10 inventory](https://app.unpkg.com/%40types/react%4019.0.10) and adjacent [React 18 inventory](https://app.unpkg.com/%40types/react%4018.3.26) show that shape. Therefore GNU `L`/`K` refusal does not appear to break any of the six named artifacts, and `@types/react@18.3.31` does not need pax `path`. The exact 18.3.31 tarball was not available for byte-level inspection in this environment, so acceptance should record a header census of all six rather than leave this inferred.

### WRONG 3 — The generation transaction is atomic but not crash-durable or single-pointer

Concrete scenario A: a generation directory is renamed into `<root>/<input>/gen-*`, then `current.json` is published and `<root>` synced. The spec never requires `fsync(<root>/<input>)` after the generation rename. Following a crash, the durable `current.json` can name a generation whose directory entry was lost.

Concrete scenario B: `current.json` flips to generation B, then the process dies before updating `env.sh`. `env.sh` still names generation A. A later `--prune` removes A, leaving the advertised environment file pointing at missing paths.

Bounded spec fix:

1. Fsync every generation file and directory bottom-up.
2. Rename staging to `gen-*`.
3. Fsync that input’s parent directory.
4. Only then publish and fsync `current.json`.
5. Make `current.json` the sole active pointer. Generate shell exports on demand from one validated snapshot, or publish `current.json` and `env.sh` inside a single atomically selected snapshot directory.

Add crash tests after generation rename, after generation-parent fsync, after pointer rename, and before/after any environment publication.

### WRONG 4 — The lock state machine makes specified recovery impossible and does not protect readers

Concrete scenario A: the required crash test kills the acquirer after staging. It leaves `.lock`. The next run is supposed to clean staging, but [§3.1](/Users/wesleyjinks/code/prism-gate-inputs/docs/superpowers/plans/2026-09-24-gate-input-durability/SPEC.md:72) says stale locks are only reported and never reclaimed. The “next run” cannot enter cleanup.

Concrete scenario B: Slice B completes `--verify`, releases or never takes the acquisition lock, then runs tests using generation A. A concurrent acquirer selects B and executes `--prune`; A is now non-current and can be deleted under the running gate. Tests observe missing TypeScript/profile files after a successful preflight.

Bounded spec fix: define one lock protocol covering acquire, verify, prune, and the runner’s complete verify/build/test interval. A coarse exclusive lock is sufficient. Add an explicit stale-lock recovery operation with recorded PID/host/nonce and a deliberate operator action; the normal path must not silently break it. Test killed-owner recovery and verify/run versus prune.

### WRONG 5 — The environment contract is internally contradictory and rejects valid Rust installations

Concrete scenario: a Linux host has a populated cache at `CARGO_HOME=/cache/cargo`, a toolchain at `RUSTUP_HOME=/cache/rustup`, and `/cache/cargo/bin` on `PATH`. It satisfies §4.4. The runner retains the PATH entry but clears both locator variables; the cargo/rustup proxy falls back to `$HOME/.cargo` and `$HOME/.rustup`, so offline build fails or reports an empty cache.

The [§4.3 test](/Users/wesleyjinks/code/prism-gate-inputs/docs/superpowers/plans/2026-09-24-gate-input-durability/SPEC.md:286) also says to simulate an empty cache by setting `CARGO_HOME`, while §4.2 says every unlisted variable is cleared. Both cannot be true.

`USER` and `SHELL` are not intrinsically required by the current Git/Node invocations and should not be inherited gratuitously. `CARGO_HOME`, `RUSTUP_HOME`, and custom mise/asdf data directories are different: they can identify the actual toolchain selected by the PATH shims. Conversely, retaining `HOME` means ambient `~/.cargo/config.toml` and `~/.gitconfig` can still change behavior, so “sealed” is overstated.

Bounded spec fix: define supported tool discovery explicitly. Resolve and record absolute executables before sealing; preserve validated absolute `CARGO_HOME`/`RUSTUP_HOME` where required, or declare default-home-only installations as a prerequisite. Either resolve through mise/asdf before sealing or admit and validate their required locator variables. Isolate Git global/system config and state whether Cargo home configuration is accepted authority or excluded.

### WRONG 6 — The population command is host-dependent and not filename-safe

Concrete scenario A: a user’s global Git excludes contain `*.test.mjs`. An untracked `new.test.mjs` is hidden by `--exclude-standard` on that host, so the drift guard passes; on another host without that global exclude, it fails. Git documents that `--exclude-standard` includes user-global and `.git/info/exclude` state, not only repository `.gitignore` files. [Git `ls-files` documentation](https://git-scm.com/docs/git-ls-files).

Concrete scenario B: an untracked valid filename contains a newline or quoting-sensitive byte. Without `-z`, Git emits its quoted display form; that string is placed in the declared population or passed to Node as a nonexistent pathname.

The command does correctly include ordinary untracked, non-ignored files; that conservative policy is reasonable because a new test cannot be silently omitted. The defect is that “ignored” and output encoding are not repository-canonical.

Bounded spec fix: use NUL-delimited output and define config-neutral exclusion semantics, for example `git -c core.quotepath=false -c core.excludesFile=/dev/null ls-files -z --cached --others --exclude-per-directory=.gitignore -- '*.test.mjs'`. Explicitly refuse unmerged index entries and invalid UTF-8 paths. Preserve the current rule that an ordinary untracked test causes population drift.

### WRONG 7 — `lstat` followed by pathname copy does not close the offline-mirror symlink race

Concrete scenario: `lstat(mirror/react.tgz)` sees a regular file. Before `copyFile` opens it, another process replaces it with a symlink to a different archive. The tool consumes a symlink despite the specification promising refusal. On the react18 bootstrap path, a repacked archive with the same tree can then be accepted and proposed as authority.

Bounded spec fix: open the mirror entry once with no-follow semantics, `fstat` that descriptor as a regular file, and stream the staged copy from that descriptor. If mutable/untrusted directory components are in scope, use an opened mirror-directory descriptor plus relative no-follow opens. Add a swap-to-symlink-at-open test, not only mutation after staging.

### WRONG 8 — The “canonical raw-byte manifest” is not canonical

Section 3.4 says every file or directory row has a size and raw-byte SHA-256, but directories have no portable raw-byte content and their filesystem `stat.size` varies. It also does not specify the serialized bytes being hashed, field order, delimiters, numeric representation, directory synthesis, or whether sorting is UTF-8 byte order versus JavaScript UTF-16 order.

Concrete scenario: the same authenticated package is installed on macOS and Linux. One implementation hashes filesystem directory sizes; another uses zero or omits directory hashes. Both comply with the prose but produce different generation IDs. The exact cached TypeScript tarball contains no directory headers, so a literal archive-entry manifest also differs from the physical installed tree, which necessarily contains `package/lib/` and locale directories.

Bounded spec fix: define a versioned byte format, for example canonical JSON lines encoded as UTF-8 and sorted by unsigned UTF-8 bytes. File rows should contain `{path,type:"file",size,sha256}`; directory rows should contain `{path,type:"dir"}` only. Define parent-directory synthesis and whether the root row is included. Add a fixed manifest-byte fixture and fixed digest shared across macOS/Linux.

## Current SMELL findings

### SMELL 1 — Reader tests emphasize rejection but do not prove real-format acceptance

The reader tests have strong negative rows but no required positive cases for ustar `prefix`, pax `x path`, pax `size`, NUL regular type, files-only archives with implicit directories, or exact EOF/padding behavior. A parser can pass every listed refusal and still reject a real npm archive.

Bounded fix: add byte fixtures generated by the relevant node-tar/npm dialect plus a checked-in header census for all six pinned npm artifacts. The real-artifact test may remain offline and need not store all package contents if it stores authenticated headers or compact structural fixtures.

### SMELL 2 — Gate-receipt publication is underspecified

The CLI accepts only `--log <new-file>`, but the runner also writes a receipt and the tests require an “unwritable receipt path” refusal. No receipt path, derivation rule, atomicity rule, or success/failure publication rule is defined. An implementation could overwrite the tracked acceptance receipt on every routine gate run or invent an incompatible second path.

Bounded fix: add `--receipt <new-file>` and define new-only atomic publication for both log and receipt. State whether a failure log is retained and whether a receipt is success-only. The controller can then copy an approved receipt into `docs/eval/gate-input-durability/receipt.md`.

### SMELL 3 — Slice A’s budget is not realistic; Slice B is only marginal

Forecast after the bounded fixes above:

| Slice | Helper forecast | Test forecast | Combined | Assessment |
|---|---:|---:|---:|---|
| A | 850–1,050 | 750–950 | 1,600–2,000 | The 700 / 700 / 1,400 caps are likely to breach. The streaming tar state machine and fixture encoder alone consume a substantial fraction. |
| B | 280–380 | 300–420 | 580–800 | The 350 / 350 / 700 caps are plausible only at the low end; toolchain resolution, locking, receipt publication, and race tests make a breach credible. |

Bounded fix: split Slice A again into an archive-reader sub-slice and acquisition/transaction sub-slice, each with independent tests and review, or recalibrate after a no-production-code reader spike. For Slice B, either raise the per-bucket caps modestly before dispatch or split population/runner tests from receipt publication. Do not rely on dense one-line JavaScript to meet “honest executable line” caps.

## Requested focused conclusions

- **GNU long names:** refusing `L`/`K` does not appear to break the six named npm artifacts. TypeScript’s exact tarball uses neither GNU nor pax and tops out at 51 path bytes. `@types/react`, including its `ts5.0` subtree, has no path remotely near 100 bytes. The defect is the incomplete claimed pax profile, not evidence that these nine inputs require GNU support.
- **Grammar archives:** correctly hash-only in Slice A; none should be sent through the npm extractor.
- **`current.json`:** a single atomic mapping across all four logical inputs is the right shape, but generation-parent fsync, stale-lock recovery, reader leases, and the duplicate `env.sh` pointer must be repaired.
- **React18 bootstrap:** installed-tree safety is defensible after a correct hostile parser, but an observed archive SRI is not derived authority. Use exact-version registry metadata as the byte authority.
- **Redirect allowlist:** the named URL forms fit. npm and codeload endpoints are direct; GitHub release assets use a redirect that clients must handle, and the expected `release-assets.githubusercontent.com`/`objects.githubusercontent.com` hosts are allowlisted. GitHub documents release downloads as either direct `200` or `302` responses. [GitHub release-asset documentation](https://docs.github.com/en/rest/releases/assets). Three hops are ample. I could not obtain a fresh signed `Location` for the binary in this managed environment, so acceptance should record the actual hop list.
- **Environment:** `PATH`, `HOME`, `LANG`, and `TMPDIR` suffice only for default-home installations. `CARGO_HOME` and `RUSTUP_HOME` must be handled explicitly; mise/asdf shims should be resolved or their required locator state declared. `USER` and `SHELL` need not be inherited merely for compatibility.
- **Population:** ordinary untracked, non-ignored tests should cause drift and currently do. Use `-z` and repository-controlled ignore semantics so the answer does not depend on global Git configuration.
- **Scope:** PASS. Nothing in v2 requires changes to `src/`, Cargo manifests/lockfiles, dependencies, or existing tests. The Rust build and full-suite acceptance are executions, not source changes. All proposed implementation paths remain new scripts/data/docs.

VERDICT: FIX (8 WRONG / 3 SMELL)