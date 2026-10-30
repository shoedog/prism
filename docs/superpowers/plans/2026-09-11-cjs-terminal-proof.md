# Direct CommonJS terminal/capture proof

Historical fourth increment, delivered in `88af6511`. Current continuation:
[enumerable refusal and scope repair](2026-09-11-cjs-refusal-scope.md).

Base032e1824, fourth local increment in one future MR. No publication. Two review
rounds maximum. Root source writer; review delegated read-only under standing authority.

Only a unique top-level function/generator declaration or a direct callable
variable initializer may supply a CJS Local target. Match the actual eager callable
name and byte identity, not a same-spelled nested candidate. Variable initializer
must finish before the export RHS; function declarations may be hoisted. Preserve
arrows, anonymous/same-named expressions and closer lexical shadow writes.
Different self-names, wrappers, aliases, imports/type collisions, duplicate values,
pre-capture writes and non-root escaping writes refuse. Only simple root identifier
assignments strictly after the entire export statement may be ignored. Proof is
per export occurrence: a first snapshot can survive while a later one refuses.

UnprovenLocal is a refusal-only exported-name claim, not a callable. Keep it for
raw duplicate accounting and valid sibling exports. Resolver distinguishes no
target from a blocked explicit claim: unproven/conflicted claims propagate through
named/star paths instead of disappearing and permitting another star branch's
callable. Existing depth/cycle bounds and class/ESM proof contracts remain.

Round2 exposed duplicate CJS whole-set claim erasure before the resolver. One
disclosed bounded repair/verification extension retains refusal markers for all
names in that rejected set, without changing independently owned ESM names.
Three captured RED tests cover star, named-then-star and revoked disjoint sibling.
Targeted independent delta approval W0/S0. Declaration-self writes use a CJS-only
scope distinction: declaration names bind outside, expression self-names inside.
Other receiver/ESM scope uses remain independent audit work.

Capture RED on unchanged production, including the complete prior module/CJS/ESM
control population before review. Add exact producer-only source epochs, actual
CPG argument-to-parameter flows, raw-fact serde replay and duplicate order cases.
Verify full Rust configurations, examples, Node/authority/Python and fresh-release
Tier-A matrix/quick with existing exclusions explicit. No require-time snapshot,
forwarding, receiver, React.FC or closure-policy expansion.
