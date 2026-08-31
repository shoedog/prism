# Roadmap #13a — sound Go Level-3 bare-function callbacks

**Status:** design settled; implementation remains subject to the red-first and same-environment gates in §9
**Recorded:** 2026-08-31
**Exact base:** `a3768a9d40903c32251346d196107d48af25eb47` (`origin/main`, PR #219 merge)
**Scope:** Go-only B1 callback values: a bare identifier naming an in-repo free function, passed to a proven in-repo free-function HOF at a callable parameter slot

## 1. Decision and slice boundary

Implement one precision-first slice of roadmap #13. A new `ParameterCallback` edge may be minted only for this chain:

1. a non-test Go source call site is occurrence-proven to call exactly one in-repo Go free-function HOF and independently resolves to that same `FunctionId` at `Exact` confidence;
2. one positional argument is an AST `identifier`, not merely text that resembles one;
3. that identifier is proven to name exactly one visible, non-test, same-package free function at the argument occurrence;
4. the destination parameter slot is proven callable from a literal `func(...) ...` type or an in-repo named type with a uniquely resolved function underlying type;
5. the callback parameter is invoked directly in the HOF's own named-function body, is never rebound, and is not invoked inside a nested `func` literal;
6. the parameter type and argument function have identical strict canonical Go signatures; and
7. the synthetic site carries the target's full `FunctionId` in `pre_resolved_target`.

Every failed or unavailable proof drops this Level-3 candidate. It does not fall through to a name-only callback resolver.

This slice deliberately does **not** resolve:

- qualified function arguments (`pkg.F`), method expressions or method values (B2);
- method HOFs, interface-dispatched HOFs, or HOF values held in variables;
- local or package variables holding function literals/functions (B3);
- anonymous function arguments (B4);
- function-returning calls, struct fields, `nil`, composite values, or spreads (B5);
- variadic callback slots, generic functions/types, external named function types, `_test.go` participation, or any non-Go language;
- assignments as callback-value evidence, even when a textual dominance argument looks plausible; or
- calls through a callback parameter from inside a nested `func` literal.

Those exclusions are separate measured slices, not fallback behavior.

## 2. Measured value and evidence limits

The retained measurement packet is:

`/Users/wesleyjinks/code/prism-lane-artifacts/2026-08-23-next10/13-hof-sweep/`

| File | SHA-256 |
|---|---|
| `hof-sweep-results.json` | `24d8607567a8b0c8d28cd39d675b19593bb356217668b3bdbdde3f091d64e05c` |
| `hof-sweep-samples.md` | `2abfa54f82e53d156b25362c4591d2d611bc585df8f6a3ee603f934d9277328d` |
| `hof-sweep-slots.tsv` | `748af6e2da085bf4c58f3ca1b62f4ed2675cc1618119a478aab3a184b9a7f665` |
| `hof-sweep.go` | `c7f975a9c298956429c44e8e96f1f19605ef0059b5969accc31e4029b4606416` |

The five corpora had zero parser errors and zero files skipped for size. Their strict unambiguous non-test B1 floor is:

| Corpus | B1 floor |
|---|---:|
| Caddy | 49 |
| Prometheus | 6 |
| etcd | 3 |
| Hugo | 9 |
| zap | 0 |
| **Total** | **67** |

The gate is met, but the packet is a syntactic `go/parser` census, not an oracle. It does not type-check, evaluate build tags, prove receiver types for HOF method calls, resolve default-import package names, or model all shadows. Implementation acceptance therefore requires site-level reconciliation of all 67 floor rows to either an emitted edge or one stable fail-closed reason. It does not require blindly emitting 67 edges. Method HOF rows and callers whose default external import names are unavailable are expected, named exclusions in this first slice.

Caddy is the primary positive control: all 49 floor sites are B1. The two main HOFs use in-repo named function types (`UnmarshalHandlerFunc` and `CommandFunc`), so accepting only literal `func` parameter syntax would fail the measured gate.

## 3. Rebound current substrate

Current main already provides most identity plumbing:

- `compute_indirect_call_sites` runs after Go dispatch, registrations, receiver rematerialization, framework entries, and JS export resolution; Level 3 is currently a terminal `(extra_sites, 0)`.
- `CallSite::pre_resolved_target` is serde-defaulted, is part of `CallSite::cmp_key`, and is also part of navigation's `CallSiteKey`.
- `resolve_call_site_full` turns a present, live `pre_resolved_target` into one `Exact` `ResolutionKind::ParameterCallback`; a missing target drops `UnknownName`.
- `ParsedFile::call_argument_texts_and_spans_at` gives positionally ordered argument spans keyed by exact call start byte and callee text.
- Step-5b's production and serial-reference paths, plus reasoning seed extraction, re-read arguments using `(site.start_byte, site.callee_name)`. A synthetic target name therefore cannot recover arguments from a source call through a differently named parameter without carrying both identities.
- `function_parameter_slot_occurrences` implements prefix-preserving Go grouped-name semantics and stops before a variadic or unrepresentable declaration.
- `GoTypeProvider` already has import-aware canonical signature extraction and `GoAliasResolver`; `go_func_type::declaration_resolves_to_func` proves literal/named function underlying types, but retains only a boolean.
- `resolve_go_bare_value_ref` proves same-package free-function identity with build-profile filtering and excludes methods, but it does not by itself prove the identifier occurrence is free of local/file/package namespace collisions.
- `CallGraph::go_dot_import_files` is the canonical dot-import poison set.
- CPG cache version is `54`; navigation resolved-edge sidecar version is `23`.

Do not revive the pre-#173 generic Level-3 name resolver. Reuse the identity-bearing substrate above and add only the missing callable-signature and occurrence-proof facts.

## 4. New facts and ownership

### 4.1 Callable signatures

Add serializable callback-specific facts to `CallGraph`:

```rust
pub struct GoCallbackParameterFact {
    pub slot: usize,
    pub name: String,
    pub signature: String,
}

pub type GoCallbackParameterIndex =
    BTreeMap<FunctionId, Vec<GoCallbackParameterFact>>;

pub type GoFreeFunctionSignatureIndex =
    BTreeMap<FunctionId, String>;
```

Names are illustrative; the contract is the key. Both maps are whole-program derived by the import-path-aware `GoTypeProvider` used in `apply_go_interface_dispatch_with_scope_inputs`, captured onto `CallGraph` before indirect recomputation, and serialized for cache parity.

The provider performs two logical phases:

1. extract all raw named-type declarations and a strict canonical signature for every direct literal-function RHS; then resolve uniquely selected alias/defined-type chains cycle-safely to an underlying callable signature;
2. scan Go free-function declarations, producing canonical free-function declaration signatures and positional callback-parameter facts after the named-type universe is complete. Methods are not indexed for #13a.

A direct literal callback type and a resolved named callback type normalize to the same signature form as a function declaration: canonical parameter types plus canonical result types, with parameter/result names removed, grouped names expanded, and variadic shape retained. Generic syntax, parse recovery, `_test.go` provenance anywhere in the chain, unresolved alias chains, multiple visible declaration variants, cycles, or an external named type produce no callback fact.

The existing boolean `underlying_func` may remain as a compatibility projection, but it must be derived from the same signature resolver. The implementation must not maintain a second independent truth for “callable.”

### 4.2 Strict signature identity

Do not call the existing permissive `canon_signatures_match` unchanged. Its legacy `Bare`↔`Bare` name match is acceptable for older interface behavior but cannot authorize a new Exact callback edge across package namespaces.

Add a callback-strict comparison mode with these rules:

- punctuation, arity, parameter order, result order, pointer/container/channel/variadic shape must match exactly;
- predeclared types compare after the existing normalization;
- two locally proven names compare only by the same proven package identity;
- local and qualified names compare only when the local import path and qualified import path prove the same package;
- qualified names compare by full import path plus type name; and
- any non-predeclared unproven bare leaf makes the signature unfit for #13a.

Callback signature extraction must also use strict import bindings. Explicit aliases are exact. A default import name is exact only when the import path resolves through the module graph to one in-repo non-test package clause. Do not use the path basename or the existing conventional major-version inference as proof; an external/default qualifier whose package name is unavailable makes that signature unfit for #13a.

The implementation may extend the current canonical token representation or introduce a callback-specific wrapper, but it must not compare raw source spelling and must not inherit bare-name equality.

### 4.3 Bare-value namespace proof

Add one callback-specific, conservative `go_bare_function_value_at` proof. It receives the caller `FunctionId`, argument byte span, and identifier, and returns either one exact free-function `FunctionId` or a stable drop reason.

The proof requires all of the following:

- caller and candidate are non-test Go files with nonempty package clauses and zero `ERROR`/`MISSING` nodes;
- the caller file is absent from `go_dot_import_files`;
- the argument node is exactly one Go `identifier` occupying the recorded argument span;
- `resolve_go_bare_value_ref` returns one visible same-package free function and no method;
- the enclosing named function is reconstructed by exact byte/kind identity;
- a whole-function conservative census finds no other binding or reassignment of the identifier anywhere in that function; and
- a package/file namespace census finds no potentially visible non-function declaration or caller-file import binding with that name.

The whole-function census intentionally over-suppresses later and sibling-block bindings. It must cover at least:

- parameters, named results, and a method receiver;
- `:=`, `var`, `const`, and local `type` declarations;
- range variables, type-switch aliases, and select/receive short declarations;
- every assignment LHS to the name, including multi-LHS forms;
- nested `func` literal parameters/locals when they create uncertainty for the outer attribution; and
- parse recovery as `Unknown`, never “no binding.”

The file/package census must cover every package-block declaration kind (`func`, `var`, `const`, `type`) with defining-file build/profile provenance. Ordinary imports are file-scoped: explicit aliases bind that alias, `_` binds nothing, and `.` is the existing whole-file poison. A default import name is accepted only when the module graph resolves it to exactly one in-repo non-test package clause. Any other default import has an unknown local name and poisons every #13a bare-value proof in the caller file; the path basename is not proof of the imported package declaration's name.

Use one reusable clean-package predicate for the caller/target package, HOF package, and every named callable-type declaration package. Any ordinary non-test Go file in the directory with an empty package clause or parse recovery, or any mixed ordinary package clause, poisons the package. Potentially visible but not exactly selectable build variants poison the relevant declaration name. `_test.go` files are outside this slice and neither donate facts nor poison the non-test package.

This finite census is intentionally stronger than `function_local_value_bindings` and the receiver recovery walker. Neither existing helper alone is the authority for #13a.

### 4.4 HOF call-occurrence proof

Singleton `resolve_call_site_full` output is necessary but not sufficient: the existing Go call ladder can still see a package function when a local value shadows the callee name. Before consulting parameter facts, prove the source occurrence itself:

- for an unqualified call, run the same exhaustive local/file/package namespace proof as §4.3 on the callee identifier and require it to return the exact HOF `FunctionId`;
- for a qualified call, require the qualifier to be an explicit import alias or the exactly proven package name of one in-repo import, require no local binding of the qualifier anywhere in the caller function, and require that import path/package identity to own the exact HOF `FunctionId`;
- reject receiver/method calls, dot imports, unresolved default imports, and any disagreement between the occurrence proof and `resolve_call_site_full`; and
- require the HOF, caller, and target packages to satisfy the reusable clean-package predicate.

Concrete guarded state: `func outer() { invoke := func(func()) {}; invoke(safe) }` must never use a package-level `invoke`'s parameter facts even if the legacy resolver reports that package function Exact.

### 4.5 HOF parameter-use proof

For each indexed callback parameter, inspect source `CallSite`s attributed to the HOF and accept an invocation only when:

- the site is Go, `origin == Source`, `kind == Call`, unqualified, and `callee_name == parameter.name`;
- the site lies in the named HOF node but in no nested callable boundary, including `func_literal`;
- the HOF node and defining file have no parse recovery;
- the parameter has exactly one declaration occurrence in its proven slot; and
- a whole-HOF conservative mutation/shadow scan finds no declaration or assignment of that name other than the parameter declaration; and
- the parameter's address is never taken or otherwise exposed through a pointer-producing expression.

One mutation, shadow, or address escape anywhere drops that parameter's entire Level-3 population. This loses path-sensitive recall but prevents an edge from a stale inbound value after `cb = other`, from an inner `cb := ...`, or from `mutate(&cb); cb()` where another function replaces the value through `*func()`.

### 4.6 Synthetic source-call identity

Extend `CallSite` with a serde-defaulted optional source callee/argument-lookup name (illustratively `source_callee_name: Option<String>`), plus one helper that returns `source_callee_name.as_deref().unwrap_or(&callee_name)`.

- ordinary source sites keep `None`;
- a #13a synthetic site stores the callback parameter name (`cb` in `cb(x)`) while `callee_name` remains the exact target function name;
- include the field in `CallSite::cmp_key` and navigation's `CallSiteKey`;
- Step-5b's production and serial-reference loops use the helper for `call_argument_texts_and_spans_at`;
- reasoning seed source-expression handling uses the same helper for the
  call-expression callee set, argument extraction, and same-name-argument
  comparison; and
- resolver and navigation target identity continue to use `callee_name` plus `pre_resolved_target`.

Do not change `call_argument_texts_and_spans_at` to fall back by start byte alone: its `(start_byte, callee)` key is an existing ambiguity guard. Do not set the new field on legacy Levels 1/2/4 in this slice; that would create an unrelated data-flow recall change requiring its own controls.

## 5. Minting algorithm

After Levels 1, 2, and 4 have collected their existing sites, run Go #13a as a separate block:

1. enumerate only original Go source sites; never consume a synthetic indirect site as inbound evidence;
2. run §4.4's HOF occurrence proof, then call `resolve_call_site_full` and require exactly one result total at `Exact` confidence for the same non-test free-function `FunctionId`, whose target has callback parameter facts;
3. obtain exact argument nodes by call start byte; reject missing, ambiguous, spread, variadic, or out-of-range slots;
4. for each callback slot whose argument is a bare identifier, call `go_bare_function_value_at`;
5. require a strict canonical-signature match between the slot and the returned free function;
6. for every clean direct invocation of that parameter in the HOF, create one synthetic `CallSite` at the invocation span with:
   - `caller = HOF FunctionId`;
   - `callee_name = target.name`;
   - `source_callee_name = Some(parameter.name.clone())`;
   - `origin = IndirectResolution`;
   - `pre_resolved_target = Some(target.clone())`;
   - no receiver identity/recovery; and
   - the source invocation's `line`, `kind`, `start_byte`, and `end_byte`.

The BTreeSet identity collapses repeated inbound evidence for the same `(HOF invocation, exact target)` while preserving different exact targets through `pre_resolved_target`. Multiple exact callback targets for one HOF invocation are legitimate context-insensitive may-edges: each target is proven by at least one exact inbound site. They are not a claim that one runtime invocation calls every target. The retained source name ensures every such site also maps the invocation's real argument occurrences to the exact target's parameter slots.

Set `level3_indirect_resolved` to the number of unique synthetic #13a sites actually installed, not the number of candidates or inbound rows.

## 6. Telemetry and custody dump

Add stable `call-stats` fields sufficient to reconcile the measured floor without reading debug logs:

- `go_level3_b1_candidates`;
- `go_level3_b1_exact_inbound_sites`;
- `go_level3_b1_accepted_inbound_sites`;
- `go_level3_b1_unique_targets`;
- `go_level3_b1_edges` (must equal the Go contribution to `level3_indirect_resolved` in this slice); and
- `go_level3_b1_drops`, a reason histogram.

Define the conservation key as one prospective `(source call span, callback slot)` after a name-based, telemetry-only join to known free-function HOF callback facts. Name matching may create a conservative candidate record but can never authorize an edge. Deduplicate this key before verdicts and require `go_level3_b1_candidates == go_level3_b1_accepted_inbound_sites + sum(go_level3_b1_drops)`.

At minimum, distinguish: non-Go/test, HOF not free, HOF occurrence shadowed/unproven, HOF not singleton Exact, strict import name unavailable, missing slot/argument identity, non-bare argument, dot import, caller/HOF/type package poison, local binding/mutation, file/package namespace collision, target unresolved/ambiguous, target not free/non-test, callable type unresolved, signature unproven/mismatch, HOF parse poison, callback parameter shadow/mutation/address escape, nested callable invocation, and no direct parameter invocation.

Extend `call-stats --dump-sites` (or add a directly adjacent stable dump) so accepted and dropped B1 inbound sites can be joined to `hof-sweep-slots.tsv` by corpus-relative `file:line`, HOF, slot, and argument. The dump must include the inbound start/end bytes, exact target `FunctionId`, HOF `FunctionId`, source callback-parameter name, invocation span(s), and drop reason. Aggregate conservation is necessary but not sufficient.

## 7. Lifecycle and cache parity

The callable indices and telemetry are whole-program derived. Follow the existing dispatch/registration lifecycle:

- clear them with the Go dispatch-derived state;
- recompute from the complete current `files` map before `recompute_indirect_calls`;
- do not merge partial callback indices from `build_direct_subset`;
- after incremental merge, rebuild the provider-derived indices and then clear/recompute all synthetic indirect sites;
- deleting or changing an inbound site, target function, named callable type, HOF parameter, or HOF invocation must remove stale edges; and
- `build_skeleton` remains direct-only with empty callback facts and no Level-3 sites.

Because `CallGraph`/`CallSite` gain serialized fields and resolved CPG plus Step-5b data-flow topology changes, bump CPG cache `54 -> 55`. Because navigation incoming/outgoing topology and `CallSiteKey` gain callback identity, bump the resolved-call-edge sidecar `23 -> 24`. Update the pinned version tests and history comments in the same change.

Required parity paths:

1. fresh full build;
2. CPG serialize/load round trip;
3. incremental add/change/remove compared byte-for-behavior with a fresh full rebuild; and
4. navigation sidecar save/load compared with a fresh index.

## 8. Red-first test matrix

Before production minting is enabled, compile and run watched tests that fail on exact base because positive `ParameterCallback` edges are absent. Keep negative fixtures green on exact base.

### Positive

- literal `func` parameter + same-package bare free function;
- named defined function type (`type Handler func(...)`) + matching bare free function;
- named callable chain/alias where every declaration and profile is uniquely proven;
- grouped parameters with the callback at a nonzero slot;
- two callable slots receiving different free functions;
- HOF in another package reached by an exact qualified call, with the argument function bare in the caller package;
- two caller packages with same-named callback functions: each exact `FunctionId` is retained and no cross-binding occurs;
- multiple inbound exact targets to one HOF invocation produce the exact target set; and
- multiple callback invocations in one HOF preserve distinct invocation spans.
- callback invocation arguments produce the exact target's Step-5b arg→param edges using the stored source callee name;
- reasoning at `cb(x)` suppresses `cb` as the call expression while preserving an unrelated variable whose name equals the resolved target; and
- `cb(cb)` retains the argument occurrence even though the source callee and argument names are equal.

### Negative / edge

- JS/TS/Python/Rust/C cases remain disabled;
- argument is qualified, a method value/expression, local func variable, func literal, return call, field, `nil`, composite value, or spread;
- caller is `_test.go`, has a dot import, empty clause, parse recovery, or uncertain build profile;
- caller has a default external import whose local package name is unproven;
- the HOF callee is shadowed by a local value, is a method/receiver call, or its qualified import binding is not exact;
- same name is bound by a parameter, named result, receiver, `:=`, `var`, `const`, local `type`, range, type-switch alias, select receive, import name, assignment, or nested literal;
- same package contains a competing `var`/`const`/`type` declaration or an empty-clause/parse-poison file;
- target is a method, test function, missing, multi-candidate, profile-conflicted, or signature-mismatched;
- callback parameter is variadic, generic, externally named, alias-cyclic, profile-conflicted, or signature-unproven;
- callback parameter is reassigned, shadowed, or address-taken anywhere in the HOF;
- invocation occurs only inside a nested `func` literal;
- inbound HOF resolution is absent, NameOnly, or multi-target;
- same-line inbound calls remain byte-distinct; and
- removing target/type/inbound/invocation in an incremental rebuild removes every stale edge.
- a synthetic site's target name differing from its source callback-parameter name preserves Step-5b and all three reasoning source-expression roles; no start-byte-only fallback is introduced.

Each new fail-closed branch needs its own negative fixture. Test names are not evidence: assertions must inspect exact target files/lines, confidence, kind, and both navigation directions.

## 9. Base controls and implementation gates

Cut an exact-base detached control at the implementation branch's actual base and use the same environment for branch and control.

Required gates:

1. watched RED: positives fail on base only because Level 3 is disabled; negatives pass on base;
2. watched GREEN after implementation;
3. focused unit/integration/cache/navigation/CLI tests;
4. `cargo fmt --all -- --check`;
5. `cargo check` and `cargo clippy --all-targets --all-features -- -D warnings`;
6. full `cargo test --all-targets --all-features --no-fail-fast`, with exact totals and exit captured;
7. immediate `cargo build --release`, then `cd eval && uv run tier-a --matrix-only --allow-stale-sut` and `uv run tier-a --quick --allow-stale-sut`;
8. five-corpus same-base `call-stats` plus the site dump, with every one of the 67 measured non-test unambiguous B1 floor rows classified accepted or dropped;
9. Caddy: the 49 pre-registered B1 rows must all be accounted for, and any drop must be adjudicated before merge rather than silently lowering the floor;
10. no new non-Go `ParameterCallback` sites and no change to unrelated resolution-kind counts beyond explained secondary effects; and
11. all four cache/navigation lifecycle paths in §7.

Do not rebaseline Tier-A. Put regressions, flip candidates, corpus deltas, and every measurement exclusion in the PR body. A suite or corpus failure is attributable only after the exact base fails or passes the same probe in the same environment.

## 10. Permitted implementation files

The expected bounded lane is:

- `src/call_graph.rs`;
- `src/ast.rs` and/or a new focused Go callback namespace module;
- `src/go_func_type.rs`;
- `src/go_type_alias.rs` and its focused submodules if strict callback signature/import mode is implemented there;
- `src/go_owner_partition.rs` if the clean-package/profile selector can reuse that authority without changing existing consumers;
- `src/go_concrete_receiver.rs` only if the existing compatibility projection must carry the one canonical callable-signature authority;
- `src/type_providers/go.rs`;
- `src/lib.rs` only to register a new focused module;
- `src/navigation/queries.rs`, `src/navigation/mod.rs`, and `src/navigation/call_edge_cache.rs`;
- `src/cpg/build.rs` and `src/reasoning/seeds.rs` only to consume the one canonical source-callee helper;
- `src/cpg_cache.rs`;
- focused tests under `tests/integration/`, `tests/navigation/`, `tests/cli/`, and Go language fixtures; and
- this spec, the roadmap row, the PR body, and the lane handoff.

If compiled reality requires changing public CLI grammar, non-Go resolvers, CPG/DFG construction semantics beyond ordinary call-edge consumption, or more than one additional prerequisite slice, stop and amend/review the design before editing those areas.

## 11. Implementation sequencing

1. Add strict callable signature extraction/index tests with production minting still disabled.
2. Add exhaustive caller-namespace and HOF-parameter-use proof tests.
3. Add positive Level-3 tests and capture RED on exact base.
4. Implement Go-only B1 minting plus telemetry.
5. Add Step-5b/source-callee, cache, incremental, navigation, and dump parity.
6. Run focused gates, full suite, Tier-A, and same-base corpora.
7. Reconcile the 67-row measurement join and update roadmap/handoff only from retained results.

This ordering keeps a failed namespace or signature prerequisite separable from the edge-minting change. Do not split a serialized field addition from its version bump or its four-path parity tests.

## 12. Review and convergence record

Design review cap: **2 rounds**. At the cap, new repeated proof-completeness findings park the design; closed enumerable findings are folded in place. A restart requires owner approval.

- Round 1: **FIX.** Three `WRONG`s were folded as one bounded occurrence-proof wave: unqualified HOF callee shadowing could donate the wrong HOF facts; direct-assignment-only scanning missed mutation through `&cb`; and default external import names were inferred from path text rather than proven. The fix narrows #13a to free-function HOFs, adds strict callee/import proof, adds a reusable clean-package predicate, rejects address escape, and makes unknown default import names a stable drop. A local Go compiler control confirmed that one same-file default-import/function collision is invalid Go, but Prism's parse-clean edit-state operating model still requires a fail-closed static proof. A retained packet triage found a nonzero Caddy subset whose caller imports are explicit or in-repo, so the stricter rule does not make the measured slice empty.
- Round 2: **FIX.** One distinct downstream-consumer `WRONG` remained: a synthetic site at source `cb(x)` stores the target function name in `callee_name`, while both Step-5b implementations and reasoning re-read arguments by `(start_byte, callee_name)`. The edge would resolve correctly but silently lose callback arg→target-param flow. Folded by carrying one serde-defaulted source-callee identity through `CallSite`/ordering/navigation keys and making source-expression consumers use one helper. This is a serialized/CPG-topology part of the same cache transition. The cap is extended by one disclosed round because findings decreased from three to one, became smaller, and did not repeat the occurrence-proof class.
- Round 3 (disclosed convergence extension): **SETTLED.** The exhaustive `site.callee_name` consumer census found that reasoning used the same value in three source-expression roles, not only argument lookup. Without the complete fold, `cb(x)` could remain as a spurious variable seed, an unrelated variable named like the resolved target could be suppressed, and `cb(cb)` could lose its argument exception. This is the closed, enumerable remainder of Round 2, not a new proof class: the two Step-5b sites and the three reasoning roles now share the one effective-source-name contract. CPG Call/Return construction and navigation resolution were separately confirmed to use the resolved `FunctionId`; they retain target identity. No open `WRONG` remains from the three-round controller audit. No independent implementation review has yet occurred.
