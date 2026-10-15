# DFG compaction execution ledger — Task 1 / PR A

Plan: `docs/superpowers/plans/2026-10-14-dfg-compaction.md`.
Parent: `cc2e612c04d4ab84a7b6843b8c3c16e1ad2ffc5e`; branch `perf/dfg-oracle`; initially clean.
Scope: Task 1 only. Controller owns commits and review (two-round cap).

## Rulings

- Ruling: Rebind source to execution parent cc2e612c — main moved from plan source 8b2744b3; retain both hash receipts and compare frozen bodies to the execution parent — cost if wrong: an upstream behavior change could be attributed to extraction.
- Ruling: Leave Git index and commits untouched; generate a full patch without git add -N — owner forbids implementation Git writes and add -N writes that protected index; combine tracked git diff with no-index additions — cost if wrong: controller must verify patch applies to the exact parent.
- Ruling: Controller owns socket exclusions/full host suites, Tier-A, mutation gate and executable replay when unavailable — explicit dispatch limits supersede the generic gate — cost if wrong: PR remains unready for merge until those receipts exist.
- Ruling: Continue with CARGO_BUILD_JOBS=4 if nice is refused — explicit owner override — cost if wrong: shared-host load may be higher.
- Ruling: Use this tracked validation ledger and preserve scratch evidence at handoff — no commit authority; skill completion/deletion scripts assume commits — cost if wrong: controller must snapshot evidence before cleanup.

## Preflight interfaces

| Producer / consumer | Result |
|---|---|
| A producer / B and E build | Single shared FileRefs producer; preserve ordered collection; E later adapts within Rayon map. |
| A oracle / C–F tests | Independent aggregation, queries, updates; share DTO/lattice/producer only. |
| A payload control / B–D gates | Retain parent bytes; never recapture candidate as baseline. |
| A assembly seam / E parity | Support module calls unchanged assembly; only same-line visibility edit in oversized build.rs. |

## Command and hypothesis log

- Initial inventory: clean expected and observed. HEAD/branch match dispatch. `df -h .`: 185 GiB available, above 20 GiB floor.
- Process inventory via ps: sandbox refused `operation not permitted`; inadmissible as a process census. One cargo process at a time within this task; controller dispatch supplies the lane build slot.
- Memory quick pass found older DFG review history, no Task 1 execution state; no historical implementation reused.
- Plan has no separate reachable normative spec; owner request and plan govern; rulings are provisional where neither resolves a detail.
- nice probe: refused `setpriority: Operation not permitted`; use owner-authorized env CARGO_BUILD_JOBS=4 henceforth. Parent source: 1,576 lines, SHA256 d68e5baec10826c393281909d2c5393c4cdb70497495ab1e310cf7041a6739ac (plan expectation 1,519 belongs to old base).
- Parent archive probe: Python 3.9 rejects extractall(filter=...); subsequent manifest error is inadmissible. Repaired by extracting this locally generated trusted git archive without the unsupported keyword. No source changed in candidate.
- Parent capture prediction: adapter compiles and emits one payload per source. Falsifier: Rust errors or no payloads; alternative: harness schema error rather than production defect. Observed inadmissible tuple-shape errors: normal common generators return triples, security generators pairs. Separate loops; normalize only test driver interfaces. This is no observation about DFG behavior.
- Ruling: Normalize generator tuples in drivers — plan pseudocode mixes pair/triple signatures; production generators remain unchanged — cost if wrong: omitted sources/diffs could weaken output cases; explicit named cases and census protect population.
- Parent stack probe: capture test overflowed its default test-thread stack before first payload. Hypothesis: large debug stack frames (generator array / CPG build), alternative: unbounded recursion on a fixture. Prediction with RUST_MIN_STACK=16777216: completed named captures supports stack-budget cause; repeated overflow at one source falsifies and identifies recursion candidate. Add per-case breadcrumbs before rerunning; no production change.
- Ruling (SUPERSEDED by Fix 1 for current oracle tests; retained as historical evidence): Increase test-thread stack to 16 MiB for parent/candidate corpus and suites if required — baseline debug capture overflowed default stack; no production stack setting changes — cost if wrong: hides a real unbounded-recursion defect; retain case breadcrumbs and source control.
- Parent control GREEN: 131 SerializedCpg payloads captured before extraction (109 fixtures + whole tree + 17 common generators + Lua/Rust + two helper cases); parent test 1 passed, 0 failed, 32.51s. Raw bytes/manifests/adapter hash retained in .review/dfg-A. 16 MiB stack completed all cases, separating finite stack budget from the proposed unbounded-recursion alternative. Correction: one bash payload existed before the earlier overflow, not zero; earlier wording was provisional.
- RED prediction: umbrella cannot import missing oracle/support APIs; unrelated generator-schema errors would be inadmissible scaffold evidence.
- Scaffold RED observed exactly: E0432/E0433 missing data_flow::test_support and cpg::dfg_compaction_support (4 errors), no tuple/schema errors; .review/dfg-compaction/logs/t1-red.log. Compile RED is scaffold evidence only.
- Tests authored before extraction name breaks: byte-span erasure, reordered hops, lost confidence/category/unlabeled state; producer/aggregation drift; changed query traversal; clone/subset/remove/merge drift. Planned deliberately wrong byte mutant supplies behavioral RED.
- Extraction compile probe predicted successful wiring; falsifier was compiler errors. Observed inadmissible wiring errors: missing AccessPath/FlowDoubt/ExactFlowEdge imports and sibling-private oracle insert_label. Alternative E0282 source type defect is explained by missing AccessPath; repair imports/visibility without method-body edits, then rerun.
- Parent release GREEN: built offline in 61s before production extraction; retained binary and SHA256 at .review/dfg-A/parent-bin. Production extraction is now present; cache versions unchanged.
- Ruling: Separate the 19 successful-dispatch generator cases from the runnable storage/query cases and require DFG_ORACLE_ALLOW_GIT=1 — every success-matrix row requires a committed current-date fixture, while owner forbids attempts at Git commits here; controller runs these named host cases — cost if wrong: output/history coverage is pending until the host run, not a local green claim.
- Existing positive/negative source selectors were inventoried before output implementation in .review/dfg-A/algorithm-selectors.txt; exact plan anchors include paper_test::{test_left_flow_python,test_full_flow_python}, misc_lang_test::{test_chop_python_verifies_path_lines,test_conditioned_slice_prunes_unreachable_python}, misc_test::{test_delta_slice_python,test_delta_slice_go}, provenance_test::{test_provenance_python_request_form_origin,test_provenance_negative_transform_not_form}. Full suites retain these assertions; new paired output matrix is separate from expected errors.
- Initial corpus GREEN: 131 passed, 0 failed, 0 ignored; 167.82s sequential. Fixture census exactly matches inherited 109 files / 9 languages, all 12 languages covered with generators. Largest fixture 58.599s and largest generator 53.390s are large_function.c; corpus cases are not mutgate selectors. Default-feature registered lib selectors remain small, timed separately.
- Ruling: Keep exhaustive large-function observations as named umbrella cases, not mutation selectors — measured 53–59s exceeds the <30s target though under 120s; no coverage reduction — cost if wrong: umbrella duration may grow; split probes before registering any large selector.
- Behavioral RED prediction: increment only candidate edge.from.start_byte (legacy Eq unchanged); exact storage comparison must fail. Falsifier: the named test passes. Alternative: compile/wiring failure is inadmissible; require ordered-storage assertion output.
- Byte mutant RED observed: compiled successfully; exact ordered-storage assertion failed, 0 passed / 1 failed in 0.19s (t1-span-red.log). Legacy Eq still treats the changed endpoint as equal; restored only the mutant line before GREEN.
- Fixture census probe initially omitted module extensions in its generated registration, while runtime walker uses Language::from_path; corrected registry includes mjs/cjs/jsx before first runtime corpus. Actual census is unchanged 109; the initial 99 was an inadmissible incomplete inventory, not source drift.
- Source-control probe prediction: moved bodies equal after indentation removal; compiler formatting may reflow lines. Current comparison failed on producer before publishing a verdict. Alternative: actual extraction change vs comparator whitespace/comment handling. Inspect first divergent tokens/literals before ruling or changing code; failed checker is inadmissible until its own mechanism is validated.
- Source-control discriminator: rustfmt removed an optional trailing comma when unwrapping a call, rather than changing producer logic. Preserve the original producer body under a method-local rustfmt::skip. Re-run control now passes all 41 method/field blocks after stripping indentation only; DTOs, aggregation, label insertion, seven queries and updates match parent. Full receipt: .review/dfg-A/extraction-control.{log,json}.
- Span GREEN: six default-feature lib tests passed, 0 failed, 0.25s; includes projection edge cases, query domain, updates and Delta positive/identical control.
- Registered-mutant predictions: retaining forward seed must fail the independent query oracle; deleting candidate Delta edges must lose return-line 4. Passing selector falsifies coverage; build errors/zero selections are inadmissible. Each mutant is restored in finally before the next run.
- Forward-seed mutant RED: exact forward observation failed (0 passed / 1 failed, 0.20s). Delta mutant's first form was inadmissible E0282: unconstrained empty BTreeSet removed type evidence. Replace only the mutant with a typed iterator filter(false); production source restored. Same behavioral prediction remains, no coverage claim from compiler refusal.
- Ruling: Keep shared producer file above the 600 aim and 620 forecast — preserving the complete cohesive baseline closure and indentation-only byte proof measures 657.5 weighted lines; below 1,000/2,000 and splitting its execution body would enlarge semantic extraction — cost if wrong: later producer work may need a separately reviewed responsibility split.
- Ruling: Use cargo --offline throughout — no installs/network authorized; all dependencies are already cached — cost if wrong: missing cached dependencies would block the gate rather than fetch silently.
- Default full suite GREEN: 5421 passed, 0 failed, 1 ignored across 29 target/doc-test summaries; no exclusions. Log t1-default.log. TempDir Git fixture tests pass in this environment; implementation-repository index remains untouched.
- Historical plan source command also succeeds: 1,519 lines from 8b2744b3 saved separately as .review/dfg-A/planning-source.rs; execution oracle remains bound to cc2e612c and its saved source.
- Ruling: Run disposable fixture commits required by the plan; retain the ban on implementation-repository index/commits — the full same-environment suite demonstrated writable TempDir fixtures using common::create_temp_git_repo, so the earlier 19-case controller-only routing was overbroad and is superseded; no protected Git writes are attempted — cost if wrong: disposable test history is created in authorized temporary roots; no implementation commit or publication occurs.
- Ruling: Serialize cargo fmt/check with test/build commands as well — an inadvertent fmt --check overlapped the MCP test process; it only reported formatting differences, but violated the one-cargo rule; rerun formatting after the suite ends — cost if wrong: shared-host process overlap; no further concurrent cargo invocation is allowed.
- Review dispatch: one fresh read-only final review via executing-plans skill; no writes/tests by reviewer, one targeted root fix pass, no re-review. Controller PR review cap remains two rounds. Source production bodies are frozen while review runs.
- MCP exclusions: 34 exact names/source reasons in .review/dfg-A/socket-exclusions.{txt,json}; complete command in .review/dfg-A/mcp-command.txt. Unnamed UnixStream::pair tests remain included. Default's sole pre-existing ignored case is resolution_test::slice_elem_variant_reserved (reserved classifier variant).
- MCP largest runnable suite GREEN: 5937 passed, 0 failed, 1 ignored, 34 filtered across 31 target/doc-test summaries. Exactly 34 named socket tests filtered; log t1-mcp.log.
- Fresh review: no WRONG production finding. Two material SMELL assertion gaps accepted for the single targeted fix pass: old refresh path shares production updates; missing paired direct-reader negatives and named Chop/Conditioned controls. Both are test-oracle gaps, not demonstrated incorrect current production behavior. No second review dispatched.
- Review dispositions: compact identity/codec/reclamation/peak and adversarial Task 4 synthetic populations remain in their named later tasks; no compact implementation belongs in Task 1. Unavailable host gates stay pending. Producer-size aim exception remains as ruled.
- Ruling: Split dedicated paired assertions into tests/dfg_compaction/controls.rs within the same umbrella — direct-reader negatives plus Chop/Conditioned controls are a real test responsibility and keeping them separate preserves the new-file size aim — cost if wrong: one additional module to retire in F.
- Review fix RED prediction: feature umbrella cannot resolve the newly requested frozen-update assembly seam; unrelated compile errors are inadmissible. After seam exists, a production-only removal mutant must fail refresh payload equality; missing-API RED alone does not establish update independence.
- Review scaffold RED observed exactly: missing legacy_refreshed_dfg API, one compile error. Added feature/test-only conversion plus frozen remove/merge before current assembly. Direct-reader paired controls pass; new Chop expectation fails on assumed intermediate assignment.
- Chop diagnostic hypothesis: CPG chop follows concrete DataFlow edges without assignment-hop synthesis, so source 2 / sink 4 returns endpoints {2,4}; alternative: producer extraction removed the intermediate edge. Same-environment untouched-parent control predicts {2,4}, direct-edge source 2 / sink 3 predicts {2,3}, cross-function sink 7 predicts empty. Any difference in parent output falsifies this explanation; do not attribute a regression or alter expected assertions before this control.
- Chop parent control observed all three predicted literal sets, 1 passed / 0 failed. This failure is a new-test expectation defect, not a regression: the exact same pre-change environment emits {2,4}, not an intermediate assignment. Use the real direct-edge source 2 / sink 3 positive (literal {2,3}) and disconnected CFG sink 7 negative (empty); preserve the plan's original top-level source/endpoints separately.
- Ruling: Use a concrete direct-edge positive for the added Chop assertion — same-environment parent proves CPG chopping does not synthesize the assumed assignment hop; literal {2,3} and cross-function empty check real contract without changing production — cost if wrong: this bounded positive does not cover arbitrary multi-assignment paths, whose current behavior is intentionally preserved.
- Ruling: Run the feature umbrella with --test-threads=1 — honors sequential fixture/generator execution and avoids concurrent dual graphs — cost if wrong: oracle wall time is longer, with no reduced case population.
- Independent refresh behavioral RED prediction: a production-only remove_files no-op leaves duplicate current edges after re-add while frozen removal/re-add stays unchanged; the new CPG payload equality must fail specifically at refresh-readd. A cold/hit/parse failure is inadmissible for this claim; a pass falsifies independence coverage.
- Review controls GREEN: 3 paired tests passed, 0 failed, 0.34s, including original LeftFlow return assertion, disconnected/empty DFG, direct Chop/disconnected CFG, Conditioned opposite branches, Delta and Provenance in cold/hit/refresh-readd modes.
- Refresh independence mutant RED: production remove_files no-op failed exact CPG payload equality at refresh-readd (0 passed / 1 failed, 0.27s); restored original body. This demonstrates the corrected output comparison no longer shares updates on both sides.
- Delta typed-filter mutant RED: candidate-edge contribution removed with type inference preserved; named default lib selector fails on missing return line 4. Restored source; final GREEN/full suites follow.
- Final feature umbrella GREEN: 154 passed, 0 failed/ignored/filtered, 366.23s sequential. Exactly 1,710 successful paired dispatch rows = 19 cases × 3 modes × all 30 algorithms, with zero dispatch errors; 3 additional paired controls per case/mode (6 backend dispatches) for return-flow/identical-source cases are checked outside that success count. Expected-error cases are separate. Dispatch receipt: .review/dfg-A/dispatch-success-ledger.json; raw t1-oracle-final.log retains parameters and case names.
- The expanded source control additionally checks the shared producer's insert_label copy: 42 blocks match after indentation removal only. No runtime compact engine, cache-version change or Task 2 work exists.
- Ruling: Retain the completed bounded Task 1 artifact despite the guessed 800 semantic-line review target — substantial new test/support coverage plus mechanically frozen/moved code is required by the plan; actual weighted per-file surface is recorded and one fresh review converged through two closed assertion repairs — cost if wrong: controller may need a named oracle/corpus review split before PR dispatch; no silent cap extension or restart is authorized.
- Ruling: Allow the test-only CPG support module to exceed its 60-line forecast — the independent refresh repair needs an explicit completed-state adapter; actual size is recorded, well below the 600 aim — cost if wrong: Task E must update this test-only legacy conversion when storage changes.
- Ruling: Leave compact identity/rank, representative, codec, reclamation and cold-peak work in Tasks 3–7 — no compact engine is authorized in PR A — cost if wrong: those guarantees remain unproved until their own gates.
- Ruling: Leave the adversarial confidence/foreign-key synthetic update population in Task 4 — PR A freezes the source contract and exercises real-corpus/basic updates; the named synthetic distinctions belong to the compact update task — cost if wrong: this PR alone does not establish those future compact-update cases.
- Execution-version rebind: current and parent cpg_cache::CACHE_VERSION are both 118 (plan inspected source said 117); Task 1 did not edit cpg_cache.rs. The future activation task must choose a then-free current-main-plus-one version; 118 is already occupied.
- Ruling: Retain parent cache v118 unchanged in PR A — execution main advanced beyond the plan's v117 source; this PR changes no serialized layout — cost if wrong: future activation could reuse an occupied version unless its own rebind is performed.
- Final GATE after the review fixes: fmt --check clean; default 5,421 passed / 0 failed / 1 pre-existing ignored across 29 targets/doc tests (197.067s); MCP 5,937 passed / 0 failed / 1 pre-existing ignored / 34 exact socket exclusions across 31 targets/doc tests (203.141s). No out-of-scope failing test or rebaseline. Complete commands/results: .review/dfg-A/final-gate-results.json.
- Fresh release GREEN: 45.18s; retained candidate CLI at .review/dfg-A/candidate-bin/prism plus SHA256, alongside parent release CLI and SHA256. Both release commands omit dfg-oracle.
- Tier-A matrix attempted immediately after that release, with uv offline mode: exit 2, `Failed to initialize cache at /Users/wesleyjinks/.cache/uv`; cause `failed to open file /Users/wesleyjinks/.cache/uv/sdists-v9/.git: Operation not permitted (os error 1)`. Inadmissible as accuracy evidence. Quick not retried after the command-class refusal. Controller runs both.
- Ruling: Attempt Tier-A with uv --offline and do not repeat quick after the cache refusal — no network authorized and one sandbox denial blocks the command class for this session — cost if wrong: Tier-A remains unverified, never green or merge-ready from this run.
- The planning-to-execution rebind is producer-only: 66 insertions/9 deletions in data_flow.rs, no confidence-lattice diff; all 17 frozen method/DTO/field/aggregation blocks are indentation-identical across 8b2744b3→cc2e612c. Main's v118 adds member-only parameter Def/projection facts in the shared producer; PR A freezes the unchanged legacy contract against that current producer. Receipts: execution-rebind-diff.log and rebind-contract.json.
- Final parent/candidate byte pin GREEN using the same read-only capture adapter: each independently reproduced all 131 original pre-A payloads byte for byte; original pin SHA256 manifest unchanged. Parent control 1 passed / 0 failed (33.25s, one unrelated diagnostic filtered); candidate control 1 passed / 0 failed (34.40s). Capture adapter source files identical. Logs: t1-{parent,candidate}-payload-final.log; summary: .review/dfg-A/payload-controls-final.log; test binaries retained beside release binaries.
- Six default-feature named lib selectors each ran exactly one test and passed; command wall seconds: observation spans 5.614 (includes rebuild), call/category 0.075, storage spans 0.070, Delta 0.269, query domain 0.265, updates 0.314. Registered selector runtime remains far below 120s. Corpus timing receipts include 130 named checks; largest fixture 58.667s, largest generator 55.655s; total final oracle 366.23s. Raw per-case and selector timings are retained as JSON.
- Mutation gate status: NOT RUN per controller dispatch (scoped advisory and full authoritative both pending). Two registered mutants were manually watched fail their exact default-feature selectors, then restored and watched GREEN. Existing authoritative-catalog runtime is not measured here; the controller must supply the jobs=1 scheduling forecast before PR dispatch, using the retained rebuild/selector timings. No zero-mutant or partial-gate pass is claimed.
- Compiler output retains seven warnings in untouched ordinary test files (unused variables/helpers); no unrelated cleanup or cargo fix performed. Tests have zero failures. The sole declared ignored case remains resolution_test::slice_elem_variant_reserved.

## Final RED / GREEN ledger

| Check | Watched RED | Restored / completed GREEN |
|---|---|---|
| Initial API scaffold | 4 missing support module/API compile errors | First corpus 131/131; final umbrella 154/154 |
| Exact byte span | Candidate-only edge.from.start_byte + 1: ordered-storage failure, 0/1 | Six lib tests passed; all final suites passed |
| Forward seed exclusion | Registered production mutant retains seed: forward bytes failure, 0/1 | Exact selector passed in 0.265s wall |
| Delta candidate edges | Typed registered mutant removes added candidate edges: return line 4 missing, 0/1 | Exact selector passed in 0.269s wall |
| Frozen refresh API scaffold | Missing legacy_refreshed_dfg compile error | Three paired controls passed |
| Independent refresh | Production-only remove_files no-op: CPG refresh-readd byte mismatch, 0/1 | Three controls, final 154 oracle cases and final suites passed |
| New Chop literal | Assumed intermediate path line absent | Same-environment parent proves old contract; direct-edge/disconnected literals pass |

Inadmissible probes (not behavioral RED evidence): Python 3.9 unsupported tar filter/ensuing missing manifest, mixed generator tuple arities, default test-stack overflow, extraction import/visibility errors, first Delta mutant's unconstrained empty-set type, initial source comparator rejected rustfmt's optional comma change. Each was corrected with its prediction/alternative/result recorded above. No failed probe was counted as coverage.

## Exact MCP exclusions

Named socket binds are unavailable. All other tests, including unnamed UnixStream::pair cases and the wrong-key daemon refusal, were included. Full names below correspond exactly to the 34 filtered tests; source reasons and runnable command are also retained in .review/dfg-A/socket-exclusions.json and mcp-command.txt.

```text
mcp::daemon::server::tests::a_hello_for_the_served_identity_is_answered_ready_and_mcp_follows
mcp::daemon::server::tests::another_identity_or_protocol_is_answered_mismatch_and_not_admitted
mcp::daemon::server::tests::a_refused_preamble_line_is_closed_without_a_reply
mcp::daemon::server::tests::a_connection_from_another_user_is_dropped_before_any_reply
mcp::daemon::server::tests::a_peer_of_another_user_is_not_answered_busy_either
mcp::daemon::server::tests::connections_over_the_bound_are_answered_busy_and_stalled_ones_time_out
mcp::daemon::server::tests::the_handshake_deadline_bounds_the_whole_line
mcp::daemon::server::tests::a_hello_completed_after_its_deadline_is_not_admitted
mcp::daemon::server::tests::one_turn_accepts_a_bounded_number_of_connections_then_services_deadlines
mcp::daemon::server::tests::a_line_whose_read_ends_at_the_deadline_is_not_answered
mcp::daemon::server::drain_tests::an_idle_daemon_retires_after_the_ttl_and_leaves_the_key_free
mcp::daemon::server::drain_tests::a_daemon_nobody_reaches_retires_after_the_ttl
mcp::daemon::server::drain_tests::a_running_build_keeps_an_idle_daemon_up_until_it_ends
mcp::daemon::server::drain_tests::a_connection_queued_while_the_daemon_retires_is_answered_retiring
mcp::daemon::server::drain_tests::a_replaced_root_drains_the_daemon_at_the_next_check
mcp::daemon::server::drain_tests::a_replaced_root_is_noticed_at_admission
mcp::daemon::server::drain_tests::replaced_files_drain_the_daemon_and_the_new_directory_is_left_alone
mcp::daemon::server::drain_tests::a_stop_request_drains_the_daemon_with_a_client_attached
mcp::daemon::server::drain_tests::a_peer_of_another_user_queued_while_the_daemon_retires_is_not_answered
mcp::daemon::files::tests::a_daemon_that_finds_the_key_taken_touches_no_file
mcp::daemon::files::tests::binding_replaces_a_stale_socket_left_by_a_dead_daemon
mcp::daemon::files::tests::the_socket_is_removed_only_if_it_is_still_the_one_this_daemon_bound
mcp::daemon::files::tests::a_replaced_runtime_directory_is_noticed_and_left_alone
mcp::daemon::files::tests::a_replaced_socket_alone_is_noticed_and_left_alone
mcp::shared::paths::tests::a_socket_can_really_be_bound_at_the_limit
mcp::daemon::connection::panic_tests::the_daemon_process_ends_when_its_standard_input_is_closed
mcp::daemon::connection::panic_tests::a_panic_in_dispatch_ends_the_daemon_process_and_every_attached_client_sees_its_connection_end
daemon_process_test::a_daemon_process_serves_a_shim_and_retires_after_its_idle_ttl
daemon_process_test::a_daemon_replaces_a_stale_socket_left_by_a_dead_one
daemon_process_test::a_daemon_that_finds_a_live_owner_exits_silently_and_touches_nothing
daemon_process_test::a_daemon_that_finds_a_live_owner_leaves_the_owner_s_log_alone
daemon_process_test::daemons_started_together_leave_exactly_one_serving
daemon_process_test::a_termination_signal_drains_a_daemon_with_a_shim_attached
daemon_process_test::a_warm_daemon_builds_before_any_tool_call
```

## Weighted sizes

Measured with the merged scripts/file_size.py; raw output, tool/source hashes and owned path manifest are retained in .review/dfg-A. The existing cpg/build.rs stays 2,202.0 (same-line visibility only). The cohesive shared producer is the sole new file above the 600 aim, covered by the ruling above; no new file exceeds 1,000 or crosses 2,000. Generated archives, patch, logs, payloads and binaries retain raw bytes and are not source-size-budget files.

| File | Weighted lines |
|---|---:|
| `Cargo.toml` | 149.0 |
| `VERIFICATION.md` | 144.0 |
| `docs/superpowers/handoffs/2026-10-14-dfg-compaction.md` | 60.0 |
| `docs/superpowers/plans/2026-10-14-dfg-compaction-validation.md` | 174.0 |
| `mutants/dfg-compaction.json` | 26.0 |
| `src/cpg.rs` | 65.5 |
| `src/cpg/build.rs` | 2202.0 |
| `src/cpg/dfg_compaction_support.rs` | 81.5 |
| `src/data_flow.rs` | 268.5 |
| `src/data_flow/build/aliases.rs` | 96.0 |
| `src/data_flow/build/exact_reads.rs` | 180.5 |
| `src/data_flow/build/file.rs` | 657.5 |
| `src/data_flow/build/mod.rs` | 130.5 |
| `src/data_flow/oracle/aggregate.rs` | 109.5 |
| `src/data_flow/oracle/mod.rs` | 41.0 |
| `src/data_flow/oracle/queries.rs` | 156.0 |
| `src/data_flow/oracle/update.rs` | 61.0 |
| `src/data_flow/test_support.rs` | 294.0 |
| `src/data_flow/tests/mod.rs` | 4.0 |
| `src/data_flow/tests/observe.rs` | 144.0 |
| `src/data_flow/tests/outputs.rs` | 45.0 |
| `src/data_flow/tests/queries.rs` | 27.0 |
| `src/data_flow/tests/updates.rs` | 38.0 |
| `src/data_flow/types.rs` | 146.5 |
| `tests/dfg_compaction/controls.rs` | 240.0 |
| `tests/dfg_compaction/corpus.rs` | 316.0 |
| `tests/dfg_compaction/main.rs` | 5.0 |
| `tests/dfg_compaction/outputs.rs` | 451.5 |
## Remaining controller work and stopping point

Task 1 implementation and local verification are finished; no Task 2 work began. This is not a merge-readiness claim. Controller must run the 34 excluded socket cases/full host suites, Tier-A matrix and quick, scoped/full authoritative mutation gates, and the exact parent-versus-candidate executable replay (all generator JSON/review/SARIF/targets plus fixture nav dfg-stats --edges). The release binaries, pre-A payloads and raw receipts are retained for those gates. No real-corpus process or performance measurement was run.

Implementation HEAD and index remain at cc2e612c; no implementation staging/commit/push was attempted. task1.patch is generated from tracked git diff plus no-index new-file diffs, with no git add -N. Controller applies/stages only the owned manifest and commits using task1-message.txt. The patch reconstruction control must match all 28 owned files (including the verification-hook report) against the exact parent; receipt retained with the patch. Snapshot custody remains OPEN until the controller copies the evidence/commits the work.

## Verification hook follow-up

- Hash audit initially confirmed all packaged source/test/helper bytes unchanged. Source inspection then found the earlier claim that the original LeftFlow formatted-return assertion had been retained was inaccurate: the scripted replacement had not inserted it. This is a test-coverage/documentation defect, not a demonstrated runtime regression. Add the literal guard now and correct the receipts; do not leave the earlier unsupported claim standing as final evidence.
- Ruling: Add root VERIFICATION.md and restore the missing original formatted-return guard — the stop hook requires a root report and source audit refuted the earlier assertion-presence claim — cost if wrong: repeat verification and repackage the 28-file artifact; no production behavior change or new golden is authorized.
- Return-guard probe prediction: corrupting only the render-source text from return to __removed__ must fail the new contains(return) assertion; a compile error/empty selection is inadmissible and a pass falsifies coverage. Restore the exact source map before GREEN and full gates.

- Hook follow-up GREEN: return rendering mutant failed contains(return), 0/1, then restored; three paired controls and all full reruns pass. Fresh counts: default 5,421/0/1; MCP 5,937/0/1 with the same 34 exclusions; oracle 154/0/0 and 1,710 successful paired rows. Exact commands and limits are now in root VERIFICATION.md. No additional runtime change or new baseline.

## Fix 1 — controller gate corrections (complete; environment-limited verification)

- Ruling: Relocate the 19 MO/PD producer mutants from src/data_flow.rs to src/data_flow/build/file.rs (17) and aliases.rs (2); adjust both anchor and replacement only by the extraction's eight-space dedent where necessary. Preserve all mutation tokens, test selectors, metadata and extra edits — cost if wrong: weakened mutation coverage; exact JSON population/token comparison and full catalog planning guard it.
- Ruling: Run every corpus and output-generator test body on prism::build_pool::install (production 256 MiB stacks), including setup, helper builders, whole-tree assembly and negative dispatch cases. Remove the environment workaround from current verification/handoff instructions — cost if wrong: hidden finite-stack defects or missed case entry; existing table fixture supplies watched default-stack RED and GREEN, and the full umbrella retains all cases.
- Ruling: Two fix/validation rounds maximum; no Git/index writes, socket binds, uv or network. Use offline Cargo and CARGO_BUILD_JOBS=4 under the owner's existing refused-nice override. Preserve Task1 artifacts, produce fix1.patch against a reconstructed task1.patch tree — cost if wrong: controller cannot apply an incremental repair; all 28 Task1 files match reconstruction before edits.
- Baseline observations: unmodified Task1/default-stack named fixture aborts with explicit stack-overflow/SIGABRT; same source/16 MiB passes 1/0. Full planner finds exactly 19 missing anchors across 425 mutants/8 lanes. Evidence: .review/dfg-A/fix1/before-results.json, anchor-audit-before.json, hypothesis-log.md.
- Relocation list with all IDs/old/new paths: .review/dfg-A/fix1/relocations.json. Anchor and mutant bodies retain identical non-whitespace tokens.
- Verification pending; this entry is not a done claim.

- Ruling: Keep the mutgate implementation and Git index unchanged despite the local untracked-file scope limitation. Run the exact native scoped command, then an evidence-only live-tree overlay that adds only the saved Task1 untracked src additions to its existing changed-line set. The overlay uses the unchanged selector/anchor/schema machinery and must select all exact 19 IDs; it does not certify a native committed-tree scoped run — cost if wrong: false coverage from empty selection; retain both populations and require the host native rerun after commit.
- Planner result: exact native scoped command exits 0 but selects 0 (untracked destinations absent from git diff); this is no coverage claim. Evidence-only scoped overlay selects exactly the 19 relocated IDs, two schema units, zero static refusals/anchor-missing. Native full catalog selects all 425, zero missing anchors. See after-scoped-plan-native.log, after-full-plan.log, scoped-live-tree-population.json, after-scoped-plan-live-tree/schemata-plan.json.
- Default-stack GREEN: the exact named table fixture now passes 1/0 with RUST_MIN_STACK explicitly removed from the subprocess environment; all test bodies keep their original assertions. Seven corpus entry forms and five output entry forms use the production pool (macro expansion covers every named case). Full suites in flight.

- Full default-stack oracle GREEN: cargo test --offline --features mcp,dfg-oracle --test dfg_compaction, RUST_MIN_STACK unset, no test-thread override: 154 passed, 0 failed/ignored/filtered, 200.87s test execution. Original test code/literal bytes recovered exactly after removing only the 12 pool wrappers and one added comment; test-body-preservation.json. Production source remains byte-identical to Task1. Default/MCP suites continue sequentially.

### Fix 1 final receipts

| Check (RUST_MIN_STACK unset) | Passed | Failed | Ignored |
|---|---:|---:|---:|
| Named stack regression | 1 | 0 | 0 |
| Full oracle umbrella | 154 | 0 | 0 |
| Default suite (29 targets/doc tests) | 5,421 | 0 | 1 |
| Runnable MCP suite (31 targets/doc tests; 34 socket exclusions) | 5,937 | 0 | 1 |

- Formatting and whitespace checks passed. Normal scoped overlay run: 19/19 admissible KILLED, advisory schema mode; full catalog planner: 425 selected, zero missing anchors. `.review/dfg-A/fix1/after-results.json` and `mutation-result.json` preserve exact commands/environment/outcomes.
- Both demonstrated WRONG gate conditions have bounded corrections: stale manifest anchors (exact 19 listed below) and default-stack fixture overflow. No inherited WRONG was downgraded; no mutant was weakened. Same-environment Task1 stack RED, larger-stack unchanged-source control, and fixed default-stack GREEN distinguish the stack-budget mechanism for this fixture. Every original assertion/literal survives the wrappers; existing corpus, helpers, cold/cache/refresh generator dispatches and negative expected-error cases all ran.
- One validation round used out of the declared two. No implementation change or retry was needed after GREEN; no unrelated failure was re-baselined.

| Mutant | Old path | New path | Dedent |
|---|---|---|---|
| MO-14-unused-formal-registers | src/data_flow.rs | src/data_flow/build/file.rs | unchanged |
| MO-15-member-only-flag-off | src/data_flow.rs | src/data_flow/build/file.rs | unchanged |
| MO-16-base-not-registered-for-projection | src/data_flow.rs | src/data_flow/build/file.rs | 8 spaces |
| MO-43-kill-skip-off | src/data_flow.rs | src/data_flow/build/file.rs | 8 spaces |
| MO-44-use-at-line-anchor | src/data_flow.rs | src/data_flow/build/file.rs | 8 spaces |
| PD-31-own-scope-lvalue-filter-off | src/data_flow.rs | src/data_flow/build/file.rs | 8 spaces |
| PD-32-own-scope-alias-filter-off | src/data_flow.rs | src/data_flow/build/file.rs | 8 spaces |
| PD-33-non-reference-filter-off | src/data_flow.rs | src/data_flow/build/file.rs | 8 spaces |
| PD-35-alias-twin-line-restriction-off | src/data_flow.rs | src/data_flow/build/aliases.rs | unchanged |
| PD-51-rvalue-read-filter-off | src/data_flow.rs | src/data_flow/build/file.rs | 8 spaces |
| PD-55-r1-nested-write-kill-off | src/data_flow.rs | src/data_flow/build/file.rs | unchanged |
| PD-60-r1-alias-byte-identity-off | src/data_flow.rs | src/data_flow/build/aliases.rs | unchanged |
| PD-61-r1-endpoint-byte-identity-off | src/data_flow.rs | src/data_flow/build/file.rs | unchanged |
| PD-72-r2-use-occurrence-off | src/data_flow.rs | src/data_flow/build/file.rs | 8 spaces |
| PD-75-r2-with-def-admission-off | src/data_flow.rs | src/data_flow/build/file.rs | unchanged |
| PD-76-r2-with-use-admission-off | src/data_flow.rs | src/data_flow/build/file.rs | unchanged |
| PD-90-r5-formal-occurrence-refusal-off | src/data_flow.rs | src/data_flow/build/file.rs | unchanged |
| PD-92-r5-uninitialised-seam-var-off | src/data_flow.rs | src/data_flow/build/file.rs | unchanged |
| PD-101-r9-receiver-def-and-kill-fence-off | src/data_flow.rs | src/data_flow/build/file.rs | unchanged |

No implementation Git/index writes, socket binds, uv or network were attempted. The 34 socket tests in `.review/dfg-A/socket-exclusions.txt` remain excluded from MCP; default has no environment exclusions. The one existing ignored test is `resolution_test::slice_elem_variant_reserved`. Tier-A and the full 425-mutant authoritative execution remain controller work; this fix ran the requested full-catalog planner and normal advisory scoped execution. Native committed-tree scoped validation remains with the controller because this clone deliberately leaves Task1 source additions untracked. Prior Task1 host-only executable replay remains outside this repair. No production file, original pin, cache version, or mutation obligation changed.

- Incremental artifact custody: fix1.patch is against the reconstructed task1.patch tree, never local HEAD's pre-Task1 tree. fix1-message.txt preserves the exact requested subject and both Task1 trailers. Source/checkpoint evidence is retained; controller must copy the fix1 evidence archive and perform authorized publication.
