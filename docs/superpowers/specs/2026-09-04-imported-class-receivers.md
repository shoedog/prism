# Imported class receiver identity

Base: merged PR #238, `350cc89f686705e28745c9abeb7b76e1c58ee8fc`.
Status: implemented and published as `99d50f1` in [PR #239](https://github.com/shoedog/prism/pull/239), open and not merged. Full suites pass; Tier-A quick retains the corpus-pin exclusion recorded in the handoff.
Owner approved the recommended continuation. Three bounded slices, each with a
three-round self-review cap; no independent review claim.

## Census and corrected assumptions

Read-only syntactic sample, not a corpus accuracy measurement: Excalidraw
`packages/excalidraw/animatedTrail.ts:95-110` imports AnimationController and
calls static methods (excluded); `components/App.tsx:835,843` constructs History
into fields (excluded); `data/library.ts:574` constructs a local AdapterTransaction
(same-file declaration, then passed into a callback; not an imported receiver gain). Django `contrib/admin/filters.py`
imports `django.db.models`, but `django/db/__init__.py` executes imports, assignments
and calls (excluded by the first initializer contract). This sample does not prove
a corpus-wide recall gain. Old unresolved percentages and pin-invalid Tier-A quick
are not prioritization evidence.

Source inspection shows BOTH export extraction and clean class indexing skip the
JS/TS export wrapper. Fixing only exported names cannot recover methods.

## Slice 1: exported declaration identity

Record directly named `export class C` in a distinct Class export target. Do not
project it into the free-function export table. Retain its identity internally
while detecting class/function barrel conflicts. Index the enclosed declaration
as a clean module class using the existing occurrence-clean rules. Unsupported
default/anonymous/decorated/abstract classes and reexports do not acquire imported
class authority. Duplicate export names poison identity, as existing exports do.

## Slice 2: imported JS/TS receiver recovery

Accept module-level ESM named value imports, including local aliases, from exact
relative indexed modules; require one module candidate, one eligible local import,
one direct Class export and a clean class in that defining file. Bare constructor
origins and TS/TSX bare parameter annotations reuse the lexical, type-namespace,
dominance and mutation fences. Imported ownership is terminal: a failed proof
must not retry a same-file/global owner. Methods remain nonstatic, direct and
unambiguous. Default/namespace/type-only/CommonJS imports, reexports, package
aliases, inherited methods, factories and receiver fields remain excluded.

Review repairs: a visible module-class write revokes the class proof (including
escaping callable writes; lexical shadows are excluded). Accessors, same-name
instance fields, unknown computed members and explicit `this` member writes fence
direct-method recovery; unrelated/static fields do not. This is bounded syntactic
authority, not a JavaScript runtime interpreter or proof against reflection and
arbitrary monkeypatching. Failed eligible unshadowed JS/TS member imports are
terminal and must not fall through to a global same-name function.

Keep local recovered spelling in CallSite; resolve defining identity at consult
time from freshly merged facts. Export changes in unchanged importers must match
full builds; exercise source-to-cache-to-incremental and served navigation edges.
Do not change existing free-function module precedence to implement this slice.

## Slice 3: Python inert regular packages

Extend absolute `from pkg import models [as m]` only when every indexed regular
package initializer along the prefix is syntactically inert: comments, pass and
plain string docstrings only, no parse recovery or interpolation. Reject a
competing module file, executable initializer, missing/unproven indexed package
initializer, and ambiguous target before class lookup. Namespace-package behavior
remains otherwise unchanged. No dynamic import execution, __getattr__, __all__,
reexports or source-root inference. Persist initializer proof and include it in
the existing imported-class proof comparison so unchanged importers re-extract
on eligibility transitions.

## Acceptance

RED against the same-environment base, exact targets plus shadow/mutation/ambiguity
negatives, full/subset/cached/incremental parity and navigation sidecar roundtrip.
Bump CPG and sidecar versions when serialized evidence/resolved topology changes.
This PR uses CPG 61 and navigation sidecar 30 for the combined schema transition.
Run full default and MCP suites, format/check/Clippy and rebuilt Tier-A matrix and
quick. Report exclusions and invalid corpus anchors; never rebaseline to pass.
