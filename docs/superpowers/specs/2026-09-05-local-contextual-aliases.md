# Local declaration-backed contextual aliases

Base db34e80 (PR248 merged). Owner approved this slice after the constructor-field
increment. Standing commit/push/PR authority; no merge, rebaseline or multicorpus.

## Bounded contract

TS/TSX direct variable-initializer arrow/function expressions may obtain their
existing destructured contextual parameter annotation from one same-file,
module-level type alias: `type F = (p: {client: Client}) => void; const run: F =
({client}) => client.m();`. A direct exported alias is equally local. Forward
references are allowed because type aliases do not require runtime initialization.

Require exactly one visible plain alias, no generic parameters, and a direct
function-type RHS. Reject duplicate/competing module type declarations or imports,
nearer type declarations/generic binders, ambient aliases, parse recovery, chains,
cycles, qualified/imported/generic aliases, overload objects, unions/intersections,
and wrapped signatures. Property-type aliases remain outside this slice.
Value-only declarations/writes of the alias spelling do not mutate type authority.

Keep all existing signature/parameter/property and receiver mutation checks.
Explicit implementation annotations remain terminal and cannot retry through the
context. The alias reference is checked for visibility at its use; receiver class
names are resolved at their original alias-declaration annotation, not at the
implementation scope. Runtime type-only import erasure is unchanged.

## Architecture and plan

1. Extend `js_ts_contextual_parameter_annotation` with a private one-hop signature
   selector; direct signatures follow the unchanged existing route. Reuse the
   original syntax node so receiver-owner type and write anchors stay distinct.
2. RED full/direct-subset owner-decoy matrix on untouched merged base; preserve a
   base release binary. Three-round self-review cap, enumerate failing populations.
3. CPG68→69/nav37→38 with positive↔negative transitions and cached A↔B owner
   replacement when only the alias declaration changes. Sidecar served controls.
4. Full default/MCP suites, fmt/clippy, immediate release-build matrix and quick.
   Paired archived source controls plus a separate served synthetic alias fixture.
   No promised gain at the remaining six React.FC/useContext-dependent real spans.
5. Readout, evidence archive, commit/push/PR, publication custody follow-up.

## Hypothesis/probe/result log

The pre-change helper accepted only function_type directly in a variable annotation.
Expected local-alias positives to lack Exact identity on db34e80, while direct
signature controls and alias exclusions remain valid. Alternative: imported class
lookup is broken; direct-signature/owner-decoy controls discriminate it.
Navigation SymbolNotFound for the current helper reflects an unavailable indexed
seed, not proof of absent callers. Current source supplies the implementation path.

Observed complete RED2/1 with sixteen failed owner assertions; initial green3/0.
Expanded round2 contextual/inline/alias matrix9/0, cache transition and owner
replacement groups2/0 each. Round3 source/consumer review found no further in-scope
defect; three-round cap completed without extension. Full default3824/0/1,
MCP4014/0/1, clippy completed with warnings, fmt and matrix104/104 pass. Served
fixture gains Exact ownership with all negatives excluded; real2780/Exact376
unchanged. Quick validity and final custody are recorded in the lane readout.

Evidence root: `/private/tmp/prism-local-aliases-mu762H`.
