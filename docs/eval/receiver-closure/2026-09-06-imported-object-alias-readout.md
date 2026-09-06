# Direct imported object-alias receiver identity

Base PR256 merge e08af858128c20ae3372b62f1688593756f06af9;
branch feat/imported-object-alias-identity. Implementation/full suites complete;
quick completed baseline-invalid. Implementation9c99f96 committed/pushed;
PR https://github.com/shoedog/prism/pull/257 merged 2026-09-06T16:32:06Z as
661a95a4ecfc4ced97a2a703a22082531b95527b. Fetched origin/main matches; its tree
equals published head153e359566e8ff6aba3745ab1f07af0f6f906f17. CPG76/nav44;
no unrelated artifacts removed. Merge reconciliation is carried on the approved
contextual imported-alias successor branch. Gates below are prior implementation-run
evidence, not rerun for the merge acknowledgement. See the separate same-date
contextual-imported-alias readout for successor scope and evidence.

## Outcome and boundary

One named ESM import (type-only and renaming included), one unambiguous relative
source module, and one directly exported non-generic object alias can now establish
the receiver class for an explicitly annotated required destructured parameter.
The proof retains the defining file, alias span and original property-type span.
Terminal class lookup occurs in that file, not under a same-spelled consumer decoy.
Existing class/member ownership checks remain required.

The new map is collected from the complete supplied ParsedFile map on both full
and direct-subset builds. Incremental merge replaces it, including an empty new map;
missing/changed declarations cannot retain a prior positive proof. CPG serialization
and navigation edge-cache versions advance together. CallSite's local classification
remains materialized/unproven; only the separate cross-file proof can authorize this
new Exact route. Unsupported bindings do not acquire import/owner fallback authority.

This is **source-snapshot proof**, not a configured TypeScript program. The module
resolver uses existing literal-relative source matching; no package exports, tsconfig
paths or dependency/typeRoots closure is claimed. Observed ambient declarations,
parse errors, prototype access (even reads), and selected Object/Reflect mutator
accesses conservatively block this new route across the supplied JS/TS snapshot.
No general runtime alias analysis or arbitrary dynamic reflection claim.

Imported interfaces, export clauses/barrels, namespace/query imports, generic aliases,
method-bearing shapes, contextual imported aliases, hook inference and React.FC remain
excluded. Receiver writes, duplicate bindings/properties/types, missing or ambiguous
modules, class/member conflicts and static-only methods remain barriers. Existing
local alias/interface routes are unchanged, including their separate support scope.

## Source and compiler evidence

The24 PR256 compiler fixtures pass with the upstream-pinned **TypeScript5.9.3**;
the outcomes agree with the prior6.0.3 run. The single direct imported object-alias
guard is promoted to Exact in the shared corpus; all other expectations are retained.
The supplied files form the isolated compiler fixture program, not a whole-app check.

Prism diagnosis: identical helper query before/after refresh. Before: SymbolNotFound;
location3532 mapped to an older function and warned StaleIndex90 paths. Refresh:
stale_before_refresh=true. After: js_ts_inline_prop_receiver_type returned as caller,
no warnings. The reproduced issue was a stale MCP snapshot; no persistent lookup
defect was demonstrated or fixed. No LSP tools were exposed in this session.

## Behavioral controls and review

Captured RED against unchanged e08af858 runtime: direct imported alias and restored
augmentation transition both produced0 Exact instead of1. The call existed and its
local receiver was materialized, so this is an admissible missing-route failure.
Initial implementation passed these, then the complete49-case negative sweep found
three admitted exclusions: extra named export,star export,prototype replacement.
Targeted source-boundary fixes passed; round3 added bracketed prototype,Object and
Reflect cases, bringing the negative population to52 (TS/TSX × full/direct-subset).
Seven additional positive forms cover imports/renames,callable forms,capture and scope.

Disk-cache24 bidirectional transitions cover class A↔B, alias-file removal/addition,
augmentation-file addition/removal, module ambiguity, barrel substitution and a
cross-file JS prototype patch. Each invalidates prior disk hits and matches fresh
full-build receiver results/proof maps. Four navigation-sidecar states prove Exact
identity or absence and reject old fingerprints after augmentation membership changes.
Previous cache versions75/43 are rejected. Three-round SELF-PASS, NOT INDEPENDENT;
all three rounds complete, no open in-scope WRONG. No rebaseline or full-multicorpus authority.

Final full gates: cargo test4012 passed/0 failed/1 existing ignored; cargo test
--features mcp4202/0/1, including two doctests each. The default suite was rerun
after the last negative-test additions. fmt/diff checks pass; Clippy completed with
warnings (including type-complexity warnings), not warning-clean. Matrix159/159.
Initial quick completed baseline-invalid: corpus e08af858 versus pinned20c8490591a3,
C-name4/6 successful probes, oracle error2/30 (6.67%), SUT error0, quiescent, no stale
adjudications. It overlapped final test-only additions, so it is not a frozen-tree
comparison. One final committed-source quick is allowed (two-run cap); no regression
attribution without a same-environment base control.

Final quick,2026-09-06-imported-alias-frozen: source and SUT9c99f96a7067,
corpus_dirty=false,prism_dirty=false. Completed exit2, baseline-invalid for SHA drift
versus20c8490591a3 and C-name4/6 successful probes. Oracle2/30 errors (6.67%),SUT0,
quiescent,zero stale adjudications; matrix159/159. Stop at cap2, no rebaseline.
Raw Exact-tier tp/fp/fn: callers17/0/22,callees23/3/10. No paired pre-change quick:
these are observations, not attributed regressions or a green accuracy gate.

Pinned observations (full source-site lists retained in raw JSON and PR body):
target-c-method flip_candidate,Exact supplementary5/0/0,30 default-tier Prism-only
sites and no oracle-only sites. module-deps-feature-gated missing literal pin:
Prism-only src/mcp/tools.rs:230. load-repo-feature-gated missing literal pin:
oracle-only examples/dfg_census.rs:359 and tests/integration/resolution_test.rs:5299,:5368;
Prism-only src/mcp/freshness.rs:401 and src/mcp/session.rs:359,:374,:400.
ambiguous-symbol-contract:ok. Failed oracle probes are inadmissible product evidence.

## Measurement and custody

Fresh real replay:2780 call-site records,376 Exact, unchanged. The pinned605-file
source audit and five measured source files still validate. Six relevant spans remain
unresolved: four React.FC,two useApp/useContext. This foundation claims no gain there.

Fresh served fixture:five receiver calls; direct and renamed resolve Exact to
client.ts:2, while overwritten/contextual/shadowed stay non-Exact. CLI host cache
writes were refused, so these are fresh rebuild results, not CLI cache-hit evidence.
Disk/sidecar cache evidence comes from the explicit tests above.

Evidence directory: /private/tmp/prism-imported-alias-O4d6E1. Includes pinned compiler
package, RED/GREEN/cache logs, fresh binary and raw real/served call-stats, and source
audit. Archive: /private/tmp/prism-imported-alias-O4d6E1-evidence.tgz;
SHA256 55fe6a37b4bdc6d97092fd3cb47b4db7ad4b10b5f98213edd9829959467c8d69.
Contains both quick runs/reports/snapshots, baseline and candidate source checkpoints,
both candidate binaries, compiler package and source/served fixtures. Generated quick
artifacts were moved there recoverably; original unrelated artifacts remain in place.
Archive checkpoint publication fields are historical; this committed readout/handoff
carry the subsequent PR257 publication state.
