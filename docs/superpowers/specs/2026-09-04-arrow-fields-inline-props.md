# Bounded arrow fields, then separate inline-prop receiver proof

Authority: owner approved both slices, in this order, after PR243/244 merged.
Base a70ea03. Separate commits/PRs; second may stack on first. Three self-review
rounds per slice. No automatic merge, baseline rewrite or full multicorpus.

## Slice A — arrow-field member ownership

Correction: arrow fields already acquire method owner/class spans from
languages::method_owner and method_owner_class_node. The existing slot/static
predicates inspect the arrow node rather than its containing field and return
unproven=false/static=false. Thus this is an authority repair, not new recall.
Stale prism-nav returned no callers with a stale-index warning; current source
shows full, subset and shallow construction use the same AST predicates.

Contract: existing recovered JS/TS/TSX constructor/typed receiver paths may target
a unique public non-static plain-name direct arrow field, including async or
expression bodies. Field/class parse recovery, decorators, private/protected,
computed slots, duplicate instance slots, non-arrow function expressions, and
visible class-body writes/delete/loop/destructuring to this.member poison it.
Unrelated static slots or other plain instance members do not collide. Preserve
declaration-backed class/import proof; do not add inheritance, reflection,
interprocedural alias mutation, this.field receiver typing or new self-call rules.
Visible local receiver-member writes use the existing lexical/time/backedge
recovery barrier; this also repairs ordinary-method receivers sharing that path.
No claims of runtime immutability or whole-program JS soundness.

Hypothesis/probe/result: on untouched a70ea03 production, safe arrow positives
pass; static/overwritten field negatives fail with ConstructorLocal Exact edges.
Alternative (missing owner metadata) is ruled out by those actual Exact targets.
Same-environment RED logs: /private/tmp/prism-arrow-members-1F4iiR/red*.log.
Function-expression/private/decorator exclusions are bounded-contract concerns,
not all demonstrated runtime errors. Wrong static/overwrite outputs are WRONG.

Plan: normalize the containing member for slot/static predicates; share bounded
write-target recognition; add cold/subset/cached transition/sidecar and edge tests;
bump CPG/nav together. Full default/MCP suites, release/matrix/quick and fixed-source
measurement. Correct prior documents that called all arrow fields unsupported.

## Slice B — destructured inline-prop receiver proof (separate)

Only TS/TSX function/arrow parameters with a direct object pattern and inline
object type can recover a plain local receiver binding from one matching required
property typed by a simple class identifier. Include property renaming; reject
rest/default/nested/computed/optional/duplicate properties, unsupported type
shapes and conflicting lexical/type bindings. Preserve type-only import erasure,
shadowing and receiver/member-write barriers; no React.FC/generic contextual
types, hook-return destructuring, this.field, arbitrary aliases or JS inference.
Design/tests must be finalized against slice A before implementation. Verify full,
subset, cached transitions and sidecar, then measure unchanged real LibraryMenu
site114 and preserve the other ten previously recorded Library sites.
