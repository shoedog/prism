# Configured callable observations — bounded producer slice

Approved successor to PR259, merge ebf7933b. Three SELF-PASS rounds, NOT INDEPENDENT;
no agents, production resolver changes, cache transition, installs or app scripts.

## Contract frozen before implementation

Implement standalone Node tooling under scripts/callable-observations/, explicitly
supplied TS5.9.3 compiler (known bytes), project root and relative config path.
No default runtime dependency. It emits observations only, always with
authorizes_runtime_edge=false; no target/class identity is promoted into Prism.

Support a single project, local JSON extends, installed in-root regular-file
dependencies, actual include/exclude/options, and direct annotated arrow/function
expressions. Inventory all project files except .git plus the pinned compiler lib
directory before creating a Program. A memory-only CompilerHost cannot read beyond
these snapshots. Re-snapshot afterwards; differences make the result unproven.
This is a trusted local audit tool, not a hostile-filesystem sandbox or atomic
filesystem transaction. No plugin/configuration/project code is executed.

Unsupported references, plugins, outside-root lookups, symlinks/special files,
invalid UTF-8 compiler inputs, compiler errors and resource limits fail closed.
Record effective-options digest, root/program membership, reads/failed lookups,
file bytes and directory membership. Automatic ancestor type/lib package lookup
may make an ordinary project unproven; never alter its options to obtain a pass.
For controlled positive fixtures, types=[] and libReplacement=false explicitly
bound these optional lookups in the actual fixture configuration, with an explicit
package.json boundary. Metadata exclusion retains sentinels so explicit .git input
requests fail rather than disappear. Lookup and canonicalization follow the pinned
compiler host's case policy; the packet records it. Mixed-policy volumes and
hostile change-and-revert races remain unsupported, not silently certified.

The first packet schema freezes a smaller v0 than PR259's sketch: anchored direct
implementation observations, contextual signature declaration anchors, parameter
and receiver/method declaration anchors, explicit parameter flags and diagnostic
codes. No alias-chain or class-authority certificate is claimed; those fields are
not invented from symbol strings. Full alias-chain/class/write proof and nested
callback binding ownership require separately approved follow-ons.

Validation takes a JSON packet plus independently caller-supplied root/compiler/
config. Before any project read, reject oversized/unknown/malformed schemas and
unsafe manifest IDs. Recompute the entire deterministic packet and require exact
structural equality, including closure, negative lookups and observation anchors.
Validated means reproducible observation, NEVER an authenticated proof or Exact
edge. No persistent cache exists; full recomputation replaces every epoch.

## Gates / plan

1. Captured RED against a minimal non-authorizing stub: require configured roots,
   actual declaration provenance and tampered-packet rejection, not missing-module
   setup errors. Enumerate negative fixtures before implementing.
2. Implement bounded snapshot/host/producer and strict schema/recompute validator.
   Child timeout bounds compiler work. Preserve zero writes to audited projects.
3. Test config/extends/paths, aliases, augmentation, explicit-any, assertions,
   missing/restored dependencies, membership and A↔B/stale/tampered packet changes,
   Unicode anchors, unsafe paths, unsupported references, symlinks and budgets.
4. Audit owner-provided application read-only with private evidence kept local;
   no dependency installation, no claimed gains. Re-run prior40 compiler fixtures,
   four audit helpers and full Rust default/MCP suites. No src/ change means no
   Tier-A trigger; inherited quick limitation remains baseline-invalid.
5. Three self-review rounds, archive, reconcile handoffs/roadmap, scoped commit,
   push and PR under standing authority. Stop at new authority needs or3GiB floor.

## Implemented contract / closeout

The executable strict schema and usage are in scripts/callable-observations/
schema.mjs and README.md. Validation reports packet_status separately: reproduced
unproven evidence is still unproven. Compiler pins are requirements until
compiler.verified=true. All results retain authorizes_runtime_edge=false.
The schema is frozen with this slice; changing it or producer bytes invalidates
earlier observations under full recomputation. No production cache exists.

Final local verification:23 producer/validator tests, prior40 compiler cases and
four audit-helper tests; full Rust default4017/0/1 and MCP4207/0/1, including
doctests. Same-date readout records RED controls, bounded final-round corrections,
limitations and custody. No alias-chain/class/write or Exact consumer is delivered.
