# Phase-IP Foundation (PR-1) plan — Round PLAN-REVIEW, SOUNDNESS/COVERAGE/EXECUTABILITY (claude opus, operator-subagent)

> Operator-driven subagent (the a2a-bridge claude leg is the open `{{reviewer_claude}}` defect).
> Complements the codex plan-review (`phase-ip-plan-review-codex-2026-06-15.md`). Read-only, verified
> against the post-`e03f547`+`d64c957` tree and the pinned tree-sitter-go 0.23.4 grammar.

Verdict: **needs changes** — one BLOCKER + three MAJORs + minors; architecture sound and faithful to
the spec (seam, wiring, cache, ExactOnly linchpin, harness plumbing all verified against real code).

## BLOCKER — Task 4 rewriting `compute_satisfaction` breaks an existing test (red tree mid-plan)
`tests/ast/type_provider_test.rs::test_go_interface_satisfaction` asserts
`provider.subtypes_of("Reader").contains("File")`. `subtypes_of` (go.rs:700-707) and
`DispatchProvider::resolve_dispatch` (go.rs:726-778) both read the bare-name, receiver-blind
`data.satisfaction` (go.rs:464-480). Task 4 says "replace the name-only body" + store in new
`sat_keys`/`dispatch_gaps` — silent on preserving `data.satisfaction`. The fixture uses pointer
receivers (`func (f *File) Read`), so an admission-keyed map would return `*File` not `File`. Fix: keep
populating bare-name `data.satisfaction` (derived from the new signature-confirmed full-interface
satisfiers) **in addition to** `sat_keys`; add `cargo test --test ast type_provider_test::` as a Task-4
regression guard.

## MAJOR — `CpgContext::build` is two-arg
`pub fn build(files, type_db: Option<&TypeDatabase>)` (src/cpg/context.rs:59-62). Tasks 9/10 call
`CpgContext::build(&files)` → compile error. Fix: `CpgContext::build(&files, None)`.

## MAJOR — Task 5 `&T{}`→`*T` is broken in the plan's own impl
`&T{}` parses as `unary_expression`(`&`) wrapping a `composite_literal` whose type text is `T`; the
plan's `is_ptr = type_text.starts_with('&')` on the composite_literal is never true → `&U{}` emits only
`U`, failing the task's own `assert!(s.contains("*U"))`. Fix: detect `&` on the enclosing
`unary_expression` (add a `unary_expression` arm or check `node.parent()`).

## MAJOR — Task 3 `channel_type` element field
`channel_type` exposes its element via field **`value`** (node-types.json), not a trailing positional
child. Fix: `node.child_by_field_name("value")`. All other canon_type field/kind names verified correct
(`slice_type.element`, `array_type.element`, `map_type.{key,value}`, `parameter_declaration.{name(multiple),type}`,
`variadic_parameter_declaration.type`, `method_declaration.{name,parameters,result}`, `function_type.{parameters,result}`,
`generic_type`, `type_spec.type_parameters`). `method_spec` is dead in 0.23.4 (it's `method_elem`); the
test helper already pairs both, so it works — drop `method_spec` from prose.

## MAJOR — Task 7 misses a 5th init site
`promoted_aliases` is initialized in **5** places (call_graph.rs:93, 213, 714, **~1013**, + test), not
the 4 the plan lists. Init `interface_impls`/`interface_gaps`/`interface_overapprox` at ~1013 too, or
that constructor leaves the fields uninitialized (compile error).

## MINORs
- Tasks 9-11: `#[cfg(test)]` accessors + `git add` target `src/cpg.rs` (a 39-line façade); `CpgContext`
  lives in `src/cpg/context.rs`. **Also:** `#[cfg(test)]` methods on a library type are NOT visible from
  integration tests (separate crate) — inspect edges via the **public** CPG graph API in the test, or add
  non-`cfg(test)` accessors.
- Task 12: filtering only the candidate index (build.rs:536-543) is the load-bearing fix (the override
  *target* comes solely from `virtual_method_nodes`); the seed gate is belt-and-suspenders. `RecordInfo.file`
  (type_db.rs:55-56) exists → prefer a tdb-owned-file set over an extension heuristic.
- Task 4 type-ripple: `extract_method_signature`/`extract_func_signature` → gap-returning ripples into
  `GoMethod.signature` (String → Result) and `GoInterface.methods`; embedding (`promoted_struct_methods`)
  uses `m.name` not `.signature`, so it's unaffected. Most under-specified step; tests are the oracle.
- Task 5: grouped `var ( x T; y U )` wrapped in `var_spec_list` is skipped (single `var x T` works);
  RTA-safe (fallback covers).
- Coverage: §13.3 (anon-iface + unknown-node gaps + no-edge + counter), §13.4 (value-only non-satisfaction),
  §13.5 (factory-body, non-local gap), §16 (direct Step-5b DataFlow fan-out) partially elided — add them.

## Verified sound (no action)
Task 1 (`as_str` resolution.rs:33/54); Task 2 (key contracts after owner_key:86); Task 7 wiring
(fields call_graph.rs:75-78, apply at :717, remove_files clear :768, build_incremental :192,
CACHE_VERSION cpg_cache.rs:49 + test :511, `build_scoped`→`build_enriched`→`CallGraph::build` so covered);
Task 8 seam (resolution.rs:428 shape matches; `exact(ids.iter(),kind)` type-checks; lifetimes sound);
Task 8 fixture flip (typed-param receiver, constructs nothing → fallback → confidence-blind matrix flip
valid); Task 9 ExactOnly oracle (`--confidence exact` filters at queries.rs:296; Step-5 materializes the
`Call(Exact)` edge; linchpin valid); Task 13 harness (`_why` sut.py:61, CallEdge model.py:35-41,
Adjudication adjudication.py:26-44; trailing-defaulted fields backward-compatible; prism emits
`{"Resolution":{"kind":...}}` in `why[]` — types.rs:61, queries.rs:326/487; call-stats insertion point
queries.rs:46 correct).
