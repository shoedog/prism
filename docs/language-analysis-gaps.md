# Language Analysis Gaps

**Date:** 2026-04-02
**Status:** Active — tracking known gaps in DFG/alias/AccessPath tracking
**Method:** Systematic testing of each language's idiomatic patterns against Prism's analysis pipeline

---

## Summary

| # | Gap | Languages | Severity | Root Cause | Fix Location |
|---|-----|-----------|----------|------------|--------------|
| 1 | Multi-target assignment | Go, Python | Critical | `extract_lvalue_paths` treats comma-separated identifiers as one string | `ast.rs`, `languages/mod.rs` | **Fixed** (PR D) |
| 2 | Optional chaining | JS/TS | Critical | `AccessPath::from_expr` splits on `.` but `?.` contains a dot | `access_path.rs` | **Fixed** (PR D) |
| 3 | For-of destructuring | JS/TS | High | Loop variable bindings not handled by destructuring alias extraction | `ast.rs` | **Fixed** (PR E) |
| 4 | `with...as` binding | Python | High | Context manager target not recognized as a declaration | `languages/mod.rs`, `ast.rs` | **Fixed** (PR E) |
| 5 | Walrus operator | Python | Medium | `named_expression` not in `is_assignment_node` | `languages/mod.rs` | **Fixed** (PR D+E) |
| 6 | Spread field provenance | JS/TS | Low | `{ ...a, ...b }` loses per-field origin tracking | Architectural | Deferred |
| 7 | Comprehension taint | Python | Low | Iteration variable binding not modeled in DFG | Architectural | Deferred |

---

## Gap 1: Multi-Target Assignment (Go, Python) — CRITICAL

### Problem

Go multi-return and Python tuple unpacking produce multiple L-values from a single assignment, but Prism treats the comma-separated text as one opaque identifier.

**Go:**
```go
val, err := getData()   // Produces def for "val, err" as one string
use(val)                // No edge — DFG has no def for "val" alone
```

**Python:**
```python
name, age = get_user()  # Produces ZERO L-values (fails is_plain_ident)
use(name)               # No edge — name is invisible to DFG
```

### Root Cause

`assignment_target()` returns the `expression_list` (Go) or `pattern_list` (Python) node. `node_text()` gives `"val, err"` or `"name, age"`. `extract_lvalue_paths()` receives this string, fails the `is_plain_ident` check (comma and space aren't alphanumeric), and returns empty.

For Go, the declaration path (`declaration_name`) also fires and creates `AccessPath::simple("val, err")` — a single def with a composite name that nothing will ever reference.

### Impact

- **Go:** Every function returning `(value, error)` — the dominant Go pattern. `val` and `err` have no DFG defs, so taint through return values and error checking is invisible.
- **Python:** Every tuple return, every `for k, v in dict.items()`, every `enumerate()`, every `zip()`. Extremely common.

### Fix

In `collect_assignment_paths`, when the assignment target is a `pattern_list` (Python) or `expression_list` (Go), iterate the child identifier nodes instead of using the composite text:

```rust
// In collect_assignment_paths, after getting the LHS node:
if lhs.kind() == "pattern_list" || lhs.kind() == "expression_list" {
    // Multi-target: split into individual identifiers
    let mut cursor = lhs.walk();
    for child in lhs.children(&mut cursor) {
        if child.kind() == "identifier" {
            let name = self.node_text(&child).to_string();
            out.push((AccessPath::simple(name), line));
        }
    }
} else {
    // Single target: existing path
    let lhs_text = self.node_text(&lhs).to_string();
    for path in extract_lvalue_paths(&lhs_text) {
        out.push((path, line));
    }
}
```

Also applies to Go `var_declaration` with multiple names (`var a, b int = 1, 2`) and Go type assertion (`val, ok := x.(Type)`).

### Additional Go patterns affected

```go
// Multi-return
val, err := getData()

// Type assertion
str, ok := x.(string)

// Range iteration
for key, value := range myMap {

// Channel receive
val, ok := <-ch
```

All use `expression_list` on the left side of `:=` or `=`.

### Additional Python patterns affected

```python
# Tuple unpack
name, age = get_user()

# Star unpack
first, *rest = get_items()     # rest is list_splat_pattern

# Dict iteration
for key, value in d.items():

# Enumerate
for i, item in enumerate(items):

# Swap
x, y = y, x

# Nested tuple
(a, b), c = nested_func()
```

All use `pattern_list` as the assignment target.

---

## Gap 2: Optional Chaining (JS/TS) — CRITICAL

### Problem

`obj?.config?.host` should produce `AccessPath { base: "obj", fields: ["config", "host"] }` but actually produces `AccessPath { base: "obj?.config?.host", fields: [] }` — a single opaque base with no field tracking.

### Root Cause

`AccessPath::from_expr` splits on `.` to extract fields. `?.` contains a `.` but the split produces fragments like `["obj?", "config?", "host"]` with trailing `?` characters — or more likely the presence of `?` before `.` prevents clean splitting.

Verified: `AccessPath::from_expr("obj?.config?.host")` → `{ base: "obj?.config?.host", fields: [] }`.
Compare: `AccessPath::from_expr("obj.config.host")` → `{ base: "obj", fields: ["config", "host"] }`.

### Impact

Optional chaining is ubiquitous in modern JS/TS — `data?.user?.email`, `props?.config?.theme`, `response?.body?.items`. Every optional chain loses field-sensitive tracking, falling back to base-level taint only.

Tree-sitter DOES parse this correctly as nested `member_expression` nodes with `optional_chain` tokens. The issue is only in `from_expr`'s text-based parsing, which is used as a fallback when AST-based extraction isn't available.

### Fix

In `AccessPath::from_expr`, normalize `?.` to `.` before splitting:

```rust
pub fn from_expr(expr: &str) -> Self {
    let normalized = expr.replace("?.", ".").replace("->", ".");
    // ... existing dot-split logic on normalized
}
```

Also verify that the AST-based field access extraction in `collect_identifier_paths` handles `member_expression` with `optional_chain` correctly (the `property_identifier` child should still be extracted as a field name regardless of whether the access is `?.` or `.`).

---

## Gap 3: For-of/For-in Destructuring (JS/TS) — HIGH

### Problem

```javascript
for (const { name, id } of items) {
    use(name);  // name has no def — 0 edges
}
```

The destructuring pattern inside a `for_in_statement` loop header isn't handled by the alias extraction.

### Root Cause

`extract_destructuring_aliases` only checks `lexical_declaration` / `variable_declaration` nodes (from `is_declaration_node`). The `for_in_statement` binds variables through its own syntax — the `const` keyword is part of the for-statement structure, not a standalone declaration.

Tree-sitter AST:
```
for_in_statement
  ( 
  const
  object_pattern
    shorthand_property_identifier "name"
    shorthand_property_identifier "id"
  of
  identifier "items"
  )
  statement_block { ... }
```

The `object_pattern` is a direct child of `for_in_statement`, not inside a `variable_declarator`.

### Fix

Extend `collect_aliases_inner` to check `for_in_statement` nodes (JS/TS `for...of` and `for...in`):

```rust
// In collect_aliases_inner:
if node.kind() == "for_in_statement" {
    // Check for destructuring pattern in loop variable
    if let Some(pattern) = find_child_pattern(&node) {
        if let Some(iterable) = node.child_by_field_name("right") {
            let rhs = self.node_text(&iterable).to_string();
            if is_plain_ident(&rhs) {
                // Each destructured property aliases iterable.property
                self.extract_pattern_aliases(&pattern, &rhs, line, out);
            }
        }
    }
}
```

Note: The alias is to `items.name`, `items.id` — treating the iterable as the base. This is semantically approximate (each iteration yields a different element) but correct for taint tracking (if `items` is tainted, `name` from any element is tainted).

---

## Gap 4: `with...as` Binding (Python) — HIGH

### Problem

```python
with open("file.txt") as f:
    data = f.read()     # f has no def
    send(data)          # taint from file → data → send is broken at f
```

`f` is bound by the `with` statement's `as` clause but isn't recognized as a declaration.

### Root Cause

`is_declaration_node` for Python returns `false` (Python uses assignment syntax for declarations). The `with_statement` → `with_clause` → `with_item` → `as_pattern` structure isn't checked anywhere.

Tree-sitter AST:
```
with_statement
  with_clause
    with_item
      as_pattern
        call "open(\"file.txt\")"
        as
        as_pattern_target "f"
```

### Fix

In `collect_assignment_paths`, add a check for Python `as_pattern` nodes:

```rust
// In collect_assignment_paths, for Python:
if node.kind() == "as_pattern" || node.kind() == "as_pattern_target" {
    if node.kind() == "as_pattern_target" {
        let name = self.node_text(&node).to_string();
        if is_plain_ident(&name) {
            out.push((AccessPath::simple(name), line));
        }
    }
}
```

This also handles `except Exception as e:` which uses the same `as_pattern` structure.

---

## Gap 5: Walrus Operator (Python) — MEDIUM

### Problem

```python
if (n := len(items)) > 10:
    use(n)    # n has no def
```

The walrus operator (`:=`) creates a `named_expression` node, which isn't in `is_assignment_node`.

### Root Cause

`is_assignment_node` for Python matches `"assignment" | "augmented_assignment"`. The `named_expression` kind is missing.

### Fix

Add `named_expression` to Python's `is_assignment_node`:

```rust
Self::Python => matches!(kind, "assignment" | "augmented_assignment" | "named_expression"),
```

The `named_expression` node has field names compatible with the existing extraction: the identifier is the `name` field, the value is the `value` field. Verify that `assignment_target` extracts from `named_expression` correctly — it may need a Python-specific case since tree-sitter-python uses `name` not `left` for the target field.

---

## Gap 6: Spread Field Provenance (JS/TS) — LOW

### Problem

```javascript
const merged = { ...defaults, ...overrides };
// If defaults.host is tainted and overrides.host is clean,
// is merged.host tainted? Prism says yes (base-level taint).
```

Taint propagates correctly at the base level (if `defaults` is tainted, `merged` is tainted), but field-level provenance is lost. The spread operator creates a shallow merge where later spreads override earlier ones, but Prism doesn't model this.

### Why LOW

Base-level taint is conservative (no false negatives). The false positive case (tainted field overridden by clean spread) is uncommon in practice — spread is typically used for defaults, not for sanitization. Modeling per-field spread semantics would require tracking object literal construction, which is a significant architectural addition.

### Recommendation

Defer. Document as a known limitation. If precision calibration data shows spread-related false positives as a category, revisit.

---

## Gap 7: Comprehension Taint (Python) — LOW

### Problem

```python
result = [x.name for x in items if x.active]
# If items is tainted, result should be tainted
# But the x binding inside the comprehension isn't tracked
```

The iteration variable `x` in the comprehension is bound from `items` but this binding isn't modeled in the DFG. The result list gets a def but no edge connects `items` → `x` → `x.name` → `result`.

### Why LOW

Comprehension taint is a false negative (misses taint), not a false positive. The LLM reviewer can reason about comprehension semantics from the source text. Modeling comprehension bindings would require extending the DFG to handle generator expressions and list/dict/set comprehension syntax across Python, JS (`Array.map`), and Go (no comprehensions).

### Recommendation

Defer. If backtesting shows comprehension-related false negatives as a significant category, implement Python comprehension `for x in items` as an implicit alias `x → items.[]`.

---

## Implementation Priority

**PR D (done):** Gaps 1, 2, 5 — multi-target assignment, optional chaining, walrus operator. Fixes in `ast.rs`, `access_path.rs`, `languages/mod.rs`.

**PR E (done):** Gaps 3, 4, plus PR D review fixes — for-of destructuring, with-as binding, walrus `assignment_value` bug. Extends `collect_aliases_inner` / `collect_assignment_paths` with new node type handling.

- Gap 3: Added `extract_for_in_lvalues` and `extract_for_in_aliases` for JS/TS `for_in_statement` with destructuring patterns (object_pattern, array_pattern) and simple identifier bindings.
- Gap 4: Added `as_pattern` / `as_pattern_target` detection in `collect_assignment_paths` for Python `with...as` and `except...as` bindings.
- Gap 5 fix: `assignment_value` now handles `named_expression` "value" field (was only checking "right", causing walrus RHS to be silently dropped).

**Deferred:** Gaps 6, 7 — spread provenance, comprehension taint. Architectural changes, low practical impact.
