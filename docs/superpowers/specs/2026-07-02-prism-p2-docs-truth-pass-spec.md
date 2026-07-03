> **Status: SHIPPED — PR #150 (merged 2026-07-02).** As-executed brief incl. codex corrections (12 grammar variants / 11 language families; `refresh_index` not read-only). Post-spec fixes from reviews: docs/MCP.md stale name-based gotcha, grammar-crate count, `needs_cpg()` 16/14 Simple-vs-Graph split (CLAUDE.md/ALGORITHMS.md/README), score-decay wording, `taint_reaches` seed column.

# Task P2 — Docs/skills truth pass (trims + fact fixes, no new prose)

You work in the git worktree `/private/tmp/prism-p2-docs-truth` on branch `p2-docs-truth-pass` (based on main @ 523db1b). The repo is prism. This is a docs-only task — no Rust changes. Every edit fixes a VERIFIED falsehood; verify each against ground truth by reading the cited code before editing (line numbers in docs may have drifted; the facts were verified).

Ground truth (re-confirmed by a codex pre-implementation review):
- **30 algorithms**: `SlicingAlgorithm` enum in src/slice.rs:47-119 (count the variants yourself to confirm; the 4 newest are ContractSlice, PeerConsistencySlice, CallbackDispatcherSlice, PrimitiveSlice).
- **Languages: 12 parsed variants** (`Language` enum src/languages/mod.rs:7-20 includes `Tsx`; `Language::all()` returns 12 at :66-81) = 11 language families with TSX as a separate grammar variant of TypeScript. README already says 12 at README.md:52. When a doc says "11 languages", either leave it if it clearly lists families, or phrase as "11 languages (12 tree-sitter grammar variants — TSX is parsed separately)". Never write "5".
- **8 MCP tools**: 6 read-only nav tools returning Evidence + read-only `taint_reaches` returning Evidence + **`refresh_index`, which is NOT read-only** (local state change — src/mcp/tools_refresh.rs:21-22; tests/mcp/smoke_test.rs:47-59 assert non-read-only). Any doc sentence claiming ALL tools are read-only or Evidence-shaped must be fixed wherever you touch it (docs/MCP.md:121 has "all read-only"; CLAUDE.md:208-216 has "six read-only navigation tools").
- **Nav result caps**: 50 items default / 1000 max (src/mcp/input.rs:9-10), 80,000-byte wire cap (src/mcp/output.rs:11). There is no "~200 nodes" cap anywhere.
- **Confidence scores**: Exact → 1.0, NameOnly → 0.6 (src/navigation/module_graph.rs:31-35). Collision warning text: "N same-name receiver call site(s) with unknown receiver type across multiple owner types; not attributed as callers" (src/navigation/queries.rs:457-458).

## Edits

1. **CLAUDE.md**: replace every "26" algorithm claim (at ~:5, :17, :71, :74, :277 — grep for them) with 30. Add the 4 new algorithms to the "Algorithm Implementation Map" and to the Simple/Graph-based categorization lists (~:225-226) — read each module's header + imports (src/algorithms/{contract_slice,peer_consistency_slice,callback_dispatcher_slice,primitive_slice}.rs) to classify accurately: does it use only `ctx.files` (Simple) or `ctx.cpg`/call graph (Graph-based)? Fix "`output.rs` — Output formatters" (~:72) to `output/` (directory: mod.rs, navigation.rs, review.rs, mermaid.rs). Fix "The server exposes six read-only navigation tools" (~:208-216) with this exact framing: "The server exposes eight tools: six read-only navigation tools returning Prism `Evidence` JSON, one read-only reasoning tool `taint_reaches` (also Evidence), and one non-destructive local-state-changing tool `refresh_index` (returns a refresh summary)." Keep the existing six-tool list and add the two. Also fix CLAUDE.md's "Supported Languages" section: "11 languages" → "11 languages (12 tree-sitter grammar variants — TSX is parsed separately from TypeScript)".
2. **README.md**: fix the two stale "27 slicing algorithms" claims (~:41, :66) → 30. Leave the "30" claims alone. Do not touch anything else.
3. **Delete LLM.md.** First `grep -rn "LLM\.md" --include="*.md" .` and fix every inbound reference (remove or repoint to ALGORITHMS.md/README.md as sensible). Before deleting, check whether its "Recommended Prompt Structure" block (LLM.md:84-99) is covered by ALGORITHMS.md; if not, fold that one block into ALGORITHMS.md at a sensible spot (no other LLM.md content moves — the algorithm tables duplicate ALGORITHMS.md and the test-repo list is dropped deliberately).
4. **docs/MCP.md**: fix "~200" truncation claim (~:144) → "50 items by default (`max_results`, up to 1000), with an 80 KB result byte cap". Add `taint_reaches` and `refresh_index` rows to the tool table (~:125-130) with one-line descriptions (taint_reaches: forward taint reachability from a seed, read-only, returns Evidence; refresh_index: re-index the repo snapshot, local state change, returns a refresh summary). ALSO fix the "all read-only" blanket claim at docs/MCP.md:121 — with `refresh_index` in the table, "all tools are read-only / return Evidence" is false; scope the read-only/Evidence claim to the six nav tools + taint_reaches.
5. **skills/prism-code-navigation/SKILL.md**:
   - Replace the first gotcha (~:67-70, the "Call resolution is name-based, not type-based" bullet — now stale: receiver typing/interface dispatch shipped) with score semantics, e.g.: "**Read the `score` field.** `1.0` = exact resolution — act on it. `0.6` = name-only candidate — read the cited site before relying on it. A warning like `N same-name receiver call site(s) ... not attributed as callers` means real callers may be missing: treat 'no callers' plus that warning as *unknown*, not *none*." Keep it tight (same bullet-length style as the file).
   - Fix the truncation gotcha (~:75): "~200 nodes" → 50 items default (raise with `max_results`, max 1000) / 80 KB byte cap; keep the ~30 s first-call note (still true).
   - DO NOT edit the YAML frontmatter `description` (it is tuned for adoption-eval activation and quoted as a block scalar — breaking it breaks skill loading).
6. **Do NOT touch** skills/prism-code-slicing/SKILL.md (another in-flight task owns it), any file under docs/archive/, docs/superpowers/, docs/eval/, or create any new doc file.

## Done-checks (run and paste into your report)

```
grep -rn "26 code slicing\|List all 26\|(26 variants)\|All 26\|all 26\|27 slicing" README.md CLAUDE.md docs/MCP.md   # expect: no output
grep -rn "~200" docs/MCP.md skills/prism-code-navigation/SKILL.md                                                    # expect: no output
grep -rn "LLM\.md" --include="*.md" . | grep -v docs/archive | grep -v docs/superpowers | grep -v docs/analysis      # expect: no output
test ! -f LLM.md && echo deleted
python3 -c "import yaml,sys; yaml.safe_load(open('skills/prism-code-navigation/SKILL.md').read().split('---')[1]); print('frontmatter ok')"
```

Note in your report: the adoption-eval re-run (eval/adoption) is deliberately DEFERRED to the owner — SKILL.md byte changes invalidate its trajectory cache and re-running costs live API trials.

## Commit style

One or a few logical commits, conventional subjects (e.g. `docs: fix algorithm count 26/27 -> 30 across CLAUDE.md/README`). End each commit message with:
Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
