# Constructor-backed own-field receiver ownership

Base main1e26301, after PR247 merged. Owner approved the recommended bounded
constructor-field continuation. Verification complete; publication pending.

## Result and boundary

JS/TS/TSX `this.field.m()` can use one direct own-constructor assignment
`this.field = new C(...)` and existing runtime constructor/import/class identity.
Shared AST evidence and receiver classification reuse FieldTyped, with no new
serialized field or resolution rung. CPG68/nav37 invalidate prior evidence.

One optional plain uninitialized public field; classes with heritage require
that own declaration to avoid inherited accessor interception. Indexed field
annotations are permitted but supply no owner authority. Normal instance methods,
own arrow members and lexical arrow captures are supported. Calls directly in the
constructor require completed initialization before their source span.

Barriers cover static/dynamic-this contexts, explicit this parameters, duplicate,
computed, accessor, initialized, optional, nonpublic and decorated slots, multiple
constructors, returns, non-direct/conditional assignments, shadowed constructor
bindings, and whole-class syntactic field/member writes. Assignment/update/delete,
destructuring, loop and recognized direct Object/Reflect mutator forms are tested.
Unrelated reads/writes, static unrelated slots and nested function returns remain
positive controls.

This is a lexical constructor invariant, not whole-program temporal/alias safety.
No transitive early-this escape/reentrancy proof, arbitrary external mutation,
dynamic rebinding/reflection, inherited-field or subclass override inference.
Source declarations assume standard define-field semantics, not legacy TS
assignment-only emission. Real App calls other this methods before its library
assignment; we have not established that those cannot reenter instance methods.
React.FC/hook-return and general returned receivers remain separate work.

## Verification and refutation

Merged-base same-environment focused RED:2pass/1fail, positive has no Exact owner.
Initial green3/0. First build's two iterator lifetime errors were compile-only,
not behavioral evidence. Three-round self-review cap completed without extension:

1. Shared classifier/evidence and full/direct-subset owner-decoy assertions.
2. WRONG: `Object.assign((this), other)` and `Object['assign'](this, other)`
   retained Exact Client.m despite overwriting the receiver. Complete probe2/1
   enumerated both; ordinary parenthesized assignments passed their barriers.
   Normalize reflective forms; round2 green and cache checks pass.
3. WRONG: inherited getter/setter could intercept `this.client = new Client()`
   without an own slot, yet Client.m was Exact. Complete probe2/1; own-field
   control passed. Require an own declaration for heritage; final3/0.

These are defects caught in this increment, not attributed to merged slices.
Constructor positive↔conditional/write-negative persisted transitions,
A→B/B→A caller-side constructor-owner replacement and navigation sidecar round
trips pass alongside existing Python/JS/TS controls. CPG67 cache refusal tested.

Full default3821/0/1 and MCP4011/0/1. Existing ignored test:
`resolution_test::slice_elem_variant_reserved`. Clippy completed with warnings
retained; fmt/diff checks clean. Immediate release-build matrix104/104.
Immediately rebuilt quick exit2: baseline-invalid solely corpus SHA drift
1e26301525f4 vs pinned20c8490591a3. Oracle quiescent; oracle/SUT errors0;
stale adjudications4. No green quick gate, full multicorpus or baseline rewrite.
Self-pass, not an independent review.

Pins: target-c-method flip_candidate, supplementary Exact5/0/0, 28 extra default
candidates; module-deps-feature-gated missing literal pin, oracle-only empty and
one MCP Prism-only; load-repo-feature-gated missing literal pin, oracle-only
resolution_test.rs:5299/5368 and four MCP Prism-only; ambiguous-symbol ok.
Sampled U-free callee Exact1/1/0 and recall gaps remain reported. No same-environment
base quick rerun was performed, so these are not attributed as regressions or
fixes from this slice. Detailed sampled counts and pin site lists remain in the
archived JSON/Markdown quick reports.

## Fixed-source and served measurement

Saved merged-base release and candidate on the same five-file Excalidraw archive
SHA0642e72cfa2d9a71198200e52f37399384610ee3:2780 stable site identities,
Exact372→376, exactly four changed records at four unique source spans:

| App.tsx line | Caller | Library target |
|---:|---|---|
| 2929 | initializeScene | updateLibrary:287 |
| 3224 | componentWillUnmount | destroy:249 |
| 12113 | handleAppOnDrop | getLatestLibrary:268 |
| 12239 | loadFileToCanvas | updateLibrary:287 |

Each changes NameOnly/R6 single-owner to Exact/FieldTyped with the same correct
declaration identity. All four served caller gains and four served callee gains
are asserted, with no removed items, fallback, warnings or truncation. This is
a source-backed sample gain, not an oracle-derived corpus-wide recall estimate.

Of eleven tracked unique Library spans, the existing LibraryMenu114 Exact span
and remaining six unproven spans are unchanged. Earlier small Black, Excalidraw
and JavaScript controls are byte-identical. `verify-constructor-fields.mjs`
asserts these claims; compact result committed alongside this readout.

## Custody and reproduction

Evidence root `/private/tmp/prism-constructor-fields-NWyrfq`: saved base-prism and
candidate-prism, paired JSONL, served JSON, focused/cache/full-suite/gate logs.
Source archive `/private/tmp/prism-indirect-default-VUbv13/excalidraw` is unchanged.
Run measure-indirect-default.mjs with the paired excalidraw JSONL and source root,
then verify-constructor-fields.mjs with the evidence directory.
Raw archive (including fixed source):
`/private/tmp/prism-constructor-fields-NWyrfq-evidence.tgz`, SHA256
`ee7ef38aadf99878d17bcb746825b1d5096c0bb1de1dbb96e57b62958c984bd7`.
Only this run's untracked quick reports and prism-1e26301525f4.json snapshot were
moved into that evidence directory; all remain recoverable. Original
.superpowers/ and eval/snapshots/prism-fb81481dafa7.json remain untouched.
