# Pilot 06 / 01 recorded results (2026-10-31)

These are the verbatim outputs of the scripts in this directory, run with the release binary
`slicing 3.2.0 (533f061a942a)` on the four pinned corpora (see `RUN.txt`). Each section is headed by the name of the
output file the specs cite (`budget.out`, `findsim2.out`, and so on), so those citations stay greppable.

`rank.py` is the shared library: `rank2.py`, `rank3.py` and `budget.py` load it with `exec`, and `atbudget.py` loads
`rank2.py` the same way. Do not rename these scripts.

The focused-ranking rows (`ppr_focus`, `ppr_focus_bidir`, `focused_2k`, `focused_4k`) are computed from the trial
question text, and every hit rate is scored against gold files. Both live in the private evidence store and are not
committed, so those rows cannot be re-derived from this repository alone (see the top-level `README.md`). The
per-question rows list counts and hit rates only, never the gold units.

## Crosswalk: spec table row to output row

| Spec table (06 section 9.2 unless noted) | Output |
|---|---|
| path order 0.400 / 0.581 / 0.810 | `rank2.out` `alpha` |
| `onboard` total degree 0.458 / 0.648 / 0.835 | `rank2.out` `degree` |
| degree x test prior 0.487 / 0.711 / 0.841 | `rank2.out` and `rank3.out` `degree_test` |
| `ppr-sym-v1` unfocused 0.487 / 0.694 / 0.833 | `rank3.out` `pr_bidir_test` |
| `ppr-sym-v1` focused 0.596 / 0.750 / 0.851 | `rank2.out` `ppr_focus_bidir` |
| equal-budget table (B1, ranked, focused at 2k / 4k) | `atbudget.out` (K values 60/94/36/57 and 60/102/36/125 are hard-coded in `atbudget.py` from `budget.out`) |
| 2k-token fits, "9.4-13.0 KB today" | `budget.out` |
| 01 section 8.3 (15/17 query-only, 17/17 with the file filter) | `findsim2.out` |

## `budget.out`

```
httpx 60 budget 1000 files 60
httpx 60 budget 2000 files 60
httpx 60 budget 4000 files 60
  sample: httpx/_models.py [in 39 out 9] json:831 (74), read:468 (32), read:876 (29)
  today first-50 canonical bytes 12826 ~tokens 3206
ripgrep 102 budget 1000 files 34
ripgrep 102 budget 2000 files 94
ripgrep 102 budget 4000 files 102
  sample: crates/globset/src/glob.rs [in 29 out 3] as_ref:84 (53), new:283 (16), parse:823 (15)
  today first-50 canonical bytes 12990 ~tokens 3247
cobra 36 budget 1000 files 36
cobra 36 budget 2000 files 36
cobra 36 budget 4000 files 36
  sample: command.go [in 32 out 5] Flags:1688 (170), AddCommand:1342 (156), Name:1541 (81)
  today first-50 canonical bytes 10807 ~tokens 2701
excalidraw 628 budget 1000 files 27
excalidraw 628 budget 2000 files 57
excalidraw 628 budget 4000 files 125
  sample: packages/excalidraw/components/App.tsx [in 101 out 107] mutateElement:4644 (48), setActiveTool:5530 (34), useExcalidrawSetAppState:580 (23)
  today first-50 canonical bytes 9400 ~tokens 2350
```

## `atbudget.out`

```
today_first50  all 0.810   excalidraw 0.378
path_2k        all 0.915   excalidraw 0.406
path_4k        all 0.930   excalidraw 0.511
ranked_2k      all 0.902   excalidraw 0.311
focused_2k     all 0.908   excalidraw 0.353
ranked_4k      all 0.935   excalidraw 0.547
focused_4k     all 0.941   excalidraw 0.589
```

## `rank2.out`

```
ALL gold files (n=21)
alpha                0.400   0.581   0.810
degree               0.458   0.648   0.835
indeg                0.479   0.648   0.788
pr_mdep              0.420   0.623   0.797
pr_mdep_test         0.437   0.628   0.797
pr_calls_test        0.433   0.650   0.797
degree_test          0.487   0.711   0.841
pr_symbol_test       0.493   0.669   0.827
ppr_focus            0.580   0.727   0.843
ppr_focus_bidir      0.596   0.750   0.851
gold files NOT named in the question (n=20)
alpha                0.366   0.564   0.798
degree               0.465   0.648   0.821
indeg                0.459   0.640   0.805
pr_mdep              0.417   0.605   0.814
pr_mdep_test         0.434   0.606   0.814
pr_calls_test        0.431   0.638   0.814
degree_test          0.496   0.677   0.827
pr_symbol_test       0.464   0.662   0.812
ppr_focus            0.522   0.690   0.833
ppr_focus_bidir      0.548   0.718   0.841
excalidraw only, all gold
alpha                0.000   0.136   0.378
degree               0.169   0.261   0.383
indeg                0.169   0.169   0.275
pr_mdep              0.142   0.169   0.225
pr_mdep_test         0.142   0.169   0.225
pr_calls_test        0.142   0.169   0.225
degree_test          0.169   0.261   0.383
pr_symbol_test       0.142   0.219   0.361
ppr_focus            0.169   0.267   0.322
ppr_focus_bidir      0.197   0.289   0.353
['P1', 'httpx', 7, 7, 0, 1, {'alpha': [0.43, 0.43, 1.0], 'degree_test': [0.43, 0.71, 1.0], 'ppr_focus': [0.29, 0.57, 1.0], 'ppr_focus_bidir': [0.43, 0.86, 1.0]}]
['P2', 'httpx', 2, 1, 1, 4, {'alpha': [1.0, 1.0, 1.0], 'degree_test': [0.5, 1.0, 1.0], 'ppr_focus': [1.0, 1.0, 1.0], 'ppr_focus_bidir': [1.0, 1.0, 1.0]}]
['P3', 'httpx', 3, 3, 0, 4, {'alpha': [1.0, 1.0, 1.0], 'degree_test': [0.67, 1.0, 1.0], 'ppr_focus': [1.0, 1.0, 1.0], 'ppr_focus_bidir': [1.0, 1.0, 1.0]}]
['P4', 'httpx', 2, 1, 1, 4, {'alpha': [1.0, 1.0, 1.0], 'degree_test': [1.0, 1.0, 1.0], 'ppr_focus': [1.0, 1.0, 1.0], 'ppr_focus_bidir': [1.0, 1.0, 1.0]}]
['P5', 'httpx', 7, 7, 0, 5, {'alpha': [0.29, 1.0, 1.0], 'degree_test': [0.14, 0.57, 0.57], 'ppr_focus': [0.71, 1.0, 1.0], 'ppr_focus_bidir': [0.71, 0.71, 1.0]}]
['P6', 'httpx', 5, 5, 0, 0, {'alpha': [0.6, 1.0, 1.0], 'degree_test': [1.0, 1.0, 1.0], 'ppr_focus': [1.0, 1.0, 1.0], 'ppr_focus_bidir': [1.0, 1.0, 1.0]}]
['R1', 'ripgrep', 4, 4, 1, 8, {'alpha': [0.0, 0.0, 0.75], 'degree_test': [0.0, 0.25, 0.5], 'ppr_focus': [0.0, 0.25, 0.25], 'ppr_focus_bidir': [0.0, 0.25, 0.25]}]
['R2', 'ripgrep', 4, 3, 1, 8, {'alpha': [0.0, 0.25, 1.0], 'degree_test': [0.25, 0.75, 1.0], 'ppr_focus': [0.5, 0.5, 1.0], 'ppr_focus_bidir': [0.5, 0.5, 1.0]}]
['R3', 'ripgrep', 6, 6, 0, 5, {'alpha': [0.0, 0.0, 0.5], 'degree_test': [0.33, 0.83, 1.0], 'ppr_focus': [0.5, 0.67, 1.0], 'ppr_focus_bidir': [0.5, 1.0, 1.0]}]
['R4', 'ripgrep', 4, 3, 1, 18, {'alpha': [0.0, 0.25, 1.0], 'degree_test': [0.25, 0.5, 0.75], 'ppr_focus': [0.25, 0.5, 1.0], 'ppr_focus_bidir': [0.5, 0.5, 1.0]}]
['R5', 'ripgrep', 8, 8, 0, 12, {'alpha': [0.0, 0.0, 0.0], 'degree_test': [0.38, 0.5, 0.88], 'ppr_focus': [0.38, 0.62, 0.75], 'ppr_focus_bidir': [0.5, 0.75, 0.75]}]
['R6', 'ripgrep', 11, 11, 0, 0, {'alpha': [0.27, 0.36, 0.64], 'degree_test': [0.18, 0.45, 0.82], 'ppr_focus': [0.27, 0.73, 0.73], 'ppr_focus_bidir': [0.09, 0.45, 0.82]}]
['G1', 'cobra', 6, 4, 2, 2, {'alpha': [0.67, 1.0, 1.0], 'degree_test': [0.5, 0.83, 1.0], 'ppr_focus': [0.67, 0.83, 1.0], 'ppr_focus_bidir': [1.0, 1.0, 1.0]}]
['G2', 'cobra', 4, 3, 1, 1, {'alpha': [0.25, 1.0, 1.0], 'degree_test': [0.75, 1.0, 1.0], 'ppr_focus': [0.75, 1.0, 1.0], 'ppr_focus_bidir': [0.5, 1.0, 1.0]}]
['G3', 'cobra', 2, 2, 0, 1, {'alpha': [1.0, 1.0, 1.0], 'degree_test': [1.0, 1.0, 1.0], 'ppr_focus': [1.0, 1.0, 1.0], 'ppr_focus_bidir': [1.0, 1.0, 1.0]}]
['G4', 'cobra', 1, 0, 1, 1, {'alpha': [1.0, 1.0, 1.0], 'degree_test': [1.0, 1.0, 1.0], 'ppr_focus': [1.0, 1.0, 1.0], 'ppr_focus_bidir': [1.0, 1.0, 1.0]}]
['G5', 'cobra', 20, 20, 0, 1, {'alpha': [0.3, 0.7, 1.0], 'degree_test': [0.55, 0.95, 1.0], 'ppr_focus': [0.55, 1.0, 1.0], 'ppr_focus_bidir': [0.4, 0.85, 1.0]}]
['G6', 'cobra', 5, 5, 0, 0, {'alpha': [0.6, 0.8, 1.0], 'degree_test': [0.8, 0.8, 1.0], 'ppr_focus': [0.8, 0.8, 1.0], 'ppr_focus_bidir': [0.8, 1.0, 1.0]}]
['J3', 'excalidraw', 12, 11, 1, 9, {'alpha': [0.0, 0.08, 0.42], 'degree_test': [0.25, 0.33, 0.5], 'ppr_focus': [0.25, 0.42, 0.58], 'ppr_focus_bidir': [0.33, 0.42, 0.42]}]
['J5', 'excalidraw', 8, 8, 0, 1, {'alpha': [0.0, 0.12, 0.25], 'degree_test': [0.12, 0.25, 0.25], 'ppr_focus': [0.12, 0.25, 0.25], 'ppr_focus_bidir': [0.12, 0.25, 0.38]}]
['J6', 'excalidraw', 15, 15, 0, 0, {'alpha': [0.0, 0.2, 0.47], 'degree_test': [0.13, 0.2, 0.4], 'ppr_focus': [0.13, 0.13, 0.13], 'ppr_focus_bidir': [0.13, 0.2, 0.27]}]
```

## `rank3.out`

```
pr_bidir_test      0.487   0.694   0.833  excalidraw    0.169   0.261   0.311
degree_test        0.487   0.711   0.841  excalidraw    0.169   0.261   0.383
wdegree_test       0.514   0.694   0.821  excalidraw    0.169   0.169   0.311
```

## `findsim2.out`

```
   with file filter: True
P2 httpx._api.request target httpx/_api.py:39 rank 1 n 60 uniqueTopTier True [('httpx/_api.py:39', (None, None), 1), ('httpx/_client.py:771', ('Client', None), 3), ('httpx/_client.py:1485', ('AsyncClient', None), 3)]
P3 Client.request target httpx/_client.py:771 rank 1 n 60 uniqueTopTier True [('httpx/_client.py:771', ('Client', None), 0), ('httpx/_api.py:39', (None, None), 3), ('httpx/_client.py:1485', ('AsyncClient', None), 3)]
   with file filter: True
P4 Client.send target httpx/_client.py:879 rank 1 n 11 uniqueTopTier True [('httpx/_client.py:879', ('Client', None), 0), ('httpx/_transports/asgi.py:148', ('ASGITransport', None), 3), ('httpx/_client.py:1594', ('AsyncClient', None), 3)]
P5 BaseTransport.handle_request target httpx/_transports/base.py:26 rank 1 n 10 uniqueTopTier True [('httpx/_transports/base.py:26', ('BaseTransport', None), 0), ('httpx/_transports/default.py:230', ('HTTPTransport', None), 3), ('httpx/_transports/mock.py:19', ('MockTransport', None), 3)]
   with file filter: True
R1 grep_searcher::Searcher::search_path target crates/searcher/src/searcher/mod.rs:643 rank 1 n 4 uniqueTopTier True [('crates/searcher/src/searcher/mod.rs:643', ('Searcher', None), 0), ('crates/core/search.rs:342', ('SearchWorker', None), 3), ('crates/searcher/src/searcher/mod.rs:678', ('Searcher', None), 3)]
   with file filter: True
R2 run target crates/core/main.rs:77 rank 2 n 9 uniqueTopTier False [('crates/searcher/src/searcher/glue.rs:166', ('MultiLine', None), 0), ('crates/core/main.rs:77', (None, None), 0), ('crates/ignore/src/walk.rs:1331', ('WalkParallel', None), 0)]
R3 SearchWorker::search target crates/core/search.rs:245 rank 1 n 31 uniqueTopTier True [('crates/core/search.rs:245', ('SearchWorker', None), 0), ('crates/core/search.rs:362', ('SearchWorker', None), 2), ('crates/core/search.rs:342', ('SearchWorker', None), 2)]
R5 Sink::matched target crates/searcher/src/sink.rs:0 rank None n 26 uniqueTopTier False [('crates/printer/src/json.rs:722', ('JSONSink', 'Sink'), 0), ('crates/printer/src/standard.rs:766', ('StandardSink', 'Sink'), 0), ('crates/printer/src/summary.rs:633', ('SummarySink', 'Sink'), 0)]
   with file filter: True
G1a ExactValidArgs target args.go:142 rank 1 n 9 uniqueTopTier True [('args.go:142', (None, None), 0), ('args_test.go:333', (None, None), 3), ('args_test.go:339', (None, None), 3)]
G1b OnlyValidArgs target args.go:51 rank 1 n 3 uniqueTopTier True [('args.go:51', (None, None), 0), ('args_test.go:155', (None, None), 3), ('args_test.go:161', (None, None), 3)]
   with file filter: True
G1c Command.ValidateArgs target command.go:1172 rank 1 n 1 uniqueTopTier True [('command.go:1172', ('Command', None), 0)]
   with file filter: True
G2 GenMarkdownTree target doc/md_docs.go:125 rank 1 n 2 uniqueTopTier True [('doc/md_docs.go:125', (None, None), 0), ('doc/md_docs.go:133', (None, None), 2)]
G3a Command.SetArgs target command.go:281 rank 1 n 1 uniqueTopTier True [('command.go:281', ('Command', None), 0)]
G3b ExecuteC target command.go:1084 rank 1 n 16 uniqueTopTier True [('command.go:1084', ('Command', None), 0), ('command.go:1062', ('Command', None), 2), ('command.go:1078', ('Command', None), 2)]
   with file filter: True
G4 Command.execute target command.go:905 rank 1 n 35 uniqueTopTier True [('command.go:905', ('Command', None), 0), ('command.go:1070', ('Command', None), 1), ('command.go:1084', ('Command', None), 2)]
G5 (*Command).Context target command.go:269 rank 1 n 14 uniqueTopTier True [('command.go:269', ('Command', None), 0), ('command.go:275', ('Command', None), 3), ('command.go:1062', ('Command', None), 3)]
   with file filter: True
J3 onChange target excalidraw-app/App.tsx:677 rank 1 n 17 uniqueTopTier False [('excalidraw-app/App.tsx:677', (None, None), 0), ('examples/with-script-in-browser/components/ExampleApp.tsx:183', (None, None), 0), ('packages/excalidraw/components/App.tsx:777', ('App', None), 0)]
J5 getRenderableElements target packages/excalidraw/scene/Renderer.ts:208 rank 1 n 2 uniqueTopTier True [('packages/excalidraw/scene/Renderer.ts:208', ('Renderer', None), 0), ('packages/excalidraw/scene/Renderer.ts:89', ('Renderer', None), 2)]
```

