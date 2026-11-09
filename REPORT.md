# #630 slice 5 — JS/TS/TSX rules U and D (branch `perf/630-ud-js`)

Status: measured on all five corpora. Base origin/main 9f69debf, cache v168 -> v169.

## Step 0 — grounding (written before coding)
1. **Owners reaching the producer's def/use loop.** Every callable `dfg_owner_name` names: *named* owners
   (`function_declaration`, `method_definition`, `generator_function_declaration`, and an arrow or function
   expression whose name is inferred) run the legacy line-based pass; *anonymous* arrows/function/generator
   expressions run a *synthetic* `<anon@L:C>` pass. Module level is not an owner (`all_functions` has no program node).
2. **Line-anchored or exact?** Named owners are line-anchored: `get_use` returns an existing exact rvalue span on the
   line, else mints a zero-width anchor at the line start (baseline dump shows `start_byte == end_byte`, e.g. 19,19).
   Synthetic owners build each Use at the exact admitted span. U and D are therefore enabled for **named owners only**;
   synthetic and module level stay off (they are a different code path with exact spans, so no extra hazard needed
   to include them, but the brief says leave them).
3. **JS-only edge sources.** The parameter projections (`Def(p) -> Use(p.<path>)`, member-only formals) are built after
   the lexical-filter loops and call neither `drop_write_only` nor `keep_def`: D does not see them. The #594
   object-literal shortcut is derived at CPG level from definition edges that reach the callee's reads, so it
   sees the pruned set (a read after a dominating reassignment gets no shortcut edge; test updated).

## Design as implemented (`src/data_flow/build/js_prune.rs`, hooks in `lexical_prune.rs`)
- Regions: `statement_block` (not a namespace body), `switch_case`/`switch_default`; static-block and `finally`
  bodies are blocks. d*: `x = ..`/non-logical `op=` statement, or one-initialised-declarator declaration.
- Excluded as d*: `var x;`, destructuring, `for` heads, `catch`, nested assignments, logical assignment, `x++`, nested callables.
- Class field initialisers are nested scopes. Alias twins are never d*.
- Hazards: `with`/direct `eval` switch U and D off; `arguments` switches D off. Hoisted functions (Annex B / same name),
  `var` redeclaration, labels, generators/async: no switch needed — D only drops defs that a dominating write
  provably overwrote, so extra unmodelled earlier writers cannot resurrect them; a label only targets enclosing
  statements; `yield`/`await` resume in place.
- Fields: identical paths only (no prefix kills), only for a stable base (`this`, or a once-bound, never-written name).

## Tests
- `tests/fixtures/dfg_prune/js/*` + `obligations.jsonl` (150 rows): 63 drop rows fail on the pre-implementation head
  (RED commit), all pass now; frozen-producer differential: `edges_after ⊆ edges_before`, labels equal, removed count == counters.
- Fixture-tree differential extended to the JS family (subset-only + dominator invariant).
- Goldens changed, each removed edge is a U or D drop: census `w06`, recorded-kill control rows (4), s1c killed row,
  optional-parameter rows, nested-owner anchors (`cb` lines 2,3), r5 named-seam goldens (listed in
  `js_param_r5_u_d_removed.json`, 23 cases: `var f;` / pure-write lines and dominated defs), synthetic-vs-named E4 rows,
  object-literal reassignment row, `dfg_label_store_test` partial-hit row.
- Environment failures (not mine): 5 `owner_activation_test::owner_admission_*`, and
  `cpg_cache::stream::tests::a_write_that_cannot_start_fails..` (permission-denied write; root defeats it; not re-run on main).

## Corpus results

| corpus | main edges | head edges | removed | added | kept-label transitions |
|--|--|--|--|--|--|
| express | 7691 | 7449 | 242 | 0 | 0 |
| excalidraw | 85645 | 80034 | 5611 | 0 | 0 |
| TypeScript | 408954 | 388055 | 20899 | 0 | 0 |
| vitess | 1573769 | 1573689 | 80 | 0 | 0 |
| django | 187316 | 187027 | 289 | 0 | 0 |

- **Added edges: 0 on every corpus. Kept-label transitions: 0 on every corpus** (`add2.py`, multiset diff: nothing added, every
  surviving edge keeps its label; `add2.py` was run main-vs-head and the removed count is `main_only`).
- **`call-stats`**: identical outside `dfg_labels` on all five corpora (checked key by key; only the label/prune counters in
  `dfg_labels` move). So CPG Variable nodes and Step 5c return-flow counters are unchanged.
- **Controls (vitess, django) are NOT byte-identical, by content:** both trees contain JS/TS/TSX
  (vitess: ts 1161->1147 and tsx 3901->3835 edges; django: js 3035->2746 in admin static files). Unchanged per-extension edge counts:
  vitess go 1,556,061 / py 2,880 / java 9,227 / sh 536 / js 3; django py 184,281. Removed 80 (vitess) / 289 (django), added 0, all
  in JS-family files. `dfg-stats`/`call-stats` differ only in the `dfg_labels` counters for the same reason. A pure Go or Python
  control was not among the assigned corpora.
- **Ledger** (rule split is a heuristic on the use-line text: a line whose only occurrences of the name are write targets or
  `var x;` is U, else D; the producer's counters are authoritative):

| corpus | removed | U (heur.) | D (heur.) | `dfg_pruned_write_only` | `dfg_pruned_dominated` |
|--|--|--|--|--|--|
| express | 242 | 156 | 86 | 158 | 84 |
| excalidraw | 5611 | 3916 | 1695 | 4204 | 1468 |
| TypeScript src | 20899 | 14011 | 6888 | 14290 | 6677 |

  (Counters count expanded edges before compaction/dedup, so they differ from the heuristic split of deduped removed edges.)
  By label, TypeScript: cfg_incomplete 14362, killed 3197, exact 2412, alias_unstable 916, sameline 12. The D/exact removals
  (62 TypeScript, 53 excalidraw) are exact-labelled edges into a read whose value a dominating write overwrote; the sampled ones were RIGHT.
- **Top functions by drops:** the edge dump has no function id, so the ranking is per file (top-20 in each analysis below);
  TypeScript: scanner.ts 2713, parser.ts 1531, fourslashImpl.ts 1003, utilities.ts 959, binder.ts 953.
- **Timing (TypeScript src, `nav --no-cache dfg-stats`, RAYON 4, interleaved M H M H):** main 38.90 / 38.58 s, head 38.33 / 39.35 s. Neutral.

## Samples (seed 750: 30 random + 10 from top-20 files per corpus)
Verdicts: **40/40 RIGHT on each of express, excalidraw and TypeScript; 0 WRONG.** Method, stated honestly: U rows are RIGHT by
construction (the use line only writes the name; checked mechanically from the line text). D rows were checked mechanically
(a direct dominating write exists between def and use) and, for every row the mechanism could not confirm (`D-manual`: def after use,
or a field path) and for loop-prone ones (scanner `pos`, preProcess `token`, fourslash `text`), by reading the source: each is a def
after the use with no enclosing loop that carries it past the dominating write, or a def before a direct dominating write. Not every
`D-mech` row was read line by line; those rest on the mechanical dominator check plus the structural proof. For excalidraw I read the
D-manual rows only superficially (newElement `y +=`, clients `context.lineWidth`): same shapes as above.

### express: removed 242 (main_edges 7691, head_edges 7449), added 0
by rule (heuristic from source text): {'D': 86, 'U': 156}
by label: {'nameonly/killed': 44, 'nameonly/cfg_incomplete': 161, 'exact/None': 37}
by rule x confidence: {'D/nameonly': 86, 'U/exact': 37, 'U/nameonly': 119}
top-20 files by drops (function ids are not in the edge dump): [('lib/response.js', 155), ('lib/application.js', 28), ('lib/utils.js', 22), ('lib/view.js', 16), ('test/express.urlencoded.js', 7), ('lib/request.js', 6), ('examples/route-map/index.js', 3), ('test/support/tmpl.js', 3), ('examples/view-constructor/github-view.js', 2)]

| # | file | path | def | use | label | rule | check | def line | use line |
|--|--|--|--|--|--|--|--|--|--|
| 1 | lib/response.js | `chunk` | 201 | 168 | nameonly/cfg_incomplete | D | D-manual | `chunk = '';` | `if (chunk !== undefined && !this.get('Transfer-Encoding')) {` |
| 2 | lib/response.js | `callback` | 279 | 269 | nameonly/cfg_incomplete | D | D-manual | `callback = callback[0];` | `var callback = this.req.query[app.get('jsonp callback name')` |
| 3 | lib/request.js | `val` | 420 | 423 | exact/None | U | U-mech | `var val = this.get('X-Forwarded-Host');` | `val = this.get('Host');` |
| 4 | lib/response.js | `len` | 174 | 179 | nameonly/cfg_incomplete | U | U-mech | `len = Buffer.byteLength(chunk, encoding)` | `len = chunk.length` |
| 5 | lib/response.js | `done` | 374 | 390 | exact/None | U | U-mech | `var done = callback;` | `done = options;` |
| 6 | lib/response.js | `opts` | 447 | 475 | nameonly/killed/454 | D | D-mech(dominator between) | `opts = null` | `opts.headers = headers` |
| 7 | lib/application.js | `view` | 552 | 545 | nameonly/cfg_incomplete | U | U-mech | `view = new View(name, {` | `view = cache[name];` |
| 8 | test/express.urlencoded.js | `str` | 789 | 799 | nameonly/killed/795 | D | D-mech(dominator between) | `var str = ''` | `str += '&' + n + '=' + n` |
| 9 | lib/application.js | `done` | 524 | 531 | exact/None | U | U-mech | `var done = callback;` | `done = options;` |
| 10 | lib/application.js | `view` | 545 | 560 | nameonly/killed/552 | D | D-mech(dominator between) | `view = cache[name];` | `? 'directories "' + view.root.slice(0, -1).join('", "') + '"` |
| 11 | lib/response.js | `opts` | 474 | 447 | nameonly/cfg_incomplete | U | U-mech | `opts = Object.create(opts)` | `opts = null` |
| 12 | lib/response.js | `body` | 854 | 817 | nameonly/cfg_incomplete | U | U-mech | `body = '';` | `var body;` |
| 13 | lib/response.js | `chunk` | 201 | 149 | nameonly/cfg_incomplete | D | D-manual | `chunk = '';` | `if (chunk === null) {` |
| 14 | lib/response.js | `opts` | 474 | 438 | nameonly/cfg_incomplete | U | U-mech | `opts = Object.create(opts)` | `var opts = options || null` |
| 15 | lib/response.js | `chunk` | 201 | 208 | nameonly/killed/208 | U | U-mech | `chunk = '';` | `chunk = ''` |
| 16 | lib/response.js | `chunk` | 177 | 150 | nameonly/cfg_incomplete | U | U-mech | `chunk = Buffer.from(chunk, encoding)` | `chunk = '';` |
| 17 | lib/application.js | `view` | 545 | 561 | nameonly/killed/552 | D | D-mech(dominator between) | `view = cache[name];` | `: 'directory "' + view.root + '"'` |
| 18 | lib/response.js | `chunk` | 208 | 179 | nameonly/cfg_incomplete | D | D-manual | `chunk = ''` | `len = chunk.length` |
| 19 | lib/view.js | `stat` | 174 | 184 | nameonly/killed/182 | D | D-mech(dominator between) | `var stat = tryStat(path);` | `if (stat && stat.isFile()) {` |
| 20 | lib/response.js | `chunk` | 208 | 169 | nameonly/cfg_incomplete | D | D-manual | `chunk = ''` | `if (Buffer.isBuffer(chunk)) {` |
| 21 | lib/application.js | `view` | 527 | 558 | nameonly/killed/545 | D | D-mech(dominator between) | `var view;` | `if (!view.path) {` |
| 22 | lib/response.js | `done` | 436 | 446 | exact/None | U | U-mech | `var done = callback;` | `done = options` |
| 23 | lib/response.js | `opts` | 454 | 438 | nameonly/cfg_incomplete | U | U-mech | `opts = filename` | `var opts = options || null` |
| 24 | lib/response.js | `encoding` | 128 | 137 | exact/None | U | U-mech | `var encoding;` | `encoding = 'utf8';` |
| 25 | lib/response.js | `encoding` | 137 | 128 | nameonly/cfg_incomplete | U | U-mech | `encoding = 'utf8';` | `var encoding;` |
| 26 | lib/utils.js | `index` | 116 | 92 | nameonly/cfg_incomplete | U | U-mech | `index = endIndex + 1;` | `var index = colonIndex === -1 ? length : colonIndex;` |
| 27 | lib/application.js | `done` | 531 | 524 | nameonly/cfg_incomplete | U | U-mech | `done = options;` | `var done = callback;` |
| 28 | lib/response.js | `callback` | 269 | 302 | nameonly/killed/279 | D | D-mech(dominator between) | `var callback = this.req.query[app.get('jsonp callback name')` | `body = '/**/ typeof ' + callback + ' === \'function\' && ' +` |
| 29 | lib/response.js | `opts` | 447 | 444 | nameonly/cfg_incomplete | U | U-mech | `opts = null` | `opts = null` |
| 30 | lib/response.js | `opts` | 438 | 478 | nameonly/killed/444 | D | D-mech(dominator between) | `var opts = options || null` | `var fullPath = !opts.root` |
| 31 | lib/response.js | `callback` | 279 | 302 | nameonly/killed/288 | D | D-mech(dominator between) | `callback = callback[0];` | `body = '/**/ typeof ' + callback + ' === \'function\' && ' +` |
| 32 | lib/response.js | `opts` | 447 | 483 | nameonly/killed/454 | D | D-mech(dominator between) | `opts = null` | `return this.sendFile(fullPath, opts, done)` |
| 33 | lib/response.js | `callback` | 288 | 279 | nameonly/cfg_incomplete | D | D-manual | `callback = callback.replace(/[^\[\]\w$.]/g, '');` | `callback = callback[0];` |
| 34 | lib/application.js | `opts` | 526 | 532 | exact/None | U | U-mech | `var opts = options || {};` | `opts = {};` |
| 35 | lib/response.js | `opts` | 454 | 475 | nameonly/killed/474 | D | D-mech(dominator between) | `opts = filename` | `opts.headers = headers` |
| 36 | lib/application.js | `view` | 545 | 560 | nameonly/killed/552 | D | D-mech(dominator between) | `view = cache[name];` | `? 'directories "' + view.root.slice(0, -1).join('", "') + '"` |
| 37 | lib/response.js | `len` | 179 | 171 | nameonly/cfg_incomplete | U | U-mech | `len = chunk.length` | `len = chunk.length` |
| 38 | lib/response.js | `opts` | 447 | 454 | exact/None | U | U-mech | `opts = null` | `opts = filename` |
| 39 | lib/response.js | `streaming` | 987 | 926 | nameonly/cfg_incomplete | U | U-mech | `streaming = true;` | `var streaming;` |
| 40 | lib/utils.js | `fn` | 145 | 140 | nameonly/cfg_incomplete | U | U-mech | `fn = exports.etag;` | `fn = exports.wetag;` |


### excalidraw: removed 5611 (main_edges 85645, head_edges 80034), added 0
by rule (heuristic from source text): {'U': 3916, 'D': 1695}
by label: {'exact/None': 763, 'nameonly/cfg_incomplete': 3989, 'nameonly/alias_unstable': 105, 'nameonly/killed': 749, 'nameonly/sameline': 5}
by rule x confidence: {'U/exact': 710, 'U/nameonly': 3206, 'D/nameonly': 1642, 'D/exact': 53}
top-20 files by drops (function ids are not in the edge dump): [('packages/element/src/binding.ts', 831), ('packages/element/src/cropElement.ts', 652), ('packages/excalidraw/subset/woff2/woff2-bindings.ts', 328), ('packages/element/src/embeddable.ts', 257), ('packages/excalidraw/components/App.tsx', 207), ('packages/excalidraw/clients.ts', 198), ('packages/element/src/transform.ts', 153), ('packages/excalidraw/tests/helpers/api.ts', 131), ('packages/element/src/resizeElements.ts', 124), ('packages/element/src/textWrapping.ts', 123), ('packages/element/src/bounds.ts', 120), ('packages/element/src/flowchart.ts', 103), ('packages/excalidraw/wysiwyg/textWysiwyg.tsx', 86), ('packages/excalidraw/data/restore.ts', 84), ('packages/element/src/linearElementEditor.ts', 79), ('packages/element/src/zindex.ts', 76), ('packages/element/src/delta.ts', 71), ('packages/element/src/newElement.ts', 69), ('packages/excalidraw/components/Stats/Dimension.tsx', 66), ('packages/math/src/curve.ts', 66)]

| # | file | path | def | use | label | rule | check | def line | use line |
|--|--|--|--|--|--|--|--|--|--|
| 1 | packages/element/src/resizeElements.ts | `scale` | 1317 | 1278 | nameonly/cfg_incomplete | U | U-mech | `scaleY = scale;` | `scale =` |
| 2 | packages/excalidraw/actions/actionProperties.tsx | `nextFontFamily` | 963 | 956 | nameonly/cfg_incomplete | U | U-mech | `nextFontFamily = currentHoveredFontFamily;` | `let nextFontFamily: FontFamilyValues | undefined;` |
| 3 | packages/element/src/bounds.ts | `currentP` | 654 | 641 | nameonly/cfg_incomplete | U | U-mech | `currentP = _p3;` | `currentP = p;` |
| 4 | packages/element/src/embeddable.ts | `link` | 171 | 243 | nameonly/killed/182 | D | D-mech(dominator between) | `export const getEmbedLink = (` | `link,` |
| 5 | packages/excalidraw/components/App.tsx | `restoredAppState` | 2952 | 2988 | nameonly/killed/2963 | D | D-mech(dominator between) | `let restoredAppState = restoreAppState(initialData?.appState` | `...restoredAppState,` |
| 6 | packages/excalidraw/snapping.ts | `endBounds` | 413 | 410 | nameonly/sameline | U | U-mech | `const [endMinX, endMinY, endMaxX] = endBounds;` | `const endBounds = verticallySorted[j];` |
| 7 | packages/element/src/binding.ts | `current` | 541 | 569 | nameonly/cfg_incomplete | U | U-mech | `current = { element: hit, mode: "inside", focusPoint: point ` | `current = {` |
| 8 | packages/element/src/binding.ts | `y` | 2702 | 2715 | nameonly/cfg_incomplete | U | U-mech | `y = midPoint[1];` | `y = midPoint[1] - OFFSET;` |
| 9 | packages/element/src/binding.ts | `x` | 2896 | 2851 | nameonly/cfg_incomplete | U | U-mech | `x = midPoint[0] + OFFSET * 0.707;` | `x = midPoint[0];` |
| 10 | packages/excalidraw/tests/helpers/api.ts | `element` | 303 | 316 | nameonly/cfg_incomplete | U | U-mech | `element = newTextElement({` | `element = newFreeDrawElement({` |
| 11 | packages/element/src/newElement.ts | `y` | 410 | 401 | nameonly/cfg_incomplete | D | D-manual | `y += deltaY2 * (1 + cos);` | `y += deltaX2 * sin;` |
| 12 | packages/excalidraw/tests/data/reconcile.test.ts | `id` | 37 | 33 | nameonly/cfg_incomplete | U | U-mech | `({ id, version, versionNonce } = opts);` | `id = match[1];` |
| 13 | packages/element/src/store.ts | `scheduledAction` | 402 | 399 | nameonly/cfg_incomplete | U | U-mech | `scheduledAction = CaptureUpdateAction.EVENTUALLY;` | `scheduledAction = CaptureUpdateAction.NEVER;` |
| 14 | packages/element/src/embeddable.ts | `link` | 232 | 289 | nameonly/cfg_incomplete | D | D-mech(dominator between) | `link = `https://player.vimeo.com/video/${target}?api=1`;` | `link,` |
| 15 | packages/element/src/binding.ts | `y` | 2912 | 2858 | nameonly/cfg_incomplete | U | U-mech | `y = midPoint[1] + OFFSET * 0.707;` | `y = midPoint[1];` |
| 16 | packages/element/src/zindex.ts | `leadingIndex` | 500 | 496 | nameonly/cfg_incomplete | U | U-mech | `leadingIndex = 0;` | `leadingIndex = indicesToMove[0];` |
| 17 | packages/element/src/zindex.ts | `selectedIndices` | 59 | 43 | nameonly/cfg_incomplete | U | U-mech | `selectedIndices = selectedIndices.concat(deletedIndices);` | `let selectedIndices: number[] = [];` |
| 18 | packages/element/src/transform.ts | `excalidrawElement` | 610 | 541 | nameonly/cfg_incomplete | U | U-mech | `excalidrawElement = newFrameElement({` | `excalidrawElement = newElement({` |
| 19 | packages/element/src/binding.ts | `x` | 2739 | 2714 | nameonly/cfg_incomplete | U | U-mech | `x = midPoint[0] + OFFSET * 0.707;` | `x = midPoint[0];` |
| 20 | packages/excalidraw/components/AppStateObserver.ts | `predicate` | 91 | 118 | exact/None | U | U-mech | `let predicate: StateChangeListener["predicate"];` | `predicate = (appState: AppState, prevState: AppState) =>` |
| 21 | packages/excalidraw/subset/woff2/woff2-bindings.ts | `invokerFnBody` | 2363 | 2401 | nameonly/killed/2383 | D | D-mech(dominator between) | `invokerFnBody += "var destructors = [];\n";` | `invokerFnBody += "runDestructors(destructors);\n";` |
| 22 | packages/element/src/cropElement.ts | `nextHeight` | 283 | 342 | nameonly/cfg_incomplete | U | U-mech | `nextHeight = nextWidth / widthAspectRatio;` | `nextHeight = nextWidth / widthAspectRatio;` |
| 23 | packages/element/src/cropElement.ts | `nextHeight` | 312 | 220 | nameonly/cfg_incomplete | U | U-mech | `nextHeight = nextWidth / widthAspectRatio;` | `nextHeight = clamp(` |
| 24 | packages/element/src/binding.ts | `y` | 2805 | 2829 | nameonly/cfg_incomplete | U | U-mech | `y = ellipseCenterY + ellipseY - OFFSET * 0.707;` | `y = ellipseCenterY + ellipseY - OFFSET * 0.707;` |
| 25 | packages/excalidraw/actions/actionProperties.tsx | `ret` | 216 | 200 | nameonly/cfg_incomplete | U | U-mech | `ret =` | `let ret: T | null = null;` |
| 26 | packages/excalidraw/renderer/staticSvgScene.ts | `uncroppedWidth` | 459 | 456 | nameonly/cfg_incomplete | U | U-mech | `({ width: uncroppedWidth, height: uncroppedHeight } =` | `let uncroppedWidth = element.width;` |
| 27 | packages/excalidraw/components/Stats/MultiDimension.tsx | `nextWidth` | 299 | 318 | nameonly/killed/314 | D | D-mech(dominator between) | `nextWidth = getStepSizedValue(nextWidth, STEP_SIZE);` | `nextWidth,` |
| 28 | packages/excalidraw/clients.ts | `context.lineWidth` | 112 | 135 | nameonly/killed/119 | D | D-manual | `context.lineWidth = 3;` | `context.lineWidth = 10;` |
| 29 | packages/excalidraw/subset/woff2/woff2-bindings.ts | `attachFinalizer` | 1543 | 1558 | nameonly/cfg_incomplete | U | U-mech | `attachFinalizer = function (handle) {` | `attachFinalizer = function (handle) {` |
| 30 | packages/excalidraw/clients.ts | `y` | 72 | 177 | nameonly/killed/77 | D | D-mech(dominator between) | `let { x, y } = pointer;` | `context.lineTo(x + 4, y + 9);` |
| 31 | packages/element/src/binding.ts | `y` | 2715 | 2759 | nameonly/cfg_incomplete | U | U-mech | `y = midPoint[1] - OFFSET;` | `y = midPoint[1] - OFFSET * 0.707;` |
| 32 | packages/element/src/transform.ts | `excalidrawElement` | 564 | 552 | nameonly/cfg_incomplete | U | U-mech | `excalidrawElement = newArrowElement({` | `excalidrawElement = newLinearElement({` |
| 33 | packages/element/src/binding.ts | `x` | 2796 | 2828 | nameonly/cfg_incomplete | U | U-mech | `x = ellipseCenterX - radiusX - OFFSET;` | `x = ellipseCenterX + ellipseX - OFFSET * 0.707;` |
| 34 | packages/element/src/cropElement.ts | `nextHeight` | 267 | 301 | nameonly/cfg_incomplete | D | D-mech(dominator between) | `nextHeight = clamp(` | `nextWidth = nextHeight * widthAspectRatio;` |
| 35 | packages/element/src/textWrapping.ts | `currentLineWidth` | 543 | 498 | nameonly/cfg_incomplete | U | U-mech | `currentLineWidth = getLineWidth(trailingLine.text, font);` | `let currentLineWidth = 0;` |
| 36 | packages/excalidraw/components/Stats/Dimension.tsx | `nextHeight` | 242 | 253 | nameonly/killed/248 | D | D-mech(dominator between) | `nextHeight = Math.round((nextWidth / aspectRatio) * 100) / 1` | `nextHeight,` |
| 37 | packages/excalidraw/subset/woff2/woff2-bindings.ts | `ptr` | 1768 | 1795 | nameonly/cfg_incomplete | U | U-mech | `ptr = upcastPointer(handle.$$.ptr, handleClass, this.registe` | `ptr = this.rawShare(` |
| 38 | packages/excalidraw/subset/woff2/woff2-bindings.ts | `attachFinalizer` | 1543 | 1558 | nameonly/cfg_incomplete | U | U-mech | `attachFinalizer = function (handle) {` | `attachFinalizer = function (handle) {` |
| 39 | packages/excalidraw/clients.ts | `y` | 90 | 171 | nameonly/killed/72 | D | D-mech(dominator between) | `y = Math.max(y, 0);` | `context.lineTo(x + 12, y + 9);` |
| 40 | packages/element/src/cropElement.ts | `nextHeight` | 342 | 356 | nameonly/cfg_incomplete | U | U-mech | `nextHeight = nextWidth / widthAspectRatio;` | `nextHeight = clamp(` |


### TypeScript: removed 20899 (main_edges 408954, head_edges 388055), added 0
by rule (heuristic from source text): {'U': 14011, 'D': 6888}
by label: {'nameonly/alias_unstable': 916, 'exact/None': 2412, 'nameonly/killed': 3197, 'nameonly/cfg_incomplete': 14362, 'nameonly/sameline': 12}
by rule x confidence: {'U/nameonly': 11661, 'D/nameonly': 6826, 'U/exact': 2350, 'D/exact': 62}
top-20 files by drops (function ids are not in the edge dump): [('src/compiler/scanner.ts', 2713), ('src/compiler/parser.ts', 1531), ('src/harness/fourslashImpl.ts', 1003), ('src/compiler/utilities.ts', 959), ('src/compiler/binder.ts', 953), ('src/compiler/program.ts', 747), ('src/services/completions.ts', 738), ('src/services/preProcess.ts', 719), ('src/harness/tsserverLogger.ts', 586), ('src/compiler/transformers/classFields.ts', 487), ('src/compiler/transformers/declarations.ts', 411), ('src/compiler/transformers/esDecorators.ts', 411), ('src/compiler/transformers/es2015.ts', 389), ('src/services/classifier.ts', 339), ('src/services/symbolDisplay.ts', 334), ('src/compiler/emitter.ts', 325), ('src/compiler/factory/nodeFactory.ts', 284), ('src/harness/harnessIO.ts', 274), ('src/compiler/transformers/generators.ts', 270), ('src/compiler/transformers/declarations/diagnostics.ts', 235)]

| # | file | path | def | use | label | rule | check | def line | use line |
|--|--|--|--|--|--|--|--|--|--|
| 1 | src/compiler/transformers/esDecorators.ts | `propertyName` | 1304 | 1309 | nameonly/cfg_incomplete | U | U-mech | `propertyName = { computed: true, name: factory.createStringL` | `propertyName = { computed: true, name: referencedName };` |
| 2 | src/harness/fourslashImpl.ts | `text` | 4298 | 4329 | nameonly/killed/4299 | D | D-mech(dominator between) | `text += `${prefix}├ selectionSpan:\n`;` | `text += `${prefix}│ ╭ from:\n`;` |
| 3 | src/compiler/program.ts | `packageMap` | 2017 | 1555 | nameonly/cfg_incomplete | U | U-mech | `packageMap = new Map();` | `let packageMap: Map<string, boolean> | undefined;` |
| 4 | src/compiler/scanner.ts | `token` | 3894 | 1048 | nameonly/cfg_incomplete | U | U-mech | `return token = SyntaxKind.BacktickToken;` | `var token: SyntaxKind;` |
| 5 | src/harness/tsserverLogger.ts | `s` | 130 | 127 | nameonly/cfg_incomplete | D | D-manual | `s = s.replace(/getIndentationAtPosition: getCurrentSourceFil` | `s = s.replace(/continuePreviousIncompleteResponse: \d+(?:\.\` |
| 6 | src/services/preProcess.ts | `token` | 238 | 260 | nameonly/killed/240 | U | U-mech | `token = nextToken();` | `token = nextToken();` |
| 7 | src/compiler/parser.ts | `tag` | 9171 | 9109 | nameonly/cfg_incomplete | U | U-mech | `tag = parseSeeTag(start, tagName, margin, indentText);` | `tag = parseImplementsTag(start, tagName, margin, indentText)` |
| 8 | src/compiler/core.ts | `ch` | 2292 | 2297 | exact/None | U | U-mech | `ch = fileName.charCodeAt(pos);` | `ch = fileName.charCodeAt(pos);` |
| 9 | src/compiler/parser.ts | `propertyName` | 8643 | 8670 | nameonly/cfg_incomplete | U | U-mech | `propertyName = name;` | `propertyName = name;` |
| 10 | src/testRunner/unittests/helpers/tscWatch.ts | `programs` | 151 | 164 | exact/None | U | U-mech | `let programs = watchBaseline({` | `programs = watchBaseline({` |
| 11 | src/compiler/transformers/es2017.ts | `catchClauseUnshadowedNames` | 335 | 321 | nameonly/cfg_incomplete | U | U-mech | `enclosingFunctionParameterNames = savedEnclosingFunctionPara` | `let catchClauseUnshadowedNames: Set<__String> | undefined;` |
| 12 | src/testRunner/projectsRunner.ts | `value` | 478 | 483 | exact/None | U | U-mech | `let value = testCase[name] as any;` | `value = optTypeValue;` |
| 13 | src/compiler/transformers/module/system.ts | `node` | 1297 | 1301 | exact/None | U | U-mech | `function visitForStatement(node: ForStatement, isTopLevel: b` | `node = factory.updateForStatement(` |
| 14 | src/compiler/transformer.ts | `statements` | 525 | 553 | exact/None | U | U-mech | `let statements: Statement[] | undefined;` | `statements = [...lexicalEnvironmentStatements];` |
| 15 | src/compiler/parser.ts | `state` | 8985 | 8947 | nameonly/cfg_incomplete | U | U-mech | `state = JSDocState.SavingComments;` | `state = JSDocState.BeginningOfLine;` |
| 16 | src/compiler/watchPublic.ts | `builderProgram` | 697 | 555 | nameonly/cfg_incomplete | U | U-mech | `builderProgram = createProgram(rootFileNames, compilerOption` | `builderProgram = readBuilderProgram(compilerOptions, compile` |
| 17 | src/compiler/watchUtilities.ts | `newPath` | 592 | 586 | nameonly/cfg_incomplete | U | U-mech | `fileOrDirectoryPath = newPath;` | `const newPath = removeIgnoredPath(fileOrDirectoryPath);` |
| 18 | src/compiler/utilities.ts | `result` | 11760 | 11704 | nameonly/cfg_incomplete | U | U-mech | `result = argumentsSymbol;` | `result = location.symbol;` |
| 19 | src/compiler/executeCommandLine.ts | `output` | 480 | 494 | exact/None | U | U-mech | `let output: string[] = [...getHeader(sys, `${getDiagnosticTe` | `output = [` |
| 20 | src/server/scriptVersionCache.ts | `child` | 195 | 176 | nameonly/cfg_incomplete | U | U-mech | `child = fresh(lineCollection);` | `let child: LineCollection | undefined;` |
| 21 | src/services/symbolDisplay.ts | `tags` | 275 | 810 | nameonly/cfg_incomplete | U | U-mech | `let tags: JSDocTagInfo[] = [];` | `tags = tagsFromAlias;` |
| 22 | src/compiler/scanner.ts | `pos` | 1895 | 2091 | nameonly/cfg_incomplete | D | D-mech(dominator between) | `tokenStart = pos;` | `if (ch === CharacterCodes.asterisk && charCodeUnchecked(pos ` |
| 23 | src/compiler/scanner.ts | `pos` | 2011 | 2061 | nameonly/cfg_incomplete | D | D-mech(dominator between) | `return pos += 2, token = SyntaxKind.AsteriskAsteriskToken;` | `if (isLineBreak(charCodeUnchecked(pos))) {` |
| 24 | src/compiler/moduleNameResolver.ts | `currentDirectory` | 484 | 489 | exact/None | U | U-mech | `let currentDirectory: string | undefined;` | `currentDirectory = host.getCurrentDirectory();` |
| 25 | src/harness/fourslashImpl.ts | `text` | 4293 | 4299 | nameonly/killed/4295 | D | D-mech(dominator between) | `text += `${prefix}├ containerName: ${callHierarchyItem.conta` | `text += this.formatCallHierarchyItemSpan(` |
| 26 | src/services/preProcess.ts | `token` | 168 | 105 | nameonly/cfg_incomplete | D | D-manual | `token = nextToken();` | `if (token === SyntaxKind.StringLiteral || token === SyntaxKi` |
| 27 | src/services/codefixes/fixMissingTypeAnnotationOnExports.ts | `baseExpr` | 691 | 675 | nameonly/cfg_incomplete | U | U-mech | `baseExpr = { expression: { kind: ExpressionType.Identifier, ` | `baseExpr = { expression: { kind: ExpressionType.Identifier, ` |
| 28 | src/harness/fourslashImpl.ts | `text` | 4330 | 4290 | nameonly/cfg_incomplete | D | D-manual | `text += this.formatCallHierarchyItem(file, incomingCall.from` | `text += `${prefix}╭ name: ${callHierarchyItem.name}\n`;` |
| 29 | src/services/refactors/extractSymbol.ts | `call` | 1193 | 1187 | nameonly/cfg_incomplete | D | D-manual | `call = factory.createJsxExpression(/*dotDotDotToken*/ undefi` | `call = factory.createYieldExpression(factory.createToken(Syn` |
| 30 | src/harness/harnessIO.ts | `index` | 1118 | 1117 | exact/None | U | U-mech | `while ((index = ts.findIndex(variations, v => v.key === excl` | `let index: number;` |
| 31 | src/compiler/binder.ts | `parentNode` | 2856 | 2854 | nameonly/cfg_incomplete | U | U-mech | `parentNode = parentNode.parent;` | `let parentNode = node.parent;` |
| 32 | src/harness/fourslashImpl.ts | `baselineText` | 3728 | 3735 | nameonly/killed/3733 | D | D-mech(dominator between) | `baselineText += `### When marker text is \`${fullNameForCode` | `baselineText += `Fix all available: ${fix.fixAllDescription}` |
| 33 | src/compiler/parser.ts | `commaStart` | 3515 | 3498 | nameonly/cfg_incomplete | U | U-mech | `commaStart = -1; // Back to the state where the last token w` | `let commaStart = -1; // Meaning the previous token was not a` |
| 34 | src/compiler/scanner.ts | `tokenValue` | 2139 | 2158 | nameonly/cfg_incomplete | U | U-mech | `tokenValue = "0";` | `tokenValue = scanBinaryOrOctalDigits(/* base */ 8);` |
| 35 | src/compiler/utilities.ts | `node` | 3338 | 3364 | nameonly/cfg_incomplete | D | D-mech(dominator between) | `export function getSuperContainer(node: Node, stopOnFunction` | `return node as SuperContainerOrFunctions;` |
| 36 | src/services/completions.ts | `symbol` | 3723 | 3726 | nameonly/killed/3725 | D | D-mech(dominator between) | `let symbol = typeChecker.getSymbolAtLocation(node);` | `if (symbol.flags & (SymbolFlags.Module | SymbolFlags.Enum)) ` |
| 37 | src/services/preProcess.ts | `token` | 228 | 248 | exact/None | U | U-mech | `token = nextToken();` | `token = nextToken();` |
| 38 | src/services/preProcess.ts | `token` | 203 | 259 | nameonly/killed/206 | D | D-mech(dominator between) | `let token = scanner.getToken();` | `if (token === SyntaxKind.Identifier || isKeyword(token)) {` |
| 39 | src/harness/tsserverLogger.ts | `s` | 127 | 138 | nameonly/killed/128 | D | D-mech(dominator between) | `s = s.replace(/continuePreviousIncompleteResponse: \d+(?:\.\` | `s = sanitizeHarnessLSException(s);` |
| 40 | src/compiler/scanner.ts | `inEscape` | 2507 | 2536 | nameonly/killed/2496 | U | U-mech | `inEscape = true;` | `inEscape = false;` |


## Draft PR description
**#630 slice 5: rules U and D for JS/TS/TSX (named owners)**

Mirrors #713 for JS/TS/TSX: a removal-only lexical proof drops (a) anchor uses into lines that only write the name (U) and (b) defs a
dominating unconditional write overwrote in a single-entry region (D). New module `data_flow/build/js_prune.rs`; `lexical_prune.rs`
gets dispatch hooks only. Named owners without parse errors; synthetic anonymous owners and module level are untouched; `with`/direct
`eval` disable U and D, `arguments` disables D. Field paths compare identical paths only, for a stable base.

Evidence: removal-only on express, excalidraw, TypeScript src (added 0, kept-label transitions 0, `call-stats` identical outside
`dfg_labels`); 40/40 samples RIGHT per corpus; timing neutral. Controls vitess/django contain JS/TS and differ only there.
CACHE_VERSION 168 -> 169 (a parallel slice also takes 169; whichever merges second bumps). Goldens changed with per-edge justification (see above).
No full suite and no mutgate were run; environment failures: 5 `owner_admission_*`, 1 `cpg_cache::stream` permission test (not re-run on main).

Progress: DONE
