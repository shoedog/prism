# Plan 3a (nav cache) — deferred follow-ups

> **Status:** Legacy query-layer note. See `docs/prism-query-layer/README.md` for current routing.

Non-blocking findings from the Plan 3a final holistic review that were
**intentionally deferred** (they need a larger refactor or carry real risk).
The low-risk findings (per-repo namespacing of `--cache-dir`, I/O-failure
diagnostics, `--no-cache`/`--cache-dir` mutual exclusion, full digest) were
fixed in the Plan 3a PR. Bucketed by the follow-on slice that should own them.

## 3d — cache hardening (later)

### 1. Robust grammar/parser fingerprint  — **Important**
- **Why deferred:** v1's cache key uses the `tree-sitter-*` crate names+versions
  (build.rs fingerprint) + a manual `SKIP_POLICY_VERSION` + `prism_version`
  (`CARGO_PKG_VERSION`). A stronger fingerprint (hashing resolved grammar
  source/checksums or generated parser artifacts) is a meaningfully larger
  change.
- **Impact:** stale cache hits when something changes that the key doesn't
  capture: a vendored/patched grammar at the same version; same-version changes
  to Prism's own parsing/query/extraction code (`ast.rs`, `queries.rs`,
  `languages/`) that don't bump `CARGO_PKG_VERSION`; or a skip-policy change
  where someone forgets to bump `SKIP_POLICY_VERSION`. Mostly a dev-loop concern
  — the per-repo, content-hashed store + crate-version fingerprint cover the
  common `cargo update` and edit-source cases.
- **Fix sketch:** derive the key from (a) a hash of the generated parser
  artifacts / grammar checksums, and (b) an explicit `PARSER_BEHAVIOR_VERSION`
  bumped whenever AST/query/extraction semantics change; consider deriving
  `SKIP_POLICY_VERSION` from a hash of the skip rules rather than a hand-bumped
  constant.

### 2. Nav-specific cache version/wrapper  — **Low–Important**
- **Why deferred:** decoupling nav caching from the generic `cpg_cache`
  format/`CACHE_VERSION` (a nav-owned version + thin wrapper) is a moderate
  refactor; v1 reuses `cpg_cache` directly.
- **Impact:** nav correctness is coupled to the shared review-CPG cache
  semantics — a future change to `cpg_cache` (partial-hit behavior, scoped-CPG
  contract, format) could silently alter nav output. **Mitigated** today: nav
  uses a separate XDG store dir, so it's namespace-isolated from the per-diff
  cache; the shared surface is only the serialized format + version constant.
- **Fix sketch:** add a `NAV_CACHE_VERSION` and a thin nav cache wrapper that
  delegates graph (de)serialization to `cpg_cache` but owns its own validity
  contract and invalidation tests.

### 3. Hard error on explicit `--cache-dir` I/O failure  — **Low**
- **Why deferred:** a CLI-contract change (error vs. degrade) with some risk;
  v1 adds a stderr diagnostic but still falls back to a rebuild.
- **Impact:** when a user *explicitly* passes `--cache-dir` and the write fails
  (unwritable / disk full), the query still succeeds by rebuilding — silent
  degradation for automation that expects caching to be in effect.
- **Fix sketch:** when `--cache-dir` is explicitly supplied and `save_cache`
  fails, return a non-zero error instead of silently degrading; keep the
  degrade-with-warning behavior only for the default store.

### 4. Type-db identity in the cache key  — **Low for nav / Important for the per-diff cache**
- **Why deferred:** a cache-key robustness item (same class as #1); adding a
  type-db content/config fingerprint.
- **Impact:** the key validates `has_type_db` as a **boolean**, not the type-db
  identity/content, so typed analysis could reuse a stale CPG if the enrichment
  inputs (`compile_commands.json`, clang config) change while source hashes and
  the boolean stay the same. **Irrelevant to nav today** — `NavigationIndex`
  builds with `type_db = None`; this is a pre-existing gap in the per-diff
  (`--compile-commands`) cache path.
- **Fix sketch:** when `has_type_db`, hash the type-db inputs (compile-commands
  path + content/mtime, relevant config) into the cache key.

### 5. Cache store / policy boundary  — **Low**
- **Why deferred:** moving cache load/save/rebuild/diagnostics out of
  `NavigationIndex` construction into a small cache-store/policy abstraction is a
  moderate refactor; reinforces #2 (nav-cache wrapper).
- **Impact:** future cache policy, metrics, alternate stores, and the deferred
  nav-cache behavior are harder to evolve while embedded in `from_ctx` /
  `build_cached_*`.
- **Fix sketch:** a `CacheStore` boundary that owns load/save/invalidation and
  returns `Hit(cpg)` | `Rebuild(reason)`, keeping `NavigationIndex::from_ctx`
  focused on indexing.
