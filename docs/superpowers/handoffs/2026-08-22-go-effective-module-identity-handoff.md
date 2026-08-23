# Handoff — #14 slice 3 effective Go module identity

**Written:** 2026-08-23T03:25:17Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-p14c-module-graph` · `go-effective-module-identity` · **Measured state:** `[MEASURED]` HEAD `b5709c4591ce5ceea397c49c01dc327dba3cd95b` · Tree DIRTY only for this untracked handoff · Probe `git status --short --branch && git rev-parse HEAD` · Output inline in the active Codex session
**Predecessor:** none — first in lane
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — another session/agent alive in this lane? `[MEASURED]` No subagents were dispatched; this Codex session owns the lane — **RESOLVED 2026-08-23T03:25:17Z**
**(b) Custody exposure** — unpushed commits, uncommitted work, single-copy/untracked artifacts: `[MEASURED]` six local commits `17ac03c..b5709c4` are unpushed; this handoff is the only untracked artifact before its custody commit — **OPEN until this handoff is committed**
**(c) In flight / irreversible** — running process, held lock, half-applied migration: `[MEASURED]` no call-stats, build, or test process remained in flight — **RESOLVED 2026-08-23T03:25:17Z**
**(d) Authorization granted but not exercised** — "Strict TDD (commit per step; no amend; no push)"; do not push.

## 1. Resume order

1. Obtain the owner's binding choice for Prometheus: preserve strict `module.CheckPath` and accept `workspace_invalid`, or treat a Go-accepted dotless active-main path such as `module compliance` as valid so the promised five-module active set can hold.
2. If the owner chooses the five-module behavior, add a red regression for the Prometheus shape, implement the narrowly authorized validation exception, then rerun focused tests, full `cargo test`, Clippy, release build, and all five call-stats pairs.
3. Run Tier-A matrix/quick and the etcd/prometheus baseline-delta oracle only in an environment allowed to access the uv cache, or after an approved Python 3.12+ eval environment is supplied.
4. Do not amend or push; refresh this living handoff after the owner decision and any resulting commit.

**STOP conditions:** Do not silently choose between contradictory owner-approved requirements, weaken module-path validation generally, re-baseline Tier-A, change Go comparator/alias behavior, or push.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Workspace construction | done | `[MEASURED]` commit `17ac03c`; focused construction suite 8/8 green |
| Replacement resolution | done | `[MEASURED]` commit `8511ae8`; graph suite 17/17 green |
| Memoized identity and call-graph integration | done | `[MEASURED]` commit `a33a9f9`; graph suite 21/21 and integration pole green |
| Positive and negative dispatch poles | done | `[MEASURED]` commit `fec66a3`; `owner_partition_fix_wave_test` 30/30 green |
| Telemetry, conservation, cache pins, parity | done | `[MEASURED]` commit `173f92e`; call-stats 16/16, cache pins 2/2, and full/incremental/cached multi-module parity green |
| Clippy correction | done | `[MEASURED]` commit `b5709c4`; removed the only new warning, focused graph tests 21/21 green |
| Full Rust suite | done | `[MEASURED]` final `cargo test`: 3,273 passed, 0 failed, 1 ignored; `cargo test -- --list` enumerated 3,274 |
| Static/release gates | done | `[MEASURED]` `cargo fmt`, `git diff --check`, `cargo clippy --all-targets --all-features`, and `cargo build --release` exited 0; touched-file extraction showed no new warning |
| Tier-A matrix/quick | blocked | `[MEASURED]` uv failed before harness start because `/Users/wesleyjinks/.cache/uv` was sandbox-denied; escalation was declined; no repo `.venv` or Python 3.12+ exists |
| Corpus measurements | blocked | `[MEASURED]` all five pairs ran, but Prometheus refuted the promised active set because the inherited CheckPath rule rejects `module compliance` |
| Hardened oracle | blocked | `[MEASURED]` gopls v0.22.0 exists, but the required Python 3.12+/uv harness is unavailable; not run |

Measured control/candidate projection (`interface_dispatch`, `typed_param`, `field_typed`, `return_typed`; then `QualifiedTypeIdentity`, `NonLocalConstructionFallback`, multi-target interface dispatch):

| Corpus | Control → candidate Exact | QTI | NLCF | Multi | Candidate graph | Proven / unproven |
|---|---|---:|---:|---:|---|---|
| ripgrep | `0/315/273/1269 → same` | `0→0` | `0→0` | `0→0` | omitted, non-Go | omitted |
| caddy | `1766/107/15/233 → same` | `0→0` | `3→3` | `17→17` | `modules=1 active=1 parsed=0 applied=0 invalid=false` | `312 / 0` |
| prometheus | `2461/770/125/2135 → 2364/770/125/2135` | `0→0` | `143→171` | `414→371` | `modules=4 active=0 parsed=0 applied=0 invalid=true` | `0 / 715`, all `workspace_invalid` |
| etcd | `1742/230/38/1064 → 2002/230/38/1064` | `0→0` | `503→578` | `182→315` | `modules=13 active=13 parsed=36 applied=0 invalid=false` | `1095 / 0` |
| hugo | `625/440/92/2927 → same` | `0→0` | `330→330` | `83→83` | `modules=2 active=1 parsed=0 applied=0 invalid=false` | `897 / 0` |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Owner step-4 dispatch pole | "root interface bare `Context` ↔ nested `root.Context`" becomes empty without go.work | `[MEASURED]` The root-side bare type never consults nested file identity, so removing go.work does not empty that match. The regression pole uses root qualified `nested.Context` versus nested bare `Context`, preserving the intended identity mechanism and target file. |
| Owner measurement expectation | Prometheus has five active, fully valid modules while the inherited `module.CheckPath` validator must be reused | `[MEASURED]` Go 1.26.2 `go work edit -json` and `go list -m` accept five mains, including dotless `module compliance`; `golang.org/x/mod/module.CheckPath` rejects it with "missing dot in first path element". Literal CheckPath therefore yields four valid declarations and whole-workspace invalidity. |
| Branch spec §4 cache line | Slice 3 says CPG 47 / sidecar 16 | `[INHERITED]` The direct owner brief overrides it with the one transition 45/14 → 46/15, which is what commits and pins implement. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Prometheus semantic decision | blocked | Owner selects literal CheckPath or Go-workspace-compatible dotless active-main handling | Contradictory acceptance requirements | `module compliance`, `modules=4 active=0` |
| 2 | Tier-A matrix/quick | blocked | Re-run after uv cache access or Python 3.12+ eval env is authorized | Sandbox permission declined | `uv run tier-a` |
| 3 | Etcd/prometheus oracle | blocked | Generate paired manifests and run `tools/dispatch_oracle.py --baseline` | Same eval environment; Prometheus semantics first | etcd `+260` interface Exact |
| 4 | Final checkpoint 6 | pending | After 1–3, refresh handoff and commit without amend/push | Items 1–3 | branch ahead 6 before handoff commit |

## 5. Invariants and traps — do not do these

- Never re-read go.mod or go.work from disk during graph construction — the loader snapshot is the sole source.
- Never fall through from an unusable winning replacement to a lower-precedence directive — opacity is fail-closed.
- Never change Go comparator or alias behavior in slice 3 — that belongs to slice 4.
- Never infer inactivity from a bare type on the opposite side of comparison — the literal root-bare step-4 pole cannot test nested identity.
- Never treat Go command acceptance and `module.CheckPath` as equivalent for a dotless workspace main — Prometheus proves they differ.
- A `#[cfg(test)]` expression embedded in a struct literal is rejected here → keep the parse-count hook structurally present and expose it only to tests.
- The supplied control binary was stale at `264c8ef`; use the measured exact rebuild in `/private/tmp`, not that path's old bytes.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Workspace | `/Users/wesleyjinks/code/slicing-p14c-module-graph` |
| Branch | `go-effective-module-identity` |
| Base | `8444b44c4269239c6f797bb2daaa40b2a54353ac` |
| Current checkpoint | `b5709c4591ce5ceea397c49c01dc327dba3cd95b` |
| Exact control binary | `/private/tmp/prism-main-8444-target/release/prism` → `slicing 3.1.2 (8444b44c4269)` |
| Supplied stale control binary | `/Users/wesleyjinks/code/slicing/target/release/prism` → `slicing 3.1.2 (264c8efefc1d)` |
| Candidate binary | `/Users/wesleyjinks/code/slicing-p14c-module-graph/target/release/prism` → `slicing 3.1.2 (b5709c4591ce)` |
| Spec | `docs/superpowers/specs/2026-08-22-go-nested-module-import-identity-design.md` v4 |
| Handoff | `docs/superpowers/handoffs/2026-08-22-go-effective-module-identity-handoff.md` |

## 7. Refutation verdict and owner questions

**§2c verdict:** REFUTED — corrected in place · claim: "literal CheckPath validation yields the promised Prometheus five-module active workspace" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: five-corpus call-stats measurement plus Go 1.26.2 `go work edit -json` / `go list -m` probes recorded in this handoff

**Questions the owner owes an answer to:** 1. Should `module compliance` remain malformed under literal `module.CheckPath`, accepting Prometheus `workspace_invalid` and the 97 Exact loss, or should active main modules follow Go workspace acceptance so all five become active? 2. Should the standing step-4 root-bare pole be corrected to the mechanism-equivalent qualified-root pole? 3. Should a future runner be allowed uv-cache access to complete Tier-A and the hardened etcd/prometheus oracle?
