# CommonJS producer mutation barriers

Base c3d110ef; third local increment in the module-binding PR bundle. Two review
rounds maximum. No publication performed or inferred.

The proposed CJS forwarding slice exposed a prerequisite: raw CJS Local facts
survive export-object aliases and unsupported overwrites. Fix this producer
boundary first; do not promote the remaining forwarding Gap cases. A singleton
const destructured require is not proof of a mutable property's read-time value.

## Bounded contract

Account for module/exports occurrences only in simple top-level export assignment
left-hand sides. Unaccounted aliases, escapes, reflective/computed/nested writes,
dynamic eval/with and the CommonJS this alias revoke CJS-derived facts. Multiple
whole-object replacements and replacement/member mixes are refused. Replacement
object properties must not hide an overwrite through computed keys, duplicates,
methods or expression values. Preserve simple direct exports and independent ESM
facts. This is a conservative producer barrier, not runtime heap or module-cache
authority. Cross-module mutation and require-time consumer snapshots remain open.

Capture the complete negative/positive fixture population on unchanged base;
inspect raw facts and resolved targets to distinguish producer loss from fallback.
Verify JS/TS/TSX full/subset, serialized facts, exact changed-set incremental CPG
flow revocation/restoration, and cache invalidation. Run full Rust configurations,
examples, helper/authority/Python gates and freshly rebuilt Tier-A matrix/quick;
carry existing unavailable evidence and timed-out oracle results explicitly.

Design round1 identified independent CJS Local terminal provenance defects,
explicitly deferred rather than called fixed by this barrier. Round2 found
duplicate temporary CJS conflicts still contaminating ESM facts. At the cap this
was closed/enumerable: two captured RED regressions, then one merge-guard repair,
no restart or broader capability. Duplicate CJS now discards the entire temporary
CJS set; a single safe ESM/CJS same-name collision remains conflicted.
One bounded hardening extension repaired the full-suite nested-shadow regression
(same-environment base passes), reused closer lexical bindings, and added escaped
identifier/property and wrapper arguments refusals. Eight RED hardening tests;
independent targeted-delta approval W0/S1. Final matched cjs_ selection:
base44pass45fail, candidate89pass; all module-binding controls62pass.
