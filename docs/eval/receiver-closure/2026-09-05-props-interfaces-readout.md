# Bounded private Props interfaces — implementation readout

Status: implementation committed as466d4368 on feat/private-props-interfaces,
rebased onto remote main71688dc7 (PR254). Owner explicitly authorized this agent to
commit,push and open a PR, overriding the earlier controller-only instruction.
Published: https://github.com/shoedog/prism/pull/255 (open,not merged).
Rebased verification is complete; the final section supersedes the historical
pre-rebase gate record below. Publication custody is a docs-only follow-up.

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
CPG cache74→75 and navigation sidecar42→43 invalidate prior persisted authority.
Rebase preserved PR254's v74 dataflow-confidence format and test name; this slice
uses v75, and its stale-version regression now rejects a v74 cache. The original
pre-rebase tests below used v74 before that version was allocated on remote main.

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
No memory edits. Owner now authorizes this agent to publish the scoped12-file change
with the quick observations; merge is not authorized. The current handoff
lists exact publication custody and the evidence archive.

Archive: /private/tmp/prism-props-interfaces-JPIR2j-evidence.tgz;
SHA256 e0aa11fee9fe6535841a6e786417250759d7bfd6a45b6595b2e91f5b956226d0.
Contents verified: saved binaries, paired outputs, raw quick artifacts, sanitized
cleanup audit and twelve-file verified-checkpoint.tgz, plus fixed Excalidraw source.
Raw machine process/open-file inventories are excluded from the archive and remain
local. Checkpoint documents are historical; this workspace readout/handoff adds
the final archive pointer. Neither an archive nor a local snapshot is publication.

## Rebase onto PR254 — current publication evidence

Fetched main71688dc785e4ea70388bce18152d6e883cb2999c; original commit3fff2100
rebased to466d436890aa1a68a312f30fa4fbba8a29a2c730. Conflict-resolution cap2,
resolved in round1 with no extension: main's v74 DFG schema and test name retained,
Props authority uses CPG75/nav43, old-version test rejects74. Normalized receiver
AST patch is byte-identical across rebase after excluding hunk offsets/index hashes.
No additional receiver behavior or unrelated main changes were introduced.

Fresh saved base-prism was built from an archive of main71688dc7, followed by
candidate-prism from466d4368. All paired real/fixture/control measurements passed
again and exactly match the committed companion JSON:2780 real records,376 Exact,
zero changed records,eight newly served Exact fixture callers,eight exclusions,
three byte-identical controls. Original RED evidence remains historical; these fresh
served before/after results also establish the positive behavior difference on main.

Fresh matrix159/159 after immediate candidate release build. Fresh quick run
2026-09-05-props-rebase followed another immediate rebuild and completed exit2:
baseline-invalid due corpus466d436890aa versus pinned20c8490591a3 and C-name4/6
successful probes. Oracle error rate2/30 (6.67%),SUT error rate0,oracle quiescent,
zero stale adjudications. Failed oracle probes are inadmissible as product evidence.

Current raw Exact-tier tp/fp/fn: callers43/1/42,callees13/2/8. Caller gaps:
C-method25,U-method17,plus C-name1 raw FP; callee gaps:C-name5,Q-scoped3,
plus U-free2 raw FPs. No paired base quick was run; changed inventory/sampling and
oracle failures prevent regression attribution from these counts. No rebaseline.

Current pinned observations for the PR (supersede pre-rebase coordinates above):

- target-c-method: flip_candidate,Exact supplementary5/0/0; default tier30
  Prism-only sites,zero oracle-only; full site list in raw quick JSON.
- module-deps-feature-gated: missing literal pin; oracle-only none,
  Prism-only src/mcp/tools.rs:230.
- load-repo-feature-gated: missing literal pin; oracle-only examples/dfg_census.rs:359
  and tests/integration/resolution_test.rs:5299,:5368; Prism-only
  src/mcp/freshness.rs:401 and src/mcp/session.rs:359,:374,:400.
- ambiguous-symbol-contract: ok.

Fresh logs,binaries,source archive and raw quick artifacts live at
/private/tmp/prism-props-rebase-ZFiPT6. Generated quick files were moved there
recoverably; original unrelated snapshot remains untouched.

Fresh full gates: cargo test4004 passed/0 failed/1 ignored; cargo test --features
mcp4194/0/1 (totals include two doctests). Same ignored SliceElem control. fmt/diff
checks pass; Clippy completed with warnings. No full multicorpus or paired base
quick run. Only documentation changed after these gates; implementation remains466d4368.

Rebase archive: /private/tmp/prism-props-rebase-ZFiPT6-evidence.tgz;
SHA256 fd629af0f9a2c6e6c4d56957f41f2645bb5379a514acc0220938fb1258efde60.
Contains saved main/candidate binaries,main source,candidate-source.tgz,paired
outputs and all gate logs/raw quick artifacts. Checkpoint docs are historical;
this committed readout and handoff carry publication at PR255.
