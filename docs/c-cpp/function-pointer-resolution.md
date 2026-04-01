# Function Pointer Call Resolution in Prism

Last updated: 2026-04-01

---

## Background

C/C++ code frequently calls functions through pointers rather than by direct
name. The call graph must resolve these indirect calls to maintain accurate
edges for CircularSlice (cycle detection), MembraneSlice (cross-file caller
detection), VerticalSlice (feature path tracing), and BarrierSlice
(interprocedural depth limiting).

Tree-sitter parses all indirect calls as `call_expression` nodes, but the
"function" child is an expression (variable, subscript, cast) rather than a
plain identifier. Prism's call graph resolves these at varying levels of
precision.

---

## Resolution Levels

### Level 0 — Field-Access Dispatch (Implemented)

**Commit:** `67018e0` (PR #7 branch)

**Pattern:**
```c
struct file_operations fops = { .open = my_open, .read = my_read };
// ...
fops->open(inode, file);    // field_expression: resolves to "open"
timer->callback(data);      // field_expression: resolves to "callback"
obj.method(args);           // field_expression: resolves to "method"
```

**How it works:** `call_function_name()` in `languages/mod.rs` detects
`field_expression` and `member_expression` nodes and extracts the `field`
child identifier. The callee name `"callback"` then matches any function
definition named `callback` in the call graph.

**Handles:**
- `ptr->func(args)` — struct field dispatch (kernel `file_operations`,
  driver `ops` tables, callback structs)
- `obj.method(args)` — dot-access dispatch

**Does NOT handle:**
- The resolution is name-based: if multiple unrelated functions share the
  same name as the field, all will match (false positive edges)
- Chained access: `a->b->func()` resolves to `func` (correct), but
  `a->b.func()` depends on tree-sitter nesting

---

### Level 1 — Local Variable Function Pointers (Implemented)

**Pattern:**
```c
void (*fptr)(int) = target_func;
fptr(42);                       // resolves to "target_func"

callback_fn cb = get_handler();
cb = fallback_handler;          // last assignment wins
cb(99);                         // resolves to "fallback_handler"
```

**How it works:** When the callee of a `call_expression` is a plain
identifier that does NOT match any known function definition, scan the
enclosing function body for assignment statements where the LHS is that
identifier and the RHS is a known function name. Use the last such
assignment (simple must-alias within a single function).

**Handles:**
- `fptr = known_function; fptr(args);` — direct local assignment
- Reassignment: tracks the last assignment in source order
- Initialization in declaration: `void (*fp)(int) = handler;`

**Does NOT handle:**
- Conditional assignment: `fptr = cond ? func_a : func_b;` — only one
  branch is captured (whichever appears last in source order)
- Assignments inside loops or branches — no path sensitivity
- Parameter-passed function pointers (`void call_it(callback_fn cb)`)
- Function pointers stored in heap-allocated structs then loaded back
- `fptr` assigned in one function, called in another

**False positives:** If `fptr` is reassigned conditionally, edges to the
wrong target may appear. This is acceptable for a code review tool — showing
a potential edge is better than showing none.

---

### Level 2 — Array/Table Dispatch (Implemented)

**Pattern:**
```c
callback_fn handlers[] = {func_a, func_b, func_c};
handlers[idx](data);           // resolves to func_a, func_b, func_c (all)

static const struct {
    const char *name;
    handler_fn fn;
} dispatch_table[] = {
    {"get", handle_get},
    {"set", handle_set},
};
dispatch_table[i].fn(req);     // resolves to "fn" via Level 0
```

**How it works:** When the callee of a `call_expression` is a
`subscript_expression` (e.g., `handlers[0]`), extract the array base name,
find its initializer list in the enclosing function (or at file scope), and
add call edges to every function name that appears in the initializer.

**Handles:**
- `handlers[N](args)` — static dispatch tables with known initializers
- Adds edges to ALL entries in the array, since the index is generally
  not statically known
- File-scope (global) dispatch tables
- Local dispatch tables

**Does NOT handle:**
- Dynamically populated arrays: `handlers[i] = get_handler(i);`
- Arrays initialized via loops
- Multi-dimensional arrays or arrays of structs (partially covered by
  Level 0 for the `.fn` field access case)
- Arrays passed as parameters

**False positives:** All entries in the initializer are treated as potential
call targets. For a table with 20 entries, this creates 20 edges. This is
correct for dispatch tables (any entry could be called) but may inflate
the call graph for unrelated arrays that happen to contain function names.

---

### Level 3 — Parameter-Passed Function Pointers (Not Implemented)

**Pattern:**
```c
void execute(callback_fn cb, int data) {
    cb(data);                   // what is cb?
}

// Callers:
execute(handler_a, 1);
execute(handler_b, 2);
```

**What it would require:** Interprocedural analysis:
1. Find all callers of `execute` (already available via the call graph)
2. At each call site, determine the argument passed for parameter `cb`
3. If the argument is a known function name, add an edge from `execute`
   to that function

**Scope:** A 1-hop version (check direct callers only) would catch most
practical cases. Multi-hop (the callback is itself passed through several
layers) gets combinatorially expensive.

**Why it's hard:**
- Requires mapping parameter positions to argument expressions
- The argument might itself be a variable, requiring Level 1 resolution
  at the caller site
- Recursive or mutually recursive callers create infinite chains
- Virtual dispatch in C++ adds another dimension

**Practical value:** High for callback-heavy APIs (Linux kernel
`register_*` functions, event loops, test frameworks). The 1-hop version
would cover ~80% of real-world cases.

**Estimated effort:** 4-8 hours for 1-hop; significantly more for multi-hop.

---

### Level 4 — Full Points-To Analysis (Not Planned)

**Pattern:**
```c
void **table = malloc(n * sizeof(void*));
table[0] = (void*)func_a;
// ... later, in a different function ...
((callback_fn)table[idx])(data);
```

**What it would require:** Whole-program alias analysis (Andersen's or
Steensgaard's algorithm):
1. Build a constraint graph for all pointer assignments across all files
2. Solve for the points-to set of every pointer variable
3. At each indirect call site, look up the points-to set to determine
   possible targets

**Why it's not practical for Prism:**
- Requires the complete compilation unit — Prism typically only has
  the files in the diff plus directly referenced files
- Andersen's is O(n^3) in the number of pointer variables; Steensgaard's
  is nearly linear but much less precise
- C casts (`(void*)func`) and pointer arithmetic defeat type-based
  narrowing
- The analysis would need to handle `malloc`, `realloc`, stack vs. heap,
  and array-of-pointers — effectively building a mini compiler
- Overkill for a code review tool: the precision gain over Levels 0-3
  is marginal for review-time feedback

**Where it exists:** LLVM's `MemorySSA`, SVF (Static Value-Flow Analysis),
and academic tools like Doop and cclyzer implement this. If Prism ever
needs Level 4, integrating with LLVM IR analysis (run `clang -emit-llvm`
then query the IR) would be more practical than reimplementing.

---

## Summary

| Level | Pattern | Status | Precision | False Positives |
|-------|---------|--------|-----------|-----------------|
| 0 | `ptr->func()`, `obj.method()` | Implemented | Name-based | Same-name collisions |
| 1 | `fptr = func; fptr()` | Implemented | Last-assignment | Conditional reassignment |
| 2 | `table[i]()` with known init | Implemented | All entries | Over-approximation |
| 3 | `cb` passed as parameter | Not implemented | 1-hop callers | Multi-hop missed |
| 4 | Full points-to analysis | Not planned | Whole-program | N/A |

For a code review tool, Levels 0-2 provide the right trade-off: they catch
the common C/C++ dispatch patterns (struct ops tables, local function
pointers, static dispatch arrays) without requiring whole-program analysis.
The false positive rate is acceptable because showing a potential edge is
more useful than silently dropping it.
