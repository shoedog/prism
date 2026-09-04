# Plan — `nav_symbol_spans` v1

**Design:** `docs/superpowers/specs/2026-09-04-nav-symbol-spans-v1-design.md`
**Exact base:** `c7cc2d9568f07f215f5da3335e4d10e1a4984f3b`
**Review cap:** two self-review rounds

1. Add query-level and AST-helper REDs for exact outer/name/body spans, Python decorators, Unicode byte offsets, CRLF/tabs, same-line/empty/body-less functions, and ambiguous/missing seeds. Add CLI and MCP REDs for the new subcommand/tool, strict input schema, read-only annotations, cap behavior, freshness, and tool-count contracts. Run focused filters before production edits and retain the exact failures.
2. Add bounded source-span/indentation helper types and exact-identity lookup on `ParsedFile`. Reuse the eager function table and language adapter; never recover by name/line alone, echo source, guess missing body structure, or truncate an allegedly exact indentation value.
3. Add the dedicated `SymbolSpans` result and `navigation::queries::symbol_spans`, reusing `seed::resolve_fn`. Project the outer CPG span plus optional AST name/body coordinates and deterministic unavailability reasons.
4. Add CLI `symbol-spans` parsing/dispatch/rendering and MCP input/schema/handler registration. Add a cap-aware structured-value response path that preserves default structured-content mode and transport freshness behavior.
5. Update exact tool-list assertions, README, `docs/MCP.md`, the bundled navigation skill, roadmap, and live handoff. Do not bump CPG or navigation-sidecar versions.
6. Run focused GREEN. Conduct review round 1, fix only closed findings, then round 2 at the declared cap. Park on recurring/open-class coordinate or grammar defects rather than extending silently.
7. Run format, diff, check, configured Clippy, full default and `mcp` suites with totals, then the release/Tier-A matrix/rebuild/quick sequence. Compare any attributable failure with exact base in the same environment. Refresh the handoff at every stable commit.
