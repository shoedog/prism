# CPG Architecture Improvements

**Date:** 2026-04-02
**Status:** Approved priorities — ready for implementation
**Scope:** Post-Phase 6 improvements derived from open questions in `docs/cpg-architecture.md`

---

## Priority Summary

| # | Item | Effort | Impact | Rationale |
|---|------|--------|--------|-----------|
| 1 | JS/TS destructuring alias tracking | 2-3 days | High | Taint blind spot in team's daily JS/TS code |
| 2 | Build CPG once, share across algorithms | 1-2 days | High | 12 algorithms rebuild identical CPG; `--algorithm all` does 12x redundant work |
| 3 | `CpgContext` bundle type | 0.5 days | Medium | Required by #2; makes CPG + ParsedFile co-dependency explicit |
| 4 | Tree-sitter struct extraction fallback | 2-3 days | Medium | Zero-dependency C/C++ field enumeration when clang unavailable |
| 5 | RTA refinement for virtual dispatch | 1 day | Low | Reduces CHA overapproximation if C++ polymorphism appears in reviewed repos |
| 6 | Python `self` cross-method tracking | Deferred | Low | Requires interprocedural alias analysis; out of scope for intraprocedural model |
| 7 | Lua colon method syntax | 0.5 days | Low | Add `method_index_expression` to `is_field_access_node` |

---

## 1. JS/TS Destructuring Alias Tracking

### Problem

JavaScript destructuring creates variables from object fields but Prism doesn't trace the relationship:

```javascript
const { name, id, config } = device;
// name is actually device.name, id is device.id, config is device.config
// Prism treats name, id, config as independent variables
```

If `device` is tainted (e.g., from user input), taint propagates to `device.name` via field-sensitive DFG, but NOT to the destructured `name` variable. This is a real taint-tracking blind spot — destructuring is idiomatic in modern JS/TS and appears in virtually every file.

### Scope

Extend `collect_alias_assignments` in `ast.rs` to recognize destructuring patterns and emit field-qualified aliases.

### Tree-sitter AST Structure

JS/TS destructuring appears as:

```
// const { name, id } = device;
lexical_declaration
  variable_declarator
    name: object_pattern
      shorthand_property_identifier_pattern "name"
      shorthand_property_identifier_pattern "id"
    value: identifier "device"
```

Renamed destructuring (`const { name: userName } = device;`) appears as:

```
lexical_declaration
  variable_declarator
    name: object_pattern
      pair_pattern
        key: property_identifier "name"
        value: identifier "userName"
    value: identifier "device"
```

Array destructuring (`const [first, second] = items;`) appears as:

```
lexical_declaration
  variable_declarator
    name: array_pattern
      identifier "first"
      identifier "second"
    value: identifier "items"
```

### Implementation

Extend `collect_aliases_inner` in `src/ast.rs` to handle destructuring for JS/TS:

```rust
// In collect_aliases_inner, after the existing declaration check:

// JS/TS destructuring: const { name, id } = obj -> name aliases obj.name, id aliases obj.id
if self.language.is_declaration_node(node.kind()) {
    if let Some(name_node) = self.find_destructuring_pattern(&node) {
        if let Some(val_node) = self.language.declaration_value(&node) {
            let rhs = self.node_text(&val_node).to_string().trim().to_string();
            if is_plain_ident(&rhs) {
                self.extract_destructuring_aliases(&name_node, &rhs, line, out);
            }
        }
    }
}
```

New helper methods:

```rust
/// Check if a declaration has a destructuring pattern (object_pattern or array_pattern).
fn find_destructuring_pattern<'a>(&self, node: &Node<'a>) -> Option<Node<'a>>

/// Extract aliases from a destructuring pattern.
///
/// { name, id } = obj -> [(name, obj.name), (id, obj.id)]
/// { name: userName } = obj -> [(userName, obj.name)]
/// [first, second] = arr -> [(first, arr.[]), (second, arr.[])]  // array-insensitive
fn extract_destructuring_aliases(
    &self, pattern: &Node<'_>, rhs_base: &str, line: usize,
    out: &mut Vec<(String, String, usize)>,
)
```

### DFG Integration

**Key design decision:** The alias map needs to store `(alias_name, target_access_path)` not just `(alias_name, target_base_name)`. Change the alias map type from `BTreeMap<String, String>` to `BTreeMap<String, AccessPath>`. For simple aliases (`ptr = dev`), the AccessPath has no fields. For destructuring (`const { name } = dev`), the AccessPath is `{ base: "dev", fields: ["name"] }`.

### Python Destructuring (Deferred)

Python has tuple unpacking (`a, b = func()`) but this doesn't map cleanly to field access — `a, b = get_pair()` doesn't tell us which field `a` corresponds to. Fundamentally different from JS object destructuring where property names establish the mapping.

### Test Plan

```
test_destructuring_object_basic_js       — const { a, b } = obj; taint on obj -> a, b tainted
test_destructuring_renamed_js            — const { name: n } = obj; taint on obj.name -> n tainted
test_destructuring_nested_js             — const { config: { host } } = obj; -> host aliases obj.config.host
test_destructuring_array_js              — const [first] = arr; -> first aliases arr.[]
test_destructuring_no_false_positive_js  — const { a } = x; const { a } = y; -> independent
test_destructuring_python_tuple          — name, age = get_user(); (if supported)
test_destructuring_typescript            — verify same behavior as JS
```

---

## 2. Build CPG Once, Share Across Algorithms

### Problem

12 algorithms internally call `CodePropertyGraph::build_enriched(files, type_db)`, which rebuilds `DataFlowGraph`, `CallGraph`, the petgraph, all indices, and CFG edges from scratch. When running `--algorithm all` (26 algorithms), the CPG is built 12 times identically.

### Design: `CpgContext`

```rust
/// Shared analysis context built once per review, passed to all algorithms.
pub struct CpgContext<'a> {
    pub cpg: CodePropertyGraph,
    pub files: &'a BTreeMap<String, ParsedFile>,
    pub type_db: Option<&'a TypeDatabase>,
}
```

### Migration Path

1. Add `CpgContext` to `src/cpg.rs`
2. Build context once in `main.rs` before algorithm loop
3. Migrate all 26 algorithm signatures from `(files, diff, type_db)` to `(ctx, diff)`
4. Update ~200 test call sites (mechanical search-and-replace)
5. `delta_slice` special case: keeps building old-file CPG internally

### `delta_slice` Special Case

`delta_slice` builds CPGs for both old and new file versions. Keep internal old-file CPG construction but accept `CpgContext` for the new-file version.

---

## 3. `CpgContext` Bundle Type

Included in the same PR as item #2. Key design decisions:

- `CpgContext` borrows `files` and `type_db` (lifetime `'a`) rather than owning them
- `cpg` is owned by `CpgContext` — built during construction
- `CpgContext` is immutable once built — algorithms receive `&CpgContext`

---

## 4. Tree-Sitter Struct Extraction Fallback

### Problem

`TypeDatabase::from_compile_commands` requires clang and `compile_commands.json`. When neither is present, type enrichment silently degrades to nothing.

### Design

Zero-dependency fallback extracting struct/class/union definitions from tree-sitter ASTs:

```rust
impl TypeDatabase {
    pub fn from_parsed_files(files: &BTreeMap<String, ParsedFile>) -> Self
}
```

### What tree-sitter gives vs. what clang gives

| Capability | tree-sitter extraction | clang AST dump |
|------------|----------------------|----------------|
| Field names | Yes | Yes |
| Field types | Approximate (declaration text) | Exact (`qualType`) |
| Typedef resolution | No (can't follow `#include`) | Yes |
| Nested struct fields | Yes (if defined inline) | Yes |
| Base classes | Yes (`base_class_clause`) | Yes |
| Virtual methods | Yes (`virtual` specifier) | Yes |
| Size/offset | No (requires compiler layout) | Yes |
| Macro-expanded types | No (sees ERROR nodes) | Yes |

~70% of the value for field enumeration and basic class hierarchy.

### Integration

Auto-enable in `main.rs` for C/C++ files when `--compile-commands` not provided.

### Test Plan

```
test_ts_fallback_extracts_c_struct     — struct with 3 fields, verify field names
test_ts_fallback_extracts_cpp_class    — class with fields + virtual method
test_ts_fallback_skips_forward_decl    — struct device; (no body) -> not extracted
test_ts_fallback_union_detection       — union -> RecordKind::Union
test_ts_fallback_nested_struct         — struct with inline struct field
test_ts_fallback_no_false_extraction   — Python/JS files -> empty TypeDatabase
```

---

## 5. RTA Refinement for Virtual Dispatch

### Problem

CHA adds Call edges to ALL classes that override a virtual method. If `Shape::draw()` has overrides in 4 classes but only 2 are instantiated, every call gets 4 extra edges.

### Design: Rapid Type Analysis (RTA)

Scan all files for instantiation expressions (`new ClassName`, `make_unique<ClassName>`, stack allocation). Build a "live classes" set. Filter virtual dispatch targets to only live classes.

Currently low-impact because firmware uses minimal C++ polymorphism. Becomes material if C++ OOP patterns appear in CMTS control plane code.

### Test Plan

```
test_rta_filters_uninstantiated_class  — only Circle instantiated -> no edge to Square::draw
test_rta_preserves_instantiated_class  — Circle instantiated -> edge preserved
test_rta_fallback_no_type_db           — without TypeDatabase -> CHA (no filtering)
test_rta_stack_allocation              — Circle c; counts as instantiation
```

---

## 6. Python `self` Cross-Method Tracking (Deferred)

`self.config = tainted` in `method_a` doesn't taint `self.config` in `method_b` on the same class. Correct for intraprocedural DFG — each method is a separate function scope. Fixing requires interprocedural alias analysis.

**Revisit trigger:** If precision calibration shows Python class-attribute false negatives > 10% of misses.

---

## 7. Lua Colon Method Syntax (Quick Fix)

`obj:close()` uses `method_index_expression`, not `dot_index_expression`. One-line addition to `is_field_access_node` in `ast.rs`.

---

## Implementation Order

```
PR A: CpgContext + build-once refactor (items #2 + #3)
  - Add CpgContext type
  - Build CPG once in main.rs
  - Migrate all 26 algorithm signatures
  - Update ~200 test call sites
  - delta_slice special case

PR B: JS/TS destructuring aliases (item #1)
  - Extend collect_alias_assignments for destructuring patterns
  - Change alias map type to BTreeMap<String, AccessPath>
  - Update DFG builder for simple-path alias resolution
  - 7 tests
  - Lua colon fix (item #7, 1 line)

PR C: Tree-sitter struct fallback (item #4)
  - TypeDatabase::from_parsed_files
  - Auto-enable for C/C++ files without --compile-commands
  - 6 tests
  - RTA refinement (item #5, 4 tests)
```

PR A is the highest-leverage change (12x build elimination). PR B closes the biggest taint-tracking gap for daily JS/TS code. PR C improves firmware analysis with zero new dependencies.
