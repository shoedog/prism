# Opus-5.5 reviewer brief — lane P plan / P1 prototype

READ: exactly **two plan review rounds**. Controller binds the plan/prototype commits and owner answers at dispatch. READ: supplied round 1 returned FIX (2 WRONG / 5 SMELL), now folded; round 2 is the next independent review; do not treat self-checks as Opus approval. READ precedents: S1 D5/§12, S1b §0 dated amendments, CLAUDE.md Exact static-binding model. Runtime mutation and the owner’s Option K are fixed. Numeric LOC caps are abolished.

READ: review the finite P1 cut, not all of TypeScript. Label every claim MEASURED / READ / ASSUMPTION. A WRONG needs input/state, reachable incorrect result, mechanism/file:line, bounded fix and a realistic failing regression. A request for unsupported P2 precision is a SMELL with measured cost unless it constructs an in-scope incorrect result. WRONG first; unproven concerns are never blockers. Do not downgrade an earlier WRONG without mechanism-level proof.

## Check the causal seams

ASSUMPTION: verify these independent questions against SPEC and actual source:

- Configuration selection: nearest supported including ancestor, invalid nearer barriers, inherited/replaced files/include/exclude, JS admission, outDir default-exclude refusal; package-folder, non-ASCII wildcard and case disagreement barriers; declaration sibling priority and explicit-files exemptions. Root-file ownership is OQ2; do not silently impose transitive ownership after the owner answers.
- Provenance: paths without baseUrl use the paths-declaring directory; inherited baseUrl keeps its declaring directory; child paths override wholesale. Exact key before longest prefix; declaration-order ties are deliberately refused. Missing selected mapping never tries a less-specific key.
- Candidate precision: outside root, opaque/symlink/unindexed/declaration blockers, extension/file/index competition and package boundary cannot manufacture a new module proof. False negatives specifically declared by the finite cut preserve base; no permissive global/stem fallback.
- Callable authority: module target differs from callable origin through barrels; only existing span-backed callable proof and wrapping/JSX/unique function guards grant new Exact. Parameter shadowing, unproven positions, require bindings sharing a module with ESM, CJS span-less origins, written/may-call exports and namespace/class routes stay base. R3 and relative export closure must not change.
- Cache: content/occupancy keys, addition/removal and inherited config edits, 104/60 bound to actual 103/59 parent, cold/full-hit/sidecar/no-cache and config-only incremental parity. No-config Go topology is unchanged.
- Evidence independence: the compiler package’s bytes and version are bound. TypeScript config parser/checker and call-token/import proof are independent of prism’s helpers. Caller/source universe is intentionally inherited from main’s dump/import facts. Root ownership and React wrapper grading use the stated contract; neither is a proof of runtime behavior.

## Verify evidence and tests

MEASURED planner reference: X 3,121 changed rows, all individually correct static bindings; R/T zero; key additions/removals zero. This is changed-row correctness, not whole-corpus precision. Private F is open until fresh controller aggregates; no historic F count certifies this body. Verify the raw JSON/source hashes and actual function spans, including typed arrow variables and React render arguments, rather than trusting aggregate prose.

ASSUMPTION: rerun the base/head controls, at least three of twenty kernel mutants, and the config-only cache control. Check the ten integration mutants’ behavioral failures. Pin both JSX/TSX grammars and wrong-file/nested decoys. Distinguish a conservative boundary mutant from a proven false-Exact mutant. Confirm every preservation control has full-row equality, not only “no Exact.” Setup/compile/zero-test errors are inadmissible.

ASSUMPTION: challenge the design with one accepted and one refused input per path, including the R1 C39–C71 controls: include/file/exclude selection; nested packages and inheritance; exact/wildcard suffix/capture/tie; malformed JSONC/duplicates; occupied declaration/ignored candidate; directory index/package.json; CJS/span/wrapper/shadow; config addition/removal/byte edit. Rebind checkout and base before attributing any failure; same-environment base controls are required.

## Finding and convergence format

ASSUMPTION: for each finding supply tag, concrete failure (for WRONG), bounded fix, at least one alternative with cost/risk/maintenance tradeoffs, and assumptions under which it would not apply. Close prior findings as CLOSED / NOT CLOSED / OUT OF MODEL (test) / DEFERRED (owner decision). Distinguish receipts from local execution.

READ: at round two do not silently extend. Closed enumerable findings get a targeted fold; open-class ownership/extensions get parked and escalated. Never restart this partially reviewed artifact without explicit owner approval and a written unsalvageability reason. Give a convergence view and verified/not-verified statement.

READ: finish with exactly `VERDICT: APPROVE` (zero WRONG) or `VERDICT: FIX (<n> WRONG / <m> SMELL)`.

## Controller notes

READ: subject `__PLAN_COMMIT__`; cumulative prototype `__PROTO_COMMIT__`; implementation parent `__IMPLEMENTATION_PARENT__`; round `__ROUND__` of 2; prior findings `__PRIOR_FINDINGS__`; owner answers `__OWNER_ANSWERS__`.
