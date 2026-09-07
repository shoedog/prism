# Nested callback outer-parameter binding observations

> Publication reconciliation (2026-09-06): alias implementation `2bfe7092` is published in [PR261](https://github.com/shoedog/prism/pull/261); nested implementation `16de216` is published in [PR262](https://github.com/shoedog/prism/pull/262), based on PR261. Merge261 first. The owner's new instruction authorizes agent commit/push/PR. Controller-pending and uncommitted-predecessor statements below are historical for these two slices. Bounded Props/property-to-class provenance is approved next; runtime authority remains excluded.

Owner approved "proceed to next" after the alias-provenance slice. Isolated
feat/callable-nested-bindings worktree at8e7744e2 overlays the verified predecessor
source-final.tgz. Predecessor is uncommitted, not merged. No agents; cap3 SELF-PASS,
NOT INDEPENDENT. Current AGENTS leaves commits/publication to the controller.

## Frozen scope and architecture

Extend only scripts/callable-observations. Schema /1→/2, producer0.2→0.3; reject
older packets pre-I/O, add new module bytes to producer hash. No runtime resolver,
CPG/navigation/cache changes, default dependencies, installs or application writes.
All authority flags remain false. Direct-body calls retain their existing meaning.

For each existing direct annotated implementation, add nested.calls and
nested.barriers. Traverse nested arrows/function expressions, retaining their
outer-to-inner function anchors. Class/function-declaration/method scopes are
explicit fences. Bound nesting at8 and nested calls at128 per observation;
caller limits may only decrease. Limits produce visible barriers, not silently
complete empty censuses. Existing file/byte/heap/time caps bound inventory work.

Each nested member call retains its call/receiver/type/method-declaration anchors
and a separate binding observation. A supported receiver is an identifier or
non-optional named property chain rooted at an identifier. Its use must resolve
to one unique binding in the OUTER implementation's first parameter: a simple
identifier or flat, non-rest, non-defaulted object binding element (renaming is
allowed). Optional/array/nested/computed/defaulted parameter forms stay unproven.
Other declarations, unresolved roots, duplicate same-scope declarations and
direct syntactic writes anywhere in the outer body produce explicit reasons.
Explicit-any remains lexical evidence only, never class authority.

Writes include ordinary/compound/logical assignments, updates, deletes and
for-in/of targets, including property/element paths and assignment destructuring.
Shorthand assignment value symbols must not be confused with property symbols.
Writes in nested closures count; writes to distinct shadow bindings do not.
Whole-body ordering is deliberately conservative. This is NOT an effect/alias
analysis: mutation through aliases, opaque calls and external effects is not
proved absent. A linked lexical binding does not establish runtime value stability.

Closed schema validates every nested anchor and status invariant pre-I/O;
recomputation rejects well-shaped forged binding identity, deleted writes/barriers,
and stale A→B→A inputs. Nested binding status and Program/type-provenance status
are independent; no closure flag is upgraded by a lexical link.

## Plan and acceptance

1. Capture predecessor RED for missing captures/write barriers and old-schema rejection.
2. Implement bounded traversal and binding/write inventories; test positive captures,
   renames/property chains, shadow/duplicate/write forms, scope fences and limits.
3. Adversarial review, strict/tampered/stale packet tests, source digest; retain38
   predecessor tests plus40 compiler and4 helper cases. Three self-review rounds.
4. Measure the four pinned public component receiver spans against actual source,
   distinguishing an isolated lexical check from a closed installed Program.
   Private replay, if runnable, stays read-only and locally segregated.
5. Full default/MCP Rust suites, fmt/diff, source/evidence checkpoints and controller
   handoff. No Tier-A trigger without runtime changes; inherited quick remains
   baseline-invalid. No full multicorpus/rebaseline; stop below3GiB disk.

Instantiated Props/class identity and any runtime consumer remain future slices.
