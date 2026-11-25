# Route listing

```sh
prism nav routes --repo <dir> [--entry <selector>] [--mounts] [--format json|text]
```

S1b provides the listing and composition core with `enabled_adapters: []`. With no
`--entry`, the query infers from the available adapter evidence and emits the
empty summary in this build. An empty listing carries no evidence of route
absence. Explicit selection requires one grounded app, factory result, URLconf,
or serialized root identity. The query refuses missing, ambiguous and invalid
selections with one `entry_unproven` error line and exit status 2. Use the
equals form for selectors that start with a hyphen, for example
`--entry=-app.js`. A missing argument produces a CLI usage error.

Selectors include Python `module:app`, `module:create_app()` and
`project.urls:urlpatterns`, JavaScript or TypeScript `src/app.ts` for the
default/app export, `src/app.ts#namedApp` for a named export, and a JSON object
with `file`, `binding`, `construction_byte` and `invocations`. Factory setup
uses an initial `{"entry":"module:create_app()"}` frame. Source call frames
contain the file and half-open byte span, in outermost-first order.

JSON output uses one unwrapped route per line, then wrapped mount lines when
requested, then one wrapped summary. Text output uses the record kind followed
by `field=value` pairs. Nested values retain their JSON spelling. Routes are
ordered by first path key or spelling, methods, and source origin. With no
exposures they use the declared literal when available, then the raw spelling.
Mounts follow
source order and alternative/child ordinals. Sites use repo-relative files,
one-based lines and zero-based, half-open UTF-8 byte spans.

## Reading route records

- A router receiver carries its grounded identity. An unproven receiver keeps a
  reason and counts as a candidate. A grounded invalid or opaque element can
  also be a candidate. `registrations = proven + unproven`, and `candidates`
  counts the subset whose candidate flag is true.
- Full exposures have an evidenced root and path. Suffix exposures carry a
  literal tail and a reason such as `no_known_mount` for a detached router or
  `variable_prefix` for an unknown prefix. Dynamic exposures carry a reason
  such as `non_literal_path` or `regex_path`. Each exposure
  carries host and method constraints, a version selector or null, and any
  binding restrictions. For scorable paths, `path_tokens` distinguishes
  literal braces from parameters. `normalized` renders that token sequence.
- Method declarations are sorted unique known strings, `"any"`, an unknown
  expression with its text, or null for omission and WebSocket registrations.
  An explicit empty set is `{"known":[]}`. Independently established defaults
  appear in `default_methods`.
- Resolved and anonymous callback slots include the callable's byte and line
  spans. An unresolved slot includes the written expression and reason. An
  unproven receiver gives every callback slot its refusal reason and clears
  callable and delegate identities. If a decoded callable lacks its written
  argument, the slot uses `<argument unavailable>` and the listing reports a
  `callback_unproven` diagnostic at the declaration. `delegate` holds a
  separately proved forwarding target or null. Hapi
  callbacks may include a pre-handler role and stage. Table records retain the
  complementary element or installation site and ordinal. Identified Hapi
  objects also carry `origin_plugin`, including on candidates.
- Mount edges count in the summary even when hidden. Diagnostics identify a
  source site and reason. A failed root records the rejecting event and blocks
  binding on every exposure for that root.

## How mount and setup events affect exposures

Live mounts see registrations installed later. Snapshots read the source at
the copy event, including nested copies. Deferred events contribute at a
proved activation point. A cycle stops at the repeated instance with
`mount_cycle`. Each source origin can retain 64 distinct exposures. A result
or traversal limit adds an `expansion_truncated` marker, a suffix for a
literal seed or a dynamic exposure for a dynamic seed. The mount listing
retains all recorded edges.

A conditional exposure carries `setup_guard` and earns no unconditional
schema credit. An overwrite adds `superseded_keys` to the earlier source
record. Uncertain installation can leave a `dispatch_conflict` suffix.
Installed alternatives with uncertain dispatch keep their full paths and
block the contested methods on potentially colliding keys. A proved
rejection blocks every binding for its attributed setup root, inferred
from the dispatcher when the dispatcher is itself a selected root. Without
either attribution the listing reports a diagnostic and fails closed on
every selected root. In both cases affected exposures carry
`binding_blocked: "any"`. Source records and written callbacks survive
these cuts. An impossible chain or a `build_only` declaration has no
exposure through that chain or declaration.

The route, mount and callback examples below use synthetic identities to
illustrate the wire contract. The final two lines illustrate the outputs
available in the S1b build.

```jsonl
{"framework":"express","protocol":"http","site":{"file":"app.js","line":4,"start_byte":50,"end_byte":75},"receiver":{"status":"router","file":"app.js","binding":"app","construction_byte":12,"invocations":[]},"methods":{"known":["GET"]},"path":{"raw":"'/items/:id'","literal":"/items/:id"},"paths":[{"root":{"file":"app.js","binding":"app","construction_byte":12,"invocations":[]},"status":"full","value":"/items/:id","path_tokens":[["literal","/items/"],["parameter"]],"normalized":"/items/{}","host_kind":"unconstrained","hosts":[],"method_guard":"any","version":null}],"callbacks":[{"position":0,"status":"resolved","file":"app.js","name":"show","start_line":2,"end_line":2,"start_byte":22,"end_byte":47,"delegate":null}]}
{"framework":"express","protocol":"http","site":{"file":"app.js","line":5,"start_byte":76,"end_byte":113},"receiver":{"status":"router","file":"app.js","binding":"child","construction_byte":0,"invocations":[]},"methods":{"known":["GET"]},"path":{"raw":"'/ready'","literal":"/ready"},"paths":[{"root":null,"status":"suffix","value":"/ready","reason":"no_known_mount","path_tokens":[["literal","/ready"]],"normalized":"/ready","host_kind":"unconstrained","hosts":[],"method_guard":"any","version":null}],"callbacks":[{"position":0,"status":"anonymous","file":"app.js","name":"<anon@5:20>","start_line":5,"end_line":5,"start_byte":96,"end_byte":112,"delegate":null}]}
{"framework":"flask","protocol":"http","site":{"file":"app.py","line":8,"start_byte":110,"end_byte":130},"receiver":{"status":"unproven","reason":"receiver_unproven"},"candidate":true,"methods":null,"path":{"raw":"'/x'","literal":"/x"},"paths":[],"callbacks":[{"position":0,"status":"unresolved","text":"status","reason":"receiver_unproven","delegate":null}]}
{"framework":"express","protocol":"http","site":{"file":"app.js","line":6,"start_byte":114,"end_byte":150},"receiver":{"status":"router","file":"app.js","binding":"app","construction_byte":12,"invocations":[]},"methods":{"known":["GET"]},"path":{"raw":"config.path","literal":null},"paths":[{"root":{"file":"app.js","binding":"app","construction_byte":12,"invocations":[]},"status":"dynamic","reason":"non_literal_path","path_tokens":null,"normalized":null,"host_kind":"unconstrained","hosts":[],"method_guard":"any","version":null}],"callbacks":[{"position":0,"status":"unresolved","text":"views.show","reason":"member_handler","delegate":null}]}
{"mount":{"parent":{"status":"router","file":"app.js","binding":"app","construction_byte":12,"invocations":[]},"child":{"status":"router","file":"app.js","binding":"child","construction_byte":0,"invocations":[]},"site":{"file":"app.js","line":7,"start_byte":151,"end_byte":174},"prefix":{"status":"literal","value":"/api"},"alternative_ordinal":0,"child_ordinal":0}}
{"mount":{"parent":{"status":"router","file":"app.js","binding":"app","construction_byte":12,"invocations":[]},"child":{"status":"unproven","reason":"mount_child_unproven"},"site":{"file":"app.js","line":8,"start_byte":175,"end_byte":193},"prefix":{"status":"absent"},"alternative_ordinal":0,"child_ordinal":0}}
{"mount":{"parent":{"status":"router","file":"app.js","binding":"app","construction_byte":12,"invocations":[]},"child":{"status":"router","file":"app.js","binding":"child","construction_byte":0,"invocations":[]},"site":{"file":"app.js","line":9,"start_byte":194,"end_byte":222},"prefix":{"status":"unevaluated","text":"config.prefix"},"alternative_ordinal":0,"child_ordinal":0}}
{"summary":{"enabled_adapters":[],"registrations":0,"proven":0,"unproven":0,"candidates":0,"mounts":0,"diagnostics":[],"failed_roots":[]}}
{"error":{"reason":"entry_unproven","entry":"app.py#missing"}}
```

The [route-model design](../../../superpowers/plans/2026-11-20-lanes/lane12-route-model/DESIGN.md)
defines the full field presence rules and the closed reason enum in sections
5.1 and 5.4.

Consumers can call `framework_entries::routes::routes` with parsed files, the
call graph, immutable manifest facts and an optional `RootSelector`. The
returned `RouteTable` exposes typed path tokens, roots, constraints, callback
proofs, source and mount identities, order and summary counts.
