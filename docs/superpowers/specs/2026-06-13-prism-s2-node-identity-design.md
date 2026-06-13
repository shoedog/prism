# S2 — Node-Identity Hardening — Design

**Date:** 2026-06-13 · **Status:** rev 2 — dual spec-review folded (codex gpt-5.5 xhigh
rigor + claude opus/max soundness via a2a-bridge; record:
`docs/prism-query-layer/s2-spec-review-2026-06-13.md`). rev 1 made the byte the primary
identity *key*, which the review showed breaks occurrence-uniqueness, the CFG, and DFG
edges; **rev 2 makes the byte additive** (owner-approved 2026-06-13). **Context:**
`docs/cpg-substrate-analysis-2026-06-10.md` (F1/F2, §3 S2, R1); S3 merged (`main` @
d972079).

## 0. Why now

CPG nodes are **line-granular** (`Variable { path, file, function: String, line, access }`,
`Statement { file, line, kind }`; function identity is the string `(file, name)` with
last-writer-wins). Two defects (substrate F1/F2): (1) **same-line def/use order is
unrecoverable** — `trace.rs::same_line_same_path_uses` sorts same-line buckets by
`NodeIndex` (insertion order, not source order); (2) **same-named functions in one file
conflate** (func_index overwrites; `Variable.function` is a bare name).

The next increment, **Plan B (taint_reaches)**, serializes a *witness* over these nodes;
once it ships, node identity is pinned by the witness wire + fixtures. S2 fixes identity
**before** Plan B so Plan B's plan can delete its ordering oracle (Slice 5) and
function-identity workaround (Slice 3d), and it fixes overload conflation for nav/all
tools now. Highest-leverage "before, not after" item (substrate §3/R1).

## 1. Decisions (owner-approved 2026-06-13; rev-2 dual-eval rationale)

| Decision | Choice | Why |
|---|---|---|
| Byte identity | **Byte range `[start_byte, end_byte)` ADDITIVE** on Variable/Statement/Function nodes — a non-key span field, NOT part of any dedup key | Dual eval (codex+opus) converged on byte-range over start_byte/ordinal: `end_byte` isn't recoverable from a serialized witness without source replay, so a later add is a wire break; ranges are the universal code-intel identity. **rev-2 correction:** byte as the *key* broke occurrence-uniqueness (B1), the CFG (B3), and DFG edges (B4); additive byte + sort-by-`start_byte` delivers the same-line ordering (the Slice-5 payoff) with none of that — verified against `same_line_same_path_uses`, which needs only order. |
| Same-line order | Recover by **sorting same-line buckets by `start_byte`** (replaces the `NodeIndex` sort) | The F1 fix; additive byte is sufficient. |
| Function identity | Key `func_index` by **`(file, start_line)`** (de-conflates same-name functions); `Variable`/`VarLocation`/`defs`/`uses` reference the function by **`function_start_line: usize`** (the node-id); `name` retained as a non-identity **display** field on nodes | `CallGraph::FunctionId` already carries `start_line` ⇒ **zero resolver change** (review B2; `start_byte` would have required threading into FunctionId/the resolver). Name kept per review M4: dropping it makes `function_of()` fallible for orphan variables and leaves `defs`/`uses` co-mingled. *(This is the review-driven refinement of the brainstorm "replace name with node-id" — identity is the node-id; name survives as display. Flag for owner spec-review if full removal is wanted.)* |
| Variable span | **Full access-path expression extent**, deterministic anchor fallback (§3) | `end_byte` only earns its keep if it spans the occurrence. |
| Occurrence node-splitting | **Deferred** — additive ordering suffices for all known consumers; priced into Plan B only if its witness must point at a specific repeated same-line occurrence | Review: node-splitting is the larger change (byte through `find_path_references_scoped`, edge reconstruction, CFG endpoints, reachability tests) — price separately. |
| Packaging | One increment, one `CACHE_VERSION` bump (v4→v5) | Batch the schema change. |

## 2. Data model (`src/cpg/types.rs`)

Additive byte ranges on all variants; Variable's function reference becomes the
de-conflating `function_start_line` (name retained as display):

```rust
Function  { name, file, start_line, end_line, start_byte, end_byte }   // +start_byte,+end_byte
Statement { file, line, kind,                 start_byte, end_byte }   // +start_byte,+end_byte
Variable  { path, file, line, access,         start_byte, end_byte,    // +start_byte,+end_byte
            function: String,            // RETAINED — display only, NOT identity
            function_start_line: usize } // +identity/grouping key for the containing function
```

- **Function identity = `(file, start_line)`.** `func_index: BTreeMap<(String,String),
  NodeIndex>` → `BTreeMap<(String, usize), NodeIndex>` (file, start_line). Distinct
  functions have distinct start lines ⇒ no last-writer-wins; the
  `step5b_param_binding_first_wins_parity` pin is retired. Resolver/`FunctionId`
  unchanged (it already exposes `start_line`).
- Byte fields are `usize`, **half-open `[start_byte, end_byte)`** (tree-sitter
  convention). Not present in any dedup key (see §4).
- Derives unchanged (Debug, Clone, PartialEq, Eq, Serialize, Deserialize).

## 3. Variable span contract (witness wire-shape rule)

`[start_byte, end_byte)` = the source extent of the **access-path expression** at that
occurrence. Per-pattern anchor table (the rule each language's extraction implements):

| Pattern | Range covers | Anchor (start_byte) |
|---|---|---|
| Single identifier `x` | the identifier token | the token |
| Member/field `a.b.c`, `dev->name` | the whole member expression | leftmost segment |
| Index `a[i].b` | the whole index/member expression | leftmost segment |
| Pointer deref `*p`, `(*p).f` | the deref expression | the `*`/leftmost |
| Alias-resolved path | the *occurrence* expression as written (not the alias target) | the occurrence's leftmost |
| Parameter (§6 M1) | the parameter token | the parameter token |
| Synthesized/normalized (no single node) | **fallback:** the anchor identifier's span | the base token |

The fallback must be deterministic; `start_byte ≤ end_byte`; MISSING/zero-width nodes
get `end_byte = start_byte` (empty range) with the existing parse-warning surfaced.

## 4. Data flow & indexes (`src/ast.rs`, `src/data_flow.rs`, `src/cpg/build.rs`)

**Keys stay line/path-addressable — byte is additive payload only:**
- `var_index`: `(file, function, line, path, access)` → **`(file, function_start_line,
  line, path, access)`** (swap name→start_line for de-conflation; NO byte in key →
  dissolves B1, and edge reconstruction still maps by these line-based fields → dissolves
  B4). Byte range carried on the node, not the key.
- statement index: **stays `(file, line) → Vec<NodeIndex>`** (line-addressable) so Step-8
  CFG wiring (`CfgEdge {file, from_line, to_line}` → `stmt_index.get((file, from_line))`)
  is intact (dissolves B3). The line-dedup in `statements_in_function` is **relaxed only
  to keep distinct statements** (keyed within the bucket); same-line statements are
  ordered by `start_byte`. CFG endpoints remain line-granular (sub-line CFG is out of
  scope, §9).
- `location_index: (file, line) → Vec` unchanged; same-line order = sort bucket by
  `start_byte`.
- `func_index`: `(file, start_line)` (§2).

**Extraction:** the byte-yielding helpers (`assignment_lvalue_paths_on_lines`,
`rvalue_identifier_paths_on_lines`, `statements_in_function`) return spans alongside the
line. Occurrences also originate from **parameter defs, raw/resolved aliases, and
call-argument text, and DFG edges from `find_path_references_scoped`** (review B4); to
avoid byte/line key drift, all locations flow through a shared
**`VarOccurrence { path, line, start_byte, end_byte, function_start_line, kind }`**, and
the four edge-location reconstruction sites in `data_flow.rs` carry `function_start_line`
+ line (no byte needed in the key, so reconstruction matches).

**`VarLocation`** gains `function_start_line: usize` (identity) plus `start_byte`/
`end_byte` and retains `function: String` as **display payload** (so the Variable node is
populated directly from VarLocation — no build-time name resolution, no orphan-resolution
fallibility). It derives `Ord`/is a `BTreeMap` key in `forward`/`backward` reachability —
so its `Ord`/`PartialOrd`/`Eq`/`Hash` are **hand-written to key only on
`(file, function_start_line, line, path, kind)`**, excluding `function` (name) and the
byte fields from identity (review M3). `defs`/`uses` re-key from function name to
`function_start_line` (review M4b). The Variable node copies all fields from VarLocation.

## 5. Consumers (blast radius)

- **`trace.rs`** — `same_line_same_path_uses` filters by `function_start_line` (was name;
  de-conflated) and **sorts by `start_byte`** (was `NodeIndex`; the F1 fix).
  `is_parameter_binding`/`function_starts_at` keep working via the retained `line` + the
  Function node.
- **Reachability consumers** (`taint`, `full_flow`, `left_flow`, `provenance_slice` via
  `forward_reachable`/`backward_reachable`) — unaffected because `VarLocation::Ord`
  excludes byte (§4); guarded by edge-set characterization fixtures (§7).
- **~26 slicing algorithms** read `node.line()` — **untouched** (line retained). The ~5
  that read `.function` keep working (name retained as display).
- **`src/cpg/query.rs`** — `var_node` key updated (function_start_line); `to_var_location`
  carries new fields; `function_at`/`nodes_at` shape unchanged.
- **`src/navigation/`** — `SymbolRef.ordinal` (hardcoded 0) populated from the same-line
  **byte rank within the emitted same-symbol set** (domain: nodes sharing
  `(file, line, path/name)`, ordered by `start_byte`; ties impossible since distinct
  occurrences have distinct start bytes) (review M5).
- **`src/cpg_cache.rs`** — `CACHE_VERSION` 4→5 (history: "v5: additive byte ranges on
  Variable/Statement/Function + `(file,start_line)` function identity (S2)"). **Parallel
  edit required:** `reconstruct_cpg` and `from_parts` (`cpg_cache.rs`) duplicate the
  index-build logic — they must adopt the new key types and byte-sort the rebuilt
  `location_index` buckets, or they won't compile (review M7). `SerializedCpg` shape
  (serializes `Vec<CpgNode>`) unchanged; v4 caches invalidate.
- **Call-site identity** (`CallSite`, nav `call_site_line`) — **out of scope** for S2
  (review M6); revisit if same-line repeated-call identity becomes a nav goal.
- **Plan B (not built):** S2's primitives let Plan B's plan delete **Slice 5** (ordering
  oracle → `start_byte` comparison) and **Slice 3d** (function identity →
  `function_start_line`). S2 ships no Plan B code.

## 6. Error handling & edge cases

- **M1 parameters:** DFG pins parameter defs to the function-start line and
  `is_parameter_binding` detects them by `line == function start`. Keep `line` = function
  start; set `start_byte` = the parameter token's byte (may be on a different physical
  line in a multi-line signature); add a parameter marker so a param node with an
  out-of-line byte in the function-start bucket is unambiguous. Multi-line-signature test.
- Synthesized/normalized paths → §3 anchor fallback (deterministic).
- **m2 parse-degraded invariants:** `start_byte ≤ end_byte`; both within file length;
  zero-width policy (`end=start`); existing parse warning emitted. Bytes stay monotonic
  within a file.
- Augmented assignment (`x += 1`) → one anchor, a Def and a Use node, separated by
  `access` in the key.

## 7. Testing & acceptance

1. **Per-language span extraction** (`tests/ast/`): the §3 anchor table per language,
   incl. the synthesized-path fallback, parameter spans (multi-line sig), Statement and
   Function spans; half-open invariant.
2. **Function de-conflation** (`tests/integration/`): two same-name functions in one file
   → distinct `func_index` entries + distinct `Variable.function_start_line`; no DFG-index
   co-mingling; the Step-5b parity test updated (workaround retired).
3. **Same-line ordering** (`tests/ast/cpg_test.rs`): `x = y` → use-of-y precedes def-of-x
   by `start_byte`; multi-statement line keeps both; `same_line_same_path_uses` returns
   byte-ordered results.
4. **Characterization fixtures (review m3 — the guard for B3/B4/M3):** before/after **DFG
   edge-set** and **CPG-node-dump** fixtures proving no edges drop and reachability is
   unchanged. Pulled into acceptance, not just risk mitigation.
5. **Determinism**: byte-additive graph identical across runs.
6. **Cache**: v5 round-trips (incl. `reconstruct_cpg`/`from_parts`); v4 invalidates.
7. **No-regression**: full `cargo test` (default + `--features mcp`); Tier-A
   `--matrix-only --allow-stale-sut` exit 0 (guards the CPG rebuild; resolver unaffected).
8. **nav ordinal**: reflects same-line byte rank (no longer 0).

## 8. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Edge drop / reachability change from the key swap (name→start_line) | `VarLocation::Ord` excludes byte; before/after DFG edge-set characterization fixtures (§7.4) are acceptance gates. |
| CFG edges dropped | statement index stays line-addressable (§4); CFG endpoints untouched; ControlFlow-edge count fixture. |
| `function_of()` fallible for orphan variables | Name retained as node display field ⇒ no resolution needed for display; `function_of()` `None` documented for the orphan case. |
| `reconstruct_cpg`/`from_parts` divergence | Named as required parallel edits (§5); v5 round-trip test (§7.6). |
| Span contract under-specified per language | §3 per-pattern anchor table + per-language tests (§7.1). |
| Determinism from new fields | usize, byte excluded from keys/Ord; determinism test (§7.5). |

## 9. Out of scope / deferred

- **Occurrence-level node-splitting** — deferred; priced into Plan B if its witness needs
  occurrence-pointing (additive ordering suffices for all known consumers).
- **Plan B re-slimming** (delete Slices 3d/5) — Plan B's own re-plan.
- **Sub-statement EOG / control-dependence edges** — NOT in S2 (substrate "recommend NOT
  doing"); byte ranges are the 80/20 EOG substitute.
- **Call-site span/ordinal** (§5 M6) and **column/UTF-16 projections** — additive later if
  a consumer needs them.
