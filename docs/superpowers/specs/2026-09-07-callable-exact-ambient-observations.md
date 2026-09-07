# Bounded singleton exact-ambient lookup observations

Approved after PR272, base `c49d5b93e896ba699ee378e7de3027fb59b6d163`.
Observation provenance only; no Program-closure policy or runtime change.

## Contract

Schema8 / producer0.9.0 adds `resolutions[].lookup` alongside unchanged
`from`, `specifier`, `target` filesystem fields. Each occurrence has an actual
literal anchor (null for synthetic requests), a bounded request-context enum,
checker module-declaration candidates, and same-name provider/augmentation
declaration censuses over the configured Program. Status `observed` means only
singleton exact ambient binding observed. It never discharges `target:null`,
refused/outside probes, diagnostics or any existing closure/class/write barrier.

Supported positive contexts: import declarations, export-from declarations,
import types (including source-backed JSDoc import types) and import-equals external module references. Require/dynamic-import
requests are distinguished but deferred; declaration names and compiler-generated
negative-position JSX requests cannot serve as import authority.

A positive requires: null filesystem target; nonsynthetic StringLiteral use in
one supported context; exact checker ValueModule binding to one declaration;
one same-name provider across all Program source ASTs; no same-name augmentation;
nonrelative, nonwildcard name; top-level ambient ModuleDeclaration with ModuleBlock
in a declaration SourceFile that is not an external module. The bound declaration
and provider must be identical nodes owned by canonical configured-Program files.
Same-name invalid/nested providers also poison the census conservatively.

Pinned TypeScript package-ID redirects inherit another SourceFile's AST and parent
pointers (`createRedirectedSourceFile`, compiler lines28286–28307,128573–128583).
The provider census must traverse `redirectInfo.unredirected` when present and
anchor its original parsed bytes/file, not the shared redirected tree. Exact
binding still requires identity with the checker-owned declaration; redirect
metadata alone supplies no positive authority. Equal package IDs with distinct
bytes remain separately counted. Tests cover identical and differing copies.

Wildcard/merged/unsupported candidates remain anchored but unproven. A symbol at
an augmentation declaration's own name is never positive. Excluded inventory
files are not Program providers. No spelling, package prefix, filesystem absence,
skipLibCheck or optional-peer heuristic supplies binding evidence.

Strict pre-I/O validation checks enums, fields, canonical Program membership,
anchor hashes/ranges, positive shape and reason consistency. Full recomputation
checks source contents, binding/census equality and every field; no persistent
positive cache. Old schemas reject before audited-root access. New implementation
file participates in the producer hash. Existing profile byte/time/heap bounds
and array ceilings still apply.

## Plan and gates

1. Capture new observation fixture RED on unchanged production at exact base.
2. Implement one per-Program AST census and per-request binding observation helper;
   integrate after existing callable observation work and before final snapshot.
3. Add schema8 strict shape/semantic checks, empty-packet field handling, producer
   version/hash update and stale/tampered/pre-I/O negatives.
4. Run focused then full observer tests, default/MCP Rust suites and authority
   helper controls. Characterization tests from PR272 remain unchanged.
5. Replay the fixed public source without installs/config/lock changes. Compare
   old packet projection, all603 null targets,14 source gaps, closure flags,
   refused/outside evidence and four program_unproven Library candidates.
6. Two SELF-PASS review rounds, NOT INDEPENDENT; classify WRONG before SMELL.
   Commit/push/open scoped PR, preserve raw RED/GREEN and public evidence custody.

No React.FC expansion, runtime consumer, closure admission, wildcard/merged
authority, package acquisition, public application repair or Tier-A baseline edit.
