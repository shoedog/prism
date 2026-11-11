# Fold 1b brief: PR #811 (#630 slice 6b, rules U and D for C `.c` owners), before review round 2

You are the implementer, on branch `perf/630-s6b-c-rules` (PR #811, head 29b09503; it now targets `main`). Make plain pushes (no force-push, no rebase).

**Never wait for a human.** Nobody will answer questions in this session. Where something is unclear, take the conservative reading, record it in the report, and continue. If something blocks you, push what you have and write the report with `Progress: STOPPED` and the reason.

## Step 0: merge main
Slice 6a (the C/C++ macro census) merged into main as ef7274e8, a squash. Your branch carries 6a's unsquashed history only up to e06c71f5. Main additionally has:
- the census identity only in `LoadedRepo.manifest_hashes`, not `scope_graph_inputs` (a prism-mcp freshness fix);
- the exotic-lexing belt narrowed to code positions, outside string and char literals and comments;
- failed text includes in the cache identity.

`git merge origin/main`, then resolve conflicts by taking main's version of 6a's files (`src/call_graph/c/macro_census*.rs`, `src/repo_loader/c_census.rs`, `src/repo_loader.rs`, `src/api/review.rs`, `src/ast.rs`) while keeping your 6b additions in them, such as `spelled_identifiers` in `macro_census.rs`. Then:
- keep both CLAUDE.md paragraphs, main's census note and your slice-6b paragraph, with "slice 6b consumes it";
- set the cache to v175 everywhere (main is v174);
- after the merge, `cargo test --lib -- c_prune macro_census f3_ cpg_cache lexical_prune` and `cargo test -p prism-mcp freshness_` must pass.

## Item: freshness for census-only text files (prerequisite before 6b consumes the census)
prism-mcp's freshness probe (`crates/prism-mcp/src/freshness*`) does not watch the census-only text include files (`.inl .inc .ipp .tcc .def .cu .cuh .H .C .HPP .CPP` and quoted-include targets). So an in-session edit to one does not mark the index stale, and 6b would keep pruning under a stale census. Make the freshness probe treat those files, and the census identity, as tracked inputs.
- Do it without reintroducing main's bug, where census entries in `scope_graph_inputs` made every load look stale. Compare like with like: the census identity recomputed the same way on both sides.
- Tests:
  - editing an `.inl` marks the session stale;
  - an unchanged repository with `.inc`/`.inl` files and an unreadable directory is not stale on load;
  - removing a text include marks it stale.

## Measurement (after the code is pushed)
- Corpora: unmodified redis 7.2.4 `src/` (d2c8a4b9; no ASCII copy needed now), cpython v3.12.7 as before, and the non-C controls.
- Builds: release builds of main ef7274e8 (the base) and your head.
- Against the base: removed only, 0 added, 0 label changes. Report the census state on each corpus (`incomplete` or not; macro counts).
- Against fold 1 on redis-ascii: report the removed count on unmodified redis, which should now be close to the earlier redis-ascii 34,713.
- Ledger: owners by verdict, as before.
- Samples: 20 random removed edges on unmodified redis (seed 8111), read in source. Push `samples-fold1b.jsonl`.
- Timing: redis, 3+3 interleaved.

## Output
- Write `REPORT-fold1b.md` on `test-results/630-s6b`.
- It contains: the merge resolution, the freshness item with its tests, the measurement, the ledger, the samples and the timing.
- Its last non-blank line must be `Progress: DONE` or `Progress: STOPPED`.

## Rules
- Run tests with a memory cap where available.
- Static analysis only: never build or execute corpus code.
- Do not open PRs, comment on GitHub, or touch branches other than `perf/630-s6b-c-rules` and `test-results/630-s6b`.
- Never read or print dotfiles or credentials.
- Do not open `eval/secbench`, and exclude it from every recursive search.
