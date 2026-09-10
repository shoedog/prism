# Native parser and representation classification

Successor to PR303 merge `17f7053df832cf0bb872aa1915899e14c01dfe91`.
Published as [PR304](https://github.com/shoedog/prism/pull/304); CI is not claimed green.
Documentation and characterization fixtures only: no production/parser/dependency,
suffix, input-selection, schema, closure or executable-owner change.

## Outcome and corrected assumptions

**WRONG — pre-existing pinned grammar misparses valid inline import-type generic
calls.** In TS and TSX, `get<typeof import("./module")>()` is recovered as a binary
expression involving a runtime-looking `typeof import(...)` and an ERROR at `>()`,
rather than the compiler's generic call with a type argument. Complete minimized
Programs are syntactically and semantically valid under the pinned compiler.
Both production source and grammar are byte-unchanged from merged main in this
same environment, so this is not a PR303 regression attribution. It remains unfixed.

The historical phrase “two public parser refusals” means two **loaded** files with
three ERROR nodes, not two `SkipReason::ParseFailed` files. The loader tolerates
error rates up to 30%; the executable-owner constructor reparses and requires zero
errors. The observation's native_status is incomplete on any error or designated
skip. These are three different decisions, none to be relaxed here.

**SMELL — unsupported representation capabilities, not demonstrated unsafe output.**
The repository JSON and .mts files are skipped as Unsupported by native extension
dispatch. The two dependency .d.cts paths also have no native language mapping,
but their ancestor node_modules directory is Ignored before per-file dispatch;
do not claim individual Unsupported skip records for these dependency files.
Forced TypeScript parsing succeeds for the exact .mts source, but does not prove
ESM resolution, declaration identity, input-set equality or executable ownership.
Current fail-closed behavior is retained, not relabeled as a parser defect.

## Exact parser population

Public Excalidraw pin `0642e72cfa2d9a71198200e52f37399384610ee3`, unchanged installed
copy `/private/tmp/prism-acquire-w2FtSq/source`. TypeScript 5.9.3 SHA256
`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`;
tree-sitter-typescript 0.23.2 / tree-sitter 0.25.10 from the unchanged Cargo.lock.

| File | ERROR location (1-based line:byte-column) | Bytes [start,end) | ERROR/total nodes |
|---|---|---|---:|
| packages/excalidraw/tests/clipboard.test.tsx |31:74|843–846|1/5,993|
| setupTests.ts |36:74;107:5|1284–1287;3004–3007|2/1,122|

All three node texts are `>()`; none is a MISSING node. Both full source files
have zero compiler parse diagnostics; **full-file semantic validity is not claimed**.
The common expression is `await importOriginal<typeof import("…")>()`.
Source anchors: [clipboard](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/excalidraw/tests/clipboard.test.tsx#L31),
[setup first](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/setupTests.ts#L36),
[setup second](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/setupTests.ts#L105).

The committed fixture matrix contains 9 cases × TS/TSX = 18 observations. Six valid
instances fail the desired zero-error contract (await, no-await, parenthesized);
10 valid controls and 2 malformed-refusal controls pass. Captured baseline RED:
**12 passed / 6 failed**, intentional unresolved defect evidence, not a green gate.
All 16 valid instances have zero compiler pre-emit diagnostics; malformed instances
have codes 2365, 2349, 1005 and native ERROR nodes. Alias/number/typeof-identifier/type
annotation/runtime-import controls distinguish this from await-only ambiguity,
TSX-only syntax, missing imports, or a parser incapable of all import types.
The current grammar includes both generic call and type-query productions; no
particular precedence edit or newer dependency version is proved to repair it.
For all 6 valid mismatches the compiler AST has a get call whose type argument is
an ImportType with isTypeOf=true; emitted JavaScript retains get and erases the
type-only import. The 2 actual dynamic-import controls retain an import call in
emitted JavaScript. No emitted code was executed.

## Separate representation requirements

The fixed population is seven configs under the same full audit root, not a reduced
package-directory census. The complete native census remains 628 files in every case.
There are 591 skipped entries: 294 Unsupported, 285 NotUtf8, 7 Ignored, 4 Hidden and
1 TooLarge (21,745,444-byte scripts/woff2/assets/Xiaolai-Regular.ttf); zero
ParseFailed skips. The 285 NotUtf8 plus 1 TooLarge entries independently keep
native_status incomplete even after a hypothetical perfect parser repair. Their
domain/representation policy is not adjudicated or changed here.

| Config | Roots | Program | Repository JSON | Repository .mts | Dependency .d.cts |
|---|---:|---:|---:|---:|---:|
| root |591|1445|3|1|2|
| common |19|805|2|0|0|
| element |50|806|2|0|0|
| excalidraw |340|831|2|0|0|
| fractional-indexing |2|304|0|0|0|
| math |16|805|2|0|0|
| utils |5|810|2|0|0|

The unique repository assets are `packages/excalidraw/locales/en.json`,
`packages/excalidraw/locales/percentages.json`, `packages/excalidraw/package.json`
and `excalidraw-app/vite.config.mts`. These are unsupported suffixes, not native
parse failures. The root config explicitly enables allowJs/resolveJsonModule;
the package configs inherit their JSON option. The locale JSONs are compiler JSON
SourceFiles with no function/call nodes, yet affect both `TranslationKeys` and
runtime language filtering ([i18n.ts](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/excalidraw/i18n.ts#L6)).
Package JSON is required by [env.cjs](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/excalidraw/env.cjs#L3)
and supplies runtime name/version values. Presence is not ownership or safe exclusion.

The .mts file is compiler ScriptKind.TS, impliedNodeFormat=ESNext, an external
module with 31 call expressions and 3 function-like nodes. It exports an executable
[defineConfig callback](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/excalidraw-app/vite.config.mts#L11).
It is not a non-code asset merely because it configures builds. Its forced native
TypeScript parse has 0 errors/1,866 nodes, while normal language detection is None.
No real source was executed. Declaration-only .d.cts files remain separate from
ordinary .cts implementation source; this population contains no repository .cts.

### Proof requirements before any admission expansion

1. **Grammar repair, first:** preserve original bytes/byte spans and native syntax
   tree fidelity. Promote all 6 valid RED instances to zero errors and prove generic
   call/type-argument structure, correct runtime callee and no fabricated dynamic
   import edge from type-only syntax. Keep actual dynamic import calls, malformed
   cases, TSX ambiguity, duplicate/write/cache barriers and epoch substitution tests.
   Reparse all 628 original native members; compare all errors, function/call facts
   and Tier-A accuracy controls, not just disappearance of `>()`. No text rewrite,
   error-count suppression, test deletion or broad parser upgrade as a shortcut.
2. **JSON representation, separate:** compiler-authenticated canonical bytes,
   effective option/config/Program identity and exact import/require request-target
   provenance. Represent data without inventing an executable owner. Negatives:
   invalid JSON, disabled option, missing/mutated target, same-name different data,
   genuine cross-config/epoch substitution and code-looking JSON strings. A require
   route does not inherit authority from an already-supported import route.
3. **.mts source identity, separate:** prove suffix dispatch plus compiler ESM
   module/resolution behavior; do not stop at assigning the TypeScript grammar.
   Negatives: .d.mts/.d.cts owner substitution, same-stem .ts/.mts/.mjs collisions,
   extensionless fallback, invalid/ambiguous syntax, .cts not implicitly admitted,
   write/duplicate/cache barriers and cross-epoch source substitution.

Items 2/3 are requirements, not newly implemented or executed admission tests.
None establishes that all remaining dependencies/effects/entry obligations are safe.
Keep all 7 incomplete semantic closures, react-scripts unresolved, and existing
React.FC policy. Private worker_failed remains inherited/unclassified; private repo
not read this slice. Parser repair has value by removing an independently proved
false refusal, but **no real eligible-owner/recall gain is promised**.

## Verification and custody

Fresh full gates passed from clean, unchanged checkpoint `af2dfdf1` (later changes
are readout/evidence/docs only): Rust default 4,037, MCP 4,230, MCP+detached-owner-audit
4,253; one existing ignored `resolution_test::slice_elem_variant_reserved` per run.
Observers 726, receiver helpers 18, authority controls 40 and native example 12 passed.
Python 940 passed / 1 deliberate guarded live-model skip. The local characterization
validator passed 29 assertions; these validate the measured defect population, not
its repair. The separate desired-behavior RED remains 12 passed / 6 failed.
Format/whitespace passed. Clippy was not rerun for docs/fixture-only changes.
Tier-A was not triggered; no accuracy rebaseline or inherited recall claim.

All seven fresh complete ordinary packets, Program facets and native censuses are
exactly equal to PR303 artifacts. Public tracked bytes/status/pin and non-following
filesystem metadata match before/after. No private repo was read. Fresh compiler
Programs reproduce all seven root/Program counts and the described representation
facets. All native/semantic closure barriers remain unchanged.

The [fixture matrix](native-parser-classification-fixtures.json) preserves complete
minimal Programs and current native/compiler expectations for the next repair.
The [machine receipt](2026-09-10-native-parser-representation-classification.json)
binds source/artifact hashes, exact counts and fresh gate provenance. These JSON
fixtures are characterization evidence, not newly wired production regression tests;
the next repair must promote the desired zero-error and structural assertions into
the regular suites. Raw diagnostic source/drivers and outputs remain in
`/private/tmp/prism-parser-classification-6vOV0e` with an archived custody copy;
they use the installed pinned compiler and local native build, not a portable installer.

Both independent review rounds approved with zero new WRONG/actionable SMELL
findings. Round 1 reproduced all 18 fixtures and real parser anchors; round 2
reconciled receipt/log hashes, all seven parity comparisons, custody, skip counts,
compiler AST facets and independently reran the 29 classification assertions.
The two-round cap converged without extension. The documented grammar defect
remains unfixed; review approval is for the classification, not parser correctness.
No production files/scripts/examples/dependencies changed from merged main.
The temporary standalone diagnostic uses the actual freshly built ParsedFile API;
it walks every ERROR/MISSING node and asserts its count equals parse_error_count.
An initial incompatible-serde standalone link failed before execution; that setup
failure is inadmissible parser evidence and was corrected before probing.

Prism navigation oriented load_repo -> collect/parse/merge, but returned a stale
index warning. Current source supplied authority: src/repo_loader.rs:15,65,105,148;
src/ast.rs:15,396; src/languages/mod.rs:24,50; src/executable_owner.rs:123,143,226;
scripts/callable-observations/membership.mjs:73. Pinned compiler source distinguishes
script/declaration suffixes (typescript.js:22518) and module formats (:126804).
This was a Prism stale-index fallback, not an LSP failure.

Next recommended checkpoint: bounded source-preserving repair of the inline
import-type generic-call grammar defect, with a candidate-vs-base structural and
accuracy comparison. Keep JSON/.mts representation and closure admission separate.
