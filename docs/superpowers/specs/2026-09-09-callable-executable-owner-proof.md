# Bounded executable-owner proof design

Original status (PR295): design and executable characterization, based on main `7040ceb6`.
Update: the approved [detached constructor](../plans/2026-09-09-detached-owner-constructor.md)
now implements the first checkpoint, with no resolver wiring, opt-in CLI,
dependency installation or cache change. The observer's internal factory changed;
its schema and closure policy did not. This narrows the
[S10 authority contract](2026-09-08-callable-production-authority-contract.md);
it does not relax it. Production integration remains a separate owner decision.

## Intended gain and meaning

One direct TS/TSX contextual receiver, with all three declarations in project source:

```ts
// contract.ts
export type Handler<P> = (props: P) => void;
// client.ts
export class Client { m(): number { return 1; } }
// app.ts (or app.tsx)
import type { Handler } from './contract';
import { Client } from './client';
type Props = { client: Client };
export const run: Handler<Props> = ({ client }) => { client.m(); };
```

Names above are illustrative, not privileged. The future gain is one static Exact
edge to the unique source method body. It does **not** establish the runtime object's
identity: TypeScript is structural, and overrides, replacement and actual invocation
are not proved by a class type. No dynamic-dispatch or real-corpus recall claim follows.

The unchanged observer already reports complete semantic closure and the method
declaration for this fixture. Current Prism resolves no Exact member edge. Its inline
signature and explicit-parameter routes still resolve their own intended owners.
See the [measured readout](../../eval/receiver-closure/2026-09-09-executable-owner-design.md).

## Constructor predicates: all required, no partial positive

1. **Acquisition and closure.** S10's independently supplied root/config/profile/link
   policy, pinned compiler/producer/library bytes, exact schema reproduction, stable
   input and fixed semantic-closure v3 gates pass. Every relevant entry, lookup and
   negative fact is in the epoch. Budget, unsupported, unknown, outside/refused or
   incomplete outcomes produce no proof. No receiver-local closure island.
2. **Index/Program agreement.** Every project JS/TS source indexed for this analysis
   must be accounted for by the authenticated Program, with the same canonical file
   identity and bytes. An indexed source excluded by tsconfig causes refusal of this
   new route, even when compiler closure is complete. Record both complete censuses;
   subset construction uses the owning full input census, not a conveniently smaller
   subset. Compiler-library files form a separate authenticated, non-executable-owner
   domain: do not reject a Program merely because standard libraries are ambient.
   Do not change what Prism indexes to make this condition pass.
3. **Implementation ownership.** A unique module-level `const` declarator has an
   explicit type annotation and an immediate, synchronous, non-generic arrow
   initializer. Exactly one required parameter is a flat, one-element shorthand
   object binding without explicit type, optionality, default, rest or rename. The
   first route accepts only a block containing the one direct member-call expression
   statement, with no call arguments. No nested function, wrapper, JSX expression,
   assertion, `satisfies`, function expression, async/generator or inferred annotation.
   TSX extension alone does not grant JSX/wrapper support.
4. **Callable provenance.** The annotation is one named type reference with one
   type argument. A unique direct named type-only relative import reaches a unique
   direct exported project-source type alias, with no barrel, default/namespace alias,
   renaming, package mapping or declaration/runtime pairing. The alias has exactly
   one unconstrained, non-defaulted generic binder and a function type with one
   required parameter whose whole type is that binder; the return is `void`. No
   overload, inheritance, interface merging, callable property or type computation.
   Require the actual checker signature and substitution to agree with this source
   shape. Binder names or printed `Handler<Props>` strings are insufficient.
5. **Props/property provenance.** The type argument resolves to a unique local,
   module-private, non-generic type alias containing exactly one required property
   signature, corresponding to the actual parameter binding. No optional/readonly,
   index, mapped, intersection/union, accessor, inherited, exported/imported or merged
   Props shape in this first constructor. The property type is a direct, uninstantiated
   reference to the class below. Derive the property symbol and class declaration
   from this instantiated contextual parameter, not from the printed receiver type.
6. **Executable class identity.** A unique direct named value import resolves one
   relative hop to a direct exported named, non-generic, non-ambient, non-abstract,
   non-decorated project-source class without heritage. Its compiler symbol has
   exactly the supported declaration, with no augmentation/merge. Its module binding,
   export and source class census agree with Prism's clean-class facts. Declaration
   files, structural interfaces, `typeof`, asserted types and package names cannot
   supply a class body. Require canonical source identity, not basename equality.
7. **Executable member identity.** The call is a non-optional, non-computed property
   access on exactly that lexical parameter binding. The checker-selected declaration
   is one own ordinary public instance method with an actual body inside that class;
   no static method, arrow field, accessor, overload, computed key or inherited member.
   Map to the unique Prism method node and existing direct-method admissibility facts.
   Require agreement of source bytes, normalized node/token boundaries, class/member
   containment and FunctionId. TypeScript's exported declaration range may include
   `export`; never require accidental equality with a parser's inner class range or
   fall back to names/line numbers if mapping fails. Refuse parse recovery, duplicate
   source spans, ambiguous FunctionId mapping or unproven instance slots.
8. **Negative ownership/effect facts.** Retain existing duplicate, shadow, export,
   module-value-write, receiver/member-write, computed-slot and `this`-write barriers.
   The entire indexed source census retains the existing imported-Props route's
   conservative ambient/augmentation, prototype and Object/Reflect mutation fences;
   the new route cannot quietly ignore them. First-arrow grammar also refuses receiver
   reassignments, aliases, opaque escapes and extra statements. This is a static
   source-domain condition, not a proof against all possible external runtime mutation.

Accepted shapes are intentionally smaller than both existing independent routes and
observer eligibility. In particular, already supported arrow fields, private Props
interfaces and React declaration observations do not imply support in this constructor.

## New facts and ownership architecture

The current direct-body observation contains call/receiver/method declaration anchors,
but **no `props_class` record**. The existing nested record is not a shortcut either.
The future trusted compiler pass must derive the direct binding, instantiated parameter,
required property, class/member symbols, declaration census and source identities from
one live Program/checker. It must retain the actual generic substitution relation.
Adding a `proven` bit to today's packet cannot implement this constructor.

Use two distinct internal layers, neither publicly constructible or deserializable:

- `AuthenticatedProgramEpoch` owns independently reproduced evidence, compiler/source
  identities, full index census and negative facts. A trusted acquisition path alone
  creates it. Raw worker JSON never directly deserializes into this authority type.
- `ExecutableOwnerProof` has private fields and a private constructor that requires
  that exact epoch, derives predicates 2–8 and binds the specific original call to
  one executable member. Any serialized additions are observations requiring the
  same full reproduction as S10; they are not serialized proofs.

Choose an owner-held `Arc` identity for the epoch. Consumption checks identity with
the active owning analysis, not just matching hashes, caller-controlled IDs or paths.
An equal-content epoch from a second session is not interchangeable. Ownership does
not replace source authentication: both are required. Do not expose setters for proof
fields, deserialize authority, or let a public helper accept a naked "validated" bool.

Each proof binds acquisition/policy versions; full source/config/compiler/library and
lookup snapshot; call/receiver/annotation/implementation/parameter anchors in original
UTF-8 and UTF-16; import/alias/binder/substitution/Props/property/class/member identities;
parser mapping; and the negative-fact/index census. Equality of call bytes is not enough.
Changing any one field requires a new derivation. Missing facts refuse, not default.

## Resolver and lifecycle plan, explicitly unimplemented

1. Construct proofs off to the side under one authenticated epoch; no graph mutation
   while any required evidence is incomplete. Install an immutable exact-call-key map
   owned by that analysis. Never place a class guess in `pre_resolved_target`, whose
   existing meaning is ParameterCallback.
2. Preserve explicit-parameter precedence and existing local/imported-Props routes.
   A call eligible for this new route may use its proof only at the shared resolver.
   Candidate refusal is terminal for this new ownership claim; it is not permission
   for a same-name fallback. No opt-in/no proof preserves independent old behavior,
   including the explicit Decoy control, rather than globally suppressing resolution.
3. Full and subset construction reference the same owning epoch and full negative
   census. Incremental positive A → positive B → unproven replaces maps atomically;
   never union, reuse a prior positive on acquisition failure, or mix sessions.
4. First production integration should keep this proof ephemeral and bypass persisted
   graph/navigation results for opted-in analyses until their complete epoch identity
   can be checked. Proof-derived edges must not enter an ordinary reusable sidecar.
   Choose and test exact load/write bypass points and any necessary cache-version
   bumps before wiring; merely declining to serialize the proof does not prevent
   serialization of an obsolete Exact edge. CPG77/navigation45 remain unchanged here.

## Implementation acceptance and value checkpoints

| Next bounded increment | Required evidence before advancing |
|---|---|
| Detached proof constructor, no production consumer | Capture candidate constructor RED on the accepted base; derive new direct facts; pass accepted TS/TSX cases and individual predicate negatives; two genuine epochs reject substitution even when call bytes match; no new production edge |
| Public opt-in wiring, only after constructor review | Capture public candidate missing on its actual base, then gain only the intended member; preserve both independent controls; full/subset and served callers/callees agree; explicit closure and terminal-refusal tests |
| Lifecycle/cache/CPG hardening, required before shipping | Exact same-owner proof consumption, positive/different/unproven and dependency/config/negative-lookup changes; cache load/write bypass verified; no stale Exact dataflow in CPG/trace; complete Rust/MCP and Tier-A gates |

Do not ship partial production wiring between the second and third checkpoints.
One implementation PR may contain those two bounded increments if the reviewed
change remains small; otherwise use a disabled/stacked lane, not an enabled partial
consumer. Installation/benchmark/closure-policy expansion is not implicit permission.

The committed corpus supplies 27 scenarios in TS and TSX, plus full/subset baseline
characterization. Its `design` labels are requirements, **not executed proof refusal
decisions**. Closure failures are exercised by the existing observer; all imported
candidate failures currently share the absent resolver route. Thus they cannot prove
the eight constructor predicates. Before implementation acceptance add focused cases
for every grammar/identity/effect predicate, Unicode anchor disagreement, each mixed
authenticated field, and lifecycle transition; do not count baseline-passing controls
as behavioral RED. The two-genuine-source fixture is presently an attack setup only.

Value gate: a constructible one-edge opportunity is established; measured real receiver
gain remains zero. The public/private Programs remain incomplete in their prior audits.
`react-scripts` remains intentionally unresolved; no install, exclusion or waiver.
React.FC/package executable pairing, larger Props shapes and out-of-Program JavaScript
are later design decisions, not additions to this fixture-sized route.
