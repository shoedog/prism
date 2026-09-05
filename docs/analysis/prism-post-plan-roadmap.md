# Prism — post-plan roadmap (follow-up queue, strategic fork, Java, Serena)

Date: 2026-07-04. Companion to `docs/analysis/prism-llm-and-accuracy-plan.md` (the
ranked plan). Written as P14 (the final live plan item) is in implementation; refer
back here when choosing what comes after. Owner decisions are marked **[OWNER]**.

## 0. Where the ranked plan ends

P1–P14 shipped. P15 measured-dead (three under-deliveries; do not re-queue). The
post-plan full-corpus Tier-A refresh completed on 2026-08-31. It found a constructible
Zap qualified-constructor-return regression, which was repaired in PR #223 and closed
out in PR #224. No committed baseline was rewritten: the pre-fix full-refresh evidence
is preserved, while the post-fix quick Prism run was invalid only for pinned corpus-SHA
drift. See the qualified-return handoff.

## 1. Follow-up queue (consolidated; living copy = pipeline-lessons.md §follow-up)

Ranked by blast-radius-per-effort:

| # | Item | Why / measured signal | Effort |
|---|---|---|---|
| 1 | ~~**Nested-test-module `use super::*` callers gap** (Rust)~~ **DONE (#166, 2026-07-05)** | Was: no caller edge from a nested module relying on `use super::*`. Fixed via `ResolutionPolicy::glob_anchor_expands` (anchor-only globs `super::*`/`crate::*`/`self::*` resolve through the existing expansion arm). Measured self-host: 832 globs newly resolved, `glob_expand.external` −72.5%. Also fixed `crate::*`/`self::*`/`super::super::*` (same shape). | ✅ |
| 2 | ~~**Return-flow taint**~~ **DONE (#193, merge `f4234013`, 2026-08-23).** Singleton-Exact callee-return → caller-LHS flow shipped as Step 5c with span-bound return enumeration, canonical non-seedable return identities, `ReturnInput`/`ReturnFlow` edges, certified assignment-shortcut suppression, sanitizer-preserving trace semantics, recursion/nested-callable fences, cache `50`, counters, and Tier-2-on/Tier-1-opt-in routing. The full suite passed `3464/0/1`; see `docs/superpowers/handoffs/2026-08-23-return-flow-taint-sol-handoff.md`. | ✅ |
| 3 | ~~**GoOwnerIdentity clause/build-partition blindness** (P13 M1)~~ **DONE (#176, 2026-08-22)** — `GoOwnerIdentity` gains `package_clause` (bare `T` = caller's clause, `pkg.T` = the dir's single ordinary clause); identity-keyed lanes (P11 S2 field types, S4 embedded-interface map, S1 func-value-field index, return lane) are per-declaring-profile with `go_same_package_visible` consult-time filtering (exactly one survivor or drop); imported S4 targets exclude `_test`; return/nested-dispatch/callback-fallback carry owner provenance; P5 registrations filtered by the invocation profile; S4 qualified signature types compare by **resolved import identity** `(import_path, name)` per file import map (root `go.mod` proves local bare types; nested modules + dot-imports + unbound aliases fail closed; `Local↔Bare` fails closed); telemetry `go_owner_identity_partition_*` + `interface_gaps/QualifiedTypeIdentity`; opt-in partition-site custody dump; CPG 44 / sidecar 13. 21 commits; sol spec review + terra r1 FIX / r2 FIX + final gate r3 FIX (1 BLOCKER: `Local↔Bare` matched by name) → wave 4 d68e2dd → r4 APPROVE; same-base control: caddy interface_dispatch 1761→1766, prometheus 2374→2461, etcd 1788→1742 (nested-module fail-closed), ripgrep byte-identical | field_typed / interface-dispatch Exact crossed `foo`/`foo_test` + build partitions (`go_owner_identity_profile_conflict`: etcd 1, prometheus 5). Process lesson: wave-2's fail-closed remedy collapsed interface dispatch (caddy 1761→42) while suite + tier-a stayed green — only the same-base `call-stats` control caught it (`docs/superpowers/pipeline-lessons.md`) | M |
| 4 | ~~**Multi-line-call Step-5b arg gap**~~ **DONE (#171, 2026-08-22)** — byte-contained arg→param selector + trace-gate containment; Java/TS parameter materialization split out → PARKED design A2 (`docs/superpowers/specs/2026-08-21-java-ts-parameter-materialization-design-PARKED.md`) | `g(\n user\n)` gets no arg→param edge (arg lookup at `site.line`); silently NotReached; pinned by a Stage-A test | S–M |
| 4b | **Go dot-import resolution — PARKED at final redesign review cap (2026-08-31).** V6's fresh cap ended non-converging: round 2 found two new `WRONG`s in the repeated proof-completeness class. The function-only export census can treat an imported type/var/const as name-absent and let R5 mint an unrelated Exact; the cited occurrence-binding pattern does not bound all Go scopes (named results and select receives are concrete misses). Separately, the nonempty-only clause census can certify a directory while silently ignoring an ordinary file whose recovered package clause is empty. No implementation from v6. A future owner-authorized redesign starts a new cap from the Go-spec-derived all-declaration namespace proof, empty-clause/parse poison, explicit consumer-purpose mode, and canonical dot-import map in the final review outcome. | Measured recall remains 4 zap `observer.New` sites; no new runtime/corpus claim. Authority: `docs/superpowers/specs/2026-08-23-go-dot-import-redesign.md` §7 and `docs/superpowers/handoffs/2026-08-31-go-dot-import-final-review-handoff.md`. | PARKED |
| 5 | ~~**Pointer-embedded Go fields** (`*Listener`)~~ **DONE (#174, 2026-08-22)** — extraction via the grammar `type` field + bare-`*`; selector vs target identity; package-scoped, fail-closed promotion/S2/S4 (locally-proven-struct S2 gate with clause+profile; multi-package outer names refused); 7 review rounds. Perf follow-up: second `GoTypeProvider` construction during receiver rematerialization (`src/call_graph.rs` ~L3085) — benchmark/reuse | Pre-existing `extract_one_field` drop; affects shipped embedding + P11 S2/S4; fails safe | S |
| 6 | ~~**`--review-no-diagrams`**~~ **DONE (#170, 2026-08-22)** | Diagram payloads dominate compacted review output (552 KB post-P1; diagrams are most of it) | S |
| 7 | ~~**Advisory/CWE sanitizer recognizers cross-match languages**~~ **DONE (#169, 2026-08-22)** — advisory tier gated by `recognizer.languages`; `sanitizer_supported` derived | P10 gated the verdict path only (deliberate); advisory noise on polyglot repos | S |
| 8 | **First-enqueue depth-lock relaxation** (P14 MIN1) + per-callee CFG scoping | Documented v1 losses in descent; only worth it if a measured case appears | S–M |
| 9 | Python pending adjudication tail | 175 characterized-not-classified (black 26 / httpx 86 / mypy 63); the 25/corpus sample said mostly prism_fp candidate-tier + prism_fn recall; bulk-adjudicate only if a Python initiative needs the denominator | S (codex batch) |
| 10 | ~~**Parameter-slot fail-closed alignment**~~ **DONE (#173, 2026-08-22)** — `src/parameter_slots.rs` prefix semantics; Go grouped names; positional consumers on slots; `prism nav call-stats --dump-sites`. **Level-3 callback minting DISABLED** (4 review rounds could not make the bare-name resolver sound: shadowing by params/locals/arrow/generator params, conditional assignments, unresolved imports) → see item 13 | sol spec review 2026-08-21 (`docs/superpowers/specs/2026-08-21-p8-p9-p10-designs-and-spec-review-sol.md`) | M |
| 11 | ~~**prism-mcp lazy handshake**~~ **DONE (#178, 2026-08-22)** — approach A+C: the index build runs on a background thread from server startup; `initialize`/`ping`/`tools/list` answer immediately; a `tools/call` arriving before readiness waits at most `--first-call-wait` (default 20 s, max 600, cumulative per build attempt) and then returns a structured, retryable `index warming` result (`isError: true`, status JSON in text + `structuredContent`, `prism/index_state`, `prism/retryable`); build failures reported and retried on the next valid call; `--eager` keeps the legacy synchronous startup (pre-warm recipe); Tier-C `warm_gate_check` uses `--eager`. Spec `docs/superpowers/specs/2026-08-22-prism-mcp-lazy-handshake-design.md` (3 sol spec rounds, cap); terra impl + 1 fix wave; sol diff review r1 FIX (tests) → r2. Live codex 0.147 probes: control (main eager, cold TypeScript) 0 prism tools; lazy (cold TypeScript) 6 tools exposed, warming result, retry; lazy (cold prometheus) warming → real data. Follow-ups: none required; optional first-call latency probe in the Tier-C gate | `docs/analysis/2026-08-21-tier-c-partd-readout.md` §Caveats | S–M |
| 12 | **Java/TS parameter materialization (A2)** — PARKED design with sol spec review (canonical slot model, rest/variadic/spread semantics, Level-3 identity, go.mod/sidecar cache); prerequisite for Java/TS interprocedural taint | `2026-08-21-java-ts-parameter-materialization-design-PARKED.md` | L |
| 13 | ~~**Sound Level-3 callback resolution**~~ **#13a B1 DONE (#221, merge `49216059`, 2026-08-31).** Go B1 now mints `Exact` `ParameterCallback` edges only for occurrence-proven bare in-repo non-test free functions passed to strict import-path-aware callable slots on singleton-Exact free-function HOF calls. Synthetic sites retain exact `pre_resolved_target` plus distinct source-callee identity through Step-5b, reasoning, CPG cache `55`, navigation sidecar `24`, full/incremental/round-trip/sidecar paths, conservation telemetry, and the custody dump. Watched positives were RED on exact base and GREEN on the branch; focused gates and the full suite passed (`3728/0/1`), Tier-A matrix passed, and Tier-A quick was invalid only for the same pinned-corpus SHA drift on branch and exact base (oracle/SUT error rates `0.000`). The original 67-row floor reconciles to 57 stable production drops plus 10 named pre-candidate exclusions. A successor 13-run screen conserved `981 = 3 accepted + 978 drops`: two Delve edges were fixture-only, while Kubernetes `k8s.io/pod-security-admission` supplied one non-fixture production edge, `addCheck(CheckProcMountRestricted)` → `f()` → exact `CheckProcMountRestricted`; its package test passed. The real-positive gate is therefore met without weakening B1. B2–B5, methods, variadics, generics, tests, external named types, assignment fallback, JS/TS, and other languages remain excluded and require separate authority. | Authority: `docs/superpowers/specs/2026-08-31-go-level3-bare-function-callback-design.md`; implementation handoff: `docs/superpowers/handoffs/2026-08-31-go-level3-bare-function-callback-design-handoff.md`; successor measurement: `docs/superpowers/handoffs/2026-08-31-go-level3-b1-real-corpus-measurement.md`. | DONE |
| 14 | **Nested-module Go import identity** (owner: full scope in four slices, 2026-08-22) — **slice 1 oracle hardening DONE (#180)**: `interface-manifest` package-qualified implementer identities + target spans; `dispatch_oracle.py` identity-aware `(dir, clause, type)` compare, concrete-receiver ground truth = definition target, C/N/U partition for `implementation` results, zero-fanout scoring (`recall_gap` / `not_dispatch`), `--baseline` delta mode with `gate_ok` = no delta blocker (over_approx / timeout / unresolved / target_mismatch among newly-Exact sites) ∧ fanout-positive site and edge coverage ≥ 0.90 (tool contract per `eval/README.md` §Dispatch oracle and PR #180; spec §2 updated to match), pinned `go env`, Unicode-safe clause lexer; identity-aware baselines on main's resolution (fanout>0 precision): caddy 1.0000, prometheus 0.94, etcd 0.989, hugo 0.992; **slice 2 hygiene DONE (#179)**: Go-only `testdata` exclusion, go.mod tokenizer + `module` grammar + `CheckPath`, symlink-safe immutable manifest snapshot (`symlink_refused` sentinel), go.work hashed, CPG 45 / sidecar 14 / skip policy 2; **slice 3 effective module identity DONE (#182, 2026-08-23)**: `src/go_module_graph.rs` (+`replacements.rs`, `semver.rs`, `identity.rs`) computes the ACTIVE module set and effective import paths from the snapshot (go.work `use`; replace precedence — workspace wins by PATH, go.work replaces first, union, RHS conflicts → `workspace_invalid`; wildcard replace gated on an active-main `require`; version-specific → unproven; active-main self-replace inert; main-module path `CheckImportPath` vs dependency `CheckPath` [owner: Go-faithful split]; x/mod-faithful semver for `require`/`exclude`/`retract`/replace; whole-workspace vs subtree fail-closed layering; snapshot-only, ≤1 parse per file, memoized identities, conservation telemetry `go_import_path_{proven,unproven}_files` + `go_module_graph/*`); CPG 46 / sidecar 15; same-base control: caddy/prometheus/hugo resolution leaves unchanged (prometheus 5/5 modules active), etcd interface_dispatch 1742→2002 (13/13 modules active), ripgrep byte-identical; hardened oracle delta: caddy/prometheus/hugo gate TRUE (0 newly-Exact), etcd 375 newly-Exact = 370 sound + 5 over_approx — the 5 are roadmap-#17 concrete-receiver class (owner exception 2026-08-22: merge, #17 next must turn them to 0 against `oracle-s3b-etcd.json`); reviews: terra r1 FIX / r2 FIX / r3 APPROVE (4 WRONGs fixed over 3 fix waves), Ox ∥ APPROVE ×3; process lesson: implementer commits are clone-local — controller must push after every wave; **slice 4 DONE (#184, 2026-08-23)**: alias-aware `Local↔Local` by path (profile/clause-scoped alias index, whole-RHS canonical expansion incl. parameterized aliases, fail-closed `AliasUnresolved`, shadow-aware predeclared normalization) + the owner/profile-keyed promoted-selector snapshot FOUNDATION (five profile-safety axes incl. receiver method-set shape; shallowest-selector rule; signature-level embedded-interface profile comparison; not consumed by routing); CPG 48 / sidecar 17; same-base vs pre-#17 main: prometheus interface_dispatch +37, etcd +60 (alias expansions 25/33/8), 0 recall transitions, oracle gates TRUE; re-verified after the #186 merge (final head 0028af1, alias_resolver threaded through #17's record_interface_type): suite 3409/0/1, tier-a 104/104, vs post-#17 main prometheus +164 / etcd +70, oracle vs p17e baselines all four gates TRUE (newly-exact 0/41/38/0, coverage >=0.9649); implemented TWICE independently (sol = merged; Ox Alpha Free = calibration branch `go-alias-aware-local-local-ox`, converged site-for-site to sol's output after one review-driven fix wave) — the parallel run surfaced two Part-B defects on the merge candidate (shallowest-selector pruning; name-only interface profiles) that the serial review loop had approved past; spec §5 clarifications recorded — spec `docs/superpowers/specs/2026-08-22-go-nested-module-import-identity-design.md` §5 | PR #176 body; PR #182 body + `docs/superpowers/handoffs/2026-08-22-go-effective-module-identity-handoff.md` (slice 3); `tests/lang/go/owner_partition_fix_wave_test.rs` nested-module fixtures | S–M |
| 15 | ~~**Go provider build-time perf SMELLs**~~ **(a) DONE (#185, 2026-08-23)** — one plain `GoTypeProvider` shared by embedding promotion, receiver rematerialization AND the `CpgContext` type registry (interface dispatch keeps its import-path-aware provider — canonical signature identity differs); dispatch reordered so peak live providers = 1; provider transferred (not retained) into the registry; incremental-rebuild path covered; test-only construction/live counters; gate: 5 corpora + 4 manifests RAW-BYTE identical, suite/tier-a = main; timing: one construction (~7–9 s on etcd/prometheus, ≈15–20 % of build wall) removed. Implemented by the Ox Alpha Free lane (first implementation trial; 2 review-driven fix waves). **(b) DEAD** — #14 slice 3 made the module graph snapshot-only (no per-file go.mod disk reads remain) | #174 / #176 review records; #185 body | S |
| 16 | ~~**Package-qualified INTERFACE identity in the Go dispatch table**~~ **DONE (#215; closeout #216, 2026-08-31).** Receiver-owner provenance now reaches one owner-qualified `go_proven_interface_outcome` consult used independently by resolver and manifest; the legacy bare-name table remains schema-compatible telemetry but cannot authorize edges. The five declared corpora had zero natural candidate rows, so the authorized constructible Go source fixture supplied the behavioral discriminator: exact base reached legacy `1/1/1` and minted `decoy.Wrong.Run`, while the candidate selected the registered `app.worker` or failed closed. Focused tests passed `15/15`, the full suite passed `3510/0/1`, ten corpus artifact pairs were byte-identical, all four oracle deltas passed at full coverage with no blockers/new Exact sites, and implementation review closed with zero open `WRONG` or in-scope `SMELL`. CPG remains `54`; navigation sidecar advanced to `23`. | Handoff: `docs/superpowers/handoffs/2026-08-31-go-interface-identity-post-provenance-handoff.md`; evidence packets: adjacent Task 1/2/4 handoffs. | DONE |
| 17 | **Concrete-receiver call sites must not enter interface dispatch / multi-target on concrete receivers** (NEW 2026-08-22, precision) — the interface manifest predicate admits any call whose method name occurs on some interface (`src/navigation/queries.rs` ~L610; caddy `adapter.Adapt` on `caddyfile.Adapter`), and concrete-receiver sites can mint multiple targets (prometheus `storage.Close` → `tsdb/agent.DB` + `storage/remote.Storage`); the oracle now classifies these (`definition_kind: concrete`) so the delta is measurable. **DONE (#186, 2026-08-23; design v8 on the PR — R1(b) wording corrected after a tier-a regression)**: one shared `go_concrete_receiver_route` (resolver + manifest) + a P10-keyed declaration-kind index (canonical alias targets, `type D I`, synthetic unnamed-interface aliases) serialized on `CallGraph` (cache 47/16, four-path cache parity); R1(a) own-method direct with on-demand owners (typed_param Exact ×7–18 per corpus; NameOnly typed_param eliminated), R1(b) existing promotion lane unchanged + deferred-drop, R1(c) depth-aware embedded supply, R1(d) named func-value fields → P5, R1(e) terminal drop; on-demand R2 with a bare-name-collision bail (v9-A; killed the 16 prometheus Iterator false edges — the first measured #16 mitigation); external new-recovery drop (v9-B; killed 4 io.Closer false edges); fail-closed on local type shadows, value rebindings (type-switch/range/closure/`:=` declarations), profile-conflicting owners; all 14 etcd + 20 prometheus concrete over_approx oracle sites resolved; oracle gates TRUE everywhere except one documented hugo env-gap waiver (tocss.go:122, `//go:build extended` vs empty oracle tag set → follow-up: oracle tag-set coverage); direct-lane gopls audit (new controller tool) 199/0/1 over 5008 new direct sites; R3 legacy projection byte-identical per site; #17b population telemetry shipped (`go_unproven_receiver_bare_fallback_{sites,hits,edges}`). Follow-ups: qualified embedded interfaces resolve nowhere on main or branch (label-only relabel; with #16); wave-5 externality proof under partially-proven module graphs (Ox); oracle tag-set coverage. **Design v7 = design-of-record (merged #183)** `docs/superpowers/specs/2026-08-22-p17-narrow-concrete-receiver-direct-design.md`: routing decomposition for PROVEN concrete-recovered receivers — R1(a) own method → direct Exact (on-demand owner), R1(c) embedded-interface → S4 as today, R1(d) func-valued field → P5, R1(e) no selector → terminal drop (`ConcreteReceiverNoSelector`); R1(b) promoted-from-embedded-concrete direct routing DEFERRED to #14 slice 4 by owner decision after four scoped confirmations each found a new profile-safety axis (package qualifier, ordinary fields, own methods, embedded-alias selector names) — such sites take `concrete_promoted_deferred_drop` (fail-closed); R2 proven interface → S4; R3 unproven/external UNCHANGED (`Ambiguous(profile conflict)` = owner with >1 declaring file ⇒ R3); declaration-kind index keyed by P10 identity with canonical alias targets + `type D I`, serialized on `CallGraph` (cache 46→47 / 15→16, cache-parity acceptance); one shared consult fn (resolver + manifest); `dispatch_route` diagnostic; route-specific subtraction audit; baselines s1e (caddy/prometheus/hugo) + `oracle-s3b-etcd.json` (etcd: s1e's 11 + slice 3's 5 → 0). Review record: sol r1–r4 + 2 scoped confirms, Ox ∥ each (calibration in lane ledger). **#17b (NEW, carved out, owner 2026-08-22: separate, measured first)** — today's R3 bare `iface_key` ladder is NOT fail-closed for EXTERNAL receivers (external concrete `q.A{}` + in-repo `p.A{M()}` → false Exact; `http.Handler` + in-repo `Handler` → wrong implementer set); #17-narrow adds `go_unproven_receiver_bare_fallback_{sites,hits,edges}` so #17b's per-corpus population is counted BEFORE its design; terminal-R3 changes interface_dispatch on every corpus and drops unverified external-interface recall — own same-base control + oracle delta | sol slice-1 review SMELL 7; oracle `definition_kind` histograms; `oracle-s3b-etcd.json` | S–M |
| 18 | ~~**Gate the empty-live RTA fallback on proven identity**~~ **CLOSED AS SUPERSEDED (#217, 2026-08-31)** — the 2026-08-22 ledger recorded a pre-owner-provenance hazard: loss of interface identity plus `NonLocalConstructionFallback` minted 131 newly-Exact Prometheus sites, including 14 over-approx sites. Current production resolution no longer emits the legacy table's candidates. Every resolver read passes through `go_visible_s4_implementers`, which ignores the bare-table candidates and recomputes satisfiers from a full `GoOwnerIdentity`, exact declaration provenance, caller build visibility, structural signatures, and then live selection; the terminal #16 route uses `go_proven_interface_outcome` directly. The manifest mirrors those same owner-qualified consults, and the only other read is a call-stats fanout histogram. Existing direct-interface and S4 regressions prove that a live same-name decoy is not reintroduced while the correct non-live implementer survives the owner-qualified fallback. The legacy provider table and its NLCF counter deliberately remain schema-compatible build telemetry (current retained counts: Caddy 3, etcd 578, Hugo 330, Prometheus 146), not edge authority; post-#16 oracle reports pass all four Go corpora with zero blockers/newly-Exact sites. No runtime, cache, or schema change is required. Evidence: `docs/superpowers/handoffs/2026-08-31-rta-empty-live-fallback-disposition.md`. | ✅ |
| 19 | **Analyzer-roadmap Phase 0 interfaces** (NEW 2026-09-04): `--format sarif` (SARIF 2.1, `src/output/sarif*.rs`), `prism targets` (projection into `docs/contracts/targets.schema.json` v1.0 for the runtime fault-injection harness, `src/targets/`), `prism::api` facade with a stated compatibility promise (`src/api/`, main.rs is its first consumer; `src/cli.rs` holds the clap structs), `src/finding_confidence.rs` (`exact|nameonly|unlabeled` + tier; CPG-derived findings are `unlabeled` until item 2), README truth pass with a README gate test. **DONE in PR #229 (merge `551adc46`, 2026-09-04).** No cache bump; existing formats byte-identical (stdout/stderr/exit) vs the c220525c binary over 1,598 invocations (`scripts/phase0-byte-control.sh`); Tier-A matrix 104/104 both binaries. Design: `docs/superpowers/specs/2026-09-04-prism-phase0-sarif-targets-api-design.md` (v5 settled, 3 sol rounds + Opus seat); plan: `docs/superpowers/plans/2026-09-04-prism-phase0-sarif-targets-api.md`. Follow-ups filed in spec §9 (multi-run `paper` gap, clap `name` rename with its two version-grammar consumers, `angle`/`delta` findings, lossy anchors, structured `FindingHint`, typed call edges via `api`). **Next: roadmap item 2** — DataFlow confidence via reaching definitions. A clean local-only worktree `/Users/wesleyjinks/code/slicing-item2` has spec v6.2, plan v4, and four branch-only commits through `9fbdd929`; it was measured 70 current-main commits behind and 4 ahead, with no remote containing the head. Ownership and integration must be rebound before any write. | Analyzer roadmap `~/code/tools/03-tooling-plan-roadmap.md` §3 Phase 0; `04-prism-plan-roadmap.md` §2; consolidated handoff `docs/superpowers/handoffs/2026-09-04-python-js-nav-sequence-handoff.md` | S–M each; PHASE 0 DONE / ITEM 2 PENDING CUSTODY |

## 2. The strategic fork **[OWNER]**

Three candidate directions once the queue's top items are drained (or interleaved):

### A. Python/JS receiver-typing (deepen the highest-value languages)
- Signal: mypy exact-callees precision **0.65** in the 2026-07-03 baseline ("the known
  Python receiver-typing gap; worth its own look"); Python 54–65% unresolved, JS ~92%;
  the 2026-06-23 maturity verdict ranked "Python+JS maturity" above C/C++ completion
  and SCIP. Prior groundwork exists (self-receiver same-file narrowing; corrected
  premise: same-class self/this already resolves — the gap is cross-class/typed-param
  recovery, the Python analogue of P11's Go lanes).
- Cost model well understood after P11/P13 (typed-fact lanes, consult-time filtering,
  rematerialization pattern all reusable).
- **Owner-selected execution queue (2026-09-04):** (1) Python imported/cross-module
  typed receivers — DONE in the bare member-import slice (#226, merge `5e54d483`) and
  module-alias-qualified slice (#227, merge `4298e548`); (2) Python authoritative
  module/scope resolution — DONE as a sequence of bounded proof increments;
  its first unaliased dotted-module increment (`import pkg.models` plus
  `pkg.models.Class`) is DONE in #228 (merge `7488bb64`), and its second increment,
  namespace-package submodule imports (`from pkg import models` plus `models.Class`),
  is DONE in #230 (merge `5051918f`);
  (3) JS/TS lexical-scope-aware receiver binding prerequisite — DONE in #231
  (merge `6771d530`); (4) JS/TS typed-parameter and `new`-constructor receiver
  recovery — DONE in #232 (merge `434764a6`); (5) product-oriented read-only
  `nav_symbol_spans` v1 — DONE in #234 (merge `90c522b`); (6) CLI-only onboarding
  report v1 — DONE in #235 (merge `a531355`): one cached build, compact
  repo-map/call-stats orientation, stdout by default, explicit create-new-only file
  output. Java native resolution versus LSP delegation remains an owner decision after
  a future J1/J2 evidence bootstrap. Do not collapse steps 2–4 into the old combined design.
- **Authorized correctness continuation (2026-09-04):** the completed merges do
  not close receiver authority. Current-main REDs demonstrate enclosing constructor
  and type-name shadows, conditional initialization, and loop-carried writes that
  incorrectly mint Exact edges. The two bounded owner-visibility and temporal-proof
  repair slices are published in [PR #238](https://github.com/shoedog/prism/pull/238)
  at `9a790419` (merged as `350cc89`): default 3,713/0/1, MCP 3,903/0/1, matrix
  104/104; quick has zero oracle/SUT errors but remains invalid for corpus pin drift. See
  `docs/superpowers/handoffs/2026-09-04-python-js-receiver-authority-repair-handoff.md`.
- **Implemented continuation — PR #239 (merged as `862166d`), `99d50f1`:** directly exported JS/TS class identity,
  imported constructor/parameter receivers, then Python inert regular-package
  initializers. See `docs/superpowers/specs/2026-09-04-imported-class-receivers.md`.
  The bounded syntactic census is not a measured corpus-recall gain.
- **Implemented continuation — PR #240 (merged as `f3bf88e`), `0a2edf6`:** separate named TS type-only class
  authority and explicitly anchored Python relative submodule imports. See
  `docs/superpowers/specs/2026-09-04-type-only-relative-receivers.md` and matching handoff.
- **Receiver measurement closure — PR #241 (merged `1886907`), `f5d766b`:** fixed-source pre/post-#240 comparison
  preserves all old Exact edges across 548 sites and adds 4 Black + 7 Excalidraw
  edges, all checked through served callers/callees. This is partial-source evidence,
  not whole-corpus recall. Paired Rust pins reproduce identical historical flip and
  obsolete literal-address outcomes; an inherited fully qualified call miss at
  `src/main.rs:183` was separated for the next Rust repair item. No new Python/JS feature
  or baseline change. See `docs/eval/receiver-closure/2026-09-04-readout.md`.
- **Next increment — PR #242 OPEN, `e9e153e`, not merged:** Cargo
  binary-to-own-library identity repairs main.rs:183; 72 Exact additions and no lost
  Exact targets on fixed current Rust source. Direct named default-class receivers
  support JS/TS/TSX value imports and erased TS type imports. Prior Python/JS samples
  remain byte-identical. Quick is baseline-invalid for SHA drift; pins unchanged.
  See `docs/eval/receiver-closure/2026-09-04-rust-default-readout.md`.

### B. Tier-C Part-C continuation (measure end-task value) — **Part-D run 2026-08-21: REFUTED on the 11-task corpus with gpt-5.5 (median ΔdR 0.0; 6/9 off-saturated; TS unmeasured) — see `docs/analysis/2026-08-21-tier-c-partd-readout.md`**
- The A/B end-task harness is BUILT (branch `tier-c-part-c`, unmerged). Prior verdict:
  citation-precision value tracks per-language maturity (Go/Rust +0.18..0.26, TS
  +0.23, Python wash); owner ruled ROI sufficient to continue.
- Natural capstone after the accuracy wave: quantifies what rounds 1–8 bought at the
  end-task level, and produces the numbers that decide fork A vs C rationally.

### C. Breadth: Java (and the Serena-informed build-vs-delegate question, §3–§4)
- Java is prism's biggest supported-but-immature language (§3).

Recommendation (controller): re-baseline → queue item 1 as a short round → run fork B
(the harness exists; it sharpens the A-vs-C choice with data) → then A or C per its
results. Fork order is an **[OWNER]** call.

## 3. Java: check it, baseline it, build it out

**Current state (grounded):**
- Java is one of the 11 parsed languages (tree-sitter-java) and has ladder support in
  spots (unqualified-call implicit-`this` rung shared with C++), but the 2026-06-23
  per-language maturity measurement recorded **Java ≈ 99% exact-tier gap** — nearly
  nothing resolves Exact. No Java corpus has ever been a tier-a anchor (committed
  anchor set: 3 Rust / 5 Go / 3 Python).

**Work items (in order; each is cheap to stop after):**
1. **Validity survey (J1)** — pick 3–4 candidate corpora spanning shapes (suggest:
   `gson` [small, clean], `junit5` [modules + annotations], `okhttp` [Kotlin-adjacent,
   check Java-only subset], `spring-petclinic` [framework-heavy, small]); oracle =
   Eclipse JDT LS (`jdtls`) through the existing LSP-oracle harness; measure OER per
   corpus against the ≤0.10/0.25 floor discipline. **Gate: a corpus is an anchor only
   if jdtls resolves cleanly** — Java annotation processors/generated sources are the
   expected oracle-invalidity risk (tokio-class).
2. **Census (J2)** — `prism nav call-stats` per corpus: unresolved% split by drop
   class, same-package vs cross-package, static vs instance. This tells us WHICH
   ladder rungs are missing (likely: package/import resolution for
   `com.foo.Bar.baz()` chains, static imports, overload arity [Go precedent #100],
   constructor `new Bar()` typing, interface dispatch).
3. **Anchor + gap plan (J3)** — commit 2–3 valid anchors to baseline.md; write the
   Java gap plan as a ranked mini-plan (the P1–P15 method, applied to one language).
4. **Build-out (J4+)** — only after J1–J3 say the wins are there; the P11/P13
   machinery (typed-fact lanes, owner identities, consult-time filtering) is the
   template. **Alternative: LSP delegation for Java instead of native build-out — see
   §4; J1/J2 produce exactly the data to decide that.** **[OWNER]** after J3.

## 4. Serena: comparison, benchmarking, ideas to pull in

Profile (from https://github.com/oraios/serena, fetched 2026-07-04): MIT-licensed MCP
toolkit (~26k stars, v1.5.3 May 2026) giving agents symbol-level retrieval AND editing
over an **LSP backend** (open-source language servers via an abstraction layer; paid
JetBrains-plugin backend as an alternative). ~40+ languages by LSP delegation.
Retrieval: `find_symbol`, `symbol_overview`, `find_referencing_symbols`,
`find_declaration`, `find_implementations`, `diagnostics`. Editing:
`replace_symbol_body`, `insert_after_symbol`, `insert_before_symbol`, `rename`,
`safe_delete` (+ JetBrains-only refactors: `move`, `inline`, `propagate_deletions`).
Plus regex search/replace, shell execution, per-project memory system + `serena init`
onboarding, and a published "unbiased evaluation prompt" (~20 routine coding tasks).

**Architectural contrast (the honest one-paragraph version):** Serena is
*LSP-with-agent-ergonomics* — type-resolved, per-language toolchain-dependent,
write-capable. Prism is *own-resolution structural analysis* — tree-sitter + the
S3 ladder, no toolchain required, deterministic cached CPG, explicit confidence tiers
(Exact/NameOnly + drop classes), and the layers Serena has no analogue for:
diff-driven slicing (30 algorithms), taint reasoning with path-proven verdicts and
witness graphs, module/repo dependency graphs, and the measured-accuracy harness
itself. Serena has what prism deliberately lacks: symbol-anchored EDITING, 40+
languages for free, and cross-session memory/onboarding UX. Our own tier-a data
tempers the "LSP is ground truth" assumption: measured LSP oracle self-error runs
0–25% per corpus, and some corpora are oracle-invalid outright (tokio).

**Benchmark plan (S1–S3, cheapest first):**
1. **S1 — nav head-to-head on anchored corpora**: Serena `find_referencing_symbols` /
   `find_declaration` vs `prism nav callers`/`callees` on the tier-a anchor seeds.
   CAVEAT: Serena is LSP-backed, so scoring both against the LSP oracle is circular
   for Serena — the informative axes are (a) wall-clock + token cost per query
   (P12-style payload measurement), (b) behavior on the oracle-INVALID corpora
   (tokio-class — where LSP breaks, does Serena degrade while prism holds?), (c)
   agreement rate + who wins each disagreement under manual adjudication (the June
   κ protocol, reusable as-is).
2. **S2 — end-task A/B via the Tier-C Part-C harness** (exists, branch
   `tier-c-part-c`): agent+serena-mcp vs agent+prism-mcp vs both on the steered task
   set. This is the benchmark that answers the question users actually have, and it
   makes fork B do double duty. Java tasks included → feeds the §3 build-vs-delegate
   decision with direct evidence.
3. **S3 — adopt Serena's ~20-task evaluation prompt** as a third-party eval run
   against prism-mcp (complements our adoption eval; their prompt, our corpora).

**Ideas worth pulling in (each its own scoped item, not a bundle):**
- **Anchored-edit coordinates (read-only-preserving alternative to editing tools):**
  Serena's `insert_after_symbol`/`replace_symbol_body` collapse "8–12 error-prone
  steps" per their eval. Prism can capture most of that value WITHOUT becoming a
  writer: an MCP tool returning byte-precise, symbol-anchored edit spans (symbol body
  span, insertion points, indentation context) that the agent's own editor applies.
  Keeps the non-destructive posture; adds the ergonomics. Candidate: `nav_symbol_spans`.
- **Full write tools** (Serena-parity `replace_symbol_body` etc.) — **[OWNER]**: this
  changes prism-mcp's safety posture (today: read-only + one local-state tool);
  decide only with a concrete consumer in hand.
- **Onboarding/memory UX**: `serena init`-style one-shot project onboarding (warm the
  cache, emit repo-map + module-deps + call-stats summary into a project memory file
  the agent reads on session start). We have all the pieces (skills, cache warming,
  repo-map); this is packaging, not analysis. Small and high-leverage.
- **LSP-delegation hybrid for immature languages**: for Java/C++ (99% gap / doesn't
  complete), a `source: Lsp` evidence path could serve nav queries via a bundled
  language server while native resolution matures — Serena's core trick, scoped to
  the languages where prism is weakest, retaining prism's evidence shape + confidence
  labeling (LSP results enter as their own confidence class, never silently mixed
  with Exact). Decide after J1/J2 (§3) and S1 (circularity data).

## 5. Standing constraints (apply to everything above)

Consumer-visibility doctrine (nothing below Exact feeds asserted findings); precision
floor (drop-not-fanout for Rust/Go); one cache transition per PR; verify-first for
any tail-chasing item (the P15 lesson, three strikes); the pipeline-lessons.md process
lessons govern any multi-agent execution.
