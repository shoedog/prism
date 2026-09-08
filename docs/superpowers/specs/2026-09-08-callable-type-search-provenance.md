# Primary design: S3-T type search provenance, then D1-L shared lib searches

Status: implementation ea81801 accepted under H1, full435/4017/4207 gates passed;
final independent H1-integration review round2/2 ACCEPT98. Historical review: after two ordinary rounds and
one disclosed numeric-order verification extension, synthetic config-link identity
exposed a recurring contract gap. The existing artifact was parked and archived,
then resumed under the reviewed [H1 identity contract](2026-09-08-callable-identity-domains.md).
That contract controls lexical synthetic from versus canonical source/target IDs.
No artifact restart or hidden extension. The audit/gate receipt records all integration evidence.

Independent source review approved the architecture, confidence88. Primary adopts
the bounded split: S3-T implements the TYPE half only; D1-L consumes one discovered-
work reserve for the independently cached lib-search/beneficiary distinction exposed
by source review. The first value checkpoint follows both halves. No policy waiver.

Depends on S2 schema13 ledger. No closure/admission/I/O expansion. Current worker
provides resolveModuleNameLiterals but no getModuleResolutionCache; pinned TS thus
uses undefined moduleResolutionCache when constructing its type and lib caches.
Keep that fact invariant: do not introduce a shared module cache in this slice.

Use pinned public resolveTypeReferenceDirectiveReferences and internal resolveLibrary
host hooks to replace exactly the currently selected compiler wrappers, not add a
second pass. Pinned implementation reference: TS126655–126697,127018–127059,
127400–127418,128851–128903. Preserve wrapper caches and callback ordering exactly.

Type hook: one shared ts.createTypeReferenceDirectiveResolutionCache(currentDirectory,
canonicalFile,undefined,undefined,undefined). Each callback gets a fresh Map keyed
by ts.createModeAwareCacheKey(name,mode), exactly loadWithModeAwareCache. Mode is
ts.getModeForFileReference(entry, source &&
ts.getDefaultResolutionModeForFileWorker(source, redirected?.commandLine.options ||
compilerOptions)). Name is string entry or entry.fileName. Invoke
ts.resolveTypeReferenceDirective(name,from,compilerOptions,host,redirected,cache,mode)
only for missing per-batch result. Each invocation is an execution; a global cache
hit can legitimately have zero boundary events. Do not infer global cache-hit state
from an empty event list. Duplicate entries within a batch share an execution but
retain each occurrence. Request and execution are separate records.

S3-T schema14 additions under search_provenance (D1-L uses the next schema):
- type_batches: id, origin(source/configured/automatic), from, size. Actual callback
  visits, including a repeated visit to the same source; independent100000 cap.
- type_requests: id, origin(source/configured/automatic), from, name, mode,
  source request anchor or null, source directive/entry occurrence index, execution id,
  batch id. Repeated occurrences within and across batches are both retained.
- type_searches: id, from, name, mode, resolver target canonical id/null.
- D1-L lib_searches: id, library name, resolve-from id, lib_file name, resolver target
  canonical id/null, selected compiler fallback/replacement canonical id, contributors.
- S3-T boundary owner adds type; D1-L adds lib, id is execution/search ordinal.

Type request source anchors are finalized by joining existing source type rows
after observeTypeLib, using the actual callback source/ref occurrence. Entry request
indices correspond to the actual configured/automatic callback input array and are
reconciled to existing entry rows. Avoid duplicate source reads just to make anchors.
Do not turn source/entry cache reconstructions into an invented callback. Source
linkage is one-to-one WITHIN A BATCH by retained callback occurrence/ref index/object
identity, then exact existing row anchor, NOT a name/from/mode-only join. Entry `from` is the
derived synthetic safe ID in config directory (`__inferred type names__.ts`), not
a snapshot or Program source file. Bind its derivation; do not demand membership.

Round1 primary design correction, source-backed counterexample: a package imported
under node_modules and then explicitly processed as a root can trigger type callbacks
again (TS128640–128649,128801–128812). A global unique-source-row check wrongly
collapses that formerly supported packet to unsupported_input. Preserve both actual
visits, each with its own fresh batch cache and distinct execution ordinals. Evidence:
probe-s3-revisit.mjs / s3-revisit-result.json, exact S2 normal versus S3 refusal.
Never deduplicate callback visits by syntax/cached final resolution.

Parser must require exact per-batch size, from/origin, indices and source/entry row
population. Use existing type rows with reason!='unprocessed' (source noResolve,
redirect originals and rootless configured rows have no actual callback). Every
such retained row needs at least one matching batch; repeat batches may name the
same source/rows. All requests point at a batch; each batch's requests remain in
actual callback order and exactly cover its expected indices, retaining duplicate
names. Within a batch identical name+mode requests share execution; no execution
can be borrowed across batches. Execution records stay separate from batch identity
and must all be used. Bound batch sizes, totals and arrays independently. An empty
actual batch may have size0 only when its expected row population is empty. Full
reproduction, not parser alone, authenticates the count/order of repeated visits.

Round1 independent omission finding: dropping the second request of configured
types:['missing','missing'] parsed before repair. Per-batch exact population rejects
that omission without incorrectly forbidding genuine source revisits. Required RED
also covers source and automatic duplicate omissions, noResolve/rootless no invented
callbacks, and positive imported-then-root revisit with same-environment old-field
and ordered host transcript equality.

Construct caches with the already-known virtual('project') current directory: no
extra getCurrentDirectory host call. Project new targets via S2 PURE classifier,
not legacy toId, so new serialization cannot invent legacy boundary events.

Lib hook: one shared ts.createModuleResolutionCache(currentDirectory,canonicalFile,
parsed.options,undefined), invoke ts.resolveLibrary(name,from,options,host,cache) once
inside try/finally execution context. Pinned Program caches libFileName globally.
No initiating-source claim from this hook. Its resolveFrom is a synthetic safe ID
derived from config directory and lib_file, not a snapshot/Program source. After Program construction, reconcile
the same lib key against program.resolvedLibReferences; retain actual fallback/target
and all source/config contributors whose EXISTING rows have actual inclusion proof.
A shared config+source lib has one search and multiple contributors. Suppressed or
unprocessed demands stay visible in old ledgers but are not borrowed as positive
contributors. `libReplacement:false` and default lib selection have no search; never
fabricate a search to populate the ledger. Default's transitive lib refs can search.
Normative terminology: contributors are inclusion-backed BENEFICIARIES, not causal
demands. A missing fallback can have a real search but zero positive beneficiaries.
Never interpret zero beneficiaries as zero obligations; unfulfilled demands remain
in existing source/entry ledgers and the later entry-obligation slice owns their
policy. Prefer field name beneficiaries in D1-L; no request-free completeness claim.

Automatic discovery before the type hook remains null-owned in this slice; mark it
explicitly outside complete attribution claims. Config parsing also remains unowned.
The public fixture has configured types, so actual existing outside events should
all become attributed after D1-L: module448/lib328/type30, refused module1269.
After S3-T alone, exactly328 public lib outside events remain null-owned. No path-based
inference in production. Existing S2 module rows and boundary event identities/order
must remain equal except deliberate owner attribution for previous nulls.

Bound all new arrays and contributor relationships independently (100000 max),
and preserve packet byte cap. Parse ordinal/membership/anchor/source-entry links,
unique lib keys, exact target and contributor relationships, actual modes enum;
reproduction authenticates compiler facts. Historical10–13 parser remains. Both
runtime/class authority flags false. Failure and overflow return empty refusal.

Required real RED controls before production edits: source type callback vs entry;
configured duplicate names sharing one execution; duplicate automatic names across
typeRoots; source conditional import/require mode; same names from multiple source
files with shared cache; unresolved type remains visible/react-scripts-like; lib
replacement success/miss/fallback; same lib from source+config and duplicate config
entries shares search and retains contributors; replacement:false no search; default
selection no fabricated search; stale/same-genuine contributor swaps rejected by
validate; orphan owners and invalid source/entry links rejected pre-root I/O.
Guard tests for nested/throwing execution restoration and independent caps.

Ordered actual host-operation transcripts and all previous packet fields must match
exact same-environment base for public and key cache fixtures. Main capture utility:
/private/tmp/prism-overnight-0zmx3n/capture-host-transcript.mjs (wraps existing basic
host calls, hashes results, no second resolver). S1 public has91409 such operations,
and its capture changes no packet field except producer hash. If any transcript
differs, stop implementation expansion and determine why; do not bless it from final
packet equality alone. If wrapper/contributor work becomes too large for two-round
review, allocate D1 to the lib half explicitly and checkpoint only after both halves.
