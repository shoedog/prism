# Handoff — #14 slice 3 effective Go module identity

**Written:** 2026-08-23T04:58:55Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-p14c-module-graph` · `go-effective-module-identity` · **Measured code/spec state:** `[MEASURED]` HEAD `9c97d0287f94` · Tree CLEAN before this handoff refresh · Probe `git rev-parse HEAD && git status --porcelain=v1` · Output inline in the active Codex session
**Predecessor:** this handoff at controller checkpoint `c5f89b7`
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[CONTROLLER]` claims are the controller evidence supplied with fix wave 2; `[INHERITED]` claims were not re-probed.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — another session/agent alive in this lane? `[MEASURED]` No subagents were dispatched; this Codex session owns the lane — **RESOLVED 2026-08-23T04:58:55Z**
**(b) Custody exposure** — unpushed commits, uncommitted work, single-copy/untracked artifacts: fix-wave-2 commits `9e678d6`, `75ab9a9`, `9c97d02`, and this dedicated handoff commit are unpushed; no uncommitted artifact remains after the handoff commit — **RESOLVED by the dedicated handoff commit; controller owns push**
**(c) In flight / irreversible** — running process, held lock, half-applied migration: `[MEASURED]` no call-stats, build, test, Tier-A, or clippy process remains in flight — **RESOLVED 2026-08-23T04:58:55Z**
**(d) Authorization granted but not exercised** — tests first, commit per item, no amend, no push; do not push.

## 1. Resume order

1. No required fix-wave-2 code, documentation, or gate remains: W1 is `9e678d6`, W2 is `75ab9a9`, and the owner-approved oracle exception is `9c97d02`.
2. Controller should verify this handoff commit, then push without amending any per-item commit.
3. Do not change corpus numbers, cache versions 46/15, amend, or push from this worker lane.

**STOP conditions:** Do not extend `CheckImportPath` semantics to inactive discovered modules or dependency paths, loosen the semver port, re-baseline Tier-A/corpora, change Go comparator/alias behavior, amend, or push.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Workspace construction | done | `[MEASURED]` commit `17ac03c`; focused construction suite 8/8 green |
| Replacement resolution | done | `[MEASURED]` commit `8511ae8`; graph suite 17/17 green |
| Memoized identity and call-graph integration | done | `[MEASURED]` commit `a33a9f9`; graph suite 21/21 and integration pole green |
| Positive and negative dispatch poles | done | `[MEASURED]` commit `fec66a3`; `owner_partition_fix_wave_test` 30/30 green |
| Telemetry, conservation, cache pins, parity | done | `[MEASURED]` commit `173f92e`; call-stats 16/16, cache pins 2/2, and full/incremental/cached multi-module parity green |
| Clippy correction | done | `[MEASURED]` commit `b5709c4`; removed the only new warning, focused graph tests 21/21 green |
| Go-faithful path split | done | `[MEASURED]` `d69da41` added `PathKind::{MainModule,Dependency}` plus Prometheus/malformed/dependency reds; `7ee910b` bounded `MainModule` to the root/default and normalized `go.work use` active set after a Hugo negative control found the over-broad first attempt |
| Controller c5 checkpoint | done | `[CONTROLLER]` `c5f89b7`: 3,278 passed / 0 failed / 1 ignored; Tier-A 104/104; five-corpus leaf diffs below; oracle caddy/Prometheus/Hugo `gate_ok=true`, etcd `gate_ok=false`, 375 newly exact = 370 sound + 5 roadmap-#17 over-approx, 0 lost-exact |
| W1 active-main replacement precedence | done | `[MEASURED]` `9e678d6`; active paths are excluded after per-module duplicate validation but before module-union conflict detection; go.work override precedence is unchanged |
| W2 x/mod semver validation | done | `[MEASURED]` `75ab9a9`; one byte-level x/mod `parse` port covers require, exclude, replace LHS, and replace RHS; active malformed versions invalidate the workspace, inactive malformed versions stay subtree-local |
| Resolver + manifest target-file parity | done | `[MEASURED]` `active_main_wins_before_cross_module_replace_conflict_with_target_file_parity` proves `b/impl.go`, parsed/applied `2/0`, invalid false; `invalid_active_root_version_blocks_resolver_and_manifest_targets` proves empty resolver + manifest targets, zero proven files, and workspace invalid |
| Owner acceptance amendment | done | `[MEASURED]` `9c97d02`; §4 records the 2026-08-22 owner exception and binds #17-narrow to `oracle-s3b-etcd.json` |
| Full Rust suite | done | `[MEASURED]` final `cargo test --quiet`: 3,283 passed, 0 failed, 1 ignored across all harness summaries |
| Static/release gates | done | `[MEASURED]` `cargo fmt --all -- --check`, `git diff --check`, `cargo clippy --all-targets --all-features`, and `cargo build --release` exited 0; clippy has repository warnings but none in a fix-wave-2 file |
| Tier-A matrix | done with local fallback | `[MEASURED]` exact `uv run` could not start because outside-workspace access was declined; immediate post-release `eval/.venv/bin/tier-a --matrix-only --allow-stale-sut` produced 104/104 ok (Rust 31, Go 23, Python 27, JavaScript 5, TypeScript 18) |
| Corpus measurements | done | `[MEASURED]` candidate `9c97d0287f94` versus existing `ctrl8444-*`; all five commands exited 0 and every candidate output is raw-byte and structurally identical to `s3b-*` |

Controller/base projection retained byte-for-byte by the measured fix-wave-2 candidate (`interface_dispatch`, `typed_param`, `field_typed`, `return_typed`; then `QualifiedTypeIdentity`, `NonLocalConstructionFallback`, multi-target interface dispatch):

| Corpus | Control → candidate Exact | QTI | NLCF | Multi | Candidate graph | Proven / unproven |
|---|---|---:|---:|---:|---|---|
| ripgrep | `0/315/273/1269 → same` | `0→0` | `0→0` | `0→0` | omitted, non-Go | omitted |
| caddy | `1766/107/15/233 → same` | `0→0` | `3→3` | `17→17` | `modules=1 active=1 parsed=0 applied=0 invalid=false` | `312 / 0` |
| prometheus | `2461/770/125/2135 → same` | `0→0` | `143→143` | `414→414` | `modules=5 active=5 parsed=2 applied=0 invalid=false` | `715 / 0`, reasons `{}` |
| etcd | `1742/230/38/1064 → 2002/230/38/1064` | `0→0` | `503→578` | `182→315` | `modules=13 active=13 parsed=36 applied=0 invalid=false` | `1095 / 0` |
| hugo | `625/440/92/2927 → same` | `0→0` | `330→330` | `83→83` | `modules=2 active=1 parsed=0 applied=0 invalid=false` | `897 / 0` |

Prometheus's former `-97` interface Exact, `+28` NLCF, and `-43` multi-target deltas were entirely the inherited malformed-workspace effect and disappear with five valid active mains. Etcd retains the first measurement's `+260` interface Exact, `+75` NLCF, and `+133` multi-target results from effective 13-module workspace identity; caddy and Hugo remain unchanged. All Go candidate corpora have zero unproven files and empty reason maps.

`[CONTROLLER]` Hardened oracle at c5: caddy, Prometheus, and Hugo have `gate_ok=true` with 0 newly-exact sites. Etcd has `gate_ok=false`: 375 newly-exact sites = 370 sound + 5 over-approx at `cache_test.go:1383/1559 Get`, `revision_test.go:114/126 Client`, and `v3_failover_test.go:93 Endpoints`; 0 lost-exact. The owner accepted these five as the tracked roadmap-#17 concrete-receiver class surfaced by recovered identity.

Current line counts: `src/go_module_graph.rs` 576 (572 at c5; never 528), `replacements.rs` 186, `semver.rs` 88, `go_module_graph_fix2_tests.rs` 77, and `tests/lang/go/module_graph_fix2_test.rs` 142. Every fix-wave-2 file is below 600 lines.

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Owner step-4 dispatch pole | "root interface bare `Context` ↔ nested `root.Context`" becomes empty without go.work | `[MEASURED]` The root-side bare type never consults nested file identity, so removing go.work does not empty that match. The regression pole uses root qualified `nested.Context` versus nested bare `Context`, preserving the intended identity mechanism and target file. |
| Owner measurement expectation | Prometheus has five active, fully valid modules while the inherited `module.CheckPath` validator must be reused | `[MEASURED]` Go 1.26.2 `go work edit -json` and `go list -m` accept five mains, including dotless `module compliance`; `golang.org/x/mod/module.CheckPath` rejects it with "missing dot in first path element". Literal CheckPath therefore yields four valid declarations and whole-workspace invalidity. |
| First wave-1 implementation | Every discovered `go.mod` module directive used `CheckImportPath` | `[MEASURED]` Hugo exposed inactive dotless `gohugoio/hugo/internal/warpc/genavif`, changing `modules=2→3`. RED control plus `7ee910b` moved active-set selection ahead of one-pass parsing: active root/uses carry `MainModule`; inactive manifests carry `Dependency`, restoring Hugo `modules=2`. |
| Branch spec §4 cache line | Slice 3 says CPG 47 / sidecar 16 | `[INHERITED]` The direct owner brief overrides it with the one transition 45/14 → 46/15, which is what commits and pins implement. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Required fix-wave-2 work | done | Controller verification and push only | — | `9e678d6`, `75ab9a9`, `9c97d02`, 3,283/0/1, 104/104, five corpora identical to s3b |
| 2 | Exact `uv run` matrix invocation | environment-refused | None needed for implementation; controller may repeat exact command | outside-workspace access was declined | repo-local fallback is 104/104 |

## 5. Invariants and traps — do not do these

- Never re-read go.mod or go.work from disk during graph construction — the loader snapshot is the sole source.
- Never fall through from an unusable winning replacement to a lower-precedence directive — opacity is fail-closed.
- Never change Go comparator or alias behavior in slice 3 — that belongs to slice 4.
- Never infer inactivity from a bare type on the opposite side of comparison — the literal root-bare step-4 pole cannot test nested identity.
- Never treat Go command acceptance and `module.CheckPath` as equivalent for a dotless workspace main — Prometheus proves they differ.
- Never treat every discovered `go.mod` as active — inactive paths retain `CheckPath`; Hugo's `gohugoio/.../genavif` is the negative control.
- Never replace x/mod semver parsing with a `starts_with('v')` approximation; numeric leading zeros, prerelease identifiers, build identifiers, and shortened versions have distinct rules.
- A `#[cfg(test)]` expression embedded in a struct literal is rejected here → keep the parse-count hook structurally present and expose it only to tests.
- The supplied control binary is now verified at `8444b44c4269`; bind measurements to `--version`, not path alone.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Workspace | `/Users/wesleyjinks/code/slicing-p14c-module-graph` |
| Branch | `go-effective-module-identity` |
| Base | `8444b44c4269239c6f797bb2daaa40b2a54353ac` |
| Current code/spec checkpoint | `9c97d0287f94094af743bb3bb4e2d059a564e255` |
| Validation split commit | `d69da41` |
| Active-only correction commit | `7ee910b` |
| W1 precedence commit | `9e678d6` |
| W2 semver commit | `75ab9a9` |
| Oracle exception commit | `9c97d02` |
| Exact control binary | `/Users/wesleyjinks/code/slicing/target/release/prism` → `slicing 3.1.2 (8444b44c4269)` |
| Candidate binary | `/Users/wesleyjinks/code/slicing-p14c-module-graph/target/release/prism` → `slicing 3.1.2 (9c97d0287f94)` |
| Spec | `docs/superpowers/specs/2026-08-22-go-nested-module-import-identity-design.md` v4 |
| Handoff | `docs/superpowers/handoffs/2026-08-22-go-effective-module-identity-handoff.md` |

## 7. Verdict and owner questions

**Fix-wave-2 verdict:** PASS — both constructible WRONGs are fixed in place with pre-change reds, resolver/manifest target-file parity, full-suite/static gates, Tier-A matrix fallback, and five-corpus byte identity to s3b · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED + CORPUS-MEASURED

**Questions the owner owes an answer to:** none. Controller repeats the exact `uv run` matrix if that invocation form, rather than the equivalent repo-local executable, is a hard custody requirement.
