# Design: Prism Query Layer (seeded analysis)

**Status:** Design (approved direction, pending spec review)
**Date:** 2026-06-07
**Branch:** `feat/prism-query-layer`
**Supersedes emphasis of:** `docs/llm-codebase-navigation-prism-analysis.md`
**Source research:** `LLM Codebase Navigation & Understanding.md`

## 1. Summary

Extend Prism from a diff-only code-review slicer into a tool that also answers
its *differentiated reasoning* questions about a single repository **from a
seed** — a symbol, a `file:line`, or a `source→sink` pair — not only from a
diff. The capability is delivered as a clean Rust library API with two thin
adapters: a CLI surface and an MCP server for coding agents. The existing
`--repo --diff` review path is preserved byte-for-byte.

The design rests on one new abstraction (`FocusSet`) and a two-tier graph
strategy:

- **Tier 1 — whole-repo symbol/call graph** (cheap, breadth): architecture and
  navigation queries across the whole repo.
- **Tier 2 — scoped CPG with data flow** (expensive, depth, on-demand): the
  taint/chop/impact reasoning that is Prism's actual moat.

## 2. Motivation and the reframe

The research note's load-bearing conclusion is that effective LLM codebase
understanding favors a layered stack — *precise symbol/graph navigation plus
lexical search as the backbone, hierarchical localization on top, semantic
embeddings only as a fallback* — and that several "obvious" additions
(LLM-generated context files, pure vector RAG, whole-repo long-context,
unselective memory) measurably hurt.

A prior analysis (`docs/llm-codebase-navigation-prism-analysis.md`) proposed
building a navigation layer on Prism's CPG. That destination is correct, but its
*emphasis was inverted*. Two corrections drive this design:

1. **Lead with differentiated reasoning, not navigation primitives.** Plain
   navigation (go-to-def, find-references, symbol search) is the commoditized
   space where Prism's heuristic name+import resolution is *weaker* than
   compiler-accurate SCIP/LSP and where agentic `grep`+LSP already suffices.
   Prism's moat is the interprocedural reasoning an agent with `grep`+LSP
   *cannot* compute: taint/data-flow chops, change-impact, structural-omission.
   Navigation primitives exist only as plumbing to reach those answers.

2. **The diff is just one seed.** Rather than building navigation as a parallel
   track, we generalize the input. A `FocusSet` is the seed the reasoning
   algorithms consume; a diff is one producer of it. This unifies the review and
   query paths and lets the query layer reuse 100% of the analysis.

### Why these choices (evidence-gated)

- **Single-repo, on-demand + Tier-1 index; not cross-repo.** Cross-repo is a
  genuinely distinct, harder problem where SCIP/Glean win and Prism's heuristic
  resolution is weakest. We stay where Prism is strongest and design a SCIP seam
  for later.
- **No vector RAG / long-context / CGM.** Rejected by the research as
  net-negative or disproportionately costly for the navigation/reasoning goal.
- **Library-first with CLI + MCP adapters.** The stated goal is *agentic*
  software architecture and development; the deployed pattern for that is MCP
  tools (e.g., Sourcegraph MCP and its token-economics argument). Neither
  consumer is privileged; the reasoning logic lives in the library.

## 3. Goals and non-goals

### Goals

- Answer seeded reasoning queries on a single repo: `impact_of_change`,
  `dataflow_between`, plus navigation/architecture plumbing.
- Build whole-repo Tier-1 symbol/call graph views for architecture/navigation.
- Expose the library through a CLI subcommand and an MCP server.
- Preserve the existing diff-review behavior and CLI invocation exactly.
- Produce structured, machine-comparable `Evidence` output suitable for a later
  A/B evaluation against an agentic-search baseline.
- Dogfood on the Prism repository itself.

### Non-goals

Two distinct buckets — **deferred** (intended, sequenced after v1) vs.
**rejected** (not pursuing). Neither is a Prism capability gap.

**Deferred (intended, later):**

- **Cross-repo / org-scale symbol resolution** — via the SCIP/Glean resolver
  seam (§9).
- **A persistent, always-fresh maintained index** (file watchers, background
  rebuilds). Whole-repo indexing *is* in scope for v1 (§7); only the
  always-fresh-under-churn operational layer is deferred.
- **Incremental-cache hardening** — the indirect-call and same-fileset gaps
  (§7.2).

**Rejected (not pursuing — evidence-gated):**

- **Vector / embedding RAG** as a primary retrieval path.
- **Whole-repo long-context prompting** as a substitute for retrieval.
- **CGM-style learned-graph models** (training, fine-tuning, serving).

> Note on framing: whole-repo indexing is **not** a non-goal. v1 performs
> whole-repo indexing via fresh-build-plus-cache, which is sufficient for a
> repo of Prism's size. Only incremental *hardening* (the same-fileset and
> indirect-call gaps in §7.2) is deferred to v2.

## 4. Core abstraction: `FocusSet`

The one new concept. Today every algorithm takes `&DiffInput`. We introduce a
seed that a diff is one producer of.

```rust
// src/query/focus.rs
pub enum Seed {
    Diff(DiffInput),
    Symbol { name: String, file: Option<String> },
    Location { file: String, line: usize },
    SourceSink { source: (String, usize), sink: (String, usize) },
}

pub struct FocusSet {
    /// Files to load into the (scoped) CPG.
    pub files: BTreeSet<String>,
    /// "The changed lines," generalized: file -> focus line numbers.
    pub focus_lines: BTreeMap<String, BTreeSet<usize>>,
    pub origin: Seed,
}

impl FocusSet {
    pub fn from_diff(diff: &DiffInput) -> Self;          // review path
    pub fn from_symbol(ctx: &CpgContext, name: &str, file: Option<&str>) -> Self;
    pub fn from_location(ctx: &CpgContext, file: &str, line: usize) -> Self;
    pub fn from_source_sink(src: (String, usize), sink: (String, usize)) -> Self;
}
```

- `from_diff` is mechanical: `DiffInput` already carries `file_line_map`. It
  **must** reproduce exactly the focus lines the diff-anchored algorithms see
  today (this is the byte-for-byte preservation contract, §8).
- `from_symbol` / `from_location` resolve the defining function(s) via
  `call_graph`/`ast` and expand a bounded caller/callee neighborhood.

### Algorithm migration (minimal blast radius)

Only the algorithms exposed by v1's reasoning tools are migrated to consume
`&FocusSet`:

- **`membrane_slice` and `echo_slice`** change signature from `slice(ctx, diff)`
  to `slice(ctx, focus)` and read `focus.focus_lines` instead of
  `diff.diff_lines`.
- At the existing dispatch boundary (`run_slicing_inner` in
  `src/algorithms/mod.rs`), the review path calls
  `FocusSet::from_diff(diff)` and passes the result. Output is unchanged.

The other diff-anchored algorithms (`delta`, `phantom`, `resonance`) are **not**
migrated in v1; they keep their current `&DiffInput` signatures. The pattern
("algorithms consume `FocusSet`") is established but rolled out incrementally.

## 5. Two-tier architecture

```text
Tier 1 — whole-repo symbol/call graph        cheap, breadth
   AST symbols + CallGraph, no data flow      serves ARCHITECTURE / navigation
   "what depends on cpg.rs", module map,      (callers/callees/ego/depends-on)
   "what calls into this subsystem"

Tier 2 — scoped CPG with data flow           expensive, depth, on-demand
   full taint/chop/impact reasoning           serves DEVELOPMENT / review
   seeded by a FocusSet                        (impact_of_change, dataflow_between)
```

Tier 1 is the RepoGraph/LocAgent/Aider-repo-map shape, built from Prism's
existing `CallGraph` **without** paying for the data-flow graph. Tier 2 builds a
scoped CPG around a seed, fresh per query, so freshness is never in question
where it matters most.

## 6. Library API

New module directory `src/query/` (each file < 600 lines per repo convention):

- `src/query/mod.rs` — public entry points and `QueryResult`/`Evidence` types.
- `src/query/focus.rs` — `Seed`, `FocusSet`, producers (§4).
- `src/query/architecture.rs` — Tier-1 whole-repo views.
- `src/query/reasoning.rs` — Tier-2 seeded reasoning.
- `src/query/evidence.rs` — uniform evidence model and rendering.

```rust
// Tier 2 — reasoning
pub fn dataflow_between(ctx, source: (String, usize), sink: (String, usize)) -> Evidence; // ← chop
pub fn impact_of_change(ctx, focus: &FocusSet) -> Evidence;                                // ← membrane + echo

// Tier 1 — architecture / navigation plumbing (read-only)
pub fn callers(ctx, target: SymbolRef, depth: usize) -> Evidence;
pub fn callees(ctx, target: SymbolRef, depth: usize) -> Evidence;
pub fn ego_graph(ctx, target: SymbolRef, hops: usize, edges: EdgeKinds) -> Evidence;
pub fn nodes_at(ctx, file: &str, line: usize) -> Evidence;
pub fn depends_on(ctx, module: &str) -> Evidence;
pub fn module_map(ctx) -> Evidence;
```

### Evidence model

Every query returns a uniform, serde-serializable package: ranked locations with
an explicit reason for each, so output is explainable and machine-comparable.

```rust
pub struct Evidence {
    pub query: String,
    pub items: Vec<EvidenceItem>,
    pub warnings: Vec<String>,   // e.g., parse-quality, resolution-approximation
}

pub struct EvidenceItem {
    pub file: String,
    pub line_range: (usize, usize),
    pub symbol: Option<String>,
    pub score: f32,
    pub why: Vec<Reason>,        // dataflow edge / caller-without-handler / call / lexical
    pub snippet: Option<String>, // only when requested (token budget)
}

pub enum Reason {
    Dataflow { from: String, to: String },
    Call { callee: String },
    CallerMissingHandler { callee: String, missing: String },
    Containment { parent: String },
    Lexical { term: String },
}
```

`Evidence` renders to text (CLI) and serializes to JSON (CLI `--format json` and
MCP tool results). Snippets are omitted by default to control token cost.

## 7. Repository loading and indexing

### 7.1 Whole-repo loader

New module `src/repo.rs` with explicit load modes:

```rust
pub enum RepoLoadMode {
    DiffFiles,              // existing review default
    ScopedDiffNeighborhood,// existing --scoped-cpg
    WholeRepo,             // navigation / architecture
}
```

The whole-repo loader walks the repo, includes only supported language
extensions, excludes `.git`, common vendor/build dirs (`target/`,
`node_modules/`, `vendor/`, `dist/`, `build/`), honors `.gitignore` when
present, parses to `ParsedFile`, and builds a `CpgContext`.

For Tier 1 it builds **only the `CallGraph` + AST symbols** (no DFG). For Tier 2
it builds a scoped CPG around the seed via `CpgContext::build_scoped`.

### 7.2 Caching strategy (v1: fresh-build-plus-cache)

- A **separate whole-repo cache namespace**, distinct from the per-diff cache,
  keyed by repo path, Prism version, fileset, per-file content hashes, and
  type-enrichment state.
- v1 uses the existing all-or-nothing / same-fileset cache (`cpg_cache.rs`): a
  fresh whole-repo build on first query (seconds for a repo of Prism's size),
  reused while hashes and fileset match.
- **Known, deferred gaps (v2 hardening), stated explicitly:**
  - *Indirect-call approximation:* `build_incremental` resolves only direct
    calls for changed files; indirect edges (function pointers, callbacks) into
    unchanged files rely on cached resolution. A soundness caveat for
    callback-heavy reasoning; fine for navigation/architecture.
  - *Same-fileset constraint:* adding/removing a file busts the partial-hit
    cache and forces a full rebuild. Fine for an edit-existing-files loop; weak
    under heavy churn.

  Neither blocks v1; both are fixable and tracked for v2.

## 8. Preserve diff-review (hard guarantee)

The bare invocation `prism --repo . --diff x.patch -a review` (and every other
existing flag combination) must produce **byte-identical** output to `main`.

Enforced by:

- `Seed::Diff` flows through the *same* algorithm code; only the input wrapper
  changes.
- Golden-output regression tests captured on `main` **before** the `FocusSet`
  refactor, asserted unchanged after.
- A `FocusSet::from_diff` equivalence test proving the generated `focus_lines`
  match the legacy `diff.diff_lines` for representative diffs.

## 9. Adapters

### 9.1 CLI (additive subcommands, back-compat preserved)

`main.rs` is restructured so the existing top-level review arguments remain and
an **optional** subcommand is added:

```rust
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,   // None => run review with top-level args (today's behavior)
    // ... existing review flags remain at top level ...
}

enum Commands {
    Review(ReviewArgs),  // explicit form of the default
    Query(QueryArgs),    // seeded reasoning (no --diff required)
    Nav(NavArgs),        // Tier-1 architecture / navigation
}
```

- `prism --repo . --diff x.patch -a review` → unchanged (implicit `Review`).
- `prism query dataflow --repo . --source src/a.rs:42 --sink src/b.rs:88 --format json`
- `prism query impact --repo . --symbol CpgContext --format json`
- `prism nav callers --repo . --symbol build_scoped --depth 2 --format json`
- `prism nav module-map --repo . --format json`

This preserves the existing CLI contract (no script breakage) while adding the
no-diff paths that whole-repo and seeded queries require.

### 9.2 MCP server

New binary target `src/bin/prism-mcp.rs` (declared in `Cargo.toml`), a thin
shell over `src/query`. It exposes ~5 tools mapping 1:1 to the library
functions: `dataflow_between`, `impact_of_change`, `callers`, `callees`,
`module_map`. Tool results are the `Evidence` JSON.

Transport/SDK: use the official Rust MCP SDK (`rmcp`); the implementation plan
includes a spike to confirm version/transport before committing to the
dependency, with a fallback to a minimal hand-rolled stdio JSON-RPC server if
`rmcp` is unsuitable.

## 10. v1 scope (value-optimized for dogfooding on the Prism repo)

Prism's Rust analysis is structurally strong (call graph, CFG, def-use, impact)
but taint is JS/C/C++-skewed and the Prism repo is not taint-rich. So v1 leads
with the **call-graph-centric** tools, where Rust support is strongest and the
immediate value on this repo is highest.

| In v1 | Backing | Rationale |
|---|---|---|
| `FocusSet` abstraction | new `src/query/focus.rs` | Underpins impact + preserves review |
| Whole-repo Tier-1 load + symbol/call graph | `src/repo.rs`, `CallGraph` | Fresh build + cache; beats LSP for architecture |
| Architecture/nav queries (callers, callees, ego, nodes-at, depends-on, module-map) | `src/query/architecture.rs` | Rust-strong; immediate repo-understanding value |
| `impact_of_change` | `membrane`/`echo` via `FocusSet` | Rust-strong; aids the refactor itself |
| `dataflow_between` | `chop` (already seed-based) | Near-free; works on Rust |
| CLI `query`/`nav` subcommands + JSON | `main.rs` | No-diff invocation surface |
| MCP server (5 tools) | `src/bin/prism-mcp.rs` | Agent-facing surface |

**Deferred to v2:** `taint_reaches` (near-free but low value on this repo),
`whats_missing` (`absence`/`symmetry`/`horizontal`), the SCIP resolver impl,
incremental-cache hardening (§7.2), and migration of the remaining diff-anchored
algorithms to `FocusSet`.

## 11. SCIP seam (designed in v1, implemented later)

Define a resolver interface with the heuristic resolver as default:

```rust
pub trait SymbolResolver {
    fn definition(&self, ctx: &CpgContext, sym: &SymbolRef) -> Vec<Location>;
    fn references(&self, ctx: &CpgContext, sym: &SymbolRef) -> Vec<Location>;
}
```

`HeuristicResolver` (Prism's current name+import logic) is the v1 default. A
future `ScipResolver` **reads `.scip` index files** produced by external
indexers (optionally shelling out to run one), returning compiler-accurate
defs/refs and falling back to the heuristic resolver when no index is present.
Prism does not reimplement or vendor SCIP machinery; it consumes SCIP output.
This is the relief valve for symbol precision and the eventual cross-repo path.

## 12. Evaluation seam

The largest risk is whether the layer beats simply letting an agent read the
code. v1 does not build the harness, but designs for it: `Evidence` output is
structured and machine-comparable so a later A/B can score localization
precision/recall and token cost against an agentic-search baseline, tracked per
language. Building the harness is gated on having tools to measure.

## 13. Testing

- **Golden review regression:** capture diff-review output on `main`, assert
  unchanged after the refactor (§8).
- **`FocusSet::from_diff` equivalence:** generated focus lines match legacy
  `diff.diff_lines`.
- **Whole-repo loader:** ignore rules, extension filtering, fileset hashing,
  cache hit/miss.
- **Query integration:** small multi-file fixtures (including Rust) for
  `impact_of_change`, `dataflow_between`, `callers`/`callees`/`ego`/`module-map`.
- **CLI:** subcommand routing, JSON shape, back-compat of the bare invocation.
- **Dogfood smoke test:** run `prism nav module-map` and
  `prism query impact --symbol CpgContext` against the Prism repo; assert
  non-empty, well-formed `Evidence`.
- **Coverage matrix:** update the three `all_test_files` copies in
  `tests/integration/coverage_test.rs` for any new test files.

## 14. Risks and open questions

- **Seed selection UX** for `dataflow_between` absent a diff: the caller (agent
  tool args / CLI flags) supplies source and sink in v1. Auto-detection is a v2
  concern tied to `taint_reaches`.
- **Heuristic-resolution ceiling** on dynamic dispatch / framework magic, even
  single-repo. Mitigated long-term by the SCIP seam.
- **MCP SDK maturity** (`rmcp`): validated by a spike in the plan.
- **`FocusSet` refactor regressions:** mitigated by golden tests (§8).
- **Default snippet inclusion / token budget** in `Evidence`: default off;
  revisit once the eval harness can measure the trade-off.

## 15. Module/file plan

New:

- `src/query/mod.rs`, `src/query/focus.rs`, `src/query/architecture.rs`,
  `src/query/reasoning.rs`, `src/query/evidence.rs`
- `src/repo.rs`
- `src/bin/prism-mcp.rs`

Modified:

- `src/algorithms/membrane_slice.rs`, `src/algorithms/echo_slice.rs` (consume
  `FocusSet`)
- `src/algorithms/mod.rs` (`run_slicing_inner` wraps diff as `FocusSet`)
- `src/main.rs` (additive subcommands)
- `src/lib.rs` (`pub mod query; pub mod repo;`)
- `Cargo.toml` (MCP binary target, `rmcp` dependency, MCP test target)
- `tests/integration/coverage_test.rs` (if new language test files are added)

## 16. Bottom line

```text
Prism diff review remains unchanged (byte-for-byte)
        +
FocusSet: the diff becomes one seed among several
        +
Tier 1 whole-repo symbol/call graph (architecture / navigation)
        +
Tier 2 on-demand scoped reasoning (impact_of_change, dataflow_between)
        +
CLI subcommands + MCP server over one library API
        +
SCIP resolver seam (implemented later)
```

This keeps Prism playing only where it is the strongest player, preserves its
zero-staleness structural edge, serves both the architecture and development
use cases, and delivers immediate dogfoodable value on the Prism repository
itself.
