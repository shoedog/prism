# Prism — LLM leverage & accuracy improvement plan

Date: 2026-07-02. Grounded in code at main `fae6240`; probe diff = commit `36429d7`.

> **Status 2026-07-03 (round 3):** P7 shipped as PR #155 (Python `@property`/`@cached_property` access edges — **nav-only** `property_access` NameOnly edges from a dedicated serialized table, mirroring P5's `go_registrations` plumbing; httpx `response.text` 0→57 callers @0.6 incl. the adjudicated `_main.py:181` site; 1361 recorded / 54 store-skips) and P4 as PR #156 (typed JS/TS export facts — default/renames/const-arrow/CJS/depth-2 barrels — through R4c `import_member`; excalidraw `kind_exact.import_member` 104→2789 with the `multi_target_exact_sites` canary flat at 188, largely `free_single`→`import_member` reclassification). Review-driven tightening worth knowing: P7 tier-1 requires a genuine instance method (first **positional** `self` + `method_owners` entry; nested/lambda scopes fenced; `del`/`for`/`with` targets excluded); P4 is fail-closed on mutable destructured `require` (const-only), spread-bearing `module.exports` literals, duplicate exported names, and non-function-valued `export const` initializers. **Doctrine ruling (codex):** barrel-resolved single-target `import_member` remains grounded → Exact is correct; multi-target NameOnly `import_member` is the pre-existing legacy class, only marginally widened — no new exclusions. **Durable trap:** `scoped_caller_site_match_count` (`src/navigation/call_resolve.rs`) independently re-derives R4c name-correlation — any R4c change must update it too (it silently zeroed nav multiplicity for default/renamed exports until the end-to-end tier-a gate caught it). Cache: CPG 34 (P7) → 35 (P4), sidecar 4 → 5.
>
> **Status 2026-07-03:** P6(a) shipped as PR #151 (confidence-stratified M2: `exact_tier`/`candidate_tier`, score through replay, live fail-fast on missing score), P3 as PR #153 (`R6MultiOwnerCandidate`: capped ≤3-target NameOnly candidates for Py/JS/TS/TSX unknown receivers; black `dropped_multi_owner` 230→30, exact-tier byte-identical), P5 as PR #154 (Go `callback_registration` + `func_value_field`; cobra `emptyRun` 0→288 labeled callers). **Doctrine addition to §3a, learned from two codex blockers:** uncertainty tiers need consumer-visibility tiers — name-coincidence candidates (P3) are **nav-only** (excluded from Step-5b DataFlow/taint and echo/membrane findings); registration-grounded candidates (P5) reach non-nav consumers **only at exactly one registered target**. Nothing below Exact ever feeds an asserted finding. Also fixed en route: PR #152 (CaseResult test drift from #144).
>
> **Status 2026-07-02:** P1 shipped as PR #149 (probe 13.49 MB → 552 KB; residual = diagram payloads) and P2 as PR #150 — both through codex-xhigh spec review → Sonnet implementation → task review → codex-xhigh implementation review → fix wave → SHIP. Corrections vs the specs below, found in review: severity vocabulary is `info < suggestion < warning < concern` (PrimitiveSlice emits `suggestion`); `--format json` shares the review serializer and is byte-pinned, so compaction is a review-only path (`src/output/review_compact.rs`); source findings are licensed by emitted sink findings' chosen source **plus** all `sink_to_path_sources` entries and bash unquoted-expansion sinks; ground truth is 11 languages / **12** tree-sitter grammar variants (TSX); `refresh_index` is not read-only, so blanket read-only claims were scoped, and the Simple/Graph-based doc split was corrected to `needs_cpg()` (16 CPG / 14 AST-only). Framework target-scoped seeds still emit no `taint_source` finding (pre-existing; belongs to P9).

## 1. Goal restatement

- **LLM-leverageable** = every tool result is compact, `file:line`-anchored, confidence-labeled resolved facts an agent can act on without re-derivation; procedure lives in the skills, not the payload.
- **Precise** = an `Exact` edge is always true; uncertainty is labeled (`NameOnly`/heuristic + score), never asserted.
- **Recall-complete** = a real edge is either present, or its absence is a *visible, bounded* candidate/warning — never a silent drop — and each closed gap is scored by Tier-A/adoption/Tier-C.

## 2. Precision/recall definitions per surface

| Surface | Precision (for the agent) | Recall (for the agent) | Scored by today |
|---|---|---|---|
| `nav_callers`/`nav_callees`/`nav_nodes_at`/`nav_ego_graph` | every returned edge is a true call/def edge; `score 1.0` items are safe to act on unverified | no true in-repo caller/callee missing (complete blast radius) | Tier-A M1/M2 corrected P/R per stratum (`eval/tier_a/cli.py:192-284`), M3 spot-check, by-construction matrix (`eval/tier_a/matrix.py:71-122`); adoption ToolCorrectness (`eval/adoption/metrics.py:32`) |
| `nav_module_deps`/`nav_repo_map` | listed dep edges are real imports | no missing module edge | **gap** — only 2 pinned probes (`eval/tier_a/pinned.py:13-18`); no module-edge P/R |
| `taint_reaches` | `Reached` only when a real flow path exists, sanitizer-aware | real source→sink flows found across function boundaries | **none** — Rust shape tests only (`tests/reasoning/taint_reaches_test.rs`); no oracle |
| slice (CLI, one algorithm) | surfaced lines are change-relevant (high signal density) | defect-relevant lines included | **none** — nearest: Tier-C citation oracle (`eval/tier_c/investigator.py:35-63`) |
| `--format review` findings | findings survive agent verification (low false-alarm rate) | real defects in the change surfaced | **none** — unmeasured; 90% of probe findings are `info` noise (measured below) |
| Skills / context docs | instructions match binary behavior | procedures cover the failure modes (score interpretation, verification triggers) | adoption SkillActivation + pass^5 gate (`eval/adoption/aggregate.py:10-24`); Tier-C SpecQualityJudge (`eval/tier_c/judges_live.py:79-110`) |

### Seeded-map drift (verified)

- MCP registry serves **8 tools** — 6 nav + `taint_reaches` + `refresh_index` (`src/mcp/registry.rs:102-107`). `resolved_import`/`unresolved_import`/`type_db` are **not** tools; `callee_definition` is a `next_query` reason label (`src/mcp/evidence_view.rs:844-847`).
- Algorithm count ground truth = **30** (`src/slice.rs:47-119`). README says 27 at `README.md:41,66` and 30 at `:106,:144`; `LLM.md:5` and `CLAUDE.md:5` say 26.
- The Rust trait-dispatch spike doc referenced in prior notes is **not in the tree** (branch-only); its NO-GO verdict is reconstructed from the adjudication ledger.
- `eval/snapshots/*.json` are frozen oracle **symbol inventories**, not P/R records; accuracy divergence lives in `eval/adjudications.jsonl` (997 `prism_fp` / 469 `prism_fn` / 133 `ambiguous` / 42 `oracle_miss` to date).

## 3. Ordered improvement plan

Ranked by value-per-effort. Format: **what · why · seam · metric · effort · confidence**.

**P1 — Collapse the `--format review` firehose.**
Why: measured on one modest commit (`36429d7`), `--algorithm review --format review` emits **13.5 MB / 2,432 findings, 2,198 = `info`**; 1,889 come from taint, because `taint_from_diff` (default when no `--taint-source`, `src/main.rs:1109`) seeds **every diff line of every file** — including unparsed files like `Cargo.toml` (`src/algorithms/taint.rs:10842-10848`) — and each seed unconditionally emits an `info` "taint source" finding (`taint.rs:11165-11180`). The `results` array (9.6 MB) triple-encodes each block: `slice_text` + `slice_lines` + `diff_lines` (`src/output/review.rs:33,36,129`), with **no byte cap** (`review.rs:154-171`), unlike the MCP path's 80 KB cap (`src/mcp/output.rs:11`). Violates every §0 criterion at once.
Seam: `taint.rs:10842` (skip seeds for files absent from `ctx.files`), `taint.rs:11165` (emit source findings only when the source participates in a path), `review.rs:154-171` (findings-first default, severity floor, slices opt-in).
Metric: none exists (that's P6) — validate now by output-byte count on pinned probe diffs + a CLI test pinning "no `taint_source` finding in unparsed file"; predicted Tier-C review-arm citation-precision gain (extrapolated).
Effort: **S–M**. Confidence: **high** (measured). Spec §4.1.

**P2 — Docs/skills truth pass (trims + fact fixes, no new prose).**
Why: active misdirection, all verified: `CLAUDE.md` says 26 algorithms in 5 places (`:5,:17,:71,:74,:277`), omits the 4 newest from its maps (`:117-151,:225-226`), cites dead `src/output.rs` (`:72`), says "six tools" (`:208`). `LLM.md` claims **5 languages (real: 11)** (`LLM.md:65-67`), 26 algorithms, dead paths, plus 40 lines of test-repo narrative — a §0 liability; its useful content duplicates `ALGORITHMS.md`. `README.md:41,66` say 27. `docs/MCP.md:144` and `skills/prism-code-navigation/SKILL.md:75` claim "~200 nodes" truncation — real caps are 50 default / 1000 max items (`src/mcp/input.rs:9-10`) and 80 KB bytes (`output.rs:11`). Nav skill's core gotcha "resolution is name-based, not type-based" (`SKILL.md:67-70`) predates receiver typing/interface dispatch and — worse — never teaches the *actual* signal: `score 1.0 = Exact`, `0.6 = NameOnly` (`src/navigation/module_graph.rs:31-35`), or what a collision warning means (`src/navigation/queries.rs:457-458`).
Seam: the files above; **delete LLM.md** (fold its one unique asset — the prompt-structure snippet — into `ALGORITHMS.md`), don't rewrite it.
Metric: adoption SkillActivation/ToolCorrectness pass^5 must not regress on skill edits; doc fixes are unscoreable — justified as removing measured falsehoods at near-zero cost.
Effort: **S**. Confidence: **high**. Spec §4.2.

**P3 — Emit capped candidate edges instead of silent drop for Python/JS/TS unknown-receiver calls.**
Why: the doctrine centerpiece (§3a below). Today an unknown-receiver `x.m()` with owners on >1 class is **dropped** (`src/resolution.rs:1497-1507` → `DropReason::MultiOwnerCollision`); `nav_callers` then returns **zero items** plus a count-only warning naming no sites (`src/navigation/queries.rs:457-458,739-744`) — the recorded "Python returned 0 callers" incident. This is correct precision doctrine for Rust (fixture-pinned: `eval/fixtures/rust/r6_multi_owner_drop/expected.toml`) but is the direct driver of Python's 54–65% unresolved rate. A labeled maybe-edge beats a silent zero for an agent.
Seam: `resolution.rs:1497-1507` — language-gated (Py/JS/TS only): owners ≤ 3 → `demoted(...)` with a new `ResolutionKind::R6MultiOwnerCandidate` (NameOnly, score 0.6, surfaces as `fallback` trust in `evidence_view.rs:1341-1364`); owners > 3 → keep drop but extend the warning to name up to 5 dropped sites.
Metric: Tier-A M2 Python callers recall (U-method stratum) ↑; `call-stats dropped_multi_owner` ↓ on fastapi/pydantic; matrix pins Rust unchanged. **Requires the P6 confidence-stratified M2 report** — otherwise the oracle diff counts labeled candidates as plain FPs and punishes the doctrine.
Effort: **M**. Confidence: **high** (incident + adjudications); recall payoff size is extrapolated. Spec §4.3.

**P4 — Model JS/TS exports: default / `export {}` lists / const-arrow / CJS / barrels.**
Why: primary driver of JS ~92% unresolved. The code admits it: `extract_js_ts_exported_functions` "deliberately accepts only `export function name(...)`; default exports, re-export lists, CommonJS exports, and exported const-arrow functions need separate modeling" (`src/ast.rs:1343-1346`); unmatched imports fall to `dropped(UnknownName)` via the import-member path (`src/resolution.rs:1509-1535`). Tier-A fixtures for exactly these forms already exist and pin the *current gap* as an executable contract — e.g. `eval/fixtures/typescript/default_export_function_deferred/expected.toml` asserts `callers = []` + `forbid_resolution_kind = "import_member"`; same pattern in `eval/fixtures/javascript/commonjs_export_deferred/` and `typescript/{arrow_const_export,default_import}_deferred/`. The done-check is rewriting each `[expect]` block to the positive contract (caller attributed, `expected_resolution_kind = "import_member"`).
Seam: `ast.rs:1347` + `resolution.rs:1509-1535`; bound barrel-chain depth (mirror `MAX_GLOB_DEPTH`, `src/name_resolution/engine.rs:107`); dynamic `require(expr)` stays out (doctrine).
Metric: deferred fixtures flip to `ok` in `--matrix-only`; `call-stats` unresolved% on excalidraw/express; JS/TS Tier-A coverage from #144.
Effort: **M**. Confidence: **high**.

**P5 — Go func-value callbacks (registration + invocation).**
Why: the top adjudicated Go FN class — cobra 7× `prism_fn` ("`Run: emptyRun` is a callback-value assignment"), zap (`EncodeCaller: ShortCallerEncoder`, `config.go:214`), caddy (`RegisterHandlerDirective(parseCaddyfile)`). Prism has no func-value model: the func-pointer-field fallback is **gated to C only** (`src/resolution.rs:1449-1458`; Go falls to `dropped(UnknownName)` at `:1480`); struct fields keep only `(name, type_string)` with no field→FunctionId binding (`src/type_providers/go.rs:483`); `callback_dispatcher_slice.rs` is C/C++-only (`:75,:313,:458`) and CLI-only — never reaches nav/MCP.
Seam: widen `resolution.rs:1458` to Go when the receiver's field type is `func(...)`; add a func-value-field index beside `apply_go_embedding_promotion` in `src/call_graph.rs`; registration side emits a NameOnly "assigned-as-callback" caller edge.
Metric: Tier-A M2 Go (cobra/zap/caddy strata); new matrix fixture `go/func_value_field`; the cobra/zap adjudicated sites flip.
Effort: **M**. Confidence: **high**.

**P6 — Measurement extensions: make the unmeasured surfaces scorable.**
Why: `--format review`, `taint_reaches`, and module-graph accuracy have **no metric** (§2), and P3's doctrine needs confidence-aware scoring. Three additions, all on existing machinery: (a) **confidence-stratified M2** — report P/R separately for Exact vs NameOnly/candidate edges (`eval/tier_a/cli.py:192-284`; per-site `resolution_kind` metadata already flows at `cli.py:350-365`); (b) **by-construction taint fixtures** (`expected.toml` pattern of `eval/tier_a/matrix.py`) asserting `taint_reaches` verdicts incl. sanitizer cuts; (c) **module-edge contracts** in `eval/tier_a/pinned.py:13-18`; optionally (d) a Tier-C review-format arm feeding `--format review` citations through `score_citations` (`eval/tier_c/investigator.py:35-63`).
Metric: self — these *are* the metrics; gate = new checks run in `--matrix-only` seconds-fast path.
Effort: **S–M**. Confidence: **medium** (harness-design judgment, not oracle-derived).

**P7 — Python `@property` access edges.**
Why: adjudicated `prism_fn` class — resolution is call-syntax-only, so an attribute access that fires a getter mints no edge: httpx `response.text` (`seed httpx/_models.py:642`), flask `max_cookie_size` (`src/flask/wrappers.py:247`), black `prev_sibling`. The durable taxonomy rule already classifies these as prism recall gaps, not oracle artifacts.
Seam: a synthetic attribute-access→getter edge path beside the call collector (`src/ast.rs:4615` `collect_calls_manual` funnels through `is_call_node`, `src/languages/mod.rs:602`); emit **NameOnly** with a distinct kind (`property_access`) to protect precision.
Metric: the adjudicated httpx/flask sites flip `prism_fn`→tp on Python M2; new python matrix fixture with `expected_resolution_kind = "property_access"`.
Effort: **M**. Confidence: **high**.

**P8 — Rust macro-argument call extraction.**
Why: calls inside `assert!(…)`, `vec![…]`, builder macros are invisible — `is_call_node()` lacks `macro_invocation` (`src/languages/mod.rs:602-615`), so they never become `CallSite`s; adjudicated `prism_fn` (e.g. `src/algorithms/primitive_slice.rs:837` `assert!(detect_weak_hash_identity(...))`, tokio `entry.rs:121`). No telemetry bucket, no fixture. The deferred design already exists with seams in place (`docs/superpowers/specs/2026-06-17-prism-macro-resolution-deferred.md`: `CallSite.kind: MacroInvocation`, `NS_MACRO`, poison-not-skip invariant).
Seam: `languages/mod.rs:602` + a macro-arg extractor descending `token_tree`; scope = calls in macro **arguments** only (derive-generated bodies stay out — adjudicated `oracle_artifact`).
Metric: new rust matrix fixture `macro_arg_call`; adjudicated sites flip; note Tier-A oracle blind spot on macros (`docs/eval/tier-a/baseline.md:58`) — matrix is the primary gate.
Effort: **M**. Confidence: **high**.

**P9 — Framework routes as entrypoint edges (Python + JS/TS).**
Why: framework decorators/registrations are the *real* call edges in dynamic code, but today they only mark taint sources: Python `@app.route` detection feeds `taint.rs:4369-4395` and all framework `SPEC`s have empty sources/sinks (`src/frameworks/python/fastapi.rs:10-16`); express handlers likewise (`src/frameworks/js_ts/express.rs:5-24`, `taint.rs:1952-1985`). `nav_callers` on any handler returns 0 by design — an agent doing blast-radius on a route handler gets a false "no callers".
Seam: the framework layer mints a synthetic `framework_entry` caller edge (NameOnly) from the registration site to the handler.
Metric: no LSP oracle will confirm these (pyright shows 0 callers too) — gate with by-construction fixtures (`eval/fixtures/python/route_handler_entry/`) + the adoption realistic probes; flag: agent-value is extrapolated, the smallest A/B is a Tier-C issue on a route-handler bug.
Effort: **M**. Confidence: **medium**.

**P10 — Honest taint output: sanitizer cuts and boundaries as facts.**
Why: in `taint_reaches`, a sanitizer in the source function emits only a `Cleansed` **warning** while `reachability` stays `Reached` (`src/reasoning/taint_reaches.rs:184-198`); path-cutting logic exists but only in the CWE finding engine (`taint.rs:5979-6062`), never surfaced as a step. No confidence field exists anywhere in reasoning output (`src/reasoning/types.rs:91-99,164-181`). Per §0: the output should carry the discriminating fact (`sanitized_by: html.escape at file:line`) — not leave the agent to reconcile a `Reached` verdict with a vague warning.
Seam: `taint_reaches.rs:184-198` (verdict-affecting sanitizer evaluation reusing `FlowPath.cleansed_for`, `src/data_flow.rs:95-102`); add sanitizer site as a witness-graph node.
Metric: P6(b) taint fixtures (sanitized fixture expects the downgraded verdict + the sanitizer step).
Effort: **S–M**. Confidence: **medium** (design judgment; fixture A/B settles it).

**P11 — Go receiver typing: field/return-typed receivers + embedded-interface promotion.**
Why: field/return receiver typing is **Rust-only** (`src/resolution_receiver.rs:84` early-returns non-Rust; indices populated only by `extract_rust_field_types`/`extract_rust_return_types`, `src/call_graph.rs:2590-2591`), so Go `x := getServer(); x.M()` and `s.server.M()` drop — adjudicated etcd `prism_fn` (`cache/demux_test.go:358/389`). Embedded interface in a struct is explicitly deferred (`src/type_providers/go.rs:1396-1398` `continue`) — adjudicated caddy `ambiguous` (`httpredirectlistener.go:78`). Package-level `var` receivers invisible (`src/ast.rs:432` scans the enclosing function only). Cross-package asserted keys documented at `resolution.rs:469-470,487-489`.
Seam: Go arm of the receiver typer at `resolution_receiver.rs:84` + Go field/return index at `call_graph.rs:2590`; `go.rs:1396` bridges embedded interfaces to `apply_go_interface_dispatch` (`call_graph.rs:2166`).
Metric: etcd/caddy M2 strata; also add a Go-specific drop-attribution bucket — today every Go receiver miss lands unattributed in `dropped_external_receiver`/`dropped_multi_owner` (`src/navigation/queries.rs:278-281`, classifier is Rust-keyed at `queries.rs:102`).
Effort: **M–L**. Confidence: **high** (adjudicated) / medium on size of buy.

**P12 — Payload trims (token cost, no information loss).**
Why, three measured redundancies: (a) `SNAPSHOT_NOTICE` (~340 ch) + `VIEW_NOTICE` (~400 ch) are appended to **every** tool description (`src/mcp/tools.rs:13-14,70`; repeated in `tools_reasoning.rs:9,30`) — ~4.5 KB of identical text in every session's tools/list; state each once in server instructions. (b) Every result ships twice: identical JSON in the `content` text block and `structuredContent` (`src/mcp/output.rs:61-64,337-338`) — halving the effective item budget under the 80 KB cap; make `structuredContent` optional. (c) Canonical items carry `start_byte`/`end_byte` twice (`src/navigation/types.rs:14-15,24-25`), `ordinal`, `snippet: null`, and `symbol`/`location` duplication — Concise mode should drop them (`output.rs:271-273` already drops `why`).
Metric: truncation-rate / items-per-result on fixed hot queries before/after; adoption ToolCorrectness must hold. Flag: (b)'s cost depends on what the client injects into context — verify with one Claude Code trace before doing it.
Effort: **S**. Confidence: **medium** (token-cost extrapolated).

**P13 — Go build-tag / same-package-collision partitioning.**
Why: no build-constraint handling exists (file collection is extension-only, `src/repo_loader.rs:639,752`); mutually-exclusive same-dir definitions collide — acknowledged in code (`src/resolution.rs:1624-1628`) and adjudicated as `prism_fp` (zap `withLogger` collision `global_test.go:244`; prometheus `NewDiscovery` cross-package `azure_test.go:684`).
Seam: parse `//go:build`/filename constraints at collection (`repo_loader.rs:752`), tag `ParsedFile`, consume at `resolution.rs:1629`.
Metric: the adjudicated FP sites flip; zap/prometheus M2 precision.
Effort: **M**. Confidence: **medium** (class confirmed; frequency unquantified).

**P14 — Interprocedural taint descent (staged, after P6+P3/P4).**
Why: taint stops at **every** function boundary — `src/cpg/trace.rs:317-330` records `BoundaryKind::CrossFunction` and `continue`s; the arg→param edge is itself the boundary (`src/data_flow.rs:277`). This, not edge confidence, is the taint recall ceiling; `taint_reaches` is an intraprocedural tool with warnings. Descending Exact call edges (confidence-gated, depth-bounded) is the single biggest `taint_reaches` value unlock — but only measurable once P6(b) fixtures exist.
Seam: the boundary handler at `trace.rs:317`; gate descent on `ResolutionConfidence::Exact` first; propagate min-confidence onto the witness path (fills the reasoning-confidence hole, `reasoning/types.rs:172-181`).
Metric: P6(b) fixtures (cross-function positive + boundary negative); perf gate = existing cold-build benchmarks.
Effort: **L**. Confidence: **medium** (doctrine-aligned; payoff unproven until fixtures exist).

**P15 — Rust re-export tail: glob depth ≥3 + `pub(in)` restrict-path.**
Why: bounded, known residue — `MAX_GLOB_DEPTH = 2` fails closed (`src/name_resolution/engine.rs:107,573`, counted in `depth_exceeded`); undecidable `pub(in)` members poison (`engine.rs:619-688`, `member_undecidable` bucket). Renames and cfg-gating are already handled (`rust_populator/scopes.rs:220,80`) — do not re-plan those.
Seam: `engine.rs:107/573`; the tri-state hook from #127 is the forward-compatible seam for restrict-path.
Metric: `call-stats glob_expand.depth_exceeded`/`member_undecidable` on ruff; M2 0-regression; canary `multi_target_exact_sites` flat.
Effort: **S–M**. Confidence: **medium** (bucket counts known; buy has under-delivered projections twice before — verify with a spike count first).

### 3a. Dynamic-language doctrine (Python, JS/TS)

Soundness is impossible; these are the operating rules, each tied to an existing seam:

1. **Grounded beats guessed.** A high-precision edge from types (`src/type_providers/typescript.rs` structural satisfaction), imports (`resolution.rs:1509-1535`), same-class `self`/`this` (`resolution.rs:1161-1189`), or the framework layer (routes/DI/decorators — P9) always outranks name-population fan-out. Extend `type_providers` + `frameworks` where the construct is framework-defined; that's where dynamic call/taint edges actually live.
2. **Uncertainty is first-class — never silently drop, never fabricate.** The binary today is Exact/NameOnly/`dropped` (`resolution.rs:26-28,1507`), and `dropped` is invisible (aggregate counters only, `src/navigation/mod.rs:296-297`). For Py/JS/TS, replace drop with **capped, ranked, labeled candidates** (P3): ≤3 owners → NameOnly candidate edges; above the cap → a warning that names the sites. A labeled maybe-edge beats both a missing edge and a confident wrong one. `Exact` is never minted from a heuristic — the Tier-A matrix `forbid_resolution_kind` pins this.
3. **Encode the candidate; trigger the check in the skill.** The output carries `score`/`trust`/`kind` (already: `module_graph.rs:31-35`, `evidence_view.rs:1341-1364`); the SKILL.md carries the procedure: *score 1.0 → act; score <1.0 → read the cited site before relying on it; a collision warning → recall gap, grep the named method* (P2). Prose never asserts a dynamic edge as fact.
4. **Rank and cap.** Existing caps stay (50 items / 80 KB, `input.rs:9`, `output.rs:11`); candidate fan-outs are capped per site, not per query. Firehose output is a defect even when every item is true (P1).
5. **Count what you can't model.** `getattr`/`__getattr__`/dynamic import/metaclasses (nothing in `type_providers/python.rs`; `getattr` appears only as a taint sink, `taint.rs:92-93`), JS `this`-rebinding/prototype mutation (absent from `typescript.rs`/`resolution.rs`) stay **out of scope for edges** — but each gets a telemetry bucket so the absence is measured, not silent. Deep MRO (beyond the depth-1 same-file base at `resolution.rs:844,851`) graduates from "counted" to "modeled" only when adjudications show volume.
6. **Static tiers keep the precision floor.** Rust/Go keep drop-not-fanout (`r6_multi_owner_drop` fixture); their recall recovery comes from *evidence-backed indices* (P5 func-value fields, P8 macro args, P11 field/return types), never name fan-out. Rust trait-object/generic precise dispatch stays shelved (spike verdict: fragmented tail, fan-out counts misleading) — the demoted `TraitCha` NameOnly edge and the authoritative-scope decline (`resolution.rs:988,1055-1065`) are the accepted trade.

## 4. Top-3 execution specs

### 4.1 P1 — Review-output collapse

**Files:** `src/algorithms/taint.rs`, `src/output/review.rs`, `src/main.rs`, `tests/cli/output_test.rs` (or nearest CLI output test file), `skills/prism-code-slicing/SKILL.md`.
**Changes:**
1. `taint.rs:10842-10848`: inside the `taint_from_diff` loop, `continue` unless `ctx.files.contains_key(&diff_info.file_path)` (unparsed files can't be traced; today they still seed and emit findings).
2. `taint.rs:11165-11180`: emit the per-source `info` finding **only** for sources that appear in at least one `paths` entry that reaches a sink (the `paths` vec from `taint_forward_cfg` at `:10903` is in scope); pure-source seeds produce no finding.
3. `src/output/review.rs`: in `to_review_output` (`:154-171`) and `MultiReviewOutput` (`:69-86`), (a) remove `slice_lines` and `diff_lines` from the serialized review JSON (`:33,:36` — they restate `slice_text`); (b) add `min_severity: Severity` filtering of `all_findings` and per-result findings, default `warning` for `--format review`; (c) include `slice_text` blocks only for blocks that contain at least one retained finding, unless a new `--review-full-slices` flag is passed.
4. `src/main.rs`: add `--review-min-severity <info|warning|concern>` (default `warning`) and `--review-full-slices` (default false); wire to (3). `--format json` behavior unchanged.
5. Update `skills/prism-code-slicing/SKILL.md` output-formats section: document the new defaults; delete the "combined output is large" warning once true.
**Done-check:** `./target/release/prism --repo . --diff <(git show 36429d7 --format= --unified=3) --algorithm review --format review | wc -c` < 200 KB (was 13.5 MB); no finding with `file: "Cargo.toml"`; new CLI test asserts (i) unparsed-file seeds produce no finding, (ii) default output contains no `info` findings, (iii) `--review-min-severity info` restores them; `cargo test --test cli` + `cargo fmt --check` pass.

### 4.2 P2 — Docs/skills truth pass

**Files & exact edits (all verified stale):**
1. `CLAUDE.md`: 26→30 at `:5,:17,:71,:74,:277`; add `contract_slice`/`peer_consistency_slice`/`callback_dispatcher_slice`/`primitive_slice` to the Algorithm Implementation Map (`:117-151`) and the Simple/Graph-based lists (`:225-226`) (categorize by reading each module's imports); `output.rs`→`src/output/` (`:72`); "six read-only navigation tools" → "eight tools (6 nav + `taint_reaches` + `refresh_index`)" (`:208`).
2. `README.md`: 27→30 at `:41` and `:66`.
3. **Delete `LLM.md`**; move its "Recommended Prompt Structure" block (`LLM.md:84-99`) into `ALGORITHMS.md` if not already covered; fix any inbound links (`grep -rn "LLM.md" --include="*.md" .`).
4. `docs/MCP.md:144`: replace "~200 nodes" with "50 items default, `max_results` up to 1000, 80 KB byte cap"; add `taint_reaches` + `refresh_index` rows to the tool table (`:125-130`).
5. `skills/prism-code-navigation/SKILL.md`: replace the first gotcha (`:67-70`) with score semantics: "Every item carries `score`: `1.0` = exact resolution (act on it); `0.6` = name-only candidate (read the cited site first). A warning like `N same-name receiver call site(s)... not attributed` means real callers may be missing — treat 'no callers' + that warning as *unknown*, not *none*." Fix `:75` truncation figure (50 items / 80 KB).
**Done-check:** `grep -rn " 26 \|26 code slicing\|27 slicing" README.md CLAUDE.md docs/MCP.md` returns nothing; `grep -c "~200" docs/MCP.md skills/prism-code-navigation/SKILL.md` = 0; adoption suite re-run (`eval/adoption`, cached trials fine) shows SkillActivation/pass^5 not regressed; no new doc files created.

### 4.3 P3 — Candidate edges for Py/JS/TS unknown receivers

**Files:** `src/resolution.rs`, `src/navigation/queries.rs`, `src/navigation/call_edge_cache.rs` (cache version), `eval/fixtures/python/multi_owner_candidate/` (new), `eval/fixtures/rust/r6_multi_owner_drop/` (unchanged — the guard), `tests/integration/` resolution tests.
**Changes:**
1. `src/resolution.rs`: add `ResolutionKind::R6MultiOwnerCandidate`. In the R6 residue block (`:1493-1507`), before the final `dropped`, gate on the **caller file's language** ∈ {Python, JavaScript, TypeScript, Tsx}: if `owners.len() <= 3`, return `ResolutionOutcome::hit(demoted(method_ids, ResolutionKind::R6MultiOwnerCandidate))` (NameOnly). All other languages and `owners.len() > 3`: unchanged drop.
2. `src/navigation/queries.rs`: count the new kind in `kind_nameonly` (`:287`); extend the collision warning (`:457-458`) to append up to 5 dropped site `file:line`s when sites are still dropped (>3 owners).
3. Bump the call-edge cache version (`src/navigation/call_edge_cache.rs` — resolved edges are cached per #148, so stale caches would hide the change).
4. Fixtures: new `eval/fixtures/python/multi_owner_candidate/{app.py,expected.toml}` — two classes `A`/`B` each defining `handle()`, plus an untyped `x.handle()` call site in a free function. The matrix checks callers of a seed (`eval/tier_a/matrix.py:102-113`): seed `A.handle`, expect the call site attributed as caller (subset mode) with `expected_resolution_kind = "r6_multi_owner_candidate"` and `exact = false`. Confirm `eval/fixtures/rust/r6_multi_owner_drop/expected.toml` still passes **unmodified** (Rust must keep `callers = []`, `exact = true`).
5. `skills/prism-code-navigation/SKILL.md`: one line (composes with P2): candidate edges appear at score 0.6 with kind `r6_multi_owner_candidate` — verify at the site.
**Done-check:** `cd eval && uv run tier-a --matrix-only --allow-stale-sut` after `cargo build --release`: new python fixture `ok`, rust fixture `ok`, 0 regressions; `uv run tier-a --quick --allow-stale-sut` M2 dogfood P=1.0 maintained on Exact stratum (candidates are NameOnly, excluded from the Exact gate — if the report can't stratify by confidence yet, note it in the PR and land P6(a) first); `prism nav call-stats --repo <fastapi checkout>` shows `dropped_multi_owner` materially lower with the delta moved into `kind_nameonly`; full `cargo test` green.

## 5. Non-goals

- **No new narrative context files** — the direction is deletion (`LLM.md`) and fact-repair, not additions.
- **No dense-by-default output** — EvidenceView stays opt-in; caps stay; P3 candidates are capped at 3; anything beyond becomes a warning, not items.
- **No whole-program soundness for dynamic languages** — `getattr`/metaclasses/prototype mutation/dynamic `require` get telemetry buckets, not guessed edges.
- **No precise Rust trait-object/generic dispatch** — spike verdict stands (fragmented tail); demoted `TraitCha` + authoritative-scope decline remain the trade.
- **No coverage-% badge chasing** — every accuracy item above must move an adjudicated site class, a matrix fixture, or a stratified M2 number, not a headline percentage.
