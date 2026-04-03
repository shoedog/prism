# Arrow Function Naming: Analysis & Fix Plan

## Status: Pre-Implementation Analysis — April 2026

-----

## Problem Statement

Arrow functions and anonymous function expressions are **completely invisible** to Prism’s call graph and data flow analysis. This is not a cosmetic naming issue — it’s a structural analysis gap that silently drops functions from all 26 slicing algorithms.

### The Failure Chain

When `function_name` returns `None` for an arrow function, three downstream consumers silently skip it:

**Call graph** (`call_graph.rs` lines 62, 89):

```rust
// Phase 1: Register function definitions
if let Some(name_node) = parsed.language.function_name(&func_node) {
    // ... register function
}
// Arrow functions: None → not registered as a function

// Phase 2: Scan for call sites within functions
let func_name = match parsed.language.function_name(&func_node) {
    Some(n) => parsed.node_text(&n).to_string(),
    None => continue,  // Arrow functions: calls INSIDE them are never scanned
};
```

**Data flow** (`data_flow.rs` line 88):

```rust
let func_name = match parsed.language.function_name(&func_node) {
    Some(n) => parsed.node_text(&n).to_string(),
    None => continue,  // Arrow functions: no def-use chains built
};
```

**Enclosing function lookup** (`ast.rs` line 78): `enclosing_function` finds the arrow function node correctly (it matches via `function_node_types` which includes `arrow_function`), but any analysis that then calls `function_name` on the result gets `None` and can’t associate the context.

### Impact Scope

This affects every language that uses `arrow_function` or anonymous `function_expression` — JavaScript, TypeScript, and (once JSX/TSX support ships) JSX and TSX. The impact is heaviest on:

- **React codebases:** The dominant component pattern is `const Component = () => <JSX />`. Every component defined this way is invisible.
- **Modern Node.js/Hapi:** Arrow functions for route handlers, middleware, and callbacks are standard.
- **Event handlers:** `element.addEventListener('click', () => { ... })` — inline handlers are invisible.
- **Array method callbacks:** `.map(item => ...)`, `.filter(x => ...)`, `.reduce((acc, x) => ...)` — these are less critical (they’re short-lived and usually don’t contain call sites of interest), but they do create noise in `all_functions` with no corresponding entry in the call graph.

### Quantifying the Gap

In a typical React component file with 10 exported components and 15 internal helper functions, if 80% are arrow functions (common in modern React), Prism’s call graph captures only 5 functions. The other 20 are invisible — no call sites within them are detected, no data flow edges are built, and no slicing algorithm can trace through them.

-----

## How Other Tools Handle This

### ECMAScript Specification: Name Inference

The ECMAScript specification (Section 13.15.1, `HasName` and `NamedEvaluation`) defines runtime name inference rules. When an anonymous function is assigned to a variable, the JS engine infers the function’s `.name` property:

```javascript
const foo = () => {};
console.log(foo.name); // "foo"

const obj = { bar: () => {} };
console.log(obj.bar.name); // "bar"
```

This is a runtime behavior, not a syntactic one — the function is still anonymous in the AST. But it demonstrates that the language spec itself recognizes the variable name as the canonical name for the function.

### React Compiler (Babel Plugin)

The React Compiler (v1.0, stable Oct 2025) does NOT skip arrow functions. Its Babel plugin registers three visitor handlers in `Program.ts`:

```typescript
FunctionDeclaration: traverseFunction,
FunctionExpression: traverseFunction,
ArrowFunctionExpression: traverseFunction,
```

All three go through the same `traverseFunction` pipeline. The compiler identifies components and hooks via `getComponentOrHookLike`, which uses Babel’s `NodePath` — a path object that provides `parent` access. For `const X = () => {}`, Babel’s path gives immediate access to the `VariableDeclarator` parent and its `name` field.

The compiler’s `compilationMode: 'infer'` (the default) determines whether a function is a component or hook by checking two things:

1. **Named like a component** (PascalCase) or hook (`useXxx`) — matching the ESLint rule convention.
1. **Creates JSX and/or calls a hook** — an additional heuristic to reduce false positives.

The key architectural insight: the React Compiler sidesteps the naming problem entirely because Babel’s AST representation gives every function its enclosing path context. It never needs to “infer” the name from the AST — the path IS the name.

### oxc Semantic Analyzer

oxc takes a different approach. Its AST distinguishes `BindingIdentifier` (declarations) from `IdentifierReference` (usages) at the parser level. For `const X = () => {}`:

- `X` is a `BindingIdentifier` in the `VariableDeclarator`
- The `arrow_function` is the `init` expression of the declarator
- After semantic analysis, `X` is a symbol in the scope tree with a `SymbolId`
- The arrow function creates a scope (with `ScopeFlags::ARROW`) that is a child of the scope containing `X`

oxc doesn’t have a “function name” concept per se — it has symbols and scopes. The “name” of the arrow function is the symbol that the enclosing `VariableDeclarator` binds it to. This is structurally different from tree-sitter’s model where the function node itself either has or doesn’t have a `name` child.

This is directly relevant to Prism’s oxc supplement architecture (from the JSX/TSX plan). When `OxcAnalysis` is available, the arrow function naming problem is solved by a different mechanism: look up which `SymbolId` has the arrow function’s scope as its declaration, and use that symbol’s name.

### ESLint `react/display-name` Rule

ESLint’s `react/display-name` rule warns when a component doesn’t have a display name. This is specifically triggered by anonymous arrow function components, `React.memo(() => {})` wrappers, and `React.forwardRef` without a displayName. The ESLint ecosystem treats this as a known problem requiring lint enforcement.

### ESLint `react/function-component-definition` Rule

This rule enforces consistent function types for components. Many teams configure it to require `function-declaration` for named components specifically because of the anonymous function problem — both for debugging (stack traces, React DevTools) and for static analysis tool compatibility.

-----

## Fix Architecture

### Approach 1: Parent-Walking in `function_name` (Immediate Fix)

Walk up via tree-sitter `node.parent()` from the arrow function to find the naming context. This is the direct analog of what Babel’s `NodePath` provides.

**Tree-sitter AST structure for `const X = () => {}`:**

```
program
  └─ lexical_declaration
       └─ variable_declarator
            ├─ name: identifier "X"        ← target
            └─ value: arrow_function       ← node we have
```

**Tree-sitter AST for `{ key: () => {} }`:**

```
object
  └─ pair
       ├─ key: property_identifier "key"   ← target
       └─ value: arrow_function            ← node we have
```

**Tree-sitter AST for `const X = React.memo(() => {})`:**

```
lexical_declaration
  └─ variable_declarator
       ├─ name: identifier "X"             ← target
       └─ value: call_expression (React.memo)
            └─ arguments
                 └─ arrow_function          ← node we have
```

#### Implementation

Insert before the final `node.child_by_field_name("name")` fallthrough in `function_name`:

```rust
// Arrow functions and anonymous function expressions inherit their name
// from the parent assignment context (ECMAScript name inference §13.15.1).
if (node.kind() == "arrow_function" || node.kind() == "function_expression")
    && node.child_by_field_name("name").is_none()
{
    if let Some(parent) = node.parent() {
        // Pattern 1: const X = () => {}
        // Parent: variable_declarator → extract name field
        if parent.kind() == "variable_declarator" {
            return parent.child_by_field_name("name");
        }

        // Pattern 2: { key: () => {} }
        // Parent: pair → extract key field
        if parent.kind() == "pair" {
            return parent.child_by_field_name("key");
        }

        // Pattern 3: const X = React.memo(() => {})
        // Parent chain: arguments → call_expression → variable_declarator
        if parent.kind() == "arguments" {
            if let Some(call) = parent.parent() {
                if call.kind() == "call_expression" {
                    if let Some(grandparent) = call.parent() {
                        if grandparent.kind() == "variable_declarator" {
                            return grandparent.child_by_field_name("name");
                        }
                    }
                }
            }
        }

        // Pattern 4: class Foo { handler = () => {} }
        // Parent: public_field_definition or field_definition → name field
        if parent.kind() == "public_field_definition"
            || parent.kind() == "field_definition"
        {
            return parent.child_by_field_name("name");
        }

        // Pattern 5: module.exports = () => {}  OR  exports.handler = () => {}
        // Parent: assignment_expression → extract left side
        if parent.kind() == "assignment_expression" {
            if let Some(left) = parent.child_by_field_name("left") {
                // For member_expression (exports.X), return the property
                if left.kind() == "member_expression" {
                    return left.child_by_field_name("property");
                }
                // For simple identifier (rare: x = () => {})
                if left.kind() == "identifier" {
                    return Some(left);
                }
            }
        }
    }
}
```

#### Pattern Coverage Assessment

|Pattern                                  |AST Parent Chain                                 |Covered         |Prevalence                           |
|-----------------------------------------|-------------------------------------------------|----------------|-------------------------------------|
|`const X = () => {}`                     |variable_declarator                              |Yes             |Very high — dominant React pattern   |
|`const X = function() {}`                |variable_declarator                              |Yes             |Common                               |
|`{ key: () => {} }`                      |pair                                             |Yes             |Common — object methods              |
|`const X = React.memo(() => {})`         |arguments → call_expression → variable_declarator|Yes             |Common — memoized components         |
|`const X = React.forwardRef((p,r) => {})`|Same as memo                                     |Yes             |Common — ref-forwarding components   |
|`class C { handler = () => {} }`         |public_field_definition                          |Yes             |Moderate — class field arrows        |
|`exports.X = () => {}`                   |assignment_expression                            |Yes             |Common — CommonJS modules            |
|`export default () => {}`                |export_statement                                 |No              |Moderate — truly anonymous           |
|`arr.map(x => x.id)`                     |arguments                                        |No              |Very high but low value              |
|`compose(withAuth)(() => {})`            |arguments → call_expression → arguments          |No              |Rare — deep HOC chains               |
|`useEffect(() => { ... })`               |arguments                                        |No (intentional)|Very high — handled by react_hooks.rs|
|`setTimeout(() => { ... }, 100)`         |arguments                                        |No              |Common but low value                 |

The uncovered patterns fall into two categories: truly anonymous functions (`export default () => {}`) where no name exists, and callback-position functions where the name would be the callee name, not a variable binding. The callback cases are better handled by the synthetic call graph edges in Layer 6 of the React plan.

#### Named Function Expression Preservation

The guard `&& node.child_by_field_name("name").is_none()` ensures that named function expressions retain their own name:

```javascript
const X = function namedFn() {}
// namedFn has child_by_field_name("name") = "namedFn"
// The parent-walk is skipped; "namedFn" is returned
```

This is correct — `namedFn` is the function’s own name (used for recursion and stack traces), and is the more semantically precise identifier.

### Approach 2: `collect_functions` Enrichment (Alternative)

Instead of fixing `function_name`, modify `all_functions` / `collect_functions` in `ast.rs` to return `(Node, Option<String>)` tuples — pairing each function node with its inferred name. This moves the name resolution logic out of the language abstraction and into the AST module.

**Pros:** Cleaner separation — `function_name` stays syntactic, name inference is a separate concern.
**Cons:** Requires changing the return type of `all_functions`, which cascades into call_graph, data_flow, and every algorithm that calls it. Much larger diff for the same result.

**Recommendation:** Fix `function_name` (Approach 1). It’s the minimal change with maximum impact, and it’s where every consumer already looks.

### Approach 3: oxc-Based Name Resolution (Future)

When `OxcAnalysis` is available (from the JSX/TSX plan Layer 5), arrow function naming can be resolved through oxc’s symbol table instead of tree-sitter parent-walking:

```rust
// In OxcAnalysis
pub fn function_name_at(&self, func_start_line: usize) -> Option<String> {
    // Find the scope created by the function at this line
    // Walk up the scope tree to find the enclosing variable declarator
    // Return the BindingIdentifier's symbol name
}
```

This is more robust than tree-sitter parent-walking because oxc’s semantic analysis handles all the edge cases (hoisting, re-declarations, export renaming) spec-correctly. However, it requires the oxc supplement to be built first.

**Recommendation:** Ship Approach 1 now, add Approach 3 as an enhancement when oxc arrives.

-----

## Interaction with Existing Architecture

### `all_functions` Implications

`all_functions` in `ast.rs` recursively collects nodes matching `function_node_types`, which includes `arrow_function` and `function_expression`. This means arrow functions are *collected* — they’re just dropped at the *naming* step. The fix in `function_name` means these collected nodes now have names and will be:

1. Registered as functions in the call graph
1. Scanned for call sites within their bodies
1. Analyzed for def-use chains in the data flow graph

This is a net positive but introduces a behavioral change: the call graph and data flow graph will suddenly contain more functions. This could surface as new findings in existing reviews (a good thing) or as unexpected test changes.

### Nested Arrow Functions

React components often contain arrow functions inside arrow functions:

```tsx
const App = () => {
    const handleClick = () => { ... };
    const items = data.map(item => <Item key={item.id} {...item} />);
    return <div onClick={handleClick}>{items}</div>;
};
```

With the fix, `App` gets named from the variable_declarator, `handleClick` gets named from its variable_declarator, and the `.map(item => ...)` callback has parent `arguments` which doesn’t match any naming pattern — it stays anonymous and is skipped. This is correct behavior: `App` and `handleClick` are meaningful function boundaries, while the `.map` callback is a transient expression.

### Performance

`node.parent()` in tree-sitter is O(1) — it follows a stored parent pointer, no tree search. Walking 2-3 levels up (for the React.memo case) is still constant time. No performance concern.

### Test Impact

Adding names to previously-anonymous functions will cause existing tests that assert on call graph or data flow results to produce *more* results than before. This is a breaking change in the “more correct” direction — the tests should be updated to expect the additional functions. No existing correct results should change.

-----

## Edge Cases and Known Limitations

### Truly Anonymous Functions

Some patterns have no assignable name:

```javascript
export default () => { ... }        // Export without binding
[1, 2, 3].forEach(() => { ... })    // Callback position
setTimeout(() => { ... }, 100)       // Timer callback
Promise.resolve().then(() => { ... }) // Promise chain
```

For these, `function_name` will continue to return `None`, and the functions will be skipped in call_graph and data_flow. This is acceptable — these functions are either:

- Truly anonymous (no meaningful name exists)
- Callbacks whose invocation context is modeled differently (synthetic call edges in Layer 6)

### IIFE (Immediately Invoked Function Expression)

```javascript
(function() { ... })();
(() => { ... })();
```

Parent is `call_expression` (the IIFE call itself), not a `variable_declarator`. The fix doesn’t name these, which is correct — IIFEs are anonymous by design.

### Re-assignment

```javascript
let handler = () => { console.log('v1'); };
handler = () => { console.log('v2'); };  // parent: assignment_expression
```

The first assignment is a `variable_declarator` (caught by Pattern 1). The second is an `assignment_expression` (caught by Pattern 5). Both get the name `handler`. In the call graph, this creates two function definitions with the same name — which is how Prism already handles function redefinition (same-named functions are stored as a `Vec<FunctionId>` keyed by name).

### TypeScript Type Annotations

```typescript
const X: React.FC<Props> = (props) => { ... };
```

The tree-sitter structure adds a `type_annotation` child to the `variable_declarator`, but the `name` field and `value` (arrow function) are still in the same positions. The fix works unchanged.

### Default Exports with Naming

```typescript
const Component = () => { ... };
export default Component;
```

Here `Component` is named via the variable_declarator (Pattern 1). The `export default Component` is a separate statement referencing the identifier — not relevant to function_name.

-----

## Implementation Plan

### Phase 1: Core Fix (Approach 1)

**Files changed:**

- `src/languages/mod.rs` — Add parent-walking logic to `function_name` for Patterns 1-5
- `tests/common/mod.rs` — Add `make_arrow_function_test()` fixture
- `tests/lang/javascript/arrow_test.rs` — Test all 5 patterns + edge cases

**Estimated effort:** 1-2 hours

**Test cases:**

1. `const X = () => {}` — verify X appears in call graph
1. `const X = function() {}` — verify X appears
1. `const X = function named() {}` — verify `named` appears (existing behavior preserved)
1. `{ key: () => {} }` — verify `key` appears
1. `const X = React.memo(() => {})` — verify X appears
1. `class C { handler = () => {} }` — verify `handler` appears
1. `exports.X = () => {}` — verify X appears
1. `export default () => {}` — verify function is still skipped (no name)
1. Nested arrows — verify only named ones appear
1. Verify all existing JS/TS tests still pass (with updated expectations)

### Phase 2: oxc Enhancement (Future, with Layer 5)

**Files changed:**

- `src/oxc_analysis.rs` — Add `function_name_at(line)` method using symbol table
- `src/languages/mod.rs` or `src/ast.rs` — Integrate oxc fallback for remaining edge cases

**Estimated effort:** 2-3 hours (after oxc integration exists)

### Phase 3: Callback-Position Functions (Future, with Layer 6)

For functions in callback position (`.map(() => {})`, `useEffect(() => {})`, `setTimeout(() => {})`), naming them after the callee isn’t the right model. Instead, the synthetic call graph edges from Layer 6 (deferred execution modeling) handle these by creating explicit edges from the calling context to the callback function. The callback doesn’t need a human-readable name — it needs a call graph edge.

-----

## Appendix: ECMAScript Name Inference Rules

The ECMAScript specification defines name inference in several places:

- **§13.15.1 `IsAnonymousFunctionDefinition`**: Returns true for arrow functions and function expressions without a `BindingIdentifier`.
- **§13.15.2 `NamedEvaluation`**: When a `VariableDeclarator` has an anonymous function as its initializer, the variable name is assigned as the function’s `.name` property.
- **§13.2.5.5 `PropertyDefinitionEvaluation`**: Object property definitions with anonymous function values infer the property name.
- **§15.2.3.8 `ClassFieldDefinitionEvaluation`**: Class field definitions with anonymous function values infer the field name.

These rules cover exactly the patterns Approach 1 handles. Prism’s parent-walking is the static analysis equivalent of the runtime name inference the spec mandates.
