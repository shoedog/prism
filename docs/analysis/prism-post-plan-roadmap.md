# Prism — post-plan roadmap (follow-up queue, strategic fork, Java, Serena)

Date: 2026-07-04. Companion to `docs/analysis/prism-llm-and-accuracy-plan.md` (the
ranked plan). Written as P14 (the final live plan item) is in implementation; refer
back here when choosing what comes after. Owner decisions are marked **[OWNER]**.

## 0. Where the ranked plan ends

P1–P13 shipped (#149–#164, rounds 1–7). P15 measured-dead (three under-deliveries;
do not re-queue). P14 (interprocedural taint descent) in flight — when it merges, the
plan is **complete**. First post-plan action regardless of any fork choice:

- **Full-corpus tier-a re-baseline** — the 2026-07-03 baseline pre-dates P13 and P14.
  One run absorbs both (zap exact-caller fps clear; BoundaryExited → Reached shifts
  where descent applies) and closes the plan's books with measured numbers. Rules:
  primary tree is the SUT (pipeline-lessons #12), rebuild release at HEAD first.

## 1. Follow-up queue (consolidated; living copy = pipeline-lessons.md §follow-up)

Ranked by blast-radius-per-effort:

| # | Item | Why / measured signal | Effort |
|---|---|---|---|
| 1 | ~~**Nested-test-module `use super::*` callers gap** (Rust)~~ **DONE (#166, 2026-07-05)** | Was: no caller edge from a nested module relying on `use super::*`. Fixed via `ResolutionPolicy::glob_anchor_expands` (anchor-only globs `super::*`/`crate::*`/`self::*` resolve through the existing expansion arm). Measured self-host: 832 globs newly resolved, `glob_expand.external` −72.5%. Also fixed `crate::*`/`self::*`/`super::super::*` (same shape). | ✅ |
| 2 | **Return-flow taint** (callee-return → caller-LHS) | P14's declared non-goal — descent finds sinks *inside* callees but tainted returns are invisible; needs Step-5b-class edge construction (`x = f(user)` where f returns its tainted param/source) | M–L |
| 3 | ~~**GoOwnerIdentity clause/build-partition blindness** (P13 M1)~~ **DONE (#176, 2026-08-22)** — `GoOwnerIdentity` gains `package_clause` (bare `T` = caller's clause, `pkg.T` = the dir's single ordinary clause); identity-keyed lanes (P11 S2 field types, S4 embedded-interface map, S1 func-value-field index, return lane) are per-declaring-profile with `go_same_package_visible` consult-time filtering (exactly one survivor or drop); imported S4 targets exclude `_test`; return/nested-dispatch/callback-fallback carry owner provenance; P5 registrations filtered by the invocation profile; S4 qualified signature types compare by **resolved import identity** `(import_path, name)` per file import map (root `go.mod` proves local bare types; nested modules + dot-imports + unbound aliases fail closed; `Local↔Bare` fails closed); telemetry `go_owner_identity_partition_*` + `interface_gaps/QualifiedTypeIdentity`; opt-in partition-site custody dump; CPG 44 / sidecar 13. 21 commits; sol spec review + terra r1 FIX / r2 FIX + final gate r3 FIX (1 BLOCKER: `Local↔Bare` matched by name) → wave 4 d68e2dd → r4 APPROVE; same-base control: caddy interface_dispatch 1761→1766, prometheus 2374→2461, etcd 1788→1742 (nested-module fail-closed), ripgrep byte-identical | field_typed / interface-dispatch Exact crossed `foo`/`foo_test` + build partitions (`go_owner_identity_profile_conflict`: etcd 1, prometheus 5). Process lesson: wave-2's fail-closed remedy collapsed interface dispatch (caddy 1761→42) while suite + tier-a stayed green — only the same-base `call-stats` control caught it (`docs/superpowers/pipeline-lessons.md`) | M |
| 4 | ~~**Multi-line-call Step-5b arg gap**~~ **DONE (#171, 2026-08-22)** — byte-contained arg→param selector + trace-gate containment; Java/TS parameter materialization split out → PARKED design A2 (`docs/superpowers/specs/2026-08-21-java-ts-parameter-materialization-design-PARKED.md`) | `g(\n user\n)` gets no arg→param edge (arg lookup at `site.line`); silently NotReached; pinned by a Stage-A test | S–M |
| 4b | **Go dot-import resolution** — **REJECTED at spec review 2026-08-21; deferred with redesign inputs** (`docs/superpowers/specs/2026-08-21-go-dot-import-resolution-deferred.md`) | Measured recall gap: 4 zap `observer.New` sites in `package foo_test` adjudicated `prism_fn` (2026-07-04 re-baseline); prism resolves neither the dot-import nor the resulting bare cross-package calls | S–M |
| 5 | ~~**Pointer-embedded Go fields** (`*Listener`)~~ **DONE (#174, 2026-08-22)** — extraction via the grammar `type` field + bare-`*`; selector vs target identity; package-scoped, fail-closed promotion/S2/S4 (locally-proven-struct S2 gate with clause+profile; multi-package outer names refused); 7 review rounds. Perf follow-up: second `GoTypeProvider` construction during receiver rematerialization (`src/call_graph.rs` ~L3085) — benchmark/reuse | Pre-existing `extract_one_field` drop; affects shipped embedding + P11 S2/S4; fails safe | S |
| 6 | ~~**`--review-no-diagrams`**~~ **DONE (#170, 2026-08-22)** | Diagram payloads dominate compacted review output (552 KB post-P1; diagrams are most of it) | S |
| 7 | ~~**Advisory/CWE sanitizer recognizers cross-match languages**~~ **DONE (#169, 2026-08-22)** — advisory tier gated by `recognizer.languages`; `sanitizer_supported` derived | P10 gated the verdict path only (deliberate); advisory noise on polyglot repos | S |
| 8 | **First-enqueue depth-lock relaxation** (P14 MIN1) + per-callee CFG scoping | Documented v1 losses in descent; only worth it if a measured case appears | S–M |
| 9 | Python pending adjudication tail | 175 characterized-not-classified (black 26 / httpx 86 / mypy 63); the 25/corpus sample said mostly prism_fp candidate-tier + prism_fn recall; bulk-adjudicate only if a Python initiative needs the denominator | S (codex batch) |
| 10 | ~~**Parameter-slot fail-closed alignment**~~ **DONE (#173, 2026-08-22)** — `src/parameter_slots.rs` prefix semantics; Go grouped names; positional consumers on slots; `prism nav call-stats --dump-sites`. **Level-3 callback minting DISABLED** (4 review rounds could not make the bare-name resolver sound: shadowing by params/locals/arrow/generator params, conditional assignments, unresolved imports) → see item 13 | sol spec review 2026-08-21 (`docs/superpowers/specs/2026-08-21-p8-p9-p10-designs-and-spec-review-sol.md`) | M |
| 11 | ~~**prism-mcp lazy handshake**~~ **DONE (#178, 2026-08-22)** — approach A+C: the index build runs on a background thread from server startup; `initialize`/`ping`/`tools/list` answer immediately; a `tools/call` arriving before readiness waits at most `--first-call-wait` (default 20 s, max 600, cumulative per build attempt) and then returns a structured, retryable `index warming` result (`isError: true`, status JSON in text + `structuredContent`, `prism/index_state`, `prism/retryable`); build failures reported and retried on the next valid call; `--eager` keeps the legacy synchronous startup (pre-warm recipe); Tier-C `warm_gate_check` uses `--eager`. Spec `docs/superpowers/specs/2026-08-22-prism-mcp-lazy-handshake-design.md` (3 sol spec rounds, cap); terra impl + 1 fix wave; sol diff review r1 FIX (tests) → r2. Live codex 0.147 probes: control (main eager, cold TypeScript) 0 prism tools; lazy (cold TypeScript) 6 tools exposed, warming result, retry; lazy (cold prometheus) warming → real data. Follow-ups: none required; optional first-call latency probe in the Tier-C gate | `docs/analysis/2026-08-21-tier-c-partd-readout.md` §Caveats | S–M |
| 12 | **Java/TS parameter materialization (A2)** — PARKED design with sol spec review (canonical slot model, rest/variadic/spread semantics, Level-3 identity, go.mod/sidecar cache); prerequisite for Java/TS interprocedural taint | `2026-08-21-java-ts-parameter-materialization-design-PARKED.md` | L |
| 13 | **Sound Level-3 callback resolution** (NEW 2026-08-22; **rescoped 2026-08-22 [OWNER]: Go-first — typed `func` parameters only; JS/TS deferred** — Level-3 covers parameter-carried function values invoked inside an in-repo callee; modern JS/TS callback use mostly flows into library code prism does not index, while Go in-repo HOFs (functional options, `Walk(fn)`, handler wrappers, worker pools) are the real population and Go's typed params make the value resolver sound without arrow/generator scoping; **gated on a measured Go case**) — re-enable indirect callback edges only with a binding-aware VALUE resolver (params/locals/closure/generator scopes shadow repo functions by byte position; imports resolve to exactly one Exact in-repo FunctionId or None; no assignment fallback without dominance; exact containing FunctionId + singleton-Exact inbound site; carried `pre_resolved_target` kept in every site-identity tuple incl. navigation). Record: PR #173 body + the four review rounds in `docs/superpowers/specs/` (P8) | main today mints NO Level-3 edges (precision-max); prior false-Exact cases are the negative fixtures already in `tests/integration/call_graph_test.rs` | M |
| 14 | **Nested-module Go import identity** (owner: full scope in four slices, 2026-08-22) — **slice 1 oracle hardening DONE (#180)**: `interface-manifest` package-qualified implementer identities + target spans; `dispatch_oracle.py` identity-aware `(dir, clause, type)` compare, concrete-receiver ground truth = definition target, C/N/U partition for `implementation` results, zero-fanout scoring (`recall_gap` / `not_dispatch`), `--baseline` delta mode with `gate_ok` = no delta blocker (over_approx / timeout / unresolved / target_mismatch among newly-Exact sites) ∧ fanout-positive site and edge coverage ≥ 0.90 (tool contract per `eval/README.md` §Dispatch oracle and PR #180; spec §2 updated to match), pinned `go env`, Unicode-safe clause lexer; identity-aware baselines on main's resolution (fanout>0 precision): caddy 1.0000, prometheus 0.94, etcd 0.989, hugo 0.992; **slice 2 hygiene DONE (#179)**: Go-only `testdata` exclusion, go.mod tokenizer + `module` grammar + `CheckPath`, symlink-safe immutable manifest snapshot (`symlink_refused` sentinel), go.work hashed, CPG 45 / sidecar 14 / skip policy 2; **slice 3 effective module identity DONE (#182, 2026-08-23)**: `src/go_module_graph.rs` (+`replacements.rs`, `semver.rs`, `identity.rs`) computes the ACTIVE module set and effective import paths from the snapshot (go.work `use`; replace precedence — workspace wins by PATH, go.work replaces first, union, RHS conflicts → `workspace_invalid`; wildcard replace gated on an active-main `require`; version-specific → unproven; active-main self-replace inert; main-module path `CheckImportPath` vs dependency `CheckPath` [owner: Go-faithful split]; x/mod-faithful semver for `require`/`exclude`/`retract`/replace; whole-workspace vs subtree fail-closed layering; snapshot-only, ≤1 parse per file, memoized identities, conservation telemetry `go_import_path_{proven,unproven}_files` + `go_module_graph/*`); CPG 46 / sidecar 15; same-base control: caddy/prometheus/hugo resolution leaves unchanged (prometheus 5/5 modules active), etcd interface_dispatch 1742→2002 (13/13 modules active), ripgrep byte-identical; hardened oracle delta: caddy/prometheus/hugo gate TRUE (0 newly-Exact), etcd 375 newly-Exact = 370 sound + 5 over_approx — the 5 are roadmap-#17 concrete-receiver class (owner exception 2026-08-22: merge, #17 next must turn them to 0 against `oracle-s3b-etcd.json`); reviews: terra r1 FIX / r2 FIX / r3 APPROVE (4 WRONGs fixed over 3 fix waves), Ox ∥ APPROVE ×3; process lesson: implementer commits are clone-local — controller must push after every wave; **slice 4 DONE (#184, 2026-08-23)**: alias-aware `Local↔Local` by path (profile/clause-scoped alias index, whole-RHS canonical expansion incl. parameterized aliases, fail-closed `AliasUnresolved`, shadow-aware predeclared normalization) + the owner/profile-keyed promoted-selector snapshot FOUNDATION (five profile-safety axes incl. receiver method-set shape; shallowest-selector rule; signature-level embedded-interface profile comparison; not consumed by routing); CPG 48 / sidecar 17; same-base vs pre-#17 main: prometheus interface_dispatch +37, etcd +60 (alias expansions 25/33/8), 0 recall transitions, oracle gates TRUE; re-verified after the #186 merge (final head 0028af1, alias_resolver threaded through #17's record_interface_type): suite 3409/0/1, tier-a 104/104, vs post-#17 main prometheus +164 / etcd +70, oracle vs p17e baselines all four gates TRUE (newly-exact 0/41/38/0, coverage >=0.9649); implemented TWICE independently (sol = merged; Ox Alpha Free = calibration branch `go-alias-aware-local-local-ox`, converged site-for-site to sol's output after one review-driven fix wave) — the parallel run surfaced two Part-B defects on the merge candidate (shallowest-selector pruning; name-only interface profiles) that the serial review loop had approved past; spec §5 clarifications recorded — spec `docs/superpowers/specs/2026-08-22-go-nested-module-import-identity-design.md` §5 | PR #176 body; PR #182 body + `docs/superpowers/handoffs/2026-08-22-go-effective-module-identity-handoff.md` (slice 3); `tests/lang/go/owner_partition_fix_wave_test.rs` nested-module fixtures | S–M |
| 15 | ~~**Go provider build-time perf SMELLs**~~ **(a) DONE (#185, 2026-08-23)** — one plain `GoTypeProvider` shared by embedding promotion, receiver rematerialization AND the `CpgContext` type registry (interface dispatch keeps its import-path-aware provider — canonical signature identity differs); dispatch reordered so peak live providers = 1; provider transferred (not retained) into the registry; incremental-rebuild path covered; test-only construction/live counters; gate: 5 corpora + 4 manifests RAW-BYTE identical, suite/tier-a = main; timing: one construction (~7–9 s on etcd/prometheus, ≈15–20 % of build wall) removed. Implemented by the Ox Alpha Free lane (first implementation trial; 2 review-driven fix waves). **(b) DEAD** — #14 slice 3 made the module graph snapshot-only (no per-file go.mod disk reads remain) | #174 / #176 review records; #185 body | S |
| 16 | **Package-qualified INTERFACE identity in the Go dispatch table** (NEW 2026-08-22, precision; surfaced by the identity-aware oracle) — the interface table is keyed by bare interface NAME, so `storage.Iterator` / `chunkenc.Iterator` conflate: prometheus `Iterator.Next` (storage/buffer.go:64) minted `web/api/testhelpers.FakeChunkSeriesIterator` while gopls lists `promql.*Iterator`; `WriteClient.Name` minted `documentation/examples/.../graphite.Client`; caddy `metrics.go:56` `next.ServeHTTP` on a stdlib `http.Handler` minted `caddyhttp.HandlerFunc` (caddy's own `Handler` interface, whose `ServeHTTP` returns `error`, conflated with `net/http.Handler`). Fix = key interfaces by `(package_dir, clause, name)` like P10 did for owners/signatures; gate with the slice-1 oracle delta mode | slice-1 oracle-s1bbase-prometheus.json over_approx list (23 sites: 6 interface-kind, 17 concrete-kind) | M |
| 17 | **Concrete-receiver call sites must not enter interface dispatch / multi-target on concrete receivers** (NEW 2026-08-22, precision) — the interface manifest predicate admits any call whose method name occurs on some interface (`src/navigation/queries.rs` ~L610; caddy `adapter.Adapt` on `caddyfile.Adapter`), and concrete-receiver sites can mint multiple targets (prometheus `storage.Close` → `tsdb/agent.DB` + `storage/remote.Storage`); the oracle now classifies these (`definition_kind: concrete`) so the delta is measurable. **DONE (#186, 2026-08-23; design v8 on the PR — R1(b) wording corrected after a tier-a regression)**: one shared `go_concrete_receiver_route` (resolver + manifest) + a P10-keyed declaration-kind index (canonical alias targets, `type D I`, synthetic unnamed-interface aliases) serialized on `CallGraph` (cache 47/16, four-path cache parity); R1(a) own-method direct with on-demand owners (typed_param Exact ×7–18 per corpus; NameOnly typed_param eliminated), R1(b) existing promotion lane unchanged + deferred-drop, R1(c) depth-aware embedded supply, R1(d) named func-value fields → P5, R1(e) terminal drop; on-demand R2 with a bare-name-collision bail (v9-A; killed the 16 prometheus Iterator false edges — the first measured #16 mitigation); external new-recovery drop (v9-B; killed 4 io.Closer false edges); fail-closed on local type shadows, value rebindings (type-switch/range/closure/`:=` declarations), profile-conflicting owners; all 14 etcd + 20 prometheus concrete over_approx oracle sites resolved; oracle gates TRUE everywhere except one documented hugo env-gap waiver (tocss.go:122, `//go:build extended` vs empty oracle tag set → follow-up: oracle tag-set coverage); direct-lane gopls audit (new controller tool) 199/0/1 over 5008 new direct sites; R3 legacy projection byte-identical per site; #17b population telemetry shipped (`go_unproven_receiver_bare_fallback_{sites,hits,edges}`). Follow-ups: qualified embedded interfaces resolve nowhere on main or branch (label-only relabel; with #16); wave-5 externality proof under partially-proven module graphs (Ox); oracle tag-set coverage. **Design v7 = design-of-record (merged #183)** `docs/superpowers/specs/2026-08-22-p17-narrow-concrete-receiver-direct-design.md`: routing decomposition for PROVEN concrete-recovered receivers — R1(a) own method → direct Exact (on-demand owner), R1(c) embedded-interface → S4 as today, R1(d) func-valued field → P5, R1(e) no selector → terminal drop (`ConcreteReceiverNoSelector`); R1(b) promoted-from-embedded-concrete direct routing DEFERRED to #14 slice 4 by owner decision after four scoped confirmations each found a new profile-safety axis (package qualifier, ordinary fields, own methods, embedded-alias selector names) — such sites take `concrete_promoted_deferred_drop` (fail-closed); R2 proven interface → S4; R3 unproven/external UNCHANGED (`Ambiguous(profile conflict)` = owner with >1 declaring file ⇒ R3); declaration-kind index keyed by P10 identity with canonical alias targets + `type D I`, serialized on `CallGraph` (cache 46→47 / 15→16, cache-parity acceptance); one shared consult fn (resolver + manifest); `dispatch_route` diagnostic; route-specific subtraction audit; baselines s1e (caddy/prometheus/hugo) + `oracle-s3b-etcd.json` (etcd: s1e's 11 + slice 3's 5 → 0). Review record: sol r1–r4 + 2 scoped confirms, Ox ∥ each (calibration in lane ledger). **#17b (NEW, carved out, owner 2026-08-22: separate, measured first)** — today's R3 bare `iface_key` ladder is NOT fail-closed for EXTERNAL receivers (external concrete `q.A{}` + in-repo `p.A{M()}` → false Exact; `http.Handler` + in-repo `Handler` → wrong implementer set); #17-narrow adds `go_unproven_receiver_bare_fallback_{sites,hits,edges}` so #17b's per-corpus population is counted BEFORE its design; terminal-R3 changes interface_dispatch on every corpus and drops unverified external-interface recall — own same-base control + oracle delta | sol slice-1 review SMELL 7; oracle `definition_kind` histograms; `oracle-s3b-etcd.json` | S–M |
| 18 | ~~**Gate the empty-live RTA fallback on proven identity**~~ **CLOSED AS SUPERSEDED (2026-08-31)** — the 2026-08-22 ledger recorded a pre-owner-provenance hazard: loss of interface identity plus `NonLocalConstructionFallback` minted 131 newly-Exact Prometheus sites, including 14 over-approx sites. Current production resolution no longer emits the legacy table's candidates. Every resolver read passes through `go_visible_s4_implementers`, which ignores the bare-table candidates and recomputes satisfiers from a full `GoOwnerIdentity`, exact declaration provenance, caller build visibility, structural signatures, and then live selection; the terminal #16 route uses `go_proven_interface_outcome` directly. The manifest mirrors those same owner-qualified consults, and the only other read is a call-stats fanout histogram. Existing direct-interface and S4 regressions prove that a live same-name decoy is not reintroduced while the correct non-live implementer survives the owner-qualified fallback. The legacy provider table and its NLCF counter deliberately remain schema-compatible build telemetry (current retained counts: Caddy 3, etcd 578, Hugo 330, Prometheus 146), not edge authority; post-#16 oracle reports pass all four Go corpora with zero blockers/newly-Exact sites. No runtime, cache, or schema change is required. Evidence: `docs/superpowers/handoffs/2026-08-31-rta-empty-live-fallback-disposition.md`. | ✅ |

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
