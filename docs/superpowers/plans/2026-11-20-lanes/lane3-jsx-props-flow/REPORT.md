# Lane 3 close-out: JSX props flow (#596)

Written 2026-11-27 by the lane orchestrator. Measured product: the merged main f720d717 (S1 to S3 merged). Corpora: Excalidraw
(X, pinned 0642e72c), cal.com (R2, f972feef); a private corpus appears only as counts. Every number gives its run in the table or the line it sits in.
The attempt ledger of every slice, fold and review round is in `ATTEMPT-LEDGER.md`.

## 0. Summary in plain language

The lane connects a JSX attribute value to the component that receives it. On the two public React corpora it adds **1,942** such
edges (X 615, R2 1,327); an independent TypeScript oracle certified all of them (0 wrong, 0 undecided), every one is connected in the final graph (100%),
and they chain through nested components **1,205** times (gate: 25). The issue's own example now produces the sink finding and a `Reached` verdict.
The hidden-write certificate withholds **50** of 1,992 candidate edges, all of them correct flows (11 already refused by the interim rule R before it, 29 more at P2's pricing, 10 more in one cal.com file).
**The lane gate is not fully met:** the review-example gate fails in both directions (cal.com: forward 2 of 15 and backward 1 of 15 commits gain context,
against 8 of 15; none of the 3 was rated useful by both readers). Per the plan the threshold is not adjusted; #596 is not closed and the owner decides.
Four follow-ups have a met threshold and an issue (F1, F3, F4, F5); two defects were filed (the resolver defect, the attribute-name anchors).

## 1. What the lane delivers

Before the lane, a JSX attribute value had no data-flow edge into the component that receives it
(`<Html html={text} />` did not connect `text` to the destructured `html` of `Html`), and an object literal
passed to a wrapped component or hook had edges only for plain callees. After the lane, on the two public
React corpora, prism adds these edges, each checked by an independent TypeScript-checker oracle:

| Slice (PR, merge commit) | What it adds | Added edges: Excalidraw (X) / cal.com (R2) | Oracle |
|---|---|---|---|
| S1 (#929, abc5b2f9) | Defs for destructured props formals, and the edges from them to their reads inside the component | 3,995 / 18,736 (TypeScript src 1,174) over the S1 base (including the 1 / 3 / 2 arg-binding edges the comparator classes unclassified); 0 removed, 0 label moves | 3,073 / 15,353 certified CORRECT, 0 WRONG; the rest are pattern-def-capture edges the props oracle does not judge (run `s1/out6`) |
| S2a (#952, 6179c7e1) | Moves the binder into its own module; no output change | 0 / 0 | byte-identical edge dumps and stats on four corpora |
| S2b (#959, e37d644d) | Object-literal arguments through React wrappers (`memo`, `forwardRef`) and hooks (`useCallback`) | 511 / 2,402 (TypeScript src 188) | 448 / 2,388 certified CORRECT, 0 WRONG; the other 63 / 14 are unclassified edges not judged by that oracle (run `s2b/out3`) |
| S3 (#964, f720d717) | JSX element props: a caller's attribute value to the component's pattern local, rest local, member read or bare read | **615 / 1,327** (TypeScript src 0 at e63fdd66, `s3/out3`; not re-run on the final head), 0 removed, 0 label moves | **615 / 1,327 CORRECT, 0 WRONG, 0 UNDECIDED** (run `s3/out9`, repeated on merged main `s4/out-main`) |

The S3 figures are the lane's headline: **1,942 JSX props edges** on the two public corpora together, all certified.
They were measured again on a product built at the merged main (f720d717): its edge dumps and statistics files are
byte-identical to the run on the approved branch head (25459f77), whose code differs from the merged head only in a doc comment.

### End-to-end paths (merged main, final graph)

| Corpus | Props edges | Connected share | Chains of 2 or more component hops (every hop CORRECT) | Paths of 2 / 3 / 4 hops | Chains ending at a sink line |
|---|---|---|---|---|---|
| Excalidraw (X) | 615 | 615 of 615 (100%) | **386** | 288 / 86 / 12 | 0 |
| cal.com (R2) | 1,327 | 1,327 of 1,327 (100%) | **819** | 415 / 388 / 16 | 5, all of one hop |
| Together | 1,942 | 100% | **1,205** | | 5 |

Tables: `paths-X.md`, `paths-R2.md`. A chain crosses one props edge per component hop. In plain terms: the edges
connect, and they connect onward through nested components (1,205 chains of two or more hops). Most chains end
at another element's attribute or at a local read; only five end at a sink line, and none of those spans more
than one hop. The lane makes values flow into components; it does not by itself make sinks deep inside component
trees reachable (section 3 and F5).

### The issue's example

`<Html html={text} />` with `Html({ html })` rendering `dangerouslySetInnerHTML={{ __html: html }}` (the example of #596),
run on the merged main binary (`s4/taint-gate/`): `prism --algorithm taint` reports a `taint_sink` finding at the
`dangerouslySetInnerHTML` line ("tainted value from line 6 reaches sink at line 3"), and
`prism nav taint-reaches --source App.tsx:7 --sink App.tsx:3` returns `Reached` (sink variable `html`, reached from `text`; the two other
variables on the sink line, the attribute name and the `__html` key, are `NotReached`).

## 2. The lane gate (DESIGN section 7), on the merged main

| Gate | Threshold | Result | Run |
|---|---|---|---|
| Census go | at least 1,000 predicted connected pairs on X and R2; each rate at least 30% | 1,018 pairs (X 308, R2 710); rates 36.07% and 38.34% | stage M census, pre-lane product 9d1a7d06 |
| Slice S1 | no unexplained legacy removal, relabel or counter move; Go control parity; 0 WRONG | 0 removed; 0 label moves besides the two pinned exceptions (the R2.6 relocation, the p35 sibling relabel); Go control identical; 0 WRONG; 87 of 87 and 89 of 89 retained projections hand-read CORRECT | `s1/out6`, PR #929 |
| Oracle | 0 WRONG; UNDECIDED at most 25%; hand-read UNDECIDED sample 0 WRONG | 0 WRONG; UNDECIDED 0 of 1,942 (0%), so the hand-read sample of UNDECIDED is empty; the other hand-reads were CORRECT (137 of 137, `s3/handread-r3.md`) | `s3/out9`, `s4/out-main` |
| Volume (#604) | at least 1,000 CORRECT props edges starting at JSX attribute values | **1,942** (X 615, R2 1,327) | same |
| End to end | connected share at least 90% on each corpus; at least 25 chains of 2 or more hops | **100%** on both; **1,205** chains (X 386, R2 819) | same |
| Taint | the issue's example gives the sink finding and `Reached` | both hold (section 1) | `s4/taint-gate/` |
| Review examples | of 15 per direction, at least 8 gain context; of those at least 60% useful and at most 10% noise | **FAILS**: forward 2 of 15, backward 1 of 15 gain context; of those 3, 0 useful, 1 noise | `g/examples/calcom` |
| Private corpus | same measurements, aggregate counts only | 420 edges, all CORRECT, 100% connected, 18 chains; examples gate would also fail (section 6) | private evidence |

Checks on the final head: Tier-A `--matrix-only` 178 ok on the S3 head (`--quick` cannot run on main, notice 28); CI 13 of 13 on #964;
macOS footprint guard on the pinned corpus: prime LOADED 3,143.5 MiB, guarded UNLOADED 16.1 MiB, control UNLOADED 2,006.4 MiB.

## 3. What the lane withholds, and why

| What is withheld | How much | Why |
|---|---|---|
| Edges lost to the certificate P2 (S3's proof that no hidden write changes the source between its definition and the JSX site) | **50** of 1,992 candidate edges in all (no-guard run `s3/out2`: X 632, R2 1,360; final X 615, R2 1,327): 11 (X 2, R2 9) were already refused by the interim rule R that P2 replaced, 29 more at P2's pricing (X 15, R2 14), and **10** more in one cal.com file (`BookingDetailsSheet.tsx`, shape `(booking.responses ?? {}) as Record<..>`); the baseline of the oracle comparison (X 630, R2 1,351) already lacked the 11 | Each of the 39 beyond rule R's 11 was CORRECT in the oracle comparison (the 11 were priced as correct flows in `s3/price/PRICE.md`). The certificate admits only a closed list of spelled forms, so each is a flow the proof could not rule a hidden write out of, not a defect found. Peeling TypeScript wrappers in the result position would give the 10 back (costed, not taken) |
| Three shapes the certificate admits although a write could change the value | 3 shapes; 0 on either corpus | `super.p = v` on a formal receiver; a const whose initializer reads an enum member while the enum is being built; `const a = x` followed by a store through another alias. Known-limit rows beside controls, linked to #783 (a write through an alias does not kill an earlier direct definition; same family) |
| Sources whose text the parser reads differently from the language | 0 incidence on both corpora (cross-check of the owners of all 1,942 kept edges against a TypeScript-tree reference: 0 genuine differences; the structural rule would have cost 10 of 1,942 before two object kinds were admitted, 0 after) | Issue #983: the parser attaches a following-line `[..]` or `(..)` to an arrow function or as-expression where the language ends the statement. The certificate refuses such owners; the lane does not fix the parser |
| Elements with no Exact singleton target | `refused_not_exact_singleton` 3,456 (X), 19,545 (R2), sites of both kinds | A props edge needs the component identified exactly. The largest refusal by far; F4 and the resolver defect (section 7) bear on it |
| Value is not data (callbacks, inline objects from calls, constants) | 3,332 (X), 6,368 (R2) | Counted, not bound |
| Zero-width bare read of a plain formal (`const { a } = props`) | 406 (X), 982 (R2) sites | F3 |
| Spread sources not bound | 166 (X), 451 (R2) | F1 |
| A wrapper other than `memo` or `forwardRef` | 0 (X), 20 (R2) sites | F9 |

The counters are the product's `dfg_props` counters on the merged head (section 4); they cover both transports (JSX and object literal), per site, except `refused_value_not_data`, `refused_key_not_read` and `refused_overwritten`, which count leaves or terminals (DESIGN 5.4).

## 4. The `dfg_props` breakdown (merged main, final graph)

Counters from `call-stats.json` of `s4/out-main/{X,calcom}/head` (identical to `s3/out9`). Units are DESIGN 5.4's: the `edges_to_*` keys count distinct emitted props edges of both transports (JSX and object literal; disjoint, except that `edges_to_container` is a subset of `edges_to_def`); `sites_*` and the `refused_*` keys count sites, one admission reason at most per site, except the per-leaf and per-read keys named in 5.4. A key that is absent is zero.

| Key | Excalidraw (X) | cal.com (R2) |
|---|---|---|
| `edges_to_bare_use` | 646 | 657 |
| `edges_to_container` | 35 | 158 |
| `edges_to_def` | 1,031 | 3,335 |
| `edges_to_rest` | 32 | 62 |
| `edges_to_use` | 373 | 2,298 |
| `refused_bracket_read` | 0 | 14 |
| `refused_key_not_read` | 206 | 770 |
| `refused_member_qualifier` | 27 | 145 |
| `refused_no_props_formal` | 59 | 118 |
| `refused_not_exact_singleton` | 3,456 | 19,545 |
| `refused_overwritten` | 114 | 373 |
| `refused_pre_initializer_unproved` | 4 | 32 |
| `refused_source_ambiguous` | 41 | 45 |
| `refused_source_in_enclosing_owner` | 26 | 48 |
| `refused_source_not_found` | 15 | 5 |
| `refused_source_unconnected` | 123 | 168 |
| `refused_target_identity` | 7 | 50 |
| `refused_unknown_wrapper` | 0 | 20 |
| `refused_unproved_read` | 12 | 46 |
| `refused_value_not_data` | 3,332 | 6,368 |
| `refused_zero_width_read` | 406 | 982 |
| `sites_jsx` | 846 | 5,363 |
| `sites_literal` | 4,405 | 18,964 |
| `spread_sources_unbound` | 166 | 451 |

Shares of all props edges (`edges_to_def` + `edges_to_rest` + `edges_to_use` + `edges_to_bare_use`: X 2,082, R2 6,352): `edges_to_bare_use` **31.0%** (X) and **10.3%** (R2); `edges_to_rest` **1.5%** (X) and **1.0%** (R2). A bare use is a whole-value read of the prop (`props` passed on, `{...props}`): the edge says the value is in the object but not which key. The oracle accepts it as correct because the object carries the key (the lane's documented accepted cost).

The product keeps no refusal list beyond these counters. The per-site attempt ledger of DESIGN 5.7 (a join of every site with its refusal) was not built; see section 8.

## 5. Review examples (cal.com, R2; merged-lane head 25459f77 against the pre-lane product 9d1a7d06)

Excalidraw was not used: the pinned copy has no history and the local clone is shallow; no full clone was fetched. All 15 per direction are cal.com
(f972feef history, first-parent commits newest first, non-test `.tsx`/`.jsx`, qualifying by the pinned TypeScript 5.9.3; `tools/examples_scan.py`, `tools/examples.py`; records in `g/examples/calcom/`).
Forward: the frontier of `taint-reaches` from each changed attribute-value line (up to 5 per commit). Backward: `fullflow` on the commit's patch.
A head-only line "gains context" when an added edge (head's dfg edges minus the base's, both ends keyed) starts at the changed line and ends in the component the line belongs to (forward), or starts at the line and ends in the changed component (backward). This is a simplification of the plan's "witness crosses an edge that `compare_props.py` classes props-jsx": the added-edge test was used because it needs no per-commit call-site capture. Head-only lines without such an edge are unrated.

| Direction | Qualifying commits taken | Gain context | Gate (at least 8 of 15) | Head-only lines without a props edge (commits) |
|---|---|---|---|---|
| Forward | 15 | **2** (fwd-04, fwd-08) | **FAILS** | 1 (fwd-08 also gained) |
| Backward | 15 | **1** (bwd-04) | **FAILS** | 7 (bwd-04 also gained) |

Forward detail (53 seed lines): 12 queries returned an error (reason not recorded in this run); 6 seeds started an added edge into another file (any edge head adds over the pre-lane base, so S1 and S2b edges are included); on 3 of them the head frontier gained lines in the receiving component, on 3 it did not (issue #991, F5).

Two readers (Opus-5.5 high, Sonnet-5.5 high, separate sessions, blind to each other; labels in `g/examples/readers/`) labelled the 3 examples that gained context:

| Example | Opus | Sonnet | Combined (useful if both, noise if either) |
|---|---|---|---|
| bwd-04 | useful | neutral | neutral |
| fwd-04 | neutral | neutral | neutral |
| fwd-08 | neutral | noise | noise |

Agreement: 1 of 3 equal labels; Cohen's kappa is not meaningful on 3 items. Disagreements, not averaged: bwd-04 (Opus: the caller at `Dialog.tsx:94` renders through the restructured header, useful; Sonnet: that caller passes no icon, so unaffected, neutral) and fwd-08 (Opus: real consumers but the changed value is unchanged, neutral; Sonnet: most shown lines unrelated to the removed prop, noise).
Result against the gate: of the examples that gained context, 0 of 3 useful (need 60%) and 1 of 3 noise (limit 10%). **The review-example gate fails in both directions on R2.** Per the plan, the threshold is not adjusted and the lane stops for the owner. F6 (`href`/`src` sinks, `useState`/`useContext`) depends on this result: the examples do not show reader value, so F6 is not proposed.

## 6. The private corpus (aggregate counts only; the measurements are kept in private evidence)

Called `private-corpus` here. Same tools, merged main binary against the S2b-base binary (e37d644d), as for the public corpora.

| Measure | `private-corpus` |
|---|---|
| Census (stage M, pre-lane): predicted pairs / Exact capitalised path attributes | 373 / 3,321 (11.2%); the public 30% rate gate does not apply to it |
| Props edges added by S3 | 420 (graph edges 45,437 to 45,857; 0 removed; 0 unexpected changes) |
| Oracle | 420 CORRECT, 0 WRONG, 0 UNDECIDED |
| Connected share | 420 of 420 (100%) |
| Chains of 2 or more component hops | 18 (paths by hops: 1 hop 562, 2 hops 18); chains ending at a sink line 0 |
| Review examples (history present, 10 per direction) | forward: 4 of 10 gain context; backward: 0 of 10. Readers on the 4: Opus 0 useful, 3 neutral, 1 noise; Sonnet 0 useful, 2 neutral, 2 noise; equal on 1 of 4; combined 0 useful, 1 neutral, 3 noise |
| Top refusals (`dfg_props`, sites of both kinds) | refused_not_exact_singleton 4,907; refused_value_not_data 3,839; refused_key_not_read 3,044; refused_zero_width_read 1,634 |

The same example gate would fail here too (4 of 10 and 0 of 10, no useful). No name, path, code or example text from this corpus is in this report.

## 7. Follow-ups F1 to F9: the number beside each threshold

Thresholds are DESIGN section 9's. The census numbers are stage M's (non-test partition, pre-lane product 9d1a7d06, `census/CENSUS.md`),
which is where the thresholds were set. "Head" gives the closest counter on the merged main, which counts both transports and is not the census ratio.
DESIGN decision 4: no follow-up starts until the owner agrees; an issue is filed only for each one whose threshold is met.

| Id | Follow-up | Threshold | Census: X / R2 (also apache-superset / apache-answer) | Head counter (X / R2) | Met? | Issue |
|---|---|---|---|---|---|---|
| F1 | Whole-object sources: spread `{...x}`, an identifier passed to a pattern formal | spread on 5% or more of JSX sites with attributes, either corpus | 37 / 2,194 (1.69%); **845 / 9,918 (8.52%)** (463 / 6,563 = 7.05%; 22 / 2,331 = 0.94%) | `spread_sources_unbound` 166 / 451 | **yes, on R2** | #987 |
| F2 | `children` as a prop | 100 or more components reading a path-valued child | 12 / 28 (8; 0) | not counted | no | none |
| F3 | Bare reads of a plain formal with a zero-width Use | 10% or more of admitted plain-target sites | 4 / 73 (5.48%); **53 / 306 (17.32%)** (34 / 101 = 33.66%; 15 / 16 = 93.75%) | `refused_zero_width_read` 406 / 982 | **yes, on R2** | #988 |
| F4 | `React.memo(Identifier)`, `export default memo(..)`, compound components | 10% or more of capitalised sites unresolved or refused for these reasons | **212 / 879 (24.12%)**; 353 / 5,881 (6.00%) (1,052 / 6,484 = 16.22%; 904 / 1,863 = 48.52%) | `refused_not_exact_singleton` 3,456 / 19,545 (all reasons) | **yes, on X** | #989 |
| F5 | Trace-engine descent across edges that end at a member read or at a name-mismatched tag | 10% or more plain-target bound attributes, or renamed tags on 10% of bound sites; and PR #841 merged | **36 / 308 (11.69%) plain**, 6 / 170 (3.53%) renamed; **125 / 705 (17.73%) plain**, 3 / 339 (0.88%) renamed | see section 5 and #991 | **yes on the plain-target half, both corpora; #841 is merged** | #991 |
| F6 | `href` and `src` sinks; `useState` and `useContext` provenance | after the review examples show reader value | not a census number | | decided by section 5 | none yet |
| F7 | Positional binding of identifier formals after a pattern formal | 100 or more call sites on a corpus | 50 / 10 (9; 1) | not counted | no | none |
| F8 | Pattern locals as seeds for `taint_reaches` and as parameter names for the algorithms | a user request, or a seed-based measurement that needs it | not a census number | | no trigger | none |
| F9 | Props through pass-through wrappers on an allow-list | 10% or more of Exact-singleton JSX sites with attributes, either corpus | 0 / 544 (0.00%); 20 / 1,019 (1.96%) (8 / 723 = 1.11%; 0 / 51 = 0.00%) | `refused_unknown_wrapper` 0 / 20 | no | none |

Also filed with this close-out, not follow-ups of the design but defects and decisions the lane found: the resolver defect (design section 10; #985) and the caller attribute-name anchors (design decision 5; #986).

## 8. What was not measured

- The per-site attempt ledger of DESIGN 5.7 (join of every JSX site with syntax, pre-Step-5b evidence, post-guard IDs, the three ambiguity buckets, unjoined sites, trace-boundary causes, F3's only-candidate proxy against all reached zero-width sites): not built. Its follow-up decisions stay pending where they depended on it (F3's proxy, F5's boundary causes).
- First failures and post-S1 recoveries as separate tables, and pending-versus-final props edge counts: not computed; only the final head is tabulated.
- Recall: the oracle certifies edges the product emits, it does not count flows the product misses.
- Go or Python control, the TypeScript-source corpus and Tier-A matrix were not re-run on f720d717; they were run at each slice head (Go control identical at every slice; TypeScript src S3 delta 0 only up to e63fdd66 (`s3/out3`; later folds only refuse more, not re-run); Tier-A matrix 178 ok on the S3 head).
- Review examples on Excalidraw (no full clone); the 12 forward seed queries that returned an error (reason not recorded).
- The "gains context" test is an added-edge test, not the `compare_props.py` witness classification.
- Hand-read of UNDECIDED edges: none were UNDECIDED.

## 9. Where things are

The evidence directory is local to the lane's host (not in the repository); summaries are here and in the linked issues, raw dumps are archived there.

Evidence directory `~/prism-evidence/jsx-props/`: `g/` (this report, `ATTEMPT-LEDGER.md`, paths tables, examples and reader labels), `s3/out9` and `s4/out-main` (product runs), `s4/bin/main` (binary f720d717), `s4/taint-gate`, `s4/issues`, `tools/` (`paths.py`, `examples.py`, `examples_scan.py`, `examples_pack.py`, `qualify.cjs`). Issues filed with this report: #985 resolver defect, #986 caller attribute-name anchors, #987 F1, #988 F3, #989 F4, #991 F5.
