# Bounded nested-callable execution-owner proof

Implementation target: **Sol high** (`gpt-5.6-sol`, high). Independent reviewer: **Terra high** (`gpt-5.6-terra`, high). Review cap: **two rounds** on the same artifact. This document authorizes a proof harness and a repair-ready design when dispatched; it does not bundle the subsequent production repair into the proof slice.

## 1. Placement and verified base

The next slice belongs to **workstream 3: occurrence/data-flow limitations**. More precisely, it establishes which nested-callable observations belong to which execution region before changing rvalue extraction. This is source-level execution-region ownership, not admission through the experimental compiler-backed executable-owner subsystem.

On 2026-09-13, `gh pr view 314 --repo shoedog/prism --json state,mergedAt,mergeCommit,headRefOid` reported MERGED, at `2026-09-12T07:51:54Z`, merge commit `c1dbc292175f8e93271e7a3699b2ee20fa41e42f`. `gh api repos/shoedog/prism/commits/main --jq '{sha:.sha,tree:.commit.tree.sha}'` reported that same main tip and tree `2ac20fcbf743d54c508f914d685936285f30a549`. The inspected checkout remains `feat/js-ts-module-binding-audit`, HEAD `36aec5f7a16e9eb90002f01235365cdc4ab6395a`; `git rev-parse HEAD^{tree}` returned the identical tree. Local `main` and `origin/main` are stale, and the merge object is not locally available. **All existing-code line anchors below are therefore main-branch line numbers proved by complete tree equality, not by trusting those stale refs.** No checkout, fetch, commit, push, or PR mutation was performed during planning.

The supplied publication handoff's OPEN/pending-publication statements are historical and superseded by this measured merge status. Its verification results remain historical evidence. Sources: `/private/tmp/prism-contained-rvalue-FSGIPx/publication-handoff.md`, `/private/tmp/prism-contained-rvalue-FSGIPx/mr-bundle.md`, and the committed contained-rvalue readout.

| Original workstream | Current disposition | What remains |
|---|---|---|
| 1. Exact parameter binding | Completed in #312 before #313/#314. `src/cpg/build.rs:137`, `compute_param_def_nodes`, intersects exact supported occurrence tokens with positional slots and validates the complete graph endpoint. | None of the named 3-public/61-private body-target defect remains in the recorded fixed population. Keep regression coverage; this does not prove universal parameter correctness. |
| 2. Parameter coverage | #313 implemented TS/TSX simple optional identifiers in initializer-free signatures and JS/TS/TSX inert-default identifiers under a whole-signature guard. | Complex/effectful defaults and default-environment value flow; optional signatures excluded by the initializer guard; destructured/rest bindings; unparenthesized-arrow occurrence/owner coverage; further forms only after measured value and positional/refusal proof. |
| 3. Occurrence/data-flow | Active. #314 fixed escaping ancestor rvalue captures; nested descendants can still contaminate an outer callable. | This ownership proof, then bounded repair; genuine same-line occurrence identity, exact loop/member reads and broader member shapes remain separate. |

Workstream 1's readout, `docs/eval/receiver-closure/2026-09-10-js-ts-exact-parameter-binding.md:48`, records unmatched flows **3→0 public / 61→0 private**, with legitimate matched flows unchanged. These are inherited fixed-population observations, not a new census. Workstream 2's contracts are in `docs/eval/receiver-closure/2026-09-11-optional-default-parameters.md:7`; `src/parameter_slots.rs:22` (`typescript_parameter_bindings`), `:122` (`typescript_inert_default_occurrences`), `:188` (`javascript_inert_default_occurrences`), and `:251` (`is_inert_default_value`) implement them. Literals admitted are number/string/boolean/null and empty object/array, with the documented sibling restrictions. Unary numbers, templates, references, calls, filled containers, erased `this`, decorators and parameter properties do not gain support from that slice.

Do not describe unparenthesized arrows as wholly unparsed: `src/parameter_slots.rs:270`, `slots`, accepts a singular `parameter` field, whereas `src/ast.rs:9470`, `ParsedFile::function_parameter_occurrences`, relies on `find_parameters_node` at `:9531`, which looks for `parameters` (or C/C++ declarators). Slot availability is not occurrence authority. Search: `rg -n 'fn (slots|typescript_parameter_bindings|typescript_inert_default_occurrences|javascript_inert_default_occurrences|is_inert_default_value)|fn (function_parameter_occurrences|find_parameters_node)' src/parameter_slots.rs src/ast.rs`.

The module/receiver repairs in #314 do not close workstreams 4–7. Node backend measurement, compiler ownership feasibility, React provenance, packaging/performance and trustworthy accuracy measurement retain their separate boundaries. Skipped CJS export-name coverage and other comment-sensitive consumers also remain independent; no claim of their complete repair follows from this plan.

## 2. Outcome and limits

Deliver a small, executable proof package that:

1. Reproduces and enumerates nested rvalue contamination on unchanged main, across JS/TS/TSX.
2. Distinguishes declaration tokens, deferred default evaluation, nested body reads, conservative lexical capture facts, and eager surrounding expressions.
3. Maps each observation through raw names/paths/spans, DFG occurrences/labels, CPG nodes/edges, reaching-definition fallback and FullFlow consumers.
4. Separates a demonstrated wrong extractor result from a demonstrated wrong exact graph edge or public result. Absence of the latter never erases the former.
5. Produces a bounded production-repair contract, explicit excluded forms, exact edit seams, and desired tests to carry forward.

**Production semantics remain unchanged in this slice.** Only test modules/registrations and proof documentation are writable implementation scope. CPG cache stays **92**, navigation call cache stays **52**. No new production public API, dependencies, compiler worker, closure/call admission, parameter syntax, receiver rules, occurrence index or serialized schema. Test-only structures described below are not persisted authority.

A green audit means its classifications are checked; it does not mean the ownership defect is repaired. Preserve one explicitly ignored aggregate desired-behavior regression with a repeatable RED command, separate from normally passing characterization tests. This additional ignore must be named in every gate total until the repair enables it. Do not turn the already enabled #314 desired-flow test back into an ignore.

## 3. Source map and seams

All paths are relative to project root; line numbers are the main tree pinned above. New files/functions have no main line number yet; use their proposed names as search anchors after creation. Read each exact region immediately before editing it.

| Existing location and symbol | Role / action in this slice | Search |
|---|---|---|
| `src/lib.rs:38`, crate module declarations | Add one `#[cfg(test)] mod nested_execution_owner_audit;` for shared fixture/observation support. | `rg -n '^pub mod ast;\|^pub mod cpg;' src/lib.rs` |
| `src/ast.rs:11104`, `contained_rvalue_tests` registration | Add adjacent `#[cfg(test)]`/`#[path]` registration for private AST collector tests only. | `rg -n 'mod contained_rvalue_tests' src/ast.rs` |
| `src/cpg/build.rs:5`, `namespace_flow_audit_tests` registration | Add adjacent test-only module to access private build/binding seams without widening production visibility. | `rg -n 'mod namespace_flow_audit_tests' src/cpg/build.rs` |
| `src/ast.rs:7079`, `ParsedFile::rvalue_capture_is_contained` | Preserve. Byte containment excludes ancestors but accepts nested descendants. | `rg -n 'fn rvalue_capture_is_contained' src/ast.rs` |
| `src/ast.rs:7089`, `ParsedFile::rvalue_identifier_paths_on_lines`; `:7166`, `rvalue_identifier_spans_on_lines`; `:7540`, `rvalue_identifiers_on_lines` | Observe all three APIs, assignment and call gates separately. Public signatures and return shapes stay unchanged. | `rg -n 'pub fn rvalue_identifier' src/ast.rs` |
| `src/ast.rs:7246`, `collect_rvalue_paths_manual`; `:7277`, `collect_rvalue_spans_manual`; `:7614`, `collect_rvalues_manual` | Private fallback observations, with the same selected lines and exact root. | `rg -n 'fn collect_rvalue.*manual' src/ast.rs` |
| `src/ast.rs:7363`, `collect_identifier_paths`; `:7369`, `collect_identifier_paths_in_value_context`; `:7420`, `collect_identifier_path_spans`; `:7426`, `collect_identifier_spans_in_value_context`; `:7644`, `collect_all_identifiers`; `:7650`, `collect_identifiers_in_value_context` | Recursive contamination route; inspect other callers before proposing an owner argument or global pruning. | `rg -n 'fn collect_(identifier\|all_identifier)' src/ast.rs` |
| `src/ast.rs:7495`, `collect_return_value_identifier_spans`; `:7526`, `collect_return_node_value_spans` | Return-query gate and recursive returned-expression walk. Names/paths intentionally lack the equivalent return-only query. | `rg -n 'fn collect_return.*spans' src/ast.rs` |
| `src/languages/mod.rs:106`, `Language::function_node_types`; `:139`, `callable_boundary_node_types` | Read-only boundary inventory. The latter additionally includes anonymous `generator_function`; inventory membership is not execution-time proof. | `rg -n 'fn (function_node_types\|callable_boundary_node_types)' src/languages/mod.rs` |
| `src/data_flow.rs:255`, `DataFlowGraph::build_from_refs`, especially `:451–613` | Raw spans become Uses; `get_use` can supply a zero-width line anchor; scoped refs and reaching labels are separate. Observe, do not repair. | `rg -n 'fn build_from_refs\|preferred_use_locs\|let mut get_use\|param_ref_jobs\|reaching_definitions' src/data_flow.rs` |
| `src/cpg/reaching/capture.rs:12`, `capture_facts`; `:40`, `is_capture`; `:50`, `collect_reference_identities` | Retained capture/timing uncertainty. Do not remove this path as a shortcut. | `rg -n 'fn (capture_facts\|is_capture\|collect_reference_identities)' src/cpg/reaching/capture.rs` |
| `src/cpg/reaching/scope.rs:871`, `use_byte` | Real span, unique matching rvalue span, then unique line identifier fallback. Audit the fallback after a hypothetical raw-span removal. | `rg -n 'fn use_byte' src/cpg/reaching/scope.rs` |
| `src/cpg/build.rs:622`, `CodePropertyGraph::assemble_graph`, Uses at `:710` | First occurrence per file/function/start-line/line/path/access. Read-only collision control. | `rg -n 'fn assemble_graph\|for locs in dfg.uses.values' src/cpg/build.rs` |
| `src/cpg/build.rs:137`, `compute_param_def_nodes`; `:1130`, `step5b_edges_for_caller`; `:1226`, `argument_var_node_in_span`; `:1583`, `collect_step5b_edges_reference` | Preserve exact callee, slots, argument containment and parallel/serial behavior; capture complete endpoints. | `rg -n 'fn (compute_param_def_nodes\|step5b_edges_for_caller\|argument_var_node_in_span\|collect_step5b_edges_reference)' src/cpg/build.rs` |
| `src/cpg/build.rs:1282`, `assemble_step5c_return_flow` | Observe return-boundary behavior separately from raw return extraction. | `rg -n 'fn assemble_step5c_return_flow' src/cpg/build.rs` |
| `src/algorithms/full_flow.rs:19`, `slice`, name-rvalue calls at `:138` and `:165` | Test DFG-backed and name-only modes; output changes require actual SliceResult evidence. | `rg -n 'pub fn slice\|rvalue_identifiers_on_lines' src/algorithms/full_flow.rs` |
| `src/cpg/namespace_flow_audit_tests.rs:84`, `observe`; `:426`, `namespace_flow_classification_epochs`; `:482`, `namespace_flow_nested_callable_ownership_observations` | Follow endpoint assertions and source-epoch pattern. Keep existing historical-behavior test unchanged in this proof slice. | `rg -n '^fn (observe\|namespace_flow_classification_epochs\|namespace_flow_nested_callable_ownership_observations)' src/cpg/namespace_flow_audit_tests.rs` |
| `src/ast_contained_rvalue_tests.rs:6`, `contained_rvalue_query_capture_boundaries`; `:101`, `contained_rvalue_whole_file_root_keeps_accepted_capture_behavior`; `:125`, `contained_rvalue_non_js_immediate_reads_are_unchanged` | Existing containment and whole-file inventory contract. Explicitly retain root behavior when designing scoped ownership. | `rg -n '^fn contained_rvalue' src/ast_contained_rvalue_tests.rs` |
| `src/ast_asserted_member_tests.rs:74`, `names_paths_and_manual_routes_agree_without_expanding_return_only_api`; `src/ast_erased_rvalue_tests.rs:46`, `manual_and_query_rvalue_routes_preserve_runtime_values_and_raw_trees` | Patterns for private route tests, erased-context and member boundaries; compare only shared API semantics. | `rg -n '^fn (names_paths\|manual_and_query)' src/ast_asserted_member_tests.rs src/ast_erased_rvalue_tests.rs` |
| `src/cpg/reaching/tests/captures.rs:3`, `assert_capture`; JS/TS cases at `:77–107` | Existing `NameOnly(CfgIncomplete)` capture controls. | `rg -n '^fn (assert_capture\|javascript_.*capture\|typescript_.*capture)' src/cpg/reaching/tests/captures.rs` |
| `tests/reasoning/taint_reaches_test.rs:1604`, `assert_nested_callable_return_does_not_escape`; `:1651`, `return_flow_javascript_arrow_return_is_fenced_from_outer_function` | Existing public negative; a bad raw span does not prove a return escape. | `rg -n 'fn (assert_nested_callable_return_does_not_escape\|return_flow_javascript_arrow)' tests/reasoning/taint_reaches_test.rs` |
| `src/cpg_cache.rs:209`, `CACHE_VERSION`; `src/navigation/call_edge_cache.rs:96`, `NAV_CALL_EDGE_CACHE_VERSION` | Read-only pins 92/52. Future runtime DFG changes need cache review; proof does not. | `rg -n '^const .*CACHE_VERSION' src/cpg_cache.rs src/navigation/call_edge_cache.rs` |

The prism-nav skill was used to trace `rvalue_identifier_spans_on_lines`: its current response had no freshness warning or truncation and identified DFG construction and reaching `use_byte`, among test/example consumers. Direct source reads establish the contracts above. No LSP tool was available; no type-resolved completeness claim is made.

## 4. Proof contract: do not reduce ownership to containment

Use these classifications in the test-only proof, with explicit fixtures validating each AST field relationship. A nearest-callable walk is evidence of lexical placement, not proof of all JavaScript execution timing.

| Region | Desired interpretation for a queried outer callable | Required distinction |
|---|---|---|
| Immediate outer RHS, argument, supported return | Outer rvalue, subject to the existing API's capture kinds and line filters. | Keep exact source bytes; preserve member/base representation and augmented LHS behavior. |
| Nested function name/parameter declaration | Not an outer runtime rvalue token. | A parameter Def remains valid for its own supported callable; declaration identity is not a read. |
| Nested block/expression body | Not an outer immediate rvalue. | Its own callable query still sees supported body reads; a free reference may also need conservative capture evidence. |
| Nested parameter default initializer | Evaluated on invocation of that nested callable, not on creation in the outer body. | A callable's own default is a separate parameter environment, not automatically an ordinary body read or an explicit argument edge. |
| Free outer variable used in nested body/default | Lexical capture candidate with deferred/unknown timing. | Preserve observable conservative graph behavior or explicitly design its replacement; never upgrade to Exact by deleting a warning source. |
| Outer argument adjacent to a callback; computed object-method key | Eager surrounding expression. | `register(eager, p => delayed(p))` keeps `eager`; `{ [keyExpr()](p) { ... } }` must not lose the key expression merely because it belongs syntactically to a method node. |
| Class heritage/computed keys/static initialization versus instance fields | Phase-sensitive, outside the first production admission target unless separately proved. | A blanket callable/class subtree prune is not a valid general execution theorem. Record concrete fields and excluded policy. |
| Type annotations / erased contexts | Preserve existing erased-value refusal. | Do not infer runtime identity from an identifier-looking token in a type. |
| Whole-file root | Preserve legacy inventory behavior in the future bounded repair. | Root inventory and callable execution query must be distinct contracts. Current root tests intentionally include nested observations. |
| Recovery/ambiguous or unindexed callable | Explicit unresolved/refused design row. | No fallback to same-name/same-line ownership; an absent inventory entry does not grant the ancestor ownership. |

An IIFE still has its own body execution region: invocation timing does not make its parameter declaration an outer read. This slice does not create higher-order call resolution or IIFE inlining.

## 5. Files to add and test-only interfaces

Keep approximately **700–1,000 added Rust lines or fewer**, using fixture tables; split further before dispatch if the proof requires a larger cross-subsystem framework. All new names below are proposed, not existing main symbols.

### `src/nested_execution_owner_audit.rs` — shared test-only support

Register only under `#[cfg(test)]` in `src/lib.rs`. Use existing serde/collections if useful; no Cargo dependency or production feature.

- `OwnerAnchor { start_byte: usize, end_byte: usize, kind: String }`: identity from this parsed tree. Use file/source hash in the enclosing record; never persist `Node::id()` as cross-parse identity.
- `TokenAnchor { needle: &'static str, occurrence: usize, offset: usize, length: usize }`: a fixture-authored exact source selection, independent of production extractor output. Validate match cardinality, UTF-8 boundaries and text; do not silently select the first duplicate.
- `ExecutionRegion`: `ImmediateBody`, `NestedBody`, `ParameterBinding`, `ParameterDefault`, `ErasedType`, `EagerDefinitionExpression`, `Unsupported`. Store the actual declaring callable separately where one exists.
- `ExpectedToken { anchor: TokenAnchor, region: ExecutionRegion, owner: Option<OwnerAnchor> }`. Fixture expectations are hand-authored first and then checked against concrete AST parent/field structure. They must not be generated by the same prospective pruning algorithm being evaluated.
- `AuditCase { id: &'static str, source: String, language: Language, queried_owner: TokenAnchor, expected: Vec<ExpectedToken> }`; identify nested/outer functions by byte ranges and kind, not display name alone.
- `ObservedOccurrence { consumer: Consumer, path: String, line: usize, start_byte: Option<usize>, end_byte: Option<usize>, owner: Option<OwnerAnchor> }`. `Consumer` distinguishes names, paths, spans and each manual route. Names/paths genuinely lack bytes: use `None`, never fabricate byte identity from their output.
- `GraphObservation`: copy file, function, function-start-line, path, access, line, start/end bytes for each endpoint and the actual confidence/refusal. NodeIndex is diagnostic only; comparisons use stable semantic tuples, preserving multiplicity.
- `resolve_anchor(source: &str, anchor: &TokenAnchor) -> Result<Range<usize>, AuditError>`; failures distinguish missing/ambiguous selector, invalid UTF-8 boundary, wrong text and out-of-range offset.
- `observe_raw(parsed: &ParsedFile, owner: Node<'_>, lines: &BTreeSet<usize>) -> Vec<ObservedOccurrence>`: call unchanged public APIs. Private manual collectors are observed by the AST child module and normalized through helpers here.
- `compare_expected(case: &AuditCase, observations: &[ObservedOccurrence]) -> Vec<OwnershipMismatch>`: pure comparison; collect the complete mismatch population before asserting. A mismatch includes case/language/consumer/token, expected region/owner and observed evidence. Separate unsupported obligations from proven failures.
- `canonical_graph_rows(...) -> Vec<GraphObservation>`: normalize real DFG/CPG output without removing duplicates or downgrading exact values to booleans.

No serialized return contract is added to Prism. An opt-in JSON-lines test printout may use schema `prism.nested-execution-owner-proof/1` for local receipts; fields must label synthetic observations and include case/language/consumer/build-mode/epoch/source SHA. Test output is observational, never an admission input.

### `src/ast_nested_execution_owner_tests.rs` — raw proof and desired RED

Register adjacent to the contained-rvalue test module in `src/ast.rs`; production method signatures stay unchanged. Add:

- `nested_execution_owner_fixture_anchors_are_exact`: literal expected ranges, repeated spelling, Unicode before tokens, wrong-owner and missing-token negatives.
- `nested_execution_owner_query_and_manual_classification`: table-driven current-observation assertions for all three API families and manual siblings. Preserve return-only and augmented-assignment asymmetry; do not enforce identical results where contracts differ.
- `nested_execution_owner_desired_contract`: one aggregate **ignored** test with an explicit known-gap reason pointing to this plan/readout. Desired exact expectations must fail behaviorally on the pinned production base. Only proven incorrect raw-owner cases belong in this assertion; undecided defaults/class cases stay in the design ledger. Run all rows before the final assertion.
- `nested_execution_owner_eager_and_own_callable_controls`: own-body, outer siblings, computed keys, erased-type, root inventory and non-JS controls. Controls that already pass are not counted as new RED wins.

Use exact full normalized vectors/multisets for bounded fixtures. Where names/paths cannot disambiguate same-spelling tokens, use unique spellings or record the limitation; do not invent precision. Reject parameter-token Uses by their exact bytes as well as checking genuine retained reads.

### `src/cpg/nested_execution_owner_audit_tests.rs` — consumer consequences

Register as a private child of `src/cpg/build.rs`, following its existing audit registration. Add:

- `observe_owner_case(files: &BTreeMap<String, ParsedFile>, cpg: &CodePropertyGraph, case: &AuditCase) -> OwnerFlowObservation`: capture raw/DFG/CPG evidence, resolved callee, exact target Def, indexed source Use and actual edge labels. Reuse patterns from the existing `observe`; do not make its internals public or rewrite that mature fixture population.
- `nested_execution_owner_graph_classification`: real full-build observations for a small representative subset, including independently inventoried nested callables, anonymous/unindexed controls and unsupported parameter targets.
- `nested_execution_owner_source_epochs`: compare full versus incremental normalized rows for every epoch, including a cached/serde round-trip where the existing supported graph API permits it. Use public builders and existing cache helpers; do not add production hooks to inject a fake success.
- `nested_execution_owner_capture_and_refusal_controls`: conservative capture labels, line-anchor fallback, wrong-token substitutions and genuine same-line refusal. Any test-only graph/index substitution must be isolated, labeled counterfactual, and followed by an out-of-span negative, following the earlier audit pattern.
- `nested_execution_owner_full_flow_controls`: invoke `src/algorithms/full_flow.rs::slice` using existing context/fixture APIs with and without DFG. Assert exact normalized file/line selections on bounded cases; record unchanged output as unchanged, not a wrong public result.

There are **no production parameter/argument/return changes** in this slice. Test helper argument types above may be refined to existing aliases after reading actual types; preserve their observational inputs and explicit return/error meaning.

### Documentation outputs of the implementation

- Add `docs/eval/receiver-closure/2026-09-13-nested-execution-owner-proof.md`: findings, hypothesis log, per-consumer contract, allowed/refused design and next repair seams.
- Add adjacent `2026-09-13-nested-execution-owner-proof-baseline.json`: exact fixed fixture population, source/test/base hashes, RED mismatch rows, positive/refusal controls, full-suite receipts and exclusions. Generate from actual results; no guessed counts.
- Add `docs/superpowers/handoffs/2026-09-13-nested-execution-owner-proof.md` using the installed template at `/Users/wesleyjinks/code/prompts-skills-steering/bootstrap/handoff-template.md`.
- Add a short successor link to `docs/eval/receiver-closure/2026-09-12-contained-rvalue-captures.md:110` (`Next` section), preserving its historical observations and receipt. Update this plan's task checklist with evidence paths. Do not rewrite old private evidence or memory.

## 6. Fixed fixture programme

Freeze IDs and expected token roles before running the matrix. Use pairwise coverage, not a full Cartesian explosion. Run the core assignment/initializer/call/return families in JS/TS/TSX; language-specific forms only in their valid grammar. Validate zero parse errors for positive fixtures and label recovery fixtures explicitly. Parsed syntax is not compiler-Program proof.

| Family | Required examples / controls |
|---|---|
| Existing six observations | Outer function containing `const cb = function(inner) { return inner; };` versus `let cb; cb = function(inner) { return inner; };`; real outer `local=seed` and `return local` survive. |
| Callable shapes | Named declaration, named/anonymous expression, assigned arrow with block body, expression-bodied arrow, generator expression/declaration, async function/arrow, object method/getter. Observe actual pinned grammar kinds, including unindexed forms. |
| Acceptance routes | Outer assignment RHS wrapping callable; call argument wrapping callable; return expression wrapping callable; independently queried nested assignment/call/return. Include augmented assignment LHS span control. |
| Eager siblings | `register(eager, function inner(p){ sink(p); })`, sequence expression before callback, nested object computed method key. Keep eager expression tokens distinct from delayed body/default tokens. |
| Signatures | Required/optional/inert default controls; effectful nested default (`p=init()`), sibling write default, destructuring/rest and unparenthesized arrow as observation/refusal rows. No syntax admission. |
| Captures | Inner use of outer `seed`, inner shadow named `seed`, write before/after callback creation, nested callable queried on its own. Assert actual conservative confidence and separately inspect fallback anchors. |
| Ownership ambiguity | Two same-name callables, same-start-line callables, Unicode before owner/token, anonymous boundary absent from production inventory, ERROR/missing body. Owner proof must not use the existing graph's lossy name/line key as its oracle. |
| Line/occurrence controls | Compact and multiline versions; selected outer-only line versus nested-only line; genuine `sink(value); ns.item(value)` collision versus different-line control. The collision remains separate and refused. |
| Member/type guards | Plain and currently supported asserted member path; erased TS type tokens; no broader member-shape admission. |
| Root/non-JS | Whole-file inventory retains current nested enumeration. Existing Python/Go/Rust/Java/C/C++ immediate-read controls stay unchanged; include existing conservative non-JS capture tests in gates. |
| Phase-sensitive exclusions | Class computed key/heritage, static block/field and instance field samples; report exact observed regions and why first repair excludes or preserves them. Do not solve class execution policy in this audit. |

For source epochs use a valid sequence: immediate outer read → same token moved into named nested body → nested body plus an eager outer sibling → shadowed nested parameter → original source restored. Pin changed-source membership to the app file; keep imported callee unchanged. Compare each incremental result against a fresh full build of that exact source, not just epoch 0 versus final counts. Repeated epochs/languages are observations, not unique real sites.

## 7. Task order and fail-first execution

### Task 0 — bind source, authority and custody

- [x] Read AGENTS.md and this plan; declare two review rounds and at most one justified environmental retry per failing gate.
- [x] Record cwd, branch, HEAD, tree, clean/dirty state and main comparison. Preserve unrelated files. On this machine leave `.git` writes to the controller; ask upfront for actual network needs, not repeated sandbox failures.
- [x] Use a new evidence directory under `/private/tmp` via `mktemp -d`. Save the immutable production base identity and a source snapshot before test edits; use `apply_patch` for text edits.
- [x] Re-run the existing six-row audit and containment controls. Expected: nested contamination remains; ancestor containment passes. If output differs, rebind the base before changing expectations.

Planning already ran `cargo test --offline --lib namespace_flow_nested_callable_ownership_observations -- --nocapture`: **1 passed**, six printed rows, nested returns under outer in all six; signature/path contamination only in the three assignment rows. `cargo test --offline --lib contained_rvalue -- --nocapture`: **3 passed**, including root/non-JS controls. Both used unchanged tree-equivalent main production. These are diagnostic checks, not a full-suite or repair claim.

### Task 1 — write source expectations and prove the harness

- [x] Add the shared helper and registrations with the smallest compiling stubs needed for meaningful tests. Capture helper tests failing behaviorally on wrong/missing/ambiguous anchors, UTF-8 endpoint errors and full-value comparison, then implement them. Compilation failure or zero selected tests is inadmissible RED.
- [x] Author fixture roles/ranges independently of production extraction and validate each against AST fields. Freeze a finite matrix and list every excluded form.
- [x] Add the aggregate desired regression. Run it explicitly with `--ignored` on unchanged main production and capture **every** wrong-owner row before its final failure. Preserve the exact test bytes/hash for later replay.
- [x] Distinguish RED for the new harness's behavior from RED exposing the existing production defect. Adding an audit file is not itself proof of a production regression.

### Task 2 — classify both contamination routes

- [x] Query-route hypothesis: nested captures pass containment; falsifier: no accepted nested capture and no wrong span. Log capture kind/range and nearest enclosing callable fields.
- [x] Recursive-route alternative: accepted outer capture recursively crosses a callable boundary; discriminant: the outer capture remains while the false parameter/body token appears in its child collector. Probe each private names/paths/spans route.
- [x] Add positive neighboring/own-callable controls, return-only asymmetry, root and erased-context controls. Aggregate assertions so a first-error gate cannot hide the remaining population.
- [x] Record results as WRONG only with an exact input/token/queried owner and incorrect output. Other unanswered design obligations are SMELL or explicit deferred scope.

### Task 3 — trace consumers without repairing them

- [x] Capture real DFG full endpoint tuples, zero-width fallback anchors, graph index choice and confidence on the representative fixture subset.
- [x] Compare normal nested captures and shadowed/noncapturing nested parameters; demonstrate whether raw-owner contamination changes any exact CPG edge or FullFlow result. A parser-level WRONG does not authorize a stronger graph claim.
- [x] Test the argument-span and exact parameter-node negatives regardless of whether an optional counterfactual repairs a synthetic flow. Do not weaken `compute_param_def_nodes` or `argument_var_node_in_span`.
- [x] Run full/incremental epochs and serialization controls; compare multiplicity, endpoint identity, labels, call resolution and refusal, not counts alone.
- [x] Examine the effect of removing a raw span on `get_use` and `use_byte` fallback. A counterfactual must be labeled as such; do not claim production safety from a manually edited graph.

### Task 4 — freeze a production-repair design

- [x] Publish an explicit acceptance table for queried callable, capture node and recursively visited token. State separate policies for own body, own signature/default, nested body/default, eager definition expressions, root inventory and unsupported forms.
- [x] Recommended future seam: an **rvalue-specific scoped traversal**, leaving general-purpose identifier collectors intact unless all their consumers are proved compatible. Document a proposed `RvalueQueryScope` (test/design only here) distinguishing `LegacyInventory` from `CallableBody { owner range, body range }` and explicit unresolved setup; unknown roots must not silently select a guessed owner.
- [x] Future design must address both capture acceptance and recursive descent. A proposed `rvalue_capture_owner(scope, capture) -> OwnerDecision` should distinguish `Owned`, `Nested`, `Signature`, `Outside`, and `Unsupported`; it cannot replace recursion barriers. A scoped value walker must preserve eager object-method keys while excluding method body/parameters. Derive any exact helper signature and lifetimes from current Tree-sitter types before dispatching repair.
- [x] Decide how conservative capture observations survive. Prefer retaining the separate scoped-reference/capture pathway with existing doubt labels if executable controls prove it; if it needs a new representation, identify that as a prerequisite slice rather than silently expanding this repair.
- [x] Retain names/paths/span return asymmetry and root inventory. Do not rename or globally change the existing callable-boundary registry to encode execution timing.
- [x] Specify future production edits at the source-map seams, cache impact (CPG bump required for changed stored DFG semantics; nav only if its stored facts change), desired test enablement and historical-audit expectation updates.
- [x] Verdict must be `READY_FOR_BOUNDED_REPAIR` with all admitted rows settled and exclusions explicit, or `DESIGN_BLOCKED` with the smallest unresolved semantic choice and discriminating evidence. No percentage-complete or real-project recall claim.

### Task 5 — verification, review and handoff

- [x] Freeze tests/source/executables and collect full test-suite totals. Use commands below and exact prerequisite pins from the prior committed verification receipt.
- [x] Re-run identical desired tests on the pinned base and proof candidate in the same environment. Production observations must agree because the slice changes no runtime behavior; unexpected differences require investigation, not rebaselining.
- [x] Request Terra high review using the separate prompt. Round 1 reviews the complete bounded artifact; round 2 reviews targeted fixes plus their blast radius. At the cap classify convergence before further action. Closed enumerable findings receive bounded fixes on the existing artifact; open-class findings go to design. Never restart the artifact without explicit owner authority and a written reason.
- [x] Capture final file hashes, canonical rows, baseline failures, exclusions and reviewer verdict separately from gate results. Refresh the handoff at each stable point and snapshot it with the proof package. Leave commit/publication to the controller under this machine's instructions.

Implementation evidence before independent review: readout and baseline receipt at
`docs/eval/receiver-closure/2026-09-13-nested-execution-owner-proof.md` and
`docs/eval/receiver-closure/2026-09-13-nested-execution-owner-proof-baseline.json`;
raw logs, base control and frozen binaries at
`/private/tmp/prism-nested-owner-proof-Rjzipk`.

## 8. Verification commands and honest completion

Prefix all fail-first pipelines with `set -o pipefail`; read test output and assert nonzero selected-test counts. Keep bounded RED output and full-suite logs separate. Do not modify input pins or increase test budgets merely to get green.

```bash
cargo test --offline --lib nested_execution_owner -- --nocapture
cargo test --offline --lib nested_execution_owner_desired_contract -- --ignored --nocapture
cargo test --offline --lib namespace_flow -- --nocapture
cargo test --offline --lib contained_rvalue -- --nocapture
cargo test --offline --no-fail-fast
cargo test --offline --no-fail-fast --features mcp
cargo test --offline --no-fail-fast --features 'mcp detached-owner-audit'
cargo test --offline --examples --features 'mcp detached-owner-audit'
cargo fmt --all -- --check
cargo clippy --offline --all-targets --features 'mcp detached-owner-audit'
git diff --check
```

The explicit desired command is expected RED until the next production slice; enumerate its exact wrong-owner population. The ordinary suites must include all new proof tests except that one declared known-gap test. Preserve `resolution_test::slice_elem_variant_reserved`. No unreported ignores, skips or zero-test filters.

Run the project's Node/authority/Python gates using the command and pin inventory in `docs/eval/receiver-closure/2026-09-12-contained-rvalue-captures-baseline.json` and its referenced receipt tooling. Inspect that tooling before reuse, rebind source and executable hashes, and write a new receipt; never overwrite the old one. Use `node --import tsx` where TS execution is required. Restore missing verification inputs only through an authorized acquisition route; otherwise run the largest available subset and name exact exclusions.

Because this slice registers tests in `src/ast.rs` and `src/cpg/build.rs`, honor the project Tier-A gate conservatively even though runtime semantics are unchanged:

```bash
cargo build --release
# From eval/, immediately after this worktree's successful rebuild:
uv run tier-a --matrix-only --allow-stale-sut
uv run tier-a --quick --allow-stale-sut
```

Use the previously documented bounded quick-run mechanism, preserving timeout output and reporting INVALID/no verdict truthfully. Do not launch a full multi-corpus run; it is human-triggered. No rebaseline. Historical #314 results are 4,501 default / 4,694 MCP / 4,717 owner-audit Rust passes, 32 examples, 786 Node, 40 authority, 940 Python with one live-adoption skip, and 159 matrix passes. These are reference totals, **not required new exact totals or current verification**. Its quick run timed out without a report, three historical Node tests lacked `real-sites.jsonl`, and one initial Node budget failure was retained despite a successful retry. Re-evaluate availability now and carry forward any unresolved exclusion; do not turn inherited totals into a new done claim.

Completion of this slice means the harness, evidence and bounded design are reviewable and verified with explicit limits. It does not mean the production ownership defect, same-line collisions, workstream 3 as a whole, or overall accuracy is solved.

## 9. Independent review plan

Review model: **Terra high**. Implementation model: **Sol high**. These are selected from the owner's supplied choices; no agent is dispatched by this planning turn.

Reviewer reads the pinned main source and actual candidate diff before relying on the implementation readout. Inspect fixture oracle independence, exact UTF-8/source/owner identity, selected-test counts and matching base/candidate test hashes. Challenge both the capture and recursive routes, own-default semantics, computed method keys, unknown/unindexed callables, whole-root behavior, conservative capture labels, fallback anchors and API asymmetry. Confirm production code/cache/index/parameter/call admission are unchanged. Verify that the proposed repair does not silently broaden this proof slice.

Replay focused passing proof tests and the explicit desired RED; inspect the generated rows. Re-run representative full/incremental, capture/refusal and wrong-token controls. Check full-gate receipts against commands, pins, source/executable hashes, counts and named exclusions; a full re-run is needed only if evidence is missing, stale or changes invalidate it. Run the same-environment base for any claimed regression. For every finding report WRONG or SMELL, confidence, exact input/state and incorrect result when applicable, source location, evidence that would raise/lower/collapse it, and bounded fix/test. Report WRONG first. A SMELL without a constructible wrong result is not a blocker; inherited WRONG can be downgraded only by mechanism-level impossibility proof.

Output `APPROVE`, `FIX-FIRST`, or `DESIGN-BLOCKED`, counts, proof strength and what was not verified. Review approval of a proof with known desired RED is not production-repair approval. Give the production-design verdict separately. Review cap is two rounds; review never authorizes publication.
