# Eleven lanes, planned 2026-11-20 and 2026-11-21

Plans for the investments ranked 2 to 5 in the 2026-11 portfolio review (lanes 2 to 5), and for seven
further lanes that the planning of those four showed to be worth doing (lanes 6 to 12). Each lane has a
design (`DESIGN.md`: what is built, why, the alternatives, the owner decisions with defaults) and a plan
(`PLAN.md`: stages, slices, files, interfaces, test rows, gates). `ORCHESTRATION.md` is the shared operating
manual for the agent that runs a lane.

## Lanes 2 to 5

| Lane | Directory | Issue | First deliverable | Days (estimate) | Depends on |
|---|---|---|---|---|---|
| 2. Faultpath target discovery | `lane2-faultpath-target-discovery/` | #613 | `prism targets --inventory python-deps` lists HTTP dependency calls faultpath can inject at | 13 to 14.5 | nothing; coordinates with PR #841 on two CLI files |
| 3. React value flow across JSX props | `lane3-jsx-props-flow/` | #596 | definitions for destructured props, then attribute → prop edges | 22 to 27 for stages M to S4 | nothing; leaves `src/cpg/trace.rs` to PR #841 |
| 4. Prototype-pollution tracing | `lane4-prototype-pollution/` | #603 | a 3 to 4 day experiment with a pre-stated gate; then facts, listing, sink | 3 to 4, then 18 to 25 | the experiment's gate; the sink slice rebases over PR #841 and waits for lane 7's taint slice if that lane runs |
| 5. Schema-aware service impact | `lane5-schema-impact/` | to be filed | a feasibility memo after about 7 days; the pilot needs an owner go-ahead | about 7, then 24 to 31 (20 to 26 if field reads fail) | owner go-ahead after the memo, which also names the pilot pair order |

The estimates are sums of the stage and slice estimates in each plan, in working days of orchestrated
agent time including review rounds. They grew when the design reviews were folded (see "How the plans were
checked"): the reviews added measurements, enumeration passes and larger executed test matrices.

## Lanes 6 to 12

Status on 2026-11-21: designs and plans written for all seven, and one design review folded into each.

| Lane | Directory | Issue | First deliverable | Days (estimate) | Depends on |
|---|---|---|---|---|---|
| 6. Provenance findings by proven identity | `lane6-provenance-identity/` | to be filed | a census memo: today's precision per origin class and recall against a 400-line source census on 240 real diffs, with a go gate; then structural fixes for every language | about 29.5 | lane 2's first slice for the shared identity entry point, or this lane carries it (one owner, whichever starts first); its edits in `targets/` are additive and follow lane 2 |
| 7. Rule identity on findings | `lane7-rule-identity/` | to be filed | taint findings carry rule id and CWE for each reached sink occurrence (JSON and review output), with identities audited at its design review; then a closed registry and opt-in SARIF rule ids with CWE tags | 15.5 to 16 | nothing; its taint slice rebases over PR #841 if merged; lane 4's sink slice reads the rule from the field when this lane's taint slice has merged first, and is migrated by this lane otherwise |
| 8. One taint answer from both engines | `lane8-taint-parity/` | to be filed | a parity census and an occurrence-level parity matrix, then a reason on every lost sink and the trace engine's return and depth repairs | 43.5 to 44.5 | PR #841 merged; lane 4's benchmark pin task (or it does that task itself); lane 3's props slice for one row; rule ids from lane 7 when available; Lane 9's callback and argument-follow slices wait for this lane's walk-repair slice |
| 9. JavaScript parameter binding | `lane9-parameter-binding/` | #595, #434 | a census on the current build and the fix for spread-argument and chained-call misbinding | 12 if only that fix runs; up to 37 if every slice passes its census gate | nothing to start; shares a harness-options change with lane 4; its arrow-parameter slice rebases over lane 3; its callback and argument-follow slices wait for lane 8's descent check; hands handler-registration cases to lane 12 |
| 10. Subscripts in the data-flow graph | `lane10-keyed-subscripts/` | to be filed | a census memo with a go or no-go, then a node-based path builder that changes no output | 28 to 33, plus 3 to 4 each for two slices the design review may admit | nothing to start; takes the next free cache version; edits one lookup in lane 3's object-literal binder and the argument-binding path lookups, coordinated with lanes 3 and 9; leaves the trace engine to PR #841 and lane 8 |
| 11. Shared measurement kit | `lane11-measurement-kit/` | to be filed | an executed truth matrix, then `scripts/measure/` reproducing an existing lane gate and a labelled fixture pair from pinned inputs | about 32.5 | a consuming lane named before its formats are fixed; otherwise they ship as provisional and adoption is reported as not met |
| 12. Routes as a first-class fact | `lane12-route-model/` | to be filed | `prism nav routes` for Flask, FastAPI and Express with mounts composed across files | 28.5 for waves 1 and 2 plus object-literal tables; 2.5 more per further framework | nothing; lane 5 can consume the listing |

## What the research changed

Each plan starts from facts verified on `origin/main` at 1179055f. Several differ from the premises the
lanes were proposed with:

- **Lane 2.** `kind: external_call` has one source, an algorithm that only sees in-repository callers of
  changed in-repository functions. A dependency call can never get such a row, so #613 needs a new row
  source, not a classification fix. Faultpath already has its own scanner for standard-library filesystem,
  parsing and configuration calls; HTTP is the uncovered family. A targets contract minor (1.1) that prism
  may emit already exists.
- **Lane 3.** Destructured props, the dominant component shape (194 of 230 component-like definitions in
  Excalidraw by a rough count), have no definition in the data-flow graph today. That is the first slice,
  and it is done through a new accessor so that positional slots and the other readers of parameter lists
  do not change. Edges into destructured props end at a definition, which is what the trace engine behind
  `taint_reaches` requires; the existing object-literal edges (#594) end at a use and stop that engine.
  A JSX attribute's source is the line-keyed use node, and member elements (`<Ns.Item>`) bind only through
  namespace imports, because other member forms can resolve to an unrelated component.
- **Lane 4.** The issue's premise is out of date: `target[key] = value` is already a definition and key
  taint already reaches the write line (41 of 95 benchmark cases with ground truth traced on an older
  build). The missing pieces are which writes can pollute, which operand selects the prototype (the key
  arrives as a line-level use of its name, so the sink matches by name), guards, and a sink. A receiver
  false positive caused by alias twins and same-line propagation is measured, not fixed. The lane needs no
  data-flow producer change.
- **Lane 5.** Route records keep no method or path, there is no client-call model, and the private pair
  available locally is code-first with no committed schema. A public Go and TypeScript pair with a
  committed schema that 68 first-parent commits change (`bench-repos/apache-answer`) carries the
  feasibility study. The private pair is measured locally as well (standing join, reader-checked joins,
  cross-repository replay, five private reports), because the public pair alone cannot show value on the
  owner's code.
- **Lane 6.** The noise is structural, not a matter of tuning: the algorithm classifies whole source lines
  by substring, tags names that are not variables (modules, callees), and looks definitions up by name
  across the file. On three decoy-heavy fixtures 14 to 31% of its findings were right. The design review
  added that a call into a database or HTTP library proves nothing about its result
  (`sqlite3.complete_statement`), so origin tables are per call, and that module-level code has no use nodes
  in the graph, so it is examined from syntax.
- **Lane 8.** Default review taint and `taint-reaches` disagree on 6 of 26 probe fixtures; five come from
  the trace engine's rule for entering a callee. Making review use the trace engine would remove findings.
  The design review also found defects of that engine on today's code (a return crossing into an unrelated
  caller, depth accounting that depends on exploration order); the lane repairs them before it relaxes
  anything.
- **Lane 9.** The benchmark numbers behind the proposal were partly stale and partly about HTTP handler
  registration, which belongs to lane 12. The benchmark alone does not justify rest-parameter or
  `arguments` work, so each slice has its own census gate. Two defects are certain today: `f(...list, v)`
  binds `v` to the wrong parameter, and a chained call (`a.m(x).m(w, z)`) hands the inner call the outer
  call's arguments as `exact`. The design review also found that an export named like a global
  (`parseInt`) becomes the Exact target of unimported calls to the global; that resolver defect is filed,
  and no slice binds into such a target.
- **Lane 10.** The first defect is recall, not precision: a value written through a subscript
  (`d["a"] = t`) reaches no later read, in Python, JavaScript and Go. Key-sensitive paths are what keep the
  recall fix from connecting every key to every other. The design review added the distinction the
  lane now rests on: two paths are the same storage for certain only when every step is static and the base
  is proven the same; anything else is possible overlap, which gives an edge that is not `exact` and kills
  nothing.
- **Lane 12.** Inline Express handlers already have graph identities; they are unresolved in the route
  records only because of an older assumption. Handler arguments are value references, so the call-site
  resolver does not apply to them as it stands.

## Order and parallelism

ORCHESTRATION section 11 lists the files and resources the lanes share and the rule for each. With one
orchestrator, for lanes 2 to 5: lane 2 first; then the censuses of lanes 3 and 4 together; lane 3's
slices; lane 4's slices if its gate passes; lane 5's feasibility study in the waits. Of lanes 6 to 12,
lane 11 (the kit other lanes measure with) and lane 7 (rule identity, which lanes 4 and 8 use) pay off most
when they run early; lane 8 waits for PR #841; lane 12 is best finished before lane 5's pilot. With several
orchestrators the lanes can run side by side, subject to "one measurement process at a time" on the host.

## Decisions waiting for the owner

Every design has an "Owner decisions" table with the default the plan applies. Nothing blocks a lane's
start. The decisions with the most consequence:

| Lane | Decision | Default |
|---|---|---|
| 2 | Should prism's inventory also cover the standard-library families faultpath's scanner already covers? | yes, as the second slice, with a row-for-row parity gate against the scanner (**confirmed 2026-11-20**) |
| 2 | Who runs the live httpie proof? | the lane runs a plan-then-run proof (required gate G-live); the pilot-level re-run goes to the tooling controller (changed by the design review) |
| 3 | Which follow-ups run after the first four slices (spread pass-through, `children`, wrapper resolution, trace-engine descent for plain `props` and for renamed tags, props through non-React wrappers)? | none without the census number stated beside each; props through unknown wrappers are refused and counted |
| 4 | Is the experiment's result enough to fund implementation? | pre-stated: 25 or more converted cases continues; 15 to 24 asks; fewer stops (**confirmed 2026-11-20**); applied to the strict reading, with the liberal reading shown beside it |
| 5 | Measure the private pair in the feasibility study? | yes, locally (**confirmed 2026-11-21**) |
| 5 | Start the pilot after the feasibility memo? | **wait for an explicit go-ahead** |
| 5 | Pilot pair order (public or private pair first) | **undecided (2026-11-21)**: taken at the memo, which presents the evidence; the go-ahead names the order. The private pair is confirmed as the real pair the lane is for (2026-11-21) |
| 6 | Remove provenance from the default review suite and targets set until it is rebuilt? | no; defaults unchanged |
| 7 | Does the new `rules` key appear by default in JSON and review output? | yes: an additive key, golden files re-recorded once (**confirmed 2026-11-21**) |
| 8 | How the MCP server gets the sink catalog | factor the shared matching functions out, then move them into the core crate (**owner, 2026-11-21**); the lane comes back to the owner only if that factoring meets its stop rule |
| 9 | Which slices run | each by its own census threshold; the spread-argument fix always |
| 10 | JavaScript: do identifier-like string keys unify with dot fields (`o["x"]` and `o.x`)? | yes |
| 12 | Which frameworks beyond Flask, FastAPI, Express, gin, net/http and chi | by census counts, one slice each |

## How the plans were checked

**Lanes 2 to 5.** Two rounds. First, for each lane a fresh Sonnet-5.5 agent read `ORCHESTRATION.md`, the
design and the plan as the orchestrator who would have to run them, verified the code references against
the worktree, and listed every point where it would have had to guess, every wrong fact and every step
that could not run as written; all four reports were folded. Second, one design review per lane by a
Fable-5.2 reviewer that wrote and ran probes. All four came back "fix" for both design and plan, with six
to ten material findings each, and all were folded on 2026-11-21. The largest changes: lane 2 now anchors
rows the way the consumer does and has a required plan-then-run gate; lane 3's edge starts at the node
that exists, and wrong member bindings are refused; lane 4's sink matches the key by name and its truth
runner has a positive control; lane 5 measures value on the private pair and its replay is adjudicated by
a reader. Each lane still runs the two-reviewer design review at its stage D, after its census and test
matrix.

**Lanes 6 to 12.** For each lane: a research agent with probes; the design; a plan drafted by an
Opus-5.5 agent that verified every path and interface against the code and reported where the design
contradicted it (applied to the designs); then one design review by gpt-6.2-astra with probes, folded by an
agent that verified and re-graded each finding. All seven reviews came back "fix" for both documents. Every
finding that was checked held, and several were defects of today's code rather than of the plans: those are
pinned as known-wrong test rows and filed at each lane's close-out. A calibration on lanes 3 and 5 compared
this reviewer with the Fable-5.2 reviewer on the same documents: neither produced a false finding, the
Fable reviewer found more (seven valid findings the other missed, against two), and the astra reviewer
over-stated severity, so its tags were re-graded. Its valid findings that the first folds of lanes 3 and 5
had not seen were applied to those lanes as well.

## Evidence

Research reports, probes, review reports and private bindings are under `~/prism-evidence/lanes-2026-11-20/`
(local, not in this repository). Each lane's own evidence directory is named in its plan.
