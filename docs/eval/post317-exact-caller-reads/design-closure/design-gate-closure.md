# Exact caller-read admission grammar — bounded design closure

## Status and fixed boundaries

This amendment repairs the admission design on the existing candidate
`1a8354b2d80fdedc3b1ab877f5b63ec50913a09d` (tree
`0f89f8488327ed8c4184289b213b81970ce9118e`). It does not authorize a new
analysis subsystem or discard the accepted artifact.

Round-two review proved one remaining WRONG: a `class` expression under
`variable_declarator.value` is accepted because the current body-statement
allowlist recursively defaults to accepting unknown descendants. In JS, TS and
TSX, the candidate adds caller-owned flow from `value` to reads after the class
field boundary. The field-initializer edge already exists on the unchanged
base and is not one of the 12 candidate-only rows. Same-environment base
comparison shows those 12 later-read additions. The terminal report is
`/private/tmp/prism-post317-implementation/review/REVIEW-round2.md`, SHA-256
`bd3dbfa62e157a438a0f66b6e2820436d6b5ec668f5a4fd37d261c8713169c1e`.

The amendment changes only `ParsedFile::exact_read_callable_is_straight_line`
and compact table-driven tests in `src/cpg/exact_caller_read_tests.rs`.
Exact-primary facts, their independently classified labels, W1 endpoint
validation, W2 binding/member filters, W3 owner multiplicity, `VarLocation`
identity, legacy DFG maps, solver/kill/alias behavior, public APIs, Step 4,
cache 97 and navigation 53 remain fixed.

## Grammar authority

The finite contract is derived from the pinned grammars, not from observed
source spellings:

| Dialect | Pinned node schema | SHA-256 |
|---|---|---|
| JavaScript | `tree-sitter-javascript` 0.23.1 `src/node-types.json` | `0d80ab597fcf1310efb9694d4276c407655d060b8bbab8f4ffda0223c43e94bf` |
| TypeScript | `tree-sitter-typescript` pinned revision, `typescript/src/node-types.json` | `c790a733fc756b54d4e54dceeb7d2d51e40d8b57136e70277753a75804cce3e3` |
| TSX | same pinned revision, `tsx/src/node-types.json` | `78b5789145286799a27a0a7ecc36cc1bcb151f94ec7fa631b248459867010c8c` |

The schemas establish the reviewed failure path: a variable declarator's
`value` accepts the expression supertype; the named `class` node owns a
`class_body`; the body owns JS `field_definition` or TS/TSX
`public_field_definition`; that field's `value` is another runtime expression.
No node on this path is a nested function or `class_static_block`.

## Replacement contract

Replace the current `sequential_tree` blacklist/default-accept walk with a
context-sensitive, default-refuse validator. It must never decide from a node
kind alone when the grammar gives the child a runtime-bearing field.

### 1. Entry and signature

The root must be an error-free named `function_declaration` in JS, TS or TSX.
It must have exactly one named `name`, `parameters` and `body` child in their
grammar fields, with kinds `identifier`, `formal_parameters` and
`statement_block`. Any unclaimed named root child is refused except the
explicit TS/TSX erased fields below. Anonymous `async` and `*` children are
refused, so an async or generator function is rejected even when its body does
not contain `await` or `yield`. Runtime identifier tokens in the root name,
parameter patterns and declarations must not contain a backslash escape or be
exactly `eval`; ordinary unescaped non-`eval` Unicode remains admitted.

The finite signature grammar is:

- JS: every named `formal_parameters` child is `identifier`.
- TS/TSX: every named child is `required_parameter`; its `pattern` is exactly
  one `identifier`, its `value` field is absent, and its optional `type` field
  is erased as described below.
- Root `type_parameters` and `return_type` fields may be ignored only in
  TS/TSX and only when obtained by those exact field names. Their subtree is
  never visited as runtime syntax.
- `optional_parameter`, `assignment_pattern`, `rest_pattern`, decorators, and
  every signature runtime initializer refuse the entire exact-expansion gate.
  This is conservative and preserves the legacy graph; the existing
  target-level required-parameter check remains a second guard.

This entry rule rules out signature-time execution and prevents type names from
being mistaken for runtime reads.

### 2. Direct body statements

After excluding named `comment` trivia, the `statement_block` admits only this
ordered list:

- `empty_statement`;
- `expression_statement` with exactly one admitted runtime expression;
- `lexical_declaration` or `variable_declaration` containing only admitted
  `variable_declarator` children;
- `return_statement` with zero or one admitted runtime expression, only as the
  final non-comment/non-empty statement.

Every other named statement, block or declaration refuses the callable. This
includes `class_declaration`, nested function declarations, `throw_statement`,
branches, loops, switch/try/with, labels, debugger statements and recovery
nodes. There is no recursive fallback from a rejected statement to its
children.

A variable declarator must have an exact `identifier` in its `name` field and
zero or one `value` field containing an admitted runtime expression. TS/TSX
may additionally have an exact erased `type` field. Array/object patterns and
additional named children refuse the callable. TS/TSX definite-assignment `!`
is refused in this declaration context. The existing local-binding token check
remains in place.

### 3. Runtime expressions

The recursive expression validator is a closed match. For every admitted node,
all named children and every visible anonymous child must be claimed by the
listed fields, child collection, punctuation or operator rule; an extra named
child, unlisted anonymous token, unexpected field,
missing required field, parse error or missing node refuses the whole callable.
Structural punctuation such as parentheses, braces, commas and semicolons is
permitted only for the matching admitted production. The normative complete
anonymous-token inventory is
`/private/tmp/prism-post317-implementation/review/design-anonymous-token-table.md`,
SHA-256 `931005cc7a7f60f5b39e19592b22976028c1fe2642a00266732b6da2cbd3a2b1`;
every row is part of this contract.

| Kind | Admitted children and constraints |
|---|---|
| `identifier` | leaf whose raw token text contains no backslash escape and is not exactly `eval`; this applies in every runtime identifier position, including root name, parameter/declaration name, callee and wrapped descendants |
| `number`, `true`, `false`, `null`, `undefined` | leaf. `string` is deliberately outside this slice because nonempty strings own named content/escape children in the pinned grammars |
| `parenthesized_expression` | exactly one admitted runtime child; TS/TSX `type` may be erased only by exact field |
| `call_expression` | `function` is an `identifier` or admitted `member_expression`; `arguments` is exactly `arguments`; optional-chain/import/template callees are refused; anonymous `?.` is refused; TS/TSX `type_arguments` is the only erased extra field; identifier callee text `eval` is refused |
| `arguments` | zero or more admitted runtime-expression children; `spread_element` is refused |
| `member_expression` | admitted `object`; `property` is exactly a `property_identifier` whose raw token text contains no backslash escape and is not exactly `eval`; optional chains, computed/subscript/private properties are refused. This recognizes syntax only: W2 still refuses the participating binding and grants no member-flow authority |
| `assignment_expression` | `left` is exactly `identifier` or an admitted `member_expression`; `right` is admitted; the operator is ordinary anonymous `=`; anonymous `using` and patterns are refused. Existing lvalue/member inventory refuses the affected binding, while an unrelated eligible binding can remain positive |
| `binary_expression` | `left` and `right` are admitted and the anonymous `operator` field is exactly `+`. No other operator is admitted in this slice |
| `non_null_expression` (TS/TSX only) | exactly one admitted runtime-expression child |

All other expression kinds refuse. In particular, `class`, `arrow_function`,
`function_expression`, object/method definitions, JSX, `new_expression`,
`await_expression`, `yield_expression`, `conditional_expression`, sequence,
update, augmented assignment, short-circuit operators, tagged templates,
nonempty strings, dynamic import, `as_expression`, `satisfies_expression` and
`type_assertion` do not enter the exact producer.

The `+` production is deliberately narrow: it preserves the existing O14
source epoch `return item(value)+sink(value)` and O08 argument
`item(value + value)` without admitting the other operators from the grammar.

### 4. Erased-type boundary

Erasure is field-based, never prefix- or descendant-based. A helper may mark a
node as erased only when its parent production above asks for the exact grammar
field (`type`, `type_arguments`, `type_parameters`, or `return_type`) and the
dialect is TS/TSX. The validator does not recurse through that subtree and does
not treat its identifiers as runtime reads.

Runtime wrappers remain visible. Only `non_null_expression` is admitted, and
its single named runtime child is validated normally. `as_expression`,
`satisfies_expression` and `type_assertion` are refused in this slice: the
pinned schemas expose their type components as ordered named children rather
than named fields (and TSX does not expose `type_assertion` at all), so accepting
them would violate the field-only erasure rule. A type-shaped child in an
unlisted field, a runtime child hidden beneath an unlisted wrapper, a decorator,
or a default initializer is refusal. This prevents "skip types" from skipping
a class, callable or signature-time expression.

### 5. Implementation shape and unknown proof

Use one private policy function whose match has no accepting wildcard, for
example `exact_read_runtime_policy(kind) -> Option<Policy>`. `_ => None` is the
only default. The node visitor then enforces the policy's field, child-count,
dialect, punctuation, keyword and operator rules. It must account for every
named child by node ID and every visible anonymous child by kind and position;
comments are explicit trivia and erased type fields are explicit non-runtime
claims. Any unclaimed named or anonymous child refuses.

A private unit assertion must call the policy classifier with a sentinel such
as `future_runtime_expression` and assert `None`. A public behavioral negative
using known-but-unlisted `new_expression` proves that the visitor follows the
default-refuse route on a real tree. This pair distinguishes a closed policy
from a test-only `class` blacklist.

## Grammar-backed fixture matrix

Every row runs for JS, TS and TSX unless the dialect column narrows it. Accepted
rows assert the exact producer tuples and full graph; rejected rows assert no
new exact facts and complete primitive equality with unchanged base behavior.

| ID | Dialects | Source shape | Gate result and oracle |
|---|---|---|---|
| A1 | all | sealed R01 `function outer(value){sink(value);return item(value);}` | accept; preserve Def46–51→Use58–63 and add later Def46–51→Use77–82 with independent NameOnly label |
| A2 | all | R02 `prepare();` then `sink(value); return item(value);` | accept; first and later exact tuples remain exact |
| A3 | all | R02 simple earlier-line `const value=source()` | accept; local Def→both authenticated Uses, no owner/slot drift |
| A4 | all | O08/O14 `value + value` and `item(value)+sink(value)` | accept only `+`; preserve full/subset/incremental and boundary rows |
| A5 | all | `value.x` plus same-line `other` reads | accept callable syntax; refuse `value` binding; retain exact `other` tuple |
| A6 | all | `value.x=1; sink(other); return sink(other);` | accept assignment syntax; refuse affected `value`; retain exact `other` tuple |
| A7 | TS/TSX | required typed parameter, typed local, generic call, and `value!` | accept only the declared erased fields plus `non_null_expression`; exact bytes refer only to runtime `value`, never type identifiers |
| R1 | all | anonymous and named class-expression field initializer from round2 | refuse complete callable; zero 12 candidate-only rows; exact base parity |
| R2 | all | nested arrow and function expressions | refuse; no captured caller fact |
| R3 | all | async outer, generator outer, await and yield | refuse at signature or expression boundary |
| R4 | all | decorated class/expression nested in an initializer | refuse before producer in all three dialects; keep malformed-source recovery as a separate existing control rather than treating valid JS decorator syntax as parser error |
| R5 | all | `new C(value)` | refuse through real-tree default route; companion sentinel policy assertion returns `None` |
| R6 | all | branch, loop, try, switch, with, throw, short-circuit, conditional | retain current complete base refusal rows |
| R7 | all | default/rest signature initializer and destructured local | refuse whole gate; legacy rows unchanged |
| R8 | all | return followed by another runtime statement | refuse because return is not terminal |
| R9 | all | nonempty string argument beside otherwise eligible reads | refuse whole gate because `string` is not an admitted atom; number-literal assignment in A6 remains the positive literal control |
| R10 | TS/TSX | `as_expression`, `satisfies_expression`, and TS-only `type_assertion` | refuse at the ordered-child wrapper; asserted-member legacy rows remain byte-identical |
| R11 | grammar-supported dialects | optional call `sink?.(value)`, assignment with anonymous `using`, and TS/TSX declarator definite-assignment `!` | refuse the unlisted anonymous token in each exact parent context; ordinary call, ordinary `=`, typed local declaration, and non-null-expression `!` controls remain positive |
| R12 | all, plus TS/TSX non-null control | escaped direct eval `\u0065val(code)`, parenthesized `(eval)(code)`, TS/TSX `eval!(code)`, and escaped binding/callee/property identifiers | refuse before producer by raw-token leaf/callee rules; native and compiler probes show the three eval spellings mutate a local parameter from 1 to 2; unescaped non-`eval` Unicode identifier remains positive and erased type identifiers remain uninspected |

For A5/A6, the exact tuple for `other` and the absence of every `value` exact
fact must be asserted, so whole-callable refusal cannot masquerade as the
required binding-local behavior. For A7, use separate TS and TSX fixtures and a
plain-JS counterpart; do not make invalid-JS parser recovery an acceptance
oracle.

## Fail-first and validation sequence

1. Before production editing, append the class-expression R1 expectation and
   real-tree R5 default-refuse expectation to the permanent table. On
   `1a8354b2`, R1 must fail with the exact added tuples already authenticated;
   R5 is expected to fail if its reads currently enter the exact producer. A
   zero-selected or compile-invalid run is inadmissible.
2. Add A6, A7 and R8–R12 controls and run them on the unchanged checkpoint. Record
   each row as preservation-PASS or desired-RED; a preservation PASS is not
   described as RED. Run the identical public table on `9fb6c823` in the same
   environment to pin complete legacy behavior.
3. Replace only the admission helper. Do not add another denylist case. Run the
   whole table and the existing exact-caller, same-line, optional/default and
   reviewer adapter/primitive oracles.
4. Re-run subset/merge/remove, all O14 epochs, fresh/warm cache reconstruction,
   genuine v96 refusal→v97 rebuild→Hit, and final Rust/non-Rust gates. Preserve
   all explicit exclusions and invalid/incomplete Tier-A classifications.

## Budget and stop conditions

The frozen candidate is at 643/700 production changed lines and 1,510/1,600
production-plus-test changed lines. The replacement must reuse the existing
admission function and compact table-driven test data:

- target final production: at most 690 changed lines (maximum +47);
- target final total: at most 1,595 changed lines (maximum +85);
- no new production file, public type/API, solver, owner, cache or parser
  module; cache stays 97/navigation53;
- remove the old blacklist/default-recursion code rather than retaining both
  systems.

Before editing, compute the projected replacement against the planning base.
After formatting, recount. If the projected or actual final diff exceeds
700/1,600, if an accepted A-row needs a grammar production outside the table,
or if another unlisted runtime form is admitted, stop and return to design or
split the slice. Do not spend another implementation review round on a new
denylist instance.
