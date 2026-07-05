# Pipeline lessons — multi-agent P-item execution (rounds 1–8, 2026-07-02 → 2026-07-04; plan complete)

Durable lessons from executing `docs/analysis/prism-llm-and-accuracy-plan.md` items P1–P14
(PRs #149–#165, the complete plan) via the owner-approved pipeline: codex gpt-5.5 xhigh spec review →
Sonnet implementers in isolated worktrees (TDD) → Opus task review ∥ codex impl review →
fix waves → codex re-review of fix deltas → owner merge. Update this file at each round
close-out; the per-round detail lives in the plan doc's status blocks and
`docs/superpowers/specs/*-spec.md` SHIPPED headers.

## Process lessons

1. **Re-review the fix delta, always.** Through round 8: **ten of eleven** codex
   re-reviews of fix waves caught a real defect in the fix itself (P3, P5, P7, P6bc,
   P8, P10, P9×2, P11, P13, P14 — where the wave-1 re-review found the fix's
   bypass check collapsing a 3-valued verdict space; see lesson 15). Counting note:
   whole-branch/impl reviews finding pre-fix defects are a SEPARATE (also excellent)
   layer — don't launder them into this record. The defects live at the *fix×interaction* layer — the fix is
   locally correct but breaks caching, incremental rebuilds, a parallel copy, or an
   adjacent behavior. The ONE clean re-review (P12) is itself instructive: its fix
   DERIVED the new sizing function from the real serializer instead of
   parallel-implementing it — the drift class was eliminated by construction, not by
   care (see doctrine 6). Exception that survives review: purely mechanical prescribed
   fixes may take controller hunk-verification instead of a full codex round.

2. **Restructures regress orthogonally.** Round 5's sharpest instance: a P9 fix wave
   restructured candidate extraction to fix a shadow-detection MAJOR and silently
   regressed *caller attribution* — behavior orthogonal to the finding being fixed
   (anonymous wrappers fell back to `<module>`). Only the re-re-review caught it.
   **Adjustment:** when a fix wave restructures (moves/reshapes code rather than
   patching in place), the re-review prompt must explicitly ask "what behavior did the
   restructure carry, and does it still hold?" — not just "is the finding fixed?"

3. **Ground the plan's cited mechanism before writing the brief.** The plan cited
   `FlowPath.cleansed_for` reuse for P10; grounding showed `taint_reaches` never builds
   FlowPaths and the naive design would have minted false negatives. A cheap
   grounding-Explore pass before the brief caught it; the controller redesign then held
   through five codex passes. Plans rot; briefs must be grounded against HEAD.

4. **Spec-review the brief before implementation.** Every round's batched codex spec
   review found at least one BLOCKER/MAJOR-class flaw in the brief (P10's node-presence
   proof, P9's non-navigable pseudo-caller, P5's CallSite-pipeline hole, P8's wildcard
   poison). Two hours of review is cheaper than a wrong implementation.

5. **Two independent reviewers see different things.** Opus task reviews verify spec
   compliance and catch report/diff mismatches; codex impl reviews find semantic
   soundness holes (every round: real defects all claude-family gates missed). Neither
   subsumes the other. Where they conflict (P9's `app.use` gap: opus "acceptable",
   codex "fix"), the controller adjudicates on the merits — precedence goes to the
   argument, not the model.

6. **Line anchors rot; symbols don't.** Briefs written against a moving main should
   name symbols to grep (`property_accesses` → every plumbing site) rather than
   line numbers, or mark anchors as hints. Post-#158 anchors were stale within one
   round.

7. **Never rebase under a running agent; never re-bump caches for pre-merge fixes.**
   Rebase after the fix wave completes. A cache version bumped once per PR covers all
   its fix waves — the shipped transition is one step.

8. **Ledger + handoff before compaction.** The git-ignored
   `.superpowers/sdd/progress.md` ledger records every verdict/dispatch; the handoff
   doc carries the cold-start map. Both saved this execution across multiple
   compactions with zero re-dispatched work.

9. **A metric moving the "wrong" way after a precision fix demands mechanism
   verification, not celebration or panic.** P11's closure fence made etcd's
   `return_typed` count RISE 903→939. The implementer's claimed mechanism (the
   pre-fix walk's sibling-closure bindings inflated ambiguity counts, causing bails
   BEFORE recovery ran — so the fence recovered false negatives) was verified by the
   re-review against the actual pre-fix code and then pinned by a
   fence-neutralization test. Never accept a surprising delta on narrative alone;
   never reject it on direction alone.

10. **One cache transition per PR.** A version bumped mid-branch (37→38, then a fix
    wave's 38→39) describes an artifact no external build ever produced. Consolidate
    to a single shipped step before merge; merge the fix-wave notes into the one doc
    entry. Enforced twice now (P9 round 5, P11 round 6 — the second time the wave
    bumped unprompted and the controller reverted it).

11. **When two reviewers conflict, adjudicate substance and mechanism separately.**
    P12's sizing conflict: codex was right on substance (the item-retention goal
    demanded mode-aware sizing) AND opus was right on mechanism (ambient env reads in
    sizing would be impure and race-prone) — the resolution took codex's WHAT with
    opus's HOW (thread the resolved mode as a parameter). Precedence goes to the
    argument, not the model or the severity tag.

12. **The primary tree IS the tier-a SUT.** Round 7's full-corpus run take-1 died at
    corpus 3 because controller edits dirtied the main tree mid-run (prism+ruff
    reports survived, written pre-edit; 9 corpora relaunched). While a run is live:
    no main-tree edits, no rebuilds, and hold `git pull` until the run finishes (a
    pull SHA-mismatches the recorded SUT). Corollary: rebuild the release binary to
    match HEAD before any future run.

13. **The codex-implementer variant works, with two structural changes** (round 7,
    owner-directed to save Fable usage). Implementation = codex gpt-5.5 HIGH via
    bridge (`sandbox_mode="workspace-write"`, session-cwd = the worktree); the Opus
    task review is replaced by the CONTROLLER personally reading brief-vs-diff (cheap,
    and it caught two defects codex's own report missed); the adversarial gate must be
    a FRESH codex session with a different role prompt — same-family diversity is
    partially restored by fresh-session + role separation, and it found three more
    real classes. Wrinkle: the bridge sandbox cannot write `.git/worktrees/<wt>`
    metadata (it lives under the main repo's `.git`), so implementers report
    "commit failed" — instruct them to leave the tree uncommitted and have the
    controller commit with the dual trailer after verification.

14. **Controller reviews should reproduce, not just read.** Round 7's two controller
    findings (C1 sentinel SAT enumeration, C2 header blank-line break) were confirmed
    by building the worktree binary and running 3-file scratch fixtures BEFORE the
    adversarial review was briefed — which let the review task-spec pass them as
    named, already-proven risks and spend its effort past them. Every fix-wave
    verdict was then re-verified the same way (all 8 defect shapes reproduced fixed
    end-to-end). A reproduced defect also can't be argued away in the fix wave.

15. **A reachability boolean is not a verdict.** P14's bypass re-walk tested
    `frontier.contains(sink)` where the verdict space is three-valued
    (Reached / BoundaryExited / NotReached) — the middle value silently collapsed
    into the wrong verdict (`Sanitized` outranked by the truth). When a fix
    re-derives an outcome the system already classifies, route it through the SAME
    classifier the primary path uses (`reachability_for_node_from_ordered`), never a
    projection of it. The projection compiles; the classifier is the contract.

16. **Prescribe the WHY with the HOW — implementers can catch prescription bugs.**
    P14 fix wave 1: the controller prescribed chain-scoped sanitizer-hop exclusion;
    the Sonnet implementer hand-traced the both-sanitized case, showed chain-scoping
    mis-verdicts it, and shipped tree-scoped exclusion instead — correct, and
    controller-verified sound in both failure directions. The brief carried the
    invariant ("Sanitized only if removing the proven cuts disconnects the sink"),
    which is what let the implementer test the prescription against it. A
    HOW-only prescription would have shipped the controller's bug.

## Engineering doctrines (bind reviewers and implementers)

1. **Consumer-visibility doctrine** (rounds 2–5, codex-ruled): uncertainty tiers need
   consumer-visibility tiers. Name-coincidence candidates = nav-only. Registration-
   grounded candidates reach non-nav consumers only at exactly one target.
   **Entrypoint facts** (P9 framework registrations) are nav-only regardless of target
   count — a registration is not an invocation; feeding it to taint/slice would assert
   dataflow that doesn't exist at the registration line. Nothing below Exact feeds an
   asserted finding.

2. **Prove structure by byte-span reconstruction, never by graph edge labels** (P10).
   Derived graphs over-approximate (first-parent BFS trees, same-line
   `AssignmentPropagation` from a use to *every* same-line def). Any claim of the form
   "this edge is a real X" must be re-proven from the AST via byte spans
   (`descendant_for_byte_range` + parent walk), not trusted from the edge kind.

3. **Index-routed scope checks are blind to anonymous scopes** (P9). Anything keyed by
   `FunctionId` (locals indices, enclosing-function lookups) structurally cannot see
   anonymous arrows/IIFEs. Shadow/binding decisions must walk the call node's actual
   AST ancestors. Conversely, *attribution* decisions (who is the caller) may want to
   skip anonymous nodes — the two walks answer different questions; don't merge them.

4. **Name inference ≠ referenceability** (P9). The function-name inference layer names
   object-property arrows, member assignments (`exports.h = ...`), and class members —
   none of which a bare identifier can reference. Any bare-identifier resolution must
   filter by definition-site AST shape, not by presence in the functions index.

5. **Repo-wide facts break per-file incremental caching** (P8). If extraction for file
   A depends on facts from file B (macro shadow sets, export maps), persist the fact on
   the graph and compare on incremental rebuild — mismatch → full rebuild. Cheap and
   sound.

6. **Second copies of resolution logic drift** (P4): `scoped_caller_site_match_count`
   independently re-derives R4c name-correlation — any R4c change must update it too.
   Grep for parallel implementations before changing a resolution rung.

7. **The safe failure direction is a design input.** Sanitizer verdicts: a missed
   `Sanitized` stays `Reached` (safe); a false `Sanitized` hides a vulnerability
   (unsafe) — so every ambiguity resolves toward `Reached` (paired-check exclusion,
   positional-only default, direct-call-RHS-only). Framework edges: under-recording is
   safe, false edges are not. State the direction in the brief; reviewers verify each
   ambiguity resolves toward it.

8. **Measurement noise is real** (P4): quick-M2 exact-tier deltas can be pure
   line-shift resampling noise when a branch adds lines to instrumented files.
   Diagnose with repeat-run + shared-probe byte-identity controls before re-baselining.
   Related: rescore, never re-run, when arm outputs are saved.

9. **Port the reference implementation; don't approximate its rules** (P13). Two
   rounds of fixes approximated go/build's file-header semantics from prose and lost
   each time (blank-line region, `/**///go:build`, too-close `+build`) — every miss a
   toolchain-verified WRONG. The fix that survived re-review was a faithful port of
   `parseFileHeader` (ended-flag, header-ends-at-most-recent-blank, comment loop),
   checked against the toolchain's own test cases (Comment4, TooCloseNo). When a
   reference implementation exists and is readable, port its control flow and cite it.

10. **Fail-open failures are silent; audit the win-side too** (P13). The header bug
    failed SAFE (no false edges) and so passed every test — but it made the entire
    build-SAT rung INERT on all four measured corpora. The tell was a shipped feature
    counter at zero everywhere (`go_build_partition_exact=0`). A new mechanism whose
    success counter never fires is presumed broken until a positive corpus case is
    reproduced; and a predicate needs BOTH failure poles exercised — fail-closed mints
    false Exacts, fail-open silently forfeits the win.

## Follow-up queue (durable, self-tracking where possible)

- P14 deferrals: return-flow taint (no callee-return→caller-LHS edges exist);
  multi-line-call args have no Step-5b edge (pinned by
  `test_multi_line_call_shape_is_currently_not_descended`); recursion descent;
  first-enqueue depth-lock relaxation (pinned deterministic);
  `src/reasoning/taint_reaches.rs` at 631 lines — module split.

- ~~Nested-test-module `use super::*` callers gap~~ — **DONE (#166)**: anchor-only globs
  (`super::*`/`crate::*`/`self::*`) resolve via `ResolutionPolicy::glob_anchor_expands`;
  832 prism-self globs newly resolved. The fixture flipped `known_fail`→`pass`.
- Pointer-embedded Go fields (`*Listener`) silently dropped by `extract_one_field`
  (tree-sitter emits a bare `*` token; `type_str == "*"` → strip → empty → drop) —
  pre-existing, affects the shipped embedding feature AND P11's S2/S4; fails safe.
- GoOwnerIdentity clause/build-partition blindness (P13 [M1], counted-not-fixed):
  `go_field_types`/`struct_embeds`/embedded-interface lanes are keyed `(package_dir,
  name)` and can cross `foo`/`foo_test` and build partitions — field_typed /
  interface-dispatch Exact recovery can still cross those lines. Measured by
  `go_owner_identity_profile_conflict` (etcd 1, prometheus 5). Re-key = P11-lane
  blast radius; schedule deliberately.
- RESOLVED round 7: MCP default flips (#163, live-verified); tier-a M2 re-baseline
  (2026-07-03 run records + 627-verdict adjudication fold + baseline.md 2026-07-03
  section); P15 re-export tail (measured NO-GO both halves — third under-delivery;
  do not re-queue).
- Advisory/CWE sanitizer recognizers still cross-match languages (P10 gated the verdict
  path only; deliberate).
- `prune_graph_to_reasoning` keeps forward-hanging leaves only for `SanitizedBy`; any
  future forward-hanging edge kind needs the same treatment (one-class fix in place).
- `--review-no-diagrams` (P1 residual: diagram payloads dominate compacted review output).
- Adoption-eval re-run after SKILL.md changes (owner-gated, live API cost).
