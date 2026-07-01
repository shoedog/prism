**MERGED REVIEW — build-identity in `--version` (Tier-A stale-SUT detection)**

---

**BLOCKER — `build.rs:` identity goes stale *within a commit*; the feature misses the exact case it exists to catch.**
*Location:* `build.rs` (rerun triggers + dirty detection).
*Issue:* The build script only reruns on `Cargo.lock`, `.git/HEAD`, and the pointed ref. In a clean build, then edit `src/main.rs` and rebuild: Cargo recompiles the binary but does **not** rerun `build.rs`, so `GIT_SHA` keeps the old SHA and `dirty=false`. Two byte-different binaries therefore print the *same* `--version`, and stale-SUT detection reports "fresh" for a stale binary — defeating the feature's stated purpose.
*Disagreement resolved:* Codex rated this BLOCKER, Claude rated the dirty-flag a MINOR smell. **Codex is right** — this isn't cosmetic; it silently produces a false "not-stale" verdict, the precise failure the feature prevents. (The comment's "harness HEAD-check backstops" only compares HEAD, so it does not catch same-commit source drift.)
*Fix:* Move identity/dirty detection to **runtime** (let the harness read working-tree state and HEAD directly), or add content-based rerun triggers so the embedded value can't outlive the source it describes. Don't keep dirty-detection split half-in-build-script, half-in-harness.

---

**MAJOR — `build.rs:` invalidation is coupled to git's on-disk layout; breaks under worktrees and packed refs, risking a rebuild livelock.**
*Location:* `build.rs` (`read_to_string(".git/HEAD")` + `rerun-if-changed=.git/{ref}`).
*Issue:* Assumes `.git` is a directory with a *loose* current ref. (a) **Worktrees/submodules:** `.git` is a *file* (`gitdir: …`), so the read fails, no ref is watched, and `rerun-if-changed=.git/HEAD` points at a nonexistent path. (b) **Packed refs:** after `git gc`/`pack-refs` the loose ref vanishes and commits update `.git/packed-refs` (not watched). Claude confirmed the live repo already has *both* a loose `tier-a` ref and a 33-entry `packed-refs` — works today, one `git gc` or worktree build from breaking. Worse than a stale string: the harness sees an old SHA, asks for a rebuild, but `cargo build` won't rerun `build.rs` (no watched input changed) → **permanently-stale identity / rebuild livelock.**
*Fix:* Don't depend on git's internal file layout — resolve the gitdir via `git rev-parse --git-dir` and also watch `.git/packed-refs`; or (cleaner) make the harness HEAD-check the single source of truth and soften the comment's absolute "can't go stale" claim; or adopt a maintained probe (vergen-style).

---

**MAJOR — no-git fallback `"nogit".repeat(2)` violates the format contract and fails the new test's own regex on legitimate gitless builds.**
*Location:* `build.rs` (`unwrap_or_else(|| "nogit".repeat(2))`) vs `tests/cli/version_test.rs` regex `[0-9a-f]{12}`.
*Issue:* The fallback yields `"nogitnogit"` (10 chars, non-hex), so `--version` becomes `slicing X.Y.Z (nogitnogit)` — which the test's `^…([0-9a-f]{12}(-dirty)?)\n$` rejects, failing `cargo test`. Reachable on real builds with no `.git`: `cargo package`/publish from an extracted tarball, source archives, Docker `COPY` excluding `.git`, vendored/distro builds, or git-not-in-PATH sandboxes.
*Fix:* Make the fallback conform to the grammar (12 hex zeros or an agreed sentinel) **and** decide explicitly whether a gitless build should carry a well-formed "unknown" marker or hard-fail. Test and build script must agree.

---

**MAJOR — the identity format is an undocumented contract with no single owner, duplicated across four sites.**
*Location:* `build.rs` (produces `{sha}[-dirty]`), `src/main.rs` (wraps as `X.Y.Z (…)`), `tests/cli/version_test.rs` (regex proxy), and the *real* consumer — the harness's `--version` parser (not in this diff).
*Issue:* The in-repo test pins the format; the harness pins it separately; nothing forces them to agree. A future tweak (e.g. `--short=12` → full 40, or changing the dirty marker) can pass this test yet silently break the harness, or vice versa.
*Fix:* Define the grammar once — a shared format/parse helper or a documented spec that both the test and the harness cite — so the contract has one owner.

---

**MINOR — exact-width `{12}` SHA assumption is brittle.**
*Location:* `tests/cli/version_test.rs` regex; `build.rs` `--short=12`.
*Issue:* `git rev-parse --short=12` auto-extends past 12 chars when the prefix is ambiguous in a large/old repo; the regex hard-codes exactly 12 hex, so a 13+ char abbreviation would fail it.
*Fix:* Use the full 40-char SHA (fixed width, unambiguous) or relax the regex to `[0-9a-f]{12,40}`.

---

*Note:* Both lenses ran successfully; no lens was missing. Codex and Claude independently flagged the git-layout MAJOR and the gitless-fallback MAJOR (merged above); the format-contract ownership and `{12}`-width findings are Claude-unique; the within-commit staleness BLOCKER is Codex-driven.

**Verdict:** Do not merge — fix the 1 BLOCKER (runtime/refreshed identity) plus the worktree/packed-ref and gitless-fallback MAJORs before shipping; the contract-ownership MAJOR and the two MINORs can follow but should land soon after.
---

**RESOLUTION (2026-06-12):** all findings in this record were fixed in the build-identity
commit that shipped (src/ rerun trigger, gitdir resolution, packed-refs watch, grammar-conformant
`unknown` fallback, single-owner grammar comment, {12,40} width); the "Do not merge" verdict
applied to the pre-fix revision only. See `tier-a-followups.md` item 1 for the residual
linked-worktree edge.
