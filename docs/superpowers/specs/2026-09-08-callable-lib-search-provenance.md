# Primary adopted D1-L design: actual lib searches and positive beneficiaries

Read the [S3 contract](2026-09-08-callable-type-search-provenance.md) and
[H1 identity contract](2026-09-08-callable-identity-domains.md) first.
This is allocated discovered-work slice D1, after S3-T,
before checkpoint one. Schema15 / producer0.16.0, observation only. Keep historical
10–14 parser behavior, module/type arrays and all old outcomes unchanged. No runtime,
closure, dependency, automatic-discovery or config-provenance expansion.

Replace the currently selected pinned TS internal library wrapper with host.resolveLibrary
(libraryName,resolveFrom,compilerOptions,libFileName), using the exact separate shared
ts.createModuleResolutionCache(virtual('project'),canonicalFile,parsed.options,undefined).
No module cache/getModuleResolutionCache introduced. No extra host calls for setup.
Invoke ts.resolveLibrary once within a lib context; target projection uses pureId.
Claim execution ordinal before resolver; bound execution array100000 independently.
Keep invocation order, never replay to recover history. The Program already caches
by libFileName; duplicate demands do not imply duplicate hook invocations.

search_provenance gains lib_searches:
{id,name,from,lib_file,target,selected,beneficiaries}.
name is actual libraryName, from actual resolveFrom projected with H1's pure
projectSyntheticAddress lexical helper (never pureId/canonicalFile/link traversal),
lib_file is actual libFileName. target is actual resolvedModule filename projected
purely (nullable). selected is actual Program cached .actual path projected purely
(nullable); it may be a MISSING compiler fallback, so don't demand snapshot/Program
membership. After construction reconcile exact cache key, actual resolution result
identity and actual selected path using program.resolvedLibReferences. Missing or
contradictory cache state refuses observation rather than inventing a selected file.
Boundary owner channel lib points at search id; nested context restoration unchanged.

Bind inferred from to config directory/__lib_node_modules_lookup_<lib_file>__.ts.
Bind name using pinned getLibraryNameFromLibFileName algorithm (TS126701): split
lib_file at dots; component1 followed by slash component2 then hyphen components
until d; prefix @typescript/lib-. Do NOT treat synthetic from as a source file.

beneficiaries are inclusion-backed observations, NOT causal requests/demands:
{origin:'source'|'configured',request:anchor|null,index}.
Only existing OBSERVED lib rows qualify. Source row ts.libMap.get(name.toLowerCase())
must equal lib_file, and target must equal selected. Configured entry row.name must
equal lib_file with target selected. Request anchor/index comes from that exact row.
No default entry beneficiary, unprocessed/missing-inclusion borrowing, or fabricated
search for default selection/libReplacement:false. Missing fallback search may have
zero beneficiaries; existing unproven rows preserve unmet obligations.
Order beneficiaries deterministically: source rows in existing ledger order, then
configured entry rows in existing ledger order. Bound each array AND total beneficiary
relationships100000 before pushing. Duplicates in config/source retain all indices.

Parser: IDs ordinal, unique lib_file, names/from derivations, safe nullable targets;
owners exist. Every supplied beneficiary must match exact observed source/entry row,
anchor/index and selected target. Require exact complete beneficiary population FOR
EACH submitted search, not just per-item membership. Existing rows cannot establish
that a hook ran (replacement:false/suppression), so do not claim parser alone proves
all searches present. Full reproduction authenticates searches and actual selections.

For pre-I/O source-row/lib-file joins, a small pure pinned mapping helper is permitted:
lib.<lowercase-name>.d.ts except the nine TS5.9.3 aliases printed below. It is used
only on existing observed lib rows, not to assert an unknown lib name resolves.
Add pinned-compiler test that it equals EVERY entry in ts.libMap, including aliases.
es6=>es2015; es7=>es2016; esnext.symbol=>es2019.symbol;
esnext.asynciterable=>es2018.asynciterable; esnext.bigint=>es2020.bigint;
esnext.string=>es2024.string; esnext.weakref=>es2021.weakref;
esnext.object=>es2024.object; esnext.regexp=>es2024.regexp.
Producer should use actual ts.libMap; mapping is parser contract, not authority.

RED-first new producer population against exact frozen S3: replacement success,
miss/builtin fallback, missing builtin fallback (disposable compiler-library copy),
shared source+configured lib and duplicate configured entries, alias source reference,
libReplacement:false no executions despite positive rows, default selection no
fabricated search but transitive source searches possible, noLib suppression,
no-default-lib suppresses configured beneficiary despite source cache/membership.
Negative parser orphan owners, wrong from/name/key, omitted/duplicate beneficiary,
source anchor and genuine contributor swaps, same-genuine selected/target substitution
with reproduce rejection; no private data. Helpers caps and nested module/type/lib
throw contexts, cache source comparison. Full copied observer suite before delivery.

Preserve H1's identity-domains.mjs in the producer digest. Read the repository H1
contract fully. Include file/directory/multi-hop config alias and mixed-case library
lookup controls: synthetic from preserves selected lexical spelling, while source
anchors and target/selected identities remain canonical. When advancing SCHEMA,
audit current-schema-only type-batch guards so historical schema14 still validates
its own batch contract and accepts genuine packets. Capture exact-base RED output.

Primary then checks public previous fields identical, all6893 module occurrences,
23 type batches and29 type requests/searches unchanged, exactly328 former null-owned outside lib encounters
newly attributed; all91409 basic-host operations ordered equal to exact S3. Public
zero remaining null boundary owners is only a fixed-population observation, not a
claim of global ownership completeness or outside absence. Also cache-fixture
transcript equality for replacement, duplicate and suppressed cases. Full repository
gates on clean implementation HEAD and read-only checkpoint after publication.
