# Handoff — `nav_symbol_spans` v1

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing` · `nav-symbol-spans-v1`
**Exact base:** PR #233 merge `c7cc2d9568f07f215f5da3335e4d10e1a4984f3b`

## 0. Current verdict

**IMPLEMENTED, DOCUMENTED, REVIEWED, AND FULL-SUITE GREEN; TIER-A PENDING.** The owner directed work past eval-only increments. Read-only callable edit coordinates now work through the navigation query, CLI, and MCP. Release/Tier-A gates, PR, and merge remain.

## 1. Authority boundary

Implement callable-only coordinate discovery through CLI and MCP. Prism returns exact outer/name/body spans, insertion anchors, and bounded indentation context; it never writes source. Classes/fields/rename/refactor/edit application and Java/JDT-LS work remain out of scope.

## 2. Evidence and plan

- `prism-nav` oriented the repository and traced the MCP registry impact before file reads. Exact seams are navigation query/result types, one AST helper, CLI parsing/dispatch, MCP input/schema/handler/registry, transport/smoke tool counts, and user/skill documentation.
- CPG `Function` nodes and `FunctionInfo` already retain exact outer UTF-8 byte spans. `ParsedFile` retains source and tree-sitter identity, so name/body spans do not require persistence or a cache bump.
- Tier-A's Java path is not immediately executable: no Java/JDT-LS registry entry or corpus exists and `jdtls` is absent. This product slice does not decide the Java native-vs-delegated fork.
- Design: `docs/superpowers/specs/2026-09-04-nav-symbol-spans-v1-design.md`.
- Plan: `docs/superpowers/plans/2026-09-04-nav-symbol-spans-v1.md`.
- Review cap: two self-review rounds; recurring/open-class coordinate or grammar findings park the slice.

## 3. Verification state

- RED contract commit: `a3bd9ae` (`test(nav): define symbol spans v1 contract`). Navigation failed at five missing `queries::symbol_spans` calls; CLI ran one test and rejected unknown `symbol-spans`; MCP failed at the missing parser; process smoke observed 8 tools versus the required 9.
- GREEN: navigation span tests 7/7; CLI span tests 2/2; focused MCP input/tool/freshness tests 4/4; registry tests 2/2; MCP lifecycle 1/1; MCP process smoke 1/1.
- The tests cover Python decorators, Unicode byte offsets, CRLF, tabs, nested Rust, same-line and empty bodies, 256-byte indentation refusal, bound-function name recovery, non-whitespace prefix refusal, body-less Java, ambiguity, source non-echo, strict MCP arguments, escaping-location fail-closed resolution, cap failure, structured-content modes, freshness, CLI JSON/text/seed validation, and additive tool counts.
- README, MCP reference, and bundled navigation skill now document the seventh nav tool, its dedicated result, coordinate semantics, and callable-only/read-only boundary; stale count-language search is clean. No project link-check script exists in this checkout.
- Review round 1 fixed one WRONG (active `CLAUDE.md` still claimed 8/6 tools) and one coverage SMELL (reachable empty/oversized/bound-function/CLI edges lacked direct assertions). An inline anonymous function proved not CPG-addressable, so no synthetic public null-name claim was added.
- Review round 2 found 0 WRONG and 0 SMELL. The folded `symbol_spans` filter passed 13/13 behavior tests across MCP, CLI, and navigation.
- Format, base-to-HEAD diff check, all-target MCP check, and configured all-target MCP Clippy are green. Clippy retains the repository's existing non-fatal warning population.
- Full default suite is green: 3,694 total = 3,693 passed + 1 ignored, 0 failed, including 2 doctests.
- Full MCP attempt 1 found exactly three stale six-tool assertions in transport recovery tests. The exact-base same-environment control passed those 3/3; enumeration found no fourth count; the branch's targeted 6-to-7 fix passes 3/3. The full MCP rerun is green: 3,884 total = 3,883 passed + 1 ignored, 0 failed.
- Release builds and Tier-A matrix/quick are not yet run.
- LSP semantic navigation remains unavailable; the repository's structural Prism navigation plus direct source reads supplied the blast-radius evidence.

## 4. Custody

- Root `main` was rebound to PR #233 merge `c7cc2d9` before this branch was created.
- Design checkpoint is `00b5866`; RED tests are `a3bd9ae`; GREEN implementation is `bea4c76`; documentation is `832655e`; review-round-1 fixes are `5c7590a`; review completion is `e613846`; the full-suite count fix is `c81a51d`; this MCP-green custody refresh is the current commit candidate.
- Root's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- PR #232 merged the JS/TS typed/new receiver recovery as `434764a6`; PR #233 reconciled its durable custody as `c7cc2d9`.
