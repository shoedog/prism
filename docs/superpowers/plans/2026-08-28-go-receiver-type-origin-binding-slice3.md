# Go receiver type-origin binding — Slice 3 terminal owner predicate

**Base:** `7fc719ae21ba130c554c318c3f8306093a804c92` (`origin/main`, after Slice 2 implementation/custody/closeout PRs #209–#211)
**Design authority:** `docs/superpowers/specs/2026-08-24-go-receiver-type-origin-binding-design.md`, Slice 3 and §§5–7
**Predecessor handoff:** `docs/superpowers/handoffs/2026-08-27-go-receiver-type-origin-binding-slice2-handoff.md`

## 1. Outcome and boundary

Fail closed when a recovered Go receiver reaches an edge-authorizing consumer without a proven declaration owner:

1. Introduce one shared terminal predicate for a Go caller whose `receiver_type` and `receiver_recovery` are present while `receiver_owner_identity` is absent.
2. Consume that predicate immediately before receiver routing in both independent production consumers: `CallGraph::resolve_call_site_full` and `navigation::queries::interface_dispatch_manifest`.
3. Let the navigation call-edge index inherit the resolver verdict; it is not a third independent predicate consumer.
4. Preserve all Slice 1/2 positives: package variables, fields, returns, typed parameters, constructor locals, local `var` declarations, and type assertions already carry exact declaration-backed owners.
5. Preserve prerequisite/materialized drops whose receiver type was removed, direct calls without receiver recovery, `CallSite::cmp_key`, owner resolver signatures, and the legacy resolver for non-receiver consumers. Shadowed sites remain ownerless and edge-free but now terminate before the old collision-bail telemetry/manifest route.

This slice does not add inference, populate owners, alter interface implementation discovery, revive parked #16 behavior, or change the global basename fallback.

## 2. Constructible wrong result and RED

Use the existing cross-file alias fixture:

- `app/vars.go` defines `Shared` as `ext.I` with `ext -> api`.
- `app/use.go` calls `Shared.M()` with `ext -> decoy`.
- The correctly materialized site carries owner `api.I` and resolves only `api/types.go`.

Clone/mutate that real site by removing only `receiver_owner_identity`, leaving recovered text `ext.I`. Exact-base RED measurement narrowed the design's predicted asymmetry: the resolver already drops the mutation as `ExternalReceiver`, and the navigation edge index inherits that drop, but the manifest still emits an unauthorized oracle-facing record for the ownerless site (`dispatch_route: "unproven_drop"`, `fanout: 0`).

The parity matrix before production changes is:

1. Resolver: GREEN control — the ownerless mutation resolves no target and reports `ExternalReceiver`.
2. Manifest: RED — the ownerless stored site must be absent, but exact base emits the zero-fanout record.
3. Navigation sidecar: GREEN control — `callees` exposes neither `api/types.go` nor `decoy/types.go` evidence.
4. Cache fences: RED — test pins require CPG `54` and sidecar `22` while exact base remains `53`/`21`.

The unmodified owner-bearing fixture is the positive control and must continue resolving only `api/types.go` in resolver, manifest, and navigation output.

## 3. File boundary

Expected production files:

- `src/resolution.rs` — define the single predicate and replace the recovery-kind-specific ownerless guard with it.
- `src/navigation/queries.rs` — call the same predicate before the manifest's independent route/legacy ladder.
- `src/cpg_cache.rs` — topology fence `53 -> 54`, history, and pin.
- `src/navigation/call_edge_cache.rs` — paired topology fence `21 -> 22`, history, and pin.

Expected test files:

- `tests/lang/go/receiver_owner_carrying_test.rs` — resolver/manifest lost-provenance negative plus the existing owner-bearing positive.
- `tests/navigation/go_concrete_cache_test.rs` — navigation sidecar negative and positive parity.

No `CallSite` field, equality/key change, generated Tier-A artifact, oracle baseline, or new dispatch behavior is committed.

## 4. Predicate contract and order

The predicate is true exactly when:

1. `Language::from_path(site.caller.file) == Some(Language::Go)`;
2. `site.receiver_type.is_some()`;
3. `site.receiver_recovery.is_some()`; and
4. `site.receiver_owner_identity.is_none()`.

The type/recovery pair distinguishes a recovered receiver from direct/unqualified calls. The Go gate prevents a repository-wide semantic change. The missing owner is terminal because Slices 0–2 made every admissible local and cross-file producer declaration-backed; remaining absence is genuinely unproven.

Resolution checks the predicate before computing `receiver_resolution_kind` or invoking `go_concrete_receiver_route`. Manifest generation checks it after its existing Go/recovery/known-interface denominator and before `go_concrete_receiver_route` or any legacy bare-name lookup. Both produce the existing external/unproven drop vocabulary; no new telemetry bucket is required.

## 5. RED/GREEN order

1. Add only the resolver/manifest/navigation parity assertions and cache-pin expectations. Require the manifest and both pins to fail, with resolver/navigation controls already green, all with nonzero selected tests.
2. Add the shared predicate and consume it in the resolver and manifest.
3. Run the exact RED selectors GREEN plus Slice 0–2 positive/negative controls.
4. Bump both topology versions and run the four-path cache battery.
5. Run full repository, Accuracy Harness, exact-base corpus/oracle, and site-count gates.
6. Complete the declared two-round review cap, classifying every finding `WRONG` or `SMELL`.

## 6. Verification gates

Focused:

```bash
cargo test --test lang_go receiver_owner_carrying -- --nocapture
cargo test --test lang_go concrete_receiver_route -- --nocapture
cargo test --test lang_go concrete_receiver_fix4 -- --nocapture
cargo test --test lang_go receiver_origin_prereq -- --nocapture
cargo test --test navigation go_concrete_cache -- --nocapture
```

Repository:

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
git diff --check
```

`--allow-stale-sut` is valid only after the immediate release rebuild in this worktree. Any failing required gate needs an exact-base same-environment control before attribution. Do not rebaseline. Re-run the established five-corpus public-output comparison against this exact base, explain every removed receiver-provenance site/edge, and prove total `CallSite` occurrence parity. Recut the oracle join at the same base and report `sound`/`recall_gap`/`over_approx` movement.

The LSP navigation skill was required but its MCP tools are unavailable in this session. The production-consumer census therefore uses exhaustive bounded text references plus compiled behavioral tests; this exclusion remains explicit in the handoff.

## 7. Acceptance

- Both independent consumers call the same terminal predicate.
- An ownerless recovered Go site cannot invoke caller-file rebinding, legacy bare-interface lookup, or produce a navigation edge.
- The cross-file alias mutation remains dropped by resolver/sidecar and is absent from the manifest; no consumer exposes `decoy/types.go`.
- Every owner-bearing Slice 1/2 positive remains byte-for-byte equivalent in target identity and route.
- Prerequisite and direct-call behavior remains unchanged; shadowed recovered sites retain no owner/edge and are absent from the manifest at the new terminal boundary.
- CPG and sidecar versions are paired at `54` and `22`; all four cache paths agree.
- Full-suite totals, Tier-A results, five-corpus/oracle deltas, review verdicts, and exclusions are recorded in the living handoff.
