# Plan 3c (MCP adapter) — deferred follow-ups

Holistic-review findings intentionally **deferred** (current behavior is correct; these are
architectural/robustness refactors the verdict said to "tighten before the adapter grows"). The four
correctness/protocol MAJORs (lifecycle 3-state, version policy, envelope-reserve cap, bounded reader)
and the low-risk schema minor (`additionalProperties:false`) were **fixed in-branch**.

## From the holistic code-review

### 1. Path-boundary explicitness (holistic MAJOR — deferred) — **Low risk; refactor**
- **Current behavior is correct:** `normalize_path` returns `RepoRelative|EscapesRoot`; an `EscapesRoot`
  (or any un-indexed) path flows to the nav query, where an exploratory field (`nodes_at`/`module_deps`)
  yields empty + `SkippedPath` (a `contains_key` miss) and a seed field yields `isError`
  (`resolve_fn` miss). The §10 divergence holds.
- **The concern:** the MCP boundary leans on nav's miss behavior; if nav path handling changes later,
  the adapter's contract could drift. **Fix when the adapter grows:** carry `RepoRelative` vs
  `EscapesRoot` explicitly through handler dispatch and decide exploratory-vs-seed *before* calling nav.

### 2. Concise `why` = empty array vs `null` (holistic MINOR — wording)
- The contract says concise "field-nulls" `why`; the impl sets `why: []` (empty array), which the tests
  pin. An empty array is a cleaner, type-stable representation than `null`. **Disposition:** keep the
  empty array; the spec wording "field-null" means "emptied." No code change.

### 3. Graph-vs-flat retained-count coupling (holistic MINOR — future-proofing)
- `shape_result` uses one retained count, preferring graph-node count when a graph is present. **No
  current query returns both a graph and a separate item list**, so there is no live bug. **Fix if** a
  future payload carries both: separate clipping policies for graph nodes vs item lists.

### 4. Windows-path normalization (holistic MINOR — Unix-only)
- `normalize_path` understands `/` only; it does not reject `\`-style absolute/traversal paths. Harmless
  under the repo's Unix-style path expectations (loaders store `/`-separated relative paths). **Disposition:**
  documented as Unix-only; revisit if Windows repos are ever a target.

## Carried from the spec (§17 — separate slices)

- **rmcp transport** — spike (spec §9 criteria a–f) + swap behind the `Transport` trait; adds
  `tokio`/`rmcp` to the `mcp` feature only if accepted. v1 ships hand-rolled.
- **Type enrichment in MCP** — `--compile-commands` → whole-repo `TypeDatabase::from_compile_commands`
  + the bootstrap-`clang` trust/timeout/stderr rules (dropped from v1: the clang subprocess is out of a
  thin read-only v1's scope).
- **`navigation::session`** extraction (both consumers proven); **`SessionPool`/multi-repo**;
  **concurrent/async transport** (real cancel/timeout; confirms session `Send+Sync` — currently
  single-threaded, `#[allow(clippy::arc_with_non_send_sync)]` on `bootstrap`).
- **Tier-2 reasoning tools** (`taint_reaches`/`dataflow_between`/`impact_of_change`/`what_missing`) +
  `FocusSet` seed variants — register on this adapter; own initiative.
- **Remote transport** (Streamable HTTP + OAuth) — only if hosted multi-client is needed.
- **Nav `Evidence v1.0` (S5)** — typed dependency `target`/`direction`, graph/node-kind enums; drops the
  `experimental` marker; coordinates with `module-deps` `location.file` overload + the graph-vs-flat
  dual shape. **Eval harness** (spec §15).
