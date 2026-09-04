# Handoff — `nav_symbol_spans` v1

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing` · `nav-symbol-spans-v1`
**Exact base:** PR #233 merge `c7cc2d9568f07f215f5da3335e4d10e1a4984f3b`

## 0. Current verdict

**DESIGN AND PLAN READY; PRODUCTION UNTOUCHED.** The owner directed work past eval-only increments. The first bounded product/navigation increment is read-only callable edit coordinates; implementation proceeds RED-first after this checkpoint.

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

- No implementation or RED tests yet.
- Required end gates include full default and `mcp` suites plus the call-resolution/navigation Tier-A sequence.
- LSP semantic navigation remains unavailable; the repository's structural Prism navigation plus direct source reads supplied the blast-radius evidence.

## 4. Custody

- Root `main` was rebound to PR #233 merge `c7cc2d9` before this branch was created.
- Root's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- PR #232 merged the JS/TS typed/new receiver recovery as `434764a6`; PR #233 reconciled its durable custody as `c7cc2d9`.
