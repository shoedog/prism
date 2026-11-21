# Lane 8: one taint answer from both engines — design

Status: architecture final; the crossing rules of section 5.2 are tentative until the parity census and the
matrix (PLAN stages M, T, E). Round 1 of the design review is folded (2026-11-21; its probes, cited below as
"review probe", are in `~/prism-evidence/lanes-2026-11-20/reviews/astra-r1/lane8/probes/`); round 2 is folded
the same day (its probes, cited as "r2 probe", are in `~/prism-evidence/lanes-2026-11-20/reviews/fable-r2/lane8/probes/`). Starts after
draft PR #841 merges (it edits the same three files and adds the crossing-policy type this lane uses). The
plan files a tracking issue. Research record: `~/prism-evidence/lanes-2026-11-20/research/lane8-taint-parity.md`.

## 1. Intent

Prism answers "does this value reach that sink" in two ways, and they disagree. Review taint
(`--algorithm taint`) follows every data-flow edge and matches a catalog of sinks. The reasoning layer
(`prism nav taint-reaches`, the MCP tool `taint_reaches`) walks with the trace engine, which crosses a
function boundary only under strict conditions, and has no catalog: the caller must already know the sink's
line. An agent using the MCP tool can be told "boundary" for a flow that review reports, and cannot ask
"which dangerous sinks does this reach" at all.

The lane does three things: makes every difference between the engines either disappear or carry a named,
tested reason; removes the differences that are defects of the trace engine; and gives the reasoning layer
a sink catalog.

## 2. What exists today (verified on `origin/main` at 1179055f; PR #841 at 493ab5ee)

| Fact | Reference |
|---|---|
| Default review taint is a breadth-first walk over all data-flow edges: no depth limit, no confidence gate, no check that a cross-function edge belongs to a call. Labels only grade the finding. | `src/cpg/cfg_queries.rs:205`, `src/cpg/query.rs:707` |
| The trace engine is used by review only with `--taint-return-flow`, and always by the reasoning layer and the MCP tool. It crosses into a callee only when the edge ends at a definition, exactly one call site *spelled like the callee* in the caller matches the source occurrence (same line, or the occurrence's bytes inside the site; a site of another name on the line is not counted), that site's resolution includes an Exact edge to the target (one of several Exact targets suffices), and the depth is under 2 (shared with return ascent). Ascent is allowed when *any* call in the destination function resolves singleton-Exact to the callee. The visited set is first-come: a node is never re-expanded. A refused hop is recorded as a boundary and not followed; other losses (the depth-0 CFG filter, a suppressed shortcut) record nothing. | `src/cpg/trace.rs:712`, `:806`, `:822`, `:538`, `:25`; `src/reasoning/taint_reaches.rs:54` |
| Return-flow edges are built only for a call that resolves singleton-Exact; a name-only call gets none (`return_flow_skipped_nameonly`). | `src/cpg/step5c.rs`, `src/cpg/step5c_facts.rs` |
| An object-literal leaf binds to every formal read whose static chain is comparable with its key path; a computed read (`p['k']`, `p[k]`) has the empty chain, so it is fed by every leaf. | `src/cpg/object_literal_args.rs` (`read_chain`, `comparable`) |
| Probe matrix, 26 fixtures in three languages: default review and `taint-reaches` agree on 20 and disagree on 6. Five are descent refusals: an object-literal argument bound to a member read (the edge ends at a use; two fixtures), two call sites of the callee on one line, a name-only callee, a call chain three deep. One is sink granularity (a use that shares the sink's line). Review in trace mode agrees with `taint-reaches` on 25 of 26. Both engines miss a captured variable read in a nested function. | research section 6 |
| Wrong results today (review probes on a0f2c427): (a) the trace engine returns into an unrelated caller: `entry` passes `t` to `id`, `separate` reads `safe = id('fixed')`, and `safe` is Reached; (b) `run(...[], t)` into `run(a, b)` reaches `b` (the producer binds positionally past a spread; lane 9 slice A); (c) exploration order decides depth: a formal reached first at depth 2 blocks a later depth-1 route, so an in-budget path is BoundaryExited; (d) a recursive `f({a:x}, false)` reaches a later read `p.a` as intra-function DataFlow at depth 0; (e) a non-simple return seeded inside the callee is dropped silently by the depth-0 CFG filter; (f) `pick({bad:t, clean:'fixed'})` has an Exact edge into the read `p['clean']`; (g) review reports a flat sink with a fixed operand on a line reached only at a parameter Def, and a certified constant-return call's left side through the same-line shortcut. | probes `return_one`, `spread_direct`, `diamond`, `self_literal_after`, `return_expr`, `literal_index`, `flat_def_line`, `constant_return_flat` |
| In trace mode, findings that are `exact` in default mode become `unlabeled`, because descent and assignment hops mark the evidence as crossing unlabelled edges. An object-literal binding edge carries the worse of Exact and the callee read's own label. | `src/finding_confidence.rs:348-379`; `src/cpg/object_literal_args.rs:523` |
| The trace engine has no notion of a sanitizer; the reasoning layer adds one on the witness chain and accepts a sanitizer of any category, by documented design (its line query names no sink category). Review cuts per category on the flow path and, as an advisory, by presence in the source function's body. | `src/reasoning/sanitizer_walk.rs`; `src/sanitizers/matching.rs` (`sanitizer_call_site`); `crates/prism-review/src/algorithms/taint/cleansing.rs` |
| PR #841 adds an optional crossing policy threaded through the walk functions. It limits only edges that enter a context file (a file added by import depth) to a confidence ceiling of Exact or name-only; `--taint-context-depth` is that import depth. The descent rule still requires Exact whatever the policy says, the depth cap and review's choice of engine are unchanged, and the reasoning layer passes no policy and has no context files. With its defaults, every difference above remains. | PR #841 (`src/cpg/taint_policy.rs` at 493ab5ee) |
| The sink catalog lives in the review crate: the flat pattern list, the positional classifier with its scope resolvers and module tables (`taint_sinks/`, 23 files, 7,322 raw lines with tests), the per-language structured tables, the matchers. The matchers call into review's cleansing and source discovery (the operand check reaches framework parameters, request aliases and `seeds.rs`), so their module closure is measured, not a set of helpers: from `taint/sink_match.rs` and `taint_sinks/` it is 48 files and 17,241 raw lines plus 12 items of `taint.rs` and one function of `seeds.rs`; with the sink loop's own source facts (framework sources, wrapper sinks) about 18,300 of the 19,146 lines in 51 of 53 files. Only `taint/report.rs` names another review module (`crate::output::mermaid::safe_node_id`). Review's FlowPaths are single-source fans, and its sink predicates read every edge of the fan and take the CPG (`taint/cleansing.rs`). A static table's argument index is not always the operand review checks (`render_template_string`: index 1; review checks the tainted keyword's template variable). The core crate holds the sink pattern type, the sanitizer recognisers and nearly empty per-framework sink lists that nothing reads. | research section 5; `taint/python/sink_tables.rs:31`, `taint/python/sinks.rs:49`; closure measured by `use` and qualified paths at 9ba4249d; `src/cpg/cfg_queries.rs:281`, `taint/cleansing.rs:477` |
| `taint-reaches` answers BoundaryExited only when the sink is a boundary's target or reachable from it inside that function; a sink one or more functions further is NotReached, with the warning in another function (r2 probes `b4`, `b5`). The trace has no call-argument facts: `CallSite` holds `arg_count` and a Go-only `arg_spread`, the graph holds no `ParsedFile`, a DataFlow edge carries a label only. | `src/reasoning/shape.rs:91`; `src/call_graph.rs:97`, `src/cpg/build.rs:302` |
| The `prism` binary is built from the review crate, so `prism nav taint-reaches` can reach the catalog. The MCP server is a separate crate that depends on core only, and a layering test forbids it to name the review crate. | `tests/integration/layering_test.rs`; `crates/prism-mcp/Cargo.toml` |
| On real diffs, the cross-file sinks that issue #602's measurement added were right 10 to 23% of the time, against 13 to 17% for in-diff sinks. | issue #602, M2 |

## 3. Goals, non-goals, anti-goals

Goals:

1. A committed parity matrix at occurrence level (5.1): for each fixture, what each engine answers at each
   parity level and, where they differ, the named reason. It is the contract; any later change that moves a
   cell fails a test.
2. The trace engine crosses the hops that are as well founded as the ones it crosses today, on evidence that
   belongs to the hop (5.2): an object-literal or props binding edge into a read its key path supports; a call
   site singled out among several on one line; a callee whose spelling at the site differs from its
   definition's name. It stops returning into callers it did not enter, and its depth accounting stops
   depending on exploration order.
3. Every lost sink, in either direction (the trace engine loses what review reports, or reaches what review
   does not), and every refused hop reports one reason from one closed list (5.3), in `taint-reaches` output
   and in the census.
4. The same crossing knobs (confidence ceiling, depth) on the core walk both clients of the trace walker call,
   with today's defaults; `taint-reaches` and the MCP tool expose them, and tests drive trace-mode review with
   given knobs through the core API. Default review has no descent gate and gets none.
5. A catalog mode for the reasoning layer: from a source, the catalog sinks reached, each with its rule,
   operand, verdict and witness; first in the CLI, then in the MCP tool.

Non-goals: replacing review's walk with the trace engine (it would drop findings review reports today);
new sinks or sources; captures in nested functions; callbacks (lane 9); improving the precision of review
taint (its wrong findings found here are pinned and filed, 9); cross-file context (PR #841); building return
edges for name-only calls (a graph change, 5.4).

Anti-goals: loosening descent by removing a condition instead of replacing it with an equally strong one;
a catalog copy in the core crate that can drift from the review crate's; changing default answers of either
engine without a matrix row that says so; widening any wrong result of section 2.

## 4. Approaches considered

For the engine differences:

**A. One engine.** Make review use the trace engine. Measured consequence: five of the six disagreeing
fixtures lose their finding, and trace-mode findings lose their `exact` grade. Rejected.

**B. Make the walk identical by loosening the trace engine to follow every edge.** It would stop being a
witness-producing, Exact-gated engine, which is what the reasoning layer's verdicts mean. Rejected.

**C. Keep both, pin the differences, fix the defects** (chosen). The engines answer different questions
(review: could this reach; trace: here is a justified path). The matrix records which cells differ and why;
the defect classes are fixed in the trace engine; the knobs let either client of the walker run under the
other's policy.

For the catalog:

**A trait in core, implemented in the review crate and passed in** (chosen for the CLI). The `prism` binary
gets catalog mode with no move. How the MCP server gets an implementation is decision 2 (section 8), the
last slice, with four branches:

- **A. Move into the core crate.** The matchers call into source discovery, so nothing smaller than the
  module closure can move: nearly the whole taint module (about 18,300 of 19,146 lines, 51 of 53 files,
  section 2), into a core module not named `algorithms` (the MCP layering patterns forbid that spelling).
  Untangling: 12 items out of `taint.rs`, two functions out of `seeds.rs` and one out of `flat.rs`, source
  detection taking the graph and files instead of `&CpgContext`, restricted visibility rewritten. Core then
  holds review's sink rules; lane 7's registry, which `SinkHit` names once that lane merges, moves with them.
- **A2. A small crate below both binaries** (`crates/prism-taint`) holding the taint module whole (`report.rs`
  stays in review); `prism-review` and `prism-mcp` depend on it, core is unchanged, the layering test gains
  one allowed crate name and a rule that the new crate never names `prism_review`, and the documented layout
  becomes four crates. The same move with fewer partial moves.
- **B.** The MCP crate depends on the review crate through one adapter file and a narrow layering exception.
- **C.** CLI only; MCP pending.

## 5. Architecture — *crossing rules tentative*

### 5.1 The parity matrix

Fixtures under `tests/fixtures/taint_parity/` with a rows file. A row names occurrences, not lines: the
source occurrence (file, line, bytes), the call site and argument slot or the return it tests, and the sink
operand (file, line, bytes). Parity has three levels, asserted separately:

- **Walk:** the trace walker's reached nodes and witness for the row's source occurrence as the only root,
  under given knobs. `taint-reaches` and trace-mode review are two clients of one walk, so with equal knobs
  they must give the same walk; this is asserted only on rows whose inputs are genuinely identical (the
  source line holds one occurrence and no framework source; flag `walk_comparable`). The cell is the
  aggregate, in the existing priority (`Reachability::aggregate`), of the verdicts of every Variable node on
  the sink line whose byte span intersects the operand's span (a zero-width node at the operand's start
  included); one operand can have several nodes (`p` and `p.a` over the same bytes, r2 probe `a1`). The row
  lists the node spans it expects, so a change in node minting fails the well-formedness test, not the verdict.
- **Finding:** review's `taint_sink` finding at the sink (file, line) and its grade, for default and trace
  mode. Review's roots (every variable on the source line, framework sources), its root exclusion and its
  cleansing differ from the walk, so a difference from the walk level carries a reason.
- **Catalog:** the multiset of (file, line, rule, operand, verdict) catalog mode reports (5.6).

Each level has its own expectation on every row (an explicit `no witness` or seed error where that is the
answer), a reason from 5.3 where it differs from another level, in either direction, and a truth label. A row whose expectation
is a known wrong answer of today's engines (section 2) names the issue or the slice that fixes it. One
data-driven test per level reads it. The 26 research fixtures and the review probes seed it; the census adds
the shapes real code shows.

### 5.2 Crossings

**Hop evidence (every crossing).** A hop is crossed only on evidence that belongs to that hop. The proof
names one emitting call site: its caller, its half-open byte span, the argument slot (or receiver, or
return) and its target. It holds when (i) the source occurrence is the argument at that slot of that site
(byte containment of the occurrence in the argument is one input; where the graph node does not carry the
occurrence's own bytes, the occurrence is reconstructed, and an ambiguous reconstruction refuses); (ii) no
other site of the caller passes (i) for the same target; (iii) the callee is one of that same site's Exact
targets, and every Exact target of the site is a candidate of one dispatch (interface satisfiers, embedded
promotions, overrides: Go interface dispatch resolves Exact to every implementation and is crossed today, r2
probe `d1`); a pool that mixes grades or holds targets of no one dispatch refuses (`not_exact`), and stage D
fixes how a pool is shown to be one dispatch; (iv) the slot is proven by a positive certificate: every
argument before it is plain positional, the argument itself is positional, formal *i* of the callee is a
plain positional formal, and no receiver offset applies; anything else (a spread, starred, double-starred or
keyword argument, a rest, default or destructured formal, `self`/`cls` offsets, `call`/`apply`/`bind`, a
signature-changing decorator, a Go variadic) refuses (`unproven_slot`); (v) the occurrence carries the value the walk reached: where a
same-line write separates two occurrences (`first('fixed', t); t = 'fixed'; first(t, 'fixed')`), the hop
refuses unless same-line ordering proves which version the site reads.

The proof reads syntax the graph does not hold (argument spans and kinds, the call on the right side of an
assignment, same-line order; section 2): `run(...[], t)` and `run(a0, t)` have the same `CallSite` facts and
the same edge. So the walk receives a syntax view, a trait beside the existing `SameLineOrderView`,
implemented once in core over the parsed files and passed by both clients; an entry point never falls back
to today's rule. The alternative is to store slot, spread flag and site span on binding and return edges at
build time (usable by a cache-loaded graph, but a graph change: `CACHE_VERSION`, edge differential, memory
on every edge, and a collision with lanes 3, 9 and 10 on Step 5b). The choice is a stage D item.

A name-only edge is never crossed
because another site on its line resolves Exact. In particular, under the default ceiling (`exact`) a binding
edge labelled name-only with doubt `call_nameonly`, or `builtin_callback` (a doubt kind lane 9 adds for the
edges its slices D and E derive from a method's name or a derived target), is never crossed (`not_exact`),
neither on its own evidence nor through another call on its line: in `cb(x); handler(0);` the name-only edge
into `cb`'s target never uses `handler`'s resolution, and an occurrence node that carries its own bytes
matches a site by containment only (the line match stays for zero-width and line-keyed nodes). This check ships as a small slice of its own right after
the reasons slice (PLAN I-guard, rows n01, n02), so lane 9's slices D and E2 wait for it, not for the whole
walk repair. A cached decision is keyed by every input it read (source
and target node, slot, site span), never by a key that drops one. Stage D states (i) to (v) against the rows,
including multi-target pools and the lane-3 JSX producer's line-keyed and zero-width source nodes.

| Change | Today | After |
|---|---|---|
| **Binding edges that end at a use** | refused: the target must be a definition | crossed when the hop evidence holds and the use is a read of the formal that the edge's own key path supports: a dotted read whose static chain is a prefix of, or extends, the leaf's key path, or a bare consumption of the whole formal. A computed or literal subscript (`p['clean']`, `p[k]`), a read that executes in a callable nested in the callee (decided by the callable at the target's bytes, not the graph node's owner) and a written path refuse (`target_not_binding`). A recursive binding (the target in the caller's own function, Def or Use) refuses `self_param`, as the Def form does today (`test_self_call_param_stays_boundary`), never an intra-function DataFlow hop |
| **Two call sites on one line** | refused when two sites spelled like the callee match the line, even when only one carries the value; a site of another name does not count (`log(t); g(t)` descends into `g`) | the hop evidence picks the site; refused (`ambiguous_site`) when it does not single one out. `other.run(t); this.run('fixed')` stays refused (`not_exact`): the edge carrying `t` is name-only |
| **Spelling differs from the definition's name** | refused in the cases lane 3's review found (aliased or default import under another name, wrapper around a differently named function) | the site is matched to the callee through its resolved target under the hop evidence, slot proof included; a forwarding wrapper is crossed only through edges of its own body |
| **Returns** (repair) | ascent into any caller with a singleton-Exact call to the callee (section 2 (a)) | the walk state carries the bounded stack of entered sites; after a descent, ascent returns only through the site entered (`return_not_entered_site`); a root inside a callee (nothing entered) keeps today's rule, since a value the callee itself produces reaches every caller |
| **Depth** (repair) | first-come visited set; a recursive Use-target binding is uncounted (section 2 (c), (d)) | the budget counts every crossing, descent and ascent (output `descent_depth` keeps counting descents); a state (node, entered-site stack) reached again with more remaining budget is re-expanded and its witness replaced, so the order of exploration does not decide the answer; stage D fixes the parent store per state (or re-derives the displayed chain under the stack discipline) and retracts boundaries of superseded expansions, so witnesses and warnings do not depend on order either; the depth-0 CFG filter places a synthetic return value at its return statement's line (section 2 (e)) |

The depth cap stays 2 by default. A name-only callee stays refused by default.

### 5.3 Reasons

One closed list explains every refused hop and every difference between default review and the trace engine,
in both directions. Boundary reasons, recorded on the refused hop: `target_not_binding`, `no_matching_site`,
`ambiguous_site`, `not_exact`, `unproven_slot`, `depth_cap`, `policy_ceiling`, `self_param`, `self_return`.
Other causes, recorded by the census and, where the walk observes them, by `taint-reaches`:
`return_not_entered_site` (a cut, not a boundary: the value never goes there, so row w01 stays NotReached),
`no_graph_edge` (no binding or return edge: a producer limit, such as return flow of a
name-only call), `shortcut_suppressed` (the certified constant-return cut of the same-line shortcut),
`cfg_pruned` (the depth-0 CFG filter), `sanitizer_model` (review's body-presence cleansing against a proof on
the path), `operand_granularity` (the line is reached, the sink's operand is not), `unmodelled` (captures,
callbacks). Review-side reasons, for a sink the trace engine reaches and default review does not (r2 probe
`b3`: a multi-line `const y = id(\n t\n)` reaches `exec(y)` only over return flow): `review_no_return_flow`,
`review_no_recovered_def_use` (a field path's recovered def-use on an earlier line), `review_cfg_filter`
(review filters same-function targets at every depth with a 20-line continuation scan), `review_context_policy`
(PR #841's policy applies to the review client only). A reason is bound to the root, the hop's from and to nodes and the requested sink, not to a
warning located in the same function; when the cause cannot be decided, every candidate is kept. A sink
beyond the refused hop's function is bound too: when the verdict would be NotReached and the root has
boundaries, the sink is tested for reach from each boundary's target by the unconditional walk
(`dfg_forward_reachable`, the walk default review uses), memoised per target, with a cap on the boundaries
tested that a warning names; if one reaches it, the verdict is BoundaryExited with that hop's reason (r2
probes `b4`, `b5`: today NotReached, the warning in another function). Anything
else is `unexplained`, which no gate accepts. Today a boundary carries only a kind (cross-function, own
parameter, own return), so every name is new; stage D fixes the names and their precedence. `taint-reaches`
shows the reason with the boundary.

### 5.4 Knobs

`taint-reaches` gains `--edges exact|nameonly` (default `exact`) and `--depth <0..=3>` (default 2, counted as
in 5.2; 3 is the deepest the matrix exercises); the MCP tool gains the same optional inputs. Trace-mode review
keeps its CLI unchanged: the core walk takes the policy, review passes the default, and tests pass other knobs
through the core API. PR #841's policy cannot carry them (it governs context files only), so the lane adds a descent
policy of its own beside it, reusing that PR's ceiling type with another meaning, stated at the flags: #841
compares a context-entering edge's own label (a `sameline` or `cfg_incomplete` doubt fails `Exact`), this
policy the call's resolution grade plus the two doubt kinds of 5.2. The ceiling governs the resolution grade the hop
evidence accepts at descent and at ascent, and only where the graph has an edge: Step 5c builds no return
edge for a name-only call, so `nameonly` does not promise name-only return flow; that loss carries
`no_graph_edge`, and building such edges is a follow-up (a graph change: stored edges, `CACHE_VERSION`, an
edge differential). Equal knobs give equal answers at the walk level of 5.1 only.

### 5.5 Confidence in trace mode

Trace-mode findings keep `unlabeled`: a grade changes labels of an opt-in mode, not reach, reasons or the
catalog. Stage D records the census grade histogram; a grading change (the weaker of the call's resolution and
the binding edge's own label, so an Exact call whose object-literal edge is `NameOnly(SameLine)` stays
name-only; recovered def-use, assignment propagation and return input keep their doubt) is a follow-up for
the owner (decision 4). The matrix carries the expected grade per row.

### 5.6 The catalog

```rust
// src/reasoning/sink_catalog.rs  (core)
pub enum CatalogStrength { Strong, Weak }
pub struct CatalogSink { pub file: String, pub line: usize, pub start_byte: usize, pub end_byte: usize,  // the call
                         pub rule: String, pub cwe: Option<String>,
                         pub category: Option<SanitizerCategory>,   // None: no category, even from the CWE
                         pub strength: CatalogStrength,
                         pub operand: Option<(usize, usize)>,       // the tainted operand decided on; None: any target on the line
                         pub review_cleansed: bool }                // review's own cleansing suppressed it
pub trait SinkCatalog: Sync {
    /// Review's sink decisions for `fans`, one single-source fan per root holding that root's full reach (the
    /// shape review builds), over the graph and files they were walked on.
    fn sink_decisions(&self, cpg: &CodePropertyGraph, files: &BTreeMap<String, ParsedFile>,
                      fans: &[FlowPath]) -> Vec<CatalogSink>;
}
```

Review's predicates are defined on fans and need the graph (section 2): asked about one line with one witness
chain, the same predicate answers otherwise (r2 probe `c2`: `exec(\n u,\n t\n)` with `u = t` is reported at
both lines because the fan holds `u`; the chain to `t` alone matches nothing). So the review crate implements
the trait by running review's own sink loop (the body of `taint/run.rs`'s loop over paths, factored into a
function that returns decisions; its per-run source facts, framework sources and wrapper sinks, computed once
per call from the graph and files), with every predicate as review runs it (`arg_is_tainted_in_path`, the
per-language outcomes including Python template keywords, `flow_path_cleansed_for_sink_call`). An operand is
never paraphrased from a table index. Catalog lines therefore equal trace-mode review's sink lines for the
same roots by construction. In catalog mode the reasoning layer walks forward from the sources as in
frontier mode, builds each root's fan from the trace as trace-mode review does, asks the catalog once, maps
each decision to the reached nodes on its line for verdict, witness and sanitizer walk, and reports each
distinct (file, sink span, rule, operand) with verdict, witness and sanitizer result, under the existing
result caps. The verdict is Sanitized only when a sanitizer of the sink's own category is on the path, for the
witness hits and the bypass exclusions alike (a `review_cleansed` decision is reported, not dropped, so a
sanitized sink stays visible); a sanitizer of another category, or any sanitizer on a rule with no category,
is reported as advisory beside Reached. The line query keeps its documented any-category meaning.
Flat rules lose two things against structured ones, reported rather than hidden: no operand (review's flat
matcher accepts any reached target on the line, so `operand` is `None`), and a category only through the
rule's CWE (79, 89, 918, 502, 78, 22 map to their sanitizer categories; another CWE gives none, so every
sanitizer is advisory). `strength` is carried: a weak hit, which review hides under its default floor, is
reported with `strength: weak`, not as a plain sink.
CLI: `prism nav taint-reaches --source … --sinks catalog`.

Before stage D the emitter families of review's sink lines are inventoried (PLAN Task E-2): each is in the
catalog with rows, or declared unsupported with a diagnostic and rows that pin its absence. Rule ids come
from lane 7's registry (`crates/prism-review/src/output/rule_registry.rs`) when that lane has merged; until
then the classifier's tag is used as is, and a structured sink, which has no tag, is named
`structured-<category>` as lane 7 spells it. The implementation returns the id, so the reasoning layer never
maps ids itself. The classifier holds interior state that cannot be shared between threads, so the
implementation builds one per call; a per-file cache is a stage D option if the cost check (JS, Python and Go
repositories) shows the need. Because the matrix alone cannot show that catalog mode reports what review
reports, a check on real code compares them (gate Catalog, section 7).

### 5.7 Where the code goes

`src/cpg/trace.rs` (1,114 weighted lines: it must not grow), so new logic goes in siblings:
`src/cpg/trace_descent.rs` (hop evidence and reasons), `src/cpg/trace_frames.rs` (walk state: entered sites,
remaining budget, re-expansion); `src/reasoning/` (catalog mode, knobs, the syntax view beside `order.rs`'s
`AstOrderView`, the boundary-target test in `shape.rs`), `crates/prism-review/src/` (the
trait's implementation, CLI flags), `crates/prism-mcp/src/` (inputs; the golden transcripts change and are
re-recorded with the reason in the commit), tests under `tests/` for the matrix.

## 6. Measurement

**Parity census (stage M, after PR #841 merges).** On the 120-diff population of issue #602: for every sink
that default review taint reports, run review in trace mode and record whether the finding survives and, if
not, the boundary. Every run freezes its commit, loaded-file manifest, options, roots and binary. The unit is
a path: source occurrence, call site and slot, sink operand. Yield is counted by sink line. A lost
finding is classed by the relaxation that recovers it, joint relaxations included (`binding+spelling`), or
`unknown`. A relaxation crosses its hop class unconditionally and the log records, per refused hop, the first
failing condition and the observable features (target access, sites on the line, whether containment singles
one out, the spelling of the resolving site, grade, depth): the census measures an upper bound, which is
enough for the ordering rule and decision 6 (an empty upper bound is an empty class), and it is not a
hand-written prototype of 5.2. The relaxation patch is applied to the base commit only and never ported.
Review parses only the diff's files, so population A observes a crossing only when caller and callee are
both in the diff and cross-file classes (spelling, cross-file bindings) would read near zero for a reason of
measurement: the same log is also collected from `taint-reaches` frontier mode on the full checkout for the
census's frozen sources, and a class's size for the ordering rule is the larger of the two. The apparatus is
first run on synthetic fixtures (two roots and two sites on one sink line; a joint barrier) and must change
the right cell. The benchmark's harness records trace-mode review, not default review, so it supplies the
Benchmark gate's paired `taint-reaches` non-regression only, not classes.

**Crossing ledger (product diagnostic, PLAN I-1).** The walk prints every accepted crossing and every refused
hop with its reason under an environment switch, tested like the other trace diagnostics. From the census's
frozen sources, `taint-reaches` frontier mode on the full checkout at each commit gives the whole-repository
ledger; a slice's new crossings are its difference against the parent build, whether or not the sink line is
new. Nothing is re-ported to a slice head.

**After each slice:** the census again, against the slice's parent and the lane baseline; the benchmark's
traced count through `taint-reaches` (it must not fall), and every benchmark case whose outcome becomes
traced is read against its ground truth (a new false witness raises the count); `taint-reaches` frontier
wall time on population A's largest repository against the parent; and a labelled sample of min(100, all)
new crossing paths from the whole-repository ledger (with the review-side ledger beside it), read blind to the
slice with the new hop and its code shown, each labelled twice: "new hop real" (the gate) and "whole path
real" (reported; old line-granular hops can fail it), adjudications kept.

## 7. Gates

| Gate | Threshold (default) |
|---|---|
| Matrix | every row's expectations hold at each level with its reasons; with equal knobs, trace-mode review and `taint-reaches` give the same walk on every `walk_comparable` row |
| Explained | in the census after the lane, every default-review sink that the trace engine does not reach, and every trace-mode finding that default review does not report (the gained list), has a reason from the closed list of 5.3; `unexplained` is zero |
| Recovered | the share of default-review sinks confirmed by the trace engine rises; the classes fixed in 5.2 account for the rise |
| Sound | at least 95 of 100 sampled new crossing paths have a real new hop (a smaller population: all of it, at most 5% not real); the whole-path label is reported; a known false witness is a WRONG whatever the tally; no observations is reported as such |
| Benchmark | traced cases through `taint-reaches` do not decrease in any class; every case that becomes traced has been read and none is a false witness |
| Catalog | on the matrix, catalog mode reports exactly each row's expected multiset of (file, line, rule, operand, verdict), an empty list included; every (file, line) where it differs from review's `taint_sink` findings carries a reason from the closed list in the row; and on real code (population A's repositories at their census commits, the census's frozen sources, one whole-repository graph through the API) the catalog's (file, line) set of decisions not `review_cleansed` equals trace-mode review's `taint_sink` lines, except for emitter families declared unsupported |
| Defaults | with default knobs, review taint's findings are unchanged on the population |

## 8. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Start before PR #841 merges | no; wait for it | starting early means rebasing three files | at start |
| 2 | How the MCP server gets the catalog | **Owner, 2026-11-21: branch A (factor the shared functions out of the review crate, then move them into the core crate).** The choice was made before the closure was measured; it is put to the owner again with the measured size (section 2: the matchers call into source discovery, so branch A moves nearly the whole taint module, about 18,300 of 19,146 lines in 51 of 53 files), and the lane stops before the MCP slices (PLAN I-9) until the owner re-confirms A or picks A2, B or C (section 4). Estimates: A 9 days, A2 7.5, B 2.5, C 0.5 (PLAN I-9) | A, A2 and B are structural changes to the crate layout; A2 adds a crate | stage D, with I-9's step-0 closure measured on the stage-D head, so the answer is in before I-9 |
| 3 | Default of `--edges` for `taint-reaches` | `exact`, as today | flipping to name-only changes agents' answers | — |
| 4 | Grade of trace-mode findings | `unlabeled`, as today; stage D records the histogram, and a change is a follow-up (5.5) | one slice and every trace-review cell | any time |
| 5 | Depth cap | 2, adjustable by flag up to 3 | — | — |
| 6 | No census class has a recoverable member on population A | asked before any descent slice is dispatched (Recovered cannot rise) | — | end of stage M |

## 9. Risks and accepted limits

- **More reach means more to be wrong about.** Each descent change replaces a refusal with a proof of equal
  strength; the labelled sample checks new crossing paths on real code.
- **Known wrong results on main** (section 2 (a) to (g)) are pinned as rows with their truth. The trace
  engine's (a), (c), (d), (e) are fixed in the lane, and (b) is refused by the slot proof until lane 9 removes
  the edge; (f) stays a wrong edge that no crossing may use, and (f) and (g) are filed at close-out. No slice
  widens one.
- **The catalog inherits review's sink precision** (in-diff sinks were right 13 to 17% of the time in one
  measurement). Catalog mode reports rule, operand and witness so a caller can filter; the lane does not make
  sinks more precise.
- **Golden transcripts of the MCP server change** when its tool gains inputs and a mode.
- **Differences that remain by design:** captures, callbacks, name-only return flow, anything review's
  unconditional walk follows that has no hop evidence. They stay in the matrix with their reasons.

Recorded, not pursued: a cheaper first binding slice that admits only proven dotted reads and bare
consumption (stage D may choose it; it is a subset of 5.2); a review-consistent `taint-reaches` query mode
(another public contract). Dropped from the lane as not changing the goal-level result, each a follow-up with
the decision it would support: the trace-mode grade slice (decision 4); descent flags on the review command
(only if a user of trace-mode review other than the benchmark harness asks for them); labelled samples of new
crossings under non-default knobs (only if the owner asks to flip decision 3 or 5); `--depth` above 3 (after
a wall-time measurement). Pre-existing defects found by the round-2 probes, pinned as rows and filed at
close-out: a mixed formal's second member read on a line gets no binding edge (r2 probe `a3`); default
review reports `exec(p.b)` when only `p.a` is tainted (`a1`; catalog mode inherits it); in Python and Go the
second occurrence of a name on a line never binds (`b1p`); Python keyword arguments get no edge (`b2`).
