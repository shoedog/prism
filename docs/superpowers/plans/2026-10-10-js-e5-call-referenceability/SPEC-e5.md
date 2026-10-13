# E5 draft — measured call referenceability

Status: **Draft for owner decision. Uncommitted prototype; no adoption or landing authority.** Base is `40ad8390d6cb5544a063346033c7bb1817ad3855`, branch `plan/js-e5-call-referenceability`. The supplied `plan-brief-step0.md` governs this packet.

## Goal and decision

Remove a measured subset of false Exact `free_single` calls using facts from the calling file and the target declaration. The owner chooses rule breadth and refusal behavior after seeing both native binding yield/cost and downstream rows. Exact is a static-binding grade, not a runtime prediction.

**WRONG:** `compiler/debug.ts:1057` calls the global `Array(height)` but base binds it Exact to the unrelated object-pair arrow `services/codefixes/inferFromUsage.ts:551`. The native checker identifies the language Array declaration. Analogous module-target refusals use the fact that a target declared in another ES module is not a script-global declaration.

This packet recommends the narrow union below. It removes 179 of 1,786 known WRONG Exact targets (10.0%), zero CORRECT targets, zero UNDECIDED targets, and 328 INADMISSIBLE targets. The latter are not credited as WRONG yield. It leaves 1,607 known WRONG targets: this is bounded E5 progress, not complete call-binding correctness.

## Recommended rule: certified-unbound Source call plus negative target syntax

Apply only at the two existing `FreeSingle` arms when all of these hold:

1. The caller is JS/TS/TSX, origin is `Source`, it is not JSX, and it has no qualifier.
2. Caller and target files differ.
3. The existing own-file binder annotated the call `JsLocalBinding::Unproven("unbound")`. Preserve every other annotation and earlier ladder rung.
4. The unique target is registered in the negative syntax set: either its file has a root-level import/export statement, or its name comes from a method definition or an unnamed function in an object pair, class field, member/subscript assignment, or a variable-initializer wrapper call.

A function expression's explicit self-name is not positional inference. An ordinary variable initialized directly with a function is not refused by syntax alone. An ES-module target may be refused irrespective of its named declaration form. Script declarations, proven imports/requires, local aliases, same-file calls, qualified calls, JSX and indirect-resolution origins retain their existing behavior.

The graph-target fast path declines the inadmissible target and lets the ordinary ladder decide; the final R5 branch drops it with `LocalBindingUnproven`. The real paired captures show only target removals, no new rung or target. Full/skeleton/subset constructors retain the negative set; remove/merge maintain it; serialization persists it. `CACHE_VERSION` becomes 115 and both mutant registry pins become 115; the separate navigation edge cache version stays 62.

No repo-wide import resolver, TypeScript path-resolution parity, global declaration graph, or runtime package execution is needed. The mask is negative evidence, not a new positive binding proof. Foreign-language targets are outside this JS target-syntax classifier.

## Measurements and oracle contract

[CENSUS.md](CENSUS.md) is the full per-corpus, per-form, per-own-file-binding cross-tab for base and recommended head. [WRONG-SAMPLES.jsonl](WRONG-SAMPLES.jsonl) supplies 155 source-bound samples across all 21 WRONG cells (ten per cell, or every row when fewer than ten). [MEASUREMENTS.json](MEASUREMENTS.json) and [MEASUREMENTS.md](MEASUREMENTS.md) carry the final rule and consumer totals.

| Corpus | Base Exact FS | Base WRONG | CORRECT | UNDECIDED | INADMISSIBLE | Head Exact FS | Head WRONG | CORRECT | UNDECIDED | INADMISSIBLE |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| X | 98 | 27 | 0 | 34 | 37 | 97 | 27 | 0 | 34 | 36 |
| T | 203 | 201 | 0 | 0 | 2 | 148 | 148 | 0 | 0 | 0 |
| SB579 admitted | 2,664 | 1,558 | 20 | 27 | 1,059 | 2,213 | 1,432 | 20 | 27 | 734 |
| Total | 2,965 | 1,786 | 20 | 61 | 1,098 | 2,458 | 1,607 | 20 | 61 | 770 |

All 583 manifest-qualified SecBench roots were attempted with a 1,500 s producer timeout. Four base exclusions are paired on head: clean-css_4.1.10 killed (exit -9, 394.508 s); atropa-ide_0.2.2-2 timeout (1,500.021 s); natural_5.1.0 timeout (1,500.931 s); three_0.122.0 timeout (1,501.928 s). Thus X/T/SB579 = 581 admitted roots of 585 requested. These are excluded, not assigned zero cost. `step0/producer-wall-times.json` retains every producer's wall time. Parallel runs are not a performance benchmark.

The primary oracle is pinned TypeScript 5.9.3 with allowJs/checkJs, ESNext/Bundler, preserved JSX, no ambient package types, and no emit. Files are statically read; corpus modules never run. Physical UTF-8 bytes, BOM/astral handling, callable syntax and spans bind the compared identities. Calling-file binding evidence and target syntax are independent of verdict admissibility. Flow parse failures, excluded source files and unresolved native callee bindings remain INADMISSIBLE with reasons. A same-signature match alone is UNDECIDED: a typed function parameter may hold another structurally compatible value. CORRECT needs checker value-declaration identity, or a literal unshadowed scalar require and matching exported physical callable. No second-oracle generic failure changes a primary verdict. Raw equals after-override, with zero overrides.

Base metadata is retained for head census axes because corpus syntax and own-file binding construction are unchanged; the real head CLI supplies head resolutions. Private controller runs obtain both sides' metadata directly. The whole-public non-JS serialized call rows (43,953 rows) are byte-identical, and the base/head CPG non-call digest is identical on every admitted root. Dedicated Python/Rust/Go controls also retain byte-identical call sites, functions, modules and normalized DataFlow.

## Drop versus NameOnly

The paired recommendation removes 507 Call and 507 Return edges. It removes 249 ordinary argument DataFlow rows: 105 WRONG and 144 INADMISSIBLE; 19 ReturnFlow rows: ten WRONG and nine INADMISSIBLE. Eight CORRECT ReturnInput and six CORRECT generated return-containment rows disappear because their Exact-triggered return endpoints are no longer generated. They are local semantic facts owned by a refused interprocedural model, not losses of CORRECT Exact call resolutions. This distinction is retained in every adjudicated row.

**Recommend drop.** The real NameOnly build keeps all 507 target relationships and all 249 argument DataFlow rows with lower labels, including the 105 known wrong argument relationships. Step 5b already emits NameOnly arguments; it is not wholly Exact-gated. ReturnFlow/ReturnInput modeling is Exact-gated and is removed in either choice. [MEASUREMENTS.md](MEASUREMENTS.md) pairs removed/added label halves as RELABELLED while preserving the raw adjudications. No new topology is introduced by demotion, but its false topology remains available to consumers that admit NameOnly.

Every differing row is adjudicated with PR-B physical owner/binding/read-role/timing checks and the primary E5 callee binding proof. Ambiguous causal provenance remains UNDECIDED. Native parser diagnostics remain INADMISSIBLE. All added, removed and relabelled rows are retained; a non-call digest difference would trigger STOP.

Symbol inventories and file nodes retain their bytes. Repo-map `ModuleDep` edges are call-derived by `collect_module_edges`; affected dependencies disappear with refused targets. Callee navigation retains the unresolved source-call item (`symbol=null`, no Resolution reason); caller navigation loses the refused relationship. All26 selected function/file inventories match.98 call-derived module edges disappear; sampled queries lose108 caller relationships and12 callee symbols while retaining12 unresolved source-call placeholders. Detailed query coverage is in the final ledger; full graph/site captures cover all admitted roots.

## Deliberate boundaries

Do not change D12, bare/rest incoming slots, callback/synthetic owner identities, alias/may-call/position-doubt handling, qualified/JSX resolution, non-JS behavior, or import/tsconfig parity. The broader rules measure how much yield those preserved paths could offer; they are comparison artifacts, not production adoption. Their experimental cache versions are isolated from production115. Some native classifications remain unjudgeable, and the private corpus remains unmeasured by the planner.

## Proposed acceptance criteria

- Zero CORRECT Exact resolutions lost that a narrower rule can retain; zero newly WRONG topology or previously correct rows made wrong. UNDECIDED/INADMISSIBLE deltas are explicit owner exposure, not hidden yield.
- Every production target/site/CPG delta matches the scoped refusal and has row-level source-bound adjudication. Non-JS and non-call graph output remain byte-identical. Symbols/file inventories remain identical; only call-derived navigation may change.
- Regression tests fail on the unchanged base product and pass on prototype, including all declaration forms, grammars, constructor/incremental/serialization paths, argument/return consumers and negative controls.
- Full available project suite, release build, matrix and authorized quick subset run with the same checkout/source binding. Any excluded check or inherited failing gate stays explicit. Cache115 and both mutant pins agree.
- Controller private run reports raw/after-override totals and paired yield/cost/consumer rows. A private positive cost or out-of-scope delta invokes the brief's STOP; public zero loss is not a private guarantee.

## Gates and repair evidence

Same-environment matched 30-binary suites: base5,254 passed and head5,259 passed, each with one intentional skip. The hook's disclosed all-target verification extension selects35 binaries/5,293 tests:5,292 passed, one failed, one intentionally skipped (`slice_elem_variant_reserved`, pending future SliceElem). The additional failure is an inherited parameter_slot_characterization example assertion; its same-environment base control has identical actual/expected payloads. It is outside E5 and remains unchanged. The full suite is not green; root VERIFICATION.md records exact commands and exclusions. Both doctests pass; the default-feature build check passes. New syntax/consumer regressions fail on base; five targeted groups pass on head. Existing fixtures that required the exact refused behavior were reconciled with their intended purpose: ownership controls use reachable script declarations with byte-preserving export padding; binding/scope controls retain real aliases; E5 escape/inference expectations explicitly require refusal. The complete first15-failure population and second b5 scope population were enumerated before their bounded repairs. The third 30-binary suite was the declared repair cap and passed; the hook extension adds no repair or further retry.

Fresh release build followed by cache-bound adapted Tier-A matrix: 178/178 ok. Fresh base/head release rebuilds in the same worktree, with byte-identical prototype restoration between controls, followed by TS/JS quick: X and SecBench-node VALID; same-environment freshly rebuilt base control has identical m1/m2/m3, pending/failure and aggregate metrics. No observed flip candidates in this sample. This uses installed read-only Python and the included cache adapter, not the literal unmodified uv/no-cache command. Rust quick was excluded before execution because rust-analyzer build scripts/proc macros violate the static-only corpus-execution boundary. Full multi-corpus Tier-A is human-triggered and was not run.

Strict all-target clippy with mcp and `-D warnings` fails on both base and head with the same diagnostic multiset (46 distinct messages); these are inherited failures, not a green gate. Final formatting/diff/patch validation and native/controller checks are recorded in [VALIDATION.md](VALIDATION.md). The hypothesis/probe/admissibility/repair history is in [PROBE-LOG.md](PROBE-LOG.md). No independent review was authorized or dispatched.

## Owner decisions

| Decision | Options | Recommendation |
|---|---|---|
| Breadth | Certified-unbound negative target syntax; all bare inferred targets; all unsupported cross-file bindings from ES-module callers | Adopt the narrow measured subset first: zero CORRECT/UNDECIDED loss, preserves established alias/JSX policy and does not recreate parked parity work. Broader measured yield is a separate bounded decision. |
| Refusal behavior | Drop; retain target as NameOnly | Drop: the NameOnly build retains 105 known WRONG argument relationships. |
| Private acceptance | Controller runs paired F measurement before adoption; accept public-only evidence | Run F before adoption using the exact command in CONTROLLER.md. The planner has never opened a directory named frontend-portal. |
| D12 follow-up | Leave blanket protection; separately narrow/remove it and fill the incoming bare-formal hole | Leave D12 unchanged now. Revisit only after remaining reachable false Exact exposures have source-bound closure and a separately authorized Def/argument-flow increment. |

PR-A D12 currently refuses outgoing new-formal paths reaching **every** cross-file Exact `free_single` argument edge. The narrow E5 prototype makes its 507 refused calls ineligible as seeds, but 1,607 known WRONG Exact targets remain. Therefore D12 cannot yet be globally removed. A future rule could make its refusal selective by proved binding provenance; that requires measurements of the remaining argument reachability, both parenthesized/bare forms, transitive local paths, and zero-new-WRONG source rows. Record only; no D12 code change in E5.

## STOP assessment and custody

No public STOP has been observed: meaningful own-file-only yield, zero measured CORRECT losses, no non-JS/non-call digest difference. Private F and four excluded roots remain unknown. This is a draft recommendation with reversible uncommitted product code. The controller owns Git custody; the planner makes no Git writes. Final `E5-src.patch` and `E5-docs.patch`, immutable binary/source bindings and snapshots are under `/Users/wesleyjinks/prism-evidence/js-e5/step0/`.
