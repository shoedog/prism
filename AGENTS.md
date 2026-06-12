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
