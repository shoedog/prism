# Verification — #16-C1 Slice-0 census

## v8 re-census

The v8 mandatory floor passes after applying the intrinsic Go `*_test.go`
exclusion exactly as specified. The Slice-0 STOP rule therefore did not fire.
This is still a temporary census probe, not the production C1 implementation.

The re-census also found one non-floor result requiring owner disposition:
Hugo `internal/warpc/warpc.go:204 time.Timer.Stop` now survives. Its locally
resolved `tpl/time.Timer` identity is absent from the all-interface universe,
so v8 routes through `identity_invalid_fallback` to the unique recorded
`tpl/debug.Timer` and mints `debug.nopTimerImpl` plus `debug.timer`. This is one
of the eight known over-approx sites, so only seven of eight remain killed.

### Commands and results

- `cargo test go_bare_interface_identity_validation_tests --lib`
  - RED before v8 implementation: the invalid-fallback route and validation bit
    did not exist.
  - GREEN after v8 implementation and edge completion: 4 passed, 0 failed.
    These tests pin unique recovery, zero-identity drop, collision drop, and the
    validated-non-dispatchable no-fallback negative.
- `cargo build --release`
  - PASS immediately before the corpus run.
- `env PRISM_C1_CENSUS=1 target/release/prism nav interface-manifest --repo /Users/wesleyjinks/code/bench-repos/caddy > /private/tmp/c1-v8-caddy.json`
  - PASS: 91 R3-eligible attempts.
- `env PRISM_C1_CENSUS=1 target/release/prism nav interface-manifest --repo /Users/wesleyjinks/code/bench-repos/prometheus > /private/tmp/c1-v8-prometheus.json`
  - PASS: 2,309 R3-eligible attempts.
- `env PRISM_C1_CENSUS=1 target/release/prism nav interface-manifest --repo /Users/wesleyjinks/code/bench-repos/etcd > /private/tmp/c1-v8-etcd.json`
  - PASS: 3,716 R3-eligible attempts.
- `env PRISM_C1_CENSUS=1 target/release/prism nav interface-manifest --repo /Users/wesleyjinks/code/bench-repos/hugo > /private/tmp/c1-v8-hugo.json`
  - PASS: 1,485 R3-eligible attempts.
- `cargo test --quiet`
  - FULL SUITE PASS: 3,439 passed, 0 failed, 1 ignored.
- `cargo fmt --all -- --check`
  - PASS.
- `git diff --check`
  - PASS.
- `cd eval && uv run tier-a --matrix-only --allow-stale-sut`
  - NOT RUN: host-level access to the managed `uv` cache was rejected.

## Verified

All corpus commands exited zero and emitted parseable JSON. The attempted nav
cache writes outside the workspace were refused, but each command rebuilt and
completed; no result depends on a cache-save success.

### Floor by corpus

- **Caddy — PASS.** At all 11 `caddy.Module.CaddyModule` sites, the 120
  currently minted non-test package-qualified implementer identities are
  preserved. `integration.MockDNSProvider` is the sole loss and is excluded
  because its entire method set is defined in
  `caddytest/integration/mockdns_test.go`. C1 additionally restores six
  package-qualified implementer identities, yielding 126 identities; the
  legacy bare-name `fanout` field reads 121 → 120 because equal bare names are
  deduplicated. All seven `caddy.StorageConverter.CertMagicStorage` sites
  preserve `filestorage.FileStorage` exactly.
- **Prometheus — PASS (no named mandatory-floor members).** All losses are
  enumerated below.
- **etcd — PASS.** All 24 `AuthBackend` sites route through
  `identity_invalid_fallback` and mint `schema.authBackend`. The only omitted
  current implementer is `auth.backendMock`, whose complete `AuthBackend`
  method set is in `server/auth/store_mock_test.go`; it is reported and excluded
  from the floor.
- **Hugo — PASS (no named mandatory-floor members).** Its non-floor losses and
  surviving known over-approx are enumerated below.

### etcd-24 two-route accounting

Every row has `identity_valid=false`, `identity_count=1`, route
`identity_invalid_fallback`, selected interface `auth.AuthBackend` declared at
`server/auth/store.go:197`, and final minted implementer set
`{schema.authBackend}`.

| Site | Method |
|---|---|
| `server/storage/schema/auth_roles_test.go:118` | `BatchTx` |
| `server/storage/schema/auth_roles_test.go:116` | `CreateAuthBuckets` |
| `server/storage/schema/auth_roles_test.go:123` | `ForceCommit` |
| `server/storage/schema/auth_roles_test.go:129` | `GetAllRoles` |
| `server/storage/schema/auth_roles_test.go:217` | `BatchTx` |
| `server/storage/schema/auth_roles_test.go:215` | `CreateAuthBuckets` |
| `server/storage/schema/auth_roles_test.go:222` | `ForceCommit` |
| `server/storage/schema/auth_roles_test.go:228` | `GetRole` |
| `server/storage/schema/auth_test.go:58` | `BatchTx` |
| `server/storage/schema/auth_test.go:72` | `BatchTx` |
| `server/storage/schema/auth_test.go:59` | `CreateAuthBuckets` |
| `server/storage/schema/auth_test.go:66` | `ForceCommit` |
| `server/storage/schema/auth_test.go:112` | `BatchTx` |
| `server/storage/schema/auth_test.go:123` | `BatchTx` |
| `server/storage/schema/auth_test.go:109` | `CreateAuthBuckets` |
| `server/storage/schema/auth_test.go:117` | `ForceCommit` |
| `server/storage/schema/auth_users_test.go:106` | `BatchTx` |
| `server/storage/schema/auth_users_test.go:104` | `CreateAuthBuckets` |
| `server/storage/schema/auth_users_test.go:111` | `ForceCommit` |
| `server/storage/schema/auth_users_test.go:117` | `ReadTx` |
| `server/storage/schema/auth_users_test.go:193` | `BatchTx` |
| `server/storage/schema/auth_users_test.go:191` | `CreateAuthBuckets` |
| `server/storage/schema/auth_users_test.go:198` | `ForceCommit` |
| `server/storage/schema/auth_users_test.go:204` | `GetUser` |

Per implementer:

- `schema.authBackend` — survives at every site. Its complete method set is in
  non-test files under `server/storage/schema/`; its unexported concrete type
  satisfies the exported-method-only `auth.AuthBackend` across the package
  boundary.
- `auth.backendMock` — not minted. It is same-package to the selected interface,
  but every file defining its method set is `_test.go`, so it is a reported
  exclusion rather than a floor failure.

### Eight known over-approx dispositions

- Prometheus `storage/remote/codec.go:295 Put` — killed by
  `walk_conflict_drop`.
- Prometheus `util/zeropool/pool_test.go:{133,137,150,155,168,172} Put` — all
  six killed by `walk_conflict_drop`.
- Hugo `internal/warpc/warpc.go:204 Stop` — **survives** via
  `identity_invalid_fallback` and fallback walk, minting
  `{debug.nopTimerImpl, debug.timer}`. This is not silently accepted; it needs
  explicit owner disposition before the census acceptance gate can close.

### Named mechanism site

Caddy `metrics.go:56 http.Handler.ServeHTTP` changes from
`{caddyhttp.HandlerFunc}` to the empty set through `collision_drop`; the
all-identities index contains two `Handler` interface identities.

### Non-floor subset-loss census

- Caddy: `metrics.go:56 ServeHTTP` loses non-test
  `caddyhttp.HandlerFunc` outright by collision, as above.
- Prometheus: `cmd/promtool/tsdb.go:{589,592,593}` loses only
  `index.postingsFailingAfterNthCall` (6 → 5); its entire method set is in
  `tsdb/index/postings_test.go`, so it is a reported test-file exclusion.
- Prometheus: `prompb/io/prometheus/client/decoder.go:302 Add` loses non-test
  `schema.IgnoreOverriddenMetadataLabelScratchBuilder` (1 → 0) by walk
  conflict.
- Prometheus: `rules/group.go:813 Iterator` loses the four non-test
  implementers `promql.StorageSeries`, `storage.SeriesEntry`,
  `testhelpers.FakeHistogramSeries`, and `testhelpers.FakeSeries` (4 → 0) by
  walk conflict.
- Prometheus: the seven known `sync.Pool.Put` over-approx sites lose
  `chunkenc.pool` (1 → 0), as listed above.
- etcd: all 24 sites lose only test-file `auth.backendMock` (2 → 1); this is the
  floor exclusion described above.
- Hugo: `hugolib/content_map_page_assembler.go:{538,601}` loses only
  `hugolib.testContentNode` (6 → 5); its complete method set is in
  `hugolib/content_map_test.go`, so it is a reported test-file exclusion.

There are **no new subset-loss tuples versus the v7 census**. Relative to v7,
24 `schema.authBackend` tuples are recovered and the two Hugo `Stop` target
tuples are no longer losses; the latter change is the surviving over-approx,
not a precision improvement.

### Artifact contract

Each `*-per-attempt-records.json` contains the prior record shape plus
`identity_valid` and `route`. The four route populations are:

| Corpus | `validated_direct` | `identity_invalid_fallback` | `unique_index` | `missing` | `collision` |
|---|---:|---:|---:|---:|---:|
| Caddy | 0 | 24 | 18 | 48 | 1 |
| Prometheus | 7 | 387 | 15 | 1,900 | 0 |
| etcd | 0 | 202 | 101 | 3,412 | 1 |
| Hugo | 11 | 587 | 11 | 876 | 0 |

## Not verified

- **environment-limited — Excluded:** Tier-A matrix and quick runs. Host-level
  access to the managed `uv` cache was rejected, so neither command executed.
- **environment-limited — Excluded:** gopls oracle adjudication of Caddy's six
  newly restored qualified identities and the Hugo survivor. The managed `uv`
  environment was unavailable, system Python is 3.9 without `tomllib`, and the
  proposed sandbox-only compatibility shim was not authorized.
- No production bare-arm wiring, production telemetry, or cache-version change
  was made. The Rust probe and its four v8 tests remain worktree-only.

CPG remains 49 and the navigation sidecar remains 18.
