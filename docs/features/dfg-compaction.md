# DFG ordered views

`DataFlowGraph` exposes read-only ordered views instead of mutable public storage
fields. Graph constructors and traversal/query signatures remain unchanged.

- Use `edges().iter()` for borrowed `FlowEdgeRef` endpoints; `to_owned()` produces
  an owned `FlowEdge`. `edges().len()` and `edges().is_empty()` preserve edge counts.
- Use `defs()`, `uses()`, `forward()` and `backward()` for ordered key/location rows.
  Each row's `Locations` supports iteration and an explicit `to_vec()` snapshot.
  Only `defs()` and `uses()` can contain existing empty rows.
- Use `successors(at)` and `predecessors(at)` for legacy-identity adjacency lookup.
  An absent key returns `None`.
- Use `label(from, to)` for an optional copied confidence, `labels()` for ordered
  endpoint/confidence rows, and `rd_function_stats()` for per-file RD counters.

Iteration preserves native legacy ordering, duplicate neighbors and the original
byte-bearing representative of every map key. Byte spans remain present even
where legacy location equality ignores them. Views borrow graph records; they do
not retain expanded graph copies. Internal supplemental exact labels and mutation
hooks are not public API.

This is a source migration for direct-field library consumers. To serialize a
view as the old edge sequence, collect `edges().iter().map(|e| e.to_owned())` into a
`Vec<_>`. Production now retains a compact arena of complete exact locations, ordered IDs
and legacy rank indexes. Views resolve borrowed locations without per-row boxes,
including `forward_reachable` and `all_locations_at` scans. Cache v122 encodes this
layout directly and rejects incompatible versions before payload decoding; the
navigation sidecar remains at main's v67. One cold rebuild is expected after upgrade.
No old-cache importer or expanded production side copy remains. Query and output
behavior is unchanged. Controller memory/load/peak acceptance passed on activation
`3739ab38`; see [the measured readout](../eval/dfg-compaction/acceptance.md).
The merge of main retains that activation and incorporates main's module-owner
format change; the combined CPG cache version is 122.

The temporary migration oracle is retired after acceptance. Ordinary default/MCP
tests retain explicit representative and ordering checks, native cache roundtrips,
renderer/dispatch controls and all 15 lane mutation witnesses. This retirement
changes no production query body, storage layout or cache version.
