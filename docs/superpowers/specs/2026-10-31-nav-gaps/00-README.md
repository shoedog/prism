# Nav-gap specs, 2026-10-31: reconciliation

Date: 2026-10-31
Status: draft for owner review
Base: main `533f061a`
Specs: `06-ranked-repo-map.md` (item 6), `01-find-symbol.md` (item 1), `04-statement-dependence.md` (item 4),
`02-composite-explore.md` (items 2, 3 and 5). The owner's priority order is 6, then 1, then 4, then 2. The motivating
analysis is [`ANALYSIS.md`](../../../analysis/2026-10-31-nav-gaps/gap-analysis/ANALYSIS.md); the evidence set behind all five documents, and what stays private, is
described in [its README](../../../analysis/2026-10-31-nav-gaps/README.md).

The four specs were written in parallel. This file records how they now fit together. Where a spec and this file
disagreed, the spec has been edited to agree. The authors' measurements and claims are unchanged. Each spec has a
`Reconciled (00-README X<n>)` note at every point that changed.

**Claim tags** follow the specs: *measured*, *source-verified*, *judgment*.

---

## 1. Reconciled decisions

| # | Decision | One-line rationale |
|---|---|---|
| X1 | The program adds **one** tool to the default MCP list: `nav_explore`, the 10th tool. The `tools/list` cap is **17,500 chars**, enforced by a test that F1b adds (section 2). | It is the only option that fits a cap near the specs' own 16,500, and `nav_explore` is the one new tool whose decision (02 §9) is about adoption, so it has to be on the list to be measured. |
| X2 | Find-symbol ships as a library and CLI (`nav find-symbol`, F1a). The seed fixes go into every seeded tool (F1b). On MCP, partial-name discovery is `nav_explore` with a `{kind:"query"}` seed, which returns 01's ranked candidates with handles. The standalone `nav_find_symbol` tool (01 S2, now **F2**) is contingent. | 01's own simulation (§8.3): the seed fixes alone resolve 17/17 seeds, the same as the search. What the new tool adds is partial-name discovery, which explore's `query` seed and `did_you_mean` already return. |
| X3 | `slice_at` is CLI plus an **opt-in** MCP tool: `prism-mcp --experimental-tools slice_at`, standalone only (S-2). From the default list it is reached only through `nav_explore`'s `dependence` section (E-5). It joins the default list (**S-P**) only if Stage C says "ship as an agent tool", and only within the cap. | Stage C (04 §2.3) is the gate that decides whether a model calls the tool, so it needs a callable tool, not a listed one. Listing it before Stage C would charge every run about 2.65k chars for an unproven tool. |
| X4 | **One seed grammar** (section 3). Spec 01 owns it, in §5.1–5.3. Two extensions: `variable` (owner 04) and the `query` kind (owner 02). F1b adds one callable-selection routine that both resolvers call: `navigation::seed` for the nav tools and explore, and `reasoning::seeds` for `taint_reaches` and `slice_at`. | Without it, qualified names and handles would work in `nav_callers` and fail in `slice_at` and `taint_reaches`, because those resolve symbols through `reasoning::seeds::resolve_symbol` (source-verified, `src/reasoning/seeds.rs:213-300`). |
| X5 | `SymbolCatalog` has one owner, spec 01 §5.10. It is built in its own small PR (**N0**), before 06's S1 and 01's S1, and gains 06's cross-file caller counts. | Both specs said "whichever lands first adds it". That would collide if they ran in parallel; N0 lets them run in parallel. |
| X6 | A type seed is 02's explore-local `TypeRef` (02 §6.1). v1 adds no `SymbolRef::Type`. 01's non-goal text is corrected to match. | 01 assumed a `SymbolRef::Type` variant that 02 rejects, because a new variant forces every exhaustive match to change. |
| X7 | Tools with a dedicated result (the ranked `nav_repo_map` and `nav_explore`) share one `format` vocabulary: `agent_markdown` (default), `canonical_json` or `agent_json`. Both JSON values render compact JSON of the dedicated result. R2 adds one generic clause to `VIEW_NOTICE`. That changes golden **id 1** (`initialize` instructions), which no spec had listed. | 02 had proposed `"agent_markdown" \| "json"`, which an agent that learned the other tools would get wrong. `VIEW_NOTICE` is composed into `instructions` (source-verified, `src/mcp/transport.rs:513`). |
| X8 | Golden transcripts get one owner PR per change. Merges are serial in this order: F1b, then R2, then E-4. New transcript ids come from reserved blocks (section 5). | All five goldens carry the id-3 `tools/list` line, so every tool or schema change re-blesses all five. |
| X9 | 02 now uses the names 01, 06 and 04 actually define (section 4). | 02 §8 assumed `find_symbol::resolve_seed/search`, `repo_rank::symbol_weight/file_rank` and `slice_at::backward`, none of which exist in the other specs. |
| X10 | PRs larger than about 800 LOC including tests are split (judgment): N0, F1a/F1b, S-1a/S-1b, E-1a/E-1b, E-3a/E-3b. | That keeps each PR inside a review cap of 2 rounds (steering: a slice too large to converge is a planning defect). |
| X11 | The cap is measured on the **compact id-3 `tools/list` line of the goldens: 13,540 chars today**. 04's 14,237 and its 2,548 for `taint_reaches`, and 02's per-tool range 714–2,548, describe the same payload serialised with Python's default separators and ASCII escaping. That serialisation reproduces 2,548 exactly (measured). Those numbers are left as written. | One metric for one test. |
| X12 | 06's Set B gates (D1/D2) and 01's Set B gates (M1/M3) run in **one host-quiet window**, once R1 and F1b have merged. | Both specs already propose it. The Part-D corpora builds are the expensive part. |
| X13 | A `loc` seed that carries `name` is a **callable handle**. Every seeded tool accepts it wherever it accepts a symbol seed, with the same meaning, including `variable` in `slice_at`. A `loc` seed without `name` keeps each tool's current line meaning. | Handles come out of find-symbol, disambiguation details, map symbol lines, explore candidates and `slice_at` stops. They are only useful if every tool takes them. |

## 2. Tool budget

**Method** (judgment, computed). Each proposed tool's input schema was drafted from its spec's parameter table in the
house style: compact serde JSON, `additionalProperties: false`, the shared seed `oneOf` inlined. Descriptions are set
at each spec's own cap plus 6% for JSON escaping. The per-tool envelope (`_meta`, `annotations`, `name`, `title`) is
237 chars, measured from `nav_callers` in the golden. The `loc` `name` property is +25 chars per inlined seed copy,
and there are 6 copies: 4 nav tools plus `taint_reaches`' `sources` and `sinks` (measured). 06's +900 is its own cap.

| Item | Estimated definition chars | The spec's own figure |
|---|---:|---|
| Baseline, 9 tools (compact id-3 line) | 13,540 | measured, 01/06 |
| 06 `nav_repo_map` ranked properties and description | +900 | 06 cap (+365 of it is schema) |
| 01 `loc` `name` on 6 seed copies (F1b) | +150 | inside 01's +2,100 |
| 01 `nav_find_symbol` (700-char description) | 1,604 | ≤ +2,100 including the seed property |
| 04 `slice_at` (1,300-char description; schema 1,039) | 2,656 | "+≈1.4k chars", which is the description only |
| 02 `nav_explore` without the `query` seed (900-char description) | 2,530 | "~400 tokens" ≈ 1.6k |
| 02 `nav_explore` with the `query` seed | 2,724 | — |

**Options** (default list only; CLI flags cost no agent tokens):

| Option | Default tools | `tools/list` chars | Δ vs today | ≈ tokens (÷4) | Fits 17,500? | What the specs' own evaluations say |
|---|---:|---:|---:|---:|---|---|
| A. All three tools | 12 | 21,380 | +7,840 (+58%) | 5.3k | no | Every new tool would be listed before its gate runs. 01 §8.3: the seed fixes alone match the search. 04 §2.3: Stage C may well return "verifier only" (the 2026-10-22 null). The trial attributed +8–25% per-run cost to unused definitions. |
| B. `slice_at` off the list; `nav_find_symbol` + `nav_explore` | 11 | 18,724 | +5,184 (+38%) | 4.7k | no | Respects 04's gate. But it lists `nav_find_symbol`, whose distinct value (01 §8.3) duplicates explore's `candidates` and `query` seed. |
| C. find-symbol folded into explore; `slice_at` + `nav_explore` | 11 | 19,970 | +6,430 (+47%) | 5.0k | no | Pays for `slice_at` before Stage C decides it is an agent tool. |
| **D. Both folded: `nav_explore` only** (recommended) | **10** | **17,314** | **+3,774 (+28%)** | **4.3k** | **yes (186 to spare)** | Each other capability stays reachable: discovery through `query` seeds plus `did_you_mean`, dependence through the `dependence` section. Standalone promotion stays open behind each spec's own gate (01 M1 plus owner, 04 Stage C). Explore's adoption gate (02 §9.1–9.2) can run. |
| E. No new tool | 9 | 14,590 | +1,050 (+8%) | 3.6k | yes | Cannot test 02's adoption decision, which is the whole point of item 2. |

**Recommendation: option D, with a single cap of 17,500 chars** on the compact id-3 line. F1b adds the test (it is
the first PR to change `tools/list`). The failure message prints per-tool sizes. Allotments per PR:

| PR | Allotment | Running total |
|---|---:|---:|
| F1b (`loc` `name`, 6 copies) | +150 | 13,690 |
| R2 (ranked `nav_repo_map`) | +900 | 14,590 |
| E-4 (`nav_explore`, 900-char description, `query` seed, `to`, all 8 section names) | +2,750 | 17,340 |
| Reserve | 160 | 17,500 |

The contingent promotions do not fit: F2 (`nav_find_symbol`, about 1,600) and S-P (`slice_at`, about 2,650). Either
one needs a recorded owner decision that raises the cap or frees the same amount (for example the `taint_reaches`
consolidation in 04 §7.1). That is deliberate: a promotion has to be weighed against its per-run cost. A 16,500 cap
would require cutting about 800 chars from `nav_explore` (dropping the `to`/`k`/`max_len`/`confidence` path controls
from the MCP schema, plus a 700-char description). 02's `path_to` design does not support that without losing the
section on MCP. Note also that 01's own allotments (13,540 + 900 + 2,100 = 16,540) already exceeded its 16,500 cap.

## 3. Unified seed grammar (owner: spec 01 §5.1–5.3)

```text
Seed :=
    {"kind":"symbol", "name": NAME, "file"?: PATH}
  | {"kind":"loc",    "file": PATH, "line": LINE}                 -- a line
  | {"kind":"loc",    "file": PATH, "line": LINE, "name": IDENT}  -- a callable handle (X13)

Extensions (any other tool rejects them with `bad arguments`, naming the tool that accepts them):
  "variable"?: IDENT   on a symbol seed or a handle               -- slice_at only        (owner 04 §4.2)
  {"kind":"query", "text": PATTERN, "file"?: GLOB}               -- nav_explore `seed` only, never `to`
                                                                   (owner 02 §3.1; matching and ranking are 01 §5.5)

NAME    := an exact indexed callable name (always tried first)
         | a qualified spelling: (*T).m  (T).m  A.b  A::b  A#b  A->b
           (01 §5.3: the last segment is the name; only the last qualifier is used in v1)
LINE    := 1-based line
PATTERN := 1–200 bytes; 01 §5.5 tiers (exact, normalised, prefix/glob, word prefix, substring, bounded fuzzy)
GLOB    := 01 §5.5 `file` filter syntax
```

**Who accepts what.**

| | `nav_symbol_spans`, `nav_callers`, `nav_callees`, `nav_ego_graph` | `taint_reaches` `sources`/`sinks` | `slice_at` (CLI; MCP opt-in) | `nav_explore` `seed` | `nav_explore` `to` |
|---|---|---|---|---|---|
| symbol, exact or qualified name, `file` | yes (F1b) | yes (F1b) | yes | yes | yes |
| loc (a line) | today's meaning | today's meaning | 04 §5.1 | 02 §4.1 | yes |
| loc + `name` (handle) | that callable (F1b) | that callable's symbol-seed meaning (F1b) | symbol-seed meaning (04 §5.1) | that callable | yes |
| `variable` | rejected | rejected in v1 | yes | rejected | rejected |
| `query` kind | rejected | rejected | rejected | yes, if 01's M1 gate passes | rejected |

**Resolution** (F1b): one callable-selection routine in `navigation::seed`. It does exact match, then the qualified
parse with qualifier penalty `q ≤ 1`, then the `file` filter, then the overload-stub rule. Both `seed::resolve_fn`
and `reasoning::seeds::resolve_symbol` call it. A handle selects the callable that 01 §5.1 picks. One candidate
resolves the seed. More than one gives `AmbiguousSymbol { candidates, details }` in 01 rank order. None gives
`SymbolNotFound { did_you_mean }`. Those error shapes therefore apply in every seeded tool. Explore turns
`AmbiguousSymbol` into its successful `candidates` section.

**Implementation note.** The shared `SeedInput` serde enum gains only the optional `loc.name` (F1b) and
`symbol.variable` (S-1a) fields. The `query` kind is parsed by explore's own input parser. That keeps golden id 14
(`unknown variant ..., expected symbol or loc`) byte-identical.

**CLI spelling.** `--symbol NAME [--file F]`, `--location F:L`, and `--location F:L --symbol NAME` together form a
handle (01 §5.1). `--variable V` is `slice-at` only and needs a symbol or a handle. `--query TEXT [--file GLOB]` is
`explore` only. The `taint-reaches --source file:line` spec string is unchanged.

**Who emits handles.** Find-symbol items, `AmbiguousSymbol.details[i].seed`, `did_you_mean`, 06's map symbol lines
(`file:line name`), explore `candidates` and `next_queries`, and `slice_at` `dependence.stops[].next_seed` (a handle
for `descent_refused`, a plain `loc` for `field_store`). 06's `focus.symbols` uses the NAME syntax but is not a seed:
it matches up to 20 defining files (06 §5.3).

## 4. Interfaces between the specs (who defines, who consumes)

| Interface | Defined by (PR) | Consumed by |
|---|---|---|
| `navigation::symbol_catalog::{SymbolCatalog, CatalogEntry}` (01 §5.10, plus `callers_exact_cross_file`, `callers_name_only_cross_file`) | 01 (N0) | 01 search and errors; 06 symbol lines (§5.4); 02 rank key 3 and `candidates` |
| `navigation::symbol_search::{find_symbols, SymbolQuery, SymbolMatch, SymbolSearchResult, to_evidence, parse_qualified}` | 01 (F1a) | 06 focus parsing (`parse_qualified`); 02 `query` seeds |
| `navigation::seed::{SeedHandle, resolve_handle}`, the shared selection, extended `resolve_fn`, `QueryError::AmbiguousSymbol.details` (`CandidateDetail`), `SymbolNotFound.did_you_mean` | 01 (F1b) | all seeded tools; 02 seed step and `candidates`; 04 symbol seeds via `reasoning::seeds` |
| `navigation::repo_rank::{rank_files, FocusSpec, FileRanking, RankedFileCore, pack_ranked_map, render_ranked_map, estimate_tokens}` | 06 (R1) | 02 `module` section (unfocused `rank_files`, seed file's `RankedFileCore`) |
| `navigation::slice_at::{run, SliceRequest}`: `run(&NavigationSession, &SliceRequest) -> Result<Evidence, QueryError>` | 04 (S-1a; control in S-1b) | 02 `dependence` section (E-5): `loc` seed, `direction: backward`, `reach: none`, `control: true`, `max_results: 12` |
| `navigation::explore::{explore, ExploreOpts, ExploreResult}`, `call_path::shortest_paths`, `implementations::query`, `type_index::TypeIndex` | 02 (E-1a, E-2, E-3a/b) | nothing in this program (optional reuse noted in 02 §8) |

## 5. Shared artifacts and their owners

| Artifact | Owner PR | Other PRs |
|---|---|---|
| `tools/list` size test, 17,500 cap | F1b | R2 and E-4 must stay inside their allotments; F2 and S-P need an owner decision |
| Golden id 1 (`VIEW_NOTICE` clause, X7) | R2 | E-4 does not touch it |
| Golden id 3 (`tools/list`) | F1b, then R2, then E-4 (serial merges, each re-blesses all 5 goldens after rebasing) | F2 and S-P if ever promoted; S-2 does not change it (opt-in) |
| Golden id 4 (`nav_repo_map {}` becomes the ranked map) | R2 | — |
| Golden id 12 (unknown-tool list) | E-4 | F2, S-P |
| Golden id 14 (seed variant error) | unchanged by design (section 3) | — |
| New transcript steps, inserted after id 17 (before the edit step) | F1b ids 30–34 (handle and qualified seeds); R2 ids 35–39 (focused, budgeted map); E-4 ids 40–49 | F2 50–54, S-P 55–59; S-2 pins its flag-on transcript in `tests/mcp/slice_at_test.rs` |
| `tests/mcp/smoke_test.rs` count (9 → 10), `registry.rs` tool-list test | E-4 | F2, S-P |
| `docs/MCP.md`: Seeding paragraph, handle gotcha, `did_you_mean` | F1b | — |
| `docs/MCP.md`: `nav_repo_map` row and ranked mode | R2 | — |
| `docs/MCP.md`: `--experimental-tools` flag row, `slice_at` (experimental) section, parameter-line and `field_store` gotchas | S-2 | S-P moves it into the tools table |
| `docs/MCP.md`: `nav_explore` row, `query` seed, "nine tools" → "ten tools" | E-4 | — |
| Skill: "Seed" step (qualified names, handles, read `details` when ambiguous) | F1b | — |
| Skill: step 1 and the "Where am I?" row | R2 | — |
| Skill: "orient with one call", "only part of the name → `nav_explore` query seed" | E-4 | F2 would add the `nav_find_symbol` row instead |
| Skill: "data flow and guards" routine (04 §7.3) | E-5 routes it to explore's `dependence` section; S-P switches it to `slice_at` | Stage C's arm carries 04 §7.3 verbatim in its harness (S-4) |
| CLAUDE.md MCP-adapter tool list and paragraphs | E-4 (tool list), S-2 (`slice_at` paragraph) | — |
| `Reason::{Match, Owner, CallerCount}` | F1a | — |
| `ReasoningReason::DependsOn`, `Evidence.dependence`, `ReasoningWarning::{FieldStore, DescentRefused, CfgIncompleteFunction, SeedAdjusted, NoDependence}` | S-1a | `ControlIgnoredForward`, `ControlFromSyntax`: S-1b |
| `evidence_view::view_reason` and `output/navigation.rs::render` arms | F1a, then S-1a (serial merges; different variants; no name collides with existing `Reason`/`WarningKind`/`ReasoningWarning` variants, source-verified) | — |
| Dedicated results `RankedRepoMap` (R1/R2), `ExploreResult` (E-1a), `TypeRef` (E-3a) | as listed | — |
| Mutation lanes (no name collides with existing `mutants/*.json`) | `mutants/slice-at.json` S-1a (extended in S-1b, S-3); `mutants/explore.json` E-1a (ranking; the fitter in E-1b); `mutants/call-path.json` E-2 | tier-2 triggers: F1b (seed resolution; lanes of `seed.rs` callers and `tools_reasoning.rs`), S-3 (trace walk) |
| Tier-A | none triggered, except `tier-a --matrix-only` as a control in S-3 (04 §9) | — |

## 6. Build order

The cap is 2 review rounds per PR (Opus-5.5 high and gpt-6.1-sol high). LOC are judgment and include tests.
"Merge-serial" means the PRs can be developed in parallel but rebase and merge one at a time.

| ID | Spec PR | Content | Depends on | Parallel with | LOC |
|---|---|---|---|---|---|
| P0 | 04 PR-0 | Stage A reproduction receipt (docs only) | — | everything | 60 |
| N0 | 01 (shared) | `symbol_catalog.rs` (`OnceLock` on `NavigationIndex`), cross-file caller counts, fixtures | — | P0 | ~200 |
| R1 | 06 S1 | `repo_rank.rs`, edge weight field, packer, renderers, CLI `repo-map --ranked`, eval harness | N0 | F1a, S-1a | ~600 |
| F1a | 01 S1a | `symbol_search.rs`, `Reason::{Match,Owner,CallerCount}`, CLI `nav find-symbol`, eval harness | N0 | R1, S-1a | ~450 |
| F1b | 01 S1b | shared callable selection (both resolvers), qualified seeds, handles, `details` and `did_you_mean`, computed sentence, MCP `loc.name` (6 copies), the **size test**, golden id 3, docs and skill Seed step | F1a | R1, S-1a | ~400 |
| **gate G-06/G-01** | 06 §9.3, 01 §8.4 | Set A anytime; Set B in one host-quiet window: D1, D2, M1, M2, M3, regressions | R1, F1b | S-lane | — |
| R2 | 06 S2 | MCP ranked mode as D1 decides, properties, `VIEW_NOTICE` clause, goldens 1/3/4, docs and skill | G-06 | S-lane, E-1a | ~350 |
| S-1a | 04 PR-1a | `dependence.rs` data walk (labelled backward, forward through the taint walk), parameter remap, `field_store` stop, ordering and budget, `Evidence` + `DependsOn` + `dependence`, `api::nav`, CLI `slice-at`, `mutants/slice-at.json` | P0; **merge after F1b** (born with the unified grammar) | R1, F1a, F1b (development) | ~650 |
| S-1b | 04 PR-1b | syntactic control step (nesting, early exits), `ControlFromSyntax`, per-language fixtures | S-1a | S-3 | ~400 |
| S-3 | 04 PR-3 | `reach: nameonly`, descent depth parameter, `trace_reach.rs`, Tier-A matrix control | S-1a (merge-serial with S-1b on `dependence.rs`) | S-1b, S-2 | ~400 |
| S-2 | 04 PR-2 | MCP `slice_at` behind `--experimental-tools slice_at` (standalone; a CLI error with `--shared`), `tools_slice.rs`, `slice_view.rs`, flag-on transcript, docs, CLAUDE.md, `js-ts-known-gaps.md` note | S-1b | S-3, E-lane | ~650 |
| **Stage B** | 04 §2.2 | one call per Stage-A seed (CLI) | S-1b, S-3 | — | — |
| S-4 | 04 PR-4 (private evidence store) | Stage C harness, arm built at the S-2/S-3 head with the flag | S-2, S-3, Stage B | E-lane | ~300 |
| **Stage C** | 04 §2.3 | 288 runs; decides S-P | S-4 | — | — |
| E-1a | 02 PR-1a | explore core: seed resolution (resolve_fn after F1b, `candidates` from `details`, `query` seeds through `find_symbols` if M1 passed), `definition`, ranked `callers`/`callees`, `blast_radius`, JSON, CLI; `mutants/explore.json` | F1b, R1, G-01 | R2, S-lane | ~800 |
| E-1b | 02 PR-1b | budget fitter, Markdown renderer, `module` section (unfocused `rank_files`), CLI markdown/text | E-1a | S-lane | ~550 |
| E-4 | 02 PR-4 | MCP `nav_explore` (own seed parser, shared `format` vocabulary), goldens 3/12 + ids 40–49, smoke 10, docs, skill, CLAUDE.md, adoption probes; within +2,750 chars | E-1b; merge after R2 | E-2, E-3a, E-5 | ~750 |
| E-2 | 02 PR-2 | call path, CLI `call-path`, `path_to`, `mutants/call-path.json`, gold-check script (§9.3) | E-1b | E-4, E-3a, E-5 | ~750 |
| E-3a | 02 PR-3a | `TypeIndex` (Go and Rust declarations), type seeds, Rust implementations and overriders, CLI `implementations` | E-1b | E-2, E-4, E-5 | ~650 |
| E-3b | 02 PR-3b | Go satisfaction, `implementations_oracle.py` and the Tier-A `implementation` request (§9.4) | E-3a | E-2, E-4, E-5 | ~650 |
| E-5 | 02 PR-5 | `dependence` section through `slice_at::run`; skill guards routine routed to it | E-1b, S-1b | E-2, E-3a/b, E-4 | ~150 |
| **evals** | 02 §9.1–9.2 | adoption probes and Haiku replicates; T1–T4 decide skill placement | E-4 | — | — |
| F2 | 01 S2 | contingent: `nav_find_symbol` MCP tool, only on an owner decision within the cap | G-01 and an owner decision | — | ~400 |
| S-P | 04 | contingent: `slice_at` joins the default list, golden 3/12, skill row, only on Stage C "ship" within the cap | Stage C and an owner decision | — | ~150 |

**Lanes at a glance.** 06: R1 → gate → R2. 01: N0 → F1a → F1b → gate (→ F2?). 04: P0 → S-1a → {S-1b, S-3} → S-2 →
Stage B → S-4 → Stage C (→ S-P?). 02: E-1a → E-1b → {E-4, E-2, E-3a → E-3b, E-5}. Golden merges run F1b → R2 →
E-4. The `view_reason` arms merge F1a → S-1a. Each spec's gates are unchanged: 06 D1/D2 before R2; 01 M2/M3 after
F1b and M1 for the `query` seed and F2; 04 Stage A before S-1a, Stage B before S-4, Stage C before S-P; 02's T1–T4
after E-4.

## 7. Consolidated owner questions (deduplicated)

| # | Question | From | Recommendation |
|---|---|---|---|
| OQ1 | **Tool budget.** Which option, and what cap? | new; 01 Q1, 04 Q2, 02 Q2 | Option D: `nav_explore` is the only new default tool, with a 17,500-char cap. Find-symbol reaches MCP through explore's `query` seed; `call-path` and `implementations` are CLI plus explore sections; `slice_at` is CLI plus opt-in MCP. F2 and S-P are contingent and must fit the cap. |
| OQ2 | Opt-in `--experimental-tools slice_at` for Stage C, or default registration now? | new; 04 Q2 | Opt-in. It costs nothing by default, keeps the default goldens unchanged, and Stage C decides promotion. |
| OQ3 | Make the ranked map the no-argument `nav_repo_map` default? | 06 Q1 | Yes, subject to D1 (path-order compact fallback if D1(b) fails). Re-bless ids 1, 3 and 4 with that reason. |
| OQ4 | Markdown as the default content for dedicated-result tools? | 06 §5.1; 02 Q1 | Yes for both, with the shared `format` vocabulary (X7) and one generic `VIEW_NOTICE` clause in R2. |
| OQ5 | Budget defaults, and a single unit? | 06 Q2; 02 Q5 | Keep 06's 2,000 tokens and 02's defaults (depth 3, `node_cap` 5,000). Also unify units: explore takes `budget_tokens` with 06's `estimate_tokens` (default 2,700 ≈ 02's 8,000 bytes at bytes/3; range 700–8,000), decided before E-1b. Not applied to 02 pending this answer. |
| OQ6 | Qualified names and handles in **every** seeded tool, including `taint_reaches` and `slice_at`? | 01 Q2, extended | Yes, through one selection routine (X4, X13). The F1b tier-2 trigger already names `tools_reasoning.rs`. |
| OQ7 | Reorder `AmbiguousSymbol` candidates by rank (changes error content)? | 01 Q6 | Yes. |
| OQ8 | Include tests in results by default? | 01 Q5 | Yes, ranked after production. That matches explore's row order. |
| OQ9 | Type declarations as seeds or results? | 01 Q3; 02 Q6 | v1: explore-local `TypeRef` for Go and Rust, answering `definition` and `implementations` only. Find-symbol lists an owner's methods when the query names the owner. No `SymbolRef::Type`. |
| OQ10 | Promote `onboard` to an MCP tool? | 06 Q3 | No: its counts go into the map header. |
| OQ11 | Git recency as a ranking signal? | 06 Q4 | No for v1. |
| OQ12 | Entry points in the ranking? | 06 Q5 | Display only; the 06 §9.4 ablation. |
| OQ13 | Replace the gap analysis's "≥ 2x at ≤ 4k" threshold, which is unreachable, with D1/D2? | 06 Q6 | Yes. |
| OQ14 | BM25 over doc comments? | 01 Q4 | No. The revisit trigger now counts explore `query` seeds (or F2 queries) that return zero matches. |
| OQ15 | Accept 04's conditional gate verdict (T0 fails, T1 passes)? | 04 Q1 | **Decided (owner 2026-10-31):** accept the conditional pass. P0, the frozen-script Stage A reproduction, then informs the decision: go, no-go, or another spec revision. Nothing past P0 starts before that decision. |
| OQ16 | Stage C design? | 04 Q3 | 8 data/guards questions × 3 conditions. |
| OQ17 | Syntactic control dependence in v1? | 04 Q4 | Yes; post-dominators only if Stage C points at syntactic false positives. |
| OQ18 | Make `MAX_TAINT_DESCENT_DEPTH` a parameter? | 04 Q5 | Yes, in S-3, with the Tier-A matrix control. |
| OQ19 | Route the J3 blockers to the A2 follow-ups? | 04 Q6 | Yes; note them in `js-ts-known-gaps.md` §6 in S-2. |
| OQ20 | `slice_at` default `max_results` of 80? | 04 Q7 | Yes. |
| OQ21 | Source text in explore? | 02 Q3 | Header line only; `snippets: "line"` opt-in; arm C decides. |
| OQ22 | Explore section defaults? | 02 Q4 | `definition`, `callers`, `callees`, `blast_radius`. `module` defaults on only if 06's D1 passes in full and its share stays under 10%. |
| OQ23 | Explore eval budget? | 02 Q7 | Haiku × 3 and Sol × 1; Sonnet only if T1 passes. |
| OQ24 | Accept the PR splits and the cross-spec order in section 6? | new; 02 Q8 | Yes. E-4 still precedes E-3a/b as 02 recommended, and they can now run in parallel. |
