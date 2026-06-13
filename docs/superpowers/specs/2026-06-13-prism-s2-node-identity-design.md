# S2 — Node-Identity Hardening — Design

**Date:** 2026-06-13 · **Status:** rev 3 — two dual spec-review rounds folded (codex
gpt-5.5 xhigh + claude opus/max). rev 1 made the byte the identity *key* (broke
uniqueness/CFG/edges → rev 2 additive); round 2 found the rev-2 function key
`(file,start_line)` *regresses* different-name-same-line and under-lists the func_index
migration → rev 3 keys functions by **`(file, name, start_line)`** and tightens the
extraction contract, failure modes, acceptance, and deferral seams. Records:
`docs/prism-query-layer/s2-spec-review-2026-06-13.md` (round 1),
`...-r2-2026-06-13.md` (round 2). **Architecture confirmed sound by both lenses across
both rounds; the remaining changes are completeness, not redesign.** Context:
substrate `cpg-substrate-analysis-2026-06-10.md` (F1/F2, §3 S2, R1); S3 merged (`main` @
d972079).

## 0. Why now

CPG nodes are **line-granular** (`Variable { path, file, function:String, line, access }`,
`Statement { file, line, kind }`; function identity = `(file, name)` last-writer-wins).
Two defects (substrate F1/F2): (1) **same-line def/use order is unrecoverable** —
`trace.rs::same_line_same_path_uses` and the assignment-propagation arm sort same-line
buckets by `NodeIndex` (insertion order); (2) **same-named functions in one file
conflate** (func_index overwrites). The next increment, **Plan B (taint_reaches)**,
serializes a witness over these nodes; once it ships, identity is pinned by the witness
wire + fixtures. S2 fixes identity **before** Plan B (so Plan B's plan drops its ordering
oracle, Slice 5, and function-identity workaround, Slice 3d) and fixes overload
conflation for nav/all tools now. Highest-leverage "before, not after" (substrate §3/R1).

## 1. Decisions (owner-approved; two-round dual-eval rationale)

| Decision | Choice | Rationale |
|---|---|---|
| Byte identity | **Byte range `[start_byte, end_byte)` ADDITIVE** on Variable/Statement/Function — never a dedup key, never in any `Ord` | Dual eval: range > start_byte > ordinal (`end_byte` unrecoverable from a serialized witness without source replay; ranges are the universal code-intel identity). Byte-as-key (rev 1) broke occurrence-uniqueness/CFG/edges; additive byte + sort-by-`start_byte` delivers the same-line ordering payoff (verified against the `same_line_same_path_uses` + assignment-propagation consumers). |
| Function identity | **`func_index` keyed `(file, name, start_line)`** — keep name, *add* start_line; `Variable`/`VarLocation` carry `function: String` (display + part of identity) **and** `function_start_line: usize` | round-2 unlock: `(file,start_line)` alone *regressed* different-name-same-line (`fn a(){}fn b(){}` on one line) and dropped name from the key, breaking Step-9 dispatch + `callers_of`/`callees_of`/`function_node` which read name from the key. `(file,name,start_line)` fixes same-name-different-line (the target F2 case) with **zero resolver change** (`FunctionId` has file+name+start_line) and **no by-name-query breakage** (name stays in the key). |
| Single-line same-name overloads | **Deferred** (extension seam §9) | `impl A{fn f(){}}impl B{fn f(){}}` on one line have byte-identical `FunctionId`s (file,name,start_line,end_line all equal) — the resolver itself can't distinguish them. Fixing needs `start_byte` on `FunctionId` (a localized later change; the byte is already on the Function node). Not introduced by S2; rare. |
| Statements | **Stay line-keyed/line-deduped** (one Statement node per `(file,line)` in CFG); gain additive byte range for display only | CFG edges are `{file, from_line, to_line}` and stay line-granular by design (sub-line CFG out of scope, §9). rev 2's "keep both same-line statements as nodes" over-reached against the line-only CFG; dropped. Same-line *ordering* is a Variable concern (byte), not a Statement one. |
| Variable span | **Full access-path extent for AST-node-sourced occurrences**; line-collapsed uses get a best-effort line/path anchor (occurrence-precise deferred §9) | `end_byte` earns its place spanning real occurrences; uses reconstructed from `find_path_references_scoped` (line-only, deduped) can't carry an occurrence byte — honest scoping, not a contract you can't meet. |
| Packaging | One increment, one `CACHE_VERSION` bump v4→v5 | Batch the schema change. |

## 2. Data model (`src/cpg/types.rs`)

```rust
Function  { name, file, start_line, end_line, start_byte, end_byte }    // +start_byte,+end_byte (additive)
Statement { file, line, kind,                 start_byte, end_byte }    // +start_byte,+end_byte (additive; line-keyed)
Variable  { path, file, line, access,         start_byte, end_byte,     // +start_byte,+end_byte (additive)
            function: String,            // RETAINED: display + part of function identity
            function_start_line: usize } // +the de-conflating line of the containing function
```

- **Function identity within a file = `(name, start_line)`.** `func_index: BTreeMap<
  (String,String,usize), NodeIndex>` (file, name, start_line). Built at Step-1 from
  `cg.functions` (FunctionId has all three). Step-1 also joins each FunctionId to its
  `ParsedFile.functions()` `FunctionInfo` (matched by name+start_line) to populate the
  node's `start_byte`/`end_byte` (already present on FunctionInfo, ast.rs:52-53).
- Byte fields `usize`, **half-open `[start_byte, end_byte)`** (tree-sitter convention);
  MISSING/zero-width node → `end_byte == start_byte`.
- Derives unchanged (Debug, Clone, PartialEq, Eq, Serialize, Deserialize). A direct
  `CpgNode::Variable {..}` construction in tests gets the new fields; add a
  `Variable::occurrence(..)` constructor so non-builder callers (reasoning, tests) don't
  hand-thread bytes.

## 3. Span contract (witness wire-shape rule)

`[start_byte, end_byte)` of a Variable = the source extent of its access-path expression,
**for occurrences backed by a real AST node** (lvalues from `extract_assignment_lvalue_paths`,
rvalues from `collect_identifier_paths`, parameters, statements, functions). Per-pattern
anchor table:

| Pattern | Range | Anchor (start_byte) |
|---|---|---|
| identifier `x` | the token | the token |
| member/field `a.b.c`, `dev->name` | whole member expr (from the matched `lhs`/value **node**, not the parsed text) | leftmost segment |
| index `a[i].b` | whole index/member expr | leftmost |
| deref `*p`, `(*p).f` | the deref expr | leftmost |
| multi-target lvalue `a, b = …` / destructuring | **per-target** node span (each target its own occurrence) | each target's leftmost |
| parameter | the parameter token (may be on a different physical line than `line`=function-start) | the token |
| alias-resolved location | the **raw occurrence as written** (see below) | raw occurrence leftmost |
| synthesized/normalized (no node) | **fallback:** anchor identifier span | base token |
| line-collapsed use (from `find_path_references_scoped`) | **best-effort line/path anchor**, NOT an occurrence locator (occurrence-precise → §9) | the line's first occurrence |

**Alias raw/resolved rule:** when the DFG registers both a raw path and a resolved-alias
path from one source occurrence, both nodes carry the **same span** (the raw occurrence's
extent); `path` is the (raw or resolved) access path as today — the span localizes the
*source text*, the path carries the *semantics*. Invariants: `start_byte ≤ end_byte`;
both within file length.

## 4. Data flow & indexes

**Shared occurrence record** — to stop byte/line key drift across the many location
sources (lvalues, rvalues, params, raw/resolved aliases, call-arg text, and edge targets
from `find_path_references_scoped`):

```rust
struct VarOccurrence {
    file: String, function: String, function_start_line: usize,
    path: AccessPath, line: usize, start_byte: usize, end_byte: usize,
    kind: VarAccessKind,
}
```

Every extraction helper that today returns line-only is widened to emit byte spans from
the matched node (return types change in lockstep — implementers thread `VarOccurrence`,
not bare tuples):
- `assignment_lvalue_paths_on_lines` / `rvalue_identifier_paths_on_lines`:
  `(AccessPath, line)` → `(AccessPath, line, start_byte, end_byte)`.
- `statements_in_function`: `(line, kind)` → `(line, kind, start_byte, end_byte)` (byte
  additive; **line-dedup retained** — one statement per line for CFG, §1).
- `find_path_references_scoped` returns `BTreeSet<usize>` (lines) today; it stays
  line-based — line-collapsed use nodes get the §3 best-effort anchor, NOT a per-occurrence
  byte (occurrence splitting deferred §9).
- parameter extraction grows a byte-bearing variant (today `function_parameter_names`
  drops the nodes); `start_byte` = the parameter token.

**`VarLocation`** gains `function_start_line`, `start_byte`, `end_byte` and retains
`function` (name). Its `Ord`/`PartialOrd`/`Eq`/`Hash` are **hand-written** to key on
`(file, function, function_start_line, line, path, kind)` — name + line de-conflate;
**byte excluded** (not identity). `defs`/`uses` re-key from `function`(name) to the
composite `(function, function_start_line)` (review M4b). Set membership is preserved;
the iteration *order* of `forward`/`backward` results shifts only where it already keyed
on the name string — guarded by an order-sensitive fixture (§7).

**Indexes (`src/cpg/build.rs`)**:
- `var_index`: `(file, function, line, path, access)` → `(file, function,
  function_start_line, line, path, access)` (add the function line; **no byte** → edges
  still reconstruct by line-based keys).
- statement index + `location_index`: **unchanged** `(file, line) → …` (CFG intact);
  same-line *Variable* order = sort the `location_index` bucket by `start_byte`.
- `func_index`: `(file, name, start_line)` (§2). A `name → Vec<NodeIndex>` secondary
  index is added so the by-name public queries below keep working unchanged.

## 5. Consumers (full blast radius)

- **`src/cpg/build.rs` assembler** (NOT the resolver): Step-1 func node byte-join +
  func_index re-key + secondary name index; Step-5/5b `caller_key`/`callee_key` use
  `(file, name, start_line)` from the resolved `FunctionId`; **Step-5b param source**
  matches on `(name, start_line)` not first-name (build.rs:344) — and the
  `step5b_param_binding_first_wins_parity` test is **kept and re-pointed** to assert the
  now-correct overload edge (it does NOT get retired); Step-6 Contains uses the composite;
  **Step-9 virtual dispatch** reads the method name from the key (still present) — works.
- **`src/cpg/trace.rs`** — `same_line_same_path_uses` (:379) filters by `(function,
  function_start_line)` and **sorts by `start_byte`**; the **assignment-propagation arm
  (:242) and `same_function_same_path_uses_any_line` (:342) also sort by `start_byte`**
  (the real `x = y` ordering path). `is_parameter_binding` unchanged (line == function
  start; byte additive).
- **`src/cpg/query.rs`** — `function_node(file,name)` becomes "by-name, may return
  candidates" (documented; or via the name index); `callers_of`/`callees_of`/
  `callers_of_in_file`/`call_reachable_functions` keep their name signatures (name in key
  / name index); `var_node` signature gains `function_start_line` (2 internal callers +
  tests); `to_var_location` populates the new fields.
- **`.function` audit rule:** every *scope/equality* use of the function string (trace,
  cfg_queries, taint filtering, algorithms) switches to the `(function,
  function_start_line)` identity; `.function` stays for *display/output* only. The spec's
  acceptance includes a grep-audit of `.function ==`/scope comparisons.
- **`src/navigation/`** — `SymbolRef.ordinal` populated from same-line `start_byte` rank
  **only on node-sourced paths** (`nodes_at` variable/enclosing-fn, ego); domain = nodes
  sharing `(file, line, path, access)` ordered by `start_byte` (access in the domain
  resolves the `x += 1` Def/Use tie). Callers/callees `SymbolRef` are built from byte-less
  `FunctionId` → **keep `ordinal: 0`** (documented; not in the §7 ordinal gate).
- **`src/cpg_cache.rs`** — `CACHE_VERSION` 4→5. **Parallel edits:** `reconstruct_cpg`
  AND `from_parts` rebuild the indexes with the new key *types* (won't compile otherwise)
  and byte-sort rebuilt `location_index` buckets. The serialized `DataFlowGraph` (defs/
  uses/edges/forward/backward contain `VarLocation`) and `Vec<CpgNode>` change byte layout
  → v4 invalidates. **`PartialHit` path:** cached CG/DFG bypass `reconstruct_cpg` and feed
  `build_incremental` → the new `VarLocation` key types must round-trip through merge +
  `rebuild_adjacency` (acceptance §7).
- **Plan B (not built):** S2's primitives let its plan delete Slice 5 (ordering →
  `start_byte` compare) and Slice 3d (function identity → the composite). No Plan B code.

## 6. Failure modes (well-defined)

| Mode | Behavior |
|---|---|
| Parameter byte vs line | `line` = function-start (preserves `is_parameter_binding`); `start_byte` = the param token (may be a different physical line in a multi-line signature). Documented, tested. No new schema field/marker. |
| Orphan variable (no Function node — `build.rs:410` `if let Some` shows it occurs) | Display name comes from the retained `Variable.function` field (no `function_of()` lookup needed) → never panics/empties. `function_of()` is a convenience helper with a documented `None` for orphans. |
| Single-line same-name overloads | Conflate (FunctionId-identical); documented limitation, deferred (§9 seam). No worse than today. |
| Parse-degraded / MISSING node | `start_byte ≤ end_byte`, both within file length, zero-width = `end==start`; emits the existing `WarningKind::ParseQuality`; bytes remain valid bounds (no monotonicity guarantee claimed across degraded regions). |
| Augmented assignment `x += 1` | one anchor, distinct Def and Use nodes (separated by `access` in the var_index key + ordinal domain). |
| Multi-target lvalue / destructuring | per-target occurrence with its own span (§3). |

## 7. Testing & acceptance

1. **Per-language span extraction** (`tests/ast/`): the §3 anchor table per language incl.
   fallback, multi-target per-target spans, parameter spans (multi-line signature),
   Statement + Function spans; half-open + `start≤end` invariants.
2. **Function de-conflation** (`tests/integration/`): same-name-different-line → distinct
   func_index entries + distinct `Variable.function_start_line`; different-name-same-line
   stays distinct (the regression guard); the **`step5b_param_binding_first_wins_parity`
   test kept and re-pointed** to assert the overload arg→param edge now resolves.
3. **Same-line ordering** (`tests/ast/cpg_test.rs`): `x = y` use-of-y precedes def-of-x by
   `start_byte` through BOTH `same_line_same_path_uses` and the assignment-propagation arm.
4. **Edge-set with EXPECTED flips** (the right invariant, not "unchanged"): before/after
   normalized DFG/call/contains edge-set + CPG-node-dump fixtures; de-conflation *should*
   flip some previously-false reachability — those flips are enumerated and asserted
   (S3-style), regressions fail.
5. **Order-sensitive consumer**: a `provenance_slice` fixture (it selects origin by
   iteration order over `backward_reachable`) proving byte-output unchanged under the
   `VarLocation::Ord` change.
6. **Cache**: v5 full round-trip (incl. `reconstruct_cpg`/`from_parts`) AND a
   **`PartialHit` incremental-rebuild** test (prune + merge + reassemble a v5 cached DFG);
   v4 invalidates.
7. **Determinism**; **nav ordinal** on node-sourced paths reflects byte rank.
8. **Repo Tier-A workflow** (CLAUDE.md): `cargo build --release` then
   `uv run tier-a --matrix-only --allow-stale-sut` (exit 0) then `--quick` before review;
   plus full `cargo test` (default + `--features mcp`).

## 8. Risks & mitigations

| Risk | Mitigation |
|---|---|
| `VarLocation::Ord`/key swap changes DFG edge sets or output order | Hand-written Ord excludes byte; §7.4 edge-set fixtures (with expected flips) + §7.5 order-sensitive provenance fixture are acceptance gates. |
| func_index migration misses a name-from-key site (Step-9, callers/callees) | name retained in the key + secondary name index ⇒ by-name queries unchanged; §5 enumerates Step-1/5/5b/6/9; compile-time type change fails loud. |
| Step-5b overload mis-binding persists | param source matched on `(name,start_line)`; the parity test is kept + re-pointed (not retired) as the guard. |
| Cache partial-hit path diverges | §7.6 partial-hit incremental rebuild test in addition to round-trip. |
| Span contract under-specified per language | §3 per-pattern anchor table + per-language tests; lvalue bytes taken from the matched node (not text-parse). |
| Determinism from new fields | usize, byte out of all keys/Ord; determinism test §7.7. |

## 9. Out of scope / deferred — with the extension seam each leaves open

Each deferral is designed so adding it later is **additive, not a refactor**:

- **Occurrence-level node-splitting** (distinct node per same-line repeated occurrence).
  *Seam:* bytes are already on every node and in `VarOccurrence`; splitting later =
  relax the `var_index` dedup to include `start_byte` and give `CfgEdge` node-id/byte
  endpoints — both additive (no node-shape change). Priced into Plan B only if its witness
  must point at a specific repeated occurrence.
- **Single-line same-name function identity.** *Seam:* `start_byte`/`end_byte` already on
  the Function node; adding `start_byte` to `CallGraph::FunctionId` later de-conflates
  these with a localized resolver change (the func_index key extends to include it). The
  byte being present now means no node-schema/cache change is forced then.
- **Sub-statement EOG / control-dependence edges.** *Seam:* byte ranges are the 80/20 EOG
  substitute now; a later PDG-lite adds a new edge kind (additive, M4-quarantine pattern),
  no node change.
- **Call-site span/ordinal** (`CallSite` line-only). *Seam:* mirror the additive byte
  fields onto `CallSite`/nav call evidence later; same pattern as this increment.
- **Column/UTF-16 projections** for LSP-style consumers. *Seam:* derivable from
  `start_byte` + `line_offsets` on demand; additive output field, no identity change.
