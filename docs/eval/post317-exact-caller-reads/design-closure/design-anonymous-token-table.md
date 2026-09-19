# Closed anonymous-token inventory for the admitted grammar

This table completes the named-node/field policy. It is derived in one pass from pinned JavaScript0.23.1 `grammar.js` and TypeScript0.23.2 `common/define-grammar.js` for **every admitted production**, including the inherited JS productions used by TS/TSX. Whitespace is not a child; comments are explicitly recognized named trivia. Hidden automatic-semicolon productions do not authorize any additional visible token.

For each visited node, every visible anonymous child must be in that node's row and occur in the grammar-prescribed position/count; every visible named child must be claimed by its existing named-field/child policy. There is no generic “ignore punctuation/modifier” fallback. Punctuation inside a claimed erased type subtree belongs to that non-runtime subtree and is not an extra child of the runtime parent.

| Parent production | Only admitted visible anonymous children |
|---|---|
| `function_declaration` | `function`; reject `async`/`*` and every other modifier |
| `formal_parameters` | `(`, `)`, `,` |
| TS/TSX `required_parameter` | none; the identifier pattern and optional type annotation are named fields |
| `statement_block` | `{`, `}` |
| `empty_statement` | `;` when exposed as a child |
| `expression_statement` | optional `;` |
| `lexical_declaration` | `let` or `const` in its `kind` field, `,`, optional `;` |
| `variable_declaration` | `var`, `,`, optional `;` |
| `variable_declarator` | optional `=` paired with the value field; reject TS/TSX definite-assignment `!` |
| `return_statement` | `return`, optional `;` |
| `parenthesized_expression` | `(`, `)` |
| `call_expression` | none; its parentheses belong to the named `arguments` node. In particular TS/TSX optional-call `?.` is refused |
| `arguments` | `(`, `)`, `,` |
| `member_expression` | ordinary `.` only; any optional-chain representation is refused |
| `assignment_expression` | ordinary `=` only; TS/TSX anonymous `using` is refused |
| `binary_expression` | `+` only, claimed by its `operator` field |
| TS/TSX `non_null_expression` | `!` only, after the runtime child |
| admitted identifier/property-identifier leaf | no child nodes, no backslash, and raw token text is not `eval`; escaped/reflective runtime spellings are refused, ordinary Unicode remains allowed |
| admitted number/boolean/null/undefined leaf | no child nodes; the leaf's own token text is not a child |

The newly identified parser-level distinctions are finite and source-backed: TS optional call uses an anonymous `?.`, TS assignment permits an anonymous `using`, and TS variable declaration has a separate definite-assignment `!` branch. These are rejected by the context table without adding any of them to the admitted language. Non-null `!` remains accepted only in its dedicated expression production. One compact negative table should distinguish the rejected modifier contexts from the already required ordinary assignment/declaration/non-null positives; no new feature is requested.

## Runtime identifier atom restriction

The pinned JS identifier rule is a lexical token and explicitly permits Unicode escape spellings. Named-child closure therefore cannot recognize reflective `eval` or same-binding identity from raw spelling alone. Refuse any runtime identifier/property-identifier token containing a backslash, including callable names, parameter patterns and local declaration names; inspect these claimed leaves rather than skipping them solely because their kind is `identifier`. Do not decode names or expand global identity. Ordinary Unicode characters and comments remain admissible, and type-only erased fields remain non-runtime. A runtime identifier/property-identifier leaf named exactly `eval` is refused everywhere, including admitted callee descendants, rather than only when the entire raw callee text equals `eval`. This conservative rule is independent of wrappers: `(eval)(code)` and TS `eval!(code)` must remain refused. The current call-production restriction to an identifier/member callee already excludes direct parenthesized/non-null callees, but the leaf restriction prevents any wrapper/refactoring route from weakening reflection refusal. Unrecognized leaf kind or unaccepted lexical form refuses; no identifier decoding or general name resolution is introduced.

Native Node control `design-escaped-identifier-probe.js` returned `{"escaped_direct_eval_result":2}` after passing1: escaped callee `\u0065val(code)` performs direct local eval. Indirect/global eval would leave this parameter unchanged. Add a compact escaped-eval/escaped-binding refusal row beside ordinary Unicode/identifier positives; this is a conservative lexical boundary, not new name-resolution semantics.

The same native probe confirms parenthesized direct eval changes the local parameter to2. Pinned TypeScript5.9.3 transpilation erases the non-null assertion and the emitted program also returns2. Preserve these two wrapper negatives with the escaped-eval negative; all three distinguish reflection refusal from raw-callee-text matching.
