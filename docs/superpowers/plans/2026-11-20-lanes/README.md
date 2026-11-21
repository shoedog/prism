# Four lanes, planned 2026-11-20

Plans for the investments ranked 2 to 5 in the 2026-11 portfolio review. Each lane has a design
(`DESIGN.md`: what is built, why, the alternatives, the owner decisions with defaults) and a plan (`PLAN.md`:
stages, slices, files, interfaces, test rows, gates). `ORCHESTRATION.md` is the shared operating manual for
the agent that runs a lane.

| Lane | Directory | Issue | First deliverable | Days (estimate) | Depends on |
|---|---|---|---|---|---|
| 2. Faultpath target discovery | `lane2-faultpath-target-discovery/` | #613 | `prism targets --inventory python-deps` lists HTTP dependency calls faultpath can inject at | 10 to 12 | nothing; coordinates with PR #841 on two CLI files |
| 3. React value flow across JSX props | `lane3-jsx-props-flow/` | #596 | definitions for destructured props, then attribute → prop edges | 18 to 21 for slices S1 to S4 | nothing; leaves `src/cpg/trace.rs` to PR #841 |
| 4. Prototype-pollution tracing | `lane4-prototype-pollution/` | #603 | a 2 to 3 day experiment with a pre-stated gate; then facts, listing, sink | 3, then 15 to 18 | the experiment's gate; the sink slice rebases over PR #841 |
| 5. Schema-aware service impact | `lane5-schema-impact/` | to be filed | a 3 to 5 day feasibility memo; the pilot needs an owner go-ahead | 5, then 14 to 23 | owner go-ahead after the memo |

The estimates are sums of the stage and slice estimates in each plan, in working days of orchestrated
agent time including review rounds. Lanes 3 and 4 come out above the portfolio review's figures (6 to 10
and 8 to 15): lane 3 because destructured props need definitions before any attribute can bind, lane 4
because the rules are fixed against an executed test matrix first.

## What the research changed

Each plan starts from facts verified on main at 1179055f, and three of them differ from the premises the
lanes were proposed with:

- **Lane 2.** `kind: external_call` has one source, an algorithm that only sees in-repository callers of
  changed in-repository functions. A dependency call can never get such a row, so #613 needs a new row
  source, not a classification fix. Faultpath already has its own scanner for standard-library filesystem,
  parsing and configuration calls; HTTP is the uncovered family. A targets contract minor (1.1) that prism
  may emit already exists.
- **Lane 3.** Destructured props, the dominant component shape (194 of 230 component-like definitions in
  Excalidraw by a rough count), have no definition in the data-flow graph at all today. That is the first
  slice. Edges into destructured props end at a definition, which is what the trace engine behind
  `taint_reaches` requires; the existing object-literal edges (#594) end at a use and stop that engine.
- **Lane 4.** The issue's premise is out of date: `target[key] = value` is already a definition and key
  taint already reaches the write line (41 of 95 benchmark cases with ground truth traced on an older
  build). The missing pieces are which writes can pollute, which operand is the key, guards, and a sink.
  The lane needs no data-flow producer change.
- **Lane 5.** Route records keep no method or path, there is no client-call model, and the private pair
  available locally is code-first with no committed schema. A public Go and TypeScript pair with a
  committed schema and 204 commits of schema history (`bench-repos/apache-answer`) carries the feasibility
  study; the private pair is measured locally for realism.

## Order and parallelism

The lanes touch different subsystems (ORCHESTRATION section 11 lists the shared resources). With one
orchestrator: lane 2 first; then the censuses of lanes 3 and 4 together; lane 3's slices; lane 4's slices
if its gate passes; lane 5's feasibility study in the waits. With several orchestrators all four can run
at once, subject to "one measurement process at a time" on the host.

## Decisions waiting for the owner

Every design has an "Owner decisions" table with the default the plan applies. Nothing blocks a lane's
start. The decisions with the most consequence:

| Lane | Decision | Default |
|---|---|---|
| 2 | Should prism's inventory also cover the standard-library families faultpath's scanner already covers? | yes, as the second slice, with a row-for-row parity gate against the scanner |
| 2 | Who runs the live httpie proof? | the lane's orchestrator, using faultpath read-only, one day at most |
| 3 | Which follow-ups run after the first four slices (spread pass-through, `children`, wrapper resolution, trace-engine descent for plain `props`)? | none without the census number stated beside each |
| 4 | Is the experiment's result enough to fund implementation? | pre-stated: 25 or more converted cases continues; 15 to 24 asks; fewer stops |
| 5 | Start the pilot after the feasibility memo? | **wait for an explicit go-ahead** |
| 5 | Pilot pair | the public pair first; the private pair's adapter last |

## Evidence

Research reports, probes and private bindings are under `~/prism-evidence/lanes-2026-11-20/` (local, not in
this repository). Each lane's own evidence directory is named in its plan.
