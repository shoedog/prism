# S2 PR #94 — Second-Pass Reviews (two deep-focus lenses) — 2026-06-14

Two additional reviewers over the full PR #94 diff (`origin/main..s2-node-identity`), each
**deep-focused on a different area** (both reviewed the whole diff). Requested because S2 is
a large diff (49 files). Setup notes:

- **Models card** (`a2a-bridge models`, live-probed): codex advertises gpt-5.5 + effort
  `xhigh`; **claude advertises only `default, sonnet, sonnet[1m], haiku` — no opus.** So the
  bridge cannot run Opus 4.8 (claude-acp model-override limitation) → the opus review ran as
  an **operator subagent** (guaranteed Opus 4.8) instead.
- **rust LSP** = the `lsp-mcp` shim (wraps rust-analyzer; design `2026-06-13-lsp-mcp-nav-design.md`).
  It had **no built binary** — built it (`cargo build -p lsp-mcp`) and wired it + prism into
  the codex reviewer's config (`examples/a2a-bridge.s2-review2.toml`). **However codex
  reported the `n`/rust-analyzer tools never appeared** in-session (only prism + github/
  computer-use) — `lsp-mcp` silently didn't expose, most likely the cold rust-analyzer index
  missing the MCP tool-discovery window. So codex reviewed with **prism + read-only source**;
  a true LSP-assisted pass needs the `lsp-mcp` cold-start debugged (pre-warm / longer
  discovery timeout). The claude operator subagent had prism MCP (LSP is bridge-only).

## Reviewer 1 — codex (gpt-5.5, xhigh) · focus: data-flow / de-conflation / span correctness
Verdict: **needs changes.**
- **MAJOR** build.rs:385 — Step-5b interprocedural arg binding by `(line, callee_name)`; same-line
  `callee(a); callee(b)` both bind `a`. → **deferred item 9** (byte-aware arg binding).
  Triage: **not an S2 regression** — `b→param` missing pre-S2 too; de-collapse only adds a
  harmless duplicate `a` edge. Raised to Priority M (both reviews flag it).
- **MAJOR** nav `Reason::Calls`/`CalledBy` drop the call-site byte. → **deferred item 5**
  (already flagged by the first review; spec §5 "may", additive).
- **MINOR** ast.rs:2130 — `o.config.timeout += 1` (nested member) misses the base `Use(o)`
  fallback. → **deferred item 10** (span-precision FN, rare).

## Reviewer 2 — claude (Opus 4.8, operator subagent) · focus: seams / cache / tests / deferrals
Verdict: **ship** (0 new blockers). Independently verified, with traced paths:
- Byte-additive identity upheld END-TO-END (byte in no key/Ord/Eq/Hash; nav goldens purely
  additive; SCHEMA_VERSION tested).
- Cache rebuilds **every index identically** to a fresh build (chased + disproved the
  first/last-writer `var_index` concern — provably one node per key); `serde(default)` sound
  across the v4→v5 bump (v4 rejected before deserialize).
- Tests pin the load-bearing invariants with strong assertions (real-text spans, raw bucket
  order, edge-flip shape).
- Witness wire (byte range + reserved ordinal) **confirmed to enable Plan B** to delete
  Slice 5 (ordering oracle) + Slice 3d (function identity); deferrals all genuine additive
  seams; the reviewer-BLOCKER (exact-`FunctionId` traversal) confirmed an over-approximation,
  not a regression — owner's merge call.
- MINORs: CallSite `receiver_recovery` Ord/Eq residue (**item 11**, pre-existing); **missing
  different-name-same-line regression test** (FIXED this pass); untested line-collapsed
  witness anchor (**item 12**); stale deferred-doc item 1 (**reconciled** — alias span is
  already solved, pinned by `alias_resolved_def_keeps_raw_occurrence_span`).

## Disposition
- **FIXED this pass:** `different_name_functions_on_same_line_are_distinct` regression test
  (the explicit invariant the `(file,name,start_line)` key exists for). Full suite re-run green.
- **Reconciled:** deferred-doc item 1 (alias span already solved).
- **Deferred (documented, items 5 + 9–12):** byte-aware arg binding (M; both reviews; not a
  regression), nav-Reason call-site byte (both reviews; additive), nested augmented base
  fallback, CallSite receiver_recovery, line-collapsed witness test.
- **Net:** both lenses confirm the PR is sound; codex's "needs changes" reduces to two
  already-deferred additive items + one MINOR. No new in-scope correctness fix required
  beyond the added regression test.
