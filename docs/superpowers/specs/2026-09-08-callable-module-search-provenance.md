# S2 — bounded module-search provenance

Stacked on PR281, source-equivalent to merged PR280. Schema13 / producer0.14.0.
This is observation-only: no resolution, closure, runtime/class, acquisition or
application behavior expansion. Historical schema10/11/12 remains readable.

## Data and identity

`search_provenance.module_requests[]` catalogs actual module-literal host callback
occurrences: encounter `id`, canonical `from`, `specifier`, compiler mode
(`import`/`require`/null), existing request anchor (null for synthetic), and actual
canonical filesystem target/null. These are callback occurrences, not a census of
all lexical imports: the compiler can short-circuit a source ambient name.

Finalize anchors from the **same** `lookupRequests` records after
`observeExactAmbient` installs `resolution.lookup.request`; no extra source reads
or guessed literal ownership. The parser requires an exact anchor-bearing multiset
match with `resolutions[]`, preserving duplicates and synthetic multiplicity.

`boundary_events[]` records every legacy outside/refused boundary encounter once:
ordinal `id`, `kind`, `operation`, normalized-probe SHA256 and nullable owner
`{channel:"module",id}`. Repeated paths remain repeated events. Operations are
identity/readFile/entries/fileExists/directoryExists/realpath; identity conversion
is not mislabeled as a host filesystem operation. There are no raw unsafe paths.

Null ownership means **not attributed in S2**, not request-free or admissible.
Source/configured/automatic type work, lib replacement and config work remain
outside module ownership. This is a boundary-event ledger, not a log of every
successful or missing in-root candidate probe.

## Execution and limits

Separate pure path classification from the legacy aggregate/event wrapper. Preserve
host return values, canonical link handling, read/missing/refused sets and outside
boolean. A try/finally stack covers resolution, target projection and legacy
insertion. Invoke the existing module resolver exactly once with its existing
arguments; do not create a new module cache or replay the resolver.

Claim module ordinals at callback entry. Independently cap requests and boundary
events at100000; overflow returns an empty budget_exceeded packet, never truncated
evidence. Packet byte, heap, time, inventory and existing receiver budgets remain.
Pass `f=>toId(f)` to array.map; an array index is not an operation label.

The parser validates shapes, ordinals, owner membership, source/anchor relations,
mode enums, exact resolution multiset, refusal digest set and outside flag. These
are structural checks; only independent full reproduction authenticates actual
compiler-derived modes, event owners and probe digests. Genuine owner/digest swaps
and repeated-event omissions must fail reproduction even if structurally consistent.

## Acceptance

Captured exact-base contract RED and candidate GREEN must cover shared paths,
duplicate source literals, virtual refusals, synthetic JSX, unattributed type/lib
events, resolved-package peer metadata, Unicode anchors, parser negatives, genuine
owner/digest substitution and source/config epochs. Unit controls cover pure path
classification, nested and throwing contexts, target projection and independent
caps. Historical schema12 with an unresolved source type reference is a required
compatibility control, not permission to drop an old refusal.

The fixed public replay must retain every previous field except schema/producer:
6,893 callbacks, 2,075 boundary events (1,269 refused and806 outside), all refused
and448 outside module-owned,358 explicitly unowned. Ordered basic virtual-host
operation transcripts must match base, including91,409 public operations and
their return-value digests. Positive source binding is not asset/runtime identity.

Full observer, default Rust, MCP, helper/authority gates and fixed-source custody
precede publication. Review cap two; classify convergence explicitly at the cap.
No Tier-A trigger unless call-resolution/navigation/CPG/AST production files change.
