# Lane PKG spec review, round 2 of 2 (FINAL under the cap)

Same role, goal and output format as round 1 (`/Users/wesleyjinks/prism-evidence/pkgres/review/spec-r1-brief.md`; read it first). In short: rigorous, never invented findings. WRONG/SMELL plus MATERIAL/IMMATERIAL tags. A fix and alternatives, and a self-critique, for each finding. Read-only: no git writes, delete `target/` afterward.

## What changed
Round 1 had both reviewers return FIX (`~/prism-evidence/pkgres/review/spec-r1-{opus,sol}.md`). Repair R1 is now committed:
- proto `origin/proto/workspace-package-resolution` = `92c1d0bc` + `03fa9c29`;
- plan `origin/plan/workspace-package-resolution` = `fa3bcb02` (PR #344), with `R1-REPORT.md` in the packet.

The repair claims:
- it folded every round-1 WRONG;
- a three-way result (`Bound` / `ProvenUnresolved` / `Unsupported`), where S2 treats only `ProvenUnresolved` as out of model (owner S2-O9) and `Unsupported` fails closed;
- a TS 5.9.3 differential gate (`probes/` in the packet): 5,096 cases, zero wrong bindings. 2,242 Bound, 861 ProvenUnresolved, 1,993 Unsupported, of which TS binds 1,651, mostly unindexed writer extensions;
- public X, installed X, R and T call streams identical to main; installed X gains one correct module proof;
- private F (controller): 0 changed, 0 lost.

## Priorities
1. **Are the round-1 WRONGs actually fixed?** Rerun your own round-1 repros against the new proto and the oracle.
2. **The `ProvenUnresolved` classification is the S2-critical boundary.** Is there any input where PKG says `ProvenUnresolved` but TS binds? That would become a silent false Exact in S2. Is any `Unsupported` case misfiled as `ProvenUnresolved`?
3. **Is the differential gate adequate?**
   - Do its axes cover the round-1 failure classes?
   - Is the oracle invoked with the writer's real options and mode?
   - Could it pass vacuously, for example because fixtures lack the files that would discriminate?
   - Is it rerunnable by the implementer?
4. **Regressions introduced by R1**, including the cache and the paths/lane-P interaction.
5. **IMPLEMENTOR dispatch** from the committed branch.

For the controller's convergence decision, state whether any material WRONG is a **new family** or a **closed instance** with a bounded fix.

Verdict: APPROVE, FIX or REJECT. List what you did not check.
