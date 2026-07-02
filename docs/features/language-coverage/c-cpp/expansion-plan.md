# Prism: C/C++ Language Support & Test Expansion Plan
### March 22, 2026

---

## Purpose

Expand Prism from 5 languages (Python, JavaScript, TypeScript, Go, Java) to 7 (adding C and C++) and build a comprehensive test suite using real-world firmware/embedded codebases with known, documented bugs. The test suite validates that Prism's 26 slicing algorithms produce output that would enable an LLM reviewer to catch the bugs.

**Strategic context:** Access network platforms include modem, ONU, CMTS, and OLT infrastructure. Escaped defects in firmware/embedded code cost $1000+ per incident (truck rolls, customer impact, NOC escalation, regional outages). Extending the code review agent to C/C++ firmware code targets the highest-cost-per-defect tier.

---

## Part 1: C/C++ Language Support

### 1.1 Tree-Sitter Grammars

Both `tree-sitter-c` and `tree-sitter-cpp` are among the oldest and most battle-tested grammars in the tree-sitter ecosystem. They handle real-world code including Linux kernel code, embedded firmware, and complex template metaprogramming (C++).

**Cargo.toml additions:**
```toml
[dependencies]
tree-sitter-c = "0.23"       # Stable, widely used
tree-sitter-cpp = "0.23"     # Extends tree-sitter-c
```

### 1.2 Language Module Extension (`src/languages/mod.rs`)

The language module needs node type mappings for C and C++. These map Prism's abstract concepts (function definition, variable declaration, call expression) to tree-sitter's concrete node types for each language.

**C node type mappings:**

| Prism Concept | Tree-Sitter C Node Type | Notes |
|--------------|------------------------|-------|
| Function definition | `function_definition` | Top-level and nested functions |
| Function name | `declarator` → `identifier` within `function_definition` | Navigate through declarator chain |
| Function parameters | `parameter_list` → `parameter_declaration` | Each param has type + declarator |
| Variable declaration | `declaration` | May contain multiple `init_declarator`s |
| Variable assignment | `assignment_expression` | `=`, `+=`, `-=`, etc. |
| Call expression | `call_expression` | `function_name(args)` |
| Return statement | `return_statement` | `return expr;` |
| If statement | `if_statement` | Includes `else` clause |
| While loop | `while_statement` | |
| For loop | `for_statement` | C-style `for(init;cond;step)` |
| Switch statement | `switch_statement` → `case_statement` | |
| Struct/union definition | `struct_specifier`, `union_specifier` | Named and anonymous |
| Typedef | `type_definition` | `typedef` declarations |
| Enum | `enum_specifier` | |
| Preprocessor include | `preproc_include` | `#include <x>` and `#include "x"` |
| Preprocessor define | `preproc_def`, `preproc_function_def` | `#define` macros |
| Preprocessor conditional | `preproc_if`, `preproc_ifdef`, `preproc_ifndef` | Conditional compilation |
| Pointer expression | `pointer_expression` | `*ptr` dereference |
| Sizeof expression | `sizeof_expression` | `sizeof(type)` |
| Cast expression | `cast_expression` | `(type)expr` |
| Goto statement | `goto_statement` | Common in kernel code |
| Label | `labeled_statement` | Goto targets |

**C++ additional node types (extends C):**

| Prism Concept | Tree-Sitter C++ Node Type | Notes |
|--------------|--------------------------|-------|
| Class definition | `class_specifier` | Class declarations with methods |
| Method definition | `function_definition` within class scope | Same node, different parent |
| Constructor | `function_definition` with name matching class | No return type |
| Destructor | `function_definition` with `~ClassName` | |
| Namespace | `namespace_definition` | `namespace foo { }` |
| Template | `template_declaration` | Template functions and classes |
| Try/catch | `try_statement`, `catch_clause` | C++ exceptions |
| Throw | `throw_statement` | |
| Lambda | `lambda_expression` | C++11+ closures |
| Auto declaration | `auto` type specifier | C++11+ type inference |
| Range-based for | `for_range_loop` | `for(auto& x : container)` |
| Smart pointers | Call expressions on `unique_ptr`, `shared_ptr` | Pattern matching, not node type |
| RAII patterns | Constructors/destructors with resource management | Detected by absence slice |
| `new`/`delete` | `new_expression`, `delete_expression` | Memory management |

### 1.3 C/C++ Specific Algorithm Considerations

Some algorithms need special handling for C/C++ idioms:

**Absence Slice — C/C++ paired patterns to add:**

```rust
// C memory management
PairedPattern { open: ["malloc(", "calloc(", "realloc("], close: ["free("], desc: "heap allocation without free" },
PairedPattern { open: ["fopen("], close: ["fclose("], desc: "file open without close" },
PairedPattern { open: ["pthread_mutex_lock("], close: ["pthread_mutex_unlock("], desc: "mutex lock without unlock" },
PairedPattern { open: ["pthread_rwlock_rdlock(", "pthread_rwlock_wrlock("], close: ["pthread_rwlock_unlock("], desc: "rwlock without unlock" },
PairedPattern { open: ["sem_wait("], close: ["sem_post("], desc: "semaphore wait without post" },
PairedPattern { open: ["socket("], close: ["close(", "closesocket("], desc: "socket without close" },
PairedPattern { open: ["mmap("], close: ["munmap("], desc: "mmap without munmap" },
PairedPattern { open: ["ioctl("], close: [], desc: "ioctl without error check" },
PairedPattern { open: ["fork("], close: ["waitpid(", "wait("], desc: "fork without wait (zombie process)" },

// C++ specific
PairedPattern { open: ["new "], close: ["delete "], desc: "new without delete (use smart pointer)" },
PairedPattern { open: ["new["], close: ["delete[]"], desc: "array new without array delete" },
PairedPattern { open: [".lock()"], close: [".unlock()"], desc: "C++ mutex lock without unlock" },
PairedPattern { open: ["std::lock_guard", "std::unique_lock", "std::scoped_lock"], close: [], desc: "RAII lock — safe (no counterpart needed)" },

// Kernel/embedded specific
PairedPattern { open: ["kmalloc(", "kzalloc(", "vmalloc("], close: ["kfree(", "vfree("], desc: "kernel allocation without free" },
PairedPattern { open: ["dma_alloc_coherent("], close: ["dma_free_coherent("], desc: "DMA allocation without free" },
PairedPattern { open: ["request_irq(", "request_threaded_irq("], close: ["free_irq("], desc: "IRQ registration without free" },
PairedPattern { open: ["spin_lock(", "spin_lock_irqsave("], close: ["spin_unlock(", "spin_unlock_irqrestore("], desc: "spinlock without unlock" },
PairedPattern { open: ["clk_prepare_enable("], close: ["clk_disable_unprepare("], desc: "clock enable without disable" },
PairedPattern { open: ["platform_driver_register("], close: ["platform_driver_unregister("], desc: "driver register without unregister" },
PairedPattern { open: ["of_node_get(", "of_find_node_by_"], close: ["of_node_put("], desc: "device tree node get without put" },
```

**Taint Slice — C/C++ taint sources:**
- `recv(`, `recvfrom(`, `read(` — network/file input
- `fgets(`, `scanf(`, `gets(` — stdin/file input
- `getenv(` — environment variables
- `argv[` — command-line arguments
- Function parameters in exposed APIs (public headers)

**Taint Slice — C/C++ sinks:**
- `strcpy(`, `strcat(`, `sprintf(` — buffer overflow sinks
- `system(`, `exec(`, `popen(` — command injection sinks
- `memcpy(` without bounds check — buffer overflow
- SQL construction via `sprintf` — injection
- `free(` (double-free if tainted pointer reaches free twice)

**Quantum Slice — C/C++ concurrency patterns:**
- `pthread_create` — thread creation
- `fork()` — process creation
- Signal handlers (`signal(`, `sigaction(`) — asynchronous interruption
- Interrupt handlers (kernel: `irq_handler_t`) — hardware interrupts
- `volatile` variables — shared state indicator
- `atomic_` operations — concurrent access
- `pthread_mutex_lock`/`unlock` — critical sections

**Symmetry Slice — C/C++ paired function patterns:**
- `serialize`/`deserialize`, `encode`/`decode`, `pack`/`unpack`
- `init`/`cleanup`, `create`/`destroy`, `alloc`/`free`
- `open`/`close`, `start`/`stop`, `enable`/`disable`
- `read`/`write`, `get`/`set`, `push`/`pop`
- `register`/`unregister`, `attach`/`detach`
- Constructor/destructor (C++ `ClassName()`/`~ClassName()`)

**Provenance Slice — C/C++ origin patterns:**
- `user_input`: `recv()`, `read()` from sockets, `fgets()`, `argv[]`, `getenv()`
- `config`: `fopen()` on config files, `getopt()`, command-line parsing
- `database`: Result of database query functions
- `constant`: `#define` macros, `const` variables, enum values
- `hardware`: `ioctl()` results, memory-mapped register reads, DMA buffer reads
- `function_param`: Function parameters in non-static functions

### 1.4 Preprocessor Handling

C/C++ preprocessor directives create unique challenges for slicing:

**`#include` directives:** Tree-sitter parses them as `preproc_include` nodes. For cross-file analysis (call graph, membrane slice), Prism would need to resolve include paths to actual files. Initial implementation can skip include resolution and rely on call graph name matching across files (already works for other languages). Full include resolution is a future enhancement.

**`#define` macros:** Macros that define functions or wrap function calls are invisible to tree-sitter's function analysis. `#define CHECK_ERROR(x) if(!(x)) { return -1; }` looks like a function call but tree-sitter sees `preproc_function_def`. Initial implementation should detect macro definitions and flag them as potential hidden behavior.

**`#ifdef` conditional compilation:** Code inside `#ifdef` blocks may or may not be compiled. For slicing purposes, include all branches — the reviewer should see what code exists under different configurations. The conditioned slice could be extended to handle `#ifdef` conditions.

**Strategy:** Phase 1 handles C/C++ without preprocessor resolution (treat it like other languages — parse what tree-sitter gives you). Phase 2 adds preprocessor-aware analysis for macros and conditional compilation.

---

## Part 2: Test Expansion Strategy

### 2.1 Test Philosophy

Each test validates that a specific Prism algorithm, given a diff containing a known bug, produces a slice that includes the buggy code with sufficient context for an LLM to identify the defect.

**Pass criterion:** The algorithm's output includes:
1. The buggy line(s)
2. Enough surrounding context (data flow, callers, control flow) that the bug is recognizable

**Fail criterion:** The algorithm's output either:
1. Doesn't include the buggy code at all (recall failure), OR
2. Includes the buggy code but without enough context to understand why it's buggy (context failure)

### 2.2 Test Target Selection

#### Tier 1: Telecom/Network Infrastructure

**OpenWrt** (`github.com/openwrt/openwrt`)
- **Relevance:** Router/embedded Linux OS. Directly relevant to access network infrastructure (modems, routers, access points).
- **Language:** C
- **CVE richness:** Extensive. Recent Trail of Bits audit (Feb 2026) found stack buffer overflows (CVE-2026-30871, CVE-2026-30872), memory leaks (CVE-2026-30873), command injection (CVE-2026-30874). Historical CVEs include heap overflows in ubus, out-of-bounds reads in umdns/relayd, CSRF/XSS in LuCI.
- **Key components for testing:**
  - `ubus` / `ubusd` — IPC system. Heap buffer overflow in event registration (CVE-2026-31167). Tests: absence (allocation checks), taint (untrusted message data), echo (callers of changed ubus API).
  - `umdns` — mDNS daemon. Stack buffer overflows in DNS parsing (CVE-2026-30871, CVE-2026-30872). Tests: taint (DNS packet data → buffer), absence (bounds checking on memcpy/strcpy).
  - `netifd` — Network interface daemon. IPv6 routing loop (CVE-2021-22161). Tests: circular (routing loop detection in data flow), echo (API changes affecting callers).
  - `procd` — Process manager. Command injection via PATH bypass (CVE-2026-30874). Tests: taint (environment variable → exec), provenance (origin classification of PATH variable).
  - `libubox` — Core utility library. JSON serialization vulnerability (CVE-2020-7248). Tests: symmetry (JSON serialize/parse consistency), taint (untrusted JSON → parsing).

**FRRouting** (`github.com/FRRouting/frr`)
- **Relevance:** Routing protocol suite (BGP, OSPF, IS-IS, LDP, MPLS). Access networks commonly interface with routers running FRR or similar.
- **Language:** C
- **CVE richness:** Regular security advisories. Buffer overflows in BGP message parsing, null pointer dereferences in route processing, use-after-free in stream handling.
- **Algorithm focus:** absence (memory alloc/free pairing in route processing), taint (network packet data → processing), quantum (threaded route processing, event loop concurrency), barrier (bounded depth through deep daemon call graphs), echo (API changes in lib/ affecting all daemons).

**Cable Haunt patterns** (synthetic test cases)
- **Relevance:** CVE-2019-19494 affects Broadcom cable modem firmware — directly relevant to DOCSIS modem deployments.
- **Approach:** Create synthetic C files matching vulnerability patterns: websocket handler with missing authentication, buffer overflow in spectrum analyzer data processing, default credentials in configuration.

#### Tier 2: Embedded Systems with Rich Bug Histories

**curl** (`github.com/curl/curl`)
- **Relevance:** Universal data transfer library. Present in virtually all embedded systems including cable modems.
- **Language:** C
- **CVE richness:** The most CVE-rich C project with meticulously documented fixes. Over 130 CVEs with fix commits.
- **Algorithm focus:** Every algorithm. Curl's bug diversity makes it the single best project for comprehensive algorithm validation.

**OpenSSL** (`github.com/openssl/openssl`)
- **Relevance:** TLS library in every network device. Heartbleed is the canonical example of a buffer over-read.
- **Key bugs:** Heartbleed (missing bounds check), CCS injection (state machine violation), padding oracle attacks.
- **Algorithm focus:** absence (bounds checking), symmetry (encrypt/decrypt pair consistency), circular (TLS state machine cycles), conditioned (slicing under protocol state assumptions).

**Zephyr RTOS** (`github.com/zephyrproject-rtos/zephyr`)
- **Relevance:** Real-time OS for embedded devices. Interrupt handlers, shared memory, DMA patterns match modem/ONU firmware.
- **Algorithm focus:** quantum (interrupt handler + main loop shared state), absence (IRQ registration without cleanup), echo (kernel API changes affecting all drivers), provenance (hardware register reads).

#### Tier 3: Focused Libraries for Specific Algorithm Testing

**mbedTLS**, **lwIP**, **cJSON**, **libuv** — each targeting specific algorithm validation needs.

#### Tier 4: Language-Specific Edge Cases

**Linux kernel** (selected subsystems: `drivers/net/`, `net/bridge/`, `net/netfilter/`) — stress-tests preprocessor handling, kernel memory management idioms, and deeply nested call graphs.

---

## Part 3: Test Matrix

### 3.1 Algorithm × Bug Class Coverage

| Algorithm | Bug Class | Best Test Project | Specific Bug/CVE |
|-----------|----------|------------------|-----------------|
| **LeftFlow** | Variable lifecycle tracking | curl | Any use-after-free CVE |
| **FullFlow** | Forward propagation | FRRouting | Route update propagation bug |
| **ThinSlice** | Minimal data chain | cJSON | Null deref from parse result |
| **BarrierSlice** | Bounded cross-function | FRRouting | Deep call graph in BGP processing |
| **Chop** | Source-to-sink paths | curl | URL input → buffer overflow |
| **Taint** | Untrusted input tracking | OpenWrt umdns | DNS packet → stack buffer overflow |
| **RelevantSlice** | Missing else/unhandled cases | OpenSSL | CCS state machine missing state |
| **ConditionedSlice** | "What if null?" | curl | Null pointer deref under specific condition |
| **DeltaSlice** | Behavioral change between versions | libubox | JSON serialization behavior change |
| **SpiralSlice** | Adaptive depth | FRRouting | Bug requiring 3 levels of call context |
| **CircularSlice** | Data flow cycles | OpenWrt netifd | IPv6 routing loop |
| **QuantumSlice** | Concurrent state | Zephyr | ISR + main loop shared variable |
| **HorizontalSlice** | Peer inconsistency | OpenWrt | Route handlers with inconsistent validation |
| **VerticalSlice** | End-to-end path | lwIP | Packet from recv → application callback |
| **AngleSlice** | Cross-cutting concern | FRRouting | Error handling across BGP subsystem |
| **3DSlice** | Risk scoring | Any project | High-churn file with security CVE |
| **AbsenceSlice** | Missing cleanup | OpenWrt | malloc without free in error path |
| **ResonanceSlice** | Co-change coupling | OpenWrt | Files that always change together |
| **SymmetrySlice** | Broken symmetry | OpenSSL | Encrypt/decrypt inconsistency |
| **GradientSlice** | Relevance ranking | curl | Deep call graph with varying relevance |
| **ProvenanceSlice** | Data origin | OpenWrt procd | PATH env var → command execution |
| **PhantomSlice** | Deleted code dependency | Any project | Function removed, callers still reference |
| **MembraneSlice** | API contract break | curl | API change affecting libcurl consumers |
| **EchoSlice** | Downstream breakage | curl | Return type change breaking callers |

### 3.2 Test Implementation Approach

```rust
#[test]
fn test_{algorithm}_{project}_{bug_class}() {
    let source = include_str!("fixtures/c/{project}/{cve_id}/vulnerable.c");
    let diff_json = include_str!("fixtures/c/{project}/{cve_id}/diff.json");
    let files = parse_files_from_source("vulnerable.c", source, "c");
    let diff = parse_diff_json(diff_json);
    let result = {algorithm}::slice(&files, &diff).unwrap();
    assert!(!result.blocks.is_empty(), "Algorithm produced no output");
    let all_lines: Vec<usize> = result.blocks.iter()
        .flat_map(|b| b.lines.iter().map(|l| l.line_number))
        .collect();
    assert!(all_lines.contains(&{buggy_line}),
        "Slice should include the buggy line {buggy_line}");
    assert!(all_lines.len() >= {minimum_context_lines},
        "Slice should include enough context for LLM review");
}
```

### 3.3 Synthetic Test Cases for Firmware Patterns

**DOCSIS-relevant patterns:**
```c
// Pattern 1: SNMP buffer overflow
void handle_snmp_set(uint8_t *pdu, size_t pdu_len) {
    char community[64];
    memcpy(community, pdu + offset, community_len); // BUG: no bounds check
}

// Pattern 2: Configuration rollback failure
void apply_config(config_t *new_config) {
    config_t *old = current_config;
    current_config = new_config;
    if (!validate_config(new_config)) {
        // BUG: old config is already lost — rollback impossible
    }
}

// Pattern 3: Interrupt handler shared state
volatile int packet_count = 0;
void rx_interrupt_handler(void) {
    packet_count++;  // BUG: not atomic
    process_packet(rx_buffer);
}
```

**GPON/EPON ONU patterns:**
```c
// Pattern 4: ONU registration state machine — missing state transition handling
// Pattern 5: DMA buffer management — free before transfer completes
```

---

## Part 4: Implementation Phases

### Phase 1: Language Support (1-2 days with Claude Code)
1. Add tree-sitter-c and tree-sitter-cpp to Cargo.toml
2. Extend src/languages/mod.rs with C and C++ node type mappings
3. Add language detection for .c, .h, .cpp, .cc, .cxx, .hpp files
4. Extend algorithm implementations with C/C++ specific patterns
5. Run existing test suite to verify no regressions
6. Add basic C/C++ integration tests

### Phase 2: Real-World CVE Test Fixtures (2-3 days)
1. Clone test target repos (OpenWrt, curl, OpenSSL)
2. Extract 3-5 CVE fix diffs per project
3. Create test fixtures (vulnerable code, diff, expected buggy lines)
4. Write integration tests per algorithm × bug class

### Phase 3: Synthetic Firmware Test Cases (1-2 days)
1. Create synthetic test files for DOCSIS, GPON/EPON, interrupt handling, DMA patterns
2. Write tests verifying algorithms detect known bugs

### Phase 4: Cross-Language and Stress Testing (1-2 days)
1. Test on large real-world files (1000+ line kernel drivers)
2. Performance benchmarking
3. Edge cases: macro-heavy code, deeply nested preprocessor conditionals

### Phase 5: Integration with Review Pipeline (2-3 days)
1. Update discover.py to call Prism for C/C++ repos
2. Add C/C++ language support to repo metadata format
3. Create steering and rubric files for C/C++ review
4. Run end-to-end review on a C codebase

---

## Part 5: Metrics and Success Criteria

### Language Support Success
- [ ] All existing tests still pass after C/C++ additions
- [ ] C and C++ files parse without errors on: OpenWrt, curl, cJSON, FRRouting source files
- [ ] LeftFlow produces non-empty output for C function modifications
- [ ] Call graph builds correctly across C files
- [ ] Absence slice detects missing `free()` for `malloc()` in test fixtures

### Test Coverage Success
- [ ] Minimum 5 real CVE test fixtures per Tier 1 project
- [ ] Minimum 3 real CVE test fixtures per Tier 2 project
- [ ] Minimum 5 synthetic firmware pattern test cases
- [ ] Each of the 26 algorithms has at least one C/C++ test
- [ ] The test matrix (Algorithm × Bug Class) has >70% coverage

### Algorithm Effectiveness
For each CVE test fixture, record which algorithms include the buggy code, which provide sufficient context, and which miss the bug entirely. This data feeds back into algorithm prioritization for the review pipeline.
