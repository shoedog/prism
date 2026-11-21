# Lane 8: one taint answer from both engines — design

Status: architecture final; the descent rules of section 5.2 are tentative until the parity census and the
matrix (PLAN stages M, T, E). Starts after draft PR #841 merges (it edits the same three files and adds the
crossing-policy type this lane uses). The plan files a tracking issue. Research record:
`~/prism-evidence/lanes-2026-11-20/research/lane8-taint-parity.md`.

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
| The trace engine is used by review only with `--taint-return-flow`, and always by the reasoning layer and the MCP tool. It crosses into a callee only when the edge ends at a definition, exactly one call site matches the source occurrence (same line, or the occurrence's bytes inside the site), that site resolves Exact to the target, and the depth is under 2 (shared with return ascent). A refused hop is recorded as a boundary and not followed. | `src/cpg/trace.rs:712`, `:25`; `src/reasoning/taint_reaches.rs:54` |
| Probe matrix, 26 fixtures in three languages: default review and `taint-reaches` agree on 20 and disagree on 6. Five are descent refusals: an object-literal argument bound to a member read (the edge ends at a use; two fixtures), two call sites of the callee on one line, a name-only callee, a call chain three deep. One is sink granularity (a use that shares the sink's line). Review in trace mode agrees with `taint-reaches` on 25 of 26. Both engines miss a captured variable read in a nested function. | research section 6 |
| In trace mode, findings that are `exact` in default mode become `unlabeled`, because descent and assignment hops mark the evidence as crossing unlabelled edges. | `src/finding_confidence.rs:348-379` |
| The trace engine has no notion of a sanitizer; the reasoning layer adds one on the witness chain. Review uses category presence in the source function's body. | `src/reasoning/sanitizer_walk.rs`; `crates/prism-review/src/algorithms/taint/cleansing.rs` |
| PR #841 adds an optional crossing policy (a confidence ceiling of Exact or name-only) threaded through the walk functions; the descent test gains one policy term; the descent rule, the depth cap and review's choice of engine are unchanged, and the reasoning layer passes no policy. With its defaults, every difference above remains. | PR #841 |
| The sink catalog lives in the review crate: the flat pattern list, the positional classifier with its scope resolvers and module tables (23 files, about 7,000 lines with tests), the per-language structured tables, the matchers. It imports core types and two review helpers. The core crate holds the sink pattern type, the sanitizer recognisers and nearly empty per-framework sink lists that nothing reads. | research section 5 |
| The `prism` binary is built from the review crate, so `prism nav taint-reaches` can reach the catalog. The MCP server is a separate crate that depends on core only, and a layering test forbids it to name the review crate. | `tests/integration/layering_test.rs`; `crates/prism-mcp/Cargo.toml` |
| On real diffs, the cross-file sinks that issue #602's measurement added were right 10 to 23% of the time, against 13 to 17% for in-diff sinks. | issue #602, M2 |

## 3. Goals, non-goals, anti-goals

Goals:

1. A committed parity matrix: for each fixture, what each engine answers and, where they differ, the named
   reason. It is the contract; any later change that moves a cell fails a test.
2. The trace engine crosses the hops that are as well founded as the ones it crosses today: an object-literal
   or props binding edge of an Exact singleton call; a call site singled out by byte containment when two
   sites share a line; a callee whose spelling at the site differs from its definition's name.
3. Every refused hop reports one reason from a closed list, in `taint-reaches` output.
4. The same crossing knobs on both engines (confidence ceiling, depth), with today's defaults.
5. A catalog mode for the reasoning layer: from a source, the catalog sinks reached, each with its rule,
   verdict and witness; first in the CLI, then in the MCP tool.

Non-goals: replacing review's walk with the trace engine (it would drop findings review reports today);
new sinks or sources; captures in nested functions; callbacks (lane 9); improving the precision of review
taint; cross-file context (PR #841).

Anti-goals: loosening descent by removing a condition instead of replacing it with an equally strong one;
a catalog copy in the core crate that can drift from the review crate's; changing default answers of either
engine without a matrix row that says so.

## 4. Approaches considered

For the engine differences:

**A. One engine.** Make review use the trace engine. Measured consequence: five of the six disagreeing
fixtures lose their finding, and trace-mode findings lose their `exact` grade. Rejected.

**B. Make the walk identical by loosening the trace engine to follow every edge.** It would stop being a
witness-producing, Exact-gated engine, which is what the reasoning layer's verdicts mean. Rejected.

**C. Keep both, pin the differences, fix the defects** (chosen). The engines answer different questions
(review: could this reach; trace: here is a justified path). The matrix records which cells differ and why;
the defect classes are fixed in the trace engine; the knobs let either be run under the other's policy.

For the catalog:

**A. Move the sink classifier into the core crate.** Then the reasoning layer and the MCP server can call
it. It is about 7,000 lines in 23 files with two review-crate helpers to untangle: mechanical, large, and a
change to the documented crate layout.

**B. A trait in core, implemented in the review crate and passed in** (chosen for the first step). The
`prism` binary gets catalog mode at once with no move. The MCP server cannot use it until either A is done
or the layering rule is relaxed for it; that choice is the owner's (section 8, decision 2) and is the last
slice.

## 5. Architecture — *descent rules tentative*

### 5.1 The parity matrix

Fixtures under `tests/fixtures/taint_parity/` with a rows file: per row, the source, the sink, the
mechanism, and the expected answer of (a) default review taint, (b) review taint in trace mode,
(c) `taint-reaches`, with a reason from the closed list of 5.3 wherever (c) is not Reached but (a) reports.
One data-driven test per engine reads it. The 26 research fixtures seed it; the census adds the shapes real
code shows.

### 5.2 Descent, three changes

| Change | Today | After |
|---|---|---|
| Binding edges that end at a use | refused: the target must be a definition | also crossed when the edge's target is a read of a formal of the callee and its source occurrence lies inside exactly one call site that resolves Exact to that callee (the same proof a definition target needs) |
| Two call sites on one line | refused: both match by line | the site whose byte range contains the source occurrence is chosen; refused only when none or several contain it |
| Spelling differs from the definition's name | refused in the cases lane 3's review found (aliased or default import under another name, wrapper around a differently named function) | the site is matched to the callee through its resolved target, not through the name |

The depth cap stays 2 by default. A name-only callee stays refused by default.

### 5.3 Boundary reasons

A closed list reported on every refused hop: `target_not_binding` (the edge ends at a use that is not a
formal's read), `no_matching_site`, `ambiguous_site`, `not_exact`, `depth_cap`, `policy_ceiling`, and the
existing return-flow reasons. `taint-reaches` shows the reason with the boundary.

### 5.4 Knobs

`taint-reaches` gains `--edges exact|nameonly` (default `exact`) and `--depth <n>` (default 2), implemented
with PR #841's crossing policy; the MCP tool gains the same optional inputs. Review taint already has the
corresponding options after PR #841. With the same knob values, trace-mode review and `taint-reaches` must
agree on every matrix row.

### 5.5 Confidence in trace mode

Whether a finding reached through a justified descent should grade `exact` is decided at stage D with the
census numbers; the matrix carries the expected grade per row either way.

### 5.6 The catalog

```rust
// src/reasoning/sink_catalog.rs  (core)
pub struct CatalogSink { pub line: usize, pub start_byte: usize, pub end_byte: usize,
                         pub rule: String, pub cwe: Option<String>,
                         pub tainted_operands: Vec<(usize, usize)> }   // byte ranges that must be tainted; empty = any use on the line
pub trait SinkCatalog: Sync {
    /// Sinks on one line of one parsed file.
    fn sinks_on_line(&self, parsed: &ParsedFile, line: usize) -> Vec<CatalogSink>;
}
```

The review crate implements it over the classifier and the structured tables it already has. In catalog
mode the reasoning layer walks forward from the sources as in frontier mode, asks the catalog about each
line that holds a tainted use, keeps the sinks whose required operands are tainted, and reports each with
verdict, witness and sanitizer result, under the existing result caps. CLI: `prism nav taint-reaches
--source … --sinks catalog`. Rule ids come from lane 7's registry when that lane has merged; until then the
classifier's tag is used as is.

### 5.7 Where the code goes

`src/cpg/trace.rs` (descent and reasons; 1,114 weighted lines, so new logic goes in a sibling module
`src/cpg/trace_descent.rs`), `src/reasoning/` (catalog mode, knobs), `crates/prism-review/src/` (the trait's
implementation, CLI flags), `crates/prism-mcp/src/` (inputs; the golden transcripts change and are
re-recorded with the reason in the commit), tests under `tests/` for the matrix.

## 6. Measurement

**Parity census (stage M, after PR #841 merges).** On the 120-diff population of issue #602 and on the
benchmark's traced cases: for every sink that default review taint reports, run review in trace mode and
record whether the finding survives and, if not, the boundary. This sizes each difference class on real
code and orders the slices.

**After each slice:** the census again; the benchmark's traced count through `taint-reaches` (it must not
fall; binding-edge descent should raise it); and, for sinks newly reached by the trace engine, a labelled
sample of 100 checked by a reader for whether the path is real.

## 7. Gates

| Gate | Threshold (default) |
|---|---|
| Matrix | every row's three answers and reasons hold; with equal knobs, trace-mode review equals `taint-reaches` on every row |
| Explained | in the census after the lane, every default-review sink that the trace engine does not reach has a boundary reason from the closed list; none is unexplained |
| Recovered | the share of default-review sinks confirmed by the trace engine rises; the classes fixed in 5.2 account for the rise |
| Sound | at least 95 of the 100 sampled newly reached paths are real |
| Benchmark | traced cases through `taint-reaches` do not decrease in any class |
| Catalog | on the matrix, catalog mode reports exactly the sinks review taint reports for the same source, less the rows whose boundary reason is listed |
| Defaults | with default knobs, review taint's findings are unchanged on the population |

## 8. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Start before PR #841 merges | no; wait for it | starting early means rebasing three files | at start |
| 2 | How the MCP server gets the catalog | move the sink classifier into the core crate by move-only changes, as the lane's last slices; the alternative is to let the MCP crate depend on the review crate and change the layering test | each is a structural change to the crate layout | before the last slice |
| 3 | Default of `--edges` for `taint-reaches` | `exact`, as today | flipping to name-only changes agents' answers | — |
| 4 | Grade of trace-mode findings | decided at stage D | — | stage D |
| 5 | Depth cap | 2, adjustable by flag | — | — |

## 9. Risks and accepted limits

- **More reach means more to be wrong about.** Each descent change replaces a refusal with a proof of equal
  strength; the labelled sample checks it on real code.
- **The catalog inherits review's sink precision** (in-diff sinks were right 13 to 17% of the time in one
  measurement). Catalog mode reports rule and witness so a caller can filter; the lane does not make sinks
  more precise.
- **Golden transcripts of the MCP server change** when its tool gains inputs and a mode.
- **Differences that remain by design:** captures, callbacks, anything review's unconditional walk follows
  that has no call-site proof. They stay in the matrix with their reasons.
