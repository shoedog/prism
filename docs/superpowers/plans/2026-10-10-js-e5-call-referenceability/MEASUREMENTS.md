# Current Step1 union measurement

| Corpus | WRONG removed | CORRECT lost | UNDECIDED removed | INADMISSIBLE removed |
|---|---|---|---|---|
| X | 27 | 0 | 34 | 37 |
| T | 201 | 0 | 0 | 2 |
| SB | 171 | 0 | 0 | 418 |

| Corpus | Arm | WRONG | CORRECT | UNDECIDED | INADMISSIBLE |
|---|---|---|---|---|---|
| X | a_only | 0 | 0 | 0 | 0 |
| X | b_only | 27 | 0 | 34 | 36 |
| X | both | 0 | 0 | 0 | 1 |
| T | a_only | 0 | 0 | 0 | 0 |
| T | b_only | 148 | 0 | 0 | 0 |
| T | both | 53 | 0 | 0 | 2 |
| SB | a_only | 123 | 0 | 0 | 203 |
| SB | b_only | 45 | 0 | 0 | 93 |
| SB | both | 3 | 0 | 0 | 122 |

| Corpus | Arm(b) invocation | WRONG | CORRECT | UNDECIDED | INADMISSIBLE |
|---|---|---|---|---|---|
| X | ordinary | 14 | 0 | 11 | 1 |
| X | jsx | 13 | 0 | 23 | 35 |
| X | qualified | 0 | 0 | 0 | 1 |
| T | ordinary | 201 | 0 | 0 | 2 |
| T | jsx | 0 | 0 | 0 | 0 |
| T | qualified | 0 | 0 | 0 | 0 |
| SB | ordinary | 47 | 0 | 0 | 123 |
| SB | jsx | 0 | 0 | 0 | 5 |
| SB | qualified | 1 | 0 | 0 | 87 |

Physical JSX is classified by the native invocation node, not the internal jsx flag:66 JSX spans are carried by IndirectResolution records with jsx=false. The table separately discloses89 qualified physical spans (1 W/88 I);66 are indirect and23 Source records have no retained product qualifier. The unchanged real R-bound guard acts on qualifier-less product call sites; no blanket physical-qualified preservation claim is made. Every ordinary row in this measured b population has Source origin: the real R-bound-source subset is X14 W/0 C/11 U/1 I, T201 W/0 C/0 U/2 I, SB47 W/0 C/0 U/123 I. R-bound minus R-bound-source therefore includes both physical JSX and qualified fallback records.

All581 call-site and graph comparisons are admitted. Every differing call site (890) and graph row (2208) is classed and adjudicated in `union-call-site-adjudications.jsonl` / each graph.adjudicated.jsonl. Added targets/sites/graph rows:0/0/0. All43953 original serialized non-JS call rows are byte-identical; all581 non-call digests match. All581 symbol/file inventories match; 225 call-derived module dependencies are removed. Sampled caller/callee queries cover affected roots; aggregate deltas are `{"callees": 27, "callees_unresolved_placeholders": 27, "callers": 148}`. Unresolved source placeholders are disclosed separately from resolved relationships. Full receipts retain every inventory/query command and any reused base artifact.

All UNDECIDED removals: UNDECIDED-step1.md. R-local accepted follow-up: SPEC-e5.md. Full current graph ledger/gates: REPORT-step1.md.

## Historical step0 measurements (unchanged evidence)

# E5 measured rule and downstream ledger

Yield is WRONG Exact targets removed; cost is CORRECT Exact targets lost. U/I are exposures, not yield. Candidate predicates below are evaluated over every admitted base census row; real-build deltas are listed separately. All raw/after-override verdicts agree.

## Complete native predicate pricing

| Rule | Corpus | WRONG yield | CORRECT cost | UNDECIDED touched | INADMISSIBLE touched |
|---|---|---:|---:|---:|---:|
| R-inferred | X | 3 | 0 | 0 | 0 |
| R-inferred | T | 5 | 0 | 0 | 2 |
| R-inferred | SB | 401 | 0 | 13 | 322 |
| R-bound | X | 27 | 0 | 34 | 37 |
| R-bound | T | 201 | 0 | 0 | 2 |
| R-bound | SB | 48 | 0 | 0 | 223 |
| R-bound-source | X | 14 | 0 | 11 | 1 |
| R-bound-source | T | 201 | 0 | 0 | 2 |
| R-bound-source | SB | 47 | 0 | 0 | 125 |
| R-inferred-form | X | 3 | 0 | 0 | 0 |
| R-inferred-form | T | 5 | 0 | 0 | 2 |
| R-inferred-form | SB | 444 | 10 | 13 | 323 |
| R-unbound-inferred | X | 0 | 0 | 0 | 0 |
| R-unbound-inferred | T | 4 | 0 | 0 | 2 |
| R-unbound-inferred | SB | 126 | 0 | 0 | 303 |
| R-unbound-module | X | 0 | 0 | 0 | 1 |
| R-unbound-module | T | 53 | 0 | 0 | 2 |
| R-unbound-module | SB | 0 | 0 | 0 | 198 |
| R-recommended | X | 0 | 0 | 0 | 1 |
| R-recommended | T | 53 | 0 | 0 | 2 |
| R-recommended | SB | 126 | 0 | 0 | 325 |

`R-inferred` respects explicit self-names; `R-inferred-form` ignores that distinction and costs ten CORRECT CJS scalar-require targets. R-bound prices the calling file as an ES module without import/require evidence. R-bound-source also requires ordinary Source invocation. Historical step0 R-recommended (now arm(a)) requires certified-unbound Source invocation and an inferred target shape or target ES-module syntax. Neither unbound component alone removes the complete recommended population.

## Real-build paired target measurements

| Build / rule | Corpus | WRONG removed | CORRECT lost | UNDECIDED touched | INADMISSIBLE touched |
|---|---|---:|---:|---:|---:|
| Recommended | X | 0 | 0 | 0 | 1 |
| Recommended | T | 53 | 0 | 0 | 2 |
| Recommended | SB | 126 | 0 | 0 | 325 |
| R-inferred | X | 3 | 0 | 0 | 0 |
| R-inferred | T | 5 | 0 | 0 | 2 |
| R-inferred | SB | 401 | 0 | 13 | 322 |
| R-bound | X | 27 | 0 | 34 | 37 |
| R-bound | T | 201 | 0 | 0 | 2 |
| R-bound | SB | 48 | 0 | 0 | 215 |
| NameOnly | X | 0 | 0 | 0 | 1 |
| NameOnly | T | 53 | 0 | 0 | 2 |
| NameOnly | SB | 126 | 0 | 0 | 325 |

Recommended full capture covers all581 admitted roots. Experimental builds capture every root with an eligible base predicate: R-inferred23, R-bound8, NameOnly12. Outside these sets the refusal-only predicates have no eligible baseline target. Base calls/sites are unchanged in identity; only resolved target sets/grades change. Each experimental source snapshot and immutable binary hash is retained under step0/variants/. Experimental cache versions10116/10117/10118 prevent production115 cache collisions.

R-inferred repair preserves the original artifact and its 817-target results but supersedes them:71 qualified extras (4 WRONG,67 INADMISSIBLE) were outside its stated bare-call rule. The corrected real build removes exactly746 priced targets:409 WRONG,13 UNDECIDED,324 INADMISSIBLE.

R-bound real syntax admission removes564 of the572 native-priced targets. All eight untouched differences are INADMISSIBLE React Native/Flow-recovery sites; they do not change measured WRONG/CORRECT/UNDECIDED counts. The real module mask is restricted to registered named JS callables from files whose product parser exposes root import/export statements. Native TS module/binding evidence is not silently substituted into the product. The admitted population contains no eligible import/require-destructuring R-bound target; the experimental broad build is therefore an observed-population comparison, not an adopted positive binding implementation.


## Complete consumer adjudication, by corpus

W/C/U/I columns judge the changed relationship, independently of call-site yield. ReturnInput/ReturnContains CORRECT rows are generated endpoint facts removed with refused Exact calls. RELABELLED pairs retain both original adjudications; they are not new topology.

| Build | Corpus | Change | Kind | W | C | U | I |
|---|---|---|---|---:|---:|---:|---:|
| Recommended | SB | REMOVED | Call | 126 | 0 | 0 | 325 |
| Recommended | SB | REMOVED | DataFlow | 75 | 0 | 0 | 144 |
| Recommended | SB | REMOVED | Return | 126 | 0 | 0 | 325 |
| Recommended | SB | REMOVED | ReturnContains | 0 | 6 | 0 | 0 |
| Recommended | SB | REMOVED | ReturnFlow | 10 | 0 | 0 | 9 |
| Recommended | SB | REMOVED | ReturnInput | 0 | 8 | 0 | 0 |
| Recommended | T | REMOVED | Call | 53 | 0 | 0 | 2 |
| Recommended | T | REMOVED | DataFlow | 30 | 0 | 0 | 0 |
| Recommended | T | REMOVED | Return | 53 | 0 | 0 | 2 |
| Recommended | X | REMOVED | Call | 0 | 0 | 0 | 1 |
| Recommended | X | REMOVED | Return | 0 | 0 | 0 | 1 |
| R-inferred | SB | REMOVED | Call | 401 | 0 | 13 | 322 |
| R-inferred | SB | REMOVED | DataFlow | 147 | 0 | 13 | 130 |
| R-inferred | SB | REMOVED | Return | 401 | 0 | 13 | 322 |
| R-inferred | SB | REMOVED | ReturnContains | 0 | 14 | 0 | 0 |
| R-inferred | SB | REMOVED | ReturnFlow | 68 | 0 | 1 | 6 |
| R-inferred | SB | REMOVED | ReturnInput | 0 | 26 | 0 | 0 |
| R-inferred | T | REMOVED | Call | 5 | 0 | 0 | 2 |
| R-inferred | T | REMOVED | Return | 5 | 0 | 0 | 2 |
| R-inferred | X | REMOVED | Call | 3 | 0 | 0 | 0 |
| R-inferred | X | REMOVED | DataFlow | 2 | 0 | 0 | 0 |
| R-inferred | X | REMOVED | Return | 3 | 0 | 0 | 0 |
| R-inferred | X | REMOVED | ReturnFlow | 1 | 0 | 0 | 0 |
| R-bound | SB | REMOVED | Call | 47 | 0 | 2 | 214 |
| R-bound | SB | REMOVED | DataFlow | 88 | 0 | 0 | 0 |
| R-bound | SB | REMOVED | Return | 47 | 0 | 2 | 214 |
| R-bound | SB | REMOVED | ReturnContains | 0 | 1 | 0 | 0 |
| R-bound | SB | REMOVED | ReturnFlow | 2 | 0 | 0 | 2 |
| R-bound | SB | REMOVED | ReturnInput | 0 | 3 | 0 | 0 |
| R-bound | T | REMOVED | Call | 201 | 0 | 0 | 2 |
| R-bound | T | REMOVED | DataFlow | 64 | 0 | 0 | 0 |
| R-bound | T | REMOVED | Return | 201 | 0 | 0 | 2 |
| R-bound | T | REMOVED | ReturnFlow | 5 | 0 | 0 | 0 |
| R-bound | X | REMOVED | Call | 27 | 0 | 34 | 37 |
| R-bound | X | REMOVED | DataFlow | 5 | 0 | 7 | 0 |
| R-bound | X | REMOVED | Return | 27 | 0 | 34 | 37 |
| R-bound | X | REMOVED | ReturnFlow | 1 | 0 | 0 | 0 |
| NameOnly | SB | RELABELLED | Call | 126 | 0 | 0 | 325 |
| NameOnly | SB | RELABELLED | DataFlow | 75 | 0 | 0 | 144 |
| NameOnly | SB | RELABELLED | Return | 126 | 0 | 0 | 325 |
| NameOnly | SB | REMOVED | ReturnContains | 0 | 6 | 0 | 0 |
| NameOnly | SB | REMOVED | ReturnFlow | 10 | 0 | 0 | 9 |
| NameOnly | SB | REMOVED | ReturnInput | 0 | 8 | 0 | 0 |
| NameOnly | T | RELABELLED | Call | 53 | 0 | 0 | 2 |
| NameOnly | T | RELABELLED | DataFlow | 30 | 0 | 0 | 0 |
| NameOnly | T | RELABELLED | Return | 53 | 0 | 0 | 2 |
| NameOnly | X | RELABELLED | Call | 0 | 0 | 0 | 1 |
| NameOnly | X | RELABELLED | Return | 0 | 0 | 0 | 1 |

Every differing raw row is retained and judged before label pairing. Recommended:1296 raw/classified rows, all removed. NameOnly:2559 raw differing halves become1296 classified rows:507 Call,507 Return and249 DataFlow RELABELLED, plus19 ReturnFlow,8 ReturnInput and6 ReturnContains removed. Regrading introduces zero new topology but retains105 known WRONG argument relationships.

Graph Call/Return rows aggregate relationships rather than individual source spans. Where multiple changed source calls disagree on admissibility, graph attribution remains UNDECIDED; therefore R-bound's aggregate Call/Return verdict totals differ from its native site-level totals. This is disclosed, not forced into a WRONG yield. DataFlow physical source endpoints and PR-B binding/read-role/timing output remain in the row details.

## Real experimental caller/callee navigation samples

All nine samples targeted the intended FunctionId name/file; all36 CLI query producers completed, and no new resolved symbol appeared. Counts below are changed full-value items; unresolved source-call placeholders are reported separately. These are samples, not exhaustive query-space coverage. Whole call-stats outputs and graph consumer rows are exhaustive over selected/admitted roots.

| Rule | Corpus | Caller relationships lost | Caller items relabelled | Callee symbols lost | Callee items relabelled | New unresolved placeholders |
|---|---|---:|---:|---:|---:|---:|
| R-inferred | SB | 6 | 0 | 2 | 0 | 2 |
| R-inferred | T | 4 | 0 | 1 | 0 | 1 |
| R-inferred | X | 1 | 0 | 1 | 0 | 1 |
| R-bound | SB | 1 | 0 | 1 | 0 | 1 |
| R-bound | T | 4 | 0 | 1 | 0 | 1 |
| R-bound | X | 12 | 0 | 9 | 0 | 9 |
| NameOnly | SB | 0 | 4 | 0 | 1 | 0 |
| NameOnly | T | 0 | 4 | 0 | 1 | 0 |
| NameOnly | X | 0 | 1 | 0 | 1 | 0 |

## Recommended symbol/file inventory and navigation parity

All26 roots selected by any candidate predicate completed base/head functions and repo-map producers. Function inventory JSON is byte-identical; repo-map file nodes, metadata and warnings are identical.98 removed ModuleDep edges are source-bound call-derived dependencies; there are no added dependencies or file nodes. Twelve roots carry recommended target refusals; sampled WRONG caller/callee query pairs cover all ten with a known WRONG refusal (X and React Native only have INADMISSIBLE refusals and are not represented as WRONG samples). These query pairs lose108 caller relationship items and12 callee symbols, adding12 corresponding unresolved source-call placeholders. The full site/graph census covers all581 admitted roots.

## Controller private run (corpus F; 2026-10-12, aggregates only)

Paired `CONTROLLER-e5.sh diff` on the frozen step-0 binaries (hashes verified 12/12 against `final-product-binding.json`; prototype source committed as `65086ee6`). Status COMPLETE; raw equals after-override; 0 overrides; empty stderr.

| | Base | Recommended head |
|---|---:|---:|
| Call sites | 13,299 | 13,299 (byte-identical file) |
| Exact `free_single` JS/TS targets | 113 | 113 |
| WRONG / CORRECT / UNDECIDED / INADMISSIBLE | 109 / 0 / 4 / 0 | 109 / 0 / 4 / 0 |
| Graph rows | 54,831 | 54,831 (byte-identical file) |

**The recommended rule removes nothing on F.** Every one of the 113 targets has own-file binding `local_declaration` (callee forms: wrapper call 82 WRONG; direct function 24 WRONG + 4 UNDECIDED; object pair 2 WRONG; function declaration 1 WRONG), which the rule's condition 3 (`Unproven("unbound")` only) excludes. No cost either: no site, target, graph or non-JS difference.

Census predicates on the base rows (not paired head builds on F): R-inferred would cover 84 WRONG, 0 CORRECT, 0 UNDECIDED; R-bound would cover 109 WRONG, 0 CORRECT, 4 UNDECIDED.

This meets the brief's "negligible WRONG yield" STOP for F. The breadth decision goes to the owner with this result. Private rows stay in the controller's private evidence directory.

## Controller private run, union head (corpus F; 2026-10-13, aggregates only)

Paired `CONTROLLER-e5.sh diff` with the step-0 base binaries and the frozen step-1 head binaries (`head-prism` `38b923a3…`, `head-facts` `f77b5364…`, `head-graph` `07b08eb0…`, matching `step1/binary-binding.json`; product source unchanged between `e164178b` and packet head `2d9253bd`). Status COMPLETE; raw equals after-override; 0 overrides; controller stderr empty; producer stderr identical in size and content class to step 0.

| | Base | Union head |
|---|---:|---:|
| Call sites | 13,299 | 13,299 (113 changed; non-JS call sites identical) |
| Exact `free_single` JS/TS targets | 113 | 0 |
| Targets removed: WRONG / CORRECT / UNDECIDED / INADMISSIBLE | | 109 / 0 / 4 / 0 |
| Targets, sites or graph rows added | | 0 / 0 / 0 |
| Graph rows | 54,831 | 54,562 |

Graph rows removed (269), each adjudicated: Call 113 (109 WRONG, 4 UNDECIDED), Return 113 (109 WRONG, 4 UNDECIDED), DataFlow 42 (all WRONG), ReturnFlow 1 (WRONG).

All 113 removals are ordinary Source calls (census predicate R-bound-source covers 113 of 113), so the JSX and indirect part of arm (b) contributes nothing on F. The four UNDECIDED removals are calls through a local declaration to a variable bound directly to a function; they are exposure, not yield. Census prices on the same base rows: R-local 109 W / 0 C / 4 U; R-local-no-require 109 W / 0 C / 4 U (F's callers are ES modules, so the follow-up predicate adds nothing there beyond the union).

The acceptance criterion "zero CORRECT Exact loss on F" passes. Private rows stay in the controller's private evidence directory.

## Controller note on arm (b) and member-expression JSX (public corpus X)

Reading the Excalidraw source behind the X JSX split (13 W / 23 U / 35 I):

- prism reaches these targets through its indirect record for a static property assignment, for example `DropdownMenu.Item = DropdownMenuItem;` in `dropdownMenu/DropdownMenu.tsx:60`. The site `<DropdownMenu.Item>` in `Actions.tsx` is recorded with callee text `DropdownMenuItem`.
- Arm (b) then asks whether the **calling** file imports `DropdownMenuItem`. It does not: it imports `DropdownMenu`. The name was written in `DropdownMenu.tsx`, which does import it.
- The 23 UNDECIDED removals are of this kind (`DropdownMenu.Item`, `DropdownMenu.Item.Badge`, `DropdownMenuSub.Content`, `DropdownMenuSub.Trigger`). By the source they are the assigned component; the checker cannot establish value identity, so they are UNDECIDED, not CORRECT.
- The 13 WRONG removals are the same mechanism choosing by property name alone: `<DropdownMenu.Trigger>` resolved to `DropdownMenuSubTrigger`, `<DropdownMenu.Content>` to `DropdownMenuSubContent`.
- A sampled INADMISSIBLE removal is `<DropdownMenuPrimitive.Item>` (imported from `radix-ui`) resolved to the local `DropdownMenuItem`.

So for indirect records arm (b)'s premise is evaluated in a file where the name was never written. This is an open owner decision (keep or cut the indirect/JSX part of arm (b)); the reviewers are asked to assess it.
