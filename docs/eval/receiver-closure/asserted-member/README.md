# Asserted-member proof harness (not production normalization)

This observes the merged runtime on synthetic source. A compiler-backed proposed
syntax candidate is not a binding, class, receiver or executable-owner proof.
No runtime sources, generic AccessPath parser, closure policy or cache format change.

From the repository root, using the locally retained SHA-pinned TypeScript5.9.3:

```sh
cargo test --offline --example asserted_member_observations
cargo build --offline --example asserted_member_observations
node --test docs/eval/receiver-closure/asserted-member/policy.test.mjs
node docs/eval/receiver-closure/asserted-member/verify.mjs \
  /private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js \
  /Users/wesleyjinks/code/slicing/target/debug/examples/asserted_member_observations
```

Paths are this lane's retained inputs, not a request to install dependencies. The
compiler argument may point elsewhere only with the exact recorded compiler bytes.
The native argument must be the helper freshly built from the reviewed checkout;
the receipt records its SHA, not an independently authenticated build attestation.
The script uses a virtual module and an ambient prelude, captures full diagnostics
and emission, and never executes emitted JavaScript. Native observations retain
the exact synthetic source, raw return slots, calls, rvalue siblings and spans,
and full/subset DFG defs/uses/edges/confidence labels. Fixture inputs and executable
bytes are checked for stability during the run.

Add `--require-repaired` to the last command to assert the desired future field
dependency. **It currently exits1 with12 UNFIXED missing-edge assertions.** Default
observation mode can pass while those defects remain; do not call that a repair.
The checker refuses missing obligation classes to prevent a vacuous repair pass.

## Fixed population and interpretation

27 fixture definitions × TS/TSX =54 observations:49 compiler-valid and5 expected
invalid (unresolved/malformed type in both dialects, angle assertion in TSX).
24 syntactic candidates include controls and identity decoys, not24 repair targets.
Six asserted shapes ×2 dialects have the demonstrated missing field edge: as-any,
as-import, satisfies-import, nested wrappers, Unicode/comments and exact8-wrapper
limit. The plain/parenthesized controls retain the edge. Different receiver/field
decoys do not gain flow from runtime.X. Full/subset records compare exactly.

The proposed allowlist is one nonoptional literal ASCII dot-member, with a literal
ASCII identifier receiver after at most8 parentheses/as/satisfies wrappers. The
9-wrapper case is refused. Calls, conditional/comma/assignment receivers, computed
members (including literal keys), optional chains, non-null/angle wrappers, member
chains and escaped tokens remain refused even when emit is a simple member read.
Invalid fixtures are a separate compiler-validity refusal; parser-clean syntax
does not establish valid types. This allowlist applies only to these selected
return-value expressions in synthetic fixtures, not arbitrary erased AST contexts.

## Requirements before the production repair

The native experiment isolates `runtime.X` Def@line2 → Use@line3, not merely a
path spelling. The first no-write experiment had no Def in either control or
asserted source and could not prove a lost edge. Raw rvalue names/paths do not
collect return-only lines; the span sibling does. Do not normalize that existing
API difference away while repairing member identity.

| Consumer | Required future parity |
|---|---|
| AST path/span collectors and base fallback | Same bounded syntax descriptor; original whole-member envelope plus exact identifier-token span |
| `find_path_references_scoped` / `collect_path_refs` | Normalize reference matching too; a Use-only change cannot restore the field edge |
| DFG full/subset and reaching labels | Preserve field isolation, occurrence bytes, selected lines, shadow/write/duplicate barriers and confidence semantics |
| CPG parallel/serial argument and return binding | Independently audit raw-text paths before changing collector identities; do not claim interprocedural repair from this intraprocedural fixture |

Keep a language/node-aware descriptor separate from generic `AccessPath::from_expr`.
Normalized text is not a source substring: a member envelope may include erased
syntax while still representing a runtime read. Its base fallback must use the
real identifier span, not invented offsets. Assertions provide no type identity
authority. Unsupported syntax retains existing calls, effects and observations;
refusal of new normalization is not deletion of runtime uses.

Assignment-RHS/call-argument contexts, same-line duplicates, asserted LHS, erased-only
outer contexts, shadow/write sequences, non-TS controls and interprocedural binding
remain subsequent implementation acceptance obligations; this matrix is deliberately
return-focused. No real receiver population/recall, taint reachability or closure
completeness claim follows. React.FC and unresolved react-scripts decisions stand.
