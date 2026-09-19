# Slice 2 bounded design decision

**Recommend B: correct the acceptance contract to exact occurrence materialization and genuine argument-Use → callee-parameter → callee-body connectivity. Keep caller reaching-definition occurrence identity as an explicit successor.** Continue the existing partial artifact after controller amendment; do not change `VarLocation` identity, producer/RD algorithms or public APIs.

This is a design review before candidate review, consuming no candidate round. Source reviewed only at accepted base `d2bbe7074d12fc98280248313d9767d237be33b8`; mutable implementation was not inspected. No source edited or full tests run.

## Mechanism-level boundary

The six-fixture discovery is useful corroboration, but it does not by itself prove impossibility. The pinned code gives the stronger, narrowly scoped explanation:

1. `src/data_flow.rs:33–45` defines `VarLocation` Eq/Ord/Hash without start/end bytes. `labels` is keyed by pairs of these locations (`:121–125`), and forward/backward maps also use that identity (`:119–122`, `:653–666`). Thus those maps cannot independently identify two same-file/function/function-start/line/path/kind occurrences. A `labels.get` or `backward.get` using a later raw location can return the earlier representative's entry; such a lookup is **not** evidence of an incoming edge to the later bytes.
2. Byte-free map identity alone would not prove absence of a later edge, because `DataFlowGraph.edges` is a Vec containing endpoint payload bytes. The decisive additional mechanism is `preferred_use_locs` at `src/data_flow.rs:455–473`: rvalue spans are appended to raw `uses`, but the `(AccessPath,line)` preferred map retains only the first location with `or_insert`.
3. The `get_use` closure at `:479–505` always returns that preferred location when it exists; only a missing key can produce a conservative zero-width anchor. All three producer branches minting intraprocedural edge targets use `get_use`: parameter references (`:508–519`), ordinary L-values (`:524–553`), and alias-resolved L-values (`:559–584`). No branch selects a later byte-distinct member of the same key.
4. Reaching definitions at `src/cpg/reaching.rs:224` classify the supplied edge vector; they do not create a new byte-distinct target. The supplied-edge-only invariant is separately asserted at `src/cpg/reaching/tests/limits.rs:153`. Label combination at `src/data_flow.rs:205–214` takes the worst confidence for colliding endpoint identities and cannot recover their missing distinction.
5. Base CPG materialization at `src/cpg/build.rs:679–737` traverses the raw Def/Use vectors in order and retains the first legacy key, matching the preferred producer location for a unique callable. Consequently the later raw Use omitted from CPG in the targeted same-key collision is also not the target chosen by the producer's `get_use` path. Step4 cannot legitimately invent an incoming edge to it.

**Scope of this conclusion:** independently indexed, identity-unambiguous callable; byte-distinct nonzero Uses sharing the exact legacy file/owner/start-line/source-line/path/access key targeted by this slice. It is not a claim about every JavaScript fixture, alternate producers, distinct paths/lines/owners, hand-constructed DFGs, or unsupported same-name owner collisions. Those do not supply an honest alternate O01 positive for this precise collapse defect.

The six cases in `slice2/base-cache/authoritative-later-use-result.md` and its raw log agree with this mechanism. In O01, raw Uses are `[58–63,77–82,77–82]`, legacy CPG retains 58–63, and the real incoming DFG edge is 46–51 → 58–63 with `NameOnly(CfgIncomplete)`. There is no stored edge endpoint at 77–82. Checking actual `FlowEdge.to` bytes is essential; byte-free map lookup would mislead.

## Why B is safe and useful

The raw later Use is genuine source occurrence evidence even though caller reaching-definition connectivity is missing. Preserving its node, registering it in Contains/location indexes and binding it to the correct positional callee parameter makes a real graph relationship available from that occurrence. The callee's existing parameter-to-body edge provides non-vacuous downstream connectivity with its original label. This repairs the authorized CPG/index/argument-binding bottleneck without pretending that upstream RD was repaired.

The graph has a public NodeIndex-based traversal seam, `CodePropertyGraph::reachable_forward` at `src/cpg/query.rs:143`, and public `nodes_at`/node inspection at `:52–64`. An independently located exact argument node can therefore reach the callee body through real DataFlow edges. Legacy `var_node_for_location` and DFG-style reachability remain line-based (`:105`, `:672–686`); their first-wins behavior must remain explicit. Do not claim a byte-distinct public VarLocation API or a caller-def-to-later-read taint path.

Callee resolution Exact authorizes the argument boundary only. It does not upgrade `NameOnly(CfgIncomplete)` on the callee's existing body edge or imply Exact end-to-end flow. CPG Step4 still copies only actual stored DFG endpoints and original labels; no edge fanout, relabeling or synthetic missing edge is permitted.

A prerequisite redesign (C) is unnecessary for this limited structural and downstream gain. It becomes necessary if the owner requires exact caller-def → every later same-line Use connectivity, byte-distinct DFG adjacency/label identities, or complete occurrence-aware legacy query semantics in this same slice. Those are not authorized here.

## Exact replacement acceptance (controller should amend before implementation resumes)

Replace the original O01 end-to-end paragraph and its incoming-later-edge obligation with the following:

> O01 must prove the complete bounded path **from the recovered source argument occurrence through its correct callee parameter to an already-produced callee body Use** using genuine DataFlow edges. Separately preserve and report the caller's actual DFG endpoint/label population, including the absence of an incoming DFG edge to the recovered later occurrence when the producer selects only the earlier representative. No synthetic caller-definition edge, fanout, reaching-definition redesign or confidence upgrade is allowed. A node with only a newly minted boundary edge and no validated downstream callee body evidence is insufficient. End-to-end here starts at the exact argument Use; it does not claim caller-definition reachability.

### Required fixed O01 oracle

Use these unchanged sources, zero-error parsed in JS/TS/TSX:

```js
// app: no comment in actual fixture bytes
import {item} from './origin';
function outer(value){sink(value);return item(value);}
```

```js
export function item(input){return input;}
```

Independently authenticate AST arguments/parameters and UTF-8 source slices, then assert exact primitive-field tuples, preserving multiplicity:

1. Caller parameter Def `value` `[46,51)`, earlier sink Use `[58,63)`, later item Use `[77,82)`. Raw duplicated 77–82 producer rows deduplicate to **one** CPG node; the two distinct Uses remain separate. Base raw population and missing later CPG node/boundary establish behavioral RED using base-existing APIs.
2. Caller raw `DFG.edges` retains the actual 46–51 → 58–63 endpoint/label relation and **no** edge whose target bytes are 77–82. Compare field-by-field extracted tuples including both byte ranges, not VarLocation Eq, BTreeSet<VarLocation>, labels lookup with a manufactured later key, or byte-free adjacency. Assert Step4's matching CPG edge remains `NameOnly(CfgIncomplete)` and does not fan out to 77–82.
3. Candidate boundary is exactly `(app,outer,2,value,Use,77,82)` → `(origin,item,1,input,Def,21,26)` at **parameter ordinal 0**, labeled by the retained call-resolution confidence (Exact for this fixture). Earlier Use58–63 must have no boundary to that parameter. Require exactly one edge for this relation.
4. Existing callee body relation is `(origin,item,1,input,Def,21,26)` → `(origin,item,1,input,Use,35,40)` with its authenticated base label, `NameOnly(CfgIncomplete)` in the fixed accepted-base fixture. Preserve it exactly. A DataFlow-only traversal from the recovered argument NodeIndex must reach both parameter and body Use. The combined two-hop label is NameOnly(CfgIncomplete), never described as fully Exact. Assert both edges and multiplicity explicitly; do not infer connectivity from node presence.
5. Select the recovered NodeIndex by independently asserted payload bytes/owner/access/path. Verify one Contains edge and one location registration, both after actual warm cache reload. An unrestricted line lookup cannot be used as the exact-argument seed.
6. Keep a separate baseline-Exact DFG-label preservation control, independently observed on an existing supported fixture before source edits. It need not have a collapsed later Use and must not be presented as evidence of recovered caller RD. A direct private Step4 injection/control can test exact-endpoint routing, but it cannot replace O01's real-source downstream path.

### Retain the rest of the contract

- O02–O04 still require both call/slot occurrences, trivia/Unicode and dialect coverage with no cross-position binding.
- O05/O09 still preserve all authoritative Def/Use occurrences and deduplicate identical rows. Step4 maps **stored** nonzero DFG endpoints exactly; do not make every newly materialized occurrence a DFG endpoint.
- O06–O12 retain full-member/base, forged-payload/ambiguity, parameter/default/rest/refusal, zero-width and owner-collision controls.
- O13/O14 include actual CPG cache plus complete node/edge/Contains/location/legacy query parity and source epochs. Preserve the upstream-absence fact through every build path.
- O15 genuine v94 cache provenance, all required gates and cost/size measurements remain mandatory. CPG95 is still justified by persisted occurrence semantics; no RD/VarLocation identity change or speculative navigation version bump follows from this correction.

## Handoff decision

Adopt this single narrow spec correction, preserve the current partial artifact and saved RED, then resume only CPG occurrence/index/assembly work. Explicitly label the outcome **exact occurrence and argument-boundary recovery, with caller RD limited by the existing line-key producer**. Track byte-distinct DFG/RD identity as a separate successor requiring its own design, consumers, labels, cache and compatibility review.

No candidate correctness verdict is issued here; all implementation claims await the ordinary frozen-candidate review with its fresh two-round cap.
