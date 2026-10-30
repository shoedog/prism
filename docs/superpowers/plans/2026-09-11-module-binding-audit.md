# Module-binding audit: executable baseline before expansion

Base: merged PR #313, `afc78147`. Owner approved a TDD audit of imports,
requires, exports, and forwarding through local/renamed symbols. Local work only;
no new publication or merge authority is inferred.

## Contract and sequence

1. Give every proposed form a named executable case, with exact target identity
   (file and declaration), not merely a resolution-kind count. Exercise JS, TS,
   TSX, and full/subset construction. Reject parser recovery in positive cases.
2. Measure on unchanged production base before edits. Separate supported cases,
   missing capability, deliberate refusals, and incorrect exact targets. Capture
   every failure in a complete run; no first-failure-only population inference.
3. Pair comment/no-comment controls for import labels, member identity, exports,
   and reflective receiver writes. Trace the failing producer/consumer before
   repair. Alternative mechanisms and discriminating observations go in the log.
4. Select bounded repairs from measured failures. Forwarding expansion is gated
   by origin/local/export/consumer identity plus shadow, write, duplicate, cycle,
   depth, and type/value barriers. Never substitute spelling for origin proof.
5. Two review rounds maximum. If findings are open-class, stop expansion and
   hand off the exact population/design requirement rather than widening blindly.
6. Full Rust feature suites, project Node/Python checks and triggered Tier-A;
   retain base failures and unavailable fixtures, never fabricate baseline data.

## Forms

- Named/default/namespace ESM imports; same-name and renamed imports/exports.
- Direct named/default/star ESM re-exports.
- Whole-module, destructured, renamed, and member-access require bindings.
- CJS local function/object/member exports and whole-module forwarding.
- Imported/required local forwarded via ESM or CJS, same and different names.
- One extra immutable local alias; dynamic or mutable forms remain unproven.
- Negatives: shadowed require/module/exports, overwritten/detached exports,
  duplicate exports, missing modules/members, type-only bindings, cycles/depth.

## Non-goals

No React.FC, closure admission, package-resolution expansion, default-value
propagation, or general alias/heap analysis. No private corpus disclosure.

## Evidence

Local logs: `/private/tmp/prism-module-binding-LgUwcS`.
Merged predecessor has the same tree as its tested closeout; its publication-held
notes describe the earlier implementation checkpoint and are superseded by #313.
