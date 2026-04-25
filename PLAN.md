# Prism Implementation Plan & Status Tracker

Last updated: 2026-04-24 (T1-006 handoff, three new algorithms, pre-handoff baseline added)

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
| MembraneSlice C error handling detection (`if (ret < 0)`, `if (!ptr)`, errno, perror, assert, CHECK_, WARN_ON) | `claude/echo-rust-lua-support` | Done |
| PhantomSlice C/C++ function extraction (`[type] *func_name(` patterns, qualified names) | `claude/echo-rust-lua-support` | Done |
| ERROR node detection and reporting | PR #3 (`42cc508`) | Done |
| Function pointer call edge resolution Level 0 (field-access dispatch: `ptr->func()`) | `claude/echo-rust-lua-support` | Done |
| Function pointer call edge resolution Level 1 (local variable fptrs: `fptr = func; fptr()`) | `claude/echo-rust-lua-support` | Done |
| Function pointer call edge resolution Level 2 (array dispatch tables: `handlers[i]()`) | `claude/echo-rust-lua-support` | Done |
| Static function name disambiguation — `static_functions` set in call graph, `resolve_callees()`, `callers_of_in_file()` | `claude/quantum-isr-static-disambiguation` | Done |
| QuantumSlice ISR/signal-handler self-detection — `collect_registered_handlers()` scans all files for `signal()`, `pthread_create()`, `request_irq()`, `.sa_handler`, `std::thread` | `claude/quantum-isr-static-disambiguation` | Done |
| `discover.py` (or Rust binary) for file enumeration | — | Not started |
| T1-006 follow-up: IPC taint sources (`g_hash_table_lookup` + variant accessors), CFG multiline call edge fix, GLib callback dispatcher detection | `eebafb6` | Done |
| Slice text empty in JSON output fix; `settings_t` provenance source added | `e3fa16d` | Done |
| Spiral added to review suite; taint sinks expanded; provenance import-suppression FP fix | `37ef823` | Done |

**Tests added:** MembraneSlice C error handling (2), PhantomSlice C/C++ extraction (1 unit test), function pointer Level 0: call graph field expression (1), membrane via field dispatch (1), circular slice via field dispatch (1), Level 1: local fptr (1), Level 2: local dispatch table (1), global dispatch table (1), membrane via local fptr (1), ISR self-detection: signal cross-function (1), pthread registered (1), IRQ cross-file (1), static disambiguation: same-name static (1), static vs non-static (1), membrane respects static (1).

### Multi-Language Pattern Coverage (In Progress)

| Item | Branch | Status |
|------|--------|--------|
| Taint sinks — add Python (pickle.loads, subprocess.Popen, compile, render_template_string, mark_safe, Markup, getattr, setattr), JS/TS (innerHTML, outerHTML, insertAdjacentHTML, Function, spawn, execFile, execSync, spawnSync, writeFile, writeFileSync, raw, literal), Go (Command, Exec, HTML, Fprintf, Sprintf, Remove, RemoveAll, WriteFile, Query, QueryRow) | `claude/echo-rust-lua-support` | Done |
| Provenance sources — add Python (request.form/json/data, Django ORM, cursor.execute/fetchone, sys.stdin), JS/TS (document.cookie, window.location, URLSearchParams, req.cookies/headers, prisma, knex, collection.find), Go (r.URL.Query, r.Header, r.FormFile, sql.Query/QueryRow, rows.Scan, viper, flag, yaml.Unmarshal) | `claude/echo-rust-lua-support` | Done |
| Absence pairs — add Python (threading.Lock/release, pool/close, socket, tempfile), JS/TS (createReadStream/destroy, createServer/close, pool.connect/release, fs.open/close), Go (sql.Open/Close, os.Create/Close, context.WithCancel/cancel, WaitGroup Add/Wait, http.Get/Body.Close) | `claude/echo-rust-lua-support` | Done |
| Quantum async — Python (threading.Thread, multiprocessing.Process, asyncio.create_task), JS/TS (Worker, process.nextTick, setImmediate, queueMicrotask), Go (select statement, channel send/receive) | `claude/echo-rust-lua-support` | Done |
| Membrane errors — Python (raise_for_status, raise), JS/TS (throw, Promise.reject, .finally), Go (errors.Is, errors.As, log.Fatal, panic) | `claude/echo-rust-lua-support` | Done |

**Tests added:** Taint Python pickle.loads (1), taint Python subprocess.Popen (1), taint JS innerHTML (1), taint JS execSync (1), taint Go exec.Command (1), taint Go template.HTML (1). Provenance Python request.form (1), provenance Python cursor.fetchone (1), provenance JS document.cookie (1), provenance JS process.env (1), provenance Go r.FormValue (1), provenance Go viper config (1). Absence Python threading.Lock (1), absence Python tempfile (1), absence JS createReadStream (1), absence JS fs.open (1), absence Go context.WithCancel (1), absence Go http.Get body (1). Quantum Python threading (1), quantum JS Worker (1), quantum Go channel/select (1). Membrane Python raise_for_status (1), membrane Go errors.Is (1).

### Algorithms — Tier 1 (T1) Capability Expansion

| Item | Branch / Commit | Status |
|------|-----------------|--------|
| **T1-002:** PeerConsistencySlice — sibling first-param NULL-guard cluster detection (uniform & divergent gap classifications). C/C++ only by design. Driven by FRR CVE-2025-61102. | `claude/t1-cleanup-pre-cwe-handoff` | Done |
| **T1-002:** CallbackDispatcherSlice — function-pointer-in-struct registration → invocation chain resolution; flags NULL-arg dispatch (zlog/functab pattern + GLib `g_signal_connect`). C/C++ only by design. | `claude/t1-cleanup-pre-cwe-handoff` | Done |
| **T1-005:** PrimitiveSlice — security-primitive fingerprint sweep (HASH_TRUNCATED_BELOW_128_BITS, HASH_TRUNCATION_VIA_CALL 2-pass, WEAK_HASH_FOR_IDENTITY, SHELL_TRUE_WITH_INTERPOLATION, CERT_VALIDATION_DISABLED, HARDCODED_SECRET). Python primary; basic C/JS/Go for cert-validation + secret rules. | `claude/t1-cleanup-pre-cwe-handoff` | Done |
| **Hapi-4552 regression smoke test** — JS event-listener-pair fixture wired into integration tests as `integration_hapi_regression`. Loose structural assertions (LeftFlow fires, diff lines surface) rather than byte-equal snapshot. | `claude/t1-cleanup-pre-cwe-handoff` | Done |
| **ALGORITHMS.md** — per-algorithm operator's guide (question, mechanism, output, limitations, source per algorithm) for all 30 algorithms. Companion to AK-team's prism-slice-glossary-verify handoff. | `claude/t1-cleanup-pre-cwe-handoff` (`10b9bfd`) | Done |

**Tests added (32 + 1 = 33):** Peer C 4 (uniform-unguarded, divergent, all-guarded negative, cluster-too-small negative), Peer C++ 3 (uniform, divergent, only-fires-on-touched-param). Callback C 4 (designated-init→null-dispatch, assignment-field clean, registrar-call-arg, no-invocations negative), Callback C++ 3 (designated-init, g_signal_connect, unrelated-function negative). Primitive Python 10 (hash-trunc direct/threshold/raw, hash-trunc 2-pass, weak-hash positive/negative, shell-true positive/negative, cert-validation, hardcoded-secret with inline negative). Primitive C 5 (cert-VERIFYPEER, cert-VERIFYHOST, dirty-function severity, outside-dirty-function severity, proper-validation negative). Primitive cross-language 3 (JS reject_unauthorized, JS hardcoded-secret object-field, Go InsecureSkipVerify). Hapi smoke 1.

### Algorithm Precision & New Language Support

| Item | Branch | Status |
|------|--------|--------|
| Echo slice — expand SAFE_PATTERNS with C/C++ return-code checks, Go errors.Is/As, Python context manager, Rust ?/unwrap, Lua pcall/xpcall; expand change_touches_error and has_error_handling | `claude/echo-rust-lua-support` | Done |
| Provenance precision — add matches_provenance() with '~' word-boundary prefix; tighten ~body, ~form, ~input, ~params, ~query, ~args, ~fetch, ~execute, ~cursor, ~select | `claude/echo-rust-lua-support` | Done |
| Rust language support — Language::Rust enum, tree-sitter-rust grammar, all node type mappings (function_item, let_declaration, match_expression, etc.) | `claude/echo-rust-lua-support` | Done |
| Lua language support — Language::Lua enum, tree-sitter-lua grammar, all node type mappings (function_declaration, local_function, function_call, dot_index_expression, etc.) | `claude/echo-rust-lua-support` | Done |
| Rust quantum async — tokio::spawn, thread::spawn, async/await, rayon::spawn | `claude/echo-rust-lua-support` | Done |
| Lua quantum async — coroutine.create/resume/wrap/yield | `claude/echo-rust-lua-support` | Done |

**Tests added:** Echo C caller without return check (1), echo C caller with return check (1), echo Go errors.Is (1), echo Python with-statement (1). Provenance transform≠form negative (1), provenance prefetch≠fetch negative (1). Rust basic parsing (1), Rust taint (1), Rust original_diff (1), Rust parent_function (1). Lua basic parsing (1), Lua taint exec (1), Lua parent_function (1), Lua absence open/close (1).

### Rust & Lua Algorithm Pattern Depth

| Item | Branch | Status |
|------|--------|--------|
| Rust taint sinks — transmute, from_raw_parts, write_volatile, read_volatile, from_utf8_unchecked, set_permissions, sql_query, query_as, deserialize, CString, CStr | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Lua taint sinks — loadstring, dofile, loadfile, format | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Rust provenance sources — std::io::stdin, BufReader, TcpStream, UdpSocket, hyper/axum/actix/rocket (user input); diesel, sqlx, sea_orm, rusqlite (database); serde_json/yaml, toml, config-rs, Figment (config); std::env::var, dotenvy (env) | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Lua provenance sources — io.read, io.stdin, socket.receive, ngx.req/var (user input); conn:execute, cursor:fetch, redis:get/hgetall (database); dofile, require (config); os.getenv (env) | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Rust absence pairs — File::open/create→drop/flush, Mutex::lock→drop, unsafe→assert/SAFETY comment, TcpListener/TcpStream→shutdown/drop, Command::new→status/output/spawn | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Lua absence pairs — io.open→close, socket.tcp/udp/connect→close, coroutine.create→resume | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Rust membrane error handling — ? operator, unwrap, expect, if let Err/Ok, match, map_err, Err() | `claude/fix-taint-patterns-tests-0fPSO` | Done |
| Lua membrane error handling — pcall, xpcall, assert, error() | `claude/fix-taint-patterns-tests-0fPSO` | Done |

**Tests added (17):** Rust taint transmute (1), Rust taint from_raw_parts (1), Rust provenance stdin (1), Rust provenance diesel (1), Rust provenance env_var (1), Rust absence file without flush (1), Rust absence command not executed (1), Rust absence unsafe without safety comment (1), Rust membrane error handling (1), Rust quantum tokio::spawn (1), Lua taint loadstring (1), Lua taint dofile (1), Lua provenance io.read (1), Lua provenance os.getenv (1), Lua provenance redis (1), Lua absence socket without close (1), Lua membrane pcall (1).

---

## Remaining Work

### P1 — Important (Reduces False Positive/Negative Rate)

| Item | Effort | Impact | Notes |
|------|--------|--------|-------|
| ~~**Function pointer Level 3: parameter-passed fptrs**~~ | — | — | **Done** — 1-hop interprocedural; `function_parameter_names()` + `call_argument_text_at()` in ast.rs, Level 3 loop in call_graph.rs. Composes with Level 1. 5 tests. |

### P2 — Valuable (Improves Analysis Depth)

| Item | Effort | Impact |
|------|--------|--------|
| ~~AccessPath type + field-sensitive DFG~~ | — | **Done** — Phase 1: AccessPath type with `from_expr()` parser. Phase 2: field assignments emit only qualified path (no base-only leakage). Field isolation verified across all 8 languages (C, C++, Python, JS, Go, Rust, Lua, Java, TS). 11 field isolation tests. |
| Virtual dispatch in C++ call graph | 1-2 weeks | Accurate analysis for C++ OOP polymorphism. Will be resolved via CPG Phase 4 + optional type enrichment (Phase 5). |
| ~~`va_list` taint tracking~~ | — | **Done** — v-variant format sinks (vprintf/vfprintf/vsprintf/vsnprintf) added to SINK_PATTERNS; variadic wrapper detection auto-discovers functions with `...` param that forward to format sinks and adds them as dynamic sinks. 4 tests. |
| ~~CVE-pattern test fixtures (format string, buffer overflow, integer overflow, double-free, use-after-free)~~ | — | **Done** — 8 tests: double-free goto, correct cleanup negative, double-unlock goto, format string, buffer overflow, strcpy+provenance, integer overflow, UAF. |
| ~~`goto`-based error path analysis for AbsenceSlice~~ | — | **Done** — `goto_statements()` + `label_sections()` in ast.rs; double-close detection in AbsenceSlice for kernel `goto cleanup` patterns. 3 tests. |
| ~~MembraneSlice C++ error handling (exceptions, RAII)~~ | — | **Done** — try/catch, throw, RAII smart ptrs, lock guards, std::optional/expected, error_code. 4 tests. |
| ~~Code Property Graph on petgraph~~ | — | **Done** — Phase 4 complete. `CodePropertyGraph::build()` constructs unified petgraph DiGraph from DFG+CallGraph. Edge-filtered reachability, `tarjan_scc`, `bfs_with_distance`, chop. All 12 algorithms migrated: `circular_slice`, `gradient_slice`, `chop`, `delta_slice`, `taint`, `provenance_slice`, `barrier_slice`, `vertical_slice`, `threed_slice`, `membrane_slice`, `echo_slice`, `spiral_slice`. The remaining 14 algorithms operate purely on AST structure and don't need graph traversal. |

### P3 — New Language Support (Procedural)

| Item | Effort | Priority | Status | Notes |
|------|--------|----------|--------|-------|
| ~~**Rust** (`tree-sitter-rust`)~~ | — | — | **Done** | Full algorithm coverage: taint, provenance, absence, membrane, quantum, echo |
| ~~**Lua** (`tree-sitter-lua`)~~ | — | — | **Done** | Full algorithm coverage: taint, provenance, absence, membrane, quantum, echo |
| **Terraform / HCL** (`tree-sitter-hcl` + `hcl-rs`) | 2-3 weeks | Must-have — team's own repos | Analysis complete | Taint through `var.`/`local.` to sensitive resource attrs; module membrane; provenance for `var.`/`data.`/`module.` origins. `hcl-rs` for reference resolution. Full plan: `docs/terraform-hcl-plan.md`. Test fixtures ready. |
| **Shell / Bash** (`tree-sitter-bash`) | 1-2 weeks | Should-have — firmware scripts | Analysis complete | Killer use case: command injection via unquoted `$var`. Covers Busybox/OpenWrt firmware scripts. Full plan: `docs/shell-bash-plan.md`. Test fixtures ready. |

### P4 — Declarative Format Context Extraction

These formats need a different analysis model: parse → find touched units → trace references → emit context. Not full slicing, but serves the same purpose of reducing what the LLM reviewer reads. Architecture decision: single binary with `prism slice` / `prism context` subcommands and Cargo feature flags. See `docs/language-expansion-plan.md` §4, §6.

| Item | Effort | Priority | Notes |
|------|--------|----------|-------|
| **Dockerfiles** (`dockerfile-parser-rs` + `docker-compose-types`) | 1-2 weeks | High — team's own repos | Multi-stage build dependency tracking, compose service graph, `ARG`/`ENV` propagation |
| **Protocol Buffers** (`tree-sitter-proto` + `protobuf-parse`) | 1 week | Medium — if gRPC IPC in firmware | Message reference graph, field number stability detection, service endpoint context |
| **YANG / NETCONF** (pyang + JSON tree) | 2-3 weeks | Medium-High — ROLT, RPD, CIN | Re-evaluated: critical for DOCSIS 4.0/WiFi 7 model changes. Shell out to pyang for resolution. Backward compat, leafref integrity, augmentation conflict detection. See `docs/access-network-analysis-evaluation.md` §2 |
| **Device Tree Source** (dtc + tree-sitter-devicetree) | 2-3 weeks | Medium — RPD, CPE | Re-evaluated: needed for WiFi 7 radio config and DOCSIS 4.0 FPGA config. Register overlap, interrupt conflict, compatible string validation. See `docs/access-network-analysis-evaluation.md` §3 |
| **Protocol Buffers** (`tree-sitter-proto` + `protobuf-parse`) | 1 week | Medium — vCMTS gRPC IPC | Message reference graph, field number stability detection, service endpoint context |
| Makefiles / CMake | — | Skip for now | Security flag auditing is rule-based, not context extraction |

### P5 — Analysis Infrastructure Improvements

| Item | Effort | Priority | Notes |
|------|--------|----------|-------|
| ~~Type enrichment via `compile_commands.json` + clang~~ | — | **Done** | Phase 5 complete. `TypeDatabase` parses `compile_commands.json`, shells out to `clang -Xclang -ast-dump=json`, extracts struct/class/union definitions, field types, typedefs, and class hierarchy. `CodePropertyGraph::build_with_types()` adds virtual dispatch Call edges via CHA. CLI: `--compile-commands <path>`. 15 tests (10 unit + 5 integration). |
| ~~Control flow graph edges in CPG~~ | — | **Done** | Phase 6 complete. `cfg.rs` builds intraprocedural CFG edges; `cpg.rs` creates Statement nodes and ControlFlow edges. PR A: core CFG (sequential flow, if/else, loops, goto). PR B: multi-language handlers (Python for/else + try/except, Go defer/select, Rust match, JS/Java try/catch/finally, C switch fall-through). PR C: algorithm integration — `taint_forward_cfg()` and `dfg_cfg_chop()` filter DFG results by CFG reachability, pruning dead-code paths. Known gaps: Go `fallthrough` keyword (sequential workaround), Lua pcall/xpcall (not modeled). Full plan: `docs/cpg-phase6-cfg-plan.md`. |
| ~~Local must-alias tracking~~ | — | **Done** | Phase 3: `ptr = dev` → `ptr->field` resolves to `dev->field`. Supports assignments and declarations with initializers. Chain resolution (a=b, b=c → a resolves to c). Tested across C, Python, JS, Go, Rust with chain and negative tests. 7 must-alias tests. |
| `oxc_parser` + `oxc_semantic` for JS/TS | 1-2 weeks | Medium | Scope-aware analysis eliminates false taint matches from same-named imports. 3-5x faster than tree-sitter |
| Preprocessor-aware analysis (`cpp -E`) | 2-4 weeks | Medium | Eliminates ERROR nodes from macro-heavy C/C++ code |
| Function-level git churn in ThreeDSlice | 1 week | Low | More accurate risk scores for large files |
| `syn`-based Rust semantic analysis | 1-2 weeks | Low — trigger if tree-sitter FP rate too high | `unsafe` block boundaries, lifetime annotations, trait resolution |
| `rayon` parallelization | 1 week | Low — trigger if >500 files becomes slow | Parallel file parsing across large firmware trees |

---

## Pre-handoff Architectural Baseline (D5: CWE Coverage)

The eval team's CWE coverage handoff (`~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md`) requests per-language sink taxonomy expansion across 6 CWE families, a category-aware sanitizer registry, and a framework-detection layer. None of those subsystems exist today. This section captures the starting state and tentative answers to the handoff's open questions, so the upcoming ACK doc has a baked baseline.

### Inventory of current source/sink/sanitizer infrastructure

- **Taint sources/sinks** live in `src/algorithms/taint.rs` (~641 lines). Sinks are encoded as language-keyed pattern arrays (`SINK_PATTERNS` and per-language extensions via `FORMAT_SINKS`). Sources are conservatively inferred from data-flow predecessors of diff lines, not from a registry. IPC sources added in T1-006 (`eebafb6`) as `IPC_SOURCE_PATTERNS`.
- **Provenance origins** live in `src/algorithms/provenance_slice.rs` (~704 lines). Origin classification: UserInput, Config, Database, Constant, EnvVar, FunctionParam, ExternalCall, Hardware, Unknown. `WEB_FRAMEWORK_MODULES` const lists Flask/Django/etc. for import-aware suppression. `PROVENANCE_OVERLAP_KEYWORDS` lists `request`, `req`, `form`, `query`, etc. that are suppressed when imported from non-web modules.
- **Sanitizer recognition:** none in algorithm logic. `provenance_slice.rs` has a `sanitize_line()` helper that strips suppressed import tokens from a line before pattern-matching — this is an import-token suppressor, not a security sanitizer. No `SanitizerRegistry`, `SanitizerKind`, or `cleansed_for` concept exists anywhere in `src/`.
- **Framework awareness:** the only piece is `WEB_FRAMEWORK_MODULES` in `provenance_slice.rs`, used for import-suppression. No detection layer activates per-framework source/sink overrides. No `FrameworkRegistry`, `detect_framework`, or `framework_for_file` function exists in `src/`.

### Tentative answers to handoff §10 open questions

- **Q1 Config vs code (source/sink/sanitizer registries):** Rust modules with declarative const arrays — matches the existing `taint.rs` / `provenance_slice.rs` pattern. The eval team's stated value of "add sources mid-run for debugging" can be served by a CLI passthrough (`--taint-source-extra`, `--taint-sink-extra`) that doesn't require config-file parsing. Type safety + fast path beats config-file flexibility for the volume of patterns expected.
- **Q2 Per-framework module structure:** per-framework modules under `src/frameworks/{flask,django,fastapi,express,nethttp,gin,gorilla_mux}.rs`, registered through a small `FrameworkRegistry` enum. Mirrors the existing `src/languages/<lang>.rs` shape.
- **Q3 Sanitizer granularity:** boolean cleansed/uncleansed per category. A `cleansed_for: BTreeSet<SanitizerCategory>` on taint values is sufficient; confidence values are unwarranted complexity for this round.
- **Q4 Phasing:** agree with the eval team — Phase 1 Go (CWE-78 + CWE-22 sinks + net/http framework + shell-escape/path-validation sanitizers) → Phase 2 Python (CWE-79/89/918/502 + Flask/Django/FastAPI + HTML-escape/SQL-parametrize/URL-allowlist/path-validation sanitizers) → Phase 3 JS (Express + same CWE coverage) → Phase 4 Java (stretch, Tier 2.6 alignment).
- **Q5 Unknown-framework default:** quiet mode (eval team's stated preference). Existing `provenance_slice` already uses import-suppression for the noisy case; that pattern generalizes.

### Phasing recommendation

| Phase | Scope | Estimate |
|---|---|---|
| Phase 0 | This hygiene pass (T1 algorithms with tests + hapi regression + this baseline) | Done |
| Phase 1 | Go CWE-78/22 + net/http framework + shell/path sanitizers (aligns with eval C1 fixtures) | 1-2 weeks |
| Phase 2 | Python CWE-79/89/918/502 sinks + Flask/Django/FastAPI detection + 4 sanitizer categories | 2-3 weeks |
| Phase 3 | JS for the same CWE coverage on Express | 1-2 weeks |
| Phase 4 stretch | Java + Spring (Tier 2.6) | TBD |

### Known cross-language gap notes (from this hygiene pass)

- `primitive_slice::detect_hardcoded_secret` only matches bare `NAME = "literal"` and `obj.field = "literal"` LHS forms. `const`/`let`/`var` (JS), `:=` (Go), and `static const char *` (C) all bypass the LHS-identifier check. Deferred rather than patched — the handoff's category-aware sanitizer/source registry is likely to subsume this rule entirely.

### Reference

- Handoff: `~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md`
- Eval-team Prism assessment: `~/code/agent-eval/analysis/prism-assessment.md`
- Eval-team algorithm matrix: `~/code/agent-eval/analysis/prism-algorithm-matrix.md`
- Implementation plan for this hygiene pass: `docs/superpowers/plans/2026-04-24-t1-hygiene-pre-cwe-handoff.md`
- Spec for this hygiene pass: `docs/superpowers/specs/2026-04-24-hygiene-pass-pre-cwe-handoff-design.md`

---

## Architecture Notes

### Key Design Decisions
- **Tree-sitter** for multi-language AST parsing (9 languages: Python, JS/TS, Go, Java, C/C++, Rust, Lua)
- **AccessPath-based variable tracking** — structured `{ base, fields }` replacing bare string names. Enables field-sensitive analysis.
- **Code Property Graph** — unified petgraph DiGraph merging DFG + call graph + containment edges. Edge-filtered traversals (SCC, reachability, hop-distance BFS). All 11 CPG-consuming algorithms migrated. Legacy DFG/CG retained as embedded fields for edge diffing (delta_slice) and call site line lookups. See `docs/cpg-architecture.md`
- **BTreeMap/BTreeSet everywhere** for deterministic sorted output
- **Shared infrastructure:** `call_graph.rs` and `data_flow.rs` retained as CPG internals; algorithms access them via `cpg.call_graph` and `cpg.dfg` when needed
- **Algorithm-specific configs** in each module, not in central `SliceConfig`
- **Single binary architecture** for future language expansion — `prism slice` (procedural) and `prism context` (declarative) subcommands with Cargo feature flags per language
- **Optional type enrichment** — `compile_commands.json` + clang for C/C++ struct/typedef info when available

### Known Limitations (C/C++)
- Pointer aliasing: local must-alias (Phase 3) handles `ptr = dev` intraprocedurally; interprocedural aliasing not yet tracked
- Function pointers: Level 0 (field-access), Level 1 (local fptr variable), Level 2 (dispatch tables), Level 3 (parameter-passed, 1-hop) resolved; Level 4 (full points-to) not implemented — see `docs/c-cpp/function-pointer-resolution.md`
- `static` function scope: disambiguated via `resolve_callees()` and `callers_of_in_file()`
- Interrupt handlers: detected by naming heuristic AND cross-file registration analysis (`signal()`, `pthread_create()`, `request_irq()`, `.sa_handler`, `std::thread`)
- Struct field flow: field-sensitive via AccessPath (CPG Phase 1-2 done). Phase 2 eliminates cross-field taint leakage. Phase 3 resolves pointer aliases.
- Virtual dispatch: name-matched, not type-resolved (CPG Phase 4-5)

### Known Limitations (Tree-sitter)
- No type information — mitigated by optional clang type enrichment (CPG Phase 5) and AccessPath field heuristics
- No import resolution — cross-file analysis uses name matching (static disambiguation for C/C++ only)
- No preprocessor handling — C/C++ macros produce ERROR nodes
- CFG is intraprocedural only — no interprocedural control flow (call/return edges exist separately). Known gaps: Go `fallthrough` keyword (sequential workaround), Lua pcall/xpcall (not modeled). Dominator analysis not yet implemented.
- No semantic scoping — `find_variable_references_scoped` handles some variable shadowing cases

### Test Coverage
- **1,406 tests** total (counts as of 2026-04-25; run `cargo test` for current)
- 11 languages covered (Python, JS, TS, Go, Java, C, C++, Rust, Lua, Terraform/HCL, Bash)
- Field isolation tests across all 8 field-capable languages
- Must-alias tests for C, Python, JS, Go, Rust with chain and negative cases
- 30 algorithms with at least basic coverage (T1-002 peer/callback and T1-005 primitive added in this branch)
- C/C++ specific: 32 tests covering taint, provenance, absence, quantum (incl. ISR self-detection), membrane, phantom, pointer aliasing, function pointer dispatch (Level 0/1/2), static linkage disambiguation
- Multi-language taint: 6 tests covering Python (pickle, subprocess), JS (innerHTML, execSync), Go (exec.Command, template.HTML)
- Multi-language provenance: 6 tests covering Python (request.form, cursor.fetchone), JS (document.cookie, process.env), Go (r.FormValue, viper config)
- Multi-language absence: 6 tests covering Python (threading.Lock, tempfile), JS (createReadStream, fs.open), Go (context.WithCancel, http.Get body)
- Multi-language quantum: 3 tests covering Python (threading.Thread), JS (Worker), Go (channel/select)
- Multi-language membrane: 2 tests covering Python (raise_for_status), Go (errors.Is)
- Echo slice: 4 tests (C return-code positive/negative, Go errors.Is, Python with-statement)
- Provenance precision: 2 negative tests (transform≠form, prefetch≠fetch)
- Rust: 14 tests (basic parsing, taint, original_diff, parent_function, taint transmute, taint from_raw_parts, provenance stdin, provenance diesel, provenance env_var, absence file, absence command, absence unsafe, membrane error handling, quantum tokio)
- Lua: 11 tests (basic parsing, taint exec, parent_function, absence open/close, quantum coroutine, taint loadstring, taint dofile, provenance io.read, provenance os.getenv, provenance redis, absence socket, membrane pcall)

---

## Reference

- **CPG architecture:** `docs/cpg-architecture.md` (AccessPath, Code Property Graph, type enrichment — design, phases 1-6 all done)
- **CPG Phase 6 plan:** `docs/cpg-phase6-cfg-plan.md` (control flow graph edges — completed, 3-PR summary)
- **CPG improvements:** `docs/cpg-improvements.md` (post-Phase 6: CpgContext build-once, JS/TS destructuring, tree-sitter struct fallback, RTA, Lua colon fix)
- **Terraform/HCL plan:** `docs/terraform-hcl-plan.md` (TerraformRefGraph architecture, algorithm mapping, dual-parser approach)
- **Shell/Bash plan:** `docs/shell-bash-plan.md` (taint sinks, unquoted variable detection, firmware-specific patterns)
- **Access network evaluation:** `docs/access-network-analysis-evaluation.md` (YANG/NETCONF, Device Tree, Busybox for ROLT/vCMTS/RPD/CIN/CPE, DOCSIS 4.0, WiFi 7)
- Language expansion plan: `docs/language-expansion-plan.md` (detailed analysis of all candidate languages, crates, architecture decisions)
- Gap analysis: `docs/prism-ccpp-gap-analysis.md`
- Algorithm taxonomy: `SLICING_METHODS.md`
- Paper: arXiv:2505.17928
