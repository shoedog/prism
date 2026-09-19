# Exact caller-read occurrence handoff

## State

- Stage: source and R01-R06 tests frozen; R07 evidence pending from the independent verifier.
- Worktree: `/private/tmp/prism-post316-slice1`, branch `feat/post317-exact-caller-reads`.
- Controller owns checkpoint commits. No push, PR, merge, release, rebaseline, or adoption has been performed.
- Candidate paths and hashes are bound by `source-manifest.md`; parallel input-custody/planning-document edits are outside this implementation manifest.

## Behavior

The DFG now retains crate-internal, byte-distinct supplemental producer facts for a bounded class of named JS/TS/TSX straight-line callables. A plain required parameter, or one unique simple local declaration on an earlier line, can reach every authenticated same-line simple-identifier read. The existing reaching-definition solve independently classifies each exact pair before legacy `(path, line)` reduction. Step 4 validates exact endpoints and adds only absent pairs.

Global `VarLocation` identity, legacy edges/labels/adjacency, public query signatures, byte payloads, control-flow/kill semantics, and navigation cache semantics are unchanged. Writes, aliases, destructuring, optional/default/rest parameters, branches, loops, try/switch, short circuits, nested captures, members, recovery, reflection, ambiguous owners, and non-JS routes remain refused.

## Evidence and open work

- Public fail-first evidence, focused candidate totals, thread-count parity, warm/incremental exact-state equality, and preservation totals are in `source-manifest.md`.
- Genuine predecessor CPG96 cache custody is `/private/tmp/prism-post317-planning/base-cache/RECEIPT.md` SHA-256 `dbee9183f888dbecc6a9e593dc09ab4517fa33d734fad218789edb537bf4adbd`; immutable cache binary SHA-256 `2bdf18dc1f485a4d00efa81033a3c90fee0e7c2a43daf373d31ed528b5859fb2`.
- Pending before review dispatch: direct CPG96 refusal, CPG97 rebuild/Hit parity, and bounded JS/TS/TSX 10/100/1,000 cost results. Historical real JS/private corpora remain input-blocked.
- Pending after independent review acceptance: full default/MCP/widest Rust suites, examples, clippy, fmt/diff, Node/compiler authority/Python and Tier-A matrix/quick under the established runbook. Any excluded or invalid gate must remain explicit.
