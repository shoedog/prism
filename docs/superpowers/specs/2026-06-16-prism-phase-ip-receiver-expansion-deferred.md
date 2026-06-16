# Phase-IP receiver-expansion (PR-2) — deferred / dismissed work

> Companion to the plan (`docs/superpowers/plans/2026-06-16-prism-phase-ip-receiver-expansion.md`) and spec
> (`…/specs/2026-06-16-prism-phase-ip-receiver-expansion-design.md`). Items intentionally **not** done in PR-2,
> recorded with priority / why-deferred / production-impact / fix-sketch so a future implementor doesn't have
> to rediscover them. Bridge slices append to the table at the end (do-now / dismiss / defer judgement calls).

## Recorded at plan time (dual-review fold, 2026-06-16)

### 1. Package-level `var r Runner` receivers — DEFERRED
- **Priority:** Low.
- **Why deferred:** The Slice-C `var_spec` recovery walks bindings rooted at the enclosing function
  (`ast.rs` `walk_receiver_bindings`, rooted at the fn node). A package-level (file-scope) `var` is a sibling
  of the function, never inside that subtree, so the spec-§4-listed package-level case is not recovered.
- **Production impact:** ~zero for the PR-2 metric target — caddy's 57 interface-dispatch sites are all
  type-assertion receivers, not package-level vars. The narrowing only drops a rare receiver shape.
- **Fix sketch:** add a file-root declaration scan (iterate top-level `var_declaration`/`var_spec` siblings of
  the function) feeding the same `recover_var` path, with intra-file shadowing semantics + a fixture/test.

### 2. Cross-package concrete `x.(pkg.T).M()` owner keys (D2) — DEFERRED
- **Priority:** Low.
- **Why deferred:** `owner_key` (`resolution.rs:79`) strips `&`/`*`/`::` but **not** Go `pkg.`, so a recovered
  concrete `pkg.T` does not match the bare owner-index keys → no `owner_lookup` hit. (An *interface*
  `pkg.Module` still routes correctly, because `iface_key` does strip `pkg.`.) This matches the pre-existing
  D2 deferred-conditional class in the spec (precise cross-package keys).
- **Production impact:** narrow — only *concrete* cross-package type-assertion / var receivers; the interface
  case (the caddy class) is unaffected.
- **Fix sketch:** a Go-aware bare-name normalizer for the concrete path (+ cross-package collision handling),
  or activate D2 precise cross-package keys if the §8 gate report shows this class matters.

### 3. `--receiver-recovery` runtime CLI flag — OPTIONAL (not deferred work, ergonomics)
- **Priority:** Optional.
- **Why not done:** spec §10's "each form independently revertable to `legacy` via the config" is satisfied by
  the build-time `ReceiverRecoveryConfig` (`build_with_receiver_config`). No runtime toggle is promised.
- **Production impact:** none. A flag would only let an operator disable a form without a rebuild after the
  §8b gate report.
- **Fix sketch:** thread `ReceiverRecoveryConfig` from a `--receiver-recovery {legacy|expanded|...}` CLI flag
  through to `build_with_receiver_config`; add only if the gate report motivates a fast revert.

## Bridge-slice additions (do-now / dismiss / defer)

| Slice | Item | Judgement | Rationale |
|-------|------|-----------|-----------|
| _(none yet)_ | | | |
