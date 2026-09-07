# Bounded refused compiler-lookup observations

Owner approved the lookup-packet repair and real-site remeasurement after merging
PR261/262/263. Base54b6755ab1cc4601e61d8e5a0b991a8775c29b76. Three SELF-PASS rounds,
NOT INDEPENDENT, no agents. No runtime authority, resolver/CPG/navigation/cache
change, default dependency, installs, application writes or rebaseline.

## Failure and proof requirements

Inherited WRONG: a compiler probe such as project/node:url is emitted into the
strict failed_lookups path-ID array; the parent rejects the entire worker packet
as worker_failed. Hypothesis: unsafe-ID serialization, not compiler failure or
lookup-count exhaustion. Capture same-environment merged-base RED with ordinary
missing imports as control. Preserve the negative result and partial observations.

## Contract

Schema /3→/4, producer0.4→0.5. Add snapshot.refused_lookup_sha256: sorted unique
SHA256 digests of normalized virtual-root lookup strings that cannot be safely
represented as IDs. They are opaque evidence, never paths to open or resolve.
Keep relative()/path schema strict and fail on unsupported real inventory names.
Do not encode unsafe names into path IDs or silently drop failed probes.

At the in-memory host boundary, refuse invalid in-root IDs before read/exists/
enumeration lookup, record the digest, return unavailable and add unsupported_lookup.
Outside-root and .git boundaries retain their existing behavior. The Program must
remain unproven, dependencies/augmentation/resolution false, and candidate Props
class observations program_unproven. Even an ambient module masking diagnostics
must not erase the refused-lookup closure barrier.

Pre-I/O schema validation requires valid digests, sorted uniqueness and consistency
between nonempty refusal evidence, reason and closure flags. Full recomputation
rejects forged/removed/stale evidence; old schema rejects before audited I/O.
Existing file/byte/heap/time/output budgets apply; no new unbounded probe payload.
Digests describe refused lexical lookups, not proven filesystem absence.

## Plan and gates

1. Capture RED for builtin/virtual probes, ordinary missing control, safe closed
   positive and obsolete schema; enumerate path/control negative cases.
2. Implement at host boundary and packet validator, preserve partial observations.
3. Three bounded self-review rounds including ambient masking, invalid actual
   filenames, traversal, tamper/pre-I/O, repeatability and source replacement.
4. Run full observer suite,40 pinned compiler fixtures,4 helpers, full default/MCP
   Rust suites, fmt/diff. No Tier-A trigger; no fresh multicorpus/rebaseline claim.
5. Read-only configured Excalidraw and private-team replay with actual configs,
   separately validate. Private source/identifiers/details stay local. No inferred
   closure/Exact claims from restored observations.
6. Update readout/handoff/roadmap, snapshot, scoped commit/push/PR. Stop below3GiB.
