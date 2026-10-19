# I0 freshness prerequisite, I0b daemon, I0c shim/operator

Design written 2026-10-18; dispatch/fold dated 2026-10-19. Checkout `prism-i0b-plan`, branch `plan/i0b-freshness`; original design base **`1d01b0a72daa2297d90529676f542ebcd4ada401`**, round-1 fold starts clean at committed plan `3748fc39`. Existing-file anchors refer to the original base; **NEW** paths have no base line. Controller commits. No cargo, builds, tests, index loads or measurements ran for this plan; weighted-size figures are inherited static counts.

Goal: a2a-bridge fan-out on a **24 GiB host without swap exhaustion**, retaining the **<1 GiB per cache-hit index / 12–16 checkout** goal. I0 returns idle sessions' memory; it does not establish the active-index or host-capacity target by itself. I0a shipped stdio-only in #491 (`6abab1e5`); S0 #487 is merged. **Absent flag/env means OFF**, including after I0c. Default activation remains an owner decision at the measured readout. Measurement commands explicitly pass **`--index-idle-secs 600`**.

## Authority, ordering and dispatch gates

- Binding parent: [optimization round 2](2026-10-18-optimization-round2.md), I0 row at :64, I0 design/acceptance table at :82–114, Round-1 fold and Round-2 post-cap fold at :116–132, M exception at :168 and I0 acceptance at :170. This plan refines those contracts; it does not relax them.
- Binding I0a evidence: `~/prism-evidence/mcp-memory/optimization-round2/idle-unload/i0a/review-r1-opus.md`, `review-r1-sol.md`, `review-r2-opus.md`, `review-r2-sol.md`. Round-2 resolutions govern earlier default/lease findings. Their execution results are supplied receipts, not executions here.
- Read implementation basis: `src/mcp/idle.rs:19,90,115`, `idle_runtime.rs:132,174,226,309`, `idle_stdio.rs:109,150`, `session_idle.rs:7,18`, `mod.rs:44,61`, `src/bin/prism-mcp.rs:294,314`. Preserve I0a's fixed disposal budget, post-wait elapsed clock, enabled-only routing and generation choice.
- Merge order: **F → I0b-1 → I0b-2 → I0c → admitted measurement → owner activation decision**. I0b-1 enables unloading only in tests; I0b-2 exposes explicit daemon options, default off. Readout additionally requires controller-owned serving observation O below; I0c consumes its schema. S0/T3a remain independent; D13 is not pulled into I0.
- File ownership is exclusive per slice below. Sequential handoff of `prism-mcp.rs` and shared test fixtures is explicit; no concurrent editing of those files. Controller reserves session hooks against tree work and daemon hooks against D13 before implementation dispatch. Everything not enumerated as owned is read-only.
- **Review cap: two rounds per PR**, declared before dispatch. One drafting pass plus one static verification pass for this document; no review agents dispatched. At cap: targeted fixes for closed, converging findings, disclose any extension; recurring open-class findings stop for design correction. No restart to evade review.
- Estimates below are guessed weighted additions including tests, not quotas or measured effort. Aim ≤600 per new file; split responsibilities above 1,000, never cross 2,000. Oversized files must not grow. `transport_tests.rs` is **2,303.5**, forbidden; current daemon tests are all <1,000 (largest `connection_tests.rs` 508.5). Recheck on dispatch/rebase; any daemon test >1,000 is frozen, with new coverage in new files.

## Round-1 review fold

Both `i0b-plan-review-r1-opus.md` and `i0b-plan-review-r1-sol.md` under `~/prism-evidence/mcp-memory/optimization-round2/idle-unload/` were read in full. Controller dispositions govern; these are design corrections, not executed verification.

| Review item | Disposition folded |
|---|---|
| Opus F W1 — MATERIAL WRONG | Bracketed validation against loaded hashes; equal content is clean despite stamp changes; refresh `verify_snapshot` unchanged; corrected tests and comparison mutant. |
| Opus F W2 — MATERIAL WRONG | Stable unreadable/missing/budget states compare as data; narrowly defined incompleteness; unchanged uncertifiable evidence cannot cause repeated auto refresh. |
| Opus S1 + sol S1 — MATERIAL | Loader-owned post-load validation, one visibility-only accessor, settled universe/neutrality table, unchanged service set and bootstrap-cost readout. |
| Opus S3 + sol S2 — MATERIAL | Sequential I0b-1 lifecycle/test enablement and I0b-2 wire/CLI/logging/A+C, separate ownership/tests/mutants/estimates. |
| Opus S7 + sol S3 — MATERIAL | Producer/owner table and explicit O prerequisite; fresh isolated arms, effective-setting exclusion, one binary per batch, both-mode real fixtures. |
| Opus S8 — MATERIAL | No gap-distribution source supplied: N stays provisional at 600, including at activation decision. |
| Opus S2/S4 — IMMATERIAL | Own `owner.rs:69`, bootstrap evidence in `known_stale_after_refresh`; missing daemon fixtures/spec and I0c relief assertion reserved; concrete reload mutant. |
| Opus S5/S6 — IMMATERIAL | Disposal panic is fatal, WouldBlock re-arms/Poisoned is fatal; retain disposal-timeout warming-without-build semantics and measure retry latency. |
| Opus S9 — IMMATERIAL | Wire field `index_idle_timeout_secs`; handoff records committed plan `3748fc39`, pending fold commit. |

## F — source-bound freshness (separate prerequisite PR)

**Goal / existing defect.** **WRONG / IMMATERIAL in the inherited review; mandatory I0 dependency.** `load_repo` reads A at `src/repo_loader.rs:1094–1132`; bootstrap stamps later at `src/mcp/session.rs:410–413` via `freshness.rs:53–87`. Editing to B in that interval makes the probe accept B's stamp while the index contains A. This predates I0; I0 increases exposure, not the defect's origin. Source evidence establishes the mechanism; same-environment executable base RED remains required.

**Owned files (exact):**

| Existing anchor / NEW path | Ownership and size forecast |
|---|---|
| `src/mcp/session.rs:174,204,285,406,432`; `src/mcp/owner.rs:69` | Bootstrap evidence transfer, effective query policy/refresh clearing, test registration; owner constructor compatibility only, no owner-serving change. Session currently 840.5; ≤60 added. Candidate verification remains unchanged. |
| `src/mcp/freshness.rs:24,53,76,132` | Probe consumes source-bound evidence and preserves mismatch evidence; module declarations. Current 605.5; ≤35 added, new logic in child module. |
| `src/repo_loader.rs:452` | Sole loader edit: expose existing `collect_supported_source_paths` as `pub(crate)` read-only accessor; no new walk, filtering, parsing or loader semantics. |
| **NEW** `src/mcp/freshness_capture.rs` | Capture/validation responsibility, ≤250. |
| **NEW** `src/mcp/freshness_capture_tests.rs` | Deterministic capture barriers and input-universe cases, ≤250. |
| **NEW** `src/mcp/freshness_policy_tests.rs` | Production bootstrap/dispatch policy witnesses, ≤220. |
| **NEW** `mutants/manual/i0-freshness.json` | Feature-aware manual registry, following `idle-unload-i0a.json:1`. |

**Must not touch:** daemon/shim/shared, idle runtime/policy/stdio, `transport.rs`/`transport_tests.rs`, loader beyond that accessor, manifest/resolver implementation, navigation/CPG/cache formats, harness or unrelated S0 code. Read loader contracts at `repo_loader.rs:36–44,64–77,188–305`; typed manifest evidence at `manifest_snapshot.rs:7–49`; resolver topology at `js_paths_snapshot.rs:222–295`.

**Chosen contract: validate after load against the loader's own inputs.** Before index construction, re-hash `LoadedRepo.file_hashes` paths and replay public `scope_graph_build_inputs(root, &loaded.files)` using the retained parsed files; compare typed manifests, topology and coverage with loaded inputs. No reparsing, second parsed repository/index, copied walker/resolver rules, new resolver semantics or cache-version change. The one accessor supplies source membership before load and again during validation; it is evidence of membership, not a promise that every candidate parsed successfully.

Bracket validation reads with two captures: stamp/hash/stamp for source bytes, and loader-owned input/membership captures around topology validation. A stamp mismatch between capture and load is **not** sticky by itself: equal stable current content and loaded hash is clean with the validated stamp baseline. Changed content, changed topology/membership, or a race during validation retains stale evidence; a later stable stamp alone cannot erase it. Directory mtime-only changes are clean when the loader-filtered membership/topology is equal. Final service recheck remains required. This is not an atomic filesystem snapshot or arbitrary same-stamp ABA detection.

| Input universe | Loaded evidence and post-load check | Service retention / reporting |
|---|---|---|
| Indexed source bytes; candidate membership | Loaded hashes plus the accessor's before/after sets; reuse its language/size/hidden/builtin/symlink rules. Loaded `skipped` records preserve Go-testdata/parse/unreadable exclusions; stable skipped candidates are not falsely treated as missing indexed files. | Existing source paths/ancestors only; no captured superset retained. |
| Typed manifests and refusals | `manifest_snapshot` bytes/hash or symlink refusal against public replay; absent→present/deletion checked by replay. | Existing manifest keys/ancestors; same unchanged `tracked_paths` and refresh summaries. |
| Resolver configs, package/type inputs, probed absence, links, case inventory, Rust coverage | Loaded `scope_graph_inputs` versus public replay, including JS ambient inputs inside indexing-skipped directories and external ancestor `node_modules` absence/opaque states. | Temporary validation only; synthetic topology keys are comparison/report labels, never filesystem paths. Preserve existing logical key counts without additional per-query stats. |
| Traversed membership and completeness/budget state | Accessor and replay own traversal and budgets; compare captured sets/topology/completeness at their existing granularity, never scan past a loader budget. | Do not retain unrelated paths or add query triggers for unsupported-file churn outside today's tracked set. |

Stable `Unreadable`, `Missing`, refusals and budget-stop states compare as data, as today. **Incomplete** means capture I/O failure on a path the loader actually used, or completeness/budget-boundary change between captures; stable loader-disabled P1 is not incomplete validation by itself. Preserve bounded stale evidence for unavailable consumed inputs. An unchanged uncertifiable reason may cause at most one auto-refresh attempt: record that reason against the snapshot/evidence, suppress repeat eligibility until evidence changes (explicit refresh remains available), including after failure or an equally uncertifiable replacement. Do not suppress distinct content changes. Preserve candidate `build_verified_candidate`/`verify_snapshot` content verification, retry and rollback; a Clean result clears replaced bootstrap evidence, and candidate stamp-only differences cannot override it.

Sticky bootstrap evidence lives in `SessionProvider.known_stale_after_refresh`, so `Published.diverged` already includes it; compact auto-attempt bookkeeping is separate from that reporting field. Per-query tracked cardinality/stat work stays at today's size; public paths/count caps stay unchanged, topology mismatches use bounded logical labels. Wider captures are bootstrap-only and dropped before serving. Record F's bootstrap wall-time delta on a large corpus at readout, including in reload latency; no measurement tonight.

**Policy:** use existing production dispatch (`transport.rs:710–823`). WarnOnly serves with stale metadata. AutoFull/AutoIncremental use existing refresh except for the unchanged uncertifiable reason already attempted: session-owned effective query policy returns WarnOnly for that case, without changing configured policy or explicit refresh; a new probe discrepancy restores configured auto eligibility. Clean success serves replacement, continued divergence carries `raced_stale`, refresh error retains resident state plus stale/auto-failure metadata. Bootstrap without a usable snapshot fails; explicit failed refresh preserves generation/session/sticky evidence. No transport edit or fail-closed promise.

**Named RED obligations** (test-only barriers, no sleeps; assert result content/graph and `prism/index_freshness`, counts/paths, refresh metadata):

| Test | Trigger / expected result / negative control |
|---|---|
| `freshness_read_a_edit_b_stamp_b` | Pause after A is read, edit B before the old stamp point; A must never be reported clean. Same fixture must fail on base. |
| `freshness_before_capture_and_after_capture` | Edit before load or after capture/before read with no further edit: B loads clean; touch-only clean. Read-A/edit-B/stamp-B and edit after validation stamp/before service: stale. Stable current hash must be compared to loaded hash, not itself. |
| `freshness_manifest_resolver_inputs` | Manifest/config edits, absent target creation/deletion, directory/symlink replacement; resolver input inside skipped indexing directory and external ancestor absence→presence. Relevant loaded/current mismatch stale; synthetic keys never stat-ed. Stable unreadable/missing/budget-stop clean; consumed-path capture failure and changed completeness retain evidence. |
| `freshness_warn_auto_policy_metadata` | All policies: clean candidate verification, repeated race, failure/rollback; repeated queries with identical uncertifiable evidence do not refresh-loop, changed evidence re-arms. Unrelated-file churn and unchanged incomplete resolver state preserve exact summaries/counts. Separate existing-green neutrality cases from base RED. |

Mutants: `F-postread-stamp-clean` restores late baseline capture; `F-drop-capture-mismatch` discards verified divergence; `F-validate-against-current` compares current hashes with themselves instead of loaded hashes; `F-ignore-manifest-resolver`; `F-skip-service-recheck`; `F-repeat-uncertifiable-refresh` removes same-reason suppression. Each maps to a discriminating assertion. **Estimate: 550–800 weighted additions**, split across capture and policy tests. Dependency: merged I0a/S0 base; merges before I0b-1's reload-race test. Review cap: two rounds.

## I0b-1 — daemon lifecycle, leases and test-only enablement

**Goal:** unload a whole idle provider while silent clients stay connected; preserve readiness, freshness and response integrity across concurrent calls, refresh, disposal and shutdown. Reuse `SessionProvider::bootstrap` and I0a's compact `Published`; no retained source/index Arc shadow.

**Owned files (exact; brace lists enumerate files, not directory-wide permission):**

| Existing anchors / NEW paths | Ownership and size forecast |
|---|---|
| `src/mcp/daemon/control.rs:10,32,69,179`; `coordinator.rs:101,127,210,331,422,474` | State, lease-bearing readiness, publication/history; keep coordinator near 600 (currently 468.5), move new idle responsibility to child files. |
| `src/mcp/daemon/connection.rs:117,135`; `budget.rs:24,72,103`; `admission.rs:144,180,253` | Request admission/release, attempt history/disposal budget, retirement exclusion. Preserve refresh locks and fatal policy. |
| `src/mcp/daemon/server.rs:13,242,263,298`; `mod.rs:8`; `fatal.rs:18` | Timer/waker and fatal disposal domain; update no-slot-on-loop documentation to permit nonblocking tick. Server currently 609, keep <1,000. |
| `src/mcp/daemon/control_tests.rs:20`; `coordinator_order_tests.rs:434`; `test_support.rs:190`; `server_support.rs:35` | Exact lifecycle API/Parts fixture adaptations, obsolete permanent-Ready assertion update and test-only enablement; no wire changes. |
| **NEW** `src/mcp/daemon/{idle,activity,idle_tests,idle_connection_tests,idle_reload_tests,idle_server_tests}.rs` | Lifecycle/lease modules ≤300 each; test modules ≤400 each. Register next to production owners; no monolithic daemon test file. |
| **NEW** `mutants/manual/i0b-1.json` | Lifecycle MCP registry. |

**Must not touch:** F files, `session_idle.rs`, stdio idle runtime, transport/framing/tests, shared wire/identity, CLI, shim, memreport, cache/nav/CPG, D13 machinery. Only test construction through `Parts` can enable unload; production construction is off with no CLI/env exposure. `Unloaded` temporarily maps to existing wire `idle`, unreachable in production. I0b-2 receives the enumerated shared lifecycle files sequentially; new test files remain disjoint.

**Admission and lifetime:** admit each validated tool call before readiness; track pending/running work independently of connection counts. Under the **same control critical section that answers Ready**, acquire/transfer the serving RAII lease. No bare Ready escape, including completed-attempt and unheard-failure shortcuts. Lease covers readiness → slot access/auto-refresh lock transition → complete owned response/error construction; release and sample the clock **before socket write**, including error/unwind cleanup. Existing fatal-on-provider/control-panic semantics remain. Handshake/list/ping do not reset tool idle; executing work pins, connected silence does not.

**Timer/disposal:** idle origin is **max(last lease release, publication)** (parent Round-2 N3), never startup or a stale pre-wait timestamp. Silent Background/warming completion sets publication time without another call. Tick uses slot **`try_write`**, then control, in slot→control order: **WouldBlock** re-arms a bounded future tick; **Poisoned** takes the fatal path, never re-arms forever. Recheck Ready, N elapsed, zero leases/waiters/pending work, no build/refresh/disposal/draining; detach and set Unloaded atomically.

Drop the detached provider **off both locks and off the server loop**, on a short-lived disposal worker; retain accounting until completion. Spawn failure leaves/recovers Ready and re-arms, never drops inline. Worker completion guard reports destructor panic as fatal `Domain::Disposal` and wakes the server; it cannot strand disposal in progress or authorize reload after panic. Reload waits with bounded per-connection budgets and no old/new overlap; timeout cannot consume a later call's budget, nor can same-call disposal→build grant a second full wait. Retain I0a's warming-without-started-build on disposal timeout; retry starts/joins reload and readout includes that delay. Shutdown rejects obsolete publication and disposes late results exactly once.

**History/reload:** `Unloaded { last_published }` is distinct from initial Idle, preserving compact fingerprint/G/divergence plus the publication attempt watermark needed to supersede old outcomes. A caller retaining failure k after success k+1→unload starts/joins reload, never receives failure k. Historical success also cannot return Ready over an empty slot (`control.rs:32` currently does). One concurrent reload across old/new clients; failure is heard then retried according to existing connection-local history. Keep configured first-call wait (20 s default), zero/immediate warming and cumulative per-attempt deadlines; no three-second wait cap.

Carry G for equal fingerprints; increment G for changed fingerprints. **Prior sticky divergence with the same fingerprint keeps G**, matching `session_idle.rs:18–23`; bootstrap replaces prior diagnostics with freshly validated evidence (clean clears them; a new race stays stale) and logs a changed-snapshot transition, compared to a fresh reference with matched history. Touch-only need not change G. Explicit unloaded `refresh_index` still bootstraps then FullOnly refreshes: record both builds, do not optimize them here.

**Named RED obligations:**

| New test | Required production seam / negative case |
|---|---|
| `daemon_idle_never_interrupts_request` | Real `serve_admitted`/`serve_bounded` → `serve_client` → `run_tool_call`; Ready→dispatch and dispatch→owned-response barriers; advance N, real unload tick. Busy among silent clients; independent provider unloads. Pending guard transfers to lease without masking its mutant; release+N lower bound. |
| `daemon_idle_blocked_writer_releases` | Block transport write after owned response; zero leases and eventual unload while socket remains blocked. Warming/failure/error/unwind release paths. |
| `daemon_idle_rearm_and_dispose` | Hold reader/writer at deadline: responsive loop, bounded re-arm then unload; destructor sees neither lock nor loop thread. Spawn failure preserves Ready; injected destructor panic wakes fatal path, poisoned slot is fatal rather than contention. |
| `daemon_idle_refresh_shutdown_races` | Query, explicit refresh, auto-refresh gap and build >N; queued calls pin. Shutdown vs publication/disposal yields no resurrection/double-drop; preserve resident refresh rollback. |
| `daemon_idle_releases_whole_session` | Weak repo/index/initialized and uninitialized edge witnesses; silent open clients, exact deadline, disabled setting, handshake-only unbuilt. Background completion without calls and warming-client silence unload at publication+N. |
| `concurrent_idle_reload_single_build` | Old/new clients, independent IDs/deadlines, zero wait, timeout, disconnect, failure→retry; stale failure and historical success across unload; disposal budgets in both directions. `idle_warming_retry_success`: bounded warming→retry→served. |
| `daemon_idle_reload_rechecks_freshness` | F merged: read-A/edit-B/stamp-B and after-stamp barriers assert metadata/source/graph under all policies; edit/add/delete/touch/manifest/resolver, absent/corrupt cache and recovery. Same/changed fingerprint and prior divergence generation. |

Registry `mutants/manual/i0b-1.json`: **`I0-unload-mid-request`** removes production serving admission; `I0b-release-before-response` targets the second barrier and `I0b-release-after-write` blocked output. Also `I0b-never-unloads`, `I0b-uncollected-build-pinned`, `I0b-no-rearm`, `I0b-drop-on-loop`, `I0b-clock-ignores-publication`, `I0b-history-ready-empty`, `I0b-old-failure-resurfaces`, `I0b-disposal-panic-strands`, `I0b-poison-as-contention`. **`I0b-reload-skips-freshness`** edits owned coordinator `run_build`: retain a mutant-only shadow provider on detach and republish it instead of invoking bootstrap; reload source/graph/metadata witness must kill it. Distinct selectors/barriers, no supplemental-selector double counting. Production-off preservation test checks ordinary construction cannot unload. **Estimate: 450–550 production + 550–700 tests = 1,000–1,250**; inventory before dispatch, two-round cap; depends on F.

## I0b-2 — daemon wire, CLI, logging and A+C

**Goal:** expose the verified lifecycle through explicit options, default off; own the single wire extension and its design-spec update. Depends on I0b-1; two-round cap. **Estimate: 200–300 production/docs + 200–300 tests/fixtures = 400–600**.

| Owned existing anchors / NEW paths | Bounded responsibility |
|---|---|
| `src/mcp/daemon/{control,coordinator,server,run,mod,log}.rs` (anchors above; `run.rs:24,66`, `log.rs:40`); NEW `src/mcp/daemon/idle.rs` from I0b-1 | Sequential transfer for option wiring, effective state, events and post-drop relief only; no lease/history redesign. |
| `src/bin/prism-mcp.rs:64,152,163` | Validated daemon explicit idle/hidden relief options; shared :294–311 unchanged until I0c. |
| `src/mcp/shared/preamble.rs:158,328,355,383`; `control.rs:29,142,165,189`; `mod.rs:21` | Effective settings, unloaded word, canonical codecs and versions. |
| `src/mcp/shared/{preamble_tests,preamble_reply_tests,control_tests,identity_tests}.rs` (:15, :8/14, :19/48, :589) | Mechanical wire/version fixtures. |
| `src/mcp/daemon/{server_tests,server_control_tests,test_support,server_support}.rs` (:43/79, :61, :190, :35); `src/mcp/shim/{shim_support,control_support,control_tests}.rs` (:69, :43, :112) | Mechanical reply/options/version fixtures; explicit reservation of `ServeOptions` literal. |
| **NEW** `src/mcp/shared/idle_wire_tests.rs`; `src/mcp/daemon/idle_wire_logging_tests.rs`; `src/bin/prism-mcp/daemon_idle_tests.rs`; `mutants/manual/i0b-2.json` | New tests ≤250 each, registration in owned modules; no edits to I0b-1 test files. |
| `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` | Replace wire-v1/permanent-Ready description with versioned unloaded lifecycle/settings. |

**Must not touch:** F files, stdio runtime, transport/framing, identity hashing, cache/nav/CPG, shim production, memreport or D13. Shared daemon files transfer after I0b-1 merge; CLI/fixtures transfer to I0c after I0b-2.

**Config/wire:** absent options **0/off**, no inherited idle env consumption before shim forwarding. Timeout/relief are first-spawner policy outside identity. READY/STATUS add `build="unloaded"`, **`index_idle_timeout_secs`** (effective, zero disabled), `index_idle_pressure_relief` (effective bool); clientless `idle_secs`/`idle_ttl_secs` unchanged. Wire generation is 0 without a resident provider; retained G is private. Bump `SHARED_PROTOCOL_VERSION` and `CONTROL_VERSION`; canonical and unknown-version fixtures change together. Different timeouts share a key within a version. Old peers report incompatible/unavailable, never invented zero, forced restart or replay.

Hidden macOS A+C calls existing `idle::pressure_relief` **after disposal**, off by default/ineffective off macOS; A is matched no-op. Capped sanitized key-log records identity/pid, event/reason, observed idle/configured timeout, event timestamp/duration, attempt/G/outcome and returned bytes; distinguish initial build/reload and explicit/auto refresh success/failure. No new MCP tool response fields.

**Tests:** `daemon_idle_wire_policy` covers actual CLI→options→READY/STATUS, unloaded vs initial idle, default/zero/nonzero, invalid/overflow, relief, round trip/version rejection and timeout-independent identity. `daemon_idle_logging_and_relief` covers initial/reload/refresh failure events, post-drop-only relief, no-op/off/non-macOS, sanitization/caps. **Mutants:** `I0b-default-on`, `I0b-wire-lies`, `I0b-drop-daemon-option`, `I0b-relief-before-drop`, `I0b-omit-lifecycle-event`; each has its own behavioral assertion. Lifecycle preservation suite remains required on the I0b-1 parent.

## I0c — shim forwarding, operator and measurement surface

**Goal:** make explicit settings reach a spawned daemon, report the daemon's actual policy, and collect sufficient evidence for an honest A/A+C readout. No default activation.

**Owned files (exact):**

| Existing anchors / NEW paths | Ownership and size forecast |
|---|---|
| `src/bin/prism-mcp.rs:294–332`; `src/mcp/idle.rs:33,40`; `src/mcp/idle_fix_tests.rs:227` | Shared flag/env resolution, remove unused flag note only when forwarding works; shared relief applicability and its obsolete always-Some assertion. Sequential ownership after I0b-2; no stdio lifecycle change. |
| `src/mcp/shim/run.rs:25,135,156`; `spawn.rs:22,39,66`; `status.rs:350`; `mod.rs:12` | Options→spawn arguments, mismatch reporting, human STATUS display and test registration. |
| `src/mcp/shim/spawn_tests.rs:9`; `status_tests.rs:1` | Small fixture updates only. |
| **NEW** `src/mcp/shim/idle_tests.rs`; `src/bin/prism-mcp/idle_tests.rs` | Forwarding/CLI/operator tests, ≤300 each; CLI registers adjacent to existing tests at `prism-mcp.rs:339` (nested path avoids Cargo auto-discovering a binary). |
| `tests/mcp/main.rs:1`; **NEW** `tests/mcp/idle_process_test.rs` | Module registration and ≤250-line host process witness; no changes to existing process test files. |
| `scripts/memreport/memreport.py:66,429`; `scripts/memreport/README.md:1`; `docs/MCP.md:238,342` | Bounded idle subcommand hook and operator/batch documentation. Existing eager command remains eager. |
| **NEW** `scripts/memreport/{memreport_idle,memreport_idle_readout,test_memreport_idle,test_memreport_idle_readout}.py` | Idle lifecycle collector, scoring/readout, fixture tests; each ≤300. |
| **NEW** `mutants/manual/i0c.json` | MCP feature mutants plus explicit Python selectors for collector mutations. |

**Must not touch:** daemon lifecycle/wire/identity (consume I0b-2 interfaces), F files, stdio runtime (only the named relief-note assertion above), cache/nav/CPG, `transport_tests.rs`, existing memreport scoring/summary modules, pinned workload contents, soak configuration, shell profiles or launchers. No new attribution framework, bridge pre-warm implementation, reconnect supervisor or process-exit arm B. Serving observation O is a separate controller prerequisite, not an I0c scope expansion.

Resolve **flag > `PRISM_MCP_INDEX_IDLE_SECS` > off** in the shim; validate invalid/overflow inputs, explicit zero overrides ambient nonzero/bad values. Pass the resolved value explicitly to a child daemon, including zero, so inherited env cannot override it; forward hidden relief intent for A+C. Direct daemon launch retains I0b-2's explicit-options contract. A running daemon keeps first-spawner settings. A differing shim emits **exactly one mismatch line** containing pid/effective/requested idle seconds (include relief mismatch on that line); identical settings emit none. Remove `prism-mcp.rs:300–302`'s I0a unused flag note. Preserve normal startup diagnostics, sanitized stderr and byte-transparent MCP pump.

Human STATUS/READY startup diagnostics distinguish index unloaded/building/ready/failed from connected/clientless state, and show effective idle seconds/relief. Status/shutdown lookup needs no idle setting and cannot reset tool-idle. For old control versions display incompatibility and the matching-binary requirement; do not invent settings, respawn, kill or replay.

Collector uses normal Lazy/Background stdio or shared sessions, **not** `PRISM_MCP_MEM_REPORT=1`'s Eager-only path (`memreport.py:66`). Reuse C1 provenance/admission conventions. Keep sampling separate from tool traffic, which would reset idle. Record stdio stderr **and daemon key-log**, with file identity/offset and instance; deduplicate events, mark caps/truncation/missing streams incomplete, never zero activity.

Required row fields: binary/commit/features, corpus pin/workload hash/cache classification, mode, A/A+C and no-op control, requested/effective idle/relief, pid/start/key and client count, first-call wait, cycle/attempt/G, publication/release/unload/disposal/reload timestamps, pre-index/loaded/+5/+30 sample times, footprint/RSS/heap/compression and footprint levels, relief-return bytes/live-heap delta, source-parse/ready/first-served durations, served-within-wait/warming/failure outcome and failure text, polling/pre-warm sequence, visibility/admission/exclusion reason. Missing samples stay UNKNOWN; preserve all outcomes/failed runs and raw logs outside the checkout. No synthetic pass from process exit or Weak-drop evidence.

**Observation prerequisite O (controller-owned, not yet implemented):** separately dispatch an opt-in normal-serving observer, proposed `src/mcp/serving_observer.rs`, with serialized ownership of session/build/cache-result, stdio/daemon lifecycle and shim sampling hooks. I0c waits for its schema; complete readout waits for its implementation/neutrality evidence. Existing `mem_sample::sample` samples only its own process, and the eager `mem_report::Recorder` is not a serving producer. O must supply the following before measurement; this plan grants no observer implementation/file edits tonight.

| Observations | Named producer → owner / admission rule |
|---|---|
| Binary hash/commit/features, corpus/workload pins, mode/arm, requested settings/wait, polling/pre-warm, admission/exclusions | Preregistered batch manifest + `memreport_idle` launch/request journal → controller/C0 for pins/admission; I0c for collection/scoring. Preserve all failures and UNKNOWN rows. |
| Effective idle/relief, pid/start/key/client count | I0b-2 READY/STATUS and O startup-instance record (stdio and shim included) → I0b-2/O; I0c joins by instance, never reports requested as effective. |
| Cache exact-hit/miss/reason, source-load/parse duration, F validation/bootstrap duration | O build-stage spans and actual cache-result hook → controller O. `load_repo` span includes source loading/parsing; separate validation and index-ready spans, never infer cache outcome from speed. |
| Cycle/attempt/G, publication, last release, unload, disposal start/end, reload and refresh outcome/error, relief returned bytes | I0b-2 key-log; O adds the corresponding stdio events and any missing structured stage fields in both modes → I0b-2/O. Timestamp at event production, not collector read. |
| Genuine pre-index/loaded/+5/+30 footprint, RSS, compression, heap and sample times; before/after relief live-heap delta | O scheduled in-process `mem_sample::sample()` records in each surviving stdio/shim/daemon process → controller O; I0c sums checkout footprint and computes levels/deltas. No tool calls to sample; unavailable OS fields remain UNKNOWN. |
| First-call/ready/first-served durations, served-within-wait/warming/failure counts, text and abandonment | O producer ready/response-owned timestamps + I0c request/response-arrival journal → O/I0c. Keep producer completion distinct from client arrival and include failed/abandoned attempts. |
| Visibility, missing/capped/duplicate streams, wrong pid/restarted instance, denominators/range separation | O stream sequence/clock/instance headers + I0c collector/scorer → O/I0c. Missing required producer fields or wrong-instance samples exclude acceptance, not the raw record. |

O schema binds event/sample sequence to pid + OS start identity + unique instance + daemon key, with monotonic event time and explicit collector clock correlation. It must run identically on resident control, A and A+C, without resetting idle, retaining providers or changing policy. Controller supplies observer-on/off neutrality evidence before using timings; real transcripts in **both modes** must demonstrate pre-index→loaded→disposed/+5/+30, cache result and the ±1 MiB relief live-heap comparison. Parser fixtures alone cannot establish producer availability.

**Named tests:** `shim_idle_flag_env_forwarding` (argv through production construction, default/zero/precedence/invalid/overflow); `shim_idle_first_spawner_mismatch_once` (two requests, one identity, actual READY wins, one note); `operator_idle_unloaded_status` (zero/nonzero, clientless distinction, old-version unavailable, no idle reset); `idle_collector_visibility` (both sinks/dedup/truncation/missing/wrong-pid/restart); `idle_readout_denominators_and_outcomes` (exact-hit/miss, failed/missing rows, effective/requested mismatch exclusion, mixed-binary rejection, units, two-corpus M, ranges, 128 MiB and 15% report-only). `shared_idle_spawn_reload_process` covers real spawn→READY→unload→reload→served plus a differing shim; controller retains transcript alongside O's stdio fixture.

Mutants: `I0c-drop-spawn-setting`, `I0c-env-beats-zero`, `I0c-suppress-mismatch`, `I0c-report-requested-as-effective`, `I0c-ignore-daemon-log`, `I0c-drop-failures`, `I0c-use-heap-denominator`, `I0c-gate-on-15pct-idle`. Each requires behavioral RED/restored GREEN. **Estimate unchanged: 300–450 production/docs + 400–600 collector/tests**, ≤600 per new file. Depends on F/I0b-1/I0b-2 and O's schema; complete acceptance also requires O's real producer evidence. Review cap: two rounds.

## Verification contract for the four slice PRs

Controller schedules builds/index-loading tests away from soak. Before implementation claims: full project default suite **and MCP-feature library/integration/binary suite**, relevant Python collector suite for I0c, required lint/format/shared-catalog gates. Report passed/failed/ignored/filtered totals and exact exclusions; socket/host checks run on controller host if unavailable. Do not attribute unrelated failures without a same-environment base control or silently fix/rebaseline them.

Every changed behavior needs a test that fails on pre-change behavior, plus negative/edge coverage per path. Carry behavior-only test/hook backports onto the slice parent for same-environment RED, preserving executable production behavior; compilation failure, zero selected tests and environment refusal are inadmissible. Existing-green preservation cases must be identified separately. F's historical base is `1d01b0a7`; I0b/I0c also record their actual merged prerequisite parent.

MCP mutants live in **`mutants/manual/<slice>.json` and are hand-run with `--features mcp`**: the shared runner builds without features and cannot prove these kills. Record exact head, selector/count, mutation anchor, before/mutant/restored hashes, behavioral RED and restored GREEN; run Python collector mutants with their named Python tests. Keep authoritative shared-catalog verification separately required; do not add MCP mutants to it as an apparent substitute. Static `python3 scripts/file_size.py OWNED_FILES...` before/after is required for each slice. None of these executable gates ran tonight.

## Post-soak readout prerequisites and unsettled items

1. Confirm S6 soak completion (expected 2026-10-20 ~12:08 MDT is inherited, not permission by date); C0 selects/pins Py and freezes E/P/ruff/Py workloads. Require ≥50 GiB free on relevant volumes, no competing index load, non-increasing swap, sequential runs and parent's reserved window/admission. All four slices plus O and verification must land for complete both-mode acceptance. No within-session gap-distribution source is supplied: **N stays provisional at 600 even at activation decision**, not a measured p90 choice.
2. Earliest admitted vertical A/A+C check precedes full matrix: same-batch matched **A vs A+C**, resident control and A no-op, **one binary/hash/features per batch**. Fresh stdio process or fresh daemon for every arm/repetition, with a **separate `PRISM_MCP_RUNTIME_DIR` per shared arm**; verify instance start/no pre-existing daemon. Preregister **effective ≠ requested idle OR relief ⇒ EXCLUDED**; keep raw row/reason, never pool it with intended arm. Every enabled run passes `--index-idle-secs 600`. Sample genuine pre-index/loaded/+5/+30 after disposal and repeated cycles; sum surviving shim/daemon memory. Relief follows drop; record return bytes, footprint and heap unchanged within ±1 MiB.
3. I0 M: median matched **current idle physical footprint** savings **≥200 MiB OR ≥15% on ≥2 corpora**, ≥3 admitted runs/side, candidate max < base min; denominator is resident base at the same elapsed idle boundary. Require **+30 s idle footprint ≤128 MiB per checkout on all four corpora in both modes**; **15% of base idle is reported, not an additional idle bar**. RSS/heap/compression/footprint levels and +16 MiB above pre-index footprint/RSS are report-only diagnostics. Inventory pool stacks/query cache/identity/allocator residuals without unmeasured attribution.
4. Exact-hit reload median ≤ same-binary/settings/cache fresh-lazy reference +max(5%,100 ms); **time-to-ready ≤3 s target**, explicit misses. Measure source parse, ready and first-served separately; configured wait remains 20 s default. Preregister per-corpus **served-within-wait / warming / failure counts and proportions**, separately for exact hits and changed-snapshot misses; include warming→poll→success and abandonment. No survivor-only latency distribution. Pre-warm must fall within N of use.
5. Require same-snapshot bytes/G parity; changed-content/stamp/prior-divergence fresh-reference parity with matched history; unchanged rollback and single concurrent build. Then host-capacity evidence names checkout mix, active/unloaded counts, simultaneous reload peaks, headroom, serialized-heavy-refresh assumption and swap. I0 alone cannot certify 12–16 active indexes. Design process-exit B only if **both** A and A+C miss the physical bar; current shim cannot reconnect safely.

- **WRONG / IMMATERIAL (pre-existing), required prerequisite:** read-A/edit-B/stamp-B false freshness above; F owns bounded repair and RED control. No regression attribution to I0a.
- **SMELL / MATERIAL, evidence gate:** physical reclaim, latency/outcome distributions and 12–16-checkout capacity remain unmeasured; acceptance/default activation waits for readout, not a source-review blocker.
- **SMELL / MATERIAL, dispatch/readout gates:** reserve the enumerated files and rebind settled input APIs after rebase; controller schedules verification and separately supplies O's missing serving producer/neutrality evidence. No overlapping writers or quiet-night execution authorized.
- **SMELL / IMMATERIAL, retained limitation:** metadata freshness is not an atomic filesystem snapshot; old-version operator control requires a matching binary; stdio blocked stdout may stall its owner loop. None is a newly demonstrated incorrect result or a reason to widen these slices.

Round-1 dispositions are folded; round-2 design review remains available. Original plan is committed at `3748fc39` (PR #493); controller commits this fold and handoff line. Implementation, O producer/neutrality evidence, tests and measurements remain pending.
