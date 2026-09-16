# Round 1 repair hypothesis log

## Exact-16 classification probe 1

- Hypothesis: all 16 candidate-only failures are ownership-observation changes caused by the new nearest-callable filter, rather than receiver-resolution regressions.
- Expected if true: the missing calls fall into three explicit categories: nested calls previously duplicated onto an outer callable, calls whose nearest anonymous callable has no graph identity, and calls whose nearest callable is conservatively refused because its syntax is ambiguous. Any retained call site still has its prior receiver metadata and resolution.
- Falsifier: a call with an indexed, unambiguous nearest callable disappears; or a retained call changes receiver binding, recovery, or exact resolution.
- Alternative: the owner filter is too broad, especially its whole-owner `has_error()` check, so calls outside an error subtree are incorrectly removed.
- Separating observation: enumerate every failing fixture across every language and full/subset route, recording the nearest callable, whether that callable has a graph identity, whether the call or its direct ancestry crosses an `ERROR`/missing node, and retained-site resolution metadata.

## Recovery policy probe 2

- Hypothesis: refusing every call whenever `owner.has_error()` is too broad; only calls whose path to the owner crosses recovered/error syntax are ambiguous.
- Expected if true: `outer`'s `api.m()` in `function outer() { function broken() { let = ; } api.m(); }` has a clean ancestry path and should remain owned, while `recovered`'s call in the same statement region as `let = ;` needs conservative refusal under the existing recovery guard.
- Falsifier: the clean `outer` call's ancestry crosses an error or missing node, or tree-sitter cannot distinguish its bounds from the recovered region.
- Alternative: any error anywhere in a callable invalidates all callable bounds, making whole-owner refusal necessary.
- Separating observation: dump the syntax-node ancestry and sibling error placement for both calls without editing production behavior.

## Results

- Probe 2 falsified whole-owner refusal. Both `api.m()` nodes and their direct `expression_statement` parents have `has_error=false`; the recovered `ERROR` nodes are siblings. Removing only the `owner.has_error()` gate restored all seven recovery-related inline-receiver failures and both TypeScript recovery controls while leaving nearest-callable ancestry active. Raw output: `recovery-ancestry-probe.log`.
- Probe 1 was partly falsified by that production bug, then supported for the remaining ownership deltas. The 16 failures classify into: nine restored by the bounded recovery fix; two inline fixtures whose nearest arrow/generator callable is unindexed and therefore produce zero sites; two receiver fixtures whose nearest anonymous nested callable is unindexed and produce zero sites; four receiver fixtures whose formerly duplicated outer site is removed while the indexed nearest-owner site and its resolution metadata remain; and one TSX fixture whose `fetchUser` call is inside an unindexed effect callback while direct `fetch` remains under the indexed `fetchUser` owner. The receiver groups overlap the six receiver test names and account for all of them.
- Complete post-reconciliation probes: inline receiver module 26 passed / 0 failed; receiver-self module 36 passed / 0 failed; the TSX regular-call test 1 passed / 0 failed; the two TypeScript recovery tests each passed individually. No retained site changed its expected exact receiver resolution in these probes.

## Genuine old-cache probe 3

- Hypothesis: unchanged base writes CPG schema 93 and navigation schema 52 bytes that the candidate rejects and rebuilds as 94/53, after which cached and uncached candidate navigation endpoints match.
- Expected if true: base writer succeeds and produces both cache layers; candidate load changes the old byte hashes and schema markers; the rebuilt caller rows are exactly `inner` and `direct`, excluding `outer`, and equal an uncached candidate build.
- Falsifier: base does not write both cache layers, candidate reuses either old artifact without rebuilding, or rebuilt cached endpoints differ from uncached candidate endpoints.
- Alternative: a path/key mismatch causes a cache miss and fresh build without exercising rejection of the base-produced artifact.
- Separating observation: preserve the exact base repo path, cache root, file inventory and hashes; invoke candidate against the same paths; compare pre/post bytes and endpoint vectors.

## Graph metadata probe 4

- Hypothesis: nearest-owner filtering changes only caller ownership; retained outer eager/direct and inner calls keep complete CallSite metadata, resolution, and CPG Call/Return/DataFlow endpoints across JS/TS/TSX, full/skeleton/subset, serialization, and incremental rebuilds.
- Expected if true: every graph producer emits identical normalized site rows; only `inner -> item` resolves; its Call/Return pair and data-flow tuples have identical multiplicity after bincode and incremental rebuild.
- Falsifier: any retained site changes qualifier, receiver, arity, origin, span, resolution kind/confidence, endpoint label, or multiplicity; or any producer retains `outer -> item`.
- Alternative: name-only equality masks a metadata or endpoint regression.
- Separating observation: normalize every relevant field and graph endpoint, print the first candidate fixture once to derive fixed source-anchored literals, then pin those literals rather than deriving expectations from the ownership walker.

## Results for probes 3 and 4

- Probe 3 supported compatibility refusal and endpoint parity. Base wrote valid 93/52 artifacts whose hashes stayed unchanged on a second base load. Candidate logged the 93/94 mismatch, rebuilt both layers as 94/53, and cached callers equaled uncached callers. Build identity also differed, so the genuine run does not isolate version as the only rejection cause; the separate forced-version tests do.
- Probe 4 found one established metadata asymmetry: skeleton sites intentionally leave `arg_count=None`, while full/subset preserve `Some(1/0/0/2)`. After pinning each route's own expected rows, all three dialects passed with identical remaining metadata, exact `inner -> item` resolution, and fixed Call/Return/DataFlow endpoints. Bincode call-graph parity, caller-only incremental epochs, cache parity, and the existing build-determinism oracle also passed.

## Receiver retained-owner probe 5

- Hypothesis: the four receiver fixtures with count 2 to 1 deltas retain only the nearest indexed callable, with identical caller identity and call span in full/subset and every supported dialect.
- Expected if true: each route emits one fixed `(caller name, caller lines, call byte span)` row and no outer caller row.
- Falsifier: a surviving site belongs to the former outer owner, differs between full/subset or dialects, or has a shifted span.
- Alternative: count and resolution assertions pass while the wrong caller owns the remaining site.
- Separating observation: print caller identities and spans once, then replace the print-only probe with fixed source-anchored tuples for the four authorized fixtures.

## Result for probe 5

- Probe 5 supported the nearest-owner hypothesis and ruled out the count-only alternative. Every full/subset route retained exactly the fixed nearest caller and byte span: `ns` at lines 2-2 and bytes 73-87 for the nested declaration, `ns` at lines 2-2 and bytes 100-114 for the outer-parameter fixture, `client` at lines 2-2 and bytes 133-151 for the typed outer-parameter fixture, and `client` at lines 2-2 and bytes 140-158 for the typed named-shadow fixture. No route retained the former outer `run` owner. The fixed assertions pass across JavaScript, TypeScript, and TSX where applicable: 36 passed / 0 failed / 0 ignored in the receiver-self module.
