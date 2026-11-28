# Lane 6 pilot — Python direct request reads

## 1. Opt-in view

Use Python `ast` on pinned postimages in `~/code/prism-provenance-corpora`. In module functions and methods (including async), emit one row per changed-line name load whose outermost receiver chain yields payload. Skip stores, bare callees, modules, types and bare handles. Payload arguments still count.

| Library | Receiver proof | Direct reads |
| --- | --- | --- |
| Flask | `from flask import request`, including aliases | `args/form/values/headers/cookies/files` maps, `json`, `get_json()`. |
| Django | Formal annotated with imported `django.http.HttpRequest` or `django.http.request.HttpRequest` | `GET/POST/COOKIES/headers/FILES` maps, `body`. `META[k]` or `META.get(k)` only for string literal `^HTTP_[A-Z0-9]+(_[A-Z0-9]+)*$`. |
| FastAPI / Starlette | Formal annotated with imported `fastapi.Request`, `fastapi.requests.Request` or `starlette.requests.Request` | `query_params/path_params/headers/cookies` maps, zero-argument `await r.json()`, `await r.body()`, `await r.form()`. |

Allow whole maps, `request.headers["X-Id"]`-style indexing and `request.args.get(k)`-style lookup. Allow `.getlist(k)` on Flask `args/form/values/headers/files`, Django `GET/POST/FILES`, and FastAPI/Starlette `query_params/headers` and awaited form, including aliases. Lookup calls take one unstarred positional key. Preserve origin only through `x = expr`, non-slice payload indexing and listed lookups (`.get(k)` also on JSON). Only table-listed member reads qualify. Out for pilot are whole `META`, defaults, converters, keyword arguments, tuple unpacking, casts (`str`, `int`), parsers, other calls, injected parameters, Bottle and aiohttp.

Resolve direct or qualified annotations via one unconditional module import above the enclosing definition. Block competing bindings, class shadowing of method annotations and local shadowing even before a write. Formals must stay unrebound. Alias handles or payloads through one earlier simple assignment in the same statement list with no other bind/delete in that function. Attribute/item stores or deletes and unlisted method calls invalidate the rooted alias family throughout the function. Skip closures, comprehensions, `global/nonlocal` names and flow across suites. Check each accessor family plus shadowing, rebinding, mutations and defaults in development.

JSONL fields `repo, commit, file, line, column, token, origin, library, read, definition_line, proof`. Use repo slug, full SHA, Git-relative path, exact spelling, `origin="user_input"`, and `flask|django|fastapi|starlette` from the import. Use 1-based lines and 0-based UTF-8 byte columns. `read` is the source receiver’s `[line,column]`. `definition_line` is the name’s assignment line or the inline read line. `proof` lists `[kind,line,column,source_text]` in binding-to-use order. Kinds are `import, formal, assign, read, project`.

## 2. Populations

**Census.** Replay the 160 Python items of `m/census2/items.jsonl` (the file the labels in `labels/census-final.jsonl` were made against; weights from `census2/strata.json`; the earlier astra/sol drafts joined `census/` by mistake, round-1 WRONG-1). Join labels by `(id,token)`. A positive needs the token among the item's `reads` at the item's line. Counts to reproduce first: 16 positives, 140 negatives, 8 uncertain (reported separately), weighted positives 81.76, base 11/16 (weighted 32.92, the report's 71.38%). If the join does not give these, stop and fix the join before any view run. Report view hits/16 and weighted, false positives over the 140 negatives, per proof kind.

**Historical.** Replay 240 parent diffs, taking each archived spec's `.py` diff lines (the spec builder drops test files and deleted files; view rows outside the spec are not analysed). Verify parent/tree against Git, ordinal/tree/spec digest against base `.meta.json`, commit/digest against `.targets.json` (`repo.sha`, `diff.sha256`). Pair archived `9d1a7d06` findings and record failures.

**Fresh.** Qualifying rule, written before any draw: a repo qualifies when at its pinned default-branch tip at least 5 Python files under its handler path contain a receiver the §1 table can prove (`from flask import request`, an `HttpRequest` or fastapi/starlette `Request` annotation). Candidates: public Python web repos found by GitHub search (flask, django, fastapi topics) sorted by stars, tried in that order, not in the historical set, not forks or frameworks themselves. Keep the first four qualifying repos not in the historical set; record the rejects and why. The earlier ckan/searxng/sentry/freqtrade list is dropped: at their tips they use wrappers (`ckan.common`, `sxng_request`), DRF `Request` or injected parameters the table cannot prove (round-1 WRONG-2). Coverage on such unsupported-library repos is reported once as a scope statistic outside the gate. Per repo take up to 200 latest single-parent commits touching `.py` files under the handler path (spec rules as historical: no tests, no deletions), exclude historical repos, forks, duplicate SHAs and stable patch IDs, shuffle SHA-sorted candidates with Python seed `20261127`, take ten and reserve ten per repo. Freeze artifact: commit the script and record its sha256 in the ledger BEFORE the pin step runs. Save ordered pins and shortfalls. Run base with the archived binary digest and flags on the same specs.

## 3. Blind precision

Per tool and set (historical, fresh), sample 30 claims uniformly or all if fewer. Sort by §4 key and column before sampling with seed `20261127`. Two different models independently apply `m/RUBRIC.md` to marked occurrences, imports and enclosing code, blind to tool, claim, proof and each other. A third model adjudicates. Mark all same-token candidates for base and credit any confirmed hit. Cap 5 claims per commit and require at least 2 repos among the sampled claims. Mix 10 independently drawn name reads (not view claims) into each reader batch with the same format, so readers cannot infer the claim. Report precision separately for import-proven and annotation-only rows (an annotation-only row can be a plain helper called with a constant; its precision is a hypothesis). Report emitted, sampled, correct, incorrect and unresolved counts, correct/all sampled and the two-sided 95% Wilson interval.

## 4. Coverage and lost base rows

Before either tool runs on a fresh commit, uniformly sample four sorted changed Python code lines with the same seed (all if fewer). Exclude blank, comment-only and docstring lines. Blind-label every read per §3, including silent lines. Weight each sampled line by the inverse of its inclusion probability (changed lines in its commit / 4). Unresolved reads count as missed positives in a conservative figure reported beside the point figure. Report hits/confirmed positives overall and by repo, weighted and raw. Use columns for view matches.

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

Budget two to four lane days. One historical development round, then freeze. Open the full reserve once before precision grading if fresh view claims are below 30, coverage positives below ten, or positive repos below two. Stop by day four. **Yes** requires ≥26 correct among ≥30 sampled fresh view claims (point estimate ≥85%; Wilson interval reported; operating characteristic disclosed to the owner: at 30 claims P(pass) is about 0.10 at true 75%, 0.52 at 85%, 0.83 at 90%) and fresh weighted coverage ≥50% over ≥10 confirmed positives across ≥2 repos with ≥1 hit in each. **No** if either gate fails on its required sample, or the table proves nothing on a qualifying repo. Otherwise **inconclusive**, including timeouts. Publish available counts. Only yes permits considering a Rust port.

## 6. Remaining risks

- Annotation-only receiver proofs can certify a constant-fed helper; reported separately. Refuse string annotations and `flask.request` after `import flask`; drop Flask `request.path` (routing metadata under the rubric chain rule).
- Base keys collapse repeated occurrences. Fresh samples are small and handler-specific.
- Static proofs miss framework injection and can overlook runtime rebinding or mutation.
