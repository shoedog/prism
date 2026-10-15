# Go type extraction speedup validation — Tasks 1–3

Implementation lane: `perf/go-signature-imports`, initial clean HEAD/base/main/origin-main `a1aa755ba418a2eb48b61e3a204cbb82362b307a`. Scope is Tasks 1–3 only; performance acceptance and paired binaries (Task 4) remain controller-owned. No real-corpus prism-mcp run is authorized here. Performance goal is **unverified**.

## Execution protocol

Every cargo command is sequential, attempts the prescribed priority initially and continues with `CARGO_BUILD_JOBS=4` after its refusal, redirects output into `.review/go-type-extract/logs/`, and is read after completion. `run.py` records each command, Expected, falsifier, timestamps and exit code before/after execution; full logs remain authoritative. All test runs explicitly unset `PRISM_MCP_BLESS_GOLDEN`. An environment failure or compiler failure does not kill a behavioral mutant. No stash, amend, push, dependency changes, cache bumps, bless, or rebaseline.

Review cap: two rounds, controller dispatches Opus 5.5 and gpt-6-astra. Implementer dispatches no reviewers. Targeted mutations: one execution per M1–M4 after admission; any invalid probe must be corrected with its deviation recorded before retry.

## Rulings

- Ruling: use `a1aa755b` as the execution base/main — user pins this clone to the merged plan; main and origin/main already resolve there and the three Go source files have no diff from `8b2744b3` — cost if wrong: comparison attribution could miss upstream drift; the prescribed old-base downstream diff is retained too.
- Ruling: implementer makes the three task commits — explicit run override supersedes controller-only commit language; no amend/stash/push — cost if wrong: unintended publication history; inspect each staged diff and trailers.
- Ruling: force cargo offline through `CARGO_NET_OFFLINE=true` — sandbox prohibits installs/network and all required dependencies must be local — cost if wrong: missing local crates leave checks excluded for the controller rather than fetched.
- Ruling: no numeric limit on the existing go.rs — O1 is explicitly resolved in the plan; no mechanical split — cost if wrong: a separate bounded split would be required.

## Hypothesis / probe / result log

- H1 (inherited from the plan): repeated signature import scans cause avoidable work. Prediction: Task 1 recomputes per signature; Task 2 baseline counts 4/18/66 across two files, then 2 after reuse. Falsifier: different counts or unequal full-data dumps. Alternative: parser/fixture/canonicalization problems; separator: old counts and full dumps must pass before the new-count assertion fails. Status: pending watched RED.
- Setup probe: expected clean requested branch at a1aa755b, existing local main, unchanged relevant Go source since 8b2744b3. Observed exactly that; cache versions live-rebound to CPG 117/nav sidecar 65. No unrelated initial changes.
- Baseline suites: prediction 0 failed, falsifier any behavioral failure; environment refusal recorded separately. Default observed 5,351 passed, 0 failed, 1 ignored across 29 targets including doc-tests. MCP baseline in flight; source unchanged.

## Receipt ledger

Exact commands, expectations, unset golden blessing and timing are in per-run JSON beside the full logs; `.review/go-type-extract/receipts.json` aggregates complete log contents and SHA-256 values. Totals include doc-tests and exclude filtered tests. Every named full-suite target completed in each passing run.

| Stage / log | Expected | Observed | Comparison |
|---|---|---|---|
| base-default | 0 failed | 5351 passed, 0 failed, 1 ignored; 29 targets | MATCH |
| base-mcp (unrestricted) | 0 failed | 2023 passed, 26 failed; stopped at lib | EXCLUDED host socket facilities, pre-change control |
| base-mcp-runnable | all runnable targets, 0 failed | 5868 passed, 0 failed, 1 ignored, 32 excluded; 31 targets | MATCH within explicit exclusion |
| t1-setup-invalid | absent APIs | missing test modules E0583 | INADMISSIBLE setup probe, repaired |
| t1-red | absent mode/constructor compile error | E0433 ImportScan + E0599 constructor | MATCH infrastructure RED |
| t1-wire | exact wiring confirmation | exact planned message | MATCH |
| t1-fmt | formatted source | no diagnostics; reviewed diff | MATCH |
| t1-parity | 7 passed, 0 failed | 7 passed, 0 failed | MATCH |
| t1-default | baseline +7; 0 failed | 5358 passed, 0 failed, 1 ignored; 29 targets | MATCH |
| t1-mcp-runnable | runnable baseline +7; 0 failed | 5875 passed, 0 failed, 1 ignored, 32 excluded; 31 targets | MATCH within explicit exclusion |

## Output and ordering audit

Pending Tasks 1–3 diff and full-suite inspection. Plain and import-aware constructors must stay distinct. Serialized data/meaning and all downstream consumers must stay unchanged.

## Exclusions / remaining work

Task 4, real-corpus measurements, paired instrumented binaries, authoritative full mutation gate before merge, independent reviews, merge and push are controller-owned. No Linux claim. Tier-A and scoped mutation gates pending environment admission.

- Ruling: continue without nice after the first priority refusal — base-default.log reports `nice: setpriority: Operation not permitted`; user explicitly authorizes CARGO_BUILD_JOBS=4 fallback — cost if wrong: checks run at normal host priority; no claim of reduced priority.

## MCP baseline environment exclusion

The unrestricted baseline feature command stopped at the library: **2,023 passed, 26 failed**, no ignored. 24 failures report `Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" }` from socket binding; two child-daemon checks report `gave up after 30s waiting for: the daemon process to listen`. These predate all source edits. Alternative (regression in this patch) is excluded because the patch does not yet exist; the two child timeouts alone do not prove bind denial, but source shows the child binds the same restricted socket class.

- Ruling: carry the unrestricted MCP gate open and run the largest runnable feature suite with exact socket-dependent exclusions and --no-fail-fast — same-environment baseline establishes host socket-bind refusal; do not retry a blocked command class — cost if wrong: 32 socket-dependent tests need controller execution; no full-MCP GREEN claim. Six integration cases are source-identified before retry, rather than rediscovering the same bind denial. Two daemon_process integration tests that do not bind remain enabled.

Exact excluded test names (also `.review/go-type-extract/mcp-exclusions.json`):

```text
mcp::daemon::files::tests::a_daemon_that_finds_the_key_taken_touches_no_file
mcp::daemon::files::tests::a_replaced_runtime_directory_is_noticed_and_left_alone
mcp::daemon::files::tests::a_replaced_socket_alone_is_noticed_and_left_alone
mcp::daemon::files::tests::binding_replaces_a_stale_socket_left_by_a_dead_daemon
mcp::daemon::files::tests::the_socket_is_removed_only_if_it_is_still_the_one_this_daemon_bound
mcp::daemon::server::drain_tests::a_connection_queued_while_the_daemon_retires_is_answered_retiring
mcp::daemon::server::drain_tests::a_daemon_nobody_reaches_retires_after_the_ttl
mcp::daemon::server::drain_tests::a_replaced_root_drains_the_daemon_at_the_next_check
mcp::daemon::server::drain_tests::a_peer_of_another_user_queued_while_the_daemon_retires_is_not_answered
mcp::daemon::server::drain_tests::a_replaced_root_is_noticed_at_admission
mcp::daemon::server::drain_tests::a_stop_request_drains_the_daemon_with_a_client_attached
mcp::daemon::server::drain_tests::a_running_build_keeps_an_idle_daemon_up_until_it_ends
mcp::daemon::server::drain_tests::an_idle_daemon_retires_after_the_ttl_and_leaves_the_key_free
mcp::daemon::server::drain_tests::replaced_files_drain_the_daemon_and_the_new_directory_is_left_alone
mcp::daemon::server::tests::a_hello_completed_after_its_deadline_is_not_admitted
mcp::daemon::server::tests::a_connection_from_another_user_is_dropped_before_any_reply
mcp::daemon::server::tests::a_hello_for_the_served_identity_is_answered_ready_and_mcp_follows
mcp::daemon::server::tests::a_line_whose_read_ends_at_the_deadline_is_not_answered
mcp::daemon::server::tests::a_peer_of_another_user_is_not_answered_busy_either
mcp::daemon::server::tests::another_identity_or_protocol_is_answered_mismatch_and_not_admitted
mcp::daemon::server::tests::a_refused_preamble_line_is_closed_without_a_reply
mcp::daemon::server::tests::connections_over_the_bound_are_answered_busy_and_stalled_ones_time_out
mcp::daemon::server::tests::one_turn_accepts_a_bounded_number_of_connections_then_services_deadlines
mcp::daemon::server::tests::the_handshake_deadline_bounds_the_whole_line
mcp::daemon::connection::panic_tests::a_panic_in_dispatch_ends_the_daemon_process_and_every_attached_client_sees_its_connection_end
mcp::daemon::connection::panic_tests::the_daemon_process_ends_when_its_standard_input_is_closed
daemon_process_test::a_daemon_process_serves_a_shim_and_retires_after_its_idle_ttl
daemon_process_test::a_daemon_replaces_a_stale_socket_left_by_a_dead_one
daemon_process_test::a_daemon_that_finds_a_live_owner_exits_silently_and_touches_nothing
daemon_process_test::daemons_started_together_leave_exactly_one_serving
daemon_process_test::a_termination_signal_drains_a_daemon_with_a_shim_attached
daemon_process_test::a_warm_daemon_builds_before_any_tool_call
```

- Ruling: discard t1-setup-invalid as evidence and rerun the planned RED after creating the missing test directory/files — the setup helper failed with FileNotFoundError and rustc E0583 named missing modules, rather than the intended absent APIs — cost if wrong: an invalid infrastructure observation could be mistaken for the specified RED. The setup artifact was repaired in place; no implementation change/restart. Subsequent dependent shell steps use set -e.

Baseline runnable MCP observed **5,868 passed, 0 failed, 1 ignored, 32 excluded**, 31 completed targets including doc-tests; unblessed standalone transcript golden test passed.

## Task 1 observed RED/GREEN

`t1-red.log`: intended infrastructure RED (E0599 missing `from_parsed_files_with_import_scan`, E0433 undeclared `ImportScan`), no missing-module diagnostics. This is not behavioral evidence. After the exact plan wire.py printed `wired per-file signature imports; extraction writes and iteration order untouched`, formatting succeeded and `t1-parity.log` recorded **7 passed, 0 failed**. Both modes still use `Cow::Owned(self.scan())`. Intermediate warnings for unused cache/mode/measured are expected until Task 2 activates them. The reviewed go.rs diff contains only private mode/context forwarding, canon_sig input reuse plumbing, child modules and removal of the redundant old wrapper. No data writer/order/profile/alias resolver changes.

The one ignored test in each full-suite run is pre-existing `resolution_test::slice_elem_variant_reserved`: `SliceElem is reserved (spec §5/§10); classifier returns None until a future slice`. All selected provider tests actually executed; the zero-test prism-mcp unit binary does not stand in for the feature suite.

Task 1 release build succeeded (`t1-release.log`, optimized, 1m 00s), immediately before the matrix command. Matrix did not start: `uv` exited 2:

```text
error: Failed to initialize cache at `/Users/wesleyjinks/.cache/uv`
  Caused by: failed to open file `/Users/wesleyjinks/.cache/uv/sdists-v9/.git`: Operation not permitted (os error 1)
```

- Ruling: exclude Tier-A matrix (T1/T2) and quick (T3) after the first uv sandbox refusal — user authorizes exact-error exclusion/continuation and steering forbids repeated probes of a denied command class — cost if wrong: accuracy regressions/flip candidates remain unmeasured until controller runs the gates; no Tier-A GREEN claim. Release builds still run at each prescribed stage.

Task 1 expectations: release MATCH; matrix EXCLUDED, not a regression verdict. Task 1 commit is authorized despite explicitly open host gates under the run override.

## Task commit environment exclusion

Task 1 staging was attempted with exactly the plan's six explicit paths. It was refused before any commit command could run:

```text
fatal: Unable to create '/private/tmp/claude-501/-Users-wesleyjinks-code-slicing/fd583811-fdc5-4426-9c4a-55872d2ecc1c/scratchpad/impl-go-clone/.git/index.lock': Operation not permitted
```

- Ruling: preserve separate Task 1/2/3 patches, message files and complete snapshots instead of retrying .git writes — the filesystem sandbox refuses index.lock even though the user authorized task commits; escalation is unavailable and steering blocks repeated denied-class attempts — cost if wrong: no task commit SHAs exist in this clone; controller must apply/commit the three preserved stages with the required messages/trailers. No permission was re-requested and no alternate Git-directory workaround is attempted.

Task 1 commit SHA: **NOT CREATED (sandbox refusal at git add)**. HEAD remains a1aa755b. Tasks 2–3 will continue on this same artifact with explicit per-task snapshots, never a restart.

## Task 2 watched behavioral RED

`t2-red.log` matched the plan: **1 passed, 1 failed**. The zero-work edge case passed. At the first repeated-signature fixture (`count=1`), the full Debug equality and old count assertion had passed before `new_count` failed with **left 4, right 2**. This discriminates repeated production scanning from parser/fixture/canonicalization alternatives. No compiler/environment failure is used as behavioral evidence. Re-read get, then installed only the exact planned OnceCell match body.

## Task 2 GREEN and targeted mutations

`t2-green.log`: **9 passed, 0 failed**, matching Expected. The assertions cover old scan counts 4/18/66 against exactly 2 for each two-file fixture in both import-path modes, full-data equality before counts, zero signature scans for empty/non-Go/no-signature inputs, and fresh data at a reused path. Counts exclude unchanged alias-resolver setup scans.

One run per targeted mutant, compiled successfully and executed all nine tests; all named behavioral assertions failed as required (not merely nonzero exit status):

| Mutant / log | Expected required failure | Observed | Verdict |
|---|---|---|---|
| M1 / t2-M1 | repeated signature new-count 4 vs 2 | 8 passed, 1 failed; exact assertion left 4, right 2 | KILLED, MATCH |
| M2 / t2-M2 | zero-signature count 1 vs 0 | 7 passed, 2 failed; zero-work left 1/right 0; old-count also 6 vs 4 because EverySignature eagerly scans too | KILLED, MATCH |
| M3 / t2-M3 | import-aware full-data / distinct constructors | 7 passed, 2 failed; named constructor case full dump loses `(~example/p::T)()` to `(T)()`, satisfaction I loses T, sat_keys empty; count also 0 vs 2 from bypassed counter | KILLED, MATCH |
| M4 / t2-M4 | last-map/first-method ordering | 7 passed, 2 failed; named order test actual a/t.go vs expected z/t.go; Inner winner also p/a.go vs q/a.go | KILLED, MATCH |

Both owned files were byte-for-byte restored and hash-checked between each mutant and after M4. Final source SHA-256: go.rs `add83580c39a03dcc606a43509b28fea214aad8a73dac286840707ca311405e5`; signature_inputs.rs `def60cfe51e5dfcce6bacb1f8e25df13a7dc6d5ff2e571429164279051c151a2`. The final get body is exactly the planned OnceCell match. No mutant is retained. Restoration rerun passed 9/9; final full-suite and release receipts are below.

## Transient storage failure during final feature suite

The first final runnable MCP run completed every target but recorded **5,870 passed, 7 failed, 1 ignored, 32 excluded**. Four Lua failures explicitly report `Os { code: 28, kind: StorageFull, message: "No space left on device" }`; three JavaScript failures report `git log failed (is this a git repository?): fatal: not a git repository (or any of the parent directories): .git`. The JavaScript failures are not yet attributed to storage: failed fixture Git initialization and other Git setup failures could produce the same output. No source fix is authorized or made for these unrelated test targets.

A shell here-document independently failed with `zsh:1: can't create temp file for here document: no space left on device`. Diagnostic prediction: persistent volume exhaustion would show near-zero available space; alternative temp-only/quota issue would differ across paths. Actual subsequent `df -h . /private/tmp` showed the same Data volume with **1.8 GiB available**, so persistent exhaustion at probe time was falsified; transient pressure remains an observation, not a reconstructed constant environment. This lane's target is 9.6 GiB, local evidence 3.6 MiB, archives under 1 MiB each. Nothing is deleted.

- Ruling: preserve the failed full-feature receipt, run same-host pre-change controls for the two affected language targets, restore exact candidate bytes, then allow one full-feature retry — completed --no-fail-fast output enumerated all seven failures and free space returned; source alternatives require a control before attribution — cost if wrong: transient host state may make original failures unreproducible; retain uncertainty and stop/report if the one retry fails, without normalizing tests.

Complete failed population:

```text
algo_test::test_resonance_slice_javascript
algo_test::test_phantom_slice_javascript
algo_test::test_threed_slice_javascript
algo_expand_test::test_delta_slice_lua
algo_expand_test::test_phantom_slice_lua
algo_expand_test::test_resonance_slice_lua
algo_expand_test::test_threed_slice_lua
```

Control prediction: both base and restored candidate should pass the 70 JavaScript + 25 Lua tests while space is available if this was transient fixture/storage failure. Falsifier: a repeatable candidate-only failure with passing base at comparable space. Base control green alone would not diagnose the original JavaScript failures. Full retry cap: one; no silent extension.

Pre-change affected-target control (`disk-base-control.log`) passed **95/95** (JavaScript 70, Lua 25), 0 failed, 0 ignored on this host. Free space before control was 1.8 GiB, after control 1.3 GiB. The constant-full-volume hypothesis was already falsified; this control alone does not establish the cause of the earlier Git setup errors. The candidate was restored byte-for-byte to its saved final hash. Run the same 95-test candidate control next, then repeat both complete default and runnable-MCP suites after this temporary source swap so the final receipts follow the final restoration. This is still the declared single full-feature retry; no additional retry is authorized silently.

Candidate affected-target control (`disk-candidate-control.log`) also passed **95/95**, 0 failed, 0 ignored; base and candidate both pass after the transient pressure. The original JavaScript failures remain unclassified rather than attributed to the optimization.

- Ruling: reclaim only this lane's generated target/debug/incremental directory between completed cargo runs, before the single MCP retry — startup inventory showed no target directory, the cache is ordinary/non-symlink and measured 4.6 GiB while free space fell to 1.2 GiB; sequential run.py completion guarantees no lane cargo is using it during removal — cost if wrong: incremental rebuild CPU/time is lost; source, binaries, test artifacts, evidence, other targets and other workspaces are preserved. Actual pre/post free space is captured in incremental-reclaim.log. No global cleanup.

## Task 2 final verification / stage boundary

| Log | Expected | Observed | Comparison |
|---|---|---|---|
| t2-restored | 9 passed after exact restore | 9 passed, 0 failed | MATCH |
| t2-fmt | formatting succeeds, new files under 600 lines | no diagnostics; 58/105/99/48 lines (helper/equivalence/import/cost) | MATCH |
| t2-default | baseline +9, 0 failed | 5360 passed, 0 failed, 1 ignored | MATCH |
| t2-mcp-runnable first attempt | runnable baseline +9, 0 failed | 5870 passed, 7 failed, 1 ignored, 32 excluded | FAILED receipt retained; transient storage and fixture failures above |
| disk-base-control | affected targets pass with free space | 95 passed, 0 failed | MATCH; does not reconstruct earlier transient host state |
| disk-candidate-control | same 95 tests pass | 95 passed, 0 failed | MATCH; original JS errors not reproduced or conclusively attributed |
| t2-default-final | complete default after final restoration | 5360 passed, 0 failed, 1 ignored; all 29 targets | MATCH |
| t2-retry-mcp-runnable | one full feature retry, exact 32 socket exclusions | 5877 passed, 0 failed, 1 ignored, 32 excluded; all 31 targets | MATCH within explicit exclusion |
| t2-release | optimized rebuild | Finished release optimized in 44.85s | MATCH |
| T2 matrix | no regressions/flip candidates | not run after known uv sandbox refusal | EXCLUDED, open controller gate |

Golden blessing was unset for every feature run, including both controls and the retry. No golden fixture changed. Expected cache reclamation was more free space; at the scheduled boundary the cache had grown from 4.6 to 4.7 GiB and free space rose from 172 MiB to 4.0 GiB after deletion. The dynamic shared-volume readings do not establish an exact unique-byte recovery amount; only the removal and increased headroom are measured.

Task 2 source remains exactly the planned helper body plus two cost tests. Task 2 commit SHA: **NOT CREATED (same denied .git-write class; not retried)**. Separate `task2.patch`, message with both required trailers, and full stage snapshot preserve the task boundary. No source/test change follows these final full suites.
