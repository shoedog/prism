# Handoff — #14 slice 3 effective Go module identity

**Written:** 2026-08-23T03:59:22Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-p14c-module-graph` · `go-effective-module-identity` · **Measured state:** `[MEASURED]` HEAD `7ee910b3fc0d` · Tree CLEAN before this handoff refresh · Probe `git status --short --branch && git log -4 --oneline` · Output inline in the active Codex session
**Predecessor:** this handoff at commit `c39104a`
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — another session/agent alive in this lane? `[MEASURED]` No subagents were dispatched; this Codex session owns the lane — **RESOLVED 2026-08-23T03:59:22Z**
**(b) Custody exposure** — unpushed commits, uncommitted work, single-copy/untracked artifacts: `[MEASURED]` nine local commits through `7ee910b` are unpushed; only this handoff refresh is uncommitted — **OPEN until this handoff is committed**
**(c) In flight / irreversible** — running process, held lock, half-applied migration: `[MEASURED]` no call-stats, build, test, or Tier-A process remains in flight — **RESOLVED 2026-08-23T03:59:22Z**
**(d) Authorization granted but not exercised** — "Strict TDD (commit per step; no amend; no push)"; do not push.

## 1. Resume order

1. No required wave-1 code or gate remains: the owner chose the Go-faithful split, commits `d69da41` and `7ee910b` implement it, and all requested measurements/checks are green.
2. If independently requested, Tier-A quick and the etcd/prometheus hardened oracle remain optional unrun follow-ups; neither was part of this wave's requested gate set.
3. Do not amend or push.

**STOP conditions:** Do not extend `CheckImportPath` semantics to inactive discovered modules or dependency paths, re-baseline Tier-A, change Go comparator/alias behavior, amend, or push.

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
| Focused parser/graph suite | done | `[MEASURED]` 29 passed, 0 failed; the pre-correction inactive Hugo-shaped control failed `modules 2 != 1` and is green after `7ee910b` |
| Full Rust suite | done | `[MEASURED]` final `cargo test --quiet`: 3,278 passed, 0 failed, 1 ignored across 28 harnesses |
| Static/release gates | done | `[MEASURED]` `cargo fmt`, `git diff --check`, `cargo clippy --all-targets --all-features`, and `cargo build --release` exited 0; touched-file extraction showed no new warning |
| Tier-A matrix | done | `[MEASURED]` repo-local Python 3.12.13 environment; `eval/.venv/bin/tier-a --matrix-only --allow-stale-sut` after the immediate release rebuild: 104/104 ok (Rust 31, Go 23, Python 27, JavaScript 5, TypeScript 18) |
| Corpus measurements | done | `[MEASURED]` final control `8444b44c4269` versus candidate `7ee910b3fc0d`; all five pairs exited 0; ripgrep full JSON byte-identical |
| Tier-A quick / hardened oracle | not run | Not requested in fix wave 1; no claim made for these optional checks |

Measured control/candidate projection (`interface_dispatch`, `typed_param`, `field_typed`, `return_typed`; then `QualifiedTypeIdentity`, `NonLocalConstructionFallback`, multi-target interface dispatch):

| Corpus | Control → candidate Exact | QTI | NLCF | Multi | Candidate graph | Proven / unproven |
|---|---|---:|---:|---:|---|---|
| ripgrep | `0/315/273/1269 → same` | `0→0` | `0→0` | `0→0` | omitted, non-Go | omitted |
| caddy | `1766/107/15/233 → same` | `0→0` | `3→3` | `17→17` | `modules=1 active=1 parsed=0 applied=0 invalid=false` | `312 / 0` |
| prometheus | `2461/770/125/2135 → same` | `0→0` | `143→143` | `414→414` | `modules=5 active=5 parsed=2 applied=0 invalid=false` | `715 / 0`, reasons `{}` |
| etcd | `1742/230/38/1064 → 2002/230/38/1064` | `0→0` | `503→578` | `182→315` | `modules=13 active=13 parsed=36 applied=0 invalid=false` | `1095 / 0` |
| hugo | `625/440/92/2927 → same` | `0→0` | `330→330` | `83→83` | `modules=2 active=1 parsed=0 applied=0 invalid=false` | `897 / 0` |

Prometheus's former `-97` interface Exact, `+28` NLCF, and `-43` multi-target deltas were entirely the inherited malformed-workspace effect and disappear with five valid active mains. Etcd retains the first measurement's `+260` interface Exact, `+75` NLCF, and `+133` multi-target results from effective 13-module workspace identity; caddy and Hugo remain unchanged. All Go candidate corpora have zero unproven files and empty reason maps.

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
| 1 | Required fix-wave work | done | None | — | `d69da41`, `7ee910b`, 104/104 matrix, five-corpus table |
| 2 | Optional Tier-A quick / hardened oracle | not run | Run only if independently requested | Not required by this wave | no result supplied |

## 5. Invariants and traps — do not do these

- Never re-read go.mod or go.work from disk during graph construction — the loader snapshot is the sole source.
- Never fall through from an unusable winning replacement to a lower-precedence directive — opacity is fail-closed.
- Never change Go comparator or alias behavior in slice 3 — that belongs to slice 4.
- Never infer inactivity from a bare type on the opposite side of comparison — the literal root-bare step-4 pole cannot test nested identity.
- Never treat Go command acceptance and `module.CheckPath` as equivalent for a dotless workspace main — Prometheus proves they differ.
- Never treat every discovered `go.mod` as active — inactive paths retain `CheckPath`; Hugo's `gohugoio/.../genavif` is the negative control.
- A `#[cfg(test)]` expression embedded in a struct literal is rejected here → keep the parse-count hook structurally present and expose it only to tests.
- The supplied control binary is now verified at `8444b44c4269`; bind measurements to `--version`, not path alone.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Workspace | `/Users/wesleyjinks/code/slicing-p14c-module-graph` |
| Branch | `go-effective-module-identity` |
| Base | `8444b44c4269239c6f797bb2daaa40b2a54353ac` |
| Current checkpoint | `7ee910b3fc0d` |
| Validation split commit | `d69da41` |
| Active-only correction commit | `7ee910b` |
| Exact control binary | `/Users/wesleyjinks/code/slicing/target/release/prism` → `slicing 3.1.2 (8444b44c4269)` |
| Candidate binary | `/Users/wesleyjinks/code/slicing-p14c-module-graph/target/release/prism` → `slicing 3.1.2 (7ee910b3fc0d)` |
| Spec | `docs/superpowers/specs/2026-08-22-go-nested-module-import-identity-design.md` v4 |
| Handoff | `docs/superpowers/handoffs/2026-08-22-go-effective-module-identity-handoff.md` |

## 7. Verdict and owner questions

**Fix-wave verdict:** PASS — owner-selected Go-faithful split implemented and corrected in place · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED + CORPUS-MEASURED · record: RED/GREEN controls, full suite, Tier-A matrix, and final five-corpus call-stats pairs recorded above

**Questions the owner owes an answer to:** none for fix wave 1. Tier-A quick and the hardened oracle require a new explicit request if desired.
