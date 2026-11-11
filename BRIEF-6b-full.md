# Slice 6b brief: rules U and D for C `.c` owners (prism #630 slice 6)

You are the implementer. Create branch `perf/630-s6b-c-rules` from `origin/perf/630-s6a-c-macro-census` (head f26f4555; slice 6a, the C/C++ macro census, is in review as PR #810). If 6a gets more commits or merges into main while you work, merge them in; never rebase or force-push. Make plain pushes. Do not touch any other branch except `test-results/630-s6b`.

**Never wait for a human.** Nobody will answer questions in this session. Where something is unclear, take the conservative reading (keep the pair), record it in the report, and continue. If something blocks you, push what you have and write the report with `Progress: STOPPED` and the reason. Push code commits as soon as tests pass, before measuring.

## Design
The owner-approved design-of-record is in the branch at `docs/superpowers/plans/2026-11-11-630-s6-cpp-ud/DESIGN.md`. Implement section 5.1 exactly, for **6b: C `function_definition` owners in `.c` files only**. `.h`, C++ and field paths stay off.
- Read 5.1 (gates G1–G6, eligibility E1–E3 and every further condition there, the D rule and its certificate, the U rule), 5.2 (soundness), 5.3 (disclosures) and 5.4 (first obligation rows).
- Also read section 1.2, the corrections F1–F7, and section 2, the pooled hazards. Every one of them that applies to C must be pinned by a row.

The census (`src/call_graph/c/macro_census.rs`) is already threaded to `LexicalFacts::new`. Consume it:
- **`incomplete`** disables U and D for every C/C++ owner;
- **poisoned or control macros** act as the design says;
- **a name passed to, or named by, an in-repo macro** (the transitive free identifiers) gets no U and no D;
- **a spelling that is itself an in-repo macro name** is ineligible.

## Structure
- Add a new `src/data_flow/build/c_prune.rs`, with explicit `Language::C` dispatch in `lexical_prune.rs`. Do not broaden any shared predicate.
- Mind the traps the design names, especially the declaration-initializer span trap: `int y = x;` must not record `x` as a write. Bind the classifier to the `declarator` field.
- Put fixtures under `tests/fixtures/dfg_prune/c/` with `obligations.jsonl`, and add `c_prune_tests.rs` copying `rust_prune_tests.rs`: owner-keyed keep and drop checks, a missing-baseline check, and a removal-only test over the fixture tree.
- **Inputs that set repo-wide census facts go in inline test strings, never in files under `tests/fixtures`** (lesson from #806: the repository loader indexes that tree). That covers `incomplete`, a poisoned or control macro that would refuse names other fixtures need, and an in-repo macro named like a common identifier.
- Update the CLAUDE.md slice-6 paragraph: the 6b rules, the disclosure (macros defined outside the repository act as functions), and the survivor-cap sentence (#634).
- Cache: bump to v175 everywhere (6a took v174).

## Tests
- RED-first: every keep and drop row is pinned, and every keep row fails without its guard (spot-check by disabling guards).
- Run `cargo test --lib`, `cargo test --test ast`, `cargo test --test lang_c` and `cargo fmt --all --check`.
- Report failures outside your change, with a same-environment run of the base.

## Measurement (after the code is pushed)
- **Corpora:**
  - redis `git clone --depth 1 --branch 7.2.4 https://github.com/redis/redis` (7.2.x is BSD-licensed; parse only). Use `src/` only, and record the commit.
  - cpython (`git clone --depth 1 --branch v3.12.7 https://github.com/python/cpython`, parse only): sample its C for the external-macro boundary.
  - Controls: one non-C tree, such as the repository's own non-C fixtures; its output must be byte-identical.
- **Builds:** release builds of base f26f4555 and your head; `nav --no-cache dfg-stats --edges` and `call-stats`; RAYON 4.
- **Against the base:** removed only, 0 added, 0 kept-label transitions. `call-stats` outside `dfg_labels` may move only through U-suppressed anchor Uses (as for Rust); list every such ReturnInput difference.
- **Ledger:** removed edges by rule (U/D), and the refusal causes on the redis owners. Give owner counts for: off by each gate G1–G6; `incomplete`; escape; macro-name refusal; scalar.
- **Samples:** 30 random removed edges on redis (seed 806) and 20 on cpython, each with its def and use source text and a verdict. Push them as `samples.jsonl`. Read each one in source.
- **Timing:** redis, 2+2 interleaved.

## Output
- Write `REPORT.md` on `test-results/630-s6b`.
- It contains: the items with commits, the RED rows, the tests, the measurement tables, the ledger, the samples, the timing, and any reading taken conservatively.
- Its last non-blank line must be `Progress: DONE` or `Progress: STOPPED`.

## Rules
- Run tests with a memory cap where available.
- Static analysis only: never build or execute corpus code. prism only parses it.
- Do not open PRs or comment on GitHub.
- Never read or print dotfiles or credentials.
- Do not open `eval/secbench`, and exclude it from every recursive search.
