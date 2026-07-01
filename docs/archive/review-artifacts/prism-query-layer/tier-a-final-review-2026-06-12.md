Both lenses (Codex correctness, Claude architecture) returned full reviews — no node failed, so nothing is synthesized from a single lens. The two agree on the two highest-impact issues (greedy site matching, unenforced `pinned_sha`); the rest divide cleanly into Codex-correctness-unique and Claude-architecture-unique findings. Neither reviewer found a build-breaker, so there are no BLOCKERs — but three unguarded seams let the baseline report "valid" while being quietly wrong, and those lead the MAJOR list.

---

## MAJOR

**1. `eval/tier_a/compare.py` — `site_compare` greedy many-to-one matching inflates precision (both reviewers).**
The `next(...)` that pairs each prism site to an oracle site never removes an already-matched oracle site from the candidate pool, so two distinct prism call sites that both fall within one oracle edge's `[start-CHAIN_TOLERANCE, end]` window each score TP against the *same* oracle edge. A genuine prism false-positive sitting within `CHAIN_TOLERANCE=2` of a real call is silently counted as a true positive — precision is inflated exactly where prism is wrong, and that is the headline number in `baseline.md`.
*Fix:* make matching one-to-one (exclude matched oracle sites from the pool / do optimal assignment); add a duplicate-near-line regression test; justify the `2`-line tolerance empirically.

**2. `eval/tier_a/cli.py` `run_corpus` + `eval/corpora.toml` — `pinned_sha` is a declared invariant with no enforcement (both reviewers).**
`corpora.toml` pins a SHA per corpus, but `run_corpus` computes `corpus_sha(cfg["path"])` from whatever is checked out and never compares it to `pinned_sha` (the field is read by no Python code). A bench-repo at the wrong commit runs to "valid", mints a fresh snapshot keyed by the actual SHA, and scores against line-keyed adjudications that no longer line up — a plausible-but-wrong baseline with no signal.
*Fix:* hard-fail (or set `baseline_invalid`) unless `corpus_sha == pinned_sha`, gated behind an explicit `--allow-drift`; or delete the field so it doesn't read as a guarantee.

**3. `eval/tier_a/cli.py` — broad `except Exception` conflates harness bugs with subject-under-test flakiness (Claude).**
The M2 loop maps any exception to `oracle_error`/`sut_error`, `run_m3_spotcheck` maps any exception to verdict `"ambiguous"`, and `resolve_capability`/oracle `version` swallow all. A `KeyError` in `map_incoming` or an adapter typo raises `oracle_error_rate` past the floor and sets `baseline_invalid=True` — an instrument bug is reported as oracle unavailability, sending the operator after the wrong thing.
*Fix:* catch the known external failures (`LspError`/`OracleError`, `SutError`, timeouts) at these seams and let unexpected exceptions propagate, or record a distinct `harness_error` outcome that fails loudly.

**4. `eval/tier_a/corpus.py` — untracked source files enter the evaluation universe while the corpus is declared clean (Codex).**
`universe()` walks all `*.rs/*.go/*.py`, but `corpus_dirty()` uses `git status --porcelain -uno`, so an untracked `src/new.rs` is evaluated under the unchanged clean HEAD SHA — the snapshot and adjudications won't match what was scored.
*Fix:* include untracked files in the dirty check, or build the universe from tracked files only.

**5. `src/navigation/inventory.rs:44-51` — dedup drops valid nested same-name functions (Codex).**
For `def f():\n    def f(): ...` the outer `f` contains the inner and `same_name` is true, so the outer record is marked not-kept. The dedup should only remove known wrapper records (`decorated_definition`), not arbitrary same-name containment.
*Fix:* drop only the wrapper-kind / decorated case; keep distinct nested same-name definitions. Add a nested-same-name fixture alongside the existing decorated-function test.

**6. `eval/tier_a/adjudication.py` — verdict store keyed by `file:line` against a moving corpus, with detection but no re-anchoring (Claude).**
`Adjudication.site`/`seed_def` are absolute line numbers; `apply_verdicts` counts stale records but only flags them. The most expensive artifact in the system — human-curated verdicts — is invalidated wholesale on every corpus SHA bump, since any insertion above a site shifts its line.
*Fix:* anchor verdicts to something diff-survivable (symbol + call-text hash, or content fingerprint) so the store migrates across SHAs instead of going stale en masse.

**7. `Cargo.toml` umbrella `[[test]]` targets + `tests/*/main.rs` vs `tests/integration/coverage_test.rs` — umbrella consolidation introduces a silent-drop failure mode that now diverges from the coverage matrix (Claude).**
A test file runs only if it has a `mod <stem>;` line in its directory's `main.rs`; `coverage_test.rs` independently reads its hardcoded `all_test_files` list off disk and scans for `fn test_*`. A file present on disk and in `all_test_files` but missing from `main.rs` is counted as covered yet never compiled or executed — the matrix reads green while the tests don't run, the inverse of the old per-`[[test]]` scheme.
*Fix:* glob/`automod`-include in `main.rs` so disk presence ⇒ execution, or add a test asserting every `all_test_files` entry appears in the corresponding `main.rs`.

**8. `eval/tier_a/strata.py` `filter_to_universe` — unguarded path-normalization invariant between two independently-built path sets (Claude, latent).**
Membership is `r.location.file in universe_files`, where `universe_files` comes from Python `os.walk`+`relpath` and `r.location.file` comes from prism's `nav functions` JSON. The day prism emits `./src/a.rs`, a symlink-resolved path, or a different case/separator, every prism record silently falls out of the universe — inflating `prism_missing` with no error.
*Fix:* normalize both sides through one canonicalizer at the seam and warn/assert when the kept intersection is empty.

---

## MINOR

**9. `build.rs:62-73` — ref invalidation not worktree-safe (Codex).**
In a linked worktree `git rev-parse --git-dir` points at `.git/worktrees/<name>`, but branch refs and `packed-refs` live in the common git dir, so commits/branch moves that don't touch `src/` can leave `GIT_SHA` stale and rebuilds may not refresh it.
*Fix:* also watch `git rev-parse --git-common-dir` refs (and `packed-refs`). (Note: the standalone `tier-a-task8-review-2026-06-11.md` BLOCKERs about this file are already incorporated in the in-diff `build.rs`; this worktree edge is the residue.)

**10. `eval/tier_a/cli.py` `run_corpus` — ~150-line composition root under one coarse `try/finally` (Claude).**
SUT construction, oracle lifecycle, M1, snapshot, sampling, M2, pinned, matrix, M3, floors, and metrics are wired in one function/one `try`; any pre-M2 throw collapses the whole corpus to `invalid_corpus_run` regardless of which stage failed (this is also the mechanism behind finding #3's mislabeling).
*Fix:* extract per-measurement steps behind a small stage interface so error scope is per-stage.

**11. `eval/tier_a/interfaces.py` — `Oracle`/`SystemUnderTest` Protocol seam declared but not load-bearing (Claude).**
Nothing imports it; `run_corpus`→`make_oracle` and `PrismCli(...)` are constructed concretely, so the "swap multilspy/SCIP/Rust behind these protocols" claim is aspirational and no adapter is type-checked against the Protocol.
*Fix:* inject the oracle/SUT factories into `run_corpus`, or at least `isinstance`-assert against the `runtime_checkable` Protocols at construction.

**12. `eval/tier_a/corpus.py` `snapshot_path` — snapshot key `<corpus>-<sha>.json` omits oracle version (Claude).**
Sampling is pinned to the snapshot but `oracle.callers()` truth is recomputed live, so a rust-analyzer/gopls upgrade silently shifts measured truth against an unchanged sample.
*Fix:* include the oracle version in the snapshot key, or invalidate when it changes.

**13. `eval/tier_a/cli.py` `main` — `--quick` silently overrides `--corpus` (Claude).**
`names = ["prism"] if args.quick else (...)` ignores `--corpus`, so `--quick --corpus tokio` runs prism.
*Fix:* let `--quick` modulate sample size only and respect `--corpus`, or reject the combination.

**14. `Cargo.toml` — `[profile.dev] debug = "line-tables-only"` smuggled into the test-consolidation diff (Claude).**
Sitting at the tail of the `[[test]]` churn, it degrades debug-info/backtrace fidelity for every dev build, unrelated to the consolidation.
*Fix:* split into its own commit with a one-line rationale.

**15. `docs/prism-query-layer/tier-a-task8-review-2026-06-11.md` — committed doc ends "Do not merge" though `build.rs` already incorporates its fixes (Claude).**
A future reader greps this file, sees a blocking verdict, and distrusts a shipped-and-fixed feature.
*Fix:* append a resolution footer mapping each finding to its fix, or drop the standalone verdict.

---

**Verdict:** No build-breaker — ship after fixing the 8 MAJORs, and prioritize the three credibility seams first (greedy site matching #1, unenforced `pinned_sha` #2, broad `except Exception` #3), since each lets the baseline read "valid" while being quietly wrong; address the umbrella silent-drop (#7) before the next test file is added.