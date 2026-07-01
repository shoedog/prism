# Design: Prism MCP server adapter (Plan 3c) — v5 (spec-review-converged)

**Status:** design (spec-review rounds 1–5 folded; CONVERGED — verdict "plan-ready"). **Initiative:**
Prism navigation layer, Tier-1 final plan. **Predecessors merged:** Plans 1, 2, 3a, 3b, 3b.5.
**Successor:** Tier-2 reasoning layer.

## 0. Provenance + spec-review disposition

Clean-room design (firewalled codex+claude) + owner decisions (spike rmcp → hand-rolled fallback; ship
`Evidence` experimental-v1) + MCP research. Provenance: `docs/archive/review-artifacts/prism-query-layer/provenance/plan3c/`.
**Round 1** (3 blockers/8 majors) → coherent subgraph extraction, field-nulling concise, type-enrichment
dropped, cargo `mcp` feature, `outputSchema`/schemars dropped, `SeedInput=Symbol|Loc`. **Round 2**
confirmed those hold and added precision fixes: the size cap bounds the **full `CallToolResult`** (§6.3);
concrete **input value-bounds** (§5/§7); **line-delimited** transport framing (§9); error-table
**lifecycle rows** (§7); `_meta` on the **result, namespaced** (§6.4); defined **path pipeline** (§10);
committed **`nav_*`** naming (§4); fixed **smoke seed** (§13). **Round 3** (no blockers, 7 spec-precision
majors): per-field path semantics (§10), binary-search shrink (§6.3), `_meta` cap-constant (§6.4),
deferred rmcp/tokio deps (§3/§15), per-method JSON-RPC schemas + initialize state machine (§9), lenient
env-cap parsing (§6.5), `x-stability` in tool `_meta` (§11). **Round 4** (no blockers, 3 majors, all
§6.3): the **two-phase shrink** (full-result probe → binary search `[1,n-1]` → defined terminal error)
in §6.3. **Round 5** (no blockers, 2 majors): authorized `WarningKind::ResultTruncated` (the single
serialization-additive nav touch) + composed `truncated = max_results_clipped || (n'<n)` (§6.3). Concrete
constants are collected in §6.5.

## 1. Goal & scope

A **thin, local, stdio MCP server** (`prism-mcp` binary, behind a cargo `mcp` feature) exposing the
existing whole-repo navigation queries as MCP **tools**, returning the existing `Evidence` JSON. Second
thin adapter beside the CLI; reuses `src/navigation` verbatim, adds **zero nav-logic** — with one
sanctioned, serialization-additive nav-type touch (`WarningKind::ResultTruncated`, §6.3) that leaves all
existing nav + diff-review goldens byte-identical. Build target:
**MCP spec 2025-11-25**, JSON-RPC 2.0 over **stdio**. **Out of scope:** remote/HTTP, OAuth; Tier-2
reasoning tools (design so they plug in); type-enrichment/resolved-imports/method precision; the nav
`Evidence v1.0` remodel (S5).

## 2. Architecture (in-process library binding)

`prism-mcp` holds **one warm `NavigationSession` per process** (whole-repo CPG + nav indexes, built once,
reused, backed by the Plan-3a on-disk cache). Each `tools/call` is a pure **in-process function call**
into `src/navigation` — not a CLI subprocess. Amortizes the index build, gives structured results, keeps
byte-parity through one serializer. Diff-review, the `prism` binary, CLI `nav` parsing/output, `NavArgs`
are **never touched** (Option C; `tests/cli/nav_compat_test.rs` guards it).

Verified spine: `src/lib.rs:44-48` exports `navigation`/`repo_loader`/`output`; only `[[bin]] prism`
exists; `Evidence`/`QueryError` are **`Serialize`-only** (`types.rs`) → JSON one-way (tests assert
`serde_json::Value`); `navigation::seed::resolve_fn` lowers seed→function (`location > symbol`,
`seed.rs:30`) — reused via the query fns. `render`/`render_err` are infallible
(`unwrap_or_else(|_|"{}")`, `navigation.rs:6,28`) → no serialize-failure error path.

### 2.1 Language choice — why Rust + (rmcp-or-hand-rolled), not Go/Python

The in-process warm-session binding forces Rust (the server must share the process with the Rust nav
library to reuse the once-built index as a function call). Go/Python would force a worse trade for a
local single-client tool: **(a) subprocess-per-call** re-loads/rebuilds the index every call (the "parse
once" anti-pattern); **(b) FFI** re-adds a build+marshaling boundary; **(c) Rust daemon + Go/Python
frontend over IPC** adds a process + bespoke protocol. The MCP surface we need is small (stdio +
`initialize`/`tools/list`/`tools/call`/`ping`), so rmcp's **Tier-2** status is low-risk and the
hand-rolled fallback covers it. Go/Python win only in the deferred remote case (§17).

## 3. Component / file boundaries (feature-gated)

```
src/bin/prism-mcp.rs   — [feature "mcp"] arg parse (repo, --no-cache/--cache-dir) → mcp::run(cfg)
src/mcp/{mod,registry,tools,input,output,session,error,transport}.rs   — [cfg(feature="mcp")]
```
`registry` = ToolDescriptor + insertion-ordered ToolRegistry; `tools` = the 6 descriptors (handlers own
parsing+shaping); `input` = per-tool structs + `SeedInput` + hand-authored `serde_json::Value` input
schemas + the **path normalizer** (§10); `output` = Evidence → `McpToolResult` (clip, coherent subgraph,
verbosity, cap, text+structured); `session` = `ServerConfig`/`SessionProvider`; `error` = the §7 mapping;
`transport` = stdio JSON-RPC behind a one-method `Transport` trait.

**Dependency confinement (MAJOR4 / round-3 M4):** the `mcp` feature gates the module and **adds NO new
deps at S0** — `[features] mcp = []`; **`mcp` is NOT in `default`**; `#[cfg(feature="mcp")] pub mod mcp;`
in `lib.rs`; `[[bin]] name="prism-mcp" required-features=["mcp"]`. The handlers + the hand-rolled
transport need only `serde_json` (already a dep), so **`tokio`/`rmcp` are not declared until S3**, and
**only if the spike accepts rmcp** (`mcp = ["dep:tokio","dep:rmcp"]`, both `optional=true`); if the
hand-rolled path wins, neither is ever added. No `schemars` (input schemas hand-authored; no
`outputSchema` — §6.1). CI/verify gains `cargo build --bin prism-mcp --features mcp` +
`cargo test --features mcp`.

## 4. Tool surface — 6 read-only tools, `nav_*` naming

Names are `nav_*` (underscore — maximally client-compatible; SEP-986 `[A-Za-z0-9_.-]`, 1–128 chars).
`tools/list` returns exactly **6** tools (no dotted variants/aliases).

| Tool | Existing fn | Input | Returns |
|---|---|---|---|
| `nav_nodes_at` | `queries::nodes_at` | `{file, line, verbosity?}` | `Evidence` (valid-empty ok) |
| `nav_callers` | `queries::callers` | `{seed, depth?, max_results?, verbosity?}` | `Evidence`/`QueryError` |
| `nav_callees` | `queries::callees` | `{seed, depth?, max_results?, verbosity?}` | `Evidence`/`QueryError` |
| `nav_ego_graph` | `queries::ego_graph` | `{seed, hops?, edges?, max_results?}` | `Evidence{graph}`/`QueryError` |
| `nav_module_deps` | `module_graph::module_deps` | `{file, max_results?, verbosity?}` | `Evidence` |
| `nav_repo_map` | `module_graph::repo_map` | `{max_results?}` | `Evidence{graph}` |

`edges` is an **enum-constrained** list of the 6 valid kinds (`Call|Return|DataFlow|Contains|ControlFlow|
FieldOf`, `queries.rs:333-360`); **default `[Call,Return,DataFlow,Contains]`** (CLI parity,
`main.rs:278-281`). A bad edge value fails as bad-args (§7), so `QueryError::UnknownEdge` is unreachable.
**Annotations on every tool:** `readOnlyHint=true`, `destructiveHint=false`, `idempotentHint=true`,
`openWorldHint=false`, `title`. **Descriptions** front-loaded with when/when-not, seed grammar, return
shape, a worked example.

## 5. Input model (+ bounds + forward-compat)

```rust
// src/mcp/input.rs — `SeedInput` (mcp::input; navigation::seed already exists)
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SeedInput {
    Symbol { name: String, file: Option<String> },  // → (Some(name), file, None)
    Loc    { file: String, line: usize },            // → (None, None, Some("file:line"))
}
```
Exactly two variants (no speculative reserved variants). `to_triple()` feeds the verbatim query fns; all
resolution errors come from `navigation::seed::resolve_fn`.

**Value bounds (BLOCKER3; enforced at the adapter boundary → bad-args `isError`, §7):**
- `line` ≥ 1 (1-indexed).  `depth` ∈ `[0, 5]` (default **1**; 0 = no expansion, matching nav's existing
  `depth=0` tests).  `hops` ∈ `[0, 5]` (default **1**).  `max_results` ∈ `[1, 1000]` (default **50**).
  `verbosity` ∈ `{concise(default), detailed}`.  `edges` ⊆ the 6 kinds (default the 4).
- **Omitted** optional fields → their defaults. **Missing required** (`file`/`line`/`seed`) or
  **out-of-range/wrong-type** → bad-args (§7). The `[0,5]`/`[1,1000]` ceilings also bound the §6 budget.

Per-tool input structs own their parsing inside the handler; the uniform handler signature
`Fn(&NavigationSession, &serde_json::Value) -> Result<McpToolResult, ToolError>` absorbs the
heterogeneity. *(Superseded by the implementation plan: handlers return `-> McpToolResult` directly —
bad-args/`QueryError`/over-cap become `is_error` results inside the handler via `error.rs` helpers — so
no separate error-wrapper stage is needed. The contract is identical; only the in-process signature
differs.)* **Forward-compat (additive):** Tier-2's `FocusSet` adds new `SeedInput` variants
(`SourceSink`/`Diff`) and new tools (`taint_reaches`/…) as **purely additive** registrations — no break
to the Tier-1 `symbol|loc` schema. No v1 code reserves them.

## 6. Output model + token budget (first-class)

### 6.1 Serialization (single chokepoint; no `outputSchema`)
- **One serializer:** `render(&shaped_ev,"json")` (`navigation.rs:26`). The `McpToolResult` carries
  **both** `content` (a text block = `render(&shaped_ev,"json")`, for content-reading clients) **and**
  `structuredContent` (the same shaped `Evidence` as a JSON object, for structured-reading clients) — no
  `outputSchema` (optional in MCP; deriving one needs `schemars` on ~11 nav types, breaking Option-C).
- **`verbosity`** (`concise` default | `detailed`) is a **shape-preserving field-nulling projection** of
  `Evidence` at the adapter boundary, applied before `render`. In v1 it gates exactly **one live field**:
  `concise → EvidenceItem.why = vec![]` (drops the reasons; the dominant verbose field). `snippet` is
  already `None` at every nav construction site (no `--snippets` path in `src/`), so nulling it is a
  forward-compat no-op. **Coverage:** item tools (`nodes_at`/`callers`/`callees`/`module_deps`). Graph
  tools (`ego_graph`/`repo_map`) carry nodes/edges with no `why`/`snippet` → `verbosity` is inert and not
  exposed on them.

### 6.2 Coherent subgraph extraction (graph tools — graph-coherence invariant)
`GraphEdge.from`/`to` are **positional indices** into `nodes` (`types.rs:105`). For `ego_graph`/
`repo_map`: `max_results` **bounds NODES**; the retained set is the **first `max_results` nodes in the
query's own deterministic emit order** (the adapter preserves and prefix-clips that order — `ego_graph`
breadth-first from the seed; `repo_map` sorted file order via `BTreeSet`/`BTreeMap`; the adapter does not
re-sort). Then **keep only edges whose BOTH endpoints are retained** and **renumber** `from`/`to` to the
new positions. Set `truncated=true` + a guided `Warning`. A handler test asserts every surviving edge's
endpoints are in-bounds.

### 6.3 Result-size guarantee (the cap bounds the FULL result — BLOCKER1/2)
The hard cap is measured against the **byte length of the fully-serialized `McpToolResult`** — the
`content` text **+** `structuredContent` + warnings + `_meta`, i.e. *everything returned*. The shaped
`Evidence` serializes twice in two forms: **`content` is pretty JSON** (`render` uses
`to_string_pretty`, `navigation.rs:28` — the dominant ~1.3–2× term) and **`structuredContent` is compact
JSON**. This duplication is **intentional** (content-reading and structured-reading clients both work)
and the cap measures the actual combined bytes. `MAX_RESULT_CHARS` defaults to **80_000** (≈ well under
the 25k-token host cap), configurable via `PRISM_MCP_MAX_RESULT_CHARS` (§6.5).

**Two layers of truncation, composed (round-5 M2).** The nav query returns ALL results (`total` items /
nodes); the adapter first applies `max_results` → `n = min(max_results, total)`, recording
`max_results_clipped = (n < total)`. §6.3 then operates on `n`. `shape` composes **both** layers so the
`max_results` signal is never lost:
```
shape(n') = Evidence with first n' items/nodes (§6.1/§6.2: graph re-clips edges+renumbers);
            truncated = max_results_clipped || (n' < n);
            if truncated: warnings ∪= ResultTruncated("showing {n'} of {total}; raise max_results or
                          narrow — e.g. lower depth/hops or a more specific seed");  _meta recomputed
let full = shape(n)                       // PHASE 1 candidate (n'=n: char-cap not yet applied)
1. if serialized_len(full) <= cap: return full        // PHASE 1 — untruncated-by-cap; covers n==0; KEEPS the
                                                       //   max_results truncated flag/warning if max_results_clipped
2. if n >= 1:                                          // PHASE 2 — full exceeds cap; every probe is char-truncated
     lo=1, hi=n-1, best=None                           //   (n'<n) → uniformly warned → strictly monotone
     while lo<=hi: mid=(lo+hi)/2; if serialized_len(shape(mid)) <= cap { best=mid; lo=mid+1 } else { hi=mid-1 }
     if best is Some(b): return shape(b)               // largest count that fits
3. return TERMINAL_OVER_CAP_ERROR                      // n==0 empty didn't fit (pathological), or even n'=1 over cap
```
The `n >= 1` guard (round-5 M3) avoids a `usize` underflow at `hi=n-1` when `n==0` (phase 1 returns for
`n==0` in any realizable run, since `shape(0)` is a few hundred bytes < FLOOR ≤ cap).

**Authorized nav-type touch (round-5 M1):** §6.3's warning contract requires a truncation
`WarningKind`, which `types.rs:79-87` lacks. Adding **`WarningKind::ResultTruncated`** is the **single
sanctioned `src/navigation` change** for Plan 3c — it is **serialization-additive** (no nav query or
diff-review path emits it; only the MCP adapter does), so all existing nav + diff-review goldens stay
byte-identical (Option-C preserved). This is the one explicit exception to the "zero nav-logic" framing.
- **Phase 1** returns the full untruncated result when it fits (so `n-1→n` non-monotonicity never bites)
  and handles `n==0` (a valid-empty `Evidence` success, §7) — it's just `shape(0)`, tiny, returned here.
- **TERMINAL_OVER_CAP_ERROR (round-4 M3):** `isError:true` with **one short text `content` block**
  ("result exceeds size cap even at 1 item; narrow the query — e.g. lower depth/hops or a more specific
  seed"), **no `structuredContent`**, `_meta` = `{prism/schema_version}` only. Its serialized length is
  **`< MAX_RESULT_CHARS_FLOOR` (4_000)** by construction; a §13 test asserts the terminal error is itself
  under the cap.
- Determinism: phase-2 monotonicity makes the search well-defined; `shape` rebuilds warnings/`_meta` each
  probe. **Tests assert only `serialized_len ≤ cap` ∧ (`truncated=true` in phase 2 / `false` in phase 1)**
  — not that `best` is maximal (the search guarantees it; a golden need not pin it).

### 6.4 `_meta` (on the result, namespaced — round-3 M3/M9)
`_meta` is **MCP `CallToolResult` metadata** (a sibling of `content`/`structuredContent`), NOT a field of
`Evidence` (which has none). Each result sets the two keys below — **except the §6.3
`TERMINAL_OVER_CAP_ERROR`, which sets only `prism/schema_version`** to stay under FLOOR (round-5 M5):
- `_meta["prism/schema_version"] = "0.1"` (our owned namespace; §11).
- `_meta["anthropic/maxResultSizeChars"] = MAX_RESULT_CHARS` — the **configured cap (a constant)**, which
  is exactly what the key name means. A Claude-Code-recognized host convention (per the MCP research:
  "server authors can set `_meta["anthropic/maxResultSizeChars"]`"). It is **not** the actual returned
  size — reporting the actual size would be self-referential (writing it changes the serialized length).
  The server-side cap (§6.3) is the real guarantee; this key is an advisory hint; `truncated`+the warning
  signal that clipping happened. (Round-4 M10: confirm the exact key spelling against current MCP /
  Claude-Code docs before S1 — a wrong/unknown key is silently ignored, so this is non-blocking.)

### 6.5 Constants (one place)
`DEPTH_MAX=5`, `HOPS_MAX=5`, `MAX_RESULTS_DEFAULT=50`, `MAX_RESULTS_CAP=1000`,
`MAX_RESULT_CHARS=80_000` (default), `MAX_RESULT_CHARS_FLOOR=4_000`, `SCHEMA_VERSION="0.1"`,
default edges `[Call,Return,DataFlow,Contains]`. **`PRISM_MCP_MAX_RESULT_CHARS` parsing (round-3 M6):**
parsed as a base-10 `usize`; **missing → 80_000**; unparseable / `< MAX_RESULT_CHARS_FLOOR` (4_000) →
**stderr warning + fall back to 80_000** (lenient — bad config never crashes startup); values ≥ the floor
are used as-is. No upper clamp (a large cap just relaxes the backstop).

## 7. Error model (SEP-1303) — request-failure mapping

`isError:true` tool-execution results (model SEES → self-corrects) for model-fixable failures; JSON-RPC
protocol errors (NOT shown to the model) only for true protocol/lifecycle faults.

| Failure | Channel | Shape |
|---|---|---|
| Bootstrap/session build fails (bad repo, index error) | **startup exit ≠ 0** + stderr diagnostic | (before serving any request) |
| Unparseable JSON | JSON-RPC `-32700` | protocol error |
| Malformed envelope / wrong `jsonrpc` / batch | `-32600` | protocol error |
| Missing/wrong-type `params` (per-method) | `-32602` | per the §9 per-method schema table |
| `initialize` protocolVersion/capability unacceptable | respond supported version+caps (§9) | client disconnects if it can't accept |
| Unknown **method** (e.g. `resources/read`, undeclared) | `-32601` | protocol error |
| Unknown **tool name** (`tools/call` `params.name` ∉ registry) | **`isError:true`** | "unknown tool 'x'; available […]" |
| `arguments` won't deserialize / unknown seed `kind` / bad `edges` / out-of-range `line`/`depth`/`hops`/`max_results` / missing required | **`isError:true`** (SEP-1303) | the validation detail + how to fix |
| `QueryError::AmbiguousSymbol` | **`isError:true`** | `render_err` + "ambiguous; candidates […]; specify `@file`" |
| `QueryError::{SymbolNotFound,LocationOutOfRange,UnsupportedFile}` | **`isError:true`** | `render_err` + actionable sentence |
| Valid-empty `Evidence` (zero items / `SkippedPath`) | **success** `isError:false` | the Evidence |
| Over-cap even at n=1 (§6.3) | **`isError:true`** | minimal "narrow the query" diagnostic |

`render_err` (`output/navigation.rs:5`) gives the `QueryError` JSON byte-parity with the CLI; the adapter
wraps it as `isError:true` content + one human-actionable sentence. Notifications never get a response.

## 8. Session lifecycle

```rust
pub struct ServerConfig { pub repo_root: PathBuf, pub cache: CacheMode }   // no type_enrichment in v1
pub enum CacheMode { Default, NoCache, Dir(PathBuf) }   // mirrors NavArgs (main.rs:432-438)
pub struct SessionProvider { session: Arc<NavigationSession> }
```
- **One repo per process, no per-request `repo` arg** (`Arc`-owned lifetime-free session `mod.rs:24-27`,
  per-canonical-root cache keying, whole-repo build). `repo_root` is the future `SessionPool` slot,
  **canonicalized at bootstrap**.
- `SessionProvider::bootstrap(cfg)` **duplicates `build_session`'s shape** (`main.rs:426-442`) for v1.
  Cache map: `NoCache`→`build`, `Default`→`build_cached`, `Dir(base)`→`build_cached_under`. A bootstrap
  failure exits non-zero with a stderr diagnostic before serving (§7).
- **No type enrichment in v1** (`type_db=None`, as nav already builds): `TypeDatabase::from_compile_commands`
  invokes `clang` via `std::process::Command` (`type_db.rs:198`) — a bootstrap subprocess + trust/timeout
  surface out of scope (§17). **This is an explicit owner decision superseding the clean-room brief's
  type-enrichment requirement** (`provenance/plan3c/01-cleanroom-brief.md`) — the brief required *accepting*
  the config; v1 defers the whole capability (round-3 BLOCKER3 / round-5 M4), not an unacknowledged gap.
- **Concurrency:** queries take `&NavigationSession`; v1 dispatches **single-threaded** (stdio = one
  client), shared-read, no lock → `Send+Sync` is not load-bearing for v1 (a future async transport
  confirms it; §9 S3 / §17).

## 9. Transport (stdio; spec 2025-11-25)

stdio JSON-RPC behind a one-method `Transport` trait (`transport.rs` is the only file differing between
SDK and hand-rolled). **Framing: line-delimited JSON** — one JSON-RPC message per line, UTF-8, no
embedded newlines (per the stdio transport spec). **Batches are rejected** (`-32600`; JSON-RPC batching
was removed in spec 2025-06-18). Requests carry `id` and get exactly one response; **notifications
(`notifications/*`) get no response**. **Shutdown on stdin EOF** (exit cleanly). **stdout carries
JSON-RPC ONLY; all diagnostics/logs go to stderr** — the MCP binary **never `println!`**; a smoke test
asserts stdout is pure JSON-RPC.

**Per-method request schema + error channel (round-3 M5):**

| Method | `params` | Missing/wrong-type → |
|---|---|---|
| `initialize` | required: `{protocolVersion, capabilities, clientInfo}` | `-32602` |
| `notifications/initialized` | none (notification) | ignored, no response |
| `tools/list` | optional (`cursor` ignored — we don't paginate) | n/a |
| `tools/call` | required: `name` (string), `arguments` (object, **optional → `{}`**) | missing/non-string `name` → `-32602`; bad `arguments` (the tool input) → **`isError:true`** (§7) |
| `ping` | none | respond `{}` |
| unknown method | — | `-32601` |

**Initialize state machine (round-3 M7):** accepted `protocolVersion` = **`"2025-11-25"`** (our build
target); if the client offers a different version, respond with `"2025-11-25"` and proceed — if the
client can't accept it, the client disconnects (spec behavior). The server **requires no client
capabilities** (it only calls tools); `roots`/`sampling`/`elicitation` are ignored if present. The
server declares only `{tools:{}}`. **Before `notifications/initialized`**, only `initialize` and `ping`
are allowed; any `tools/*` beforehand → `-32600` (wrong lifecycle). Receipt of `notifications/initialized`
**transitions to initialized regardless of its (param-less) body** (round-4 M6 — it carries no required
params; being lenient avoids a deadlock). **Param-leniency rule (round-4 M7):** only `initialize` and
`tools/call` strictly validate `params` (→ `-32602`/`isError` per the table); for `none`/`optional`
methods (`ping`, `tools/list`, notifications) unrecognized or wrong-type fields are **ignored**, never an
error.

**rmcp spike (S3) — accept `rmcp` iff:** (a) tokio stays confined under the `mcp` feature so
`cargo build --bin prism` doesn't compile it; (b) data-driven registration hosting the `ToolDescriptor`
table; (c) emits our raw `render(ev,"json")` string verbatim; (d) structured tool errors express
`isError:true` with our content; (e) Rust 2021; **(f) it drives handlers on a `current_thread` runtime,
OR `NavigationSession: Send+Sync` is confirmed** (the session transitively holds tree-sitter `Tree`s with
`!Sync` corners — the hand-rolled no-async fallback sidesteps this). **Else** hand-rolled framing +
`serde_json` (no async; inherits all determinism). rmcp is **Tier-2** → spike-and-fallback is prudent.

## 10. Security & hygiene (local stdio surface)

- **stdout hygiene** (§9), guarded by a smoke test.
- **No subprocess / no shell** (query-time and, since enrichment is dropped, bootstrap) → the dominant
  MCP CVE class is **structurally absent**.
- **Path input pipeline (round-2 M7 / round-3 M1):** **every** file-bearing field is first
  **lexically normalized** by the adapter (`input.rs`) to a clean repo-relative `/`-separated form —
  collapse `./` and duplicate separators, resolve interior `..` lexically (no disk access). Examples:
  `src//lib.rs`→`src/lib.rs`; `./src/lib.rs`→`src/lib.rs`; `src/../src/lib.rs`→`src/lib.rs`. The
  normalizer returns `NormalizedPath = RepoRelative(String) | EscapesRoot` (round-4 M8): an absolute path
  or a leading `..`/sequence that escapes the root yields **`EscapesRoot`**, which takes the un-indexed
  branch directly (it is **never** coerced into a plausible repo-relative suffix). After normalization
  the per-field semantics differ by query type (this divergence is **intentional**, not a bug):

  | Field | Normalized? | Lookup | Un-indexed / escaping path → |
  |---|---|---|---|
  | `nav_nodes_at.file` | yes | exact `repo.files.contains_key` | **success**, empty `items` + `SkippedPath` warning |
  | `nav_module_deps.file` | yes | exact `repo.files.contains_key` | **success**, empty `items` + `SkippedPath` warning |
  | `SeedInput::Symbol.file` (optional disambiguator) | yes | passed to `resolve_fn` as the file filter | `SymbolNotFound` → **`isError:true`** (the seed is a precondition the model fixes) |
  | `SeedInput::Loc.file` | yes | `resolve_fn` resolves `file:line` to a node | `LocationOutOfRange` → **`isError:true`** |

  **Why the split:** direct file tools (`nodes_at`/`module_deps`) are *exploratory* — "nothing here" is a
  valid empty answer (matches the §5 nav behavior). A **seed** file is a *resolution precondition* — a
  miss is a model-fixable failure surfaced as `isError` (SEP-1303). Neither path reads disk or traverses
  (loaders store normalized relative paths + skip symlinks, `repo_loader.rs:54-59,116`).
- **Untrusted output:** `Evidence` is structured (names/paths/lines); `snippet` off → no free-form code
  text. Results are data, never instructions.
- **Cancellation/timeout:** v1 has **none mid-query** — queries are bounded-fast over the warm index (the
  one slow op, the index build, is at **bootstrap, before** serving). `notifications/cancelled` for an
  in-flight call is accepted and ignored (the call completes). Real cancel/timeout is a future
  async-transport enhancement (§17).
- **Read-only by construction** — no tool mutates; annotations advertise it.

## 11. Evidence-contract stability — experimental-v1

Each tool sets `tool._meta["prism/x-stability"] = "experimental"` (annotations are a fixed set, so custom
markers live in `_meta`); each result carries `_meta["prism/schema_version"] = "0.1"` (§6.4).
The warts — `module_deps` `location.file` overload (`output/navigation.rs:52-55`) and the graph-vs-flat
dual shape — live in the **nav layer** and share `render`, so fixing them is out of 3c's adapter scope.
Experimental framing keeps a later coordinated nav `Evidence v1.0` a **non-breaking** event (S5).

## 12. Preserve diff-review + CLI (Option C)

Zero CPG-core edits; no change to `main.rs` parsing/output, `NavArgs`, or any diff-review path. New files
+ a feature-gated `[[bin]] prism-mcp` + `#[cfg(feature="mcp")] pub mod mcp;` only. **Guardrail (concrete,
not "byte-identical artifact"):** (1) **no `tokio`/`rmcp` compiled into the default `prism` build**
(`mcp` not in `default`); (2) `nav_compat_test.rs` + diff-review goldens unchanged; (3) the default
`cargo test` (no `--features mcp`) is unchanged. `Cargo.lock` lists `tokio`/`rmcp` **only after S3, and
only if the spike accepts rmcp** (round-4 M9 — at S0–S2, and under the hand-rolled outcome, no async deps
exist at all); when present they compile **only under `--features mcp`**.

## 13. Testing

- **In-process handler tests** (S1/S2): `handler(&session, json) -> McpToolResult`, asserting the
  `Evidence` JSON (`serde_json::Value`), the §7 mapping (incl. out-of-range bad-args and the n=1 over-cap
  terminal), the §6.2 in-bounds-edges invariant, and §6.3 truncation (small `max_results` → ≤N +
  `truncated:true` + guided warning).
- **Render byte-parity golden (precise):** for a fixture **below all clip/cap thresholds in `detailed`**,
  `content` bytes == `render(&full_ev,"json")`. (When shaped/truncated, `content` == `render` of the
  *same shaped Evidence the adapter built* — the invariant is "content mirrors the shaped Evidence," not
  "content == the unshaped query result.")
- **Protocol smoke** (S4, explicit `[[test]] mcp_smoke` behind `--features mcp`, NOT in coverage-matrix
  scanners): drive the in-memory/stdio transport (or MCP Inspector CLI in CI); assert `tools/list` → 6
  annotated tools and `tools/call nav_callees {seed:{kind:symbol,name:"run_slicing_inner",
  file:"src/algorithms/mod.rs"}}` → well-formed `Evidence` with cross-file items (a **file-qualified**,
  non-ambiguous seed — `name:"run"` alone is `AmbiguousSymbol` here); **stdout is pure JSON-RPC**. Dogfood.

## 14. Evaluation seam (later)

`Evidence` is structured/comparable → a later A/B harness (vs an agentic-search baseline) can measure
localization precision/recall + token cost. This server is the vehicle; golden prompts come with the tools.

## 15. Build order

1. **S0 — feature wiring + skeleton:** cargo `mcp` feature = `[]` (**no new deps** — handlers + hand-rolled
   transport use the existing `serde_json`); `mcp` not in `default`; gated `prism-mcp` shell + `pub mod
   mcp;`; `SessionProvider::bootstrap` reusing `build_session`'s shape; prove the §12 guardrail (default
   `prism` build unchanged) + `--features mcp` compiles. (`tokio`/`rmcp` are NOT added here.)
2. **S1 — registry + input(+path normalizer + bounds) + output(+cap/§6.5) + error(§7 table) +
   `nav_nodes_at`** (no `QueryError` path; first proof of Evidence JSON + annotations + token shaping + §7).
3. **S2 — remaining 5 tools.** Each: query delegation **plus** `max_results` clip, `truncated`+guided
   warning, `verbosity` field-nulling (item tools), (ego/repo_map) §6.2 coherent subgraph + in-bounds
   test, full §7 mapping, §6.3 cap. (Sized realistically — underlying fns take no `max_results`.)
4. **S3 — transport** (line-delimited framing; the §9 per-method schemas + initialize state machine;
   stdin-EOF shutdown; stdout-purity guard); `mcp::run`. **Run the rmcp spike here** — if accepted, add
   `tokio`/`rmcp` to the `mcp` feature (`optional=true`); else hand-rolled (`serde_json` only, no async).
   Either way the §12 guardrail (no async deps in the default `prism` build) holds.
5. **S4 — `mcp_smoke`** (in-memory/Inspector CLI) + dogfood + the detailed-mode render byte-parity golden.
6. **S5 (separate, out of scope) — nav `Evidence v1.0`:** explicit `target`/`direction`; drop `experimental`.

S0–S2 carry no async/network risk; the only uncertain step (S3) sits behind the `Transport` trait.

## 16. Risks

- **R1 rmcp pulls async/tokio** → confined behind `mcp`; §12 guardrail gate (spike).
- **R2 token-cap breaches** (`repo_map` 862 edges) → §6 (max_results bounds nodes, coherent subgraph,
  full-result hard char cap + deterministic shrink + terminal fallback, guided truncation). Tested S2/S4.
- **R3 graph-clip corruption** → §6.2 + in-bounds test.
- **R4 Evidence-shape leak** → deferred behind `experimental` (§11), S5.
- **R5 stdout pollution** → stderr-only; smoke asserts purity.
- **R6 dep leak into lib/review build** → cargo feature; §12 guardrail.
- **R7 touching `main.rs`/`NavArgs`** → strictly additive; nav-compat tests guard it.
- **R8 session `!Sync`** under a multi-threaded rmcp runtime → S3 criterion (f); single-thread fallback.

## 17. Tracked follow-ups (separate slices)

- **S5 nav `Evidence v1.0`** — typed dependency `target`/`direction`, graph/node-kind enums; drop `experimental`.
- **Type enrichment in MCP** — `--compile-commands` → whole-repo `TypeDatabase::from_compile_commands(path, None)` + the bootstrap-`clang` trust/timeout/stderr rules.
- **`navigation::session`** extraction; **`SessionPool`/multi-repo**; **concurrent/async transport** (real cancel+timeout; confirms session `Send+Sync`).
- **Tier-2 reasoning tools** (`taint_reaches`/`dataflow_between`/`impact_of_change`/`what_missing`) + `FocusSet` seed variants — register on this adapter; own initiative.
- **Remote transport** (Streamable HTTP + OAuth). **Eval harness** (§14).
