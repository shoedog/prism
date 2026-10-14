# Reviews in this folder

| PR | Round 1 | Round 2 | Outcome |
|---|---|---|---|
| #365 matrix test | sol FIX, Opus FIX | sol APPROVE, Opus FIX (closed items, no further review) | merged `8c681d77` |
| #368 import suffix | sol FIX, Opus FIX | Opus APPROVE, sol FIX (wording and test pins, no further review) | merged `38a59994` |
| #372 alias following | sol FIX, Opus FIX | sol FIX (5 material), Opus FIX (1 material) | **at the two-round cap, not approved** |

## #372 at the cap (controller, 2026-10-13)

Round 2 found more material items than round 1, in the same three families: an invalid destructuring pattern admitted; a not-proven site whose records differ from main's (two mechanisms: the callable terminal returning its own drop; the textual path standing down by binding kind); a consumer keyed by callee spelling (the `--format callers` scan added in the fold lost target identity; taint descent cannot cross a followed alias). By the convergence rule that is not a converging loop, so no third round is dispatched without the owner.

The fold in progress closes each family by construction instead of by input: one predicate (the alias route produced a proven hit) is the only thing the resolver's early return, the textual stand-down and the callers counts may depend on; a structural test over every matrix fixture asserts that a site where the predicate is false resolves exactly as it would with main's binding; pattern admission requires unique bound names; the formatter keeps the target's identity. The taint descent gate is not changed (it fails closed and main does not reach either); the two reviewers grade it differently (sol: WRONG, MATERIAL; Opus: SMELL, IMMATERIAL), which is for the owner.
