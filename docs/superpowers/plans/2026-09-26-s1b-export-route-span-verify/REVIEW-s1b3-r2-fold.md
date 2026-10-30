# S1b-3 spec round 2 of 2 (the cap): disclosed at-cap fold

Reviews: `~/prism-evidence/s1b/reviews/s1b3-spec-r2-{opus,sol}.md`. Opus: FIX 1 WRONG / 2 SMELL, converging. sol: FIX
2 WRONG, **open-class** (both are new instances of round 1's W1 / W3 mechanisms, and the lexical auditor shared the
W1 blind spot). Round 2 is the cap, so there is no round 3. Per the convergence rule, sol's open-class verdict went
to the owner, who on 2026-09-29 chose a **conservative cut that closes the class by construction** (no more
precision) plus bounded fixes, and approved the caps. This record is that disclosed at-cap fold. Every number below
is from the clean-built v12 (scratch `0968ef78`, `BUILD-v12.txt`); C185–C189 were pre-registered (addendum 10,
`bfcd6ba3`) before v12 was built.

| # | Finding | Disposition | Where | Controls and mutants | Measured |
|---|---|---|---|---|---|
| sol W1 | NoFn ignored destructuring defaults: `const { missing: x = fallback } = {}` binds `fallback`, a right edge dropped | **Conservative cut (owner):** a destructuring pattern holding a default anywhere is `Alias` (keeps base), whatever its value; NoFn stays the minimal literal class | SPEC §0 (2), §3.8 (11) | C-48: C185–C188 both grammars; mutant C-M43 | C185–C188 keep base (v11 dropped); 0 corpus rows v11 → v12 |
| sol W2 | Comment trivia defeated the recovered-import check (`import /* c */ . meta`) | **Fixed:** the first two tokens skip comments and other non-`ERROR` extras (tree-sitter flags recovery `ERROR`s as extras; skipping them regressed C173/C181 in a first build, caught by the controls) | SPEC §3.8 (7) | C-49: C189 (block and line comment, `.meta` and `(`); mutants C-M44, C-M45 | C189 Exact ×3; C173/C181 still drop; 0 corpus rows |
| Opus W1 | Parameter removals lose right edges; the SPEC said 0 | **Owner: keep dropping parameters, on the stated ground** (a parameter's value is not statically bound), **with the measured cost recorded and accepted** | SPEC §0 (3), §3.8 (11), §8 | – | `param_valueflow.py` (lower bounds): X 6, F 2, T 5 right lost (single caller); +X 5, F 2, T 10 under the all-suppliers reading |
| Opus S1 | 3b's tests credible only with the C-5/C-6 guards in 3a | **Owner:** guards move to 3a as unit rows; 3b report point 1,190 in its dispatch | IMPLEMENTOR S1b-3a/3b dispatch | – | 3b tests forecast ≈ 1,260 of 1,320 |
| Opus S2 | `js_binding_site.rs` needs a split | **Planned:** `js_binding_writes.rs`, `js_binding_recovery.rs`, `js_binding_values.rs` split out | SPEC §3.8 (13) | – | v12: site 292, `js_binding.rs` 515 → about 425 after the move |
| sol (auditor) | `audit_s1b3.py` classed C900 as `removed_wrong` | **Fixed:** `removed_alias` covers default-bearing patterns; re-run on every corpus | `probes/audit_s1b3.py` | – | C185 (v11) now `removed_alias`; v12 `removed_alias` 0 on every corpus |

## Per-corpus result (clean v12 vs `prism-head-3961cc21`)

| Corpus | Rows | Re-targeted right | Wrong removed | Right added | Right lost: E6 | Right lost: parameters (owner-accepted, lower bound) |
|---|---|---|---|---|---|---|
| X | 198 | 134 | 58 | 0 | 0 | 6 (+5 all-suppliers) |
| F | 52 | 1 | 45 | 0 | 4 | 2 (+2) |
| R | 0 | – | – | – | – | – |
| T | 913 | 635 | 222 | 10 | 41 | 5 (+10) |

Parameter rows not classed lost are `passed_other` (X 22, F 9, T 43) or undetermined (X 29, F 33, T 155). **3a alone
changes 0 rows on all four corpora.**

## Caps (owner-approved 2026-09-29)

3a src 700 / tests 740 (measured 596 src; tests ≈ 710); 3b src 220 / tests 1,320, report point 1,190 (measured 161
src; tests ≈ 1,260). Cache 3a 102 / 58, 3b 103 / 59, S1b-4 104 / 60.

## Prior-round closure

Round 1: Opus W2, S1, S3 and sol W1, W2 (Unicode), S1 closed by both reviewers; Opus W1 closed by the owner's round-2
rulings (aliases, defaults, parameters on the stated ground); Opus W3 closed by the trivia fix; Opus S2 closed by the
auditor fix.
