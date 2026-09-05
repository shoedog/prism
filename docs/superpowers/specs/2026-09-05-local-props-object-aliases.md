# Module-local props object aliases

Base 2aff090 (PR249 merged). Owner: “merged proceed to mext”. Next bounded
source-backed slice: one local props object alias, not ambient React or hooks.

## Contract

TS/TSX destructured parameters may resolve `Props` in
`type Props = {client: Client}; function run({client}: Props) { client.m(); }`.
The same object alias is allowed in a direct contextual signature and in a proven
module-local function alias: `type F = (p: Props) => void; const run: F = ...`.
This composes one function-shape lookup with one object-shape lookup, not recursive
alias expansion. Each lookup must terminate in its requested direct syntax shape.

Reuse the existing single, non-generic, module-local alias authority gate:
duplicate/competing type declarations or imports, nearer type/generic shadowing,
ambient declarations and parse recovery reject. Direct exported aliases and
forward references are allowed. Alias chains/cycles, nested/imported/qualified/
generic aliases, interfaces, mapped/conditional/union/intersection/wrapped shapes
stay excluded. Receiver member types must still be direct supported class names;
this does not expand `type C = Client` inside `{client: C}`.

Keep required-property/parameter and destructuring duplicate/write barriers.
An explicit implementation annotation remains terminal; failure cannot fall back
to its contextual annotation. Resolve Props at its reference node and Client at
the original Props declaration, retaining implementation-parameter write timing.
Same-spelled value bindings do not mutate a type alias. Ordinary JavaScript and
non-parameter destructuring do not acquire type-alias inference.

## Architecture and plan

1. Generalize the private contextual signature selector to a shape-parameterized
   one-hop local alias selector. Reuse it for function_type and object_type only.
   No new resolution rung or serialized metadata.
2. Complete RED on untouched merged2aff090, save base release binary. Positive
   explicit/contextual/composed cases and negative/edge cases, full/subset parity.
3. Three-round self-review cap. CPG69→70/nav38→39; good↔bad cache transitions,
   Props-RHS-only A↔B owner replacement, persisted sidecar negatives/old-version miss.
4. Full default/MCP, fmt/clippy, immediate rebuild matrix and quick. Pair fixed
   Excalidraw sample and prior controls; prove served synthetic paths separately.
   No promised gain for the remaining six React.FC/useContext-dependent real spans.
5. Archive evidence, commit/push/open PR, then docs-only publication custody update.
   No merge, rebaseline or full multicorpus.

## Hypothesis/probe/result log

Pre-change object recovery requires literal object_type; expect declared Props
positives to lack Exact ownership while existing direct-object controls work.
Alternative imported-owner lookup failure is separated by those direct controls.
The indexed navigation seed is unavailable; SymbolNotFound is not absence proof.
Current source shows shared parameter recovery supplies both explicit/contextual
annotations to the same property checker. Outcomes are recorded in the readout.

Evidence: `/private/tmp/prism-props-aliases-MhRneP`.
