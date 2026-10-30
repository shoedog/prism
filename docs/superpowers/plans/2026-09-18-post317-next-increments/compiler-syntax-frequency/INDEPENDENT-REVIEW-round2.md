# Independent planning review — round 2 of 2

**Artifact:** `artifact-manifest.json` SHA-256 `768e9249535267922e4bdb038d90213e58520c8edfb4a228ab888c6425a2c7fe`; `SPEC.md` SHA-256 `a44bf6002b129862a9eb906fab9e9a31074aa017ae62ca5d198a663a9c31d13f`.

Round one found one finite schema contradiction and one API ambiguity. Both are corrected in the same planning artifact:

- Counter derivation now names file rows, diagnostics arrays, nested callable/parameter rows, and the separate bodyless-exclusion tally. The exact function-type control requires one exclusion and zero emitted rows.
- Parenthesized-arrow detection now requires `arrow.getChildren(sourceFile)` and an `OpenParenToken` before `equalsGreaterThanToken`, with the required arrow vectors retained.

The packet remains syntax-only and holds the 600 implementation / 600 test / 1,200 total caps, early checkpoint, fixed compiler/414-member custody, ten complete control groups, core-before-public gate, and no native/Program/dependency/support claims.

**VERDICT: APPROVE — 0 WRONG / 0 SMELL.** This is planning approval only. No implementation, public parse, census, test suite, publication, or merge occurred in this review lane.
