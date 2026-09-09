# Bounded owner opt-in activation

Base: merged PR297, a120220f. Owner approved the recommended activation slice.
Two self-review rounds maximum; open-class findings stop activation.

## Source-backed contract

- Explicit `--owner-compiler /absolute/path/to/typescript.js` paired with
  `--owner-config tsconfig.json`; neither environment discovery nor installation.
  Compiler bytes remain pinned to the existing TypeScript 5.9.3 authority.
- Navigation only: CLI nav/API and MCP; review/targets remain unchanged. Opt-in
  requires `--no-cache`, rejects cache directories. MCP additionally requires
  `--eager` and the default warn-only policy. No lazy or incremental activation.
- A separate private MCP runtime acquires a full fresh epoch before every tool
  dispatch, including refresh. It clears its active slot first and reports ordinary
  build failure on refusal. It never enters the ordinary transport's auto-refresh
  error fallback, which deliberately queries historical state.
- CLI and MCP share the staged acquisition/session constructor. No raw proof API.
  Public options select inputs, not authority. Historical API handles remain
  historical snapshots; the served runtime never republishes them after failure.
- Keep worker asset custody at the compile-time source checkout for this first
  experimental activation. Missing/changed assets refuse explicitly. Packaging
  portable worker assets is deferred; do not imply an installed binary is portable.
- Reject loaded non-JS/TS files and type databases at the public boundary for now:
  staged CPG assembly currently passes no TypeDatabase. Do not silently remove
  sources or claim mixed-language typed parity. Existing private staging stays intact.
- Closure-v3, full index/Program census, grammar, cache markers and react-scripts
  disposition unchanged. Unsupported receiver shapes may yield an acquired session
  with no new proof; acquisition/closure failure is an error, not ordinary fallback.

## Proof and gates

Capture public API missing-Exact RED against a no-op options seam on the base.
Then test intended edge and unchanged controls, API/CLI selection refusals, MCP
wire callers/callees, A/B/unproven/failure/restore, config and failed-lookup changes,
no cached directory writes, compiler-independent argument validation, and true
compiler acquisition under the audit feature. Read actual response artifacts.
Full observer/default/MCP/audit/helpers/authority suites plus formatting/clippy,
release rebuild, Tier-A matrix and quick. Preserve baseline-invalid observations;
matched base controls before regression attribution. No new accuracy baseline.

## Hypothesis/probe/result log

1. Source hypothesis: ordinary MCP refresh failure retains its old session; an
   alternative was that readiness invalidates it before dispatch. Inspection of
   SessionProvider::refresh_verified and transport::auto_refresh_tool_response
   confirms retention and direct old-session handler use. This is intentional
   default behavior, not a defect report. Isolate owner activation from that path.
2. RED hypothesis: accepting owner input options without wiring cannot grant the
   contextual Exact edge. Exact would falsify it; name-only output supports it.
   Captured `api-red-corrected.log`: NameOnly vs Exact, 0 pass/1 fail. Initial
   nonexistent ctx accessor was an inadmissible compile error, corrected first.
   Wired public API positive subsequently passed.
3. MCP matcher failures: distinguish absent activation from concise-wire omission.
   `wire-diagnostic.log` contains src/client.ts/m at score1.0 with no location and
   empty why. Default concise shaping, confirmed in source, explains both failures.
   Detailed wire requests now carry the fields required for provenance assertions.
   No product-regression attribution or product fix follows from these test errors.
4. MCP subprocess probe initially omitted required initialize fields: captured
   Invalid params / Invalid Request, not activation evidence. Corrected full
   handshake passes, alongside API/CLI equality and missing-compiler error tests.
5. Review round 1 WRONG: after source A→B, refresh summary hard-coded false for
   stale_before_refresh. Alternative stamp-race explanation ruled out by literal
   constants in the new runtime. `refresh-report-red.log` captures false vs true.
   Fix retains prior metadata stamps, clears active before IO, and reports their
   measured delta. Stamps never authorize reuse; full acquisition remains mandatory.
6. Full helper gate found three missing-fixture errors. Alternative activation
   regression ruled out by the unchanged helper source and same-environment base
   reproducing all three. Reconstructed five files from pinned local Git objects;
   their historical SHA256 values match. Full helpers passed18/18 on both revisions.
7. Full suites and two review rounds complete. Tier-A quick remains baseline-invalid;
   see the [readout](../../eval/receiver-closure/2026-09-09-owner-opt-in-activation.md)
   for exact counts, limitations and matched default-path controls. No rebaseline.
