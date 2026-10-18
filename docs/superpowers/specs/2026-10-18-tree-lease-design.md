# Tree ownership, leases, and language split — T3a (#3)

Design only · 2026-10-18 · `plan/tree-lease-design` · inspected HEAD `f0ce8b6aa0cce571068de9d51067fafdb6250dcf` (requested main base). Controller commits. No implementation, build, index load, runtime measurement, or network work was performed.

## 1. Decision and boundaries

| Decision | Contract |
|---|---|
| Recommended | Keep public eager `ParsedFile`, `LoadedRepo`, `NavigationSession` and MCP library handles intact. Add a private runtime snapshot and closure-scoped, zero-idle tree access for the built-in MCP runtime. Share query/build cores through internal adapters, with public eager wrappers. |
| Target | **<1 GiB live heap per cache-hit index**, separately on excalidraw/prometheus/ruff/C0 Python winner; final host check is 12–16 named checkouts on 24 GiB, serialized heavy refreshes, measured headroom and no swap growth. |
| Inherited opportunity | Owner/round-2 plan §1: prometheus 317 MiB tree allocation / 1,210 MiB hit heap; historical slicing 419 / 1,252. Arithmetic ceilings leave 893 / 833 MiB before lease/helper overhead. These are not new measurements or guaranteed savings; slicing is not the Python acceptance corpus. Prometheus whole-repo read+parse receipt is 0.4–0.5 s, not a per-query latency bound. |
| Authority | `docs/superpowers/plans/2026-10-18-optimization-round2.md` §§1,3–7 (S9/S13, T3a–d). S0 precedes implementation; #3 acceptance precedes #10. Public API, zero idle trees, measured reparse cost and bounded residency are settled. |
| Bound | Final private runtime admits at most **K simultaneous parsed trees across its query, loader, provider, builder and verifier work**, summed across languages and overlapping generations in one process. External eager library objects are explicitly outside this private-runtime guarantee. No hidden eager fallback in an activated runtime. |
| Scope discipline | Preparatory PRs may retain eager behavior, make no memory/bound claim and do not activate eviction. If construction adapters cannot meet the bound in reviewable slices, park activation; do not quietly exempt cold construction or verification. Cold high-water reduction is not promised. |

## 2. Inventory and current execution paths

The census below covers Rust `src/`, `tests/`, `examples/`, and `benches/` at the pinned HEAD. It includes elided `Node`, fully qualified types, inferred nodes from tree/node-returning helpers, and node-bearing context types; `petgraph::NodeIndex` and graph `enclosing_function` are excluded. A function is counted once even if it has many accesses; nested functions count separately, closures belong to their enclosing function. Imports and field/type declarations are listed separately. Tests/oracles/examples retain the eager path.

Evidence (controller snapshots it with the design): `~/prism-evidence/mcp-memory/optimization-round2/t3a/{consumer-ledger.tsv,census.json,tree-node-raw.txt,census.py}`. The JSON records names, complete signatures, start/end lines, direct-tree/explicit-node/returned-node flags, inferred helper calls and test status. The compact table records **every selected function declaration line**, not line ranges or representative examples; resolve each integer against its file. `T` lists every `.tree`/`root_node` source access line (including tests); `F` is production function count, `t` is test/oracle declarations. Declaration lines and access-site counts are different units. This is static source enumeration, not an executed call graph or a runtime profile.

| Family | Production functions | Runs during construction? | Runs after index build? / slicing consequence |
|---|---:|---|---|
| A — Shared AST/language helpers | 543 | Yes | Yes through public ParsedFile operations and internal reasoning; keep helpers inside one per-file lease, not one lease per helper. |
| C — Construction / CPG / scope and owner construction | 188 | Yes | Construction work can also be invoked by refresh/review; not an ordinary nav tree query. Split graph phases and language fact extraction. |
| P — Providers / registry / receiver typing | 153 | Yes, including cache hits | Provider lookups use owned type data; AST extraction and receiver classification are build paths. Public node-context helpers remain eager-compatible. |
| N — Navigation reasoning / spans / evidence | 19 | Some shared helpers | Yes: seed nodes, scope warnings, sanitizer transitions. Spans are an additional owned-result boundary below. |
| M — MCP views | 0 | No direct node consumer | Source-only snippets; retain source adapters, no gratuitous parse. |
| R — Review / algorithms / frameworks / targets | 404 | Some shared framework extraction | Yes: built review contexts, public algorithm calls and MCP sanitizer advice. Migrate only MCP-reachable boundaries; other public algorithms stay eager. |
| F — Refresh / verifier / loader ambient parse | 1 | Yes | Refresh is construction during service, not a stale-query disk read. Loader and verifier must use the same bounded admission. |
| K — Cache | 0 | No Tree/Node payload | CPG and sidecar decode do not restore trees; exact-hit provider construction still parses. Global Query cache is distinct from trees. |

Total: **1308 production functions**; **466 test/oracle functions** and **14 non-test example functions**. Phase labels describe the family, with the precise entry boundaries below; A includes construction-only parse/table preparation (`ast.rs:49,56,478,809`), while its reusable public operations can serve post-build calls; shared public AST helpers are callable after construction even when their current internal caller is a builder.

| Current boundary (not extra direct-consumer counts) | Source and classification |
|---|---|
| Loader / accepted file set | `src/repo_loader.rs:64,124,144,187,221`: parallel eager parse, >0.3 error-rate rejection, hashes, manifests and JS resolver present/absent closure. Build/refresh only; preserve all acceptance rules. |
| Cache hit / types | `src/navigation/cache.rs:70`; `src/cpg/context.rs:138,169,337`: graph cache hit still constructs type providers and scans live types. Provider objects own facts, not nodes. |
| Span query | `src/navigation/queries.rs:1785,1822` → `src/ast.rs:701,675`: `symbol_spans` returns owned coordinates but reconstructs a node; definitely serves after build. |
| MCP reasoning | `src/mcp/tools_reasoning.rs:41`; `src/reasoning/taint_reaches.rs:40,577` → seeds, scope honesty, sanitizer walk, `algorithms/taint.rs:10634,10739,10823`. Do not call all taint code construction-only. |
| Source-only evidence and order | `src/mcp/evidence_view.rs:725,740,765` reads retained text; `src/reasoning/order.rs:24` uses CPG byte order and file presence. No tree needed at these boundaries. |
| Deferred sidecar | `src/navigation/mod.rs:266,340`: lazy call-edge build reads stored call-graph facts; it has no repo/tree argument. Keep owned facts sufficient for cold/missing/corrupt sidecar rebuild after eviction. |
| Refresh | `src/mcp/session.rs:263,361,384,403,429`: candidate build/verification before publication; full verifier calls `load_repo`. Two candidate attempts and retained active generation must share the budget. |
| Cache persistence | `src/cpg_cache.rs:274,543`; `src/navigation/call_edge_cache.rs:110`; `src/queries.rs:31,34`: no serialized Tree/Node; compiled queries are process-global and presently initialized for all languages. No schema bump for ownership alone (current CPG/sidecar 123/68; rebind parent). |

### Complete compact function/access ledger

`src/` omitted from source file names below. A/C/P/N/R/F identify the families above; X is examples/integration tests. In a cell, `–` means zero. Non-test example counts are in F but excluded from the production total. `T` includes borrowed tree roots as well as field uses; only `ast.rs:447` is the owned Tree field. Test status comes from test modules/attributes and test/oracle files, including path-mounted test modules.

| Family / file | F: function declaration lines | t: test/oracle declaration lines | T: tree access lines |
|---|---|---|---|
| A `ast.rs` | 299: 49,56,71,383,396,478,530,538,544,559,578,604,613,646,650,662,675,701,708,771,800,809,840,859,880,904,931,941,973,1170,1175,1194,1240,1271,1289,1304,1346,1353,1428,1434,1506,1521,1529,1545,1592,1611,1617,1628,1707,1750,1799,1863,1877,1896,1949,1980,2017,2128,2158,2163,2205,2231,2276,2359,2367,2383,2402,2467,2569,2577,2597,2619,2673,2715,2742,2751,2770,2825,3027,3095,3158,3181,3229,3304,3411,3462,3495,3567,3658,3730,3731,3739,3823,3836,3898,3963,3976,4024,4059,4120,4132,4188,4216,4241,4441,4455,4463,4535,4551,4566,4589,4631,4680,4743,4750,4785,4797,4859,4860,4896,5038,5062,5074,5105,5158,5206,5368,5403,5425,5454,5469,5480,5612,5658,5690,5730,5740,5893,5938,5984,6030,6063,6159,6191,6205,6240,6263,6295,6333,6373,6430,6463,6495,6514,6554,6591,6657,6717,6763,6864,6882,6907,6927,6974,7006,7017,7082,7142,7174,7270,7278,7318,7336,7364,7420,7443,7461,7547,7548,7554,7563,7590,7596,7660,7775,7821,7855,7947,7957,8017,8027,8116,8142,8148,8199,8217,8279,8285,8324,8342,8398,8416,8428,8463,8476,8495,8577,8587,8646,8652,8669,8687,8723,8755,8775,8786,8813,8842,8863,8873,8898,8924,8931,8938,8969,9000,9027,9042,9063,9091,9106,9120,9213,9244,9251,9359,9390,9434,9440,9459,9481,9505,9542,9571,9616,9649,9654,9677,9726,9791,9850,9939,9948,9991,10000,10029,10038,10076,10085,10111,10115,10137,10166,10226,10232,10239,10261,10276,10294,10337,10365,10372,10386,10406,10416,10437,10444,10522,10552,10946,11002,11012,11032,11043,11056,11081,11139,11189,11203,11224,11251,11260,11306,11357,11434,11448,11477,11903,11909 | 15: 11342,11395,11977,11990,12231,12249,12272,12295,12313,12333,12335,12558,12593,12677,12706 | 52,540,607,677,678,783,793,1187,1524,1571,1572,1613,1953,1962,2387,2406,2476,2570,2600,2656,2717,2771,3080,3220,3294,3701,3848,3903,4064,4126,4167,4171,4666,4781,6219,6309,6347,6379,6380,6432,6433,7483,7511,7874,7908,8444,8514,8542,8737,9073,9521,9651,9692,9742,9806,9868,10112,11257,11308,11349,11458,11905,12239,12261,12347 |
| A `ast/binding_verdict_memo.rs` | 5: 116,135,152,187,196 | – | – |
| A `ast/binding_verdict_memo/tests.rs` | – | 3: 6,44,270 | 49,81,82,275 |
| A `ast/binding_verdict_probe.rs` | – | 4: 27,60,117,151 | – |
| A `ast/call_membership.rs` | 5: 15,23,30,119,123 | – | – |
| A `ast/call_membership_tests.rs` | – | 10: 18,28,54,115,138,139,151,196,255,285 | 123,147 |
| A `ast/call_names.rs` | 2: 8,45 | 1: 85 | 21,41,94,101,112 |
| A `ast/js_anonymous.rs` | 9: 9,20,25,54,89,181,230,268,307 | – | 28 |
| A `ast/js_binding.rs` | 10: 81,143,156,170,206,306,319,345,370,427 | – | 163,307 |
| A `ast/js_binding_alias.rs` | 7: 43,95,119,137,167,205,221 | – | – |
| A `ast/js_binding_alias_tests.rs` | – | 2: 33,560 | 42,43 |
| A `ast/js_binding_checks.rs` | 4: 9,53,121,122 | – | 129 |
| A `ast/js_binding_checks_tests.rs` | – | 1: 8 | 22 |
| A `ast/js_binding_decls.rs` | 4: 40,168,180,217 | – | – |
| A `ast/js_binding_helper_tests.rs` | – | 4: 24,25,45,53 | 37,47,54 |
| A `ast/js_binding_namespace.rs` | 1: 11 | – | 19 |
| A `ast/js_binding_recovery.rs` | 2: 14,38 | – | – |
| A `ast/js_binding_recovery_tests.rs` | – | 6: 7,19,38,58,77,108 | 10,23,43,97 |
| A `ast/js_binding_returned.rs` | 3: 21,54,73 | – | – |
| A `ast/js_binding_returned_tests.rs` | – | 1: 32 | 41,42 |
| A `ast/js_binding_site.rs` | 5: 12,25,43,57,162 | – | – |
| A `ast/js_binding_site_tests.rs` | – | 1: 21 | – |
| A `ast/js_binding_tests.rs` | – | 4: 27,49,228,272 | 29,54,257,274 |
| A `ast/js_binding_values.rs` | 5: 28,73,107,124,139 | – | 82 |
| A `ast/js_binding_values_tests.rs` | – | 12: 10,11,27,28,45,69,94,114,130,162,196,216 | 23,40,100,120,156,184,202 |
| A `ast/js_binding_walk.rs` | 3: 145,162,248 | – | 257 |
| A `ast/js_binding_walk_tests.rs` | – | 15: 11,30,44,61,78,96,113,130,147,165,200,212,232,252,274 | – |
| A `ast/js_binding_writes.rs` | 3: 15,49,85 | – | 51 |
| A `ast/js_binding_writes_tests.rs` | – | 4: 9,34,45,54 | 13,38,48,58,59 |
| A `ast/js_cjs_export_barriers.rs` | 4: 8,64,84,122 | – | 9,148 |
| A `ast/js_cjs_terminal.rs` | 3: 8,18,110 | – | 19,111 |
| A `ast/js_generator.rs` | 1: 6 | – | – |
| A `ast/js_module_forwarding.rs` | 5: 10,35,76,94,107 | 1: 159 | 11,36,194 |
| A `ast/js_return_facts.rs` | 14: 52,91,108,140,182,216,243,272,344,396,413,432,446,462 | – | – |
| A `ast/js_return_facts_tests.rs` | – | 1: 47 | – |
| A `ast/js_wrapped_export.rs` | 4: 24,133,201,212 | – | 39,138 |
| A `ast/member_reference_index.rs` | 10: 24,72,90,136,145,156,171,184,250,266 | – | 27,219 |
| A `ast/member_reference_index/identifiers.rs` | 6: 24,54,96,103,112,133 | – | 34 |
| A `ast/member_reference_index/identifiers/oracle.rs` | – | 2: 4,48 | 23,81 |
| A `ast/member_reference_index/identifiers/tests.rs` | – | 8: 20,64,119,179,309,381,471,500 | 183,393 |
| A `ast/member_reference_index/oracle.rs` | – | 4: 9,32,63,108 | 92 |
| A `ast/member_reference_index/regressions.rs` | – | 2: 4,85 | 44,58 |
| A `ast/member_reference_index/tests.rs` | – | 7: 19,50,65,220,243,286,365 | 67,236 |
| A `ast/statement_traversal.rs` | 10: 44,81,113,126,175,264,275,310,347,390 | – | – |
| A `ast/statement_traversal/oracle.rs` | – | 10: 12,49,75,92,135,218,229,254,281,314 | – |
| A `ast/statement_traversal/tests.rs` | – | 5: 27,42,100,126,326 | 136,137 |
| A `ast_asserted_member.rs` | 4: 12,16,79,96 | – | 84,85,108,109 |
| A `ast_asserted_member_tests.rs` | – | 6: 85,139,170,213,230,279 | 103,105,111,145,164,175,182,217,248,283 |
| A `ast_call_execution_owner_tests.rs` | – | 8: 3,42,58,149,283,305,426,455 | 296,318,442 |
| A `ast_callback_identity.rs` | 39: 16,30,34,76,191,213,260,288,306,362,383,389,414,437,460,476,534,558,596,645,678,693,838,862,875,918,987,1010,1018,1050,1063,1131,1210,1233,1274,1288,1303,1321,1328 | 1: 1361 | 223,224,296,297,996,1011,1012,1051,1052,1069,1070,1280,1281,1322,1323,1364,1365 |
| A `ast_contained_rvalue_tests.rs` | – | 3: 6,101,125 | 110 |
| A `ast_erased_rvalue_tests.rs` | – | 4: 3,4,18,46 | 30,51 |
| A `ast_inert_default_parameter_tests.rs` | – | 5: 13,22,164,219,316 | – |
| A `ast_js_param_defs_tests.rs` | – | 5: 4,28,108,127,152 | – |
| A `ast_loop_header_tests.rs` | – | 7: 23,45,73,94,111,134,152 | – |
| A `ast_member_projection.rs` | 8: 61,75,93,114,176,181,199,262 | – | – |
| A `ast_nested_execution_owner_tests.rs` | – | 7: 158,171,234,360,496,604,605 | 181,182,378,507,519,621,642,656 |
| A `ast_required_parameter_tests.rs` | – | 5: 7,95,271,293,407 | – |
| A `languages/js_inferred_name.rs` | 3: 21,34,56 | 1: 107 | 131 |
| A `languages/mod.rs` | 26: 293,341,387,509,636,660,689,704,792,836,1023,1067,1166,1292,1333,1422,1439,1456,1471,1478,1489,1504,1508,1521,1537,1547 | 2: 1641,1656 | – |
| A `parameter_slots.rs` | 23: 22,123,141,207,270,289,314,327,336,341,346,353,372,393,427,465,472,481,495,522,556,582,604 | – | – |
| A `profile/gates.rs` | 5: 17,37,56,75,94 | – | 112 |
| A `profile/owner.rs` | 1: 57 | 1: 272 | – |
| A `profile/search.rs` | 1: 8 | – | 36 |
| A `rust_macro_args.rs` | 9: 137,170,261,288,315,341,461,498,536 | – | 143 |
| A `rust_macro_args/tests.rs` | – | 4: 11,28,347,420 | 36,349,422 |
| C `anon_enable_tests.rs` | – | 3: 131,191,252 | 197 |
| C `anon_eval_tests.rs` | – | 2: 205,299 | 208,316 |
| C `anon_owner.rs` | 12: 43,69,80,84,121,157,172,178,217,223,240,259 | – | 88,89,97 |
| C `anon_owner/module.rs` | 2: 13,23 | – | 18,25,26 |
| C `anon_owner/module_tests.rs` | – | 1: 14 | 17 |
| C `anon_owner/source_index.rs` | 1: 58 | – | – |
| C `anon_owner/source_index_tests.rs` | – | 4: 9,73,144,168 | 154 |
| C `anon_owner_tests.rs` | – | 3: 173,213,258 | – |
| C `anonymous/label.rs` | 1: 14 | – | – |
| C `anonymous/metadata.rs` | 9: 8,120,125,141,151,184,188,211,297 | – | 39 |
| C `anonymous/tests/fold2.rs` | – | 1: 5 | 9,11 |
| C `anonymous/tests/fold2/computed_keys.rs` | – | 1: 36 | 53 |
| C `call_graph.rs` | 37: 1209,1347,1662,2213,2227,3152,3293,3298,3308,3356,3361,3373,3383,3439,3443,3455,3597,3761,4010,4320,4391,4566,4719,4886,4926,5142,5584,5730,5743,5794,5897,5909,6207,6239,6259,6270,7023 | 5: 71,110,250,284,3815 | 2249,3170,3345,3423,3440,4033,5810,5811,5899,5900,5914,5915 |
| C `call_graph_anon_authority_tests.rs` | – | 6: 74,95,169,225,256,274 | 90 |
| C `call_graph_anon_barrier_tests.rs` | – | 4: 5,17,382,417 | 387,388,435 |
| C `call_graph_anon_prep_tests.rs` | – | 8: 23,66,122,198,266,316,398,434 | – |
| C `cfg.rs` | 17: 151,196,280,291,378,399,408,456,540,566,584,624,686,751,801,821,841 | 1: 1541 | – |
| C `cpg/anon_prep_tests.rs` | – | 2: 9,58 | – |
| C `cpg/build.rs` | 8: 122,159,325,1820,2374,2379,2398,2403 | 2: 2262,2285 | 2269,2375,2399 |
| C `cpg/callback_identity_tests.rs` | – | 13: 83,139,183,229,747,1456,1491,1559,1638,1720,1885,1907,2233 | – |
| C `cpg/exact_caller_read_tests.rs` | – | 2: 27,377 | 35,411 |
| C `cpg/exact_parameter_binding_tests.rs` | – | 1: 380 | – |
| C `cpg/member_only_param_r1_tests.rs` | – | 5: 190,269,372,485,600 | – |
| C `cpg/member_only_param_r2_tests.rs` | – | 2: 309,370 | – |
| C `cpg/member_only_param_tests.rs` | – | 4: 306,355,389,476 | – |
| C `cpg/namespace_flow_audit_tests.rs` | – | 4: 69,84,372,509 | 77,145,529 |
| C `cpg/nested_execution_owner_audit_tests.rs` | – | 1: 674 | – |
| C `cpg/optional_parameter_tests.rs` | – | 1: 227 | – |
| C `cpg/reaching.rs` | 3: 122,132,608 | – | – |
| C `cpg/reaching/capture.rs` | 4: 12,13,50,81 | – | – |
| C `cpg/reaching/scope.rs` | 14: 132,333,409,498,536,565,584,603,661,798,849,878,885,893 | – | 357,358 |
| C `cpg/reaching/tests.rs` | – | 15: 18,61,82,118,150,197,217,260,319,337,360,378,397,416,440 | – |
| C `cpg/reaching/tests/captures.rs` | – | 4: 3,27,147,160 | – |
| C `cpg/reaching/tests/cfg_joins.rs` | – | 6: 4,21,44,67,80,97 | – |
| C `cpg/reaching/tests/limits.rs` | – | 8: 4,22,38,58,110,123,140,153 | – |
| C `cpg/reaching/tests/review_regressions.rs` | – | 26: 11,29,47,61,90,108,126,144,162,176,189,239,257,271,292,317,335,367,385,401,417,435,453,475,505,556 | – |
| C `cpg/same_line_occurrence_tests.rs` | – | 4: 13,260,559,947 | 26,574,951 |
| C `cpg/source_index_tests.rs` | – | 1: 6 | – |
| C `cpg/statement_calls.rs` | 3: 9,23,42 | – | 15,28 |
| C `data_flow/build/aliases.rs` | 1: 11 | – | – |
| C `data_flow/build/exact_reads.rs` | 1: 24 | – | – |
| C `data_flow/build/file.rs` | 1: 14 | – | 90,91,599,600 |
| C `executable_owner.rs` | 3: 282,304,336 | – | 288 |
| C `executable_owner/integration/tests.rs` | – | 1: 199 | 231,234 |
| C `framework_entries/express.rs` | 12: 64,157,195,223,238,256,298,348,374,431,480,497 | – | 80 |
| C `framework_entries/mod.rs` | 2: 96,259 | – | 260 |
| C `framework_entries/python.rs` | 3: 29,101,148 | – | – |
| C `go_build_profile.rs` | 1: 144 | – | 145 |
| C `go_callback.rs` | 27: 29,154,208,247,265,277,286,455,532,613,681,700,720,727,733,746,762,828,829,866,883,888,912,962,1046,1050,1091 | 5: 1311,1338,1361,1421,1496 | 67,190,191,274,303,483,484,685,704,852,909,983,1102 |
| C `js_anonymous.rs` | 2: 57,175 | – | 96 |
| C `js_ts_props.rs` | 3: 20,29,106 | – | 30,114,115,151 |
| C `name_resolution/rust_populator/scopes.rs` | 11: 14,21,32,80,117,180,190,314,320,338,350 | – | – |
| C `name_resolution/rust_populator/walk/items.rs` | 2: 117,421 | – | – |
| C `name_resolution/rust_populator/walk/locals.rs` | 2: 355,421 | – | – |
| C `name_resolution/rust_populator/walk/mod.rs` | 6: 51,128,138,156,177,256 | – | 60,158,159 |
| C `navigation/anon_guard_tests.rs` | – | 2: 44,67 | – |
| C `nested_execution_owner_audit.rs` | – | 3: 207,215,273 | – |
| C `queries.rs` | – | 3: 315,493,593 | 323,540 |
| F `js_paths_snapshot.rs` | 1: 835 | – | 846 |
| M `mcp/mem_attribution.rs` | – | 1: 174 | – |
| N `reasoning/sanitizer_walk.rs` | 4: 139,185,232,318 | 1: 389 | 163,164,209,210 |
| N `reasoning/scope_honesty.rs` | 12: 94,112,176,188,196,210,247,339,351,364,372,399 | – | – |
| N `reasoning/seeds.rs` | 3: 202,293,306 | – | – |
| P `go_promoted_snapshot.rs` | 2: 419,486 | – | 428 |
| P `go_receiver_index.rs` | 8: 57,92,148,177,230,361,508,860 | – | 67,186 |
| P `go_type_alias.rs` | 5: 28,51,55,254,278 | 2: 602,618 | 294,609 |
| P `go_type_alias/canon.rs` | 10: 7,222,242,263,277,310,329,368,392,437 | – | 400 |
| P `live_types.rs` | 17: 73,77,120,144,148,167,171,278,283,288,313,317,340,344,373,387,403 | – | 74,145,168,279,284,314,341 |
| P `receiver_index.rs` | 9: 13,52,139,204,307,318,329,364,426 | – | 25,59 |
| P `resolution.rs` | 7: 789,795,802,937,944,955,984 | – | – |
| P `resolution_receiver.rs` | 11: 83,130,191,283,315,495,520,545,576,594,615 | – | 498,499 |
| P `resolution_receiver/tests.rs` | – | 2: 29,130 | – |
| P `type_db.rs` | 12: 572,588,760,766,792,879,908,943,973,995,1019,1031 | – | 581,761 |
| P `type_provider.rs` | 2: 311,337 | – | 325 |
| P `type_providers/go.rs` | 23: 783,823,856,979,1064,1075,1091,1102,1143,1155,1206,1239,1423,1455,1510,1530,1657,1669,1691,1796,1866,1896,1915 | 2: 1737,1742 | 792,1765 |
| P `type_providers/java.rs` | 12: 148,163,191,253,290,315,340,365,382,421,449,531 | – | 149 |
| P `type_providers/python.rs` | 12: 116,125,151,174,198,232,267,321,344,396,412,446 | – | 117 |
| P `type_providers/rust_provider.rs` | 12: 149,163,199,231,268,325,365,393,445,480,496,511 | – | 150 |
| P `type_providers/typescript.rs` | 11: 154,163,194,235,270,290,375,434,453,517,542 | – | 155 |
| R `algorithms/absence_slice.rs` | 1: 503 | – | – |
| R `algorithms/angle_slice.rs` | 1: 116 | – | – |
| R `algorithms/callback_dispatcher_slice.rs` | 1: 64 | – | – |
| R `algorithms/conditioned_slice.rs` | 4: 285,294,343,349 | – | 290,345 |
| R `algorithms/contract_slice.rs` | 15: 146,369,464,492,497,542,576,588,589,911,1079,1081,1103,1299,1558 | 2: 1636,1650 | 1658 |
| R `algorithms/full_flow.rs` | 1: 19 | – | – |
| R `algorithms/horizontal_slice.rs` | 2: 35,147 | – | – |
| R `algorithms/left_flow.rs` | 2: 42,297 | – | – |
| R `algorithms/mod.rs` | 1: 63 | – | 76 |
| R `algorithms/parent_function.rs` | 1: 12 | – | – |
| R `algorithms/peer_consistency_slice.rs` | 1: 179 | – | – |
| R `algorithms/phantom_slice.rs` | 1: 44 | – | – |
| R `algorithms/primitive_slice.rs` | 2: 145,297 | – | – |
| R `algorithms/provenance_slice.rs` | 1: 566 | – | – |
| R `algorithms/quantum_slice.rs` | 9: 27,175,183,429,541,559,584,595,636 | – | – |
| R `algorithms/relevant_slice.rs` | 3: 23,188,198 | – | – |
| R `algorithms/resonance_slice.rs` | 1: 57 | – | – |
| R `algorithms/spiral_slice.rs` | 1: 48 | – | 213,253 |
| R `algorithms/symmetry_slice.rs` | 1: 157 | – | – |
| R `algorithms/taint.rs` | 290: 252,267,273,713,719,754,758,765,912,1419,1473,1493,1601,1638,1643,1655,1666,1693,1740,1796,1815,1826,1858,1871,1882,1940,1963,1998,2013,2031,2045,2056,2076,2083,2101,2164,2181,2217,2233,2240,2274,2303,2311,2317,2331,2340,2361,2386,2396,2502,2536,2548,2574,2595,2637,2707,2757,2837,2916,2974,2993,3115,3167,3203,3244,3321,3383,3414,3490,3512,3528,3568,3594,3616,3711,3834,3911,3979,3996,4044,4060,4082,4114,4125,4131,4151,4211,4311,4329,4358,4388,4497,4506,4515,4552,4579,4607,4619,4631,4650,4685,4701,4724,4769,4778,4791,4863,4966,5072,5086,5107,5328,5420,5490,5547,5561,5570,5582,5612,5616,5629,5695,5708,5771,5786,5858,5953,6024,6109,6207,6259,6311,6325,6351,6363,6395,6416,6429,6458,6515,6571,6581,6617,6629,6646,6651,6674,6693,6711,6715,6730,6756,6778,6840,6881,6927,6960,6971,6993,7009,7066,7083,7092,7101,7110,7138,7150,7162,7178,7190,7216,7254,7292,7313,7322,7364,7382,7405,7423,7436,7464,7502,7510,7535,7555,7573,7588,7625,7660,7696,7719,7736,7778,7796,7809,7824,7867,7878,7937,7948,7966,7987,8022,8046,8071,8103,8139,8203,8210,8246,8269,8302,8333,8359,8364,8369,8405,8542,8577,8616,8666,8693,8704,8739,8803,8836,8879,8887,8938,8964,9027,9050,9059,9064,9072,9088,9132,9255,9275,9288,9313,9324,9357,9390,9400,9443,9475,9489,9499,9506,9527,9554,9581,9604,9627,9643,9691,9732,9752,9836,9878,9893,9908,9919,9928,9969,9996,10031,10063,10112,10122,10178,10209,10234,10286,10369,10442,10517,10523,10527,10531,10537,10547,10563,10580,10596,10607,10634,10823,10969 | 4: 11800,11822,11842,11855 | 1436,1486,1947,2021,2726,3987,4319,5907,6118,6148,6181,6371,6785,6887,7471,7516,7560,8033,8551,8622,8748,9094,9145,9168,9697,9765,9809,9843,9975,10303,10376,10428,10449,11804,11830,11846,11863 |
| R `algorithms/thin_slice.rs` | 1: 15 | – | – |
| R `frameworks/mod.rs` | 1: 127 | – | – |
| R `frameworks/python/fastapi.rs` | 15: 18,50,65,72,83,104,130,156,186,199,223,242,293,305,317 | – | 57 |
| R `frameworks/python/flask.rs` | 13: 18,37,48,77,107,128,154,194,218,226,234,246,258 | – | 44 |
| R `output/review.rs` | 1: 349 | – | – |
| R `react_hooks.rs` | 6: 115,145,171,207,228,277 | – | – |
| R `targets/dependency_hint.rs` | 10: 89,109,119,148,176,181,213,241,255,277 | – | 92,270 |
| R `targets/dependency_identity.rs` | 17: 17,65,94,110,122,128,146,269,270,341,355,369,376,392,423,429,459 | – | 100,247,298,351 |
| R `targets/mod.rs` | 1: 328 | – | – |
| X `examples/asserted_member_observations.rs` | 1: 51 | – | 56 |
| X `examples/dfg_census.rs` | 5: 137,243,244,265,353 | 1: 565 | – |
| X `examples/parameter_site_census.rs` | 3: 18,69,94 | 1: 339 | – |
| X `examples/parameter_slot_characterization.rs` | 5: 332,342,347,387,493 | – | – |
| X `tests/ast/binding_test.rs` | – | 6: 163,186,226,253,318,346 | – |
| X `tests/ast/field_test.rs` | – | 3: 534,565,597 | – |
| X `tests/ast/import_test.rs` | – | 4: 149,168,188,210 | – |
| X `tests/ast/owner_test.rs` | – | 10: 16,28,38,48,58,90,103,129,144,155 | – |
| X `tests/ast/parameter_slots_test.rs` | – | 6: 3,9,23,120,188,248 | 212,226 |
| X `tests/ast/parse_quality_test.rs` | – | 1: 40 | 52,69 |
| X `tests/common/mod.rs` | – | 1: 1149 | – |
| X `tests/integration/cjs_export_barrier_test.rs` | – | 1: 152 | 155 |
| X `tests/integration/cjs_export_state_test.rs` | – | 1: 6 | 31 |
| X `tests/integration/cjs_refusal_state_test.rs` | – | 1: 14 | 41 |
| X `tests/integration/cjs_refusal_test.rs` | – | 2: 16,213 | 24,230 |
| X `tests/integration/cjs_terminal_state_test.rs` | – | 1: 6 | 41 |
| X `tests/integration/cjs_terminal_test.rs` | – | 2: 166,250 | 171,261 |
| X `tests/integration/esm_forwarding_state_test.rs` | – | 1: 53 | 94 |
| X `tests/integration/js_alias_matrix_test.rs` | – | 1: 175 | 191 |
| X `tests/integration/js_method_name_binding_test.rs` | – | 2: 8,357 | 20,394 |
| X `tests/integration/module_binding_audit_test.rs` | – | 2: 24,439 | 37,60,451 |
| X `tests/integration/module_binding_flow_test.rs` | – | 1: 9 | 20 |
| X `tests/integration/receiver_self_binding_test.rs` | – | 5: 19,35,61,423,508 | 29,68,69,429,513 |
| X `tests/integration/type_relative_receiver_test.rs` | – | 1: 423 | 458 |
| X `tests/lang/bash/bash_test.rs` | – | 1: 4 | – |
| X `tests/lang/c/algo_test.rs` | – | 1: 39 | – |
| X `tests/lang/c/cve_test.rs` | – | 1: 450 | – |
| X `tests/lang/cpp/cpp_test.rs` | – | 2: 4,241 | – |
| X `tests/lang/cpp/decorated_test.rs` | – | 1: 38 | – |
| X `tests/lang/go/algo_test.rs` | – | 2: 6,54 | – |
| X `tests/lang/go/span_test.rs` | – | 4: 4,16,33,49 | – |
| X `tests/lang/javascript/destructuring_test.rs` | – | 9: 4,38,70,102,133,158,354,384,414 | – |
| X `tests/lang/javascript/span_test.rs` | – | 3: 4,16,33 | – |
| X `tests/lang/lua/lua_test.rs` | – | 2: 4,168 | – |
| X `tests/lang/python/decorated_test.rs` | – | 1: 208 | – |
| X `tests/lang/python/span_test.rs` | – | 3: 4,16,33 | – |
| X `tests/lang/rust/rust_test.rs` | – | 2: 4,206 | – |
| X `tests/lang/rust/span_test.rs` | – | 3: 4,17,34 | – |
| X `tests/lang/terraform/terraform_test.rs` | – | 1: 4 | – |
| X `tests/lang/typescript/erased_rvalue_test.rs` | – | 8: 39,58,77,98,129,141,179,239 | 45,64,85,102,134,149,150,151,159,163,167,184,243 |
| X `tests/lang/typescript/generic_call_boundary_test.rs` | – | 6: 13,48,71,106,138,186 | 25,52,79,80,91,92,115,116,131,132,149,150,162,163,231,232 |
| X `tests/lang/typescript/import_type_grammar_test.rs` | – | 1: 29 | 46,66 |
| X `tests/lang/typescript/type_only_call_test.rs` | – | 7: 68,81,100,120,142,232,275 | 72,89,103,123,145,239,281 |
| X `tests/navigation/index_test.rs` | – | 1: 24 | – |
| X `tests/output/dfg_compaction_test.rs` | – | 1: 86 | – |
| X `tests/reasoning/scope_honesty_test.rs` | – | 2: 25,173 | – |

### Tree and borrowed-node owners; public surface

| Holder / exposed boundary | Exact anchors and required treatment |
|---|---|
| All explicit Tree type uses | `src/ast.rs:5` import, `:49` count_error_nodes, `:447` public ParsedFile.tree, `:11903` collect_error_lines. `:478` creates the owned Tree; `src/js_paths_snapshot.rs:835` creates a temporary TSX Tree outside ParsedFile. No other production explicit Tree type. |
| Node-bearing AST results | `CallSiteMeta.receiver_node` (`ast.rs:127,135`); `CallMembership.owner` (`ast/call_membership.rs:8`); `MemberPath.receiver` (`ast_asserted_member.rs:4`); `JsInferredName` (`languages/js_inferred_name.rs:14`); `Tok::{Ident,TokenTree}` (`rust_macro_args.rs:201`). All node-bearing values remain within the originating lease. |
| Node-bearing helper caches | `ast/js_binding.rs:57,60,61,68` ExportScope/Index/NamespaceImports/JsBindingCache; `js_binding_alias.rs:33` Link; `js_binding_walk.rs:33` Walk; `member_reference_index.rs:12,18,128` Candidate/Index/MemberReferenceSearch; `member_reference_index/identifiers.rs:14,19,89` Candidate/Index/IdentifierReferenceSearch. Destroy caches and iterators before tree release; never promote them to snapshot OnceLocks. |
| Node-bearing analysis contexts | `resolution.rs:706` ReceiverCtx and `:733` ReceiverClassifier::classify (trait declaration); `resolution_receiver.rs:22,514` ReceiverTypeCtx/MethodCallParts; `go_receiver_index.rs:466` GoReceiverCtx; `frameworks/mod.rs:114` CallSite (also re-exported by sanitizers); `algorithms/taint.rs:4956` TargetSeedSynthesisContext; `reasoning/scope_honesty.rs:148` FunctionEntry. Containing functions/impl methods are in the ledger. |
| Owned identities are different | `ast.rs:188` FunctionInfo stores spans/kind, not Node; `name_resolution/rust_populator/walk/mod.rs:121,156,177` NodeId stores kind/range and refetches; `anonymous/metadata.rs:8` uses node.id only within extraction. Any persisted reconstruction key must add snapshot/file/grammar identity; raw node.id is lease-local. |
| Stable prism::api | `src/api/mod.rs:1,12` explicitly guarantees facade names/signatures and certain handle use. `ReviewInputs.files` (`api/review.rs:51`), build_context/BuiltContext, run/review outcomes and `nav_session` (`api/nav.rs:16`) expose/retain eager handles. No direct Tree/Node re-export in api, but those handles lead to them. Keep signatures and documented field access. |
| Public graph/session containers | `lib.rs:40–116` exposes ast/languages/repo_loader/navigation/cpg/etc.; `LoadedRepo.files` (`repo_loader.rs:37`), `NavigationSession.repo/index` (`navigation/mod.rs:128`), `CpgContext.files` (`cpg/context.rs:43`), `mcp::SessionProvider::session` (`mcp/session.rs:186`) expose eager storage. Preserve these too: the owner rule is broader than api's narrow stability promise. |
| Public ParsedFile node-returning API | `ast.rs:604` enclosing_function, `:646` all_functions, `:840` function_node_spanning, `:1521` identifiers_on_line, `:10111` find_function_by_name; `:9726,9850` return CallSiteMeta with a node. Existing callers may retain these borrows as long as ParsedFile lives. |
| Public ParsedFile node-taking API | `ast.rs:530,859,3158,3181,4859,6205,6295,6333,7006,7461,7855,8495,8723,8775,8863,8924,9000,9027,9063,9106,9434,9459,9505,9571,9616,9677,9726,9791,9850,10137,10166,10226,10232,11434,11448`; additional impls `ast/statement_traversal.rs:44,264`, `ast/member_reference_index.rs:266`, `ast_callback_identity.rs:16,191,534,1288`. Preserve each signature; full names are in census.json. |
| Other public node APIs | Language accessors at `languages/mod.rs:293,341,387,509,636,704,792,836,1023,1067,1166,1292,1333,1422,1439,1456,1471,1489`; `frameworks::CallSite` and semantic-check callbacks `frameworks/mod.rs:75,87`; Flask/FastAPI route/decorator helpers (ledger); `reasoning::scope_honesty::warnings_for_function:94`; `resolution::{ReceiverCtx,ReceiverClassifier,legacy_recover,RustReceiverTyper,ReceiverTypeCtx}`; `rust_macro_args::extract_calls:288`; `algorithms::left_flow::left_flow_core:42`. Public structs containing nodes remain eager-compatible even where their defining submodule is private. |

## 3. Ownership and lease contract

| Option | Compatibility / memory consequence | Decision |
|---|---|---|
| Change ParsedFile.tree to optional/lazy tree; change node-returning APIs | Breaks field access and existing borrows; a temporary reparse cannot back a returned Node. Unsafe lifetime extension is forbidden. | Reject here; breaking API design is separate. |
| Keep eager public path; private runtime snapshot with scoped per-file access | Preserves all public names/types; private runtime must adapt tools, refresh and build boundaries without retaining a public eager mirror. Existing AST helpers run on one temporary eager ParsedFile inside a closure. | **Recommend.** Largest work is internal repository adapters, not rewriting every AST helper. |
| Eager compatibility mirror beside an evicted snapshot, or whole-repo eager reconstruction per query | Mirror preserves the very trees being removed; reconstruction defeats a K=1/4 bound and repeats whole-repo work. | Reject for activation; eager reference only in separate differential runs. |

| Object / transition | Contract |
|---|---|
| Frozen source owner | One immutable source/metadata record per accepted file, owned by an Arc snapshot with file hash, language/grammar identity, line/function tables, parse-quality facts, manifests, resolver absence/coverage and type-db identity. Source stays resident; no query reopens disk. Graphs/types and this snapshot are published as one generation. |
| Tree owner | A private, non-cloneable admission guard owns the temporary ParsedFile/Tree and parser resources. No idle LRU, TTL, OnceLock<Tree>, Arc<Tree> escape or tree retained between requests. Initially reparse through existing ParsedFile parsing semantics; bounded source/table copies count against measured overhead. Later borrowed-source optimization requires its own compatibility proof. |
| Closure API | `with_tree(snapshot, path, operation)`/`with_parsed_file` invokes a higher-ranked callback over a fresh borrow; its result type is independent of that borrow. Return owned spans/strings/facts/errors only. No Node, cursor, iterator, CallSiteMeta-with-node or node-bearing helper escapes. HRTB prevents borrowed escape, not owned Tree::clone: callbacks are audited crate-private leaf adapters, with a sealed tree-free owned-result vocabulary. Reject cloned Tree/ParsedFile results too; audit callback side effects for clone/stash. Compile-fail fixtures cover node, cursor, iterator, helper and owned-tree return escape; retention witnesses cover stashing. |
| Helper compatibility | Existing public eager methods remain implementations/control. Internal leaf adapters run a complete per-file operation in one lease; build reference-search and JS binding caches inside it. Do not reparse per helper or carry raw node.id across calls. Source-only operations use the frozen record without a lease. |
| Per-request reuse | Group all work for one file into one lease, emit owned intermediate facts, release it before visiting another file. One parse per distinct file/request; request memo contains owned results, never trees. Multi-file algorithms stage facts first, then join graph relations. If a family cannot be scheduled this way, split/redesign it before activation rather than retain every touched tree. |
| Global admission | One process-shared K-slot mutex/condition-variable admission object, acquired before parser allocation; it covers languages, requests, old/new generations and internal construction/verifier. Predicate loop handles spurious wakes; FIFO tickets avoid starvation; release wakes waiters. No lock held during parse/callback; no acquisition from inside a lease. Reuse current borrow for same-file helpers; stage cross-file facts after release. |
| Deadlock / worker scheduling | Do not block all Rayon workers waiting for children that need a permit held by the parent. Admit at outer per-file jobs; never spawn/join leased children while holding a lease. K=1 and nested same/cross-file negative tests are mandatory. K=0 rejected; “unbounded” is diagnostic-only. |
| Byte bound | K bounds tree count, not tree bytes. Record peak tree bytes and largest admitted file; existing loader's 2 MiB source limit (`repo_loader.rs:13`) does not bound AST expansion. Choose K from the sweep, reduce it or park if active-tree overhead defeats heap/latency gates. No new file rejection to make results fit. |
| Generation / refresh | A request captures an Arc of one immutable generation before seed resolution and uses it through response serialization. Key work by generation+path+hash+grammar, not path alone; old reader never looks up current disk/source. Candidate and verifier have distinct identities and share permits. Verified atomic publication preserves old-reader ownership, rollback and per-request summary/generation semantics; no D13 off-lock refresh change here. |
| Failure / panic / cancellation | Callback-local nodes/helpers drop first; guard Drop destroys Tree and parser before releasing its slot on success, parse error, callback error or unwinding. Admission state is not locked across callbacks, so callback panic does not poison it. Queued cancellation retires its ticket; running synchronous parse is accounted until it actually returns/unwinds. Preserve current fatal-daemon panic behavior; process abort does not promise Rust Drop. No partial publication; preserve current best-effort cache-save timing/identity rather than inventing a new cache transaction. |
| Construction and verifier | Retain eager public constructors. Private bounded loader emits owned accepted-file facts; provider/graph phases consume per-file leases and owned facts, preserve sorting/first-winner order, and do not require BTreeMap<String,ParsedFile> to remain resident. JS ambient parses also acquire admission. Full verifier retains exact loader semantics, including manifests/JS closure; this is not the separate #9v hash-only optimization. |
| Runtime/public boundary | Private runtime provider/session/tool context uses frozen records; public SessionProvider::session and ToolContext/registry/transport entry points keep their eager forms. Extract common logic with eager wrappers and private adapters; do not change a public field type or fabricate an eager view on demand. Built-in stdio/shared runtime and memreport must exercise the same private path. |
| Construction completion | Publish only owned facts/graphs/source; all build guards gone. No dormant LoadedRepo, Arc<ParsedFile>, builder receiver context or helper cache may remain reachable. Differential eager reference lives in a different run, not next to candidate for memory acceptance. Lazy call-edge initialization remains graph-only. |

## 4. Per-language evaluation

| Choice | Saving beyond zero-idle eviction / correctness | Recommendation |
|---|---|---|
| Grammar selection | Already one grammar per Language (`languages/mod.rs:50`), with distinct TS and TSX. Compiled/static grammar code is not one copy per parsed file; splitting files does not reclaim it as tree heap. | Keep now; identify grammar on every lease. |
| Per-language tree pools | With zero idle trees, idle saving is **zero**. May reduce parser setup or isolate scheduling, but retained parsers can retain allocator capacity. One cap per language would multiply peak by language count. | No retained tree pools. Later only measure bounded parser reuse under the same global K; do not assume Parser::reset frees memory. |
| Lazy per-language compiled queries | Current query_cache initializes all languages (`queries.rs:34`). Could avoid unused Query allocations; allocator-hook comment estimates ~54 KB fixed process-wide (`mem_tree_alloc.rs:23`), not measured here. Public query_cache promises the full map. | Later separate private lazy-query experiment if useful; likely IMMATERIAL to 200 MiB/15% M. Never claim a grammar-pool saving from that comment. |
| Per-language index partitions | Can additionally defer source/graph/type/index bytes for untouched languages, unlike tree eviction. Needs global symbols/topology/edges, stable IDs/order and exact whole-repo discovery; hidden partitions must not look like absent targets. Whole-repo callers/taint/module queries may require multiple partitions. | Later, separate design after #3/#10; no partition activation in T3. |
| Corpus mix | Prometheus Go-dominant, ruff Rust-dominant, excalidraw TS/TSX-heavy, Python winner Python-heavy (owner-supplied mix; no fresh census). Dominant language must load, so minority partitions cannot be assumed to clear M. TS/TSX/JS imports share resolution; larger Python+TS agents are later scale checks. | Measure per-language contribution after soak before investing. Do not substitute historical C-heavy slicing for Py. |
| Cross-language edges | Retain whole-repo CPG/CG/DFG and registry now. Partitioning later needs a generation-bound global edge/symbol directory; load all participating partitions before joins and preserve ambiguity, unknown/refusal, language-specific identity and deterministic ordering. Never prune an edge because its language is unloaded. | Current leases change tree lifetime only; inter-language outputs remain byte-identical. Rare edges are still correctness gates. |

## 5. Post-soak measurement and stopping rules

| Step | Required evidence / gate |
|---|---|
| Admission | Controller confirms soak complete; use plan §§2/5 host admission (including ≥50 GiB free, no competing index load, non-increasing swap). No measurement in this design turn. Pin revised E/P/ruff/Py corpus commits, settings, seeds, actual worker count, build/compiler/allocator/features and current caches. |
| Existing stages | Reuse `mem_report.rs:290–357`: after_load_repo, after_index, after_call_edges, after_workload, after_settle, after_refresh; allocator hooks installed before parse. Capture heap, tree-sitter live bytes, both classified footprint levels and named fresh-process lifetime peak. Add only prototype-local parse count/time, wait time, active/peak leases and active-tree bytes; distinguish fixed queries/parser buffers from Trees. |
| Reparse rows | For each corpus: whole-source read+parse baseline separately from retained-source lease reparsing; count/total/median/p95/max parse time, source bytes, tree peak bytes and per-language split. Report every reparse, including failures, and bounded source/helper copies. Old 0.4–0.5 s receipt is context, not the new result. Missing counters invalidate a residency claim. |
| Cap sweep | K=1/4/15/unbounded diagnostic; same snapshot/output workload and request concurrency, plus concurrent different-language/different-generation stress. Record throughput, request p95, queue wait, parse time, peak leases/tree bytes and settled heap. Serial AST requests alone do not measure contention. Count total trees across build/refresh/verifier too; no per-language multiplication. Select smallest passing K, preregister it before final acceptance. |
| AST + ordinary workload | 50 pinned callable seeds × three symbol-span passes =150 requests; add fixture review/span/reasoning negatives and repeated/multi-file requests. Run ordinary 200-request C1 separately (it omits spans). Cold, true-hit warm, workload, no-change refresh and S0 changed-refresh each get T3d full-n rows on all four corpora; same-build eager/leased transcript pairs check exact bytes. Freeze full expected outputs and seed/workload hashes before timings; every failed request invalidates its timing run. |
| T3b recorder | Non-default feature, fixed 256-entry u64 wall-duration array (2 KiB+state), delivery→matching response, excludes handshake/notifications. Validate count equality, units, capacity, missing/duplicate/mismatched responses, nonnegative finite values, zero errors; overflow invalidates, never truncates. Identical support patch/probe/features on both sides. Known-duration/empty/handshake/error/overflow controls and R2-latency-drops-sample. |
| p95 | Nearest-rank per 150-request run: rank 143. Compare medians of ≥3 admitted run-p95 values per side; increase ≤max(10%,10 ms). No p95 inferred from stage totals. Retain all runs/failures and AB/BA alternation. |
| Memory M | ≥200 MiB **or** ≥15% of base live heap on ≥2 revised corpora; ≥3 admitted runs/side, candidate max < base min. Workload-settled heap and corroborating same-level footprint must both meet M; preclassify footprint levels before unblinding. Heap-only benefit is not host-relief acceptance; unmatched levels/noisy ranges are INCONCLUSIVE. Report <1 GiB separately for every corpus. |
| Other gates | Exact nav/review/agent/MCP evidence/spans/warnings/order/IDs/cache/refresh summaries; no non-target steady increase >1 MiB, median load/build/refresh/workload slowdown >max(5%,100 ms), or peak increase >max(1%,16 MiB). Persistent query cache allowance is explicit, not mistaken for an idle tree. |
| Residency witness | After every repeated pass and quiescent refresh: 0 idle Trees, no graph-reachable tree/helper owners; peak admitted live Trees ≤K through barriers, errors/unwind and overlapping generations. Same-environment eager base must fail the residency witness. Parse-from-disk, retain-all and mix-generations mutants must be killed. |
| Stop / park | API break, unaccounted Tree owner, unsafe lifetime, wrong bytes, K violation/deadlock or generation mixing stops activation. M/p95/non-target gate FAIL parks the prototype; noise is INCONCLUSIVE, not permission to rebaseline. Large-file tree bytes or repeatedly reparsing files that cannot be grouped require a bounded design correction. Report any corpus still ≥1 GiB; do not move target or claim host capacity from heap alone. |

## 6. Accepted-parent slice plan (estimates include tests)

Two review rounds per PR, declared before dispatch. Estimates below are **guessed weighted non-mechanical changed lines including tests**, not source measurements. Pure-move volume is stated separately. Rebind the ledger and exact ownership before each PR; split before dispatch if >~600. At cap, fold closed converging fixes with an explicit extension; repeating open-class findings park for design. Never restart a partially reviewed artifact.

| PR / family | Owned seam / bounded change | Weighted lines incl. tests / acceptance |
|---|---|---|
| T3a.1 move only | Extract `ast.rs:604–834` callable lookup/reconstruction/span helpers to `src/ast/tree_access.rs`; preserve public paths and visibility; no new lease semantics. Further AST moves only as independently needed operation families. | ~230–330 moved + 40–80 module/visibility/move controls; ≤450 total. `src/ast.rs` already >2,000 weighted lines: must shrink, never grow; do not hide a large AST rewrite in extraction. |
| T3b store + recorder, test-only | New private snapshot/admission/lease modules and tiny test fixtures; latency transport/report/validator/tests. No runtime eviction. | 300–400 store/source/drop tests + 150–200 recorder = **450–600**. UTF-8/CRLF, parse/callback errors, unwind, K=1 wait/release, missing path, large single file, compile-fail escape and generation controls. If this does not fit, split store and recorder before dispatch; combined budget is not permission to omit cases. |
| T3c-N1 spans / source views | Shared internal span query core plus eager wrapper; frozen source lookup for snippets/order. N family has 19 reasoning consumers plus one span boundary; MCP view has zero direct node functions. | 350–550. Empty/malformed/anonymous callable spans, indentation/CRLF, source-only zero parses; exact eager/lazy bytes; reparse/drop witness. |
| T3c-N2 seeds / scope honesty | `reasoning/{seeds,scope_honesty}` (15 selected functions together); per-file lease around all node work. | 350–550. Missing/duplicate symbol, decorated/nested functions, warning order, multi-file requests. Keep helpers unchanged inside lease. |
| T3c-N3 sanitizer queries | Four sanitizer_walk functions plus narrow taint advisory/node-recognizer entry points; framework CallSite lives inside lease. Do not migrate all ~340 algorithm functions. | 400–600. Wrong-language recognizer, keyword/data argument, paired-check exclusion, negative sanitizer and multi-file reuse; byte parity. |
| T3c-S1 public/runtime facade | Private frozen runtime session/provider and shared query adapters, eager public wrappers. Own only facade extraction, no activation. | 400–600; external-style compilation fixtures for api, direct tree/node borrowing, SessionProvider::session and ToolContext. Separate transport/registry switch below. |
| T3c-S2 refresh / dispatch plumbing | Private stdio/shared runtime, tools/registry and memreport use frozen snapshot; generation publication/freshness/rollback core shared with eager path. | 400–600; stale old reader/edit/delete, failed candidate/verification, panic and per-request refresh bytes. No off-lock refresh policy. Split refresh from transport if inventory exceeds budget. |
| T3c-P1…P6 provider construction | Separate Go, Rust, Python, TS/TSX, Java, then C/C++/live-type scans. Provider function counts Go 23 / Rust 12 / Python 12 / TS 11 / Java 12, plus live-types 17, bound helper fan-out; each outer per-file extraction emits owned type facts before release. Preserve duplicate/first-result order. | **300–550 each**, at least six PRs. Each has malformed/unsupported/type-alias negative, same-snapshot cache-hit registry parity and live-tree-drop witness. Go aliases/receiver postpasses are separate from Go provider if needed. |
| T3c-C1 accepted-file snapshots | Bounded loader/ambient parse/manifest+resolver priming, source/table freeze; full verifier adapter reuses exact acceptance behavior. | 400–600; skip/hash/absence/add/remove/recovery/symlink and disk-edit snapshot negatives. No #9v hash-only shortcut. Split JS closure from loader if >600. |
| T3c-C2 scope / receiver facts | Rust scope walker, Go receiver/alias/callback and JS anonymous/export/owner families each get their own PR, never one combined migration. NodeIds/facts are owned and generation-bound between passes. | **350–600 per language/operation family**. Count anchors: call_graph 37, CPG 32, shared AST 475; these forbid a monolithic change. K=1, cross-file import/receiver and duplicate-order negatives for each. |
| T3c-C3 graph phase boundaries | Separate call-graph skeleton, direct/callback/receiver enrichment, DFG file jobs, CFG/reaching and CPG assembly adapters. Each consumes owned per-file facts; no all-file ParsedFile map in private runtime. | **400–600 each**, minimum five PRs; exact graph IDs/order, framework/anonymous/ambiguity negatives, single-file leases and base-red retention witnesses. Reuse 475 AST functions inside leases, do not copy them to a second implementation. |
| T3c-R conditional review | Keep public eager review/algorithm/targets paths. Only additional MCP-reachable review boundaries require migration; taint's ~290 selected functions stay in their existing helper family, not a wholesale rewrite. | **300–550 per newly reached operation**; prove reachability before dispatch. Full review/targets bytes protect public compatibility even when no migration is needed. |
| T3d activation | After all private serving/build/verifier consumers pass inventory and bounded tests: select measured K, publish frozen runtime by default; same binary keeps eager reference selectable for controlled runs. No public/default library mode change. | 250–450; full acceptance batch all kinds + AST/cap/concurrency. No hidden eager fallback and no format bump. Separate later retirement of test controls. |

The family census is a **scope bound, not an estimate that all functions must change**. Construction's bulk `BTreeMap<String,ParsedFile>` APIs are the largest cost risk; adapters must preserve complete graph semantics without retaining that map. A pre-dispatch slice manifest names exact functions, tests and affected workload kinds; a new >600-line projection/fact model needs another accepted-parent slice, not an expanded T3d.

Implementation gates (later): full `cargo test`, full `cargo test --features mcp`, all prototype-feature suites, fmt/clippy, and memreport Python unit tests for recorder changes; report totals and exclusions. For AST/nav/CPG changes, immediate release rebuild then Tier-A matrix before commit and quick before review, with same-worktree `--allow-stale-sut`; full multi-corpus remains human-triggered. Same-environment base-red resource witnesses plus edge/negative cases; pure moves use move/byte controls. Required mutgate runs and watched tree/latency mutants follow round-2 §6; zero selection/environment refusal is INADMISSIBLE, never green. T3a sanity pairs, T3b full AST timing validation, T3c all affected kinds, T3d full all-kinds batches follow plan §5; none run now.

## 7. Risks and owner questions

| Tag | Concrete condition / consequence / disposition |
|---|---|
| WRONG / MATERIAL — rejected design | Removing public tree field or returning a node borrowed from a temporary lease breaks existing `let n = parsed.all_functions()[0]` callers (or is unimplementable safely). Retain eager API; compile external-style fixtures and reject escape at compile time. This is a counterexample to the proposed substitution, not a current production bug. |
| WRONG / MATERIAL — rejected design | Reparse current disk for an old snapshot after editing a function: old graph byte ranges point into new syntax, yielding wrong spans/sanitizer evidence. Freeze source/generation; barrier test edits disk between capture and lease. |
| WRONG / MATERIAL — rejected design | K=1 outer lease waits for a second-file nested lease: it holds the only permit and cannot finish. Prohibit nested acquisition; extract owned facts then release; negative/barrier tests. |
| SMELL / MATERIAL | Broad eager map coupling and public MCP handles could hide a full eager mirror or exceed slice budgets. Inventory counts require the separate facade/construction families; activation stays off until all internal owners are accounted. No measured regression alleged. |
| SMELL / MATERIAL | Count cap alone may leave a large-tree peak or queue delay above gates; repeated fact passes may cost more than inherited parse receipts suggest. Sweep K with real workload concurrency; stop on failing bytes/heap/p95 gates. |
| SMELL / MATERIAL | Node-id caches or independently keyed language stores can mix trees/generations; keep caches lease-local and global admission shared. Verify with overlapping generation/language fixtures. |
| SMELL / IMMATERIAL | Lazy grammar/query initialization may save fixed process overhead, but has no demonstrated M-sized opportunity beyond zero-idle eviction. Defer optimization; do not count it toward 317/419 MiB. |

**Owner questions: none blocking this design.** The existing controller C0 decision supplies the Python winner and final capacity checkout mix; code cannot choose those workloads. K is selected by the preregistered sweep, not another policy question. API preservation, zero idle trees, language-split evaluation and no measurement during soak are not reopened.

Verification of this artifact: static source/identity/census and document checks only; two self-review passes, no independent agent review. No tests, cargo/builds, index loads, runtime measurements, network operations or dotfile reads. Performance numbers above are inherited/derived; all activation and host-capacity claims remain unverified. Controller owns commit custody; evidence handoff/snapshot accompanies the document.
