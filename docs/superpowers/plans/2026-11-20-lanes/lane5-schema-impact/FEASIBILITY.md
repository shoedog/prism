# Lane 5: feasibility memo (schema-aware service impact)

Date: 2026-11-24. Tracking issue: #848. Public numbers only. The private pair is `pilot-producer` and
`pilot-consumer`; its detail, rows and reports stay in the local private evidence directory and only
aggregates and class names appear here. Corpus P is `apache-answer` at `6744295f`. The prototype is
`eval/schema_impact/` on branch `eval/schema-impact-feasibility`; evidence summaries (predictions, label files, ten
reports) are in `feasibility/` next to this file.

Bottom line: G1, G2 and G3 pass; G4's measured arms pass and it waits for one owner input (the owner rating of
five private reports). The study took about one working day, not 7, and cut nothing from DESIGN 6.5's cut line except
per-commit lift on the private pair (see section 7). **The pilot is an owner go-ahead; the pair order is the
owner's too (section 5).**

## 1. Measures and gates

Prism build: none used (the study is a Python prototype; no `prism` binary ran). Replay and extractors read git and
source text only.

| Id | Question | Result on P | Result on pair R (private, aggregates) |
|---|---|---|---|
| F1 | annotations vs registrations | 199 of 199 operations bound to exactly one function by `@Router` annotation (0 none, 0 several, 0 annotations without an operation); 218 literal registrations; cross-check 161 of 162 agree; residue 34 rows (list 2: 6, list 3: 26, list 4: 1, name disagreement: 1) | not applicable (code-first producer, route table) |
| F2 | client calls to operations | denominator 200: 154 unique (77.0%), 1 several, 45 none (dynamic URL 28, no operation 10, non-string key 6, unconfigured client form 1); every unique match is a string equality (P has no path parameters) | denominator 316: bound 236, candidate 2, unbound 78 (dynamic URL 44, no operation 27, unknown client method 6, dynamic method 1); recoverable URL pattern 265 of 316 (83.9%) |
| F3 | response-field reads | moved to slice P5 (not measured) | not measured |
| F4 | would the report have helped (replay) | 607 first-parent revisions, 37 usable commits (operation-only 5, field-only 26, both 6), 194 predicted locations: same commit 33, later 70, present at pin 91; 616 controls; pooled lift 33.0 (52 of 194 vs 5 of 616 touched within 10 commits) | 1,216 producer first-parent commits in the window (2023-05 to 2026-07; the `/api` mount holds only from 2023-05), 42 usable commits (105 removed routes, 0 method changes), 63 predicted locations on 22 commits: within 14 days 62, later 1, present at head 0; no controls (the cross-repository replay runs with controls off), so no lift |
| F5 | private pair behaves like P? | | routes extracted 399 (census 400 method entries, minus one non-API catch-all); 118 routes with a proven validation object, 124 more with a spread whose validation is unseen; handler site resolved 0 (393 handlers are inline functions); consumer files importing the client resolved 25 of 25; 5 reports rendered |
| F6 | cost | see section 6 | |
| F8 | standing mismatches | lists 8 / 6 / 26 / 3 (43 rows) | lists 27 / 162 of 399 routes reached by no call / 27 (the same calls as list 1) / 5 (a subset of list 1) |

Reader results (all readers were told the class definitions of DESIGN 6.1, and were not shown a verdict):

| Reader task | P | R |
|---|---|---|
| R1, 50 unique or bound joins | 50 yes, 0 no, 0 cannot tell (decided) | 50 yes, 0 no, 0 cannot tell (decided) |
| R2 on F1 residue (34 rows) | outside schema 21, stale annotation 5, extractor miss 5, cannot tell 3 | |
| R2 on F8 lists (43 rows) | outside schema 24, stale annotation 7, extractor miss 4, real mismatch 1, dead client code 1, cannot tell 6 | 27 rows (every list 1 and list 4 row): extractor miss 22, real mismatch 3, dead client code 2 |
| R3 on replay locations | 149 of the 150 packet rows labelled (cap 150 of 161 eligible; one row has no label): coordinated 41, fix after release 2, dead code 1, real mismatch 5, unrelated edit 15, false prediction 63, cannot tell 22 | all 63: coordinated 56, dead code 4, fix after release 1, cannot tell 2 |

Gates (DESIGN 6.3):

| Gate | Threshold | Result | Pair |
|---|---|---|---|
| G1 | at least 90% of operations bound and not contradicted by drift | **PASS**: 196 of 199 (98.49%); three operations are excluded by DESIGN 6.3's rule (an annotation path that disagrees with the router, an annotation method that disagrees with the router, and the one cross-check name disagreement, which reader R2 labelled an extractor miss but which the rule excludes whatever its label). The drift test covers the 162-registration cross-check population; the other 37 operations have no drift check. Not applicable to a code-first producer | P |
| G2 | P: at least 70% bound uniquely and at most 2 wrong in 50; matrix rows pass; R: at most 2 wrong in 50 | **PASS**: P 154 of 200 (77.0%), 0 wrong in 50; synthetic matching matrix 15 of 15; R 0 wrong in 50 joins. P has no path parameters, so parameter matching is exercised by the matrix and by R only | P and R |
| G3 | at least 15 usable commits and at least 2 counted locations or rows | **PASS**: 37 usable commits on P; counted locations on P: 8 (fix after release 2 on one commit, dead code 1, real mismatch 5), plus 2 counted rows of F8; these overlap (the two F8 rows are two of the eight replay locations, and the two fix-after-release locations are one removed field on one commit), so the distinct counted elements are 8; the drift in F8's 7 stale-annotation rows is 2 distinct annotations. On R: 5 counted rows of the standing join and 5 counted replay locations on 3 commits (also overlapping evidence). The threshold is 2, so overlap does not change the result. No owner arm needed | P and R |
| G4 | R: recoverable pattern at least 60%, at most 2 wrong in 50, owner rates at least one rendered report useful | **PENDING** (the measured arms pass: 83.9%, 0 wrong in 50; 5 reports exist, so not "R payoff unmeasured"); **owner rating pending** (the five private reports are at the local private directory, `private/reports/`) | R |

How far to trust these. The R1 samples were read against raw call sites by reader seats and found no wrong join, but
on P every unique match is a string equality, so a clean R1 there is close to guaranteed; the informative R1 is R's.
The R3 labels on P include 63 `false_prediction` and 22 `cannot_tell` among 149 labels: field-level prediction is the
weak part (predicted locations for a field change are calls to an operation whose wire tree holds the field, not reads
of the field; F3's measurement belongs to slice P5). The pooled lift of 33.0 is not evidence of 33 separate effects: all 52 touches come from five adjacent release-merge
commits of one day (2026-01-22; 2, 40, 2, 6 and 2 touches), and one touching commit accounts for every one of them.
Outside that burst the lift is 0 (0 of 124 predicted vs 4 of 439 control locations touched within 10 commits). The
five `coordinated` reports 06 to 10 are that one event. Treat lift as unproven; G3 does not rest on it.

## 2. Standing mismatches (F8)

P at the pin (class counts of reader R2, rows in `feasibility/f8-r2-labels.tsv`): 43 rows over four lists
(client calls with no operation: 8; operations with no registration: 6; registrations with no operation: 26; method
disagreements: 3). Classes: outside the schema by intent 24 (template pages, plugin routes, health), stale
annotation 7, extractor miss 4 (installation routes registered with a configured prefix), cannot tell 6, real
mismatch 1, dead client code 1. Counted rows: 2.

R (aggregate): list 1 client calls with no route 27; list 2 route entries no client call reaches 162 of 399; list 3
(R form: client calls with no route) the same 27 calls as list 1; list 4 5 calls, a subset of list 1 (5 of the 27 match
a route under another method). R2 on the 27 rows: 22 extractor misses (19 are routes whose path ends in a multi-segment wildcard
parameter, 3 are multi-line template paths that kept a trailing newline), 3 real mismatches and 2 dead client code.
Counted rows: 5. The extractor misses are limits of the prototype's id rules (DESIGN section 9, "no id for a served
operation"), not of the pair.

## 3. Reports

Ten public reports from P's replay are in `feasibility/reports/` (`index.tsv` lists each one's class and commit):
counted classes first (one fix after release, one dead code, three real mismatch), then five coordinated. Three are
quoted in full in `feasibility/reports/02.md`, `03.md` and `05.md` (21 to 22 lines each); report sizes run from 21 to 286 lines (`04.md` 243 and `06.md` 286 are the largest). `01.md` (a release merge, 86 lines) shows that the producer location for a field change is the schema file line today,
because struct-field binding (`go_struct_field`) is a pilot slice. Five reports from R's replay are rendered
privately (one fix after release, two dead code, two coordinated); the memo names only their classes.

## 4. What the study found that the design did not say

1. Pair R is nearer the proposal's shape than expected on the client side (one axios instance, a bare-specifier
   module path resolved through `jsconfig` `baseUrl`) and farther on the producer side: no committed schema, 399 route
   entries declared as data, 393 with inline handlers (no handler site to bind), validation objects imported from other
   files for about 124 routes.
2. Prototype gaps that cost recall on R and are named for the pilot's adapters: `jsconfig` `baseUrl` resolution
   (the prototype got by with an alias per top-level directory), bare-identifier URLs through a single-assignment
   local (28 of R's calls), template literals with a parameter inside a segment (16),
   a constant endpoint table (2 to 3), method values from `.bind` (1), interceptor members that are not calls (4).
3. Multi-segment wildcard route parameters (`{path*}`) are 19 of R's list-1 misses; DESIGN section 9 already lists
   this open class and stage D must give it an id rule and a matching rule.
4. The route set of R is only recoverable with its `/api` mount from 2023-05, so the cross-repository replay window
   is 37.7 months, not the full history.
5. `replay_cross` runs without controls, so R has no lift; and its blame attributes touches to first-parent merge
   commits (33 of 63 locations share one merge). Tags on R do not separate coordinated from fix-after-release (every
   consumer touch maps to the same release tag), so R's `coordinated` count is a broad residue. These weaken R's
   replay evidence; the standing join and the R1 sample are not affected.
6. Public readers on P (R1, R2, R3) ran as gpt-6.1-terra read-only sessions, not Sonnet seats (the run's Claude agent
   cap); private readers were Claude seats as the rules require. Predictions were written by the implementing
   worker before each run (timestamps precede outputs); outputs were staged by the workers and copied by the
   orchestrator.

## 5. Pilot-order evidence and recommendation

Default rule (DESIGN 6.4): recommend private-first when pair R passes G4 and the owner rates at least three of its
five private reports useful; otherwise public-first. G4's measured arms pass; **the rule cannot be applied until the
owner has rated the five private reports**. If the owner rates three or more as useful, the rule says private-first;
fewer, public-first. The order is the owner's either way.

| | Public-first | Private-first |
|---|---|---|
| Built first | P1 schema reader and ids; P2 `swaggo_annotation` and `go_struct_field`; P3 consumer for P | P1; P2-R route-table schema source and adapter; P3 consumer for P and R |
| First report on the public pair | after P4 (about 17 to 23 days from the go-ahead) | after P6 (about 24 to 32 days; P's producer adapters come last unless pulled forward at +3 to 4 days) |
| First report on the owner's pair | after P6 (about 25 to 33 days) | after P4 (about 16 to 22 days) |
| What R needs that is not built | `route_table` and the consumer gaps of section 4 item 2 at P6 | the same, at P2-R and P3 |
| Field-level on R | none (R has no field source; validation objects cover about 60% of routes at best) | none |

Evidence for each pair: P has an annotation-complete producer (199 of 199), a measured replay with a lift, and 8 counted
replay locations, and is where field-level work (P5) can be measured. R has the real producer/consumer relationship
(236 of 316 calls bound, 0 wrong in 50), a route table the lane 12 Hapi adapter will extract as a first-class fact,
but its producer has no schema, so its impact is operation-level only, and its replay evidence is thin (no controls,
merge attribution). Lane 12 is running the object-literal table and Hapi plugin work; P2-R should consume that route
listing instead of adding its own, which lowers P2-R to about 2 to 3 days.

## 6. Cost and refreshed estimates

Prototype lines (counted as non-blank, non-comment source lines by a small counter; `scripts/file_size.py` weighted counts differ by up to 10 per area; every module is under 600): producer 857 (Go 327, route table 530), consumer 802, schema reader and ids
413, standing mismatches 60, replay and renderer 862, outbound-HTTP count 27. Run time on P: producer 0.77 s,
consumer 0.81 s, mismatch 0.12 s, replay of 37 commits 74 s. Outbound HTTP call sites in the Go and Python code:
P 4 Go and 0 Python; R 0 in both repositories (the consumer is JavaScript and TypeScript only), so a Go or Python
service-consumer adapter would have nothing to bind on either pair.

| Slice | Plan | Revised | Why |
|---|---|---|---|
| E-1, E-2 (matrices) | 2 to 3 | 2 to 3 | the private enumeration already gives about 30 neutral client shapes |
| Stage D | 2 | 2 to 3 | wildcard parameter ids, bare-specifier resolution, constant-endpoint tables |
| P1 | 3 to 4 | 3 to 4 | `schema.py` ports directly |
| P2 (public) | 3 to 4 | 3 to 4 | |
| P2-R | 3 to 4 | 2 to 3 | consumes lane 12's route listing |
| P3 | 3 to 4 | 4 to 5 | largest adapter; five resolver and URL gaps named in section 4 |
| P4 | 3 to 4 | 3 to 4 | |
| P5 | 5 to 6 | 5 to 6 | 63 of 150 adjudicated field locations were false predictions, so the field-read step is where precision work sits |
| P6 | 3 to 4 | 3 to 4 | |

Totals (sum of the table): public-first about 25 to 33 days, private-first about 24 to 32 (24 to 31 as planned). DESIGN 7's table:
no row triggers (G1, G2, G3 pass; G4 waits on the owner rating; P5's measurement is still ahead).

## 7. What was not done or not verified

- The owner rating of the five private reports (G4's third arm, the pilot-order rule, G3's owner arm is not needed).
- F3 (response-field reads) belongs to slice P5.
- Per-commit lift on the private pair: `replay_cross` runs controls off; not re-run with controls.
- R3 labels were not cross-checked beyond reading the sentences of the eight counted P locations; 22 `cannot_tell`
  labels sit in one packet of 50.
- The Python prototype parses with the upstream tree-sitter Go (0.25.0) and TypeScript (0.23.2) wheels, not the vendored,
  patched grammars of the Rust build; parse behaviour may differ on edge syntax.
- The full repository test suite was not run (CI runs it); the prototype's tests were: 99 passed, 0 failed.
- No `prism` binary ran; no CPG or navigation cache was touched; no product crate changed.
- The cut line of DESIGN 6.5 was not needed; the roadmap-corrections file is written locally
  (`~/prism-evidence/boundary-impact/roadmap-corrections.md`) for the owner.
