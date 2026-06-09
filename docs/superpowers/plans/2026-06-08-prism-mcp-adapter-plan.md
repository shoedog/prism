# Prism MCP Adapter (Plan 3c) — Implementation Plan (v3, plan-review-hardened)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** A thin, local, **stdio** MCP server (`prism-mcp`, behind a cargo `mcp` feature) exposing the 6 whole-repo nav queries as read-only MCP tools returning the existing `Evidence` JSON.

**Architecture:** In-process library binding over `src/navigation` — one warm `NavigationSession` per process, each `tools/call` a pure function call. Additive (Option C): the `prism` binary, CLI, and diff-review are byte-for-byte unchanged; the **only** nav-layer touch is the additive `WarningKind::ResultTruncated`. Hand-rolled line-delimited JSON-RPC transport (no async/tokio); the rmcp spike is a deferred swap behind a `Transport` trait.

**Spec (authoritative — read it):** `docs/superpowers/specs/2026-06-08-prism-mcp-adapter-design.md` (v5, converged). When a step says "per spec §X", that section is the exact contract.

**Tech Stack:** Rust, `serde`/`serde_json`, `clap` (existing deps); **no new deps**; `assert_cmd`/`tempfile` already dev-deps.

**Handler contract (plan-review v3):** a tool handler is `Fn(&NavigationSession, &serde_json::Value) -> McpToolResult` — it **always returns an `McpToolResult`**; bad-args / `QueryError` / over-cap-terminal become `is_error:true` results **inside the handler** via `error.rs` helpers. (No separate "wrapper" task; protocol-level errors — parse/envelope/unknown-method — are handled in the transport, Task 7.)

**Module layout (created as compiling stubs in Task 1, filled task-by-task):** `src/mcp/{mod,session,registry,input,output,error,tools,transport}.rs` + `src/bin/prism-mcp.rs`. **Task 1 creates ALL files as compiling stubs** so the tree compiles green from the first commit; every later task **fills** its stub.

**Build/verify:** MCP behind `--features mcp`. Two configs: **default** (`cargo build`/`cargo test` — must stay byte-for-byte unchanged) and **`--features mcp`**.

---

## Task 1 (S0): Module skeleton (compiling stubs) + bootstrap + `WarningKind::ResultTruncated`

**Files:** Modify `Cargo.toml`, `src/lib.rs`, `src/navigation/types.rs`; create `src/mcp/{mod,session,registry,input,output,error,tools,transport}.rs` + `src/bin/prism-mcp.rs`. Test in `session.rs`.

- [ ] **Step 1:** `Cargo.toml`: `[features]\nmcp = []` (no `default` list exists; don't create one). `[[bin]] name="prism-mcp" path="src/bin/prism-mcp.rs" required-features=["mcp"]`.
- [ ] **Step 2:** `src/lib.rs`: `#[cfg(feature = "mcp")] pub mod mcp;`.
- [ ] **Step 3:** `src/navigation/types.rs`: add `ResultTruncated` to `WarningKind` (additive; emitted by no nav query — spec §6.3).
- [ ] **Step 4: Failing bootstrap test** in `src/mcp/session.rs`:
```rust
#[cfg(test)]
mod tests { use super::*;
  #[test] fn bootstrap_builds_a_queryable_session() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def helper():\n    return 1\n").unwrap();
    let cfg = ServerConfig { repo_root: dir.path().to_path_buf(), cache: CacheMode::NoCache };
    let p = SessionProvider::bootstrap(&cfg).expect("bootstrap");
    assert_eq!(crate::navigation::queries::nodes_at(p.session(), "a.py", 1).query, "nodes-at:a.py:1");
  } }
```
- [ ] **Step 5:** `cargo test --features mcp --lib mcp::session 2>&1 | head` → FAIL.
- [ ] **Step 6: Implement the skeleton.**
  - `session.rs` (REAL; verified APIs `NavigationIndex::build(&LoadedRepo)`, `build_cached(&LoadedRepo)`, `build_cached_under(&LoadedRepo, &Path)`; `Arc<LoadedRepo>` derefs to `&LoadedRepo`): as in v2 (canonicalize `repo_root`, `load_repo`, match `CacheMode` → index, wrap in `NavigationSession`).
  - `error.rs` stub: `#[derive(Debug)] pub enum ToolError { BadArguments(String), Query(crate::navigation::types::QueryError) }` (no `NotImplemented` needed — placeholders return a result, see registry).
  - `output.rs` stub: `pub struct McpToolResult { pub content_text: String, pub structured: Option<serde_json::Value>, pub is_error: bool, pub meta: serde_json::Map<String, serde_json::Value> }` + `impl McpToolResult { pub fn not_implemented() -> Self { /* is_error:true, content "tool not implemented" */ } }`.
  - `registry.rs` stub: `pub struct ToolRegistry; impl ToolRegistry { pub fn nav_v1() -> Self { Self } }`.
  - `input.rs` stub: empty. `tools.rs` stub: `pub fn register_all(_r: &mut super::registry::ToolRegistry) {}`.
  - `transport.rs` stub: `pub fn serve_stdio(_p: &super::SessionProvider, _r: &super::registry::ToolRegistry) -> anyhow::Result<()> { anyhow::bail!("transport: Task 7") }`.
  - `mod.rs`: `pub mod`-s + `pub use session::{ServerConfig, CacheMode, SessionProvider}` + the **exact** `run` signature (B1 — `?` requires a `Result` return):
```rust
pub fn run(cfg: ServerConfig) -> anyhow::Result<()> {
    let p = SessionProvider::bootstrap(&cfg)?;
    let r = registry::ToolRegistry::nav_v1();
    transport::serve_stdio(&p, &r)
}
```
  - `src/bin/prism-mcp.rs` (minimal `clap`, MINOR 14):
```rust
#[derive(clap::Parser)]
struct Cli { #[arg(long)] repo: std::path::PathBuf,
             #[arg(long, conflicts_with="cache_dir")] no_cache: bool,
             #[arg(long)] cache_dir: Option<std::path::PathBuf> }
fn main() -> anyhow::Result<()> {
    let c = <Cli as clap::Parser>::parse();
    let cache = if c.no_cache { prism::mcp::CacheMode::NoCache }
        else if let Some(d)=c.cache_dir { prism::mcp::CacheMode::Dir(d) } else { prism::mcp::CacheMode::Default };
    prism::mcp::run(prism::mcp::ServerConfig { repo_root: c.repo, cache })   // diagnostics via eprintln!/clap
}
```
- [ ] **Step 7:** `cargo build` (default — no tokio/rmcp); `cargo build --bin prism-mcp --features mcp`; `cargo test --features mcp --lib mcp::session 2>&1 | tail` → PASS; `cargo test --test cli_nav_compat` → goldens unchanged.
- [ ] **Step 8: Commit** — `feat(mcp): S0 module skeleton + session bootstrap + ResultTruncated (Plan 3c T1)`.

---

## Task 2 (S1a): Fill registry + annotations + the 6 input schemas + `tools/list` projection

**Files:** Fill `src/mcp/registry.rs`, `src/mcp/tools.rs` (placeholder descriptors). Test in `registry.rs`.

- [ ] **Step 1: Failing test:**
```rust
#[test] fn registry_lists_six_tools_with_annotations() {
  let r = ToolRegistry::nav_v1();
  assert_eq!(r.list().iter().map(|d| d.name).collect::<Vec<_>>(),
      ["nav_nodes_at","nav_callers","nav_callees","nav_ego_graph","nav_module_deps","nav_repo_map"]);
  let listed = r.get("nav_callers").unwrap().to_listed();
  assert_eq!(listed["annotations"]["readOnlyHint"], true);
  assert_eq!(listed["annotations"]["openWorldHint"], false);
  assert_eq!(listed["_meta"]["prism/x-stability"], "experimental");
  assert!(listed["inputSchema"]["properties"]["seed"].is_object()); // schema inlined, not empty
  // description acceptance (round-4 M3 — asserted BEFORE impl, in the failing test):
  for d in r.list() { let desc = &d.description;
    assert!(desc.contains("Example") && desc.contains("NOT"),  // when-NOT + worked example (§4)
      "tool {} description must front-load when/when-NOT + a worked Example: {desc}", d.name); }
}
```
- [ ] **Step 2:** Run, expect fail.
- [ ] **Step 3: Fill.** `ToolAnnotations` (M4 — snake_case Rust fields, camelCase JSON via serde rename):
```rust
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations { pub title: String,
    pub read_only_hint: bool, pub destructive_hint: bool, pub idempotent_hint: bool, pub open_world_hint: bool }
impl ToolAnnotations { pub fn read_only(title: &str) -> Self {
    Self { title: title.into(), read_only_hint: true, destructive_hint: false, idempotent_hint: true, open_world_hint: false } } }
``` `ToolDescriptor { name:&'static str, description:String, input_schema:serde_json::Value, annotations:ToolAnnotations, handler: Box<dyn Fn(&NavigationSession,&serde_json::Value)->McpToolResult + Send+Sync> }`. **`to_listed(&self)` BORROWS** (B1 fix):
```rust
pub fn to_listed(&self) -> serde_json::Value {
  serde_json::json!({ "name": self.name, "title": &self.annotations.title, "description": &self.description,
      "inputSchema": &self.input_schema, "annotations": &self.annotations,
      "_meta": {"prism/x-stability": "experimental"} })
}
```
`ToolRegistry { tools: Vec<ToolDescriptor> }` (insertion-ordered) with `nav_v1`/`register`/`get`/`list`.
In `tools.rs`, fill `register_all` to register all **6 descriptors** with real name/description/annotations + **inlined `input_schema`** (the MCP public contract — author per §4/§5; e.g.):
```rust
// the shared seed sub-schema (symbol|loc) reused by callers/callees/ego:
fn seed_schema() -> serde_json::Value { serde_json::json!({ "oneOf": [
  {"type":"object","properties":{"kind":{"const":"symbol"},"name":{"type":"string"},"file":{"type":"string"}},"required":["kind","name"]},
  {"type":"object","properties":{"kind":{"const":"loc"},"file":{"type":"string"},"line":{"type":"integer","minimum":1}},"required":["kind","file","line"]} ]}) }
// nav_callers: {"type":"object","properties":{"seed":seed_schema(),"depth":{"type":"integer","minimum":0,"maximum":5},
//   "max_results":{"type":"integer","minimum":1,"maximum":1000},"verbosity":{"enum":["concise","detailed"]}},"required":["seed"]}
// nav_nodes_at: {properties:{file:{type:string},line:{type:integer,minimum:1},verbosity:…},required:[file,line]}
// nav_ego_graph: {seed, hops:0..5, edges:{type:array,items:{enum:[Call,Return,DataFlow,Contains,ControlFlow,FieldOf]}}, max_results}
// nav_module_deps: {file, max_results, verbosity}   nav_repo_map: {max_results}
```
Placeholder `handler: Box::new(|_,_| McpToolResult::not_implemented())` (Tasks 5/6 replace bodies).
- [ ] **Step 4:** Run → PASS (the Step-1 description assertion forces each `description` to carry the four §4 elements — *what it does / when-and-when-NOT / seed grammar (or file/line) / return shape + a worked example with real values*; write them accordingly). **Checklist:** confirm `anthropic/maxResultSizeChars` spelling vs current Claude-Code docs before S1 ends.
- [ ] **Step 5: Commit** — `feat(mcp): S1 registry + annotations + inlined input schemas (Plan 3c T2)`.

---

## Task 3 (S1b): Fill input — `SeedInput`, per-tool structs, path normalizer (+ per-field `EscapesRoot`), bounds

**Files:** Fill `src/mcp/input.rs`. Test in `input.rs`. (Spec §5/§10/§6.5.)

- [ ] **Step 1: Failing tests** (normalizer + bounds; M9 representation):
```rust
use serde_json::json;
#[test] fn normalize_path_cases() { use NormalizedPath::*;
  assert!(matches!(normalize_path("src//lib.rs"),    RepoRelative(p) if p=="src/lib.rs"));
  assert!(matches!(normalize_path("./src/lib.rs"),   RepoRelative(p) if p=="src/lib.rs"));
  assert!(matches!(normalize_path("src/../src/x.rs"),RepoRelative(p) if p=="src/x.rs"));
  assert!(matches!(normalize_path("/abs/x"),         EscapesRoot));
  assert!(matches!(normalize_path("../escape"),      EscapesRoot)); }
#[test] fn bounds() {
  assert!(parse_callers(&json!({"seed":{"kind":"symbol","name":"f"},"depth":99})).is_err());
  assert!(parse_callers(&json!({"seed":{"kind":"symbol","name":"f"}})).is_ok());
  assert!(parse_nodes_at(&json!({"file":"a.rs","line":0})).is_err());
  assert!(parse_nodes_at(&json!({"file":"a.rs","line":1})).is_ok()); }
```
- [ ] **Step 2–4:** Run fail → implement: `SeedInput`(§5); `pub enum NormalizedPath { RepoRelative(String), EscapesRoot }` + lexical `normalize_path` (collapse `./`+`//`, resolve interior `..`; absolute/root-escaping → `EscapesRoot`, never coerced); per-tool `parse_*(&Value)->Result<TypedInput,ToolError>` (§5/§6.5 bounds; omitted→default; bad→`BadArguments`); `SeedInput::to_triple()`. **`EscapesRoot` representation (M9):** `parse_*` resolves a file field to a `String`; for `EscapesRoot` it returns a path **guaranteed not to be an indexed repo file** (use the original arg verbatim — an absolute/`..` path can never be a `contains_key` hit, and `resolve_fn` likewise misses). So an exploratory field (`nodes_at`/`module_deps`) → native empty+`SkippedPath`; a seed field → `resolve_fn` miss → `isError` (the §10 intentional divergence — handled by where the string flows, not by special-casing). → run PASS.
- [ ] **Step 5: Commit** — `feat(mcp): S1 input model + path normalizer + bounds (Plan 3c T3)`.

---

## Task 4 (S1c): Fill output — verbosity, subgraph, two-phase cap, `resolve_cap`, byte-parity golden

**Files:** Fill `src/mcp/output.rs`. Test in `output.rs`. Implement spec §6 exactly.

- [ ] **Step 1: Failing tests (compile-ready fixtures + explicit cap + `resolve_cap` + byte-parity):**
```rust
use crate::navigation::types::*;
fn item(n: usize) -> EvidenceItem { EvidenceItem{ symbol:Some(SymbolRef::Function{file:"a.rs".into(),name:format!("f{n}"),start_line:n,end_line:n,ordinal:0}),
  location:Location{file:"a.rs".into(),start_line:n,end_line:n}, score:1.0, source:Source::PrismCpg, fallback:false,
  why:vec![Reason::Calls{callee:format!("g{n}"),call_site_line:n,qualifier:None}], snippet:None } }
fn flat(n: usize) -> Evidence { Evidence{query:"callees:x@a.rs".into(), items:(0..n).map(item).collect(), truncated:false, warnings:vec![], graph:None} }
fn graph(nodes: usize) -> Evidence { Evidence{query:"repo-map".into(), items:vec![], truncated:false, warnings:vec![],
  graph:Some(GraphPayload{ nodes:(0..nodes).map(|i| GraphNode{symbol:None, location:Location{file:format!("f{i}.rs"),start_line:1,end_line:1}}).collect(),
                           edges:(0..nodes.saturating_sub(1)).map(|i| GraphEdge{from:i,to:i+1,kind:"ModuleDep".into()}).collect() })} }

#[test] fn full_under_cap_untruncated() { let r=shape_result(flat(2),2,false,Verbosity::Detailed,100_000);
  let v:serde_json::Value=serde_json::from_str(&r.content_text).unwrap(); assert_eq!(v["truncated"],false);
  assert!(!v["warnings"].as_array().unwrap().iter().any(|w| w["kind"]=="ResultTruncated"));
  assert_eq!(r.meta["prism/schema_version"], "0.1"); assert!(r.meta.contains_key("anthropic/maxResultSizeChars")); } // M12 positive _meta
#[test] fn phase1_max_results_clip_keeps_warning() { // M10 composed phase-1 truncation
  let r=shape_result(flat(50),500,true,Verbosity::Detailed,100_000); let v:serde_json::Value=serde_json::from_str(&r.content_text).unwrap();
  assert_eq!(v["truncated"],true); assert!(v["warnings"].as_array().unwrap().iter().any(|w| w["kind"]=="ResultTruncated")); }
#[test] fn over_cap_truncates_under_cap() { let r=shape_result(flat(200),200,false,Verbosity::Detailed,2_000);
  assert!(!r.is_error && serialized_len(&r)<=2_000); }
#[test] fn terminal_over_cap_iserror_under_floor() { let r=shape_result(flat(200),200,false,Verbosity::Detailed,300);
  assert!(r.is_error && r.structured.is_none() && serialized_len(&r)<4_000); }
#[test] fn graph_clip_edges_in_bounds() { let r=shape_result(graph(50),50,false,Verbosity::Detailed,1_500);
  let v:serde_json::Value=serde_json::from_str(&r.content_text).unwrap(); let n=v["graph"]["nodes"].as_array().unwrap().len();
  assert!(v["graph"]["edges"].as_array().unwrap().iter().all(|e| (e["from"].as_u64().unwrap() as usize)<n && (e["to"].as_u64().unwrap() as usize)<n)); assert_eq!(v["truncated"],true); }
#[test] fn concise_nulls_why() { let c:serde_json::Value=serde_json::from_str(&shape_result(flat(1),1,false,Verbosity::Concise,100_000).content_text).unwrap();
  assert!(c["items"][0]["why"].as_array().unwrap().is_empty()); }
#[test] fn detailed_content_is_render_byte_parity() { // M8 §13 golden
  let ev=flat(2); let r=shape_result(ev.clone(),2,false,Verbosity::Detailed,100_000);
  assert_eq!(r.content_text, crate::output::navigation::render(&ev,"json")); }
#[test] fn resolve_cap_branches() { // M7 §6.5 env parse
  assert_eq!(resolve_cap_from(None), 80_000);
  assert_eq!(resolve_cap_from(Some("bad")), 80_000);   // warn + default
  assert_eq!(resolve_cap_from(Some("100")), 80_000);   // < FLOOR(4000) -> default
  assert_eq!(resolve_cap_from(Some("50000")), 50_000); }
```
> **Shared result envelope (MAJOR2 — the cap measures what the transport sends):** add `impl McpToolResult { pub fn to_call_tool_result_value(&self) -> serde_json::Value }` producing the exact `tools/call` result JSON — `{"content":[{"type":"text","text":content_text}], "structuredContent":<structured?>, "isError":is_error, "_meta":meta}`. **`serialized_len(&self) = self.to_call_tool_result_value().to_string().len()`** (so §6.3 bounds the real returned bytes), and **Task 7's `tools/call` MUST emit `to_call_tool_result_value()` verbatim** (no re-assembly). The detailed-mode byte-parity golden still compares `content_text == render(&ev,"json")`.

- [ ] **Step 2–4:** Run fail → implement `Verbosity`, `to_call_tool_result_value`, `serialized_len` (over that value), `shape_result(ev,total,max_results_clipped,verbosity,cap)` per §6 (verbosity null `why`; §6.2 node prefix-clip→in-set edges→renumber; §6.3 two-phase: phase-1 full probe / phase-2 binary search `[1,n-1]` with `n≥1` guard / `TERMINAL` one-text-block no-structured `_meta`={schema_version} `<FLOOR`; `truncated = max_results_clipped||(n'<n)`; `ResultTruncated` warning carrying `total`); `_meta` per §6.4 (positive keys; terminal carve-out); `resolve_cap()`+`resolve_cap_from(Option<&str>)` (§6.5 lenient). `content_text = crate::output::navigation::render(&ev,"json")`; `structured = serde_json::to_value(&ev)`. → run PASS.
- [ ] **Step 5: Commit** — `feat(mcp): S1 output shaping + two-phase cap + resolve_cap + byte-parity (Plan 3c T4)`.

---

## Task 5 (S1d→e): Fill error helpers + `nav_nodes_at` handler

**Files:** Fill `src/mcp/error.rs` (→ `McpToolResult`), `tools.rs` (`nav_nodes_at` body + `test_support::session`). Test in `tools.rs`.

- [ ] **Step 1: Failing tests** (handler returns `McpToolResult`; bad-args + escaping-file divergence):
```rust
#[test] fn nodes_at_ok() { let s=test_support::session(&[("a.py","def f():\n    return 1\n")]);
  let out=(ToolRegistry::nav_v1().get("nav_nodes_at").unwrap().handler)(&s, &json!({"file":"a.py","line":1}));
  assert!(!out.is_error); let v:serde_json::Value=serde_json::from_str(&out.content_text).unwrap(); assert_eq!(v["query"],"nodes-at:a.py:1"); }
#[test] fn nodes_at_bad_line_iserror() { let s=test_support::session(&[("a.py","x=1\n")]);
  let out=(ToolRegistry::nav_v1().get("nav_nodes_at").unwrap().handler)(&s, &json!({"file":"a.py","line":0})); assert!(out.is_error); }
#[test] fn nodes_at_escaping_file_is_empty_skippedpath() { // M9 exploratory divergence
  let s=test_support::session(&[("a.py","def f():\n    return 1\n")]);
  let out=(ToolRegistry::nav_v1().get("nav_nodes_at").unwrap().handler)(&s, &json!({"file":"/etc/passwd","line":1}));
  assert!(!out.is_error); let v:serde_json::Value=serde_json::from_str(&out.content_text).unwrap();
  assert!(v["items"].as_array().unwrap().is_empty()); assert!(v["warnings"].as_array().unwrap().iter().any(|w| w["kind"]=="SkippedPath")); }
```
- [ ] **Step 2–4:** Run fail → fill `error.rs`: `impl ToolError { pub fn into_result(self)->McpToolResult }` + `pub fn query_error_result(QueryError)->McpToolResult` (build the `is_error` `McpToolResult` from **`render_err(&qe, "json").0`** — note `render_err` returns `(String, i32)`, MINOR 6 — plus one actionable sentence, §7). Add the test helper as **`#[cfg(test)] pub(crate) mod test_support { pub(crate) fn session(files: &[(&str,&str)]) -> NavigationSession {…} }`** in `tools.rs` (MAJOR 3 — `transport.rs` tests reuse it, so it must be `pub(crate)`; bootstrap a temp-repo session). Replace `nav_nodes_at` placeholder: `match parse_nodes_at(args){Ok(i)=>…, Err(e)=>return e.into_result()}` → `queries::nodes_at` → `shape_result(ev, total, false, verbosity, resolve_cap())`. → run PASS.
- [ ] **Step 5: Commit** — `feat(mcp): S1 error helpers + nav_nodes_at handler (Plan 3c T5)`.

---

## Task 6 (S2): Fill the other 5 handlers (compile-ready tests)

**Files:** `src/mcp/tools.rs` (5 handler bodies). Test in `tools.rs`.

**Handler recipe (pin these — round-4 M1/M2):** `parse_*` → call the query → (seed tools) map `QueryError` via `query_error_result` (§7) → **apply `max_results` in the handler before `shape_result`:** `let total = ev.items.len()` (flat) or `ev.graph.nodes.len()` (graph) = the native count; clip `ev` to `n = min(max_results, total)`; `let max_results_clipped = n < total;` → `shape_result(ev, total, max_results_clipped, verbosity, resolve_cap())` (§6.3). **Ego arg mapping:** `queries::ego_graph(s, symbol, file, location, hops, &edges)` takes the **three `Option<&str>` from `seed.to_triple()`** plus `hops` + the `edges` slice (it is the one query whose signature isn't a clean `(session, parsed)` — thread `to_triple()` explicitly). `module_deps`/`repo_map` take `(s, file)`/`(s)`.

- [ ] **Step 1: Failing tests (compile-ready, file-qualified seeds — M11):**
```rust
#[test] fn callees_scoped_seed() { let s=test_support::session(&[("util.py","def helper():\n    return 1\n"),
    ("main.py","from util import helper\n\ndef run():\n    return helper()\n")]);
  let out=(ToolRegistry::nav_v1().get("nav_callees").unwrap().handler)(&s, &json!({"seed":{"kind":"symbol","name":"run","file":"main.py"}}));
  let v:serde_json::Value=serde_json::from_str(&out.content_text).unwrap();
  assert!(v["items"].as_array().unwrap().iter().any(|i| i["symbol"]["Function"]["name"]=="helper")); }
#[test] fn callers_ambiguous_seed_iserror() { let s=test_support::session(&[("a.py","def run():\n    return 1\n"),("b.py","def run():\n    return 2\n")]);
  let out=(ToolRegistry::nav_v1().get("nav_callers").unwrap().handler)(&s, &json!({"seed":{"kind":"symbol","name":"run"}})); assert!(out.is_error); }
#[test] fn ego_escaping_seed_iserror() { // M9 seed divergence
  let s=test_support::session(&[("a.py","def f():\n    return 1\n")]);
  let out=(ToolRegistry::nav_v1().get("nav_ego_graph").unwrap().handler)(&s, &json!({"seed":{"kind":"loc","file":"/etc/passwd","line":1}})); assert!(out.is_error); }
#[test] fn repo_map_graph_in_bounds() { let s=test_support::session(&[("util.py","def helper():\n    return 1\n"),
    ("main.py","from util import helper\n\ndef run():\n    return helper()\n")]);
  let out=(ToolRegistry::nav_v1().get("nav_repo_map").unwrap().handler)(&s, &json!({"max_results":1}));
  let v:serde_json::Value=serde_json::from_str(&out.content_text).unwrap(); let n=v["graph"]["nodes"].as_array().unwrap().len();
  assert!(v["graph"]["edges"].as_array().unwrap().iter().all(|e| (e["from"].as_u64().unwrap() as usize)<n && (e["to"].as_u64().unwrap() as usize)<n)); }
#[test] fn module_deps_lists_targets() { let s=test_support::session(&[("util.py","def helper():\n    return 1\n"),
    ("main.py","from util import helper\n\ndef run():\n    return helper()\n")]);
  let out=(ToolRegistry::nav_v1().get("nav_module_deps").unwrap().handler)(&s, &json!({"file":"main.py"}));
  let v:serde_json::Value=serde_json::from_str(&out.content_text).unwrap(); assert!(v["items"].as_array().unwrap().iter().any(|i| i["location"]["file"]=="util.py")); }
```
- [ ] **Step 2–4:** Run fail → fill the 5 handlers → run PASS (existing nav tests unaffected).
- [ ] **Step 5: Commit** — `feat(mcp): S2 callers/callees/ego/module-deps/repo-map handlers (Plan 3c T6)`.

---

## Task 7 (S3): Fill transport — `Transport` trait + stdio + lifecycle + full protocol-error coverage

**Files:** Fill `src/mcp/transport.rs`. Test in `transport.rs`. Implement spec §9 exactly.

```rust
pub trait Transport { fn read_message(&mut self)->anyhow::Result<Option<String>>;  // None=EOF
                      fn write_message(&mut self,v:&serde_json::Value)->anyhow::Result<()>; }
// serve_session(&NavigationSession,&ToolRegistry,&mut impl Transport)->anyhow::Result<()> drives the lifecycle.
// serve_stdio(provider,registry) wraps a StdioTransport over stdin/stdout and calls serve_session(provider.session(),…).
```

- [ ] **Step 1: Failing tests (all NON-vacuous — M4/M5; one `InMemoryTransport::new(Vec<&str>)`+`responses()`):**
```rust
fn run(msgs: Vec<&str>) -> Vec<serde_json::Value> { let s=crate::mcp::tools::test_support::session(&[("a.py","def f():\n    return 1\n")]);
  let mut t=InMemoryTransport::new(msgs); serve_session(&s,&ToolRegistry::nav_v1(),&mut t).unwrap(); t.responses().to_vec() }
const INIT:&str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
const INITED:&str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
#[test] fn lifecycle_list_and_call() { let o=run(vec![INIT, INITED,
   r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
   r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#]);
  assert_eq!(o[0]["result"]["protocolVersion"],"2025-11-25"); assert_eq!(o[1]["result"]["tools"].as_array().unwrap().len(),6);
  assert!(o[2]["result"]["content"][0]["text"].as_str().unwrap().contains("nodes-at:a.py:1")); assert_eq!(o[2]["result"]["isError"],false); }
#[test] fn ping_returns_empty() { let o=run(vec![INIT, INITED, r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#]); assert!(o[1]["result"].is_object()); }
#[test] fn notification_no_response() { let o=run(vec![INIT, INITED, r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":9}}"#]); assert_eq!(o.len(),1); /* only initialize replied */ }
#[test] fn tools_call_before_initialized_is_32600() { let o=run(vec![r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nav_repo_map","arguments":{}}}"#]); assert_eq!(o[0]["error"]["code"],-32600); }
#[test] fn unknown_method_is_32601() { let o=run(vec![INIT, INITED, r#"{"jsonrpc":"2.0","id":2,"method":"resources/read"}"#]); assert_eq!(o[1]["error"]["code"],-32601); }
#[test] fn unparseable_is_32700() { let o=run(vec![INIT, INITED, r#"{not json"#]); assert_eq!(o[1]["error"]["code"],-32700); }
#[test] fn missing_call_name_is_32602() { let o=run(vec![INIT, INITED, r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"arguments":{}}}"#]); assert_eq!(o[1]["error"]["code"],-32602); }
#[test] fn unknown_tool_name_is_iserror() { let o=run(vec![INIT, INITED, r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nope","arguments":{}}}"#]); assert_eq!(o[1]["result"]["isError"],true); }
```
- [ ] **Step 2–4:** Run fail → implement `serve_session` (line read loop; §9 initialize state machine + per-method dispatch + param table; `tools/call`→`registry.get` [`None`→an `isError` `McpToolResult` "unknown tool"] →handler→**the result is `McpToolResult::to_call_tool_result_value()` verbatim** as the `tools/call` `result` (MAJOR2 — same bytes the cap measured); `-32700/-32600/-32601/-32602` per the §7/§9 tables; notifications→no response; batches→`-32600`; stdout=framed JSON only; EOF→return) + `serve_stdio` + wire `mcp::run`. → run PASS.
- [ ] **Step 5: Commit** — `feat(mcp): S3 stdio JSON-RPC transport + lifecycle + protocol errors (Plan 3c T7)`.

---

## Task 8 (S4): Protocol smoke (real binary) + dogfood + CI + docs

**Files:** Create `tests/mcp/smoke_test.rs`; `Cargo.toml` `[[test]] name="mcp_smoke" path="tests/mcp/smoke_test.rs" required-features=["mcp"]`; `.github/workflows/ci.yml`; `CLAUDE.md`. Do **NOT** touch `tests/integration/coverage_test.rs`.

- [ ] **Step 1: Failing smoke test** — `assert_cmd::Command::cargo_bin("prism-mcp")` `.arg("--repo").arg(".")`, write the 4-message lifecycle to stdin (line-delimited), **close stdin** (EOF shutdown), capture stdout: every stdout line parses as JSON-RPC (pure stream); `tools/list`→6 tools, `annotations.readOnlyHint==true`; `tools/call nav_callees {seed:{kind:symbol,name:"run_slicing_inner",file:"src/algorithms/mod.rs"}}`→`isError==false`, ≥1 cross-file item (file-qualified; bare `run` ambiguous — §13). **Confirm the ≥1-cross-file expectation against the live tool when writing it** (3b.5: `run_slicing_inner`→27 cross-file callees).
- [ ] **Step 2–4:** Run fail → (binary serves after T7) → `cargo test --features mcp --test mcp_smoke` → PASS.
- [ ] **Step 5: CI + docs** — `.github/workflows/ci.yml`: add a step/job `cargo build --bin prism-mcp --features mcp` + `cargo test --features mcp` + **`cargo clippy --all-targets --features mcp -- -W clippy::all`** (MINOR 13 — the existing clippy step omits the feature). `CLAUDE.md`: document `prism-mcp`, the 6 `nav_*` tools, the `mcp` feature.
- [ ] **Step 6: Commit** — `test(mcp): S4 protocol smoke + dogfood + CI/docs (Plan 3c T8)`.

---

## Task 9: Full-suite green + fmt + clippy (both configs)

- [ ] `cargo fmt` + `cargo fmt --all -- --check`; `cargo test` (default — unchanged); `cargo test --features mcp` (all pass incl. `mcp_smoke`); `cargo clippy --all-targets --features mcp -- -W clippy::all` (no new `src/mcp` warnings). Commit fmt-only changes.

---

## Deferred / follow-up (spec §17)
rmcp transport (spike+swap); type enrichment; `navigation::session` extraction; `SessionPool`/multi-repo; async transport (cancel/timeout + `Send+Sync`); Tier-2 tools + `FocusSet`; remote (Streamable HTTP+OAuth); eval harness; nav `Evidence v1.0`.

---

## Self-Review

**Handler contract:** handlers return `McpToolResult` (errors → `is_error` results inside the handler via `error.rs`), so Task 5's tests pass at Task 5 (no Task-7 wrapper dependency) — resolves plan-review B2. `to_listed` borrows (B1); `ToolError: Debug` (M3).

**Compile/coverage:** Task 1 all-stubs-compile; `ToolDescriptor` full shape + inlined 6 input schemas (M6) in T2; `resolve_cap` consumed by T5/T6 handlers + 3-branch test (M7); §13 byte-parity golden a real T4 step (M8); `EscapesRoot` representation + per-field divergence tests in T3/T5/T6 (M9); phase-1 composed-truncation test (M10); Task 6 compile-ready fixtures (M11); Task 7 non-vacuous protocol tests across the §7/§9 branches + `registry.get==None→isError` (M4/M5); positive `_meta` assertion + key-spelling checklist (M12); CI `--features mcp` clippy (M13); `prism-mcp` `clap` struct (M14).

**Spec coverage:** §8/§12→T1; §3/§4/§11→T2; §5/§10/§6.5→T3; §6→T4; **§7 tool-execution errors→T5/T6, §7 protocol errors (`-32700/-32600/-32601/-32602`)→T7**; §4 tools→T6; §9→T7; §13→T8 (+ the T4 byte-parity golden). The one nav touch (`WarningKind::ResultTruncated`) is in T1, authorized by §6.3.

**Note:** all test snippets assume `use serde_json::json;` at the test module scope. The handler contract here (`-> McpToolResult`) **supersedes spec §5's `-> Result<…, ToolError>`** (the spec carries a superseded-by-plan note). The rmcp spike is **pre-resolved to hand-rolled** (a spec §15-step-4-sanctioned outcome) with the exercise deferred behind the `Transport` seam (§17).

**Option-C:** default `cargo build`/`cargo test` unchanged; the variant is emitted by no nav/diff-review path → goldens byte-identical. T9 verifies both configs.
