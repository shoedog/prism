# Independent evaluation: can one controller session orchestrate three prism lanes? Plus a design for the JS/TS parameter-Def gaps lane

You are an independent senior/principal evaluator for the owner of **prism**, a Rust static code-slicing and navigation tool for defect-focused review (repo `/Users/wesleyjinks/code/prism-secbench`, now on `origin/main`; fetch read-only if needed). Be candid. The owner wants your honest judgement, not reassurance.

## Context to read (read-only)
- **Controller handoffs:**
  - `/Users/wesleyjinks/prism-evidence/HANDOFF-controller-2026-10-04b.md` (current; its UPDATE blocks are newest-first);
  - `/Users/wesleyjinks/prism-evidence/HANDOFF-controller-2026-10-04.md`;
  - `/Users/wesleyjinks/prism-evidence/HANDOFF-controller-2026-10-03.md`.
- **Memory index and files:** `/Users/wesleyjinks/.claude/projects/-Users-wesleyjinks-code-slicing/memory/MEMORY.md` plus the files it links. These hold the owner's standing preferences on model lanes, review policy, merge-on-green and measure-first.
- **Global steering:** `/Users/wesleyjinks/.claude/CLAUDE.md`, covering proportionality, review severity, convergence caps, attribution control and durable custody. Also the repo's `CLAUDE.md`.
- **Measurement synthesis:** `/Users/wesleyjinks/prism-evidence/survey/synthesis.md` (read the "Measured outcome" section) and the two surveys beside it.
- **SecBench results:** `docs/superpowers/plans/2026-10-04-secbench-ground-truth/R1.md` on main, plus the prism-bug fixtures under `/Users/wesleyjinks/prism-evidence/meas/secbench/repair-r1/product-probes/`. Also the reviews `/Users/wesleyjinks/prism-evidence/meas/review/r2-opus-A.md` and `r1-*-A.md`.
- **Tier-A:** `docs/superpowers/plans/2026-10-04-tier-a-quick-repair/r1-readout.md` and `r2-readout.md`.
- **Lessons from the recent parked lanes** (S2 and PKG: global-negative-proof yield collapse, cyber-filter kills, overclaimed readouts caught by review):
  - `/Users/wesleyjinks/prism-evidence/s2/review/`;
  - the `OQ-s2.md` park note on `origin/plan/s2-import-qualifiers`;
  - `docs/superpowers/pipeline-lessons.md` on main.
- **Relevant source:**
  - `src/data_flow.rs` (around line 600: member-only parameters get no Def, an intentional field-isolation guard);
  - JS/TS parameter extraction in `src/ast.rs` and the `src/ast/` modules;
  - `src/repo_loader.rs` (dist/build skip);
  - `src/cpg/`.

## The owner's question
The owner agreed the next product slice is **"JS/TS parameter-Def gaps"**:
1. **Callback-argument parameter registration:** a function or arrow passed as a call argument gets no parameter Defs. This affects top-level, nested-statement, assignment and generic callbacks.
2. **Member-only formal root Def:** relaxing the field-isolation guard soundly.
3. **Unparenthesised single arrow parameter**: `x => …`.
4. **Rest parameter Def.**

They also want, in parallel:
- **Lane 2:** a small independent slice for **loading `dist/`/`build/` files** (workstream 4 admission, 7 SecBench conversions).
- **Lane 3:** orchestrating a **measurement** lane for things still unmeasured:
  - Node runtime and async beyond callbacks (workstream 6; SecBench seeds handlers directly, bypassing ingress);
  - React props and hooks (workstream 5);
  - server-side destructuring (SecBench's old npm corpus has almost none).

The controller is a single Claude Code session (Opus 5.5). It dispatches gpt-6.1-sol via `codex exec` for planning and repairs, Sonnet for implementation, and Opus plus gpt-6.1-sol reviewers (cap 2). It commits, runs the private-corpus F measurements, and merges on green. Its context is long-running and relies on compaction and handoff files.

## Deliver (write to `/Users/wesleyjinks/prism-evidence/fable-eval/EVALUATION.md`)
1. **Verdict:** can and should this one controller session reliably orchestrate all three lanes concurrently? Assess:
   - context and compaction risk;
   - shared-clone and disk contention;
   - cache and branch conflicts: lanes 1 and 2 both touch JS/TS CPG and cache epochs, and may both bump the CPG cache version;
   - reviewer and model-lane load;
   - the private-corpus F bottleneck (controller-only);
   - the owner's attention for escalations;
   - failure modes seen in this project's history.

   Give a concrete recommendation. Examples: all three in one session; lane 1 plus lane 3 here and lane 2 later; sequential.
2. **If not all three here:** a written **process for the owner to hand lanes off to separate sessions**:
   - how sessions coordinate (branch and cache-epoch ownership, merge ordering, the F-run custody rule, the handoff template at `/Users/wesleyjinks/.claude/handoff-template.md` if present);
   - **ready-to-paste prompts** for each session;
   - **recommended models** per role, and why.
3. **For the JS/TS parameter-Def gaps lane:**
   - **HLD:** goals and non-goals, the soundness contract (Exact is a static-binding grade), the components touched, data flow, and how each of the four gaps fits the CPG/DFG model.
   - **LLD:** per gap, the specific functions and modules to change, the data structures, the cache-epoch bump, and edge cases:
     - shadowing;
     - default and optional parameters;
     - destructured callbacks;
     - `this` and `arguments`;
     - async and generator functions;
     - JSX callbacks (`onClick={e => …}`);
     - `.map(x => …)` and `useEffect`.
   - **Design guidance:** how to keep each change JS/TS-only, with byte-identical controls for the other languages.
   - **The "old row vs new row correctness" method:** how to decide, for every changed or lost DFG or call row, whether the old row or the new row is correct. Use the native TS checker for symbol identity as the oracle, plus CFG reachability, SecBench demonstrated traces, the Tier-A tsserver call hierarchy, and a blinded adjudication sample where no oracle decides.
   - Measure-first steps, the review gates, and slice sizing. Should it be one PR or four?
   - **Risks:** for example, the member-only guard exists for field isolation; relaxing it may add false flows.

Keep it actionable: at most about 3,500 words plus tables and prompts. Read-only everywhere else, with no git writes.
