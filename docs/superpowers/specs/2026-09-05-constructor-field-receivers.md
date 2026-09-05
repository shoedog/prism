# Bounded constructor-field receiver ownership

Base: main1e26301, PR247 merged. Owner approved the recommended constructor-backed
`this.library` slice; four candidate App.tsx spans in the fixed Excalidraw archive.

## Contract and architecture

Recover plain `this.field` calls in an own instance method or lexical arrow from
one direct top-level constructor assignment `this.field = new C(...)`. Reuse the
existing runtime constructor binding/import/class proof and shared `FieldTyped`
resolution route for full, direct-subset and navigation consumers. Type annotation
spelling alone is never evidence. An optional single plain uninitialized public
field declaration may have an indexed annotation, as in the real App source.
Classes with heritage require that own declaration: otherwise an inherited
accessor could intercept assignment. This conservatively includes implements-only
heritage. The source-field model assumes standard define-field semantics, not
legacy TypeScript assignment-only emission.

Reject dynamic-this functions, static members, explicit `this` parameters, field
initializer evaluation, class expressions, parse errors, decorators, ambiguous
slots, initialized/optional/nonpublic fields, multiple constructors, constructor
returns, non-direct/conditional initialization, and constructor-local calls before
initialization completes. Whole-class syntactic field/member writes invalidate
the proof, including computed/destructured/loop/update/delete targets. Recognized
direct Object/Reflect mutation helpers targeting this or the field also invalidate.
Static unrelated slots and unrelated writes must not invalidate a valid proof.

This is a bounded lexical constructor invariant, not a whole-program temporal or
alias analysis. No inherited-field recovery, transitive early-this escape/calls,
external mutation, dynamic rebinding, arbitrary reflection or subclass override
proof is claimed. Existing local constructor recovery has the same external
mutation boundary. Real App performs other this calls before its library assignment;
we do not claim those calls have been proved unable to reenter an instance method.
React.FC, hooks, returned receivers, and initialization from constructor parameters
remain excluded. A future temporal/alias authority layer is a separate design.

## Plan and acceptance

1. RED full/direct-subset identity tests against merged1e26301; positive JS/TS/TSX
   and negative/edge controls for every gate. Preserve a base release binary.
2. Shared AST evidence and classifier integration; three-round self-review cap.
3. Invalidate CPG67/nav36 artifacts with CPG68/nav37; exercise good/bad transitions
   and cached constructor-owner replacement, including served navigation.
4. Run complete default and MCP suites, formatting/clippy, immediate release build
   before Tier-A matrix and quick. No rebaseline or full multicorpus.
5. Pair base/candidate over the same five-file archive and earlier small controls;
   report unique source spans separately from caller-expanded records and verify
   actual served callers/callees. Do not promise four record gains.
6. Publish implementation, readout and handoff; commit, push and open PR, not merge.

## Hypothesis/probe/result log

- Shared classifier excludes compound receivers; base positive fixture should lack
  Exact identity, while existing exclusions should pass. `red.log`: two pass, one
  fails with no Exact target. Alternative owner lookup failure is separated by
  the existing direct imported constructor route and later owner-decoy assertions.
- First green build hit two Rust iterator lifetime errors; inadmissible behavioral
  evidence. Fixed lifetime locals; `green-complete.log`: three pass, zero fail.
- Round2: reflective-call normalization missed parenthesized this and bracketed
  Object.assign. Complete probe2/1 named both false Exact cases; ordinary
  parenthesized assignment controls passed. Fixed both; round2-green and cache
  transition/owner-replacement logs pass.
- Round3: inherited setter without own field intercepted assignment yet produced
  Exact Client.m; declared-own-field control passed. Complete probe2/1 names one
  failure; require an own slot for classes with heritage. Three-round cap reached
  with closed bounded findings; final verification and measurement in readout.

Evidence root: `/private/tmp/prism-constructor-fields-NWyrfq`.
