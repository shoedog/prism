# S2 — Identity Key, Runtime Directory and Preamble Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship, as tested library code with no daemon and no shim yet, the three things that decide who shares a prism daemon and what a client may ask of it: the identity key, the validated runtime directory with the daemon's file paths, and the preamble with its per-client settings. The in-process server's behavior does not change.

**Architecture:** A new Unix-only module `src/mcp/shared/` holds six small files: `runtime_dir.rs` (where daemons live, and the owner/mode/symlink validation run on every use), `cache_base.rs` (one canonical spelling of the cache base directory before it exists), `identity.rs` (`DaemonIdentity`, its lossless encoding and its SHA-256 `IdentityKey`), `paths.rs` (the five file names and the 103-byte socket limit), `preamble.rs` (`HELLO` and the four replies: encode, strict decode, the full-identity comparison) and `preamble_reader.rs` (a bounded line reader that never reads past the newline). `src/mcp/settings.rs` gains `ClientSettings`, the validated per-client settings the preamble carries. Nothing here listens on a socket, takes a lock or spawns a process; S3a, S4 and S5a are the consumers.

**Tech Stack:** Rust 2021, `sha2`, `serde`/`serde_json`, `dirs` (all already dependencies), `libc` (new optional dependency of the `mcp` feature; only `geteuid` is used here), `tempfile` for tests.

**Spec:** `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` (§4.1 steps 1–4, §4.3, §4.4, §7, §9 unit list, §10, §12 row S2). Lane plan: `docs/superpowers/plans/2026-10-12-shared-prism-daemon/README.md` ("Notes for the planners of later slices", S2). Built on S1 as merged: `origin/main` `f6354e07`.

## Global Constraints

- All work is under `--features mcp`. The default build must still compile (`cargo check`).
- **Standalone behavior is unchanged.** S2 adds code nothing calls yet. The proof is structural and is checked in Task 8: the golden transcripts under `tests/fixtures/mcp/golden/` are unmodified and pass (never set `PRISM_MCP_BLESS_GOLDEN`); `src/mcp/transport_tests.rs` and `src/mcp/owner_transport_tests.rs` have a zero-line diff against the base; and no serving file (`transport.rs`, `framing.rs`, `output.rs`, `session.rs`, `lazy.rs`, `owner.rs`, `src/bin/prism-mcp.rs`) is touched. The only existing source files this slice edits are `src/mcp/mod.rs` (two lines) and `src/mcp/settings.rs` (additions; `ConnectionSettings` and `from_env` are not changed).
- Do not widen `ConnectionSettings` (lane plan, S2 note). `first_call_wait` and `warm_at_startup` live beside it in `ClientSettings`.
- Everything under `src/mcp/shared/` is `#[cfg(unix)]` through the one `mod` line in `src/mcp/mod.rs`. `ClientSettings` is platform-neutral.
- A value that arrives from another process is checked before it becomes a typed value, and no decode path can panic: no `unwrap`/`expect` outside tests, text is sliced only at a checked character boundary, and no arithmetic is done on a client-supplied number. A panic in dispatch is fatal to a daemon (spec §3.1).
- Nothing in this slice creates a file or directory except `validate_runtime_dir`, which creates exactly the runtime directory. Resolving an identity or a cache base creates nothing (spec §9 test 5: "no cache dir created anywhere").
- Spec values are copied exactly: mode `0700`; socket path at most `103` bytes; preamble line at most `4096` bytes; file suffixes `.sock`, `.start`, `.owner`, `.pid`, `.log`; `key16` = first 16 hex digits; protocol version `1`; the wire text of §4.4.
- Keep every new file under 600 lines. A module's tests are in a sibling `*_tests.rs` included with `#[path]`.
- No new clippy warning in any file this slice adds or edits (the base prints about 258 warning lines; CI does not deny warnings).
- **Commit before any experiment.** The "prove it bites" steps edit a source file and restore it with `git checkout -- <file>`; that discards uncommitted work in the file. They are ordered after the task's commit for that reason.
- Logs go to files; read the file. Do not pipe cargo output into `grep` or `tail`.

## Decisions this plan makes

Each is something the spec leaves open or states ambiguously. The reason is one line; "cost if wrong" is what it takes to reverse.

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| D1 | The module is `pub mod shared` with `pub` items, not `pub(crate)`. | S4 and S5a's integration tests (`tests/mcp/`, a separate crate) must compute a key and its paths to plant fixture sockets and lock files before a shim starts (spec §9 tests 3, 4, 10). It also avoids a blanket `allow(dead_code)` until the first consumer lands. | A visibility edit; the module is behind the `mcp` feature and is not part of the `prism::api` facade. |
| D2 | `prism-mcpd` is joined to **every** base, including `$PRISM_MCP_RUNTIME_DIR`. Spec §4.3's sentence reads either way (see Spec issues, SI-2). | The directory that is created and validated is then always one prism named, so the override can point at any existing directory the user owns (it need not itself be `0700`). | Remove one `join`; paths under an override are 11 bytes longer than under the other reading, which matters against the 103-byte limit (see the note for S4/S5a tests in the self-review). |
| D3 | An empty `$PRISM_MCP_RUNTIME_DIR` counts as unset. A relative base, from any source, is an error. | Empty-means-unset is the XDG convention `dirs` itself follows. A relative base would resolve differently in each shim's working directory, so two shims would look for one daemon in two places. | One `filter` (empty) / one check (relative). |
| D4 | Validation creates only the runtime directory itself, never its parents. | A mistyped override should fail with a message, not grow a directory tree; every default base already exists. | Add `.recursive(true)`; the created parents would then be `0700` too. |
| D5 | "Mode 0700" means the nine permission bits are exactly `rwx------`. Set-id and sticky bits are ignored. A wrong mode is refused, never repaired; that includes a directory this code created under a umask that removes owner bits. | On Linux a directory created under a set-group-id parent is `02700`; comparing those bits would refuse a directory this code just created. Repairing would bless a directory that may already hold someone else's files. | A mask constant. |
| D6 | Checks run in the order symlink, not-a-directory, owner, mode; the first failure is the one reported. | The most specific cause is named; a foreign directory is reported as foreign, not as "mode 0755". | Reorder four `if`s. |
| D7 | Validation is by path (`lstat`), not through a held directory descriptor. | `bind(2)` and `connect(2)` take paths; there is no `bindat` on Darwin. The spec asks for validation "on every use", which this is. | See Spec issues, SI-5. |
| D8 | A cache base is made canonical component by component: every component that exists is resolved through symlinks; components that do not exist yet are kept as written; `..` drops the last component. A component that is unreadable, lies below a regular file, or is a dangling symlink is an error. A relative base is resolved against the working directory, which is read only in that case. | It is the kernel's own resolution as far as the path exists, and it agrees with what `create_dir_all` then does with the rest (a test creates the directory afterwards and compares). A location that can never hold a cache is not a base two processes could share. | The function is 30 lines; replacing the policy for the non-existent tail is local. On a case-insensitive filesystem two spellings of a not-yet-existing tail that differ only in case give two keys, so two daemons (extra memory, same answers). |
| D9 | `default_cache_base()` repeats the one-line default from `src/navigation/cache.rs` and a test pins the two together, instead of exporting one function from `navigation::cache`. | It keeps this slice out of `src/navigation/`, which by `CLAUDE.md` would require a release build and the Tier-A harness for a change that cannot affect resolution. | Replace the body with a call once a later slice touches `navigation::cache` anyway. |
| D10 | `CacheMode::NoCache` has no identity: `DaemonIdentity::resolve` returns `IdentityError::CacheBase(CacheBaseError::NoCache)`. | Spec §4.1 step 2: shared mode requires a cache-backed configuration. The CLI error itself is S5a's; the type simply cannot express a cache-free daemon. | None; the alternative would be a key with a missing field. |
| D11 | The encoding is ten fields in a fixed order, each an 8-byte little-endian length followed by the bytes: a domain string `prism-mcpd identity`, the root's bytes, device (8 bytes LE), inode (8 bytes LE), package version, grammar fingerprint, analyzer build identity, protocol version (4 bytes LE), the cache base's bytes, the refresh policy's command-line name. | Length prefixes make the encoding injective (spec §4.3 "length-prefixed, lossless"); raw path bytes survive non-UTF-8 names; the domain string keeps the digest from colliding with a hash of something else. | Any change gives every configuration a new key, which only orphans daemons that a new build would not have reused anyway (the build identity is in the key). |
| D12 | A key's text form is exactly 64 lowercase hex digits; anything else is not a key. | One key, one spelling: comparing keys is comparing bytes, on the wire and in file names. | Loosen `from_hex`. |
| D13 | `SHARED_PROTOCOL_VERSION: u32 = 1`. The name avoids `PROTOCOL_VERSION`, which `transport.rs` uses for the MCP protocol date. | Spec §4.4 shows `"protocol":1`. | A rename. |
| D14 | `ClientSettings` is a new type with private fields and one constructor; `ConnectionSettings` keeps its public fields and `from_env` is untouched. | Lane plan S2 note: do not widen `ConnectionSettings`; decode the cap through one validating constructor. Changing `ConnectionSettings` itself would edit `transport.rs` and the S1 tests. | In-crate code can still write a `ConnectionSettings` literal with a sub-floor cap. The daemon's only source of settings is `ClientSettings::output()`; S3a's review should hold that line. |
| D15 | A result cap below the floor is **refused** (`SettingsError::ResultCapBelowFloor`), not replaced by the default. The lane plan's wording is "applies the `resolve_cap_from` rule"; that rule has two halves (accept only caps ≥ 12,000; otherwise warn on stderr and use 80,000). This plan applies the first half and not the second (see Spec issues, SI-6). | The fallback exists so an operator's typo does not stop a server from starting, and it tells them on stderr. A client of a daemon cannot see the daemon's stderr, so the same fallback would silently return 80,000-byte results to a client that asked for 100. A correct shim never sends a sub-floor cap, because `ConnectionSettings::from_env` already applied the rule. | One `match` arm: substitute `MAX_RESULT_CHARS` instead of returning the error. |
| D16 | `first_call_wait_secs` above 600 is refused; the bound is `FIRST_CALL_WAIT_MAX`, the same as the command line. | A daemon adds the wait to an `Instant`; an unbounded value overflows, and an overflow there is a panic. | A constant. |
| D17 | An unknown word for `structured_content` or `concise_shape` is refused. The words are the environment variables' (`always`, `omit-default-path`; `legacy`, `slim`); spec §4.4 elides the second pair. | Same reasoning as D15: no silent fallback across a process boundary. | Two `match` arms. |
| D18 | A message has exactly one accepted spelling: the bytes its own `encode` writes. Decoding parses strictly (object only, unknown and repeated fields refused), then re-encodes and compares. | Both ends are the same build (the build identity is in the key), so there is no second producer to be lenient towards; and "what a client may ask" becomes exactly the image of the encoder, which is small enough to review. | Delete the comparison (three lines per direction) to accept any JSON spelling. A hand-written client must match the encoder byte for byte; tests in later slices should build lines with `Hello::encode`. |
| D19 | The `protocol` field is read and checked before the rest of the payload is interpreted, and reported as `PreambleError::UnsupportedProtocol`. | Another version's payload may have another shape; reporting "missing field" for it would hide the real cause. | Reorder two calls. |
| D20 | `READY`'s `start_time` is whole seconds since the Unix epoch; `pid` and `max_clients` are `u32`; `generation` and `start_time` are `u64`. | The spec gives `N`. Seconds since the epoch is what a person reading `daemon-status` expects, and pid plus second identifies an instance. | The field is a `u64` either way; S4 writes it. |
| D21 | The 4096-byte limit counts the line without its newline. | Same convention as S1's `MAX_REQUEST_BYTES`. | Off by one byte. |
| D22 | S2 ships the bounded line reader (`PreambleLineReader`), generic over `Read`. It reads one byte at a time, keeps a partial line across `WouldBlock`/`TimedOut`, and refuses for good after an overflow. | A size limit checked after an unbounded read bounds nothing; and a buffered reader would swallow the MCP bytes that follow the preamble on the same socket. Timeout and non-blocking policy stay with S4 and S5a: the reader passes those errors through. | S4 replaces a 40-line type. |
| D23 | S2 ships `Hello::mismatch_against(&served)`, the full-identity comparison, as a pure function. | Spec §4.4 calls this check "what makes the 16-hex filename a locator rather than proof of equality"; it belongs with a test that uses two keys sharing their first 16 digits. | S4 inlines it. |
| D24 | The peer-uid check (`getpeereid`, spec §4.4) is **not** in S2. `current_uid()` is. | It is a check on an accepted connection, which is S4's handshake stage, and the Linux and Darwin calls differ (`SO_PEERCRED` vs `getpeereid`); a same-uid test is the only one that can run without root. | S4 adds ~20 lines. If the controller wants the whole "who may connect" boundary in one slice, it is a small addition here. |
| D25 | S2 names the lock, pid and log files; it does not create or open them (mode 0600, `O_NOFOLLOW`, close-on-exec are S4's). | Spec §12 gives locks to S4. | None. |
| D26 | Encoders build the line with `format!`; only decoders use `serde`. | Every interpolated value is a number, a boolean, 64 hex digits or a fixed word, so there is nothing to escape and no failure path to `unwrap`. The line in the code reads like the line in the spec. | Switch to `serde_json::to_string` on the wire structs. |

## Spec issues found (not resolved by this plan)

- **SI-1 (§4.3, functional on multi-user Linux hosts).** With no override and no `$XDG_RUNTIME_DIR`, the runtime directory is `<temp dir>/prism-mcpd`. Where the temp directory is shared (`/tmp`), that is one name for every user. The first user to create it owns it; every other user's validation then fails on ownership and their shims fail closed on every start. macOS is unaffected (`$TMPDIR` is per user). This plan implements the text as written; the refusal names `PRISM_MCP_RUNTIME_DIR` as the way out. A per-user name in the temp-dir fallback (`prism-mcpd-<uid>`) would remove it; that is a spec change.
- **SI-2 (§4.3, ambiguous).** "`$PRISM_MCP_RUNTIME_DIR` if set …, else `dirs::runtime_dir()` …, else `std::env::temp_dir()` …, joined with `prism-mcpd/`" does not say whether the join applies to the override. Decided as D2.
- **SI-3 (§4.4, §10, gap).** The spec defines four replies and no outcome for a `HELLO` the daemon cannot accept for a reason other than identity: another protocol version, a malformed line, or settings out of range. S2 makes each expressible (`PreambleError`); which reply, if any, the daemon sends is S4's decision. One fact for that decision: for another protocol version, `MISMATCH {"expected": <served key>}` is truthful, because the version is part of the key.
- **SI-4 (§4.3, observation).** The key lists `CARGO_PKG_VERSION`, `GRAMMAR_FINGERPRINT` and `PRISM_CACHE_BUILD_IDENTITY`. The third is a content hash of `src/`, `build.rs`, `Cargo.toml`, `Cargo.lock` and the vendored grammar (`build.rs`), so it already determines the first two. None of the three covers the build profile or the compiler: a debug and a release binary of one source tree share a key and therefore a daemon (same answers, different speed). Implemented as written.
- **SI-5 (§7, limit of the stated check).** "Validated on every use" can only be a path check, because the socket calls take paths. Between validation and `bind`, a process that can rename the runtime directory could substitute another. That needs write access to its parent, which other users do not have for `$TMPDIR` on macOS or `$XDG_RUNTIME_DIR` on Linux, and which the sticky bit denies them in `/tmp`. Recorded, not pursued.
- **SI-6 (lane plan, two readings).** "Decode it through one validating constructor that applies the `resolve_cap_from` rule" can mean refuse a sub-floor cap or replace it with the default. Decided as D15 (refuse).
- **SI-7 (§3.1, §4.2, §8 step 4, open for S4/S5a).** `--warm-at-startup` is "forwarded in the preamble; honored only by the daemon this shim starts", and the daemon's command line in §4.2 has no such flag. A daemon cannot tell from a `HELLO` whether its sender is the shim that started it: under a start race another shim's `HELLO` can arrive first. Either the daemon's command line needs the flag (and the preamble field is then informational), or "the first admitted `HELLO` decides" is the rule. S2 carries the field exactly as §4.4 shows it and decides neither.

## Review Focus

Conditions the spec implies that a person will hit, each pinned by a test in the task named. The appendix lists every wrong implementation that was applied to the code during plan validation, with the tests that failed for it.

1. **Another user pre-creates the runtime directory, or replaces it with a symlink to one of their own.** Refused on ownership, or as a symlink, with or without a trailing slash in the spelling. → Task 1, `a_private_directory_owned_by_a_different_uid_is_refused`, `a_directory_that_really_belongs_to_root_is_refused`, `a_symlink_to_a_private_directory_is_refused_with_or_without_a_trailing_slash`.
2. **The runtime directory exists with group or other access.** Refused and left as it was, for every mode but 0700. → Task 1, `any_mode_other_than_0700_is_refused_and_left_as_it_was`.
3. **The directory is swapped between `mkdir` and the first use, or `lstat` itself fails.** What is found after creating is still checked; an inspection error is never read as "absent". → Task 1, `what_is_found_after_a_successful_creation_is_still_checked`, `an_inspection_error_is_reported_and_nothing_is_created`.
4. **One checkout and one cache directory reached through different spellings** (trailing slash, `.`, `..`, a symlink, `CacheMode::Default` against the same directory given explicitly, before the directory exists). One key. → Task 2, `default_and_its_explicit_equivalents_are_one_base_before_the_directory_exists`, `the_resolved_base_is_where_create_dir_all_then_puts_it`; Task 3, `one_checkout_and_one_cache_base_have_one_key_however_they_are_spelled`.
5. **Two checkouts, cache bases, builds, protocol versions or refresh policies.** Two keys; and a root replaced at the same pathname is a new key. → Task 3, `each_configuration_difference_gives_a_different_key`, `every_input_changes_the_key`, `a_root_replaced_at_the_same_pathname_has_a_new_key`.
6. **A daemon started from the shim's resolved values must compute the shim's key**, or every connection is answered `MISMATCH`. → Task 3, `a_daemon_started_from_an_identity_s_own_values_computes_the_same_key`; Task 2, `resolving_a_resolved_base_changes_nothing`.
7. **A shim whose key shares its first 16 digits with another daemon's finds that daemon's socket.** The full keys are compared and the reply names the served key. → Task 6, `a_hello_for_a_neighbouring_identity_is_a_mismatch_that_names_the_served_key`; Task 4, `keys_that_share_their_first_16_digits_share_their_files`.
8. **A client asks for a result cap below the floor or an unbounded wait.** Refused when the settings are constructed, so neither reaches dispatch or an `Instant`. → Task 5, `a_result_cap_below_the_floor_is_refused`, `a_first_call_wait_above_the_command_line_maximum_is_refused`, `the_smallest_accepted_cap_serves_the_query_that_reserves_the_most`; Task 6, `a_hello_asking_for_settings_a_server_must_not_run_with_is_refused`.
9. **Something that is not a shim connects** (an MCP client, a peer that sends megabytes without a newline, a line of another protocol version). Each is refused for its own reason, within 4 KiB of memory, without a panic. → Task 6, the `a_hello_…`/`a_payload_…`/`a_verb_…` tests; Task 7, `an_endless_line_is_given_up_on_after_one_byte_past_the_limit`.
10. **The first MCP frame is already in the socket buffer when the preamble is read.** It is still there for the transport. → Task 7, `the_mcp_frame_that_follows_the_preamble_on_a_socket_is_intact`.
11. **The runtime directory's path is too long for a socket address.** Refused at 104 bytes, counted in bytes; 103 is accepted and can really be bound. → Task 4, `a_socket_path_of_103_bytes_fits_and_104_does_not`, `the_limit_counts_bytes_not_characters`, `a_socket_can_really_be_bound_at_the_limit`.

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `Cargo.toml`, `Cargo.lock` | `libc` as an optional dependency of the `mcp` feature | Modify (Task 1), only if not already present |
| `src/mcp/mod.rs` | Module list | Modify (Task 1): `#[cfg(unix)] pub mod shared;` |
| `src/mcp/shared/mod.rs` | Module list of shared mode; `SHARED_PROTOCOL_VERSION` | Create (Task 1), extend (Tasks 2, 3, 4, 6, 7) |
| `src/mcp/shared/runtime_dir.rs` | `runtime_dir`, `validate_runtime_dir`, `current_uid`, `RuntimeDirError` | Create (Task 1) |
| `src/mcp/shared/runtime_dir_tests.rs` | Its tests | Create (Task 1) |
| `src/mcp/shared/cache_base.rs` | `effective_cache_base`, `default_cache_base`, `CacheBaseError` | Create (Task 2) |
| `src/mcp/shared/cache_base_tests.rs` | Its tests | Create (Task 2) |
| `src/mcp/shared/identity.rs` | `DaemonIdentity`, `BuildIdentity`, `FsIdentity`, `IdentityKey`, `IdentityError` | Create (Task 3) |
| `src/mcp/shared/identity_tests.rs` | Its tests | Create (Task 3) |
| `src/mcp/shared/paths.rs` | `DaemonPaths`, `MAX_SOCKET_PATH_BYTES`, `SocketPathTooLong` | Create (Task 4) |
| `src/mcp/shared/paths_tests.rs` | Its tests | Create (Task 4) |
| `src/mcp/settings.rs` | adds `ClientSettings`, `SettingsError` beside the unchanged `ConnectionSettings` | Modify (Task 5) |
| `src/mcp/settings_tests.rs` | Tests for `ClientSettings` | Create (Task 5) |
| `src/mcp/shared/preamble.rs` | `Hello`, `Reply`, `Ready`, `BuildState`, `PreambleError`, `PREAMBLE_MAX_BYTES` | Create (Task 6) |
| `src/mcp/shared/preamble_tests.rs` | Tests for `HELLO` and the identity comparison | Create (Task 6) |
| `src/mcp/shared/preamble_reply_tests.rs` | Tests for the replies | Create (Task 6) |
| `src/mcp/shared/preamble_reader.rs` | `PreambleLineReader`, `PreambleReadError` | Create (Task 7) |
| `src/mcp/shared/preamble_reader_tests.rs` | Its tests | Create (Task 7) |
| `CLAUDE.md` | MCP Adapter notes | Modify (Task 8) |
| `src/mcp/transport_tests.rs`, `src/mcp/owner_transport_tests.rs`, `tests/fixtures/mcp/golden/*` | The standalone contract | **Must not change** |

How a step names a file edit, so there is no doubt what to type:

- "Create `path`:" is followed by the file's complete content.
- "Replace the whole of `path` with:" is followed by the file's complete content.
- "In `path`, replace this:" is followed by text that occurs exactly once in the file, then "with this:" and its replacement.
- "Append to `path`:" is followed by text to add at the end of the file.

Every test step that expects a failure expects a **compile** failure: the test file is written first against a module that has no items yet. That is this codebase's red step for a new module (S1 Task 3 did the same).

---

### Task 1: Runtime directory, and the `libc` dependency

Where every daemon's files live, and the check that keeps other users out of it.

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock` (only if `libc` is not already a dependency)
- Modify: `src/mcp/mod.rs`
- Create: `src/mcp/shared/mod.rs`, `src/mcp/shared/runtime_dir.rs`, `src/mcp/shared/runtime_dir_tests.rs`

**Interfaces:**
- Consumes: `dirs::runtime_dir()`, `std::env::temp_dir()`, `libc::geteuid`.
- Produces (all in `crate::mcp::shared::runtime_dir`):
  - `pub const RUNTIME_DIR_ENV: &str = "PRISM_MCP_RUNTIME_DIR";`
  - `pub const RUNTIME_DIR_NAME: &str = "prism-mcpd";`
  - `pub enum RuntimeDirError { NotAbsolute { path: PathBuf }, Io { op: &'static str, path: PathBuf, source: io::Error }, Symlink { path: PathBuf }, NotADirectory { path: PathBuf }, NotOwned { path: PathBuf, owner: u32, current: u32 }, InsecureMode { path: PathBuf, mode: u32 } }` (`Debug`, `Display` on one line naming the path and `PRISM_MCP_RUNTIME_DIR`, `Error`)
  - `pub fn runtime_dir() -> Result<PathBuf, RuntimeDirError>`
  - `pub fn validate_runtime_dir(dir: &Path) -> Result<(), RuntimeDirError>`
  - `pub fn current_uid() -> u32`
  - Crate-internal seams for tests: `runtime_dir_from(Option<OsString>, Option<PathBuf>, PathBuf)`, `DirFacts`, `trait DirAccess`, `RealDirAccess`, `validate_runtime_dir_with(&impl DirAccess, &Path, u32)`.

- [ ] **Step 1: Record the base test totals**

```bash
cargo test --features mcp --lib mcp:: > /tmp/s2-base-lib.log 2>&1; grep -E '^test result' /tmp/s2-base-lib.log
cargo test --features mcp --test mcp > /tmp/s2-base-mcp.log 2>&1; grep -E '^test result' /tmp/s2-base-mcp.log
```

Expected on `origin/main` at `f6354e07`: `216 passed; 0 failed` and `13 passed; 0 failed`. If the sibling slice C1 (PR #362) has merged, the numbers are higher; write down the two you got. Every later expectation in this plan is "base + N".

- [ ] **Step 2: Add `libc`, if it is not already there**

Run: `grep -n 'libc' Cargo.toml`

If it prints both `mcp = ["dep:libc"]` and `libc = { version = "0.2", optional = true }`, PR #362 merged first: skip the rest of this step. Otherwise make exactly these two edits and no other (they are the same two lines PR #362 adds, so whichever merges second rebases cleanly).

In `Cargo.toml`, replace this:

```toml
mcp = []
```

with this:

```toml
mcp = ["dep:libc"]
```

In `Cargo.toml`, replace this:

```toml
dirs = "6.0.0"
```

with this:

```toml
dirs = "6.0.0"
libc = { version = "0.2", optional = true }
```

`Cargo.lock` already pins `libc 0.2.183` as a transitive dependency; the next cargo command adds one line (`"libc",`) to the `prism` package's dependency list. Commit that change with this task.

- [ ] **Step 3: Declare the module**

In `src/mcp/mod.rs`, replace this:

```rust
pub mod settings;
```

with this:

```rust
pub mod settings;
#[cfg(unix)]
pub mod shared;
```

Create `src/mcp/shared/mod.rs`:

````rust
//! Building blocks of shared mode, where one daemon per repository checkout is reached through
//! a stdio shim: where a daemon's files live, which configuration it serves, and the private
//! preamble the shim and the daemon exchange before any MCP byte. Nothing here listens on a
//! socket, takes a lock or starts a process; the daemon and the shim are the consumers.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §4.3, §4.4, §7.

pub mod runtime_dir;
````

- [ ] **Step 4: Write the tests**

Create `src/mcp/shared/runtime_dir_tests.rs`:

````rust
//! Tests for where the runtime directory is and for the validation run on every use. Each
//! validation test names the wrong implementation it rules out.
use super::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn mode_of(path: &Path) -> u32 {
    std::fs::symlink_metadata(path).unwrap().mode() & 0o777
}

fn private_dir(parent: &Path, name: &str) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

// ── Where the directory is ──

#[test]
fn the_override_is_the_base_and_the_directory_name_is_joined_to_it() {
    let dir = runtime_dir_from(
        Some(OsString::from("/override/base")),
        Some(PathBuf::from("/run/user/501")),
        PathBuf::from("/tmp"),
    )
    .unwrap();
    assert_eq!(dir, PathBuf::from("/override/base/prism-mcpd"));
}

#[test]
fn without_an_override_the_user_runtime_dir_is_the_base_then_the_temp_dir() {
    let with_user_dir = runtime_dir_from(
        None,
        Some(PathBuf::from("/run/user/501")),
        PathBuf::from("/tmp"),
    )
    .unwrap();
    assert_eq!(with_user_dir, PathBuf::from("/run/user/501/prism-mcpd"));

    let temp_only = runtime_dir_from(None, None, PathBuf::from("/var/folders/x/T/")).unwrap();
    assert_eq!(temp_only, PathBuf::from("/var/folders/x/T/prism-mcpd"));
}

#[test]
fn an_empty_override_counts_as_unset() {
    let dir = runtime_dir_from(Some(OsString::new()), None, PathBuf::from("/tmp")).unwrap();
    assert_eq!(dir, PathBuf::from("/tmp/prism-mcpd"));
}

/// Rules out resolving a relative base against the working directory: two shims started in
/// different directories would then look for the same daemon in two places.
#[test]
fn a_relative_base_is_refused_whichever_source_it_came_from() {
    let from_override = runtime_dir_from(
        Some(OsString::from("run/prism")),
        None,
        PathBuf::from("/tmp"),
    );
    assert!(
        matches!(&from_override, Err(RuntimeDirError::NotAbsolute { path }) if path == Path::new("run/prism")),
        "{from_override:?}"
    );
    let from_temp = runtime_dir_from(None, None, PathBuf::from("tmp"));
    assert!(
        matches!(from_temp, Err(RuntimeDirError::NotAbsolute { .. })),
        "{from_temp:?}"
    );
}

#[test]
fn the_environment_reader_agrees_with_the_pure_resolver() {
    let expected = runtime_dir_from(
        std::env::var_os(RUNTIME_DIR_ENV),
        dirs::runtime_dir(),
        std::env::temp_dir(),
    )
    .unwrap();
    assert_eq!(runtime_dir().unwrap(), expected);
    assert!(expected.ends_with(RUNTIME_DIR_NAME));
}

// ── Validation against the real filesystem ──

/// Rules out `create_dir`/`create_dir_all` with the default mode: under the usual umask that
/// makes a 0755 directory, which this same validation must then refuse.
#[test]
fn an_absent_directory_is_created_0700_and_passes() {
    let base = tempfile::tempdir().unwrap();
    let dir = base.path().join("prism-mcpd");
    validate_runtime_dir(&dir).expect("created and valid");
    assert!(std::fs::symlink_metadata(&dir).unwrap().is_dir());
    assert_eq!(mode_of(&dir), 0o700);
    validate_runtime_dir(&dir).expect("valid on the next use too");
}

/// Rules out `create_dir_all`: a mistyped override must not grow a directory tree.
#[test]
fn a_missing_base_is_an_error_and_nothing_is_created() {
    let base = tempfile::tempdir().unwrap();
    let dir = base.path().join("no-such-base").join("prism-mcpd");
    let outcome = validate_runtime_dir(&dir);
    assert!(
        matches!(&outcome, Err(RuntimeDirError::Io { op: "create", .. })),
        "{outcome:?}"
    );
    assert!(!base.path().join("no-such-base").exists());
}

#[test]
fn an_existing_private_directory_passes_and_its_contents_are_untouched() {
    let base = tempfile::tempdir().unwrap();
    let dir = private_dir(base.path(), "prism-mcpd");
    std::fs::write(dir.join("0123456789abcdef.pid"), "41\n").unwrap();
    validate_runtime_dir(&dir).expect("valid");
    assert_eq!(
        std::fs::read_to_string(dir.join("0123456789abcdef.pid")).unwrap(),
        "41\n"
    );
}

/// Rules out `fs::metadata` (which follows the link and sees a perfectly good directory) and
/// rules out `lstat` on a spelling with a trailing slash (which also follows it).
#[test]
fn a_symlink_to_a_private_directory_is_refused_with_or_without_a_trailing_slash() {
    let base = tempfile::tempdir().unwrap();
    let target = private_dir(base.path(), "elsewhere");
    let link = base.path().join("prism-mcpd");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let plain = validate_runtime_dir(&link);
    assert!(
        matches!(&plain, Err(RuntimeDirError::Symlink { path }) if path == &link),
        "{plain:?}"
    );
    let mut with_slash = link.clone().into_os_string();
    with_slash.push("/");
    let slashed = validate_runtime_dir(Path::new(&with_slash));
    assert!(
        matches!(slashed, Err(RuntimeDirError::Symlink { .. })),
        "{slashed:?}"
    );
}

/// Rules out checking only "not world-writable", only the group bits, or only the other bits,
/// and rules out repairing the mode instead of refusing (a directory that was open may already
/// hold files someone else put there).
#[test]
fn any_mode_other_than_0700_is_refused_and_left_as_it_was() {
    for mode in [0o750, 0o705, 0o710, 0o701, 0o755, 0o770, 0o777, 0o500] {
        let base = tempfile::tempdir().unwrap();
        let dir = base.path().join("prism-mcpd");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(mode)).unwrap();

        let outcome = validate_runtime_dir(&dir);
        assert!(
            matches!(&outcome, Err(RuntimeDirError::InsecureMode { mode: found, .. }) if *found == mode),
            "mode {mode:o}: {outcome:?}"
        );
        assert_eq!(mode_of(&dir), mode, "mode {mode:o} must not be repaired");
    }
}

#[test]
fn a_regular_file_in_the_directory_s_place_is_refused() {
    let base = tempfile::tempdir().unwrap();
    let file = base.path().join("prism-mcpd");
    std::fs::write(&file, "").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o700)).unwrap();
    let outcome = validate_runtime_dir(&file);
    assert!(
        matches!(outcome, Err(RuntimeDirError::NotADirectory { .. })),
        "{outcome:?}"
    );
}

/// Rules out a validation with no ownership check. The directory is real and 0700; only the
/// uid it is compared against differs, so nothing but the owner comparison can refuse it.
#[test]
fn a_private_directory_owned_by_a_different_uid_is_refused() {
    let base = tempfile::tempdir().unwrap();
    let dir = private_dir(base.path(), "prism-mcpd");
    let me = current_uid();
    let someone_else = me.wrapping_add(1);
    let outcome = validate_runtime_dir_with(&RealDirAccess, &dir, someone_else);
    assert!(
        matches!(
            &outcome,
            Err(RuntimeDirError::NotOwned { owner, current, .. })
                if *owner == me && *current == someone_else
        ),
        "{outcome:?}"
    );
}

/// The same check with nothing injected: `/` belongs to root. Skipped when the tests run as
/// root, where `/` is the caller's own.
#[test]
fn a_directory_that_really_belongs_to_root_is_refused() {
    if current_uid() == 0 {
        eprintln!("skipped: running as root, so `/` is owned by the current user");
        return;
    }
    let outcome = validate_runtime_dir(Path::new("/"));
    assert!(
        matches!(&outcome, Err(RuntimeDirError::NotOwned { owner: 0, .. })),
        "{outcome:?}"
    );
}

#[test]
fn a_relative_directory_is_refused_before_the_filesystem_is_touched() {
    let fake = FakeDirs::new(vec![], Ok(()));
    let outcome = validate_runtime_dir_with(&fake, Path::new("prism-mcpd"), 501);
    assert!(
        matches!(outcome, Err(RuntimeDirError::NotAbsolute { .. })),
        "{outcome:?}"
    );
    assert!(fake.calls().is_empty());
}

#[test]
fn the_current_uid_is_the_owner_of_what_this_process_creates() {
    let base = tempfile::tempdir().unwrap();
    assert_eq!(
        std::fs::symlink_metadata(base.path()).unwrap().uid(),
        current_uid()
    );
}

// ── Validation with injected filesystem outcomes ──

type InspectOutcome = std::io::Result<Option<DirFacts>>;

struct FakeDirs {
    inspections: RefCell<VecDeque<InspectOutcome>>,
    creation: RefCell<Option<std::io::Result<()>>>,
    calls: RefCell<Vec<String>>,
}

impl FakeDirs {
    fn new(inspections: Vec<InspectOutcome>, creation: std::io::Result<()>) -> Self {
        Self {
            inspections: RefCell::new(inspections.into()),
            creation: RefCell::new(Some(creation)),
            calls: RefCell::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl DirAccess for FakeDirs {
    fn inspect(&self, _path: &Path) -> InspectOutcome {
        self.calls.borrow_mut().push("inspect".into());
        self.inspections
            .borrow_mut()
            .pop_front()
            .expect("an inspection the test did not script")
    }

    fn create_private(&self, _path: &Path) -> std::io::Result<()> {
        self.calls.borrow_mut().push("create".into());
        self.creation
            .borrow_mut()
            .take()
            .expect("a second creation the test did not script")
    }
}

const UID: u32 = 501;
const GOOD: DirFacts = DirFacts {
    is_symlink: false,
    is_dir: true,
    owner: UID,
    mode: 0o40700,
};

fn denied() -> std::io::Error {
    std::io::Error::from(std::io::ErrorKind::PermissionDenied)
}

fn check(fake: &FakeDirs) -> Result<(), RuntimeDirError> {
    validate_runtime_dir_with(fake, Path::new("/run/user/501/prism-mcpd"), UID)
}

/// Rules out treating "could not look" as "not there": that would go on to create the
/// directory, or to use one nobody inspected.
#[test]
fn an_inspection_error_is_reported_and_nothing_is_created() {
    let fake = FakeDirs::new(vec![Err(denied())], Ok(()));
    let outcome = check(&fake);
    assert!(
        matches!(
            &outcome,
            Err(RuntimeDirError::Io { op: "inspect", source, .. })
                if source.kind() == std::io::ErrorKind::PermissionDenied
        ),
        "{outcome:?}"
    );
    assert_eq!(fake.calls(), ["inspect"]);
}

#[test]
fn a_creation_error_is_reported_as_an_unwritable_runtime_directory() {
    let fake = FakeDirs::new(vec![Ok(None)], Err(denied()));
    let outcome = check(&fake);
    assert!(
        matches!(
            &outcome,
            Err(RuntimeDirError::Io { op: "create", source, .. })
                if source.kind() == std::io::ErrorKind::PermissionDenied
        ),
        "{outcome:?}"
    );
    assert_eq!(fake.calls(), ["inspect", "create"]);
}

/// Rules out trusting a directory because this process just created it: between the `mkdir`
/// and the first use, whoever can write the parent can put something else there.
#[test]
fn what_is_found_after_a_successful_creation_is_still_checked() {
    let swapped = DirFacts {
        is_symlink: true,
        is_dir: false,
        ..GOOD
    };
    let fake = FakeDirs::new(vec![Ok(None), Ok(Some(swapped))], Ok(()));
    let outcome = check(&fake);
    assert!(
        matches!(outcome, Err(RuntimeDirError::Symlink { .. })),
        "{outcome:?}"
    );
    assert_eq!(fake.calls(), ["inspect", "create", "inspect"]);
}

/// Two shims start together; the loser of the `mkdir` race must accept the winner's directory
/// if (and only if) it validates.
#[test]
fn losing_the_creation_race_to_a_valid_directory_passes() {
    let exists = std::io::Error::from(std::io::ErrorKind::AlreadyExists);
    let fake = FakeDirs::new(vec![Ok(None), Ok(Some(GOOD))], Err(exists));
    check(&fake).expect("the winner's directory is valid");

    let exists = std::io::Error::from(std::io::ErrorKind::AlreadyExists);
    let foreign = DirFacts {
        owner: UID + 1,
        ..GOOD
    };
    let fake = FakeDirs::new(vec![Ok(None), Ok(Some(foreign))], Err(exists));
    let outcome = check(&fake);
    assert!(
        matches!(outcome, Err(RuntimeDirError::NotOwned { .. })),
        "{outcome:?}"
    );
}

#[test]
fn a_directory_that_vanishes_after_creation_is_an_error() {
    let fake = FakeDirs::new(vec![Ok(None), Ok(None)], Ok(()));
    let outcome = check(&fake);
    assert!(
        matches!(
            &outcome,
            Err(RuntimeDirError::Io { op: "inspect", source, .. })
                if source.kind() == std::io::ErrorKind::NotFound
        ),
        "{outcome:?}"
    );
}

/// The file-type and set-id bits `lstat` reports beside the permission bits are not access
/// rights: a 0700 directory under a set-group-id parent is created `02700` on Linux and is
/// exactly as private.
#[test]
fn only_the_nine_permission_bits_are_compared() {
    let setgid = DirFacts {
        mode: 0o42700,
        ..GOOD
    };
    check(&FakeDirs::new(vec![Ok(Some(setgid))], Ok(()))).expect("02700 is private");

    let group_readable = DirFacts {
        mode: 0o42740,
        ..GOOD
    };
    let outcome = check(&FakeDirs::new(vec![Ok(Some(group_readable))], Ok(())));
    assert!(
        matches!(
            outcome,
            Err(RuntimeDirError::InsecureMode { mode: 0o740, .. })
        ),
        "{outcome:?}"
    );
}

#[test]
fn every_refusal_names_the_path_and_how_to_point_elsewhere() {
    let path = PathBuf::from("/tmp/prism-mcpd");
    let errors = [
        RuntimeDirError::NotAbsolute { path: path.clone() },
        RuntimeDirError::Io {
            op: "create",
            path: path.clone(),
            source: denied(),
        },
        RuntimeDirError::Symlink { path: path.clone() },
        RuntimeDirError::NotADirectory { path: path.clone() },
        RuntimeDirError::NotOwned {
            path: path.clone(),
            owner: 0,
            current: 501,
        },
        RuntimeDirError::InsecureMode {
            path: path.clone(),
            mode: 0o755,
        },
    ];
    for error in errors {
        let text = error.to_string();
        assert!(text.contains("/tmp/prism-mcpd"), "{text}");
        assert!(text.contains(RUNTIME_DIR_ENV), "{text}");
        assert!(!text.contains('\n'), "one line: {text}");
    }
    let owned = RuntimeDirError::NotOwned {
        path,
        owner: 0,
        current: 501,
    }
    .to_string();
    assert!(
        owned.contains("uid 0") && owned.contains("uid 501"),
        "{owned}"
    );
}
````

Create `src/mcp/shared/runtime_dir.rs`:

```rust
#[cfg(test)]
#[path = "runtime_dir_tests.rs"]
mod tests;
```

- [ ] **Step 5: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::shared::runtime_dir > /tmp/s2-t1-red.log 2>&1; grep -cE '^error\[' /tmp/s2-t1-red.log; tail -1 /tmp/s2-t1-red.log
```

Expected: a count above 50 and a last line beginning `error: could not compile`. The errors are `cannot find function validate_runtime_dir`, `use of undeclared type RuntimeDirError` and the like.

- [ ] **Step 6: Implement**

Replace the whole of `src/mcp/shared/runtime_dir.rs` with:

````rust
//! The per-user runtime directory that holds every daemon's socket, locks, pid file and log
//! (spec §4.3), and the validation that the shim and the daemon both run on every use (§7).
//!
//! Validation is what keeps another local user out: the directory must be a real directory
//! (not a symlink), owned by the current user, with mode 0700. It is checked by path with
//! `lstat`, not through a held descriptor, so it is a check against other users who cannot
//! write the parent directory; it is not a defense against a process of the same user (§2).
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};

/// Environment variable that replaces the base directory. Every launcher that should share a
/// daemon must resolve the same value (spec §4.3).
pub const RUNTIME_DIR_ENV: &str = "PRISM_MCP_RUNTIME_DIR";
/// The directory joined to the base. This, not the base, is what gets validated and created.
pub const RUNTIME_DIR_NAME: &str = "prism-mcpd";

/// The nine permission bits a runtime directory must have, exactly.
const PRIVATE_DIR_MODE: u32 = 0o700;
const PERMISSION_BITS: u32 = 0o777;

/// Why the runtime directory cannot be used. Every variant means "fail closed" (spec §4.1 step
/// 7, §4.6 last row); none is retried.
#[derive(Debug)]
pub enum RuntimeDirError {
    /// The base is not an absolute path, so two processes could resolve it differently.
    NotAbsolute {
        path: PathBuf,
    },
    /// The filesystem refused: `op` is `"inspect"` (the `lstat`) or `"create"` (the `mkdir`).
    Io {
        op: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Symlink {
        path: PathBuf,
    },
    NotADirectory {
        path: PathBuf,
    },
    NotOwned {
        path: PathBuf,
        owner: u32,
        current: u32,
    },
    /// `mode` is the directory's nine permission bits.
    InsecureMode {
        path: PathBuf,
        mode: u32,
    },
}

impl fmt::Display for RuntimeDirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAbsolute { path } => write!(
                f,
                "runtime directory base {} is not an absolute path",
                path.display()
            )?,
            Self::Io { op, path, source } => write!(
                f,
                "cannot {op} runtime directory {}: {source}",
                path.display()
            )?,
            Self::Symlink { path } => write!(
                f,
                "runtime directory {} is a symlink; it must be a real directory",
                path.display()
            )?,
            Self::NotADirectory { path } => {
                write!(f, "runtime directory {} is not a directory", path.display())?
            }
            Self::NotOwned {
                path,
                owner,
                current,
            } => write!(
                f,
                "runtime directory {} is owned by uid {owner}, not by the current user (uid {current})",
                path.display()
            )?,
            Self::InsecureMode { path, mode } => write!(
                f,
                "runtime directory {} has mode {mode:04o}; it must be 0700",
                path.display()
            )?,
        }
        write!(
            f,
            " (set {RUNTIME_DIR_ENV} to an absolute directory you own to use another location)"
        )
    }
}

impl std::error::Error for RuntimeDirError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// The runtime directory for this process: `$PRISM_MCP_RUNTIME_DIR` if set and non-empty, else
/// the user runtime directory (Linux `$XDG_RUNTIME_DIR`), else the temp directory (macOS
/// `$TMPDIR`), with `prism-mcpd` joined to whichever base applied. Nothing is created or
/// checked here; call [`validate_runtime_dir`] on the result before every use.
pub fn runtime_dir() -> Result<PathBuf, RuntimeDirError> {
    runtime_dir_from(
        std::env::var_os(RUNTIME_DIR_ENV),
        dirs::runtime_dir(),
        std::env::temp_dir(),
    )
}

/// [`runtime_dir`] with its three inputs explicit.
pub(crate) fn runtime_dir_from(
    override_base: Option<OsString>,
    user_runtime_dir: Option<PathBuf>,
    temp_dir: PathBuf,
) -> Result<PathBuf, RuntimeDirError> {
    let base = match override_base.filter(|value| !value.is_empty()) {
        Some(value) => PathBuf::from(value),
        None => user_runtime_dir.unwrap_or(temp_dir),
    };
    if !base.is_absolute() {
        return Err(RuntimeDirError::NotAbsolute { path: base });
    }
    Ok(base.join(RUNTIME_DIR_NAME))
}

/// What validation needs to know about a path, as `lstat` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DirFacts {
    pub(crate) is_symlink: bool,
    pub(crate) is_dir: bool,
    pub(crate) owner: u32,
    /// `st_mode`, file-type bits included.
    pub(crate) mode: u32,
}

/// The two filesystem operations validation performs, so tests can inject failures and races.
pub(crate) trait DirAccess {
    /// `lstat`: never follows a symlink at `path`. `Ok(None)` when nothing is there.
    fn inspect(&self, path: &Path) -> io::Result<Option<DirFacts>>;
    /// Create exactly `path`, not its parents, with mode 0700.
    fn create_private(&self, path: &Path) -> io::Result<()>;
}

pub(crate) struct RealDirAccess;

impl DirAccess for RealDirAccess {
    fn inspect(&self, path: &Path) -> io::Result<Option<DirFacts>> {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => Ok(Some(DirFacts {
                is_symlink: metadata.file_type().is_symlink(),
                is_dir: metadata.is_dir(),
                owner: metadata.uid(),
                mode: metadata.mode(),
            })),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn create_private(&self, path: &Path) -> io::Result<()> {
        std::fs::DirBuilder::new()
            .mode(PRIVATE_DIR_MODE)
            .create(path)
    }
}

/// The effective user id: the owner of files this process creates, and the uid a peer
/// credential check compares against.
pub fn current_uid() -> u32 {
    // SAFETY: `geteuid` takes no arguments, cannot fail and touches no memory.
    unsafe { libc::geteuid() }
}

/// Validate the runtime directory, creating it with mode 0700 if it is absent (its parent must
/// already exist). Call this on every use, not once per process: `Ok` says the directory was a
/// real directory, owned by the current user, mode 0700, at the moment of the call.
pub fn validate_runtime_dir(dir: &Path) -> Result<(), RuntimeDirError> {
    validate_runtime_dir_with(&RealDirAccess, dir, current_uid())
}

pub(crate) fn validate_runtime_dir_with(
    access: &impl DirAccess,
    dir: &Path,
    uid: u32,
) -> Result<(), RuntimeDirError> {
    if !dir.is_absolute() {
        return Err(RuntimeDirError::NotAbsolute {
            path: dir.to_path_buf(),
        });
    }
    // `lstat("dir/")` follows a symlink named `dir`. Rebuilding the path from its components
    // drops a trailing slash, so the final component itself is what gets inspected.
    let dir: PathBuf = dir.components().collect();
    let facts = match inspect(access, &dir)? {
        Some(facts) => facts,
        None => {
            match access.create_private(&dir) {
                Ok(()) => {}
                // Another process created it first; what is there now is checked like any
                // other existing directory.
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => {
                    return Err(RuntimeDirError::Io {
                        op: "create",
                        path: dir,
                        source,
                    })
                }
            }
            // A directory this process just created is inspected like one it found: creating
            // it proves nothing about what is at the path now.
            match inspect(access, &dir)? {
                Some(facts) => facts,
                None => {
                    return Err(RuntimeDirError::Io {
                        op: "inspect",
                        path: dir,
                        source: io::Error::new(
                            io::ErrorKind::NotFound,
                            "the directory disappeared right after it was created",
                        ),
                    })
                }
            }
        }
    };

    if facts.is_symlink {
        return Err(RuntimeDirError::Symlink { path: dir });
    }
    if !facts.is_dir {
        return Err(RuntimeDirError::NotADirectory { path: dir });
    }
    if facts.owner != uid {
        return Err(RuntimeDirError::NotOwned {
            path: dir,
            owner: facts.owner,
            current: uid,
        });
    }
    let permissions = facts.mode & PERMISSION_BITS;
    if permissions != PRIVATE_DIR_MODE {
        return Err(RuntimeDirError::InsecureMode {
            path: dir,
            mode: permissions,
        });
    }
    Ok(())
}

fn inspect(access: &impl DirAccess, dir: &Path) -> Result<Option<DirFacts>, RuntimeDirError> {
    access.inspect(dir).map_err(|source| RuntimeDirError::Io {
        op: "inspect",
        path: dir.to_path_buf(),
        source,
    })
}

#[cfg(test)]
#[path = "runtime_dir_tests.rs"]
mod tests;
````

- [ ] **Step 7: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::shared::runtime_dir > /tmp/s2-t1-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s2-t1-green.log
```

Expected: `test result: ok. 22 passed; 0 failed`. `a_directory_that_really_belongs_to_root_is_refused` prints a "skipped" line and passes when the tests run as root.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock src/mcp/mod.rs src/mcp/shared/mod.rs src/mcp/shared/runtime_dir.rs src/mcp/shared/runtime_dir_tests.rs
git commit -m "feat(mcp): runtime directory resolution and validation for shared mode"
```

- [ ] **Step 9: Prove the symlink test bites (after the commit)**

Temporarily, in `src/mcp/shared/runtime_dir.rs`, replace this:

```rust
        match std::fs::symlink_metadata(path) {
```

with this:

```rust
        match std::fs::metadata(path) {
```

```bash
cargo test --features mcp --lib mcp::shared::runtime_dir > /tmp/s2-t1-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s2-t1-bite.log
git checkout -- src/mcp/shared/runtime_dir.rs
```

Expected: `a_symlink_to_a_private_directory_is_refused_with_or_without_a_trailing_slash ... FAILED` and `21 passed; 1 failed`. The `git checkout` restores the committed file; confirm with `git status --short` (no output).

---

### Task 2: Canonical cache base

`CacheMode::Default` and an explicit `--cache-dir` that names the same place must be one input to the key, before the directory exists (spec §4.1 step 1).

**Files:**
- Create: `src/mcp/shared/cache_base.rs`, `src/mcp/shared/cache_base_tests.rs`
- Modify: `src/mcp/shared/mod.rs`

**Interfaces:**
- Consumes: `crate::mcp::CacheMode`; `dirs::cache_dir()`; in tests, `crate::navigation::cache::{nav_cache_dir, nav_cache_subdir}` and `crate::repo_loader::load_repo`.
- Produces (in `crate::mcp::shared::cache_base`):
  - `pub enum CacheBaseError { NoCache, Io { path: PathBuf, source: io::Error } }` (`Debug`, `Display`, `Error`)
  - `pub fn default_cache_base() -> PathBuf`
  - `pub fn effective_cache_base(cache: &CacheMode) -> Result<PathBuf, CacheBaseError>`
  - Crate-internal: `effective_cache_base_in(cache: &CacheMode, default_base: &Path, working_dir: impl FnOnce() -> io::Result<PathBuf>) -> Result<PathBuf, CacheBaseError>`

- [ ] **Step 1: Write the tests**

Create `src/mcp/shared/cache_base_tests.rs`:

````rust
//! Tests for the one canonical spelling of a configuration's cache base directory.
use super::*;
use crate::mcp::CacheMode;
use std::path::{Path, PathBuf};

/// A temp directory by its canonical path (on macOS the temp directory is reached through the
/// `/var` symlink).
fn canonical_temp() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let real = std::fs::canonicalize(dir.path()).unwrap();
    (dir, real)
}

fn dir(path: impl Into<PathBuf>) -> CacheMode {
    CacheMode::Dir(path.into())
}

/// A working directory that cannot be read. Every test that passes this shows that an
/// absolute base is resolved without asking for the working directory.
fn no_working_dir() -> std::io::Result<PathBuf> {
    Err(std::io::Error::from(std::io::ErrorKind::NotFound))
}

#[test]
fn a_cache_free_configuration_has_no_cache_base() {
    let outcome =
        effective_cache_base_in(&CacheMode::NoCache, Path::new("/unused"), no_working_dir);
    assert!(
        matches!(outcome, Err(CacheBaseError::NoCache)),
        "{outcome:?}"
    );
}

/// Spec §4.1 step 1 and §9: `CacheMode::Default` and the same directory given explicitly are
/// one base, and that is decided before the directory exists. Rules out comparing the strings
/// the user typed, and rules out creating the directory in order to canonicalize it.
#[test]
fn default_and_its_explicit_equivalents_are_one_base_before_the_directory_exists() {
    let (_guard, real) = canonical_temp();
    std::fs::create_dir(real.join("store")).unwrap();
    std::os::unix::fs::symlink("store", real.join("alias")).unwrap();
    let default_base = real.join("alias").join("nav-base");
    let expected = real.join("store").join("nav-base");

    let from_default =
        effective_cache_base_in(&CacheMode::Default, &default_base, no_working_dir).unwrap();
    assert_eq!(from_default, expected);

    let spellings = [
        dir(real.join("store/nav-base")),
        dir(real.join("store/nav-base/")),
        dir(real.join("store/./nav-base")),
        dir(real.join("alias/nav-base")),
        dir(real.join("store/not-made-yet/../nav-base")),
        dir(real.join("store/nav-base/deeper/..")),
    ];
    for spelling in &spellings {
        let resolved = effective_cache_base_in(spelling, Path::new("/unused"), no_working_dir)
            .unwrap_or_else(|error| panic!("{spelling:?}: {error}"));
        assert_eq!(resolved, expected, "{spelling:?}");
    }
    assert!(
        !expected.exists() && !real.join("store/not-made-yet").exists(),
        "resolving a cache base must not create it"
    );
}

#[test]
fn a_relative_base_is_resolved_against_the_working_directory() {
    let (_guard, real) = canonical_temp();
    std::fs::create_dir(real.join("work")).unwrap();
    let resolved = effective_cache_base_in(&dir("../caches/nav"), Path::new("/unused"), || {
        Ok(real.join("work"))
    })
    .unwrap();
    assert_eq!(resolved, real.join("caches").join("nav"));
}

/// A process whose working directory has been removed can still name an absolute base (every
/// other test here passes `no_working_dir`); a relative base then has no meaning and is an
/// error that names it.
#[test]
fn only_a_relative_base_needs_the_working_directory() {
    let outcome = effective_cache_base_in(&dir("caches/nav"), Path::new("/unused"), no_working_dir);
    assert!(
        matches!(&outcome, Err(CacheBaseError::Io { path, .. }) if path == Path::new("caches/nav")),
        "{outcome:?}"
    );
    let relative_default =
        effective_cache_base_in(&CacheMode::Default, Path::new("caches"), no_working_dir);
    assert!(
        matches!(relative_default, Err(CacheBaseError::Io { .. })),
        "{relative_default:?}"
    );
}

#[test]
fn an_existing_base_is_canonical_through_symlinks_dots_and_slashes() {
    let (guard, real) = canonical_temp();
    std::fs::create_dir_all(real.join("a/base")).unwrap();
    std::os::unix::fs::symlink(real.join("a"), real.join("link")).unwrap();
    for spelling in [
        real.join("a/base"),
        real.join("a/base/"),
        real.join("a/./base/."),
        real.join("link/base"),
        real.join("a/base/../base"),
        // The uncanonical spelling of the temp directory itself (differs on macOS).
        guard.path().join("a/base"),
    ] {
        let resolved =
            effective_cache_base_in(&dir(&spelling), Path::new("/unused"), no_working_dir).unwrap();
        assert_eq!(resolved, real.join("a/base"), "{}", spelling.display());
    }
}

/// Rules out removing `..` textually: `jump/..` is the parent of what `jump` points at, which
/// is where the cache layer's `create_dir_all` would put the directory.
#[test]
fn dot_dot_after_a_symlink_is_the_parent_of_the_link_target() {
    let (_guard, real) = canonical_temp();
    std::fs::create_dir_all(real.join("a/b")).unwrap();
    std::os::unix::fs::symlink(real.join("a/b"), real.join("jump")).unwrap();
    let resolved = effective_cache_base_in(
        &dir(real.join("jump/../leaf")),
        Path::new("/unused"),
        no_working_dir,
    )
    .unwrap();
    assert_eq!(resolved, real.join("a/leaf"));
}

/// A `..` that climbs out of a part that does not exist lands back on real ground, and the
/// components after it are resolved through symlinks again.
#[test]
fn a_symlink_after_climbing_out_of_a_missing_part_is_still_resolved() {
    let (_guard, real) = canonical_temp();
    std::fs::create_dir(real.join("store")).unwrap();
    std::os::unix::fs::symlink("store", real.join("alias")).unwrap();
    let resolved = effective_cache_base_in(
        &dir(real.join("missing/../alias/nav-base")),
        Path::new("/unused"),
        no_working_dir,
    )
    .unwrap();
    assert_eq!(resolved, real.join("store/nav-base"));
}

/// The claim behind all of the above, checked against the operation the cache layer performs
/// (`create_dir_all` on a path under the base): the spelling resolved before anything exists
/// is the canonical path of what then gets created. Rules out any normalization that
/// disagrees with the filesystem, whichever way it disagrees.
#[test]
fn the_resolved_base_is_where_create_dir_all_then_puts_it() {
    let (_guard, real) = canonical_temp();
    std::fs::create_dir_all(real.join("a/b")).unwrap();
    std::os::unix::fs::symlink(real.join("a/b"), real.join("jump")).unwrap();
    for (index, tail) in [
        "plain/base",
        "jump/base",
        "jump/../from-link-parent",
        "missing/../jump/after-missing",
        "deep/er/../../shallow/./base/",
        "a/b/../b/x/y/..",
    ]
    .into_iter()
    .enumerate()
    {
        let spelling = real.join(tail);
        let resolved =
            effective_cache_base_in(&dir(&spelling), Path::new("/unused"), no_working_dir)
                .unwrap_or_else(|error| panic!("case {index} ({tail}): {error}"));
        assert!(!resolved.exists(), "case {index} ({tail}) starts absent");
        std::fs::create_dir_all(&spelling).unwrap();
        assert_eq!(
            std::fs::canonicalize(&spelling).unwrap(),
            resolved,
            "case {index} ({tail})"
        );
    }
}

/// The daemon is started with the base the shim resolved, and resolves it again; the two must
/// agree or every connection would be answered `MISMATCH`.
#[test]
fn resolving_a_resolved_base_changes_nothing() {
    let (_guard, real) = canonical_temp();
    std::fs::create_dir(real.join("store")).unwrap();
    std::os::unix::fs::symlink("store", real.join("alias")).unwrap();
    for spelling in [real.join("alias/x/../nav"), real.join("alias")] {
        let once =
            effective_cache_base_in(&dir(&spelling), Path::new("/unused"), no_working_dir).unwrap();
        let twice =
            effective_cache_base_in(&dir(&once), Path::new("/unused"), no_working_dir).unwrap();
        assert_eq!(once, twice);
    }
}

#[test]
fn climbing_above_the_root_stays_at_the_root() {
    let resolved = effective_cache_base_in(
        &dir("/../../prism-no-such-dir"),
        Path::new("/unused"),
        no_working_dir,
    )
    .unwrap();
    assert_eq!(resolved, PathBuf::from("/prism-no-such-dir"));
}

/// Neither location can ever hold a cache, so neither names a base two processes could share.
#[test]
fn a_base_below_a_regular_file_or_through_a_dangling_symlink_is_an_error() {
    let (_guard, real) = canonical_temp();
    std::fs::write(real.join("file"), "").unwrap();
    std::os::unix::fs::symlink(real.join("nowhere"), real.join("dangling")).unwrap();
    for spelling in [
        real.join("file/base"),
        real.join("dangling/base"),
        real.join("dangling"),
    ] {
        let outcome =
            effective_cache_base_in(&dir(&spelling), Path::new("/unused"), no_working_dir);
        assert!(
            matches!(&outcome, Err(CacheBaseError::Io { .. })),
            "{}: {outcome:?}",
            spelling.display()
        );
    }
}

/// The default base is computed here and, separately, inside the navigation cache. This pins
/// the two together: a `CacheMode::Default` daemon caches exactly where the in-process server
/// does.
#[test]
fn the_default_base_is_the_one_the_navigation_cache_uses() {
    let repo_dir = tempfile::tempdir().unwrap();
    std::fs::write(repo_dir.path().join("a.py"), "def f():\n    return 1\n").unwrap();
    let repo = crate::repo_loader::load_repo(repo_dir.path()).unwrap();
    assert_eq!(
        crate::navigation::cache::nav_cache_dir(&repo),
        crate::navigation::cache::nav_cache_subdir(&default_cache_base(), &repo)
    );
}

#[test]
fn the_environment_reader_agrees_with_the_pure_resolver() {
    let (_guard, real) = canonical_temp();
    let explicit = dir(real.join("explicit"));
    assert_eq!(
        effective_cache_base(&explicit).unwrap(),
        real.join("explicit")
    );
    assert_eq!(
        effective_cache_base(&CacheMode::Default).unwrap(),
        effective_cache_base(&dir(default_cache_base())).unwrap()
    );
    assert!(matches!(
        effective_cache_base(&CacheMode::NoCache),
        Err(CacheBaseError::NoCache)
    ));
}
````

Create `src/mcp/shared/cache_base.rs`:

```rust
#[cfg(test)]
#[path = "cache_base_tests.rs"]
mod tests;
```

In `src/mcp/shared/mod.rs`, replace this:

```rust
pub mod runtime_dir;
```

with this:

```rust
pub mod cache_base;
pub mod runtime_dir;
```

- [ ] **Step 2: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::shared::cache_base > /tmp/s2-t2-red.log 2>&1; grep -cE '^error\[' /tmp/s2-t2-red.log; tail -1 /tmp/s2-t2-red.log
```

Expected: a count of 20 or more (`cannot find function effective_cache_base_in`, `use of undeclared type CacheBaseError`) and `error: could not compile`.

- [ ] **Step 3: Implement**

Replace the whole of `src/mcp/shared/cache_base.rs` with:

````rust
//! The effective cache base directory of a server configuration, in one canonical spelling
//! (spec §4.1 step 1, §4.3). The navigation cache treats `--cache-dir` as a base and appends a
//! per-repository suffix, and it creates the directory lazily, so two spellings of one base
//! have to compare equal before anything exists there. Nothing here creates a directory.
use crate::mcp::CacheMode;
use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub enum CacheBaseError {
    /// `CacheMode::NoCache`: a cache-free server is never shared (spec §4.1 step 2).
    NoCache,
    /// The base cannot be resolved: a component is unreadable, is not a directory, or is a
    /// symlink that leads nowhere. `path` is the component that failed.
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for CacheBaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCache => write!(f, "a server without a cache has no cache base directory"),
            Self::Io { path, source } => write!(
                f,
                "cannot resolve the cache base directory at {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for CacheBaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoCache => None,
            Self::Io { source, .. } => Some(source),
        }
    }
}

/// The base `CacheMode::Default` caches under. Must stay equal to the default inside
/// `navigation::cache` (`nav_cache_dir`, `NavigationIndex::build_cached`); a test pins it.
pub fn default_cache_base() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir)
}

/// The canonical cache base of `cache` for this process: `Default` and an explicit directory
/// that names the same place give the same path, whether or not the place exists yet.
pub fn effective_cache_base(cache: &CacheMode) -> Result<PathBuf, CacheBaseError> {
    effective_cache_base_in(cache, &default_cache_base(), std::env::current_dir)
}

/// [`effective_cache_base`] with the default base and the working directory explicit.
pub(crate) fn effective_cache_base_in(
    cache: &CacheMode,
    default_base: &Path,
    working_dir: impl FnOnce() -> io::Result<PathBuf>,
) -> Result<PathBuf, CacheBaseError> {
    let base = match cache {
        CacheMode::NoCache => return Err(CacheBaseError::NoCache),
        CacheMode::Default => default_base,
        CacheMode::Dir(base) => base.as_path(),
    };
    if base.is_absolute() {
        return canonical_spelling(base);
    }
    // Only a relative base depends on the working directory, so only then is it read: a
    // process whose working directory has been removed can still name an absolute base.
    let working_dir = working_dir().map_err(|source| CacheBaseError::Io {
        path: base.to_path_buf(),
        source,
    })?;
    canonical_spelling(&working_dir.join(base))
}

/// Resolve `absolute` the way the kernel would, as far as it exists, and keep the rest as
/// written. Every component that exists is resolved through symlinks, so `resolved` is always
/// a real path or a real path plus names that do not exist yet; `..` can therefore drop the
/// last name: the parent of a real directory, or of a directory `create_dir_all` would make.
fn canonical_spelling(absolute: &Path) -> Result<PathBuf, CacheBaseError> {
    let mut resolved = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => resolved.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            Component::Normal(name) => {
                resolved.push(name);
                match std::fs::symlink_metadata(&resolved) {
                    Ok(_) => {
                        resolved = std::fs::canonicalize(&resolved).map_err(|source| {
                            CacheBaseError::Io {
                                path: resolved.clone(),
                                source,
                            }
                        })?;
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(source) => {
                        return Err(CacheBaseError::Io {
                            path: resolved,
                            source,
                        })
                    }
                }
            }
        }
    }
    Ok(resolved)
}

#[cfg(test)]
#[path = "cache_base_tests.rs"]
mod tests;
````

- [ ] **Step 4: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::shared::cache_base > /tmp/s2-t2-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s2-t2-green.log
```

Expected: `test result: ok. 13 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add src/mcp/shared/mod.rs src/mcp/shared/cache_base.rs src/mcp/shared/cache_base_tests.rs
git commit -m "feat(mcp): canonical cache base directory for shared mode"
```

---

### Task 3: Identity key

**Files:**
- Create: `src/mcp/shared/identity.rs`, `src/mcp/shared/identity_tests.rs`
- Modify: `src/mcp/shared/mod.rs`

**Interfaces:**
- Consumes: `effective_cache_base`, `CacheBaseError` (Task 2); `crate::api::build_info()` (`package_version`, `grammar_fingerprint`, `build_identity`); `crate::mcp::{RefreshPolicy, ServerConfig}`; `sha2::Sha256`; in tests, `crate::mcp::session::canonical_config`.
- Produces:
  - In `crate::mcp::shared`: `pub const SHARED_PROTOCOL_VERSION: u32 = 1;`
  - In `crate::mcp::shared::identity`:
    - `pub struct BuildIdentity { pub package_version: String, pub grammar_fingerprint: String, pub cache_build_identity: String }` with `pub fn current() -> Self`
    - `pub struct FsIdentity { pub dev: u64, pub ino: u64 }` with `pub fn of(path: &Path) -> io::Result<Self>`
    - `pub fn refresh_policy_name(policy: RefreshPolicy) -> &'static str` (`"warn-only"`, `"auto-full"`, `"auto-incremental"`)
    - `pub enum IdentityError { RepoRoot { path: PathBuf, source: io::Error }, CacheBase(CacheBaseError) }`
    - `pub struct DaemonIdentity` (private fields; `Clone`, `Debug`, `PartialEq`, `Eq`) with `pub fn resolve(cfg: &ServerConfig) -> Result<Self, IdentityError>`, `pub fn resolve_with(cfg: &ServerConfig, build: BuildIdentity, protocol: u32) -> Result<Self, IdentityError>`, `pub fn repo_root(&self) -> &Path`, `pub fn root_fs(&self) -> FsIdentity`, `pub fn build(&self) -> &BuildIdentity`, `pub fn protocol(&self) -> u32`, `pub fn cache_base(&self) -> &Path`, `pub fn refresh_policy(&self) -> RefreshPolicy`, `pub fn key(&self) -> IdentityKey`
    - `pub struct IdentityKey` (`Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`; `Display` and `Debug` print the hex) with `pub fn to_hex(&self) -> String`, `pub fn locator(&self) -> String`, `pub fn from_hex(text: &str) -> Result<Self, InvalidIdentityKey>`
    - `pub struct InvalidIdentityKey;`

- [ ] **Step 1: Write the tests**

Create `src/mcp/shared/identity_tests.rs`:

````rust
//! Tests for the identity key: what goes into it, that the encoding loses nothing, and that
//! one configuration has one key however it is spelled.
use super::*;
use crate::mcp::{CacheMode, RefreshPolicy, ServerConfig, StartupMode};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn sample() -> DaemonIdentity {
    DaemonIdentity {
        repo_root: PathBuf::from("/work/repo"),
        root_fs: FsIdentity {
            dev: 16_777_233,
            ino: 4_242,
        },
        build: BuildIdentity {
            package_version: "3.1.2".into(),
            grammar_fingerprint: "grammars-v1".into(),
            cache_build_identity: "build-aaaa".into(),
        },
        protocol: 1,
        cache_base: PathBuf::from("/home/u/.cache"),
        refresh_policy: RefreshPolicy::WarnOnly,
    }
}

/// A decoder written from the format's description, not from the encoder: a sequence of
/// fields, each an 8-byte little-endian length followed by that many bytes.
fn decode_fields(mut rest: &[u8]) -> Vec<Vec<u8>> {
    let mut fields = Vec::new();
    while !rest.is_empty() {
        let (length, tail) = rest.split_at(8);
        let length = u64::from_le_bytes(length.try_into().unwrap()) as usize;
        let (field, tail) = tail.split_at(length);
        fields.push(field.to_vec());
        rest = tail;
    }
    fields
}

fn expected_fields(identity: &DaemonIdentity) -> Vec<Vec<u8>> {
    vec![
        b"prism-mcpd identity".to_vec(),
        identity.repo_root.as_os_str().as_bytes().to_vec(),
        identity.root_fs.dev.to_le_bytes().to_vec(),
        identity.root_fs.ino.to_le_bytes().to_vec(),
        identity.build.package_version.as_bytes().to_vec(),
        identity.build.grammar_fingerprint.as_bytes().to_vec(),
        identity.build.cache_build_identity.as_bytes().to_vec(),
        identity.protocol.to_le_bytes().to_vec(),
        identity.cache_base.as_os_str().as_bytes().to_vec(),
        refresh_policy_name(identity.refresh_policy)
            .as_bytes()
            .to_vec(),
    ]
}

// ── The encoding and the hash ──

/// The shim and the daemon are separate processes that must compute the same key, so the key
/// is a fixed function of its inputs. Rules out a per-process hasher (`RandomState`), a
/// non-cryptographic one, and any accidental change of field order or width. The expected
/// digest was computed outside this crate:
///
/// ```text
/// import hashlib, struct
/// field = lambda b: struct.pack('<Q', len(b)) + b
/// fields = [b'prism-mcpd identity', b'/work/repo', struct.pack('<Q', 16777233),
///           struct.pack('<Q', 4242), b'3.1.2', b'grammars-v1', b'build-aaaa',
///           struct.pack('<I', 1), b'/home/u/.cache', b'warn-only']
/// print(hashlib.sha256(b''.join(map(field, fields))).hexdigest())
/// ```
#[test]
fn the_key_is_sha256_of_the_length_prefixed_fields() {
    assert_eq!(
        sample().key().to_hex(),
        "70434641ffa9426029dab705875d9c4ad19cba621e148bf560bc8a060e0a88d8"
    );
}

/// Spec §4.3 "lossless": every input can be read back out of the encoding, so two different
/// inputs cannot encode alike. The second identity uses bytes a lossy conversion would
/// destroy: invalid UTF-8, a NUL, a newline, and empty strings.
#[test]
fn every_input_can_be_read_back_from_the_encoding() {
    let nasty = DaemonIdentity {
        repo_root: PathBuf::from(OsStr::from_bytes(b"/work/\xff\xfe/re\0po\n")),
        root_fs: FsIdentity {
            dev: u64::MAX,
            ino: 0,
        },
        build: BuildIdentity {
            package_version: String::new(),
            grammar_fingerprint: String::new(),
            cache_build_identity: "x".into(),
        },
        protocol: u32::MAX,
        cache_base: PathBuf::from(OsStr::from_bytes(b"/c\xc3\x28")),
        refresh_policy: RefreshPolicy::AutoIncremental,
    };
    for identity in [sample(), nasty] {
        assert_eq!(
            decode_fields(&identity.encode()),
            expected_fields(&identity)
        );
    }
}

/// Rules out concatenating the inputs without lengths: both identities below join to the same
/// bytes.
#[test]
fn moving_bytes_across_a_field_boundary_changes_the_key() {
    let mut left = sample();
    left.build.package_version = "3.1".into();
    left.build.grammar_fingerprint = "2grammars".into();
    let mut right = sample();
    right.build.package_version = "3.12".into();
    right.build.grammar_fingerprint = "grammars".into();
    assert_ne!(left.key(), right.key());

    let mut left = sample();
    left.repo_root = PathBuf::from("/a/b");
    left.cache_base = PathBuf::from("/c");
    let mut right = sample();
    right.repo_root = PathBuf::from("/a");
    right.cache_base = PathBuf::from("/b/c");
    assert_ne!(left.key(), right.key());
}

/// Rules out `to_string_lossy`: both roots become the same replacement character.
#[test]
fn roots_that_differ_only_in_invalid_utf8_bytes_have_different_keys() {
    let mut left = sample();
    left.repo_root = PathBuf::from(OsStr::from_bytes(b"/work/\xff"));
    let mut right = sample();
    right.repo_root = PathBuf::from(OsStr::from_bytes(b"/work/\xfe"));
    assert_ne!(left.key(), right.key());
}

/// Spec §4.3 and §9: each input changes the key. One change per row; every row must give a
/// key no other row gives. Rules out an encoder that forgets a field.
#[test]
fn every_input_changes_the_key() {
    type Change = (&'static str, fn(&mut DaemonIdentity));
    let changes: Vec<Change> = vec![
        ("repo root", |i| i.repo_root = PathBuf::from("/work/repo2")),
        ("root device", |i| i.root_fs.dev += 1),
        ("root inode", |i| i.root_fs.ino += 1),
        ("package version", |i| {
            i.build.package_version = "3.1.3".into()
        }),
        ("grammar fingerprint", |i| {
            i.build.grammar_fingerprint = "grammars-v2".into()
        }),
        ("analyzer build identity", |i| {
            i.build.cache_build_identity = "build-bbbb".into()
        }),
        ("protocol version", |i| i.protocol = 2),
        ("cache base", |i| {
            i.cache_base = PathBuf::from("/home/u/.cache2")
        }),
        ("refresh policy auto-full", |i| {
            i.refresh_policy = RefreshPolicy::AutoFull
        }),
        ("refresh policy auto-incremental", |i| {
            i.refresh_policy = RefreshPolicy::AutoIncremental
        }),
    ];
    let mut keys = BTreeSet::new();
    keys.insert(sample().key().to_hex());
    for (name, change) in &changes {
        let mut identity = sample();
        change(&mut identity);
        assert!(
            keys.insert(identity.key().to_hex()),
            "changing the {name} did not give a new key"
        );
    }
    assert_eq!(keys.len(), changes.len() + 1);
}

/// Rules out combining fields in an order-blind way (a sum or an XOR of per-field hashes).
#[test]
fn swapping_two_inputs_changes_the_key() {
    let mut swapped = sample();
    std::mem::swap(&mut swapped.root_fs.dev, &mut swapped.root_fs.ino);
    assert_ne!(swapped.key(), sample().key());

    let mut swapped = sample();
    std::mem::swap(&mut swapped.repo_root, &mut swapped.cache_base);
    assert_ne!(swapped.key(), sample().key());
}

#[test]
fn the_current_build_is_the_three_values_the_spec_names() {
    let build = BuildIdentity::current();
    assert_eq!(build.package_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(build.grammar_fingerprint, env!("GRAMMAR_FINGERPRINT"));
    assert_eq!(
        build.cache_build_identity,
        env!("PRISM_CACHE_BUILD_IDENTITY")
    );
    assert!(!build.cache_build_identity.is_empty());
}

#[test]
fn refresh_policies_are_keyed_by_their_command_line_names() {
    assert_eq!(refresh_policy_name(RefreshPolicy::WarnOnly), "warn-only");
    assert_eq!(refresh_policy_name(RefreshPolicy::AutoFull), "auto-full");
    assert_eq!(
        refresh_policy_name(RefreshPolicy::AutoIncremental),
        "auto-incremental"
    );
}

// ── The key as text ──

#[test]
fn a_key_prints_as_64_lowercase_hex_digits_and_reads_back() {
    let key = sample().key();
    let hex = key.to_hex();
    assert_eq!(hex.len(), 64);
    assert!(hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
    assert_eq!(key.to_string(), hex);
    assert_eq!(IdentityKey::from_hex(&hex).unwrap(), key);
    assert_eq!(key.locator(), hex[..16]);
}

/// One key has one spelling, so comparing keys is comparing bytes.
#[test]
fn a_malformed_key_is_not_a_key() {
    let hex = sample().key().to_hex();
    let padded = format!(" {}", &hex[1..]);
    let non_ascii = format!("{}é", &hex[..62]);
    assert_eq!(non_ascii.len(), 64, "64 bytes, 63 characters");
    for bad in [
        "",
        &hex[..63],
        &format!("{hex}0"),
        &hex.to_uppercase(),
        &hex.replacen(|c: char| c.is_ascii_hexdigit(), "g", 1),
        &hex[..16],
        &padded,
        &non_ascii,
        &format!("+f{}", &hex[2..]),
    ] {
        assert_eq!(
            IdentityKey::from_hex(bad),
            Err(InvalidIdentityKey),
            "{bad:?}"
        );
    }
}

// ── Resolving an identity from a configuration ──

struct Checkout {
    _guard: tempfile::TempDir,
    /// The temp directory as `tempfile` names it (on macOS, through the `/var` symlink).
    as_given: PathBuf,
    /// The same directory by its canonical path.
    real: PathBuf,
}

/// A temp directory holding `repo/`, `repo/sub/`, `cache/` and the symlink `repo-link -> repo`.
fn checkout() -> Checkout {
    let guard = tempfile::tempdir().unwrap();
    let as_given = guard.path().to_path_buf();
    let real = std::fs::canonicalize(&as_given).unwrap();
    std::fs::create_dir_all(real.join("repo/sub")).unwrap();
    std::fs::create_dir(real.join("cache")).unwrap();
    std::os::unix::fs::symlink("repo", real.join("repo-link")).unwrap();
    Checkout {
        _guard: guard,
        as_given,
        real,
    }
}

fn config(root: impl Into<PathBuf>, cache: impl Into<PathBuf>) -> ServerConfig {
    let mut cfg = ServerConfig::new(root.into());
    cfg.cache = CacheMode::Dir(cache.into());
    cfg
}

fn key_of(cfg: &ServerConfig) -> IdentityKey {
    DaemonIdentity::resolve(cfg).unwrap().key()
}

fn other_build() -> BuildIdentity {
    let mut build = BuildIdentity::current();
    build.cache_build_identity.push_str("-rebuilt");
    build
}

#[test]
fn resolving_reads_the_root_the_cache_base_the_policy_and_this_build() {
    let checkout = checkout();
    let mut cfg = config(checkout.real.join("repo"), checkout.real.join("cache"));
    cfg.refresh_policy = RefreshPolicy::AutoFull;
    let identity = DaemonIdentity::resolve(&cfg).unwrap();

    assert_eq!(identity.repo_root(), checkout.real.join("repo"));
    assert_eq!(identity.cache_base(), checkout.real.join("cache"));
    assert_eq!(identity.refresh_policy(), RefreshPolicy::AutoFull);
    assert_eq!(identity.build(), &BuildIdentity::current());
    assert_eq!(
        identity.protocol(),
        crate::mcp::shared::SHARED_PROTOCOL_VERSION
    );
    assert_eq!(
        identity.root_fs(),
        FsIdentity::of(&checkout.real.join("repo")).unwrap()
    );
    let metadata = std::fs::metadata(checkout.real.join("repo")).unwrap();
    use std::os::unix::fs::MetadataExt;
    assert_eq!(
        identity.root_fs(),
        FsIdentity {
            dev: metadata.dev(),
            ino: metadata.ino()
        }
    );
}

/// Spec §4.1 step 1: the root is canonical "exactly as today".
#[test]
fn the_root_is_spelled_as_the_in_process_server_spells_it() {
    let checkout = checkout();
    let cfg = config(
        checkout.as_given.join("repo-link/sub/.."),
        checkout.real.join("cache"),
    );
    let canonical = crate::mcp::session::canonical_config(&cfg).unwrap();
    assert_eq!(
        DaemonIdentity::resolve(&cfg).unwrap().repo_root(),
        canonical.repo_root
    );
}

/// The same inputs spelled differently give the same key.
#[test]
fn one_checkout_and_one_cache_base_have_one_key_however_they_are_spelled() {
    let checkout = checkout();
    let reference = key_of(&config(
        checkout.real.join("repo"),
        checkout.real.join("cache"),
    ));

    let mut trailing_slash = checkout.real.join("repo").into_os_string();
    trailing_slash.push("/");
    let roots = [
        PathBuf::from(trailing_slash),
        checkout.real.join("repo/."),
        checkout.real.join("repo/sub/.."),
        checkout.real.join("repo-link"),
        checkout.as_given.join("repo"),
    ];
    for root in &roots {
        assert_eq!(
            key_of(&config(root, checkout.real.join("cache"))),
            reference,
            "root spelled {}",
            root.display()
        );
    }

    std::os::unix::fs::symlink("cache", checkout.real.join("cache-link")).unwrap();
    let bases = [
        checkout.real.join("cache/"),
        checkout.real.join("cache/."),
        checkout.real.join("cache/not-there/.."),
        checkout.real.join("cache-link"),
        checkout.as_given.join("cache"),
    ];
    for base in &bases {
        assert_eq!(
            key_of(&config(checkout.real.join("repo"), base)),
            reference,
            "cache base spelled {}",
            base.display()
        );
    }
}

/// Two checkouts, two cache bases, two builds, two protocol versions and two refresh policies
/// each give two keys. This is what keeps one index from answering for another checkout,
/// another analyzer build or another cache.
#[test]
fn each_configuration_difference_gives_a_different_key() {
    let checkout = checkout();
    std::fs::create_dir(checkout.real.join("repo2")).unwrap();
    let base_cfg = config(checkout.real.join("repo"), checkout.real.join("cache"));
    let reference = key_of(&base_cfg);

    let other_checkout = config(checkout.real.join("repo2"), checkout.real.join("cache"));
    assert_ne!(key_of(&other_checkout), reference, "two checkouts");

    // A worktree nested in the checkout is its own checkout (spec §2).
    let nested = config(checkout.real.join("repo/sub"), checkout.real.join("cache"));
    assert_ne!(
        key_of(&nested),
        reference,
        "a directory inside the checkout"
    );

    let other_cache = config(checkout.real.join("repo"), checkout.real.join("cache2"));
    assert_ne!(key_of(&other_cache), reference, "two cache bases");

    for policy in [RefreshPolicy::AutoFull, RefreshPolicy::AutoIncremental] {
        let mut other_policy = base_cfg.clone();
        other_policy.refresh_policy = policy;
        assert_ne!(key_of(&other_policy), reference, "{policy:?}");
    }

    let rebuilt = DaemonIdentity::resolve_with(
        &base_cfg,
        other_build(),
        crate::mcp::shared::SHARED_PROTOCOL_VERSION,
    )
    .unwrap();
    assert_ne!(rebuilt.key(), reference, "two builds");

    let next_protocol = DaemonIdentity::resolve_with(
        &base_cfg,
        BuildIdentity::current(),
        crate::mcp::shared::SHARED_PROTOCOL_VERSION + 1,
    )
    .unwrap();
    assert_ne!(next_protocol.key(), reference, "two protocol versions");

    let same_again = DaemonIdentity::resolve_with(
        &base_cfg,
        BuildIdentity::current(),
        crate::mcp::shared::SHARED_PROTOCOL_VERSION,
    )
    .unwrap();
    assert_eq!(same_again.key(), reference, "and the same inputs agree");
}

/// Spec §4.3 "excluded by design" and §9 "`--first-call-wait` does not": connection-local
/// settings travel in the preamble. Rules out hashing the whole `ServerConfig`.
#[test]
fn connection_local_settings_do_not_change_the_key() {
    let checkout = checkout();
    let base_cfg = config(checkout.real.join("repo"), checkout.real.join("cache"));
    let reference = key_of(&base_cfg);

    let mut patient = base_cfg.clone();
    patient.first_call_wait = Duration::from_secs(600);
    assert_eq!(key_of(&patient), reference, "--first-call-wait");

    let mut warm = base_cfg.clone();
    warm.startup = StartupMode::Background;
    assert_eq!(key_of(&warm), reference, "--warm-at-startup");
}

/// Spec §4.3 and §4.6: a root that is renamed away and replaced is a different checkout at the
/// same pathname. Rules out a key made of the path alone.
#[test]
fn a_root_replaced_at_the_same_pathname_has_a_new_key() {
    let checkout = checkout();
    let root = checkout.real.join("repo");
    let cfg = config(&root, checkout.real.join("cache"));
    let before = DaemonIdentity::resolve(&cfg).unwrap();

    // Keep the old directory alive so its inode cannot be reused for the new one.
    std::fs::rename(&root, checkout.real.join("repo-moved")).unwrap();
    assert!(
        FsIdentity::of(&root).is_err(),
        "a missing root has no filesystem identity"
    );
    std::fs::create_dir(&root).unwrap();
    let after = DaemonIdentity::resolve(&cfg).unwrap();

    assert_eq!(before.repo_root(), after.repo_root());
    assert_ne!(before.root_fs(), after.root_fs());
    assert_ne!(before.key(), after.key());
    assert_eq!(
        FsIdentity::of(&checkout.real.join("repo-moved")).unwrap(),
        before.root_fs(),
        "the moved directory keeps its identity"
    );
}

/// Spec §9: "cache-base normalization (Default vs. explicit equivalent)".
#[test]
fn the_default_cache_and_the_same_directory_given_explicitly_have_one_key() {
    let checkout = checkout();
    let by_default = ServerConfig::new(checkout.real.join("repo"));
    let explicit = config(
        checkout.real.join("repo"),
        crate::mcp::shared::cache_base::default_cache_base(),
    );
    assert_eq!(key_of(&by_default), key_of(&explicit));
}

/// The shim starts the daemon with the canonical root, the canonical cache base and the
/// policy; the daemon resolves its own identity from those arguments. If the two disagreed,
/// every `HELLO` would be answered `MISMATCH`.
#[test]
fn a_daemon_started_from_an_identity_s_own_values_computes_the_same_key() {
    let checkout = checkout();
    std::os::unix::fs::symlink("cache", checkout.real.join("cache-link")).unwrap();
    let mut shim_cfg = config(
        checkout.as_given.join("repo-link/sub/.."),
        checkout.as_given.join("cache-link/later/.."),
    );
    shim_cfg.refresh_policy = RefreshPolicy::AutoIncremental;
    shim_cfg.startup = StartupMode::Background;
    shim_cfg.first_call_wait = Duration::from_secs(5);

    for shim_cfg in [shim_cfg, ServerConfig::new(checkout.as_given.join("repo"))] {
        let shim = DaemonIdentity::resolve(&shim_cfg).unwrap();
        let mut daemon_cfg = config(shim.repo_root(), shim.cache_base());
        daemon_cfg.refresh_policy = shim.refresh_policy();
        let daemon = DaemonIdentity::resolve(&daemon_cfg).unwrap();
        assert_eq!(daemon, shim);
        assert_eq!(daemon.key(), shim.key());
    }
}

#[test]
fn a_cache_free_configuration_has_no_identity() {
    let checkout = checkout();
    let mut cfg = ServerConfig::new(checkout.real.join("repo"));
    cfg.cache = CacheMode::NoCache;
    let outcome = DaemonIdentity::resolve(&cfg);
    assert!(
        matches!(
            outcome,
            Err(IdentityError::CacheBase(
                crate::mcp::shared::cache_base::CacheBaseError::NoCache
            ))
        ),
        "{outcome:?}"
    );
}

#[test]
fn a_missing_root_is_an_error_that_names_it() {
    let checkout = checkout();
    let cfg = config(
        checkout.real.join("no-such-repo"),
        checkout.real.join("cache"),
    );
    let outcome = DaemonIdentity::resolve(&cfg);
    match outcome {
        Err(IdentityError::RepoRoot { path, source }) => {
            assert_eq!(path, checkout.real.join("no-such-repo"));
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("{other:?}"),
    }
}

/// Spec §9 test 5 expects a failed start to leave "no cache dir created anywhere"; resolving
/// the identity is the first thing a shim does, so it must create nothing.
#[test]
fn resolving_an_identity_creates_nothing() {
    let checkout = checkout();
    let cache = checkout.real.join("cache/not/made/yet");
    let cfg = config(checkout.real.join("repo"), &cache);
    let before = listing(&checkout.real);
    let identity = DaemonIdentity::resolve(&cfg).unwrap();
    assert_eq!(identity.cache_base(), cache);
    assert_eq!(listing(&checkout.real), before);
    assert!(!checkout.real.join("cache/not").exists());
}

/// Every path under `root`, without following symlinks.
fn listing(root: &Path) -> BTreeSet<PathBuf> {
    let mut found = BTreeSet::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                pending.push(entry.path());
            }
            found.insert(entry.path());
        }
    }
    found
}
````

Create `src/mcp/shared/identity.rs`:

```rust
#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
```

In `src/mcp/shared/mod.rs`, replace this:

```rust
pub mod cache_base;
pub mod runtime_dir;
```

with this:

```rust
pub mod cache_base;
pub mod identity;
pub mod runtime_dir;

/// Version of the private shim–daemon protocol (spec §4.4). It is part of the identity key
/// (§4.3), so builds that speak different versions never look for each other's socket, and
/// the preamble repeats it so that a version difference is reported, not misparsed.
pub const SHARED_PROTOCOL_VERSION: u32 = 1;
```

- [ ] **Step 2: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::shared::identity > /tmp/s2-t3-red.log 2>&1; grep -cE '^error\[' /tmp/s2-t3-red.log; tail -1 /tmp/s2-t3-red.log
```

Expected: a count of 40 or more (`use of undeclared type DaemonIdentity`, `BuildIdentity`, `FsIdentity`, `IdentityKey`) and `error: could not compile`.

- [ ] **Step 3: Implement**

Replace the whole of `src/mcp/shared/identity.rs` with:

````rust
//! The identity of a shared daemon: everything that changes the index content or the analyzer's
//! answers (spec §4.3). Its SHA-256 key names the daemon's socket, locks, pid file and log, and
//! is compared in full in the preamble (§4.4), so one daemon never answers for another
//! checkout, another analyzer build, another cache or another refresh policy.
//!
//! Connection-local settings (output shape, result cap, `--first-call-wait`,
//! `--warm-at-startup`) are deliberately not part of it; they travel in the preamble.
use super::cache_base::{effective_cache_base, CacheBaseError};
use super::SHARED_PROTOCOL_VERSION;
use crate::mcp::{RefreshPolicy, ServerConfig};
use sha2::{Digest, Sha256};
use std::fmt;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// First field of the encoding, so the digest cannot be mistaken for a hash of something else.
const ENCODING_DOMAIN: &[u8] = b"prism-mcpd identity";

/// The build of the analyzer, as far as it can change an answer or an on-disk cache.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildIdentity {
    /// `CARGO_PKG_VERSION`.
    pub package_version: String,
    /// `GRAMMAR_FINGERPRINT`: the tree-sitter grammar versions.
    pub grammar_fingerprint: String,
    /// `PRISM_CACHE_BUILD_IDENTITY`: the analyzer build identity the CPG cache checks.
    pub cache_build_identity: String,
}

impl BuildIdentity {
    /// The build of the running binary.
    pub fn current() -> Self {
        let info = crate::api::build_info();
        Self {
            package_version: info.package_version.to_string(),
            grammar_fingerprint: info.grammar_fingerprint.to_string(),
            cache_build_identity: info.build_identity.to_string(),
        }
    }
}

/// Which directory a pathname names right now: the mounted device number and the inode. A
/// directory that is renamed away and replaced keeps its pathname and changes this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FsIdentity {
    pub dev: u64,
    pub ino: u64,
}

impl FsIdentity {
    /// The identity of what `path` names now. An error means there is nothing usable there.
    pub fn of(path: &Path) -> io::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        Ok(Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
        })
    }
}

/// The name a refresh policy has on the command line (`--refresh-policy`); also its spelling
/// in the key.
pub fn refresh_policy_name(policy: RefreshPolicy) -> &'static str {
    match policy {
        RefreshPolicy::WarnOnly => "warn-only",
        RefreshPolicy::AutoFull => "auto-full",
        RefreshPolicy::AutoIncremental => "auto-incremental",
    }
}

#[derive(Debug)]
pub enum IdentityError {
    /// The repository root cannot be resolved or inspected.
    RepoRoot {
        path: PathBuf,
        source: io::Error,
    },
    CacheBase(CacheBaseError),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RepoRoot { path, source } => write!(
                f,
                "cannot resolve the repository root {}: {source}",
                path.display()
            ),
            Self::CacheBase(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for IdentityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RepoRoot { source, .. } => Some(source),
            Self::CacheBase(error) => Some(error),
        }
    }
}

impl From<CacheBaseError> for IdentityError {
    fn from(error: CacheBaseError) -> Self {
        Self::CacheBase(error)
    }
}

/// Everything one daemon is for. Two configurations share a daemon exactly when these are
/// equal; the fields are private so that an identity always comes from [`Self::resolve`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonIdentity {
    repo_root: PathBuf,
    root_fs: FsIdentity,
    build: BuildIdentity,
    protocol: u32,
    cache_base: PathBuf,
    refresh_policy: RefreshPolicy,
}

impl DaemonIdentity {
    /// The identity `cfg` has for the running binary. Reads the filesystem and creates nothing.
    /// Only `repo_root`, `cache` and `refresh_policy` are read; `startup` and
    /// `first_call_wait` are connection-local and do not take part.
    pub fn resolve(cfg: &ServerConfig) -> Result<Self, IdentityError> {
        Self::resolve_with(cfg, BuildIdentity::current(), SHARED_PROTOCOL_VERSION)
    }

    /// [`Self::resolve`] for a given build and protocol version (what a different binary would
    /// compute for the same configuration).
    pub fn resolve_with(
        cfg: &ServerConfig,
        build: BuildIdentity,
        protocol: u32,
    ) -> Result<Self, IdentityError> {
        let root_error = |source| IdentityError::RepoRoot {
            path: cfg.repo_root.clone(),
            source,
        };
        // The same canonicalization `canonical_config` applies for the in-process server.
        let repo_root = std::fs::canonicalize(&cfg.repo_root).map_err(root_error)?;
        let root_fs = FsIdentity::of(&repo_root).map_err(root_error)?;
        let cache_base = effective_cache_base(&cfg.cache)?;
        Ok(Self {
            repo_root,
            root_fs,
            build,
            protocol,
            cache_base,
            refresh_policy: cfg.refresh_policy,
        })
    }

    /// The canonical repository root.
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// The root's device and inode when the identity was resolved. A daemon compares this with
    /// [`FsIdentity::of`] of the same path to notice that its root was lost or replaced.
    pub fn root_fs(&self) -> FsIdentity {
        self.root_fs
    }

    pub fn build(&self) -> &BuildIdentity {
        &self.build
    }

    pub fn protocol(&self) -> u32 {
        self.protocol
    }

    /// The canonical cache base directory (the value for the daemon's `--cache-dir`).
    pub fn cache_base(&self) -> &Path {
        &self.cache_base
    }

    pub fn refresh_policy(&self) -> RefreshPolicy {
        self.refresh_policy
    }

    /// The identity key: SHA-256 of [`Self::encode`].
    pub fn key(&self) -> IdentityKey {
        IdentityKey(Sha256::digest(self.encode()).into())
    }

    /// The lossless encoding the key is a digest of: ten fields in a fixed order, each written
    /// as its length (8 bytes, little-endian) followed by its bytes. Paths are their raw bytes;
    /// the device, inode and protocol numbers are fixed-width little-endian.
    fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::new();
        let mut field = |bytes: &[u8]| {
            encoded.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            encoded.extend_from_slice(bytes);
        };
        field(ENCODING_DOMAIN);
        field(self.repo_root.as_os_str().as_bytes());
        field(&self.root_fs.dev.to_le_bytes());
        field(&self.root_fs.ino.to_le_bytes());
        field(self.build.package_version.as_bytes());
        field(self.build.grammar_fingerprint.as_bytes());
        field(self.build.cache_build_identity.as_bytes());
        field(&self.protocol.to_le_bytes());
        field(self.cache_base.as_os_str().as_bytes());
        field(refresh_policy_name(self.refresh_policy).as_bytes());
        encoded
    }
}

/// The 256-bit identity key. Its text form is 64 lowercase hexadecimal digits; the first 16
/// name the daemon's files. The 16 digits locate a daemon; only the full key proves it is the
/// right one (spec §4.4).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdentityKey([u8; 32]);

/// Text that is not exactly 64 lowercase hexadecimal digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidIdentityKey;

impl fmt::Display for InvalidIdentityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "an identity key is 64 lowercase hexadecimal digits")
    }
}

impl std::error::Error for InvalidIdentityKey {}

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

fn hex_value(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}

impl IdentityKey {
    /// All 64 hexadecimal digits.
    pub fn to_hex(&self) -> String {
        let mut hex = String::with_capacity(64);
        for byte in self.0 {
            hex.push(HEX_DIGITS[usize::from(byte >> 4)] as char);
            hex.push(HEX_DIGITS[usize::from(byte & 0x0f)] as char);
        }
        hex
    }

    /// The first 16 hexadecimal digits: the stem of the daemon's file names (`key16`).
    pub fn locator(&self) -> String {
        let mut hex = self.to_hex();
        hex.truncate(16);
        hex
    }

    /// Parse the text form. Accepts exactly what [`Self::to_hex`] produces.
    pub fn from_hex(text: &str) -> Result<Self, InvalidIdentityKey> {
        let digits = text.as_bytes();
        if digits.len() != 64 {
            return Err(InvalidIdentityKey);
        }
        let mut key = [0u8; 32];
        for (byte, pair) in key.iter_mut().zip(digits.chunks_exact(2)) {
            let high = hex_value(pair[0]).ok_or(InvalidIdentityKey)?;
            let low = hex_value(pair[1]).ok_or(InvalidIdentityKey)?;
            *byte = high << 4 | low;
        }
        Ok(Self(key))
    }
}

impl fmt::Display for IdentityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl fmt::Debug for IdentityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IdentityKey({})", self.to_hex())
    }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
````

- [ ] **Step 4: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::shared::identity > /tmp/s2-t3-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s2-t3-green.log
```

Expected: `test result: ok. 21 passed; 0 failed`. `the_key_is_sha256_of_the_length_prefixed_fields` compares against a digest computed with Python's `hashlib` (the snippet is in the test's comment); if it fails, the encoding differs from decision D11, not the test.

- [ ] **Step 5: Commit**

```bash
git add src/mcp/shared/mod.rs src/mcp/shared/identity.rs src/mcp/shared/identity_tests.rs
git commit -m "feat(mcp): daemon identity key for shared mode"
```

- [ ] **Step 6: Prove the replaced-root test bites (after the commit)**

Temporarily, in `src/mcp/shared/identity.rs`, replace this:

```rust
        field(&self.root_fs.ino.to_le_bytes());
```

with this:

```rust
        field(&0u64.to_le_bytes());
```

```bash
cargo test --features mcp --lib mcp::shared::identity > /tmp/s2-t3-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s2-t3-bite.log
git checkout -- src/mcp/shared/identity.rs
```

Expected: four tests fail, among them `a_root_replaced_at_the_same_pathname_has_a_new_key` and `every_input_changes_the_key`; `17 passed; 4 failed`.

---

### Task 4: Daemon file paths and the socket path limit

**Files:**
- Create: `src/mcp/shared/paths.rs`, `src/mcp/shared/paths_tests.rs`
- Modify: `src/mcp/shared/mod.rs`

**Interfaces:**
- Consumes: `IdentityKey::locator` (Task 3); `RUNTIME_DIR_ENV` (Task 1).
- Produces (in `crate::mcp::shared::paths`):
  - `pub const MAX_SOCKET_PATH_BYTES: usize = 103;`
  - `pub struct SocketPathTooLong { pub path: PathBuf, pub len: usize }` (`Clone`, `Debug`, `PartialEq`, `Eq`, `Display`, `Error`)
  - `pub struct DaemonPaths` (`Clone`, `Debug`, `PartialEq`, `Eq`) with `pub fn new(runtime_dir: &Path, key: &IdentityKey) -> Result<Self, SocketPathTooLong>` and `runtime_dir`, `socket`, `start_lock`, `owner_lock`, `pid_file`, `log_file`, each `(&self) -> &Path`

- [ ] **Step 1: Write the tests**

Create `src/mcp/shared/paths_tests.rs`:

````rust
//! Tests for the daemon's file names and the socket path length limit.
use super::*;
use crate::mcp::shared::identity::IdentityKey;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

const KEY: &str = "0123456789abcdeffedcba98765432100123456789abcdeffedcba9876543210";

fn key(hex: &str) -> IdentityKey {
    IdentityKey::from_hex(hex).unwrap()
}

/// An absolute directory path of exactly `len` bytes (it need not exist).
fn dir_of_len(len: usize) -> PathBuf {
    let path = PathBuf::from(format!("/{}", "d".repeat(len - 1)));
    assert_eq!(path.as_os_str().len(), len);
    path
}

/// `/` + 16 hex digits + `.sock`.
const SOCKET_NAME_BYTES: usize = 1 + 16 + 5;

/// Spec §4.3 "Files".
#[test]
fn the_five_files_are_named_by_the_first_16_hex_digits_inside_the_runtime_dir() {
    let dir = Path::new("/run/user/501/prism-mcpd");
    let paths = DaemonPaths::new(dir, &key(KEY)).unwrap();
    assert_eq!(paths.runtime_dir(), dir);
    assert_eq!(paths.socket(), dir.join("0123456789abcdef.sock"));
    assert_eq!(paths.start_lock(), dir.join("0123456789abcdef.start"));
    assert_eq!(paths.owner_lock(), dir.join("0123456789abcdef.owner"));
    assert_eq!(paths.pid_file(), dir.join("0123456789abcdef.pid"));
    assert_eq!(paths.log_file(), dir.join("0123456789abcdef.log"));
}

/// The file names are a locator, not the identity: two keys that agree in their first 16
/// digits name the same files, which is why the preamble compares the full key (spec §4.4).
#[test]
fn keys_that_share_their_first_16_digits_share_their_files() {
    let dir = Path::new("/run/user/501/prism-mcpd");
    let other = format!("{}{}", &KEY[..16], "0".repeat(48));
    assert_ne!(key(KEY), key(&other));
    assert_eq!(
        DaemonPaths::new(dir, &key(KEY)).unwrap(),
        DaemonPaths::new(dir, &key(&other)).unwrap()
    );

    let different = format!("f{}", &KEY[1..]);
    assert_ne!(
        DaemonPaths::new(dir, &key(KEY)).unwrap().socket(),
        DaemonPaths::new(dir, &key(&different)).unwrap().socket()
    );
}

/// Spec §4.3: the socket path must be at most 103 bytes. Rules out an off-by-one in either
/// direction.
#[test]
fn a_socket_path_of_103_bytes_fits_and_104_does_not() {
    let fits = DaemonPaths::new(&dir_of_len(103 - SOCKET_NAME_BYTES), &key(KEY)).unwrap();
    assert_eq!(fits.socket().as_os_str().len(), 103);

    let dir = dir_of_len(104 - SOCKET_NAME_BYTES);
    let outcome = DaemonPaths::new(&dir, &key(KEY));
    match outcome {
        Err(SocketPathTooLong { path, len }) => {
            assert_eq!(len, 104);
            assert_eq!(path, dir.join("0123456789abcdef.sock"));
        }
        other => panic!("{other:?}"),
    }
}

/// Rules out counting characters: this path is 97 characters and 111 bytes.
#[test]
fn the_limit_counts_bytes_not_characters() {
    let dir = PathBuf::from(format!("/{}", "é".repeat(14) + &"d".repeat(60)));
    let socket_chars = dir.to_str().unwrap().chars().count() + SOCKET_NAME_BYTES;
    assert!(socket_chars <= 103, "{socket_chars} characters");
    let outcome = DaemonPaths::new(&dir, &key(KEY));
    assert!(
        matches!(&outcome, Err(SocketPathTooLong { len, .. }) if *len == socket_chars + 14),
        "{outcome:?}"
    );
}

/// The limit is the kernel's, not a preference: a path of exactly 103 bytes can be bound on
/// every supported platform (`sun_path` holds 104 bytes on Darwin, 108 on Linux, including
/// the terminating NUL). The directory is made under `/tmp` so its length does not depend on
/// `$TMPDIR`.
#[test]
fn a_socket_can_really_be_bound_at_the_limit() {
    let base = tempfile::Builder::new()
        .prefix("s2")
        .tempdir_in("/tmp")
        .unwrap();
    let base_len = base.path().as_os_str().as_bytes().len();
    let padding = 103 - SOCKET_NAME_BYTES - base_len - 1;
    let dir = base.path().join("p".repeat(padding));
    std::fs::create_dir(&dir).unwrap();

    let paths = DaemonPaths::new(&dir, &key(KEY)).unwrap();
    assert_eq!(paths.socket().as_os_str().as_bytes().len(), 103);
    let listener = std::os::unix::net::UnixListener::bind(paths.socket()).expect("bind at 103");
    drop(listener);
}

#[test]
fn the_error_states_the_length_the_limit_and_the_way_out() {
    let error = DaemonPaths::new(&dir_of_len(120), &key(KEY)).unwrap_err();
    let text = error.to_string();
    assert!(text.contains("142 bytes"), "{text}");
    assert!(text.contains("103"), "{text}");
    assert!(text.contains("0123456789abcdef.sock"), "{text}");
    assert!(text.contains("PRISM_MCP_RUNTIME_DIR"), "{text}");
    assert!(!text.contains('\n'), "one line: {text}");
}
````

Create `src/mcp/shared/paths.rs`:

```rust
#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
```

In `src/mcp/shared/mod.rs`, replace this:

```rust
pub mod identity;
pub mod runtime_dir;
```

with this:

```rust
pub mod identity;
pub mod paths;
pub mod runtime_dir;
```

- [ ] **Step 2: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::shared::paths > /tmp/s2-t4-red.log 2>&1; grep -cE '^error\[' /tmp/s2-t4-red.log; tail -1 /tmp/s2-t4-red.log
```

Expected: a count of 12 (`use of undeclared type DaemonPaths`, `SocketPathTooLong`) and `error: could not compile`.

- [ ] **Step 3: Implement**

Replace the whole of `src/mcp/shared/paths.rs` with:

````rust
//! The files of one daemon inside the runtime directory (spec §4.3): the socket, the two lock
//! files, the pid file and the log, all named by the first 16 hex digits of the identity key.
//! Only names are computed here; nothing is created, opened or removed.
use super::identity::IdentityKey;
use super::runtime_dir::RUNTIME_DIR_ENV;
use std::fmt;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// Longest socket path, in bytes, that every supported platform can bind: `sun_path` holds 104
/// bytes on Darwin including the terminating NUL (spec §4.3).
pub const MAX_SOCKET_PATH_BYTES: usize = 103;

/// The socket path for this runtime directory does not fit in a socket address. The shim fails
/// closed (spec §4.1 step 4); a shorter `PRISM_MCP_RUNTIME_DIR` is the way out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketPathTooLong {
    pub path: PathBuf,
    /// Length of `path` in bytes.
    pub len: usize,
}

impl fmt::Display for SocketPathTooLong {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "daemon socket path {} is {} bytes; a Unix socket path can be at most \
             {MAX_SOCKET_PATH_BYTES} (set {RUNTIME_DIR_ENV} to a shorter absolute directory)",
            self.path.display(),
            self.len
        )
    }
}

impl std::error::Error for SocketPathTooLong {}

/// Where one daemon's files are. Two keys with the same first 16 hex digits have the same
/// paths: the paths locate a daemon, and the preamble's full-key comparison identifies it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonPaths {
    runtime_dir: PathBuf,
    socket: PathBuf,
    start_lock: PathBuf,
    owner_lock: PathBuf,
    pid_file: PathBuf,
    log_file: PathBuf,
}

impl DaemonPaths {
    /// The paths for `key` under `runtime_dir`, or an error when the socket path is longer
    /// than [`MAX_SOCKET_PATH_BYTES`]. `runtime_dir` is used as given; validating it is the
    /// caller's separate step (`validate_runtime_dir`).
    pub fn new(runtime_dir: &Path, key: &IdentityKey) -> Result<Self, SocketPathTooLong> {
        let locator = key.locator();
        let file = |extension: &str| runtime_dir.join(format!("{locator}.{extension}"));
        let socket = file("sock");
        let len = socket.as_os_str().as_bytes().len();
        if len > MAX_SOCKET_PATH_BYTES {
            return Err(SocketPathTooLong { path: socket, len });
        }
        Ok(Self {
            runtime_dir: runtime_dir.to_path_buf(),
            socket,
            start_lock: file("start"),
            owner_lock: file("owner"),
            pid_file: file("pid"),
            log_file: file("log"),
        })
    }

    pub fn runtime_dir(&self) -> &Path {
        &self.runtime_dir
    }

    /// `<key16>.sock`: the daemon's listening socket.
    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// `<key16>.start`: held by a shim while it arbitrates who starts the daemon.
    pub fn start_lock(&self) -> &Path {
        &self.start_lock
    }

    /// `<key16>.owner`: held by the daemon for its whole life.
    pub fn owner_lock(&self) -> &Path {
        &self.owner_lock
    }

    /// `<key16>.pid`: pid, start time and build identity of the daemon.
    pub fn pid_file(&self) -> &Path {
        &self.pid_file
    }

    /// `<key16>.log`: the daemon's stdout and stderr.
    pub fn log_file(&self) -> &Path {
        &self.log_file
    }
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
````

- [ ] **Step 4: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::shared::paths > /tmp/s2-t4-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s2-t4-green.log
```

Expected: `test result: ok. 6 passed; 0 failed`. `a_socket_can_really_be_bound_at_the_limit` needs a writable `/tmp`.

- [ ] **Step 5: Commit**

```bash
git add src/mcp/shared/mod.rs src/mcp/shared/paths.rs src/mcp/shared/paths_tests.rs
git commit -m "feat(mcp): daemon file paths and the socket path limit"
```

---

### Task 5: Per-client settings

The settings that travel in the preamble, behind one constructor that refuses what a server must not run with.

**Files:**
- Modify: `src/mcp/settings.rs`
- Create: `src/mcp/settings_tests.rs`

**Interfaces:**
- Consumes: `ConnectionSettings` (S1, unchanged); `MAX_RESULT_CHARS_FLOOR` (`output.rs`, 12,000); `FIRST_CALL_WAIT_MAX` (`session.rs`, 600 s); in tests, `resolve_cap_from`, `transport::serve_connection`, `framing::InMemoryTransport`, `SessionProvider::bootstrap`.
- Produces (in `crate::mcp::settings`):
  - `pub enum SettingsError { ResultCapBelowFloor { cap: usize, floor: usize }, FirstCallWaitTooLong { secs: u64, max_secs: u64 } }` (`Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `Display`, `Error`)
  - `pub struct ClientSettings` (private fields; `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`) with `pub fn new(output: ConnectionSettings, first_call_wait_secs: u64, warm_at_startup: bool) -> Result<Self, SettingsError>`, `pub fn output(&self) -> &ConnectionSettings`, `pub fn first_call_wait(&self) -> Duration`, `pub fn first_call_wait_secs(&self) -> u64`, `pub fn warm_at_startup(&self) -> bool`

- [ ] **Step 1: Write the tests**

Create `src/mcp/settings_tests.rs`:

````rust
//! Tests for the settings a client may ask a shared daemon for. They arrive from another
//! process, so every bound is enforced where the value is constructed.
use super::*;
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::{
    resolve_cap_from, StructuredContentMode, MAX_RESULT_CHARS, MAX_RESULT_CHARS_FLOOR,
};
use crate::mcp::FIRST_CALL_WAIT_MAX;
use std::time::{Duration, Instant};

fn output(result_cap: usize) -> ConnectionSettings {
    ConnectionSettings {
        result_cap,
        structured_content: StructuredContentMode::OmitDefaultPath,
        concise_shape: ConciseShapeMode::Slim,
    }
}

/// A cap below the floor reaching dispatch trips `cap_after_reserve`'s debug assertion (a
/// panic in dispatch is fatal to a daemon) and silently saturates in release. Rules out
/// carrying the client's number through unchecked.
#[test]
fn a_result_cap_below_the_floor_is_refused() {
    for cap in [0, 1, 100, MAX_RESULT_CHARS_FLOOR - 1] {
        assert_eq!(
            ClientSettings::new(output(cap), 20, false),
            Err(SettingsError::ResultCapBelowFloor {
                cap,
                floor: MAX_RESULT_CHARS_FLOOR
            }),
            "cap {cap}"
        );
    }
}

#[test]
fn a_result_cap_at_or_above_the_floor_is_kept_exactly() {
    for cap in [
        MAX_RESULT_CHARS_FLOOR,
        MAX_RESULT_CHARS_FLOOR + 1,
        MAX_RESULT_CHARS,
        usize::MAX,
    ] {
        let settings = ClientSettings::new(output(cap), 20, false).expect("accepted");
        assert_eq!(settings.output().result_cap, cap);
    }
}

/// One acceptance rule for a cap, wherever it comes from: this constructor accepts exactly
/// the caps `resolve_cap_from` keeps (the environment path replaces the others with the
/// default and warns; a decoded cap is refused instead, because a daemon has no stderr the
/// client can see).
#[test]
fn the_constructor_accepts_exactly_the_caps_the_environment_resolver_keeps() {
    for cap in [
        0,
        1,
        MAX_RESULT_CHARS_FLOOR - 1,
        MAX_RESULT_CHARS_FLOOR,
        MAX_RESULT_CHARS_FLOOR + 1,
        MAX_RESULT_CHARS - 1,
        MAX_RESULT_CHARS + 1,
    ] {
        let kept_by_environment = resolve_cap_from(Some(&cap.to_string())) == cap;
        assert_eq!(
            ClientSettings::new(output(cap), 20, false).is_ok(),
            kept_by_environment,
            "cap {cap}"
        );
    }
}

/// The smallest cap the constructor accepts is usable where dispatch subtracts the most from
/// it: a query on a stale index under an automatic refresh policy pays both the auto-refresh
/// and the freshness reserve. In a debug build this is the path whose assertion a smaller cap
/// trips; here it must answer, within the cap.
#[test]
fn the_smallest_accepted_cap_serves_the_query_that_reserves_the_most() {
    use crate::mcp::framing::InMemoryTransport;
    use crate::mcp::registry::ToolRegistry;
    use crate::mcp::{CacheMode, RefreshPolicy, ServerConfig, SessionProvider, StartupMode};

    let repo = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join("util.py"), "def helper():\n    return 1\n").unwrap();
    std::fs::write(
        repo.path().join("main.py"),
        "from util import helper\n\ndef run():\n    return helper()\n",
    )
    .unwrap();
    let mut cfg = ServerConfig::new(repo.path().to_path_buf());
    cfg.cache = CacheMode::NoCache;
    cfg.refresh_policy = RefreshPolicy::AutoFull;
    cfg.startup = StartupMode::Eager;
    let mut provider = SessionProvider::bootstrap(&cfg).unwrap();
    // Make the index stale, so the call takes the refresh path.
    std::fs::write(
        repo.path().join("util.py"),
        "def helper():\n    return 1\n\n\ndef extra():\n    return helper()\n",
    )
    .unwrap();

    let settings = ClientSettings::new(output(MAX_RESULT_CHARS_FLOOR), 20, false).unwrap();
    let mut transport = InMemoryTransport::new(vec![
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_callers","arguments":{"seed":{"kind":"symbol","name":"helper","file":"util.py"}}}}"#,
    ]);
    crate::mcp::transport::serve_connection(
        &mut provider,
        &ToolRegistry::all_v1(),
        &mut transport,
        settings.output(),
    )
    .unwrap();

    let response = &transport.responses()[1];
    assert_eq!(response["result"]["isError"], false, "{response}");
    assert_eq!(
        response["result"]["_meta"]["prism/auto_refresh"], "refreshed",
        "the call went through the refresh path: {response}"
    );
    assert!(response.to_string().len() <= MAX_RESULT_CHARS_FLOOR);
}

/// Rules out an unbounded wait: a daemon adds it to an `Instant`, and an overflow there is a
/// panic.
#[test]
fn a_first_call_wait_above_the_command_line_maximum_is_refused() {
    let max_secs = FIRST_CALL_WAIT_MAX.as_secs();
    assert_eq!(max_secs, 600, "the `--first-call-wait` range is 0..=600");
    for secs in [max_secs + 1, 86_400, u64::MAX] {
        assert_eq!(
            ClientSettings::new(output(MAX_RESULT_CHARS), secs, false),
            Err(SettingsError::FirstCallWaitTooLong { secs, max_secs }),
            "{secs} s"
        );
    }
    for secs in [0, 1, 20, max_secs] {
        let settings = ClientSettings::new(output(MAX_RESULT_CHARS), secs, false).expect("ok");
        assert_eq!(settings.first_call_wait_secs(), secs);
        assert_eq!(settings.first_call_wait(), Duration::from_secs(secs));
        assert!(Instant::now()
            .checked_add(settings.first_call_wait())
            .is_some());
    }
}

#[test]
fn the_settings_are_returned_as_given() {
    let always_legacy = ConnectionSettings {
        result_cap: 20_000,
        structured_content: StructuredContentMode::Always,
        concise_shape: ConciseShapeMode::Legacy,
    };
    let settings = ClientSettings::new(always_legacy, 7, true).unwrap();
    assert_eq!(settings.output(), &always_legacy);
    assert_eq!(settings.first_call_wait_secs(), 7);
    assert!(settings.warm_at_startup());
    assert!(!ClientSettings::new(always_legacy, 7, false)
        .unwrap()
        .warm_at_startup());
}

#[test]
fn each_refusal_says_what_was_asked_and_what_is_allowed() {
    let cap = SettingsError::ResultCapBelowFloor {
        cap: 100,
        floor: MAX_RESULT_CHARS_FLOOR,
    }
    .to_string();
    assert!(cap.contains("100") && cap.contains("12000"), "{cap}");
    let wait = SettingsError::FirstCallWaitTooLong {
        secs: 601,
        max_secs: 600,
    }
    .to_string();
    assert!(wait.contains("601") && wait.contains("600"), "{wait}");
}
````

Append to `src/mcp/settings.rs`:

```rust

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
```

- [ ] **Step 2: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::settings > /tmp/s2-t5-red.log 2>&1; grep -cE '^error\[' /tmp/s2-t5-red.log; tail -1 /tmp/s2-t5-red.log
```

Expected: a count of 10 or more (`use of undeclared type ClientSettings`, `SettingsError`) and `error: could not compile`.

- [ ] **Step 3: Implement**

`ConnectionSettings` and its `impl` stay exactly as they are. Two edits.

In `src/mcp/settings.rs`, replace this:

```rust
use super::concise_shape::{resolve_concise_shape_mode, ConciseShapeMode};
use super::output::{resolve_cap, resolve_structured_content_mode, StructuredContentMode};
```

with this:

```rust
use super::concise_shape::{resolve_concise_shape_mode, ConciseShapeMode};
use super::output::{
    resolve_cap, resolve_structured_content_mode, StructuredContentMode, MAX_RESULT_CHARS_FLOOR,
};
use super::FIRST_CALL_WAIT_MAX;
use std::fmt;
use std::time::Duration;
```

In `src/mcp/settings.rs`, replace this:

```rust
#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
```

with this:

````rust
/// A setting a client asked for that a server must not run with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsError {
    /// The result cap is below `MAX_RESULT_CHARS_FLOOR`, the same floor
    /// `PRISM_MCP_MAX_RESULT_CHARS` has.
    ResultCapBelowFloor { cap: usize, floor: usize },
    /// The first-call wait is above `FIRST_CALL_WAIT_MAX`, the `--first-call-wait` maximum.
    FirstCallWaitTooLong { secs: u64, max_secs: u64 },
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResultCapBelowFloor { cap, floor } => {
                write!(f, "result cap {cap} is below the floor of {floor}")
            }
            Self::FirstCallWaitTooLong { secs, max_secs } => write!(
                f,
                "first-call wait of {secs} s is above the maximum of {max_secs} s"
            ),
        }
    }
}

impl std::error::Error for SettingsError {}

/// Everything one client of a shared daemon chooses for its own connection (spec §4.1 step 3,
/// §4.4): the output settings dispatch reads, plus the two that only the readiness path reads.
/// The shim resolves it from its own environment and flags and sends it in the preamble; it is
/// never inherited from the client that started the daemon.
///
/// [`ClientSettings::new`] is the only way to make one, and the fields are private, so a value
/// of this type always holds a result cap dispatch can subtract its reserves from and a wait
/// that can be added to an `Instant`. A daemon builds each connection's `ConnectionSettings`
/// from [`ClientSettings::output`] and from nowhere else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientSettings {
    output: ConnectionSettings,
    first_call_wait_secs: u64,
    warm_at_startup: bool,
}

impl ClientSettings {
    /// Check the two bounded settings and keep the rest as given. `first_call_wait_secs` is
    /// `--first-call-wait`; `warm_at_startup` is `--warm-at-startup`.
    pub fn new(
        output: ConnectionSettings,
        first_call_wait_secs: u64,
        warm_at_startup: bool,
    ) -> Result<Self, SettingsError> {
        if output.result_cap < MAX_RESULT_CHARS_FLOOR {
            return Err(SettingsError::ResultCapBelowFloor {
                cap: output.result_cap,
                floor: MAX_RESULT_CHARS_FLOOR,
            });
        }
        let max_secs = FIRST_CALL_WAIT_MAX.as_secs();
        if first_call_wait_secs > max_secs {
            return Err(SettingsError::FirstCallWaitTooLong {
                secs: first_call_wait_secs,
                max_secs,
            });
        }
        Ok(Self {
            output,
            first_call_wait_secs,
            warm_at_startup,
        })
    }

    /// The output settings for this connection's dispatch.
    pub fn output(&self) -> &ConnectionSettings {
        &self.output
    }

    /// How long this connection's first readiness-requiring call may wait for the index.
    pub fn first_call_wait(&self) -> Duration {
        Duration::from_secs(self.first_call_wait_secs)
    }

    pub fn first_call_wait_secs(&self) -> u64 {
        self.first_call_wait_secs
    }

    /// Whether this client asked for the index to be built at daemon start. Only the daemon
    /// this client starts honors it (spec §8).
    pub fn warm_at_startup(&self) -> bool {
        self.warm_at_startup
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
````

For reference, the whole file after both edits:

````rust
//! Output settings of one MCP connection. The in-process server resolves them once from its own
//! environment; the shared daemon builds one per connection from the shim's preamble, so a
//! client's output shape never depends on which client started the daemon.
use super::concise_shape::{resolve_concise_shape_mode, ConciseShapeMode};
use super::output::{
    resolve_cap, resolve_structured_content_mode, StructuredContentMode, MAX_RESULT_CHARS_FLOOR,
};
use super::FIRST_CALL_WAIT_MAX;
use std::fmt;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConnectionSettings {
    /// Wire byte cap per tool result (`PRISM_MCP_MAX_RESULT_CHARS`).
    pub result_cap: usize,
    /// Whether the default path carries `structuredContent` (`PRISM_MCP_STRUCTURED_CONTENT`).
    pub structured_content: StructuredContentMode,
    /// Item shape of concise results (`PRISM_MCP_CONCISE_SHAPE`).
    pub concise_shape: ConciseShapeMode,
}

impl ConnectionSettings {
    /// Resolve from this process's environment. Each invalid value warns on stderr when it is
    /// resolved, so call this once per connection, not once per request.
    pub fn from_env() -> Self {
        Self {
            result_cap: resolve_cap(),
            structured_content: resolve_structured_content_mode(),
            concise_shape: resolve_concise_shape_mode(),
        }
    }
}

/// A setting a client asked for that a server must not run with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsError {
    /// The result cap is below `MAX_RESULT_CHARS_FLOOR`, the same floor
    /// `PRISM_MCP_MAX_RESULT_CHARS` has.
    ResultCapBelowFloor { cap: usize, floor: usize },
    /// The first-call wait is above `FIRST_CALL_WAIT_MAX`, the `--first-call-wait` maximum.
    FirstCallWaitTooLong { secs: u64, max_secs: u64 },
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResultCapBelowFloor { cap, floor } => {
                write!(f, "result cap {cap} is below the floor of {floor}")
            }
            Self::FirstCallWaitTooLong { secs, max_secs } => write!(
                f,
                "first-call wait of {secs} s is above the maximum of {max_secs} s"
            ),
        }
    }
}

impl std::error::Error for SettingsError {}

/// Everything one client of a shared daemon chooses for its own connection (spec §4.1 step 3,
/// §4.4): the output settings dispatch reads, plus the two that only the readiness path reads.
/// The shim resolves it from its own environment and flags and sends it in the preamble; it is
/// never inherited from the client that started the daemon.
///
/// [`ClientSettings::new`] is the only way to make one, and the fields are private, so a value
/// of this type always holds a result cap dispatch can subtract its reserves from and a wait
/// that can be added to an `Instant`. A daemon builds each connection's `ConnectionSettings`
/// from [`ClientSettings::output`] and from nowhere else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientSettings {
    output: ConnectionSettings,
    first_call_wait_secs: u64,
    warm_at_startup: bool,
}

impl ClientSettings {
    /// Check the two bounded settings and keep the rest as given. `first_call_wait_secs` is
    /// `--first-call-wait`; `warm_at_startup` is `--warm-at-startup`.
    pub fn new(
        output: ConnectionSettings,
        first_call_wait_secs: u64,
        warm_at_startup: bool,
    ) -> Result<Self, SettingsError> {
        if output.result_cap < MAX_RESULT_CHARS_FLOOR {
            return Err(SettingsError::ResultCapBelowFloor {
                cap: output.result_cap,
                floor: MAX_RESULT_CHARS_FLOOR,
            });
        }
        let max_secs = FIRST_CALL_WAIT_MAX.as_secs();
        if first_call_wait_secs > max_secs {
            return Err(SettingsError::FirstCallWaitTooLong {
                secs: first_call_wait_secs,
                max_secs,
            });
        }
        Ok(Self {
            output,
            first_call_wait_secs,
            warm_at_startup,
        })
    }

    /// The output settings for this connection's dispatch.
    pub fn output(&self) -> &ConnectionSettings {
        &self.output
    }

    /// How long this connection's first readiness-requiring call may wait for the index.
    pub fn first_call_wait(&self) -> Duration {
        Duration::from_secs(self.first_call_wait_secs)
    }

    pub fn first_call_wait_secs(&self) -> u64 {
        self.first_call_wait_secs
    }

    /// Whether this client asked for the index to be built at daemon start. Only the daemon
    /// this client starts honors it (spec §8).
    pub fn warm_at_startup(&self) -> bool {
        self.warm_at_startup
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
````

- [ ] **Step 4: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::settings > /tmp/s2-t5-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s2-t5-green.log
git diff origin/main -- src/mcp/settings.rs | grep -E '^-[^-]'
```

Expected: `test result: ok. 7 passed; 0 failed`. The last command prints exactly one removed line, the old `use super::output::{…}` import: nothing else in the existing file changed.

- [ ] **Step 5: Commit**

```bash
git add src/mcp/settings.rs src/mcp/settings_tests.rs
git commit -m "feat(mcp): validated per-client settings for shared mode"
```

---

### Task 6: Preamble encode and decode

**Files:**
- Create: `src/mcp/shared/preamble.rs`, `src/mcp/shared/preamble_tests.rs`, `src/mcp/shared/preamble_reply_tests.rs`
- Modify: `src/mcp/shared/mod.rs`

**Interfaces:**
- Consumes: `IdentityKey` (Task 3); `SHARED_PROTOCOL_VERSION` (Task 3); `ClientSettings`, `ConnectionSettings`, `SettingsError` (Task 5); `StructuredContentMode`, `ConciseShapeMode`.
- Produces (in `crate::mcp::shared::preamble`):
  - `pub const PREAMBLE_MAX_BYTES: usize = 4096;`
  - `pub enum PreambleError { TooLong { len: usize }, NotUtf8, Malformed, UnknownVerb { verb: String }, NotJson { detail: String }, UnsupportedProtocol { got: u64 }, Settings(SettingsError), Invalid { detail: String } }` (`Clone`, `Debug`, `PartialEq`, `Eq`, `Display` on one line of at most 400 bytes, `Error`)
  - `pub enum BuildState { Idle, Building, Ready, Failed }`
  - `pub struct Hello { pub identity: IdentityKey, pub settings: ClientSettings }` with `pub fn encode(&self) -> String` (newline included), `pub fn decode(line: &[u8]) -> Result<Self, PreambleError>` (line without its newline), `pub fn mismatch_against(&self, served: &IdentityKey) -> Option<Reply>`
  - `pub struct Ready { pub identity: IdentityKey, pub pid: u32, pub start_time: u64, pub generation: u64, pub build: BuildState }`
  - `pub enum Reply { Ready(Ready), Retiring, Busy { max_clients: u32 }, Mismatch { expected: IdentityKey } }` with `pub fn encode(&self) -> String`, `pub fn decode(line: &[u8]) -> Result<Self, PreambleError>`
  - All four types are `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`.

- [ ] **Step 1: Write the tests**

Create `src/mcp/shared/preamble_tests.rs`:

````rust
//! Tests for `HELLO`, the line a daemon receives: its exact wire text, the round trip, every
//! way a line from another process can be wrong, and the identity comparison. The replies'
//! tests are in `preamble_reply_tests.rs`.
use super::*;
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::{StructuredContentMode, MAX_RESULT_CHARS_FLOOR};
use crate::mcp::settings::{ClientSettings, ConnectionSettings, SettingsError};
use crate::mcp::shared::identity::IdentityKey;

const KEY: &str = "0123456789abcdeffedcba98765432100123456789abcdeffedcba9876543210";
/// Same first 16 digits as `KEY`, so the same socket file; a different identity.
const NEIGHBOUR: &str = "0123456789abcdef000000000000000000000000000000000000000000000000";

/// The `HELLO` line of spec §4.4 for the default settings, without its newline.
const HELLO_LINE: &str = r#"HELLO {"protocol":1,"identity":"0123456789abcdeffedcba98765432100123456789abcdeffedcba9876543210","settings":{"structured_content":"omit-default-path","concise_shape":"slim","result_cap":80000,"first_call_wait_secs":20,"warm_at_startup":false}}"#;

fn key(hex: &str) -> IdentityKey {
    IdentityKey::from_hex(hex).unwrap()
}

fn settings(
    structured_content: StructuredContentMode,
    concise_shape: ConciseShapeMode,
    result_cap: usize,
    first_call_wait_secs: u64,
    warm_at_startup: bool,
) -> ClientSettings {
    ClientSettings::new(
        ConnectionSettings {
            result_cap,
            structured_content,
            concise_shape,
        },
        first_call_wait_secs,
        warm_at_startup,
    )
    .unwrap()
}

fn default_hello() -> Hello {
    Hello {
        identity: key(KEY),
        settings: settings(
            StructuredContentMode::OmitDefaultPath,
            ConciseShapeMode::Slim,
            80_000,
            20,
            false,
        ),
    }
}

fn invalid_detail(outcome: Result<Hello, PreambleError>) -> String {
    match outcome {
        Err(PreambleError::Invalid { detail }) => detail,
        other => panic!("expected Invalid, got {other:?}"),
    }
}

// ── Wire text ──

#[test]
fn hello_is_written_exactly_as_the_spec_shows_it() {
    assert_eq!(default_hello().encode(), format!("{HELLO_LINE}\n"));
}

#[test]
fn the_settings_use_the_same_words_as_the_environment_variables() {
    use crate::mcp::concise_shape::resolve_concise_shape_mode_from;
    use crate::mcp::output::resolve_structured_content_mode_from;
    for mode in [
        StructuredContentMode::Always,
        StructuredContentMode::OmitDefaultPath,
    ] {
        let word = structured_content_word(mode);
        assert_eq!(resolve_structured_content_mode_from(Some(word)), mode);
    }
    for mode in [ConciseShapeMode::Legacy, ConciseShapeMode::Slim] {
        let word = concise_shape_word(mode);
        assert_eq!(resolve_concise_shape_mode_from(Some(word)), mode);
    }
}

// ── Round trips ──

/// Spec §9 "preamble encode/decode": every combination of the settings survives encode then
/// decode, including the extremes of the two numeric settings.
#[test]
fn every_hello_survives_encode_then_decode() {
    for structured_content in [
        StructuredContentMode::Always,
        StructuredContentMode::OmitDefaultPath,
    ] {
        for concise_shape in [ConciseShapeMode::Legacy, ConciseShapeMode::Slim] {
            for (result_cap, wait) in [(MAX_RESULT_CHARS_FLOOR, 0), (80_000, 20), (usize::MAX, 600)]
            {
                for warm_at_startup in [false, true] {
                    let hello = Hello {
                        identity: key(NEIGHBOUR),
                        settings: settings(
                            structured_content,
                            concise_shape,
                            result_cap,
                            wait,
                            warm_at_startup,
                        ),
                    };
                    let line = hello.encode();
                    assert!(line.ends_with('\n') && line.len() <= PREAMBLE_MAX_BYTES);
                    let decoded = Hello::decode(line.trim_end_matches('\n').as_bytes());
                    assert_eq!(decoded, Ok(hello), "{line}");
                }
            }
        }
    }
}

// ── What a daemon must refuse in a HELLO ──

/// Rules out decoding first and measuring afterwards, and an off-by-one at the limit. The
/// oversized line is well-formed apart from its length, so only the length can refuse it.
#[test]
fn a_line_over_4_kib_is_refused_for_its_length_alone() {
    let pad_to = |len: usize| {
        let mut line = HELLO_LINE.as_bytes().to_vec();
        line.resize(len, b' ');
        line
    };
    assert_eq!(
        Hello::decode(&pad_to(PREAMBLE_MAX_BYTES + 1)),
        Err(PreambleError::TooLong {
            len: PREAMBLE_MAX_BYTES + 1
        })
    );
    // At the limit the length is fine; the padding is what is wrong with this one.
    let at_limit = Hello::decode(&pad_to(PREAMBLE_MAX_BYTES));
    assert!(
        matches!(at_limit, Err(PreambleError::Invalid { .. })),
        "{at_limit:?}"
    );
    assert_eq!(PREAMBLE_MAX_BYTES, 4096);
}

#[test]
fn bytes_that_are_not_utf8_are_refused() {
    let mut line = HELLO_LINE.as_bytes().to_vec();
    line[20] = 0xff;
    assert_eq!(Hello::decode(&line), Err(PreambleError::NotUtf8));
    assert_eq!(Hello::decode(b"\xff\xfe"), Err(PreambleError::NotUtf8));
}

#[test]
fn a_line_that_is_not_a_verb_and_a_payload_is_refused() {
    for line in ["", "HELLO", "HELLO{}", "\tHELLO {}"] {
        let outcome = Hello::decode(line.as_bytes());
        assert!(
            matches!(
                outcome,
                Err(PreambleError::Malformed | PreambleError::UnknownVerb { .. })
            ),
            "{line:?}: {outcome:?}"
        );
    }
}

/// An MCP client that connects to the socket directly, or a reply sent in the wrong direction,
/// is answered as an unknown verb, and a long first word is not echoed back in full.
#[test]
fn a_verb_the_receiver_does_not_expect_is_refused() {
    let jsonrpc = r#"{"jsonrpc":"2.0","id":1,"method": "initialize"}"#;
    for line in [
        "READY {\"protocol\":1}",
        "RETIRING {}",
        "hello {}",
        "HELLO2 {}",
        " HELLO {}",
        jsonrpc,
    ] {
        let outcome = Hello::decode(line.as_bytes());
        assert!(
            matches!(&outcome, Err(PreambleError::UnknownVerb { verb }) if verb.len() <= 16),
            "{line:?}: {outcome:?}"
        );
    }
}

#[test]
fn a_payload_that_is_not_json_is_refused() {
    for payload in [
        "{",
        "{\"protocol\":1",
        "{'protocol':1}",
        "{\"protocol\":1} {}",
        "{\"protocol\":1}}",
    ] {
        let line = format!("HELLO {payload}");
        let outcome = Hello::decode(line.as_bytes());
        assert!(
            matches!(outcome, Err(PreambleError::NotJson { .. })),
            "{line:?}: {outcome:?}"
        );
    }
}

/// The payload is a JSON object and nothing else. Rules out a decoder that takes the
/// positional array form its JSON library would also accept for a struct.
#[test]
fn a_payload_that_is_not_a_json_object_is_refused() {
    let positional = format!(r#"[1,"{KEY}",["omit-default-path","slim",80000,20,false]]"#);
    for payload in [
        "",
        "not json",
        "[]",
        "5",
        "null",
        "\"HELLO\"",
        " {}",
        &positional,
    ] {
        let line = format!("HELLO {payload}");
        let detail = invalid_detail(Hello::decode(line.as_bytes()));
        assert!(detail.contains("JSON object"), "{line:?}: {detail}");
    }
}

/// Each of the eight fields, removed one at a time.
#[test]
fn a_hello_with_any_field_missing_is_refused_and_the_field_is_named() {
    let key_field = format!(r#""identity":"{KEY}","#);
    let removals = [
        ("identity", key_field.as_str()),
        (
            "settings",
            r#","settings":{"structured_content":"omit-default-path","concise_shape":"slim","result_cap":80000,"first_call_wait_secs":20,"warm_at_startup":false}"#,
        ),
        (
            "structured_content",
            r#""structured_content":"omit-default-path","#,
        ),
        ("concise_shape", r#""concise_shape":"slim","#),
        ("result_cap", r#""result_cap":80000,"#),
        ("first_call_wait_secs", r#""first_call_wait_secs":20,"#),
        ("warm_at_startup", r#","warm_at_startup":false"#),
    ];
    for (field, text) in removals {
        assert_eq!(HELLO_LINE.matches(text).count(), 1, "{field}");
        let line = HELLO_LINE.replace(text, "");
        let detail = invalid_detail(Hello::decode(line.as_bytes()));
        assert!(
            detail.contains(&format!("missing field `{field}`")),
            "{field}: {detail}"
        );
    }
    let no_protocol = HELLO_LINE.replace(r#""protocol":1,"#, "");
    let detail = invalid_detail(Hello::decode(no_protocol.as_bytes()));
    assert!(detail.contains("missing field `protocol`"), "{detail}");
}

/// A version this build does not speak. The version is read before anything else, because
/// another version's payload may have another shape: the last two lines are nonsense as
/// version 1 and must still be reported as a version difference, not as a missing field.
#[test]
fn a_hello_of_another_protocol_version_is_refused_as_such() {
    for (got, line) in [
        (
            2,
            HELLO_LINE.replace(r#""protocol":1,"#, r#""protocol":2,"#),
        ),
        (
            0,
            HELLO_LINE.replace(r#""protocol":1,"#, r#""protocol":0,"#),
        ),
        (2, r#"HELLO {"protocol":2}"#.to_string()),
        (
            7,
            r#"HELLO {"protocol":7,"id":"k","opts":{"result_cap":1}}"#.to_string(),
        ),
    ] {
        assert_eq!(
            Hello::decode(line.as_bytes()),
            Err(PreambleError::UnsupportedProtocol { got }),
            "{line}"
        );
    }
    for bad in [
        r#""protocol":"1","#,
        r#""protocol":-1,"#,
        r#""protocol":1.5,"#,
    ] {
        let line = HELLO_LINE.replace(r#""protocol":1,"#, bad);
        let outcome = Hello::decode(line.as_bytes());
        assert!(
            matches!(outcome, Err(PreambleError::Invalid { .. })),
            "{line}: {outcome:?}"
        );
    }
}

/// A result cap below the floor, and the other bounded setting. Rules out copying
/// the wire numbers into settings without the validating constructor.
#[test]
fn a_hello_asking_for_settings_a_server_must_not_run_with_is_refused() {
    for cap in [0, 100, MAX_RESULT_CHARS_FLOOR - 1] {
        let line = HELLO_LINE.replace("\"result_cap\":80000", &format!("\"result_cap\":{cap}"));
        assert_eq!(
            Hello::decode(line.as_bytes()),
            Err(PreambleError::Settings(
                SettingsError::ResultCapBelowFloor {
                    cap,
                    floor: MAX_RESULT_CHARS_FLOOR
                }
            )),
            "{line}"
        );
    }
    let at_floor = HELLO_LINE.replace("\"result_cap\":80000", "\"result_cap\":12000");
    let decoded = Hello::decode(at_floor.as_bytes()).expect("the floor itself is allowed");
    assert_eq!(decoded.settings.output().result_cap, 12_000);

    for secs in [601_u64, u64::MAX] {
        let line = HELLO_LINE.replace(
            "\"first_call_wait_secs\":20",
            &format!("\"first_call_wait_secs\":{secs}"),
        );
        assert_eq!(
            Hello::decode(line.as_bytes()),
            Err(PreambleError::Settings(
                SettingsError::FirstCallWaitTooLong {
                    secs,
                    max_secs: 600
                }
            )),
            "{line}"
        );
    }
}

/// Unlike the environment variables, where an unknown word warns and falls back to the
/// default, an unknown word from a client is refused and the setting is named: the daemon has
/// no stderr the client sees.
#[test]
fn a_hello_with_an_unknown_word_for_a_setting_is_refused_and_the_setting_is_named() {
    let swaps = [
        (
            "structured_content",
            r#""structured_content":"omit-default-path""#,
            r#""structured_content":"sometimes""#,
        ),
        (
            "structured_content",
            r#""structured_content":"omit-default-path""#,
            r#""structured_content":"ALWAYS""#,
        ),
        (
            "concise_shape",
            r#""concise_shape":"slim""#,
            r#""concise_shape":"wide""#,
        ),
        (
            "concise_shape",
            r#""concise_shape":"slim""#,
            r#""concise_shape":"""#,
        ),
    ];
    for (setting, from, to) in swaps {
        assert_eq!(HELLO_LINE.matches(from).count(), 1);
        let line = HELLO_LINE.replace(from, to);
        let detail = invalid_detail(Hello::decode(line.as_bytes()));
        assert!(
            detail.starts_with(&format!("{setting} is not")),
            "{to}: {detail}"
        );
    }
}

#[test]
fn a_hello_with_a_value_of_the_wrong_kind_is_refused() {
    let swaps = [
        (
            r#""structured_content":"omit-default-path""#,
            r#""structured_content":true"#,
        ),
        (r#""result_cap":80000"#, r#""result_cap":"80000""#),
        (r#""result_cap":80000"#, r#""result_cap":-80000"#),
        (r#""result_cap":80000"#, r#""result_cap":80000.5"#),
        (
            r#""result_cap":80000"#,
            r#""result_cap":99999999999999999999999"#,
        ),
        (r#""result_cap":80000"#, r#""result_cap":null"#),
        (
            r#""first_call_wait_secs":20"#,
            r#""first_call_wait_secs":"20""#,
        ),
        (r#""warm_at_startup":false"#, r#""warm_at_startup":"false""#),
        (r#""warm_at_startup":false"#, r#""warm_at_startup":0"#),
    ];
    for (from, to) in swaps {
        assert_eq!(HELLO_LINE.matches(from).count(), 1);
        let line = HELLO_LINE.replace(from, to);
        let outcome = Hello::decode(line.as_bytes());
        assert!(
            matches!(outcome, Err(PreambleError::Invalid { .. })),
            "{to}: {outcome:?}"
        );
    }
}

#[test]
fn a_hello_whose_identity_is_not_a_full_key_is_refused() {
    let upper = KEY.to_uppercase();
    for identity in [&KEY[..16], "", &upper, &format!("{KEY}00"), "not-a-key"] {
        let line = HELLO_LINE.replace(KEY, identity);
        let detail = invalid_detail(Hello::decode(line.as_bytes()));
        assert!(detail.contains("identity"), "{identity:?}: {detail}");
    }
}

/// A field this version does not define is refused by name, in the message and in its
/// settings. (The one-spelling rule below would refuse these lines too; this pins the reason.)
#[test]
fn a_hello_with_a_field_this_version_does_not_define_is_refused_by_name() {
    for (field, line) in [
        (
            "extra",
            HELLO_LINE.replace(r#""protocol":1,"#, r#""protocol":1,"extra":true,"#),
        ),
        (
            "debug",
            HELLO_LINE.replace(
                r#""warm_at_startup":false"#,
                r#""warm_at_startup":false,"debug":1"#,
            ),
        ),
    ] {
        let detail = invalid_detail(Hello::decode(line.as_bytes()));
        assert!(
            detail.contains(&format!("unknown field `{field}`")),
            "{detail}"
        );
    }
}

/// One message has one spelling: the one `encode` writes. Rules out a decoder that accepts
/// whatever its JSON library accepts (extra fields, repeated fields, the positional array form
/// of the nested settings, other key orders, added whitespace, other number spellings).
#[test]
fn a_hello_in_any_spelling_other_than_the_encoder_s_is_refused() {
    let variants = [
        HELLO_LINE.replace(r#""protocol":1,"#, r#""protocol":1,"extra":true,"#),
        HELLO_LINE.replace(r#""warm_at_startup":false"#, r#""warm_at_startup":false,"debug":1"#),
        HELLO_LINE.replace(
            r#""result_cap":80000,"#,
            r#""result_cap":80000,"result_cap":90000,"#,
        ),
        HELLO_LINE.replace(r#""protocol":1,"#, r#""protocol":1,"protocol":1,"#),
        HELLO_LINE.replace(
            r#"{"structured_content":"omit-default-path","concise_shape":"slim","result_cap":80000,"first_call_wait_secs":20,"warm_at_startup":false}"#,
            r#"["omit-default-path","slim",80000,20,false]"#,
        ),
        HELLO_LINE.replace(
            r#""concise_shape":"slim","result_cap":80000,"#,
            r#""result_cap":80000,"concise_shape":"slim","#,
        ),
        HELLO_LINE.replace(r#"{"protocol":1,"#, r#"{ "protocol":1,"#),
        HELLO_LINE.replace(r#""result_cap":80000"#, r#""result_cap":080000"#),
        format!("{HELLO_LINE} "),
        format!("{HELLO_LINE}\r"),
        HELLO_LINE.replacen("HELLO ", "HELLO  ", 1),
    ];
    for line in variants {
        let outcome = Hello::decode(line.as_bytes());
        assert!(
            matches!(
                outcome,
                Err(PreambleError::Invalid { .. } | PreambleError::NotJson { .. })
            ),
            "{line}: {outcome:?}"
        );
    }
    assert!(Hello::decode(HELLO_LINE.as_bytes()).is_ok());
}

// ── The identity comparison ──

/// Spec §4.2 "Refusal" and §4.4: the daemon compares the full identity, not the 16 hex digits
/// in its file names. `NEIGHBOUR` has `KEY`'s first 16 digits, so a shim for it finds `KEY`'s
/// socket. Rules out comparing locators, or only a prefix.
#[test]
fn a_hello_for_a_neighbouring_identity_is_a_mismatch_that_names_the_served_key() {
    let served = key(KEY);
    assert_eq!(key(NEIGHBOUR).locator(), served.locator());

    let mut hello = default_hello();
    assert_eq!(hello.mismatch_against(&served), None);

    hello.identity = key(NEIGHBOUR);
    assert_eq!(
        hello.mismatch_against(&served),
        Some(Reply::Mismatch { expected: served })
    );

    let last_digit_differs = format!("{}1", &KEY[..63]);
    hello.identity = key(&last_digit_differs);
    assert_eq!(
        hello.mismatch_against(&served),
        Some(Reply::Mismatch { expected: served })
    );
}

#[test]
fn every_error_prints_on_one_bounded_line() {
    let long_verb = format!("{} {{}}", "V".repeat(3000));
    let errors = [
        Hello::decode(&[b' '; 5000]).unwrap_err(),
        Hello::decode(b"\xff").unwrap_err(),
        Hello::decode(b"HELLO").unwrap_err(),
        Hello::decode(long_verb.as_bytes()).unwrap_err(),
        Hello::decode(format!("HELLO {}", "[".repeat(3000)).as_bytes()).unwrap_err(),
        Hello::decode(format!("HELLO {{{}", "[".repeat(3000)).as_bytes()).unwrap_err(),
        Hello::decode(b"HELLO {\"protocol\":9}").unwrap_err(),
        Hello::decode(format!("HELLO {{\"protocol\":1,\"{}\":1}}", "x".repeat(3000)).as_bytes())
            .unwrap_err(),
        Hello::decode(HELLO_LINE.replace("80000", "1").as_bytes()).unwrap_err(),
    ];
    for error in errors {
        let text = error.to_string();
        assert!(!text.contains('\n'), "{text}");
        assert!(text.len() <= 400, "{} bytes: {text}", text.len());
    }
}
````

Create `src/mcp/shared/preamble_reply_tests.rs`:

````rust
//! Tests for the replies, the line a shim receives: their exact wire text, the round trip,
//! and every way a line from another process can be wrong.
use super::*;
use crate::mcp::shared::identity::IdentityKey;

const KEY: &str = "0123456789abcdeffedcba98765432100123456789abcdeffedcba9876543210";
const NEIGHBOUR: &str = "0123456789abcdef000000000000000000000000000000000000000000000000";
const READY_LINE: &str = r#"READY {"protocol":1,"identity":"0123456789abcdeffedcba98765432100123456789abcdeffedcba9876543210","pid":4242,"start_time":1791900090,"generation":3,"build":"building"}"#;

fn key(hex: &str) -> IdentityKey {
    IdentityKey::from_hex(hex).unwrap()
}

fn ready() -> Ready {
    Ready {
        identity: key(KEY),
        pid: 4242,
        start_time: 1_791_900_090,
        generation: 3,
        build: BuildState::Building,
    }
}

#[test]
fn every_reply_is_written_exactly_as_the_spec_shows_it() {
    assert_eq!(Reply::Ready(ready()).encode(), format!("{READY_LINE}\n"));
    assert_eq!(Reply::Retiring.encode(), "RETIRING {}\n");
    assert_eq!(
        Reply::Busy { max_clients: 64 }.encode(),
        "BUSY {\"max_clients\":64}\n"
    );
    assert_eq!(
        Reply::Mismatch {
            expected: key(NEIGHBOUR)
        }
        .encode(),
        format!("MISMATCH {{\"expected\":\"{NEIGHBOUR}\"}}\n")
    );
}

/// Spec §9: "every reply variant".
#[test]
fn every_reply_survives_encode_then_decode() {
    let mut replies = vec![
        Reply::Retiring,
        Reply::Busy { max_clients: 0 },
        Reply::Busy {
            max_clients: u32::MAX,
        },
        Reply::Mismatch { expected: key(KEY) },
    ];
    for build in [
        BuildState::Idle,
        BuildState::Building,
        BuildState::Ready,
        BuildState::Failed,
    ] {
        replies.push(Reply::Ready(Ready { build, ..ready() }));
    }
    replies.push(Reply::Ready(Ready {
        identity: key(NEIGHBOUR),
        pid: u32::MAX,
        start_time: u64::MAX,
        generation: u64::MAX,
        build: BuildState::Ready,
    }));
    for reply in replies {
        let line = reply.encode();
        assert!(line.ends_with('\n') && line.len() <= PREAMBLE_MAX_BYTES);
        let decoded = Reply::decode(line.trim_end_matches('\n').as_bytes());
        assert_eq!(decoded, Ok(reply), "{line}");
    }
}

#[test]
fn build_states_are_spelled_as_the_spec_spells_them() {
    let spelled = |build| match Reply::Ready(Ready { build, ..ready() }).encode() {
        line if line.contains("\"build\":\"idle\"") => "idle",
        line if line.contains("\"build\":\"building\"") => "building",
        line if line.contains("\"build\":\"ready\"") => "ready",
        line if line.contains("\"build\":\"failed\"") => "failed",
        line => panic!("{line}"),
    };
    assert_eq!(spelled(BuildState::Idle), "idle");
    assert_eq!(spelled(BuildState::Building), "building");
    assert_eq!(spelled(BuildState::Ready), "ready");
    assert_eq!(spelled(BuildState::Failed), "failed");
}

#[test]
fn a_ready_of_another_protocol_version_is_refused_as_such() {
    let line = READY_LINE.replace(r#""protocol":1,"#, r#""protocol":2,"#);
    assert_eq!(
        Reply::decode(line.as_bytes()),
        Err(PreambleError::UnsupportedProtocol { got: 2 })
    );
}

#[test]
fn a_reply_that_is_malformed_in_any_way_is_refused() {
    type Expectation = fn(&PreambleError) -> bool;
    let cases: Vec<(String, Expectation)> = vec![
        ("READY {}".into(), |e| {
            matches!(e, PreambleError::Invalid { .. })
        }),
        (
            READY_LINE.replace(r#","build":"building""#, ""),
            |e| matches!(e, PreambleError::Invalid { detail } if detail.contains("missing field `build`")),
        ),
        (
            READY_LINE.replace("building", "warming"),
            |e| matches!(e, PreambleError::Invalid { detail } if detail.contains("build")),
        ),
        (
            READY_LINE.replace("\"pid\":4242", "\"pid\":4294967296"),
            |e| matches!(e, PreambleError::Invalid { .. }),
        ),
        (
            READY_LINE.replace(KEY, &KEY[..16]),
            |e| matches!(e, PreambleError::Invalid { detail } if detail.contains("identity")),
        ),
        ("RETIRING".into(), |e| matches!(e, PreambleError::Malformed)),
        ("RETIRING {\"why\":1}".into(), |e| {
            matches!(e, PreambleError::Invalid { .. })
        }),
        ("RETIRING []".into(), |e| {
            matches!(e, PreambleError::Invalid { .. })
        }),
        (
            "BUSY {}".into(),
            |e| matches!(e, PreambleError::Invalid { detail } if detail.contains("max_clients")),
        ),
        ("BUSY {\"max_clients\":-1}".into(), |e| {
            matches!(e, PreambleError::Invalid { .. })
        }),
        (
            "MISMATCH {}".into(),
            |e| matches!(e, PreambleError::Invalid { detail } if detail.contains("expected")),
        ),
        (
            format!("MISMATCH {{\"expected\":\"{}\"}}", &KEY[..16]),
            |e| matches!(e, PreambleError::Invalid { detail } if detail.contains("expected")),
        ),
        ("WELCOME {}".into(), |e| {
            matches!(e, PreambleError::UnknownVerb { .. })
        }),
        ("READY {".into(), |e| {
            matches!(e, PreambleError::NotJson { .. })
        }),
        ("\u{fffd}".repeat(2000), |e| {
            matches!(e, PreambleError::TooLong { len: 6000 })
        }),
    ];
    for (line, expected) in cases {
        let error = Reply::decode(line.as_bytes()).expect_err(&line);
        assert!(expected(&error), "{line}: {error:?}");
    }
    assert_eq!(Reply::decode(b"READY \xff"), Err(PreambleError::NotUtf8));
}

#[test]
fn a_reply_with_a_field_this_version_does_not_define_is_refused_by_name() {
    let extra_field = |line: &str| match Reply::decode(line.as_bytes()) {
        Err(PreambleError::Invalid { detail }) => detail,
        other => panic!("{line}: {other:?}"),
    };
    for line in [
        "RETIRING {\"why\":1}".to_string(),
        "BUSY {\"max_clients\":64,\"why\":1}".to_string(),
        format!("MISMATCH {{\"expected\":\"{KEY}\",\"why\":1}}"),
        READY_LINE.replace(r#""pid":4242,"#, r#""pid":4242,"why":1,"#),
    ] {
        let detail = extra_field(&line);
        assert!(detail.contains("unknown field `why`"), "{line}: {detail}");
    }
}

/// Rules out a reply decoder without the one-spelling rule (each line below parses as JSON
/// into a valid reply).
#[test]
fn a_reply_in_any_spelling_other_than_the_encoder_s_is_refused() {
    let variants = [
        "RETIRING { }".to_string(),
        "RETIRING {} ".to_string(),
        "BUSY {\"max_clients\": 64}".to_string(),
        format!("MISMATCH {{\"expected\":\"{KEY}\"}}\r"),
        READY_LINE.replace(
            r#""pid":4242,"start_time":1791900090,"#,
            r#""start_time":1791900090,"pid":4242,"#,
        ),
    ];
    for line in variants {
        let outcome = Reply::decode(line.as_bytes());
        assert!(
            matches!(outcome, Err(PreambleError::Invalid { .. })),
            "{line}: {outcome:?}"
        );
    }
    assert!(Reply::decode(READY_LINE.as_bytes()).is_ok());
    assert!(Reply::decode(b"BUSY {\"max_clients\":64}").is_ok());
}

/// A `HELLO` that comes back to a shim (it connected to something that echoes) is not a reply.
#[test]
fn a_hello_is_not_a_reply() {
    assert_eq!(
        Reply::decode(b"HELLO {\"protocol\":1}"),
        Err(PreambleError::UnknownVerb {
            verb: "HELLO".into()
        })
    );
}
````

Create `src/mcp/shared/preamble.rs`:

```rust
#[cfg(test)]
#[path = "preamble_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "preamble_reply_tests.rs"]
mod reply_tests;
```

In `src/mcp/shared/mod.rs`, replace this:

```rust
pub mod paths;
pub mod runtime_dir;
```

with this:

```rust
pub mod paths;
pub mod preamble;
pub mod runtime_dir;
```

- [ ] **Step 2: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::shared::preamble:: > /tmp/s2-t6-red.log 2>&1; grep -cE '^error\[' /tmp/s2-t6-red.log; tail -1 /tmp/s2-t6-red.log
```

Expected: a count above 100 (`use of undeclared type PreambleError`, `Hello`, `Reply`, `BuildState`) and `error: could not compile`.

- [ ] **Step 3: Implement**

Replace the whole of `src/mcp/shared/preamble.rs` with:

````rust
//! The private preamble: one line each way on a fresh connection, before any MCP byte (spec
//! §4.4). The shim says which identity it wants and how its own connection should behave; the
//! daemon admits it (`READY`) or says why not.
//!
//! ```text
//! shim  -> daemon:  HELLO {"protocol":1,"identity":"<64 hex>","settings":{...}}
//! daemon -> shim:   READY {"protocol":1,"identity":"<64 hex>","pid":N,"start_time":N,"generation":N,"build":"..."}
//!              or   RETIRING {}  |  BUSY {"max_clients":N}  |  MISMATCH {"expected":"<64 hex>"}
//! ```
//!
//! A line arrives from another process, so decoding refuses everything it does not expect: a
//! message has exactly one spelling, the one its `encode` writes, and every value is checked
//! before it becomes a typed setting. Both ends are the same build (the build identity is part
//! of the key), so there is nothing to be lenient towards.
use super::identity::IdentityKey;
use super::SHARED_PROTOCOL_VERSION;
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::StructuredContentMode;
use crate::mcp::settings::{ClientSettings, ConnectionSettings, SettingsError};
use serde::Deserialize;
use std::fmt;

/// Longest preamble line, in bytes, not counting its newline (spec §4.2, §7: "4 KiB").
pub const PREAMBLE_MAX_BYTES: usize = 4096;

/// Longest piece of the other side's text an error message repeats.
const MAX_DETAIL_BYTES: usize = 200;
const MAX_VERB_BYTES: usize = 16;

/// Why a preamble line was refused. Nothing here is retried; what a daemon or a shim does
/// about each is theirs to decide (spec §10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreambleError {
    /// The line is longer than [`PREAMBLE_MAX_BYTES`].
    TooLong {
        len: usize,
    },
    NotUtf8,
    /// The line is not `VERB`, one space, payload.
    Malformed,
    /// A verb this side does not accept. `verb` is at most 16 bytes of it.
    UnknownVerb {
        verb: String,
    },
    /// The payload starts like a JSON object and is not valid JSON.
    NotJson {
        detail: String,
    },
    /// A well-formed message of another protocol version. Its identity cannot be the one
    /// served here, because the version is part of the identity key.
    UnsupportedProtocol {
        got: u64,
    },
    /// Settings a server must not run with (result cap below the floor, wait above the
    /// maximum).
    Settings(SettingsError),
    /// Anything else: not a JSON object, a missing, unknown or repeated field, a value of the
    /// wrong kind, an unknown word, a malformed key, or a spelling `encode` would not write.
    Invalid {
        detail: String,
    },
}

impl fmt::Display for PreambleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong { len } => write!(
                f,
                "preamble line is {len} bytes; the limit is {PREAMBLE_MAX_BYTES}"
            ),
            Self::NotUtf8 => write!(f, "preamble line is not UTF-8"),
            Self::Malformed => write!(f, "preamble line is not a verb followed by a payload"),
            Self::UnknownVerb { verb } => write!(f, "unexpected preamble verb {verb:?}"),
            Self::NotJson { detail } => write!(f, "preamble payload is not JSON: {detail}"),
            Self::UnsupportedProtocol { got } => write!(
                f,
                "preamble protocol version {got} is not {SHARED_PROTOCOL_VERSION}"
            ),
            Self::Settings(error) => write!(f, "preamble settings refused: {error}"),
            Self::Invalid { detail } => write!(f, "invalid preamble: {detail}"),
        }
    }
}

impl std::error::Error for PreambleError {}

/// At most `max` bytes of `text`, cut on a character boundary.
fn clip(text: &str, max: usize) -> String {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

/// An `Invalid` whose detail is this module's own wording (short by construction).
fn invalid(detail: impl Into<String>) -> PreambleError {
    PreambleError::Invalid {
        detail: detail.into(),
    }
}

/// The word for a structured-content mode: the one `PRISM_MCP_STRUCTURED_CONTENT` uses.
pub(crate) fn structured_content_word(mode: StructuredContentMode) -> &'static str {
    match mode {
        StructuredContentMode::Always => "always",
        StructuredContentMode::OmitDefaultPath => "omit-default-path",
    }
}

fn structured_content_from(word: &str) -> Result<StructuredContentMode, PreambleError> {
    match word {
        "always" => Ok(StructuredContentMode::Always),
        "omit-default-path" => Ok(StructuredContentMode::OmitDefaultPath),
        _ => Err(invalid(
            "structured_content is not \"always\" or \"omit-default-path\"",
        )),
    }
}

/// The word for a concise-shape mode: the one `PRISM_MCP_CONCISE_SHAPE` uses.
pub(crate) fn concise_shape_word(mode: ConciseShapeMode) -> &'static str {
    match mode {
        ConciseShapeMode::Legacy => "legacy",
        ConciseShapeMode::Slim => "slim",
    }
}

fn concise_shape_from(word: &str) -> Result<ConciseShapeMode, PreambleError> {
    match word {
        "legacy" => Ok(ConciseShapeMode::Legacy),
        "slim" => Ok(ConciseShapeMode::Slim),
        _ => Err(invalid("concise_shape is not \"legacy\" or \"slim\"")),
    }
}

/// State of the daemon's index build, as `READY` reports it (spec §4.2 readiness coordinator).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildState {
    Idle,
    Building,
    Ready,
    Failed,
}

impl BuildState {
    fn word(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Building => "building",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }

    fn from_word(word: &str) -> Result<Self, PreambleError> {
        match word {
            "idle" => Ok(Self::Idle),
            "building" => Ok(Self::Building),
            "ready" => Ok(Self::Ready),
            "failed" => Ok(Self::Failed),
            _ => Err(invalid(
                "build is not \"idle\", \"building\", \"ready\" or \"failed\"",
            )),
        }
    }
}

/// Split a line into its verb and payload after the checks every line gets: length, UTF-8,
/// shape, and that the verb is one of `verbs`.
fn verb_and_payload<'l>(
    line: &'l [u8],
    verbs: &[&'static str],
) -> Result<(&'static str, &'l str), PreambleError> {
    if line.len() > PREAMBLE_MAX_BYTES {
        return Err(PreambleError::TooLong { len: line.len() });
    }
    let text = std::str::from_utf8(line).map_err(|_| PreambleError::NotUtf8)?;
    let (verb, payload) = text.split_once(' ').ok_or(PreambleError::Malformed)?;
    match verbs.iter().find(|known| **known == verb) {
        Some(known) => Ok((known, payload)),
        None => Err(PreambleError::UnknownVerb {
            verb: clip(verb, MAX_VERB_BYTES),
        }),
    }
}

/// Parse a payload as the JSON object `T`. Only an object is accepted: `serde` would also take
/// a positional array for a struct, and nothing here writes one.
fn parse_object<'p, T: Deserialize<'p>>(payload: &'p str) -> Result<T, PreambleError> {
    if !payload.starts_with('{') {
        return Err(invalid("the payload is not a JSON object"));
    }
    serde_json::from_str(payload).map_err(|error| {
        let detail = clip(&error.to_string(), MAX_DETAIL_BYTES);
        match error.classify() {
            serde_json::error::Category::Data => PreambleError::Invalid { detail },
            _ => PreambleError::NotJson { detail },
        }
    })
}

/// Read `protocol` alone and refuse any version but this build's. This runs before the rest of
/// the payload is interpreted, because another version's payload may have another shape.
fn check_protocol(payload: &str) -> Result<(), PreambleError> {
    #[derive(Deserialize)]
    struct Versioned {
        protocol: u64,
    }
    let Versioned { protocol } = parse_object(payload)?;
    if protocol != u64::from(SHARED_PROTOCOL_VERSION) {
        return Err(PreambleError::UnsupportedProtocol { got: protocol });
    }
    Ok(())
}

fn key_field(name: &str, hex: &str) -> Result<IdentityKey, PreambleError> {
    IdentityKey::from_hex(hex)
        .map_err(|error| invalid(format!("{name} is not an identity key: {error}")))
}

/// Refuse a decoded message whose line is not byte for byte what `encode` writes for it.
fn require_canonical(line: &[u8], encoded: &str) -> Result<(), PreambleError> {
    if encoded.strip_suffix('\n').map(str::as_bytes) == Some(line) {
        Ok(())
    } else {
        Err(invalid(
            "the line is not spelled the way this build writes that message",
        ))
    }
}

/// What a shim sends: the identity it wants and its own connection settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hello {
    pub identity: IdentityKey,
    pub settings: ClientSettings,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HelloWire {
    #[allow(dead_code)] // read by `check_protocol`
    protocol: u64,
    identity: String,
    settings: SettingsWire,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsWire {
    structured_content: String,
    concise_shape: String,
    result_cap: u64,
    first_call_wait_secs: u64,
    warm_at_startup: bool,
}

impl Hello {
    /// The line to send, newline included.
    pub fn encode(&self) -> String {
        let output = self.settings.output();
        format!(
            "HELLO {{\"protocol\":{SHARED_PROTOCOL_VERSION},\"identity\":\"{}\",\"settings\":{{\"structured_content\":\"{}\",\"concise_shape\":\"{}\",\"result_cap\":{},\"first_call_wait_secs\":{},\"warm_at_startup\":{}}}}}\n",
            self.identity,
            structured_content_word(output.structured_content),
            concise_shape_word(output.concise_shape),
            output.result_cap,
            self.settings.first_call_wait_secs(),
            self.settings.warm_at_startup(),
        )
    }

    /// Decode one received line (without its newline). `Ok` means: this protocol version, a
    /// full identity key, and settings that passed `ClientSettings::new`.
    pub fn decode(line: &[u8]) -> Result<Self, PreambleError> {
        let (_, payload) = verb_and_payload(line, &["HELLO"])?;
        check_protocol(payload)?;
        let wire: HelloWire = parse_object(payload)?;
        let identity = key_field("identity", &wire.identity)?;
        let output = ConnectionSettings {
            result_cap: usize::try_from(wire.settings.result_cap)
                .map_err(|_| invalid("result_cap does not fit this platform's usize"))?,
            structured_content: structured_content_from(&wire.settings.structured_content)?,
            concise_shape: concise_shape_from(&wire.settings.concise_shape)?,
        };
        let settings = ClientSettings::new(
            output,
            wire.settings.first_call_wait_secs,
            wire.settings.warm_at_startup,
        )
        .map_err(PreambleError::Settings)?;
        let hello = Self { identity, settings };
        require_canonical(line, &hello.encode())?;
        Ok(hello)
    }

    /// `None` when this client asked for exactly the identity `served`; otherwise the
    /// `MISMATCH` reply it is owed. The comparison is of the full key: the 16 digits in the
    /// socket's file name only located the daemon (spec §4.2 "Refusal", §4.4).
    pub fn mismatch_against(&self, served: &IdentityKey) -> Option<Reply> {
        (self.identity != *served).then_some(Reply::Mismatch { expected: *served })
    }
}

/// The admitting reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ready {
    /// The identity the daemon serves, in full.
    pub identity: IdentityKey,
    pub pid: u32,
    /// When the daemon started, in seconds since the Unix epoch. With `pid` it names one
    /// daemon instance (spec §4.7).
    pub start_time: u64,
    /// The index generation at admission.
    pub generation: u64,
    pub build: BuildState,
}

/// What a daemon answers to a `HELLO`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    /// Admitted: MCP traffic may follow.
    Ready(Ready),
    /// The daemon is draining; start over (spec §3.1).
    Retiring,
    /// The daemon is at its connection bound (spec §4.2 `--max-clients`).
    Busy { max_clients: u32 },
    /// The daemon serves another identity; `expected` is the one it serves.
    Mismatch { expected: IdentityKey },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadyWire {
    #[allow(dead_code)] // read by `check_protocol`
    protocol: u64,
    identity: String,
    pid: u32,
    start_time: u64,
    generation: u64,
    build: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RetiringWire {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BusyWire {
    max_clients: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MismatchWire {
    expected: String,
}

impl Reply {
    /// The line to send, newline included.
    pub fn encode(&self) -> String {
        match self {
            Self::Ready(ready) => format!(
                "READY {{\"protocol\":{SHARED_PROTOCOL_VERSION},\"identity\":\"{}\",\"pid\":{},\"start_time\":{},\"generation\":{},\"build\":\"{}\"}}\n",
                ready.identity,
                ready.pid,
                ready.start_time,
                ready.generation,
                ready.build.word(),
            ),
            Self::Retiring => "RETIRING {}\n".to_string(),
            Self::Busy { max_clients } => format!("BUSY {{\"max_clients\":{max_clients}}}\n"),
            Self::Mismatch { expected } => format!("MISMATCH {{\"expected\":\"{expected}\"}}\n"),
        }
    }

    /// Decode one received line (without its newline).
    pub fn decode(line: &[u8]) -> Result<Self, PreambleError> {
        let (verb, payload) = verb_and_payload(line, &["READY", "RETIRING", "BUSY", "MISMATCH"])?;
        let reply = match verb {
            "READY" => {
                check_protocol(payload)?;
                let wire: ReadyWire = parse_object(payload)?;
                Self::Ready(Ready {
                    identity: key_field("identity", &wire.identity)?,
                    pid: wire.pid,
                    start_time: wire.start_time,
                    generation: wire.generation,
                    build: BuildState::from_word(&wire.build)?,
                })
            }
            "RETIRING" => {
                let RetiringWire {} = parse_object(payload)?;
                Self::Retiring
            }
            "BUSY" => {
                let wire: BusyWire = parse_object(payload)?;
                Self::Busy {
                    max_clients: wire.max_clients,
                }
            }
            _ => {
                let wire: MismatchWire = parse_object(payload)?;
                Self::Mismatch {
                    expected: key_field("expected", &wire.expected)?,
                }
            }
        };
        require_canonical(line, &reply.encode())?;
        Ok(reply)
    }
}

#[cfg(test)]
#[path = "preamble_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "preamble_reply_tests.rs"]
mod reply_tests;
````

- [ ] **Step 4: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::shared::preamble:: > /tmp/s2-t6-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s2-t6-green.log
```

Expected: `test result: ok. 27 passed; 0 failed` (19 under `tests::`, 8 under `reply_tests::`). The filter ends in `::` so that Task 7's `preamble_reader` tests are not counted here later.

- [ ] **Step 5: Commit**

```bash
git add src/mcp/shared/mod.rs src/mcp/shared/preamble.rs src/mcp/shared/preamble_tests.rs src/mcp/shared/preamble_reply_tests.rs
git commit -m "feat(mcp): shim-daemon preamble encode and decode"
```

- [ ] **Step 6: Prove the neighbouring-identity test bites (after the commit)**

Temporarily, in `src/mcp/shared/preamble.rs`, replace this:

```rust
        (self.identity != *served).then_some(Reply::Mismatch { expected: *served })
```

with this:

```rust
        (self.identity.locator() != served.locator()).then_some(Reply::Mismatch { expected: *served })
```

```bash
cargo test --features mcp --lib mcp::shared::preamble:: > /tmp/s2-t6-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s2-t6-bite.log
git checkout -- src/mcp/shared/preamble.rs
```

Expected: `a_hello_for_a_neighbouring_identity_is_a_mismatch_that_names_the_served_key ... FAILED` and `26 passed; 1 failed`.

---

### Task 7: Bounded preamble line reader

**Files:**
- Create: `src/mcp/shared/preamble_reader.rs`, `src/mcp/shared/preamble_reader_tests.rs`
- Modify: `src/mcp/shared/mod.rs`

**Interfaces:**
- Consumes: `PREAMBLE_MAX_BYTES`, `Reply` (Task 6); in tests, `crate::mcp::framing::{ReadOutcome, Transport, UnixStreamTransport}` (S1).
- Produces (in `crate::mcp::shared::preamble_reader`):
  - `pub enum PreambleReadError { TooLong, Eof { received: usize }, Io(io::Error) }` (`Debug`, `Display`, `Error`)
  - `pub struct PreambleLineReader` (`Debug`, `Default`) with `pub fn new() -> Self` and `pub fn read_line(&mut self, source: &mut impl Read) -> Result<Vec<u8>, PreambleReadError>`

- [ ] **Step 1: Write the tests**

Create `src/mcp/shared/preamble_reader_tests.rs`:

````rust
//! Tests for the bounded preamble line reader.
use super::*;
use crate::mcp::framing::{ReadOutcome, Transport, UnixStreamTransport};
use crate::mcp::shared::preamble::PREAMBLE_MAX_BYTES;
use std::collections::VecDeque;
use std::io::{Cursor, ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// A reader that hands out scripted chunks and errors, one per `read` call, and counts the
/// bytes it has handed out.
struct Scripted {
    steps: VecDeque<Result<Vec<u8>, ErrorKind>>,
    handed_out: usize,
}

impl Scripted {
    fn new(steps: Vec<Result<&[u8], ErrorKind>>) -> Self {
        Self {
            steps: steps
                .into_iter()
                .map(|step| step.map(<[u8]>::to_vec))
                .collect(),
            handed_out: 0,
        }
    }
}

impl Read for Scripted {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.steps.pop_front() {
            None => Ok(0),
            Some(Err(kind)) => Err(kind.into()),
            Some(Ok(mut chunk)) => {
                let n = chunk.len().min(buf.len());
                buf[..n].copy_from_slice(&chunk[..n]);
                if n < chunk.len() {
                    self.steps.push_front(Ok(chunk.split_off(n)));
                }
                self.handed_out += n;
                Ok(n)
            }
        }
    }
}

#[test]
fn a_line_is_returned_without_its_newline_and_the_next_byte_is_left_unread() {
    let mut source = Cursor::new(b"READY {}\nrest of the stream".to_vec());
    let line = PreambleLineReader::new().read_line(&mut source).unwrap();
    assert_eq!(line, b"READY {}");
    assert_eq!(
        source.position(),
        9,
        "only the line and its newline were taken"
    );
    let mut rest = String::new();
    source.read_to_string(&mut rest).unwrap();
    assert_eq!(rest, "rest of the stream");
}

/// The reason for reading a byte at a time: after the preamble the same socket carries MCP
/// frames, and the first of them may already be in the socket buffer. Rules out reading the
/// preamble through a `BufReader` that is then dropped with the MCP bytes inside it.
#[test]
fn the_mcp_frame_that_follows_the_preamble_on_a_socket_is_intact() {
    let (mut writer, reader) = UnixStream::pair().unwrap();
    let mcp = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    writer
        .write_all(format!("RETIRING {{}}\n{mcp}\n").as_bytes())
        .unwrap();
    // Closed, so a reader that lost the MCP bytes meets end of stream instead of waiting.
    drop(writer);

    let mut stream = reader;
    let line = PreambleLineReader::new().read_line(&mut stream).unwrap();
    assert_eq!(line, b"RETIRING {}");

    let mut transport = UnixStreamTransport::from_stream(stream).unwrap();
    match transport.read_message().unwrap() {
        Some(ReadOutcome::Message(message)) => assert_eq!(message, mcp),
        _ => panic!("the MCP frame after the preamble was lost"),
    }
}

/// Spec §4.2 and §7: a preamble line is at most 4 KiB.
#[test]
fn a_line_of_exactly_4_kib_is_read_and_one_byte_more_is_too_long() {
    let mut at_limit = vec![b'a'; PREAMBLE_MAX_BYTES];
    at_limit.push(b'\n');
    let line = PreambleLineReader::new()
        .read_line(&mut Cursor::new(at_limit))
        .unwrap();
    assert_eq!(line.len(), PREAMBLE_MAX_BYTES);

    let mut over = vec![b'a'; PREAMBLE_MAX_BYTES + 1];
    over.push(b'\n');
    let outcome = PreambleLineReader::new().read_line(&mut Cursor::new(over));
    assert!(
        matches!(outcome, Err(PreambleReadError::TooLong)),
        "{outcome:?}"
    );
}

/// Rules out reading the whole line and measuring it afterwards: a megabyte with no newline
/// is offered, and the reader must give up having taken 4 KiB and one byte.
#[test]
fn an_endless_line_is_given_up_on_after_one_byte_past_the_limit() {
    let megabyte = vec![b'a'; 1 << 20];
    let mut source = Scripted::new(vec![Ok(megabyte.as_slice())]);
    let mut reader = PreambleLineReader::new();
    let outcome = reader.read_line(&mut source);
    assert!(
        matches!(outcome, Err(PreambleReadError::TooLong)),
        "{outcome:?}"
    );
    assert_eq!(source.handed_out, PREAMBLE_MAX_BYTES + 1);
}

/// Once a line was too long the byte that proved it is gone, so nothing later on this
/// connection can be trusted to be a line.
#[test]
fn a_reader_that_overflowed_stays_overflowed() {
    let mut bytes = vec![b'a'; PREAMBLE_MAX_BYTES + 1];
    bytes.extend_from_slice(b"\nHELLO {}\n");
    let mut source = Cursor::new(bytes);
    let mut reader = PreambleLineReader::new();
    assert!(matches!(
        reader.read_line(&mut source),
        Err(PreambleReadError::TooLong)
    ));
    let again = reader.read_line(&mut source);
    assert!(
        matches!(again, Err(PreambleReadError::TooLong)),
        "{again:?}"
    );
}

/// Spec §4.1 step 5: end of stream before the reply is a transient condition the shim acts on,
/// so it is its own outcome, not an I/O error and not an empty line.
#[test]
fn end_of_stream_before_the_newline_is_reported_with_what_had_arrived() {
    let outcome = PreambleLineReader::new().read_line(&mut Cursor::new(b"READ".to_vec()));
    assert!(
        matches!(outcome, Err(PreambleReadError::Eof { received: 4 })),
        "{outcome:?}"
    );
    let outcome = PreambleLineReader::new().read_line(&mut Cursor::new(Vec::new()));
    assert!(
        matches!(outcome, Err(PreambleReadError::Eof { received: 0 })),
        "{outcome:?}"
    );
}

/// A non-blocking socket or a read timeout interrupts a line part-way. Rules out keeping the
/// partial line in a local variable: the second call must finish the same line.
#[test]
fn a_line_interrupted_by_would_block_continues_on_the_next_call() {
    let mut source = Scripted::new(vec![
        Ok(b"HEL"),
        Err(ErrorKind::WouldBlock),
        Ok(b"LO {}"),
        Err(ErrorKind::TimedOut),
        Ok(b"\nnext"),
    ]);
    let mut reader = PreambleLineReader::new();
    let first = reader.read_line(&mut source);
    assert!(
        matches!(&first, Err(PreambleReadError::Io(error)) if error.kind() == ErrorKind::WouldBlock),
        "{first:?}"
    );
    let second = reader.read_line(&mut source);
    assert!(
        matches!(&second, Err(PreambleReadError::Io(error)) if error.kind() == ErrorKind::TimedOut),
        "{second:?}"
    );
    assert_eq!(reader.read_line(&mut source).unwrap(), b"HELLO {}");
    assert_eq!(source.handed_out, 9, "nothing after the newline was taken");
}

#[test]
fn an_interrupted_read_is_retried_and_other_errors_are_passed_on() {
    let mut source = Scripted::new(vec![Ok(b"BU"), Err(ErrorKind::Interrupted), Ok(b"SY {}\n")]);
    let line = PreambleLineReader::new().read_line(&mut source).unwrap();
    assert_eq!(line, b"BUSY {}");

    let mut source = Scripted::new(vec![Ok(b"BU"), Err(ErrorKind::ConnectionReset)]);
    let outcome = PreambleLineReader::new().read_line(&mut source);
    assert!(
        matches!(&outcome, Err(PreambleReadError::Io(error)) if error.kind() == ErrorKind::ConnectionReset),
        "{outcome:?}"
    );
}

/// The same continuation against a real socket with a read timeout, which is how a shim
/// bounds its wait for the reply.
#[test]
fn a_socket_read_timeout_is_an_io_outcome_and_the_line_continues_afterwards() {
    let (mut writer, mut reader_end) = UnixStream::pair().unwrap();
    reader_end
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    writer.write_all(b"MISMATCH {\"expec").unwrap();

    // The rest is written once the first call has returned. Should that call never return (a
    // reader that waits out the timeout itself), the rest is written anyway after two
    // seconds, so that mistake fails this test instead of hanging it.
    let (first_call_returned, released) = std::sync::mpsc::channel::<()>();
    let rest = std::thread::spawn(move || {
        let _ = released.recv_timeout(Duration::from_secs(2));
        writer.write_all(b"ted\":\"k\"}\n").unwrap();
        // `writer` is dropped here, which ends the stream for the reader.
    });
    let timed_out = |error: &std::io::Error| {
        matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
    };

    let mut reader = PreambleLineReader::new();
    let first = reader.read_line(&mut reader_end);
    let _ = first_call_returned.send(());
    assert!(
        matches!(&first, Err(PreambleReadError::Io(error)) if timed_out(error)),
        "{first:?}"
    );

    let line = loop {
        match reader.read_line(&mut reader_end) {
            Ok(line) => break line,
            Err(PreambleReadError::Io(error)) if timed_out(&error) => {}
            Err(other) => panic!("{other:?}"),
        }
    };
    assert_eq!(line, b"MISMATCH {\"expected\":\"k\"}");

    rest.join().unwrap();
    let closed = reader.read_line(&mut reader_end);
    assert!(
        matches!(closed, Err(PreambleReadError::Eof { received: 0 })),
        "{closed:?}"
    );
}

#[test]
fn what_one_side_encodes_the_other_reads_and_decodes() {
    use crate::mcp::shared::identity::IdentityKey;
    use crate::mcp::shared::preamble::Reply;
    let reply = Reply::Mismatch {
        expected: IdentityKey::from_hex(&"ab".repeat(32)).unwrap(),
    };
    let (mut daemon, mut shim) = UnixStream::pair().unwrap();
    daemon.write_all(reply.encode().as_bytes()).unwrap();
    drop(daemon);
    let line = PreambleLineReader::new().read_line(&mut shim).unwrap();
    assert_eq!(Reply::decode(&line), Ok(reply));
}

#[test]
fn every_outcome_prints_on_one_line() {
    for error in [
        PreambleReadError::TooLong,
        PreambleReadError::Eof { received: 12 },
        PreambleReadError::Io(ErrorKind::TimedOut.into()),
    ] {
        let text = error.to_string();
        assert!(!text.is_empty() && !text.contains('\n'), "{text}");
    }
    assert!(PreambleReadError::TooLong.to_string().contains("4096"));
}
````

Create `src/mcp/shared/preamble_reader.rs`:

```rust
#[cfg(test)]
#[path = "preamble_reader_tests.rs"]
mod tests;
```

In `src/mcp/shared/mod.rs`, replace this:

```rust
pub mod preamble;
pub mod runtime_dir;
```

with this:

```rust
pub mod preamble;
pub mod preamble_reader;
pub mod runtime_dir;
```

- [ ] **Step 2: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::shared::preamble_reader > /tmp/s2-t7-red.log 2>&1; grep -cE '^error\[' /tmp/s2-t7-red.log; tail -1 /tmp/s2-t7-red.log
```

Expected: a count of 25 or more (`use of undeclared type PreambleLineReader`, `PreambleReadError`) and `error: could not compile`.

- [ ] **Step 3: Implement**

Replace the whole of `src/mcp/shared/preamble_reader.rs` with:

````rust
//! Reads one preamble line from a connection, bounded while the bytes arrive (spec §4.2, §7: a
//! preamble line is at most 4 KiB) and without taking a single byte past its newline, because
//! the same connection carries MCP frames next.
use super::preamble::PREAMBLE_MAX_BYTES;
use std::fmt;
use std::io::{self, Read};

/// Why no line was returned.
#[derive(Debug)]
pub enum PreambleReadError {
    /// More than [`PREAMBLE_MAX_BYTES`] bytes arrived without a newline. The reader has taken
    /// one byte past the limit and will not return a line again.
    TooLong,
    /// The stream ended before the newline; `received` bytes of the line had arrived. For a
    /// shim waiting for the reply this is the transient case of spec §4.1 step 5.
    Eof { received: usize },
    /// A read failed. For `WouldBlock` and `TimedOut` (a non-blocking socket, a read timeout)
    /// the bytes read so far are kept: calling [`PreambleLineReader::read_line`] again
    /// continues the same line. Whether to call again is the caller's deadline to decide.
    Io(io::Error),
}

impl fmt::Display for PreambleReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong => write!(f, "preamble line is longer than {PREAMBLE_MAX_BYTES} bytes"),
            Self::Eof { received } => write!(
                f,
                "connection closed {received} bytes into the preamble line"
            ),
            Self::Io(error) => write!(f, "reading the preamble line failed: {error}"),
        }
    }
}

impl std::error::Error for PreambleReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

/// Accumulates one line across as many reads as it takes. Holds at most
/// [`PREAMBLE_MAX_BYTES`] bytes.
#[derive(Debug, Default)]
pub struct PreambleLineReader {
    line: Vec<u8>,
    overflowed: bool,
}

impl PreambleLineReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read from `source` up to and including the next newline and return the line without
    /// it. Reads one byte at a time: a preamble is a few hundred bytes read once per
    /// connection, and whatever follows the newline must stay in `source` for the transport
    /// that takes over. `source` is typically a `UnixStream`, not a `BufReader`.
    pub fn read_line(&mut self, source: &mut impl Read) -> Result<Vec<u8>, PreambleReadError> {
        if self.overflowed {
            return Err(PreambleReadError::TooLong);
        }
        let mut byte = [0u8; 1];
        loop {
            match source.read(&mut byte) {
                Ok(0) => {
                    return Err(PreambleReadError::Eof {
                        received: self.line.len(),
                    })
                }
                Ok(_) if byte[0] == b'\n' => return Ok(std::mem::take(&mut self.line)),
                Ok(_) => {
                    if self.line.len() == PREAMBLE_MAX_BYTES {
                        self.overflowed = true;
                        return Err(PreambleReadError::TooLong);
                    }
                    self.line.push(byte[0]);
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(PreambleReadError::Io(error)),
            }
        }
    }
}

#[cfg(test)]
#[path = "preamble_reader_tests.rs"]
mod tests;
````

- [ ] **Step 4: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::shared::preamble_reader > /tmp/s2-t7-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s2-t7-green.log
```

Expected: `test result: ok. 11 passed; 0 failed`. One test waits out a 50 ms socket read timeout; none sleeps.

- [ ] **Step 5: Commit**

```bash
git add src/mcp/shared/mod.rs src/mcp/shared/preamble_reader.rs src/mcp/shared/preamble_reader_tests.rs
git commit -m "feat(mcp): bounded preamble line reader"
```

---

### Task 8: Proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md` (MCP Adapter section)

**Interfaces:**
- Consumes: everything above.
- Produces: the PR, with the invariants and the exclusions stated.

- [ ] **Step 1: Update the architecture notes**

In `CLAUDE.md`, replace this:

```markdown
change that is meant to alter tool output, and say why in the commit.
```

with this:

````markdown
change that is meant to alter tool output, and say why in the commit.

Shared-mode building blocks (`src/mcp/shared/`, Unix only; the daemon and the shim that use them
are later slices): `identity.rs` computes the daemon identity key, a SHA-256 over the canonical
repo root, its device and inode, the build identity, the shim–daemon protocol version, the
canonical cache base and the refresh policy; `cache_base.rs` gives `CacheMode::Default` and an
explicit `--cache-dir` one canonical spelling before the directory exists; `runtime_dir.rs`
resolves `$PRISM_MCP_RUNTIME_DIR`, else the user runtime directory, else the temp directory,
joined with `prism-mcpd`, and validates it on every use (a real directory, owned by the current
user, mode 0700); `paths.rs` names a daemon's files by the key's first 16 hex digits and enforces
the 103-byte socket path limit; `preamble.rs` and `preamble_reader.rs` encode, decode and read the
one-line `HELLO` / `READY` / `RETIRING` / `BUSY` / `MISMATCH` exchange (4 KiB bound, one accepted
spelling per message). Settings a client sends in the preamble are a `ClientSettings`
(`src/mcp/settings.rs`), whose constructor refuses a result cap below the floor and a first-call
wait above 600 s. A change to what goes into the key or to the preamble's wire text bumps
`SHARED_PROTOCOL_VERSION`.
````

- [ ] **Step 2: Run the MCP surface and the checks**

```bash
cargo fmt --check
cargo test --features mcp --lib mcp:: > /tmp/s2-final-lib.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s2-final-lib.log
env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > /tmp/s2-final-mcp.log 2>&1; grep -E '^test result|FAILED|differs|^error' /tmp/s2-final-mcp.log
cargo check > /tmp/s2-final-default.log 2>&1; tail -1 /tmp/s2-final-default.log
cargo clippy --all-targets --features mcp -- -W clippy::all > /tmp/s2-final-clippy.log 2>&1
grep -E -A8 '^warning|^error' /tmp/s2-final-clippy.log | grep -E 'src/mcp/shared/|src/mcp/settings|src/mcp/mod\.rs' | sort -u
```

Expected: `cargo fmt --check` prints nothing; the lib line shows base + 107 passed (`323 passed; 0 failed` on a 216 base: 22 + 13 + 21 + 6 + 7 + 27 + 11); the `mcp` target shows the base number unchanged (`13 passed; 0 failed`), which includes the golden transcripts; `cargo check` ends with `Finished`; the last command prints nothing.

- [ ] **Step 3: State the invariants**

```bash
BASE=$(git merge-base origin/main HEAD)
git diff --stat "$BASE" -- src/mcp/transport_tests.rs src/mcp/owner_transport_tests.rs | wc -l
git diff --stat "$BASE" -- tests/fixtures/mcp/golden | wc -l
git diff --stat "$BASE" -- src/mcp/transport.rs src/mcp/framing.rs src/mcp/output.rs src/mcp/session.rs src/mcp/lazy.rs src/mcp/owner.rs src/bin/prism-mcp.rs tests/mcp | wc -l
git diff --name-only "$BASE" -- src tests | grep -v -E '^src/mcp/shared/|^src/mcp/settings_tests\.rs$'
wc -l src/mcp/shared/*.rs src/mcp/settings.rs src/mcp/settings_tests.rs | sort -n | tail -3
```

Expected: three zeros (the test corpus, the goldens, and every serving file are untouched); the fourth command prints exactly `src/mcp/mod.rs` and `src/mcp/settings.rs`; the largest new file is under 600 lines.

- [ ] **Step 4: Run the remaining pre-PR checks**

These are the lane plan's "verification before a slice is called done". They were **not** run while this plan was validated (the planning host was short on memory); run them here and report what you get.

```bash
cargo test --features mcp > /tmp/s2-full.log 2>&1; grep -E '^test result|FAILED|failed' /tmp/s2-full.log
cargo test --doc --features mcp > /tmp/s2-doc.log 2>&1; grep -E '^test result' /tmp/s2-doc.log
python3 scripts/mutgate/mutgate.py > /tmp/s2-mutgate.log 2>&1; tail -5 /tmp/s2-mutgate.log
```

Expected: every `test result` line shows `0 failed` (sum the `passed` counts and report the total next to the Task 1 base numbers); the doc tests pass (this slice adds no doc test; the fenced blocks in its doc comments are marked `text`); the mutation gate exits 0. Report the gate as: "full gate green; it builds without `--features mcp`, so it selects no code from this slice." A failure outside this slice's files is reported, not fixed or re-baselined here.

- [ ] **Step 5: Commit and open the PR**

Write the PR body to `/tmp/s2-pr-body.md`. It states, in this order:

1. **Goal:** the lane goal from the lane plan, and that S2 adds library code no server path calls yet.
2. **Invariants:** the Step 3 output (three zeros, the two edited files), and that the golden transcripts pass unblessed.
3. **Decisions:** the D-table of this plan, and the seven spec issues, verbatim or by link. D2, D15, D18 and D24 are the ones a reviewer is most likely to want to change.
4. **Totals:** the Step 2 and Step 4 numbers next to the Task 1 base numbers.
5. **Not verified:** a non-Unix build (the module is `#[cfg(unix)]`; no Windows target is installed or in CI); behavior as root (one test skips itself); the mutation gate does not cover MCP code; the "different owner" check against a directory that really belongs to another non-root user (needs a second account; the injected-uid test and the `/` test stand in for it).
6. **Tier-A harness:** not run; this slice touches none of `src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`.

Then:

```bash
git add CLAUDE.md
git commit -m "docs(mcp): shared-mode building blocks"
git push -u origin HEAD
gh pr create --draft --title "feat(mcp): identity key, runtime directory and preamble (shared daemon S2)" --body-file /tmp/s2-pr-body.md
```

Review brief (lane plan): Opus 5.5 and gpt-6.1-sol, same commit, WRONG/SMELL and MATERIAL/IMMATERIAL tags against the lane goal, a recommended fix plus alternatives per finding, and when each finding would not apply. Cap: two rounds.

---

## Self-review record

- **Spec coverage (§12 row S2):** identity key with build identity and protocol version → Task 3 (`BuildIdentity`, `SHARED_PROTOCOL_VERSION` in the encoding); filesystem identity → Task 3 (`FsIdentity`, `root_fs`); normalized cache base → Task 2; runtime-dir validation → Task 1; preamble encode/decode → Tasks 6 and 7; connection-local settings → Task 5.
- **Spec §9 unit list, S2's items:** "identity key stability and sensitivity (each input, including build identity and protocol version, changes it; `--first-call-wait` does not)" → `the_key_is_sha256_of_the_length_prefixed_fields`, `every_input_changes_the_key`, `each_configuration_difference_gives_a_different_key`, `connection_local_settings_do_not_change_the_key`. "Lossless encoding" → `every_input_can_be_read_back_from_the_encoding`, `moving_bytes_across_a_field_boundary_changes_the_key`. "Cache-base normalization (Default vs. explicit equivalent, before the directory exists)" → `default_and_its_explicit_equivalents_are_one_base_before_the_directory_exists`. "Socket path length" → Task 4. "Runtime-dir validation (owner/mode/symlink, injected `fs` errors)" → Task 1, real-filesystem tests plus the `FakeDirs` tests. "Preamble encode/decode and every reply variant" → `every_hello_survives_encode_then_decode`, `every_reply_survives_encode_then_decode` and the refusal tests. The two remaining unit items are not S2's: the framing suite is S1's, the readiness coordinator is S3a's.
- **Spec §10 rows S2's types must express:** "Socket dir unwritable / path too long / runtime dir fails validation" → `RuntimeDirError` (all six variants), `SocketPathTooLong`. "EOF/reset/broken pipe before `READY`" → `PreambleReadError::Eof` and `Io` with the kind preserved. "Owner lock held but preamble times out" → `PreambleReadError::Io` with `WouldBlock`/`TimedOut`, partial line kept. "`RETIRING` / `BUSY` / `MISMATCH`" → `Reply`. "Root identity lost or changed" → `FsIdentity::of` (error = lost) compared with `DaemonIdentity::root_fs` (different = changed). "Peer uid mismatch" → S4 (D24); `current_uid()` is here.
- **Out of S2 by the spec's own slicing:** listening, accepting, the handshake timeout, `--max-clients`, locks and their files, the pid file and log, spawning, the idle TTL (S4, S5a); the readiness coordinator and the status probe (S3a); the `--shared` flag and its CLI errors (S5a); `docs/MCP.md` (S6).
- **Lane plan's two S2 notes:** `ConnectionSettings` is not widened (Task 5 adds `ClientSettings` beside it); the client-supplied cap goes through one validating constructor (`ClientSettings::new`), which `Hello::decode` is the only caller of on the decode path. The reading of "applies the `resolve_cap_from` rule" is D15.
- **A fact measured during validation that the lane plan stated from reading:** a sub-floor cap does panic in a debug build. `ConnectionSettings { result_cap: 100, .. }` passed straight to `serve_connection` on a stale index under `--refresh-policy auto-full` panics at `src/mcp/transport.rs:803` ("accepted MCP cap must leave room for reserve and minimum shaped result"). Task 5's `the_smallest_accepted_cap_serves_the_query_that_reserves_the_most` runs the same path at the floor and passes.
- **Facts measured on the planning host (macOS 26.6.2, APFS, case-insensitive):** `realpath` returns the on-disk case for an existing path, so two case spellings of an existing root or cache base give one key; a path of 103 bytes binds and one of 104 fails with "AF_UNIX path too long"; `$TMPDIR` is 49 bytes, so the default socket path is 81 bytes.
- **For the planners of S4 and S5a:** (1) put a test's runtime directory under `/tmp` (`tempfile::Builder::new().tempdir_in("/tmp")`), not under `$TMPDIR`: with D2 a `tempfile::tempdir()` on macOS leaves 11 bytes of headroom under the 103-byte limit, and none once the path is canonicalized to `/private/var/...` and nested one level. (2) `PreambleLineReader` is meant to be given the `UnixStream` itself; hand the same stream to `UnixStreamTransport::from_stream` afterwards. (3) A daemon must build its `ConnectionSettings` from `ClientSettings::output()` only. (4) The pid file's `build_identity` line and `daemon-status`'s "build identity" (spec §3.1, §4.7) do not say which of the three build values they mean; `BuildIdentity` has all three. (5) `Hello::decode` errors need a daemon-side policy (SI-3), and `warm_at_startup` needs a rule for who it is honored from (SI-7). (6) The default runtime directory follows `$TMPDIR`, which some sandboxes set per session; S6's launchers should set `PRISM_MCP_RUNTIME_DIR` explicitly, as spec §8 step 1 already says for the bridge.
- **Size:** 20 files, 4,024 added lines: about 1,400 of code (`runtime_dir.rs` 271, `cache_base.rs` 116, `identity.rs` 288, `paths.rs` 103, `preamble.rs` 422, `preamble_reader.rs` 91, `settings.rs` +100, `mod.rs` 18), about 2,600 of tests, 15 of docs. If the review loop needs a smaller unit, the plan splits with no rework after Task 4: Tasks 1–4 (where a daemon lives and which configuration it serves; about 2,200 lines) and Tasks 5–8 (what a client may ask; about 1,800 lines). Task 5 depends on nothing in Tasks 1–4; Task 6 needs only `IdentityKey` from Task 3.
- **Deliberate deviations from the spec text:** none in values. D2, D15 and D18 are readings or additions where the text is ambiguous or silent, and are listed as such.

## Validation record

The code in this plan was written test-first in a throwaway worktree, then **this plan's text was applied from scratch**: a script reset the worktree to `origin/main` `f6354e07`, performed every "Create / Replace / In … replace / Append" directive in order, and ran every `bash` block exactly as written (it skipped only `git push`, `gh pr create`, and Task 8 Step 4). The result is the local branch `scratch/s2-validate` in `~/code/slicing` (not pushed, not for merge as it stands), one commit per task:

| Task | Commit |
|---|---|
| 1 | `60dadeff` feat(mcp): runtime directory resolution and validation for shared mode |
| 2 | `af2ed540` feat(mcp): canonical cache base directory for shared mode |
| 3 | `0f818b89` feat(mcp): daemon identity key for shared mode |
| 4 | `ff5931d5` feat(mcp): daemon file paths and the socket path limit |
| 5 | `e531a3d2` feat(mcp): validated per-client settings for shared mode |
| 6 | `43eb8b28` feat(mcp): shim-daemon preamble encode and decode |
| 7 | `a0aa033e` feat(mcp): bounded preamble line reader |
| 8 | `cf873945` docs(mcp): shared-mode building blocks |

What ran in that replay, on macOS 26.6.2 (arm64, APFS, rustc 1.94.0), with the output each step printed:

| Step | Command (as in the plan) | Output |
|---|---|---|
| T1 S1 | `cargo test --features mcp --lib mcp::` on the base | `216 passed; 0 failed; 0 ignored; 0 measured; 1390 filtered out` |
| T1 S1 | `cargo test --features mcp --test mcp` on the base | `13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| T1 S5 / S7 | red, then `mcp::shared::runtime_dir` | 59 compile errors; then `22 passed; 0 failed` |
| T1 S9 | bite: `fs::metadata` | `a_symlink_to_a_private_directory_is_refused_with_or_without_a_trailing_slash ... FAILED`; `21 passed; 1 failed` |
| T2 S2 / S4 | red, then `mcp::shared::cache_base` | 25 compile errors; then `13 passed; 0 failed` |
| T3 S2 / S4 | red, then `mcp::shared::identity` | 43 compile errors; then `21 passed; 0 failed` |
| T3 S6 | bite: inode zeroed in the encoding | 4 tests FAILED, among them `a_root_replaced_at_the_same_pathname_has_a_new_key`; `17 passed; 4 failed` |
| T4 S2 / S4 | red, then `mcp::shared::paths` | 12 compile errors; then `6 passed; 0 failed` |
| T5 S2 / S4 | red, then `mcp::settings` | 12 compile errors; then `7 passed; 0 failed`; the one removed line is the old `use super::output::{…}` import |
| T6 S2 / S4 | red, then `mcp::shared::preamble::` | 121 compile errors; then `27 passed; 0 failed` |
| T6 S6 | bite: compare locators | `a_hello_for_a_neighbouring_identity_is_a_mismatch_that_names_the_served_key ... FAILED`; `26 passed; 1 failed` |
| T7 S2 / S4 | red, then `mcp::shared::preamble_reader` | 30 compile errors; then `11 passed; 0 failed` |
| T8 S2 | `cargo fmt --check` | no output, exit 0 |
| T8 S2 | `cargo test --features mcp --lib mcp::` | `323 passed; 0 failed; 0 ignored; 0 measured; 1390 filtered out` (216 base + 107) |
| T8 S2 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` | `13 passed; 0 failed`, including `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok` |
| T8 S2 | `cargo check` (default features) | `Finished` |
| T8 S2 | clippy, filtered to this slice's files | no line (258 `warning` lines in the whole log, none in `src/mcp/shared/`, `src/mcp/settings*.rs` or `src/mcp/mod.rs`) |
| T8 S3 | the three zero-diff checks | `0`, `0`, `0` |
| T8 S3 | existing source files edited | `src/mcp/mod.rs`, `src/mcp/settings.rs` |
| T8 S3 | largest new files | `identity_tests.rs` 587 lines, `preamble_tests.rs` 531 |

After the replay the tree was byte-identical to the test-first development tree (`git diff` between the two: 0 lines), so every code block here is code that compiled and passed.

**Wrong implementations.** The appendix's 77 changes were each applied to the committed code and the module's tests run; all 77 made at least one test fail. (Rows C1–C8 ran on the final `cache_base.rs`; the other rows ran one commit earlier, before defect 6 below was fixed, when every file other than `cache_base.rs` and its tests was already what this plan contains.)

**Defects the validation found in this plan's own first draft, all corrected in the text above:**

1. Two test expectations were wrong: `HELLO {} {}` is refused for its missing `protocol` before the trailing text is reached (so `Invalid`, not `NotJson`), and one replacement string in a table produced invalid JSON instead of an empty word.
2. Three wrong implementations survived the first pass over the preamble: dropping the one-spelling rule for replies (no test fed a reply in another spelling), the environment-style fallback for an unknown word, and removing `deny_unknown_fields` (both masked by the one-spelling rule, which refused the same lines for a different reason). Tests were added that pin the reason: `a_reply_in_any_spelling_other_than_the_encoder_s_is_refused`, `a_hello_with_an_unknown_word_for_a_setting_is_refused_and_the_setting_is_named`, and the two `…_does_not_define_is_refused_by_name` tests. A `clip` applied to this module's own fixed messages turned out to be dead code and was removed.
3. Two reader tests **hung** instead of failing under a wrong implementation (a buffered reader; a reader that waits out `WouldBlock` itself). The writer end is now closed after writing, and the timeout test's second half is written by a helper thread with a two-second fallback, so both mistakes fail in bounded time.
4. `effective_cache_base` read the working directory unconditionally, so an absolute cache base failed for a process whose working directory was gone. It is now read only for a relative base, and every absolute-base test passes a working directory that cannot be read.
5. `preamble_tests.rs` reached 635 lines; the reply tests moved to `preamble_reply_tests.rs`.
6. Two `clippy::type_complexity` warnings in new test files; fixed with local type aliases.
7. A test that only restated `transport.rs`'s compile-time assertion was replaced by `the_smallest_accepted_cap_serves_the_query_that_reserves_the_most`, which runs dispatch at the floor.

**Not run:** the full `cargo test --features mcp` suite; `cargo test --doc --features mcp`; `cargo check --tests --features mcp,detached-owner-audit`; the mutation gate; a release build; the Tier-A harness; CI. Everything above ran on macOS only, so what differs on Linux is unverified there: the `ErrorKind` of a socket read timeout (both kinds are accepted), `ENOTDIR` for a path below a regular file, the `/` ownership test under a CI user, and binding at 103 bytes (Linux allows 107). Nothing ran as root, and no directory owned by a second real non-root user was available (the injected-uid test and the `/` test cover that check).

## Appendix: wrong implementations applied during plan validation

Each row is one change made to the committed code in the scratch worktree, followed by the module's tests; the change was reverted afterwards. "Fails" lists the tests that failed (at most three are shown; the count is the full number). A row is evidence that the named tests would catch that mistake, not a claim that no other mistake exists.

| # | File | Wrong implementation | Result | Fails |
|---|---|---|---|---|
| R1 | `runtime_dir.rs` | inspect with fs::metadata, which follows a symlink | killed (1) | `a_symlink_to_a_private_directory_is_refused_with_or_without_a_trailing_slash` |
| R2 | `runtime_dir.rs` | lstat the path as given, trailing slash included | killed (1) | `a_symlink_to_a_private_directory_is_refused_with_or_without_a_trailing_slash` |
| R3 | `runtime_dir.rs` | no ownership check | killed (3) | `a_directory_that_really_belongs_to_root_is_refused`, `a_private_directory_owned_by_a_different_uid_is_refused`, `losing_the_creation_race_to_a_valid_directory_passes` |
| R4 | `runtime_dir.rs` | refuse only other-accessible modes | killed (2) | `any_mode_other_than_0700_is_refused_and_left_as_it_was`, `only_the_nine_permission_bits_are_compared` |
| R5 | `runtime_dir.rs` | refuse only group- or other-accessible modes (0500 passes) | killed (1) | `any_mode_other_than_0700_is_refused_and_left_as_it_was` |
| R6 | `runtime_dir.rs` | create with the default mode | killed (1) | `an_absent_directory_is_created_0700_and_passes` |
| R7 | `runtime_dir.rs` | create missing parents too | killed (1) | `a_missing_base_is_an_error_and_nothing_is_created` |
| R8 | `runtime_dir.rs` | trust a directory this process just created | killed (2) | `losing_the_creation_race_to_a_valid_directory_passes`, `what_is_found_after_a_successful_creation_is_still_checked` |
| R9 | `runtime_dir.rs` | treat an lstat error as 'absent' | killed (1) | `an_inspection_error_is_reported_and_nothing_is_created` |
| R10 | `runtime_dir.rs` | accept a relative base | killed (1) | `a_relative_base_is_refused_whichever_source_it_came_from` |
| R11 | `runtime_dir.rs` | use the base itself, without prism-mcpd | killed (4) | `an_empty_override_counts_as_unset`, `the_environment_reader_agrees_with_the_pure_resolver`, `the_override_is_the_base_and_the_directory_name_is_joined_to_it` (+1 more) |
| R12 | `runtime_dir.rs` | compare set-id and sticky bits too | killed (1) | `only_the_nine_permission_bits_are_compared` |
| C1 | `cache_base.rs` | do not resolve symlinks (textual normalization) | killed (6) | `a_base_below_a_regular_file_or_through_a_dangling_symlink_is_an_error`, `a_symlink_after_climbing_out_of_a_missing_part_is_still_resolved`, `an_existing_base_is_canonical_through_symlinks_dots_and_slashes` (+3 more) |
| C2 | `cache_base.rs` | create the directory in order to canonicalize it | killed (3) | `climbing_above_the_root_stays_at_the_root`, `default_and_its_explicit_equivalents_are_one_base_before_the_directory_exists`, `the_resolved_base_is_where_create_dir_all_then_puts_it` |
| C3 | `cache_base.rs` | give NoCache the default base | killed (2) | `a_cache_free_configuration_has_no_cache_base`, `the_environment_reader_agrees_with_the_pure_resolver` |
| C4 | `cache_base.rs` | ignore the working directory for a relative base | killed (1) | `a_relative_base_is_resolved_against_the_working_directory` |
| C5 | `cache_base.rs` | treat every lstat error as 'does not exist yet' | killed (1) | `a_base_below_a_regular_file_or_through_a_dangling_symlink_is_an_error` |
| C6 | `cache_base.rs` | keep `..` in the result | killed (6) | `a_relative_base_is_resolved_against_the_working_directory`, `a_symlink_after_climbing_out_of_a_missing_part_is_still_resolved`, `climbing_above_the_root_stays_at_the_root` (+3 more) |
| C7 | `cache_base.rs` | a default base other than the navigation cache's | killed (1) | `the_default_base_is_the_one_the_navigation_cache_uses` |
| C8 | `cache_base.rs` | read the working directory even for an absolute base | killed (7) | `a_symlink_after_climbing_out_of_a_missing_part_is_still_resolved`, `an_existing_base_is_canonical_through_symlinks_dots_and_slashes`, `climbing_above_the_root_stays_at_the_root` (+4 more) |
| I1 | `identity.rs` | leave the repo root out of the key | killed (4) | `every_input_can_be_read_back_from_the_encoding`, `every_input_changes_the_key`, `roots_that_differ_only_in_invalid_utf8_bytes_have_different_keys` (+1 more) |
| I2 | `identity.rs` | leave the root device out of the key | killed (3) | `every_input_can_be_read_back_from_the_encoding`, `every_input_changes_the_key`, `the_key_is_sha256_of_the_length_prefixed_fields` |
| I3 | `identity.rs` | leave the root inode out of the key | killed (4) | `a_root_replaced_at_the_same_pathname_has_a_new_key`, `every_input_can_be_read_back_from_the_encoding`, `every_input_changes_the_key` (+1 more) |
| I4 | `identity.rs` | leave the package version out of the key | killed (3) | `every_input_can_be_read_back_from_the_encoding`, `every_input_changes_the_key`, `the_key_is_sha256_of_the_length_prefixed_fields` |
| I5 | `identity.rs` | leave the grammar fingerprint out of the key | killed (3) | `every_input_can_be_read_back_from_the_encoding`, `every_input_changes_the_key`, `the_key_is_sha256_of_the_length_prefixed_fields` |
| I6 | `identity.rs` | leave the analyzer build identity out of the key | killed (4) | `each_configuration_difference_gives_a_different_key`, `every_input_can_be_read_back_from_the_encoding`, `every_input_changes_the_key` (+1 more) |
| I7 | `identity.rs` | leave the protocol version out of the key | killed (4) | `each_configuration_difference_gives_a_different_key`, `every_input_can_be_read_back_from_the_encoding`, `every_input_changes_the_key` (+1 more) |
| I8 | `identity.rs` | leave the cache base out of the key | killed (4) | `each_configuration_difference_gives_a_different_key`, `every_input_can_be_read_back_from_the_encoding`, `every_input_changes_the_key` (+1 more) |
| I9 | `identity.rs` | leave the refresh policy out of the key | killed (4) | `each_configuration_difference_gives_a_different_key`, `every_input_can_be_read_back_from_the_encoding`, `every_input_changes_the_key` (+1 more) |
| I10 | `identity.rs` | concatenate fields without lengths | killed (3) | `every_input_can_be_read_back_from_the_encoding`, `moving_bytes_across_a_field_boundary_changes_the_key`, `the_key_is_sha256_of_the_length_prefixed_fields` |
| I11 | `identity.rs` | encode the root with to_string_lossy | killed (2) | `every_input_can_be_read_back_from_the_encoding`, `roots_that_differ_only_in_invalid_utf8_bytes_have_different_keys` |
| I12 | `identity.rs` | let --first-call-wait / --warm-at-startup into the key | killed (3) | `a_daemon_started_from_an_identity_s_own_values_computes_the_same_key`, `connection_local_settings_do_not_change_the_key`, `resolving_reads_the_root_the_cache_base_the_policy_and_this_build` |
| I13 | `identity.rs` | key the root as typed, not canonical | killed (2) | `one_checkout_and_one_cache_base_have_one_key_however_they_are_spelled`, `the_root_is_spelled_as_the_in_process_server_spells_it` |
| I14 | `identity.rs` | no filesystem identity (path only) | killed (2) | `a_root_replaced_at_the_same_pathname_has_a_new_key`, `resolving_reads_the_root_the_cache_base_the_policy_and_this_build` |
| I15 | `identity.rs` | accept uppercase hex in a key | killed (1) | `a_malformed_key_is_not_a_key` |
| I16 | `identity.rs` | use the git SHA as the build identity | killed (1) | `the_current_build_is_the_three_values_the_spec_names` |
| I17 | `identity.rs` | key an explicit cache dir as typed | killed (1) | `one_checkout_and_one_cache_base_have_one_key_however_they_are_spelled` |
| I18 | `identity.rs` | ignore the configured refresh policy | killed (2) | `each_configuration_difference_gives_a_different_key`, `resolving_reads_the_root_the_cache_base_the_policy_and_this_build` |
| P1 | `paths.rs` | limit of 104 bytes | killed (2) | `a_socket_path_of_103_bytes_fits_and_104_does_not`, `the_error_states_the_length_the_limit_and_the_way_out` |
| P2 | `paths.rs` | refuse a path of exactly 103 bytes | killed (2) | `a_socket_can_really_be_bound_at_the_limit`, `a_socket_path_of_103_bytes_fits_and_104_does_not` |
| P3 | `paths.rs` | count characters instead of bytes | killed (1) | `the_limit_counts_bytes_not_characters` |
| P4 | `paths.rs` | name files by the full key | killed (6) | `a_socket_can_really_be_bound_at_the_limit`, `a_socket_path_of_103_bytes_fits_and_104_does_not`, `keys_that_share_their_first_16_digits_share_their_files` (+3 more) |
| P5 | `paths.rs` | no length check | killed (3) | `a_socket_path_of_103_bytes_fits_and_104_does_not`, `the_error_states_the_length_the_limit_and_the_way_out`, `the_limit_counts_bytes_not_characters` |
| S1 | `settings.rs` | no floor on the result cap | killed (2) | `a_result_cap_below_the_floor_is_refused`, `the_constructor_accepts_exactly_the_caps_the_environment_resolver_keeps` |
| S2 | `settings.rs` | refuse the floor itself | killed (3) | `a_result_cap_at_or_above_the_floor_is_kept_exactly`, `the_constructor_accepts_exactly_the_caps_the_environment_resolver_keeps`, `the_smallest_accepted_cap_serves_the_query_that_reserves_the_most` |
| S3 | `settings.rs` | raise a sub-floor cap to the floor silently | killed (2) | `a_result_cap_below_the_floor_is_refused`, `the_constructor_accepts_exactly_the_caps_the_environment_resolver_keeps` |
| S4 | `settings.rs` | no bound on the first-call wait | killed (1) | `a_first_call_wait_above_the_command_line_maximum_is_refused` |
| S5 | `settings.rs` | refuse the maximum wait itself | killed (1) | `a_first_call_wait_above_the_command_line_maximum_is_refused` |
| H1 | `preamble.rs` | no length check in decode | killed (2) | `a_line_over_4_kib_is_refused_for_its_length_alone`, `a_reply_that_is_malformed_in_any_way_is_refused` |
| H2 | `preamble.rs` | refuse a line of exactly 4096 bytes for its length | killed (1) | `a_line_over_4_kib_is_refused_for_its_length_alone` |
| H3 | `preamble.rs` | decode invalid UTF-8 lossily | killed (2) | `a_reply_that_is_malformed_in_any_way_is_refused`, `bytes_that_are_not_utf8_are_refused` |
| H4 | `preamble.rs` | no protocol version check | killed (2) | `a_hello_of_another_protocol_version_is_refused_as_such`, `a_ready_of_another_protocol_version_is_refused_as_such` |
| H5 | `preamble.rs` | check the version after interpreting the payload | killed (2) | `a_hello_of_another_protocol_version_is_refused_as_such`, `a_payload_that_is_not_json_is_refused` |
| H6 | `preamble.rs` | clamp out-of-range settings instead of refusing | killed (1) | `a_hello_asking_for_settings_a_server_must_not_run_with_is_refused` |
| H7 | `preamble.rs` | no one-spelling rule for HELLO | killed (2) | `a_hello_in_any_spelling_other_than_the_encoder_s_is_refused`, `a_line_over_4_kib_is_refused_for_its_length_alone` |
| H8 | `preamble.rs` | no one-spelling rule for replies | killed (1) | `a_reply_in_any_spelling_other_than_the_encoder_s_is_refused` |
| H9 | `preamble.rs` | accept a payload that is not a JSON object | killed (1) | `a_payload_that_is_not_a_json_object_is_refused` |
| H10 | `preamble.rs` | compare only the 16-digit locator | killed (1) | `a_hello_for_a_neighbouring_identity_is_a_mismatch_that_names_the_served_key` |
| H11 | `preamble.rs` | MISMATCH echoes the client's key | killed (1) | `a_hello_for_a_neighbouring_identity_is_a_mismatch_that_names_the_served_key` |
| H12 | `preamble.rs` | fall back to the default for an unknown word, as the env resolver does | killed (1) | `a_hello_with_an_unknown_word_for_a_setting_is_refused_and_the_setting_is_named` |
| H13 | `preamble.rs` | echo the JSON library's message unclipped | killed (1) | `every_error_prints_on_one_bounded_line` |
| H14 | `preamble.rs` | echo an unknown verb unclipped | killed (2) | `a_verb_the_receiver_does_not_expect_is_refused`, `every_error_prints_on_one_bounded_line` |
| H15 | `preamble.rs` | accept HELLO as a reply | killed (1) | `a_hello_is_not_a_reply` |
| H16 | `preamble.rs` | ignore unknown fields in HelloWire | killed (1) | `a_hello_with_a_field_this_version_does_not_define_is_refused_by_name` |
| H17 | `preamble.rs` | ignore unknown fields in SettingsWire | killed (1) | `a_hello_with_a_field_this_version_does_not_define_is_refused_by_name` |
| H18 | `preamble.rs` | ignore unknown fields in ReadyWire | killed (1) | `a_reply_with_a_field_this_version_does_not_define_is_refused_by_name` |
| H19 | `preamble.rs` | ignore unknown fields in RetiringWire | killed (1) | `a_reply_with_a_field_this_version_does_not_define_is_refused_by_name` |
| H20 | `preamble.rs` | ignore unknown fields in BusyWire | killed (1) | `a_reply_with_a_field_this_version_does_not_define_is_refused_by_name` |
| H21 | `preamble.rs` | ignore unknown fields in MismatchWire | killed (1) | `a_reply_with_a_field_this_version_does_not_define_is_refused_by_name` |
| L1 | `preamble_reader.rs` | read through a BufReader | killed (10) | `a_line_interrupted_by_would_block_continues_on_the_next_call`, `a_line_is_returned_without_its_newline_and_the_next_byte_is_left_unread`, `a_line_of_exactly_4_kib_is_read_and_one_byte_more_is_too_long` (+7 more) |
| L2 | `preamble_reader.rs` | accept a line of 4097 bytes | killed (3) | `a_line_of_exactly_4_kib_is_read_and_one_byte_more_is_too_long`, `a_reader_that_overflowed_stays_overflowed`, `an_endless_line_is_given_up_on_after_one_byte_past_the_limit` |
| L3 | `preamble_reader.rs` | no limit while reading | killed (3) | `a_line_of_exactly_4_kib_is_read_and_one_byte_more_is_too_long`, `a_reader_that_overflowed_stays_overflowed`, `an_endless_line_is_given_up_on_after_one_byte_past_the_limit` |
| L4 | `preamble_reader.rs` | return a line again after an overflow | killed (1) | `a_reader_that_overflowed_stays_overflowed` |
| L5 | `preamble_reader.rs` | drop the partial line between calls | killed (2) | `a_line_interrupted_by_would_block_continues_on_the_next_call`, `a_socket_read_timeout_is_an_io_outcome_and_the_line_continues_afterwards` |
| L6 | `preamble_reader.rs` | return what arrived at end of stream as a line | killed (2) | `a_socket_read_timeout_is_an_io_outcome_and_the_line_continues_afterwards`, `end_of_stream_before_the_newline_is_reported_with_what_had_arrived` |
| L7 | `preamble_reader.rs` | fail on an interrupted read | killed (1) | `an_interrupted_read_is_retried_and_other_errors_are_passed_on` |
| L8 | `preamble_reader.rs` | wait out WouldBlock/TimedOut inside the reader | killed (2) | `a_line_interrupted_by_would_block_continues_on_the_next_call`, `a_socket_read_timeout_is_an_io_outcome_and_the_line_continues_afterwards` |

77 changes applied; 77 made at least one test fail; 0 did not.
