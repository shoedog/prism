# Handoff — Go Level-3 B1 real-corpus measurement

**Refreshed:** 2026-08-31 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-measure-go-b1-real-corpus` · `measure-go-b1-real-corpus`
**Exact starting tree:** `[MEASURED]` `8730298c900efad28eba7a4ff0d3c6321e310b69`, byte-identical to server `main` at `1bed1ab4d1df5b9fa1e0fbda8d87e04f8af30f5f` before this documentation change.

## 0. Verdict and authority

**CLOSED — a non-fixture real Go corpus row satisfies the shipped B1 authority.** Kubernetes module `k8s.io/pod-security-admission` contains `addCheck(CheckProcMountRestricted)` in ordinary production source. Prism proves the bare target, the strict `func() Check` signature, the singleton free-function HOF, and the direct `f()` invocation, then emits one exact `ParameterCallback` edge to `CheckProcMountRestricted`.

This closes only the evidence gap recorded after PR #221. It makes no runtime change and does not authorize B2–B5, methods, variadics, generics, tests, external named types, assignments, JS/TS, or any other language.

The steering carrier references `handoff-template.md`, which is absent on this machine. This handoff follows the established adjacent lane shape.

## 1. Exact accepted site

Corpus checkout: `/Users/wesleyjinks/code/bench-repos/kubernetes` at clean `8d34ac5ef882ce8ad3b3713f615f5947011e9730`; module root `staging/src/k8s.io/pod-security-admission` declares `module k8s.io/pod-security-admission`.

| Role | Exact source |
|---|---|
| Inbound call | `policy/check_procMount_restricted.go:36` — `addCheck(CheckProcMountRestricted)` |
| Exact target | `policy/check_procMount_restricted.go:40-56` — `func CheckProcMountRestricted() Check` |
| HOF | `policy/checks.go:154-162` — `func addCheck(f func() Check)` |
| Direct callback invocation | `policy/checks.go:156` — `c := f()` |
| Prism result | `1` accepted inbound site, `1` unique target, `1` Level-3 edge |

The module's independent compile/test control passed under `go version go1.26.2 darwin/arm64`: `go test ./policy` → `ok k8s.io/pod-security-admission/policy`.

## 2. Measurement population

The retained syntactic census first ranked additional real Go corpora without authorizing edges. Cobra, go-redis, Gin, Delve, go-github, and the Kubernetes checkout were inspected. Go-github had zero strict B1 rows, so its long-running cold Prism build was stopped as nondiscriminating. Kubernetes had `259` unambiguous non-test B1 floor rows; exact `go.mod` module roots were then measured before any full 17,240-file monorepo build.

Thirteen no-cache production `call-stats` runs produced:

| Metric | Total |
|---|---:|
| B1 candidates | `981` |
| Exact inbound sites | `93` |
| Accepted inbound sites | `3` |
| Unique targets | `3` |
| Level-3 edges | `3` |
| Stable named drops | `978` |

Every run conserved independently: `candidates == accepted + sum(drops)`. Delve supplied two accepted rows, but both are debugger programs under `_fixtures/`; they prove the production path can mint edges on a real checkout but are not the non-fixture authority for closing this gate. Pod-security-admission supplied the one accepted ordinary-source row. The other eleven runs accepted zero and failed closed with named reasons.

## 3. Binary and artifact custody

The measured binary reports `slicing 3.1.2 (f00a6a92fb1a)` and has SHA-256 `50543cf718b13525dba53058a5feae396a1a9c372236aaed16d864bb228f3a53`. Live Git resolves that checkpoint to `f00a6a92fb1a78d3b2fc9ff6d680fb75f9f2646b`. Subsequent local changes through the exact starting tree touch only the qualified-return handoff; runtime source is unchanged. PR #223 merged the verified runtime tree as `f8f3d02eccbc643337fbae03cf2ef7c1ef6f0dbf`.

Retained packet: `/Users/wesleyjinks/code/prism-lane-artifacts/2026-08-31-go-level3-b1-real-corpus/`

| Artifact | SHA-256 |
|---|---|
| `accepted-sites.jsonl` | `23c8277543107e0cac4e0679a8cb9655803a5db1059375d5cd69531996f800f2` |
| `call-stats-screen.jsonl` | `9fbcbb97e30a7a7cbf77ce2907d3eeef7778c3a169a314dcc4dbe313d0c93cd7` |
| `hof-sweep-results.json` | `a1bc29a92a1d7cfe80f2ab3d151f0f971f765e998a489294c018a4434c8ef274` |
| `hof-sweep-samples.md` | `fb209bfc57c3cd344f8e0ca982e812b2c67dad7d9bb75beaf3c7a1ffe02f3d47` |
| `hof-sweep-slots.tsv` | `c07fd1d62b28cbdfb931fdd1acd74aa4dc9c87a555e61fe0582221802f911d0a` |
| `kubernetes-modules-results.json` | `2b0021c27232577d7f21b552a71a37d2c27154bd85b80ff22e0a507326a0fd07` |
| `kubernetes-modules-samples.md` | `f4ea6000c870dc5389ed3e1779bad92301e8925a3e8b88e557d4e59b64c45056` |
| `kubernetes-modules-slots.tsv` | `656bb90533364c1d17a2575cbc77747b6ce48cda5b6bdde7ecf6ec879ee5c3c2` |

The census script is the retained `hof-sweep.go` from the original packet, SHA-256 `c7f975a9c298956429c44e8e96f1f19605ef0059b5969accc31e4029b4606416`.

## 4. Evidence limits and review

Declared measurement/documentation review cap: `2` rounds, plus one disclosed targeted custody extension. Round 1 found one `WRONG` in adjacent custody: the qualified-return handoff expanded the verified `f00a6a92` prefix to a non-existent full commit, yielding an unusable checkpoint identity. Live Git supplied the bounded correction to `f00a6a92fb1a78d3b2fc9ff6d680fb75f9f2646b`. Round 2 found one `SMELL`: baseline wording blurred the pre-fix full refresh with the post-fix quick Prism run; the scopes are now explicit. After the cap, rebinding the instructed next task exposed one closed, non-repeating `WRONG`: three handoffs and the roadmap directed a new return-flow implementation even though PR #193 had already shipped Step 5c. The same artifact was preserved and targeted in place: row #2 now records live merge `f4234013e00db266f76bb422fcb0850e23e42cb2`, and the next action is the strategic-fork decision brief. No finding remains open.

- The full Kubernetes monorepo was not cold-built. The accepted row was measured at its canonical generated `go.mod` module root, with the exact source checkout and module identity intact.
- `code-generator` and `client-go` were not run after the non-fixture positive closed the declared gate; no claim is made about their accepted populations.
- The corpus measurement changes no code, baseline, cache version, or authority boundary.
- The prior corpus-positive `SMELL` is resolved. No `WRONG` or in-scope `SMELL` remains open in the accepted row or custody reconciliation.
- Confidence collapses if the retained site cannot be reproduced at the exact Kubernetes revision and binary hash, if it fails conservation, or if its target/HOF/signature ceases to be unique.

## 5. Next exact steps

1. Preserve this measurement packet and exact starting-tree binding; publish only the four named documentation files.
2. Preserve the B1 boundary; do not infer permission for B2–B5 expansion from one positive.
3. Advance to the strategic-fork decision brief. The latest fork-B evidence is confounded across time windows, so do not start a Python/JS or Java implementation lane until that decision is explicitly rebound.

## 6. Custody

- Active worktree/branch: `/private/tmp/slicing-measure-go-b1-real-corpus` · `measure-go-b1-real-corpus`.
- Runtime binary: `/private/tmp/slicing-fix-go-root-return-typed/target/release/prism` at the hash above.
- Kubernetes checkout remained clean and unmodified at `8d34ac5ef882ce8ad3b3713f615f5947011e9730`.
- Delve checkout remained clean and unmodified at `de699e32661f41852bf045655d24ba6bcd3b5915`.
- The primary `/Users/wesleyjinks/code/slicing` checkout and its user-owned untracked files remain untouched.
