# Singleton wildcard observation closeout

Base: merged PR273, `5a4b9b5eee03dc18c1d2b6cc794b0950e3f39b80`.
Implementation checkpoint: `9d31f54f`. Schema9 / producer0.10.0; observer only.
No asset-existence, closure-policy, class, runtime or React.FC authority expansion.

## Result

The fixed public Program adds315 wildcard candidate records:231 observed
singletons (230 `*.woff2`,1 `*.css`) and84 merged `*.scss` bindings retained as
unproven/duplicate_provider. The existing272 exact-ambient observations are
unchanged. The new nullable lookup.wildcard record is independent of the existing
exact-ambient lookup.status/reason; a wildcard can be observed while its exact
lane correctly remains non_exact_binding. Nothing changes filesystem target:null.

| Check | Result |
|---|---|
| Legacy packet projection, including exact lookup records and result order | identical to exact main |
| Filesystem requests / null targets |6893 /603, unchanged |
| Source gaps | same14 request anchors as PR273 |
| Independent source anchors |14,788 occurrences /7,600 unique, all checked |
| Full Program provider/augmentation census and raw-specifier pattern matches | equal to independent source parse |
| Callable observations / nested calls |30 /53, unchanged |
| Library class candidates |4 remain program_unproven |
| Runtime/class authority flags | both false |
| Validation | reproduced_observation, valid:true, packet_status:unproven |

The snapshot still contains59,924 files,9,073 directories,249 links,591 roots and
1,445 Program files. All264 refusal digests and outside_lookup evidence are
unchanged. Zero compiler diagnostics still does not establish closure. Public
source/config/lock bytes and execute bits match the clean original checkout
before/after; no acquisition, installs, project scripts or app/config repairs ran.

Compact checked evidence is in `2026-09-07-callable-singleton-wildcard-evidence.json`.
Producer SHA256: `f33f0edcaadaaac15fb6e1682f094a7aa1b07f5e2352bbe9e81c8ac7ace387bf`.
Packet SHA256: `f8b266d6bf7de4124d968d31187c82cd32fd94d8e3aae786baf89afe3c509fa5`.

## Proof boundaries and review

A positive requires the actual supported source literal/checker binding,
singleton original-source declaration and matching-provider census, supported
top-level non-external declaration-file body, no relevant augmentation and no
competing pattern. Pattern matching follows the pinned compiler's single-star,
case-sensitive prefix/suffix and minimum-length rule. Longest-prefix competition
is deliberately withheld, even where TypeScript itself chooses a winner.

The original-source census preserves PR273's package-ID redirect correction.
Tests cover identical and differing redirected augmentation copies. JSDoc imports,
receiver writes, duplicate insertion/restoration, unsupported requests, excluded
providers, merged/invalid/competing patterns, pre-I/O schema refusals and tampered
or removed observations are covered. Asset-present and asset-absent fixtures both
observe the same binding: this is explicit evidence against an existence claim.

Two SELF-PASS rounds, NOT INDEPENDENT, no cap extension. Round1 source/helper/schema
inspection and RED/GREEN controls: no WRONG or SMELL findings. Round2 independent
source/pattern census, exact-main projection and consumer/authority checks: no
WRONG or SMELL findings. No production debugging correction or re-baseline was
needed in this slice. The newly added pattern-matching memo is Program-local,
bounded by existing acquisition/heap/time/packet limits and never reused across
snapshots. There is no Prism runtime consumer.

## Gates and custody

- Initial RED before implementation:0/24 passed,24 behavioral failures.
- Final31 fixtures against exact merged main in the same environment:0/31 passed,
 31 behavioral failures; only the test module's index import points to the control.
- Full observer suite:212 passed,0 failed,0 skipped.
- Default Rust:4017 passed,0 failed,1 ignored;28 groups including2 doctests.
- MCP Rust: in flight at this documentation checkpoint; final total in handoff.
- Authority controls:40 results,failures=[]; audit helper tests:7/7 passed.
- fmt/diff checks and independent public replay passed. No Rust source/tests or
 Cargo files changed; no Tier-A-triggering resolver/navigation/CPG paths changed.
 Full multicorpus evaluation, app builds, private acquisition and RSS/runtime
 recall measurements were not run or claimed.

```sh
PRISM_TYPESCRIPT=/private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js PRISM_CALLABLE_PROFILES=/private/tmp/prism-callable-authority-98TLLN/public/profiles node --test scripts/callable-observations/*.test.mjs
CARGO_TARGET_DIR=/Users/wesleyjinks/code/slicing/target cargo test --offline
CARGO_TARGET_DIR=/Users/wesleyjinks/code/slicing/target cargo test --offline --features mcp
```

Public produce/validate used Node24.15.0, installed profile, in-root links and
unchanged tsconfig.json. Tests used Node26.0.0. TypeScript5.9.3 remains pinned to
SHA256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.
LSP tools were unavailable; the navigation-skill fallback used compiler/source
evidence. Raw controls, packets, source verifier and logs live in
`/private/tmp/prism-singleton-wildcard-65xC4h`; archive checksum is in the handoff.

## Next boundary

Recommend source-backed proof requirements for the84 merged wildcard bindings
before any merged-binding observation expansion. Asset inventory presence and
closure admission remain separate approval boundaries; the14 real source gaps
cannot be erased by ambient annotations.
