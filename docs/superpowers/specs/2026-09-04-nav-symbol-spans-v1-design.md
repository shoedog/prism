# `nav_symbol_spans` v1

**Status:** design of record; implementation authorized by the owner-selected next-increment authority
**Recorded:** 2026-09-04
**Exact base:** `c7cc2d9568f07f215f5da3335e4d10e1a4984f3b` (PR #233 merge)
**Scope:** read-only callable edit coordinates through CLI and MCP

## 1. Decision and slice boundary

Add one read-only navigation query, `nav_symbol_spans`, with CLI parity as `prism nav symbol-spans`. It accepts the existing callable seed grammar (a symbol name with optional file, or a file/line location) and returns exact source coordinates already supported by Prism's loaded parse tree: the outer callable, its name when exposed by the grammar, its body when exposed by the grammar, before/after insertion anchors, and indentation context.

V1 is callable-only. Classes, fields, variables, arbitrary statements, edit application, rename/refactor semantics, syntax generation, formatter invocation, filesystem writes, and full Serena-style write tools are out of scope. The result is an aid to the consumer's own editor, never permission for Prism to mutate source.

## 2. Why this increment

The receiver-typing queue is complete, and the owner previously directed work away from more eval-only increments. The remaining roadmap's first bounded product idea is symbol-anchored edit coordinates. Existing `SymbolRef::Function`, CPG nodes, and `FunctionInfo` already retain outer byte spans; `LoadedRepo` retains `ParsedFile`, source text, and the tree-sitter tree. The missing product surface is a precise, explicit projection of those facts.

Java J1 is not immediately executable on this machine: the Tier-A registry has no Java/JDT-LS adapter or Java corpus, and `jdtls` is absent. This slice does not choose between native Java and LSP delegation; it advances the independent product/navigation path without claiming Java evidence.

## 3. Input and selection contract

- MCP input is exactly `{ "seed": SeedInput }`, with `additionalProperties: false`.
- CLI accepts the existing mutually exclusive `--symbol`/`--file` or `--location file:line` shape and `--format text|json`.
- Selection reuses `navigation::seed::resolve_fn`; location takes precedence only through the existing normalized seed representation, same-name ambiguity remains an error, and no new fuzzy lookup is introduced.
- The selected symbol must be a CPG function node with a matching current `ParsedFile` function identity. A missing AST identity is reported as unavailable structure, never reconstructed from name or line alone.

## 4. Output contract

Return a dedicated, tool-specific serializable result rather than overloading ordinary navigation `Evidence` reasons:

- `schema_version`: tool-specific `1.0`.
- `query`: deterministic selected-seed description.
- `symbol`: the canonical `SymbolRef::Function`.
- `symbol_span`: exact outer function/decorator span as a `Location`.
- `name_span`: exact grammar name-node `Location`, or `null`.
- `body_span`: exact grammar `body` field `Location`, or `null`. This is the raw grammar body node, including delimiters when the grammar includes them; it is not advertised as a delimiter-free body-content range.
- `insert_before`: zero-width anchor at `symbol_span.start_byte` and its 1-indexed line.
- `insert_after`: zero-width anchor at `symbol_span.end_byte` and its 1-indexed line. Anchors do not silently add whitespace or newlines.
- `indentation.symbol`: exact spaces/tabs preceding the outer symbol on its line, or `null` when the line prefix contains other text or exceeds the bounded context limit.
- `indentation.body`: exact spaces/tabs preceding the first named body child, or `null` for empty/same-line/non-whitespace/unsupported bodies.
- `unavailable`: deterministic field-to-reason entries for every `null` coordinate/context.
- `warnings`: an ordinary navigation warning array, empty on a fresh result and available to the existing transport freshness wrapper for `StaleIndex`.

All offsets are UTF-8 byte offsets into the indexed source snapshot, end-exclusive. Lines are 1-indexed and inclusive, matching existing `Location`. The response contains coordinates and bounded whitespace only—no symbol body/source echo.

## 5. AST and query seam

Add a small `ParsedFile` helper keyed by the exact function byte identity. It reconstructs the function node through the eager table, unwraps a Python decorated definition only for name/body discovery, and obtains `name` through the language adapter plus `body` through the grammar field. It computes whitespace-only line prefixes without character-count assumptions and refuses oversized indentation rather than truncating an allegedly exact value.

`navigation::queries::symbol_spans` resolves the callable once, projects the CPG outer identity, consults that helper, and creates the dedicated result. It does not rebuild the CPG, rescan the repository, or change call resolution.

## 6. MCP and CLI exposure

- Register a seventh read-only `nav_*` tool with a when/when-not/example description and the standard snapshot notice hedge.
- Add a cap-aware structured-value result helper so a pathological identifier/path cannot bypass the existing MCP wire budget. Default-path `content[0].text` remains canonical; `structuredContent` follows the configured transport mode.
- Preserve freshness metadata and stale-index warnings through the existing transport wrapper.
- Add `NavQuery::SymbolSpans` and dispatch it through the same `api::nav_session` as other CLI navigation.
- Update tool-list counts, transport/smoke contracts, README, `docs/MCP.md`, and the bundled `prism-code-navigation` skill.

## 7. Persistence and compatibility

No CPG cache or navigation sidecar field changes. The query derives its result from `ParsedFile` and existing CPG function identity, so cache versions remain unchanged. Existing six navigation outputs are byte-for-byte structurally unchanged. The new result uses its own `schema_version` and is additive to MCP `tools/list` and the CLI subcommand enum.

## 8. RED/GREEN acceptance

- A named multiline callable returns exact outer/name/body source slices, correct end-exclusive UTF-8 byte offsets, before/after anchors, and symbol/body indentation.
- A Python decorated callable keeps decorators in `symbol_span` while name/body point to the inner function definition.
- A Unicode prefix proves offsets are bytes, not characters; CRLF and tabs do not corrupt anchors or indentation.
- A same-line body does not invent body indentation. An empty or body-less declaration yields `null` plus a reason, not guessed content coordinates.
- Same-name ambiguity, missing symbols, invalid/escaping locations, missing seed, and unknown MCP properties retain fail-closed errors.
- MCP lists and invokes the seventh navigation tool as read-only, respects the result cap and structured-content mode, and carries freshness metadata.
- CLI JSON matches the query result and text output names all available coordinates without source echo.
- Existing navigation, transport, cache, and language behavior remains unchanged.

Every behavior branch needs a pre-production RED plus a negative or edge pole. Source-text-only assertions do not count as production proof.

## 9. Verification and review

Review cap: two self-review rounds because delegation is not authorized in this session. At the cap, recurring/open-class coordinate or grammar findings park the artifact; closed non-repeating findings get bounded fixes on the same branch.

Required gates: focused RED/GREEN; AST, navigation, CLI, MCP input/tool/transport/smoke targets; format/diff/check/configured Clippy; full default and `mcp` suites with exact totals; release build; Tier-A matrix-only; immediate second release build; Tier-A quick. Any failure attribution requires an exact-base run in the same environment. Tier-A pin drift is reported, never re-baselined.
