# Bounded instantiated Props/property-to-class observations

Owner explicitly approved this slice and agent commit/push/PR. Base PR262 head
f0c9053c942525d59683849df83fcd98dc141f89, dependent on PR261. Cap3 SELF-PASS,
NOT INDEPENDENT; no agents. Only standalone observer tooling/docs/tests change.

## Contract and architecture

Schema /2→/3, producer0.3→0.4. Add props_class to existing nested calls. Preserve
the lexical and callable-provenance barriers; require a unique contextual call
signature, its actual instantiated first parameter type, and no explicit
implementation parameter annotation. No assumption about React/FC spelling or
which outer generic argument represents Props.

Support one property from a plain Props parameter, or a flat destructured/renamed
first-parameter binding. Props must be a singleton non-inherited interface or
type literal, optionally a generic type-alias literal. Record defining-source
Props/property declarations and paired generic binder/instantiated argument types
and declaration anchors. Reject merging, inherited/mapped/union/intersection Props,
optional/computed/accessor/index shapes, property aliases requiring arbitrary
type evaluation, and deeper receiver paths. The selected property annotation must
reference a class or a type parameter belonging to this Props declaration.

Use the compiler-instantiated property type, not its printed name. Record only a
singleton non-generic ClassDeclaration instance, not a structural interface,
constructor typeof Class, union, any, inherited/merged class or class expression.
Import resolution is the pinned compiler's configured Program; defining-file
declarations and the existing packet's full source/module-resolution manifest
are evidence. This is NOT a certificate of every intermediate type-argument alias.

Only mark observed when lexical binding is linked, callable provenance is traced
and final configured-program closure is observed. Preserve partial candidate
anchors when the final Program is unproven; downgrade status explicitly. Never
upgrade closure from compiler type evidence. All runtime/class-authority flags
remain false: declared class instance type does not prove the runtime value,
subclasses, class/member writes or opaque effects.

At most8 Props type arguments (caller may lower), one property per bounded nested
call. Existing input/heap/time/call budgets remain. Source digest includes the new
module. Strict pre-I/O nested-anchor/status checks and complete recomputation
reject obsolete, malformed, forged and stale A→B→missing→A packets.

## Plan and gates

1. Capture RED for missing imported generic Props/class provenance and old-schema rejection.
2. Implement bounded observations; positive inline/interface/alias/generic/import/
   rename/misleading-FC-order fixtures; negative scope/type/constructor/augmentation/
   duplicate/write/explicit-any cases and mutation/replacement controls.
3. Three self-review rounds; retained54 observer tests,40 compiler fixtures,4 helpers,
   full default/MCP Rust suites and fmt/diff. No Rust/runtime change means no Tier-A
   trigger; prior quick remains baseline-invalid. No full multicorpus/rebaseline.
4. Read-only real/private source checks if runnable; private details remain local.
   No dependency installs, application writes or manufactured config closure.
5. Snapshot, commit/push and dependent PR with exact limits and gate totals. Stop
   below3GiB disk. Runtime consumer and alias/effect proof remain separate work.
