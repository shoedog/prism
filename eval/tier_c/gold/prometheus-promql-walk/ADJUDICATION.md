# prometheus-promql-walk — ADJUDICATION

Status: DRAFT — controller+Fable review pending.

## Source And Scope

- Repo: `~/code/bench-repos/prometheus` at `505095b`.
- Target: `parser.Walk(v Visitor, node Node, path []Node) error` in `promql/parser/ast.go:350`.
- Scope applied: production code only; `_test.go` parser calls are excluded. `promql/promqltest/test.go` is admitted because non-test production code imports it (`cmd/promtool`, `util/fuzzing`, and the promqltest migration command).
- Generator of record: grep-per-hop only. No prism/LSP/agent enumeration was used.

## Closure Walk

L0 — `Walk`

- Probe: `git -C ~/code/bench-repos/prometheus grep -nw Walk`
- Count: 18 raw word-hit lines / 11 files. Non-test Go subset: 14 lines / 8 files.
- True direct production caller: `promql/engine.go:4492` inside `PreprocessExpr`, calling `parser.Walk(&durationVisitor{...}, expr, nil)`.
- Thin forwarder: `Inspect` at `promql/parser/ast.go:396`, calling `Walk(f, node, pathBuf[:0])`.
- Scope-required Visitor impls included:
  - `inspector.Visit` (`promql/parser/ast.go:384`), the adapter for callbacks passed through `Inspect`.
  - `durationVisitor.Visit` (`promql/durations.go:39`), passed by `PreprocessExpr`.
- Neutral notes / exclusions: Walk's recursive self-call (`ast.go:361`), target/interface definition surface, comment-only hits, `filepath.Walk`, and tests.

Hop 1 — `Inspect`

- Probe: `git -C ~/code/bench-repos/prometheus grep -nw Inspect`
- Count: 16 raw word-hit lines. Production `parser.Inspect` call sites: 10.
- Forwarder: `ExtractSelectors` (`promql/parser/ast.go:370`, token `Inspect@372`).
- Terminal consumers:
  - `labelsSetPromQL` and `labelsDeletePromQL` in `cmd/promtool/main.go`.
  - `validateOpts`, `FindMinMaxTime`, `populateSeries`, `setOffsetForAtModifier`, and `detectHistogramStatsDecoding` in `promql/engine.go`.
  - `infoSelectHints` in `promql/info.go`.
  - `atModifierTestCases` in `promql/promqltest/test.go`.
  - `buildDependencyMap` in `rules/group.go`.
- Excluded: Go stdlib `ast.Inspect` in `web/api/v1/openapi_coverage_test.go`.

Hop 2 — `ExtractSelectors`

- Probe: `git -C ~/code/bench-repos/prometheus grep -nw ExtractSelectors`
- Count: 3 raw word-hit lines.
- Forwarder definition: `promql/parser/ast.go:370`, already included via hop 1.
- Terminal consumer: `API.queryExemplars` (`web/api/v1/api.go:713`, token `ExtractSelectors@732`).
- Excluded: `promql/parser/parse_test.go:6109` test call.

## Counts

- Real gold sites: 16.
- D1 site count: 6. D1 files: `cmd/promtool/main.go`, `promql/info.go`, `promql/promqltest/test.go`, `rules/group.go`, `web/api/v1/api.go`.
- Scorer D denominator is site-key-level: `d_gold_size=6`; secondary file-level denominator is `d_gold_file_size=5`.
- D2 count: 0. Verified repo-wide `Walk` word count is 18, below the >100 D2 threshold.
- Admission: PASS (`8 <= 16 <= 60`, D1 sites `6 >= 3`).

## Dry Run

Command run from `eval/`:

```text
uv run python -c "from tier_c.structural import score_structural,load_gold; g=load_gold('tier_c/gold/prometheus-promql-walk/gold.json'); real=[{'file':s['file'],'symbol':s['symbol']} for s in g['sites'] if s['adjudication']=='real']; r=score_structural(real,g); print(r.file_f1, r.d_recall, r.gold_size, r.d_gold_size, r.d_gold_file_size, r.phantom)"
```

Output:

```text
1.0 1.0 16 6 5 0
```

## Exclusions

- `filepath.Walk` collisions: `tsdb/fileutil/dir.go:23`, `tsdb/fileutil/fileutil.go:79`, `util/testutil/directory.go:127`, and `_test.go` checkpoint uses.
- Comment-only `Walk` collisions: `promql/durations.go:36`, `promql/engine.go:4661`, `storage/generic.go:295`, `tsdb/index/index.go:1205`, and checkpoint test comments.
- Definition/self surface: `Visitor` interface, `Walk` definition, and recursive `Walk` call inside Walk. The `(promql/parser/ast.go, Walk)` recursive self-call is now neutral notes only, not an excluded `sites[]` phantom-bait entry.
- Other-package `Inspect`: Go stdlib `ast.Inspect` in `web/api/v1/openapi_coverage_test.go`.
- Tests: `promql/durations_test.go:40` direct `parser.Walk`; `promql/parser/parse_test.go:6109` `ExtractSelectors`.

## Uncertain / Review

- I included `inspector.Visit` and `durationVisitor.Visit` because the task scope explicitly says real Visitor impls passed to Walk. Their anchors are grep-derived from the implementation tokens, not from `Walk`/`Inspect` call tokens in the same file, so this is the main controller review point.
- I admitted `promql/promqltest/test.go` because it is imported by non-test production code; if the controller interprets all `promqltest` package code as test helper surface, move `atModifierTestCases` to `excluded_test_helpers`.
- `ExtractSelectors` does selector aggregation, not a one-line return. I still classified it as a forwarder because the task caveat explicitly identifies it as the 3-hop wrapper.
