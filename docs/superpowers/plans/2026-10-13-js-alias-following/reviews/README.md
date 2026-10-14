# Reviews in this folder

| PR | Round 1 | Round 2 | Outcome |
|---|---|---|---|
| #365 matrix test | sol FIX, Opus FIX | sol APPROVE, Opus FIX (closed items, no further review) | merged `8c681d77` |
| #368 import suffix | sol FIX, Opus FIX | Opus APPROVE, sol FIX (wording and test pins, no further review) | merged `38a59994` |
| #372 alias following | sol FIX, Opus FIX | sol FIX (5 material), Opus FIX (1 material); round 3 (owner-approved, last): sol FIX (4 WRONG, material), Opus APPROVE (3 WRONG, immaterial) | **round-3 fold in progress; merge gate with the owner** |

## #372 at the cap (controller, 2026-10-13)

Round 2 found more material items than round 1, in the same three families: an invalid destructuring pattern admitted; a not-proven site whose records differ from main's (two mechanisms: the callable terminal returning its own drop; the textual path standing down by binding kind); a consumer keyed by callee spelling (the `--format callers` scan added in the fold lost target identity; taint descent cannot cross a followed alias). By the convergence rule that is not a converging loop, so no third round is dispatched without the owner.

The fold in progress closes each family by construction instead of by input: one predicate (the alias route produced a proven hit) is the only thing the resolver's early return, the textual stand-down and the callers counts may depend on; a structural test over every matrix fixture asserts that a site where the predicate is false resolves exactly as it would with main's binding; pattern admission requires unique bound names; the formatter keeps the target's identity. The taint descent gate is not changed (it fails closed and main does not reach either); the two reviewers grade it differently (sol: WRONG, MATERIAL; Opus: SMELL, IMMATERIAL), which is for the owner.

## #372 round 3 (controller, 2026-10-13)

Both reviewers found the same three mechanisms; sol grades them MATERIAL (FIX), Opus IMMATERIAL (APPROVE). Controller reproduced each on the frozen release binaries (`r3-controller-repro/` in the evidence root).

| Mechanism | sol | Opus | Fold |
|---|---|---|---|
| The predicate is true at a site the resolver answers before the alias arm (lowercase JSX tag; an installed executable owner, by reading) | W2 | W1 | the predicate becomes the resolver's own answer; the rule test asserts the predicate-true half |
| The stand-down withholds a Level-1 record that Level 4's same-line dedupe reads, adding a record at a neighbouring site | W4 | W2 | the legacy computation keeps main's population; the stand-down only filters finished output |
| `--format callers` takes a followed site from its spelling bucket at depth 2 | W3 | (depth > 1 not checked) | a predicate-true site is admitted only through its proven identity |
| Early-error declarations (`x: eval`, reserved words, a name declared twice across declarators or in a catch body) are still followed | W1 | W3 | not folded: both reviewers point to a contract boundary (main binds the same shapes Exact when the value is directly a callable); owner decision |

The brief's structural-test tallies were stale (taken from the 1,335-fixture run); on `568c284f` the test prints 1,825 Source sites, 348 alias-bound, 303 true, 45 false compared, 15 held, 6 suppressed. Both reviewers flagged it.
