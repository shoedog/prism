# Plan 3c (MCP adapter) — deferred follow-ups

Holistic-review findings intentionally **deferred** (current behavior is correct; these are
architectural/robustness refactors the verdict said to "tighten before the adapter grows"). The four
correctness/protocol MAJORs (lifecycle 3-state, version policy, envelope-reserve cap, bounded reader)
and the low-risk schema minor (`additionalProperties:false`) were **fixed in-branch**. A later
architecture-lens round flagged that the envelope reserve was *owned by the result shaper* (`output`)
though the envelope is a transport concern; that ownership was **moved to `transport` in-branch**
(`transport::ENVELOPE_RESERVE` + `payload_budget`, used by `shape_result` and the reserve test). Two
clarity MINORs from that round are documented below.

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

### 5. Cap semantics: serialized wire bytes vs characters (architecture-lens MINOR — naming)
- The cap is measured in **serialized UTF-8 wire bytes** (`String::len()`), but the names say "chars"
  (`MAX_RESULT_CHARS`, `PRISM_MCP_MAX_RESULT_CHARS`, `anthropic/maxResultSizeChars`). For ASCII JSON
  (the normal case) bytes == chars; heavy non-ASCII could clip a few bytes earlier than a literal
  char reading implies. **Disposition:** documented as wire bytes at the cap definition; the `_CHARS`
  names are kept for experimental-v1 wire-contract stability. **Fix when** a semantic character limit
  is actually needed: add a separate pre-serialization char cap rather than renaming the wire contract.

### 6. Floor enforcement at the shaper boundary (architecture-lens MINOR — refactor)
- `resolve_cap_from` floors caps at `MAX_RESULT_CHARS_FLOOR` (4000) at **construction**, so production
  never passes a sub-floor `cap` to `shape_result`. A sub-floor `cap` (only from tests) makes
  `payload_budget` saturate toward zero, which routes to the terminal `is_error` path — *safe, not a
  bug*. **The concern:** the floor is enforced at one site, not the shaper. **Fix when worthwhile:** a
  `Cap` newtype whose constructor enforces the floor, threaded through `shape_result`'s signature
  (deferred because the deliberate `terminal_over_cap_iserror_under_floor` test feeds a sub-floor cap
  to exercise the terminal path, so a naive in-shaper clamp would mask that path).

## From the round-5 in-depth review (codex correctness + claude architecture)

The three input-boundary MAJORs the synth named as ship-gating were **fixed in-branch**
(`fix(mcp): bound untrusted-client input on error/id/frame paths`): malformed envelopes echo a
sanitized (`null`) id via `safe_id_from_message`/`is_safe_id`; error-path results bound user-controlled
strings via `output::clamp_user_text` (`MAX_ECHO_BYTES`); and bad-UTF-8/oversized inbound frames are
now recoverable (`ReadOutcome::Malformed` → `-32700` + resync) instead of crashing the server. Two
trivial MINORs were also fixed (dead binary-search branch removed; `renumber_edge` → `retain_edge`
with the prefix-only precondition documented). The inline transport test module was split into
`src/mcp/transport_tests.rs` (`#[path]` sibling) to bring `transport.rs` back under the 600-line rule.

The synth itself classified the remaining MAJORs as "pre-Tier-2 hardening, **not merge blockers**" —
they are the larger cap/truncation **ownership refactor** cluster, intentionally deferred (current
behavior is correct and tested):

### 7. Cap-ownership cycle: `output::shape_result` names `super::transport::payload_budget` (MAJOR — refactor)
- The envelope-reserve lives in `transport` (round-4 fix), but `output::shape_result` reaches *up* into
  `transport::payload_budget(cap)` while `transport` depends *down* on `output::{McpToolResult,
  to_call_tool_result_value, SCHEMA_VERSION}`. The wire-bytes invariant is proven in two halves
  (`reserve_covers_envelope_and_max_id` + `over_cap_*`) with nothing asserting composed end-to-end wire
  bytes. **Fix (with #8/#9):** give one unit ownership of wire serialization+sizing — a transport-level
  sizer that nets the envelope and hands `shape_result` an already-net budget as a plain parameter, so
  `output` never names `transport`. *(Note this is the inverse of the round-4 architecture-lens ask,
  which moved the reserve into transport; the stable end state is "transport computes the net budget and
  passes it down," done as one deliberate refactor rather than oscillating mid-merge.)*

### 8. `shape_result` is not the sole truncation owner (MAJOR — refactor)
- `tools.rs` (`clip_flat`/`clip_graph`) clips to `max_results` and passes the *pre-clip* `total` + a
  `max_results_clipped` flag into `shape_result`, which clips again for the cap. Three facts (shortened
  `ev`, un-shortened `total`, the flag) must stay consistent across all 6 call sites; a future tool that
  passes the clipped length as `total` yields "showing N of N" and an agent treats a partial answer as
  exhaustive. **No live bug** (the 6 sites are consistent today). **Fix:** make `shape_result` the sole
  truncation owner — pass it the full `Evidence` + `max_results` and let it own both clip layers + the
  flag. Folds in the duplicated graph-clip helper (`clip_graph` vs `retain_edge`).

### 9. Result-size cap is ambient process state, not `ServerConfig` (MAJOR — refactor)
- Every handler calls `output::resolve_cap()` → reads `PRISM_MCP_MAX_RESULT_CHARS` per `tools/call`, so a
  misconfigured env var warns on *every* request (vs once at startup) and re-parses per call, and the cap
  can't follow `repo_root` into a future `SessionPool`. **Fix:** resolve once in
  `SessionProvider::bootstrap`, store on the session/config, thread to `shape_result` (composes with #7).

### Structural MINORs (deferred with the #7–#9 cluster)
- **`Verbosity` declared twice** (`input::Verbosity`, `output::Verbosity`) bridged by
  `output_verbosity()` — define once when the cluster is refactored.
- **Per-handler execution glue is copy-paste** (`parse → to_triple → query → map-error → clip →
  shape_result(resolve_cap())`) — extract a `run_query` combinator when the 4 Tier-2 tools land (keeps
  `tools.rs` under 600).

## From the round-6 in-depth review (codex correctness + claude architecture)

Round 6 confirmed **no BLOCKER** and verified the round-5 trio correct. It found two fix-before-ship
MAJORs, both **fixed in-branch** (`fix(mcp): preserve error payloads + validate arguments type`):
the round-5 whole-blob clamp was destroying `AmbiguousSymbol` candidates mid-JSON — now
`query_error_result` bounds the *typed* error (clamps `seed`/`file`/`edge` leaves, caps candidate
count) so the rendered JSON stays valid and usable; and `tools/call` with a non-object `arguments`
now returns protocol `-32602` instead of `isError`. The low-risk compose-fix for `build_result`
(don't clobber a query's own `truncated`/warnings — round-6 #3) was also taken in-branch. The rest are
**deferred** (no live bug; fold into the cap/truncation ownership refactor with #7–#9):

### 10. Error path has no single wire-size chokepoint (MAJOR — refactor; distinct from #7)
- Success results are structurally bounded by `shape_result`/`payload_budget`, but **error results never
  pass through it** — each error site bounds user strings by convention (`clamp_user_text`,
  `bound_query_error`). A future Tier-2 handler or new error kind that returns a result without clamping
  ships unbounded past the cap; `transport::write_message` does no size check. No failing scenario today
  (envelope ≤ ~290B vs 512B reserve). #7 is the success-path *ownership*; this is the error-path *gap*.
  **Fix (with #7):** enforce one size chokepoint at the transport write boundary so every outbound frame
  is bounded regardless of kind. (The round-6 #1 fix is a targeted instance of this class.)

### 11. `nav_nodes_at` truncation remediation names a knob it doesn't have (MINOR)
- `nav_nodes_at`'s schema is `{file, line, verbosity}` (no `max_results`), but a cap-truncated result
  still attaches `ResultTruncated` with "raise max_results or narrow" — unactionable, costing a wasted
  retry. Low likelihood (location-scoped results are small). **Fix:** tool-aware remediation text (needs
  tool identity threaded to `shape_result`, so it composes with #8's truncation-owner refactor).

### 12. "Single serializer" is technically two (MINOR — future-proofing)
- `build_result` derives `content_text` via `render(&shaped,"json")` and `structured` via
  `serde_json::to_value(&shaped)` — equivalent today (render's json branch is `to_string_pretty`), but
  if `render` ever transforms evidence (redaction/renaming) the two views silently diverge. **Fix:**
  derive `structured` from the same chokepoint, or assert their equivalence in a test, when the output
  module is refactored.

## From the round-7 in-depth review (codex correctness + claude architecture)

Round 7 confirmed **no BLOCKER** and verified the round-6 fixes correct. It found one fix-before-ship
MAJOR — the **concrete live instance of §10** — now **fixed in-branch**: `bound_query_error` capped the
candidate *count* and clamped the scalar `seed`/`file`/`edge` leaves, but the strings *inside* each
`AmbiguousSymbol` candidate (`SymbolRef` `name`/`file`/`path`/…) were still unbounded and could push a
valid-JSON error payload past the cap. `bound_symbol_ref` now clamps every string in every retained
candidate across all three `SymbolRef` variants, **closing the error-path string-bounding class** (no
unbounded string reaches an error result by any path). The paired MINOR (silent candidate cap) was also
**fixed**: a "Showing N of M matches" signal is appended when the cap fires. The remaining round-7
findings are **deferred** (no live bug; fold into the §7–§9 ownership refactor):

### 13. Error-result construction + `_meta` diffused across three modules (MINOR — folds into §7/§9)
- `isError` results are hand-built in `error.rs` (query/bad-args), `output.rs::terminal_over_cap_result`,
  and `transport.rs::unknown_tool_result`, each assembling its own `_meta`; success results carry
  `anthropic/maxResultSizeChars` but error/terminal paths omit it. **Fix:** one `error_result` constructor
  with a single `_meta` source, taken when the cap-ownership refactor lands.

### 14. `build_result` compose branch reports the post-query-truncation total (MINOR — within §8)
- When the adapter clips AND a future query already set `ResultTruncated`, the adapter re-emits
  `showing {retained} of {total}` where `total` is the post-query-truncation count, losing the true
  upstream total. No live bug (no nav query truncates today). The §8 "sole truncation owner" refactor
  resolves this exact spot.

### 15. Idless non-notification request returns `-32600` instead of silent ignore (MINOR — accepted for v1)
- Strict JSON-RPC treats any idless message as a notification deserving silence; the impl replies
  `-32600` (pinned by `idless_request_is_32600`) as a deliberate diagnostic for a buggy local client.
  Acceptable for a local single-client v1; revisit if a strict multi-client transport is added.

## From the round-8 in-depth review (codex correctness + claude architecture) — verdict: SHIP

Round 8 verified round-7 closed the unbounded-error-string class with **no new defect** and returned an
explicit **SHIP**. The low-risk items were **fixed in-branch** (`fix(mcp): id-agnostic initialized
transition + dead-code cleanup`): the one independent correctness nit — an id-bearing
`notifications/initialized` deadlocking the session — now transitions regardless of body (MCP §9), and
the dead `not_implemented()` stub + duplicate free-fn `serialized_len` were removed. The residual MAJORs
are the already-documented, pre-accepted deferrals (not merge-blockers per both lenses): the error-path
fixed-ceiling-vs-configured-cap gap is **§10** (concretely: an `AmbiguousSymbol` with 20 long candidates
emits a ~20–40 KB frame that exceeds a *deliberately-lowered* sub-26 KB operator cap; safe at the 80 KB
default — fix by adding a `transport::write_message` size chokepoint and `anthropic/maxResultSizeChars`
to `error_meta()` when the §7 refactor lands), and the cap-ownership cycle is **§7/§14**. One genuinely
new deferral:

### 16. `registry ↔ tools` cycle bakes the nav toolset into the generic constructor (MINOR — Tier-2 seam)
- `registry.rs::ToolRegistry::nav_v1()` calls `tools::register_all`, coupling the generic registry
  mechanism to the specific nav toolset at the seam meant for Tier-2 extension. **Fix when Tier-2 tools
  land:** an empty `ToolRegistry::new()` + a `tools::nav_v1_registry()` builder, keeping the registry
  independent of any tool set. (`shape_result`'s per-probe re-clone/serialize is the §8 efficiency item.)

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
