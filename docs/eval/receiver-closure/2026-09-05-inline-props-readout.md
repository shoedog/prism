# Destructured inline-prop receiver proof

Separate authorized slice B, based on d26ae0a (PR245 arrow-field repair).
Implementation4a982ed pushed in [PR246](https://github.com/shoedog/prism/pull/246),
OPEN against feat/bounded-arrow-field-members. Merge245 first, then retarget246
to main; neither was merged by this worker.

## Bounded result

TS/TSX direct object-pattern parameters with inline object types recover one
required plain property's simple class type, including renamed local bindings.
Reuse TypedParam, declaration-end timing, lexical/type shadow checks, imported
declaration identity, and receiver/member-write barriers. Type-only imports stay
erased from runtime import bindings. CPG66/nav35 invalidate stale receiver facts.

Reject optional selected properties, defaults/rest/nested/computed patterns,
duplicate keys/locals/type members, accessor/index/method type members, unsupported
selected types, conflicting bindings and writes. Unrelated optional type properties
are allowed. No React.FC contextual types, hook-return destructuring, this.field,
arbitrary aliases or JavaScript inference. No new serialized field/resolver rung.

## Hypothesis, controls and verification

On untouched d26ae0a production, the positive test failed because receiver type
and recovery were absent; both exclusion matrices passed (RED2/1). Missing arrow
ownership is ruled out by slice-A safe-arrow Exact tests. Initial green3/0;
expanded focused run4/0 includes persisted annotation A→B/B→A owner replacement.
Full/subset parity, cached positive↔optional/write-negative transitions and served
sidecar round trips cover the new proof. Same-environment base evidence lives in
red.log and the saved base-prism, not an inferred attribution from another host.

Full default3815/0/1; MCP4005/0/1. The one existing ignored test is
resolution_test::slice_elem_variant_reserved. fmt/diff checks and Clippy complete
(warnings retained). Immediately rebuilt Tier-A matrix104/104. Immediately rebuilt
quick exit2: baseline-invalid solely corpus SHA drift d26ae0a02c13 vs pinned
20c8490591a3; oracle quiescent, oracle/SUT errors0, four stale adjudications.
Pins: target-c-method flip_candidate (Exact5/0/0; 28 extra default candidates),
module-deps and load-repo feature-gated literal pins missing (MCP Prism-only1/4;
load-repo oracle-only resolution_test.rs:5299/5368); ambiguous-symbol ok.
Sampled quick also reports U-free callee Exact1/1/0 and other recall gaps; no
same-environment base quick rerun, so no new-regression or improvement attribution
from these samples. No baseline rewrite/full multicorpus; not a green accuracy gate.

Three-round self-review cap: round1 settled the direct inline-property contract
and RED; round2 added accessor/duplicate/default/shadow/timing and cache replacement
controls (all pass); round3 traced the shared AST receiver path through existing
TypedParam resolution and persistence, with no further in-scope defect found.
Self-pass, not independent review or proof of whole-program runtime immutability.

## Fixed-source evidence

Same five-file Excalidraw archive at 0642e72cfa2d9a71198200e52f37399384610ee3,
saved slice-A base vs rebuilt B candidate, cache bypassed: 2780 identical site
identities, Exact369→372, exactly three changed caller records. They represent
**one source span**, LibraryMenu.tsx:114 library.setLibrary(nextItems), enclosed
by LibraryMenuContent, _onAddToLibrary and addToLibrary. The target is
data/library.ts:351–400, setLibrary, TypedParam Exact. Other records are unchanged;
the other ten tracked unique Library receiver spans remain unproven.

Served exact-only callers adds those three enclosing callables; existing callers
are preserved. Served exact-only callees for addToLibrary changes from empty to
that one setLibrary target, with TypedParam reason and fallback=false. Neither
direction reports warnings or truncation. verify-inline-props.mjs asserts the
paired-record and served-output contracts; its compact result is committed beside
this readout. Earlier Black/Excalidraw/JS control
samples are byte-identical (400/136/12 records; Exact79/26/4). This is a bounded
partial-source gain, not whole-corpus recall or an LSP-oracle adjudication.

## Reproduction and custody

Raw evidence: /private/tmp/prism-inline-props-BC2ce0. Saved base-prism and
candidate-prism; run each with `nav --no-cache call-stats --repo ARCHIVE --dump-sites`.
Compare with measure-indirect-default.mjs, then verify-inline-props.mjs against
that evidence directory. Served probes use `nav --no-cache callers` at
packages/excalidraw/data/library.ts:351 and `callees` at
packages/excalidraw/components/LibraryMenu.tsx:92, both `--confidence exact --format json`.
Existing baselines and unrelated untracked artifacts are preserved.
Raw evidence archive: /private/tmp/prism-inline-props-BC2ce0-evidence.tgz, SHA256
c002c353ec2f8f4963b1a7881ed661ffd32bf4955efc8a7c209212cc814522ec.
