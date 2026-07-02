# Prism C/C++ Expansion: Gap Analysis & Implementation Plan

### March 30, 2026

---

## 1. Current Implementation Status

### 1.1 Tree-Sitter Dependencies and Language Detection

**What's in place:**

`Cargo.toml` (lines 18–19) already pulls both C and C++ grammars:

```toml
tree-sitter-c = "0.24"
tree-sitter-cpp = "0.23"
```

`src/languages/mod.rs` (lines 23–26) detects the expected extensions:

```rust
"c" | "h" => Some(Self::C),
"cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Some(Self::Cpp),
```

`tree_sitter_language()` (lines 44–45) correctly wires both grammars.

**What's missing:**

- `.c` / `.h` extension detection does not disambiguate C from C++. A `device.hpp` is correctly
  identified as `Cpp`, but a `.h` file included by C++ code is treated as `Language::C` and parsed
  with the C grammar. Objective-C (`.m`, `.mm`) is silently undetected, which matters for macOS
  firmware tooling.
- No detection by shebang or `#pragma once` / `extern "C"` content heuristics. On embedded
  targets, a `.h` with `__cplusplus` guards is effectively C++, but Prism will parse it as C.
- `discover.py` does not exist anywhere in the repository. The expansion plan likely assumed a
  Python discovery/orchestration script, but none has been written.

---

### 1.2 Node Type Mappings in `src/languages/mod.rs`

**What's correct:**

| Feature | Coverage |
|---|---|
| `function_definition`, `template_declaration` as function nodes (line 70–71) | Present |
| Declarator chain navigation (`pointer_declarator`, `function_declarator`, `array_declarator`, `reference_declarator`) (lines 369–373) | Present |
| `qualified_identifier` for `Namespace::function` (lines 339–348) | Present |
| `destructor_name` for `~ClassName` (lines 349–358) | Present |
| `compound_statement` as scope block (line 273) | Present |
| `assignment_expression`, `update_expression` as assignments (lines 101–108) | Present |
| `declaration`, `init_declarator`, `field_declaration` as declarations (lines 127–134) | Present |
| `goto_statement` as control flow (line 233) | Present |

**What's absent:**

1. **Lambda expressions** — C++11 lambdas (`[capture](params) { body }`) produce a
   `lambda_expression` node in tree-sitter-cpp. `function_node_types()` (line 70–71) does not
   include `lambda_expression`, so lambdas are invisible to function-boundary logic, enclosing
   function lookup, and the call graph. Any diff inside a lambda body will fail `enclosing_function()`
   silently.

2. **Operator overloads** — An `operator delete` or `operator[]` definition is a
   `function_definition` with a `function_declarator` whose name is `operator_name`. The declarator
   chain walker in `find_identifier_in_c_declarator` (lines 335–383) returns `None` for
   `operator_name` nodes because it only handles `identifier` and `field_identifier`. The call graph
   will miss calls through overloaded operators.

3. **Constructor / destructor bodies** — Constructors and destructors are `function_definition`
   nodes, but constructor initializer lists (`mem_initializer_list`) are not traversed by variable
   reference finders, causing incomplete data flow for fields initialized in the ctor list.

4. **`const`, `volatile`, `virtual`, `override`, `noexcept` qualifiers** — These appear as
   sibling nodes adjacent to the function declarator. Prism's declarator navigation ignores them
   entirely. Downstream effects: `virtual` dispatch is invisible (see §3), and the absence of
   `noexcept` on changed functions cannot be detected.

5. **`for_range_loop`** — Listed in `is_control_flow_node` (line 230) but tree-sitter-cpp uses
   `for_range_statement` for range-based for. The mismatch means range-based-for is not recognized
   as a control flow boundary for condition extraction.

6. **`try_statement` / `catch_clause`** — Listed in `is_control_flow_node` (line 232) but C++
   exception handling (catch type, exception variable) is not surfaced to any algorithm. AbsenceSlice
   checks `lt.contains("try")` as a text substring (line 208) — this is fragile and misses
   `try { ... } catch(...)` blocks that span multiple lines.

---

### 1.3 Algorithm-Specific C/C++ Patterns

#### TaintSlice (`src/algorithms/taint.rs`)

`SINK_PATTERNS` (lines 15–38) contains: `exec`, `eval`, `system`, `popen`, `subprocess`, `query`,
`execute`, `raw_sql`, `cursor`, `open`, `write`, `unlink`, `remove`, `rmdir`, `send`, `respond`,
`render`, `redirect`, `innerHTML`, `dangerouslySetInnerHTML`, `Exec`, `Command`.

**Absent C/C++ sinks:**

| Missing Sink | Bug Class |
|---|---|
| `strcpy`, `strcat`, `sprintf`, `vsprintf` | Classic stack buffer overflow |
| `gets` | Always-unsafe input function (removed in C11, still encountered in firmware) |
| `scanf`, `fscanf`, `sscanf` with `%s` | Unbounded string read |
| `memcpy`, `memmove`, `memset` | Destination size not validated |
| `execv`, `execve`, `execvp`, `execlp` | Command injection via exec family |
| `recv`, `recvfrom`, `read` (POSIX) | Network/file data as untrusted source |
| `fgets` | Correct—but needs to mark output as tainted |
| `ioctl` with user buffer | Kernel I/O untrusted data path |
| `copy_from_user`, `get_user` | Linux kernel user-space data ingress |

None of these appear in `SINK_PATTERNS`. A taint analysis on `handle_snmp_set` in
`tests/integration_test.rs` (line 2243) — which calls `memcpy` with an unvalidated length — will
not fire a sink finding because `memcpy` is absent from the list.

#### ProvenanceSlice (`src/algorithms/provenance_slice.rs`)

`USER_INPUT_PATTERNS` (lines 70–89) covers HTTP/web idioms (`request`, `req.`, `body`, `query`,
`r.Body`) and `stdin`/`argv`. It does not cover:

```
// C/C++ system input patterns — all absent:
fgets, fread, read(), recv(), recvfrom(),
getchar(), fgetc(), scanf(), fscanf(),
getenv(),           // already in ENV_PATTERNS — correct
ioctl(),            // kernel driver input
copy_from_user(),   // Linux kernel user-space ingress
ntohs(), ntohl()    // network-byte-order conversion of received data
```

`ENV_PATTERNS` (lines 121–130) correctly includes `getenv(` and `os.Getenv`. `DATABASE_PATTERNS`
(lines 91–105) has no C equivalents (MySQL C API `mysql_query`, SQLite `sqlite3_exec`).

The `classify_line` function (lines 132–165) checks for literal assignment via text patterns like
`= "` and `= 0`. In C this produces false positives: `= (char *)0` (null pointer) will match
`= 0` but is not a constant value assignment for taint purposes.

#### AbsenceSlice (`src/algorithms/absence_slice.rs`)

`default_pairs()` (lines 28–104) contains a `malloc`/`free` pair (line 79–84):

```rust
PairedPattern {
    open_patterns: vec!["malloc(", "calloc(", "realloc(", "alloc(", "new "],
    close_patterns: vec!["free(", "dealloc(", "delete ", "release("],
    description: "allocation without free",
},
```

This is correct for C, but will generate **false positives for C++ RAII** patterns:

- `std::lock_guard<std::mutex> lock(mtx)` — destructor is automatic. Prism sees `lock(` matching
  the open pattern `.lock(` and looks for an `.unlock(` close. The `has_defer_or_finally` check
  (lines 202–212) matches `using ` (C# / Python context manager) but NOT `std::lock_guard`,
  `std::unique_lock`, `std::scoped_lock`, `std::unique_ptr`, or `std::shared_ptr`. These are the
  primary C++ RAII patterns and all of them will generate spurious "lock without unlock" or
  "allocation without free" findings.

- The `DeviceManager` C++ fixture in `tests/integration_test.rs` (line 1396) uses
  `std::lock_guard<std::mutex> lock(mtx)` in `add_device`. AbsenceSlice will flag this as a missing
  `unlock(` call, which is incorrect — `lock_guard` automatically unlocks on destruction.

There are also no C-specific paired patterns for:
- `pthread_mutex_lock` / `pthread_mutex_unlock`
- `sem_wait` / `sem_post`
- `mmap` / `munmap`
- `open(fd)` / `close(fd)` (POSIX file descriptors, distinct from C++ stream `close()`)
- `kmalloc` / `kfree` (Linux kernel)
- `spin_lock` / `spin_unlock`

#### QuantumSlice (`src/algorithms/quantum_slice.rs`)

`is_async_function()` for `Language::C` (lines 247–255) checks the full function text for
`pthread_create`, `fork(`, `signal(`, `sigaction(`. This is text substring matching on the entire
function body — a correct approach in spirit but crude in practice.

`is_async_function()` for `Language::Cpp` (lines 256–266) checks for `std::async`, `std::thread`,
`std::jthread`, `co_await`, `co_yield`, `pthread_create`. This is reasonable.

**Key gap:** `find_async_inner()` for `Language::C` (lines 191–199) fires on `kind == "call_expression"`
whose text contains the pattern strings. However, C coroutines (`longjmp`/`setjmp`-based state
machines) and interrupt service routines (ISR functions registered via `request_irq` or
`DECLARE_TASKLET`) are not detected. More critically:

- A function that is *itself* a signal handler or ISR (i.e., it IS the async entry point) will
  not be detected by `is_async_function()` because it does not *call* `signal()` — it only needs
  to be registered externally. The diff to the handler body will produce no `QuantumSlice` output
  at all even though the function executes asynchronously.

---

### 1.4 Test Coverage

**C/C++ tests that exist** (as of current integration test file):

| Test fixture | Algorithm(s) exercised | Bug class |
|---|---|---|
| `make_c_test` (src/device.c) | C parsing, call graph, all standard algorithms | Generic C function structure |
| `make_c_multifile_test` (device.c + handler.c) | Cross-file call graph | Cross-TU linkage |
| `make_cpp_test` (device_manager.cpp) | C++ class methods, mutex, templates | C++ OOP basics |
| `make_snmp_overflow_test` | OriginalDiff, ParentFunction, LeftFlow, ThinSlice | SNMP buffer overflow |
| `make_onu_state_machine_test` | OriginalDiff, LeftFlow | ONU PLOAM state machine |
| `make_double_free_test` | Appears in fixture list | double-free with goto |
| `make_ring_overflow_test` | Appears in fixture list | Ring buffer overflow |
| `make_timer_uaf_test` | Appears in fixture list | Use-after-free |
| `make_large_function_test` | Stress/perf | Large C function |
| `make_macro_heavy_test` | Stress | Macro-heavy C |
| `make_deep_switch_test` | Stress | Deep switch/state machine |
| `make_c_threaded_test` (src/threaded.c) | QuantumSlice | pthread concurrency |
| `make_cpp_async_test` (src/config.cpp) | QuantumSlice | std::thread |

**C/C++ tests that do NOT exist:**

| Missing fixture | Why it matters |
|---|---|
| CVE-pattern fixtures (format string, heap spray, integer overflow) | Firmware contains historically exploited vulnerability classes |
| Kernel-style code (`kmalloc`/`kfree`, spinlock, RCU, DMA) | Modem/OLT drivers are kernel modules |
| C++ RAII false-positive regression | `lock_guard` should NOT trigger AbsenceSlice |
| Virtual dispatch / vtable fixture | C++ polymorphism invisible to call graph |
| Function pointer registration fixture (`ops->open = my_open`) | Kernel and DPDK-style driver idioms |
| `goto`-based error path fixture | Standard Linux kernel error style; double_free fixture exists but is not tested against AbsenceSlice |
| Template instantiation fixture | Template functions need call graph coverage |
| `va_list` / variadic functions | `printf`, `snprintf`, custom logging — taint through `...` args |

**`discover.py` status:** The file does not exist anywhere in the repository tree. If the expansion
plan referenced a Python discovery script for enumerating source files to analyze, it remains
entirely unimplemented.

---

## 2. Critical Gaps (Will Cause Failures)

### 2.1 No C/C++ Taint Sinks

**Evidence:** `taint.rs` lines 15–38. The `SINK_PATTERNS` slice does not include `strcpy`,
`sprintf`, `gets`, `scanf`, `execv`, `memcpy`, `recv`, `read`, or any POSIX/C-stdlib buffer
function.

**Consequence:** Given the SNMP fixture:

```c
void handle_snmp_set(uint8_t *pdu, size_t pdu_len) {
    char community[64];
    size_t community_len = pdu[7];        // diff line 7 — tainted
    memcpy(community, pdu + 8, community_len);  // diff line 8 — sink, but NOT recognized
```

TaintSlice will propagate taint from line 7 forward but will never fire a finding because `memcpy`
is not in `SINK_PATTERNS`. The taint analysis is **silent** on the most common firmware buffer
overflow class.

**Fix required:** Extend `SINK_PATTERNS` with at minimum:
`strcpy`, `strncpy`, `strcat`, `strncat`, `sprintf`, `snprintf`, `vsprintf`, `vsnprintf`,
`gets`, `scanf`, `fscanf`, `sscanf`, `memcpy`, `memmove`, `memset`, `recv`, `recvfrom`,
`read`, `fread`, `execv`, `execve`, `execvp`, `execl`, `execlp`, `system`, `popen`.

---

### 2.2 No C/C++ Provenance Sources

**Evidence:** `provenance_slice.rs` lines 70–89. `USER_INPUT_PATTERNS` has no `fgets`, `fread`,
`read`, `recv`, `recvfrom`, `scanf`, or socket-receive idioms.

**Consequence:** In `handle_snmp_set`, `pdu` is a function parameter carrying network data. The
parameter heuristic (lines 248–261) will classify it as `FunctionParam` — risk level "MEDIUM —
depends on caller" — which is technically correct but unhelpful. The root origin (network socket
`recv()` call upstream) will never be traced. More critically, if the incoming data is assigned
via `fgets(buf, sizeof(buf), stdin)`, ProvenanceSlice will classify it as `Unknown` because
`fgets` does not appear in any pattern list.

---

### 2.3 AbsenceSlice False Positives on C++ RAII

**Evidence:** `absence_slice.rs` lines 36–43 (lock pair), lines 202–212 (cleanup check).

The cleanup heuristic checks for `defer `, `finally`, `with `, and `using ` keywords. The C++
RAII guard types (`std::lock_guard`, `std::unique_lock`, `std::scoped_lock`, `std::unique_ptr`,
`std::shared_ptr`) are not in this list. Any C++ code using standard RAII locking will produce
spurious "lock without unlock" findings.

Concretely, the existing C++ test fixture `make_cpp_test` (integration_test.rs line 1396) uses:

```cpp
std::lock_guard<std::mutex> lock(mtx);
```

AbsenceSlice will match `.lock(` in `lock(mtx)` as the open pattern, scan the function body for
`.unlock(` and not find it (because unlock is automatic via RAII), and emit a "lock without unlock"
warning. This is a **guaranteed false positive** against existing test code.

**Fix required:** The `has_defer_or_finally` check must be extended to recognize C++ RAII type
names in the function scope: if any `std::lock_guard`, `std::unique_lock`, `std::scoped_lock`,
`std::unique_ptr`, `std::shared_ptr`, or `std::make_unique` appears in the function text, the
lock and allocation pair checks should be suppressed.

---

### 2.4 QuantumSlice `is_async_function` Unreliability for C/C++

**Evidence:** `quantum_slice.rs` lines 247–266.

`is_async_function()` for `Language::C` does full-text substring matching on the function body.
This approach works only if the async primitive is *called inside* the function. A function that
IS a signal handler (registered externally by `signal(SIGTERM, my_handler)`) will not be detected
as async even though it can be interrupted at any time and every shared-variable access within it
has race conditions.

Similarly, C++ coroutines (`co_await`, `co_yield`) are detected by text search on the whole
function node (line 263), but `co_await` expressions inside lambdas within the function are not
detected because `is_async_function` operates at function scope, not recursively through nested
lambdas.

---

### 2.5 No Kernel Patterns

No algorithm contains any of the following Linux kernel primitives that appear routinely in modem,
ONU, and CMTS firmware:

| Pattern class | Specific symbols |
|---|---|
| Memory | `kmalloc`, `kzalloc`, `kfree`, `vmalloc`, `vfree`, `devm_kzalloc` |
| Locking | `spin_lock`, `spin_unlock`, `spin_lock_irqsave`, `spin_unlock_irqrestore`, `mutex_lock`, `mutex_unlock`, `down`, `up` |
| RCU | `rcu_read_lock`, `rcu_read_unlock`, `synchronize_rcu`, `call_rcu` |
| DMA | `dma_alloc_coherent`, `dma_free_coherent`, `dma_map_single`, `dma_unmap_single` |
| IRQ | `request_irq`, `free_irq`, `disable_irq`, `enable_irq` |
| Reference counting | `kref_init`, `kref_get`, `kref_put`, `atomic_inc`, `atomic_dec_and_test` |
| Slab/memory pool | `kmem_cache_alloc`, `kmem_cache_free` |

AbsenceSlice will miss every kernel lock/memory pair. TaintSlice will miss kernel user-space data
ingress (`copy_from_user`, `get_user`). ProvenanceSlice has no concept of kernel-side data origin.

---

## 3. Gaps NOT in the Expansion Plan

These issues exist independently of whatever was documented in the expansion plan and will affect
C/C++ analysis quality.

### 3.1 Pointer Aliasing Invisible to the Data Flow Graph

`data_flow.rs` tracks variables by **name** (lines 74–84). In C/C++, two different names can
alias the same memory:

```c
device_t *dev = create_device(input, 42);
device_t *alias = dev;
free(alias->name);  // data_flow.rs tracks "alias" not "dev"
```

The DFG will not propagate taint from `dev` to `alias->name` because they are tracked as separate
names. The call `free(alias->name)` will not appear as a use of `dev`. This breaks TaintSlice,
ProvenanceSlice, LeftFlow, and ThinSlice for pointer-aliased code, which is the norm in C.

### 3.2 Function Pointer Calls Not in the Call Graph

`call_graph.rs` (lines 78–90) resolves calls via `parsed.function_calls_on_lines()` which uses
tree-sitter identifiers. A call through a function pointer:

```c
struct ops {
    int (*open)(struct inode *, struct file *);
};
ops->open(inode, file);   // call_expression, but function name is "open" via field access
timer->callback(timer->data);  // timer_uaf.c line 18 — callback via struct field
```

The field-access call (`timer->callback(timer->data)`) will appear in the AST as a
`call_expression` whose function child is a `field_expression`, not an `identifier`. The call
graph builder extracts the function name via `call_function_name()` (`languages/mod.rs` line 257)
which calls `node.child_by_field_name("function")`. For a field-expression call, this returns the
entire `timer->callback` expression, which will not match any `FunctionId.name` in the graph.

**Consequences:** MembraneSlice, CircularSlice, VerticalSlice, and BarrierSlice all lose edges
through function pointers. In the `timer_uaf.c` fixture, the `callback` invocation on line 18 is
the use-after-free site, but CircularSlice will not find a cycle because the edge is missing.

### 3.3 Struct/Union Field Flow Not Tracked

`data_flow.rs` tracks by variable name. `dev->id` and `dev->name` are both represented as
the identifier `dev` (or as separate identifiers `id`/`name` depending on which tree-sitter node
is extracted). Field-level tracking does not exist.

This means:

- If `dev->name` is tainted (via `strdup(user_input)`), TaintSlice will taint `dev` — but all
  subsequent reads of `dev->id`, `dev->active`, and every other field will also appear tainted,
  producing false positives.
- Conversely, if only `dev->id` is modified in a diff, ProvenanceSlice will trace the origin of
  `dev` (the allocation site) rather than the origin of `id` specifically.

### 3.4 ERROR Nodes from Macro-Heavy Code Not Surfaced to Users

Tree-sitter produces `ERROR` nodes when it cannot parse a construct. In macro-heavy C firmware,
complex macros (container_of, list_for_each_entry, BUILD_BUG_ON) expand to syntactically valid C
but often leave intermediate forms that confuse the parser. Prism never checks for `ERROR` nodes
in the AST and will silently produce incomplete analysis over code regions that failed to parse.
Users have no indication that analysis quality is degraded.

### 3.5 Virtual Dispatch Missing from C++ Call Graph

The call graph in `call_graph.rs` resolves callee names to `FunctionId` by exact name match
(`self.functions.get(&site.callee_name)`). For a virtual call:

```cpp
DeviceBase *dev = get_device(id);
dev->process();    // call_expression, callee is "process"
```

The call graph will look up `"process"` and find it — but it will find ALL functions named
`process` across ALL files and classes. For a class hierarchy with `DeviceA::process`,
`DeviceB::process`, `DeviceC::process`, it will add edges to all three, creating false positive
cycle detections and inflated membrane analysis. More commonly, for calls through base class
pointers to overridden methods, the graph will either find nothing (if the base `process` is pure
virtual) or find the wrong implementation.

### 3.6 `va_list` Trio Not Tracked

Variadic functions (`printf`, `snprintf`, custom `log_message(const char *fmt, ...)`) pass
arguments through `va_list`. Taint propagated into the format string argument is effectively
invisible beyond the call site because the DFG tracks only named variable assignments. A format
string injection via:

```c
char msg[256];
snprintf(msg, sizeof(msg), user_input);  // format string bug
```

...will not be caught because `user_input` flows into `snprintf`'s format argument, but `snprintf`
is not in `SINK_PATTERNS`, and even if it were, the taint path from `user_input` to the `snprintf`
format parameter is through positional arguments that the DFG does not model.

### 3.7 `goto`-Based Error Handling Patterns

The Linux kernel and most embedded C codebases use `goto cleanup` patterns for error handling.
The `double_free.c` fixture (`tests/integration_test.rs` line 2322) demonstrates this:

```c
if (validate_header(frame) < 0) {
    free(frame->payload);
    free(frame);
    goto cleanup;    // jumps to label
}
// ...
cleanup:
    free(frame->payload);  // double-free: already freed above
    free(frame);
```

AbsenceSlice is not designed to analyze `goto` targets. It finds `free(` matching the `close`
pattern in the function scope and concludes that `malloc` has a corresponding `free` — correct
from a naive scan, but missing the double-free in the `goto` error path. There is no test
asserting that `AbsenceSlice` correctly identifies the double-free scenario.

### 3.8 Integer Promotion and Implicit Conversion Bugs Not Modeled

C/C++ integer promotion rules are a major source of firmware security bugs. A
`size_t community_len = pdu[7]` followed by `memcpy(dst, src, community_len)` appears safe unless
one notes that `pdu[7]` is a `uint8_t` (0–255) being used as a copy length into a 64-byte buffer
— a potential overflow. Prism has no numeric range or type tracking. The DFG tracks name-level
flow but not type or value range.

---

## 4. Algorithm Methodology Evaluation

### 4.1 TaintSlice

**What it actually does** (`taint.rs` lines 73–181): Builds a `DataFlowGraph`, identifies taint
sources from diff lines or explicit source locations, calls `dfg.taint_forward()` (BFS through
def-use edges), then checks all reachable identifier names at use-sites against `SINK_PATTERNS`
by substring match.

**Mechanism:** Genuine def-use chain analysis (intraprocedural), backed by real AST node
extraction. Interprocedural propagation is absent — taint stops at function call boundaries.

**Genuine/Pattern-matching:** Genuine intraprocedural DFG traversal; pattern-matching at sink
detection.

**C/C++ readiness:** Not ready. Missing all C/C++ sinks (§2.1). Missing C/C++ source patterns
(§2.2). Will not fire on the primary C firmware vulnerability classes.

**Improvements needed:** Add C/C++ sink list. Add C/C++ source patterns to ProvenanceSlice and
expose them as taint sources. Add interprocedural propagation for at least one-hop callees.

---

### 4.2 AbsenceSlice

**What it actually does** (`absence_slice.rs` lines 131–255): For each diff line, checks whether
a known "open" pattern (call name via AST) appears. If so, scans the enclosing function's entire
body for the matching "close" pattern. Emits a finding if neither the close pattern nor a
`defer/finally/with/using` cleanup construct is found.

**Mechanism:** AST-backed call name extraction for call patterns; text substring fallback for
keyword patterns (`new `, `defer `). The pairing logic itself is structural (function-scoped), not
a simple text scan.

**Genuine/Pattern-matching:** Hybrid — AST call name lookup plus pattern string matching.

**C/C++ readiness:** Partially ready for C (malloc/free pair works). Broken for C++ RAII (false
positives on `lock_guard`). Missing kernel lock pairs entirely.

**Improvements needed:** RAII guard type recognition; kernel lock pair patterns; `goto`-aware
cleanup path analysis.

---

### 4.3 QuantumSlice

**What it actually does** (`quantum_slice.rs` lines 27–154): Finds all assignments to variables
on diff lines within the enclosing function, detects async context via language-specific node
patterns, and labels each assignment as "async-dependent" or "synchronous." Only emits output if
at least one assignment is async-dependent.

**Mechanism:** Pattern matching. The async detection (`find_async_inner`, lines 164–221) is
entirely a name/text substring check — it does not analyze data flow to or from the async
primitives. The "superposition" concept is a labeling scheme, not actual state enumeration.

**Genuine/Pattern-matching:** Predominantly pattern-matching with structural framing.

**C/C++ readiness:** Partial. `pthread_create`, `fork`, `signal`, `std::thread`, `std::async`
are detected. ISR functions and signal handlers that are themselves the async entry point are not.
C++ coroutines (`co_await`) detected by text search, which misses coroutines in nested lambdas.

**Improvements needed:** ISR/signal-handler self-detection; coroutine-in-lambda detection;
`longjmp`/`setjmp` state machine detection.

---

### 4.4 ProvenanceSlice

**What it actually does** (`provenance_slice.rs` lines 179–322): For each identifier on a diff
line, performs backward DFG traversal to find definition sites, then classifies the definition
line's text against `USER_INPUT_PATTERNS`, `DATABASE_PATTERNS`, `CONFIG_PATTERNS`, and
`ENV_PATTERNS` via substring matching.

**Mechanism:** Genuine backward DFG traversal to reach definition sites; pattern matching for
origin classification. The classification is text-based and can be defeated by variable renaming
or indirection.

**Genuine/Pattern-matching:** Genuine backward traversal; pattern-matching for classification.

**C/C++ readiness:** Not ready. Missing all C/C++ input sources (§2.2). The `classify_line`
function has C-specific false positives on null pointer assignments (§1.3).

**Improvements needed:** C/C++ network/file input patterns; kernel user-space ingress patterns;
null-pointer assignment false positive fix.

---

### 4.5 SymmetrySlice

**What it actually does** (`symmetry_slice.rs` lines 157–264): Identifies which function name
in the diff has a symmetric counterpart (e.g., `encode` → look for `decode`). Searches all parsed
files for that counterpart by name. If found and not in the diff, emits a "broken symmetry"
finding.

**Mechanism:** Function name pattern matching (string contains check). Counterpart name generation
is lowercase replacement of the matching substring, losing original casing.

**Genuine/Pattern-matching:** Pure pattern-matching on function names. The semantic insight is
in the curated list, not in any code analysis.

**C/C++ readiness:** Works for C/C++ as well as for any other language — it only needs function
names to be parseable, which they are. The symmetric pairs cover generic patterns that apply to C
(serialize/deserialize, encode/decode). C-specific pairs like `htons`/`ntohs` or `htobe32`/
`be32toh` are absent but can be added.

**Improvements needed:** Add network byte-order conversion pairs; add crypto-specific pairs
(`AES_encrypt`/`AES_decrypt`). Case-sensitive matching would improve precision.

---

### 4.6 CircularSlice

**What it actually does** (`circular_slice.rs` lines 17–117): Builds the call graph, finds
functions containing diff lines, runs DFS cycle detection from those functions, and emits
blocks for each cycle. Also does a secondary check for data flow self-cycles on diff lines.

**Mechanism:** Genuine DFS cycle detection on a call graph. The call graph itself has the
limitations described in §3.2 (function pointer calls missing).

**Genuine/Pattern-matching:** Genuine graph algorithm on a potentially incomplete graph.

**C/C++ readiness:** Works for C (direct function calls). Broken for C++ virtual dispatch and
function pointer calls. `find_cycles_from` (call_graph.rs line 198) deduplicates by function
name, not by (file, name) pair — so two different classes with a method named `process` will
falsely appear to call each other if they both happen to call a third function also named
`process`.

**Improvements needed:** Function pointer edge resolution; name disambiguation by file/class scope.

---

### 4.7 PhantomSlice

**What it actually does** (`phantom_slice.rs` lines 44–98): Runs `git log --diff-filter=D` to
find recently deleted files, then heuristically extracts function names from deleted file content
using language-specific prefix patterns (`def `, `function `, `func `, Java visibility modifiers).

**Mechanism:** Git history query plus text pattern extraction. The function-name extraction
(`extract_function_name`, lines 166–215) has no C/C++ case.

**Genuine/Pattern-matching:** Git-backed, but function name extraction is pattern-matching and
incomplete.

**C/C++ readiness:** No C/C++ support. `extract_function_name` handles Python (`def`), JavaScript
(`function`), Go (`func`), and Java (visibility modifiers). A deleted C function like
`static int process_packet(...)` will not be extracted. Deleted C++ class methods will not be
extracted.

**Improvements needed:** Add C/C++ function definition detection to `extract_function_name` —
at minimum, heuristically detect `[type] identifier(` patterns after filtering out common
non-function keywords.

---

### 4.8 MembraneSlice

**What it actually does** (`membrane_slice.rs` lines 20–144): Builds the call graph, finds
functions modified by the diff, looks for callers in other files, and checks whether those
callers have error handling keywords (`try`, `catch`, `except`, `if err`, `if error`, `.catch(`).

**Mechanism:** Genuine cross-file call graph traversal. Error handling detection is text
substring matching.

**Genuine/Pattern-matching:** Genuine for call graph traversal; pattern-matching for error
handling detection.

**C/C++ readiness:** Structural analysis works for C/C++ direct calls. Function pointer calls
are missed (§3.2). Error handling detection is broken for C: the canonical C error handling
idiom is `if (ret < 0)` or `if (ret == NULL)`, neither of which matches any of the listed
patterns (`try`, `catch`, `except`, `if err`, `if error`). Every C cross-file caller will
falsely appear to have no error handling, generating spurious "unprotected caller" findings.

**Improvements needed:** Add C/C++ error handling patterns: `if (ret < 0)`, `if (ret == -1)`,
`if (ptr == NULL)`, `if (!ptr)`, `assert(`, `errno`, `perror(`, `ASSERT_`, `CHECK_`.

---

### 4.9 ResonanceSlice

**What it actually does** (`resonance_slice.rs` lines 57–128): Queries `git log` for commit
history, builds file co-change frequency counts from `git diff-tree`, and reports files that
historically co-change with the current diff but are absent from it.

**Mechanism:** Genuine git repository mining. Entirely language-agnostic — operates on file
paths, not code.

**Genuine/Pattern-matching:** Genuine statistical co-change analysis.

**C/C++ readiness:** Fully ready. No language-specific code involved. Skips merge commits
(line 174, `MAX_FILES_PER_COMMIT = 50` threshold). O(n²) pair counting over files per commit
(lines 179–192) is bounded by the 50-file cap.

**Improvements needed:** None specific to C/C++. General improvements: configurable file path
normalization (absolute vs. relative), filtering of generated/vendored files, confidence interval
reporting.

---

### 4.10 GradientSlice

**What it actually does** (`gradient_slice.rs` lines 51–195): BFS from diff lines through both
data flow (lvalue/rvalue references within functions) and call graph (callers/callees), assigning
decaying scores by hop count. Collects all (file, line) pairs above a threshold.

**Mechanism:** Genuine BFS traversal over DFG + call graph edges, with configurable decay.

**Genuine/Pattern-matching:** Genuine graph traversal; no language-specific patterns.

**C/C++ readiness:** Works structurally. Inherits all DFG limitations (no aliasing, no field
tracking) and call graph limitations (no function pointers, no virtual dispatch). For large C
files with many variables, the BFS can expand aggressively since every variable reference is a
potential hop.

**Improvements needed:** Pruning heuristics for C pointer variables (avoid treating every
dereference as a data flow edge). Performance monitoring for large files (see §6).

---

### 4.11 ThreeDSlice

**What it actually does** (`threed_slice.rs` lines 55–~150): Computes a risk score for each
function in the diff as `structural_coupling * temporal_activity * change_complexity`. Structural
coupling is callers + callees count. Temporal activity is git commit frequency for the file.
Change complexity is diff line count within the function.

**Mechanism:** Genuine multi-dimensional scoring. Git query is real; call graph is real but
incomplete.

**Genuine/Pattern-matching:** Genuine composite metric.

**C/C++ readiness:** Works for C/C++ with the same call graph limitations. File-level churn
(not function-level) means a frequently-modified file gives every function in it the same
temporal score regardless of which function actually changes most.

**Improvements needed:** Function-level git churn (`git log -L :function_name:file.c`); virtual
dispatch edges; function pointer edges.

---

### Summary Table

| Algorithm | Mechanism | Genuine / Pattern-matching | C/C++ Readiness |
|---|---|---|---|
| TaintSlice | Def-use BFS + sink name matching | Genuine DFG + pattern sinks | **Not ready** — missing all C/C++ sinks |
| AbsenceSlice | AST call lookup + function scope scan | Hybrid AST + pattern | **Broken for C++** — RAII false positives |
| QuantumSlice | Async node detection + assignment labeling | Mostly pattern | **Partial** — misses ISR/signal handler self-detection |
| ProvenanceSlice | Backward DFG + origin classification | Genuine traversal + pattern | **Not ready** — missing C/C++ input sources |
| SymmetrySlice | Function name substring matching | Pure pattern | **Ready** — language-agnostic |
| CircularSlice | DFS call graph cycle detection | Genuine graph | **Partial** — misses function pointers, virtual dispatch |
| PhantomSlice | Git history + text function extraction | Git + pattern | **Not ready** — no C/C++ function extraction |
| MembraneSlice | Cross-file call graph + error keyword scan | Genuine graph + pattern | **Broken for C** — error handling detection misses C idioms |
| ResonanceSlice | Git co-change frequency | Genuine statistical | **Fully ready** — language-agnostic |
| GradientSlice | BFS DFG + call graph with decay | Genuine BFS | **Mostly ready** — inherits DFG/call graph gaps |
| ThreeDSlice | Multi-metric risk score | Genuine composite | **Mostly ready** — file-level churn is too coarse |

---

## 5. Firmware Language Gap Analysis

Prism currently supports: Python, JavaScript, TypeScript, Go, Java, C, C++. For access network
firmware/embedded infrastructure (DOCSIS modems, GPON ONUs, CMTS platforms, OLTs, managed
switches), additional languages are relevant.

### 5.1 Rust

**Relevance to access network infrastructure:**
Rust is actively entering systems programming. The Linux kernel has official Rust support since
6.1 (December 2022). Google, Microsoft, and the Android team are migrating security-critical
kernel components to Rust. New CMTS control-plane components and network daemon rewrites are
increasingly written in Rust. Firmware teams are likely to encounter or adopt Rust
in new kernel driver work within a 12–24 month horizon.

**Tree-sitter support:** `tree-sitter-rust` is mature, actively maintained, and handles the
full Rust grammar including lifetimes, traits, macros, and async/await. The crate
`tree-sitter-rust` is available on crates.io.

**Which Prism algorithms provide value:**
- TaintSlice: `unsafe` blocks are explicit taint boundaries; `*mut T` dereferences in unsafe
  blocks are the primary safety concern
- AbsenceSlice: `lock()` without explicit drop / scope boundary is a real concern in Rust when
  using raw `Mutex::lock()` returns
- MembraneSlice: `pub` vs. `pub(crate)` boundary analysis is directly meaningful
- CircularSlice: Rust's ownership prevents some cycles but `Rc<RefCell<T>>` cycles are a real
  memory leak class
- ProvenanceSlice: `std::io::stdin()`, `std::env::args()` as input origins

**Priority: Must-have.** Rust is the designated successor to C for kernel and safety-critical
firmware. As teams migrate new drivers and daemons to Rust, absence of Rust support means
Prism cannot analyze the safest components of the new firmware stack.

---

### 5.2 Assembly (ARM, MIPS, x86)

**Relevance to access network infrastructure:**
Bootloaders (U-Boot, cBoot) contain ARM or MIPS assembly for CPU initialization, exception
vectors, and cache management. Interrupt vector tables are pure assembly. Inline assembly in C
drivers is common for MIPS/ARM DSP operations and memory-mapped I/O. DOCSIS MAC-layer code
on Broadcom/Cavium platforms has historically used handwritten MIPS assembly for performance.

**Tree-sitter support:** `tree-sitter-asm` and various architecture-specific grammars exist but
are less mature. ARM assembly (`tree-sitter-asm`) handles the basic instruction structure.
MIPS grammars are less maintained.

**Which Prism algorithms provide value:**
Structural analysis provides limited value for assembly. However, OriginalDiff and ParentFunction
(showing surrounding context) have value for assembly diffs. Call graph analysis for assembly
would require resolving jump targets, which is an undecidable problem in general.

**Assessment:** Assembly analysis at the structural/data-flow level that Prism implements is not
practical. The value would be limited to diff-context display (OriginalDiff, ParentFunction),
which does not require language-specific parsing beyond basic line tokenization.

**Priority: Skip** for algorithmic analysis. Consider a lightweight "display mode" that shows
assembly diff context without attempting call graph or data flow. Full assembly analysis is a
separate specialized tool domain.

---

### 5.3 Device Tree Source (`.dts` / `.dtsi`)

**Relevance to access network infrastructure:**
Every Linux-based modem, ONU, and gateway has a device tree that describes hardware topology:
interrupt mappings, memory-mapped register ranges, I2C/SPI bus configurations, Ethernet PHY
addresses. A wrong value in the device tree (e.g., wrong IRQ number, wrong memory address) causes
hardware initialization failure or kernel panics. DTS changes in firmware updates are a known
source of regressions.

**Tree-sitter support:** No mature tree-sitter grammar for DTS/DTSI exists. DTS is a structured
key-value format with `compatible` strings and `reg` properties. It is not a general-purpose
programming language.

**Which Prism algorithms provide value:**
SymmetrySlice (verifying that add_device and compatible string changes are paired) and
AbsenceSlice (interrupt-parent without interrupt specifier) could provide some value. However,
the data model is fundamentally different from procedural code — DTS analysis requires
hardware topology reasoning, not def-use chains.

**Priority: Skip** for this release. DTS changes are better served by a dedicated DTS-aware
linter (e.g., dtc warnings, dt-validate) than by a general code slicing tool.

---

### 5.4 Linker Scripts (`.ld` / `.lds`)

**Relevance to access network infrastructure:**
Linker scripts define memory layout: section placement, stack/heap boundaries, BSS initialization.
An incorrect `MEMORY` region size causes stack overflow at runtime. A changed section address
causes boot failure if it conflicts with ROM mapping. Linker script bugs are catastrophic and
difficult to debug.

**Tree-sitter support:** No tree-sitter grammar for GNU linker script syntax exists.

**Priority: Skip.** Linker script review is better handled by build system validation (comparing
`MEMORY` region sizes against hardware specs, checking alignment constraints). This is outside
Prism's analysis model.

---

### 5.5 Lua

**Relevance to access network infrastructure:**
OpenWrt's LuCI web interface (used on home gateways and some ONUs) is entirely Lua. Configuration
management scripts, device management pages, and firmware update UIs are implemented in Lua.
Lua security bugs in LuCI are a real attack surface — command injection through `os.execute()`,
path traversal in file access, and authentication bypasses have appeared in the CVE database.

**Tree-sitter support:** `tree-sitter-lua` is mature and actively maintained. Available on
crates.io as `tree-sitter-lua`.

**Which Prism algorithms provide value:**
- TaintSlice: `os.execute()`, `io.popen()` are sinks; `ngx.req.*`, form POST body are sources
- ProvenanceSlice: `ngx.var.*`, form field access as user input origin
- MembraneSlice: Module `require` boundaries
- AbsenceSlice: `io.open` without `file:close()`
- SymmetrySlice: `encode`/`decode`, `serialize`/`deserialize` pairs

**Priority: Should-have.** LuCI is a direct attack surface for home gateways. TaintSlice would
provide immediate value for Lua command injection detection.

---

### 5.6 Shell / Bash

**Relevance to access network infrastructure:**
Init scripts (`/etc/init.d/`), firmware update scripts (`sysupgrade`, `fw_update`), factory
provisioning scripts, and build automation are all shell/bash. Bugs in update scripts can brick
devices. Command injection through unquoted variables in shell scripts is a common class. Shell
scripts also set security-sensitive configurations (firewall rules, permissions).

**Tree-sitter support:** `tree-sitter-bash` is mature, handles most bash constructs, and is
available as `tree-sitter-bash` on crates.io.

**Which Prism algorithms provide value:**
- TaintSlice: `$1`, `$@`, `$INPUT` as sources; `eval`, `exec`, backtick execution as sinks.
  This is the most impactful use case — shell command injection is trivial to introduce and
  hard to audit manually.
- AbsenceSlice: `trap` for cleanup signal handling
- ProvenanceSlice: Script argument origin classification

**Priority: Should-have.** Shell command injection in firmware update scripts is a high-value
target. `tree-sitter-bash` support is mature. TaintSlice for shell would catch a significant
class of firmware update vulnerabilities.

---

### 5.7 Makefiles / CMake

**Relevance to access network infrastructure:**
Build system files control compiler flags, security hardening options, and library linkage.
Missing `-D_FORTIFY_SOURCE=2`, wrong `-fstack-protector` flag, or linking against a vulnerable
library version are all build-system-level security issues. CMake is increasingly used in
embedded projects (Zephyr RTOS, ESP-IDF, DPDK).

**Tree-sitter support:** `tree-sitter-make` exists (Makefile grammar). `tree-sitter-cmake` is
available and reasonably mature.

**Which Prism algorithms provide value:**
SymmetrySlice (if a Makefile adds a library, the corresponding removal should be present). Most
other algorithms are not meaningful for Makefiles.

**Assessment:** Makefile/CMake analysis as a security tool is better served by specific flag
auditing tools (e.g., checking for missing `-fstack-protector`, `-fPIE`, `-D_FORTIFY_SOURCE`).
Prism's current algorithm set does not provide the right analysis primitives for build files.

**Priority: Skip.** Flag audit tooling is more appropriate. Prism's slice-based analysis does
not map well to build file semantics.

---

### 5.8 YANG / NETCONF Models

**Relevance to access network infrastructure:**
CMTS and OLT devices are managed via NETCONF/YANG. The YANG data model defines the configuration
schema. Changes to YANG models affect configuration validation, default values, and constraint
enforcement. An incorrect `must` constraint or missing `min-elements` check in a YANG model can
cause configuration acceptance of invalid states.

**Tree-sitter support:** No tree-sitter grammar for YANG exists.

**Priority: Skip.** YANG model review requires domain-specific schema validation tools, not
code slicing. Tools like `pyang` and `yanglint` are the appropriate validators.

---

### 5.9 Protocol Buffers / FlatBuffers

**Relevance to access network infrastructure:**
Modern firmware IPC (between management daemon, data-plane agent, and hardware abstraction
layer) increasingly uses Protocol Buffers or FlatBuffers for structured messaging. Schema changes
break wire compatibility between components that may be on different update schedules.

**Tree-sitter support:** `tree-sitter-proto` exists for Protocol Buffers. No mature grammar for
FlatBuffers schema files.

**Which Prism algorithms provide value:**
SymmetrySlice is the most natural fit: if a message field is added on the producer side, the
consumer schema should also be updated. However, schema analysis requires understanding semantic
versioning and field numbering, not just function names.

**Priority: Nice-to-have.** SymmetrySlice on `.proto` files could catch proto schema
asymmetries. Worth implementing for gRPC/protobuf control-plane IPC.

---

### Firmware Language Priority Summary

| Language | Relevance | tree-sitter maturity | Recommended Priority |
|---|---|---|---|
| **Rust** | High — kernel drivers, safety-critical daemons | Mature | **Must-have** |
| **Shell/Bash** | High — update scripts, init, provisioning | Mature | **Should-have** |
| **Lua** | Medium-high — LuCI web interface on gateways | Mature | **Should-have** |
| **Protocol Buffers** | Medium — IPC schema versioning | Available | **Nice-to-have** |
| **CMake/Make** | Medium — build flags, security hardening | Available | **Skip** (use flag audit tools) |
| **Assembly** | Low for algorithmic analysis | Immature | **Skip** (display-only mode) |
| **Device Tree (DTS)** | High operationally — hardware init | None | **Skip** (use dtc/dt-validate) |
| **Linker Scripts** | Medium — memory layout | None | **Skip** (build system validation) |
| **YANG/NETCONF** | Medium for config validation | None | **Skip** (use pyang/yanglint) |

---

## 6. Hardening Recommendations

### 6.1 ERROR Node Detection and Reporting

Tree-sitter continues parsing after encountering syntax errors and inserts `ERROR` nodes into the
AST. Prism currently performs no error node detection anywhere in the codebase. Macro-heavy C
code (Linux kernel headers with `container_of`, `list_for_each_entry`, `DECLARE_SPINLOCK`, etc.)
will produce `ERROR` subtrees silently.

**Recommendation:** Add an `error_node_count` field to `ParsedFile`. During `ParsedFile::parse`,
walk the tree and count `ERROR` nodes. If the count exceeds a configurable threshold (e.g., 5%
of total nodes), emit a warning to stderr and include an `analysis_quality: degraded` field in
the JSON output. This makes partial-parse failures visible to the user instead of producing
silently wrong output.

### 6.2 Performance Concerns on Large C Files

**O(n²) algorithms:**

- **ResonanceSlice** (`resonance_slice.rs` lines 179–192): The co-change pair counting loop is
  O(f²) per commit where f is files-per-commit. The `MAX_FILES_PER_COMMIT = 50` cap bounds this
  at 2,500 pairs per commit, which is acceptable. The outer loop over commits can be large for
  active repositories.

- **DataFlowGraph::build** (`data_flow.rs` lines 59–158): For each function, for each defined
  variable, calls `find_variable_references_scoped`. If a function has N lines and M variables,
  this is O(N × M) per function. Large C functions (the `make_large_function_test` fixture) with
  many local variables can be slow. The test fixture exists precisely to exercise this path.

- **GradientSlice BFS** (`gradient_slice.rs` lines 73–151): BFS with a 5-hop limit and 0.6
  decay factor. With a decay of 0.6, the score after 5 hops is 0.6^5 = 0.078, above the 0.1
  threshold. This means the BFS will reach the full 5-hop limit from every diff line. In large
  files with many variables, this can create very large BFS frontiers.

**Recommendation:**
- Profile `DataFlowGraph::build` on `make_large_function_test`. Add a line-count guard: if a
  function exceeds 500 lines, warn and skip DFG construction for that function.
- Add a max-nodes limit to GradientSlice BFS (e.g., cap at 10,000 scored lines across all files).

### 6.3 Assumptions That Break for C/C++

The following assumptions baked into the current implementation are valid for GC languages
(Python, JavaScript, Go, Java) but break for C/C++:

| Assumption | Where it lives | Breaks because |
|---|---|---|
| Variables have a single definition site | `data_flow.rs` — def-use chains | C variables can be defined via pointer assignment to the same storage; register variables have no addressable definition |
| Function names are globally unique | `call_graph.rs` line 58 — `functions.entry(name)` accumulates all functions with the same name | C supports `static` functions with file scope; two files can have `static int init()` and the call graph conflates them |
| No aliasing | DFG throughout | C pointers alias freely |
| Single thread of execution (for data flow) | DFG assumes sequential execution | Interrupt handlers and signal handlers create truly concurrent execution even in single-threaded C programs |
| All function calls are direct | `call_graph.rs` call site extraction | C uses function pointers extensively |
| Scopes are lexical blocks | `is_scope_block` in `languages/mod.rs` line 268 | C also has implicit scopes introduced by `for (int i = ...)` loop initializers; this is a minor issue since the loop body is a `compound_statement` |

---

## 7. Prioritized Implementation Roadmap

### P0 — Critical: Blocks C/C++ Usefulness

These items cause silent incorrect output on the primary C/C++ firmware vulnerability classes.

| Item | Effort | Impact |
|---|---|---|
| **Add C/C++ taint sinks** to `taint.rs` `SINK_PATTERNS`: `strcpy`, `strcat`, `sprintf`, `snprintf`, `gets`, `scanf`, `memcpy`, `memmove`, `recv`, `recvfrom`, `read`, `fread`, `execv`, `execve`, `execvp` | 1–2 hours | TaintSlice fires on buffer overflow, command injection |
| **Add C/C++ provenance sources** to `provenance_slice.rs`: `fgets`, `fread`, `read(`, `recv(`, `recvfrom(`, `scanf`, `fgetc`, `getchar` | 1–2 hours | ProvenanceSlice correctly classifies network/file input |
| **Fix AbsenceSlice RAII false positives**: detect `std::lock_guard`, `std::unique_lock`, `std::scoped_lock`, `std::unique_ptr`, `std::shared_ptr` in function body and suppress lock/alloc findings | 2–4 hours | Eliminates guaranteed false positives on all modern C++ code |
| **Add kernel lock/memory pairs** to `absence_slice.rs`: `kmalloc`/`kfree`, `spin_lock`/`spin_unlock`, `spin_lock_irqsave`/`spin_unlock_irqrestore`, `mutex_lock`/`mutex_unlock`, `request_irq`/`free_irq`, `dma_alloc_coherent`/`dma_free_coherent` | 2–3 hours | Absence analysis covers kernel driver code |
| **Fix MembraneSlice C error handling detection**: add `if (ret < 0)`, `if (!ptr)`, `if (ptr == NULL)`, `assert(`, `errno`, `perror(`, `CHECK_`, `WARN_ON_ONCE` | 1–2 hours | Eliminates guaranteed false positives on all C cross-file callers |

---

### P1 — Important: Reduces False Positive / Negative Rate

| Item | Effort | Impact |
|---|---|---|
| **Function pointer call edge resolution**: in `call_graph.rs`, when the callee name is a field expression or pointer dereference, attempt name extraction from the field identifier | 4–8 hours | CircularSlice, MembraneSlice, VerticalSlice get edges for function-pointer dispatch |
| **ERROR node detection and reporting**: add `error_node_count` to `ParsedFile`, warn on parse degradation | 2–4 hours | Users know when macro-heavy code is producing incomplete analysis |
| **QuantumSlice C/C++ fix for ISR/signal-handler self-detection**: check if a function's name appears in a `signal()`, `request_irq()`, or `pthread_create()` call anywhere in the parsed files | 3–5 hours | QuantumSlice correctly identifies signal/IRQ handlers as async |
| **Add `discover.py`** (or equivalent Rust binary) for file enumeration and analysis orchestration | 4–8 hours | Enables automated analysis of entire firmware trees without manual file specification |
| **Static function name disambiguation**: in `call_graph.rs`, index functions as `(file, name)` pairs; only merge cross-file if a `static` modifier is absent | 3–5 hours | Eliminates call graph conflation of same-named static functions |

---

### P2 — Valuable: Improves Analysis Depth

| Item | Effort | Impact |
|---|---|---|
| **Struct/union field tracking**: extend DFG to track `dev->field` accesses as separate def/use nodes using `(var_name, field_name)` pairs | 1–2 weeks | Eliminates false taint propagation across struct fields |
| **Virtual dispatch in C++ call graph**: for a call through a pointer-to-base-class, add edges to all known overrides of the virtual method | 1–2 weeks | CircularSlice, MembraneSlice, VerticalSlice accurate for C++ OOP |
| **`va_list` taint tracking**: when a variadic function call appears as a taint sink, propagate taint through format string argument position | 3–5 days | Detects format string injection class |
| **CVE-pattern test fixtures**: add fixtures for heap spray, format string injection, integer overflow, and kernel double-free patterns | 2–3 days | Regression coverage for known firmware bug classes |
| **C/C++ function extraction in PhantomSlice**: add C/C++ function definition detection to `extract_function_name` | 1–2 hours | PhantomSlice works for deleted C/C++ functions |

---

### P3 — Future: Additional Languages and Advanced Analysis

| Item | Effort | Impact |
|---|---|---|
| **Rust language support** (`tree-sitter-rust`): add `Language::Rust`, wire all node type methods | 2–3 weeks | Analysis of new kernel drivers and safety-critical daemons written in Rust |
| **Shell/Bash support** (`tree-sitter-bash`): add `Language::Bash`, taint sources (`$1`, `$@`), sinks (`eval`, backtick execution) | 1–2 weeks | Command injection detection in firmware update scripts |
| **Lua support** (`tree-sitter-lua`): add `Language::Lua`, taint for `os.execute()`, `io.popen()` | 1–2 weeks | LuCI web interface security analysis |
| **`goto`-based error path analysis**: extend AbsenceSlice to follow goto targets in the cleanup scan | 3–5 days | Correct double-free/double-unlock detection for kernel-style error patterns |
| **Preprocessor-aware analysis** (best-effort): run `cpp -E` on C files before parsing to expand macros; treat expanded source as the analysis target | 2–4 weeks | Eliminates ERROR nodes from macro-heavy code; enables analysis of macro-defined functions |
| **Protocol Buffers support** (`tree-sitter-proto`): add `Language::Proto`, wire SymmetrySlice for field addition/removal | 1 week | Schema asymmetry detection for gRPC/protobuf IPC |
| **Function-level git churn in ThreeDSlice**: use `git log -L :funcname:file` for per-function temporal data | 1 week | More accurate risk scores for large files |

---

*Analysis based on codebase state as of commit `6d82494` (March 30, 2026). All line number
references are to the files as read during analysis; they may shift as development continues.*
