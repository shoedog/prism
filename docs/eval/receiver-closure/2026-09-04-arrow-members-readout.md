# Arrow-field authority repair

Base a70ea03 (PR243 and244 merged). Owner approved this bounded slice before a
separate destructured inline-prop proof. Published implementation a03382a in
[PR245](https://github.com/shoedog/prism/pull/245), OPEN, not merged.

## Result and corrected assumption

Arrow fields already had method/class metadata and resolved through simple
constructor/typed receivers. What was missing was their slot/static validation:
the AST predicates expected a direct class-body parent, so arrows inside fields
bypassed them. Base RED: safe-arrow positive passed; member and local-write
negative matrices failed (1 pass/2 fail). Actual static/overwritten fields produced
incorrect ConstructorLocal Exact targets, ruling out missing owner metadata.

The repair normalizes the field wrapper, preserves only bounded direct public
non-static plain-name arrows, and checks duplicate/accessor/computed slots and
explicit this.member writes (assignment/update/delete/loop/destructuring).
Local receiver member writes now use the existing lexical/time/backedge barrier,
also protecting ordinary methods on that shared path. Parenthesized receivers
were a closed round2 failure and are normalized. Non-arrow callable fields,
private/protected/decorated and parse-recovered fields remain outside the proof.
CPG65/nav34 invalidate prior authority. No new self-call, inheritance, reflection
or interprocedural alias-write modeling; no whole-program JS soundness claim.

## Verification

- Full default suite: 3811 passed, 0 failed, 1 existing ignored.
- Full MCP suite: 4001 passed, 0 failed, 1 existing ignored.
- Four focused matrices cover safe and async/expression-body fields, noncolliding
  members, static/duplicate/write/excluded forms, scoped shadow writes, RHS reads,
  after-call writes and loop backedges. Full/subset parity is checked throughout.
- Cached good↔bad transitions and nav-sidecar positives/negative writes cover
  JS/TS/TSX; v64 caches are refused.
- fmt/diff checks and Clippy complete (warnings retained). Rebuilt matrix104/104.
- Rebuilt quick exit2: baseline-invalid solely SHA drift a70ea03cde5f vs pinned
  20c8490591a3; oracle quiescent, oracle/SUT errors0, four stale adjudications.
  Pins: target-c-method flip_candidate (Exact5/0/0, 28 extra default candidates),
  module-deps and load-repo feature-gated literal pins missing, ambiguous-symbol
  ok. Load-repo oracle-only resolution_test.rs:5299/5368; MCP Prism-only counts
  1/4 for the two missing pins. No base quick rerun for regression attribution.
  No baseline rewrite or full multicorpus; not a green accuracy gate.

Self-review cap3: round1 identified the predicate bypass; round2 closed
parenthesized receiver writes; round3 source/consumer/persistence review survived.
This is a self-pass, not independent review.

## Fixed-source measurement

Same five unchanged Excalidraw files at
0642e72cfa2d9a71198200e52f37399384610ee3 as the preceding readout. Saved clean-base
binary vs rebuilt candidate, cache bypassed: **2780 records, 369 Exact both sides,
zero changed records**. All11 unique tracked Library receiver spans remain
NameOnly (18 caller-expanded records). Earlier Black/Excalidraw/JS controls also
byte-identical (400/136/12 records, 79/26/4 Exact).

This corrects the prior claim that arrow membership alone blocked these real
sites. Remaining receiver shapes are inline object props, React.FC props,
this.library and useApp-return destructuring. Only inline object props are the
next authorized slice. Partial-source evidence, not whole-corpus recall.

## Custody

Raw RED/round2/full-suite logs, binaries and paired JSONL:
/private/tmp/prism-arrow-members-1F4iiR. Reproduce comparison with
docs/eval/receiver-closure/measure-indirect-default.mjs and the preceding five-file
archive /private/tmp/prism-indirect-default-VUbv13/excalidraw. Existing baselines
and unrelated untracked artifacts are preserved.
Evidence archive: /private/tmp/prism-arrow-members-1F4iiR-evidence.tgz, SHA256
bc4bae43d372a10d7ac7af337482969606080b206e7e3371fa1ec28691978f13.
