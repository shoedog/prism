## Script-repair round (the one allowed; used before the census of record)

The first census pass (axum, ripgrep) was inspected for script defects before any count was used:

1. **`implied` was purely syntactic.** `T = any(feature="__private", test)` under a `cfg(test)` caller was "not implied" (569 axum pairs). The ratified contract assumes *implied* conditions true, so `implied` is now a sound implication from the caller's conjuncts (an `any` with an implied member, an `all` of implied members; `cfg_attr(q, cfg(p))` via `p`). The syntactic version is still reported (`implied_syntactic`, `A_syntactic` in the summaries).
2. **std-trait name list** contained names that are not std trait methods (`get get_mut iter iter_mut is_empty`); removed before any row was read under it.
3. **cfg-wrapper macros.** tokio declares most modules inside its own `cfg_*! { ... }` macros (`($($item:item)*) => { $( #[cfg(P)] $item )* }`); 289 tokio files (3,556 of 5,910 Exact pairs) had no parent. The census now expands that one macro shape (origin `macro_cfg`; 75 macros, 350 expansions) and `automod::dir!` (clap tests). tokio orphans: 289 → 9 files.
4. Alternatives no longer include the target itself when a file is loaded under two contexts.

All §2.4 fixtures and the matrix were re-validated after the repair (`validation/validation-report.json`, 0 mismatches).

**Adjudicator defect fixed during the adjudication round (disclosed; not a census change):** cargo stops scheduling other targets after the first failing one, so each E0308 batch type-checked only one test target. Added `--keep-going`; the tokio adjudication was rerun from scratch with it. The first run's outputs are not used. This sits outside the design's one repair round, which the census had already used. The owner may treat it as a second repair.

## Adjudication (one round; stopped at rule 5)

tokio's 72 sampled sites were adjudicated: 58 (a)∧¬implied, 2 (a)∧implied, 9 (b-std), 1 (b-other). The other 22 sampled sites (uv 5, ruff 5, prism 5, ripgrep 3, axum 2, clap 2) were **not** adjudicated: the §4 outcome was already fixed (see DECISION.md) and the brief stops at escalation.

Configurations (x86_64-unknown-linux-gnu, dev profile, `RUSTFLAGS=--cap-lints=warn`, `cargo check --keep-going -p tokio -p tokio-util`):
`C0` = `--lib --bins` (default features); `C_test` = `--all-targets` (**baseline red**: tokio-util tests need non-default features, so INADMISSIBLE for every site); `C_flip` = `--all-targets --features tokio/full,tokio/test-util,tokio-util/full`; `C_unstable` = `C_flip` + `tokio/tracing` + `--cfg tokio_unstable`; `C_uring` = `C_unstable` + `tokio/io-uring,tokio/taskdump`. Every adjudicated site had at least one green configuration.

Probes: deprecation marker (target and every in-repo alternative, trait-impl targets via their in-repo trait declaration) + E0308 found-type, and, for every WRONG candidate, a declared-type control (`{ let _p527c: <prism's target type> = CALL; unreachable!() }`) in `C_uring`. Positive controls (3 sites known correct) produced no E0308 at the site.

| group | sites | correct | WRONG (any cause) | WRONG, cfg-caused | other |
|---|---:|---:|---:|---:|---|
| (a) ∧ ¬implied | 58 | 50 | **7** | **0** | 1 config_unreachable (bsd aio) |
| (a) ∧ implied (control) | 2 | 2 | 0 | 0 | |
| (b-std) | 9 | 6 | 0 | 0 | 2 indistinct (std-trait impl targets), 1 config_unreachable (loom) |
| (b-other) | 1 | 1 | 0 | 0 | |

(a)∧¬implied, decided n = 57: any-cause WRONG 7/57 = 12.3%, Wilson 95% [6.1%, 23.2%]. cfg-caused WRONG 0/57, Wilson upper 6.3%. With pooled A = 120: W⁺(any cause) = 27.9, W⁺(cfg-caused) = 7.6.

**All 7 WRONGs are wrong in every configuration that compiles the site.** None is the #527 mechanism, where a cfg-disabled target is reported while a cfg-enabled alternative wins. Causes:
- 5 × **std path resolved to a same-named in-repo type**: `std::fs::OpenOptions::new` ×3 → `tokio::fs::OpenOptions::new`; `.read` on that std receiver (`constructor_local`); `sync::RwLock::new`, where `sync` is `std::sync`, → `tokio::sync::RwLock::new`.
- 2 × **owner-name collision with an impl that prism does not index** because it sits inside a cfg-wrapper macro: `scheduler::Handle::current` → `runtime::Handle::current`; `Timer::new` → `time_alt::Timer::new`, where rustc binds `runtime::Timer::new`.

F2 would demote all 7, because the census finds their competitors. It would also demote the 50 correct sites and the other ~57 unsampled (a)∧¬implied tokio pairs.

### Per-site verdicts (adjudicated)

| group | class | rung | callee | site | verdict | per configuration | cause |
|---|---|---|---|---|---|---|---|
| a_implied_control | a2 | self_receiver | `as_socket` | tokio/src/net/udp.rs:2225 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_implied_control | a1 | constructor_local | `assert_num_drops` | tokio/tests/sync_set_once.rs:77 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `Timer::new` | tokio/src/time/sleep.rs:375 | **WRONG** | C0:dead_in_config C_flip:WRONG_candidate C_test:INADMISSIBLE C_unstable:WRONG_candidate C_uring:WRONG_candidate | owner-name collision: rustc binds runtime::Timer::new (impl inside a cfg-wrapper macro prism does not index); target is runtime::time_alt::Timer |
| a_not_implied | a2 | constructor_local | `read` | tokio/src/net/unix/pipe.rs:262 | **WRONG** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:dead_in_config C_uring:dead_in_config | non_cfg: receiver is std::fs::OpenOptions (constructor_local follows the std::fs::OpenOptions::new mis-resolution) |
| a_not_implied | a1 | qualified_owner | `scheduler::Handle::current` | tokio/src/io/async_fd.rs:247 | **WRONG** | C0:dead_in_config C_flip:unindexed_binding C_test:INADMISSIBLE C_unstable:unindexed_binding C_uring:unindexed_binding | owner-name collision: rustc binds runtime::scheduler::Handle::current, an impl inside a cfg-wrapper macro prism does not index; target is a different type (runtime::Handle) |
| a_not_implied | a1 | qualified_owner | `std::fs::OpenOptions::new` | tokio/tests/net_unix_pipe.rs:240 | **WRONG** | C0:dead_in_config C_flip:unindexed_binding C_test:INADMISSIBLE C_unstable:unindexed_binding C_uring:unindexed_binding | non_cfg: std path owner resolved to same-named in-repo type (tokio::fs::OpenOptions) |
| a_not_implied | a1 | qualified_owner | `std::fs::OpenOptions::new` | tokio/tests/net_unix_pipe.rs:199 | **WRONG** | C0:dead_in_config C_flip:unindexed_binding C_test:INADMISSIBLE C_unstable:unindexed_binding C_uring:unindexed_binding | non_cfg: std path owner resolved to same-named in-repo type (tokio::fs::OpenOptions) |
| a_not_implied | a1 | qualified_owner | `std::fs::OpenOptions::new` | tokio/src/net/unix/pipe.rs:261 | **WRONG** | C0:dead_in_config C_flip:unindexed_binding C_test:INADMISSIBLE C_unstable:unindexed_binding C_uring:unindexed_binding | non_cfg: std path owner resolved to same-named in-repo type (tokio::fs::OpenOptions) |
| a_not_implied | a1 | qualified_owner | `sync::RwLock::new` | tokio/src/loom/std/rwlock.rs:12 | **WRONG** | C0:unindexed_binding C_flip:unindexed_binding C_test:INADMISSIBLE C_unstable:unindexed_binding C_uring:unindexed_binding | non_cfg: `sync` is std::sync here; owner resolved to tokio::sync::RwLock |
| a_not_implied | a1 | qualified_owner | `scheduler::Handle::current` | tokio/src/io/bsd/poll_aio.rs:143 | **config_unreachable** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:dead_in_config C_uring:dead_in_config |  |
| a_not_implied | a1 | qualified_owner | `Handle::current` | tokio/tests/dump.rs:41 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:dead_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `Handle::current` | tokio/tests/rt_handle_block_on.rs:124 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `Handle::current` | tokio/tests/rt_handle_block_on.rs:132 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `Handle::current` | tokio/tests/rt_handle_block_on.rs:134 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `Handle::current` | tokio/tests/rt_handle_block_on.rs:112 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `crate::runtime::Handle::current` | tokio/src/fs/read.rs:69 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:dead_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `crate::runtime::Handle::current` | tokio/src/fs/write.rs:38 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:dead_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `length_delimited::Builder::new` | tokio-util/tests/length_delimited.rs:681 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `length_delimited::Builder::new` | tokio-util/tests/length_delimited.rs:115 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `length_delimited::Builder::new` | tokio-util/tests/length_delimited.rs:416 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `length_delimited::Builder::new` | tokio-util/tests/length_delimited.rs:310 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `length_delimited::Builder::new` | tokio-util/tests/length_delimited.rs:665 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `length_delimited::Builder::new` | tokio-util/tests/length_delimited.rs:326 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `length_delimited::Builder::new` | tokio-util/tests/length_delimited.rs:97 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `length_delimited::Builder::new` | tokio-util/tests/length_delimited.rs:560 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `length_delimited::Builder::new` | tokio-util/tests/length_delimited.rs:358 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `length_delimited::Builder::new` | tokio-util/tests/length_delimited.rs:285 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `length_delimited::Builder::new` | tokio-util/tests/length_delimited.rs:379 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | stem_single | `oneshot::channel` | tokio/src/runtime/blocking/shutdown.rs:22 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `pipe::OpenOptions::new` | tokio/tests/net_unix_pipe.rs:90 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `pipe::OpenOptions::new` | tokio/tests/net_unix_pipe.rs:55 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `pipe::OpenOptions::new` | tokio/tests/net_unix_pipe.rs:360 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `pipe::OpenOptions::new` | tokio/tests/net_unix_pipe.rs:145 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `pipe::OpenOptions::new` | tokio/tests/net_unix_pipe.rs:115 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `pipe::OpenOptions::new` | tokio/tests/net_unix_pipe.rs:408 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `pipe::OpenOptions::new` | tokio/tests/net_unix_pipe.rs:290 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `pipe::OpenOptions::new` | tokio/tests/net_unix_pipe.rs:258 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `pipe::OpenOptions::new` | tokio/tests/net_unix_pipe.rs:361 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `pipe::OpenOptions::new` | tokio/tests/net_unix_pipe.rs:409 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a2 | typed_param | `poll_read` | tokio/src/io/util/read_to_end.rs:87 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a2 | qualified_owner | `read` | tokio/tests/fs_uring_cancel_open.rs:44 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:dead_in_config C_uring:correct_in_config |  |
| a_not_implied | a2 | qualified_owner | `read` | tokio/tests/fs_uring_runtime_shutdown.rs:48 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:dead_in_config C_uring:correct_in_config |  |
| a_not_implied | a2 | qualified_owner | `read` | tokio/tests/fs_uring.rs:122 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:dead_in_config C_uring:correct_in_config |  |
| a_not_implied | a2 | qualified_owner | `shutdown` | tokio-util/tests/task_join_map.rs:664 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a2 | qualified_owner | `shutdown` | tokio/tests/task_join_set.rs:615 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a2 | qualified_owner | `shutdown` | tokio-util/tests/task_join_map.rs:611 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a2 | qualified_owner | `shutdown` | tokio/tests/task_join_set.rs:472 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a2 | qualified_owner | `shutdown` | tokio-util/tests/task_join_queue.rs:142 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a2 | qualified_owner | `shutdown` | tokio-util/tests/task_join_map.rs:752 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a2 | qualified_owner | `shutdown` | tokio/tests/task_join_set.rs:526 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `sync::Barrier::new` | tokio/tests/tracing_sync.rs:40 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `sync::Mutex::new` | tokio/tests/tracing_sync.rs:84 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `task::Builder::new` | tokio/tests/tracing_task.rs:165 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `task::Builder::new` | tokio/tests/tracing_task.rs:90 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `task::Builder::new` | tokio/tests/tracing_task.rs:67 | **correct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `tokio::runtime::Handle::current` | tokio/tests/macros_test.rs:98 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `tokio::runtime::Handle::current` | tokio/tests/task_local_set.rs:411 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `tokio::runtime::Handle::current` | tokio/tests/rt_threaded.rs:682 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `tokio_test::io::Builder::new` | tokio/tests/io_copy_bidirectional.rs:135 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| a_not_implied | a1 | qualified_owner | `tokio_test::io::Builder::new` | tokio/tests/io_copy_bidirectional.rs:123 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| b_other | b-other | qualified_owner | `spawn_local` | tokio/tests/task_local_set.rs:266 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| b_std | b-std | qualified_owner | `len` | tokio/src/runtime/tests/loom_join_set.rs:18 | **config_unreachable** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:dead_in_config C_uring:dead_in_config |  |
| b_std | b-std | qualified_owner | `len` | tokio-util/tests/task_join_map.rs:191 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| b_std | b-std | qualified_owner | `len` | tokio-util/tests/task_join_map.rs:168 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| b_std | b-std | qualified_owner | `len` | tokio/tests/task_join_set.rs:153 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| b_std | b-std | qualified_owner | `len` | tokio-util/tests/task_join_map.rs:182 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| b_std | b-std | qualified_owner | `len` | tokio-util/tests/task_join_queue.rs:247 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| b_std | b-std | qualified_owner | `len` | tokio-util/tests/task_join_map.rs:159 | **correct** | C0:dead_in_config C_flip:correct_in_config C_test:INADMISSIBLE C_unstable:correct_in_config C_uring:correct_in_config |  |
| b_std | b-std | qualified_owner | `LogHistogram::default` | tokio/tests/rt_unstable_metrics.rs:449 | **indistinct** | C0:dead_in_config C_flip:dead_in_config C_test:INADMISSIBLE C_unstable:indistinct C_uring:indistinct |  |
| b_std | b-std | qualified_owner | `SetOnce::from` | tokio/tests/sync_set_once.rs:84 | **indistinct** | C0:dead_in_config C_flip:indistinct C_test:INADMISSIBLE C_unstable:indistinct C_uring:indistinct |  |

## Other findings and disclosures

- **Cross-language Exact pairs (not cfg):** 1,082 pooled Rust-target Exact pairs have a non-Rust caller, for example a Python `len(...)` → a Rust `fn len` in `tests/fixtures/...` via `free_single`. By corpus: prism py 982, go 17, js 11, ts 11, sh 4, tf/java/c 1 each; ruff py 37; uv py 16; ripgrep sh 1. They are counted in E as the unit definition requires. One (ruff, b-std) was sampled and not adjudicated.
- **Not seen by the census** (§2.3): cfgs set only by RUSTFLAGS are treated as false in C0 (`tokio_unstable`, `loom`, `docsrs`); `cfg_if!` (0 files in all corpora); macros other than the wrapper shape and `automod::dir!`; alternatives in dependency crates other than the std trait-name list. Atoms emitted by build.rs are `unknown` under C0.
- **Over-count by design:** alternatives match by the type's last path segment across the whole corpus (e.g. `Builder::new` on 4 unrelated `Builder`s). Most of tokio's 114 are this. Adjudication found 50/57 correct.
- **Configurations not reachable here:** only x86_64-unknown-linux-gnu is installed, so windows/macos/bsd conditions were not adjudicated. tokio's `--all-targets` with default features is red. loom was not built.
- **Fixture f2** (`Self::b`) is reconstructed from the #524 review description; the original counterexample file is not in this repository.
- Network: no fetch failures (crates.io and GitHub reachable; `cargo check` online).
