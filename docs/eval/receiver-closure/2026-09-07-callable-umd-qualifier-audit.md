# UMD qualifier audit: singleton bridge, not a proven React merge

Source/compiler audit on merged main dc9b9e84. PR267,268,269 are confirmed merged
at33cc2e47,ec0b29db,dc9b9e84 respectively. No observer production code, schema,
runtime resolver, CPG, navigation or cache changes are included in this slice.

## Result

The watched qualifier's ambiguous_declaration reason is produced by the observer's
scope-insensitive candidate collection, not multiple compiler declarations at that
hop. The bounded observer's own Program and a direct pinned-compiler control both
report a singleton global alias → export= alias → module-local namespace chain.
The global alias and local namespace are distinct compiler symbols. The actual
global-export syntax population for React contains one declaration in this Program.
This is not a claim of whole-Program augmentation closure.

All three declaration anchors are in project/node_modules/@types/react/index.d.ts,
file SHA25651409be337d5cdf32915ace99a4c49bf62dbc124a49135120dfdff73236b0bad:

| Kind | UTF16 half-open range | Byte half-open range |
|---|---|---|
| NamespaceExportDeclaration |2149–2175|2149–2175|
| ExportAssignment |2133–2148|2133–2148|
| ModuleDeclaration |2177–170379|2177–170413|

Use anchor: LibraryMenuHeaderContent.tsx,1252–1257 UTF16/bytes;
file SHA256c02b2e4d82f6e1916f18a07d5c8abb0666a10bb7b50f559a3fe973379f3bb711.
The helper starts with the global-export declaration and adds the local namespace
under the same local-name key. A differently named local target removes that
accidental candidate collision, exposing the still-unsupported UMD gateway.

Correction: neither “missing React types” nor “this qualifier has real namespace
augmentation” is supported by the measured installed Program. Its contextual
signature is available. Also, allowUmdGlobalAccess is not a prerequisite for this
type-only annotation: its relevant compiler check governs value use. No app option
was changed to produce this result.

## Preserved outcome

Normal merged-main validation reproduces the prior15688997-byte public packet:
SHA2560c4acbb619db405af129588cd7fe992945de1c6d93ce03c5a423570752c52a04;
producer47e122b154ab46335cbeb5bd6c7da6def57138d143cf8f0068c607528ca1bff5.
It remains unproven with outside_lookup, unresolved_module and unsupported_lookup.
The four library sites at155,160,184,265 remain linked/callable_unproven. Census
remains30 annotated observations,53 nested calls,10 lexical links and0 observed
Props/class records. Both authority flags remain false. No runtime recall gain.

Temporary instrumentation inside worker.mjs emitted only raw declaration anchors
and counts to stderr. Its packet, excluding only producer identity, is bytewise
canonical-equal to the prior packet. The patch was removed and git diff confirms
unchanged worker/provenance/schema/index code. The direct compiler Program has
1445 source files and no diagnostics, consistent with the bounded replay; the
actual observer instrumentation, not count agreement alone, settles this hop.

Original public source0642e72cfa2d9a71198200e52f37399384610ee3 and acquired copy
retain all tracked bytes and executable bits. Before/after custody manifests are
equal and original source is clean. No install, source rewrite, private acquisition,
app build or runtime proof was attempted. LSP tools are unavailable in this session;
the exact pinned compiler and source supplied the semantic fallback. Prism's
structural navigation is not a substitute for these compiler alias identities.

## Fixtures and probe log

15 new top-level characterization tests cover the singleton source bridge plus
duplicates, merging/augmentation, shadows/imports, absent/invalid targets, UMD
type/value option behavior and stale-epoch replacement. First run13 passed,2 failed:
the audit harness assumed missing/invalid qualifiers returned no symbol object.
The separating probe showed transient symbols (flags34078720) with zero declarations,
diagnostic2503, and1315 for an invalid global export in a .ts file. The corrected
assertions require zero declaration evidence and the relevant diagnostics; they
do not accept the transient symbols as bindings. No production oracle was changed.

Full observer suite:124 passed,0 failed,0 skipped. These are characterization
controls on unchanged production behavior, not an implementation RED claim.
Full default/MCP Rust and authority controls are running; final totals follow
before publication completion. No Tier-A trigger because relevant Rust surfaces
are untouched. Review cap two SELF-PASS rounds, NOT INDEPENDENT.

Hypothesis/probe/result:

- Candidate inflation versus actual augmentation: direct compiler has raw counts
  1/1/1, while same-named syntax is2; observer instrumentation confirms those same
  declarations. Actual multi-declaration attribution at this hop is rejected.
- Instrumentation affects observations versus only diagnostic evidence: all packet
  fields except producer identity equal the predecessor. Normal main validates it.
- Missing binding versus a recovered real declaration: transient symbols have no
  declarations. Fix the harness assumption, preserve unproven behavior.

## Custody and next step

Task root /private/tmp/prism-qualifier-audit-5LdjYI. Raw evidence:
observer-symbols.json (SHA256bce6752fa074c1528c4f626b3f5317e3a121383bdea2539b26380bcdbe47b754),
instrumentation.patch (389c0c0e58b9126a05936a51c1d6062d26c8833a568975315571262b0d84046a),
direct-compiler.json (a7793ff7209b78c4c800c21ba89d7fc1c31bfa4dbacef49573fba5e1c8194641),
instrumented-packet.json, main-validation.json, source-before.json, source-after.json,
characterization.log, unresolved-symbol-probe.log, observer-full.log, cargo-default.log,
cargo-mcp.log, authority.log and authority-tests.log. Instrumentation is removed,
not a shipped observer option; the small source/probe files are retained with logs.

Next recommendation is a separately bounded, source-identity-based UMD declaration
bridge, following the [proof requirements](../../superpowers/specs/2026-09-07-callable-umd-qualifier-proof.md).
Preserve all duplicate/write/cache and Program-closure barriers; do not special-case
React/FC names or infer runtime class authority from these observations.
