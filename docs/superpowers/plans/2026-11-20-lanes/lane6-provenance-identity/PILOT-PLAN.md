# Lane 6 pilot — Python direct request reads

## 1. Opt-in view

Use Python `ast` on pinned postimages in `~/code/prism-provenance-corpora`. In module functions and methods (including async), emit one row per changed-line name load whose outermost receiver chain yields payload. Skip stores, bare callees, modules, types and bare handles. Payload arguments still count.

| Library | Receiver proof | Direct reads |
| --- | --- | --- |
| Flask | `from flask import request`, including aliases | `args/form/values/headers/cookies/files` maps, `json`, `get_json()`, `path`. |
| Django | Formal annotated with imported `django.http.HttpRequest` or `django.http.request.HttpRequest` | `GET/POST/COOKIES/headers/FILES` maps, `body`. `META[k]` or `META.get(k)` only for string literal `^HTTP_[A-Z0-9]+(_[A-Z0-9]+)*$`. |
| FastAPI / Starlette | Formal annotated with imported `fastapi.Request`, `fastapi.requests.Request` or `starlette.requests.Request` | `query_params/path_params/headers/cookies` maps, zero-argument `await r.json()`, `await r.body()`, `await r.form()`. |

Allow whole maps, `request.headers["X-Id"]`-style indexing and `request.args.get(k)`-style lookup. Allow `.getlist(k)` on Flask `args/form/values/headers/files`, Django `GET/POST/FILES`, and FastAPI/Starlette `query_params/headers` and awaited form, including aliases. Lookup calls take one unstarred positional key. Preserve origin only through `x = expr`, non-slice payload indexing and listed lookups (`.get(k)` also on JSON). Only table-listed member reads qualify. Out for pilot are whole `META`, defaults, converters, keyword arguments, tuple unpacking, casts (`str`, `int`), parsers, other calls, injected parameters, Bottle and aiohttp.

Resolve direct or qualified annotations via one unconditional module import above the enclosing definition. Block competing bindings, class shadowing of method annotations and local shadowing even before a write. Formals must stay unrebound. Alias handles or payloads through one earlier simple assignment in the same statement list with no other bind/delete in that function. Attribute/item stores or deletes and unlisted method calls invalidate the rooted alias family throughout the function. Skip closures, comprehensions, `global/nonlocal` names and flow across suites. Check each accessor family plus shadowing, rebinding, mutations and defaults in development.

JSONL fields `repo, commit, file, line, column, token, origin, library, read, definition_line, proof`. Use repo slug, full SHA, Git-relative path, exact spelling, `origin="user_input"`, and `flask|django|fastapi|starlette` from the import. Use 1-based lines and 0-based UTF-8 byte columns. `read` is the source receiver’s `[line,column]`. `definition_line` is the name’s assignment line or the inline read line. `proof` lists `[kind,line,column,source_text]` in binding-to-use order. Kinds are `import, formal, assign, read, project`.

## 2. Populations

**Census.** Replay 160 Python items. Join labels by `(id,token)`, then use item ordinal and line to form the §4 key. A positive needs the token in `reads` and a name load at that pinned line. Treat missing anchors as misses. Use `definition_line` only as evidence. Report hits/16, weighted hits/total positive weight (per label), false positives over 140 negatives, and eight uncertain labels separately. Base hits 4/16 (weighted 4/118.3253).

All 12 missing tokens are also absent from `reads`. Pinned lines match saved text. Numbers before paths are manifest ordinals. Other-file matches are audit clues only.

| Labels | Pinned evidence |
| --- | --- |
| `c0015,c0016` | 113 `saleor/thumbnail/views.py:131,134` are except/logging lines. EOF 197. `database_id:250` and `datasource_dict:243` match 129 `superset/views/datasource/views.py`. |
| `c0020` | 121 `superset/datasource/api.py:581` is an annotation. 250 is doc text. `database_id:250` matches 129 above. |
| `c0028,c0030` | 126 `apply_dashboard_filters.py:818,820` are docstring lines. `request` is a formal at 798 and read at 855. Both definitions are null. |
| `c0042,c0046` | 129 `views/datasource/views.py:274` reads `requested_table` (`data` defined at 239, 172 blank). 137 `views/error_handling.py:252` reads `status/request` (171 comment). `data:172,171` match 171 and 172 respectively in `saleor/graphql/views.py`. |
| `c0048,c0049` | 137 `views/error_handling.py:258,259` test status and log. EOF 341. `body:493` matches 173 `saleor/graphql/views.py`. |
| `c0095` ×2 | 109 `saleor/product/managers.py:194` reads `attribute_pk`. EOF 414 precedes `name:666` and `raw_dimensions:656`. Weight 35.1084 per label. |
| `c0145` | 148 `superset/charts/data/api.py:673` reads `slice_/query_context`. Cited `data:53` is a constants import. |

**Historical.** Replay 240 parent diffs, taking added postimage lines in all `.py` paths. Verify parent/tree against Git, ordinal/tree/spec digest against base `.meta.json`, commit/digest against `.targets.json` (`repo.sha`, `diff.sha256`). Pair archived `9d1a7d06` findings and record failures.

**Fresh.** Pin these GitHub default-branch tips and handler paths in order. `ckan/ckan:ckan/views/`, `searxng/searxng:searx/webapp.py`, `getsentry/sentry:src/sentry/api/endpoints/`, `freqtrade/freqtrade:freqtrade/rpc/api_server/`. Per repo take up to 200 latest reachable single-parent commits touching Python there (committer time, then SHA). Exclude historical repos, forks, duplicate SHAs and full-diff stable patch IDs across sets. Shuffle SHA-sorted candidates per repo with Python seed `20261127`. Take ten and reserve ten. Save ordered pins and shortfalls. Use identical postimages and changed lines. Use the archived base binary digest and flags with fresh repo/spec paths.

## 3. Blind precision

Per tool and set (historical, fresh), sample 30 claims uniformly or all if fewer. Sort by §4 key and column before sampling with seed `20261127`. Two different models independently apply `m/RUBRIC.md` to marked occurrences, imports and enclosing code, blind to tool, claim, proof and each other. A third model adjudicates. Mark all same-token candidates for base and credit any confirmed hit. Report emitted, sampled, correct, incorrect and unresolved counts, correct/all sampled and the two-sided 95% Wilson interval.

## 4. Coverage and lost base rows

Before either tool runs on a fresh commit, uniformly sample four sorted changed Python code lines with the same seed (all if fewer). Exclude blank, comment-only and docstring lines. Blind-label every read per §3, including silent lines. Report hits/confirmed positives overall and by repo. Use columns for view matches.

Base key `(repo,commit,file,line,token)` uses manifest repo/commit, `.findings[]` file/line and token from `.description` via `^variable '([^']+)' has user_input origin:`. Select matching `.py` rows with `category="untrusted_origin"`. Any matching view occurrence retains a base row. Reconcile retained + lost to 569 historical rows, repeat for fresh and count view-only keys. Write every loss as JSONL with key fields, `reason` and `evidence` (pinned line and trace or failure).

Use the leftmost rubric variable read and the first matching code.

1. `non_variable` — parsed line has no variable read of the token.
2. `handle_or_metadata` — rubric establishes a handle or request metadata.
3. `other_origin` — rubric establishes another non-user origin, including transforms.
4. `cross_scope` — required value flow crosses functions or files.
5. `ambiguous_definition` — competing bindings, mutation or suite boundaries defeat the trace.
6. `unproven_library` — request-shaped read lacks receiver identity evidence.
7. `unsupported_read` — proven input uses a library, accessor or step outside §1.
8. `missed_supported_read` — the use meets every §1 rule.
9. `unverifiable_source` — pinned source is missing, unparsable or insufficient.

## 5. Decision and stop

Budget two to four lane days. One historical development round, then freeze. Open the full reserve once before precision grading if fresh view claims are below 30, coverage positives below ten, or positive repos below two. Stop by day four. **Yes** requires ≥26 correct among 30 fresh view claims and ≥50% fresh view coverage over ≥10 confirmed positives across ≥2 repos. **No** if either gate fails on its required sample. Otherwise **inconclusive**, including timeouts. Publish available counts. Only yes permits considering a Rust port.

## 6. Remaining risks

- Misbound labels depress census coverage, especially at high weights.
- Base keys collapse repeated occurrences. Fresh samples are small and handler-specific.
- Static proofs miss framework injection and can overlook runtime rebinding or mutation.
