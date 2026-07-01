Both designs are second-pass refinements that converged hard against the actual code. Let me adjudicate the one load-bearing claim only one architect made — the `navigation::seed` naming collision — since it drives a module-naming decision.Both unique claims verified: `navigation::seed` exists with `resolve_fn(symbol, file, location)` (precedence location>symbol, `seed.rs:30`), and every `Evidence`/`QueryError` type is `Serialize`-only — no `Deserialize` anywhere (`types.rs:3-128`). Both corrections from the STRUCTURE lens hold. Synthesizing.

---

# Merged Design — Plan 3c: MCP server adapter for Prism's navigation layer

## Approach (convergent spine)

Both architects, clean-room, landed on the **same architecture**: a new `prism-mcp` binary + an `src/mcp/` adapter layer that is *purely* request-parse → call existing `src/navigation` query → serialize existing `Evidence`. Zero nav-logic duplication. All semantics stay in `src/navigation/*`; the adapter only parses, dispatches, maps errors, and serializes. Every load-bearing claim below was verified against the tree by at least one architect, and I re-verified the two that mattered.

**Verified facts both designs rest on:**
- `src/lib.rs:44-48` already exports `navigation`, `repo_loader`, `output` → a second binary consumes the library cleanly; no module-tree duplication.
- Only `[[bin]] prism` is declared (`Cargo.toml`); no `rmcp`/`tokio`/`async` present → a second `[[bin]]` is additive and `cargo build --bin prism` stays byte-identical.
- CLI nav behavior is byte-pinned by `tests/cli/nav_compat_test.rs` → the adapter **must not touch** `main.rs` parsing/output paths or `NavArgs`.
- Query signatures are **heterogeneous** (see below) — there is **no universal seed**.
- `Evidence`/`QueryError` are `Serialize`-only (`types.rs`, re-verified) → MCP emits JSON one-way; tests assert against `serde_json::Value`, never typed deser.
- `navigation::seed::resolve_fn` (re-verified, `seed.rs:30`) already does seed→function lowering, precedence `location > symbol`. The adapter must **reuse it via the query fns**, never re-resolve.

## Component / file boundaries

```
src/bin/prism-mcp.rs   — arg parse → mcp::run(config)
src/mcp/mod.rs         — pub use; ServerConfig; mcp::run() (transport-agnostic entrypoint)
src/mcp/registry.rs    — ToolDescriptor + ToolRegistry (the extensibility seam, req #4a)
src/mcp/tools.rs       — the 6 Tier-1 tool descriptors (request JSON → query → Evidence)
src/mcp/input.rs       — per-tool input structs + shared SeedInput  (NOT "seed" — see decision D6)
src/mcp/session.rs     — SessionProvider + ServerConfig (lifecycle + enrichment knob)
src/mcp/error.rs       — ToolError + QueryError→MCP mapping
src/mcp/transport.rs   — stdio JSON-RPC behind a one-method Transport trait (rmcp OR hand-rolled)
```

`src/lib.rs` gains `pub mod mcp;`. `Cargo.toml` gains an explicit second `[[bin]]` (both architects: explicit, to match manifest style) and — only after the spike accepts rmcp — `tokio`/`rmcp` confined to that bin's target deps.

> **Merge note:** STRUCTURE proposed `registry.rs`/`input.rs`/`transport.rs`; EXECUTABILITY proposed `config.rs`/`schema.rs` with no explicit transport file. I take STRUCTURE's layout — it names the transport seam (the one file that changes between rmcp and hand-rolled) and the registry as first-class, both of which the requirements demand. `config.rs` folds into `session.rs` (`ServerConfig` lives with the provider that consumes it).

## Tool surface — **6 tools, heterogeneous inputs** (both architects independently corrected the draft "universal seed")

| Tool | Existing fn | Signature | Returns | Input shape |
|---|---|---|---|---|
| `nodes_at` | `queries::nodes_at` (`queries.rs:13`) | `(s, file, line)` | **plain `Evidence`** | `{file, line}` |
| `callers` | `queries::callers` (`queries.rs:155`) | `(s, symbol, file, location, depth)` | `Result<Evidence,QueryError>` | `{seed, depth}` |
| `callees` | `queries::callees` (`queries.rs:255`) | `(s, symbol, file, location, depth)` | `Result` | `{seed, depth}` |
| `ego_graph` | `queries::ego_graph` (`queries.rs:476`) | `(s, symbol, file, location, hops, edges)` | `Result` | `{seed, hops, edges}` |
| `module_deps` | `module_graph::module_deps` (`module_graph.rs:67`) | `(s, file)` | **plain `Evidence`** | `{file}` (not a seed) |
| `repo_map` | `module_graph::repo_map` (`module_graph.rs:174`) | `(s)` | **plain `Evidence`** | `{}` (no input) |

Only `callers`/`callees`/`ego_graph` consume the resolve-triple and take a `symbol|loc` seed. `nodes_at` is statement-level `(file,line)`; `module_deps` takes a bare file path; `repo_map` takes nothing.

**Defaults match the CLI** (both verified): `depth = 1` (`main.rs:250,264`), `hops = 1`, edges `Call,Return,DataFlow,Contains` (`main.rs:278-281`).

## Key interfaces / seams

### Seam 1 — Tool registry (extensibility, req #4a)
```rust
pub struct ToolDescriptor {
    pub name: &'static str,                 // "nav.callers", ...
    pub description: &'static str,          // carries the x-stability:experimental marker
    pub input_schema: serde_json::Value,    // per-tool; only resolve tools advertise seed variants
    pub handler: Box<dyn Fn(&NavigationSession, &serde_json::Value)
                          -> Result<Evidence, ToolError> + Send + Sync>,
}
pub struct ToolRegistry { /* deterministic order */ }
impl ToolRegistry { fn nav_v1() -> Self; fn register(..); fn get(name) -> Option<&ToolDescriptor>; fn list(); }
```
The uniform handler signature `(&NavigationSession, &Value) -> Result<Evidence, ToolError>` is the load-bearing seam: each handler owns its **own** input parsing, so the six queries' heterogeneity (depth vs hops+edges vs file vs nothing) is absorbed inside the closures and never leaks into the registry or transport. Infallible queries are lifted to `Ok(...)`; `Result`-returning queries pass `Err(QueryError)` straight through.

> **Divergence (registry ordering):** EXECUTABILITY used `BTreeMap<&str, ToolDescriptor>`; STRUCTURE used insertion-ordered `Vec`. **Pick:** either is deterministic; take `Vec` (insertion order) so the displayed `tools/list` order is authored intentionally rather than alphabetized. Trivial, owner can flip.

### Seam 2 — Input convention (req #4b) — **renamed to avoid collision (D6)**
```rust
// src/mcp/input.rs — name is `SeedInput`, module is `mcp::input`, because `navigation::seed` already exists.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeedInput {
    Symbol { name: String, file: Option<String> },  // → (Some(name), file, None)
    Loc    { file: String, line: usize },            // → (None, None, Some("{file}:{line}"))
    // Tier-2 reserved (Tier-1 handlers reject): SourceSink{..}, Diff{..}
}
impl SeedInput { pub fn to_triple(&self) -> (Option<String>, Option<String>, Option<String>); }

#[derive(Deserialize)] pub struct CallersInput   { pub seed: SeedInput, #[serde(default="d1")] pub depth: usize }
#[derive(Deserialize)] pub struct CalleesInput   { pub seed: SeedInput, #[serde(default="d1")] pub depth: usize }
#[derive(Deserialize)] pub struct EgoInput       { pub seed: SeedInput, #[serde(default="d1")] pub hops: usize,
                                                    #[serde(default="default_edges")] pub edges: Vec<String> }
#[derive(Deserialize)] pub struct NodesAtInput   { pub file: String, pub line: usize }
#[derive(Deserialize)] pub struct ModuleDepsInput{ pub file: String }
// repo_map: no input struct; handler ignores args
```
`to_triple()` produces exactly the `(symbol, file, location)` the existing query fns accept; those fns call `navigation::seed::resolve_fn` internally. **The adapter adds zero resolution logic** — `AmbiguousSymbol`/`SymbolNotFound`/`LocationOutOfRange` all arise from the verbatim resolver.

**Forward-compat (the defensible core, scoped honestly):** the three resolve tools advertise `seed: {kind: symbol|loc}`. When `FocusSet` lands, `source_sink`/`diff` become new `SeedInput` variants — the resolve-tool schemas extend without breaking Tier-1 signatures, and a Tier-1 handler fed a Tier-2 variant returns `ToolError::UnsupportedSeed` cleanly. `nodes_at`/`module_deps`/`repo_map` are explicitly **not** seed-shaped and are excluded from this story.

### Seam 3 — Session lifecycle (req #6)
```rust
pub struct ServerConfig {
    pub repo_root: PathBuf,
    pub cache: CacheMode,                 // mirrors NavArgs no_cache/cache_dir (main.rs:432-438)
    pub type_enrichment: Option<PathBuf>, // compile_commands.json
    pub evidence_contract: EvidenceContract, // ExperimentalV1
}
pub enum CacheMode { Default, NoCache, Dir(PathBuf) }
pub struct SessionProvider { session: Arc<NavigationSession> }
```
`SessionProvider::bootstrap(cfg)` reuses the **verified** `build_session` shape (`main.rs:426-442`): `load_repo` → set `type_db` if enrichment requested → build the index. Cache mapping (both architects agree, verified against `cache.rs`):
- `NoCache` → `NavigationIndex::build`
- `Default` → `NavigationIndex::build_cached`
- `Dir(base)` → `NavigationIndex::build_cached_under`
- **No "rebuild-and-save" mode** — `build_cached_at` is private; don't invent `--cache auto|rebuild|off`. Mirror the existing nav flags (`--no-cache`, `--cache-dir`).

**Decision — one repo per process, no per-request `repo` arg.** Grounded in: `Arc`-owned lifetime-free session (`mod.rs:24-27`), per-canonical-root cache keying (`cache.rs`), whole-repo build. `repo_root` is the future `SessionPool` slot.

**Concurrency:** queries take `&NavigationSession`; session holds `Arc<LoadedRepo>` + `Arc<NavigationIndex>` (plain owned graph + `BTreeMap` indices, no `Mutex`/`RefCell` observed). Start with a shared `Arc`, no lock. **Caveat (both flagged):** neither read the full `cpg.rs`/`call_resolve.rs` bodies — confirm thread-safety in S0 before enabling concurrent handlers; if a non-`Sync` type surfaces, fall back to single-threaded dispatch or an adapter-level mutex (do **not** add locks preemptively).

### Seam 4 — Type-enrichment knob (req #7)
Construction-time only, **no tool-schema change**. Plumbing verified: `LoadedRepo.type_db` is a public field (`repo_loader.rs:36`), hardwired `None` today (`:50`), and `NavigationIndex::build*` already threads `repo.type_db.as_ref()` with the cache keying on `has_type_db`. `bootstrap` sets `type_db` before building.
- **Unverified dependency (both flagged):** the exact `TypeDatabase`-from-`compile_commands.json` constructor symbol. Review-mode uses `TypeDatabase::from_compile_commands` (`main.rs:581-589`) but that path is diff-scoped — **do not copy its diff-file filter**; for whole-repo nav, pass the full/indexed file set (or `None` filter). **Confirm this symbol in S0.**

> **Divergence (when to wire enrichment):** EXECUTABILITY suggested omitting `--compile-commands` from the first slice (or accepting it with a "not wired yet" warning); STRUCTURE included it as a real knob. **Recommendation:** accept the flag in `ServerConfig` from S0 but treat actual wiring as gated on the S0 constructor confirmation — if confirmed cheaply, wire it; if not, accept-and-warn and defer to a fast-follow. Keeps the schema stable either way.

### Seam 5 — Error mapping (req #9)
```rust
pub enum ToolError { Query(QueryError), BadArguments(String), UnsupportedSeed(String) }
```
- Valid-empty `Evidence` (e.g. `SkippedPath` warning) → **SUCCESS** (`isError:false`); content = `render(&ev,"json")`.
- `QueryError` → tool error (`isError:true`), serialized via the **existing** `render_err` `{"error": <QueryError>}` (`output/navigation.rs:5`) → byte-identical to CLI; `AmbiguousSymbol.candidates` preserved for agent disambiguation. Emit-only — never parsed (consistent with the `Serialize`-only contract).
- `BadArguments`/`UnsupportedSeed` → JSON-RPC `-32602 Invalid params` (protocol-level, before `Evidence`).

### Seam 6 — Serialization (req #8)
Single serializer: `output::navigation::render(&ev,"json")` (`navigation.rs:26`). CLI/MCP byte-parity for free; one place that can break the agent contract, so it never forks.

**Determinism:** `BTreeMap`/`BTreeSet` throughout nav, `serde_json` over BTree-backed structs, insertion-ordered registry, no `HashMap`/clock/RNG in the adapter.

## Per-request flow
```
stdin frame → transport decode → { initialize | tools/list | tools/call }
  tools/list → registry.list() → [{name, description(+x-stability), input_schema}]   (session untouched)
  tools/call → registry.get(name)?            (-32601 unknown tool)
             → handler(provider.session(), &args)
                  parse the tool's OWN input struct (input.rs) → BadArguments on fail
                  [resolve tools only] SeedInput.to_triple(); UnsupportedSeed on Tier-2 variant
                  call the matching existing query (the ONLY nav touchpoint):
                     nodes_at / module_deps / repo_map → Evidence → Ok(_)
                     callers / callees / ego_graph     → Result<Evidence,QueryError> passthrough
             → Ok(Evidence)   → render(ev,"json") → success content
             → Err(ToolError) → error.rs → isError:true {"error":..} | -32602
  → transport encode → stdout
```

## Decisions + rationale
- **Registry over 5 hardwired handlers** — req #4a; new Tier-2 tools register without restructuring.
- **Per-tool input structs, shared `SeedInput` only across resolve tools** — the code's actual signatures forbid a universal seed; forward-compat retained exactly where it's true.
- **D6: input module is `mcp::input`, wire type `SeedInput`** — `navigation::seed` already exists (re-verified); reuse its `resolve_fn` via the query fns, don't shadow the name.
- **One repo per process, on-disk per-repo cache, no in-memory multi-session cache in v1** — matches the `Arc`-owned, per-root-keyed reality.
- **Single `render` serializer** — guarantees CLI/MCP byte-parity and a single contract chokepoint.
- **Evidence exposed as `experimental-v1`** (`x-stability` in each tool description + `schema_version:"0.x"`); do **not** remodel `module_deps` now.

### The three required explicit answers
- **rmcp-vs-stdio spike (#3):** stdio JSON-RPC either way, behind a one-method `Transport` trait (`transport.rs` is the only file that changes). **Accept rmcp iff:** (a) its tokio flavor confines to the `prism-mcp` target leaving `cargo build --bin prism` byte-identical; (b) data-driven (not macro-fixed) registration that can host the `ToolDescriptor` table; (c) emits our raw `render(ev,"json")` string verbatim, no key reordering; (d) structured tool errors so `QueryError` maps unflattened; (e) Rust 2021. **Else** hand-rolled framing + `serde_json` (small, inherits all determinism).
- **Session lifecycle (#6):** one `Arc<NavigationSession>` per process via `build_cached*`, shared read-only, no lock; repo is a server arg; on-disk per-repo cache only.
- **Evidence-contract stability (#5):** ship v1 as **experimental/unstable**. The real defects — `location.file` overload (target-file for call-edge items vs source-file for `UnresolvedImport`, see the explicit warning at `output/navigation.rs:52-55`) and the graph-vs-flat dual shape (`Evidence.graph` only for ego/repo-map, `types.rs:124`) — are genuine but live in the **nav layer** (`types.rs`/`module_graph.rs`) and share the CLI's `render`, so fixing them is **out of 3c's adapter-only scope**. Marking experimental keeps a later coordinated `Evidence` v1.0 (explicit `target`/`direction` modeling) a non-breaking event. Tracked as S5 follow-up.

## Risks
- **R1 — rmcp pulls async/tokio into the workspace.** Mitigated by target-confined deps + the byte-identical `prism` build gate in the spike accept criteria.
- **R2 — Evidence-shape leak** (`location.file` overload / dual graph-flat). Precisely located at `output/navigation.rs:52-55` and `types.rs:124`; deferred behind the `experimental` label.
- **R3 — concurrency assumption** — thread-safety asserted from index/session shape but full `cpg.rs`/`call_resolve.rs` not read. Verify in S0; single-thread fallback ready.
- **R4 — TypeDatabase constructor symbol unconfirmed** (G5). Confirm in S0; accept-and-warn fallback keeps schema stable.
- **R5 — input-surface heterogeneity tempts a fake universal seed.** Mitigation: per-tool structs; only the three resolve tools share `SeedInput`; the uniform handler signature absorbs the difference inside `input.rs`/`tools.rs`.
- **R6 — accidentally touching `main.rs`/`NavArgs`** breaks byte-pinned compat tests. Mitigation: adapter is strictly additive; CLI nav compat tests are the guardrail.

## Smallest shippable slices + build order
1. **S0** — skeleton: `prism-mcp` shell, `pub mod mcp;`, second `[[bin]]`, `SessionProvider::bootstrap` reusing `build_session`. **Confirm in S0:** (a) `NavigationSession` thread-safety (R3), (b) `TypeDatabase` constructor symbol (R4). Proves req #1 + #6.
2. **S1** — `registry.rs` + `input.rs` (`SeedInput` + `to_triple`) + `error.rs` + `tools.rs` with `nav.nodes_at` only (no `QueryError` path → cleanest first proof of `Evidence` JSON). In-process unit test `handler(session, json) → Evidence`.
3. **S2** — remaining 5 descriptors (`callers`/`callees`/`ego_graph`/`module_deps`/`repo_map`), each one-line-delegating; `Result`/plain-`Evidence` reconciliation; full `QueryError` mapping.
4. **S3** — `transport.rs` (`initialize` + `tools/list` + `tools/call`) per spike outcome; `mcp::run`.
5. **S4** — explicit `[[test]] mcp_smoke` (NOT in the coverage-matrix scanners): dogfood on this repo; assert `tools/list` → 6 tools and `tools/call nav.callers {seed:{kind:symbol,name:"run"}}` → well-formed `Evidence` JSON parsed as `serde_json::Value` (no typed deser — `Serialize`-only). Plus in-process golden: MCP bytes == `render(ev,"json")`.
6. **S5 (separate, out of scope)** — nav-layer `Evidence` v1.0: explicit `target`/`direction` on dependency items, drop `experimental`.

S0–S2 carry no async/network risk; the only uncertain step (S3) sits behind the one-method `Transport` trait.

---

## DECISIONS FOR THE OWNER
1. **rmcp vs hand-rolled stdio** — *Recommendation:* run the S3 spike with the 5 accept criteria above; default to hand-rolled if any fail (especially byte-identical `prism` build or verbatim-string emission). Reversible behind `transport.rs`.
2. **When to wire `compile_commands` enrichment** — *Recommendation:* accept the flag in `ServerConfig` from S0; actually wire it only if the `TypeDatabase` constructor is confirmed cheap in S0, else accept-and-warn and fast-follow. (Divergence: EXECUTABILITY leaned defer, STRUCTURE leaned include — this splits the difference without schema churn.)
3. **Session helper: extract `build_session` into a shared library fn, or duplicate its shape in `mcp::session`?** — *Recommendation:* duplicate the shape in `mcp::session` for v1 (keeps the byte-pinned CLI path untouched), extract into `navigation::session` as a follow-up once both consumers are proven. (Both architects agreed on this ordering.)
4. **Evidence v1 = experimental now, or fix `location.file`/dual-shape first?** — *Recommendation:* ship experimental now; the fix is a nav-layer change outside 3c's adapter scope (S5). Surface explicitly because it's the one contract call that's genuinely a judgment about how soon agents will depend on the shape.
5. **Registry ordering: `Vec` (insertion) vs `BTreeMap` (alphabetical)** — *Recommendation:* `Vec`, for intentional `tools/list` ordering. Trivial, both deterministic.

**Readiness verdict:** ready to plan after the owner settles decisions 1–4 (decision 5 is cosmetic), with S0's two confirmations (thread-safety + `TypeDatabase` constructor) as the first executable checkpoint.