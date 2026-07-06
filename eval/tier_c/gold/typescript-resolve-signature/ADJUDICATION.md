# typescript-resolve-signature — ADJUDICATION

Status: DRAFT — controller+Fable review pending.

## Source And Scope

- Repo: `~/code/bench-repos/TypeScript` at `7964e22f2`.
- Target: `resolveSignature` in `src/compiler/checker.ts:37631`.
- Scope applied: resolver, thin public/checker wrappers, checker-internal consumers, and external public API consumers. Tests/generated API baselines excluded.
- Generator of record: grep-per-hop only. `typescript-language-server` was not used; no prism/LSP/agent enumeration was used.

## Closure Walk

L0 — `resolveSignature`

- Probe: `git -C ~/code/bench-repos/TypeScript grep -nw resolveSignature`
- Count: 3 raw lines / 1 file.
- True caller: `getResolvedSignature` at `checker.ts:37658`, token `resolveSignature@37679`.
- Excluded: definition at `37631` and assert string at `37648`.
- Thinness: `getResolvedSignature` adds cache/resolution-stack plumbing, then calls `resolveSignature`; if the resolver contract changes, this wrapper necessarily changes.

Hop 1 — `getResolvedSignature`

- Probe: `git -C ~/code/bench-repos/TypeScript grep -nw getResolvedSignature`
- Count: 24 raw lines / 9 files.
- Forwarder: `getResolvedSignatureWorker` at `checker.ts:2040`, token `getResolvedSignature@2043`.
- Public thin wrapper: `TypeChecker#getResolvedSignature` at `checker.ts:1778`, token `getResolvedSignatureWorker@1778`.
- Internal consumers in `checker.ts`: `getEffectsSignature`, `getContextualTypeForArgumentAtIndex`, `checkJsxOpeningLikeElementOrOpeningFragment`, `checkCallExpression`, `checkTaggedTemplateExpression`, `checkInstanceOfExpression`, `checkDecorator`.
- External public API consumers: `argument-trivia.cjs`, `fixAddMissingMember.ts`, `fixAddVoidToPromise.ts`, `goToDefinition.ts`, `inlayHints.ts`, `symbolDisplay.ts`.
- Excluded: comments, `types.ts` interface declaration, generated API baseline.

Hop 2 — `getResolvedSignatureWorker`

- Probe: `git -C ~/code/bench-repos/TypeScript grep -nw getResolvedSignatureWorker`
- Count: 5 raw lines / 1 file.
- Thin public wrappers: `TypeChecker#getResolvedSignature` and `TypeChecker#getResolvedSignatureForSignatureHelp`.
- Consumer boundary: `getCandidateSignaturesForStringLiteralCompletions` calls the worker twice and adds candidate-set aggregation; included as a real consumer, no further recursion.

Hop 3 — `getResolvedSignatureForSignatureHelp`

- Probe: `git -C ~/code/bench-repos/TypeScript grep -nw getResolvedSignatureForSignatureHelp`
- Count: 3 raw lines / 3 files.
- Real consumer: `signatureHelp.ts:getCandidateOrTypeInfo`, token `getResolvedSignatureForSignatureHelp@188`.
- Excluded: `types.ts` declaration; wrapper already included at `checker.ts:1780`.

## Counts

- Real gold sites: 19.
- D1 site count: 7. D1 files: `scripts/eslint/rules/argument-trivia.cjs`, `src/services/codefixes/fixAddMissingMember.ts`, `src/services/codefixes/fixAddVoidToPromise.ts`, `src/services/goToDefinition.ts`, `src/services/inlayHints.ts`, `src/services/symbolDisplay.ts`, `src/services/signatureHelp.ts`.
- Scorer D denominator is file-level: `d_gold_size=7`.
- D2 count: 0. Verified repo-wide `resolveSignature` word count is 3, below the >100 D2 threshold.
- Admission: PASS (`8 <= 19 <= 60`, D1 sites `7 >= 3`).

## Dry Run

Command run from `eval/`:

```text
uv run python -c "from tier_c.structural import score_structural,load_gold; g=load_gold('tier_c/gold/typescript-resolve-signature/gold.json'); perfect=[{'file':s['file'],'symbol':s['symbol']} for s in g['sites'] if s['adjudication']=='real']; r=score_structural(perfect,g); print('perfect',r.file_f1,r.d_recall,r.gold_size,r.d_gold_size,r.phantom)"
```

Output:

```text
perfect 1.0 1.0 19 7 0
```

## Exclusions

- Definition/comment/interface surface: `checker.ts:1572`, `checker.ts:37631`, `checker.ts:37648`, `checker.ts:36659-36664`, `types.ts:5240-5242`.
- Generated/test API baseline: `tests/baselines/reference/api/typescript.d.ts:6269`.
- Consumer boundary: `src/services/stringCompletions.ts:595` calls `getCandidateSignaturesForStringLiteralCompletions`, which was classified as a consumer rather than a thin forwarder.

## Uncertain / Review

- Whether to admit `src/services/stringCompletions.ts:getStringLiteralCompletionsFromSignature` as a public candidate-API consumer. I excluded it because recursion stops at `getCandidateSignaturesForStringLiteralCompletions`.
- Whether the public object-literal wrappers at `checker.ts:1778/1780` should be reported as property symbols or collapsed to `createTypeChecker`. I used explicit TypeChecker API property names for reviewability.
