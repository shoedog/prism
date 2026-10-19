# I0 freshness prerequisite, I0b daemon, I0c shim/operator

Design written 2026-10-18; dispatch document dated 2026-10-19. Checkout `prism-i0b-plan`, branch `plan/i0b-freshness`, clean starting HEAD **`1d01b0a72daa2297d90529676f542ebcd4ada401`**. All existing-file anchors below refer to that commit; **NEW** paths have no base line. Controller commits. No cargo, builds, tests, index loads or performance measurements were run for this plan; file-size counts are static source counts.

Goal: a2a-bridge fan-out on a **24 GiB host without swap exhaustion**, retaining the **<1 GiB per cache-hit index / 12–16 checkout** goal. I0 returns idle sessions' memory; it does not establish the active-index or host-capacity target by itself. I0a shipped stdio-only in #491 (`6abab1e5`); S0 #487 is merged. **Absent flag/env means OFF**, including after I0c. Default activation remains an owner decision at the measured readout. Measurement commands explicitly pass **`--index-idle-secs 600`**.

## Authority, ordering and dispatch gates

- Binding parent: [optimization round 2](2026-10-18-optimization-round2.md), I0 row at :64, I0 design/acceptance table at :82–114, Round-1 fold and Round-2 post-cap fold at :116–132, M exception at :168 and I0 acceptance at :170. This plan refines those contracts; it does not relax them.
- Binding I0a evidence: `~/prism-evidence/mcp-memory/optimization-round2/idle-unload/i0a/review-r1-opus.md`, `review-r1-sol.md`, `review-r2-opus.md`, `review-r2-sol.md`. Round-2 resolutions govern earlier default/lease findings. Their execution results are supplied receipts, not executions here.
- Read implementation basis: `src/mcp/idle.rs:19,90,115`, `idle_runtime.rs:132,174,226,309`, `idle_stdio.rs:109,150`, `session_idle.rs:7,18`, `mod.rs:44,61`, `src/bin/prism-mcp.rs:294,314`. Preserve I0a's fixed disposal budget, post-wait elapsed clock, enabled-only routing and generation choice.
- Merge order: **freshness → I0b → I0c → admitted measurement → owner activation decision**. Freshness implementation may overlap daemon drafting, but merges before I0b's read-A/edit-B/stamp-B reload witness. I0b is explicitly disabled by default before forwarding exists. S0/T3a remain independent; D13 is not pulled into I0.
- File ownership is exclusive per slice below. Sequential handoff of `prism-mcp.rs` and shared test fixtures is explicit; no concurrent editing of those files. Controller reserves session hooks against tree work and daemon hooks against D13 before implementation dispatch. Everything not enumerated as owned is read-only.
- **Review cap: two rounds per PR**, declared before dispatch. One drafting pass plus one static verification pass for this document; no review agents dispatched. At cap: targeted fixes for closed, converging findings, disclose any extension; recurring open-class findings stop for design correction. No restart to evade review.
- Estimates below are guessed weighted additions including tests, not quotas or measured effort. Aim ≤600 per new file; split responsibilities above 1,000, never cross 2,000. Oversized files must not grow. `transport_tests.rs` is **2,303.5**, forbidden; current daemon tests are all <1,000 (largest `connection_tests.rs` 508.5). Recheck on dispatch/rebase; any daemon test >1,000 is frozen, with new coverage in new files.

## F — source-bound freshness (separate prerequisite PR)

**Goal / existing defect.** **WRONG / IMMATERIAL in the inherited review; mandatory I0 dependency.** `load_repo` reads A at `src/repo_loader.rs:1094–1132`; bootstrap stamps later at `src/mcp/session.rs:410–413` via `freshness.rs:53–87`. Editing to B in that interval makes the probe accept B's stamp while the index contains A. This predates I0; I0 increases exposure, not the defect's origin. Source evidence establishes the mechanism; same-environment executable base RED remains required.

**Owned files (exact):**

| Existing anchor / NEW path | Ownership and size forecast |
|---|---|
| `src/mcp/session.rs:406,432` | Bootstrap and refresh-candidate capture wiring only; test module declaration. Current 840.5; ≤30 added. |
| `src/mcp/freshness.rs:24,53,76,132` | Probe consumes source-bound evidence and preserves mismatch evidence; module declarations. Current 605.5; ≤35 added, new logic in child module. |
| **NEW** `src/mcp/freshness_capture.rs` | Capture/validation responsibility, ≤250. |
| **NEW** `src/mcp/freshness_capture_tests.rs` | Deterministic capture barriers and input-universe cases, ≤250. |
| **NEW** `src/mcp/freshness_policy_tests.rs` | Production bootstrap/dispatch policy witnesses, ≤220. |
| **NEW** `mutants/manual/i0-freshness.json` | Feature-aware manual registry, following `idle-unload-i0a.json:1`. |

**Must not touch:** daemon/shim/shared, idle runtime/policy/stdio, `transport.rs`/`transport_tests.rs`, `repo_loader.rs`, manifest/resolver implementation, navigation/CPG/cache formats, harness or unrelated S0 code. Read loader contracts at `repo_loader.rs:36–44,64–77,188–305`; typed manifest evidence at `manifest_snapshot.rs:7–49`; resolver topology at `js_paths_snapshot.rs:222–295`.

**Chosen contract:** stamp **before loading**, then re-check after capture and before service. The capture covers a conservative superset of the loader's source, manifest and resolver inputs, including directory membership, present/absent resolution candidates, symlink/refusal states and relevant ancestors. A pre-load metadata walk must cover loader-consulted inputs, not merely `LoadedRepo.files` after the fact; newly created/deleted paths must be observable. Do not treat synthetic topology keys such as `js_paths_occupancy` or `js_paths_config:…` as literal filesystem paths.

Preserve pre-read evidence through bootstrap/candidate construction; any intervening discrepancy remains stale for that snapshot even if post-load stamps subsequently stabilize. An input discovered only after reading has no pre-read proof: validate it against captured content/topology before admitting it as clean, or conservatively retain stale/unknown evidence. Incomplete/unreadable capture cannot certify clean. Retain compact evidence, not a second repository/index; do not introduce a second full parsed repository as the bootstrap verifier.

This may use the allowed alternative, captured-versus-current hash/topology validation, for late-discovered inputs; validation must bracket its own reads with stamp checks. Metadata checks do **not** promise filesystem atomicity or detection of arbitrary same-stamp ABA edits. No new resolver semantics or cache-version change is authorized. If the input universe cannot be covered within these files, stop for a bounded ownership/design amendment rather than silently omitting manifest/resolver evidence.

**Policy:** use existing production dispatch (`transport.rs:710–823`). WarnOnly may serve the captured coherent snapshot with stale metadata. AutoFull/AutoIncremental must take the existing refresh path; clean success serves the replacement, continued divergence carries `raced_stale`, refresh error retains the resident snapshot and adds stale plus auto-refresh-failure metadata. Bootstrap failure without a usable snapshot remains build failure. Do not promise fail-closed behavior where today's auto policy deliberately serves stale with failure evidence. Explicit failed refresh preserves old generation/session/sticky evidence.

**Named RED obligations** (test-only barriers, no sleeps; assert result content/graph and `prism/index_freshness`, counts/paths, refresh metadata):

| Test | Trigger / expected result / negative control |
|---|---|
| `freshness_read_a_edit_b_stamp_b` | Pause after A is read, edit B before the old stamp point; A must never be reported clean. Same fixture must fail on base. |
| `freshness_before_capture_and_after_capture` | Edit before pre-read capture: B loads clean. Edit after capture/before read, after read/before stamp, and after stamp/before service: divergence detected; unchanged control stays clean. |
| `freshness_manifest_resolver_inputs` | Manifest contents, resolver config, absent target creation, target deletion, directory/symlink replacement; each relevant change is stale, synthetic keys are not stat-ed as paths. Include unreadable/incomplete evidence. |
| `freshness_warn_auto_policy_metadata` | Above barriers under WarnOnly, AutoFull and AutoIncremental; clean refresh, repeated race, refresh failure/rollback, and unchanged control. Existing error-result metadata rules remain unchanged. |

Mutants: `F-postread-stamp-clean` restores late baseline capture; `F-drop-capture-mismatch` discards sticky capture divergence; `F-ignore-manifest-resolver` removes those inputs; `F-skip-service-recheck` bypasses the final probe. Each maps to its discriminating barrier assertion, not merely a stale-source assertion. **Estimate: 500–750 weighted additions**, split across capture and policy tests. Dependency: merged I0a/S0 base; merges before I0b's reload-race test. Review cap: two rounds.

## I0b — daemon lifecycle, leases and wire foundation

**Goal:** unload a whole idle provider while silent clients stay connected; preserve readiness, freshness and response integrity across concurrent calls, refresh, disposal and shutdown. Reuse `SessionProvider::bootstrap` and I0a's compact `Published`; no retained source/index Arc shadow.

**Owned files (exact; brace lists enumerate files, not directory-wide permission):**

| Existing anchors / NEW paths | Ownership and size forecast |
|---|---|
| `src/mcp/daemon/control.rs:10,32,69,179`; `coordinator.rs:101,127,210,331,422,474` | State, lease-bearing readiness, publication/history; keep coordinator near 600 (currently 468.5), move new idle responsibility to child files. |
| `src/mcp/daemon/connection.rs:117,135`; `budget.rs:24,72,103`; `admission.rs:144,180,253` | Request admission/release, attempt history/disposal budget, retirement exclusion. Preserve refresh locks and fatal policy. |
| `src/mcp/daemon/server.rs:242,263,298,524,570`; `run.rs:24,66`; `mod.rs:8`; `log.rs:40` | Timer/waker, options, effective wire snapshots, bounded lifecycle events; server currently 609, keep <1,000. |
| `src/bin/prism-mcp.rs:64,152,163` | Daemon subcommand accepts validated explicit idle setting/hidden relief arm. Shared branch :294–311 stays unchanged until I0c. |
| `src/mcp/shared/preamble.rs:158,328,355,383`; `control.rs:29,142,165,189`; `mod.rs:21` | `unloaded` build word and effective setting in both replies; versioned canonical encoding/decoding. |
| `src/mcp/shared/preamble_tests.rs:15`; `preamble_reply_tests.rs:8,14`; `control_tests.rs:19,48`; `identity_tests.rs:589` | Mechanical protocol fixture/version updates only; new behavioral cases go in new files. |
| `src/mcp/daemon/server_tests.rs:43,79`; `server_control_tests.rs:61`; `test_support.rs:190`; `src/mcp/shim/shim_support.rs:69`; `control_support.rs:43`; `control_tests.rs:112` | Mechanical struct/version fixture updates only, no unrelated test rewrites. |
| **NEW** `src/mcp/daemon/{idle,activity,idle_tests,idle_connection_tests,idle_reload_tests,idle_server_tests}.rs` | Lifecycle/lease modules ≤300 each; test modules ≤400 each. Register next to production owners; no monolithic daemon test file. |
| **NEW** `src/mcp/shared/idle_wire_tests.rs`; `mutants/manual/i0b.json` | Wire/compatibility witnesses and manual MCP registry. |

**Must not touch:** F-owned files, `session_idle.rs`, `idle_runtime.rs`, `idle_stdio.rs`, ordinary transport/framing, `transport_tests.rs`, shared identity hashing, cache/nav/CPG, shim production, memreport, D13 flight machinery. `prism-mcp.rs` transfers to I0c only after I0b merges. Enumerated shim fixtures are the sole cross-concern exception needed for new reply fields.

**Admission and lifetime:** admit each validated tool call before readiness; track pending/running work independently of connection counts. Under the **same control critical section that answers Ready**, acquire/transfer the serving RAII lease. No bare Ready escape, including completed-attempt and unheard-failure shortcuts. Lease covers readiness → slot access/auto-refresh lock transition → complete owned response/error construction; release and sample the clock **before socket write**, including error/unwind cleanup. Existing fatal-on-provider/control-panic semantics remain. Handshake/list/ping do not reset tool idle; executing work pins, connected silence does not.

**Timer/disposal:** idle origin is **max(last lease release, publication)** (parent Round-2 N3), never server startup or a stale pre-wait timestamp. Completion of a silent Background/warming build establishes publication time without another client call. Server tick uses slot **`try_write`**, then control, in existing slot→control order (`coordinator.rs:12`); on contention **re-arm a bounded future tick**, do not block/spin or lose the deadline. Recheck Ready, N elapsed, zero leases/waiters/pending tool work, no build/refresh/disposal/draining; detach and set Unloaded atomically.

Drop the detached provider **off both locks and off the server loop**, on a short-lived disposal worker; retain disposal accounting until completion. Arrange worker startup so a spawn failure cannot drop the provider inline on the server loop; leave/recover the Ready provider and re-arm on startup failure. Reload waits for disposal with bounded per-connection budgets; no old/new overlap. A timed-out disposal call cannot spend a later call's budget; a same-call disposal→build transition cannot grant a second full wait. Completion wakes the server; shutdown/retirement rejects obsolete publication and disposes late results exactly once.

**History/reload:** `Unloaded { last_published }` is distinct from initial Idle, preserving compact fingerprint/G/divergence plus the publication attempt watermark needed to supersede old outcomes. A caller retaining failure k after success k+1→unload starts/joins reload, never receives failure k. Historical success also cannot return Ready over an empty slot (`control.rs:32` currently does). One concurrent reload across old/new clients; failure is heard then retried according to existing connection-local history. Keep configured first-call wait (20 s default), zero/immediate warming and cumulative per-attempt deadlines; no three-second wait cap.

Carry G for equal fingerprints; increment G for changed fingerprints. **Prior sticky divergence with the same fingerprint keeps G**, matching `session_idle.rs:18–23`; bootstrap clears sticky diagnostics and logs a changed-snapshot transition, compared to a fresh reference with matched history. Touch-only need not change G. Explicit unloaded `refresh_index` still bootstraps then FullOnly refreshes: record both builds, do not optimize them here.

**Config/wire split (N5/N6):** I0b accepts explicit daemon options but absent settings remain **0/off**; do not independently consume inherited idle env before I0c forwarding. Timeout is first-spawner policy outside `DaemonIdentity`, never a new key component. I0b owns `build="unloaded"`, `index_idle_secs` (effective, zero disabled) and `index_idle_pressure_relief` (effective bool) in READY/STATUS. Keep existing `idle_secs`/`idle_ttl_secs` meaning clientless retirement; generation is 0 while no provider is resident, retained G stays private until reload.

Strict existing decoders reject added fields. Bump private `SHARED_PROTOCOL_VERSION` and `CONTROL_VERSION` for this extension; update canonical fixtures and unknown-version rejection together. Protocol version already belongs to identity; **different timeouts within this version share a key**. Old/new binaries do not silently share an incompatible wire. Old control peers report incompatible/unavailable, not idle=0; never kill/restart an old daemon or replay MCP traffic to compensate. I0c owns operator explanation and mismatch diagnostics, not another wire redesign.

I0b supports hidden macOS A+C via existing `idle::pressure_relief` **after disposal**; A is matched no-op, relief off by default and ineffective off macOS. Log identity/pid, event, reason, observed idle interval, configured timeout, duration, attempt/G, outcome and released bytes through capped/sanitized daemon logging; initial build differs from reload; explicit/auto-refresh success/failure included. No added MCP tool response fields.

**Named RED obligations:**

| New test | Required production seam / negative case |
|---|---|
| `daemon_idle_never_interrupts_request` | Real `serve_connection`/`run_tool_call`, barriers Ready→dispatch and dispatch→owned response; advance N, run real unload tick. Busy client among silent clients; independent provider can unload. Release+N lower bound. |
| `daemon_idle_blocked_writer_releases` | Block transport write after owned response; zero leases and eventual unload while socket remains blocked. Warming/failure/error/unwind release paths. |
| `daemon_idle_rearm_and_dispose` | Hold slot reader/writer at deadline; server/control remain responsive, bounded re-arm then unload; destructor barrier observes neither lock nor server-loop execution. Disposal-spawn failure retains usable Ready. |
| `daemon_idle_refresh_shutdown_races` | Query, explicit refresh, auto-refresh gap and build >N; queued calls pin. Shutdown vs publication/disposal yields no resurrection/double-drop; preserve resident refresh rollback. |
| `daemon_idle_releases_whole_session` | Weak repo/index/initialized and uninitialized edge witnesses; silent open clients, exact deadline, disabled setting, handshake-only unbuilt. Background completion without calls and warming-client silence unload at publication+N. |
| `concurrent_idle_reload_single_build` | Old/new clients, independent IDs/deadlines, zero wait, timeout, disconnect, failure→retry; stale failure and historical success across unload; disposal budgets in both directions. `idle_warming_retry_success`: bounded warming→retry→served. |
| `daemon_idle_reload_rechecks_freshness` | F merged: read-A/edit-B/stamp-B and after-stamp barriers assert metadata/source/graph under all policies; edit/add/delete/touch/manifest/resolver, absent/corrupt cache and recovery. Same/changed fingerprint and prior divergence generation. |
| `daemon_idle_wire_policy` | READY/STATUS unloaded vs initial idle, zero effective default, explicit values/relief; canonical round trip, overflow/malformed values and old-version rejection; timeout-independent identity. |

Registry `mutants/manual/i0b.json`: **`I0-unload-mid-request` daemon witness** removes production serving admission (not only a test-created lease); separate `I0b-release-before-response` targets the second barrier and `I0b-release-after-write` targets blocked output. Also `I0b-never-unloads`, `I0b-uncollected-build-pinned`, `I0b-no-rearm`, `I0b-drop-on-loop`, `I0b-clock-ignores-publication`, `I0b-history-ready-empty`, `I0b-old-failure-resurfaces`, `I0b-reload-skips-freshness`, `I0b-default-on`, `I0b-wire-lies`. Record distinct edits/selectors; supplemental selectors are not independent kills. **Estimate: 600–850 production + 700–950 tests/fixtures**, inventory before dispatch; if it exceeds this bounded lifecycle concern, split at design before implementation. Review cap: two rounds; depends on F.

## I0c — shim forwarding, operator and measurement surface

**Goal:** make explicit settings reach a spawned daemon, report the daemon's actual policy, and collect sufficient evidence for an honest A/A+C readout. No default activation.

**Owned files (exact):**

| Existing anchors / NEW paths | Ownership and size forecast |
|---|---|
| `src/bin/prism-mcp.rs:294–332`; `src/mcp/idle.rs:33,40` | Shared flag/env resolution, remove unused flag note only when forwarding works; applicable relief diagnostics. Sequential ownership after I0b. |
| `src/mcp/shim/run.rs:25,135,156`; `spawn.rs:22,39,66`; `status.rs:350`; `mod.rs:12` | Options→spawn arguments, mismatch reporting, human STATUS display and test registration. |
| `src/mcp/shim/spawn_tests.rs:9`; `status_tests.rs:1` | Small fixture updates only. |
| **NEW** `src/mcp/shim/idle_tests.rs`; `src/bin/prism-mcp/idle_tests.rs` | Forwarding/CLI/operator tests, ≤300 each; CLI registers adjacent to existing tests at `prism-mcp.rs:339` (nested path avoids Cargo auto-discovering a binary). |
| `tests/mcp/main.rs:1`; **NEW** `tests/mcp/idle_process_test.rs` | Module registration and ≤250-line host process witness; no changes to existing process test files. |
| `scripts/memreport/memreport.py:66,429`; `README.md:1`; `docs/MCP.md:238,342` | Bounded idle subcommand hook and operator/batch documentation. Existing eager command remains eager. |
| **NEW** `scripts/memreport/{memreport_idle,memreport_idle_readout,test_memreport_idle,test_memreport_idle_readout}.py` | Idle lifecycle collector, scoring/readout, fixture tests; each ≤300. |
| **NEW** `mutants/manual/i0c.json` | MCP feature mutants plus explicit Python selectors for collector mutations. |

**Must not touch:** daemon lifecycle/wire/identity (consume I0b interfaces), F files, stdio runtime, cache/nav/CPG, `transport_tests.rs`, existing memreport scoring/summary modules, pinned workload contents, soak configuration, shell profiles or launchers. No new attribution framework, bridge pre-warm implementation, reconnect supervisor or process-exit arm B.

Resolve **flag > `PRISM_MCP_INDEX_IDLE_SECS` > off** in the shim; validate invalid/overflow inputs, explicit zero overrides ambient nonzero/bad values. Pass the resolved value explicitly to a child daemon, including zero, so inherited env cannot override it; forward hidden relief intent for A+C. Direct daemon launch retains I0b's explicit-options contract. A running daemon keeps first-spawner settings. A differing shim emits **exactly one mismatch line** containing pid/effective/requested idle seconds (include relief mismatch on that line); identical settings emit none. Remove `prism-mcp.rs:300–302`'s I0a unused flag note. Preserve normal startup diagnostics, sanitized stderr and byte-transparent MCP pump.

Human STATUS/READY startup diagnostics distinguish index unloaded/building/ready/failed from connected/clientless state, and show effective idle seconds/relief. Status/shutdown lookup needs no idle setting and cannot reset tool-idle. For old control versions display incompatibility and the matching-binary requirement; do not invent settings, respawn, kill or replay.

Collector uses normal Lazy/Background stdio or shared sessions, **not** `PRISM_MCP_MEM_REPORT=1`'s Eager-only path (`memreport.py:66`). Reuse C1 provenance/admission conventions and supplied process samples; do not pretend the eager driver already measures idle unload. Keep sampling separate from tool traffic, which would reset the timer. Record stdio stderr **and daemon key-log**, with file identity/offset and daemon instance; deduplicate events and flag caps/truncation/missing streams as incomplete, not zero activity.

Required row fields: binary/commit/features, corpus pin/workload hash/cache classification, mode, A/A+C and no-op control, requested/effective idle/relief, pid/start/key and client count, first-call wait, cycle/attempt/G, publication/release/unload/disposal/reload timestamps, pre-index/loaded/+5/+30 sample times, footprint/RSS/heap/compression and footprint levels, relief-return bytes/live-heap delta, source-parse/ready/first-served durations, served-within-wait/warming/failure outcome and failure text, polling/pre-warm sequence, visibility/admission/exclusion reason. Missing samples stay UNKNOWN; preserve all outcomes/failed runs and raw logs outside the checkout. No synthetic pass from process exit or Weak-drop evidence.

**Named tests:** `shim_idle_flag_env_forwarding` (argv through production command construction, default/zero/precedence/invalid/overflow); `shim_idle_first_spawner_mismatch_once` (two requests, one identity, actual READY wins, no duplicate note); `operator_idle_unloaded_status` (effective zero/nonzero, clientless distinction, old-version unavailable, no idle reset); `idle_collector_visibility` (both sinks/dedup/truncation/missing/instance change); `idle_readout_denominators_and_outcomes` (exact-hit/miss distributions, failed/missing rows, units, two-corpus M, range separation, 128 MiB boundary and 15% report-only). `shared_idle_spawn_reload_process` covers real spawn→READY→unload→reload→served plus a second differing shim; run later on the controller host and retain its transcript as a collector fixture.

Mutants: `I0c-drop-spawn-setting`, `I0c-env-beats-zero`, `I0c-suppress-mismatch`, `I0c-report-requested-as-effective`, `I0c-ignore-daemon-log`, `I0c-drop-failures`, `I0c-use-heap-denominator`, `I0c-gate-on-15pct-idle`. Each requires a behavioral RED and restored GREEN. **Estimate: 300–450 production/docs + 400–600 collector/tests**, ≤600 per new file. Depends on I0b/F; review cap: two rounds.

## Verification contract for all three future PRs

Controller schedules builds/index-loading tests away from soak. Before implementation claims: full project default suite **and MCP-feature library/integration/binary suite**, relevant Python collector suite for I0c, required lint/format/shared-catalog gates. Report passed/failed/ignored/filtered totals and exact exclusions; socket/host checks run on controller host if unavailable. Do not attribute unrelated failures without a same-environment base control or silently fix/rebaseline them.

Every changed behavior needs a test that fails on pre-change behavior, plus negative/edge coverage per path. Carry behavior-only test/hook backports onto the slice parent for same-environment RED, preserving executable production behavior; compilation failure, zero selected tests and environment refusal are inadmissible. Existing-green preservation cases must be identified separately. F's historical base is `1d01b0a7`; I0b/I0c also record their actual merged prerequisite parent.

MCP mutants live in **`mutants/manual/<slice>.json` and are hand-run with `--features mcp`**: the shared runner builds without features and cannot prove these kills. Record exact head, selector/count, mutation anchor, before/mutant/restored hashes, behavioral RED and restored GREEN; run Python collector mutants with their named Python tests. Keep authoritative shared-catalog verification separately required; do not add MCP mutants to it as an apparent substitute. Static `python3 scripts/file_size.py OWNED_FILES...` before/after is required for each slice. None of these executable gates ran tonight.

## Post-soak readout prerequisites and unsettled items

1. Confirm S6 soak completion (expected 2026-10-20 ~12:08 MDT is inherited, not permission by date); C0 selects/pins Py and freezes E/P/ruff/Py workloads. Require ≥50 GiB free on all relevant volumes, no competing index load, non-increasing swap, sequential corpus runs and the parent's reserved measurement window/admission rules. All three slices and their verification must land before complete both-mode acceptance.
2. Run earliest admitted vertical A/A+C check before the full matrix; same-batch matched **A vs A+C**, with resident base control and A no-op. Every enabled measurement passes **`--index-idle-secs 600`**. Sample genuine pre-index, loaded, +5/+30 seconds after disposal, repeated cycles; report all surviving shim/daemon memory per checkout. A+C macOS relief follows drop; record returned bytes, actual footprint effect and live heap unchanged within ±1 MiB.
3. I0 M: median matched **current idle physical footprint** savings **≥200 MiB OR ≥15% on ≥2 corpora**, ≥3 admitted runs/side, candidate max < base min; denominator is resident base at the same elapsed idle boundary. Require **+30 s idle footprint ≤128 MiB per checkout on all four corpora in both modes**; **15% of base idle is reported, not an additional idle bar**. RSS/heap/compression/footprint levels and +16 MiB above pre-index footprint/RSS are report-only diagnostics. Inventory pool stacks/query cache/identity/allocator residuals without unmeasured attribution.
4. Exact-hit reload median ≤ same-binary/settings/cache fresh-lazy reference +max(5%,100 ms); **time-to-ready ≤3 s target**, explicit misses. Measure source parse, ready and first-served separately; configured wait remains 20 s default. Preregister per-corpus **served-within-wait / warming / failure counts and proportions**, separately for exact hits and changed-snapshot misses; include warming→poll→success and abandonment. No survivor-only latency distribution. Pre-warm must fall within N of use.
5. Require same-snapshot bytes/G parity; changed-content/stamp/prior-divergence fresh-reference parity with matched history; unchanged rollback and single concurrent build. Then host-capacity evidence names checkout mix, active/unloaded counts, simultaneous reload peaks, headroom, serialized-heavy-refresh assumption and swap. I0 alone cannot certify 12–16 active indexes. Design process-exit B only if **both** A and A+C miss the physical bar; current shim cannot reconnect safely.

- **WRONG / IMMATERIAL (pre-existing), required prerequisite:** read-A/edit-B/stamp-B false freshness above; F owns bounded repair and RED control. No regression attribution to I0a.
- **SMELL / MATERIAL, evidence gate:** physical reclaim, latency/outcome distributions and 12–16-checkout capacity remain unmeasured; acceptance/default activation waits for readout, not a source-review blocker.
- **SMELL / MATERIAL, dispatch gate:** file reservations, post-rebase input-universe inventory and controller verification scheduling must be explicit; no authority for overlapping writers or quiet-night execution is implied by this plan.
- **SMELL / IMMATERIAL, retained limitation:** metadata freshness is not an atomic filesystem snapshot; old-version operator control requires a matching binary; stdio blocked stdout may stall its owner loop. None is a newly demonstrated incorrect result or a reason to widen these slices.

No unresolved owner choice blocks design dispatch. Controller owns custody of this new plan and the dated handoff line; implementation/test/measurement claims remain pending.
