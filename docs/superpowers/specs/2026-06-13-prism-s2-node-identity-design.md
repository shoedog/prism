# S2 — Node-Identity Hardening — Design

**Date:** 2026-06-13 · **Status:** rev 4 — three dual spec-review rounds folded (codex
gpt-5.5 xhigh + claude opus/max). rev 1 made the byte the identity *key* (broke
uniqueness/CFG/edges → rev 2 additive); round 2 found the rev-2 function key
`(file,start_line)` *regresses* different-name-same-line and under-lists the func_index
migration → rev 3 keyed functions by **`(file, name, start_line)`**; **round 3
(architecture / tradeoff lens)** confirmed the core sound under both reviewers and
resolved the one open decision — the **witness wire**: byte *range* now goes on the public
node identity (`Location`/`SymbolRef`/`GraphNode` + `CallSite`), the occurrence-
discriminating *ordinal* is **reserved** (the part genuinely deferred), and the remaining
deferrals get named additive seams. Round 3 also surfaced the mechanical tightenings
folded below (named occurrence record, `function_candidates`, the `.function` audit
sites, the `Ord≡Eq` invariant, macro/multi-target failure modes). Records:
`docs/prism-query-layer/s2-spec-review-2026-06-13.md` (r1), `...-r2-2026-06-13.md` (r2),
`...-r3-2026-06-13.md` (r3). **Architecture confirmed sound by both lenses across all
three rounds; rev-4 changes are completeness + the wire scoping, not redesign.** Context:
substrate `cpg-substrate-analysis-2026-06-10.md` (F1/F2, §3 S2, R1); S3 merged (`main` @
d972079).

## 0. Why now

CPG nodes are **line-granular** (`Variable { path, file, function:String, line, access }`,
`Statement { file, line, kind }`; function identity = `(file, name)` last-writer-wins).
Two defects (substrate F1/F2): (1) **same-line def/use order is unrecoverable** —
`trace.rs::same_line_same_path_uses` and the assignment-propagation arm sort same-line
buckets by `NodeIndex` (insertion order); (2) **same-named functions in one file
conflate** (func_index overwrites). The next increment, **Plan B (taint_reaches)**,
*serializes* a witness over these nodes; once it ships, identity is pinned by the witness
wire + fixtures. S2 fixes identity **before** Plan B (so Plan B's plan drops its ordering
oracle, Slice 5, and function-identity workaround, Slice 3d) and fixes overload
conflation for nav/all tools now. Highest-leverage "before, not after" (substrate §3/R1).

Because Plan B serializes and pins this wire, **S2 sets the wire shape deliberately now**:
byte *range* on the public identity, populated from the per-occurrence nodes the CPG
already has. Later occurrence/call-site precision is then an **additive field-add** — the
house style (`Evidence.reasoning` is omit-when-absent / byte-compatible) — never a wire
break. The reserved `ordinal` slot keeps the discriminator's seat warm at zero cost.

## 1. Decisions (owner-approved; three-round dual-eval rationale)

| Decision | Choice | Rationale |
|---|---|---|
| Byte identity (node) | **Byte range `[start_byte, end_byte)` ADDITIVE** on Variable/Statement/Function — never a dedup key, never in any `Ord` | Dual eval: range > start_byte > ordinal (`end_byte` unrecoverable from a serialized witness without source replay; ranges are the universal code-intel identity). Byte-as-key (rev 1) broke occurrence-uniqueness/CFG/edges; additive byte + sort-by-`start_byte` delivers the same-line ordering payoff (verified against the `same_line_same_path_uses` + assignment-propagation consumers). |
| Function identity | **`func_index` keyed `(file, name, start_line)`** — keep name, *add* start_line; `Variable`/`VarLocation` carry `function: String` (display + part of identity) **and** `function_start_line: usize` | round-2 unlock: `(file,start_line)` alone *regressed* different-name-same-line (`fn a(){}fn b(){}` on one line) and dropped name from the key, breaking Step-9 dispatch + `callers_of`/`callees_of`/`function_node` which read name from the key. `(file,name,start_line)` fixes same-name-different-line (the target F2 case) with **zero resolver change** (`FunctionId` has file+name+start_line) and **no by-name-query breakage** (name stays in the key). |
| **Witness wire / public identity** (round-3) | **Byte RANGE added now to `Location`, `SymbolRef`, `GraphNode`, and `CallSite`; the occurrence `ordinal` stays `0` (reserved, not populated)** | The CPG is already **one node per occurrence** at line granularity (`var_index` = `(file,function,line,path,access)`), so the witness (`node_of` → `to_var_location`) reads each occurrence's *own* byte: the range is **meaningful now** — the exact span for the ~99% one-occurrence-per-line case — not vacuous. Vacuous-now is only the *ordinal*: its domain `(file,line,path,access)` collapses under the var_index dedup → always `0` until occurrence-splitting, so it is reserved, not populated from byte rank (opus r3). `CallSite` byte additionally **de-collapses same-line duplicate calls** (its `Ord` was line-only). Doing the *range* now vs deferring is ~pure low cost (no correctness risk — byte & line share one `to_var_location` source) and makes the wire byte-ready so Plan B never pays a wire break. |
| Single-line same-name overloads | **Deferred** (extension seam §9) | `impl A{fn f(){}}impl B{fn f(){}}` on one line have byte-identical `FunctionId`s (file,name,start_line,end_line all equal) — the resolver itself can't distinguish them. Fixing needs `start_byte` on `FunctionId` (a localized later change; the byte is already on the Function node). Not introduced by S2; rare. |
| Statements | **Stay line-keyed/line-deduped** (one Statement node per `(file,line)` in CFG); gain additive byte range for display only | CFG edges are `{file, from_line, to_line}` and stay line-granular by design (sub-line CFG out of scope, §9). rev 2's "keep both same-line statements as nodes" over-reached against the line-only CFG; dropped. Same-line *ordering* is a Variable concern (byte), not a Statement one. |
| Variable span | **Full access-path extent for AST-node-sourced occurrences**; line-collapsed uses get a best-effort line/path anchor (occurrence-precise deferred §9) | `end_byte` earns its place spanning real occurrences; uses reconstructed from `find_path_references_scoped` (line-only, deduped) can't carry an occurrence byte — honest scoping, not a contract you can't meet. The **wire** byte is therefore occurrence-precise for node-sourced occurrences and the best-effort line anchor for line-collapsed uses; `node_of` surfaces whatever the node carries (§5/§6). |
| Packaging | One increment, one `CACHE_VERSION` bump v4→v5 (covers `CpgNode` bytes, `VarLocation` bytes, **and** `CallSite` bytes) | Batch the schema change. |

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
- **Parallel additive change in `src/call_graph.rs`:** `CallSite` gains
  `start_byte`/`end_byte` (the call expression's extent) — §4/§5. Same `CACHE_VERSION`
  bump covers its serialized layout.

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

**Deterministic tie-breaker (round-3):** every byte-sort (the `location_index` bucket,
the trace same-line arms) is made **total** by breaking equal `start_byte` (zero-width /
MISSING nodes, or genuinely coincident anchors) on `end_byte`, then `access` (Def < Use),
then build-order `NodeIndex`. No byte-sort relies on `start_byte` alone.

**Wire projection:** the same byte the node carries is what the witness/nav surfaces
(§5) — occurrence-precise for node-sourced occurrences, best-effort anchor for
line-collapsed uses. The wire never claims more precision than the node holds (§6).

## 4. Data flow & indexes

**Shared occurrence record** — to stop byte/line key drift across the many location
sources (lvalues, rvalues, params, raw/resolved aliases, call-arg text, and edge targets
from `find_path_references_scoped`), the extraction helpers **return this named record,
not widened tuples** (round-3 codex #5 — parallel byte/line tuple fields invite drift):

```rust
struct VarOccurrence {
    file: String, function: String, function_start_line: usize,
    path: AccessPath, line: usize, start_byte: usize, end_byte: usize,
    kind: VarAccessKind,
}
```

Every extraction helper that today returns line-only is reworked to **return
`VarOccurrence`** (or its span-bearing subset), threading one record instead of
lockstep tuples:
- `assignment_lvalue_paths_on_lines` / `rvalue_identifier_paths_on_lines`: return
  occurrence records carrying `(path, line, start_byte, end_byte)` from the matched node.
- `statements_in_function`: returns a `StatementSpan { line, kind, start_byte, end_byte }`
  record (byte additive; **line-dedup retained** — one statement per line for CFG, §1).
- `find_path_references_scoped` keeps returning `BTreeSet<usize>` (lines) — it stays
  line-based; line-collapsed use nodes get the §3 best-effort anchor, NOT a per-occurrence
  byte (occurrence splitting deferred §9).
- parameter extraction grows a byte-bearing record variant (today `function_parameter_names`
  drops the nodes); `start_byte` = the parameter token.

**`VarLocation`** gains `function_start_line`, `start_byte`, `end_byte` and retains
`function` (name). Its `Ord`/`PartialOrd`/`Eq`/`Hash` are **all hand-written and must
agree** — they key on `(file, function, function_start_line, line, path, kind)` (name +
line de-conflate; **byte excluded** — not identity). `Eq` is hand-written over the *same*
tuple as `Ord`: deriving `Eq` (which would include the byte fields) while hand-writing
`Ord` (which excludes them) would make `a == b` disagree with `cmp(a,b) == Equal` and
silently corrupt every `BTreeMap`/`BTreeSet` keyed on `VarLocation`. The `Ord≡Eq≡Hash`
agreement is pinned by an invariant test (§7). `defs`/`uses` re-key from `function`(name)
to the composite `(function, function_start_line)` (review M4b). Set membership is
preserved; the iteration *order* of `forward`/`backward` results shifts only where it
already keyed on the name string — guarded by an order-sensitive fixture (§7).

**Indexes (`src/cpg/build.rs`)**:
- `var_index`: `(file, function, line, path, access)` → `(file, function,
  function_start_line, line, path, access)` (add the function line; **no byte** → edges
  still reconstruct by line-based keys).
- statement index + `location_index`: **unchanged keys** `(file, line) → …` (CFG intact);
  same-line *Variable* order = sort the `location_index` bucket by `start_byte` with the
  §3 tie-breaker.
- `func_index`: `(file, name, start_line)` (§2). A `name → Vec<NodeIndex>` secondary
  index is added so the by-name public queries (§5) keep working unchanged and so
  `function_candidates` can return overloads.

**`CallSite` (`src/call_graph.rs`)**: gains `start_byte`/`end_byte` (the call
expression's extent, taken from the call node at extraction). The hand-written `cmp_key`
(call_graph.rs:1176) appends `start_byte, end_byte` **after** `line`, so two same-line
calls to one callee stop collapsing (derived `Eq` already differs once the bytes differ);
populated wherever call sites are extracted.

## 5. Consumers (full blast radius)

- **`src/cpg/build.rs` assembler** (NOT the resolver): Step-1 func node byte-join +
  func_index re-key + secondary name index; Step-5/5b `caller_key`/`callee_key` use
  `(file, name, start_line)` from the resolved `FunctionId`; **Step-5b param source**
  matches on `(name, start_line)` not first-name (build.rs:344) — and the
  `step5b_param_binding_first_wins_parity` test is **kept and re-pointed** to assert the
  now-correct overload edge (it does NOT get retired); Step-6 Contains uses the composite;
  **Step-9 virtual dispatch** reads the method name from the key (still present) — works.
- **`src/cpg/trace.rs`** — `same_line_same_path_uses` (:379) filters by `(function,
  function_start_line)` and **sorts by `start_byte`** (§3 tie-break); the
  **assignment-propagation arm (:242) and `same_function_same_path_uses_any_line` (:342)
  also sort by `start_byte`** (the real `x = y` ordering path). **The general DataFlow
  neighbor sort `taint_neighbors` (:211) stays `NodeIndex`-sorted** — it orders arbitrary
  downstream uses across lines, not a same-line concern, and `NodeIndex` is
  build-deterministic; only the same-line/same-path arms switch to byte. **`node_file_fn`
  (:173) is the primary `.function`-identity edit** — it returns `(file, function)` for
  scope checks and becomes `(file, function, function_start_line)`. `is_parameter_binding`
  unchanged (line == function start; byte additive).
- **`src/cpg/query.rs`** — **add `function_candidates(file, name) -> Vec<NodeIndex>`**
  (via the name index) as the explicit overload-aware API (round-3 codex #6 — don't hide
  overload ambiguity); `function_node(file, name)` is documented to return the unique node,
  or the first by `start_line` when overloaded, for back-compat; `callers_of`/`callees_of`/
  `callers_of_in_file`/`call_reachable_functions` keep their name signatures (name in key /
  name index); `var_node` signature gains `function_start_line` (2 internal callers +
  tests); `to_var_location` populates the new byte fields (and thus feeds the wire).
- **`.function` audit rule:** every *scope/equality* use of the function string switches
  to the `(function, function_start_line)` identity; `.function` stays for *display/output*
  only. Named sites: **`node_file_fn` (trace.rs:173, primary)**, `cfg_queries.rs:237`,
  `taint.rs:4875`, plus the algorithms. Acceptance includes a grep-audit of
  `.function ==` / scope comparisons.
- **`src/navigation/` + witness wire (`src/reasoning/shape.rs`)** —
  **`node_of` (shape.rs:206) emits the node's byte on `Location` (`start_byte`/`end_byte`)
  and on `SymbolRef::Variable`; `ordinal` stays `0` — reserved, NOT populated from byte
  rank** (its domain `(file,line,path,access)` collapses under the var_index dedup →
  always 0 until occurrence-splitting; opus r3). `nodes_at` variable + enclosing-fn and
  ego `SymbolRef`s likewise carry byte. Callers/callees `SymbolRef` are built from a
  byte-less `FunctionId` → carry the **function node's** byte (joined via func_index), or a
  documented zero-byte sentinel where no node exists; their `ordinal` stays `0`. **`CallSite`
  byte surfaces on call evidence** — `Reason::Calls`/`CalledBy` may additively carry the
  call span.
- **`src/cpg_cache.rs`** — `CACHE_VERSION` 4→5. **Parallel edits:** `reconstruct_cpg`
  AND `from_parts` rebuild the indexes with the new key *types* (won't compile otherwise)
  and byte-sort rebuilt `location_index` buckets with the §3 tie-breaker. The serialized
  `DataFlowGraph` (defs/uses/edges/forward/backward contain `VarLocation`), `Vec<CpgNode>`,
  **and `Vec<CallSite>`** change byte layout → v4 invalidates. **`PartialHit` path:** cached
  CG/DFG bypass `reconstruct_cpg` and feed `build_incremental` → the new `VarLocation` key
  types must round-trip through merge + `rebuild_adjacency` (acceptance §7).
- **Plan B (not built):** S2's primitives let its plan delete Slice 5 (ordering →
  `start_byte` compare) and Slice 3d (function identity → the composite). **Surfaced for
  the Plan B re-plan (round-3 opus #2):** S2 *changes the same-line admission semantics* —
  Plan B's conservative *any-after-any* same-line ordering becomes S2's *byte-ordered*
  compare, so Plan B's same-line fixtures must be re-validated against byte order (its
  round-6 gate). The wire Plan B serializes is byte-bearing (range) with a reserved
  ordinal — so occurrence/call-site precision is later a populate-only field flip, not a
  wire break. No Plan B code in S2.

## 6. Failure modes (well-defined)

| Mode | Behavior |
|---|---|
| Parameter byte vs line | `line` = function-start (preserves `is_parameter_binding`); `start_byte` = the param token (may be a different physical line in a multi-line signature). Documented, tested. No new schema field/marker. |
| Orphan variable (no Function node — `build.rs:410` `if let Some` shows it occurs) | Display name comes from the retained `Variable.function` field (no `function_of()` lookup needed) → never panics/empties. `function_of()` is a convenience helper with a documented `None` for orphans. |
| Single-line same-name overloads | Conflate (FunctionId-identical); documented limitation, deferred (§9 seam). No worse than today. |
| Parse-degraded / MISSING node | `start_byte ≤ end_byte`, both within file length, zero-width = `end==start`; emits the existing `WarningKind::ParseQuality`; bytes remain valid bounds (no monotonicity guarantee claimed across degraded regions). §3 tie-break keeps sorts total under zero-width. |
| Augmented assignment `x += 1` | one anchor; **the LHS emits BOTH a Def and a Use node** (distinct `access`), separated by `access` in the var_index key + the §3 byte tie-breaker. The extraction rule is explicit: an augmented-assign target is read-then-written. |
| Multi-target lvalue / destructuring | per-target occurrence with its own span (§3). |
| Macro-generated def/call | tree-sitter parses the macro *invocation*, not its *expansion* (Rust `macro_rules!`, C `#define`); spans anchor at the macro-call/def tokens (coarse), remain valid file bounds. Documented limitation; no fabricated occurrence-precision. |
| Multi-target call resolution | S2 does **not** change call resolution (S3 owns it); multi-target (CHA/NameOnly) calls keep surfacing every target via per-callee evidence — the byte-bearing `CallSite`/wire must not silently collapse distinct targets. |
| Wire byte for line-collapsed use | the wire carries the §3 best-effort line anchor (NOT occurrence-precise) until occurrence-splitting (§9); documented so consumers don't over-trust sub-line precision there. |

## 7. Testing & acceptance

1. **Per-language span extraction** (`tests/ast/`): the §3 anchor table per language incl.
   fallback, multi-target per-target spans, parameter spans (multi-line signature),
   Statement + Function spans; half-open + `start≤end` invariants; zero-width tie-break.
2. **Function de-conflation** (`tests/integration/`): same-name-different-line → distinct
   func_index entries + distinct `Variable.function_start_line`; different-name-same-line
   stays distinct (the regression guard); `function_candidates` returns both overloads;
   the **`step5b_param_binding_first_wins_parity` test kept and re-pointed** to assert the
   overload arg→param edge now resolves.
3. **Same-line ordering** (`tests/ast/cpg_test.rs`): `x = y` use-of-y precedes def-of-x by
   `start_byte` through BOTH `same_line_same_path_uses` and the assignment-propagation arm.
4. **Edge-set with EXPECTED flips** (the right invariant, not "unchanged"): before/after
   normalized DFG/call/contains edge-set + CPG-node-dump fixtures; de-conflation *should*
   flip some previously-false reachability — those flips are enumerated and asserted
   (S3-style), regressions fail.
5. **Order-sensitive consumer**: a `provenance_slice` fixture (it selects origin by
   iteration order over `backward_reachable`) proving byte-output unchanged under the
   `VarLocation::Ord` change.
6. **`VarLocation` `Ord≡Eq≡Hash` invariant** (`tests/ast/`): over a generated set that
   includes byte-differing-but-key-equal pairs, assert `a == b ⟺ a.cmp(&b) == Equal` and
   that equal keys hash equal (byte excluded from all three). The corruption guard for the
   hand-written impls.
7. **Witness wire byte** (`tests/` reasoning/shape): `node_of` emits the occurrence byte
   on `Location`/`SymbolRef`; `ordinal` asserted `== 0` (reserved). **The merged
   `shape.rs` same-line witness tests are enumerated in the §4 expected-flip set** — their
   serialized output gains byte fields; listed, re-blessed, regressions fail.
8. **`CallSite` same-line de-collapse** (`tests/` call_graph): two same-line calls to one
   callee yield two `CallSite`s + two caller edges (was one); the (intended) count flip is
   enumerated.
9. **Cache**: v5 full round-trip (incl. `reconstruct_cpg`/`from_parts`, `CallSite` bytes)
   AND a **`PartialHit` incremental-rebuild** test (prune + merge + reassemble a v5 cached
   DFG); v4 invalidates.
10. **Determinism**; nav byte projection reflects the node's span; ordinal reserved-0.
11. **Repo Tier-A workflow** (CLAUDE.md): `cargo build --release` then
    `uv run tier-a --matrix-only --allow-stale-sut` (exit 0) then `--quick` before review;
    plus full `cargo test` (default + `--features mcp`).

## 8. Risks & mitigations

| Risk | Mitigation |
|---|---|
| `VarLocation::Ord`/key swap changes DFG edge sets or output order | Hand-written Ord excludes byte; §7.4 edge-set fixtures (with expected flips) + §7.5 order-sensitive provenance fixture are acceptance gates. |
| Hand-written `Ord`/`Eq`/`Hash` disagree (byte in one, not another) → BTreeMap corruption | All three hand-written over the identical tuple; §7.6 `Ord≡Eq≡Hash` invariant test is a gate. |
| func_index migration misses a name-from-key site (Step-9, callers/callees) | name retained in the key + secondary name index ⇒ by-name queries unchanged; §5 enumerates Step-1/5/5b/6/9; compile-time type change fails loud. |
| Step-5b overload mis-binding persists | param source matched on `(name,start_line)`; the parity test is kept + re-pointed (not retired) as the guard. |
| `FunctionId` `Ord/Eq` includes `end_line` while S2 CPG function identity is `(name,start_line)` | Deliberate: a function whose body length changes is a different `FunctionId` but the **same** CPG node — node identity must be stable under body edits. The Step-1 `FunctionId`→node join matches on `(name, start_line)`, ignoring `end_line`; documented so the asymmetry isn't read as a bug. |
| Equal `VarLocation` key but differing byte metadata → map insertion keeps one byte instance | Byte is display-only / non-identity; the kept instance is deterministic (BTreeMap build order) and the §3 tie-break makes byte-sorts total; no consumer keys on which instance won. |
| Wire now byte-bearing → consumers over-trust sub-line precision | §6 documents the precision boundary (occurrence-precise node-sourced vs best-effort line-collapsed); `ordinal` documented reserved-0; not occurrence-grade until §9. |
| `CallSite` de-collapse moves call counts / caller edges | §7.8 de-collapse test enumerates the intended flips; telemetry counts move by design (same-line dup calls were under-counted). |
| Cache partial-hit path diverges | §7.9 partial-hit incremental rebuild test in addition to round-trip. |
| Span contract under-specified per language | §3 per-pattern anchor table + per-language tests; lvalue bytes taken from the matched node (not text-parse). |
| Determinism from new fields | usize, byte out of all keys/Ord; §3 total tie-break; determinism test §7.10. |

## 9. Out of scope / deferred — with the extension seam each leaves open

Each deferral is designed so adding it later is **additive, not a refactor**:

- **Occurrence-level node-splitting** (distinct node per same-line repeated occurrence)
  **+ the occurrence `ordinal`.** *Seam:* bytes are already on every node and in
  `VarOccurrence`, and the wire already carries the reserved `ordinal` field (currently
  `0`). Splitting later = relax the `var_index` dedup to include `start_byte`, give
  `CfgEdge` node-id/byte endpoints, and **populate** the ordinal from byte rank — all
  additive (no node-shape, no wire-shape change). Priced into Plan B only if its witness
  must point at a specific repeated occurrence on one line.
- **Single-line same-name function identity.** *Seam:* `start_byte`/`end_byte` already on
  the Function node; adding `start_byte` to `CallGraph::FunctionId` later de-conflates
  these with a localized resolver change (the func_index key extends to include it). The
  byte being present now means no node-schema/cache change is forced then. (Cost note: the
  later `FunctionId` change touches call-graph maps, providers, nav, and cache because
  `FunctionId` is the serialized call-graph key — broad but mechanical, and **no more
  expensive after Plan B than before it**, since Plan B's wire carries the function node's
  byte additively, not a `FunctionId`.)
- **Call-site *ordinal*** (a same-line duplicate-call discriminator beyond the byte). The
  call-site **span is now in scope** (§4/§5 — `CallSite` byte de-collapses duplicates);
  only an ordinal on top rides with occurrence-splitting. *Seam:* mirror the reserved-
  ordinal pattern onto call evidence if ever needed.
- **Sub-statement EOG / control-dependence edges.** *Seam:* byte ranges are the 80/20 EOG
  substitute now; a later PDG-lite adds a new edge kind (additive, M4-quarantine pattern),
  no node change.
- **Column/UTF-16 projections** for LSP-style consumers. *Seam:* derivable from
  `start_byte` + `line_offsets` on demand; additive output field, no identity change.
