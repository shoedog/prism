# Task — build a Part-D gold set via renaming-forwarder closure

Worktree: /private/tmp/prism-partd (branch tier-c-part-d). You produce a DRAFT
`gold.json` for your assigned task(s); the controller source-reviews + freezes. Do NOT
push. Do NOT run live agent arms.

## Read first (all three)
1. METHODOLOGY: `docs/superpowers/specs/2026-07-05-tier-c-part-d-gold-methodology.md` —
   the renaming-forwarder-closure rule, prism-independence invariant, gold.json schema,
   and the per-task STARTING ENUMERATIONS (Fable's, which you VERIFY, not trust).
2. TEMPLATE (copy its exact structure): `eval/tier_c/gold/prometheus-matchstring/gold.json`
   — a completed worked example: `sites` array with per-site {file, symbol, line,
   token_anchor, receiver_evidence, hop_distance, role, d_member, provenance, adjudication,
   reason}; `adjudication:"excluded"` sites for phantom bait; `closure_summary`;
   `excluded_test_helpers`.
3. CORPUS: `eval/tier_c/issues/structural.toml` — your task's symbol, def_site, dispatch,
   scope (in/out) lines. Honor the scope exactly.

## The procedure (per task)
1. **Cross-check (optional but recommended):** run build-gold for the LSP∪prism candidate
   set + D-membership: `cd eval && PRISM_BIN=/Users/wesleyjinks/code/slicing/target/release/prism
   PRISM_MCP_BIN=/Users/wesleyjinks/code/slicing/target/release/prism-mcp uv run tier-c
   build-gold --task <task-id> --bench-root ~/code/bench-repos --gold-root tier_c/gold`.
   This writes candidates.json + adjudicate.md (cross-check + exclusion data). It is a
   CROSS-CHECK ONLY — grep-per-hop below is the generator of record.
2. **Level 0:** `git grep -nw <S>` at the repo's checked-out SHA. Source-verify each hit's
   RECEIVER TYPE. True direct callers of S (the exact receiver) → gold; same-name/
   other-receiver hits → `adjudication:"excluded"` sites (phantom bait) with a reason.
3. **Find the thin FORWARDER(s):** the caller(s) that merely forward S's result (thinness
   test in the methodology: "if S's contract changes W's necessarily changes and W adds
   nothing of its own" — negation/plumbing/aggregation/tag-dispatch/monomorphization only).
   Record it as a gold site, role="forwarder".
4. **Recurse per forwarder W:** `git grep -nw <W>` → receiver-verify → classify each caller
   CONSUMER (gold, stop) or FORWARDER (gold, recurse). Terminate when no forwarders remain.
5. **Collapse to (file, symbol)** — the enclosing function of each call site (multiple call
   sites in one function = ONE gold entry). Determine each site's enclosing symbol.
6. **D-membership per gold site:** d_member="D1" iff the site's FILE has ZERO textual
   occurrence of S's name (`grep -c <S> <file>` == 0); "D2" iff name present AND repo-wide
   `git grep -c <S>` total > 100; else "none".
7. **Every gold site carries a TOKEN ANCHOR** (`<name>@file:line` — the textual token grep
   found it by; S's name at L0, the forwarder's name at each hop) and a **receiver_evidence**
   line (the declaration that types the receiver). PRISM-INDEPENDENCE INVARIANT: no gold
   site may be provenance:["prism"]-only — every site must be grep-anchored.
8. **Exclusions:** put same-name/other-receiver collisions + out-of-scope siblings as
   `adjudication:"excluded"` sites (phantom bait). Exclude non-_test.go TEST-HELPER files
   from gold (list them under `excluded_test_helpers`) — production code sites only.
9. **Admission:** |gold(real)| must be 8-60 and D1 count ≥3 (or (D1+D2)/|gold|≥0.3). If the
   closure exceeds 60, STOP and report — the target fails admission (do NOT truncate).
10. **Dry-run** the scorer to sanity-check (perfect arm → file_f1 1.0/d_recall 1.0/phantom 0;
    a grep-S-only arm that misses the D1 sites → d_recall low):
    `uv run python -c "from tier_c.structural import score_structural,load_gold; g=load_gold('tier_c/gold/<task-id>/gold.json'); perfect=[{'file':s['file'],'symbol':s['symbol']} for s in g['sites'] if s['adjudication']=='real']; r=score_structural(perfect,g); print(r.file_f1, r.d_recall, r.gold_size, r.d_gold_size, r.phantom)"`

## Task-specific notes
- **caddy-requestmatcher-migration** (archetype-B, Go, ~/code/bench-repos/caddy @ 77e9ce7):
  NOT a single-symbol closure — it's an interface MIGRATION. Gold = every site that must
  change to delete `RequestMatcher` (caddyhttp.go:43) and consolidate on
  `RequestMatcherWithError` (caddyhttp.go:55): the legacy `Match(r *http.Request) bool`
  impls (D1 — impl files never spell RequestMatcher; grep -lw RequestMatcher = 5 files
  only), the dual-dispatch type-switches (routes.go:352/373/445, httptype.go:1572), the
  interface guards (`var _ RequestMatcher = ...`), CEL factories (vars.go:236/369/392), and
  deprecated MatcherSet.Match (routes.go:411). Verify Fable's ~17 impls + switches by source.
  The "forwarders" here are the dispatch type-switches; the D1 impls are found by their
  METHOD SIGNATURE `Match(...) bool` (grep `func.*Match(.*http.Request) bool`), not the
  interface name.
- **ruff-typechecker-match-annotation** (Rust, ~/code/bench-repos/ruff @ 44f6d18): 2 hops.
  S=match_annotation (trait TypeChecker, typing.rs:615) ← forwarder `check_type::<T>`
  (typing.rs:625, monomorphization) ← ~10 named wrapper fns (is_list/is_dict/is_set/is_int/
  is_float/is_string/is_bytes/is_tuple/is_io_base/... — VERIFY the full list via
  `git grep -n "check_type::<"` and the fns that call check_type) ← lint-rule consumers of
  those is_* wrappers. Also handle match_initializer (sibling protocol method) if in scope.
  Decoy: `match_annotation_to_complex_bool` (flake8_boolean_trap) = excluded phantom bait.
  Recurse the is_* wrappers to their callers (that's the D1 layer — consumers say is_list,
  never match_annotation). This may be LARGE — if |gold|>60, report and propose a scope
  narrowing (e.g. a subset of wrappers) to the controller rather than truncating.
- **ruff-imported-qualified-name** (Rust, ruff @ 44f6d18): the D2/PRECISION task, NOT a deep
  closure. S=qualified_name (trait Imported, binding.rs:719; 4 impls incl AnyImport variant
  forwarding). Gold = the ~40 `.qualified_name()` call sites in ~16 non-test files where the
  receiver is an `Imported`/`AnyImport` (NOT the QualifiedName type, NOT same-name struct
  fields binding.rs:500/511/520, NOT same-name methods imports.rs:60/definition.rs:63/
  ty class.rs:693/1012/type_alias.rs:302 — all excluded phantom bait). The hard part is
  RECEIVER DISAMBIGUATION per call site (873 grep hits). d_member here is mostly D2 (name
  present, >100 repo hits). Collapse to (file, enclosing symbol).

## Deliverables
- `eval/tier_c/gold/<task-id>/gold.json` (draft, `status:"DRAFT — controller review pending"`),
  same schema as the prometheus template.
- A short `eval/tier_c/gold/<task-id>/ADJUDICATION.md`: the closure walk (hops, forwarders +
  thinness justification, per-hop grep counts), the |gold|/D1 numbers, the dry-run output,
  the exclusion list, and ANY site you were UNSURE about (flag for controller).
- Commit per task (trailer `Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>`), NO push.
- FINAL MESSAGE ≤14 lines per task: |gold|, D1 count, hops, dry-run numbers, admission
  pass/fail, and your top 1-3 uncertain sites for controller review.

## Escalate (stop, report) if
- The closure exceeds 60 sites (admission fail — propose narrowing, don't truncate).
- A forwarder's thinness is genuinely ambiguous (you can't decide consumer vs forwarder).
- The receiver type at a call site can't be determined from local source.