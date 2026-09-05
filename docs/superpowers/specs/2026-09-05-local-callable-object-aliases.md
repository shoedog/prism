# Local single-call-signature object aliases

Approved next slice after PR251 merged89f6ebf7. Branch
feat/local-callable-object-aliases; three-round SELF-PASS cap.

## Contract

TS/TSX variable-initializer arrows/function expressions may recover a destructured
receiver from a unique module-local alias whose direct object RHS contains exactly
one call_signature and no other non-comment named member. Support non-generic
`type F = { (p: Props): void }` and the already-bounded one-parameter generic form
`type F<P> = { (p: P): void }; const run: F<Props> = ...`.
Plain direct object parameters and proven local Props aliases retain their bounds.

No inline anonymous callable-object annotations, interfaces, inheritance, overloads,
extra members (even optional), constructors, method slots, unions/intersections,
wrapped RHS, alias chains, nested/imported/ambient/qualified aliases, defaults,
constraints/variance or generic inference. No React.FC/hook authority. Call-signature
generic parameters reject, independently of the alias's allowed generic binder.

The same required plain contextual parameter, implementation-pattern, explicit
annotation, duplicate and write gates apply. Keep original signature nodes for
non-generic declarations and concrete argument nodes for generic instantiation;
never reconstruct type strings. Return annotations do not grant receiver authority.

## Architecture and plan

1. Save untouched base release and complete RED on all new positives/negatives.
   Existing function-alias controls discriminate the missing shape gate from a
   broken owner route. Migrate the two obsolete single-call-object negative rows
   into new positives; retain overload and extra-member negatives.
2. Contextual selection obtains one proven local alias's RHS, then normalizes a
   single-call object to its original call_signature. Direct function annotations
   remain accepted; direct object annotations remain excluded. Reuse downstream
   parameter and bounded substitution proof without broadening props shape lookup.
3. CPG71→72/nav40→41. Persisted good↔bad and declaration/argument-only A↔B owner
   replacement, full/subset/incremental parity, sidecar negatives/old-version miss.
   Round2 boundary expansion; round3 source/consumer review.
4. Full default/MCP, fmt/clippy. Immediate release rebuild before each matrix/quick.
   Pair saved base/candidate on fixed Excalidraw and prior Python/JS controls;
   assert served fixture gains separately. No real React.FC/useContext gain promise.
5. Archive evidence, commit/push/open PR and docs-only publication update. No merge,
   rebaseline or full multicorpus.

## Hypothesis/probe/result log

Current contextual gate requires function_type; expect callable-object positives
to lack Exact ownership and existing function-alias positives to pass. Alternative
owner failure is discriminated by that control. Indexed helper unavailable:
SymbolNotFound is not no-callers evidence. Current source is the navigation fallback.
Results in readout; evidence /private/tmp/prism-callable-aliases-LhPrH3.
