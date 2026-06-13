# S2 — Node-Identity Hardening — Design

**Date:** 2026-06-13 · **Status:** rev 1 — owner-approved in brainstorm (identity
primitive, function-identity representation, and Variable span contract all decided
interactively; dual evaluation by codex gpt-5.5 xhigh + claude-opus-4-8 max converged
on byte-range). **Context docs:** `docs/cpg-substrate-analysis-2026-06-10.md` (§2 F1/F2,
§3 S2 row, R1 register), `docs/prism-query-layer/tier2-planB-taint-reaches-constraints-merged-2026-06-10.md`
(the Slices 3d/5 this enables deleting), `project_prism_s3_precision` (S3, just merged —
`origin/main` @ d972079).

## 0. Why now

prism's CPG nodes are **line-granular**: `Variable { path, file, function: String, line,
access }`, `Statement { file, line, kind }`, function identity is the string `(file,
name)` with last-writer-wins (`src/cpg/build.rs` func_index). Two consequences (substrate
F1/F2):

1. **Same-line occurrence order is unrecoverable from the graph.** `trace.rs::
   same_line_same_path_uses` resolves same-line def/use propagation (`x = y` on one line)
   from `location_index` buckets sorted by `NodeIndex` (insertion order, *not* source
   order), and multiple occurrences of one path on a line collapse to one node.
2. **Same-named functions in one file conflate** (`impl A { fn f }` / `impl B { fn f }`,
   C++ overloads, nested fns) — func_index overwrites; `Variable.function` is a bare name.

The next increment, **Plan B (taint_reaches)**, emits a serialized *witness* over these
nodes. Its rev-4 plan compensates for both defects with an entire AST-based ordering
oracle (Slice 5: `order.rs`, `SameLineOrderView`, `line_on_cfg_cycle`, conservative-keep)
and a function-identity workaround (Slice 3d). **Once Plan B ships, the node identity is
pinned by the witness wire shape + regression fixtures** — fixing it then means
re-validating the first reasoning wire contract. S2 is the cheapest moment that will ever
exist. It also fixes overload conflation for nav and every future tool *now*.

This is the highest-leverage "before, not after" item (substrate §3/R1).

## 1. Decisions (owner-approved 2026-06-13)

| Decision | Choice | Why (dual-eval rationale) |
|---|---|---|
| Identity primitive on Variable/Statement | **Byte range** (`start_byte` + `end_byte`) | Strict superset of start_byte; `end_byte` is **not recoverable** from a serialized node/witness without reparsing the original source, so a later add is a wire break for already-emitted witnesses — exactly what S2 must avoid. Cost is one integer/node (trivial). Ordinal is dominated (byte→ordinal trivial; ordinal→byte needs source replay) and extractor-version-fragile. Ranges are the universal code-intel identity (LSP/tree-sitter/SCIP). |
| Function identity | **Content-stable span id** `(file, start_byte)`; `Variable` references it via `function_start: usize`, dropping the name from identity | Cleaner model than name+tiebreaker; a *positional* function-table index would renumber under `build_incremental` and break the wire, so the id is the span key, not a position. Name stays as a display field on the Function node. |
| Variable span contract | **Full access-path expression extent**, fallback to anchor identifier | `end_byte` only earns its place if it spans the occurrence; `self.config.timeout` covers all segments. Synthesized/normalized paths with no single AST node fall back to the anchor identifier's span. |
| Packaging | **One increment, one `CACHE_VERSION` bump (v4→v5)** | Substrate §3: batch the schema change so consumers (and Plan B's wire) absorb it once. |

## 2. Data model

`src/cpg/types.rs` — all three `CpgNode` variants gain byte spans; Variable's function
reference changes from name to span id:

```rust
Function { name: String, file: String, start_line: usize, end_line: usize,
           start_byte: usize, end_byte: usize },          // +start_byte, +end_byte
Statement { file: String, line: usize, kind: StmtKind,
            start_byte: usize, end_byte: usize },         // +start_byte, +end_byte
Variable { path: AccessPath, file: String, line: usize, access: VarAccess,
           start_byte: usize, end_byte: usize,            // +start_byte, +end_byte
           function_start: usize },                       // REPLACES `function: String`
```

- **Function identity = `(file, start_byte)`.** `func_index: BTreeMap<(String,String),
  NodeIndex>` (file,name) → `BTreeMap<(String, usize), NodeIndex>` (file, start_byte).
  Collisions become impossible (distinct functions have distinct start bytes) → the
  last-writer-wins quirk and its `step5b_param_binding_first_wins_parity` pin are retired.
- `Variable.function_start` = the containing function's `start_byte` (its id within the
  shared `file`). The name, where needed for display, is resolved via the Function node
  (`cpg.function_of(var) -> Option<&Function>`, a new query helper).
- Derives unchanged (Debug, Clone, PartialEq, Eq, Serialize, Deserialize). The new fields
  are `usize` — deterministic in any BTreeMap key.

## 3. Variable span contract (the precise wire-shape rule)

For a Variable occurrence, `[start_byte, end_byte]` is the source extent of the
**access-path expression** at that occurrence:
- Single identifier `x` → that token's span.
- Member/field/index expression `self.config.timeout`, `dev->name`, `a[i].b` → the span
  of the whole expression (first byte of the leftmost segment .. last byte of the
  rightmost). `start_byte` is therefore the occurrence's first byte (ordering anchor).
- **Fallback (synthesized/normalized paths):** when the AccessPath was built from
  normalized text with no single backing node (e.g. some destructuring projections), use
  the **anchor identifier's span** (the base token), with `end_byte` = anchor end. Each
  language's extraction defines its anchor; the fallback must be deterministic.

This is the contract Plan B's witness and any future span consumer depends on, so it is
specified here, not left to the implementation.

## 4. Data flow & indexes

**Extraction (`src/ast.rs`)** — the helpers that currently return line-only gain spans
(the tree-sitter node yields `start_byte()`/`end_byte()` in the same call):
- `assignment_lvalue_paths_on_lines` / `rvalue_identifier_paths_on_lines`:
  `(AccessPath, line)` → `(AccessPath, line, start_byte, end_byte)`.
- `statements_in_function`: `(line, kind)` → `(line, kind, start_byte, end_byte)`, and
  **stop the line-dedup** (`sort + dedup_by_key(line)`) so distinct same-line statements
  survive (deduped instead by `start_byte`).

**`VarLocation` (`src/data_flow.rs`)** gains `start_byte: usize`, `end_byte: usize`,
`function_start: usize`. The extraction loop already iterates real function AST nodes
(`for func_node in parsed.all_functions()`), so it records the function's `start_byte` as
`function_start` at the source — no extra parse.

**Indexes (`src/cpg/build.rs`)**:
- `var_index`: `(file, function, line, path, access)` → `(file, start_byte, access)` (an
  occurrence is unique by its start byte within a file; `access` disambiguates the
  augmented-assignment read+write at one anchor). `path`/`function_start` carried on the
  node, not the key.
- statement index: `(file, line)` → `(file, start_byte)`.
- `location_index: (file, line) → Vec<NodeIndex>` **stays** (powers `nodes_at(file,
  line)`); same-line order is recovered by sorting the bucket by node `start_byte`.
- `func_index`: re-keyed `(file, start_byte)` (above).

**Node creation (Steps 1-4):** Function/Variable/Statement nodes populate the new byte
fields from FunctionInfo (functions) / VarLocation (variables) / statement extraction
(statements).

## 5. Consumers (blast radius & migration)

- **`src/cpg/trace.rs`** — `same_line_same_path_uses` and same-line propagation switch
  from `NodeIndex`/name grouping to **`function_start` equality + `start_byte` ordering**
  (strictly more correct; the load-bearing F1 fix). `is_parameter_binding` /
  `function_starts_at` continue to work via `nodes_at` + the Function node's span.
- **~26 slicing algorithms** read `node.line()` — **untouched** (the `line` field stays).
  Purely additive for them.
- **`src/cpg/query.rs`** — `var_node` signature/key updated; new `function_of(var)`
  helper; `function_at`/`nodes_at` unchanged in shape. `to_var_location` carries the new
  fields.
- **`src/navigation/`** — `SymbolRef.ordinal` (currently hardcoded `0`) is **populated**
  from the same-line byte rank, giving nav real occurrence identity for free.
- **`src/cpg_cache.rs`** — `CACHE_VERSION` 4 → 5 (+ history line: "v5: byte-range identity
  on Variable/Statement/Function + span-keyed function identity (S2)"). `SerializedCpg`
  shape unchanged (it serializes `Vec<CpgNode>` directly); the bump invalidates v4 caches.
- **Plan B (not built):** S2's primitives let Plan B's plan delete **Slice 5** (ordering
  oracle → a `start_byte` comparison) and **Slice 3d** (function identity → `function_start`).
  S2 ships no Plan B code; it removes the *need* for those slices.

## 6. Error handling & edge cases

- Synthesized/normalized access paths → §3 anchor fallback (deterministic).
- Parse-degraded files → spans may be approximate but are still monotonic within the
  file; the existing parse-warning machinery covers quality. No new failure mode.
- Augmented assignment (`x += 1`) → one anchor, both a Def and a Use node; `access` in the
  var_index key separates them.
- Macro-expanded / generated code → spans point at the source occurrence as tree-sitter
  reports it (same as today's lines).

## 7. Testing & acceptance

1. **Per-language span extraction** (`tests/ast/`): each language's Variable span = full
   access-path extent for member/field/index exprs; the anchor fallback for synthesized
   paths; Statement spans; Function spans.
2. **Function identity / overload de-conflation** (`tests/ast/` or `tests/integration/`):
   two same-name functions in one file (`impl A { fn f }` / `impl B { fn f }`) →
   distinct `func_index` entries, distinct `Variable.function_start`; the Step-5b parity
   test is updated (the first-match workaround is gone).
3. **Same-line ordering** (`tests/ast/cpg_test.rs`): `x = y` on one line → use-of-y
   precedes def-of-x by `start_byte`; a multi-statement line keeps both statements;
   `same_line_same_path_uses` returns byte-ordered results.
4. **Determinism**: byte-keyed indexes produce identical graphs across runs.
5. **Cache**: v5 round-trips; a v4 cache invalidates (forces rebuild).
6. **No-regression**: full `cargo test` (default + `--features mcp`); Tier-A
   `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut`
   stays exit 0 (the resolver/matrix are unaffected; this guards the CPG rebuild).
7. **nav ordinal**: `SymbolRef.ordinal` reflects same-line byte rank (no longer 0).

## 8. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Span contract under-specified for a language's projections | §3 makes the rule explicit + per-language extraction tests (§7.1) with the fallback. |
| `Variable.function` name removal ripples wider than expected | Most uses are grouping/equality (→ `function_start`, strictly better); display uses get `function_of()`. Migrate consumer-by-consumer; full suite guards. |
| var_index re-key changes dedup semantics | `(file, start_byte, access)` is occurrence-unique; characterization tests on DFG edge sets before/after. |
| Cache bump churn | Single batched v5 bump (S1 made cold rebuilds cheap); v4 invalidation is the intended effect. |
| Determinism regression from byte keys | All new fields `usize`; BTreeMap ordering preserved; determinism test (§7.4). |

## 9. Out of scope / deferred

- **Plan B re-slimming** (deleting its Slices 3d/5) happens in Plan B's own re-plan, not
  here.
- **Sub-statement EOG / control-dependence edges** — explicitly NOT in S2 (substrate
  "recommend NOT doing"); byte-range identity is the 80/20 substitute for EOG.
- **Column/UTF-16 positions** for LSP-style consumers — byte offsets only; a column
  projection can be derived later if a consumer needs it (additive).
