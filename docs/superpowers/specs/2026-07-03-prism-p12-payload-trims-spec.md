> **Status: SHIPPED — PR [#161](https://github.com/shoedog/prism/pull/161), merged 2026-07-03 (main ff1cbeb).** As-executed brief incl. the folded codex spec-review rulings (S2/S3 env-gated DEFAULT-OFF — no client trace exists; serialization-time-only omission with internal `structured` populated; S1 per-tool hedge; Concise-is-the-MCP-default finding; extended golden gates). As-shipped deltas beyond this text (one post-impl review wave): cap sizing made MODE-AWARE via `wire_len(mode)` DERIVED from `to_call_tool_result_value` (controller adjudication reconciling the codex MAJOR [item-retention win never materialized under frozen sizing] with the opus defense [purity/env-race] — the resolved mode is THREADED as a parameter, never read ambiently in sizing; agent-view sizing stays Always); item retention 7→10 pinned by test; VIEW_NOTICE-absence pin added; refresh_index omit-mode pin added. Implementer corrected the brief's wrong grounding premise: content_text is pretty-printed vs compact structuredContent → realistic wire saving ~31% not 50%. Measured: tools/list 15718→12225 B (−22%, ACTIVE); result −31% (S2 opt-in) / −34% (S3 opt-in). **The fix-delta codex re-review returned SHIP with zero findings — the first clean re-review of the execution** (drift class avoided by construction). Default flips remain owner-gated on a live claude -p verification (docs/MCP.md).

# Task P12 — MCP payload trims: notices→instructions, default-path structuredContent gate, Concise item slimming

You work in the git worktree `/private/tmp/prism-p12-payload-trims` on branch `p12-payload-trims` (based at current main). The repo is prism. Follow TDD. All MCP work builds/tests with `--features mcp`. Locate code by SYMBOLS (grep), not line numbers.

## Safe-failure direction (binding)

**Information loss is the unsafe direction.** Hard gates: ALL pinned surfaces in `tests/cli/nav_compat_test.rs` must pass UNMODIFIED — the 4 nav JSON goldens (`callees_golden_qualified`, `ego_golden`, `module_deps_golden`, `repo_map_golden`) AND the review-side pins in the same file (leftflow/thin/parentfunction/list goldens near the top, codex catch) — if your change requires regenerating any golden, the change is wrong. CLI output (`prism nav --format json` via `crate::output::navigation::render`) is untouchable. MCP smoke tests (`tests/mcp/smoke_test.rs`) and transport_tests keep their default-state contracts (new behavior is env-gated). Agent views keep their full contract. Every trim is MCP-only; the RESULT-shape trims (S2/S3) are env-gated DEFAULT-OFF this slice, while S1 (notices→instructions, with per-tool hedge) ships active — it moves text, losing nothing.

## Grounding facts (verified at HEAD — trust these over the plan text)

- No MCP SDK: hand-rolled JSON-RPC in `src/mcp/transport.rs`, protocol `2025-11-25`. 8 tools (`all_v1()`). No tool declares `outputSchema` → `structuredContent` is protocol-OPTIONAL; `content` (text) is required.
- Notices: `SNAPSHOT_NOTICE` (271 B, `tools.rs`) + `VIEW_NOTICE` (319 B) appended per nav tool (`tool_with_handler`); `tools_reasoning.rs` duplicates SNAPSHOT only; `refresh_index` carries neither. True redundancy ≈ 3.2 KB per tools/list (plan's 4.5 KB was high).
- `initialize_response` (`transport.rs`) sends NO `instructions` field today — the protocol-legal home for state-once text.
- Double-ship: `to_call_tool_result_value` emits `content[0].text` AND `structuredContent`; `serialized_len` bills BOTH against `MAX_RESULT_CHARS` (80_000, env `PRISM_MCP_MAX_RESULT_CHARS`, floor 12_000, `ENVELOPE_RESERVE` 512). **Default `CanonicalJson` view: content_text == structuredContent byte-identical (pure redundancy). Agent views (`agent_json`/`agent_markdown`): `evidence_view.rs` REWRITES content_text and structuredContent is the ONLY canonical-Evidence carrier — must be KEPT there.**
- `structured_count` (`evidence_view.rs`) reads `result.structured` to drive the agent-view binary search — needs a non-wire count source if structured is gated.
- Concise mode (`build_result_with_options`, MCP-only — CLI never applies `Verbosity`): currently empties `why` (`why.clear()` — keeps the key), compacts reasoning non-verdict detail (keeps `sanitized_by`), drops the witness `graph`. Items still carry: `symbol` (with `start_byte`/`end_byte`/`ordinal`), `location` (duplicate file/lines/bytes), `snippet: null` (no skip attr), `score`, `source`, `fallback`.
- Precedent for an MCP-only slim DTO: `ViewSourceLocation` in `evidence_view.rs` (Option byte fields + skip_serializing_if).
- **No saved trace shows what Claude Code injects** (adoption harness discards tool_result payloads; docs/MCP.md silent). The (b) rollout below is env-gated pending live verification.
- NO cache bumps anywhere (output types have no Deserialize, never persisted — verified).

## S1 — notices → initialize `instructions` (a)

Move the snapshot + view text to a single `instructions` string in `initialize_response` (state each ONCE, wording preserved or lightly merged); strip the full `SNAPSHOT_NOTICE`/`VIEW_NOTICE` appends from `tool_with_handler` and `tools_reasoning.rs` but **keep a one-line hedge in each affected tool description** — e.g. "Snapshot/view details: see server instructions." (codex MAJOR: client ingestion of `instructions` is unverified; the hedge preserves discoverability at ~50 B/tool vs 592 B/tool today). Update the pin test `registry_lists_six_tools_with_annotations` to pin: descriptions contain the hedge but NOT the full notice text + initialize result contains `instructions` with both notices. Measure and paste tools/list byte size before/after.

## S2 — default-path structuredContent gate (b)

- In the DEFAULT (CanonicalJson) path only: support omitting `structuredContent` from the WIRE (content text carries the identical JSON). Agent-view path: UNCHANGED always (structuredContent stays — it is the only canonical carrier; the smoke test `:92-99` contract holds).
- **Omission happens ONLY at serialization time in `to_call_tool_result_value`** — the internal `McpToolResult.structured` field stays POPULATED regardless (codex: transport freshness checks `structured.is_some()` at transport.rs near :418, and `structured_count` reads it; clearing the field would silently drop stale-index warnings). Also derive `structured_count`'s value from the internal field or shaped Evidence, never from the wire JSON.
- Env switch `PRISM_MCP_STRUCTURED_CONTENT` = `always` | `omit-default-path`. **DEFAULT = `always` (trim INACTIVE) — codex-adjudicated rollout, overriding the earlier draft**: no client trace exists proving Claude Code reads `content[0].text` rather than structuredContent, so the trim ships opt-in. Implement + test BOTH states; make the default a one-line change. A post-merge live verification session (2–3 probes from `eval/adoption/goldens/probes.toml` through real `claude -p` with `omit-default-path` set — owner-gated, flagged in the PR as follow-up) is the gate for flipping the default later; document this in the env var's doc comment and docs/MCP.md.
- transport_tests structuredContent assertions (33 field-level): default-state tests keep asserting presence (default unchanged); ADD `omit-default-path` tests asserting wire absence + content-text intactness + agent-view presence + freshness warnings still emitted. Cap math test: under `omit-default-path`, `serialized_len` of a canonical result ≈ halves for identical Evidence (assert).

## S3 — Concise-mode item slimming (c)

MCP-only, `Verbosity::Concise` transform ONLY (Detailed stays byte-parity with CLI render — do not touch it). **Grounded fact (codex-verified): Concise IS the MCP default** (input.rs near :475, output.rs near :42) — so an unconditional slim projection would be a default-shape change for every MCP client. **Therefore the slim shape is env-gated (codex MAJOR)**: `PRISM_MCP_CONCISE_SHAPE` = `legacy` (DEFAULT — current shape, byte-unchanged) | `slim`. The slim projection (MCP-only DTO, `ViewSourceLocation` precedent — shared-type serde changes break CLI goldens): keep `symbol` name/kind/file/lines (drop its byte fields + ordinal), keep ONE location (drop the symbol/location duplication; keep both only when they differ), omit `snippet` when None, keep `score`/`source`/`fallback`/emptied-`why` semantics. `content_text` renders the projection under `slim`; structuredContent (when on the wire) matches. Tests for BOTH shapes; leave `detailed_content_is_render_byte_parity` and the legacy Concise pins (`concise_nulls_why` etc.) INTACT for the default state. The same post-merge live verification session that gates S2's default flip evaluates flipping this one; note it in docs/MCP.md.

## Measurement (paste in report)

Local, no live API: for a fixed probe set (adapt queries from `eval/adoption/goldens/probes.toml` against a bench repo or the tier_c fixture repo), record per-tool `serialized_len` and items-retained before/after each slice (a small `#[cfg(test)]` harness or dev script in the worktree is fine — do not commit a new binary). Target: default-path wire bytes ≈ halve (S2); tools/list shrinks ~3.2 KB (S1); Concise items shrink measurably (S3). Local gates: full `cargo test --features mcp`, `tests/mcp/smoke_test.rs`, transport_tests, and the UNMODIFIED nav_compat goldens. The adoption ToolCorrectness live run is NOT required for merge (owner-gated; coarse tool-selection proxy only).

## Done-checks (run and paste)

```
cargo build --release && cargo test && cargo test --features mcp
cargo test --test cli nav_compat   # the 4 goldens byte-identical, UNMODIFIED
cd eval && uv run tier-a --matrix-only --allow-stale-sut   # 0 regressions (should be untouched)
cd eval && uv run pytest -q --ignore=adoption
# paste: tools/list bytes before/after; one default-path tool result JSON (no structuredContent) + same call with PRISM_MCP_STRUCTURED_CONTENT=always; one Concise item before/after
```

## Commit style
Small logical commits (S1 / S2 / S3 / measurement notes in report). End each commit message with:
Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
