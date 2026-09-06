# Contextual imported object-alias identity

Approved successor to PR257 merge661a95a4. Three-round SELF-PASS cap (not
independent); Tier-A quick cap2. No React.FC or hook expansion.

## Contract and architecture

Reuse the existing source-backed contextual parameter producer for a required,
unannotated destructured parameter of a directly annotated arrow/function
expression. Supported producers remain direct function types and module-local
function/callable-object aliases or private callable interfaces, with their existing
single-binder whole-parameter substitution. The selected Props must be one direct
relative named import of a non-generic directly exported object alias.

The producer returns the original annotation/argument AST node. Check imported
type shadowing at THAT node, not the implementation parameter. Keep receiver
write anchors at the implementation. Reuse PR257 declaration-file/property/class
proofs, snapshot-wide barriers and replacement on incremental merge. Explicit
parameter annotations remain terminal even if unsupported; no contextual fallback.
No serialization shape change; bump CPG77/nav45 because edge authority expands.

No imported callable signatures, imported interfaces, barrels, inference, framework
names, React.FC, hook returns or wider TypeScript-program closure. Local routes
remain unchanged. Six pinned real sites are expected to remain unresolved.

## Plan / acceptance

1. Capture RED on unchanged661a95a4 runtime for each contextual producer family.
   Alternative cause: foreign class resolution failure; explicit imported control
   distinguishes it from a missing contextual producer.
2. Reuse the same contextual-node selection for local and imported routes. Enumerate
   positive and negative TS/TSX full/subset cases before the next fixing round.
3. Exercise contextual disk-cache owner/deletion/augmentation transitions in both
   directions and sidecar identity/invalidation. Compare fresh/incremental maps.
4. Pinned TS5.9.3 compiler fixtures; full default/MCP suites, fmt/clippy; release
   rebuild then Tier-A matrix and quick. Report invalid baselines without rebaseline.
5. Replay pinned real sites, archive evidence, reconcile merge records and handoff,
   commit/push/open PR under standing publication approval. No merge authority.

## React.FC follow-up guidance (not implemented here)

Authoring style is separate from analysis semantics. React's guide demonstrates
direct props annotations: https://react.dev/learn/typescript . FC is still an alias
of FunctionComponent in DefinitelyTyped, not a generally deprecated type:
https://github.com/DefinitelyTyped/DefinitelyTyped/blob/master/types/react/index.d.ts .
The v17 declaration accepts PropsWithChildren<P>; current master accepts P:
https://github.com/DefinitelyTyped/DefinitelyTyped/blob/master/types/react/v17/index.d.ts .
These links were checked2026-09-06; they are explanatory, not pinned proof inputs.
The v18 declaration also takes P (with an optional legacy-context parameter):
https://github.com/DefinitelyTyped/DefinitelyTyped/blob/master/types/react/v18/index.d.ts .
Context comes from TypeScript semantics, not an authoring preference:
https://www.typescriptlang.org/docs/handbook/type-inference#contextual-typing .
The refreshed pinned real-source audit records @types/react19.0.10 and
TypeScript5.9.3; these, not current web master, should seed any future FC fixtures.

Recommended separate design: derive contextual receiver provenance from the actual
TypeScript program and resolved declaration bytes. Resolve aliases/imports at the
use site, instantiate the call signature, track relevant merging/augmentation,
and retain selected property/class declaration identity. Missing dependencies,
ambiguous/unsupported signatures and any/unknown do not authorize Exact. Cache
identity must cover compiler version/options, resolved modules, all relevant input
bytes and membership changes. A pinned React declaration profile is a possible
bounded alternative only if it checks identity and augmentation, not package/name
spelling alone. Static component properties are distinct from props members.
Neither contextual typing nor a compiler signature proves runtime instance identity
on its own: existing receiver/member/write barriers remain independently required.

## Diagnostic / review log

Source inspection: imported proof requests currently read only explicit parameter
annotations, while the local path already uses contextual nodes. Prism returned
the expected parameter-binding caller but warned StaleIndex; read source is the
current authority. LSP tools absent.

Round1: red.log and red-population.log capture missing Exact on unchanged base
for all10 contextual positive forms; explicit controls pass (rules out foreign
class lookup as the cause). Two unnamed wrapper negatives had no indexed call:
inadmissible setup evidence, replaced with named-function controls.
The production change only reuses the contextual type-node producer.

Round1 candidate: all full positives pass; caller-only subsets lack target methods.
Alternative was lost contextual proof. Source build_direct_subset Phase1 filters
methods to only_files; the corrected test asserts equal nonempty proof maps,
no invented partial edge, and Exact when the target is included. This corrects a
test assumption, not the product contract. Round2-integration.log:7/7 pass.

Round2:34 contextual negatives plus27 defining-source/ownership/snapshot fences
replayed contextually; all TS/TSX full/direct-subset. Disk64 bidirectional
transitions (32 new contextual,32 explicit controls); sidecar8 states (four new).
Owner A/B, missing alias, augmentation, ambiguous module, barrel, prototype,
explicit-any override and use-site generic shadow all replace old proof and match
full builds. round2-cache.log:4/4 pass. Compiler contextual11/11 and audit24/24.
Round3: reviewed the final source diff and all new branches against explicit
terminal behavior, original-node shadowing, partial/full proof scope and cache
owner replacement. No in-scope WRONG demonstrated. SMELL: dependency/program
augmentation closure remains intentionally unproved; no broadening or downgrade
of an inherited WRONG. SELF-PASS, NOT INDEPENDENT; cap3 complete.
Full default4016/0/1 including two doctests; immediate release/matrix159/159.
MCP4206/0/1 including two doctests; Clippy completes with warnings, fmt/diff pass.
Fresh same-environment real replay2780 records byte-identical/376 Exact; served
three newly Exact and three exclusions; Exact callers/callees verified.
Frozen48dfc45 quick complete, source/SUT clean: baseline-invalid SHA drift/C-name4/6,
oracle2/30,SUT0,quiescent,zero stale adjudications,matrix159/159. One run of cap2;
no paired base quick, attribution or rebaseline. PR258 opened; final readout/handoff
carry archive and publication custody. No runtime/test edits aftere3fb6ba.
