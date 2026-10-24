# O — normal-serving observation for the I0 readout

Plan only, 2026-10-23. Inspected clean `plan/serving-observer` at `6c46a465169ef71b5222aba4da898f8412ec1fcf`.
Only this new file is written; controller owns commit/snapshot and lane handoff custody. No implementation, cargo,
builds, tests, index loads, daemons, measurements or dotfile reads. Static source reads and weighted-size checks only.
**[R]** denotes a fact read at this revision; **[D]** denotes a chosen implementation/verification contract, not measured evidence.

## Decision and dependencies

O supplies evidence for deciding whether I0 should become default on a 24 GiB host: physical idle relief versus reload
delay and failed/abandoned use. It cannot establish the separate <1 GiB active cache-hit index / 12–16 checkout capacity claim.
Binding: `2026-10-19-i0b-i0c-freshness.md:142–200` (O producers/schema, I0c rows, verification, readout prerequisites 1–4);
parent `2026-10-18-optimization-round2.md:64,82–132,168–170`. Both plans and the complete I0b-2 brief were read.
I0b-2 brief: `~/prism-evidence/mcp-memory/optimization-round2/idle-unload/i0b-2/brief.md`, especially rulings 1–5.

**[D] Ordering:** preserve **F → I0b-1 → I0b-2 → I0c → admitted measurement → owner decision**.
O implementation starts only after I0b-2 merges: **O1 → O2 → O3**; I0c can design against this schema but takes shared
files only after O3. Complete both-mode acceptance waits for O3, I0c, real producer transcripts and neutrality evidence.
No concurrent O edits to I0b-2 files; rebind its final symbols/schema before dispatch, without treating its work as absent.
Two review rounds per PR, declared before dispatch; one drafting plus one static verification pass for this document.
At cap, target closed converging findings and disclose any extension; recurring open-class findings return to design.

## Read implementation basis and chosen hooks

Unqualified MCP filenames below are under `src/mcp/`; `navigation/` paths are under `src/`.

| Read fact on `6c46a465` [R] | O hook / decision served [D] |
|---|---|
| `src/mcp/mem_sample.rs:6–32`, `sample()` returns this process's optional RSS/footprint/compression/lifetime peak/allocator-in-use bytes. | Call this function in each process; never sample a daemon by calling it in a shim. Physical relief admission needs current footprint, not a peak or heap proxy. |
| `mem_report.rs:50–72` `Recorder::{start,mark}` retains a Vec; `run():299–330` bootstraps and forces lazy edges. `src/bin/prism-mcp.rs:239–257` requires eager report mode. | Reuse `MemSample` and units, not Recorder's lifetime, eager driver or forced edge initialization. O observes normal Lazy/Background, including idle-disabled resident control. |
| `mem_attribution.rs:35–37` `attribute()` walks/allocates; `mem_tree_alloc.rs:26,51` `live_bytes()/install()` require global allocator hooks installed before parsing. | Do not install hooks or run a census. Optional attribution stays deferred; no unmeasured residual attribution or new framework. |
| `src/profile/mod.rs:38,204` `Session::start` / `Phase::serial` are opt-in; phase drop locks a collector, output is finalized at exit. `src/repo_loader.rs:64–65` profiles `load_parse`. | Reuse stage meanings and existing call boundaries; do not enable PRISM_PROFILE, flush its collector, copy per-file profiling, or depend on shutdown for serving spans. Profile files are optional diagnostics only. |
| `session.rs:167–189,425–447` `bootstrap/bootstrap_observed/build_state`; `load_repo` → `FreshnessProbe::from_loaded_repo` → index construction. `freshness.rs:89` calls `capture::validate`. | Scoped `bootstrap`, `source_load_parse`, `freshness_capture`, `f_validation`, `index_construct` intervals; tiny timestamp hooks immediately around actual calls. Source interval includes load/parse, not a claimed parse-only measurement. |
| `session.rs:383,406,451` `build_verified_candidate/verify_snapshot/build_candidate_state` can construct twice and load again for verification. | Each construction and verifier gets a distinct build ID/parent span; explicit unloaded refresh retains bootstrap plus refresh. No flattening into one misleading reload span. |
| `navigation/cache.rs:44–143` `NavigationIndex::build_cached_at` matches `CacheResult::{Hit,PartialHit,Miss}`; partial graphs drop before rebuild. | Record the actual matched result before ownership moves, keeping drops in place. Reasons: `exact_hash_topology`, `partial_rejected`, `unusable_cpg`; `disabled`/`bypassed` at session's no-cache/incremental branch. Never infer from elapsed time. |
| `navigation/call_edge_cache.rs:199–225` `load_best_effort`; `load():242–263` collapses absent/decode/fingerprint failures. `navigation/mod.rs:261–292` getters do not force edges. | Hook only actual sidecar attempts: `exact_hit`, `cpg_not_exact`, `dirty_build_refused`, `absent_decode_or_fingerprint`, `io_error`. Until normal use forces edges, report `not_attempted`; no fabricated miss. Finer CPG reject taxonomy is deferred: it does not change hit/miss admission. |
| `mcp/mod.rs:61–100` `run_with_idle_policy/route_stdio/serve_legacy` selects the legacy path when idle is off. `lazy.rs:123,180` `ensure_ready/spawn_build` collects completion on demand. | Start O before either runtime is constructed; instrument legacy build completion and publication separately. Never route control through IdleRuntime or drain its completion receiver to get observations. |
| `idle_runtime.rs:146,171,235` `spawn/accept/tick`, `Activity::drop():68`; `idle_stdio.rs:109–157` `serve_events` releases activity before writing. | Stdio initial/reload publication, build failure, last release, unload, disposal start/end, relief, explicit/auto refresh outcome; preserve `finished_at` versus actual publication/collection timestamps. Emit response-owned timestamp after dispatch, flush after lease release. |
| `session_idle.rs:7,18` `idle_publication/resume_idle_generation` computes snapshot identity; `idle.rs:116` `pressure_relief` is post-drop/no-op. | Reuse existing compact publication values, never recompute a fingerprint for O. Bracket the existing relief/no-op once with samples; do not change applicability, generation, or call count. |
| `transport.rs:103,127,620` `serve_connection/serve_loop/dispatch_tool`; daemon `connection.rs:150,165` `run_tool_call/dispatch_tool_call`. | Request accepted, readiness answer and owned response/error spans with connection/request ordinals; capture before write, report write failure separately. Client arrival remains I0c's timestamp. |
| `daemon/coordinator.rs:499,537,554,610` `start_attempt/run_build/publish/settle`; daemon `idle.rs:74,169` `unload_tick/DisposalCompletion::drop`; daemon `activity.rs:71` release guard. | Consume I0b-2's lifecycle records, including locked-unload duration and existing release timestamp. Add build-context propagation at `run_build`, response spans outside provider locks, post-drop sample brackets only; no second lifecycle emitter. |
| `daemon/run.rs:40,63–73` `run_daemon` creates a userspace epoch-seconds `start_time`; `log.rs:10–58` truncates >1 MiB at startup and caps at 8 MiB. | OS start identity is additional evidence, not this timestamp. After merge, enrich the single I0b-2 log serialization with O envelope/headers; preserve the key-log lifecycle owner and its cap/truncation semantics. |
| `shim/run.rs:39,50,156` `run_shim/shim`; shim `spawn.rs:66` `daemon_command` inherits env; shim `pump.rs:43` `pump` is byte-transparent. | Start the shim's own observer before arbitration; bind READY peer to daemon header asynchronously on O's worker. Inherited opt-in enables newly spawned daemon; existing unobserved daemon remains unobserved and is excluded. No pump parsing, observer-driven reconnect or extra STATUS polling. |

## Exact record contract [D]

UTF-8 JSONL, one discriminated format for both modes and every process. O schema **1** is independent of I0b-2
protocol/control **2**. Required keys are always present; unavailable values are JSON null with a reason, never zero.
Integers are checked u64 (signed i64 for deltas/clock offsets); bool is not an integer. No float NaN/Infinity or saturating timestamps.
Each record has exactly these envelope fields; variant payload fields below are exhaustive for version 1:

| Envelope field | Type / meaning |
|---|---|
| `o_version`, `kind`, `name` | `1`; `header/clock/event/span/cache/sample/health/end`; stable enum name. |
| `pid`, `os_start`, `instance_id`, `host_boot_id` | PID; `{source,value}` OS start identity or null; fresh 128-bit random hex nonce per process; OS boot identity or null. Never use PID alone. |
| `mode`, `role`, `key` | `stdio/shared`; `stdio/daemon/shim`; `{kind:daemon/stdio,value:hex}`. Full daemon key, not key16. Stdio equivalent hashes canonical root + root dev/inode + configured cache mode/base + build identity + refresh policy. No observer/idle setting enters either key. |
| `stream_id`, `seq`, `mono_ns` | Unique stream nonce; starts at 0, increases per produced record including dropped records; host CLOCK_MONOTONIC nanoseconds at the producer boundary. |
| `cycle_id`, `attempt_id`, `build_id`, `connection_id`, `request_ordinal` | Nullable u64 scoped to instance; cycle 0 initial, then each successful detach; attempt from runtime, build distinguishes candidate/retry/verifier; connection ordinal and accepted-tool ordinal disambiguate repeated RPC IDs. |
| `generation`, `payload` | Nullable published G (unloaded wire G=0 is separate from retained G); object specified below. |

| Kind/name | Exact payload fields |
|---|---|
| `header/stream` | `role_stream` (`observation/lifecycle`), `build_identity`, `git_sha`, `binary_input_dirty`, `platform_os`, `platform_arch`, `allocator`, `startup`, `refresh_policy`, `cache_mode`, `first_call_wait_ns`, `effective_idle_secs`, `effective_relief`, `wire_start_time`, `root_digest`, `peer_instance_id`, `expected_streams`, `segment`, `previous_segment_last_seq`, `sample_period_ns`, `queue_capacity`, `record_cap_bytes`, `stream_cap_bytes`, `identity_error`. `expected_streams` entries: `{stream_id,role_stream,sink}`. `peer_instance_id` null before binding. |
| `clock/correlation` | `mono_before_ns`, `unix_ns`, `mono_after_ns`, `instant_offset_ns`, `calibration_error_ns`, `source`, `error`. Bracket realtime with monotonic reads; uncertainty is bracket width plus Instant calibration error. Emit at startup, every 30 s, and close. |
| `event/<lifecycle>` | `reason`, `outcome`, `event_start_ns`, `event_end_ns`, `observed_idle_ns`, `configured_idle_ns`, `locked_section_ns`, `fingerprint`, `diverged`, `retained_generation`, `released_bytes`, `error_code`, `error_text`, `error_truncated`. Names: `publication_initial`, `publication_reload`, `build_failure`, `last_release`, `unload`, `disposal_start`, `disposal_end`, `relief`, `refresh_explicit`, `refresh_auto`; inapplicable fields null. |
| `event/peer_binding` | `peer_pid`, `peer_wire_start_time`, `peer_key`, `peer_instance_id`, `peer_os_start`, `effective_idle_secs`, `effective_relief`, `client_count`, `binding_error`. Actual READY/header join; late discovery appends this event, never rewrites the header. |
| `span/<stage>` | `span_id`, `parent_span_id`, `start_ns`, `end_ns`, `outcome`, `error_code`, `error_text`, `error_truncated`, `rpc_id_digest`, `tool_name`, `first_for_publication`. Stages: `bootstrap`, `source_load_parse`, `freshness_capture`, `f_validation`, `index_construct`, `verify_snapshot`, `build_complete`, `index_ready`, `request`, `readiness`, `response_owned`, `write_complete`. Point stages have equal start/end, not invented duration. |
| `cache/result` | `layer` (`cpg/sidecar`), `result` (`exact_hit/partial_rejected/miss/disabled/bypassed/not_attempted`), `reason`, `start_ns`, `end_ns`. Coarse reasons above are deliberate; they never claim an unavailable finer cause. |
| `sample/memory` | `trigger` (`pre_index/loaded/periodic/before_relief/after_relief`), `scheduled_ns`, `sample_start_ns`, `sample_end_ns`, `rss_bytes`, `phys_footprint_bytes`, `compressed_bytes`, `peak_bytes`, `heap_in_use_bytes`, `unavailable`. `unavailable` maps each null metric to `unsupported_or_read_failed`; peak scope is always process lifetime. |
| `health/visibility` | `state` (`ok/gap/capped/truncated/io_error/identity_unavailable/clock_error`), `first_lost_seq`, `last_lost_seq`, `lost_count`, `reason`, `last_complete_seq`. Unknown loss count is null. |
| `end/stream` | `last_produced_seq`, `lost_count`, `reason` (`normal/cap/error`), `complete`. No terminal record after crash is incomplete, not a clean end. |

Numeric suffixes/counts/IDs are integers; flags are bools; names, hashes and reasons are strings. `outcome` is
`success/served/warming/indexing/failure/cancelled/panicked/not_applicable`; first-served requires an actual successful tool result,
not JSON-RPC success wrapping warming or tool failure. Null inapplicable fields follow role/event applicability, not missing evidence:
shim has no local attempt/G; resident has no unload/relief; fingerprint is required only where existing I0 publication supplies it.
Periodic samples carry no guessed cycle/attempt/G: I0c binds their intervals against lifecycle records, excluding overlaps/ambiguity.

OS identity: macOS kernel process-start seconds/useconds; Linux `/proc/self/stat` start ticks plus boot ID and tick units
encoded in `os_start.value`. Unsupported/unreadable identity remains null and excludes acceptance. Do not substitute wall launch time.
Existing producer `Instant` values are converted using a bracketed Instant↔CLOCK_MONOTONIC calibration, with error included
in clock correlation; no collector-read timestamp substitutes for release, build finish or disposal. Calibration/bracket error >1 ms,
clock failure or host sleep spanning a scored interval excludes timing; do not extrapolate an offset through suspend.
Sequence is production order within a stream, not timestamp order across concurrent workers; delayed emission may carry older time.
Span IDs and runtime attempt IDs join causality; I0c must not sort by arrival and invent causality. Same-host monotonic time joins
shim/daemon samples; collector journals its own realtime/monotonic brackets. Across hosts or boot IDs, reject the join.

## Sampling, lifecycle and neutrality [D]

Opt-in is `PRISM_MCP_SERVING_OBSERVER=1`; absent/empty/0 is off. Invalid values disable O with one sanitized stderr note,
leaving serving configuration unchanged. `PRISM_MCP_SERVING_OBSERVER_DIR` optionally selects an absolute private output directory;
otherwise use the validated runtime directory. A path alone never enables O. No new feature or identity-key input.
Eager/report/owner runs are outside this readout; O does not turn them into ordinary sessions or install allocator hooks.
Off cost: one startup env resolution, then an empty handle/branch at hooks; no worker, file open, OS sampling, clock reads,
formatting, heap buffer, provider retention or extra cache work. Non-MCP builds compile observation hooks out.

One bounded worker per process samples `mem_sample::sample()` every **100 ms**, on absolute deadlines, without catch-up bursts.
This common cadence runs unchanged in resident, A and A+C; it is needed to align every surviving shim without synchronizing
serving or disposal with a collector. It also samples with no incoming bytes, blocked output and silent completed builds.
Initialize the bounded worker/buffers before the synchronous `pre_index` sample, then allow normal runtime construction;
shim pre-index is before arbitration/spawn. Failure to obtain this boundary is UNKNOWN, never a post-build substitute.
Bootstrap worker takes `loaded` after successful construction and before handing off the provider, with no serving lock;
refresh under a provider lock records scalar stage times only, never invokes a sampler there. Periodic samples cover its memory.
For shim loaded and each surviving process's +5/+30, I0c selects the first scheduled sample starting at/after publication
or **actual disposal_end + 5/30 s**, with lateness ≤100 ms and sample duration ≤20 ms. Keep both endpoints and actual lateness.
Late/absent samples or a reload/refresh overlapping an idle window exclude that cycle. A sampling cadence is not proof of punctuality.

Stdio/daemon disposal workers bracket their existing post-drop `pressure_relief` call (including A's no-op) with immediate
before/after samples, with no formatting/allocation between the pair and call; their signed live-heap delta is after minus before.
Shims never call relief: use their last completed periodic sample before relief_start and first sample after relief_end,
each within 100 ms. Keep per-process deltas, bracket widths, and the summed checkout delta; this is a bracketed observation,
not an assertion of atomic cross-process sampling. Missing/bracket-too-wide/concurrent-work pairs are UNKNOWN/excluded.
Require ±1 MiB heap stability for the releaser and checkout aggregate before attributing A+C's physical effect to allocator relief.
Resident control has no disposal/relief events: align scheduled samples to the matched candidate's elapsed-from-last-release
boundary (including disposal duration), explicitly labelled `matched_resident_boundary` by I0c, never emit a fake unload.

Observer state contains only IDs, clocks, bounded scalar records and capped text; no provider/repo/index Arc, response Value,
source path collection or retained error object. Capture scalar timing locally; defer queue submission, serialization, sampling
and I/O until after existing provider/control guards and activity leases are released. Refresh/cache stage records use a fixed
thread-local buffer scoped to the build/request, then drain outside locks; propagate context explicitly into build threads.
No added serving-lock acquisition/hold, timestamp-driven policy branch, wait deadline change, idle tick/message, wake of the
runtime, fingerprint serialization, forced cache, extra stat/hash validation, profiler collector or blocking queue send.
Use a bounded nonwaiting queue: 256 records, total observer record storage ≤256 KiB/process, worker stack 256 KiB;
overflow or sink failure drops observation, records loss, and never delays/fails service. Bound scoped buffers for all 64 clients.

**Unit preservation witnesses:** off spies observe zero observer work; identical runtime branch, builder/drop/relief counts,
activity/deadline histories, cache results and output bytes with on/off; Weak/drop barriers prove no lifetime extension.
Block the sink, fill the queue, hold a provider lock, stall stdout/socket, inject worker failure, EOF, panic and timer races;
serving/release proceeds with the same lease boundary. Except pre-runtime baseline, samples run off main/server loop and outside serving locks.
No unit witness proves physical or timing neutrality; allocator statistics can contend with real malloc work even off-lock.

**Controller on/off protocol:** admitted post-soak, one binary/hash/features, frozen corpus/cache/workload/settings/threads,
fresh instances, isolated shared runtime dirs, alternating on/off order, ≥3 pairs per mode/arm on each of E/P/ruff/Py.
Include Lazy/Background, cache hit/miss, resident control, A, A+C, no-op, refresh, blocked-write and repeated unload/reload cases.
Compare full response bytes/G/freshness/errors, cache dispositions, build/relief/drop counts, last-release/unload eligibility,
warming/failure/served/abandonment counts; deterministic barrier cases have **zero semantic tolerance**.
Compare client-journal load/ready/first-served/workload medians: increase ≤max(5%,100 ms); idle-boundary jitter ≤100 ms;
heap increase ≤1 MiB/process, current +30 footprint increase ≤1 MiB/process and ≤2 MiB/checkout;
lifetime peak increase ≤max(1%,16 MiB). External host footprint/RSS sampling is identical on/off; heap checks use an identical
minimal endpoint-only support probe on both sides, without periodic O recording. Record its patch/hash and cost separately.
Separated excess is FAIL, noisy overlap/excess INCONCLUSIVE; a changed warming/abandonment classification at the wait boundary
also requires resolution. No subtracting guessed observer overhead or relaxing the 128 MiB bar. Neutrality failure blocks timings
and activation evidence; fix O rather than changing I0 policy. At most one preregistered follow-up pair under parent admission rules.

## Sink and stream completeness [D]

Observation stream: `<dir>/o-<instance_id>-<stream_id>.<segment>.jsonl` for stdio, shim and daemon alike; mode 0600,
exclusive creation through a validated private directory descriptor, no symlink following, no overwrite or cross-instance append.
Reuse `shared/runtime_dir.rs:181,265` validation and `daemon/files.rs` descriptor-relative conventions; do not invent a second ACL policy.
Single writer uses append writes; segment limit 4 MiB, four segments / **16 MiB per instance** including reserved terminal space;
max record 4096 bytes, error text ≤256 UTF-8 bytes with truncation flag. Flush at most every 100 ms, never fsync on serving paths.
Rotate only between records; sequence continues, new segment repeats identity/clock header and links with `previous_segment_last_seq`.
Never delete old segments automatically. Reserve room for health/end; crash, failed terminal write or partial JSON line stays incomplete.
At cap stop recording, retain a loss counter/terminal state; never silently wrap. Text is JSON escaped and control characters sanitized;
no source, arguments or env dump; paths in errors are escaped/capped. RPC IDs are hashed; allowlisted tool names and bounded failure text suffice.

Daemon lifecycle remains **only in I0b-2's key-log**, not copied into the observation file. O3 adds envelope/sequence/clock headers
to that existing emitter's single record (retain its normal text when O is off); prefixed JSON is parsed after the fixed log prefix.
Extend only the O-enabled record limit to 4096 for valid JSON, preserve 1 MiB startup truncation and 8 MiB total cap with a terminal
visibility marker. Carry I0b-2's producer timestamps, last-release event and locked-unload duration unchanged; no fresh snapshot work.
The observation header declares that lifecycle stream; I0c retains stderr, key-log inode/device/offset and O segments. Missing key-log,
startup truncation, file replacement/shrink, sequence gaps, cap notes, malformed tail or missing close cannot mean zero activity.
Deduplicate `(instance_id,stream_id,seq)` only when bytes match; conflicting duplicates exclude. A killed process cannot mark its own
missing stream: I0c's launch manifest and expected-stream header identify absence. No acceptance until the observation window closes.
MCP stdout/framing and shim pump bytes remain identical; no observer record or marker goes to stdout or an MCP tool result.

## Exclusive file ownership, size and sequencing [D]

Brace lists below enumerate exact files, not directory-wide permission. Estimates include tests/mutants, not measured effort.
O1 is schema/sampler/sink (550–750 weighted additions), O2 stages+stdio (650–850), O3 daemon+shim (450–650).
Inventory each PR before dispatch; split further before implementation if its finite behavior set cannot converge in two rounds.

| Slice | Exclusive owned files / scope and estimated additions |
|---|---|
| O1 | NEW `src/mcp/serving_observer.rs` (facade/context, ≤120), `src/mcp/serving_observer/{schema,clock,sink,sampler}.rs` (≤80/100/140/100), `src/mcp/serving_observer/{schema_tests,sink_tests,sampler_tests}.rs` (≤80/80/100); `src/mcp/mod.rs` registration only. No serving hooks activated yet. |
| O2 | NEW `src/mcp/serving_observer/{stages,stdio,stage_tests,stdio_tests,neutrality_tests}.rs` (≤100/100/150/150/140); existing `src/mcp/{session,freshness,lazy,idle_runtime,idle_stdio,transport}.rs`, `src/navigation/{cache,call_edge_cache}.rs` emission/context hooks only (≤150 total). `src/mcp/mod.rs` and `serving_observer.rs` transfer from O1 for startup/child registration. |
| O3 | NEW `src/mcp/serving_observer/{shared,shared_tests}.rs` (≤100/160); existing `src/mcp/daemon/{run,log,coordinator,connection,idle}.rs` (≤100 total), `src/mcp/shim/run.rs` (≤20); transfer `serving_observer.rs` for shared registration. NEW `tests/mcp/serving_observer_test.rs` (≤180), `tests/mcp/main.rs` registration only. |
| Sequential O1 → O2 → O3 | NEW `mutants/manual/o.json`, existing `mutants/manual/README.md`: add only implemented mutants/selectors each slice (≤80 total). This plan receives settled schema changes sequentially. No parallel ownership. |

From I0b-2 **after merge**, O3 takes `daemon/{run,log,coordinator,connection,idle}.rs` for the narrow hooks above;
`activity.rs` remains read-only, consuming its already-produced release through I0b-2. No wire/CLI ownership transfer to O.
After O3, `shim/run.rs` transfers to I0c. I0c exclusively retains `src/bin/prism-mcp.rs`, `src/mcp/idle.rs`,
`idle_fix_tests.rs`, `shim/{spawn,status,mod,spawn_tests,status_tests,idle_tests}.rs`, CLI idle tests, operator docs,
`scripts/memreport/{memreport.py,README.md,memreport_idle.py,memreport_idle_readout.py,test_memreport_idle.py,test_memreport_idle_readout.py}`
and `mutants/manual/i0c.json`; `tests/mcp/main.rs` transfers O3→I0c for its separate `idle_process_test.rs` registration.

**Must not touch:** cache formats/version/validity, CPG/AST/call resolution, loader/resolver, F validation/policy semantics,
`session_idle.rs`, idle settings/relief implementation, transport framing, `transport_tests.rs`, daemon admission/leases/fatal policy,
shared identity/wire, shim pump/arbitration/spawn policy, eager memreport/attribution/tree allocator/profile modules, existing scoring,
workload pins, launchers, soak configuration, shell profiles and env files. The named freshness/transport/cache hooks change observation only.
S0/tree/D13 owners must reserve the hook files before dispatch; no ownership inferred from a shared symbol.

**[R] Static sizes checked with `python3 scripts/file_size.py`:** session 862, freshness 631.5, lazy 470,
idle_runtime 398.5, idle_stdio 169.5, transport 907.5, navigation/cache 144.5, call_edge_cache 893;
daemon run/log/coordinator/connection/idle 139.5/73.5/608.5/197/168.5; shim/run 168.5; mcp/mod 136.
Navigation/mod is 1145.5 and cpg_cache 1526: both stay read-only by using existing getters/coarse actual-result reasons.
Aim ≤600 weighted lines per new file, keep touched facades <1,000; **no file already >2,000 may grow**.
Re-run `python3 scripts/file_size.py` on every owned existing/new file before/after each PR; a required extraction is a separate move-only PR.

## RED, negative witnesses, mutants and real transcripts [D]

Each named test below asserts records/behavior, not mere successful execution; every new behavior needs a same-environment
pre-slice-parent RED via behavior-only hook backport, with unchanged production behavior. Compilation/zero-selection/refusal is inadmissible.
Existing-green on/off preservation witnesses are listed separately from new-producer RED. Register tests in the owning module.

| RED test / slice | Positive assertion; negative/edge case; manual mutant in `mutants/manual/o.json` |
|---|---|
| `o_identity_clock_sequence` / O1 | Restart/PID reuse changes instance/start binding; contiguous sequence and bounded clock bracket. Missing OS identity, wall jump, unknown version, counter overflow reject completeness. `O-pid-only` must fail distinct-instance assertion; `O-wall-as-monotonic` must fail wall-jump duration assertion. |
| `o_sink_loss_is_unknown` / O1 | Complete escaped records, bounded queue/rotation and explicit loss; full disk, blocked writer, cap, symlink, multibyte truncation and abrupt EOF never complete. `O-hide-gap` must fail health/lost-count assertion. |
| `o_sampler_self_and_schedule` / O1 | Injected own-PID sampler records deadlines/endpoints through silence; unavailable metric stays null, missed deadline is visible, no catch-up burst. `O-null-is-zero` must fail null assertion; `O-sample-on-tool` must fail silent-period sample count. |
| `o_actual_cache_and_stages` / O2 | Exact/partial/corrupt/no-cache/bypass and sidecar refused/not-forced classify actual branches with scoped begin/end; F failure and verifier retry keep separate error spans. Artificially fast miss stays miss. `O-speed-is-hit` fails that classification; `O-omit-f-validation` fails required span presence. |
| `o_stdio_lifecycle_boundaries` / O2 | Lazy/Background, resident/enabled, initial/reload/failure, release, disposal and both refresh outcomes carry real IDs/times. Delayed completion, EOF, disposal timeout and warming→retry preserve distinction. `O-collection-time` fails preserved finished-at assertion; `O-refresh-kind-lost` fails explicit/auto distinction. |
| `o_response_before_write` / O2/O3 | Block output after owned response: response time precedes write and lease release is unchanged; failure/warming are not first-served. `O-served-at-arrival` fails blocked-writer timestamp assertion. Concurrent identical RPC IDs remain distinct by connection/ordinal. |
| `o_off_and_retention_neutral` / O2 | Off has zero observer calls/allocations; on/off output/cache/deadline/build/drop parity under blocked/full sink, no retained provider or forced edges. `O-force-sidecar` fails uninitialized-edge witness; `O-reset-idle` fails original unload deadline; `O-retain-provider` fails Weak release before +5. |
| `o_shared_single_lifecycle_and_relief` / O3 | Exactly one I0b-2 record/event with original event time; daemon/shim identities differ, startup/peer binding agrees with READY. Existing unobserved peer, v1, restart, late attachment exclude. `O-reemit-daemon` fails exactly-once count; `O-shim-as-daemon` fails sample identity assertion. |
| `o_relief_brackets_after_drop` / O2/O3 | Destructor witness precedes before-relief→call→after-relief; A is one no-op, off/non-macOS no physical relief, null heap never zero delta. `O-relief-before-drop` fails ordering assertion; `O-relief-pair-reused` fails distinct sample/timestamp assertion. |

Manual catalog selectors are fully qualified, one executed MCP-feature test each (shared variants get distinct selectors).
Follow `mutants/manual/README.md`: exact anchors, before/mutant/restored SHA-256, behavioral RED, restoration in finally,
restored GREEN. Hand-run `--features mcp`; shared no-feature mutation runner cannot certify O. Keep its full gate separately required.
Full default and MCP library/integration/binary suites, fmt/clippy/shared-catalog gates and affected Tier-A matrix/quick after immediate
release rebuild are future controller gates (navigation hooks trigger Tier-A); report passed/failed/ignored/filtered and all host exclusions.
I0c runs its full Python collector suite. Unrelated failures need same-environment base controls, never quiet rebaselining.

**Real producer obligations, separate from parser fixtures:** controller retains raw binary-bound stdio and shared transcripts,
each covering Lazy/Background, resident/A/A+C, genuine pre-index→loaded→disposal→+5/+30, repeated exact reload and changed miss,
explicit/auto refresh success/failure, warming→poll→success and abandonment. Shared evidence includes every surviving shim plus
daemon and the original key-log; demonstrate the ±1 MiB relief heap comparison with real samples. Small process fixtures prove
availability/byte parity only; four-corpus admitted measurements prove relief/latency. Parser fixtures cannot stand in for either.

## I0c readout interface and admission [D]

I0c reads O JSONL segments + I0b-2 prefixed lifecycle JSON in key-log + stdio/shim stderr + actual READY/STATUS receipts,
then joins its frozen batch manifest and request/response-arrival journal. It owns parsing, deduplication, cycle construction,
derived durations/deltas/footprint levels, all denominators and exclusions; O owns no scorer or admission verdict.

| Every required I0c row field (`i0b-i0c-freshness.md:167`) | Producer → readout decision |
|---|---|
| Binary/commit/features; corpus pin/workload hash; mode; A/A+C/resident/no-op | Controller manifest hashes actual executable/features and pins workload; O header corroborates build/mode/role. One binary per batch and matched inputs, no side mixing. |
| Cache classification/reason; source-parse, F/bootstrap, ready, first-served durations | O actual cache/stage spans: build-start→index_ready; demand request→readiness (includes disposal wait); build-start→first successful response_owned per publication. Background-ready before request is reported as already ready, not a negative duration. I0c client arrival times remain separate. Exact-hit relative reload gate versus explicit misses. |
| Requested/effective idle/relief; first-call wait; pid/start/key/client count | Manifest/request journal for requested/wait; O stdio resolved policy and own identity; I0b-2 READY/STATUS for daemon effective/client count; O peer binding/header for OS start/instance. Mismatch excludes intended arm; requested never fills effective. Stdio client count is one. |
| Cycle/attempt/G; publication/release/unload/disposal/reload timestamps | O stdio lifecycle; I0b-2 daemon lifecycle; O bootstrap/readiness spans. Separate finish/publication, retained/wire G, failed attempts and disposal waits; no survivor-only reload distributions. |
| Pre-index/loaded/+5/+30 sample times; footprint/RSS/heap/compression/levels | O own-process samples and clock brackets; I0c chooses scheduled boundaries, sums distinct surviving instances once and classifies footprint−heap levels before unblinding. Missing member makes checkout total UNKNOWN. |
| Relief-return bytes/live-heap delta | I0b-2 daemon relief or O stdio relief; O before/after samples and shim brackets; I0c signed deltas. Real zero no-op differs from unavailable. |
| Served-within-wait/warming/failure, failure text; polling/pre-warm and abandonment | O readiness/response_owned outcomes plus client journal arrival/EOF/timeout/poll sequence. Hash joins include request occurrence, not just RPC ID; missing response remains outcome evidence. Pre-warm within N of use. |
| Visibility/admission/exclusion reason | O headers/seq/health/end plus I0c file identity/offset/launch inventory and controller host/admission journal. Keep raw row, every failure and exclusion; UNKNOWN is not zero or PASS. |

Required producer field missing, wrong-instance/start/boot/key sample, unsupported required metric, cap/gap/truncation,
unbounded clock alignment, missing survivor, invalid sample window or effective≠requested idle/relief ⇒ **EXCLUDED from acceptance**,
retaining raw row and reason. Legitimate build/query failure remains in preregistered outcome counts; no successful-sample denominator
may erase it. Sidecar `not_attempted` does not pass a true CPG+sidecar-hit workload row; do not force it to manufacture admission.
Genuine pre-index must precede any build; peer already loaded at attach cannot supply a missing baseline by subtraction.
Clean close/complete sequences plus all expected streams are required for the scored interval; neither exit 0 nor Weak-drop proves relief.

Controller still supplies confirmed soak completion, C0 Py pin, ≥50 GiB free, quiet/serial admitted window and non-increasing swap.
I0c preserves M: matched current idle footprint median savings ≥200 MiB OR ≥15% on ≥2 corpora, ≥3 admitted runs/side,
candidate max < base min; +30 footprint ≤128 MiB/checkout on all four corpora in both modes. 15% idle and pre-index +16 MiB are
report-only. Exact-hit reload median ≤ fresh-lazy +max(5%,100 ms), ready ≤3 s target; wait stays configured (20 s default).
Keep missed targets and served/warming/failure distributions visible. N remains provisional 600; O neither selects it nor activates I0.

## Owner questions and handoff

- **Cadence/cost:** accept 100 ms sampling/alignment and the stated neutrality tolerances? Recommended yes provisionally;
  require measured neutrality, and redesign sampling if it fails, rather than subtract overhead or weaken the physical bar.
- **Scope:** accept three sequential PRs and coarse actual cache refusal reasons? Recommended yes; finer diagnostics, census,
  general profiling, adaptive cadence, process-exit B and active-capacity instrumentation are deferred because they do not decide I0 activation.
- **Activation/N:** default-on and timeout remain owner decisions at admitted readout. Recommended keep off/600 provisional
  until complete both-mode evidence; no gap-distribution evidence is supplied here.

Stable handoff: design only; O1/O2/O3 and I0c pending. Controller commits this single document and records exclusive transfers,
verification/admission scheduling and final I0b-2 rebind in the controller handoff. No separate handoff or evidence file is written here.
