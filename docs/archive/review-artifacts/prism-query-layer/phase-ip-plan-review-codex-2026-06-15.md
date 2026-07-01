BLOCKER

1. Task 4/6: Full-interface satisfaction is not decomposed.
Issue: The plan’s `(iface, method) -> satisfiers` map can admit a type that has only one method of a multi-method interface.
Fix: Compute full canonical interface satisfaction first, then populate method entries only from full-interface satisfiers. Add a multi-method partial-satisfier regression.

2. Task 4/6: Go provider migration is incomplete.
Issue: Existing `subtypes_of` and `resolve_dispatch` still depend on `data.satisfaction`; replacing it with `sat_keys` or leaving it name-only breaks or preserves false positives.
Fix: Derive compatibility `satisfaction` from signature-confirmed full-interface satisfiers, and make `resolve_dispatch` delegate to the same admission/RTA/fallback logic.

3. Task 5/Step 3: Go liveness for `&U{}` is wrong.
Issue: The snippet checks `starts_with('&')` on the composite literal type text, but `&U{}` is a unary expression around a composite literal whose type is `U`.
Fix: Detect address-of via the `unary_expression` parent/ancestor or operator/operand shape. Also avoid filtering valid unexported in-repo types by uppercase-only casing; filter against known concrete Go types.

4. Tasks 8-10: Selector call snippets use the wrong callee name.
Issue: Current extraction stores `r.Go()` as callee `Go` with qualifier `r`, not `r.Go`. Task 8 notes this, but later snippets still use `site_in(..., "r.Go")`.
Fix: Use `"Go"` in call-site lookup and assert qualifier/receiver separately where needed.

5. Tasks 9-11: CPG test snippets are not compile-executable.
Issue: `CpgContext::build(&files)` has the wrong arity, and `has_exact_call_edge` / `exact_callees_of` do not exist. `#[cfg(test)]` accessors on `CpgContext` would not be visible from integration tests.
Fix: Use `CpgContext::build(&files, None)` or `CodePropertyGraph::build(&files)`, and put edge-inspection helpers in the integration test using public CPG graph/node/edge APIs.

6. Task 12: CHA regression is a placeholder.
Issue: `cha_does_not_mint_cross_language_edge` is empty/ignored, so it cannot prove the C++-only CHA filter.
Fix: Add a real mixed Go/C++ or helper-level test using a manual `TypeDatabase`, and assert no C++ CHA seed can mint an Exact edge to a Go same-named function.

7. Task 13: Harness attribution is not spec-complete.
Issue: The plan adds `resolution_kind` fields but defers the manifest/gate apparatus, while the spec still assigns that to PR-1. Current probe replay stores only site triples, so dispatch attribution is lost in replay and pending records.
Fix: Either amend the spec to defer manifest/gate work, or persist per-site `resolution_kind` / `dispatch_kind` in probe JSON and pending records now, with replay tests.

8. Task 13/Step 2: Python test path/imports are wrong.
Issue: The test imports `eval.tier_a...` from inside `cd eval`, but the package is exposed as `tier_a`; existing tests live under `eval/tests`.
Fix: Put the test under `eval/tests` and import from `tier_a.model` / `tier_a.sut`.

MAJOR

9. Task 10: Barrier/DataFlow gating coverage is underpowered.
Issue: The plan checks only call-edge exact callees for one live-intersection case. It does not cover empty-live fallback or Step-5b DataFlow fan-out.
Fix: Add both live-intersection and empty-live fallback variants, and assert call edges plus arg-to-param DataFlow edges equal the satisfier set with no same-name non-satisfier leakage.

10. Task 4: Receiver-kind and generics/type-set gates are underspecified.
Issue: The plan can be read as excluding value receiver methods from `*T`’s method set, and tests only a generic interface.
Fix: Define `set_value(T)` and `set_ptr(T)` explicitly, with `set_ptr(T)` including value receiver methods. Add tests for mixed pointer/value satisfaction, generic concrete receivers/type specs, and type-set interfaces failing closed.

11. Task 4: Promoted-method satisfaction must preserve existing embedding semantics.
Issue: Existing behavior has shadowing/diamond handling; the rewrite does not explicitly reuse or preserve it.
Fix: Reuse/refactor `promoted_struct_methods` or prove equivalent behavior. Add embedded satisfier, pointer promoted satisfier, shadowed method, and equal-depth ambiguity tests.

12. Tasks 7/13/15: Telemetry is incomplete.
Issue: `fanout` and `fallback_fired` are computed then discarded; `call-stats` reports only gaps/overapprox. `CrossPackageBareName` is declared but never emitted.
Fix: Store aggregate fanout/fallback counters on `CallGraph` or stable call-stats counters, and emit `CrossPackageBareName` when canonicalization strips package qualifiers.

13. Task 11/Step 3: Cache guidance references a nonexistent promoted-alias round-trip.
Issue: There are cache tests, but no promoted-alias round-trip to mirror.
Fix: Provide concrete cache test code for populated `interface_impls`, plus a forced version-8 miss after bumping to 9.

14. Task 13/Step 1: Rust test command targets the wrong binary.
Issue: `call_stats` tests live under the `cli` test binary, not `integration`.
Fix: Use `cargo test --test cli call_stats`.

MINOR

15. Task 3: Array canonicalization disagreement.
Issue: Executability is right that `[N]T` matches the current spec, but Coverage is right that this can mint false Exact edges because Go array length is type identity.
Fix: Either preserve literal array lengths or explicitly update the spec/plan to accept this precision tradeoff and add a negative/gap test for unsupported lengths.

16. Task 7/build_scoped: Scoped behavior should be pinned.
Issue: This is not a blocker because the spec accepts scoped-subset dispatch and nav builds whole-repo, but the behavior is easy to misread.
Fix: Add a scoped-mode regression documenting that subset dispatch may skip absent target nodes.

17. Task 7/13 commit lists are incomplete.
Issue: Task 7 omits `tests/integration/resolution_test.rs`; Task 13 omits the CLI call-stats test file.
Fix: Fix the literal `git add` lists if the plan is meant to be followed step by step.

Verdict: Not executable as-is; fix the satisfaction model, liveness snippet, selector/CPG test snippets, CHA regression, harness persistence/gating decision, telemetry, and test layout before building.