# Spec: Algorithm × Language Coverage Expansion

**Date:** April 6, 2026
**Status:** Handoff -- ready for implementation
**Goal:** Increase algorithm × language test coverage from ~79% to ≥95%,
prioritizing languages used in CPE firmware and access network code review

-----

## 1. Current State

27 algorithms × 12 languages = 324 cells. TEST_GAPS.md (from PR 50-51)
reported 256/324 covered (79%). Since then, PRs 53-69 added type provider
tests and CVE fixtures but did NOT add algorithm × language coverage tests.
The gap matrix from TEST_GAPS.md is still the accurate picture for algorithm
coverage.

**Languages at 100%:** Python, TypeScript, Go

**Languages with significant gaps:**

|Language |Coverage   |Gap Count|Priority for CPE/Access                    |
|---------|-----------|---------|-------------------------------------------|
|C        |85% (22/27)|5        |**Critical** -- firmware, drivers           |
|C++      |77% (20/27)|7        |**Critical** -- data planes, vCMTS          |
|Java     |65% (17/27)|10       |**High** -- enterprise applications            |
|Rust     |69% (18/27)|9        |**Medium** -- future kernel drivers         |
|Lua      |65% (17/27)|10       |**High** -- LuCI (OpenWrt CPE UI)           |
|Bash     |54% (14/27)|13       |**Critical** -- firmware init/config scripts|
|Terraform|50% (13/27)|14       |**Medium** -- infra provisioning            |
|TSX      |~50%       |~12      |**Low** -- React components                 |

-----

## 2. Priority Tiers

### Tier 1: CPE Firmware Languages (C, C++, Bash)

These are the primary languages in DOCSIS modems, WiFi routers, ONUs, RPDs,
and ROLT platforms. Every algorithm should work on these.

**C gaps (5 missing):**

|Algorithm     |Why It Matters for Firmware                          |Fixture Design                                                  |
|--------------|-----------------------------------------------------|----------------------------------------------------------------|
|SpiralSlice   |Adaptive depth around changed firmware functions     |Modified ISR handler → trace callers/callees at increasing depth|
|ThreeDSlice   |Temporal risk on actively-changing firmware files    |Requires git history setup -- use existing git_dir pattern       |
|ResonanceSlice|Co-change coupling in firmware modules               |Same as ThreeDSlice -- git history                               |
|PhantomSlice  |Recently deleted firmware code that callers depend on|Deleted function, caller still references it                    |
|ContractSlice |Already partially tested -- fill remaining patterns   |C-style null guards with `NULL == ptr` Yoda                     |

**C++ gaps (7 missing):**

|Algorithm     |Fixture Design                                  |
|--------------|------------------------------------------------|
|SpiralSlice   |Virtual method change → trace call chain        |
|ThreeDSlice   |Git history (shared with C)                     |
|ResonanceSlice|Git history (shared with C)                     |
|PhantomSlice  |Deleted method, caller still uses base class    |
|ContractSlice |C++ exceptions in guard, `std::optional` returns|
|DeltaSlice    |Changed method signature between old/new        |
|VerticalSlice |End-to-end feature path through class hierarchy |

**Bash gaps (13 missing):**

|Algorithm       |Firmware Relevance                      |Fixture Design                        |
|----------------|----------------------------------------|--------------------------------------|
|LeftFlow        |Variable tracing in init scripts        |`$CONFIG_FILE` → `source` → `eval`    |
|FullFlow        |Complete data flow in update scripts    |Firmware update script with hash check|
|RelevantSlice   |Alternate branches in error handling    |`if [ -f "$FW" ]; then ... else ...`  |
|ConditionedSlice|Conditional execution paths             |`if [ "$MODE" = "bridge" ]; then ...` |
|BarrierSlice    |Call depth in script sourcing chains    |`source ./lib.sh` → function calls    |
|Chop            |Data flow between source and sink       |`$INPUT` flows through to `iptables`  |
|DeltaSlice      |Changed script behavior between versions|Modified init script                  |
|SpiralSlice     |Adaptive depth around changed function  |Multi-function script                 |
|CircularSlice   |Recursive function calls                |`process() { ... process ... }`       |
|HorizontalSlice |Peer pattern detection                  |Multiple similar init scripts         |
|MembranSlice    |Module boundary (script sourcing)       |Functions exported via `source`       |
|EchoSlice       |Caller of changed function              |Modified shared library function      |
|ContractSlice   |Guard validation in scripts             |`[ -z "$VAR" ] && exit 1` patterns    |

### Tier 2: Application Languages (Java, Lua)

**Java gaps (10 missing):**

|Algorithm     |Fixture Design                     |
|--------------|-----------------------------------|
|SpiralSlice   |Changed method → trace call chain  |
|ThreeDSlice   |Git history                        |
|ResonanceSlice|Git history                        |
|PhantomSlice  |Deleted interface method           |
|ContractSlice |`Objects.requireNonNull()` guards  |
|DeltaSlice    |Changed method signature           |
|CircularSlice |Circular dependency between classes|
|VerticalSlice |Controller → Service → DAO path    |
|SymmetrySlice |Serialize/deserialize pair         |
|GradientSlice |Distance-based relevance scoring   |

**Lua gaps (10 missing):**

|Algorithm       |CPE Relevance (LuCI)              |Fixture Design                         |
|----------------|----------------------------------|---------------------------------------|
|ConditionedSlice|Conditional config rendering      |`if config.enabled then ...`           |
|ContractSlice   |Input validation in LuCI handlers |`if not value then error() end`        |
|Chop            |Data flow in LuCI form processing |Form input → UCI write                 |
|DeltaSlice      |Changed LuCI handler              |Modified form handler                  |
|SpiralSlice     |Changed function call depth       |Multi-level LuCI dispatch              |
|CircularSlice   |Circular module requires          |`local M = require("mod"); mod.init(M)`|
|BarrierSlice    |Call depth in module chain        |Module A → B → C                       |
|MembranSlice    |Module boundary                   |LuCI controller/model boundary         |
|HorizontalSlice |Peer handlers                     |Multiple similar CBI models            |
|VerticalSlice   |Request → controller → model → UCI|LuCI MVC path                          |

### Tier 3: Infrastructure Languages (Rust, Terraform, TSX)

Lower priority -- these are either future languages (Rust in firmware) or
have less direct relevance to CPE review. Fill as capacity allows.

-----

## 3. Implementation Strategy

### 3.1 Git-History Algorithms (ThreeDSlice, ResonanceSlice, PhantomSlice)

These three algorithms require a git repository with commit history. They
account for 3 × 9 = 27 missing cells across languages. A shared test
fixture approach:

```rust
/// Create a temp git repo with 3 commits for algorithm testing.
fn make_git_test_repo(lang: Language, ext: &str) -> (TempDir, BTreeMap<String, ParsedFile>, ...) {
    let dir = TempDir::new().unwrap();
    // Commit 1: initial version
    // Commit 2: add function B that calls A
    // Commit 3: modify function A (diff target)
    // Return files parsed at HEAD + git_dir path
}
```

One fixture per language × 3 algorithms = 27 tests with shared setup.
**Estimated effort: 1-2 days** (mostly boilerplate).

### 3.2 Smoke Tests vs Behavioral Tests

For Tier 1 (C, C++, Bash), write **behavioral tests** -- assertions on
specific findings, block contents, or line inclusion. These validate that
firmware-relevant patterns are correctly analyzed.

For Tier 2-3, **smoke tests** are acceptable for most algorithms -- verify
the algorithm runs without crashing and produces blocks. Add behavioral
tests only for language-specific patterns that are likely failure points.

### 3.3 Firmware-Specific Test Fixtures

Create a shared firmware fixture set in `tests/fixtures/firmware/`:

```
tests/fixtures/firmware/
  init_script.sh        # OpenWrt-style procd init
  sysupgrade.sh         # Firmware update script
  network_config.sh     # VLAN/bridge config with taint sources
  device_driver.c       # Simplified kernel driver pattern
  docsis_mac.c          # DOCSIS MAC scheduler stub
  wifi_driver.c         # WiFi 7 driver configuration stub
  luci_controller.lua   # LuCI CBI model
  vlan_handler.java     # Java VLAN management service
```

These fixtures serve double duty: algorithm coverage tests AND
domain-specific pattern validation for CPE review.

-----

## 4. Phased Delivery

### Phase 1: Critical Coverage (C, C++, Bash) -- 3-5 days

**Target: 100% algorithm coverage for firmware languages.**

1. Implement git-history test helper.
1. Add ThreeDSlice, ResonanceSlice, PhantomSlice for C, C++, Bash (9 tests).
1. Add remaining C gaps: SpiralSlice, ContractSlice (2 tests).
1. Add remaining C++ gaps: SpiralSlice, ContractSlice, DeltaSlice,
   VerticalSlice (4 tests).
1. Add remaining Bash gaps: 13 tests with firmware-relevant fixtures.

**Total: ~28 tests. C→100%, C++→100%, Bash→100%.**

### Phase 2: Application Coverage (Java, Lua) -- 2-3 days

1. Add git-history tests for Java, Lua (6 tests).
1. Add remaining Java gaps (7 non-git tests).
1. Add remaining Lua gaps (7 non-git tests) with LuCI fixtures.

**Total: ~20 tests. Java→100%, Lua→100%.**

### Phase 3: Completeness (Rust, Terraform, TSX) -- 2-3 days

1. Add git-history tests for Rust, Terraform, TSX (9 tests).
1. Fill remaining Rust gaps (6 non-git tests).
1. Fill remaining Terraform gaps (11 non-git tests).
1. Fill remaining TSX gaps (9 non-git tests).

**Total: ~35 tests. All languages→100%.**

-----

## 5. Firmware-Specific Patterns to Add

Beyond test coverage, these source code additions improve Prism's
effectiveness for CPE firmware review:

### 5.1 Bash/Busybox Taint Sinks (add to `taint.rs`)

```rust
// Firmware-critical command injection sinks
"mtd",         // Flash partition write -- device bricking
"fw_setenv",   // U-Boot env modification -- boot-loop
"uci",         // OpenWrt UCI config write -- persistent injection
"iptables",    // Firewall rule modification -- security bypass
"ip6tables",
"brctl",       // Bridge configuration -- VLAN hopping
"vconfig",     // VLAN configuration -- network segmentation bypass
"insmod",      // Kernel module loading -- rootkit installation
"modprobe",
"swconfig",    // Switch chip configuration -- L2 manipulation
```

### 5.2 Bash/Busybox Absence Pairs (add to `absence_slice.rs`)

```rust
// Firmware update: hash check before flash write
PairedPattern {
    open_patterns: vec!["mtd write", "mtd -r write"],
    close_patterns: vec!["sha256sum", "md5sum", "openssl dgst"],
    description: "Firmware flash without hash verification",
},
// Kernel module: insmod without error check
PairedPattern {
    open_patterns: vec!["insmod ", "modprobe "],
    close_patterns: vec!["lsmod | grep", "$? -ne 0", "$? != 0"],
    description: "Kernel module load without verification",
},
```

### 5.3 C/C++ Firmware Patterns

```rust
// DMA buffer: alloc without free
PairedPattern {
    open_patterns: vec!["dma_alloc_coherent(", "dma_pool_alloc("],
    close_patterns: vec!["dma_free_coherent(", "dma_pool_free("],
    description: "DMA buffer allocated without free",
},
// Spinlock in interrupt context
PairedPattern {
    open_patterns: vec!["spin_lock_irqsave("],
    close_patterns: vec!["spin_unlock_irqrestore("],
    description: "IRQ-safe spinlock without restore",
},
// MMIO: ioremap without iounmap
PairedPattern {
    open_patterns: vec!["ioremap(", "devm_ioremap("],
    close_patterns: vec!["iounmap("],
    description: "MMIO region mapped without unmap (non-devm)",
},
```

### 5.4 Estimated Effort for Pattern Additions

|Item                        |Effort  |Tests           |
|----------------------------|--------|----------------|
|Bash firmware sinks         |1 hour  |+5 taint tests  |
|Bash firmware absence pairs |2 hours |+4 absence tests|
|C/C++ kernel driver patterns|2 hours |+6 absence tests|
|**Total**                   |~5 hours|+15 tests       |

-----

## 6. Success Metrics

|Metric                            |Current       |Target        |
|----------------------------------|--------------|--------------|
|Algorithm × language cells covered|~256/324 (79%)|324/324 (100%)|
|C algorithm coverage              |85%           |100%          |
|C++ algorithm coverage            |77%           |100%          |
|Bash algorithm coverage           |54%           |100%          |
|Java algorithm coverage           |65%           |100%          |
|Lua algorithm coverage            |65%           |100%          |
|Firmware-specific taint sinks     |0             |10+           |
|Firmware-specific absence pairs   |0             |6+            |
|Total new tests                   |0             |~85-100       |

**Estimated total effort:** 2-3 weeks across all three phases.
Phase 1 (firmware languages) can be done in 3-5 days and delivers the
highest-value coverage for CPE review.
