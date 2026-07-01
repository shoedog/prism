# S2 Full-Branch Review (codex xhigh) — 2026-06-13

**Reviewer:** codex (gpt-5.5, **xhigh**) via a2a-bridge `run-workflow code-review-codex`
(`examples/a2a-bridge.slicing-review-codex.toml`): correctness + architecture lenses, both
with the prism MCP (warmed on the S2 HEAD), → synth. Input: the full S2 branch diff
`origin/main..s2-node-identity` (6977 lines, 49 files). Raw: `/tmp/s2-review-codex.md`.

**Verdict:** *"ship after fixing the 2 BLOCKERs and the navigation/call-site MAJORs."*

## Disposition

**FIXED (in-scope S2 gaps) — full suite green after (2014 tests, 0 fail):**

| # | Finding | Fix |
|---|---|---|
| BLOCKER 2 | `dfg_forward_reachable` propagates same-line `Use→Def` by `(file,line)` only → cross-function taint leak on minified/same-line code (query.rs) | Filter the same-line `Def` by `(function, function_start_line)` matching the `Use` (mirrors `trace.rs::taint_neighbors`). Regression `dfg_forward_reachable_does_not_leak_across_same_line_functions`. |
| MAJOR | `nodes_at` enclosing-function `Location` mixed the queried `line` with the full-function byte range (incoherent) (navigation/queries.rs:174-202) | `Location` now mirrors the function's full span (`start_line`/`end_line` + bytes), coherent with the `SymbolRef::Function` it carries. |
| MAJOR | Synthesized indirect `CallSite`s zeroed their bytes → same-line indirect dups (`fp(); fp();`) still collapsed (call_graph.rs ×5 ctors) | All 5 synthetic ctors carry the source `site.start_byte/end_byte` (the source call expression's span). |

**DEFERRED (out of S2 spec scope — name-based query/resolution APIs were kept by design,
§5; pre-existing imprecisions S2 *enables* fixing, not regressions; each additively
fixable) → `docs/superpowers/specs/2026-06-13-prism-s2-deferred.md` items 4-8:**

| # | Finding | Why deferred |
|---|---|---|
| BLOCKER 1 | Exact-`FunctionId` algorithm traversal — `vertical`/`threed`/`barrier` slices traverse by name, imprecise for same-name fns | Spec §5 kept name-based `callers_of`/`callees_of`; pre-S2 was last-writer-wins (also imprecise). S2's node identity now enables `callers_of_node`; delivering it + migrating the slices is the natural next increment (Priority M). |
| MAJOR | CallSite byte not in nav `Reason::Calls`/`CalledBy` | Spec §5 said the span "may additively" surface; CPG-level de-collapse (Task 8) IS in place, only the nav projection is missing (Priority L). |
| MAJOR | Level-3 param-fptr resolution name-only (call_graph.rs:584-636) | Indirect-call resolution (S3 / Phase-IP territory), pre-existing (Priority L). |
| MINOR | `CpgNode` Eq excludes byte → `assert_eq!` blind to span corruption | Deliberate (byte additive); mitigated by `node_byte_dump` + span tests (Priority L). |
| MINOR | Span extractors duplicate line-only traversal (two sources of truth) | The sibling-API trade-off (chosen to not break line-only callers); project line-only from span-bearing later (Priority L). |

**Disagreement note (from synth):** the architecture lens held the exact-traversal gap
(BLOCKER 1) as a BLOCKER. Triage (me): it is a real precision *capability* S2 enables but
the spec scoped out (§5 kept by-name queries), and it is not an S2 regression (pre-S2 was
equally/more imprecise). Surfaced prominently in the PR body for the owner's merge
decision rather than expanding S2's scope.
