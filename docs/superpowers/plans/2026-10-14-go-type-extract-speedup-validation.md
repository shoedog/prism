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
