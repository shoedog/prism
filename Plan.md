# Prism Implementation Plan & Status Tracker

Last updated: 2026-04-01 (multi-language taint sink patterns)

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
| Function pointer call edge resolution Level 0 (field-access dispatch: `ptr->func()`) | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Function pointer call edge resolution Level 1 (local variable fptrs: `fptr = func; fptr()`) | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Function pointer call edge resolution Level 2 (array dispatch tables: `handlers[i]()`) | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Static function name disambiguation — `static_functions` set in call graph, `resolve_callees()`, `callers_of_in_file()` | `claude/quantum-isr-static-disambiguation` | Done |
| QuantumSlice ISR/signal-handler self-detection — `collect_registered_handlers()` scans all files for `signal()`, `pthread_create()`, `request_irq()`, `.sa_handler`, `std::thread` | `claude/quantum-isr-static-disambiguation` | Done |
| `discover.py` (or Rust binary) for file enumeration | — | Not started |

**Tests added:** MembraneSlice C error handling (2), PhantomSlice C/C++ extraction (1 unit test), function pointer Level 0: call graph field expression (1), membrane via field dispatch (1), circular slice via field dispatch (1), Level 1: local fptr (1), Level 2: local dispatch table (1), global dispatch table (1), membrane via local fptr (1), ISR self-detection: signal cross-function (1), pthread registered (1), IRQ cross-file (1), static disambiguation: same-name static (1), static vs non-static (1), membrane respects static (1).

### Multi-Language Pattern Coverage (In Progress)

| Item | Branch | Status |
|------|--------|--------|
| Taint sinks — add Python (pickle.loads, subprocess.Popen, compile, render_template_string, mark_safe, Markup, getattr, setattr), JS/TS (innerHTML, outerHTML, insertAdjacentHTML, Function, spawn, execFile, execSync, spawnSync, writeFile, writeFileSync, raw, literal), Go (Command, Exec, HTML, Fprintf, Sprintf, Remove, RemoveAll, WriteFile, Query, QueryRow) | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Provenance sources — add Python (request.form/json/data, Django ORM, cursor.execute/fetchone, sys.stdin), JS/TS (document.cookie, window.location, URLSearchParams, req.cookies/headers, prisma, knex, collection.find), Go (r.URL.Query, r.Header, r.FormFile, sql.Query/QueryRow, rows.Scan, viper, flag, yaml.Unmarshal) | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Absence pairs — add Python (threading.Lock/release, pool/close, socket, tempfile), JS/TS (createReadStream/destroy, createServer/close, pool.connect/release, fs.open/close), Go (sql.Open/Close, os.Create/Close, context.WithCancel/cancel, WaitGroup Add/Wait, http.Get/Body.Close) | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Quantum async + Membrane errors — Python threading, JS nextTick/RxJS, Go channels/select | — | Not started |

**Tests added:** Taint Python pickle.loads (1), taint Python subprocess.Popen (1), taint JS innerHTML (1), taint JS execSync (1), taint Go exec.Command (1), taint Go template.HTML (1). Provenance Python request.form (1), provenance Python cursor.fetchone (1), provenance JS document.cookie (1), provenance JS process.env (1), provenance Go r.FormValue (1), provenance Go viper config (1). Absence Python threading.Lock (1), absence Python tempfile (1), absence JS createReadStream (1), absence JS fs.open (1), absence Go context.WithCancel (1), absence Go http.Get body (1).

---

## Remaining Work

### P1 — Important (Reduces False Positive/Negative Rate)

| Item | Effort | Impact | Notes |
|------|--------|--------|-------|
| **Function pointer Level 3: parameter-passed fptrs** | 4-8h | Resolves `cb(data)` where `cb` is a parameter by checking callers' arguments | 1-hop interprocedural; see `docs/c-cpp/function-pointer-resolution.md` |

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
- Function pointers: Level 0 (field-access), Level 1 (local fptr variable), Level 2 (dispatch tables) resolved; Level 3 (parameter-passed) and Level 4 (full points-to) not implemented — see `docs/c-cpp/function-pointer-resolution.md`
- `static` function scope: disambiguated via `resolve_callees()` and `callers_of_in_file()`
- Interrupt handlers: detected by naming heuristic AND cross-file registration analysis (`signal()`, `pthread_create()`, `request_irq()`, `.sa_handler`, `std::thread`)
- Struct field flow: `dev->name` taints all of `dev` (P2 item)
- Virtual dispatch: name-matched, not type-resolved (P2 item)

### Test Coverage
- **143 tests** total (unit + integration)
- 5 languages covered in integration tests
- 26 algorithms with at least basic coverage
- C/C++ specific: 32 tests covering taint, provenance, absence, quantum (incl. ISR self-detection), membrane, phantom, pointer aliasing, function pointer dispatch (Level 0/1/2), static linkage disambiguation
- Multi-language taint: 6 tests covering Python (pickle, subprocess), JS (innerHTML, execSync), Go (exec.Command, template.HTML)
- Multi-language provenance: 6 tests covering Python (request.form, cursor.fetchone), JS (document.cookie, process.env), Go (r.FormValue, viper config)
- Multi-language absence: 6 tests covering Python (threading.Lock, tempfile), JS (createReadStream, fs.open), Go (context.WithCancel, http.Get body)

---

## Reference

- Gap analysis: `docs/prism-ccpp-gap-analysis.md`
- Algorithm taxonomy: `SLICING_METHODS.md`
- Paper: arXiv:2505.17928
