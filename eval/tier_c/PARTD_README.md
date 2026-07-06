# Tier-C Part-D — structural blast-radius scaffold

Design-of-record: `docs/superpowers/specs/2026-07-05-tier-c-part-d-structural-tasks-design.md`.
Corpus: `eval/tier_c/issues/structural.toml` (2 tasks locked: `prometheus-matchstring`,
`ruff-typechecker-match-annotation`; a 3rd archetype-A slot is deferred — see the
file's STATUS comment).

This is the measurement **scaffold**: it builds gold CANDIDATES and scores an arm's
claimed impact set against a frozen, adjudicated gold set via pure set math. It does
**not** adjudicate gold itself — that is the controller's source-verified step — and
it never runs a live agent arm.

## Pieces (P1–P6)

- **P1 — `structural_corpus.py`**: loads `issues/structural.toml` into
  `StructuralTask` (id, repo, lang, sha, symbol, receiver, `def_site: (file, line)`,
  dispatch, prompt_change, grep_name_stats, notes). Mirrors `corpus.py`'s loader
  style; fails loudly on a malformed entry.

- **P2 — `buildgold.py`** (`tier-c build-gold`): for one task, emits candidate
  impact sites WITHOUT deciding truth.
  1. Opens the repo checkout at `sha` (`checkout.Checkout`).
  2. LSP candidates: `tier_a.oracles.LspOracle` (rust-analyzer/gopls) —
     `document_symbols` at `def_site` → match by name + line-containment → direct
     `callers()` (incoming call hierarchy only; no transitive closure here).
  3. prism candidates: shells `prism nav callers --repo <repo> --symbol <symbol>
     --format json` (env `PRISM_BIN`, else `target/release/prism`), parsed via the
     already-battle-tested `tier_a.sut.extract_callers` Evidence-JSON mapping.
  4. Merges on a normalized `(file, norm_symbol)` key → provenance `{both, lsp,
     prism}`.
  5. D-membership per site (from the design §2 admission gate, reused as the
     scoring headline's denominator): `git grep -nw <name>` at the sha — `D1` =
     zero occurrences of the target name in the candidate's file, `D2` = name
     present but the name has >100 repo-wide hits, else `none`.
  6. Writes `gold/<task_id>/candidates.json` (full site list + `oracle_health`)
     and `gold/<task_id>/adjudicate.md` (ONLY the `lsp`/`prism` disagreement band,
     each entry with a source snippet + blank `verdict:`/`reason:` fields; `both`
     sites are listed separately as auto-accepted, no verdict needed).
  Graceful degradation: any LSP/prism failure → empty candidates for that source
  + a recorded `oracle_health` string (`"unavailable: ..."` / `"seed-miss: ..."`),
  never a crash — both files are always written.

- **P3 — `structural.py`**: the pure set-math scorer (no LLM, no prism; mirrors
  `investigator.py`'s shape). `norm_symbol` strips a leading receiver/scope
  qualifier + generics and casefolds (`(*T).MatchString` → `matchstring`,
  `TypeChecker::match_annotation` → `match_annotation`). `score_structural(claimed,
  gold, *, verify_exists=None)` returns a `StructuralReport`:
  - `file_f1` / `symbol_f1` (precision+recall+F1 over files, then over normalized
    `(file, symbol)` pairs) — file-F1 is primary (forgiving of symbol spelling).
  - `d_recall` — **the headline**: file-level recall restricted to gold sites
    with `d_member in {D1, D2}` (does structural nav change the ANSWER?).
  - `phantom` — claimed sites verified (via the injected `verify_exists(file,
    symbol)`, backed by `verify_site_exists`/`Checkout.read_text`) to be truly
    nonexistent in the checkout. `unmatched_extra` — claimed sites not in gold
    but NOT verified-nonexistent (a precision miss, routed to adjudication, never
    a phantom penalty — this is the design §5a "claimed-but-real-and-not-in-gold
    is not a phantom" rule).
  `load_gold(path)` reads a frozen `gold.json`.

- **P4 — `impact.py` + `prompts.py`**: `_STAGE["impact"]` is one IDENTICAL
  instruction for both arms (a fenced ` ```json ` block `{"impact":
  [{"file","symbol","reason"}], "migration_order": [...]}` then free-text
  discussion). `parse_impact_block` deterministically extracts the LAST such
  block (a draft block earlier in the text never shadows the final answer).
  `run_impact_with_retry(run_once, prompt)` calls the arm once; on a parse
  failure it re-prompts ONCE with `RETRY_PROMPT` appended; failure-after-retry
  raises `ArmRunError` (the existing arm-failure path — never a silent zero
  score).

- **P5 — `partd.py`** (`tier-c run-partd`): `run_partd_cell` clones
  `run_partc_cell`'s composition (fresh off-arm → score → fresh on-arm → gate on
  `used_prism` → score → `scan_leak` → deltas) producing a `PartDCell`
  (`d_recall_delta`, `file_f1_delta`, dose/administered/leaked, token/cost/wall
  accounting, migration orders). `_run_partd_live` clones
  `cli._run_partc_live`'s persistence EXACTLY: `manifest.json` written before any
  arm runs; per-arm `meta.json`/`prompt.txt`/`out.md`/`raw.jsonl` written
  immediately via the REUSED `cli._persist_one_arm`; `status.json` always
  written (success or failure, with `failed_stage`); ONE run-id per run dir
  (`--force-new` clobbers, the Part-C loop bug); the cell JSON is persisted via
  the REUSED `cli._persist_partc_cell` (fully generic — no Part-D-specific
  duplication needed). Both arms use the same steered-on/unsteered-off
  composition, same warm-gate + matched-binary preflight, same `Checkout`.

- **P6 — dry-run harness check**: `tests/test_tc_structural.py`'s
  `test_p6_dry_run_scorer_against_frozen_gold_fixture` scores a hand-written fake
  arm impact list against a hand-written frozen `gold.json`-shaped fixture and
  asserts exact `file_f1`/`symbol_f1`/`d_recall`/`phantom`/`unmatched_extra`
  values (design §8.3).

## `gold.json` schema (frozen by the controller; read-only to the scorer)

```json
{
  "task_id": "prometheus-matchstring",
  "repo": "prometheus",
  "sha": "505095b",
  "symbol": "MatchString",
  "sites": [
    {
      "file": "labels/dispatch.go",
      "symbol": "CallSite",
      "line": 4,
      "provenance": "both",
      "adjudication": "real",
      "d_member": "D1",
      "reason": "source-verified: dispatches via .Matches(), name never appears"
    }
  ]
}
```

`adjudication` is `"real"` or `"excluded"`; only `"real"` sites count toward gold.
`provenance` and `d_member` ride along for reporting — the scorer never weights
by provenance (LSP is a candidate source and cross-check, never the measurand).

## Commands the controller runs next

Build gold candidates for both locked tasks (writes
`eval/tier_c/gold/<task_id>/{candidates.json,adjudicate.md}`; run from `eval/`):

```bash
cd eval
uv run tier-c build-gold --task prometheus-matchstring \
    --bench-root ~/code/bench-repos --gold-root tier_c/gold
uv run tier-c build-gold --task ruff-typechecker-match-annotation \
    --bench-root ~/code/bench-repos --gold-root tier_c/gold
# or both at once:
uv run tier-c build-gold --all --bench-root ~/code/bench-repos --gold-root tier_c/gold
```

Then: open each `gold/<task_id>/adjudicate.md`, source-verify the disagreement
band (fill in `verdict: real|excluded` + `reason:` per site), fold the verdicts
plus the auto-accepted `both` sites into a hand-frozen `gold/<task_id>/gold.json`
(schema above).

Once `gold.json` is frozen, run one cell (steered-on vs unsteered-off, scored
against that file):

```bash
uv run tier-c run-partd --task prometheus-matchstring --model opus-4.8 \
    --live --run-id partd-2026-07-05-01 \
    --bench-root ~/code/bench-repos --gold-root tier_c/gold
```

`rescore.py`/the `rescore` subcommand were NOT extended for Part-D in this pass
(scoring fixes can be re-derived from the persisted `*.raw.jsonl` the same way
Part-C does, since `_run_partd_live` reuses `_persist_one_arm` verbatim — wiring
a `rescore-partd` path is straightforward follow-up, not required for the
scaffold to be rescorable-in-principle).

## Live smoke-test finding (read before adjudicating)

`build-gold` was dry-run once against the real pinned `prometheus-matchstring`
checkout (real gopls + the real `prism` binary — a deterministic tool
invocation, not a live agent arm) to validate the pipeline. Result: 21
candidates (4 `both`, 17 `prism`-only), `oracle_health: {"lsp": "ok", "prism":
"ok"}`, **every site's `d_member` came back `none`** — i.e. this direct-callers
pass alone will NOT satisfy the task's own admission gate (`D1 >= 3`) as
authored in `structural.toml`. Two things going on, both expected given the
spec's own scoping, not tooling bugs:

1. Most `prism`-only candidates (`config.go:UnmarshalYAML`,
   `discovery/http/http.go:Refresh`, ...) are prism's `r6_single_owner`
   NameOnly-tier candidates for calls like `patRulePath.MatchString(sf)` —
   these almost certainly call the **stdlib** `regexp.Regexp.MatchString`, not
   `FastRegexMatcher.MatchString`; prism assigns them to the one in-repo owner
   named `MatchString` because Go's `regexp.Regexp` isn't a repo symbol prism
   can rule out (the documented P6-lite precision floor in `CLAUDE.md`). Expect
   most of these to adjudicate `prism_fp`.
2. The task's own worked example is that the TRUE name-absent callers reach
   `FastRegexMatcher.MatchString` two hops away, through
   `labels.Matcher.Matches` — but P2's build-gold intentionally does **direct
   callers only** (spec P2 step 2: "do NOT try to compute transitive closure
   here"). So depth-1 LSP/prism candidates will never surface those D1 sites;
   `model/labels/matcher.go`'s `Matches` itself is a genuine DIRECT caller
   (`m.re.MatchString(s)`, `m.re` is `*FastRegexMatcher`) and correctly grades
   `d_member: none` (it does name "MatchString").

**Action for the controller**: after running `build-gold`, ALSO run
`prism nav callers --location model/labels/matcher.go:108` (the `Matches`
method) — or the LSP equivalent — and fold the transitive callers of `Matches`
into `gold.json` by hand as `D1` sites (with `provenance` left `"prism"`/`"lsp"`
and a `reason` noting the 2-hop path), so the task clears its own admission
gate. This is a one-time manual step per task, not a recurring tooling gap.

## Deviations from the spec's literal wording

- P2 shells `prism nav callers --symbol ... --format json` through a thin
  `_default_prism_runner`, but the JSON→CallEdge mapping is NOT re-derived — it
  reuses `tier_a.sut.extract_callers` (already exercised against the real
  binary's wire shape; verified live once against this repo's own binary to
  confirm the shape still matches before writing any code).
- `Checkout` gained one new read-only method, `read_text(rel)` (whole-file text),
  needed because the phantom check (P3) has no line number to anchor
  `read_window`/`read_line` on for an arm-claimed `(file, symbol)` pair.
- `rescore`-for-Part-D and a `run-partd --all`/matrix runner were not built —
  out of scope for "stand up the scaffold" (P5 explicitly says unit-test the
  wiring only, not run it live).
