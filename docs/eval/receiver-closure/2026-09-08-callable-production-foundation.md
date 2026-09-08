# S10 — source-backed production integration foundation

This is a read-only foundation for the [authority contract](../../superpowers/specs/2026-09-08-callable-production-authority-contract.md),
not an implemented consumer. The inspected Rust blobs were first bound at S7
`59fd6eb` and rechecked unchanged at S9 `b826a668f43adf9176b01a78132071c1cd7abc88`
on 2026-09-08 at 11:17 UTC. S10 changes none of these files or cache versions.

## Current seams

| Seam | Current source and requirement |
|---|---|
| Lexical receiver and contextual parameter | [ast.rs](../../../src/ast.rs#L3394), contextual type at line 3466; explicit annotations remain terminal, and writes refuse at lines 3370–3390 |
| Receiver classification | [resolution.rs](../../../src/resolution.rs#L835): recovered versus materialized/imported-Props classification; no compiler type-name fallback |
| Existing imported-Props proof | [js_ts_props.rs](../../../src/js_ts_props.rs#L106) collects source-backed proofs, including indexed-source ambient/prototype/Reflect barriers |
| Full/subset installation and lifecycle | [call_graph.rs](../../../src/call_graph.rs#L1836) and line 5100 install imported-Props proofs; line 2297 clears, line 2417 replaces |
| Existing proof consumption | [resolution.rs](../../../src/resolution.rs#L2644) uses exact call-span imported-Props evidence for a typed-parameter direct member, otherwise terminal ExternalReceiver |
| Full/subset call-site construction | [call_graph.rs](../../../src/call_graph.rs#L1726) and line 5018; new facts must reach both within the same epoch |
| Shared resolution | [resolution.rs](../../../src/resolution.rs#L2291), `resolve_call_site_full`; `pre_resolved_target` has distinct ParameterCallback meaning, not compiler-class authority |
| Navigation | [call_resolve.rs](../../../src/navigation/call_resolve.rs#L15), [queries.rs](../../../src/navigation/queries.rs#L241) and line 384 use shared resolution |
| Served navigation | [api/nav.rs](../../../src/api/nav.rs#L14) constructs sessions; callers at line 37 and callees at line 51 expose confidence filtering |
| CPG return flow and trace | [build.rs](../../../src/cpg/build.rs#L1209) requires singleton Exact; [trace.rs](../../../src/cpg/trace.rs#L824) and line 875 consume shared outcomes |
| Persistence | [cpg_cache.rs](../../../src/cpg_cache.rs#L193) version 77; [call_edge_cache.rs](../../../src/navigation/call_edge_cache.rs#L89) version 45; later integration must choose explicit migration/rejection |

Important correction: the receiver classifier's `ImportedProps => materialized`
branch does **not** mean imported Props never recover. The separate collection,
installation and call-span consumption route above already does bounded recovery.
The new route must preserve it and its effect barriers, not duplicate it with names.

`Exact` denotes Prism's static source call-graph confidence. It is not a guarantee
of concrete runtime object identity, dynamic override selection, execution or
reachability. A declaration-file class or printed receiver type is not executable
member ownership. The contract requires a new internal proof constructor and
two-genuine-object substitution/epoch tests before a future consumer can ship.

## Navigation-tool limitation, precisely

Both navigation skills were used for the earlier source-backed planning. Prism's
`nav_repo_map` was available but returned `StaleIndex` and a 50-of-971 truncated map.
It was used only for orientation, not as current caller evidence. LSP tools were
separately unexposed. The locations above were verified directly in source;
there is no claim of all generic/type-resolved references from either tool.

## Rebound source identity

| Source | Git blob |
|---|---|
| src/api/nav.rs | 0d3db0df68fc858683731fb9456b1a46a55b1c62 |
| src/ast.rs | b9e3306370c912cc49ff2e78de99ee3cc8cab60b |
| src/call_graph.rs | 90ac77f07fa7727a079db9eb8f9b3f9262ac707a |
| src/cpg/build.rs | b99ad52b3c33a289eaa36b06453f42dbe6a68fd8 |
| src/cpg/trace.rs | 179fd8b8d67538a536f95b2dbc37e71148084036 |
| src/cpg_cache.rs | b07f3ab5737903b8517a5996762ca0f269193468 |
| src/js_ts_props.rs | 286873e4b09a2b64daa0745b35149c97e704a6af |
| src/navigation/call_edge_cache.rs | c9aa71d60a863a1d82adcab81a84ec236dfb814b |
| src/navigation/call_resolve.rs | 83cd2c3c7b5b2f36240e92e3c604f609bed3eeb0 |
| src/navigation/queries.rs | d482518dba47e2bd79aa93395ea47d53b7a542f8 |
| src/resolution.rs | f7177e971a01c24a8d22d5884efe6bd1cefaa64e |

The primary-owned final authority contract was independently accepted at round
1/2, WRONG 0 / SMELL 0, confidence 98/100. Its RED matrix specifies future work;
it does not claim an implemented proof constructor, receiver gain or new authority.
