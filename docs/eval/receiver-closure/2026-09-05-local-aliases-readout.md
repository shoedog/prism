# Module-local contextual type aliases

Base db34e80, PR248 merged. Owner approved the recommended local declaration-backed
contextual alias slice. Implementation166d2d8 pushed in
[PR249](https://github.com/shoedog/prism/pull/249), merged into main at 2aff090
(GitHub merge state and fetched origin/main verified for the props-alias continuation).

## Result and boundary

TS/TSX direct variable-initializer arrows/function expressions can recover an
unannotated destructured receiver through one module-local type alias:

```ts
type F = (props: {client: Client}) => void;
const run: F = ({client}) => client.m();
```

Accept a single plain, non-generic type alias with a direct function-type RHS,
including direct exported declarations and forward references. Alias visibility
is checked at use; the original RHS syntax node keeps receiver class identity
anchored to the alias declaration. Receiver writes remain anchored to the actual
implementation parameter. For example, an unrelated `Client` generic on the
implementation's enclosing function cannot change the module alias's Client type.

Reject duplicate/competing type declarations and imports, nearer type shadowing,
ambient aliases, generic/qualified/imported/nested aliases, chains/cycles, wrapped
types, overload objects, unions/intersections and parse recovery. Existing required
parameter/property, explicit-annotation and receiver-write barriers are retained.
Value-only declarations and writes of the alias spelling do not affect its type
identity. CPG69/nav38 invalidate earlier evidence; no serialized field or new rung.

No property-type alias expansion, generic substitution, React.FC, dependency/ambient
authority, hook-return inference or JavaScript type-alias semantics. This is a
small source-backed extension, not general TypeScript compiler equivalence.

## Verification and review

Same-environment complete focused RED on untouched db34e80:2pass/1fail. All sixteen
initial TS/TSX positive cases lack Exact owner identity; exclusions pass. Direct
contextual signature controls already establish the imported-owner path, separating
the missing alias selector from a broken class lookup.

Initial green3/0. Round2 expanded collision/signature/scope controls: all nine
inline/contextual/alias test groups pass. Twenty positive variants now cover export,
forward reference, comments, renaming, async/named bodies, nested captures, separate
type/value namespaces and declaration-scope ownership. Negative controls cover each
new gate and retain the existing parameter/property/write exclusions.

Persisted alias-positive↔optional/type-collision transitions, alias-RHS-only A→B
and B→A owner replacement, full/direct-subset/incremental parity, nav-sidecar
positive/negative round trips, and old CPG68/nav37 cache refusal are tested.

Three-round SELF-PASS cap: round1 RED and implementation; round2 expanded boundary
and persistence checks; round3 source/consumer/persistence trace. No further
in-scope WRONG identified, no extension. Indexed navigation lacked the new helper;
that SymbolNotFound was not absence evidence. Current-source checks supplied the
shared AST→classification→call-site→resolution path. No independent review claim.

Full default3824/0/1 and MCP4014/0/1; clippy completed with warnings retained.
Existing ignored test: `resolution_test::slice_elem_variant_reserved`.
Fmt/diff checks pass. Immediately rebuilt matrix104/104. Immediately rebuilt quick
exit2: baseline-invalid solely corpus SHA drift db34e80b01e7 vs pinned20c8490591a3.
Oracle quiescent; oracle/SUT errors0; stale adjudications4. No green quick gate,
full multicorpus or baseline rewrite.

Pins: target-c-method flip_candidate, supplementary Exact5/0/0, 28 extra default
candidates; module-deps-feature-gated missing literal pin, oracle-only empty and
one MCP Prism-only; load-repo-feature-gated missing literal pin, oracle-only
resolution_test.rs:5299/5368 and four MCP Prism-only; ambiguous-symbol ok.
Sampled U-free callee Exact1/1/0 and recall gaps remain reported. No same-environment
base quick rerun for attribution: these are not claimed as regressions or fixes
from this slice. Full site lists and sampled counts are in the archived reports.

## Measurement

Saved base release versus candidate on the same five-file Excalidraw archive
SHA0642e72cfa2d9a71198200e52f37399384610ee3:2780 stable site identities,
Exact376 on both sides, zero changed records. The remaining six tracked Library
spans still need React.FC/useContext authority; no real-site recall gain claimed.
Earlier small Black/Excalidraw/JavaScript controls are byte-identical.

Separate served fixture: base exact-only callers/callees empty; candidate serves
app.ts:3:visible→client.ts:2:m as TypedParam Exact. Optional, overwritten and
generic-shadowed receivers are excluded. No fallback, warnings or truncation.
`verify-local-aliases.mjs` asserts the real controls, served gain and exclusions;
its compact result and source hashes are committed beside this readout.

## Custody and reproduction

Evidence root `/private/tmp/prism-local-aliases-mu762H` contains saved base-prism
and candidate-prism, paired JSONL, served fixture sources/JSON and all test/gate
logs. Fixed source root `/private/tmp/prism-indirect-default-VUbv13/excalidraw`.
Run measure-indirect-default.mjs on the paired excalidraw JSONL and fixed root,
then verify-local-aliases.mjs on the evidence directory.
Raw archive (including fixed source):
`/private/tmp/prism-local-aliases-mu762H-evidence.tgz`, SHA256
`0e9433044e07c86ef91f3137cb7754538d4598206f47ae8167466c79583baf2e`.
Only this run's untracked quick reports and prism-db34e80b01e7.json snapshot were
moved into the evidence directory; all remain recoverable. Original .superpowers/
and eval/snapshots/prism-fb81481dafa7.json untouched. PR248 merge-record corrections
from the owner notification are included in this slice.
