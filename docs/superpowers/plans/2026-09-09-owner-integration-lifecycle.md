# Disabled executable-owner integration core

Base: merged PR296, `a892b67ddec0883fc0ba7cfaae7fa33fdfb97eaf`.
Owner approved proceeding to opt-in integration with epoch/cache safeguards.
The approved design explicitly permits disabled staging rather than shipping a
partial consumer. This slice implements that staging boundary; no CLI/MCP option,
automatic acquisition, public activation factory, install or closure-policy change.

## Source-backed architecture

1. A private staged session loads the full repository and acquires a fresh epoch.
   It replaces JS/TS parser inputs with the epoch's privately reparsed inputs,
   retaining other languages unchanged. The owning session builds full/subset
   graphs using that same full input census; callers cannot install arbitrary maps.
2. A non-serializable installed sidecar holds the epoch and original CallSite/proof
   pairs. The shared resolver consults only exact site identity, owning epoch and
   graph-present target. An installed but invalid/missing-target entry is terminal.
   Explicit/local/imported-Props routes are unchanged; the new compiler grammar
   does not claim their sites. No pre_resolved_target class guesses.
3. The new resolution kind is appended, not inserted into persisted enum ordinals.
   The sidecar is serde-skipped; a CallGraph roundtrip loses authority and ordinary
   graph bytes stay equal. No existing serialized field/layout changes, so CPG77
   and navigation45 stay unchanged. Binary build identity still invalidates old
   ordinary caches. Proof-derived CPG or navigation edges must never be saved.
4. CPG assembly receives the installed graph BEFORE constructing edges. A private
   sticky origin marker on the graph propagates through CPG reconstruction; the
   CPG's own marker survives clearing its graph sidecar and rejects
   cache writes before directory creation. Navigation discards any supplied cache
   store for such a CPG, fencing both lazy sidecar load and write. The staged entry
   never calls cache load. These guards apply even to a successfully acquired empty
   proof map (opt-in/unproven), not just positive edges.
5. Refresh clears the active slot before fallible acquisition and publishes only a
   complete new session. It never reuses old proof-derived CPG/DFG/index state.
   Ordinary remove/merge/recomputation/incremental graph paths clear authority;
   ordinary cached contexts rebuild without ephemeral edges. Historical held
   sessions remain historical. Future public activation must apply this lifecycle
   at its actual CLI/MCP publication boundary; that is not enabled here.

## RED, tests and gates

- Archive a base-plus-no-op staging seam before edits. Capture the intended missing
  Exact result through the real shared resolver, CPG and navigation functions, not
  a fake classifier. Preserve existing negatives as compatibility controls.
- Cover full/subset, exact-site field mutation, genuine epoch substitution, target
  omission, clone/serde/remove/merge, positive A/B/unproven/failure/restore, config
  and negative lookup changes, ordinary cache roundtrip, rejected CPG writes and
  a sentinel navigation sidecar that must neither load nor change.
- Keep grammar at the PR296 fixture: block with one no-argument member call.
  This has no call-result assignment, so do not claim new ReturnFlow/dataflow gain.
  Check actual CPG Call/Return edges and their disappearance on replacement.
- Full observer/default/MCP/explicit compiler audit/helpers/authority and clippy,
  plus same-worktree release rebuild and Tier-A matrix/quick. No full-corpus run,
  silent rebaseline or claim that private query tests are an activated MCP server.
- Two review rounds maximum. Closed findings receive bounded fixes; open-class
  findings at cap stop activation and require a design decision.

Value checkpoint: prove consumer parity and ephemeral custody while disabled.
Then a smaller activation slice can select and test exact CLI/MCP opt-in arguments,
failure reporting and refresh publication without reopening provenance grammar.
