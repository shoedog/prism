# S2 — Deferred Work (execution-discovered)

Follow-ups surfaced during S2 subagent execution. None are correctness/identity
regressions — byte fields are additive (never in any key/Ord/Eq/Hash), so these are
span-*quality* or coverage refinements. Priority L unless noted.

## Task 4 — span extraction

1. **Alias-resolved destructuring Defs carry a zero-width line anchor, not the raw
   occurrence span.** (Priority L.) For `const { name } = device` the resolved-alias Def
   (`device.name`) is registered with `line_start_byte` for both ends.
   *Why acceptable now:* this is the spec §3 "synthesized / alias-resolved (no clean node)
   → best-effort line anchor" case; byte is additive, so data flow / de-conflation /
   identity are unaffected. Only the Task-6 wire byte for these specific occurrences is
   coarse (zero-width) rather than the raw extent.
   *Fix sketch:* thread the raw destructuring occurrence's node span into the alias-
   resolved `VarLocation` before the line-anchored fallback registers it (so it wins the
   byte-excluded dedup), per the §3 alias raw/resolved rule. Reviewer flagged in Task 4.

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

The S2 full-branch review (`docs/prism-query-layer/s2-full-branch-review-2026-06-13.md`)
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
