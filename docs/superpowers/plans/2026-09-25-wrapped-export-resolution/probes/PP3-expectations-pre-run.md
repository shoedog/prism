# Branch-P + r3 W2 fold expectations (written BEFORE running; 2026-09-25)
Binary: proto-wt-p3 (Branch-P prototype + hoisted `var` competitor walk), b44d8b1c.
C01-C56: identical to PP-controls-proto.txt (the fold only adds refusals for hoisted `var memo`/`var forwardRef`; no C01-C56 fixture has one).
C57 (sol r3 W2: `if (flag) { var memo = fake; }`): drop; reason callee_provenance. (Base: drop UnknownName, no facts.)
C58 (`for (var memo of [1]) {}`): drop; callee_provenance.
C59 (`try { for (var memo = 0; ...) {} }`): drop; callee_provenance.
C60 (`if (flag) { let memo = 1; }` block-scoped): Exact import_member -> lib.tsx:Island.
C61 (`var memo` inside a nested function and a class method): Exact import_member -> lib.tsx:Island.
C62 (S1b RED #1, sol r3 W1a: `<Lib.Island/>` via namespace import, nested decoy Island@3): UNCHANGED from base under S1 = two Exact import_qualified targets (Island@3-3 and Island@6-8). Recorded, not fixed (owner: defer to S1b).
C63 (S1b RED #2, sol r3 W1b: producer-local direct call `Island({}...)`): UNCHANGED from base = Exact local_def -> lib.tsx:Island@2-4 on the plain call site. Recorded, not fixed.
Root-only-walk mutant (M14) must flip C57, C58, C59 to Exact.
# addendum before running C64-C67 (T-R6-P4 rows as controls) and the unified walk (same expectations for both binaries)
C64 top-level `function memo`: drop callee_provenance. C65 `class memo {}`: drop callee_provenance.
C66 `const { memo } = x`: drop callee_provenance. C67 component-local `const memo = 1` in a function: Exact.
Unified single walk (budget trim): C01-C67 summary identical to the two-function P3 binary.
