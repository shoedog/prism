# P2a — fixed-source syntax integrity and frequency census

Normative with `AMENDMENT.md`; planning draft, no execution yet. The inherited P2 spec's integrity, source/compiler custody, deterministic identity, recovery, offset and mutation rules remain binding where applicable. This document explicitly narrows the active output and proof population.

## Active budget and bounds

Keep the **original ceilings**: Rust observer/schema≤900 lines, compiler observer≤350, all tests/fixtures/characterization helpers≤850, combined active source≤2100. Count formatted nonblank-and-blank physical lines consistently as before; docs/raw outputs/snapshots are separate custody, never executable loopholes. Worker estimates650–750Rust before finishing syntax obligations,190–220compiler,500–700tests make this plausible, not guaranteed. Before extraction attach a short seam/remaining-work allocation; stop on a forecast/actual breach, no source compression or automatic amendment.

Keep512manifest members,8MiBsource,200,000combined output rows,128MiBserialized output,15minwall limit. The wall deadline covers both observers and serialization, with a controlled owned-process shutdown. Independent synthetic controls exercise each limit under reduced test ceilings, not public default changes.

## Output schema and complete syntax population

Emit only `prism.p2a-syntax-census.v1`, `authorizes_runtime_edge:false`, `measurement_kind:syntax_frequency_only`. No v1 full-readiness envelope with silently missing native ledgers. Output includes provenance, files, callables, parameters, syntax joins and frequency tables. Do not emit `constructible`, `supported`, native-owner/slot/entry/binding/Step5b fields or decisions.

- **files:** exactly one terminal row per member; hash/bytes/language/declaration/test strata, independent parse/diagnostic status/count, observed callable/parameter counts. Use the exclusive terminal decision table below; unknown status is a schema error. Recovery is an observation and processing continues; packet integrity failure is fatal/incomplete with no complete-public frequency claim.
- **callables:** independent direct grammar and TypeScript walks, not `all_functions()`. One row per observed callable, including zero parameters and anonymous/signature-only forms. Record raw/canonical kind, raw/full and token spans, name or null, signature/body presence, class/object/variable-binding syntactic context, parenthesization, recovery state and an ordered parameter-ID array. A zero-length array differs from absent/unrecognized parameter syntax.
- **parameters:** preserve each raw ordinal and complete parameter-node span separately from binding-token/pattern span; exact shape and nested/default/rest/optional dimensions, raw spelling, recovery, callable/file references. No native slots or compressed positional interpretation.
- **joins:** exact per-callable/per-parameter matches between observers after UTF16→UTF8 and canonical kind/shape mapping; retain zero/one/multiple and unmatched raw kinds. No name/line fallback. Keep separate observers' counts; never merge their observations as though they were duplicate native facts.

Before implementation, pin a small explicit raw-kind discovery table from the existing grammars and TS syntax API. Cover function/generator declarations/expressions, arrows, methods/accessors/constructors and declared/signature-only forms; distinguish type-only call/construct/function-type signatures from body-bearing runtime syntax. Every discovered form has an explicit canonical category or `unmatched` row; unknown categories cannot silently disappear. This is observation taxonomy, not admission. Do not grow a new parser or normalize source.

Pinned discovery rows (one row per AST node; descend through non-callable containers):

|Native kind|TS syntax kind|Classification|
|---|---|---|
|function_declaration / generator_function_declaration / function_signature|FunctionDeclaration|declaration; body and generator flags distinguish runtime/signature|
|function_expression / generator_function|FunctionExpression|expression; lexical name/binding and generator dimensions|
|arrow_function|ArrowFunction|parenthesized/unparenthesized; identifier-bound/anonymous|
|method_definition|MethodDeclaration / Constructor / GetAccessor / SetAccessor|member; exact syntax name/modifier/context distinguishes categories|
|method_signature / abstract_method_signature|MethodSignature / bodyless MethodDeclaration|signature-only member, explicit abstract dimension|
|call_signature|CallSignature|type-only call signature|
|construct_signature|ConstructSignature|type-only construct signature|
|function_type|FunctionType|type-only function signature|
|constructor_type|ConstructorType|type-only constructor signature|

`index_signature`, `property_signature`, class containers and parameter property/type nodes do not themselves add callables; descend so nested function-type signatures are observed. Root file counts and recorded diagnostics remain complete even when zero callable rows exist. Keep type-only signature frequencies in a separate context dimension, not executable declaration/body frequencies. A raw kind unexpectedly lacking an agreed cross-observer mapping remains unmatched rather than being omitted. Anonymous/method computed names are raw syntax spans/text, never evaluated names.

Pinned TS node-schema SHA `c790a733fc756b54d4e54dceeb7d2d51e40d8b57136e70277753a75804cce3e3`; TSX `78b5789145286799a27a0a7ecc36cc1bcb151f94ec7fa631b248459867010c8c`. JS grammar follows the accepted native dependency pin and contains only applicable runtime rows; implementation preflight binds its schema hash too. This table defines discovery and distinctions, not proof of current implementation coverage.

Use accepted P2 canonical callable categories for directly compatible forms; retain generator/async/signature/accessor/constructor/context dimensions instead of falsely equating them. Object/array/nested/default/rest remain separate parameter dimensions. A `rest_pattern` is not automatically an object/array destructuring count; classify its contained pattern separately.

## Exclusive file status and numerator rules

For each observer, record exactly `clean` (tree exists, zero recovery/diagnostics), `recovered` (tree exists, one or more recovery nodes/syntax diagnostics), or `unavailable` (no usable tree, explicit per-file failure reason). Counts unavailable to an observer are null, not zero. A compiler-process failure, missing compiler output, or invalid transport/schema makes the **whole packet incomplete**, not a fabricated per-file parse state.

Apply the following rows in order, choosing exactly one terminal file status:

|First true predicate|Terminal status|
|---|---|
|both observer states are unavailable|`unobserved`|
|exactly one observer is unavailable|`syntax_only`|
|either observer is recovered|`recovered_quarantined`|
|both clean and manifest declaration_only=true|`declaration_only`|
|both clean and declaration_only=false|`complete`|

Declaration/test flags are independent strata and never overwritten by terminal status. Thus a recovered `.d.ts` is `recovered_quarantined` with declaration_only=true; native-clean/compiler-unavailable is `syntax_only`. Report the five terminal counts as a partition summing to414, not overlapping statuses.

Per-observer **valid-syntax** numerators include only rows from that observer's clean files with representable mapped spans; rows from its recovered files contribute only to recovery-tagged numerators, even if the particular node lacks a local error. An unavailable observer contributes no invented syntax rows and an explicit unavailable-file denominator. The other observer's own clean rows remain visible in its valid numerator. Cross-observer valid exact-join numerators require both file states clean and both rows mapped. Recovered rows may still have equal match keys, but are recorded as recovery-tagged joins and never counted as valid exact joins. Unknown kind/offset rows remain unmatched with separate counts. `zero_syntax_frequency` requires complete observation for the stated observer/cohort/stratum, with no unavailable or unclassifiable relevant rows; otherwise report incomplete alongside the observed count, not a proven zero.

## Canonical identities

All IDs are SHA256 of canonical JSON arrays prefixed by schema+ledger. File ID `(path,member_hash)`; callable ID `(observer,file_id,token_start,end,canonical_kind,raw_kind,context,signature_only,multiplicity_ordinal)`; parameter ID `(callable_id,raw_ordinal,parameter_token_start,parameter_end,binding_start,binding_end,shape,dimensions,multiplicity_ordinal)`; join ID `(join_level,native_id_or_null,compiler_id_or_null,status,match_ordinal)`. Identity order and array sorting use explicit scalar/code-point ordering, not locale-dependent collation. Duplicate facts retain multiplicity ordinal; IDs and foreign keys must be independently recomputable from full rows. The exact ordered projections below are normative; no unspecified object encoding participates in IDs.

Zero-based half-open UTF8 offsets refer to exact source bytes. Retain TS raw UTF16 `pos/getStart/end`, convert boundaries by code-unit/byte mapping, reject unpaired/unmappable boundaries. Trivia/full spans and token spans stay separate. BMP+astral prefixes, comments, zero-parameter and both arrow spellings are mandatory fixed exact-vector controls.

### Ordered scalar projections

`context` is this exact array, in order:
`[container_kind, container_token_start, container_end, member_role, computed_name, binding_kind, binding_name, is_async, is_generator, is_abstract, signature_only]`.

- `container_kind`: `none`, `class`, `object`, or `type`. It is the **direct syntactic member container** for a method/accessor/constructor/signature, not an arbitrary ancestor of an ordinary function/arrow. For `none`, both container offsets and member_role are null and computed_name=false. Otherwise offsets identify that container's token span. `type` denotes an interface/type-literal member container; standalone function/constructor-type nodes use none.
- `member_role`: `method`, `constructor`, `get`, `set`, `call_signature`, or `construct_signature`, or null as above. A computed method is role method with computed_name=true; its literal raw name span/text is retained separately, never evaluated.
- `binding_kind`: `none`, `identifier`, or `pattern`. Only a direct variable-declarator initializer contributes this binding. `binding_name` is exact identifier text for identifier; otherwise null. A function expression's own lexical name is a separate row field, not this variable-binding name.
- The final four fields and computed_name are JSON booleans, never strings/null. They reflect only present syntax modifiers, generator syntax and body absence. A type-only/signature node has signature_only=true. Unknown/unmappable context uses an explicit unmatched row and null context; it cannot join or collide with the concrete array.

`dimensions` is exactly:
`[is_optional, has_outer_initializer, is_rest, binding_pattern_kind, has_nested_pattern, has_nested_default]`.

The five flag positions are JSON booleans. `binding_pattern_kind` is `identifier`, `object`, `array`, or `other`, after peeling only the parameter's outer optional/default/rest wrappers. `has_outer_initializer` records the parameter initializer, not arbitrary initializer expressions inside nested patterns. `has_nested_pattern` means an object/array binding pattern strictly below the outer binding root. `has_nested_default` means an assignment/default binding strictly below that root, not a type annotation or expression containing `=`. Unknown/unmappable shapes keep raw syntax, shape unknown and dimensions=null, and cannot exact-join. Missing values do not default to false. An empty parameter list has no parameter row; its callable parameter array is empty, distinct from absent/unrecognized parameter syntax.

### Exact join keys and deterministic multiplicity

The callable candidate key is the canonical array:
`[file_id, callable_token_start, callable_end, canonical_kind, context]`.
A key is eligible only when all fields are mapped, context is concrete and canonical_kind is recognized. Raw kind/spelling and leading trivia are retained evidence but are not equality surrogates.

The parameter candidate key is:
`[unique_parent_callable_match_key, raw_ordinal, parameter_token_start, parameter_end, binding_start, binding_end, shape, dimensions]`.
`parameter_token_start` is the whole parameter node's first nontrivia token (`getStart` on TS); `parameter_end` is that whole parameter node's end. `binding_start/end` is the peeled binding identifier/pattern's token span. This records defaults/types/rest syntax without confusing the whole parameter with its binding. Raw TS pos/full spans remain separate diagnostic fields, **excluded** from candidate keys. A parameter can join only if its parent callable group has exactly one row from each observer; otherwise it gets `parent_unmatched`/`parent_ambiguous` and no guessed cross-observer pairing.

For each eligible key, group native and compiler rows without deduplication. Sort each side by canonical row-ID and multiplicity ordinal. If each side has exactly one row, emit one `exact` pair. If both sides are nonempty and either has more than one row, emit the full ordered Cartesian pairs as `ambiguous`, preserving multiplicity; no first-match choice. If one side is empty, emit one `unmatched` row per present row with the other ID null. Ineligible rows emit one `unmatched` row with their specific reason. Assign `match_ordinal` from zero in the sorted emitted group; keep group key and both cardinalities for independent reconciliation. Resource caps apply before Cartesian materialization; overflow is incomplete, never silent truncation. Recovery eligibility is a separate boolean/tag as defined above; it does not change the deterministic key grouping.

Required exact join controls include `(x /*comment*/ = 1)` with distinct raw/trivia versus token spans; same-shaped parameters at different full/token locations; object/class/computed method contexts; duplicate identical-key synthetic rows proving zero/one/many behavior; recovered declarations and native-clean/compiler-unavailable file statuses. Assert all projected arrays, IDs, group cardinalities, emitted join multiplicities and valid/recovery/unknown denominators.

## Frequency questions and denominators

Report separate tables per observer, cohort, language and disjoint strata `(declaration_only,test_path)`. Each shows files scanned, files with diagnostics/recovery, callable count, parameter count, cohort callable/parameter counts, valid-syntax and recovery-tagged counts, exact/unmatched/ambiguous join counts. Include all414file dispositions and reconcile partitions. Frequency outcomes are `nonzero_syntax_frequency`, `zero_syntax_frequency`, `input_blocked` or `incomplete`; they never mean support or constructible demand. Recovery/unknown forms are shown, not silently converted to an exact zero.

Cohorts: (1)object/array destructuring anywhere, separately the same-callable raw-ordinal pattern **before a later optional identifier**; keep nested/default combinations; (2)rest parameters with their preceding raw syntax prefix, no slot inference; (3)arrows split parenthesized/unparenthesized and identifier-bound/anonymous plus parameter forms. Count unique callables separately from parameters so one destructured pattern with multiple leaves is not multiple parameters. Overlap is explicit; publish no combined cohort numerator.

## Complete control population

Reuse stopped syntax fixtures and same-schema baseline concept, but the baseline now emits the P2a protocol and intentionally models table-derived omissions, recovery abortion or bad byte mapping. Preserve genuine wrong/missing-row RED, separately from passing Prism API characterization. Historical caller/Step5b RED remains parked P2b evidence, not P2a verification.

Controls cover: callable syntax absent from native inventory; all discovery-table kinds or explicit unmatched forms; same-name callables at distinct spans; zero parameters versus missing parameter syntax; object/array/nested default/rest/optional-later; both arrow spellings; anonymous/declaration-only context; recovered then valid file; duplicate and escaped identifiers as literal syntax with no invented support/refusal; BMP+astral/trivia exact mapping; complete row-ID/FK/multiplicity recomputation; every fatal manifest/cap/category control; per-observer/cohort/stratum partitions. No callgraph/CPG construction or entry/slot API required.

Use inherited exact80/86/82-byte mutation fixtures and hashes. For P2a assert only syntax/callable/parameter/join/frequency changes. Leading `/*p2*/` shifts unchanged token anchors+6 while TS first `(pos,start)=(0,6)`; recompute full spans per observer. Destructuring insertion asserts the new object span directly, transforms unrelated downstream token coordinates and IDs, changes only declared shape/frequency/join facts, then restores exactly. Do not assert candidate/entry/flow changes in P2a. Stale-member hash fails before parser; refreshed derived manifest binds parent fixture and all hashes. Synthetic data never enters public frequency denominators.

Freeze compiling focused-green source, fixture expected vectors and RED/negative controls for independent core review before any public member is parsed. After approval, run public cold/repeat once each, then controls as needed on disposable fixture copies; complete canonical output must reproduce. On refusal/budget/nondeterminism retain incomplete evidence and stop. Full observer tests required; no Prism full-suite/TierA rerun for an unchanged external-only syntax observer.

## Acceptance and next decision

Accept only complete414-member terminal custody, exact per-observer inventories/joins, reconciled separate frequency tables and actual complete synthetic/mutation proofs. A truthful zero or refusal is acceptable. This independently useful syntax artifact closes P2a only. P2b readiness is **unmeasured**, not zero. Feature-support or closure proposals remain gated on P2b and any later compiler/authority prerequisites.
