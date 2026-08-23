# Handoff — P17 narrow concrete-receiver routing implementation and acceptance

**Written:** 2026-08-23T16:48:55Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-p17-narrow` · `p17-narrow-concrete-receiver` · **Measured state:** `[MEASURED]` implementation HEAD `9863f610d720076fddcbb634f5118364c59d2c21` · Tree CLEAN before this handoff-only commit · Probe `git rev-parse HEAD; git status --short` · Output inline in the implementing session
**Predecessor:** none — first in lane
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — another session/agent alive in this lane? `[MEASURED]` No subagents were started; `origin/p17-narrow-concrete-receiver` equaled implementation HEAD `9863f61` at 2026-08-23T16:48:55Z, indicating the controller had already published through the acceptance-test commit — **RESOLVED implementation is complete; controller owns remaining external gates**
**(b) Custody exposure** — unpushed commits, uncommitted work, single-copy/untracked artifacts: `[MEASURED]` implementation HEAD equaled the origin branch and the tree was clean before writing this document. This handoff will be one new local commit for the controller to push — **OPEN controller push of the handoff-only commit**
**(c) In flight / irreversible** — running process, held lock, half-applied migration: `[UNKNOWN]` final `ps` probe was refused by the managed environment. All tracked tool sessions had completed; the detached control worktree `/private/tmp/p17-base-514cfe3` remains intentionally available — **RESOLVED no known in-flight mutation; temporary worktree is read-only control state**
**(d) Authorization granted but not exercised** — the standing instruction a successor may not re-derive: “Do NOT run the gopls oracle (controller does). Do NOT push (controller pushes after each wave — tell me when a wave is committed).”

## 1. Resume order

1. Provide an accessible Python >=3.12 eval environment or approve uv-cache access, then run `cargo build --release` immediately followed by `cd eval && uv run --offline tier-a --matrix-only --allow-stale-sut`; expected matrix count is 104.
2. Run the controller-owned gopls oracle and the route-specific subtraction audit from design §8. Preserve the measured on-demand R2 additions in caddy/Hugo; do not classify them as concrete-route fallback.
3. Review this handoff commit and push it. The implementer did not push.

**STOP conditions:** Stop on any Tier-A regression; any concrete route with nonzero interface fanout; any R3 output change beyond the three new telemetry counters; or any oracle mismatch at an R1(a) direct target.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Design custody | done | `[MEASURED]` Commit `d90a883` contains the authoritative v7 design on this branch. The supplied object `5ff0cb2` was absent from this clone; the live post-#182 design blob was recovered and committed without editing. |
| R1 direct route | done | `[MEASURED]` `5698e1a`; red-first direct/factory/pointer/typed-param/S1/var/type-assertion poles, then 5/5 focused and 139/139 Go tests. Target files are asserted. |
| R1 selector boundaries | done | `[MEASURED]` `9f4af8b`; R1(b) promoted-deferred, R1(c) embedded-interface S4, R1(d) P5, R1(e) no-selector, interface controls, generic and pre-drop controls. Red 4 cases; then 12/12 focused and 146/146 Go. |
| Declaration-kind graph | done | `[MEASURED]` `090199c`; P10 identity-keyed serialized index, per-declaration import environment, transitive alias/defined-underlying resolution, pointer aliases, cycles/unresolved fail closed, literal-interface alias satisfaction. Red 5 poles plus cycle control; then 18/18 focused and 152/152 Go. |
| R3 telemetry | done | `[MEASURED]` `6743220`; unchanged bare ladder with additive sites/hits/edges counters. Both ambiguous-basename and duplicate-profile hit/drop poles are pinned. |
| Manifest and caches | done | `[MEASURED]` `0d6366b`; shared full route verdict, seven route strings, resolver/manifest target parity, CPG 47, sidecar 16, exact CPG/sidecar/no-cache byte parity, non-Go key omission. Focused 29/29, manifest integration 10/10, cache parity 1/1, Go 163/163. |
| Inherited promotion expectations | done | `[MEASURED]` Same-environment base controls passed 1/1 CLI and 6/6 integration tests; candidate exposed intended v7 behavior. `9863f61` updates only tests to pin terminal promoted-deferred zero-edge behavior. |
| Full Rust suite | done | `[MEASURED]` `cargo test --quiet --color never`: 3,320 passed, 0 failed, 1 ignored across 28 test binaries. Ignored test is the pre-existing reserved `SliceElem` case. |
| Formatting | done | `[MEASURED]` `cargo fmt --all` completed; final `cargo fmt --all -- --check` passed. |
| Release build | done | `[MEASURED]` `cargo build --release`: success. |
| Tier-A matrix | blocked | `[UNKNOWN]` Requested `eval/.venv/bin/tier-a` does not exist. `uv run --offline` was sandbox-refused at `/Users/wesleyjinks/.cache/uv`; escalation was rejected. System Python is 3.9.6, below required >=3.12, and no sibling eval venv exists. No matrix verdict was produced. |
| gopls oracle | pending | `[INHERITED]` Owner explicitly reserved this gate for the controller; implementer did not run it. |
| Five-corpus call-stats | done | `[MEASURED]` Fresh release candidate `--no-cache` vs controller controls built from `514cfe3`; detailed table below. Ripgrep is byte-identical. |

### Same-base call-stats

Values are `base -> candidate (delta)`. Missing map entries are reported as zero. “R3 s/h/e” means unproven bare-fallback sites/hits/edges.

| Corpus | interface Exact | constructor Exact | typed-param Exact | var Exact | assertion Exact | return Exact | FuncValueField Exact | NLCF | multi-target sites | direct / promoted / no-selector | R3 s/h/e |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| caddy | 1766 -> 1776 (+10) | 241 -> 258 (+17) | 107 -> 1972 (+1865) | 0 -> 0 | 0 -> 0 | 233 -> 233 | 0 -> 0 | 3 -> 3 | 21 -> 21 | 2470 / 7 / 431 | 1978 / 19 / 1339 |
| prometheus | 2461 -> 2294 (-167) | 978 -> 1441 (+463) | 770 -> 1349 (+579) | 0 -> 0 | 0 -> 0 | 2135 -> 2135 | 0 -> 0 | 143 -> 143 | 3855 -> 3813 (-42) | 4876 / 19 / 91 | 4299 / 7 / 7 |
| etcd | 2002 -> 1904 (-98) | 109 -> 116 (+7) | 230 -> 489 (+259) | 0 -> 0 | 0 -> 0 | 1064 -> 1064 | 0 -> 0 | 578 -> 578 | 515 -> 412 (-103) | 1703 / 12 / 80 | 6620 / 164 / 196 |
| hugo | 625 -> 653 (+28) | 193 -> 201 (+8) | 440 -> 592 (+152) | 0 -> 0 | 0 -> 0 | 2927 -> 2927 | 0 -> 0 | 330 -> 330 | 2568 -> 2565 (-3) | 3811 / 11 / 684 | 3682 / 6 / 7 |

`[MEASURED]` Ripgrep non-Go output is byte-identical: 3,019 control bytes and 3,019 candidate bytes.

`[MEASURED]` Route-level audit of the positive interface deltas:

- caddy: base manifest owner fanout 1,769 -> 1,754. All concrete/direct/drop routes have zero fanout. Four proven `interface_dispatch` sites add 19 file-distinct identities while owner-level fanout decreases overall; examples are `Module.CaddyModule` and `MiddlewareHandler.ServeHTTP`.
- hugo: base manifest fanout 625 -> 644. Fifty-two positive sites are all proven `interface_dispatch`; no concrete route gains fanout. Examples are typed `config.Provider` and `navigation.Page` receivers. This is the design-required on-demand R2 path bypassing the old carried-identity gate.
- prometheus and etcd have negative interface Exact deltas and unchanged NLCF. No corpus increases multi-target sites.

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Owner task text, spec object identity | Supplied `git log` expectation named spec commit `5ff0cb2`, but that object was absent in this clone. | `[MEASURED]` Equivalent authoritative v7 blob was present on the live post-#182 line and is committed here as `d90a883`; the spec content was not edited. |
| Inherited CLI/integration tests | Four tests asserted concrete promoted methods still mint `EmbeddedPromotion` Exact edges. | `[MEASURED]` v7 makes these terminal `ConcreteReceiverPromotedDeferred` drops. Base controls proved the expectations were pre-change behavior; `9863f61` pins the new zero-edge result. |
| None elsewhere | None. | `[MEASURED]` No memory update was authorized or written. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Tier-A 104-case matrix | blocked | Release rebuild, then run the matrix command in §1 | Python >=3.12 plus uv dependency environment/cache access | `--matrix-only --allow-stale-sut` |
| 2 | gopls oracle/subtraction audit | pending | Controller runs design §8 gate and records changed-site target identities | Explicit owner reservation | caddy, prometheus, etcd, hugo |
| 3 | Publish handoff commit | next | Controller reviews and pushes current branch | Implementer has no push authorization | `p17-narrow-concrete-receiver` |

## 5. Invariants and traps — do not do these

- Never route proven concrete receivers into the bare `iface_key` ladder — R1(b) and R1(e) are terminal zero-edge drops.
- Never create a second proof/consult path in the manifest — resolver and manifest share `go_concrete_receiver_route`.
- Never use the bare global alias map as declaration evidence — aliases resolve transitively in the alias declaration file’s import environment.
- Never treat duplicate declaring files as one owner, even when declarations are text-identical — this is `AmbiguousProfileConflict` and stays R3.
- Never interpret positive aggregate interface deltas alone as concrete fallback — inspect candidate `dispatch_route` and full implementer identities; on-demand R2 is required.
- A missing Tier-A executable or uv cache refusal is inadmissible evidence, not a matrix pass or regression.
- Do not run gopls and do not push in the implementer lane — both are controller-owned by explicit instruction.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Base | `514cfe3` |
| Branch | `p17-narrow-concrete-receiver` |
| Spec | `d90a883` |
| Wave 1 | `5698e1a` |
| Wave 2 | `9f4af8b` |
| Wave 3 | `090199c` |
| Wave 4 | `6743220` |
| Wave 5 | `0d6366b` |
| Acceptance-test follow-up | `9863f61` |
| CPG cache | `47` |
| Navigation sidecar | `16` |
| Control worktree | `/private/tmp/p17-base-514cfe3` |
| Control files | `/private/tmp/claude-501/-Users-wesleyjinks-code-slicing/a3bf14f1-6b47-464b-ba09-fc62e2ad7efb/scratchpad/ctrl514-{ripgrep,caddy,prometheus,etcd,hugo}.txt` |
| Handoff | `docs/superpowers/handoffs/2026-08-23-p17-narrow-handoff.md` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: “A proven concrete-recovered Go receiver cannot reach the bare-name interface-dispatch fallback.” · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: this handoff’s full-suite, same-environment base-control, cache-parity, non-Go byte-parity, and route-level corpus evidence

**Questions the owner owes an answer to:** 1. Will the controller run Tier-A in its existing Python >=3.12 environment, or authorize this lane to access the uv cache? 2. After the controller’s gopls audit, may the temporary base worktree `/private/tmp/p17-base-514cfe3` be removed?
