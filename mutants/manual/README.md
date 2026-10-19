# Manual feature-gated mutants

`idle-unload-i0a.json` is deliberately outside `mutants/*.json`.
`scripts/mutgate/mutgate.py` builds without features, so MCP mutants are
INADMISSIBLE in that runner. Do not include this file in its root registry.

For each record, snapshot the current source; apply every exact `old` → `new`
edit once; run `nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp
--lib SELECTOR -- --exact`; read the behavioral assertion failure (compile
failure or zero selected tests is inadmissible); restore the original bytes;
rerun the exact selector and require one PASS. Record start time, command,
assertion, and before/mutant/restored SHA256 in the external I0a evidence root.
No tracked-parent source replacement. First four records are required; later
records supply extra negative witnesses. These are fault injections, not
claims that the parent supported the new lifecycle API.

Receipts: `~/prism-evidence/mcp-memory/optimization-round2/idle-unload/i0a/`.
The controller owns commits. Do not start cargo at/after 2026-10-19 00:45 MDT.
