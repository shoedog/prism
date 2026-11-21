# Lane 8: one taint answer from both engines — design

Status: architecture final; the crossing rules of section 5.2 are tentative until the parity census and the
matrix (PLAN stages M, T, E). Round 1 of the design review is folded (2026-11-21; its probes, cited below as
"review probe", are in `~/prism-evidence/lanes-2026-11-20/reviews/astra-r1/lane8/probes/`). Starts after
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
| The sink catalog lives in the review crate: the flat pattern list, the positional classifier with its scope resolvers and module tables (23 files, about 7,000 lines with tests), the per-language structured tables, the matchers. The matchers import review's cleansing, framework-parameter and source-detection helpers and `TaintSeed` (`taint/sink_match.rs`, `taint/{js_ts,python,go}/sinks.rs`, `taint_sinks/structured.rs`). A static table's argument index is not always the operand review checks (`render_template_string`: index 1; review checks the tainted keyword's template variable). The core crate holds the sink pattern type, the sanitizer recognisers and nearly empty per-framework sink lists that nothing reads. | research section 5; `taint/python/sink_tables.rs:31`, `taint/python/sinks.rs:49` |
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
3. Every lost sink and every refused hop reports one reason from one closed list (5.3), in `taint-reaches`
   output and in the census.
4. The same crossing knobs (confidence ceiling, depth) on both clients of the trace walker, `taint-reaches`
   and trace-mode review, with today's defaults. Default review has no descent gate and gets none.
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

**A. Move the sink classifier into the core crate.** Then the reasoning layer and the MCP server can call
it. It is about 7,000 lines in 23 files, and the matchers reach review's cleansing and source detection, so
shared occurrence-matching functions must first be factored out of them: large, and a change to the
documented crate layout.

**B. A trait in core, implemented in the review crate and passed in** (chosen for the first step). The
`prism` binary gets catalog mode at once with no move. The MCP server cannot use it until either A is done
or the layering rule is relaxed for it; that choice is the owner's (section 8, decision 2) and is the last
slice.

## 5. Architecture — *crossing rules tentative*

### 5.1 The parity matrix

Fixtures under `tests/fixtures/taint_parity/` with a rows file. A row names occurrences, not lines: the
source occurrence (file, line, bytes), the call site and argument slot or the return it tests, and the sink
operand (file, line, bytes). Parity has three levels, asserted separately:

- **Walk:** the trace walker's reached nodes and witness for the row's source occurrence as the only root,
  under given knobs. `taint-reaches` and trace-mode review are two clients of one walk, so with equal knobs
  they must give the same walk; this is asserted only on rows whose inputs are genuinely identical (the
  source line holds one occurrence and no framework source; flag `walk_comparable`).
- **Finding:** review's `taint_sink` finding at the sink (file, line) and its grade, for default and trace
  mode. Review's roots (every variable on the source line, framework sources), its root exclusion and its
  cleansing differ from the walk, so a difference from the walk level carries a reason.
- **Catalog:** the multiset of (file, line, rule, operand, verdict) catalog mode reports (5.6).

Each level has its own expectation on every row (an explicit `no witness` or seed error where that is the
answer), a reason from 5.3 where it differs from another level, and a truth label. A row whose expectation
is a known wrong answer of today's engines (section 2) names the issue or the slice that fixes it. One
data-driven test per level reads it. The 26 research fixtures and the review probes seed it; the census adds
the shapes real code shows.

### 5.2 Crossings

**Hop evidence (every crossing).** A hop is crossed only on evidence that belongs to that hop. The proof
names one emitting call site: its caller, its half-open byte span, the argument slot (or receiver, or
return) and its target. It holds when (i) the source occurrence is the argument at that slot of that site
(byte containment of the occurrence in the argument is one input; where the graph node does not carry the
occurrence's own bytes, the occurrence is reconstructed, and an ambiguous reconstruction refuses); (ii) no
other site of the caller passes (i) for the same target; (iii) that same site resolves to exactly one Exact
target and it is the callee; (iv) the slot is proven: no spread or other unknown expansion (JS), starred or
double-starred argument (Python) precedes it; (v) the occurrence carries the value the walk reached: where a
same-line write separates two occurrences (`first('fixed', t); t = 'fixed'; first(t, 'fixed')`), the hop
refuses unless same-line ordering proves which version the site reads. A name-only edge is never crossed
because another site on its line resolves Exact. In particular, under the default ceiling (`exact`) a binding
edge labelled name-only with doubt `call_nameonly`, or `builtin_callback` (a doubt kind lane 9 adds for the
edges its slices D and E derive from a method's name or a derived target), is never crossed (`not_exact`),
neither on its own evidence nor through another call on its line: in `cb(x); handler(0);` the name-only edge
into `cb`'s target never uses `handler`'s resolution. Lane 9's slices D and E2 merge only after this check
exists (PLAN I-walk, rows n01, n02). A cached decision is keyed by every input it read (source
and target node, slot, site span), never by a key that drops one. Stage D states (i) to (v) against the rows,
including multi-target sites (today one Exact target among several suffices) and the lane-3 JSX producer's
line-keyed and zero-width source nodes.

| Change | Today | After |
|---|---|---|
| **Binding edges that end at a use** | refused: the target must be a definition | crossed when the hop evidence holds and the use is a read of the formal that the edge's own key path supports: a dotted read whose static chain is a prefix of, or extends, the leaf's key path, or a bare consumption of the whole formal. A computed or literal subscript (`p['clean']`, `p[k]`), a read that executes in a callable nested in the callee (decided by the callable at the target's bytes, not the graph node's owner) and a written path refuse (`target_not_binding`). A recursive binding (the target in the caller's own function, Def or Use) is a self-parameter crossing counted against the depth, never an intra-function DataFlow hop |
| **Two call sites on one line** | refused when two sites spelled like the callee match the line, even when only one carries the value; a site of another name does not count (`log(t); g(t)` descends into `g`) | the hop evidence picks the site; refused (`ambiguous_site`) when it does not single one out. `other.run(t); this.run('fixed')` stays refused (`not_exact`): the edge carrying `t` is name-only |
| **Spelling differs from the definition's name** | refused in the cases lane 3's review found (aliased or default import under another name, wrapper around a differently named function) | the site is matched to the callee through its resolved target under the hop evidence, slot proof included; a forwarding wrapper is crossed only through edges of its own body |
| **Returns** (repair) | ascent into any caller with a singleton-Exact call to the callee (section 2 (a)) | the walk state carries the bounded stack of entered sites; after a descent, ascent returns only through the site entered (`return_not_entered_site`); a root inside a callee (nothing entered) keeps today's rule, since a value the callee itself produces reaches every caller |
| **Depth** (repair) | first-come visited set; a recursive Use-target binding is uncounted (section 2 (c), (d)) | the budget counts every crossing, descent and ascent (output `descent_depth` keeps counting descents); a state (node, entered-site stack) reached again with more remaining budget is re-expanded and its witness replaced, so the order of exploration does not decide the answer; the depth-0 CFG filter places a synthetic return value at its return statement's line (section 2 (e)) |

The depth cap stays 2 by default. A name-only callee stays refused by default.

### 5.3 Reasons

One closed list explains every refused hop and every default-review sink that the trace engine does not
reach. Boundary reasons, recorded on the refused hop: `target_not_binding`, `no_matching_site`,
`ambiguous_site`, `not_exact`, `unproven_slot`, `return_not_entered_site`, `depth_cap`, `policy_ceiling`,
`self_param`, `self_return`. Other causes, recorded by the census and, where the walk observes them, by
`taint-reaches`: `no_graph_edge` (no binding or return edge: a producer limit, such as return flow of a
name-only call), `shortcut_suppressed` (the certified constant-return cut of the same-line shortcut),
`cfg_pruned` (the depth-0 CFG filter), `sanitizer_model` (review's body-presence cleansing against a proof on
the path), `operand_granularity` (the line is reached, the sink's operand is not), `unmodelled` (captures,
callbacks). A reason is bound to the root, the hop's from and to nodes and the requested sink, not to a
warning located in the same function; when the cause cannot be decided, every candidate is kept. Anything
else is `unexplained`, which no gate accepts. Today a boundary carries only a kind (cross-function, own
parameter, own return), so every name is new; stage D fixes the names and their precedence. `taint-reaches`
shows the reason with the boundary.

### 5.4 Knobs

`taint-reaches` gains `--edges exact|nameonly` (default `exact`) and `--depth <n>` (default 2, counted as
in 5.2); the MCP tool gains the same optional inputs, and review taint gains the same two options for its
trace mode. PR #841's policy cannot carry them (it governs context files only), so the lane adds a descent
policy of its own beside it, reusing that PR's ceiling type. The ceiling governs the resolution grade the hop
evidence accepts at descent and at ascent, and only where the graph has an edge: Step 5c builds no return
edge for a name-only call, so `nameonly` does not promise name-only return flow; that loss carries
`no_graph_edge`, and building such edges is a follow-up (a graph change: stored edges, `CACHE_VERSION`, an
edge differential). Equal knobs give equal answers at the walk level of 5.1 only.

### 5.5 Confidence in trace mode

Whether a finding reached through a justified crossing should grade better than `unlabeled` is decided at
stage D with the census numbers; the matrix carries the expected grade per row either way. If it does, the
crossing grades as the weaker of the call's resolution and the binding edge's own label (an Exact call whose
object-literal edge is `NameOnly(SameLine)` stays name-only), and recovered def-use, assignment propagation
and return input keep their doubt.

### 5.6 The catalog

```rust
// src/reasoning/sink_catalog.rs  (core)
pub struct CatalogSink { pub file: String, pub line: usize, pub start_byte: usize, pub end_byte: usize,
                         pub rule: String, pub cwe: Option<String>,
                         pub category: Option<SanitizerCategory>,   // None: a rule with no category
                         pub operand: Option<(usize, usize)> }      // the tainted operand decided on; None: any target on the line
pub trait SinkCatalog: Sync {
    /// The sinks on `line` that `path`, a witness ending on that line, taints, decided by the implementation's
    /// own operand, semantic and per-path cleansing predicates.
    fn sinks_for_path(&self, parsed: &ParsedFile, line: usize, path: &FlowPath) -> Vec<CatalogSink>;
}
```

The review crate implements it with review's own decisions: the flat classifier, and the structured outcome
dispatch with its per-path predicates (`arg_is_tainted_in_path`, the per-language outcomes including Python
template keywords, `flow_path_cleansed_for_sink_call`). An operand is never paraphrased from a table index.
In catalog mode the reasoning layer walks forward from the sources as in frontier mode, asks the catalog
about every line holding a reached occurrence, Def or Use (review's flat matcher sees a line reached only at
a parameter Def), passing the witness to that occurrence as a `FlowPath`, and reports each distinct (file,
sink span, rule, operand) with verdict, witness and sanitizer result, under the existing result caps. The
verdict is Sanitized only when a sanitizer of the sink's own category is on the path, for the witness hits
and the bypass exclusions alike; a sanitizer of another category, or any sanitizer on a rule with no
category, is reported as advisory beside Reached. The line query keeps its documented any-category meaning.
CLI: `prism nav taint-reaches --source … --sinks catalog`.

Before stage D the emitter families of review's sink lines are inventoried (PLAN Task E-2): each is in the
catalog with rows, or declared unsupported with a diagnostic and rows that pin its absence. Rule ids come
from lane 7's registry (`crates/prism-review/src/output/rule_registry.rs`) when that lane has merged; until
then the classifier's tag is used as is, and a structured sink, which has no tag, is named
`structured-<category>` as lane 7 spells it. The implementation returns the id, so the reasoning layer never
maps ids itself. The classifier holds interior state that cannot be shared between threads, so the
implementation builds one per call; a per-file cache is a stage D option if the cost check (JS, Python and Go
repositories) shows the need.

### 5.7 Where the code goes

`src/cpg/trace.rs` (1,114 weighted lines: it must not grow), so new logic goes in siblings:
`src/cpg/trace_descent.rs` (hop evidence and reasons), `src/cpg/trace_frames.rs` (walk state: entered sites,
remaining budget, re-expansion); `src/reasoning/` (catalog mode, knobs), `crates/prism-review/src/` (the
trait's implementation, CLI flags), `crates/prism-mcp/src/` (inputs; the golden transcripts change and are
re-recorded with the reason in the commit), tests under `tests/` for the matrix.

## 6. Measurement

**Parity census (stage M, after PR #841 merges).** On the 120-diff population of issue #602: for every sink
that default review taint reports, run review in trace mode and record whether the finding survives and, if
not, the boundary. Every run freezes its commit, loaded-file manifest, options, roots and binary. The unit is
a path: source occurrence, call site and slot, sink operand. Yield is counted by sink line; soundness is
sampled from a ledger of new crossings, so a new path to a sink line already reported is sampled too. A lost
finding is classed by the relaxation that recovers it, joint relaxations included (`binding+spelling`), or
`unknown`; a relaxation applies the tentative proof of 5.2, so it recovers only what the rule could. The
apparatus is first run on synthetic fixtures (two roots and two sites on one sink line; a joint barrier) and
must change the right cell. The benchmark's harness records trace-mode review, not default review, so it
supplies the Benchmark gate's paired `taint-reaches` non-regression only, not classes.

**After each slice:** the census again, against the slice's parent and the lane baseline; the benchmark's
traced count through `taint-reaches` (it must not fall); and a labelled sample of min(100, all) new crossing
paths, read blind to the slice with the new hop and its code shown, adjudications kept.

## 7. Gates

| Gate | Threshold (default) |
|---|---|
| Matrix | every row's expectations hold at each level with its reasons; with equal knobs, trace-mode review and `taint-reaches` give the same walk on every `walk_comparable` row |
| Explained | in the census after the lane, every default-review sink that the trace engine does not reach has a reason from the closed list of 5.3; `unexplained` is zero |
| Recovered | the share of default-review sinks confirmed by the trace engine rises; the classes fixed in 5.2 account for the rise |
| Sound | at least 95 of 100 sampled new crossing paths are real (a smaller population: all of it, at most 5% not real); a known false witness is a WRONG whatever the tally; no observations is reported as such |
| Benchmark | traced cases through `taint-reaches` do not decrease in any class |
| Catalog | on the matrix, catalog mode reports exactly each row's expected multiset of (file, line, rule, operand, verdict), an empty list included; every (file, line) where it differs from review's `taint_sink` findings carries a reason from the closed list in the row |
| Defaults | with default knobs, review taint's findings are unchanged on the population |

## 8. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Start before PR #841 merges | no; wait for it | starting early means rebasing three files | at start |
| 2 | How the MCP server gets the catalog | **Owner, 2026-11-21: branch A (factor the shared functions out, then move them into the core crate).** The lane no longer stops for this answer; it stops only if branch A's first step meets its own stop rule, and then proposes B or C to the owner. Background: the originally planned default, moving the classifier into core by move-only changes, meets the plan's own stop condition: the matchers import two cleansing helpers (`taint/sink_match.rs`), path-aware cleansing, framework parameters and source access (`taint/js_ts/sinks.rs`), Flask source detection (`taint/python/sinks.rs`), `TaintSeed` (`taint/go/sinks.rs`) and a review matcher (`taint_sinks/structured.rs`). Branches: A, factor the shared occurrence-matching functions out of source discovery and emission, then move them into core (or a small crate below both binaries); B, the MCP crate depends on the review crate through one adapter file and a narrow layering exception; C, CLI only, MCP pending | A and B are structural changes to the crate layout | asked at stage D with the measured closure, so the answer is in before I-9 |
| 3 | Default of `--edges` for `taint-reaches` | `exact`, as today | flipping to name-only changes agents' answers | — |
| 4 | Grade of trace-mode findings | decided at stage D, within 5.5's weaker-of rule | — | stage D |
| 5 | Depth cap | 2, adjustable by flag | — | — |
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
(another public contract).
