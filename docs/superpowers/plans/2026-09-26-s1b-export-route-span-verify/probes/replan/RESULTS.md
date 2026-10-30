# S1b re-plan probe results (Fable, 2026-09-26)

Expectations: `EXPECTATIONS-pre-run.md` (written first; addendum 1 after the first RP1 run). Binaries: base
`prism-base-a6d853f5` (`df22fbcf…`) and `prism-proto-v8e` (`346eddef…`). Fixtures: `fixtures/` (from
`probes/gen_rp.py`); raw dumps: `out/<binary>/RP*.dump.jsonl`, summary `out/RP-summary.txt`. Every run exited 0 with
empty stderr (`out/*/RP*.stderr`, 0 bytes). Corpus F appears below as aggregates only.

## RP1 Evaluation-context probes: H1 holds

| Probe | Base | v8 | Verdict on v8 | As expected |
|---|---|---|---|---|
| RP1-a computed method key, inner only (JS, TS) | both calls Exact → inner `f@3` | both → inner `f@3` | key call **wrong Exact** (sol r2 W1) | yes |
| RP1-b computed method key + outer `f` | both: 2 targets | both → inner `f@4` | key call **wrong Exact** | yes |
| RP1-c namespace merge (TS) | 2 targets | Exact → outer `f@1` | **wrong Exact** (Opus r2 W1) | yes |
| RP1-d enum member scope (TS) | no site row | no site row | inadmissible (site at module scope; addendum 1) | – |
| **RP1-d2** enum member scope inside a function (TS) | Exact → `f@1` | Exact → `f@1` | **wrong Exact** (`f` denotes `E.f`; a third position miss, new) | yes |
| RP1-e `with (f())` object position (JS) | Exact → `f@1` | drop `LocalBindingUnproven` | right edge lost (the object expression is evaluated outside the object environment) | yes |
| RP1-f nested dotted namespace `A.B` (TS) | Exact → `A.f@2` | drop `LocalBindingUnproven` | right edge lost | yes |
| RP1-g computed key on an object-literal method | 2 targets | both → inner `f@4` | key call **wrong Exact** | yes |
| RP1-h heritage `class f extends f()` | no site row | no site row | inadmissible (addendum 1) | – |
| **RP1-h2** heritage inside a function | Exact → `f@1` | drop | v8 right (T8: the inner class binding, TDZ) | yes |
| RP1-i parameter default `h(g, a = g())` | Exact → `g@1` | drop | v8 right (T3/J1, C122) | yes |

Three wrong-Exact positions (two from round 2, one new) and two right-edge losses, all in kinds the r2 table
classifies. The falsifier for H1 (every spec-derived position already right on v8) did not occur.

## RP2 Bounded round-2 items

| Probe | Base | v8 | Folded auditor (`jsscope.py` r3) |
|---|---|---|---|
| RP2-a header error (Opus W2) | Exact → `f@1` (base wrong too) | Exact → `f@1` (wrong) | `parse` (refuses) in JS and TS |
| RP2-b string brace (sol W2) | Exact → `f@1` | drop (right edge lost) | `callable` (kept) in JS and TS |
| RP2-c string-literal export name (sol W3) | `UnknownName` | `UnknownName` | – (collector-side; R3 gap) |

## RP3 Position census (`probes/position_census.py`, `census/*-census.json`)

| Corpus | Files | Sites | Distinct `(kind, field)` pairs crossed | Sites on any E-position | Classes (sites) |
|---|---|---|---|---|---|
| X | 619 | 22,970 | 82 | 18 (0.08%) | `for_in_right` 12, `param_env` 6 |
| R | 46 | 576 | 53 | 0 | – |
| T | 601 | 63,193 | 88 | 4,479 (7.1%) | `namespace_body` 4,312, `for_in_right` 107, `param_env` 33, `for_head` 23, `computed_key:pair` 10, `heritage` 3 |
| F (aggregate) | 1,119 | 24,431 | 67 | 1 | `param_env` 1 |

Grammar-possible pairs across the three pinned grammars: **319** (186 concrete kinds). Pairs crossed by any corpus
site (union): **103**. Never crossed: **216**. Decorators, computed *member* keys, `with` and enum bodies are crossed
by **0** sites on every corpus.

Joined with the v8d row-diffs (intrinsic relabels excluded): changed rows on an E-position: **X 0 / 215, R 0 / 0,
T 42 / 870, F 0 / 145** (4 F rows unmatched: their site keys are not identifier callees in the census; none are on an
E-position by construction).

## RP3b Base edges on E-positions (`probes/e_sites_join.py`, `census/*-esites.json`)

T `namespace_body`: base `local_def` targets on **3,195** sites; of all 4,312 namespace-body sites, the auditor's
walk binds **inside** the body for 3,342 (2,938 with `local_def` targets), **leaves** the body for 959 (257 with
`local_def` targets), 11 unbound. Of T's 42 changed rows crossing a namespace body, **41 bind inside** (13 removals,
28 re-targets) and **1 leaves** (`compiler/parser.ts:8797`, a nested `namespace JSDocParser` inside `namespace Parser`
calling Parser's own `createSourceFile@1978`: base 2 targets → v8 1, lexically right).

Merge-partner predicate (`census/T-namespace-merge-predicate.txt`): among every T site whose walk leaves a
namespace body, **0** have another same-file namespace block sharing the first name segment, and **0** are in script
files (all 18 namespace-bearing files are modules). So a rule that keeps base (or refuses) only when a partner exists
or the file is a script changes **0 rows** on T; X, R and F have no namespace-body sites.

Other E-positions with base `local_def` targets: T `param_env` 17, `for_in_right` 27, `for_head` 6, `heritage` 3; X
`param_env` 1, `for_in_right` 2; F `param_env` 1. All are positions v8 already evaluates correctly (RP1-i, T5, T8).

## RP4 Delimited-child sealing (`probes/rp4_delimited.py`)

Rows the narrow E6 rule kept and the broad rule refused: **T 12 / 12 kept, F 7 / 7 kept** under Opus's
delimited-child rule (every error lies inside a braced body of a function strictly inside the scope). The 45 rows E6
refuses (T 41, F 4) stay refused under the folded auditor (their errors are in interface type parameters, not inside
any function). **Cost of the Opus W2 fold on the corpora: 0 rows.**

## Custody

`MANIFEST.sha256` at this root (written last).
