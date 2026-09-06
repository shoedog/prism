# Bounded private Props interfaces — implementation readout

Status: implemented and verified locally on feat/private-props-interfaces, based on
PR253 merge eb884824efc1686da2e789248783afe089c2cd14. Not committed, pushed or opened
as a PR: supplied machine instructions reserve committing for the controller.

## Contract and implementation

Support unique module-private, non-generic Props interfaces with required own
class-typed properties through explicit parameters and all eight already-supported
explicit/contextual forms. Reuse the existing private declaration/heritage proof;
the Props consumer rejects generic declarations and admits object_type/interface_body.
Original property, type-argument and implementation write positions are retained.
The own-property traversal and class-owner resolver are unchanged.

Duplicate declarations/properties, script globals, ambient/nested declarations,
inheritance, merging, exported/imported Props, selected optional properties, extra
non-property members, shadows and receiver/member writes remain barriers. Existing
direct class imports remain supported; this does not authorize importing Props.
No React.FC, inference, hook, alias-chain or imported-type expansion.
CPG cache73→74 and navigation sidecar42→43 invalidate prior persisted authority.

## RED, controls, review and gates

Same-environment base RED: four new groups produce two passes/two failures;
46 positive cases lack Exact ownership and four explicit-module comparisons fail.
The existing Props-alias positive control passes, separating the missing shape
support from an alternative class-owner resolution failure. Candidate GREEN4/4.
Coverage includes46 TS/TSX positives,256 declaration/consumer combinations,
36 receiver/shape negatives, and full/direct-subset script/module comparisons.

Persisted coverage adds16 good↔bad transitions, eight sidecar round-trip forms and
32 declaration-only A↔B owner transitions across eight forms × TS/TSX × both
directions. The consumer stays identical during owner replacement; assertions check
the exact new target line and full-build/incremental call-site equality. Old-version
cache misses remain tested.

Three-round SELF-PASS (NOT INDEPENDENT), cap3, no extension: round1 RED/GREEN;
round2 adversarial tests and export-value language probe; round3 declaration→property
→TypedParam→class-owner→persisted/served consumer review. No in-scope WRONG found.
Prism navigation skill was used, but the indexed helper returned SymbolNotFound;
current source was inspected instead, not treated as proof of absent callers.
The export-value probe produced TS2749 when another module used an exported
same-spelled value as a type; that observation did not expose the private interface.

| Check | Fresh result |
|---|---|
| All receiver integration groups | 26 passed |
| Persisted transition/sidecar groups | 2 passed |
| Persisted owner/version groups | 2 passed |
| cargo test | 3841 passed,0 failed,1 ignored |
| cargo test --features mcp | 4031 passed,0 failed,1 ignored |
| cargo fmt --all -- --check; git diff --check | passed |
| cargo clippy --all-targets --features mcp | completed with warnings; not warning-clean |
| Immediate release build + Tier-A matrix-only | 104/104 |
| Immediate release rebuild + Tier-A quick | completed,exit2,baseline-invalid |

The ignored control is resolution_test::slice_elem_variant_reserved. No full
multicorpus run or baseline rewrite was authorized or performed. Clippy warnings
were not fixed or attributed to this slice without a same-environment base run.

## Quick accuracy observations — not a green accuracy gate

Run2026-09-05-props-interfaces: corpus eb884824efc1 differs from pinned20c8490591a3;
four stale adjudications; oracle/SUT error rates0; oracle_not_quiescent=false.
Exact-tier raw tp/fp/fn: callers46/0/5; callees31/1/5. Caller gaps are C-method4
and Q-scoped1. Callee gaps are C-method2,C-name1,Q-scoped2, plus U-free1 raw FP.
These are observations, not regressions attributed to this slice: no paired base
quick run was performed, and changed source inventory can change sampling.

Pinned observations to carry into the PR description:

- target-c-method: flip_candidate; Exact supplementary5/0/0; default tier has28
  Prism-only sites and no oracle-only sites. Raw report retains every site.
- module-deps-feature-gated: missing literal pin; oracle-only none;
  Prism-only src/mcp/tools.rs:230.
- load-repo-feature-gated: missing literal pin; oracle-only
  tests/integration/resolution_test.rs:5299 and:5368; Prism-only
  src/mcp/freshness.rs:401 and src/mcp/session.rs:359,:374,:400.
- ambiguous-symbol-contract: ok.

## Paired measurements

Saved untouched base-prism and candidate-prism were run against identical fixed
Excalidraw source at0642e72cfa2d9a71198200e52f37399384610ee3. All2780 call records
are unchanged; Exact376→376. Eleven relevant unique source spans remain unchanged.
Synthetic capability gains are not real-corpus recall gains.

Eight synthetic served callers go from absent to their exact client.ts:2:m owner:
explicit,contextual,functionAlias,genericAlias,callableAlias,genericCallableAlias,
callableInterface,genericCallableInterface. Each reverse callee query agrees;
TypedParam provenance,no fallback,no warnings,no truncation. Optional,merged,
inherited,exported,genericProps,written,explicitUnknown and shadowed callers remain
excluded, with a same-named decoy owner present. Python Black, Excalidraw and JS
small control JSONL are byte-identical across binaries.

Reproduce from the evidence directory with measure-indirect-default.mjs and
verify-props-interfaces.mjs; the companion measurement JSON records source/control
hashes and served identities. Raw sources,binaries,paired outputs and gate logs live
at /private/tmp/prism-props-interfaces-JPIR2j.

## Cleanup and custody

Separate cleanup approvals were fulfilled exactly: earlier current-worktree
target/debug/incremental removal5.2G; then30 Tier-C prism-cache directories31.067GiB
and six old temporary Prism worktree target directories45.376GiB. Canonical paths,
clean old worktrees and no active holders were checked before the second batch;
all36 targets were verified absent with their parent directories retained.
Immediate df readout was10→86GiB. Later free space changed independently and is not
attributed to this cleanup. Removed cache bytes are gone but rebuildable.
Source,logs,prompts,results,archives and worktree registrations were retained.

Original .superpowers/ and eval/snapshots/prism-fb81481dafa7.json are untouched.
Generated quick artifacts are retained in evidence, not added to the baseline.
No memory edits. Controller must commit the scoped12-file change, push and open a
PR with the quick observations above; merge is not authorized. The current handoff
lists exact publication custody and the evidence archive.

Archive: /private/tmp/prism-props-interfaces-JPIR2j-evidence.tgz;
SHA256 e0aa11fee9fe6535841a6e786417250759d7bfd6a45b6595b2e91f5b956226d0.
Contents verified: saved binaries, paired outputs, raw quick artifacts, sanitized
cleanup audit and twelve-file verified-checkpoint.tgz, plus fixed Excalidraw source.
Raw machine process/open-file inventories are excluded from the archive and remain
local. Checkpoint documents are historical; this workspace readout/handoff adds
the final archive pointer. Neither an archive nor a local snapshot is publication.
