# Imported/exported type identity and augmentation audit

Base: PR255 merge e298ae7c58ca25fdf0acfd8587dacbfdbb7aa4ba.
Branch: audit/imported-props-identity. Audit/fixtures only; no production resolution
or cache changes. CPG75/nav43 remain unchanged. Audit committed df485b0 and pushed;
PR https://github.com/shoedog/prism/pull/256 merged e08af858 on2026-09-06.
The following audit results are historical. Its approved successor is recorded in
[the imported object-alias readout](2026-09-06-imported-object-alias-readout.md),
including the single promoted guard, pinned5.9.3 rerun and resolved stale Prism lookup.

## Outcome and corrected assumptions

Imported Props support alone would close **none of the six remaining source spans**
in the fixed measurement. Four depend on unsupported React.FC contextual authority;
two depend on useApp/useContext return flow. This is a source prerequisite finding,
not an implementation experiment or a claim about every site in the full repository.
The hook chain reaches an exported **type alias**, AppClassProperties, not a Props
interface. Its body also includes focusContainer(), which the current own-property-only
shape proof rejects. Resolving the import would not by itself admit that shape or
authorize the hook producer. No React.FC expansion was performed or recommended here.

Three distinctions matter:

- Exported interfaces and exported aliases are not one barrier: current local
  exported object aliases already work; imported aliases do not. The corpus preserves
  this positive control rather than describing all exported types as unsupported.
- A conflicting augmentation of a non-method property is rejected by TypeScript;
  it is an ill-typed negative, not a valid program demonstrating owner replacement.
- Default-import spelling does not prove a symbol is unaugmentable: the same class
  can have a named export. Our paired default-only/default-plus-named fixtures
  discriminate this case. Runtime writes remain a separate proof obligation.

## Source custody and receiver trace

Upstream is Excalidraw0642e72cfa2d9a71198200e52f37399384610ee3. The source verifier
checks the complete605 tracked TS/TSX/MTS/CTS file set and each Git blob against that
commit, plus package.json. All five measured files match those blobs. The committed
source-audit JSON includes all five SHA256 values, path/content census hash, exact
byte spans, duplicate caller records and source anchors. Zero parse errors.

Fresh Prism call-stats:2780 records,376 Exact,11 relevant unique spans. Five relevant
spans are already Exact; six remain NameOnly. Nested caller records are deduplicated
by file/start-byte/end-byte, not counted as extra recall opportunities.

| Remaining spans in LibraryMenuHeaderContent.tsx | Receiver proof chain | Missing authority |
|---|---|---|
|155,160,184,265|library binding at47–56 ← inline property41 ← import type Library at30 ← library.ts class197/default export403|React.FC contextual producer at38; not imported Props|
|297,307|destructure at286 ← imported useApp at14 ← App.tsx:568 useContext(AppContext) ← context503 with imported AppClassProperties ← types.ts:801/810 ← class type import56|hook return/context flow, imported alias identity and unsupported method-bearing shape|

Pinned sources: [receiver declarations](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/excalidraw/components/LibraryMenuHeaderContent.tsx#L30-L56),
[hook use](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/excalidraw/components/LibraryMenuHeaderContent.tsx#L277-L307),
[context and hook](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/excalidraw/components/App.tsx#L503-L568),
[exported alias](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/excalidraw/types.ts#L799-L819),
[class export](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/excalidraw/data/library.ts#L197-L403).

## Augmentation census — source scope, not program closure

The605-file AST census finds five global augmentations, one external-module
augmentation targeting csstype, six ambient module declarations and eleven top-level
script interfaces. Global blocks contribute Window/browser-related declarations;
none of the observed blocks directly targets the traced Library or AppClassProperties
declaration. Ambient module declarations are not automatically augmentations:
external-module status and declaration scope distinguish them.
The census uses syntactic import/export markers via isExternalModule; it does not
apply tsconfig moduleDetection to classify files.

This does **not** certify a closed TypeScript program. node_modules, typeRoots,
compiler-selected libs, project references, generated files, module resolution
conditions and tsconfig module-detection behavior were not typechecked. The pinned
package declares TypeScript5.9.3 and @types/react19.0.10; installed compiler6.0.3
was used for syntax census and isolated fixtures. No dependency install or whole-app
check was run. No LSP tools were available. The Prism indexed helper lookup returned
SymbolNotFound; current source inspection replaced it, not an absent-caller claim.

## Language evidence and compiler probes

TypeScript distinguishes type and value declarations; type-only imports constrain
emission, but may refer to values without creating a type alias. Export/namespace
syntax must be resolved in the requested namespace. See the official
[module reference](https://www.typescriptlang.org/docs/handbook/modules/reference.html#type-only-imports-and-exports).

Interface non-method duplicates must agree; module augmentations merge into the
resolved named declaration. Default exports cannot be directly augmented by the
default export name; named visibility of the same symbol is a separate question.
See the official [declaration-merging reference](https://www.typescriptlang.org/docs/handbook/declaration-merging.html#module-augmentation).

verify-imported-props-types.mjs compiles the shared24-case corpus in memory with
strict=true,noEmit=true,types=[],ES2022/ESNext/bundler resolution. It asserts every
diagnostic-code set and, where specified, the declaration file of client.m. Thus
the declaration-scope decoy fixtures prove client.ts ownership rather than merely
compiling. No emitted files or library installation is needed.

Eighteen fixtures are compiler-valid; six deliberately exercise errors:2717
incompatible property,2749 value used as type,2308 ambiguous star export,2303 cycle,
2307 missing module,2339 default-only augmentation witness. These are not six
runtime defects. The valid default-plus-named witness succeeds with the added tag;
the default-only variant rejects that tag. Both preserve the existing m owner.

## Proof requirements before future admission

These are requirements for a future separately approved slice, not new runtime
structures implemented by this audit. Missing or ambiguous evidence must leave the
current route unproven; do not erase existing independent evidence from other routes.

| ID | Required evidence | Negative or control fixture / future gate |
|---|---|---|
|P1 type identity|Use-site binding in the type namespace; declaration kind and original declaration span; local shadow/duplicate barriers|value_only_type_import,private_vs_global_control,exported_local_alias_control|
|P2 module/export identity|Canonical module under supported resolver options; imported/exported/local names; every hop; cycle/ambiguity/missing classification|type_reexport_rename,ambiguous_star_reexports,cyclic_reexport,missing_module|
|P3 declaration scope|Resolve each property type at its defining declaration, not in the consumer. A bare returned class-name string is insufficient across files|named_import_declaration_scope,imported_alias_declaration_scope; both contain consumer decoy DeclaredClient|
|P4 shape authority|Explicitly supported declaration kind and own required properties; retain generic/heritage/optional/member barriers; distinguish imported alias/interface/default/namespace/query forms|default_interface,namespace_type_import,import_type_query,exported_interface_same_file; existing PR255 shape tests|
|P5 augmentation closure|Program/file universe and relevant augmentations resolved to the same declaration; consider included d.ts and globals. A subset or source-only census cannot prove absence|module_augmentation,ambient_dts_augmentation,global_augmentation,ambient_external_module|
|P6 class ownership|Class declaration identity, not interface member shape; named/default visibility and existing member/write barriers remain separate|ambient_external_module,default_only_augmentation,default_also_named_augmentation,private_receiver_write|
|P7 producer authority|Independently prove contextual signature or return flow; explicit annotation remains terminal. Identity does not license framework names or hooks|react_fc_homonym,hook_return_homonym; six real spans|
|P8 persisted evidence|Key proof on all declaration/export/augmentation/config inputs, including negative lookups and file membership; replace old owner metadata rather than accumulating it|future add/delete/rename/change augmentation and barrel A↔B transitions; existing PR255 owner replacement tests are controls, not augmentation-cache proof|

P8 acceptance must include cached full-build parity in both transition directions,
new/deleted d.ts files, paths/exports/typeRoots/program changes, missing dependencies,
scope-limited builds and stale navigation sidecars. No success can be inferred from
unchanged cache rows when the imported route is currently always rejected. Any
runtime expansion gets its own coordinated CPG/navigation version decision.

The current seam is ParsedFile::js_ts_local_props_shape → local_type_definition
and its privacy guard → inline_prop_receiver_type, while
CallGraph::js_ts_recovered_class_owner resolves classes relative to the caller file.
Do not loosen the first helper and feed a foreign declaration's spelling unchanged
to the latter. A future proof must carry the foreign declaration/module identity
across that seam. No such proof representation is implemented here.

## Review disposition and next boundary

WRONG: none demonstrated in the unchanged in-scope resolver. Unsupported imported
forms are intentional rejection, not newly discovered precision defects.
SMELL / prerequisite gap: current local proof cannot certify cross-file declaration
scope or augmentation closure; the fixtures prevent accidental admission while those
requirements remain open. No inherited WRONG was downgraded.

Before proposing runtime expansion, select one concrete consumer and specify its
producer proof separately from type identity. Imported non-generic object aliases
through one explicit relative named import could be a smaller identity-only design,
but would not close these six spans and is **not approved or implemented here**.
Framework-return provenance would be a different slice. React.FC remains excluded.

## Verification and custody

Round1 source trace plus24 compiler fixtures and96 TS/TSX full/subset comparisons
pass (five Exact controls,nineteen no-Exact guards,each across four build/language
combinations). They intentionally pass on unchanged main; no RED-first production
claim. Round2 strengthened census custody beyond the five measured files: the tool
rejects changed augmentation bytes and added declaration files elsewhere in the tree.
Three source-custody tests pass, including the positive pinned-tree control.

Fresh release/matrix and real-site replay are captured in
/private/tmp/prism-imported-props-audit-LjHVKt. Initial uv launcher refusal opening
its protected cache is inadmissible as product evidence; host rerun follows an
immediate rebuild. Final gates: cargo test **4005 passed,0 failed,1 ignored**;
cargo test --features mcp **4195 passed,0 failed,1 ignored** (both include two
doctests). The existing ignored SliceElem control remains ignored. Matrix **159/159**;
compiler **24/24**, Rust characterization **96/96**, source custody **3/3**.
Formatting/diff checks pass; Clippy completed with warnings, not warning-clean.
Round3 consistency review completed within the three-round cap: SELF-PASS, NOT
INDEPENDENT; no open in-scope WRONG. No fresh Tier-A quick or full multicorpus run:
this audit changes no production resolution, navigation or CPG code. The prior
PR255 baseline-invalid quick is historical, not a fresh green gate here.
No production code or cache changes, no rebaseline, no cleanup. Original unrelated
artifacts remain untouched; three PR255 merge-record docs are carried into this PR.

## Reproduction

Run the shared compiler corpus with:

```sh
node docs/eval/receiver-closure/verify-imported-props-types.mjs /absolute/path/to/typescript.js
cargo test --test integration imported_props_identity_audit
```

The source verifier takes five positional arguments: TypeScript module path,
archive of the pinned upstream tree, measured five-file slice, call-stats JSONL,
and upstream Git repository containing the pinned commit. The repository is read
only; all tracked source blobs and the path universe are checked against its tree.
See audit-imported-props-source.test.mjs for the five PRISM_AUDIT_* environment
variables required by its three standalone node --test custody tests. Paths for this
run are recorded in the handoff; the companion source-audit JSON matches fresh replay.

Evidence archive: /private/tmp/prism-imported-props-audit-LjHVKt-evidence.tgz;
SHA256 1e5e5a441ef4b444d20de2b7b7c3c230f76ffac5c36d8456b89c94335e292c96.
Contains the pinned upstream tree, fresh binary/output, gate logs and the13-file
audit checkpoint. Checkpoint publication fields are historical; this readout and
handoff carry the subsequent publication state. No raw quick artifacts were generated.
