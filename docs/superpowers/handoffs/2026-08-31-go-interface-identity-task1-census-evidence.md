# #16 Task 1 census evidence — zero-selection park

**Captured:** 2026-08-31T00:59:49Z  
**Lane:** `/Users/wesleyjinks/code/slicing-16-post-provenance` · `a-go-interface-identity-post-provenance`  
**Exact base:** `b7a5cf934a44060de98588837b3c8c75ddffdc37`  
**Task 1 checkpoint:** `b529d29595bac1c7040be94992bf01bcd320eb97`

## Verdict

**PARK before Task 2.** The corrected five-corpus census is exhaustive for the declared denominator but selected zero sites for the candidate consult. Design v14 §4 says a zero-selection probe is inadmissible, so it cannot authorize public REDs or the production switch. No oracle join exists because there are no changed corpus target sets.

The first census implementation admitted candidates from the manifest's local `legacy_bare` flag. That was a harness `WRONG`: a constructible owner-bearing site can reach the resolver's legacy `iface_key` arm while the manifest independently labels the same site `unproven_drop`. Those first artifacts are retained at `/private/tmp/p16-census-task1-20260831` but are inadmissible. The corrected v2 harness admits candidates only when `resolve_call_site_full` reports `go_unproven_receiver_bare_fallback_sites > 0`; that telemetry is set only at the actual resolver arm.

## Bound identities

| Item | SHA / state |
|---|---|
| Candidate release binary | `7164d872d8e49585fc892887146c85a4ae3a24c42c5a9bba55df14aa0cc8ee79` |
| Exact-base release binary | `631bedb9a3ac8904574e552b886318fffe802e1df8a3489d82cb027c3c6a48a1` |
| Default ripgrep manifest, candidate | `f81862b181de76a79ee6c0ed04eb48a287477d9c7bcdef9fc27636e4f47ded3f` |
| Default ripgrep manifest, exact base | `f81862b181de76a79ee6c0ed04eb48a287477d9c7bcdef9fc27636e4f47ded3f` |
| ripgrep corpus | `82313cf95849bfe425109ad9506a52154879b1b1` |
| Caddy corpus | `77e9ce7404c4a76853e101a9f5687a929ee56654` |
| Prometheus corpus | `505095b64b43dd76baf08839e1800a8d473c97e0` |
| etcd corpus | `61d518f55effaf5edcedcb2a696504795b4fa7bd` |
| Hugo corpus | `a00b5c72ac57afe26df6688ece3ca544a56df372` |

The exact-base control worktree is `/private/tmp/slicing-p16-base-b7a5cf93`. The corrected generated artifacts are under `/private/tmp/p16-census-task1-v2-20260831`; they are outside the source worktree and are hash-bound below.

## Corrected census results

| Corpus | prerequisite | ownerless terminal | owner-bearing | candidate | changed terminal sets | artifact SHA-256 |
|---|---:|---:|---:|---:|---:|---|
| ripgrep | 0 | 0 | 0 | 0 | 0 | `b9b996e48f2ffad2f83536c5ecbb2259a1c45839f8e21ce6170a9c4049ba744d` |
| Caddy | 452 | 0 | 452 | 0 | 0 | `6423a8e02fae5198d96e41c1ba5ec79af303551da1140f0e549fff5d28c76604` |
| Prometheus | 3,107 | 30 | 3,077 | 0 | 0 | `fd15a0d78eea69562a0eee6eec70c69b1d3d7fa1328989e1cf7296b5d8e4d8cf` |
| etcd | 3,504 | 0 | 3,504 | 0 | 0 | `6eb1b1d61ef8f4c1f4fb57be7589a6152e9c646bc12ac3fa80f58701f1f6c171` |
| Hugo | 1,808 | 16 | 1,792 | 0 | 0 | `480961f4ccbe3249f86ec3a80bdd5728712f93fa52e3074f6b68e650cc33212f` |
| **Total** | **8,871** | **46** | **8,825** | **0** | **0** | — |

For every corpus, prerequisite keys were unique, candidate keys were unique, candidate-minus-owner-bearing was zero, and unknown route values were zero. Independent whole-graph `nav --no-cache call-stats` probes on all four Go corpora also reported `go_unproven_receiver_bare_fallback_{sites,hits,edges} = 0/0/0`; therefore the empty candidate ledger is not a manifest-denominator omission.

## Commands and exits

All commands exited `0` unless explicitly described as inadmissible.

- Candidate and exact-base release builds: `cargo build --release`.
- Default byte control: `env -u PRISM_P16_CENSUS .../prism nav --no-cache interface-manifest --repo <ripgrep> | shasum -a 256`, with `set -o pipefail`.
- Corrected census: `PRISM_P16_CENSUS=1 target/release/prism nav --no-cache interface-manifest --repo <corpus> | tee /private/tmp/p16-census-task1-v2-20260831/<corpus>.json | shasum -a 256`, with `set -o pipefail`.
- Whole-graph discriminator: `target/release/prism nav --no-cache call-stats --repo <corpus>` on Caddy, Prometheus, etcd, and Hugo.
- One help probe used nonexistent `interface-dispatch-manifest`; it failed for its own command-shape error and was discarded. Correct compiled syntax is `nav interface-manifest --repo <REPO>`.
- One shell custody probe assigned zsh's reserved `path` variable and thereby hid `git`; it was discarded and rerun with `corpus_dir`.

## Verification completed

- `cargo test --lib p16_census_ -- --nocapture`: **5 passed, 0 failed**, including all five consult routes, target identity encoding, ownerless no-invocation, and the resolver-witness/manifest-proxy discriminator.
- Existing promoted-snapshot suite: **20 passed, 0 failed**.
- Existing library S4 suite: **5 passed, 0 failed**.
- Existing Go integration S4 filter: **24 passed, 0 failed**.
- Focused compatibility aggregate: **49 passed, 0 failed**.
- `cargo check --all-targets --all-features`: exit `0`; only pre-existing test warnings were emitted.
- Default ripgrep manifest is byte-identical to exact base.

Not run because the Task 1 STOP gate fired: public RED matrix, production switch, cache bump, Clippy, full `cargo test --no-fail-fast`, Tier-A, Go oracle joins, implementation review rounds, CI, push, or merge. Their absence is carried into the park verdict.

## Required owner decision

One of these is required before further state-changing work:

1. Keep v14's zero-selection rule and park #16 until a pinned corpus contains an actual owner-bearing legacy-arm site; or
2. Explicitly amend the acceptance rule to permit constructible source fixtures (plus exact-base RED proof) to authorize a route absent from all pinned corpora, and specify any replacement oracle/corpus coverage floor.

The current general push/merge authority does not itself relax the design's zero-selection STOP condition.
