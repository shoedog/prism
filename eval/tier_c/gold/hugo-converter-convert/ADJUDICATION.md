# hugo-converter-convert — ADJUDICATION

Status: DRAFT — controller+Fable review pending.

## Source And Scope

- Repo: `~/code/bench-repos/hugo` at `a00b5c7`.
- Target: `converter.Converter.Convert(ctx RenderContext) (ResultRender, error)` in `markup/converter/converter.go:91`.
- Scope applied: production Go code sites only. `_test.go`, docs, examples, and non-Go templates are excluded. Converter implementation definitions are recorded as excluded definition/implementation surface, not downstream caller sites; this is flagged for review.
- Generator of record: grep-per-hop only. No prism/LSP/agent enumeration was used.

## Closure Walk

L0 — `Convert`

- Probe: `git -C ~/code/bench-repos/hugo grep -nw Convert`
- Verified count: 65 raw word-hit lines / 34 files; production non-test Go subset: 33 lines / 15 files.
- True direct production caller: `hugolib/page__per_output.go:460` inside `renderContentWithConverter`; receiver is parameter `c converter.Converter`.
- Forwarder: `(*pageContentOutput).renderContentWithConverter` is thin: it builds `converter.RenderContext`, calls `c.Convert`, and returns `(r, err)`.
- Excluded: reflect `Value.Convert`, command strings/comments, converter implementation definitions, docs/tests/examples.

Hop 1 — `renderContentWithConverter`

- Probe: `git -C ~/code/bench-repos/hugo grep -nw renderContentWithConverter -- '*.go' ':!*_test.go'`
- Count: 4 lines / 2 files.
- Real callers:
  - `(*pageContentOutput).ParseAndRenderContent` at `hugolib/page__per_output.go:409`, token `renderContentWithConverter@417`, forwarder.
  - `(*cachedContentScope).RenderString` at `hugolib/page__content.go:878`, tokens `renderContentWithConverter@977/1026`, forwarder by task scope.
- Definition line `hugolib/page__per_output.go:459` is already the L0 real site.

Hop 2 — `ParseAndRenderContent`

- Probe: `git -C ~/code/bench-repos/hugo grep -nw ParseAndRenderContent -- '*.go' ':!*_test.go'`
- Count: 9 lines / 6 files.
- Real terminal consumers: `contentRendered` (`page__content.go:608`), `contentToC` (`page__content.go:733`), `prepareShortcode` (`shortcode.go:471`).
- Real same-name forwarder: `LazyContentProvider.ParseAndRenderContent` (`resources/page/page_lazy_contentprovider.go:128/129`).
- Excluded: interface declaration in `resources/page/page.go`; no-op stub in `resources/page/page_nop.go`.

Hop 2/3 — `RenderString`

- Probe: `git -C ~/code/bench-repos/hugo grep -nw RenderString -- '*.go' ':!*_test.go'`
- Count: 15 lines / 9 files.
- Real same-name forwarders: `pageContentOutput.RenderString` (`page__per_output.go:223/224`) and `LazyContentProvider.RenderString` (`page_lazy_contentprovider.go:124/125`).
- Real terminal consumer: `Namespace.Markdownify` (`tpl/transform/transform.go:214/219`); it calls `home.RenderString`, then trims returned HTML, so recursion stops there.
- Excluded: comments, interface declarations, no-op stubs, and embedded `.html` templates. The embedded template exclusion is flagged for review.

## Counts

- Real gold sites: 10.
- D1 site count: 7. D1 files: `hugolib/page__content.go`, `hugolib/shortcode.go`, `resources/page/page_lazy_contentprovider.go`, `tpl/transform/transform.go`.
- Scorer D denominator is file-level: `d_gold_size=4`.
- D2 count: 0. Verified repo-wide `Convert` word count is 65, below the >100 D2 threshold.
- Admission: PASS (`8 <= 10 <= 60`, D1 sites `7 >= 3`).

## Dry Run

Command run from `eval/`:

```text
uv run python -c "from tier_c.structural import score_structural,load_gold; g=load_gold('tier_c/gold/hugo-converter-convert/gold.json'); perfect=[{'file':s['file'],'symbol':s['symbol']} for s in g['sites'] if s['adjudication']=='real']; r=score_structural(perfect,g); print('perfect',r.file_f1,r.d_recall,r.gold_size,r.d_gold_size,r.phantom)"
```

Output:

```text
perfect 1.0 1.0 10 4 0
```

## Exclusions

- Other receiver collisions: `common/hreflect/convert.go` and `tpl/internal/go_templates/texttemplate/funcs.go` use `reflect.Value.Convert`, not `converter.Converter.Convert`.
- String/comment collisions: `commands/convert.go`, docs, README, image comments, and template-store comments.
- Definition/implementation surface: converter implementations in `markup/asciidocext`, `markup/converter`, `markup/goldmark`, `markup/org`, `markup/pandoc`, and `markup/rst`; wrapper interface/stub declarations in `resources/page`.
- Tests: direct Convert/RenderString hits in `_test.go` excluded by production-code policy.

## Uncertain / Review

- Whether converter implementation methods should be real gold for this task. I excluded them to match the renaming-forwarder convention used by the worked examples, but the task wording says "converter interface method" and a controller could interpret implementations as review sites.
- Whether production embedded templates using `.Page.RenderString` should be admitted despite lacking Go enclosing symbols.
- Whether `cachedContentScope.RenderString` is thin enough as a forwarder. I included it because the task explicitly names RenderString wrappers and its callers are the intended name-absent layer.
