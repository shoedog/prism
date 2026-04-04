# Test Gap Analysis — Algorithm × Language Coverage

Generated 2026-04-04. Tracks untested code paths identified via coverage matrix
and source-level analysis of algorithm implementations.

## Coverage Matrix Summary

**916 tests, 256/324 cells covered (79%).** Python, TypeScript, and Go have
100% algorithm coverage.

## Tier 1 — Algorithm Logic Exercised but Language Path Untested

High-value gaps where algorithm code paths exist but have zero test coverage.

- [x] **1. Contract: Java / C++ / Lua** — 3 major languages with zero contract
  tests; guard detection is language-generic so these should work but aren't
  validated
- [x] **2. Absence: Bash patterns** — 6+ Bash-specific paired patterns exist in
  code (`mktemp`/`rm`, `mount`/`umount`, `pushd`/`popd`, `exec FD`/close,
  `flock`/unlock, `trap`/restore), zero tests
- [x] **3. Absence: Terraform S3/Lambda** — 3 IaC security patterns
  (encryption, public access block, versioning), zero tests
- [x] **4. Quantum: Rust async** (`tokio::spawn`, `thread::spawn`,
  `rayon::spawn`) — common Rust async pattern, untested
- [x] **5. Quantum: C++ `std::async` / `std::jthread`** — modern C++ async,
  untested
- [x] **6. Taint: JS basic sinks** (`innerHTML`, `execSync`) — JS taint has no
  dedicated language test

## Tier 2 — Cross-Cutting Feature Gaps

Patterns that span multiple languages or test important algorithm sub-features.

- [x] **7. Absence: JS timers** (`setInterval`/`clearInterval`) — very common
  bug pattern
- [x] **8. Absence: DB transactions** (`begin`/`commit`) — common across
  languages
- [x] **9. Absence: Go `context.WithTimeout`** — only `WithCancel` tested, not
  timeout/deadline variants
- [x] **10. Contract: Yoda conditions** — code handles `NULL == ptr`,
  `nil != err` but no test validates it
- [x] **11. Contract: Range/bounds checks** — `x < 0`, `x >= max` implemented
  but untested
- [x] **12. Quantum: Go channels** — `select`/`send`/`receive` patterns in
  code, untested
- [x] **13. Echo: Rust `?` operator** — very common Rust error propagation,
  untested

## Tier 3 — Completeness

Fill remaining cells in the coverage matrix.

- [x] **14. Provenance: C++** — only language with zero provenance coverage
- [x] **15. Absence: Event sub/unsub** — `subscribe`/`unsubscribe`
  cross-language pattern, no test
- [x] **16. Quantum: Python `asyncio.create_task`/`gather`** — beyond basic
  `await`

## Algorithm × Language Gap Matrix

Algorithms with ≥6 missing languages:

| Algorithm | Missing Languages | Gap Count |
|-----------|-------------------|-----------|
| ThreeDSlice | Java, C, C++, Rust, Lua, TF, TSX, Bash | 8 |
| ResonanceSlice | Java, C, C++, Rust, Lua, TF, TSX, Bash | 8 |
| PhantomSlice | Java, C, C++, Rust, Lua, TF, TSX, Bash | 8 |
| VerticalSlice | Java, C++, Rust, Lua, TF, TSX, Bash | 7 |
| ContractSlice | TS, TF, TSX, Bash | 4 |
| DeltaSlice | Java, Rust, Lua, TF, TSX, Bash | 6 |
| SpiralSlice | Java, Rust, Lua, TF, TSX, Bash | 6 |
| CircularSlice | Java, Rust, Lua, TF, TSX, Bash | 6 |
| GradientSlice | Java, C++, Rust, Lua, TF, Bash | 6 |

Note: ThreeDSlice, ResonanceSlice, and PhantomSlice require git history setup,
making them inherently harder to test.

## Language Gap Summary

| Language | Gaps | Notable Missing |
|----------|------|-----------------|
| Python | 0 | — |
| TypeScript | 0 | — |
| Go | 0 | — |
| Bash | 15 | Contract, Conditioned, Membrane |
| Terraform | 14 | Most taxonomy + theoretical |
| TSX | 12 | Taint, Contract, Conditioned |
| Java | 12 | Contract, Symmetry, many theoretical |
| Lua | 10 | Contract, Chop, most theoretical |

## Untested Code Paths (Source-Level)

### Absence Slice (`src/algorithms/absence_slice.rs`)

62 paired patterns in `default_pairs()`. Key untested paths:

| Pattern | Language | Tested? |
|---------|----------|---------|
| `pool.apply_async`/`pool.close` | Python | No |
| `setInterval`/`clearInterval` | JS | Yes (item 7) |
| `createServer`/`server.close` | Node.js | No |
| `pool.connect`/`client.release` | DB (multi) | No |
| `sql.Open`/`db.Close` | Go | No |
| `context.WithTimeout`/`cancel` | Go | Yes (item 9) |
| `TcpListener::bind`/`.shutdown` | Rust | No |
| `subscribe`/`unsubscribe` | Multi | Yes (item 15) |
| `beginTransaction`/`commit` | Multi | Yes (item 8) |
| All 6 Bash pairs | Bash | Partial (3/6: mktemp, pushd, exec FD) |
| All 3 Terraform pairs | Terraform | Partial (2/3: encryption, public access) |

### Quantum Slice (`src/algorithms/quantum_slice.rs`)

| Pattern | Language | Tested? |
|---------|----------|---------|
| `asyncio.create_task`, `gather` | Python | Yes (item 16) |
| `Promise`, `Worker`, `nextTick` | JS | No |
| channel ops (select/send/recv) | Go | Yes (item 12) |
| `std::async`, `std::jthread` | C++ | Yes (item 5) |
| `tokio::spawn`, `rayon::spawn` | Rust | Yes (item 4) |
| `coroutine.wrap`, `yield` | Lua | No |
| `nohup`, `coproc`, `&` | Bash | No |

### Contract Slice (`src/algorithms/contract_slice.rs`)

| Pattern | Language | Tested? |
|---------|----------|---------|
| Yoda conditions (`NULL == ptr`) | Any | Yes (item 10) |
| Range checks (`x < 0`) | Any | Yes (item 11) |
| `len(x) == 0` / `.length === 0` | Python/JS | Yes (item 11) |
| `assert_eq!`, `debug_assert!` | Rust | No |
| All guard types | Java | Yes (item 1) |
| All guard types | C++ | Yes (item 1) |
| All guard types | Lua | Yes (item 1) |
