# Tier-C Part-D — Structural analyze/design/refactor tasks (design-of-record)

Date: 2026-07-05. Status: **DESIGN — first slice greenlit by owner.** Companion to the
Part-C measurement (`docs/superpowers/specs/2026-06-27-tier-c-part-c-*`) and the
scorecard-v2 rubric (branch `partc-rubric-v2`). Synthesizes a Fable design consultation
(2026-07-05) + the owner's LSP-fallibility caveat.

## 0. Why Part-D exists (the finding that forced it)

The Part-C pilot (5 live cells, ruff/pydantic × opus-4.8/gpt-5.5, spec stage) showed:
prism engages fine (dose 2–8 nav calls/arm) and reliably crushes citation hallucination
(opus 19→0, gpt-5.5 31→2), **but on debug-and-fix tasks BOTH arms pin the same root
cause and propose the same fix** — prism changes the *grounding*, not the *answer*.
Debug-fix is localizable (grep + read one function suffices), so prism's real value
surface — **structural navigation** (`nav_repo_map`, `nav_callers`/`nav_callees`,
`nav_ego_graph`, `nav_module_deps` = architecture, dependency and blast-radius reasoning
over a large unfamiliar codebase) — is never exercised. Part-D is the task family where
structural nav should change the ANSWER, not just its citations.

## 1. Task archetypes

- **A. Refactor blast-radius** (first-slice archetype): "We will change the
  signature/semantics of symbol S. Enumerate every site that must change (file+symbol),
  grouped by module, with a migration order and risk notes." The enumerated set IS the
  answer. Worked example (sized live): prometheus `(*FastRegexMatcher).MatchString`
  (`model/labels/regexp.go:328`) — `git grep -w MatchString` returns 20+ files, mostly
  *other* types' `MatchString`; the true caller set needs type-resolved nav + the
  `labels.Matcher.Matches` dispatch path where the name never appears. (Counter-example
  that fails admission: `NewFastRegexMatcher` — 3 grep hits, both arms converge.)
- **B. Interface/dispatch impact**: "Add a method to interface I / change concrete type
  T — what breaks, which impls change." Oracle = LSP implementations + references;
  strongest name-absent density (Go/Rust traits).
- **C. Feature-scoping with peer-derived gold**: "Add a new X analogous to peer P" — gold
  = P's registration/wiring/test surface, computed from P's reference set. Leakage-prone
  where "how to add a rule" is documented; admit only under the recency criterion.
- **D. Architecture path**: "How does subsystem A drive B — name the modules and boundary
  functions and the seams." Weakest oracle (multiple true narratives); slice 2, scored on
  the edge SET named, not prose.

## 2. Admission gate (mechanical, frozen before any run; extends `corpus.py:12` Goldilocks)

1. Gold size 8 ≤ |gold| ≤ 60 distinct sites.
2. **Non-grep-localizability, operationalized**: `G = git grep -nw <name>` at the SHA.
   `D1` = gold sites in files with ZERO textual occurrence of the name (aliased imports,
   re-exports, dispatch). `D2` = name present but repo-wide hits > 100. **Admit iff
   `D1 ≥ 3` OR `(D1+D2)/|gold| ≥ 0.3`.** Persist D-membership per site — it is the
   denominator of the headline metric AND the degeneracy detector.
3. Answer-changing: deliverable is the enumerated set (§4 contract); narrative secondary.
4. Training-leak resistance: prefer symbols added/moved/renamed after model cutoff
   (`git log --follow --diff-filter=AR`); verify pinned-SHA gold differs from any famous
   historical layout. (Doubles as degeneracy defense — memorized architecture lets the
   off-arm converge without grep.)
5. Repo scale ≥ ~1.5k source files (ruff/prometheus/excalidraw qualify).
6. Oracle health: the language server completes + call-hierarchy answers on the target
   (Tier-A: rust-analyzer/gopls trusted; pyright/tsserver need a larger adjudication band).

## 3. Arm structure (unchanged, deliberately)

Fresh **steered prism-ON** (`prompts.py:18 _STEER_PRISM_ON`, re-weighted toward
`nav_callers`/`nav_ego_graph`/`nav_module_deps` for this family) vs fresh **unsteered
prism-OFF** (normal grep/read) — same prompt otherwise, same `run_partc_cell`
composition. The off-arm's "normal grep/read on an unfamiliar repo" IS the status quo we
price. Keep the **GO-only capability-steer sentinel** (`prompts.py:43`) — MORE
load-bearing here since the "enumerate all sites" contract is itself a mild capability
steer, so a GO must show `on > capability-off` to be a *prism* GO, not a *prompt* GO.
Dose/administration (`partc.py:212-228`), leak scan (`leak.py:22`), immutability carry over.

## 4. Output contract (parity, mechanical parse)

Both arms get an identical new `_STAGE["impact"]` instruction: a fenced JSON block
`{"impact": [{"file","symbol","reason"}], "migration_order": [...]}` + free-text design
discussion. Deterministic parse; ONE auto re-prompt on parse failure; failure-after-retry
= arm failure (not a zero score). Identical contract both arms → no condition tell.

## 5. The scoring oracle — adjudicated gold, LSP-primary, prism as candidate generator

**Scoring is pure set arithmetic against a frozen, adjudicated gold set.** Build once per
task (offline, ~free):

1. Candidates = `LSP(S) ∪ prism(S)` (reuse `tier_a/oracles.py LspOracle`,
   `tier_a/lsp_client.py`; call-hierarchy/references/implementations), provenance-tagged
   `{lsp, prism, both}`.
2. **Auto-accept `both`** (LSP∩prism agree).
3. **Adjudicate the disagreement band** (`lsp-only`, `prism-only`) with the κ-validated
   Tier-A taxonomy (`eval/tier_a/adjudication.py`), **SOURCE-VERIFIED** (read the actual
   code — the arbiter is the source, NOT which tool found it):

   ### 5a. The LSP-fallibility handling (owner caveat — load-bearing)
   LSP is NOT ground truth. Tier-A has an `oracle_miss` category: real edges prism found
   that LSP missed (~14 ripgrep / ~20 cobra in the 2026-07-04 pass alone). Therefore:
   - **prism-only, adjudicated real** (`oracle_miss`/`prism_fn`) → **ENTERS gold** (prism
     correctly found what LSP couldn't — this becomes prism's *credit*, not a phantom
     penalty). This is the exact case the owner flagged.
   - **lsp-only, adjudicated real** (prism false-negative) → enters gold (prism penalized,
     correctly).
   - **either-only, adjudicated not-real** (`prism_fp` / `oracle_artifact` = source-
     invisible) → excluded.
   Gold = **adjudicated truth**, i.e. LSP output is a *candidate source and cross-check*,
   never the measurand.
4. Freeze `gold.json` with per-site D-membership + provenance + adjudication verdict.
   Arm-blind by construction.

**Metrics per arm (all mechanical, rescorable forever):**
- **file-F1** (primary; forgiving of symbol spelling), **symbol-F1** (secondary,
  normalized).
- **D-recall** — recall restricted to the D (grep-hard) subset. **THE headline "does
  structural nav change the answer" number.**
- **phantom-site count** — arm-claimed sites NOT in gold **AND adjudicated non-existent**
  (never "LSP-absent but real"). Novel claimed sites route through the same adjudication
  (the "adjudicated-extras channel") — a *correct* novel find is ADDED to gold, not
  penalized.
- precision vs gold ∪ a small adjudicated-extras allowance (tests/docs; scope the prompt
  to code sites to minimize).

**Bias control (the reciprocal risk — adjudication rubber-stamping prism):** source-
verification is the arbiter, not provenance; dual-rater on a sample with the κ check
(Tier-A discipline); **report the LSP-vs-prism disagreement rate per task** as a health
metric on the gold. This is the same process that already handled prism-vs-LSP conflicts
across the whole Tier-A baseline.

Why this beats alternatives: planting real edges mutates the pinned repo (synthetic
code, sanitation burden, grades one fact not the set) — keep planting for the §1.5
false-premise *probe* only. Stronger-model set-grading re-imports the ±0.18 judge noise
the 2026-06-28 ensemble redesign escaped — use strong models ONLY inside the one-time
gold adjudication band. The set-math oracle is deterministic, rescorable, prism-
INDEPENDENT (fair to the off-arm), and makes the ANSWER the measurand.

Biggest oracle risk: LSP incompleteness on dynamic code (Python metaclasses, TS barrels)
→ under-complete gold. De-risk: pilot on Rust/Go/TS (Tier-A trusts those servers); the
adjudicated-extras channel catches novel-but-real claims; hold Python for slice 2.

## 6. Degeneracy prevention & detection

Prevention: the §2 admission gate (D-share + scale + recency) + record (don't cap)
budgets. Detection, per cell and pooled: (i) **ΔD-recall** — if off ≈ on on the D subset,
the family failed its purpose regardless of F1; (ii) **claimed-set Jaccard(on, off)** —
the convergence measure the debug-fix pilot lacked; (iii) audit the off-arm's `commands`
(`model.py:55`) — if it recovered D1 sites via grep, that site's D1 classification is
falsified; investigate before trusting the cell. **Pre-register**: pooled ΔD-recall < 0.1
= a failed instrument AND evidence AGAINST the thesis (accept it; do not redesign until
prism wins).

## 7. Minimal first slice (greenlit)

- Corpus: **3 tasks, archetype A** — ruff (Rust), prometheus (Go, the `MatchString`
  family), excalidraw (TS). Already cloned/pinned/prewarm-tested; reuse SHAs. Python
  deferred (oracle risk). Optional +1 archetype-B Go task (caddy — gopls dispatch oracle
  already validated) if gold-building is smooth.
- Cells: 3 tasks × 2 models × {on, off} = **12 arms** + ≤3 GO-only sentinel arms
  (comparable spend to the Part-C pilot just run).
- New code (small, parallel to Part-C, reusing its spine): `issues/structural.toml` +
  a loader with the §2 gate; `structural.py` scorer (set math, ~100 lines, mirrors
  `investigator.py`); a `tier-c build-gold` subcommand wrapping `LspOracle` + prism
  candidate merge + the adjudication file; an `impact` stage in `prompts.py:11` + JSON
  parser; a `run-partd` path cloning `_run_partc_live`'s persistence so `rescore.py`
  works day one. Judges/ensemble/leak/detect/dose reused unmodified.
- **Pre-registered read-out**: VALIDATED if median ΔD-recall ≥ +0.2 and Δfile-F1 > 0 in
  ≥2/3 tasks per model, with off-arm phantom-count ≥ on-arm; REFUTED if ΔD-recall ≈ 0
  with high off-arm D-recall (grep suffices — a real finding, accept it); INSTRUMENT-FAIL
  if off-arm D-recall is high AND its commands show it never structurally searched (D
  classification was wrong).

## 8. Build order

1. `build-gold` tooling + adjudicate the 3 tasks' disagreement bands (§5a — the human/
   controller step; source-verified). Freeze `gold.json`.
2. `structural.py` scorer + `impact` stage + `run-partd` persistence + tests.
3. Dry-run the scorer against the frozen gold with a hand-written fake arm output.
4. Run the 12 arms (matched binaries + warm-gate, per the harness hardening) with unique
   run-ids; score; report the §7 read-out.
