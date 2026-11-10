# #630 slice 5 fold 1 — JS/TS/TSX rules U and D (PR #788, round 1 of 2)

Code commit: `1bbba2c` on `perf/630-ud-js` (parent ffcffdc1). Base for measurement: origin/main 9f69debf.
Corpora (pinned, from the `corpora-v1` release, sha256 checked): express 18e5985b, excalidraw 0642e72c, TypeScript src 7964e22f,
vitess 16bcb943 (control), django e8cff292 (control). Release builds of main, ffcffdc1 and the fold head; `nav --no-cache`, RAYON 4.

## Tests run
- `cargo test --lib` on the fold head: 3032 passed, 2 failed (`cpg_cache::stream::tests::a_write_that_cannot_start_fails_and_leaves_the_previous_cache_usable`,
  `resolution::js_ts::paths_snapshot::boundary::tests::unreadable_source_and_directory_skip`). Both rely on chmod-denied access and this session
  runs as root (uid 0); not re-run on main, so environmental by inspection only. The `cli`/integration targets and `cargo test -p prism-mcp` were **not run**.
- `js_prune_matrix` passes; 17 of the new rows fail on the ffcffdc1 `src/` (RED), see the table.

## Disposition table
Helpers reused (item 3), no new approximation: `ParsedFile::js_ts_direct_eval_anywhere`, `ParsedFile::js_ts_mapped_arguments_possible`
(`src/ast/callback_identity.rs`), `inside_with_body` (`src/ast/js_write_census/refusals.rs`, made `pub(crate)`), and the S1b-1 census
`walk::precondition` for the escaped-spelling refusal (it also refuses a parse error and assignment-form `using`). The new entry point is `js_prune::owner_enabled`.
All new rows are in `tests/fixtures/dfg_prune/js/06_fold1.js` + `obligations.jsonl` (case `06_fold1`).

| # | finding | disposition | RED row on ffcffdc1 |
|--|--|--|--|
| 1 | Opus W1 case-label assignment dominates | fixed 1bbba2c: `classify_write` returns the identifier for a non-statement assignment, and `window()` admits only `expression_statement`/`lexical_declaration`/`variable_declaration` for the JS family | `case_label_not_dominating` x4 (`case x = 5:`, `x += 1`, `x \|\|= 5`, formal def) |
| 2 | sol #1, Opus W2 multi-field prefix swap | fixed 1bbba2c: JS field D only for `base.f` (single field) | `two_field_swap` x3 (`o.x.y`, `this.a.b`, `o.a.b.c`) |
| 3a | sol #2 `(eval)(..)` / Opus S1 | fixed: owner switch uses `js_ts_direct_eval_anywhere` | `paren_eval`, `escaped_eval` (`(eval)`); control `indirect_eval_control` (`(0, eval)`) still drops |
| 3b | sol #3 shorthand `arguments` (D), sol #4 / Opus W3 (U) | fixed: both rules off when `js_ts_mapped_arguments_possible` | `shorthand_arguments`, `mapped_arguments_u` (`a = arguments[0]`, `cb = arguments[0] \|\| noop`); controls `strict_arguments_control`, `nonsimple_arguments_control`, `strict_arguments_d_control` still drop |
| 3c | sol #5 inherited `with` (Opus S1 `within_with`) | fixed: `inside_with_body` + `with_statement` in owner | `inherited_with_program_level` RED. `outer_with` (the review's exact input, function inside a `with` body inside `outer`) is **not RED**: ffcffdc1 already kept that pair (the edge is attributed to `outer`, which contains a `with`); kept as a guard row |
| 3d | Opus W3 escaped spelling | fixed: census `precondition` refusal | `escaped_owner` RED. Opus's own input (`sink(a); a = src();`) is **not reproducible**: the rvalue collector records the read, so ffcffdc1 keeps the pair |
| 4 | Opus S2 `stable_bases` scope | fixed: declarator must be in the owner's own scope (no nested callable/static block; `let`/`const` only directly in the owner's body block); also removed a nested callable's default-formal admission | `nested_declarator_not_stable`, `nested_block_not_stable`; control `own_declarator_control` still drops |
| 5 | Opus S3 label goldens | fixed: rows keep their labels again (`recorded_kill_tests`: `equal_path_kill` now `o.a.x`, the two loop rows `update.b.c`/`o.b.c`, `unflagged_route` keeps `update.b` with a read on the else line; `s1c_label_tests` `killed_edge_keeps_its_label` uses `[a] = [3]`). Note: a read between the writes does **not** help, D drops across any direct dominating write, so I used paths D keeps | rows pass with original labels; `GONE` expectations removed |
| 6 | sol #7, Opus S4 hazard fixtures | fixed: `h_fn_same` and `h_no_init` use `var`; `h_annexb` uses `var x` (line counts unchanged, all 150 old rows pass) | n/a (fixture validity) |
| 7a | docs | fixed: `js_prune.rs` module doc and the CLAUDE.md JS paragraph match the code (items 1-4, getters/setters/Proxy line in the module doc) | n/a |
| 7b | survivor cap (sol #6) | **disclosed, not changed** (controller disposition): CLAUDE.md JS paragraph now carries the #634 sentence. Not repeated in the module doc | n/a |
| 7c | report's 53/62 "D/exact" claim | withdrawn, see below: none of them is D | n/a |
| S5 | Opus: samples not independent | disclosed: the heuristic split below is the same kind of check; the controller's excalidraw and my TypeScript read-through find no real D/exact row | n/a |

## Measurement

### Against main 9f69debf
| corpus | main edges | removed | added | kept-label transitions |
|--|--|--|--|--|
| express | | 246 | 0 | 0 |
| excalidraw | | 5612 | 0 | 0 |
| TypeScript src | | 20899 | 0 | 0 |
| vitess (control) | | 80 | 0 | 0 |
| django (control) | | 289 | 0 | 0 |

`add2.py` multiset diff (per-corpus JSON in my scratch, not pushed). `call-stats` identical outside `dfg_labels` on all five (main == head, and
ffcffdc1 == head). Controls: only JS-family files change. vitess `.ts` 1161->1147, `.tsx` 3901->3835; django `.js` 3035->2746; unchanged:
vitess go 1,556,061 / java 9,227 / py 2,880 / sh 536, django py 184,281. (The `dfg_pruned_*` counters also move on vitess/django because
they count Go/Python prunes that main already had plus the JS ones; the JS-only effect is the per-extension table.)

### Against ffcffdc1 (fold may only restore)
| corpus | head minus ffcffdc1 (restored) | ffcffdc1 minus head (newly removed) |
|--|--|--|
| express | 0 | **4** |
| excalidraw | 0 | **1** |
| TypeScript src | 0 | 0 |
| vitess | 0 | 0 |
| django | 0 | 0 |

**Stop-and-explain: 5 newly removed edges.** All five are D drops in owners that mention `arguments` and are **strict**: express
`lib/request.js` `arr` 270->275, 270->276 (`req.is`, file has `'use strict'`, `var arr = types; if (..) { arr = new Array(arguments.length); ..}`) and
`lib/response.js` `address` 839->826, 839->830 (`res.redirect`, strict file, `arguments.length`/`arguments[0]` in the owner); excalidraw
`woff2-bindings.ts` `name` 2121->2151 (an ES module, `arguments` in an enclosing function). ffcffdc1 switched D off for any syntactic `arguments`;
the fold's `js_ts_mapped_arguments_possible` is false for strict code, so D is on again there, which is exactly the control the brief asks for
(item 3: "U/D still drop in a strict-mode or ES-module owner that mentions `arguments` but cannot map it"). Each is a plain dominating
write (e.g. `arr = new Array(..)` directly in the `if` block before the nested loop's use). They are removals relative to ffcffdc1 but still
only removals relative to main (0 added, 0 label transitions). I kept them rather than refusing, because refusing would contradict the item-3
control; flag if the controller prefers the stricter reading. Restored edges are 0 on every corpus: none of items 1-4 occurs in these trees.

### Ledger (fold head)
Counters (`dfg_pruned_write_only` / `dfg_pruned_dominated`), ffcffdc1 -> head: express 158/84 -> 158/88; excalidraw 4204/1468 -> 4204/1469;
TypeScript 14290/6677 -> 14290/6677. (Counters count expanded edges before dedup, so they differ from the deduped removed edges.)

Heuristic split of removed edges (use-line text, a field-path write target `a.b = ..` counted as a write of that path):
| corpus | removed | U | D | D by field depth |
|--|--|--|--|--|
| express | 246 | 156 | 90 | depth 0: 89, depth 1: 1 |
| excalidraw | 5612 | 4126 | 1486 | depth 0: 1481, depth 1: 5 |
| TypeScript src | 20899 | 14143 | 6756 | depth 0: 6735, depth 1: 21 |

Field-path D drops are depth 1 only, none at depth 2 or more, as expected.

### D/exact reclassification
Removed `exact` edges classified D by the improved heuristic: express 0, excalidraw 1, TypeScript 3 (earlier: 53 and 62). I read each in source:
excalidraw `subset-shared.chunk.ts` `base64` 66->70 (`base64 = Buffer.from(..).toString("base64");`), TypeScript `program.ts` `details` 4489->4491
(`details = chainDiagnosticMessages(/*details*/ ..)`), `sys.ts` `timerToUpdateChildWatches` 768->770 (`x = setTimeout(.., "timerToUpdateChildWatches")`),
`editorServices.ts` `project` 5352->5355 (`project = this.create...(.., \`Creating configured project ..\`)`). All four are write-only lines
(U) whose name also appears inside a comment or string literal, which fooled the text heuristic. **No true D/exact row remains**; the earlier "D/exact" claim is withdrawn, the 53/62 were U.

### Timing (TypeScript src, `nav --no-cache dfg-stats`, RAYON 4, interleaved M H M H)
main 43.20 / 42.65 s, fold head 45.24 / 43.53 s (mean 42.92 vs 44.38, +3.4%). One pair each; noise on this host was 1-2 s in the first report
(38.9/38.6 vs 38.3/39.4), so +3% is within what I would call noise but I cannot rule out a small cost from the heavier helpers
(`js_ts_mapped_arguments_possible`, census `precondition`, per owner). Not measured further.

## Not done / caveats
- cli, integration and prism-mcp test targets not run; mutation gate not run.
- The 2 failing lib tests are unattributed on main (root-permission tests).
- Samples: no new 40/40 sample was drawn; the corpus evidence is the diff against main and ffcffdc1 above.

Progress: DONE
