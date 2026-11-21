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

Status on 2026-11-21: designs written for all seven; the plans are being drafted against the code, and the
design review of these lanes has not run yet. A row without a day figure has no finished plan.

| Lane | Directory | Issue | First deliverable | Days (estimate) | Depends on |
|---|---|---|---|---|---|
| 6. Provenance findings by proven identity | `lane6-provenance-identity/` | to be filed | a census memo: precision of today's findings on 180 real diffs (300 labelled), with a go gate; then structural fixes for every language | about 23.5 | lane 2's first slice for the shared identity entry point, or this lane carries it; its edits in `targets/` are additive and follow lane 2 |
| 7. Rule identity on findings | `lane7-rule-identity/` | to be filed | taint findings carry rule id and CWE (JSON and review output), then a registry and opt-in SARIF rule ids with CWE tags | 12 to 13 | nothing; its taint slice rebases over PR #841 if merged; lane 4's sink slice waits for it |
| 8. One taint answer from both engines | `lane8-taint-parity/` | to be filed | a parity census and a committed parity matrix for review taint and `taint-reaches`, then a named reason on every refused hop | 31 to 35 | PR #841 merged; the benchmark pin task of lane 4 (or it does that task itself); lane 3's props slice for one matrix row; rule ids from lane 7 when available |
| 9. JavaScript parameter binding | `lane9-parameter-binding/` | #595, #434 | a census on the current build and the fix for spread-argument misbinding | 9 if only the spread fix runs; up to 31 if every slice passes its census gate | nothing; shares a harness-options change with lane 4; its arrow-parameter slice rebases over lane 3; hands handler-registration cases to lane 12 |
| 10. Subscripts in the data-flow graph | `lane10-keyed-subscripts/` | to be filed | a census, then a node-based path builder that changes no output | plan in drafting | nothing; coordinates with lane 4, which reads subscript writes |
| 11. Shared measurement kit | `lane11-measurement-kit/` | to be filed | `scripts/measure/` reproducing an existing lane gate from pinned inputs | 25 to 29 | nothing; a lane can adopt it if its measurement stage starts after the comparator slice merges |
| 12. Routes as a first-class fact | `lane12-route-model/` | to be filed | `prism nav routes` for Flask, FastAPI and Express with mounts composed across files | 24.5 for waves 1 and 2 plus object-literal tables; 2.5 more per further framework | nothing; lane 5 can consume the listing |

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
  across the file. On three decoy-heavy fixtures 14 to 31% of its findings were right.
- **Lane 8.** Default review taint and `taint-reaches` disagree on 6 of 26 probe fixtures; five come from
  the trace engine's rule for entering a callee. Making review use the trace engine would remove findings.
- **Lane 9.** The benchmark numbers behind the proposal were partly stale and partly about HTTP handler
  registration, which belongs to lane 12. The benchmark alone does not justify rest-parameter or
  `arguments` work, so each slice has its own census gate. One defect is certain: `f(...list, v)` binds
  `v` to the wrong parameter today.
- **Lane 10.** The first defect is recall, not precision: a value written through a subscript
  (`d["a"] = t`) reaches no later read, in Python, JavaScript and Go. Key-sensitive paths are what keep the
  recall fix from connecting every key to every other.
- **Lane 12.** Inline Express handlers already have graph identities; they are unresolved in the route
  records only because of an older assumption. Handler arguments are value references, so the call-site
  resolver does not apply to them as it stands.

## Order and parallelism

The lanes touch different subsystems (ORCHESTRATION section 11 lists the shared resources). With one
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
| 5 | Pilot pair order (public or private pair first) | **undecided (2026-11-21)**: taken at the memo, which presents the evidence; the go-ahead names the order |
| 6 | Remove provenance from the default review suite and targets set until it is rebuilt? | no; defaults unchanged |
| 8 | How the MCP server gets the sink catalog | move the sink classifier into the core crate by move-only changes, as the lane's last slices; the alternative changes the crate layering rule |
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

**Lanes 6 to 12.** For each lane: a research agent with probes; the design; then a plan drafted by an
Opus-5.5 agent that verified every path and interface against the code and reported where the design
contradicted it (those corrections are applied to the designs as they arrive). The design review of these
lanes has not run yet.

## Evidence

Research reports, probes, review reports and private bindings are under `~/prism-evidence/lanes-2026-11-20/`
(local, not in this repository). Each lane's own evidence directory is named in its plan.
