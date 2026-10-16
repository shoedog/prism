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
`Vec<_>`. The graph still uses its original eight serialized fields: this change
has no cache-layout/version change and no claimed resident-memory saving. Compact
storage and a native codec follow in separately gated work.
