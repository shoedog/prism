## Accuracy Harness (Tier-A)

When a change touches call resolution, navigation queries, or CPG construction
(`src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`):

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut   # seconds, no LSP — run before committing
cd eval && uv run tier-a --quick --allow-stale-sut         # minutes, needs rust-analyzer — before review
```

Use `--allow-stale-sut` only with an immediate preceding rebuild in the same
worktree; it is for the normal dirty pre-commit state, not stale binaries.
Paste regressions/flip-candidates into the PR description rather than re-baselining.
Full multi-corpus runs (`uv run tier-a --corpus all`) are human-triggered; see
`eval/README.md`. The committed baseline lives in `docs/eval/tier-a/`.

## File size

Size is measured in weighted lines: code 1, comment-only 0.5, blank 0, via
`python3 scripts/file_size.py FILE...` (`--over N` lists only files above N and exits 1).

- New files aim for 600 or fewer.
- Over 1,000, split the file along a real responsibility boundary; a PR that leaves a
  file over 1,000 says why.
- **2,000 is a hard ceiling: no PR may take a file past it.**
- Files already over 2,000 (e.g. `src/ast.rs`, `src/call_graph.rs`,
  `src/algorithms/taint.rs`) must not grow in a feature PR: small edits are fine, new
  code goes in new modules. Split such a file in its own move-only PR (no behaviour
  change, reviewed as a pure move) before the work that needs it, never inside a
  feature PR.
