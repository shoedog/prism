# Configured observations: opt-in producer and validator

Base PR259 merge ebf7933bc3ac7fea1c3201eb48ca84611ea9a702; branch
feat/configured-callable-observations. Standalone scripts only: no production
resolver imports, default TypeScript dependency, runtime CLI addition, cache bump,
served Exact edge or recall improvement. Authorizes_runtime_edge is always false.
Implementation3bc46a4 and docs closeout54843d15 published;
[PR260](https://github.com/shoedog/prism/pull/260) merged8e7744e2 on2026-09-06.
The merge tree equals the published closeout. Gates below remain historical.

## Outcome

The producer builds a bounded Program from actual config and an in-memory snapshot
of caller-supplied roots, preserving source/dependency bytes, membership, options,
negative lookups and declaration anchors. Reacquisition checks for observed changes.
The validator rejects malformed packets before root access and recomputes all
evidence; no stale-positive cache or packet-directed path reads exist.

`valid:true` means reproducible observation, not authentication or class authority.
Its separate packet_status remains unproven when dependencies, resolution or
augmentation coverage are incomplete. Missing dependencies are not installed and
actual options are not rewritten. See scripts/callable-observations/README.md and
the same-date spec for the strict schema, resource limits and scope exclusions.

## RED and final-round corrections

Initial minimal scaffold failed three behavioral controls: configured root set was
empty, contextual declaration observations were absent, and forged authority was
accepted. Setup worked; red.log captures all three. This is tooling RED, not a
Prism runtime recall change. The initial positive fixture lacked a package.json
boundary, causing ancestor lookup with zero diagnostics. Adding the actual fixture
boundary made it observed; its removal is a retained negative, not a relaxed oracle.

Three SELF-PASS rounds, NOT INDEPENDENT. The final-round findings were closed,
enumerable and corrected on the existing artifact without a restart or feature
expansion. Captured WRONG cases, with same-environment controls:

| Input / state | Incorrect result before correction | Bounded correction / evidence |
|---|---|---|
| Conflicting anchor range or source hash | Validator consulted root options before rejecting | Pre-I/O cross-field checks; round3-red.log, final-node-23.log |
| Explicit include of excluded .git inputs | Program reported observed while inputs disappeared | Exclusion-boundary sentinels plus refusal; first fence-only attempt remained RED because directory matching never visited the hidden entry; final test passes |
| Directory import with trailing slash | Valid worker observations discarded as worker_failed by schema | Normalize probe IDs; trailing-slash-red.log and final test |
| Case-insensitive host with thing.ts and THING.d.ts | Snapshot selected declaration shadow instead of source file | Use compiler host canonicalization/lookup policy; case-policy-red.log includes live TypeScript module-resolution control and wrong target; final test passes |

The trailing-slash issue was distinguished from compiler failure by reading the
raw worker packet; that packet already contained configured roots/diagnostics.
The case-policy control uses the same pinned compiler and filesystem, not an
oracle from a different environment. No inherited runtime WRONG was downgraded.
Final controls include strict/oversized packets, Unicode/CRLF ranges, stale options,
membership, dependency removal/restoration, A↔B declaration changes, augmentation,
explicit-any/assertions, references/plugins, symlinks, invalid UTF-8/compiler bytes,
file/byte/depth/time/observation caps and a concurrent monotonically changing input.
Both installed React18 and React19 declaration profiles are exercised.

## Gates and limitations

Final Node producer/validator23/23. Prior compiler40/40 and helper4/4 replayed.
Full default4017 passed/0 failed/1 existing ignored (28 summary groups),
MCP4207/0/1 (30 groups); both include two doctests. Existing ignored case:
resolution_test::slice_elem_variant_reserved. fmt/diff checks pass. Test-code
warnings remain; no warning-clean or fresh Clippy claim. Rust source/tests/Cargo
inputs are unchanged; later Node hardening does not alter the tested Rust artifact.

Tier-A not triggered: no call-resolution/navigation/CPG production change. Prior
quick remains INHERITED baseline-invalid (SHA drift/C-name4/6/oracle2/30/SUT0),
not freshly green. No full multicorpus, rebaseline, installed application dependency
closure or application test success is claimed. Private application observations
remain local; no private source, paths, names or measurements enter public evidence.

SMELL / explicit scope limit: this trusted-worktree acquisition is not an atomic
or adversarial-filesystem proof. Two snapshots cannot rule out change-and-revert;
mixed-policy mounts are unsupported. Direct-body observations also omit nested
callback binding ownership, arbitrary callable aliases and runtime class/write
proof. No consumer may treat these omissions or zero diagnostics as Exact authority.

Successor approved: bounded declaration/alias-chain provenance; see the same-date
callable-alias-provenance spec/handoff. Nested parameter-binding ownership remains
a separate future slice as needed by the public FC sites.
Keep both non-authorizing until installed real-program closure and Prism's own
class/member/write/duplicate/cache obligations have independently been established.

## Custody

Evidence: /private/tmp/prism-configured-observation-HoDwRN/public.
Code/spec checkpoints and complete gate/RED logs are retained. Private evidence
is separately local. Public archive:
/private/tmp/prism-configured-observation-public-evidence.tgz, SHA256
`dbcc09e18c29e3daaf9552d404e4f45638c705fefd927bf32acbc65365a44cdb`.
It captures3bc46a4 scoped source/checkpoints/public gate logs and predecessor public
compiler/dependency evidence, before this docs-only closeout. No private packet,
raw application worker output or private report is included.
