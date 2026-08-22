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
| 11 | **prism-mcp lazy handshake** (NEW 2026-08-21, adoption-critical) — **IN FLIGHT 2026-08-22 (approach A+C: background bootstrap at startup, instant handshake, cumulative `--first-call-wait` then a structured `index warming` result, `--eager` for the legacy path; spec `docs/superpowers/specs/2026-08-22-prism-mcp-lazy-handshake-design.md`, 3 sol spec rounds; terra implementing)** — answer `initialize`/`tools/list` immediately and load/build the index on first tool call; codex 0.147 silently drops servers not ready in ~10 s regardless of `startup_timeout_sec` (proven by probe; voided the first Part-D slate; TS cells unmeasurable with a 17–19 s warm load). Claude Code has short MCP startup limits too. | `docs/analysis/2026-08-21-tier-c-partd-readout.md` §Caveats | S–M |
| 12 | **Java/TS parameter materialization (A2)** — PARKED design with sol spec review (canonical slot model, rest/variadic/spread semantics, Level-3 identity, go.mod/sidecar cache); prerequisite for Java/TS interprocedural taint | `2026-08-21-java-ts-parameter-materialization-design-PARKED.md` | L |
| 13 | **Sound Level-3 callback resolution** (NEW 2026-08-22; **rescoped 2026-08-22 [OWNER]: Go-first — typed `func` parameters only; JS/TS deferred** — Level-3 covers parameter-carried function values invoked inside an in-repo callee; modern JS/TS callback use mostly flows into library code prism does not index, while Go in-repo HOFs (functional options, `Walk(fn)`, handler wrappers, worker pools) are the real population and Go's typed params make the value resolver sound without arrow/generator scoping; **gated on a measured Go case**) — re-enable indirect callback edges only with a binding-aware VALUE resolver (params/locals/closure/generator scopes shadow repo functions by byte position; imports resolve to exactly one Exact in-repo FunctionId or None; no assignment fallback without dominance; exact containing FunctionId + singleton-Exact inbound site; carried `pre_resolved_target` kept in every site-identity tuple incl. navigation). Record: PR #173 body + the four review rounds in `docs/superpowers/specs/` (P8) | main today mints NO Level-3 edges (precision-max); prior false-Exact cases are the negative fixtures already in `tests/integration/call_graph_test.rs` | M |
| 14 | **Nested-module Go import identity** (NEW 2026-08-22, recall follow-up to #3) — `CallGraph::go_package_import_paths` proves a file's import path from the ROOT `go.mod` only; packages below a nested `go.mod` (etcd: api/, client/, server/, pkg/…; prometheus has 5 modules) get no proven path, so their bare signature leaves stay `Bare` and `Bare↔Qualified`/`Bare↔Local` fail closed (etcd interface_dispatch Exact −46 vs main). Sol measured **+214** candidate etcd Exact edges when nested modules were resolved (etcd 2,002) and refused them pending a per-site audit (possible mass fan-out). Also: `Local↔Local` leaves with DIFFERENT proven paths still match by name (retained bare rule — disproof available but unused), and unaliased import names come from the path tail, not the declared clause (`x/go-bar` used as `bar` → `QualifiedTypeIdentity`, recall only). Design: nearest-`go.mod` module path + module-aware `Local` tokens, audit the +214 with the partition custody dump, then decide whether `Local↔Local` by-path is safe | PR #176 body; `tests/lang/go/owner_partition_fix_wave_test.rs` nested-module fixtures | S–M |
| 15 | **Go provider build-time perf SMELLs** (NEW 2026-08-22, low) — (a) receiver rematerialization constructs a second `GoTypeProvider` to obtain P9 field targets (`src/call_graph.rs` ~L3085, #174 terra r7); (b) `go_package_import_paths` walks up and reads `go.mod` from disk once per Go FILE (memoizable per directory/module root; `go.mod` IS in `repo_loader` topology hashing so caches invalidate correctly). Deterministic and correct today; benchmark on etcd/prometheus before touching | #174 / #176 review records | S |

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
