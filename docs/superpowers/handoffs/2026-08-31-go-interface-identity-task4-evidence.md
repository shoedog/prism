# #16 Task 4 verification and review evidence

**Recorded:** 2026-08-31T02:28:14Z  
**Lane:** `/Users/wesleyjinks/code/slicing-16-post-provenance` · `a-go-interface-identity-post-provenance`  
**Exact base:** `b7a5cf934a44060de98588837b3c8c75ddffdc37`  
**Verified implementation checkpoint:** `cf5def8c840b54e151f507732f737d6bc6355f2f`  

## Local gates

| Gate | Result |
|---|---|
| `cargo fmt --check` | exit `0` |
| `cargo check` | exit `0` |
| `cargo clippy --all-targets --all-features` | exit `0`; repository baseline warnings only |
| strict Clippy | exit `101`; candidate exactly matches the same-environment base at `129` library and `168` library-test diagnostics; no P16 diagnostic |
| Focused P16 selectors | `15/15`: public source `5`, direct consult/retirement `7`, four-state sidecar `1`, incremental refresh `1`, sidecar pin `1` |
| Full `cargo test --no-fail-fast` | exit `0`; `28` summaries, `3,510` passed, `0` failed, `1` ignored, `0` measured |

The new cache selectors are discriminating. On exact base, the four-state sidecar selector emits `decoy/types.go::Run` as `interface_dispatch` and omits both registered callbacks. The refresh selector proves refreshed/full equality and a nonempty `go_interface_live_types`, then fails because `workerA` is absent. Candidate passes both, retains `workerA` and `workerB`, excludes the decoy, and asserts the deserialized live set contains `S`.

## Release and Tier-A

- Candidate release binary SHA-256: `1a9fa180c589acfbe0150e8058d4f25db85e2428e0b0f32bbeac443cec2a9c93`.
- Exact-base release binary SHA-256: `f16c690dd33946ef8613bd1823565d8d08f3925172415c1cb9f8e305870193ec`.
- Tier-A matrix-only: `104` cases listed, exit `0`.
- Candidate quick: exit `2`, `104` matrix cases and `32` probes. The sole invalid reason is corpus-SHA drift (`4f74ace724ae...` versus pinned `20c8490591a3...`); oracle and SUT error rates are `0`, and quiescence is not the failure.
- Same-environment exact-base quick: exit `2` for the same sole invalid mechanism (`b7a5cf934a44...` versus the same pin), the same `104/32`, the same zero error rates, and the same unrelated `target-c-method` flip metrics. The nonzero candidate exit is therefore baseline-controlled, not attributed to P16.
- Candidate Tier-A artifacts moved out of the worktree to `/private/tmp/p16-task4-eval-20260831T0159Z`: JSON `caed39d01d4f79a7fd158705c38825bbb83f8f9db84e46ac667db3279cdd9c39`, Markdown `4b2247d9d833e26deca8226bc67f68e541f932b1c9d11bb2df44de1b1be8b4db`, snapshot `225c9f0ea00a316730f67fe785f1b344f01ceaf566285490c2e1b462fb9baf5c`.

## Five-corpus conservation and parity

Retained harness: `/private/tmp/p16-task4-corpora-20260831T0204Z/run.zsh`, SHA-256 `1fd7890b39e49634930acd9d6a958c92034104dd88532ecc09866fc5e29a7368`.

All `20` no-cache commands succeeded: candidate and exact base, manifest and call-stats, over five pinned corpora. Every one of the ten candidate/base artifact pairs is byte-identical.

| Corpus | Pinned SHA | Total call sites | Manifest sites | #17b sites/hits/edges |
|---|---|---:|---:|---:|
| Caddy | `77e9ce7404c4a76853e101a9f5687a929ee56654` | 20,594 | 452 | `0/0/0` |
| etcd | `61d518f55effaf5edcedcb2a696504795b4fa7bd` | 69,207 | 3,504 | `0/0/0` |
| Hugo | `a00b5c72ac57afe26df6688ece3ca544a56df372` | 58,681 | 1,792 | `0/0/0` |
| Prometheus | `505095b64b43dd76baf08839e1800a8d473c97e0` | 110,647 | 3,077 | `0/0/0` |
| ripgrep | `82313cf95849bfe425109ad9506a52154879b1b1` | 14,169 | 0 | not emitted for non-Go corpus |

Normal manifests contain no `p16_candidates`. The #17b population is exactly zero on all four Go corpora, so no dump-sites sample or false-edge-rate estimate exists; no sample is claimed. The owner-authorized, Go-compiler-valid source fixtures remain the positive behavior oracle.

## Dispatch oracle

The retained Slice 3 candidate oracle reports are admissible exact-base baselines: their four manifest hashes are byte-identical to both current exact-base and candidate manifests, and their embedded corpus/Go/gopls/GOOS/GOARCH/GOWORK/tag pins match the current runs.

| Corpus | Site coverage | Edge coverage | Gate | Blockers | Newly Exact | Report SHA-256 |
|---|---:|---:|---|---:|---:|---|
| Caddy | 1.0 | 1.0 | pass | 0 | 0 | `a3d1ac27ef06ccd6f47cada45246583aa8706d904a422504283d4cb45b1f3550` |
| etcd | 1.0 | 1.0 | pass | 0 | 0 | `6f21a30bc591ebfc32e8e0dc4d9ff9699596de6a1648d8c2c85b971e9291405d` |
| Hugo | 1.0 | 1.0 | pass | 0 | 0 | `1bbcd986761883c616b4fddbd7dbeb42f25671e7e767b885cc90beb8feaba579` |
| Prometheus | 1.0 | 1.0 | pass | 0 | 0 | `88baf0f1be8d662f2c0b5b9eca270219669389383c0f3894a6c205f6672ddc11` |

All four also report zero oracle-unresolved, timeout, target-mismatch, and over-approx sites. Reports and the replayable harness are retained at `/private/tmp/p16-task4-oracle-20260831T0230Z`.

## Implementation review round 2

Review cap: `2`; this was the final declared round.

- `WRONG`: zero.
- `SMELL`: two closed documentation/name mismatches: a stale comment claimed the bare R3 ladder was unchanged, and the sidecar pin test name still described the predecessor increment. Both were corrected without runtime change.
- Structural branch-local caller evidence found exactly two production consumer functions: `resolve_call_site_full` and `interface_dispatch_manifest_inner`; sidecar resolution remains derived through the resolver. No third consumer exists.
- The shared terminal predicate runs before both consumers. The shared consult alone owns exact-owner declaration selection, direct/promoted supply, profile/conflict filtering, live selection, and arity. Empty results continue once through the owner-qualified func-field terminal helper and never retry the bare table.
- Cache behavior remains synchronous and schema-neutral except for the required sidecar topology pin `22 -> 23`; CPG stays `54`.

The review converged within the cap: zero open `WRONG`, zero open in-scope `SMELL`, and no recurrence of the proxy-for-provenance class.

## Exclusions

- Strict Clippy is not green repository-wide; its candidate failure is exactly same-environment baseline debt as quantified above.
- Tier-A quick is not a valid pass because the committed baseline pin rejects both candidate and exact base. It is retained as a controlled invalid result, not rebaselined.
- The initial zero-selected focused command, a malformed jq projection, guessed artifact filenames, and the truncated first full-suite aggregate are inadmissible and excluded. The corrected selectors and non-truncated `28`-summary aggregate supersede them.
- Coverage is not a publication wait condition by explicit owner direction. Relevant non-coverage CI remains required.
