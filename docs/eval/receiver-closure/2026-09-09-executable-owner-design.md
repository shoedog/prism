# Executable-owner design: source-backed readout

Scope: one direct imported contextual TS/TSX receiver plus refusal requirements,
before production wiring. Base `7040ceb63c77142793ba08fed5946e6e54f29a86` is merged
PR293 on freshly fetched main. The previous stack branch is retained unchanged.
The [constructor design](../../superpowers/specs/2026-09-09-callable-executable-owner-proof.md)
and [fixture corpus](executable-owner-fixtures.json) introduce no runtime authority.

## What the fixture establishes

The candidate uses a project-local imported `Handler<P> = (props: P) => void`, local
`Props = { client: Client }`, a direct imported source class and `client.m()` in the
contextual arrow body. Both `.ts` and `.tsx` are characterized. TSX here has no JSX.

- The pinned TypeScript 5.9.3 observer resolves the signature to `src/contract.ts`
  and the receiver method to the executable `src/client.ts` method at bytes 24–49.
  The call is bytes 164–174 in the original app source. Semantic closure is complete.
- The fresh debug Prism binary, built on unchanged base production source, returns
  no Exact method edge through `nav --no-cache callees`. The inline-signature control
  returns `Client.m`; the explicitly annotated Decoy control returns `Decoy.m`.
- The second genuine input changes only `return 1` to `return 2` in the class body.
  Both inputs independently reproduce; call bytes and anchors match, while method
  source and snapshot hashes differ. This is an attack setup for future epoch tests,
  **not** a passing cross-epoch proof-rejection test.

The corpus has 27 scenarios × two extensions. The Rust characterization covers
both full and direct-subset graphs: 108 cells in one integration test. Existing
refusals are compatibility controls, not implemented new-route predicates. Current
Prism lacks the imported contextual route for every candidate; zero edges by itself
does not discriminate among the proposed owner refusal reasons.

## Corrections and proof boundaries

| Source-backed fact | Design consequence |
|---|---|
| Direct-body call observations do not contain `props_class`; [props-class.mjs](../../../scripts/callable-observations/props-class.mjs#L7) processes only `observation.nested.calls` | Derive direct instantiated Props/property/class identity in a new trusted pass; do not relabel an old packet bit |
| [ast.rs](../../../src/ast.rs#L3466) can substitute a bounded **local** contextual alias; its local-type lookup excludes imported alias bindings | The candidate demonstrates a distinct absent imported callable route, not that all contextual parameters fail |
| [js_ts_props.rs](../../../src/js_ts_props.rs#L106) scans indexed source for ambient/prototype/reflective effects | Complete compiler closure is insufficient when Prism indexes another source outside the configured Program |
| [resolution.rs](../../../src/resolution.rs#L2157) requires clean class spans and unique admissible instance methods | A compiler MethodDeclaration or class name alone cannot authorize an executable target |
| [ast.rs](../../../src/ast.rs#L4199) rejects ambiguous member slots and writes to `this` | Keep duplicate, computed-slot, accessor and class-body write barriers, not just receiver-local checks |
| Standard libraries are ambient but not project executable owners | Preserve domain separation; a blanket ban on ambient declarations throughout the compiler Program would reject the positive fixture |
| Proof-derived edges can outlive an ephemeral proof in persisted graph state | First wiring must bypass both positive cache reads and writes, or bind and validate the full epoch before reuse |

`out_of_program_effect` deliberately adds `outside/effect.ts` with a prototype
write while tsconfig includes only `src`. Compiler closure remains complete; the
extra file is present in the Prism indexed input. The initial new constructor must
refuse this index/Program mismatch. This is a stronger admission precondition for
the new route, not a change to compiler closure or to source indexing.

The existing required-path repair is retained: a missing triple-slash path with
`skipLibCheck: true` yields empty compiler diagnostics but incomplete semantic
closure. Unresolved `react-scripts`, unresolved module, uncertain automatic entries
and `noResolve` also remain incomplete. No packages were installed or removed.

## Hypothesis / probe / result log

1. **Hypothesis:** a valid imported contextual receiver lacks Prism owner authority.
   **Alternative:** malformed TypeScript or a missing class/method index produces
   the same empty result. **Probe:** unchanged compiler/observer plus inline-signature
   and explicit-parameter controls using the fresh public CLI. **Result:** compiler
   complete, candidate absent, controls resolve intended owners. This establishes a
   constructible static-edge opportunity, not a production regression.
2. **Hypothesis:** refusal scenarios preserve their original closure outcomes.
   **Probe:** materialize all cases, independently produce/validate each packet and
   enumerate mismatches. Initial harness failures were inadmissible: it read a
   nonexistent navigation `target` field, incorrectly demanded `status=observed`
   for genuine incomplete observations, and omitted caller identity in the nested
   Rust census. Actual CLI/schema/source output distinguished those setup errors
   from changed behavior. Corrected selectors/guards retain the original fixtures
   and expected outcomes; no production patch or rebaseline.
3. **Review round 1/2, self-review:** WRONG: the first future-RED mode required both
   zero and one edge, making a future implementation unable to pass. Bounded fix:
   select the future expectation instead of the baseline for both genuine inputs.
   Only the final verifier receipt is admissible RED evidence; earlier runs remain
   in local custody. **Round 2/2:** SELF-PASS for the corrected design-only scope,
   no open WRONG, confidence 95/100; not an independent review. The final run passed
   all 54 baseline cells and failed exactly the four intended future-positive cells.

## Reproduction and verification scope

Use the existing pinned compiler; do not install a replacement or invoke project scripts.
From the repository root, build the current debug binary and pass an existing writable
evidence parent **outside the indexed fixture**:

```sh
cargo build --offline
node docs/eval/receiver-closure/verify-executable-owner-design.mjs \
  /path/to/pinned/typescript.js target/debug/prism /path/to/evidence-parent
cargo test --offline --test integration executable_owner_design
```

Adding `--expect-future-positive` selects the intended future Exact edge for both
genuine source variants in both extensions. On the unchanged production base this
must fail exactly those four cells with `0 !== 1`, while the other 50 pass. The normal
mode characterizes today's behavior and must pass all 54. After a future observer
schema/producer change, rebind this historical verifier rather than claiming it is
already the final production acceptance suite. Reports retain all rows, raw packets,
input/binary/compiler/producer hashes, warnings, diagnostics and closure reasons.

Pinned compiler SHA: `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.
Unchanged observer producer SHA: `32ae5bc44000af31bb7cd5994a14de5700bafb02a1089f3ac16a79687e40f88d`.
Raw evidence root: `/private/tmp/prism-owner-proof-xutDxX`.
The [fixture receipt](2026-09-09-executable-owner-design-receipt.json) records all54
rows and the four captured intended RED failures. The [full gate receipt](2026-09-09-executable-owner-design-gates.json)
binds clean `e1355b40`: observer591, Rust default4018, Rust MCP4208, receiver helpers18
and authority profiles40 passed; fmt/diff passed. Both Rust runs include doctests and
one existing ignored `resolution_test::slice_elem_variant_reserved`. No failures or
other required-suite exclusions. Subsequent closeout edits are documentation/receipts
only and do not claim another full-suite run. No production `src/`, observer or
workflow change; CPG77/nav45. Published as [PR295](https://github.com/shoedog/prism/pull/295).
Tier-A is not triggered. Constructor field substitution, cache/lifecycle, served
callers and CPG integration remain unimplemented acceptance work, not skipped passes.

Both navigation skills guided seam selection. The Prism caller query was stale and
truncated (20/242); LSP tools were separately unexposed. Those results are orientation
only. Exact locations were checked directly in source; compiler and fresh CLI provide
the executable fixture evidence. No complete type-resolved blast-radius claim.

## Recommendation

Update after PR295 merged: the owner approved this recommendation and the
[detached constructor](2026-09-09-detached-owner-constructor.md) now implements
that checkpoint. The following recommendation is retained as historical context;
production integration/lifecycle remains the next separate boundary.

Proceed next, if approved, to the detached constructor and new direct compiler facts,
without production wiring. Require per-predicate negatives and genuine-epoch custody
tests before accepting it. Then decide the public integration/lifecycle checkpoint.
Measured real receiver gain remains zero; earlier real-Program incompleteness and the
owner's unresolved `react-scripts` disposition are unchanged, not freshly benchmarked.
