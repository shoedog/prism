# Merged wildcard source-binding proof requirements

Status: audit/specification only, on merged PR274 (`d66328d1`). No producer,
schema, resolver, cache or closure-policy change. Owner requested source-backed
requirements for the remaining 84 bindings, with asset presence and closure
admission separate. React.FC expansion remains excluded.

## What the fixed population actually contains

The byte-identical PR274 packet contains 84 `*.scss` merged-binding occurrences
in 80 source files. Every request is an `ImportDeclaration` with **no import
clause**, not a default, namespace, named, type, export, or dynamic import.
Each binds these two declarations, independently confirmed against original
Program source bytes:

- `node_modules/vite/client.d.ts:41`: `declare module '*.scss' {}` — empty block.
- `packages/excalidraw/global.d.ts:59`: `declare module "*.scss";` — bodyless shorthand.

There are no additional matching patterns, exact-name providers or relevant
augmentations at these 84 requests. Inventory contains all 75 distinct relative
asset candidates (84 occurrences). That is a captured-file/hash observation,
not a module-loader, bundler, runtime availability or closure proof.

The current `duplicate_provider` refusal is intentional and correct. It is not
a demonstrated production defect. Two declarations are not two equivalent
typed providers, and clean diagnostics cannot establish that equivalence.

## Compiler mechanism and counterexamples

Authority is the pinned TypeScript 5.9.3 distribution, SHA256
`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.
Line numbers below refer to its `lib/typescript.js`, not current upstream:

| Source | Mechanism relevant to the proof |
|---|---|
| `setValueDeclaration`, 18825 | For the two same-kind ambient declarations in these controls, the first value declaration is retained. Do not generalize this to all symbol kinds. |
| `mergeSymbol`, 52261–52310 | Appends declarations, merges exports/members, and consults `setValueDeclaration`; declaration multiplicity alone says nothing about resulting value semantics. |
| `isShorthandAmbientModuleSymbol`, 17329 | Tests the selected `valueDeclaration`, not whether any member of `declarations` is bodyless. |
| `getTargetofModuleDefault`, 53207; import member resolution, 53413 | Shorthand selected declaration enables otherwise absent default/named imports. |
| `getTypeOfFuncClassEnumModule`, 61356 | A shorthand module symbol supplies `any`. |
| `resolveExternalModule`, 54132–54140 | Pattern selection and request-specific augmentation participate in binding. |
| `mergeModuleAugmentation`, 52382–52447 | Augmentation can alter the merged symbol; it is not another harmless provider. |

Captured compiler controls use `noUncheckedSideEffectImports:true`:

| Provider order | Default / named import result | Side-effect request |
|---|---|---|
| Empty, shorthand | Missing default (1192), missing named exports (2305) | No diagnostics |
| Shorthand, empty | All imported values are `any`, no diagnostics | No diagnostics |
| Typed `value:string`, shorthand | `value` is `string`; missing names diagnose | No diagnostics |
| Shorthand, typed `value:string` | Even `value` is `any`; no diagnostics | No diagnostics |
| Duplicate typed declarations | Two binding anchors still exist | Duplicate value diagnostics (2451) |

Thus diagnostics alone do not distinguish semantics, and a reported `any` can
also be error recovery for a rejected import. The distinguishing observations
are selected source declaration, import context, alias/type result **and**
diagnostics together. The public packet does not record `valueDeclaration`:
its actual public identity is **not measured** by this audit. We do not infer
it from sorted Program files, census order or declaration-array order.

## Recommended next slice: observation-only empty/shorthand side-effect pairs

Implement a separately identifiable, bounded observation, preserving the old
exact/wildcard dispositions including `duplicate_provider`. Any additive wire
field needs its own schema/producer revision, strict parser and full-recompute
validation. This specification is not authority to admit closure or values.

All of the following are required for an eligible observation:

1. **Request identity:** an owned source literal at the actual moduleSpecifier
   of a side-effect-only import. No synthetic or recovered range; verify source
   hash, UTF-16/byte spans, parent and absence of `importClause`.
2. **Actual binding:** ask the configured Program's checker for this exact
   request's symbol. Require a supported ValueModule, exactly two distinct
   source-owned module declarations and identity equality with the census.
   No declaration-name lookup or spelling-based substitute.
3. **Closed contributor census, not closed Program:** census original parsed
   source behind redirects for all configured Program members. Exactly two
   top-level non-external `.d.ts` providers, same valid one-star pattern, one
   empty ModuleBlock and one bodyless shorthand. Both must have actual source
   ownership; excluded inventory files cannot supply or remove providers.
   Third/duplicate, nested, typed, nonempty or unsupported providers refuse.
4. **Selection exclusions:** independently enumerate all patterns matching
   the raw case-sensitive request, all exact-name providers and all matching
   wildcard/request-specific augmentations. Require only the selected pair;
   never let an earlier `duplicate_provider` reason hide competition or
   augmentation. Longest-prefix compiler selection alone is insufficient.
5. **Order-sensitive provenance:** record/check the actual checker-selected
   value declaration and its empty/shorthand shape in that same Program; it
   must be one of the two owned contributors. Preserve declaration order as
   evidence, never sort it into a fabricated selection rule. Either selected
   shape may explain a side-effect binding; neither authorizes value imports.
6. **Epoch and bounds:** retain snapshot/options/producer/compiler identity,
   budgets, refusal/outside-lookup ledgers and Program-local caches. Provider
   body/order, request context, membership, augmentation and competing-pattern
   edits must invalidate cached/serialized observations. No persistent
   name-only cache; package-ID redirects must retain original-source census.
7. **Non-authority:** filesystem target stays null; all existing closure bits,
   residual source gaps, receiver-write barriers and class refusals retain
   their existing meaning. Runtime/class authority remains false. Asset
   presence is not an eligibility input and must not enter this lane's proof.

The useful result is “this side-effect request binds this bounded source pair
in this Program,” not “SCSS resolves,” “the asset loads,” or “Program is closed.”
No general declaration merging, typed providers, React.FC, asset loader model,
or closure-policy expansion belongs in that implementation.

## Negative-fixture requirements before that implementation

Characterization controls added here pass unchanged production (9/9 on exact
base); they are **not implementation RED**. Existing wildcard tests cover
augmentation, competing/exact patterns, excluded/nested providers, redirects,
receiver writes and stale/forged packets. These establish current refusals,
not coverage of a future observation field.

The next implementation must first capture exact-base failures for its new
positive record and source/selection fields. Add future-lane negatives for:

- Default/named/namespace/type/export/dynamic/require and synthetic contexts.
- Both declaration orders; empty/empty, shorthand/shorthand, typed/shorthand,
  third same-name, nested and non-declaration-file contributors.
- Exact and wildcard augmentations, competing patterns including tied prefix,
  excluded inventory-only provider, and redirected source ownership mismatch.
- Genuine declaration substitution, changed body/context/order/Program epoch,
  forged selected declaration and malformed fields rejected before root I/O.
- Identical binding with asset present/absent; null filesystem target and
  unchanged closure/class/write-barrier results in both cases.

Measure the fixed 84 afterward, with complete old-field projection equality.
Do not predict a positive count until actual selected-declaration provenance
is measured. Keep closure-policy design in a separately approved slice.

## Execution and stop conditions

This audit: source/compiler probes → characterization/base control → exhaustive
84-site source audit → full observer/default/MCP/authority gates → two SELF-PASS
rounds → commit/push/PR. No independent reviewer is claimed. A new unbounded
merge category is a stop/design question, not a reason to broaden the slice.

Evidence and reproduction: [readout](../../eval/receiver-closure/2026-09-07-callable-merged-wildcard-proof.md),
[audit script](../../eval/receiver-closure/audit-merged-wildcard.mjs),
[fixtures](../../../scripts/callable-observations/merged-wildcard-proof.test.mjs).
