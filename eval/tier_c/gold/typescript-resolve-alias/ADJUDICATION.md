# typescript-resolve-alias — ADJUDICATION

Status: DRAFT — controller+Fable review pending.

## Source And Scope

- Repo: `~/code/bench-repos/TypeScript` at `7964e22f2`.
- Target: `resolveAlias` in `src/compiler/checker.ts:4301`.
- Scope applied: alias resolver direct checker consumers plus public TypeChecker API forwarder `getAliasedSymbol` and its code consumers. Generated API baselines excluded.
- Explicit scope stop: do not hop into `getTargetOfAliasDeclaration`; it is called inside `resolveAlias`, not a downstream caller, and task notes say it blows past admission.
- Generator of record: grep-per-hop only. `typescript-language-server` was not used; no prism/LSP/agent enumeration was used.

## Closure Walk

L0 — `resolveAlias`

- Probe: `git -C ~/code/bench-repos/TypeScript grep -nw resolveAlias`
- Count: 32 raw lines / 1 file.
- Excluded from L0: definition at `checker.ts:4301`; comment-only mentions at `4332`, `4344`, `4347`, and `4400`.
- Executable call-token lines: 27, collapsed to 24 real checker sites by enclosing symbol.
- Forwarder: `TypeChecker#getAliasedSymbol` at `checker.ts:1805`, token `resolveAlias@1805`.
- Thinness: the object-literal property maps the public TypeChecker API name directly to `resolveAlias`; if the resolver contract changes, this public API necessarily changes and adds no behavior.
- Direct consumers retained at L0: `resolveSymbol`, `tryResolveAlias`, `getSymbolFlags`, `getTypeOnlyAliasDeclaration`, `resolveEntityName`, `trySymbolTable`, `needsQualification`, `lookupTypeParameterNodes`, `serializeTypeName`, `getTypeOfAlias`, `getDeclaredTypeOfAlias`, `getTypeFromTypeAliasReference`, `markExportSpecifierAliasReferenced`, `markAliasReferenced`, `markExportAsReferenced`, `markEntityNameOrEntityExpressionAsReference`, `getJSXFragmentType`, `checkExportsOnMergedDeclarationsWorker`, `checkAliasSymbol`, `resolveAliasWithDeprecationCheck`, `checkImportEqualsDeclaration`, `isAliasResolvedToValue`, `getTypeReferenceSerializationKind`.
- Consumer rationale: these sites add policy gates, circularity guards, symbol-flag semantics, type-only handling, accessibility/qualification, node-builder serialization, reference marking, checks, diagnostics, deprecation handling, or emit serialization.

Hop 1 — `getAliasedSymbol`

- Probe: `git -C ~/code/bench-repos/TypeScript grep -nw getAliasedSymbol`
- Count: 21 raw lines / 16 files.
- Excluded: implementing wrapper already counted at `checker.ts:1805`; interface declaration `types.ts:5264`; comment-only mention `symbolDisplay.ts:587`; generated API baseline `tests/baselines/reference/api/typescript.d.ts:6279`.
- Real hop-1 consumer sites: 17 sites / 13 files.
- Consumer files: `scripts/dtsBundler.mjs`, `src/compiler/moduleSpecifiers.ts`, `src/compiler/utilities.ts`, `src/services/callHierarchy.ts`, `src/services/classifier.ts`, `src/services/classifier2020.ts`, `src/services/codefixes/fixAddMissingMember.ts`, `src/services/codefixes/importFixes.ts`, `src/services/findAllReferences.ts`, `src/services/goToDefinition.ts`, `src/services/navigateTo.ts`, `src/services/rename.ts`, `src/services/symbolDisplay.ts`.
- Termination: all hop-1 callers consume the aliased symbol for bundling, module specifier resolution, utilities, service classification/navigation/codefix/reference/display behavior, or diagnostics. No further public forwarder was admitted.

## Counts

- Real gold sites: 41.
- D1 site count: 17.
- D1 files: `scripts/dtsBundler.mjs`, `src/compiler/moduleSpecifiers.ts`, `src/compiler/utilities.ts`, `src/services/callHierarchy.ts`, `src/services/classifier.ts`, `src/services/classifier2020.ts`, `src/services/codefixes/fixAddMissingMember.ts`, `src/services/codefixes/importFixes.ts`, `src/services/findAllReferences.ts`, `src/services/goToDefinition.ts`, `src/services/navigateTo.ts`, `src/services/rename.ts`, `src/services/symbolDisplay.ts`.
- Scorer D denominator is file-level: `d_gold_size=13`.
- D2 count: 0. Verified repo-wide `resolveAlias` word count is 32, below the >100 D2 threshold.
- Admission: PASS (`8 <= 41 <= 60`, D1 sites `17 >= 3`).

## Dry Run

Command run from `eval/`:

```text
uv run python -c "from tier_c.structural import score_structural,load_gold; g=load_gold('tier_c/gold/typescript-resolve-alias/gold.json'); perfect=[{'file':s['file'],'symbol':s['symbol']} for s in g['sites'] if s['adjudication']=='real']; r=score_structural(perfect,g); print('perfect',r.file_f1,r.d_recall,r.gold_size,r.d_gold_size,r.phantom)"
```

Output:

```text
perfect 1.0 1.0 41 13 0
```

## Exclusions

- Definition/comment surface: `checker.ts:4301`, `checker.ts:4332`, `checker.ts:4344`, `checker.ts:4347`, `checker.ts:4400`.
- Interface/generated API surface: `src/compiler/types.ts:5264`, `tests/baselines/reference/api/typescript.d.ts:6279`.
- Comment-only hop-1 hit: `src/services/symbolDisplay.ts:587`.
- Explicit out-of-scope non-hop: `getTargetOfAliasDeclaration` at `checker.ts:4253`; `resolveAlias` calls it at `4308`, but the task says not to recurse into it.

## Scout Variance

- The scout note said 23 internal callers and 12 external consumer files. Source verification found 27 executable `resolveAlias` call-token lines collapsing to 24 checker sites because the two createNodeBuilder calls live in distinct local helpers, `lookupTypeParameterNodes` and `serializeTypeName`.
- The scout note counted external files; this gold counts 17 external hop-1 consumer sites across 13 D1 files.

## Uncertain / Review

- `src/compiler/utilities.ts:skipAlias` is a small public utility around `getAliasedSymbol`. I classified it as a consumer boundary per the task scope ("public-API consumers") rather than a further forwarder to recurse.
- The two createNodeBuilder-local sites are reported as immediate enclosing local functions (`lookupTypeParameterNodes`, `serializeTypeName`) rather than one outer `createNodeBuilder` site; this follows the immediate enclosing-symbol rule but should be reviewed.
- `scripts/dtsBundler.mjs:resolveSymbol` resembles the checker helper name but uses the public TypeChecker API in the bundler script; I treated it as a hop-1 consumer and did not recurse.
