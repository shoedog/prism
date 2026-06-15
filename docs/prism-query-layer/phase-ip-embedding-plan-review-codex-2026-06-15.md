BLOCKER

1. Task 5 / missing scoped-build task: build_scoped contradicts the spec. The plan says scoped mode is intentionally unchanged, but the spec requires whole-repo promotion across build, incremental, and scoped builds; current scoped CPG builds from filtered files only, so aliases can miss out-of-scope embedded types or direct shadows. Fix: add a scoped-build task that computes promotion/owner data from all files while assembling the scoped graph, plus a regression where run(w Wrap) is in scope and the embedded type or method is out of scope. Disagreement resolved: Coverage/Executability are right over the plan self-review because the implementation path demonstrably passes only filtered files into build_enriched.

2. Task 1 Step 3 / Task 4 Step 3b: global visited breaks promotion semantics. collect_promotions uses one visited set per outer struct, which suppresses duplicate equal-depth diamond paths and can also hide a shallower path reached after a deeper traversal. Fix: use path-local cycle detection, preserve all valid promotion paths through grouping, and add A embeds B,C; B,C embed D; D.M tests asserting ambiguity, no alias, and embedding_gaps["ambiguous"] == 1.

3. Task 3 / Task 4: Go generic key normalization is incomplete. The plan normalizes promoted alias keys and recovered receivers, but direct method owner keys still flow through method_metadata -> owner_key, and provider receiver keys still come from strip_pointer only; Go [...] generics will not match consistently. Fix: apply normalize_go_struct_key or equivalent Go-specific stripping where Go receiver/method owner keys are extracted and where provider method keys are stored; add generic direct-method-shadow and generic embedded-method tests.

4. Task 5 Step 1: incremental regression is underpowered and copy-paste unsafe. The test uses cpg.call_graph(), but CodePropertyGraph exposes call_graph as a field, and the single-file fixture does not prove stale aliases are removed when the promoted target’s FunctionId file is unchanged. Fix: use cpg.call_graph, rewrite the regression as cross-file with base.go holding Base.Ping and wrap.go removing the embedding, and assert promoted_aliases, methods[(Wrap,Ping)], and w.Ping() resolution are cleared.

MAJOR

5. Task 4 Step 1 / Step 3b: direct field shadowing is missing. Go selector resolution includes fields as well as methods; a Wrap field named Ping should block or make ambiguous a promoted Base.Ping, but the plan only checks direct methods. Fix: expose/use provider field data in promotion filtering and add field-shadow and field/method ambiguity tests.

6. Task 4 Step 3e: EmbeddedPromotion labeling is too narrow. The plan relabels only the P6-lite recovered receiver path, but Go receiver-variable/self calls can resolve through another owner_lookup path and be labeled SelfReceiver even when the hit is a promoted alias. Fix: centralize promoted-alias relabeling after all owner_lookup paths that can hit (owner, method), or add explicit handling and tests for the Go receiver-var path.

7. Task 4 Step 1: direct-method line assertion is wrong. The fixture’s direct Wrap.Ping is on line 7, not line 6, so the test will fail for the wrong reason. Fix: assert start_line == 7 or, better, assert the resolved target owner/FunctionId identifies Wrap.Ping.

8. Task 5 Step 1: cpg.call_graph() accessor does not exist. The snippet will not compile as written. Fix: replace cpg.call_graph().promoted_aliases with cpg.call_graph.promoted_aliases.

9. Task 7: telemetry is surfaced but not meaningfully validated. The CLI test only checks embedding_gaps is an object; it does not prove the required ambiguous counter works. Fix: add a resolution-level ambiguity assertion and a call-stats fixture verifying embedding_gaps["ambiguous"] == 1.

10. Tasks 8 and 9 plus earlier test steps: validation commands can mask failures. cargo test ... | tail, uv run ... | grep, and cd eval && ...; cd .. can return success from tail, grep, or cd instead of the real command. Fix: avoid pipes for pass/fail checks, or use set -o pipefail; run Tier-A in subshells such as cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut).

11. Task sequencing / commit steps: commits happen before the repo-required Accuracy Harness. AGENTS requires cargo build --release plus Tier-A matrix before committing changes touching call_graph, navigation, CPG, or call resolution. Fix: move commits until after the required build and matrix run, or insert that harness before each affected commit; keep Tier-A quick before review.

12. Spec / acceptance text: pkg.Wrap and generic-gap requirements contradict the design. The spec defers cross-package pkg. normalization but acceptance asks for pkg.Wrap, and one sentence still says generic structs are a recorded gap while later sections say they resolve by stripping [...]. Fix: either implement dot-qualifier normalization and tests, or remove that acceptance; update the stale generic-gap sentence.

MINOR

13. Task 4 / coverage: transitive and pointer/addressable acceptance coverage is thin. Provider-level tests alone do not prove the resolver, CPG, and navigation seams handle transitive embedded methods or addressable pointer-receiver cases. Fix: add integration tests for transitive resolution and the addressable pointer-receiver case required by the spec.

14. Dropped draft finding: let fid = shallowest[0].clone() is not a compile blocker. Method resolution can clone the underlying FunctionId from &FunctionId, so Executability is right to drop it; the real fix is path-local traversal, not changing that clone. Optional clarity fix: write (*shallowest[0]).clone().

15. Dropped draft finding: Task 7’s serde_json::json! embedding_gaps expression is not a move-from-&CallGraph bug. Coverage is right because json! borrows arbitrary expressions for serialization. No code fix needed.

16. Dropped draft concern: test filter suspicion is unsupported. resolution_test and call_stats_test are real integration/CLI modules, so the named filters should match once the test names are corrected. No fix needed beyond avoiding failure-masking pipes.

Verdict: not executable as-is; fix the scoped-build gap, generic key normalization, path-local promotion traversal, unsafe snippets, shadowing/labeling coverage, and validation-command sequencing before building.