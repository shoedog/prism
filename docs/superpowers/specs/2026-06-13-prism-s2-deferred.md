# S2 — Deferred Work (execution-discovered)

Follow-ups surfaced during S2 subagent execution. None are correctness/identity
regressions — byte fields are additive (never in any key/Ord/Eq/Hash), so these are
span-*quality* or coverage refinements. Priority L unless noted.

## Task 4 — span extraction

1. **Alias-resolved destructuring Def spans — LARGELY RESOLVED** (reconciled by the
   second-pass claude review). The original concern (resolved-alias Def `device.name` from
   `const { name } = device` gets a zero-width line anchor) is **contradicted by a passing
   test**: `alias_resolved_def_keeps_raw_occurrence_span` asserts the Def spans the real
   `name` token — codex's xhigh Task-4 impl threads the raw occurrence span so it wins the
   byte-excluded dedup, per the §3 alias raw/resolved rule. Any *other* synthesized-alias
   path with no clean node still falls to the §3 best-effort line anchor (additive byte —
   data flow / identity unaffected). Kept here only so Plan B's re-plan doesn't budget for
   an already-solved case.

2. **Multiline-signature same-line body references.** (Priority L.) `has_bare_references`
   excludes references on each parameter's declaration line; when a signature closes and
   the body starts on the same physical line (`def f(\n    x): return x`), a legitimate
   same-line body use can be dropped from the line-anchored scan.
   *Why acceptable now:* real-span rvalue extraction still captures the use; only the
   line-anchored *supplement* over-excludes, and same-line-signature-and-body is rare.
   *Fix sketch:* compute the parameter byte range and exclude only that range (not the
   whole line), preserving same-line body rvalue spans.

## Task 4 — coverage bookkeeping

3. **New `lang_python` test target.** ✅ RESOLVED in Task 10 — verified **no change
   needed.** Task 4 added `tests/lang/python/{main.rs,span_test.rs}` and a
   `[[test]] name = "lang_python"` target (Python previously had no per-language target).
   The `span_test.rs` files are span-*extraction* tests, not algorithm tests, so they do
   not belong in the algo×language matrix; Python's algorithm coverage already comes from
   `tests/algo/*` (via `make_python_test`). `coverage_test::*` (all 4) and
   `umbrella_completeness_test` pass as-is (the new files are registered in their lang
   `main.rs`), so the matrix does not under-report.

## Full-branch review (codex xhigh) — deferred (out of S2 spec scope)

The S2 full-branch review (`docs/archive/review-artifacts/prism-query-layer/s2-full-branch-review-2026-06-13.md`)
found 2 BLOCKERs + 5 MAJOR/MINOR. **Three were in-scope and FIXED** (BLOCKER 2 same-line
`dfg_forward_reachable` cross-function leak; MAJOR `nodes_at` enclosing-function
line/byte coherence; MAJOR synthetic-`CallSite` source-span). These four are deferred
because the spec deliberately kept the name-based query/resolution APIs (§5) — they are
**pre-existing imprecisions S2 enables fixing, not S2 regressions**, each additively
fixable (no costly refactor):

4. **Exact-`FunctionId` algorithm traversal** (reviewer-labeled BLOCKER). Priority **M**.
   `callers_of`/`callees_of` stayed name-based (spec §5), so `vertical_slice` /
   `threed_slice` / `barrier_slice` traverse by name and union/pick overloads — imprecise
   for same-name functions. *Pre-S2 was last-writer-wins (also imprecise); S2's node
   identity now ENABLES exact traversal but doesn't deliver it.* *Fix:* add
   `callers_of_node(NodeIndex)` / `callees_of_node` exact APIs and migrate those algorithms
   (`function_at` → node → exact traversal). The natural next increment after S2.

5. **CallSite byte in nav `Reason::Calls`/`CalledBy`.** Priority **L**. Spec §5 said the
   call span "may additively" surface on call evidence; it doesn't yet, so two same-line
   duplicate calls serialize indistinguishably in nav evidence (the CPG-level de-collapse
   from Task 8 IS in place; only the nav projection is missing). *Fix:* add a call-site
   `Location`/byte to those `Reason`s + include in ordering/dedup.

6. **Level-3 parameter-passed function-pointer resolution is name-only.** Priority **L**.
   `call_graph.rs` resolves the caller via `find_function_by_name(&caller_id.name)` /
   `callers.get(&caller_id.name)`, mixing same-name functions with different callback
   params. Indirect-call resolution (S3 / Phase-IP territory), pre-existing. *Fix:* resolve
   the caller by `(file, name, start_line)`.

7. **`CpgNode` equality excludes byte → `assert_eq!`/snapshots blind to span corruption.**
   Priority **L**. Deliberate (byte is additive identity), mitigated by `node_byte_dump` +
   the per-language span tests. *Optional:* add a byte-sensitive identity helper for tests.

8. **Span-bearing extractors duplicate the line-only traversal logic** (two sources of
   truth; future grammar support could diverge). Priority **L**. The sibling-API trade-off
   (chosen to avoid breaking line-only callers). *Fix:* make the span-bearing records the
   source of truth and project the line-only APIs from them.

## Second-pass review (codex xhigh focus=data-flow/span + claude opus focus=seams/cache/tests) — 2026-06-14

Record: `docs/archive/review-artifacts/prism-query-layer/s2-second-pass-review-2026-06-14.md`. claude verdict
**ship** (0 new blockers); codex verdict **needs changes** (its 2 MAJORs are items 5 + 9
below — both already-deferred, additive, NOT S2 regressions). One in-scope test (the
different-name-same-line regression guard) was **added** this pass. New/raised deferrals:

9. **Byte-aware interprocedural argument binding** (codex MAJOR — raises spec §9 item).
   Priority **M** (top precision follow-up — flagged by BOTH the first review and codex's
   second pass). Step-5b binds args via `call_argument_texts(site.line, callee_name)`, which
   stops at the first same-line call; for `callee(a); callee(b)` on one line both bind `a`.
   **Verified NOT an S2 regression:** `b→param` is missing pre-S2 too (the line-only
   `cmp_key` collapsed to one `CallSite`); Task-8 de-collapse only adds a *harmless
   duplicate* `a→param` edge — it does not create a wrong/false-positive edge. The
   `CallSite` byte now makes the fix additive: select the call expr by `site.start_byte`
   and use *that* call's args. Natural pairing with the exact-`FunctionId` next increment.
10. **Augmented assignment on a NESTED member misses the base fallback** (codex MINOR).
    Priority **L**. `o.field += 1` emits a base `Use(o)`, but `o.config.timeout += 1` does
    not (the base fallback only fires when the immediate receiver is an identifier). *Fix:*
    peel nested field/index receivers to the leftmost identifier before the base fallback.
    Span-precision FN for the base var in nested augmented targets (rare; additive byte).
11. **`CallSite` `Ord`/`Eq` asymmetry on `receiver_recovery`** (claude MINOR; pre-existing,
    not worsened by S2). Priority **L**. `cmp_key` (the `BTreeSet` dedup key) excludes
    `receiver_recovery` while derived `Eq` includes it → two sites differing only there are
    `Ord::Equal` but `Eq`-unequal. Not reachable as a bug (all dedup goes through the
    `BTreeSet`/`cmp_key`). *Fix:* add `receiver_recovery` (it's `Copy`) to `cmp_key`, or
    hand-write `PartialEq` to mirror `cmp_key`. (Touching `cmp_key` affects de-collapse
    dedup — verify against the de-collapse tests if done.)
12. **Untested precision boundary: line-collapsed witness anchor.** Priority **L** (claude
    MINOR — test coverage). The `node_of` best-effort branch (line-collapsed use → wire
    byte `start==end`) has no assertion. *Fix:* add a witness assertion on a line-collapsed
    use confirming `start_byte == end_byte`, pinning the §6 precision boundary.
