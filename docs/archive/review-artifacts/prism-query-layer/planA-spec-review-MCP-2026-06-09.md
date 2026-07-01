# Merged Spec Review — Prism Tier 2, Plan A (Substrate Hardening Design)

Both lenses (Rigor = completeness/ambiguity/verifiability; Soundness = design/decomposition fit) returned full reviews — no node failed, so this synthesis draws on both. They converge on a single thesis: the overlay-first *architecture* is sound, but the **slice descriptions for A3/A4/A5 model three subsystems (the witness engine, sanitizers, the Rust CFG) as more vacuous/greenfield than the verified code actually is**, and as literally worded each would mutate production output and break Option-C. The two lenses reinforce each other almost everywhere; disagreements are resolved inline.

---

## BLOCKER

**B1 — §3.2 / §4.A4 / §8 (Sanitizers): the cleansing subsystem is already production-wired, so "populate recognizers" is the wrong deliverable and one reading breaks Option-C.**
*Issue:* The spec finding "sanitizers vacuous; `cleansed_for` always empty" is stale. Soundness verified (sharper than Rigor here) that `active_recognizers()` is already populated — python 5, js_ts 3, path 2 — and that `SHELL_RECOGNIZERS` is **empty by design** because Go OsCommand cleansing uses deliberate sink-time AST+CFG validation, not the textual registry. Production `apply_cleansers` (`taint.rs:10645`, called at `:10847-10848` right after `taint_forward_cfg`) already enriches `FlowPath.cleansed_for`. Two consequences both lenses agree on: (a) A4's "≥1 recognizer per Go/Python/JS-TS" is wrong — Python/JS-TS are covered, and a Go textual recognizer would *regress* the sink-time architecture; (b) if a planner implements "populate recognizers" by adding to the global registry, production suppresses more sinks → taint-sink/`algo_taxonomy_sanitizers` fixtures change → byte-identity fails. The genuine gap (Rigor's §8 MAJOR, which Soundness correctly folds into this same BLOCKER — they are one finding, not two) is that the overlay reads raw `taint_forward_cfg` output where `cleansed_for` is hardcoded empty (`cfg_queries.rs:198`).
*Resolution:* Reframe A4 as **overlay consultation of the existing seam**, not population. Expose `apply_cleansers`/`function_body_cleansed_for` as `pub(crate)` (the §6 "minimal `pub(crate)` now" decision already anticipates this) over the **frozen** `active_recognizers()` set; A7's overlay invokes it so `taint_reaches` surfaces the `cleansed_for` production already computes. Do not add to the global registry; do not add a Go recognizer; "honest-empty for C/C++/Rust" stays.

**B2 — §3.1 / §4.A3 / §3.7 (Chain-BFS, D4): make it additive and pin the witness substrate to the `CpgContext` petgraph engine, not `DataFlowGraph`.**
*Issue:* `CpgContext::dfg_forward_reachable` (`query.rs:508`) returns a flat `BTreeSet<VarLocation>` with no predecessor chain and has **6 call sites** — `cfg_queries.rs:164`, `query.rs:615` (internal), plus production `left_flow.rs:82,164`, `full_flow.rs:121`, `taint.rs:4870`. Rigor flags the API ambiguity (new witness API vs. changing `FlowPath.edges` vs. changing the set-returning API) and the blast radius; Soundness adds the decisive engine fact: production routes through the `CpgContext` petgraph (`self.graph.edges`), while `DataFlowGraph::forward_reachable` walks a separate `forward` map and has only internal+test callers. Both lenses agree the spec's finding #7 ("`DataFlowGraph` has zero unit tests") is misaimed — Rigor verified direct `DataFlowGraph::build`/`forward_reachable` tests already exist (`tests/ast/dfg_test.rs`), and Soundness shows that even if added they would test the engine production doesn't use. A literal return-type change is only behavior-preserving if all 6 sites project back to `BTreeSet` — needless churn and drift risk.
*Resolution:* Add the chain-reconstructing method **additively** on the `CpgContext` petgraph engine (zero call-site edits); put D4 chain-shape tests against `dfg_forward_reachable` in `src/cpg/tests.rs`. Specify shortest-path tie-breaks, start-node inclusion, no-path shape, same-line synthetic-edge labeling, and backward-provenance scope. If `DataFlowGraph` construction tests are still wanted, label them separate general hardening, not the D4 witness coverage.

**B3 — §4.A5 (Rust `?` early-return CFG edge): no typed-edge target exists in the model, and the naive fix breaks byte-identity.**
*Issue:* `CfgEdge` is only `{file, from_line, to_line}` (`cfg.rs:17-23`); CPG projection maps line pairs straight to `CpgEdge::ControlFlow` (`build.rs:431-439`); and the `?` handling is self-contradictory in comments (says it creates an early-return edge, then says the edge is not modeled — `cfg.rs:527-532`). Rigor rates this BLOCKER (no typed-edge target to project from; modeling must be designed). Soundness rates it MAJOR (intent is clear; risk is byte-identity, since `taint_forward_cfg` prunes on CFG reachability so adding the Err→return edge to the *production* CFG changes Rust Taint output). **Resolution of disagreement: BLOCKER is correct** — Rigor is right that a slice author cannot implement from current wording because the typed edge does not exist in the data model and must be invented; Soundness's contribution is the concrete proof obligation, not a downgrade.
*Resolution:* Specify a new **overlay-only** edge kind (`TypedCfgEdge` or equivalent) with a synthetic return/exit target; production `build_rust_edges` and `taint_forward_cfg` consume the unchanged CFG. Add a Rust-`?` Taint golden that *proves* production output is unchanged.

**B4 — §2 / §4.A2 (`compute_bindings`): the contract is not implementable as written, and the §2 single-source claim contradicts field-insensitivity.**
*Issue:* Two complementary facets. Rigor (BLOCKER): the spec says only "pure fn (no petgraph); `build.rs` delegates," but Step 5b depends on `resolve_callees_qualified`, `call_argument_texts`, `function_parameter_names`, `var_index`, first-call-on-line behavior, `.`/`->` base truncation, Use-before-Def fallback, and callee-param lookup by line range (`build.rs:327-405`; `ast.rs:2734-2750,2925-2969`) — none specified. Soundness (raised as MAJOR, but it is really part of this same A2 design gap): §2 promises the overlay "calls the *same* function for **richer** flows," yet Step 5b splits args **field-insensitively** (`build.rs:367-368`), so one function cannot be both byte-identical for production and field-sensitive for the overlay. The single-source-of-truth claim breaks.
*Resolution:* Define the exact module, signature, output record, ordering, resolver interface, and byte-identity quirks — **and** add an explicit precision parameter (production-mode = today's field-insensitive; overlay-mode = field-sensitive) from A2, so the A2 characterization test constrains only production mode and the deferred `FieldOf` (A.5) doesn't force a refactor of the seam this slice establishes.

**B5 — §4.A7 (`src/reasoning/` scaffolding): needs a concrete public surface.**
*Issue:* (Rigor) `ReasoningGraphView`, bounded deterministic BFS, and `SeedSet` are named but not specified. Plan B has concrete `SeedSet`/`taint_reaches`/evidence-shape/placement text (`...taint-reaches-design.md:65-133`), but Plan A does not import it normatively, leaving two partially-overlapping sources of truth. Soundness confirms the overlay is "over CPG node indices" — reinforcing B2's substrate pin.
*Resolution:* Either import Plan B's reasoning sections into Plan A as normative or strip A7's implementation detail to a pure seam. Specify module/file names, structs, lifetimes, traversal order, bounds, warning/edge kinds, and the `Evidence`/`GraphPayload` mapping.

---

## MAJOR

**M1 — §2 / §5 (Option-C overclaim): "true by construction" holds only for A7.**
*Issue:* Both lenses converge here. The spec asserts Option-C is "true by construction" with "nothing to re-baseline," but A2 changes `build.rs` delegation, A4 may touch production taint, and A5 changes CFG modeling/projection — all of which feed production output. Only A7 (pure overlay) is genuinely true-by-construction.
*Resolution:* Restrict the "by construction" claim to A7; attach an explicit characterization/golden proof obligation to A2, A4, and A5 (this is the proof side of B1/B3/B4).

**M2 — Architecture: the two reachability engines are never reconciled.**
*Issue:* (Soundness, unique) `DataFlowGraph` (`forward` map, the persisted/cached def-use store built at `build.rs:127`) and `CpgContext` (petgraph) each carry independent `forward_reachable`/`taint_forward` pairs; production routes only through `CpgContext`. Leaving "which engine is the Tier-2 substrate?" undeclared invites effort landing on the secondary structure (the misaimed finding #7) and a permanent maintenance ambiguity.
*Resolution:* Add a §6 row pinning the Tier-2 substrate to the `CpgContext` petgraph engine and stating `DataFlowGraph`'s reachability methods are legacy relative to Tier-2.

**M3 — §6 / §8 (`CallResolver`/`NameResolver`): a resolved decision with no slice.**
*Issue:* (Rigor, unique) Owner decisions say land `CallResolver` + `NameResolver` now (§6; finding #8), but A2–A7 contain no resolver slice. Rigor also corrects the spec's "`resolve_dispatch` verified dead" framing: it is dead only for `src/` call sites — extensive provider tests exist under `tests/ast/*type_provider_test.rs`, so the real gap is the missing slice/contract, not absent characterization. (Soundness independently confirms zero `src/` call sites.)
*Resolution:* Add an explicit A-slice for the resolver (file/API/tests), fold it into A2, or defer it — but do not leave a resolved decision unscheduled.

**M4 — §3.1 / §6 (Backward provenance scope) is contradictory.**
*Issue:* (Rigor, unique) §3.1 says predecessor tracking "unlocks" backward provenance and variable-symbol v2, while §6 defers variable provenance to v2.
*Resolution:* State that A3 ships **forward witness chains only** unless backward APIs/tests are explicitly in scope this slice.

**M5 — §5 / §7 (Testing language is not executable).**
*Issue:* (Rigor, unique and strong) "nav-golden + diff-review suites" is not actionable. The repo has a named `cli_nav_compat` target with deterministic diff-review and nav goldens (`Cargo.toml:495-496`; `tests/cli/nav_compat_test.rs:33-98,383-467`), but the aggregate `review` preset is **explicitly not byte-stable** because Taint is pre-existing nondeterministic in that fixture (`nav_compat_test.rs:17-22`). Demanding byte-for-byte proof on the default suite is impossible as stated.
*Resolution:* Replace prose with exact commands — at minimum `cargo test --test cli_nav_compat`, the relevant per-slice `ast_*`/CFG/CPG targets, and `cargo fmt --check`. Do not require byte-for-byte aggregate `review` unless a stable fixture is named or the Taint gap is replaced.

---

## MINOR

**m1 — Reference/anchor corrections (both lenses).** Re-anchor the "where cleansing lives" reference from `data_flow.rs:610` (that is `DataFlowGraph::taint_forward`, the *secondary*-engine construction site) to the production seam `apply_cleansers` (`taint.rs:10645`, called `:10848`) so implementers land on the right engine. The nav catch-all `match` opens at `navigation.rs:42` but the `other =>` arm is at `:73` (cite `:73`). "Split `src/cpg.rs` is merged" is imprecise — `src/cpg.rs` remains as a facade/re-export surface (`cpg.rs:14-16,34-36`).

**m2 — Label ambiguity (Rigor).** "Plan A.5" (interprocedural return→caller, out of scope) collides visually with "A5" (Rust `?` CFG slice); and `CpgEdge::FieldOf` "decide at A.5" (§6) is ambiguous against the A2/A5 labels. Rename one axis (e.g., A‑slices vs. Phase A.5) to remove the collision.

**m3 — §4.A6 wording (Soundness).** Spell out "(+ CPG version per DQ4)" as "fix the push-guard in the `CpgContext`-engine method too," **not** a `CACHE_VERSION` bump (§5 forbids it). Note explicitly that the fix is output-neutral (the visited set is identical regardless of duplicate enqueues), so it is safely additive.

---

**Verified positives worth preserving (do not regress):** overlay-only architecture; the no-`CACHE_VERSION`-bump reasoning (cold-rebuild vs. ACP-handshake fragility); honest-empty sanitizers for C/C++/Rust; the §5 guardrails — `error_text` is an exhaustive 5-arm match (`navigation.rs:13-23`) so "never add a `QueryError` variant" is correct, while the `Reason` catch-all + `{:?}` rendering make additive `Reason`/`WarningKind` vocab safe; and "do not touch `taint_forward_cfg`" (its interprocedural pass-through is deliberate and the Taint slice depends on it).

---

**Verdict: Not ready to plan — resolve B1–B5 first (reframe A4 as seam-consultation, make A3 additive on the CpgContext engine, give A5 a typed overlay-only edge + golden, specify `compute_bindings` with a precision parameter, and pin A7's public surface), then tighten M1–M5; the architecture survives, but the A3/A4/A5 slice descriptions must be rewritten as overlay-only/additive before handing to `writing-plans`.**