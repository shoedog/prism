# Shared per-repo prism daemon — lane plan

Date: 2026-10-12 · Spec: [`docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md`](../../specs/2026-10-09-shared-prism-daemon-design.md)
(v3, owner-approved 2026-10-12) · Predecessor: #356 (main `cb0eeec0`).

**Goal (the one every slice and every review brief is measured against):** let the a2a-bridge run its reviewer and
implementer fan-out on this host without swap exhaustion, by holding **one resident index per compatible
(checkout, prism build, cache configuration, refresh policy)** instead of one per agent thread, with no query-latency
regression under fan-out.

**Owner decisions (spec §11, 2026-10-12):** idle TTL 15 min · opt-in `--shared` first, default flips only after the
§8 soak gate · graceful, instance-bound `daemon-shutdown` verb.

## Slices and plan status

A slice plan is written when its predecessor's interfaces exist on `main`, because each slice's code is argued from
the previous slice's real signatures. Writing S2–S6 task-level plans now would mean guessing those signatures.

| Slice | Content (spec §12) | Depends on | Plan |
|---|---|---|---|
| **S1** | Transport/framing refactor, explicit `ConnectionSettings`, control/dispatch split, standalone byte-for-byte proof | #356 | [`S1-transport-refactor.md`](S1-transport-refactor.md) — **MERGED #361** (`f6354e07`, 2026-10-13) |
| **C1** | `PRISM_MCP_MEM_REPORT` harness, attribution counters, five-corpus readout | none | [`C1-memory-harness.md`](C1-memory-harness.md) — harness **MERGED #362** (`7db8f195`, 2026-10-13); review changed what is measured, see the note at the top of that plan. Readout: [`2026-10-prism-mcp-memory-readout.md`](../../../analysis/2026-10-prism-mcp-memory-readout.md), four corpora (kubernetes not run: projected past this host's memory). **M1 and M2 advance on all four; M3 does not advance.** The readout also found that the spec's pinned steady-state metric is not repeatable; see the owner questions below. |
| S2 | Identity key, runtime-dir validation, preamble encode/decode, connection-local settings | S1 | [`S2-identity-preamble.md`](S2-identity-preamble.md) — **S2a MERGED #364** (`802844a8`, 2026-10-13; from `scratch/s2-validate`, owner decision 7; review made runtime-directory validation stricter than spec §4.3, see the notes for S4 and S5a below). **S2b MERGED #370** (`8587c7b9`, 2026-10-13; both reviewers approved). About 4,000 added lines (1,400 code, 2,600 tests), so it ships as two PRs: **S2a** = Tasks 1–4 (runtime directory, cache base, identity key, file paths), **S2b** = Tasks 5–8 (client settings, preamble, bounded reader, proof). Waiting on the owner questions below. |
| S3a | Readiness coordinator, `RwLock<Option<SessionProvider>>`, test-only status probe (tests 1, 14) | S2 | [`S3a-readiness-coordinator.md`](S3a-readiness-coordinator.md) — **MERGED** (plan #373 `4e2e9c3c`; S3a-1 #374 `e7dc1cc2`; S3a-2 #377 `14e6f66c`, 2026-10-13). The owner's `indexing` answer while a retry builds, served at once when the index is ready (D26 as amended); a timed-out client's next call hears its own attempt's failure (D32, spec §4.2 clarified). S4 owes parity for a failed startup attempt under `--warm-at-startup`. |
| S3b | Concurrent dispatch, refresh visibility, auto-refresh recheck, panic boundaries (tests 6, 7, 9, 12) | S3a | [`S3b-concurrent-dispatch.md`](S3b-concurrent-dispatch.md) — **MERGED** (plan #384 `c3e1d3c3`; S3b-1 #385 `c19ba553`; S3b-2 #386 `a9ac0a63`, 2026-10-14). The automatic refresh re-checks its trigger under the write lock and answers under the same access (D34–D36); a response write held at a gate stalls no other client; a panic in dispatch, the build or the control plane aborts the daemon while its lock is still held (D39–D47), one elsewhere on a connection's thread closes only that connection (`serve_bounded`). Recorded for the owner's attention, not owed: SI-10 (a builder panic is fatal by the spec, unlike in process; S5a must bound the restart loop) and SI-11 (a refresh holds the write lock for a whole rebuild; measure at the §6.2 fan-out gate). |
| S4 | Daemon ownership, bounded handshake stage, admission/retirement, Draining (tests 2, 3, 4, 8, 13, 15, 16) | S3b | next; inputs: S3b plan "Interfaces for the next slices" (lock, then guard, for every new control-plane entry), S3a's Background-start parity requirement |
| S5a | Shim: connect, arbitration, spawn, fail-closed, pump (tests 5, 10, 11) | S4 | written when S4 merges |
| S5b | `daemon-status`, `daemon-shutdown` | S5a | written when S5a merges |
| S6 | Opt-in launcher/config changes, docs, soak instrumentation | S5b | written when S5b merges |
| C2-driver | `memreport`: `run --kinds`, a `build` kind (first build that writes the cache, repeated), `compare` under a pinned rule | C1 | brief [`C2-M2-driver-brief.md`](C2-M2-driver-brief.md); **MERGED #367** (`c785fa4f`, 2026-10-13). `scripts/memreport/README.md` is the reference for the pinned rule. |
| C2-M2 | Stream `cpg-cache.bin` on save and on load; move instead of clone on load | C2-driver; shipping gate per spec §6.2 | [`C2-M2-cache-streaming.md`](C2-M2-cache-streaming.md) (**MERGED #366**). Experiment on the full candidate (draft #369): load peak −19 to −24 % on four corpora, first-build peak −20 to −27 %, live heap lower; load and build time not resolvable on a loaded host. Owner, 2026-10-13: ship the **save half** now — **MERGED #380** (`88f92603`, 2026-10-14; after two review rounds found an open class in the stale-temp sweep, the owner chose a per-directory lock file instead) and measure the rest at 06:00 2026-10-14: the save half's build time, and a **memory-mapped load half** (`perf/cpg-cache-mmap-load`) against it. The fan-out latency gate is waived for M2 only (spec §11 item 11). |
| C2-M1 | Drop retained tree-sitter trees (M1a, M1b) | C2-driver; S5a to ship | not written. Metric decided (spec §6.1 as revised): live heap with a minimum effect size, plus a footprint confirmation. M3 did not advance and gets no plan. |
| B | Default flip | S6 soak | owner decision after the soak |

**S1 and C1 run in parallel.** They share no file except one `mod` line each in `src/mcp/mod.rs` (different
positions). C1 deliberately consumes only the public `serve_session_with_freshness` and `Transport` API so it does
not depend on S1's refactor of `transport.rs`.

## Owner decisions of 2026-10-13 (questions raised by S2 planning and by the C1 readout)

Recorded in spec §11 items 4–10. The full text of each question is in this file's history (`e2701e89`).

| # | Question | Decision |
|---|---|---|
| 1 | Runtime directory on multi-user Linux (SI-1) | **As the spec says.** `/tmp/prism-mcpd` for every user; a second user's shim fails closed with an error naming `PRISM_MCP_RUNTIME_DIR`. |
| 2 | A client-supplied result cap below the floor (D15) | **Open.** The owner asked what it means (answered in the session). S2b is built to refuse it, the plan's reading. Not a two-line change, as first recorded: the decoder accepts a line only if it equals the re-encoding of what it decoded (D18), so a cap replaced by the default no longer matches the line that carried it. Replacing needs the canonical comparison made before the settings are normalised, three test rewrites, and a way for the client to learn its cap was replaced (`READY` echoes no settings). Both reviewers of S2b recommend keeping "refuse": a conforming shim resolves its settings from the environment as a standalone server does (warning on its own stderr, default applied), so a user's misconfiguration behaves as it does today and only a non-conforming client is ever refused. |
| 3 | Where the peer-uid check lives (D24) | **S4**, where connections are accepted. |
| 4 | `--warm-at-startup` on a shared daemon (SI-7) | **A flag on the daemon's command line**, passed by the starting shim. The preamble field stays as information only. S4 adds the flag; S2 is unchanged. |
| 5 | Steady-state metric for C2 experiments | **Live heap with a pinned minimum effect size as the gate, plus a footprint at a known level as a required confirmation for M1.** M2 is judged on a peak. Spec §6.1 is revised. |
| 6 | Kubernetes | **Accept the four measured corpora.** |
| 7 | How S2 is implemented | **From the validated branch `scratch/s2-validate`.** |
| 8 | What a call is told when its failed build attempt has been superseded by a retry (S3a, D26) | **A retryable `indexing` status** (owner: "Building...retry in <retry interval seconds> or ignore if you retried and call succeeded", with "indexing" in place of "building", as IDEs say); daemon only. A call whose failure has not been superseded hears its own failure. |
| 9 | M2 with time criteria unresolved on a loaded host | **Ship the save half now; measure the other half at 06:00 MDT**, a quiet hour. |
| 10 | Superseded call when the index is already ready; retry interval | **Served at once**; **5 s** ("right without further evidence"). |
| 11 | M2 save half: patch the temp sweep again, or change the design | **A per-directory lock file** (no sweep), with orphan temp cleanup on load. |
| 2 (closed) | Sub-floor result cap | Settled as **refuse**, as merged in S2b. |

Still recorded with no decision needed: a debug and a release binary of one source tree share a daemon (SI-4); the
runtime-directory check is by path, so a rename between check and bind is possible for a process with write
access to its parent (SI-5); the spec defines no reply for a `HELLO` refused for a reason other than identity
(SI-3, S4 decides). A side result of the readout is also unranked: if the higher footprint level persists in a
serving daemon, releasing free memory after a load is worth 157–715 MiB on the four corpora; nobody has tested
whether the process can do it.

## Facts measured on 2026-10-12 (branch `spec/shared-prism-daemon`, tree = main `cb0eeec0` + spec commits)

Each is something a slice plan relies on. Re-measure if the code has moved. `origin/main` advanced to `bab0dfbd`
during planning; `git diff cb0eeec0 bab0dfbd -- src/mcp tests/mcp Cargo.toml` is empty, so these still hold there.
Tool output for JS/TS can differ on the newer main, which is why S1 records its goldens on the commit it branches
from rather than reusing the prototype's.

| Fact | How it was measured | Consequence |
|---|---|---|
| `SessionProvider` is `Send + Sync`. | Appended `const _: fn() = \|\| { fn send<T: Send>() {} fn sync<T: Sync>() {} send::<SessionProvider>(); sync::<SessionProvider>(); };` to `src/mcp/session.rs`; `cargo check --features mcp --lib` finished clean; edit reverted. | The spec's `RwLock<Option<SessionProvider>>` with concurrent readers (S3a) is viable. The comment above `build_state` saying `LoadedRepo` is not `Sync` is stale; S3a replaces it with a permanent compile-time assertion. |
| Output settings are read from the environment on **every** `tools/call` today (`call_tool_response`, `src/mcp/transport.rs`), so an invalid value warns on stderr once per call. | Read of the code; base binary with `PRISM_MCP_CONCISE_SHAPE=bogus` and two calls printed the warning twice. | S1 resolves settings once per server. Stdout is unchanged; the stderr warning cadence changes from per-call to once. S1 tests and discloses this. |
| The line framing is already generic: `StdioTransport<R: BufRead, W: Write>`. | Read of the code. | `UnixStreamTransport` is an instantiation of the same type, not a second implementation (spec §4.5 "shared, not copied"). |
| The resolved call-edge index is built lazily on the first caller/callee query (`NavigationIndex::resolved_call_edges`, a `OnceLock`). | Read of `src/navigation/mod.rs`. | C1 must force it to get an "after call-edge index" stage. Under S3b's read lock, the first query pays the build while other readers wait on the `OnceLock`, not on the provider lock. |
| A scripted 25-response session against the base binary is byte-identical across runs and temp directories, in five modes (eager, lazy, always+legacy, auto-full, auto-incremental with a 20,000 cap); no temp path appears in any response. | Prototype driver against `target/release/prism-mcp` built at `cb0eeec0`, two runs per mode in different temp dirs. | S1's golden-transcript proof is sound. The only build-dependent bytes are `serverInfo.version`. |
| `libc` 0.2.183 is in `Cargo.lock` transitively but is not a direct dependency. It has `task_info`, `mach_task_self`, `proc_pid_rusage`, `rusage_info_v4`, `getrusage`, `flock`, `getpeereid`, `setsid`; it has no `task_vm_info` struct. | Read of the vendored crate; a scratch program using a hand-declared rev1 `task_vm_info` matched `ps -o rss` to the byte and tracked a 200 MiB allocation. | C1 adds `libc` as an optional dependency of the `mcp` feature; S2/S4 reuse it. C1 declares the struct itself. |
| Freed memory can stay in `phys_footprint`: after dropping a touched 200 MiB buffer the footprint did not fall. | Same scratch program. | C1 never attributes by drop-and-measure deltas (spec §6.1 "independent of RSS deltas"). |
| On macOS 26 the footprint of a cache-loaded `prism-mcp` sits at one of two levels, 460–790 MiB apart on slicing and prometheus (the lower level was not observed on excalidraw or ruff), with the same live heap at both. At the higher level the footprint exceeds the live heap by 157–715 MiB across the four corpora. The move to the lower level did not happen at a reproducible time: during the load in two runs, after about 18 s in one, not within 60 s in nine of thirteen. The mechanism was not identified and forcing the move was not tried. | C1 readout, 2026-10-13: `run-1`, the 60 s supplement and the normal-mode probe under `~/prism-evidence/mcp-memory/c1/`. | A footprint sampled seconds after a load is not repeatable. Where the lower level was observed (slicing, prometheus) idle is near the live heap; size a host from the refresh peak, which is about twice the footprint. Do not gate a C2 experiment on a footprint whose level is unknown (spec §6.1 as revised 2026-10-13). |
| A cache load of prometheus takes 13.2 s (three runs, 13.17–13.24 s) where ruff, with a cache file three quarters the size, takes 1.6 s and slicing 1.0 s. | C1 `run-1`: `after_index.elapsed_ms − after_load_repo.elapsed_ms` of the warm runs. | Something in that load is not proportional to file size. Not investigated; outside M2. Whoever plans load-time work starts here. |
| `save_cache_with_topology` clones the whole index (node list, edge list, call graph, data-flow graph) and then holds the encoded file in one buffer before writing it; `load_cache_with_topology` holds the file's bytes until it returns, and `reconstruct_cpg` clones every decoded node and edge into the graph. | Read of `src/cpg_cache.rs` at `e2701e89`, 2026-10-13. | These are the three transients behind the load peak and the first-build peak. The C2-M2 plan removes all three without changing the file's bytes. |
| The five corpora exist and are clean: ruff `44f6d1865fd2`, excalidraw `0642e72cfa2d`, prometheus `505095b64b43`, kubernetes `8d34ac5ef882` (30,485 tracked files), plus slicing itself. | `git rev-parse` / `git status --porcelain` in `~/code/bench-repos/*`. | C1's readout pins these commits. Kubernetes needs a per-run time cap. |
| The mutation gate cannot select MCP code: `scripts/mutgate/mutgate.py` builds `--lib` and `--test <target>` with no feature flags, and `src/mcp` exists only under `--features mcp`. | Read of the driver. | The pre-merge gate is still run (it guards the rest of the crate) but its verdict says nothing about this lane's code. Recorded, not pursued here. |
| `cargo clippy --all-targets --features mcp -- -W clippy::all` already prints 259 warning lines on the base, and CI does not deny warnings. | Ran it. | The bar for each slice is "no new warning in a file the slice touches", not a clean run. |

## The plans' code was compiled and run before hand-off (2026-10-12)

Both slice plans were applied verbatim, block by block, to a throwaway worktree on `origin/main` `bab0dfbd`. The
results were on two local branches, which became the starting points of #361 and #362 and were deleted after
those merged (2026-10-13):

| Plan | Scratch branch | What ran | Result |
|---|---|---|---|
| S1, Tasks 1–4 | `scratch/s1-validate` `24ccfaba` | `cargo test --features mcp --lib mcp::`; `cargo test --features mcp --test mcp`; `cargo check --tests --features mcp,detached-owner-audit`; `cargo check`; clippy filtered to the touched files | 211 passed (195 base + 4 + 12); 8 passed; goldens recorded in Task 1 are byte-identical after Task 4; zero-line diff in `transport_tests.rs` and `owner_transport_tests.rs`; no warning in a touched file |
| C1, Tasks 1–6 | `scratch/c1-validate` `af033444` | same two test commands; `python3 -m unittest scripts/memreport/test_memreport.py`; the Task 6 driver smoke with the debug binary | 212 passed (195 base + 17); 10 passed; 19 Python tests OK; `DRIVER-SMOKE-OK` with 8 of 8 runs admissible |

Not run on either branch: the full suite, `cargo fmt --check` against the plans' exact text (the scratch files were
formatted with `cargo fmt`), CI on Linux (so the goldens' platform independence is still unproven), a release build,
S1 Task 5, and C1 Task 7 (the five-corpus measurement).

Four defects surfaced and are fixed in the plans: a borrow of a temporary in a C1 attribution test; a wrong Python
test count in C1; a wrong expectation in S1's transcript-comparator test; and S1 Task 3 ran its "prove the test
bites" experiment before committing, where restoring the file would have discarded the task's own work.

## How each slice is executed

These follow the owner's standing preferences. Confirm or change them before S1 and C1 are dispatched.

- **Implementer:** one Sonnet (high) subagent per slice, in its own git worktree off `origin/main`, following the
  slice plan task by task with TDD. The implementer sees the slice plan and the spec.
- **Reviewers:** Opus 5.5 and gpt-6.1-sol on every slice PR, same brief, same commit. The brief states the lane goal
  above, asks for WRONG/SMELL and MATERIAL/IMMATERIAL tags, a recommended fix plus alternatives per finding, and a
  per-finding note on when it would not apply. Stance: senior developer optimizing the long-term outcome.
- **Review cap:** two rounds per slice PR, declared here. At the cap, classify: converging (fold and continue,
  disclosed in one line) or open-class (park and escalate). A reviewer disagreement with the controller goes to the
  owner with the tradeoffs.
- **Merge:** squash-merge on reviewer approval plus green CI, reported each time.
- **Verification before a slice is called done:** `cargo fmt --check`; `cargo test --features mcp` with totals;
  `cargo check` on default features; `cargo test --doc --features mcp`; no new clippy warning in touched files; the
  full mutation gate before merge with its coverage limit stated. Anything not run is named.
- **Custody:** commit at every green task. Measurement evidence (C1 raw reports) goes to `~/prism-evidence/` with the
  readout in `docs/analysis/`. The lane handoff is refreshed at each merged slice.

## Notes for the planners of later slices

- **S2:** `ConnectionSettings` (S1) carries the three output settings. The preamble adds `first_call_wait_secs` and
  `warm_at_startup` beside it; do not widen `ConnectionSettings` with fields the dispatch path never reads.
  **From the S1 review:** `ConnectionSettings` has public fields and no validating constructor. `from_env` floors
  the result cap at 12,000, but a cap below the floor reaching `cap_after_reserve` panics in debug builds and
  saturates silently in release. Once the cap arrives from a client in the preamble, decode it through one
  validating constructor that applies the `resolve_cap_from` rule; a panic in dispatch is daemon-fatal for every
  attached agent.
- **S3a:** the read path is `dispatch_tool_read(&call, &provider, &settings)` with `&SessionProvider` as the read-only
  facade (it sees the sticky `known_stale_after_refresh` evidence through `effective_stale_report`, with today's
  precedence). The coordinator is a new daemon type; `LazySessionProvider` stays as the standalone path.
  **From the S1 review:** `serve_connection` takes `&mut impl SessionRuntime`, which a lock-guarded provider
  cannot implement (`session(&self) -> &NavigationSession` cannot borrow from a guard), and `error_response` is
  private. Extract the loop with an injected tool-call step
  (`serve_loop(transport, registry, startup_mode, settings, |call| -> Value)`) and have `serve_connection` call
  it, so the daemon does not re-implement the malformed-input and `-32700` handling that spec §10 requires to
  match stdio. The first read on a fresh snapshot also builds the call-edge index and writes its cache sidecar:
  thread-safe behind a `OnceLock`, but a disk write on the read path.
- **S3b:** `ReadDispatch::NeedsAutoRefresh` is the hand-off point for the spec's "re-check the trigger after acquiring
  exclusive access". S1 does not re-check, because standalone has no interleaving; S3b adds the re-check inside the
  write-locked path.
  **From the S1 review:** `dispatch_tool_auto_refresh` runs the query handler while still holding `&mut provider`,
  so under a write lock the post-refresh query (including the lazy call-edge build on the fresh snapshot) blocks
  every reader. It splits mechanically into refresh (exclusive) and finish (shared) if S3b wants the query under
  shared access. `ReadDispatch::NeedsAutoRefresh` carries no generation and the facade exposes none; spec §4.2
  wants responses bound to the generation queried, so add a generation accessor (additive).
- **S4, from the S2a reviews:**
  - Runtime-directory validation (`shared::runtime_dir::validate_runtime_dir`) now also refuses a directory
    another user could replace (any directory on its resolution that is group/other-writable and not sticky,
    owned by a third user, or on macOS carrying a list entry that allows anything) and, on macOS, a runtime
    directory with any access control list. It judges the location before creating anything. It is still a
    check by path and does not examine mount options (a volume mounted to ignore ownership reports every file
    as the caller's). **It is hardening. The isolation guarantee between users is the peer-uid check**, which
    the daemon makes on every connection.
    Open the lock, pid and log files through a held directory descriptor (`openat`, `O_NOFOLLOW`); "the socket
    calls take paths" covers only the socket.
  - "Validate on every use" is a convention the types do not carry: `validate_runtime_dir` returns `()` and
    `DaemonPaths::new` takes any path. Route every use through one function that validates first, or change
    the return to a newtype that `DaemonPaths::new` requires.
  - If the runtime directory is removed under a live daemon (logind removing `/run/user/<uid>` at last logout,
    a temp cleaner), the lock file's inode is replaced and a second daemon can start for the same key until the
    first idles out. The daemon's periodic check can compare its socket's inode.
  - The key includes the content-hashed build identity and lock files are never unlinked, so every rebuild leaves
    five files per repository; on Linux the default directory is a small tmpfs and logs may reach 8 MB each.
    A cleanup rule is needed (S4 or S5b).
  - Delete-and-recreate of a repository root can reuse the inode (ext4, tmpfs): same key, and the root check
    passes. Only the freshness machinery notices.
  - `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY` (`src/navigation/call_edge_cache.rs`) changes which sidecar loads and is in
    neither the key nor the preamble; spec §7 says every answer-affecting setting is in one of them. A daemon
    would inherit it from whichever shim started it.
- **S4 and S5a, from the S2b reviews:**
  - Admission is a convention the types do not carry. `Hello` has public `identity` and `settings`, so a daemon
    can read the settings without calling `mismatch_against`; and nothing makes a shim compare `Ready.identity`
    with its own key. Give each side one call that does both (for example `Hello::admit(self, served) ->
    Result<ClientSettings, Reply>` and `Ready::is_for(&key)`), or keep exactly one call site each and let spec
    §9 test 10 pin it.
  - `PreambleLineReader::read_line` has no deadline. A socket read timeout bounds each silence, not the line: a
    peer sending one byte per timeout holds a pending slot for up to 4,097 timeouts, and pending slots count
    against `--max-clients`. The handshake stage needs a non-blocking socket polled against its own deadline
    (spec §4.2 already says so).
  - A `HELLO` whose settings are refused is an error before the caller sees its identity, so a client of
    another identity with refused settings cannot be answered `MISMATCH`. Decide with SI-3 whether every decode
    failure gets one generic refusal or the error carries the parsed key.
  - New verbs (S3a's status probe, S5b's `daemon-shutdown`) change the wire format and so, by S2's own rule,
    bump `SHARED_PROTOCOL_VERSION` and every key. Harmless before anything ships; decide before S6 whether
    control verbs are part of the versioned preamble.
  - A daemon builds its `ConnectionSettings` from `ClientSettings::output()` only; `ConnectionSettings` still
    has public fields.
- **S5a, from the S2a reviews:**
  - **The shim checks the uid of the socket's peer too.** Spec §4.4 has only the daemon check its peer. A
    listener another user planted can echo the key in `READY`; the shim would then pump the client's traffic
    to it. The runtime-directory checks make planting hard; the peer check makes it not matter. Two review
    rounds of S2a each found another way to plant (mode bits, lists, lists on ancestors, mount options), which
    is why this check, not more path checks, is where the guarantee has to live. It is a requirement on S5a
    that the spec does not yet state; the owner is asked to confirm it.
  - When the shim hands its key to the daemon, the daemon **checks** it against the key it computes and refuses
    on a difference; it does not adopt it. Adopting it would hide a root that was replaced between the two
    resolutions.
  - **Pass the shim's full key to the daemon it starts**, and have the daemon use or check it. The cache base of
    a directory that does not exist yet is compared as written, so on a case-insensitive filesystem a shim and
    its daemon can resolve different keys if the directory appears between the two resolutions; the shim then
    waits out its start deadline. `IdentityKey::from_hex` exists for this.
  - `DaemonIdentity::resolve` refuses a cache-free configuration and accepts an eager one although neither can
    be shared (spec §4.1 step 2). The command-line check rejects both before `resolve`, with a message that
    says what to change.
  - Paths in error messages are printed raw. A path with a newline or terminal control characters breaks the
    one-line diagnostic the shim promises; escape them where the line is written.
  - One `sudo -E` run that keeps the runtime base creates a root-owned `prism-mcpd`, after which the user's own
    shims fail with `NotOwned` until it is removed. The message already names the uid; the docs should say it.
- **S5a:** the transcript driver in `tests/mcp/transcript.rs` (S1) is the tool for the "same snapshot, same
  settings, same readiness state" equivalence in spec §5: run one script standalone and through the shim and compare.
- **S6:** the launcher is `scripts/prism-mcp-launch.sh`; it passes extra arguments through, so `--shared` is a config
  change, not a script change.
