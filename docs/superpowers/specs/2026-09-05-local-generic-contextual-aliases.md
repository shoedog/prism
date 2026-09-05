# Bounded local generic contextual aliases

Approved after PR250 merged83594a8a. Base83594a8a, branch
feat/local-generic-contextual-aliases. Three-round SELF-PASS review cap.

## Contract

Support TS/TSX `type F<P> = (p: P) => void; const run: F<Props> =
({client}) => client.m();`, where the sole explicit type argument is a direct
object type or an already-proven module-local non-generic props object alias.
The alias RHS must be a direct non-generic function type with one required plain
parameter whose entire type is exactly the alias's sole plain type parameter.
No recursive or nested substitution. Return type does not contribute receiver
authority. Forward/exported declarations and comments are allowed.

Reuse unique module-local declaration/import/shadow/ambient/error gates for F.
Reject missing/extra arguments or binders, constraints/defaults/variance modifiers,
nested generic signatures, wrapped/union/intersection/conditional/mapped shapes,
alias chains, imported/qualified/nested/ambient aliases and generic props aliases.
No React.FC special casing, library authority, inference, hooks or JS semantics.

Retain the concrete argument's syntax node, not a substituted string. F visibility
is checked at the generic name reference; Props at the argument node; property
class names at their original direct-object or Props declaration node. Generic
binder names must not capture same-spelled concrete argument names. Implementation
parameters retain write timing. Explicit annotations remain terminal, even when
unknown. Existing required-property, duplicate/pattern and write barriers remain.

## Architecture and plan

1. Save untouched base release and run complete positive/negative RED. Existing
   non-generic alias positives distinguish missing generic selection from owner
   lookup failure. Enumerate all failures, not just the first.
2. Split module-local declaration selection from non-generic shape checking.
   The contextual path alone admits the bounded generic function alias; return
   the original concrete type node into the existing object-shape checker.
   Existing simple typed parameters keep their original annotation path.
3. CPG70→71/nav39→40. Test persisted positive↔barrier transitions, argument-only
   A↔B owner replacement, full/direct-subset/incremental parity, sidecar negatives
   and old-version refusal. Round2 boundary expansion; round3 source/consumer review.
4. Full default/MCP, fmt/clippy; immediate release rebuild before matrix and quick.
   Saved-base/candidate pairs on fixed Excalidraw and prior Python/JS controls;
   assert served synthetic gain separately. No real React.FC/useContext gain promised.
5. Archive evidence, commit/push/open PR; reconcile publication in docs-only follow-up.
   Do not merge, rebaseline or run full multicorpus.

## Hypothesis/probe/result log

Generic-type syntax currently cannot pass the local shape selector; expect new
positive fixtures to lack Exact edges, with existing local Props controls green.
Alternative owner-lookup failure is separated by those controls. Indexed helper
unavailable: SymbolNotFound is not absent-callers evidence. Current source shows
parameter recovery → contextual annotation → shape selection → member proof.
Results recorded in the readout. Evidence: /private/tmp/prism-generic-aliases-sb86aA.
