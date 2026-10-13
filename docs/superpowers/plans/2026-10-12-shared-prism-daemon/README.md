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
| S2 | Identity key, runtime-dir validation, preamble encode/decode, connection-local settings | S1 | [`S2-identity-preamble.md`](S2-identity-preamble.md) — **written and validated; not started**. About 4,000 added lines (1,400 code, 2,600 tests), so it ships as two PRs: **S2a** = Tasks 1–4 (runtime directory, cache base, identity key, file paths), **S2b** = Tasks 5–8 (client settings, preamble, bounded reader, proof). Waiting on the owner questions below. |
| S3a | Readiness coordinator, `RwLock<Option<SessionProvider>>`, test-only status probe (tests 1, 14) | S2 | written when S2 merges |
| S3b | Concurrent dispatch, refresh visibility, auto-refresh recheck, panic boundaries (tests 6, 7, 9, 12) | S3a | written when S3a merges |
| S4 | Daemon ownership, bounded handshake stage, admission/retirement, Draining (tests 2, 3, 4, 8, 13, 15, 16) | S3b | written when S3b merges |
| S5a | Shim: connect, arbitration, spawn, fail-closed, pump (tests 5, 10, 11) | S4 | written when S4 merges |
| S5b | `daemon-status`, `daemon-shutdown` | S5a | written when S5a merges |
| S6 | Opt-in launcher/config changes, docs, soak instrumentation | S5b | written when S5b merges |
| C2-* | One PR per reduction candidate the C1 readout advances: M2 and M1 (M1a, M1b). M3 did not advance and gets no plan. | C1; S5a to ship | not written. The M2 plan can be written now (its base is the load peak). The M1 plan waits on the steady-state metric question below. |
| B | Default flip | S6 soak | owner decision after the soak |

**S1 and C1 run in parallel.** They share no file except one `mod` line each in `src/mcp/mod.rs` (different
positions). C1 deliberately consumes only the public `serve_session_with_freshness` and `Transport` API so it does
not depend on S1's refactor of `transport.rs`.

## Owner questions raised by S2 planning (2026-10-13)

The S2 plan records 26 decisions it made where the spec was silent (table D1–D26 in the plan) and seven spec
issues it did not resolve (SI-1–SI-7). These four change what gets built and are the owner's to settle before S2
is implemented:

1. **Runtime directory on multi-user Linux (SI-1).** With no `$XDG_RUNTIME_DIR` and no override, spec §4.3 gives
   `/tmp/prism-mcpd`: one name for every user. The first user owns it and every other user's shim fails closed.
   macOS is unaffected. Options: keep as written (the error names `PRISM_MCP_RUNTIME_DIR` as the way out), or use
   a per-user name in the temp-dir fallback (`prism-mcpd-<uid>`), which is a spec change.
2. **A client-supplied result cap below the floor (D15).** The plan refuses it. The alternative is to replace it
   with the default as the environment path does, but a daemon's client cannot see the stderr warning, so the
   replacement would be silent.
3. **Where the peer-uid check lives (D24).** The plan leaves `getpeereid`/`SO_PEERCRED` to S4, where connections
   are accepted. Moving it into S2 would put all of "who may connect" in one slice at a cost of about 20 lines.
4. **`--warm-at-startup` across a shared daemon (SI-7).** The spec says it is honored only by the daemon this
   shim starts, but the daemon has no such flag and cannot tell which `HELLO` came from its starter. Either the
   daemon command line gets the flag, or the first admitted `HELLO` decides. This one is S4's, not S2's; S2 only
   carries the field.

Recorded, no decision needed now: a debug and a release binary of one source tree share a daemon (SI-4); the
runtime-directory check is by path, so a rename between check and bind is possible for a process with write
access to its parent (SI-5); the spec defines no reply for a `HELLO` refused for a reason other than identity
(SI-3, S4 decides).

## Owner questions raised by the C1 readout (2026-10-13)

5. **The steady-state metric for C2 experiments.** Spec §6.1 pins `phys_footprint` after a 5 s settle. In the
   readout that figure sat at one of two levels after a cache load, 460–790 MiB apart on slicing and prometheus,
   and the settle time did not decide which: 35 of 36 runs at 5 s were at the higher level, 4 of 13 at 60 s were
   at the lower. Under the pinned noise rule a candidate could win or lose on that accident. What moves a process
   between the levels was not identified, and forcing it was not tried. Options:
   - (a) Steady live heap (`heap_in_use_bytes` at the settle stage) as the primary metric. Repeatable to
     0.13 MiB for a cache-loaded index, no code change. It is an allocation metric: for M1 it is circular (the
     M1 estimate is part of the live heap, so the experiment could not fail and would not show the host gets
     memory back), and it needs a minimum effect size pinned with it, because the noise rule passes on any change.
   - (b) Keep the footprint and have report mode ask the allocator to release free memory before the settled
     sample. A harness change. Untested: it would have to be shown to work, and a forced release can differ
     from what a serving daemon does.
   - (c) Keep the footprint and tag each run with its level, which the saved reports already allow (footprint
     minus live heap is at least 157 MiB at one level and at most 14 MiB at the other). No code change;
     compares like with like; does not say which level a daemon lives at.
   - Recommendation: live heap with a pinned minimum effect size as the repeatable gate, **and** a footprint at
     a known level (c, or b once tested) as a required confirmation for M1. M2 is judged on the load peak and
     does not depend on this question.
   A side result for the owner to weigh: if the higher level persists in a serving daemon, releasing free memory
   after a load is itself worth 157–715 MiB on these corpora. It is unranked until someone tests whether the
   process can do it.
6. **Kubernetes.** It was not run: the four measured corpora project its first build at 27–70 GiB on a 24 GiB
   host. A fifth corpus cannot change a verdict (M1 and M2 have four where two are needed; M3 has none). Options:
   accept four corpora (the recommendation), run a subtree of kubernetes, or run it on a larger host.
7. **How S2 is implemented** (a process question, open since S2 planning): start from the validated branch
   `scratch/s2-validate` as S1 and C1 did, or have implementers work the plan from scratch.

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
| On macOS 26 the footprint of a cache-loaded `prism-mcp` sits at one of two levels, 157–715 MiB apart on the four corpora, with the same live heap at both. The move to the lower level did not happen at a reproducible time: during the load in two runs, after about 18 s in one, not within 60 s in nine of thirteen. The mechanism was not identified and forcing the move was not tried. | C1 readout, 2026-10-13: `run-1`, the 60 s supplement and the normal-mode probe under `~/prism-evidence/mcp-memory/c1/`. | A footprint sampled seconds after a load is not repeatable. Where the lower level was observed (slicing, prometheus) idle is near the live heap; size a host from the refresh peak, which is about twice the footprint. Pending owner question 5, do not gate a C2 experiment on a footprint whose level is unknown. |
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
- **S5a:** the transcript driver in `tests/mcp/transcript.rs` (S1) is the tool for the "same snapshot, same
  settings, same readiness state" equivalence in spec §5: run one script standalone and through the shim and compare.
- **S6:** the launcher is `scripts/prism-mcp-launch.sh`; it passes extra arguments through, so `--shared` is a config
  change, not a script change.
