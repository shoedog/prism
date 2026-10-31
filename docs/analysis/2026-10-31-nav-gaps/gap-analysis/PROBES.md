# Probe log — gap analysis 2026-10-31

Binary: `<repo>/target/release/prism` (built 2026-10-30 22:00 from main at or near
533f061a; `prism --version` not re-checked). Corpora: `<bench-repos>/{click,flask,gin,axum,express}`
(click 8a1b1a3, flask 36e4a82, gin d75fcd4, axum 485c603, express 18e5985). Every run used
`prism nav --cache-dir <this dir>/cache-<corpus> <query> --repo <corpus>`; the caches were deleted after the
probes (`cache-*` no longer present). No cargo build or test was run.

Raw probe outputs (the `probes/` files named in the first column below) are held in the private evidence store and are not committed. This table is the committed record: each row gives the command and the key numbers the analysis and specs cite. Commands are abridged; the later pilots report the same binary as `slicing 3.2.0 (533f061a942a)`.

| File | Command (abridged) | Key numbers |
|---|---|---|
| `click-repo-map.json` / `.txt` | `repo-map --format json` / text | cold build 1.4 s wall; 63 nodes, 224 edges, 33 KB JSON, 7.5 KB text; warning `93 module import(s) not filesystem-resolved`; node order is sorted path order (`docs/conf.py` first) |
| `click-onboard.md` | `onboard` (CLI only) | 1.5 KB; inventory (63 files, 1668 functions), top-12 modules by degree, call-resolution counters |
| `click-callers-invoke.json` | `callers --symbol invoke` | `AmbiguousSymbol` with 7 candidates, 5 in `src/click/core.py`, 1 in `src/click/testing.py`, 1 in `tests/test_commands.py`; 0 items |
| `click-callees-main-d3.json` | `callees --symbol main --file src/click/core.py --depth 3` | `AmbiguousSymbol` (overload stubs at 1416 and 1427) |
| `click-callees-main-loc-d{1,2,3}.json` | `callees --location src/click/core.py:1430` | 0 items, no error (seed line inside the parameter list; unexplained, not investigated) |
| `click-callees-main-body-d{1,2,3}.json` | `callees --location src/click/core.py:1504` (inside `main` body) | depth 1: 26 items / 17.7 KB; depth 2: 98 / 62.6 KB; depth 3: 188 / 120.6 KB; `Command.invoke` (core.py:1353) appears at depth 1 with score 1.0 |
| `click-callers-ctx-invoke-d2.json` | `callers --location src/click/core.py:853 --depth 2` (Context.invoke) | 5 items; warning: `421 same-name receiver call site(s) with unknown receiver type across multiple owner types; not attributed as callers (src/click/core.py:1367, ...)` — the `ctx.invoke(...)` site in `Command.invoke` is dropped |
| `click-ego-invoke.json` | `ego --location src/click/core.py:1360 --hops 1` (default edges) | 2 nodes, 1 Contains edge |
| `click-ego-invoke-h2.json` | `ego --location core.py:1353 --hops 2 --edges Call,DataFlow,ControlFlow` | 520 nodes, 528 edges (all Call), 305 KB — over the 80 KB MCP view cap |
| `click-ego-stmt-h3.json` | `ego --location core.py:1360 --hops 3 --edges DataFlow,ControlFlow,Contains` | 26 nodes, 27 edges = 23 Contains + 4 DataFlow + 0 ControlFlow, 16.9 KB |
| `click-nodes-at.txt` | `nodes-at --location core.py:1360` | one Variable (`format`) plus enclosing function `invoke`; statements are not surfaced |
| `click-taint-frontier.txt` | `taint-reaches --source core.py:1360` | frontier of 1 item; `SkippedPath: No CFG statement node at src/click/core.py:1360; using pure-taint fallback` |
| `click-dfg-stats.json` | `dfg-stats` | exact 6119, loop_carried 36, killed 1080, sameline 27, cfg_incomplete 5288, alias_unstable 89, call 178; rd_functions_without_cfg 544 |
| `click-dfg-edges.jsonl` | `dfg-stats --edges` | 12,781 labeled DataFlow edges, 3.2 MB JSONL |
| `click-functions.json` | `functions` (CLI only) | 1669 records, 260 KB; 22 names contain `param` (lexical baseline) |
| `flask-callers-index.json`, `flask-callers-create.txt` | `callers --symbol index/create --file examples/tutorial/flaskr/blog.py` | one caller each: synthetic `<module>` at the decorator line, score 0.6, `Resolution { kind: "framework_entry" }`; no route path or method in the record |
| `flask-call-stats.json` | `call-stats` | `framework_entries_recorded` 248, `framework_entry_unresolved_handlers` 0; 4840 sites; grep of `@x.route/get/post(...)` across the repo = 312 (includes docs/tests) |
| `flask-callees-create.txt` | `callees --symbol create --file .../blog.py` | 7 items: `get_db` import_member, `url_for`/`flash`/`render_template` free_single, `redirect` free_multi 0.6, `db.execute`/`db.commit` unresolved sites |
| `gin-onboard.md` | `onboard` | 1.65 s; 98 files, 1288 functions, 203 edges; top-12 modules |
| `gin-interface-manifest.json` | `interface-manifest` (CLI only, Go only) | 428 sites, 145 KB; per-site `dispatch_route`, `receiver_class`, `implementers`; `go_promoted_snapshot` owners 122 / promoted_methods 22 |
| `gin-call-stats.json` | `call-stats` | kind_exact: interface_dispatch 54, embedded_promotion 215, return_typed 166, typed_param 244, same_package 818; `interface_fanout` {1:34, 2:3, 3:1, 5:5, 6:1, 8:3, 13:2, 19:2}; `framework_entries_recorded` 0 |
| `gin-callers-servehttp.txt`, `gin-callers-next.txt` | `callers --symbol ServeHTTP --file gin.go`; `--symbol Next --file context.go` | Exact callers via `return_typed` / `typed_param`; mostly test files |
| `axum-repo-map.json` | `repo-map --format json` | 1.2 s; 302 nodes, 804 edges, 140 KB (about 35k tokens); MCP default would truncate to the first 50 nodes in path order |
| `axum-onboard.md` | `onboard` | 2114 functions; `axum/src/routing/mod.rs` 125 dependents; 97 isolated files; 11,754 sites, Exact 2385, unresolved 5952, dropped_multi_owner 1956 |
| `axum-callers-into_response.json` | `callers --symbol into_response` | `AmbiguousSymbol` with 85 candidates: 32 in `axum-core`, 16 `axum-extra`, 15 `axum-macros`, 11 `axum`, 11 `examples` (no way to ask for all impls of the trait method) |
| `axum-callers-frp.txt` | `callers --symbol from_request_parts --file axum/src/extract/state.rs` | 0 items (trait method reached through generic dispatch) |
| `axum-call-stats.json` | `call-stats` | kind_nameonly includes `trait_cha` 39; kind_exact `qualified_owner` 1424 |
| `axum-functions.json` | `functions` | 2120 records (function_item 2113) |
| `express-call-stats.json` | `call-stats` | `framework_entries_recorded` 0 on the express repo itself (examples register `site.index` member expressions and inline functions; no bare-identifier handler found by grep) |
| `express-callers-site-index.txt` | `callers --symbol index --file examples/route-separation/site.js` | 0 items |

MCP tool-list size (from `tests/fixtures/mcp/golden/*.jsonl`, not a live server): 9 tools, 13,540 JSON chars
(about 3.4k tokens); per tool 714–2,548 chars (`taint_reaches` largest). The tracing trial measured prism's
first-turn overhead at about 5.7k tokens per Claude session (COMPARATIVE §6, private).
