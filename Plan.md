# Prism Implementation Plan & Status Tracker

Last updated: 2026-04-01

---

## Completed Work

### P0 — Critical C/C++ Patterns (All Done)

| Item | PR/Commit | Status |
|------|-----------|--------|
| Add C/C++ taint sinks to `taint.rs` (strcpy, sprintf, memcpy, etc.) — bare names without `(` | PR #5 (`7bdbe54`) | Done |
| Add C/C++ provenance sources (recv, fgets, ioctl→Hardware, getenv→EnvVar) | PR #5 (`7bdbe54`) | Done |
| Fix AbsenceSlice RAII false positives (lock_guard, unique_ptr, shared_ptr suppression) | PR #5 (`7bdbe54`) | Done |
| Add kernel lock/memory pairs (kmalloc/kfree, spin_lock/spin_unlock, DMA, IRQ) | PR #5 (`7bdbe54`) | Done |
| QuantumSlice C/C++ async detection (signal, pthread, ISR `_handler` heuristic) | PR #5 (`7bdbe54`) | Done |
| Pointer aliasing: `extract_lvalue_names()` in ast.rs (*p, dev->field, buf[i]) | PR #5 (`7bdbe54`) | Done |
| Tree-sitter parse error detection and reporting | PR #3 (`42cc508`) | Done |
| **Bug fix:** Stripped trailing `(` from all 18 C/C++ taint sink patterns | PR #5 (`7bdbe54`) | Done |

**Tests added (PR #5):** 17 C/C++ tests — taint sinks (3), provenance origins (3), absence pairs (4), quantum async (3), pointer aliasing (2), data flow pointer deref (1), taint buffer overflow (1).

### P1 — Important Fixes (Partial)

| Item | PR/Commit | Status |
|------|-----------|--------|
| MembraneSlice C error handling detection (`if (ret < 0)`, `if (!ptr)`, errno, perror, assert, CHECK_, WARN_ON) | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| PhantomSlice C/C++ function extraction (`[type] *func_name(` patterns, qualified names) | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| ERROR node detection and reporting | PR #3 (`42cc508`) | Done |
| Function pointer call edge resolution in `call_graph.rs` | — | Not started |
| Static function name disambiguation (file-scoped `static` functions) | — | Not started |
| QuantumSlice ISR/signal-handler self-detection (function registered externally) | — | Not started |
| `discover.py` (or Rust binary) for file enumeration | — | Not started |

**Tests added:** MembraneSlice C error handling (2), PhantomSlice C/C++ extraction (1 unit test).

---

## Remaining Work

### P1 — Important (Reduces False Positive/Negative Rate)

| Item | Effort | Impact | Notes |
|------|--------|--------|-------|
| **Function pointer call edge resolution** | 4-8h | CircularSlice, MembraneSlice, VerticalSlice get edges for `ops->open()` style dispatch | In `call_graph.rs`, when callee is a field expression, extract the field identifier |
| **Static function name disambiguation** | 3-5h | Eliminates call graph conflation of same-named `static` functions across files | Index as `(file, name)` pairs; only merge cross-file if `static` absent |
| **QuantumSlice ISR self-detection** | 3-5h | Detects signal/IRQ handler functions that ARE the async entry point | Check if function name appears in `signal()`, `request_irq()` calls in other parsed files |

### P2 — Valuable (Improves Analysis Depth)

| Item | Effort | Impact |
|------|--------|--------|
| Struct/union field-level tracking in DFG | 1-2 weeks | Eliminates false taint propagation across struct fields (`dev->name` vs `dev->id`) |
| Virtual dispatch in C++ call graph | 1-2 weeks | Accurate analysis for C++ OOP polymorphism |
| `va_list` taint tracking | 3-5 days | Detects format string injection (`snprintf(buf, sz, user_input)`) |
| CVE-pattern test fixtures (heap spray, format string, integer overflow) | 2-3 days | Regression coverage for known firmware bug classes |
| `goto`-based error path analysis for AbsenceSlice | 3-5 days | Correct double-free/double-unlock detection for kernel-style `goto cleanup` |
| MembraneSlice C++ error handling (exceptions, RAII) | 2-3h | Better precision for C++ cross-file callers |

### P3 — Future Languages & Advanced Analysis

| Item | Effort | Priority |
|------|--------|----------|
| **Rust** (`tree-sitter-rust`) | 2-3 weeks | Must-have — kernel drivers, safety-critical daemons |
| **Shell/Bash** (`tree-sitter-bash`) | 1-2 weeks | Should-have — command injection in firmware update scripts |
| **Lua** (`tree-sitter-lua`) | 1-2 weeks | Should-have — LuCI web interface security |
| Protocol Buffers (`tree-sitter-proto`) | 1 week | Nice-to-have — schema asymmetry for gRPC IPC |
| Preprocessor-aware analysis (`cpp -E`) | 2-4 weeks | Eliminates ERROR nodes from macro-heavy code |
| Function-level git churn in ThreeDSlice | 1 week | More accurate risk scores for large files |

---

## Architecture Notes

### Key Design Decisions
- **Tree-sitter** for multi-language AST parsing (5 languages: Python, JS/TS, Go, Java, C/C++)
- **Name-based variable tracking** (no `varId` like cppcheck)
- **BTreeMap/BTreeSet everywhere** for deterministic sorted output
- **Shared infrastructure:** `call_graph.rs` and `data_flow.rs` reused across algorithms
- **Algorithm-specific configs** in each module, not in central `SliceConfig`

### Known Limitations (C/C++)
- Pointer aliasing: tracked at name level, not memory level (extract_lvalue_names mitigates)
- Function pointers: invisible to call graph (P1 item)
- `static` function scope: not disambiguated in call graph (P1 item)
- Interrupt handlers: detected by naming heuristic only, not by registration analysis (P1 item)
- Struct field flow: `dev->name` taints all of `dev` (P2 item)
- Virtual dispatch: name-matched, not type-resolved (P2 item)

### Test Coverage
- **112 tests** total (unit + integration)
- 5 languages covered in integration tests
- 26 algorithms with at least basic coverage
- C/C++ specific: 19 tests covering taint, provenance, absence, quantum, membrane, phantom, pointer aliasing

---

## Reference

- Gap analysis: `docs/prism-ccpp-gap-analysis.md`
- Algorithm taxonomy: `SLICING_METHODS.md`
- Paper: arXiv:2505.17928
