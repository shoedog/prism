# Fable-review gold fixes (apply exactly; do NOT open-endedly re-derive)

The golds passed Fable's adversarial review with SPECIFIC fixes. Apply these to the
`gold.json` files (and their `ADJUDICATION.md` notes) under `eval/tier_c/gold/<task-id>/`.
Re-run the perfect-arm dry-run after each and record the new numbers. Do NOT commit/push.

## The governing principle (phantom-channel calibration)
The scorer turns a gold's `sites[]` entries with `adjudication:"excluded"` into PHANTOM
penalties (matched by (file, norm_symbol)); the separate `exclusion_table` dict is INERT
(never scored). Therefore:
- `sites[]` excluded entries = ONLY genuine same-name/other-RECEIVER collision bait, each
  with the REAL enclosing symbol (so an arm's wrong claim actually matches → phantom).
- Definition/impl-surface and judgment-boundary sites = move to `exclusion_table`/notes
  (neutral `unmatched_extra`, never phantom) — they must NOT punish a defensible claim.

## Per-task fixes

### prometheus-matchstring — add real-symbol bait for 8 uncovered collision files
`git grep -lw MatchString` (non-test) = 15 files; only 5 are in `sites[]` as excluded. ADD
excluded `sites[]` entries (real enclosing symbol via `git grep -nw MatchString <file>` then
find the enclosing func) for: `relabel/relabel.go`, `cmd/promtool/main.go`-area `cors.go`
(util/cors? verify path), `discovery/.../dedupe`? — VERIFY each path by grep; the 8 are the
non-test `MatchString` files beyond {config.go, discovery/file/file.go, discovery/http/http.go,
discovery/puppetdb/puppetdb.go, web/api/v1/api.go}. Likely: relabel.go, cors.go, dedupe.go,
template.go, azuread.go, promql/promqltest/test.go, promql/promqltest/cmd/.../migrate, and
cmd/promtool/unittest.go — CONFIRM the actual set with `git grep -lw MatchString -- '*.go' ':!*_test.go'`
and add one excluded entry per file with its real enclosing symbol. (Phantom is this task's
co-primary — the bait must fire.)

### prometheus-promql-walk — neutralize the walk-driver bait
The excluded `sites[]` entry `(promql/parser/ast.go, "Walk")` phantom-baits a claim of the
walk driver, but the scope declares the walk driver IN. Remove it from `sites[]` (move to
notes). Everything else ACCEPT.

### caddy-requestmatcher-migration — 3 fixes
1. Scope line names "interface guards" as IN, but zero legacy guards remain at this SHA →
   reword scope to "interface guards (none remain legacy at this SHA)" or drop that IN clause.
2. Scope's "deprecated set method" steers arms to `MatcherSets.AnyMatch` (routes.go:412),
   which is a phantom-baited excluded entry → neutralize the AnyMatch excluded entry (move to
   notes) OR reword scope so it isn't steered.
3. The `caddyhttp.go:43` `RequestMatcher` interface DECL is phantom-baited, but the prompt
   says "every site that must change" and the decl MUST be deleted → make it a REAL gold site
   (role: the interface to delete) or neutral; do not leave it as bait.

### ruff-typechecker-match-annotation — biggest fix (do-not-trust #3 + missed hop)
1. Scope/prompt mismatch: gold is the is_list+is_dict sub-closure but prompt/scope demand the
   whole protocol. REWORD `scope` NAME-FREE: "IN: only the protocol's list- and dict-type
   matching paths; OUT: matching paths for other builtin/library types." Adjust `prompt_change`
   so it does NOT demand the entire protocol (keep it forwarder-blind — no is_list/check_type).
2. MISSED HOP inside scope: `is_known_to_be_of_type_dict` (crates/ruff_python_semantic/src/
   analyze/typing.rs:49) is a thin forwarder over `is_dict` with 3 missed D1 consumers — add as
   real gold sites (D1, verify each file has zero `match_annotation`):
   - `crates/ruff_linter/src/rules/flake8_simplify/.../if_else_block_instead_of_dict_get.rs:148`
   - `crates/ruff_linter/src/rules/.../falsy_dict_get_fallback.rs:86`
   - `crates/ruff_linter/src/rules/.../if_key_in_dict_del.rs:68`
   (verify exact paths + enclosing symbols by grep). Gold 28→32, D1 25→28, D-files 16→19.
3. Move the parser `is_list` bait (`crates/ruff_python_parser/src/parser/mod.rs:949`,
   `.../pattern.rs:349`, `:365`) from `exclusion_table` into real excluded `sites[]` entries
   with enclosing symbols.

### ruff-imported-qualified-name — CRITICAL (inert precision bait)
This is the D2/PRECISION task; its phantom channel is INERT because decoy excluded entries use
placeholder symbols like `"n/a (4 sites: lines 633,777,835,921)"` which can never norm-match.
REPLACE with one excluded `sites[]` entry PER decoy call site using the REAL enclosing symbol
(e.g. `(crates/ruff_python_semantic/src/binding.rs? / statement.rs, <enclosing fn of line 633>)`,
`(unnecessary_future_import.rs, unnecessary_future_import)`, the ty-crate class.rs:693/1012 +
type_alias.rs:302 + imports.rs:60 + definition.rs:63 diagnostic/display fns). Verify each
enclosing symbol by grep. Also FIX `closure_summary.gold_size`: JSON has 17 real entries, not 18.

### hugo-converter-convert — 2 fixes
1. Add to `scope`: "OUT: the converter implementations themselves" (so the 5 converter-impl
   excluded entries goldmark/org/pandoc/rst/asciidoc don't punish a scope-compliant claim) —
   OR neutralize those 5 excluded entries (move to notes). Prefer the scope addition.
2. Fix `receiver_evidence` for the `page__content.go:878` site: the method receiver is
   `*cachedContentScope`, not `*pageContentOutput` (ADJUDICATION.md already has it right).

### typescript-resolve-signature — neutralize 1
Neutralize the excluded entry `stringCompletions.ts :: getStringLiteralCompletionsFromSignature`
(judgment-boundary consumer, not an adjudicated wrong answer) — move to notes. Else ACCEPT.

### mypy-meet-types — drop impl-body sites (now the Python SECONDARY task)
10 of the 25 real sites are meet.py-internal `TypeMeetVisitor.visit_*`/helpers = S's own
implementation body, which every other task excludes as definition surface. DROP them from
real `sites[]` (gold ~25→~15, D1 unchanged at 3). KEEP `narrow_declared_type` + its 2 consumers.
Document the deviation in ADJUDICATION.md. (D1=3 is inherent — do not try to inflate it.)

### django-check-registry-run-checks — add 1 missed D1 (now the Python HEADLINE task)
ADD the missed D1 site `django/tasks/checks.py :: check_tasks` (`@checks.register`, zero
`run_checks` text → D1). Gold 58→59, D1 50→51, D-files 18→19. Neutralize the
`DiscoverRunner.run_checks` bait (transitively reaches CheckRegistry.run_checks via
`call_command("check")` — beyond the consumer stop, but NOT a wrong answer → move to notes).

## ACCEPT unchanged
- typescript-resolve-alias (cleanest in the corpus)
- guava-equivalence-doequivalent (D-file denominator 3 is a documented caveat, not a fix)

## REJECT (no edit; excluded from the run matrix)
- guava-forwardingmap-standard-containskey (admission FAIL, kept as a failure record)
- guava-converter-doforward artifacts if any (admission FAIL)

After all edits: for EACH changed gold, run the perfect-arm dry-run
(`cd eval && uv run python -c "from tier_c.structural import score_structural,load_gold; g=load_gold('tier_c/gold/<id>/gold.json'); real=[{'file':s['file'],'symbol':s['symbol']} for s in g['sites'] if s['adjudication']=='real']; r=score_structural(real,g); print(r.file_f1, r.d_recall, r.gold_size, r.d_gold_size, r.d_gold_file_size, r.phantom)"`)
and confirm file_f1=1.0/d_recall=1.0/phantom=0. Record the new |gold|/D1 in each ADJUDICATION.md.
