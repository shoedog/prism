# S1b-4 owner questions — 2026-10-01

## OQ-S1b4-1: alias exports on the namespace route (OPEN)

MEASURED: C220 (`probes/controls_gen.py`, JSX and TSX) has `make()` return its
unique nested function f and exports `const f = make()`. Base 915fca43 R3
resolves `ns.f()` to that f@2. The independent Node check verifies the returned
callable's source identity, not merely its return value:
`target/plan-s1b4/valueflow-c220/{check.mjs,output.jsonl}`. Base rows are in
`target/plan-s1b4/base/controls-r2/C220_ns_export_alias_valueflow_*.dump.jsonl`.

READ: `js_ts_local_export_target` maps Alias to UnprovenLocal
(`src/ast/js_binding.rs:392`); `resolve_one_inner` treats that as BlockedClaim
(`src/js_exports.rs:322`). Literal §3.4 reuse of D4's table therefore has no
export candidate. This is a mechanism-level forecast, not an executed prototype
result. The existing lexical auditor cannot certify that removal as wrong: it
does not follow `make()`'s return into the exported value.

ASSUMPTION / recommendation: preserve R3 base behavior for alias export terminals
using namespace-only opacity metadata, including its propagation through named
and star barrels, while preserving D4 rows and counters exactly. This extends
S1b-4's owned facts/producers and export traversal by an estimated 60–100 src
lines plus 150–250 tests; the rest of §3.4 still fixes proven bindings.

READ alternative: keep literal D4 refusal on R3. It needs less code, but C220
loses one right edge in each grammar, and alias-export value-flow losses become
a new accepted cost. No real-corpus alias-export loss has been measured. The
planner does not accept that cost or choose the scope extension for the owner.

## Custody / prototype location (OPEN; operational, not a semantic cost)

READ: the environment makes `.git` read-only and disables escalation. The
planner cannot create `proto/s1b-4` or commit `plan/s1b-4`. The controller can
create the prototype worktree under the permitted evidence root:

```bash
git worktree add -b proto/s1b-4 target/plan-s1b4/proto 915fca43d84ea1730959453091fbf8ae97763af8
```

ASSUMPTION / recommendation: controller creates that worktree and handles Git
writes. Alternative: owner authorizes an isolated source snapshot there, with
patch, source/binary hashes and later controller import to proto/s1b-4. That
alternative loses commit-bound build custody until import and rebuild. Neither
alternative has been authorized in response to the pending question.

## Controller disposition, 2026-10-01

- **OQ-S1b4-1: RESOLVED as an application of existing owner rulings, so no new decision is needed.**
  - OQ12 (narrowed) and Option K already say that an Alias binding keeps base behavior. Only a binding that provably holds no function may drop, and an unproven position keeps base.
  - An alias export terminal reached through R3 is an Alias, so it keeps base. This is the planner's recommendation: namespace-only opacity metadata, propagated through named and star barrels, with D4 rows and counters preserved exactly.
  - The literal-D4-refusal alternative would create a new accepted cost, and the owner has not approved one.
  - The owner dropped LOC caps (2026-09-30), so the scope extension needs no approval.
- **Custody: RESOLVED.**
  - The controller created the git worktree `target/plan-s1b4/proto` on branch `proto/s1b-4` at `915fca43`.
  - The planner edits files in both trees and never runs git write commands. The controller commits at each stable point the planner names.
