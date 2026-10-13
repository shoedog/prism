# C1 — Memory Measurement Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Measure where one `prism-mcp` index's memory goes, on five corpora, so the reduction candidates M1–M3 are ranked by evidence before anyone builds one.

**Architecture:** A measurement-only mode of the `prism-mcp` binary (`PRISM_MCP_MEM_REPORT=1 … --eager`) builds the index in observed stages, optionally runs a fixed query workload through the in-process transport and one refresh, samples process memory at each stage, counts the components that explain the footprint, prints one JSON line to stderr and exits without serving. A Python driver runs that mode four ways, three times each, per corpus, rejects inadmissible runs, and renders the readout.

**Tech Stack:** Rust 2021, `libc` (new optional dependency of the `mcp` feature), `serde`/`serde_json`, Python 3.9 standard library for the driver.

**Spec:** `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` (§6.1, §6.2, §12 row C1). Lane plan: `docs/superpowers/plans/2026-10-12-shared-prism-daemon/README.md`.

## Global Constraints

- All Rust work is under `--features mcp`; the default build must still compile (`cargo check`).
- **Measurement-only.** With `PRISM_MCP_MEM_REPORT` unset and no `--mem-*` flag, `prism-mcp` behaves exactly as before: same stdout, same stderr, same exit codes. `SessionProvider::bootstrap` keeps its signature and behavior.
- Report mode never serves MCP and never reads stdin.
- The harness is **coarse triage** (spec §6): it ranks candidates for an experiment; it does not select one. No code in this slice changes how the index is built, stored or queried.
- Attribution is by **component counts**, never by drop-and-measure deltas: freed memory can stay in the footprint (measured 2026-10-12; lane plan).
- Pinned experiment parameters (spec §6.1): footprint metric `phys_footprint` (resident size where the platform has no footprint), sampling window after the workload and after a 5 s settle, three repetitions reported as median with min and max.
- Decision rule (spec §6.2), as a prioritization heuristic only: a candidate advances to its experiment if the readout shows ≥ 15 % of steady-state footprint or ≥ 200 MiB on at least two of the five corpora; for M2, ≥ 15 % of peak or ≥ 200 MiB.
- Corpora, pinned: slicing (a clean worktree of `origin/main`), and `~/code/bench-repos/` ruff `44f6d1865fd2`, excalidraw `0642e72cfa2d`, prometheus `505095b64b43`, kubernetes `8d34ac5ef882`.
- Does not touch `src/call_graph.rs`, `src/navigation/`, `src/cpg/` or `src/ast.rs` (no Tier-A run is triggered), and does not touch `src/mcp/transport.rs` (S1 is refactoring it in parallel). It uses only `crate::mcp::transport::{serve_session_with_freshness, Transport, ReadOutcome}`, which S1 keeps.
- New files stay under 600 lines. Python must run on 3.9 (no `match`, no `X | Y` annotations).

## Review Focus

1. **`PRISM_MCP_MEM_REPORT=1` is still exported when someone starts a normal server.** They must get one clear line naming the variable, not a silent non-server. → Task 5, `report_mode_without_eager_fails_naming_the_variable` and the announcement line asserted in `report_mode_prints_one_report_and_does_not_serve`.
2. **A workload whose seeds do not exist in this corpus or build.** The failures are counted and the run is rejected as inadmissible, never averaged in. → Task 4, `workload_errors_are_counted`; Task 6, `test_run_with_workload_errors_is_inadmissible`.
3. **A "warm" run that silently missed the cache.** It measured a cold build; reject it. → Task 6, `test_warm_run_that_missed_the_cache_is_inadmissible`.
4. **A corpus that does not finish.** Recorded as a timeout; the other runs proceed. → Task 6, `test_timeout_is_recorded_and_does_not_raise`.
5. **A platform without footprint metrics.** The report is still produced with those fields null, and the driver falls back to resident size. → Task 1, `parse_proc_status_reads_resident_and_peak`; Task 6, `test_footprint_falls_back_to_rss`.

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `Cargo.toml` | `libc` as an optional dependency enabled by `mcp` | Modify (Task 1) |
| `src/mcp/mem_sample.rs` | One process-memory sample: resident, footprint, compressed, lifetime peak; `ru_maxrss` | Create (Task 1) |
| `src/mcp/mem_attribution.rs` | Component counts from a built session | Create (Task 2) |
| `src/mcp/session.rs` | `BuildStage`, `SessionProvider::bootstrap_observed` | Modify (Task 3) |
| `src/cpg_cache.rs` | `current_cache_format_version()` accessor | Modify (Task 4) |
| `src/mcp/mem_report.rs` | Staged run, workload transport, workload file loading and generation, the report | Create (Task 4) |
| `src/mcp/mod.rs` | Module list and re-exports | Modify (Tasks 1–4) |
| `src/bin/prism-mcp.rs` | Hidden `--mem-*` flags and the mode switch | Modify (Task 5) |
| `tests/mcp/mem_report_test.rs` | Binary-level tests | Create (Task 5) |
| `tests/mcp/main.rs` | Umbrella module list | Modify (Task 5) |
| `scripts/memreport/memreport.py` | Driver: `gen-workload`, `run`, `render` | Create (Task 6) |
| `scripts/memreport/test_memreport.py` | Driver unit tests | Create (Task 6) |
| `scripts/memreport/README.md` | How to run it and what a run is evidence of | Create (Task 6) |
| `scripts/memreport/workloads/*.jsonl`, `MANIFEST.json` | The fixed per-corpus workloads | Create (Task 7) |
| `docs/analysis/2026-10-prism-mcp-memory-readout.md` | The five-corpus readout | Create (Task 7) |

`tests/mcp/main.rs` is also edited by S1. Whichever PR merges second keeps every `mod` line from both.

---

### Task 1: Process-memory sampler

**Files:**
- Modify: `Cargo.toml`
- Create: `src/mcp/mem_sample.rs`
- Modify: `src/mcp/mod.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)] pub struct MemSample { pub rss_bytes: Option<u64>, pub phys_footprint_bytes: Option<u64>, pub compressed_bytes: Option<u64>, pub peak_bytes: Option<u64> }`
  - `pub fn sample() -> MemSample`
  - `pub fn ru_maxrss_bytes() -> Option<u64>`

`peak_bytes` is the process-lifetime peak: peak footprint on macOS (`ri_lifetime_max_phys_footprint`), peak resident size on Linux (`VmHWM`). It only rises, so the stage in which it rose is the stage that set the peak.

- [ ] **Step 1: Add the dependency**

In `Cargo.toml` change the feature and add the dependency (keep `[dependencies]` in its existing order; `libc` goes with the other small utility crates):

```toml
[features]
mcp = ["dep:libc"]
```

```toml
libc = { version = "0.2", optional = true }
```

Run: `cargo check --features mcp > /tmp/c1-t1-dep.log 2>&1; tail -1 /tmp/c1-t1-dep.log; git diff --stat Cargo.lock`

Expected: `Finished`. `Cargo.lock` gains only a `"libc"` entry in the `prism` package's dependency list; `libc` 0.2.183 was already locked transitively, so no version line changes. If any other package version changes, stop and report it.

- [ ] **Step 2: Write the failing tests**

Create `src/mcp/mem_sample.rs` with only the tests and the type:

```rust
//! One sample of this process's memory, for the memory report (shared-daemon spec §6.1).
//! macOS: `task_info(TASK_VM_INFO)` for resident, footprint and compressed bytes, and
//! `proc_pid_rusage` for the lifetime peak footprint. Linux: `/proc/self/status`. Elsewhere every
//! field is `None` and the report is still produced.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct MemSample {
    /// Resident set size.
    pub rss_bytes: Option<u64>,
    /// Physical footprint (macOS): what the system charges the process, compressed pages included.
    pub phys_footprint_bytes: Option<u64>,
    /// Bytes held by the compressor on this process's behalf (macOS).
    pub compressed_bytes: Option<u64>,
    /// Process-lifetime peak: footprint on macOS, resident size on Linux. Never decreases.
    pub peak_bytes: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proc_status_reads_resident_and_peak() {
        let status = "Name:\tprism-mcp\nVmPeak:\t  999999 kB\nVmHWM:\t    2048 kB\nVmRSS:\t    1024 kB\nThreads:\t4\n";
        assert_eq!(
            parse_proc_status(status),
            MemSample {
                rss_bytes: Some(1024 * 1024),
                phys_footprint_bytes: None,
                compressed_bytes: None,
                peak_bytes: Some(2048 * 1024),
            }
        );
        assert_eq!(parse_proc_status("Name:\tx\n"), MemSample::default());
        assert_eq!(parse_proc_status("VmRSS:\tnot-a-number kB\n").rss_bytes, None);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn sample_tracks_a_touched_allocation() {
        const SIZE: usize = 64 << 20;
        let before = sample();
        let mut block = vec![0u8; SIZE];
        for offset in (0..SIZE).step_by(4096) {
            block[offset] = (offset % 251) as u8 + 1;
        }
        let after = sample();
        // Keep the block alive and observed past the second sample.
        assert!(block.iter().step_by(1 << 20).map(|byte| *byte as u64).sum::<u64>() > 0);

        let grew = after.rss_bytes.expect("resident size") as i64
            - before.rss_bytes.expect("resident size") as i64;
        assert!(
            grew >= (32 << 20),
            "touching 64 MiB should add at least 32 MiB of resident memory, added {grew}"
        );
        let peak = after.peak_bytes.expect("lifetime peak");
        assert!(peak >= before.peak_bytes.expect("lifetime peak"), "the peak never decreases");
        #[cfg(target_os = "macos")]
        {
            let footprint = after.phys_footprint_bytes.expect("footprint on macOS");
            assert!(footprint >= (32 << 20) as u64);
            assert!(peak >= footprint, "lifetime peak {peak} is below current footprint {footprint}");
            assert!(after.compressed_bytes.is_some());
        }
    }

    #[cfg(unix)]
    #[test]
    fn ru_maxrss_is_reported_in_bytes() {
        let max = ru_maxrss_bytes().expect("getrusage on unix");
        let now = sample().rss_bytes.unwrap_or(0);
        // A value in kilobytes would be about a thousand times smaller than the resident size.
        assert!(max >= now / 2, "ru_maxrss {max} is implausibly small against rss {now}");
    }
}
```

In `src/mcp/mod.rs` add, in alphabetical position (after `lazy`):

```rust
pub mod mem_sample;
```

- [ ] **Step 3: Run and see them fail**

Run: `cargo test --features mcp --lib mcp::mem_sample > /tmp/c1-t1-red.log 2>&1; grep -E 'error\[|cannot find' /tmp/c1-t1-red.log | head`

Expected: compile errors for `parse_proc_status`, `sample` and `ru_maxrss_bytes` not found.

- [ ] **Step 4: Implement the sampler**

Insert between the struct and the test module in `src/mcp/mem_sample.rs`:

```rust
/// Sample this process now.
pub fn sample() -> MemSample {
    imp::sample()
}

/// Peak resident set size from `getrusage`, in bytes (macOS reports bytes, Linux kilobytes).
pub fn ru_maxrss_bytes() -> Option<u64> {
    #[cfg(unix)]
    {
        // SAFETY: `usage` is a writable `rusage`; `getrusage` fills it or returns non-zero.
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } != 0 {
            return None;
        }
        let raw = u64::try_from(usage.ru_maxrss).ok()?;
        Some(if cfg!(target_os = "macos") {
            raw
        } else {
            raw.saturating_mul(1024)
        })
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Parse the two fields the report uses out of `/proc/self/status`.
#[cfg(any(target_os = "linux", test))]
fn parse_proc_status(text: &str) -> MemSample {
    let kilobytes = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|rest| rest.trim().strip_suffix("kB"))
            .and_then(|number| number.trim().parse::<u64>().ok())
            .map(|kb| kb.saturating_mul(1024))
    };
    MemSample {
        rss_bytes: kilobytes("VmRSS:"),
        phys_footprint_bytes: None,
        compressed_bytes: None,
        peak_bytes: kilobytes("VmHWM:"),
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::MemSample;

    /// `task_vm_info` through `phys_footprint`: the rev1 layout in `<mach/task_info.h>`, which is
    /// declared under `#pragma pack(4)`. The `libc` crate has no binding for this struct. The
    /// kernel fills as many fields as the passed count covers.
    #[repr(C, packed(4))]
    #[derive(Default, Clone, Copy)]
    struct TaskVmInfoRev1 {
        virtual_size: u64,
        region_count: i32,
        page_size: i32,
        resident_size: u64,
        resident_size_peak: u64,
        device: u64,
        device_peak: u64,
        internal: u64,
        internal_peak: u64,
        external: u64,
        external_peak: u64,
        reusable: u64,
        reusable_peak: u64,
        purgeable_volatile_pmap: u64,
        purgeable_volatile_resident: u64,
        purgeable_volatile_virtual: u64,
        compressed: u64,
        compressed_peak: u64,
        compressed_lifetime: u64,
        phys_footprint: u64,
    }

    /// `TASK_VM_INFO` in `<mach/task_info.h>`.
    const TASK_VM_INFO: libc::task_flavor_t = 22;

    pub(super) fn sample() -> MemSample {
        let mut out = MemSample::default();

        let mut info = TaskVmInfoRev1::default();
        let mut count = (std::mem::size_of::<TaskVmInfoRev1>()
            / std::mem::size_of::<libc::natural_t>())
            as libc::mach_msg_type_number_t;
        // SAFETY: `info` is a writable buffer of `count` natural_t units whose layout matches the
        // flavor; the kernel writes at most `count` units.
        #[allow(deprecated)] // libc points to the `mach2` crate; one call does not justify a dependency.
        let status = unsafe {
            libc::task_info(
                libc::mach_task_self(),
                TASK_VM_INFO,
                (&mut info as *mut TaskVmInfoRev1).cast(),
                &mut count,
            )
        };
        if status == libc::KERN_SUCCESS {
            // Copy out of the packed struct before use; references to packed fields are not allowed.
            let (resident, footprint, compressed) =
                (info.resident_size, info.phys_footprint, info.compressed);
            out.rss_bytes = Some(resident);
            out.phys_footprint_bytes = Some(footprint);
            out.compressed_bytes = Some(compressed);
        }

        // SAFETY: `usage` is a writable `rusage_info_v4`, and the flavor names that layout.
        let mut usage: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
        let status = unsafe {
            libc::proc_pid_rusage(
                libc::getpid(),
                libc::RUSAGE_INFO_V4,
                (&mut usage as *mut libc::rusage_info_v4).cast(),
            )
        };
        if status == 0 {
            out.peak_bytes = Some(usage.ri_lifetime_max_phys_footprint);
        }
        out
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::MemSample;

    pub(super) fn sample() -> MemSample {
        std::fs::read_to_string("/proc/self/status")
            .map(|text| super::parse_proc_status(&text))
            .unwrap_or_default()
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod imp {
    use super::MemSample;

    pub(super) fn sample() -> MemSample {
        MemSample::default()
    }
}
```

This layout and these calls were compiled and run on this host on 2026-10-12 in a scratch program: resident size matched `ps -o rss` exactly (1,687,552 bytes idle; 211,533,824 after touching 200 MiB) and the footprint rose by the allocation.

- [ ] **Step 5: Run the tests**

Run: `cargo test --features mcp --lib mcp::mem_sample > /tmp/c1-t1-green.log 2>&1; grep -E '^test |^test result' /tmp/c1-t1-green.log`

Expected: 3 passed on macOS and Linux.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add Cargo.toml Cargo.lock src/mcp/mem_sample.rs src/mcp/mod.rs
git commit -m "feat(mcp): process-memory sampler for the memory report"
```

---

### Task 2: Component attribution counters

**Files:**
- Create: `src/mcp/mem_attribution.rs`
- Modify: `src/mcp/mod.rs`

**Interfaces:**
- Consumes: `crate::navigation::NavigationSession` (`repo: Arc<LoadedRepo>`, `index: Arc<NavigationIndex>`), `crate::cpg::{CpgNode, StmtKind}`, `ParsedFile::{source, parse_node_count}`, `CodePropertyGraph::{node_indices, node, node_count, edge_count}`.
- Produces:
  - `#[derive(Debug, Clone, PartialEq, serde::Serialize)] pub struct Attribution { … }` (fields in Step 1)
  - `pub fn attribute(session: &NavigationSession) -> Attribution`

Estimates and what they stand for:

- **Tree bytes** are bracketed, not measured. tree-sitter 0.25.10 (`src/subtree.h`) stores a small leaf inline in an 8-byte `Subtree` slot in its parent, and every other node as an 80-byte `SubtreeHeapData` plus that 8-byte slot. So a node costs between 8 and 88 bytes before allocator rounding. `parse_node_count` is the node count. The M1 experiment measures the real saving.
- **String heap bytes** assume each non-empty `String` is its own allocation rounded up to 16 bytes (the macOS and glibc small-allocation quantum). The M3 experiment measures the real saving.

- [ ] **Step 1: Write the failing tests**

Create `src/mcp/mem_attribution.rs`:

```rust
//! Component counts that attribute an index's memory independently of resident-size deltas
//! (shared-daemon spec §6.1). Counts are exact; byte figures derived from them are estimates and
//! say so in their names.
use crate::cpg::{CpgNode, StmtKind};
use crate::navigation::NavigationSession;
use std::collections::HashSet;

/// Lower bound per tree-sitter node: an inline subtree is one 8-byte slot in its parent.
pub const TREE_NODE_BYTES_LOW: u64 = 8;
/// Upper bound per tree-sitter node before allocator rounding: an 80-byte `SubtreeHeapData`
/// plus its 8-byte slot (tree-sitter 0.25.10, `src/subtree.h`).
pub const TREE_NODE_BYTES_HIGH: u64 = 88;
/// Small-allocation quantum assumed for string heap estimates.
const STRING_HEAP_QUANTUM: u64 = 16;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Attribution {
    pub files: u64,
    pub source_bytes: u64,
    pub tree_nodes: u64,
    pub tree_bytes_estimate_low: u64,
    pub tree_bytes_estimate_high: u64,
    pub cpg_nodes: u64,
    pub cpg_edges: u64,
    /// `size_of::<CpgNode>()` times the node count: the inline payload, strings' heap excluded.
    pub cpg_node_inline_bytes: u64,
    pub node_strings: u64,
    pub node_string_bytes: u64,
    pub node_string_heap_bytes_estimate: u64,
    pub node_unique_strings: u64,
    pub node_unique_string_bytes: u64,
    pub node_unique_string_heap_bytes_estimate: u64,
    /// Unique bytes over total bytes; 1.0 means no string is repeated. `None` with no strings.
    pub node_string_unique_ratio: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access_path::AccessPath;
    use crate::cpg::VarAccess;

    fn function(name: &str, file: &str) -> CpgNode {
        CpgNode::Function {
            name: name.into(),
            file: file.into(),
            start_line: 1,
            end_line: 2,
            start_byte: 0,
            end_byte: 10,
        }
    }

    #[test]
    fn tally_counts_every_string_payload_and_its_duplicates() {
        let nodes = [
            function("f", "a.py"),
            function("g", "a.py"),
            CpgNode::Statement {
                file: "a.py".into(),
                line: 2,
                kind: StmtKind::Call {
                    callee: "f".into(),
                },
                start_byte: 0,
                end_byte: 3,
            },
            CpgNode::Statement {
                file: "a.py".into(),
                line: 3,
                kind: StmtKind::Return,
                start_byte: 0,
                end_byte: 3,
            },
            CpgNode::Variable {
                path: AccessPath {
                    base: "self".into(),
                    fields: vec!["config".into(), "timeout_seconds_value".into()],
                },
                file: "a.py".into(),
                function: "g".into(),
                function_start_line: 1,
                line: 2,
                access: VarAccess::Use,
                start_byte: 0,
                end_byte: 4,
            },
        ];
        let mut tally = StringTally::default();
        for node in &nodes {
            tally.add_node(node);
        }
        // f, a.py | g, a.py | a.py, f | a.py | self, a.py, g, config, timeout_seconds_value
        assert_eq!(tally.count, 12);
        assert_eq!(tally.bytes, 1 + 4 + 1 + 4 + 4 + 1 + 4 + 4 + 4 + 1 + 6 + 21);
        // unique: f, a.py, g, self, config, timeout_seconds_value
        assert_eq!(tally.unique.len(), 6);
        assert_eq!(tally.unique_bytes(), 1 + 4 + 1 + 4 + 6 + 21);
        // every string here is 1..=16 bytes except the 21-byte one, which rounds to 32
        assert_eq!(tally.heap_bytes_estimate, 11 * 16 + 32);
        assert_eq!(tally.unique_heap_bytes_estimate(), 5 * 16 + 32);
    }

    #[test]
    fn goto_and_label_names_are_string_payloads_too() {
        let mut tally = StringTally::default();
        for kind in [
            StmtKind::Goto {
                target: "out".into(),
            },
            StmtKind::Label { name: "out".into() },
        ] {
            tally.add_node(&CpgNode::Statement {
                file: "a.c".into(),
                line: 1,
                kind,
                start_byte: 0,
                end_byte: 1,
            });
        }
        assert_eq!(tally.count, 4);
        assert_eq!(tally.unique.len(), 2);
    }

    #[test]
    fn attribute_counts_a_real_session() {
        let source = "def f():\n    return 1\n\ndef g():\n    return f()\n";
        let session = crate::mcp::tools::test_support::session(&[("a.py", source)]);
        let got = attribute(&session);

        assert_eq!(got.files, 1);
        assert_eq!(got.source_bytes, source.len() as u64);
        let parsed_nodes = session.repo.files["a.py"].parse_node_count as u64;
        assert!(parsed_nodes > 0);
        assert_eq!(got.tree_nodes, parsed_nodes);
        assert_eq!(got.tree_bytes_estimate_low, parsed_nodes * 8);
        assert_eq!(got.tree_bytes_estimate_high, parsed_nodes * 88);
        assert_eq!(got.cpg_nodes, session.index.cpg().node_count() as u64);
        assert_eq!(got.cpg_edges, session.index.cpg().edge_count() as u64);
        assert_eq!(
            got.cpg_node_inline_bytes,
            got.cpg_nodes * std::mem::size_of::<CpgNode>() as u64
        );
        // Two functions in one file: the file name alone repeats.
        assert!(got.node_strings >= 4);
        assert!(got.node_unique_strings < got.node_strings);
        assert!(got.node_unique_string_bytes < got.node_string_bytes);
        let ratio = got.node_string_unique_ratio.expect("strings exist");
        assert!(ratio > 0.0 && ratio < 1.0, "ratio {ratio}");
        assert_eq!(
            ratio,
            got.node_unique_string_bytes as f64 / got.node_string_bytes as f64
        );
    }

    #[test]
    fn an_empty_session_has_no_ratio() {
        let session = crate::mcp::tools::test_support::session(&[]);
        let got = attribute(&session);
        assert_eq!(got.files, 0);
        assert_eq!(got.node_strings, 0);
        assert_eq!(got.node_string_unique_ratio, None);
    }
}
```

In `src/mcp/mod.rs` add, before `pub mod mem_sample;`:

```rust
pub mod mem_attribution;
```

- [ ] **Step 2: Run and see them fail**

Run: `cargo test --features mcp --lib mcp::mem_attribution > /tmp/c1-t2-red.log 2>&1; grep -E 'error\[|cannot find' /tmp/c1-t2-red.log | head`

Expected: compile errors for `StringTally` and `attribute` not found.

- [ ] **Step 3: Implement**

Insert between the `Attribution` struct and the test module:

```rust
/// Count the components of a built session. Borrows every string; allocates one hash set of
/// string slices, so call it after the memory samples are taken, not between them.
pub fn attribute(session: &NavigationSession) -> Attribution {
    let files = session.repo.files.len() as u64;
    let source_bytes = session
        .repo
        .files
        .values()
        .map(|file| file.source.len() as u64)
        .sum();
    let tree_nodes: u64 = session
        .repo
        .files
        .values()
        .map(|file| file.parse_node_count as u64)
        .sum();

    let cpg = session.index.cpg();
    let mut tally = StringTally::default();
    for index in cpg.node_indices() {
        tally.add_node(cpg.node(index));
    }
    let cpg_nodes = cpg.node_count() as u64;

    Attribution {
        files,
        source_bytes,
        tree_nodes,
        tree_bytes_estimate_low: tree_nodes * TREE_NODE_BYTES_LOW,
        tree_bytes_estimate_high: tree_nodes * TREE_NODE_BYTES_HIGH,
        cpg_nodes,
        cpg_edges: cpg.edge_count() as u64,
        cpg_node_inline_bytes: cpg_nodes * std::mem::size_of::<CpgNode>() as u64,
        node_strings: tally.count,
        node_string_bytes: tally.bytes,
        node_string_heap_bytes_estimate: tally.heap_bytes_estimate,
        node_unique_strings: tally.unique.len() as u64,
        node_unique_string_bytes: tally.unique_bytes(),
        node_unique_string_heap_bytes_estimate: tally.unique_heap_bytes_estimate(),
        node_string_unique_ratio: (tally.bytes > 0)
            .then(|| tally.unique_bytes() as f64 / tally.bytes as f64),
    }
}

fn heap_bytes_estimate(len: usize) -> u64 {
    if len == 0 {
        0
    } else {
        (len as u64).div_ceil(STRING_HEAP_QUANTUM) * STRING_HEAP_QUANTUM
    }
}

/// Totals and the distinct set of every string a CPG node owns.
#[derive(Default)]
struct StringTally<'a> {
    count: u64,
    bytes: u64,
    heap_bytes_estimate: u64,
    unique: HashSet<&'a str>,
}

impl<'a> StringTally<'a> {
    fn add(&mut self, text: &'a str) {
        self.count += 1;
        self.bytes += text.len() as u64;
        self.heap_bytes_estimate += heap_bytes_estimate(text.len());
        self.unique.insert(text);
    }

    fn unique_bytes(&self) -> u64 {
        self.unique.iter().map(|text| text.len() as u64).sum()
    }

    fn unique_heap_bytes_estimate(&self) -> u64 {
        self.unique
            .iter()
            .map(|text| heap_bytes_estimate(text.len()))
            .sum()
    }

    /// Every `String` the node owns. The `match` is exhaustive on purpose: a new node variant or
    /// string-bearing statement kind must be counted here before this compiles.
    fn add_node(&mut self, node: &'a CpgNode) {
        match node {
            CpgNode::Function { name, file, .. } => {
                self.add(name);
                self.add(file);
            }
            CpgNode::Statement { file, kind, .. } => {
                self.add(file);
                match kind {
                    StmtKind::Call { callee } => self.add(callee),
                    StmtKind::Goto { target } => self.add(target),
                    StmtKind::Label { name } => self.add(name),
                    StmtKind::Assignment
                    | StmtKind::Return
                    | StmtKind::Branch
                    | StmtKind::Loop
                    | StmtKind::Declaration
                    | StmtKind::Other => {}
                }
            }
            CpgNode::Variable {
                path,
                file,
                function,
                ..
            } => {
                self.add(&path.base);
                self.add(file);
                self.add(function);
                for field in &path.fields {
                    self.add(field);
                }
            }
            CpgNode::ReturnValue { file, function, .. } => {
                self.add(file);
                self.add(function);
            }
        }
    }
}
```

If `CpgNode` or `StmtKind` has gained a variant since this plan was written, the exhaustive `match` fails to compile: add the variant, counting each `String` it owns, and extend `tally_counts_every_string_payload_and_its_duplicates` with one node of that kind.

- [ ] **Step 4: Run the tests**

Run: `cargo test --features mcp --lib mcp::mem_attribution > /tmp/c1-t2-green.log 2>&1; grep -E '^test |^test result' /tmp/c1-t2-green.log`

Expected: 4 passed. If `tally_counts_every_string_payload_and_its_duplicates` fails on a byte total, recount from the comment in the test before changing either side: the test's arithmetic is the specification.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/mem_attribution.rs src/mcp/mod.rs
git commit -m "feat(mcp): component attribution counters for the memory report"
```

---

### Task 3: Observable build stages

**Files:**
- Modify: `src/mcp/session.rs` (`SessionProvider::bootstrap`, `build_state`)
- Modify: `src/mcp/mod.rs` (re-export)
- Test: `src/mcp/mem_report.rs` is created in Task 4; this task's test goes in a small new file, `src/mcp/session_stage_tests.rs`, because `session.rs` is already over 600 lines.

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum BuildStage { RepoLoaded, IndexBuilt }`
  - `impl SessionProvider { pub fn bootstrap_observed(cfg: &ServerConfig, observe: &mut dyn FnMut(BuildStage)) -> anyhow::Result<Self> }`
  - `SessionProvider::bootstrap(cfg)` unchanged in signature and behavior.

- [ ] **Step 1: Write the failing test**

Create `src/mcp/session_stage_tests.rs`:

```rust
use super::*;

fn config(dir: &tempfile::TempDir) -> ServerConfig {
    std::fs::write(dir.path().join("a.py"), "def f():\n    return 1\n").unwrap();
    ServerConfig {
        repo_root: dir.path().to_path_buf(),
        cache: CacheMode::NoCache,
        refresh_policy: RefreshPolicy::WarnOnly,
        startup: StartupMode::Eager,
        first_call_wait: std::time::Duration::from_secs(20),
    }
}

#[test]
fn bootstrap_observed_reports_each_stage_once_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut seen = Vec::new();
    let provider =
        SessionProvider::bootstrap_observed(&config(&dir), &mut |stage| seen.push(stage)).unwrap();
    assert_eq!(seen, [BuildStage::RepoLoaded, BuildStage::IndexBuilt]);
    assert_eq!(provider.session().repo.files.len(), 1);
}

#[test]
fn a_failed_load_reports_no_stage() {
    let mut cfg = ServerConfig::new("/prism/no/such/repository".into());
    cfg.cache = CacheMode::NoCache;
    let mut seen = Vec::new();
    assert!(SessionProvider::bootstrap_observed(&cfg, &mut |stage| seen.push(stage)).is_err());
    assert!(seen.is_empty());
}

#[test]
fn bootstrap_builds_the_same_session_as_bootstrap_observed() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(&dir);
    let plain = SessionProvider::bootstrap(&cfg).unwrap();
    let observed = SessionProvider::bootstrap_observed(&cfg, &mut |_| {}).unwrap();
    assert_eq!(
        plain.session().index.cpg().node_count(),
        observed.session().index.cpg().node_count()
    );
    assert_eq!(
        plain.session().repo.file_hashes,
        observed.session().repo.file_hashes
    );
}
```

At the bottom of `src/mcp/session.rs` add:

```rust
#[cfg(test)]
#[path = "session_stage_tests.rs"]
mod stage_tests;
```

- [ ] **Step 2: Run and see it fail**

Run: `cargo test --features mcp --lib mcp::session::stage_tests > /tmp/c1-t3-red.log 2>&1; grep -E 'error\[|cannot find|no function' /tmp/c1-t3-red.log | head`

Expected: compile errors for `BuildStage` and `bootstrap_observed`.

- [ ] **Step 3: Implement**

In `src/mcp/session.rs`, add after the `StartupMode` enum:

```rust
/// A point in the index build a caller can observe. The memory report samples at each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildStage {
    /// Sources are read and parsed; no graph exists yet.
    RepoLoaded,
    /// The code property graph is loaded from cache or built, and the navigation index exists.
    IndexBuilt,
}
```

Replace `SessionProvider::bootstrap` with:

```rust
    pub fn bootstrap(cfg: &ServerConfig) -> anyhow::Result<Self> {
        Self::bootstrap_observed(cfg, &mut |_| {})
    }

    /// `bootstrap`, calling `observe` as each build stage completes.
    pub fn bootstrap_observed(
        cfg: &ServerConfig,
        observe: &mut dyn FnMut(BuildStage),
    ) -> anyhow::Result<Self> {
        let cfg = canonical_config(cfg)?;
        let state = build_state(&cfg, observe)?;
        Ok(Self {
            cfg,
            session: state.session,
            freshness: state.freshness,
            snapshot: state.snapshot,
            known_stale_after_refresh: None,
            generation: 0,
            #[cfg(test)]
            forced_refresh: None,
        })
    }
```

Change `build_state` to take and call the observer. Only the signature and the two `observe` lines are new:

```rust
#[allow(clippy::arc_with_non_send_sync)]
fn build_state(
    cfg: &ServerConfig,
    observe: &mut dyn FnMut(BuildStage),
) -> anyhow::Result<SessionState> {
    let loaded_repo = load_repo(&cfg.repo_root)?;
    observe(BuildStage::RepoLoaded);
    let snapshot = SnapshotFingerprint::from_repo(&loaded_repo);
    let freshness = FreshnessProbe::from_loaded_repo(&loaded_repo);
    let repo = Arc::new(loaded_repo);
    let index = match &cfg.cache {
        CacheMode::NoCache => NavigationIndex::build(&repo),
        CacheMode::Default => NavigationIndex::build_cached(&repo),
        CacheMode::Dir(base) => NavigationIndex::build_cached_under(&repo, base),
    };
    observe(BuildStage::IndexBuilt);
    let index = Arc::new(index);
    Ok(SessionState {
        session: NavigationSession { repo, index },
        freshness,
        snapshot,
        strategy: RefreshStrategy::Full,
        fallback_reason: None,
    })
}
```

Keep the comment block above `build_state` as it is. In `src/mcp/mod.rs`, add `BuildStage` to the public re-export list:

```rust
pub use session::{
    BuildStage, CacheMode, RefreshPolicy, RefreshSummary, ServerConfig, SessionProvider,
    StartupMode, FIRST_CALL_WAIT_MAX,
};
```

- [ ] **Step 4: Run the new tests and the whole MCP module**

```bash
cargo test --features mcp --lib mcp::session::stage_tests > /tmp/c1-t3-green.log 2>&1; grep -E '^test |^test result' /tmp/c1-t3-green.log
cargo test --features mcp --lib mcp:: > /tmp/c1-t3-lib.log 2>&1; grep -E '^test result' /tmp/c1-t3-lib.log
```

Expected: 3 passed; then the whole module with 0 failed (base 195 + 3 + 4 + 3 = 205 if S1 has not merged; more if it has).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/session.rs src/mcp/session_stage_tests.rs src/mcp/mod.rs
git commit -m "feat(mcp): SessionProvider::bootstrap_observed reports build stages"
```

---

### Task 4: The staged report, the workload, and workload generation

**Files:**
- Create: `src/mcp/mem_report.rs`
- Modify: `src/cpg_cache.rs` (one accessor)
- Modify: `src/mcp/mod.rs`

**Interfaces:**
- Consumes: `MemSample`, `sample`, `ru_maxrss_bytes` (Task 1); `Attribution`, `attribute` (Task 2); `BuildStage`, `SessionProvider::bootstrap_observed` (Task 3); `crate::mcp::transport::{serve_session_with_freshness, ReadOutcome, Transport}`; `crate::mcp::registry::ToolRegistry`; `crate::api::build_info()`; `NavigationIndex::{name_index, resolved_call_edges, cpg}`.
- Produces:
  - `pub struct MemReportOptions { pub workload: Option<PathBuf>, pub refresh: bool, pub settle: Duration }`
  - `pub fn run(cfg: &ServerConfig, options: &MemReportOptions, out: &mut dyn Write) -> anyhow::Result<()>` — writes exactly one line, a JSON object whose first key is `prism_mem_report`.
  - `pub fn emit_workload(cfg: &ServerConfig, functions: usize, out: &mut dyn Write) -> anyhow::Result<()>` — writes four `tools/call` lines per selected function.
  - `pub fn current_cache_format_version() -> u32` in `crate::cpg_cache`.

The report (one line, shown wrapped):

```json
{"prism_mem_report":1,
 "build":{"package_version":"3.1.2","git_sha":"…","build_identity":"…","binary_input_dirty":false,
          "grammar_fingerprint":"…","cpg_cache_version":114,"allocator":"system"},
 "platform":{"os":"macos","arch":"aarch64"},
 "config":{"repo_root":"/abs/repo","cache":"dir:/abs/cache","refresh_policy":"warn-only",
           "workload":"/abs/w.jsonl","refresh":false,"settle_secs":5},
 "stages":[{"name":"start","elapsed_ms":0,"rss_bytes":1,"phys_footprint_bytes":1,"compressed_bytes":0,"peak_bytes":1}],
 "ru_maxrss_bytes":1,
 "attribution":{"files":1},
 "workload":{"requests":200,"responses":200,"errors":0,"response_bytes":1},
 "refresh":{"status":"refreshed","strategy":"full","generation":1}}
```

Stage names, in order: `start`, `after_load_repo`, `after_index`, `after_call_edges`, `after_workload` (only with a workload), `after_settle`, `after_refresh` (only with `refresh`).

- [ ] **Step 1: Write the failing tests**

Create `src/mcp/mem_report.rs` with the test module only:

```rust
//! Memory report (measurement only): stage-by-stage process memory for one eager index build,
//! an optional query workload and an optional refresh, plus component counts that attribute the
//! footprint (shared-daemon spec §6.1). Not part of serving; `prism-mcp` runs it instead of the
//! server when `PRISM_MCP_MEM_REPORT=1`.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::{CacheMode, RefreshPolicy, StartupMode};
    use serde_json::Value;

    fn repo() -> (tempfile::TempDir, ServerConfig) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("util.py"), "def helper():\n    return 1\n").unwrap();
        std::fs::write(
            dir.path().join("main.py"),
            "from util import helper\n\ndef run():\n    return helper()\n",
        )
        .unwrap();
        let cfg = ServerConfig {
            repo_root: dir.path().to_path_buf(),
            cache: CacheMode::NoCache,
            refresh_policy: RefreshPolicy::WarnOnly,
            startup: StartupMode::Eager,
            first_call_wait: std::time::Duration::from_secs(20),
        };
        (dir, cfg)
    }

    fn options(workload: Option<PathBuf>, refresh: bool) -> MemReportOptions {
        MemReportOptions {
            workload,
            refresh,
            settle: Duration::ZERO,
        }
    }

    fn report(cfg: &ServerConfig, options: &MemReportOptions) -> Value {
        let mut out = Vec::new();
        run(cfg, options, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with("{\"prism_mem_report\":1,"), "{text}");
        assert_eq!(text.matches('\n').count(), 1, "exactly one line");
        serde_json::from_str(&text).unwrap()
    }

    fn stage_names(report: &Value) -> Vec<&str> {
        report["stages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|stage| stage["name"].as_str().unwrap())
            .collect()
    }

    fn workload_file(dir: &tempfile::TempDir, cfg: &ServerConfig, functions: usize) -> PathBuf {
        let mut lines = Vec::new();
        emit_workload(cfg, functions, &mut lines).unwrap();
        let path = dir.path().join("workload.jsonl");
        std::fs::write(&path, lines).unwrap();
        path
    }

    #[test]
    fn a_plain_report_has_the_build_stages_and_no_optional_ones() {
        let (_dir, cfg) = repo();
        let report = report(&cfg, &options(None, false));
        assert_eq!(
            stage_names(&report),
            ["start", "after_load_repo", "after_index", "after_call_edges", "after_settle"]
        );
        assert!(report["workload"].is_null());
        assert!(report["refresh"].is_null());
        assert_eq!(report["attribution"]["files"], 2);
        assert_eq!(report["config"]["cache"], "no-cache");
        assert_eq!(report["config"]["refresh_policy"], "warn-only");
        assert_eq!(report["build"]["package_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(report["build"]["allocator"], "system");
        assert!(report["build"]["cpg_cache_version"].as_u64().unwrap() > 0);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn the_lifetime_peak_never_decreases_across_stages() {
        let (_dir, cfg) = repo();
        let report = report(&cfg, &options(None, true));
        let peaks: Vec<u64> = report["stages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|stage| stage["peak_bytes"].as_u64().expect("peak on this platform"))
            .collect();
        assert!(peaks.windows(2).all(|pair| pair[0] <= pair[1]), "{peaks:?}");
        let elapsed: Vec<u64> = report["stages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|stage| stage["elapsed_ms"].as_u64().unwrap())
            .collect();
        assert!(elapsed.windows(2).all(|pair| pair[0] <= pair[1]), "{elapsed:?}");
    }

    #[test]
    fn a_generated_workload_runs_with_no_errors_and_adds_its_stage() {
        let (dir, cfg) = repo();
        let workload = workload_file(&dir, &cfg, 2);
        let lines = std::fs::read_to_string(&workload).unwrap().lines().count();
        assert_eq!(lines, 8, "four queries for each of two functions");

        let report = report(&cfg, &options(Some(workload), true));
        assert_eq!(
            stage_names(&report),
            [
                "start",
                "after_load_repo",
                "after_index",
                "after_call_edges",
                "after_workload",
                "after_settle",
                "after_refresh"
            ]
        );
        assert_eq!(report["workload"]["requests"], 8);
        assert_eq!(report["workload"]["responses"], 8);
        assert_eq!(report["workload"]["errors"], 0);
        assert!(report["workload"]["response_bytes"].as_u64().unwrap() > 0);
        assert_eq!(report["refresh"]["generation"], 1);
        assert_eq!(report["refresh"]["status"], "refreshed");
    }

    #[test]
    fn workload_generation_is_deterministic_and_bounded_by_the_function_count() {
        let (_dir, cfg) = repo();
        let emit = |functions| {
            let mut out = Vec::new();
            emit_workload(&cfg, functions, &mut out).unwrap();
            String::from_utf8(out).unwrap()
        };
        assert_eq!(emit(2), emit(2));
        // The fixture has two functions; asking for more yields all of them, not an error.
        assert_eq!(emit(50).lines().count(), 8);
        for line in emit(1).lines() {
            let request: Value = serde_json::from_str(line).unwrap();
            assert_eq!(request["method"], "tools/call");
            assert!(request["id"].is_u64());
        }
    }

    #[test]
    fn workload_errors_are_counted() {
        let (dir, cfg) = repo();
        let path = dir.path().join("bad-seeds.jsonl");
        std::fs::write(
            &path,
            concat!(
                r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nav_callers","arguments":{"seed":{"kind":"symbol","name":"no_such_function","file":"main.py"}}}}"#,
                "\n",
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run","file":"main.py"}}}}"#,
                "\n"
            ),
        )
        .unwrap();
        let report = report(&cfg, &options(Some(path), false));
        assert_eq!(report["workload"]["requests"], 2);
        assert_eq!(report["workload"]["responses"], 2);
        assert_eq!(report["workload"]["errors"], 1);
    }

    #[test]
    fn a_workload_file_is_validated_before_the_build() {
        let (dir, cfg) = repo();
        let write = |name: &str, contents: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, contents).unwrap();
            path
        };
        let error = |path: PathBuf| {
            let mut out = Vec::new();
            let error = run(&cfg, &options(Some(path), false), &mut out).unwrap_err();
            assert!(out.is_empty(), "no report is written for a rejected workload");
            error.to_string()
        };

        let message = error(write(
            "not-a-call.jsonl",
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{}}\n\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n",
        ));
        assert!(message.contains("line 3"), "{message}");
        assert!(message.contains("tools/call"), "{message}");

        let message = error(write("not-json.jsonl", "nope\n"));
        assert!(message.contains("line 1") && message.contains("not JSON"), "{message}");

        let message = error(write("empty.jsonl", "\n\n"));
        assert!(message.contains("has no requests"), "{message}");

        let message = error(dir.path().join("missing.jsonl"));
        assert!(message.contains("missing.jsonl"), "{message}");
    }

    #[test]
    fn an_empty_repository_cannot_generate_a_workload() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = ServerConfig::new(dir.path().to_path_buf());
        cfg.cache = CacheMode::NoCache;
        cfg.startup = StartupMode::Eager;
        let mut out = Vec::new();
        let error = emit_workload(&cfg, 5, &mut out).unwrap_err().to_string();
        assert!(error.contains("no functions are indexed"), "{error}");
        assert!(out.is_empty());
    }
}
```

In `src/mcp/mod.rs` add, between `mem_attribution` and `mem_sample`:

```rust
pub mod mem_report;
```

- [ ] **Step 2: Run and see them fail**

Run: `cargo test --features mcp --lib mcp::mem_report > /tmp/c1-t4-red.log 2>&1; grep -E 'error\[|cannot find|unresolved' /tmp/c1-t4-red.log | head`

Expected: compile errors for `run`, `emit_workload`, `MemReportOptions` and the unresolved imports.

- [ ] **Step 3: Add the cache-version accessor**

In `src/cpg_cache.rs`, directly after `pub fn current_cache_build_identity()`:

```rust
/// The on-disk CPG cache format version this binary reads and writes.
pub fn current_cache_format_version() -> u32 {
    CACHE_VERSION
}
```

- [ ] **Step 4: Implement the report**

Insert above the test module in `src/mcp/mem_report.rs`:

```rust
use super::mem_attribution::{attribute, Attribution};
use super::mem_sample::{ru_maxrss_bytes, sample, MemSample};
use super::registry::ToolRegistry;
use super::transport::{serve_session_with_freshness, ReadOutcome, Transport};
use super::{BuildStage, CacheMode, RefreshPolicy, RefreshSummary, ServerConfig, SessionProvider};
use crate::cpg::CpgNode;
use anyhow::Context;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const INIT: &str = r#"{"jsonrpc":"2.0","id":"prism-mem-init","method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"prism-mem-report","version":"0"}}}"#;
const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

pub struct MemReportOptions {
    /// A file of `tools/call` requests, one JSON object per line, run after the build.
    pub workload: Option<PathBuf>,
    /// Run one full refresh at the end, for the refresh peak.
    pub refresh: bool,
    /// How long to wait after the workload before the steady-state sample.
    pub settle: Duration,
}

#[derive(serde::Serialize)]
struct Stage {
    name: &'static str,
    elapsed_ms: u64,
    #[serde(flatten)]
    memory: MemSample,
}

struct Recorder {
    started: Instant,
    stages: Vec<Stage>,
}

impl Recorder {
    fn start() -> Self {
        let mut recorder = Self {
            started: Instant::now(),
            stages: Vec::new(),
        };
        recorder.mark("start");
        recorder
    }

    fn mark(&mut self, name: &'static str) {
        self.stages.push(Stage {
            name,
            elapsed_ms: self.started.elapsed().as_millis() as u64,
            memory: sample(),
        });
    }
}

#[derive(Default, serde::Serialize)]
struct WorkloadStats {
    requests: u64,
    responses: u64,
    /// JSON-RPC errors plus tool results with `isError`. A workload that does not fit the corpus
    /// shows up here, and such a run measures nothing.
    errors: u64,
    response_bytes: u64,
}

/// Feeds a handshake and the workload to the serve loop and counts what comes back, keeping no
/// response, so the transport itself adds nothing to the footprint being measured.
struct WorkloadTransport {
    lines: VecDeque<String>,
    handshake_answered: bool,
    stats: WorkloadStats,
}

impl Transport for WorkloadTransport {
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>> {
        Ok(self.lines.pop_front().map(ReadOutcome::Message))
    }

    fn write_message(&mut self, response: Value) -> anyhow::Result<()> {
        if !self.handshake_answered {
            self.handshake_answered = true;
            anyhow::ensure!(
                response.get("result").is_some(),
                "the in-process handshake failed: {response}"
            );
            return Ok(());
        }
        self.stats.responses += 1;
        self.stats.response_bytes += serde_json::to_vec(&response)?.len() as u64 + 1;
        if response.get("error").is_some() || response["result"]["isError"] == true {
            self.stats.errors += 1;
        }
        Ok(())
    }
}

fn run_workload(provider: &SessionProvider, requests: &[String]) -> anyhow::Result<WorkloadStats> {
    let mut lines = VecDeque::with_capacity(requests.len() + 2);
    lines.push_back(INIT.to_string());
    lines.push_back(INITED.to_string());
    lines.extend(requests.iter().cloned());
    let mut transport = WorkloadTransport {
        lines,
        handshake_answered: false,
        stats: WorkloadStats {
            requests: requests.len() as u64,
            ..WorkloadStats::default()
        },
    };
    serve_session_with_freshness(
        provider.session(),
        Some(provider.freshness()),
        &ToolRegistry::all_v1(),
        &mut transport,
    )?;
    Ok(transport.stats)
}

/// Read a workload file: every non-empty line is a `tools/call` request with an id.
fn load_workload(path: &Path) -> anyhow::Result<Vec<String>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading workload {}", path.display()))?;
    let mut requests = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let number = index + 1;
        let value: Value = serde_json::from_str(line).map_err(|error| {
            anyhow::anyhow!("workload {} line {number}: not JSON: {error}", path.display())
        })?;
        anyhow::ensure!(
            value["method"] == "tools/call" && !value["id"].is_null(),
            "workload {} line {number}: every line must be a tools/call request with an id",
            path.display()
        );
        requests.push(line.to_string());
    }
    anyhow::ensure!(!requests.is_empty(), "workload {} has no requests", path.display());
    Ok(requests)
}

#[derive(serde::Serialize)]
struct BuildSection {
    package_version: &'static str,
    git_sha: &'static str,
    build_identity: &'static str,
    binary_input_dirty: bool,
    grammar_fingerprint: &'static str,
    cpg_cache_version: u32,
    /// The crate installs no global allocator. Update this if it ever does.
    allocator: &'static str,
}

#[derive(serde::Serialize)]
struct PlatformSection {
    os: &'static str,
    arch: &'static str,
}

#[derive(serde::Serialize)]
struct ConfigSection {
    repo_root: String,
    cache: String,
    refresh_policy: &'static str,
    workload: Option<String>,
    refresh: bool,
    settle_secs: u64,
}

#[derive(serde::Serialize)]
struct Report {
    /// Report format version. First field, so the line starts with `{"prism_mem_report":`.
    prism_mem_report: u32,
    build: BuildSection,
    platform: PlatformSection,
    config: ConfigSection,
    stages: Vec<Stage>,
    ru_maxrss_bytes: Option<u64>,
    attribution: Attribution,
    workload: Option<WorkloadStats>,
    refresh: Option<RefreshSummary>,
}

fn build_section() -> BuildSection {
    let info = crate::api::build_info();
    BuildSection {
        package_version: info.package_version,
        git_sha: info.git_sha,
        build_identity: info.build_identity,
        binary_input_dirty: info.binary_input_dirty,
        grammar_fingerprint: info.grammar_fingerprint,
        cpg_cache_version: crate::cpg_cache::current_cache_format_version(),
        allocator: "system",
    }
}

fn config_section(cfg: &ServerConfig, options: &MemReportOptions) -> ConfigSection {
    ConfigSection {
        repo_root: cfg.repo_root.display().to_string(),
        cache: match &cfg.cache {
            CacheMode::NoCache => "no-cache".to_string(),
            CacheMode::Default => "default".to_string(),
            CacheMode::Dir(base) => format!("dir:{}", base.display()),
        },
        refresh_policy: match cfg.refresh_policy {
            RefreshPolicy::WarnOnly => "warn-only",
            RefreshPolicy::AutoFull => "auto-full",
            RefreshPolicy::AutoIncremental => "auto-incremental",
        },
        workload: options
            .workload
            .as_ref()
            .map(|path| path.display().to_string()),
        refresh: options.refresh,
        settle_secs: options.settle.as_secs(),
    }
}

/// Build the index in observed stages, run the optional workload and refresh, and write one
/// JSON line describing the process's memory at each stage.
pub fn run(
    cfg: &ServerConfig,
    options: &MemReportOptions,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    // Reject a bad workload before paying for the build.
    let requests = options.workload.as_deref().map(load_workload).transpose()?;

    let mut recorder = Recorder::start();
    let mut provider = SessionProvider::bootstrap_observed(cfg, &mut |stage| {
        recorder.mark(match stage {
            BuildStage::RepoLoaded => "after_load_repo",
            BuildStage::IndexBuilt => "after_index",
        })
    })?;

    // The resolved call-edge index is built by the first caller/callee query. Force it here so
    // its cost is a stage of its own instead of hiding inside the workload.
    let _ = provider.session().index.resolved_call_edges();
    recorder.mark("after_call_edges");

    let workload = match &requests {
        Some(requests) => {
            let stats = run_workload(&provider, requests)?;
            recorder.mark("after_workload");
            Some(stats)
        }
        None => None,
    };

    std::thread::sleep(options.settle);
    recorder.mark("after_settle");

    let refresh = if options.refresh {
        let summary = provider.refresh()?;
        recorder.mark("after_refresh");
        Some(summary)
    } else {
        None
    };

    // Attribution allocates; it runs after the last sample.
    let report = Report {
        prism_mem_report: 1,
        build: build_section(),
        platform: PlatformSection {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        },
        config: config_section(cfg, options),
        stages: recorder.stages,
        ru_maxrss_bytes: ru_maxrss_bytes(),
        attribution: attribute(provider.session()),
        workload,
        refresh,
    };
    serde_json::to_writer(&mut *out, &report)?;
    out.write_all(b"\n")?;
    Ok(())
}

fn requests_for(first_id: u64, file: &str, name: &str, start_line: usize) -> Vec<String> {
    let seed = json!({"kind": "symbol", "name": name, "file": file});
    [
        ("nav_callers", json!({ "seed": seed.clone() })),
        ("nav_callees", json!({ "seed": seed.clone() })),
        ("nav_ego_graph", json!({ "seed": seed })),
        ("nav_nodes_at", json!({"file": file, "line": start_line})),
    ]
    .into_iter()
    .enumerate()
    .map(|(offset, (tool, arguments))| {
        json!({
            "jsonrpc": "2.0",
            "id": first_id + offset as u64,
            "method": "tools/call",
            "params": { "name": tool, "arguments": arguments }
        })
        .to_string()
    })
    .collect()
}

/// Write a workload for this repository: up to `functions` functions, evenly spaced through the
/// sorted `(file, name)` index, four queries each. A function is kept only if all four of its
/// queries succeed against this build, so the workload is admissible by construction; when one
/// does not, the next function in order is tried, up to the next evenly spaced position.
pub fn emit_workload(
    cfg: &ServerConfig,
    functions: usize,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    anyhow::ensure!(functions > 0, "ask for at least one function");
    let provider = SessionProvider::bootstrap(cfg)?;
    let index = &provider.session().index;
    let candidates: Vec<(&str, &str, usize)> = index
        .name_index
        .iter()
        .filter_map(|((file, name), nodes)| match index.cpg().node(*nodes.first()?) {
            CpgNode::Function { start_line, .. } => Some((file.as_str(), name.as_str(), *start_line)),
            _ => None,
        })
        .collect();
    anyhow::ensure!(
        !candidates.is_empty(),
        "no functions are indexed under {}",
        cfg.repo_root.display()
    );

    let wanted = functions.min(candidates.len());
    let stride = candidates.len() / wanted;
    let mut next_id = 1u64;
    let mut emitted = 0usize;
    for slot in 0..wanted {
        for offset in 0..stride {
            let (file, name, start_line) = candidates[slot * stride + offset];
            let requests = requests_for(next_id, file, name, start_line);
            if run_workload(&provider, &requests)?.errors == 0 {
                for request in &requests {
                    writeln!(out, "{request}")?;
                }
                next_id += requests.len() as u64;
                emitted += 1;
                break;
            }
        }
    }
    anyhow::ensure!(
        emitted > 0,
        "no function under {} answers all four workload queries",
        cfg.repo_root.display()
    );
    Ok(())
}
```

`RefreshSummary` is re-exported from `crate::mcp` and derives `Serialize`; `crate::api::build_info()` returns the `BuildInfo` whose fields are read above.

- [ ] **Step 5: Run the tests**

Run: `cargo test --features mcp --lib mcp::mem_report > /tmp/c1-t4-green.log 2>&1; grep -E '^test |^test result' /tmp/c1-t4-green.log`

Expected: 7 passed. If `a_generated_workload_runs_with_no_errors_and_adds_its_stage` reports fewer than 8 lines, one fixture function failed a query: print the four responses for that function before changing anything, because the generator dropping a function on a two-function fixture means a query the plan assumed valid is not.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add src/mcp/mem_report.rs src/mcp/mod.rs src/cpg_cache.rs
git commit -m "feat(mcp): staged memory report with a query workload and workload generation"
```

---

### Task 5: The `prism-mcp` mode switch

**Files:**
- Modify: `src/bin/prism-mcp.rs`
- Create: `tests/mcp/mem_report_test.rs`
- Modify: `tests/mcp/main.rs`

**Interfaces:**
- Consumes: `prism::mcp::mem_report::{run, emit_workload, MemReportOptions}`.
- Produces the command line the driver uses:
  - `PRISM_MCP_MEM_REPORT=1 prism-mcp --repo X --eager [--no-cache | --cache-dir D] [--mem-workload FILE] [--mem-refresh] [--mem-settle-secs N]` → one report line on stderr, nothing on stdout, exit 0.
  - `PRISM_MCP_MEM_REPORT=1 prism-mcp --repo X --eager [...] --mem-emit-workload N` → the workload on stdout, exit 0.

Rules: report mode is on exactly when `PRISM_MCP_MEM_REPORT` is `1`. In report mode `--eager` is required and `--owner-*` is rejected. A `--mem-workload`, `--mem-refresh` or `--mem-emit-workload` flag without the variable is an error. Report mode announces itself on stderr before doing anything, so a person who left the variable exported sees why no server started.

- [ ] **Step 1: Write the failing tests**

Create `tests/mcp/mem_report_test.rs`:

```rust
#![cfg(feature = "mcp")]

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp repo");
    std::fs::write(dir.path().join("util.py"), "def helper():\n    return 1\n").expect("util.py");
    std::fs::write(
        dir.path().join("main.py"),
        "from util import helper\n\ndef run():\n    return helper()\n",
    )
    .expect("main.py");
    dir
}

fn report_command(repo: &tempfile::TempDir) -> Command {
    let mut command = Command::cargo_bin("prism-mcp").expect("prism-mcp binary");
    command
        .arg("--repo")
        .arg(repo.path())
        .arg("--no-cache")
        .arg("--eager")
        .arg("--mem-settle-secs")
        .arg("0")
        .env("PRISM_MCP_MEM_REPORT", "1");
    command
}

fn report_line(stderr: &[u8]) -> Value {
    let text = String::from_utf8(stderr.to_vec()).expect("stderr utf8");
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("{\"prism_mem_report\":"))
        .collect();
    assert_eq!(lines.len(), 1, "exactly one report line in: {text}");
    serde_json::from_str(lines[0]).expect("report is JSON")
}

#[test]
fn report_mode_prints_one_report_and_does_not_serve() {
    let repo = repo();
    let output = report_command(&repo)
        // A real client would send this; report mode must not answer it.
        .write_stdin(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{},\"clientInfo\":{\"name\":\"t\",\"version\":\"0\"}}}\n",
        )
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(output.stdout.is_empty(), "report mode writes nothing to stdout");
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(
        stderr.starts_with("prism-mcp: PRISM_MCP_MEM_REPORT=1"),
        "report mode announces itself first: {stderr}"
    );
    assert!(stderr.contains("not serving MCP"), "{stderr}");
    let report = report_line(&output.stderr);
    assert_eq!(report["stages"].as_array().unwrap().len(), 5);
    assert_eq!(report["attribution"]["files"], 2);
}

#[test]
fn a_generated_workload_feeds_a_workload_and_refresh_run() {
    let repo = repo();
    let emitted = report_command(&repo)
        .arg("--mem-emit-workload")
        .arg("2")
        .assert()
        .success()
        .get_output()
        .clone();
    let workload = repo.path().join("..").join(format!(
        "{}-workload.jsonl",
        repo.path().file_name().unwrap().to_string_lossy()
    ));
    std::fs::write(&workload, &emitted.stdout).expect("write workload");
    assert_eq!(String::from_utf8_lossy(&emitted.stdout).lines().count(), 8);

    let output = report_command(&repo)
        .arg("--mem-workload")
        .arg(&workload)
        .arg("--mem-refresh")
        .assert()
        .success()
        .get_output()
        .clone();
    std::fs::remove_file(&workload).ok();
    let report = report_line(&output.stderr);
    assert_eq!(report["workload"]["requests"], 8);
    assert_eq!(report["workload"]["errors"], 0);
    assert_eq!(report["refresh"]["generation"], 1);
    assert_eq!(report["stages"].as_array().unwrap().len(), 7);
}

#[test]
fn report_mode_without_eager_fails_naming_the_variable() {
    let repo = repo();
    Command::cargo_bin("prism-mcp")
        .expect("prism-mcp binary")
        .arg("--repo")
        .arg(repo.path())
        .env("PRISM_MCP_MEM_REPORT", "1")
        .write_stdin("")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("PRISM_MCP_MEM_REPORT"))
        .stderr(predicate::str::contains("--eager"))
        .stderr(predicate::str::contains("unset"));
}

#[test]
fn mem_flags_without_the_variable_are_rejected() {
    let repo = repo();
    for flag in [vec!["--mem-refresh"], vec!["--mem-emit-workload", "2"]] {
        Command::cargo_bin("prism-mcp")
            .expect("prism-mcp binary")
            .arg("--repo")
            .arg(repo.path())
            .arg("--eager")
            .arg("--no-cache")
            .args(&flag)
            .env_remove("PRISM_MCP_MEM_REPORT")
            .write_stdin("")
            .assert()
            .failure()
            .stderr(predicate::str::contains("PRISM_MCP_MEM_REPORT=1"));
    }
}

#[test]
fn any_other_value_of_the_variable_leaves_the_server_alone() {
    let repo = repo();
    let output = Command::cargo_bin("prism-mcp")
        .expect("prism-mcp binary")
        .arg("--repo")
        .arg(repo.path())
        .arg("--no-cache")
        .env("PRISM_MCP_MEM_REPORT", "0")
        .write_stdin("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n")
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "the server answered the ping: {stdout}");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("prism_mem_report"));
}

#[test]
fn a_rejected_workload_file_fails_with_its_line_number() {
    let repo = repo();
    let workload = repo.path().join("..").join(format!(
        "{}-bad.jsonl",
        repo.path().file_name().unwrap().to_string_lossy()
    ));
    std::fs::write(&workload, "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n").unwrap();
    report_command(&repo)
        .arg("--mem-workload")
        .arg(&workload)
        .assert()
        .failure()
        .stderr(predicate::str::contains("line 1"));
    std::fs::remove_file(&workload).ok();
}
```

The two workload files are written beside the repository, not inside it, so they are not indexed as part of it.

Add `mod mem_report_test;` to `tests/mcp/main.rs`, keeping every existing `mod` line and alphabetical order.

- [ ] **Step 2: Run and see them fail**

Run: `cargo test --features mcp --test mcp mem_report_test > /tmp/c1-t5-red.log 2>&1; grep -E '^test |^test result' /tmp/c1-t5-red.log`

Expected: 5 FAILED, 1 passed. `any_other_value_of_the_variable_leaves_the_server_alone` already passes (it pins behavior that must not change). The others fail: clap rejects the unknown `--mem-*` flags, and a plain server with the variable set serves instead of failing.

- [ ] **Step 3: Implement the switch**

Replace `src/bin/prism-mcp.rs` with:

```rust
#[derive(clap::Parser)]
struct Cli {
    #[command(flatten)]
    owner: prism::cli::OwnerArgs,
    #[arg(long)]
    repo: std::path::PathBuf,
    #[arg(long, conflicts_with = "cache_dir")]
    no_cache: bool,
    #[arg(long)]
    cache_dir: Option<std::path::PathBuf>,
    #[arg(long, default_value = "warn-only", value_parser = ["warn-only", "auto-full", "auto-incremental"])]
    refresh_policy: String,
    #[arg(long)]
    eager: bool,
    #[arg(long, conflicts_with = "eager")]
    warm_at_startup: bool,
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u64).range(0..=600))]
    first_call_wait: u64,
    /// Measurement only (needs PRISM_MCP_MEM_REPORT=1): run this tools/call workload after the build.
    #[arg(long, hide = true)]
    mem_workload: Option<std::path::PathBuf>,
    /// Measurement only: run one refresh at the end, for the refresh peak.
    #[arg(long, hide = true)]
    mem_refresh: bool,
    /// Measurement only: seconds to wait before the steady-state sample.
    #[arg(long, hide = true, default_value_t = 5, value_parser = clap::value_parser!(u64).range(0..=600))]
    mem_settle_secs: u64,
    /// Measurement only: print a workload for this many functions and exit.
    #[arg(long, hide = true, value_parser = clap::value_parser!(u64).range(1..=10000), conflicts_with_all = ["mem_workload", "mem_refresh"])]
    mem_emit_workload: Option<u64>,
}

fn main() -> anyhow::Result<()> {
    let c = <Cli as clap::Parser>::parse();
    let cache = if c.no_cache {
        prism::mcp::CacheMode::NoCache
    } else if let Some(d) = c.cache_dir {
        prism::mcp::CacheMode::Dir(d)
    } else {
        prism::mcp::CacheMode::Default
    };
    let refresh_policy = match c.refresh_policy.as_str() {
        "warn-only" => prism::mcp::RefreshPolicy::WarnOnly,
        "auto-full" => prism::mcp::RefreshPolicy::AutoFull,
        "auto-incremental" => prism::mcp::RefreshPolicy::AutoIncremental,
        _ => unreachable!("clap value_parser restricts refresh-policy"),
    };
    let mut cfg = prism::mcp::ServerConfig::new(c.repo);
    cfg.cache = cache;
    cfg.refresh_policy = refresh_policy;
    cfg.startup = if c.eager {
        prism::mcp::StartupMode::Eager
    } else if c.warm_at_startup {
        prism::mcp::StartupMode::Background
    } else {
        prism::mcp::StartupMode::Lazy
    };
    cfg.first_call_wait = std::time::Duration::from_secs(c.first_call_wait);
    let owner = c.owner.options()?;

    let report_mode = std::env::var("PRISM_MCP_MEM_REPORT").is_ok_and(|value| value == "1");
    let mem_flag = c.mem_workload.is_some() || c.mem_refresh || c.mem_emit_workload.is_some();
    if mem_flag && !report_mode {
        anyhow::bail!(
            "--mem-workload, --mem-refresh and --mem-emit-workload are measurement-only and need PRISM_MCP_MEM_REPORT=1"
        );
    }
    if report_mode {
        eprintln!("prism-mcp: PRISM_MCP_MEM_REPORT=1 puts this process in memory-report mode; it is not serving MCP");
        anyhow::ensure!(
            c.eager,
            "PRISM_MCP_MEM_REPORT=1 selects memory-report mode, which needs --eager and never serves MCP; unset PRISM_MCP_MEM_REPORT to run the server"
        );
        anyhow::ensure!(
            owner.is_none(),
            "memory-report mode does not support --owner-compiler/--owner-config"
        );
        if let Some(functions) = c.mem_emit_workload {
            return prism::mcp::mem_report::emit_workload(
                &cfg,
                functions as usize,
                &mut std::io::stdout().lock(),
            );
        }
        let options = prism::mcp::mem_report::MemReportOptions {
            workload: c.mem_workload,
            refresh: c.mem_refresh,
            settle: std::time::Duration::from_secs(c.mem_settle_secs),
        };
        return prism::mcp::mem_report::run(&cfg, &options, &mut std::io::stderr().lock());
    }

    match owner {
        Some(owner) => prism::mcp::run_with_owner(cfg, owner),
        None => prism::mcp::run(cfg),
    }
}
```

The only change on the serving path is that `c.owner.options()?` is evaluated a few lines earlier; it has no side effect.

- [ ] **Step 4: Run the new tests and every existing MCP binary test**

```bash
cargo test --features mcp --test mcp > /tmp/c1-t5-green.log 2>&1; grep -E '^test |^test result' /tmp/c1-t5-green.log
cargo run -q --features mcp --bin prism-mcp -- --help > /tmp/c1-t5-help.log 2>&1; grep -c -- '--mem-' /tmp/c1-t5-help.log
```

Expected: every test in the target passes, including the 4 from `smoke_test`; the last command prints `0` (the flags are hidden from `--help`).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/bin/prism-mcp.rs tests/mcp/mem_report_test.rs tests/mcp/main.rs
git commit -m "feat(mcp): PRISM_MCP_MEM_REPORT memory-report mode in prism-mcp"
```

---

### Task 6: The driver

**Files:**
- Create: `scripts/memreport/memreport.py`
- Create: `scripts/memreport/test_memreport.py`
- Create: `scripts/memreport/README.md`

**Interfaces:**
- Consumes: the command line from Task 5 and the report format from Task 4.
- Produces:
  - `memreport.py gen-workload --bin BIN --corpus NAME=PATH [--functions 50]` → `scripts/memreport/workloads/NAME.jsonl` and an entry in `workloads/MANIFEST.json`.
  - `memreport.py run --bin BIN --corpus NAME=PATH [--corpus …] --out DIR [--reps 3] [--timeout 1800] [--settle 5]` → `DIR/raw/NAME/KIND-REP.json`, `DIR/summary.json`.
  - `memreport.py render --run DIR --out FILE.md` → the Markdown readout.

Run kinds: `warm` (primed cache), `cold` (`--no-cache`), `workload` (primed cache plus the corpus workload), `refresh` (primed cache plus one refresh). A run is **inadmissible**, and excluded from every statistic, when: it timed out; it exited non-zero; it has no single report line; it is a `warm`, `workload` or `refresh` run whose stderr says the cache missed; or its workload had errors.

Derived per run, in bytes (`footprint` = `phys_footprint_bytes`, or `rss_bytes` where that is null):

| Name | Kind | Formula |
|---|---|---|
| `steady` | workload | `footprint(after_settle)` |
| `peak` | warm | `peak_bytes(after_index)` |
| `m2_transient` | warm | `max(0, peak_bytes(after_index) − max(peak_bytes(after_load_repo), footprint(after_index)))` |
| `refresh_peak` | refresh | `peak_bytes(after_refresh)` |
| `cold_peak` | cold | `peak_bytes(after_call_edges)` |
| `m1_low`, `m1_high` | workload | `attribution.tree_bytes_estimate_low`, `…_high` |
| `m3` | workload | `attribution.node_string_heap_bytes_estimate − attribution.node_unique_string_heap_bytes_estimate` |

`m2_transient` is the part of the load peak that is gone once the index exists and that the load itself, not parsing, set. If parsing already set a higher peak, streaming the cache cannot lower the process peak and the value is 0.

Rule: a candidate **advances** when, on at least two corpora, its estimate is ≥ 200 MiB or ≥ 15 % of the base (M2 against `peak`; M1 with `m1_high` and M3 against `steady`). M1 uses the high bound because the rule only decides whether the experiment is worth running; the readout shows both bounds.

- [ ] **Step 1: Write the failing tests**

Create `scripts/memreport/test_memreport.py`:

```python
#!/usr/bin/env python3
"""Unit tests for the memory-report driver.

Run with:
    python3 -m unittest scripts/memreport/test_memreport.py
"""
import json
import stat
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402

MIB = 1024 * 1024


def stage(name, footprint, peak, rss=None):
    return {'name': name, 'elapsed_ms': 0, 'rss_bytes': rss if rss is not None else footprint,
            'phys_footprint_bytes': footprint, 'compressed_bytes': 0, 'peak_bytes': peak}


def report(stages, attribution=None, workload=None):
    return {'prism_mem_report': 1, 'build': {'build_identity': 'b'}, 'platform': {'os': 'macos'},
            'config': {}, 'stages': stages, 'ru_maxrss_bytes': 1,
            'attribution': attribution or {}, 'workload': workload, 'refresh': None}


def fake_binary(directory, body):
    """A stand-in for prism-mcp: a Python script that runs `body`."""
    path = Path(directory) / 'fake-prism-mcp'
    path.write_text('#!/usr/bin/env python3\nimport json, sys, time\n' + body)
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


class ParseTests(unittest.TestCase):
    def test_parse_report_takes_the_single_marked_line(self):
        text = 'nav cache: noise\n{"prism_mem_report":1,"stages":[]}\ntrailing\n'
        self.assertEqual(memreport.parse_report(text)['stages'], [])

    def test_parse_report_rejects_zero_or_two_reports(self):
        with self.assertRaises(ValueError):
            memreport.parse_report('no report here\n')
        with self.assertRaises(ValueError):
            memreport.parse_report('{"prism_mem_report":1}\n{"prism_mem_report":1}\n')

    def test_footprint_falls_back_to_rss(self):
        self.assertEqual(memreport.footprint({'phys_footprint_bytes': 7, 'rss_bytes': 9}), 7)
        self.assertEqual(memreport.footprint({'phys_footprint_bytes': None, 'rss_bytes': 9}), 9)

    def test_summarize_reports_min_median_max(self):
        self.assertEqual(memreport.summarize([3, 1, 2]), {'min': 1, 'median': 2, 'max': 3, 'n': 3})
        self.assertIsNone(memreport.summarize([None, None]))
        self.assertEqual(memreport.summarize([5, None])['n'], 1)


class DeriveTests(unittest.TestCase):
    def test_m2_transient_is_the_load_peak_above_the_built_index(self):
        warm = report([stage('after_load_repo', 100, 120), stage('after_index', 400, 700)])
        self.assertEqual(memreport.derive('warm', warm), {'peak': 700, 'm2_transient': 300})

    def test_m2_transient_is_zero_when_parsing_set_the_peak(self):
        warm = report([stage('after_load_repo', 100, 900), stage('after_index', 400, 900)])
        self.assertEqual(memreport.derive('warm', warm)['m2_transient'], 0)

    def test_workload_run_yields_steady_state_and_the_estimates(self):
        attribution = {'tree_bytes_estimate_low': 8, 'tree_bytes_estimate_high': 88,
                       'node_string_heap_bytes_estimate': 100,
                       'node_unique_string_heap_bytes_estimate': 40}
        run = report([stage('after_settle', 500, 900)], attribution)
        self.assertEqual(memreport.derive('workload', run),
                         {'steady': 500, 'm1_low': 8, 'm1_high': 88, 'm3': 60})

    def test_refresh_and_cold_runs_yield_their_peaks(self):
        self.assertEqual(memreport.derive('refresh', report([stage('after_refresh', 1, 77)])),
                         {'refresh_peak': 77})
        self.assertEqual(memreport.derive('cold', report([stage('after_call_edges', 1, 66)])),
                         {'cold_peak': 66})


class RuleTests(unittest.TestCase):
    def row(self, **kw):
        base = {'steady': 1000 * MIB, 'peak': 1500 * MIB, 'm2_transient': 0, 'm1_high': 0, 'm3': 0}
        base.update(kw)
        return base

    def test_a_candidate_advances_on_two_corpora_by_fraction_or_absolute(self):
        rows = {'a': self.row(m2_transient=230 * MIB),            # 15.3 % of peak
                'b': self.row(m2_transient=200 * MIB, peak=9000 * MIB),  # absolute
                'c': self.row(m2_transient=10 * MIB)}
        verdict = memreport.evaluate(rows)
        self.assertEqual(verdict['M2'], {'corpora': ['a', 'b'], 'advance': True})
        self.assertEqual(verdict['M1'], {'corpora': [], 'advance': False})

    def test_one_corpus_is_not_enough(self):
        rows = {'a': self.row(m3=400 * MIB), 'b': self.row(m3=1 * MIB)}
        self.assertEqual(memreport.evaluate(rows)['M3'], {'corpora': ['a'], 'advance': False})

    def test_a_missing_measurement_never_counts(self):
        rows = {'a': {'steady': None, 'peak': None, 'm2_transient': None, 'm1_high': 900 * MIB, 'm3': None},
                'b': self.row(m1_high=900 * MIB)}
        self.assertEqual(memreport.evaluate(rows)['M1'], {'corpora': ['b'], 'advance': False})


class CommandTests(unittest.TestCase):
    def test_each_kind_gets_its_flags(self):
        base = ['bin', '--repo', 'r', '--eager', '--mem-settle-secs', '5']
        cmd = lambda kind: memreport.command_for('bin', 'r', 'c', kind, 'w', 5)  # noqa: E731
        self.assertEqual(cmd('warm'), base + ['--cache-dir', 'c'])
        self.assertEqual(cmd('cold'), base + ['--no-cache'])
        self.assertEqual(cmd('workload'), base + ['--cache-dir', 'c', '--mem-workload', 'w'])
        self.assertEqual(cmd('refresh'), base + ['--cache-dir', 'c', '--mem-refresh'])


class RunOneTests(unittest.TestCase):
    def run_fake(self, body, kind='warm', timeout=20):
        with tempfile.TemporaryDirectory() as tmp:
            binary = fake_binary(tmp, body)
            return memreport.run_one([str(binary)], kind, timeout)

    def test_a_good_run_is_admissible_and_carries_its_report(self):
        result = self.run_fake(
            "print(json.dumps({'prism_mem_report': 1, 'stages': [], 'workload': None}), file=sys.stderr)\n")
        self.assertEqual(result['status'], 'ok')
        self.assertEqual(result['report']['stages'], [])

    def test_timeout_is_recorded_and_does_not_raise(self):
        result = self.run_fake('time.sleep(30)\n', timeout=1)
        self.assertEqual(result['status'], 'timeout')
        self.assertNotIn('report', result)

    def test_a_nonzero_exit_is_recorded_with_the_stderr_tail(self):
        result = self.run_fake("print('boom', file=sys.stderr)\nsys.exit(3)\n")
        self.assertEqual(result['status'], 'failed')
        self.assertEqual(result['exit'], 3)
        self.assertIn('boom', result['stderr_tail'])

    def test_warm_run_that_missed_the_cache_is_inadmissible(self):
        body = ("print('nav cache: CPG cache miss in /x, rebuilding', file=sys.stderr)\n"
                "print(json.dumps({'prism_mem_report': 1, 'stages': [], 'workload': None}), file=sys.stderr)\n")
        self.assertEqual(self.run_fake(body, kind='warm')['status'], 'inadmissible')
        self.assertIn('cache', self.run_fake(body, kind='warm')['reason'])
        # A cold run is expected to build; the same output is fine there.
        self.assertEqual(self.run_fake(body, kind='cold')['status'], 'ok')

    def test_run_with_workload_errors_is_inadmissible(self):
        body = ("print(json.dumps({'prism_mem_report': 1, 'stages': [], "
                "'workload': {'requests': 4, 'responses': 4, 'errors': 1, 'response_bytes': 9}}), file=sys.stderr)\n")
        result = self.run_fake(body, kind='workload')
        self.assertEqual(result['status'], 'inadmissible')
        self.assertIn('workload', result['reason'])

    def test_a_run_without_a_report_is_inadmissible(self):
        result = self.run_fake("print('nothing useful', file=sys.stderr)\n")
        self.assertEqual(result['status'], 'inadmissible')


class RenderTests(unittest.TestCase):
    def test_render_shows_values_inadmissible_counts_and_the_verdict(self):
        summary = {
            'provenance': {'build': {'build_identity': 'b', 'git_sha': 'abc'}, 'reps': 3, 'settle': 5},
            'corpora': {
                'a': {'commit': '111', 'dirty': 0, 'admissible': {'warm': 3, 'cold': 2, 'workload': 3, 'refresh': 3},
                      'inadmissible': [{'kind': 'cold', 'rep': 2, 'status': 'timeout', 'reason': 'timeout'}],
                      'metrics': {'steady': {'min': 900 * MIB, 'median': 1000 * MIB, 'max': 1100 * MIB, 'n': 3}},
                      'attribution': {'files': 10}},
            },
            'verdict': {'M1': {'corpora': [], 'advance': False},
                        'M2': {'corpora': ['a'], 'advance': False},
                        'M3': {'corpora': [], 'advance': False}},
        }
        text = memreport.render(summary)
        self.assertIn('1000.0', text)       # median steady state in MiB
        self.assertIn('timeout', text)      # the excluded run is named
        self.assertIn('| M2 | a | no |', text)
        self.assertIn('coarse triage', text)


if __name__ == '__main__':
    unittest.main()
```

- [ ] **Step 2: Run and see them fail**

Run: `python3 -m unittest scripts/memreport/test_memreport.py > /tmp/c1-t6-red.log 2>&1; tail -3 /tmp/c1-t6-red.log`

Expected: `ModuleNotFoundError: No module named 'memreport'`.

- [ ] **Step 3: Write the driver**

Create `scripts/memreport/memreport.py`:

```python
#!/usr/bin/env python3
"""Memory-report driver for prism-mcp (shared-daemon spec section 6.1).

    memreport.py gen-workload --bin BIN --corpus NAME=PATH [--functions 50]
    memreport.py run --bin BIN --corpus NAME=PATH [--corpus ...] --out DIR
    memreport.py render --run DIR --out FILE.md

Runs are sequential on purpose: one index in memory at a time.
Unit tests: python3 -m unittest scripts/memreport/test_memreport.py
"""
import argparse
import hashlib
import json
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

MARKER = '{"prism_mem_report":'
KINDS = ('warm', 'cold', 'workload', 'refresh')
CACHE_MISS_MARKERS = ('CPG cache miss', 'partial CPG cache hit')
MIB = 1024 * 1024
RULE_FRACTION = 0.15
RULE_ABSOLUTE = 200 * MIB
CANDIDATES = (('M1', 'm1_high', 'steady'), ('M2', 'm2_transient', 'peak'), ('M3', 'm3', 'steady'))
HERE = Path(__file__).resolve().parent
WORKLOADS = HERE / 'workloads'


def parse_report(stderr_text):
    """The single report line in a prism-mcp stderr capture, parsed."""
    lines = [line for line in stderr_text.splitlines() if line.startswith(MARKER)]
    if len(lines) != 1:
        raise ValueError('expected exactly one report line, found %d' % len(lines))
    return json.loads(lines[0])


def stage(report, name):
    for entry in report['stages']:
        if entry['name'] == name:
            return entry
    raise KeyError('report has no stage %r' % name)


def footprint(entry):
    """Physical footprint where the platform reports it, resident size otherwise."""
    value = entry.get('phys_footprint_bytes')
    return value if value is not None else entry.get('rss_bytes')


def summarize(values):
    values = [value for value in values if value is not None]
    if not values:
        return None
    return {'min': min(values), 'median': statistics.median(values), 'max': max(values), 'n': len(values)}


def derive(kind, report):
    """The per-run quantities the readout is built from (bytes)."""
    if kind == 'warm':
        loaded, built = stage(report, 'after_load_repo'), stage(report, 'after_index')
        transient = built['peak_bytes'] - max(loaded['peak_bytes'], footprint(built))
        return {'peak': built['peak_bytes'], 'm2_transient': max(0, transient)}
    if kind == 'cold':
        return {'cold_peak': stage(report, 'after_call_edges')['peak_bytes']}
    if kind == 'refresh':
        return {'refresh_peak': stage(report, 'after_refresh')['peak_bytes']}
    if kind == 'workload':
        attribution = report['attribution']
        return {
            'steady': footprint(stage(report, 'after_settle')),
            'm1_low': attribution['tree_bytes_estimate_low'],
            'm1_high': attribution['tree_bytes_estimate_high'],
            'm3': attribution['node_string_heap_bytes_estimate']
            - attribution['node_unique_string_heap_bytes_estimate'],
        }
    raise ValueError('unknown kind %r' % kind)


def meets(estimate, base):
    if estimate is None or not base:
        return False
    return estimate >= RULE_ABSOLUTE or estimate / base >= RULE_FRACTION


def evaluate(rows):
    """rows: {corpus: {metric: median bytes}} -> {candidate: {'corpora': [...], 'advance': bool}}"""
    verdict = {}
    for candidate, estimate_key, base_key in CANDIDATES:
        hits = sorted(name for name, row in rows.items() if meets(row.get(estimate_key), row.get(base_key)))
        verdict[candidate] = {'corpora': hits, 'advance': len(hits) >= 2}
    return verdict


def command_for(binary, repo, cache_dir, kind, workload, settle):
    command = [str(binary), '--repo', str(repo), '--eager', '--mem-settle-secs', str(settle)]
    command += ['--no-cache'] if kind == 'cold' else ['--cache-dir', str(cache_dir)]
    if kind == 'workload':
        command += ['--mem-workload', str(workload)]
    if kind == 'refresh':
        command += ['--mem-refresh']
    return command


def run_one(command, kind, timeout):
    """Run one measurement. Never raises for a bad run: the result says what happened."""
    env = dict(os.environ, PRISM_MCP_MEM_REPORT='1')
    started = time.time()
    try:
        done = subprocess.run(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE, timeout=timeout, env=env)
    except subprocess.TimeoutExpired:
        return {'status': 'timeout', 'reason': 'timeout', 'command': command, 'wall_secs': timeout}
    result = {'command': command, 'wall_secs': round(time.time() - started, 1)}
    stderr = done.stderr.decode('utf-8', 'replace')
    if done.returncode != 0:
        result.update(status='failed', reason='exit %d' % done.returncode, exit=done.returncode,
                      stderr_tail=stderr[-2000:])
        return result
    try:
        report = parse_report(stderr)
    except ValueError as error:
        result.update(status='inadmissible', reason=str(error), stderr_tail=stderr[-2000:])
        return result
    result['report'] = report
    if kind != 'cold' and any(marker in stderr for marker in CACHE_MISS_MARKERS):
        result.update(status='inadmissible', reason='the cache missed, so this was a cold build')
    elif report.get('workload') and report['workload']['errors']:
        result.update(status='inadmissible',
                      reason='workload had %d errors' % report['workload']['errors'])
    else:
        result['status'] = 'ok'
    return result


def capture(command):
    try:
        return subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                              timeout=30).stdout.decode('utf-8', 'replace').strip()
    except (OSError, subprocess.TimeoutExpired):
        return None


def host_state():
    """Swap and compressor state at the time of a run (spec 6.1)."""
    if sys.platform == 'darwin':
        return {'swapusage': capture(['sysctl', '-n', 'vm.swapusage']), 'vm_stat': capture(['vm_stat'])}
    try:
        return {'meminfo': Path('/proc/meminfo').read_text()[:1500]}
    except OSError:
        return {}


def corpus_provenance(path):
    status = capture(['git', '-C', str(path), 'status', '--porcelain']) or ''
    return {'path': str(path), 'commit': capture(['git', '-C', str(path), 'rev-parse', 'HEAD']),
            'dirty': len([line for line in status.splitlines() if line])}


def parse_corpora(pairs):
    corpora = {}
    for pair in pairs:
        name, sep, path = pair.partition('=')
        if not sep or not name or not path:
            raise SystemExit('--corpus takes NAME=PATH, got %r' % pair)
        corpora[name] = Path(path).expanduser().resolve()
    return corpora


def cmd_gen_workload(args):
    WORKLOADS.mkdir(parents=True, exist_ok=True)
    manifest_path = WORKLOADS / 'MANIFEST.json'
    manifest = json.loads(manifest_path.read_text()) if manifest_path.exists() else {}
    for name, path in parse_corpora(args.corpus).items():
        command = [args.bin, '--repo', str(path), '--eager', '--no-cache',
                   '--mem-emit-workload', str(args.functions)]
        done = subprocess.run(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE, timeout=args.timeout,
                              env=dict(os.environ, PRISM_MCP_MEM_REPORT='1'))
        if done.returncode != 0:
            raise SystemExit('%s: workload generation failed:\n%s' % (name, done.stderr.decode()[-2000:]))
        (WORKLOADS / (name + '.jsonl')).write_bytes(done.stdout)
        manifest[name] = dict(corpus_provenance(path), requests=done.stdout.count(b'\n'),
                              functions_requested=args.functions,
                              sha256=hashlib.sha256(done.stdout).hexdigest())
        print('%s: %d requests' % (name, manifest[name]['requests']))
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n')


def cmd_run(args):
    out = Path(args.out)
    summary = {'provenance': {'reps': args.reps, 'settle': args.settle, 'timeout': args.timeout,
                              'bin': args.bin, 'build': None},
               'corpora': {}}
    for name, path in parse_corpora(args.corpus).items():
        workload = WORKLOADS / (name + '.jsonl')
        if not workload.exists():
            raise SystemExit('%s: no workload at %s; run gen-workload first' % (name, workload))
        raw = out / 'raw' / name
        raw.mkdir(parents=True, exist_ok=True)
        cache_dir = out / 'cache' / name
        entry = dict(corpus_provenance(path), admissible={kind: 0 for kind in KINDS},
                     inadmissible=[], metrics={}, attribution=None)
        # Prime the cache with a discarded run so that "warm" means a cache hit.
        prime = run_one(command_for(args.bin, path, cache_dir, 'warm', workload, 0), 'cold', args.timeout)
        (raw / 'prime.json').write_text(json.dumps(prime, indent=1))
        values = {}
        for kind in KINDS:
            for rep in range(1, args.reps + 1):
                before = host_state()
                result = run_one(command_for(args.bin, path, cache_dir, kind, workload, args.settle),
                                 kind, args.timeout)
                result['host_before'] = before
                (raw / ('%s-%d.json' % (kind, rep))).write_text(json.dumps(result, indent=1))
                print('%s %s #%d: %s (%ss)' % (name, kind, rep, result['status'], result['wall_secs']),
                      flush=True)
                if result['status'] != 'ok':
                    entry['inadmissible'].append({'kind': kind, 'rep': rep, 'status': result['status'],
                                                  'reason': result['reason']})
                    continue
                entry['admissible'][kind] += 1
                report = result['report']
                summary['provenance']['build'] = report['build']
                if kind == 'workload':
                    entry['attribution'] = report['attribution']
                for metric, value in derive(kind, report).items():
                    values.setdefault(metric, []).append(value)
        entry['metrics'] = {metric: summarize(series) for metric, series in sorted(values.items())}
        summary['corpora'][name] = entry
    rows = {name: {metric: (stats or {}).get('median') for metric, stats in entry['metrics'].items()}
            for name, entry in summary['corpora'].items()}
    summary['verdict'] = evaluate(rows)
    (out / 'summary.json').write_text(json.dumps(summary, indent=2, sort_keys=True) + '\n')
    print('wrote %s' % (out / 'summary.json'))


def mib(stats):
    if not stats:
        return 'n/a'
    return '%.1f (%.1f–%.1f, n=%d)' % (stats['median'] / MIB, stats['min'] / MIB, stats['max'] / MIB, stats['n'])


def render(summary):
    build = summary['provenance'].get('build') or {}
    lines = [
        '# prism-mcp memory readout',
        '',
        'This is **coarse triage**: it ranks reduction candidates for an experiment and selects none.',
        'Byte figures marked "estimate" come from component counts, not from allocation measurement.',
        '',
        '## Provenance',
        '',
        '- Binary: build identity `%s`, git `%s`, CPG cache version %s, allocator %s.' % (
            build.get('build_identity'), build.get('git_sha'), build.get('cpg_cache_version'),
            build.get('allocator')),
        '- %s repetitions per kind, %s s settle. Values are median (min–max, n) in MiB.' % (
            summary['provenance'].get('reps'), summary['provenance'].get('settle')),
        '',
        '| Corpus | Commit | Dirty files | Admissible runs (warm/cold/workload/refresh) |',
        '|---|---|---|---|',
    ]
    for name, entry in sorted(summary['corpora'].items()):
        counts = '/'.join(str(entry['admissible'][kind]) for kind in KINDS)
        lines.append('| %s | `%s` | %s | %s |' % (name, (entry.get('commit') or '')[:12], entry.get('dirty'), counts))
    lines += ['', '## Footprint', '',
              '| Corpus | Steady state | Warm-load peak | Load transient (M2) | Refresh peak | Cold peak |',
              '|---|---|---|---|---|---|']
    for name, entry in sorted(summary['corpora'].items()):
        metrics = entry['metrics']
        lines.append('| %s | %s | %s | %s | %s | %s |' % (
            name, mib(metrics.get('steady')), mib(metrics.get('peak')), mib(metrics.get('m2_transient')),
            mib(metrics.get('refresh_peak')), mib(metrics.get('cold_peak'))))
    lines += ['', '## Attribution (estimates)', '',
              '| Corpus | Files | Source | Tree nodes | Trees, low–high (M1) | CPG nodes / edges | Repeated node strings (M3) | Unique-byte ratio |',
              '|---|---|---|---|---|---|---|---|']
    for name, entry in sorted(summary['corpora'].items()):
        attribution, metrics = entry.get('attribution') or {}, entry['metrics']
        ratio = attribution.get('node_string_unique_ratio')
        lines.append('| %s | %s | %.1f | %s | %s – %s | %s / %s | %s | %s |' % (
            name, attribution.get('files'), (attribution.get('source_bytes') or 0) / MIB,
            attribution.get('tree_nodes'), mib(metrics.get('m1_low')), mib(metrics.get('m1_high')),
            attribution.get('cpg_nodes'), attribution.get('cpg_edges'), mib(metrics.get('m3')),
            'n/a' if ratio is None else '%.3f' % ratio))
    lines += ['', '## Decision rule', '',
              'A candidate advances to its experiment when its estimate is at least 200 MiB or 15 % of the',
              'base (steady state for M1 and M3, warm-load peak for M2) on at least two corpora.', '',
              '| Candidate | Corpora meeting the rule | Advances |', '|---|---|---|']
    for candidate, _, _ in CANDIDATES:
        verdict = summary['verdict'][candidate]
        lines.append('| %s | %s | %s |' % (candidate, ', '.join(verdict['corpora']) or '—',
                                           'yes' if verdict['advance'] else 'no'))
    excluded = [(name, item) for name, entry in sorted(summary['corpora'].items())
                for item in entry['inadmissible']]
    lines += ['', '## Excluded runs', '']
    if excluded:
        lines += ['| Corpus | Kind | Repetition | Status | Reason |', '|---|---|---|---|---|']
        lines += ['| %s | %s | %s | %s | %s |' % (name, item['kind'], item['rep'], item['status'], item['reason'])
                  for name, item in excluded]
    else:
        lines.append('None.')
    return '\n'.join(lines) + '\n'


def cmd_render(args):
    summary = json.loads((Path(args.run) / 'summary.json').read_text())
    Path(args.out).write_text(render(summary))
    print('wrote %s' % args.out)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest='command', required=True)
    gen = commands.add_parser('gen-workload')
    gen.add_argument('--bin', required=True)
    gen.add_argument('--corpus', action='append', required=True, metavar='NAME=PATH')
    gen.add_argument('--functions', type=int, default=50)
    gen.add_argument('--timeout', type=int, default=3600)
    gen.set_defaults(handler=cmd_gen_workload)
    run = commands.add_parser('run')
    run.add_argument('--bin', required=True)
    run.add_argument('--corpus', action='append', required=True, metavar='NAME=PATH')
    run.add_argument('--out', required=True)
    run.add_argument('--reps', type=int, default=3)
    run.add_argument('--timeout', type=int, default=1800)
    run.add_argument('--settle', type=int, default=5)
    run.set_defaults(handler=cmd_run)
    rendering = commands.add_parser('render')
    rendering.add_argument('--run', required=True)
    rendering.add_argument('--out', required=True)
    rendering.set_defaults(handler=cmd_render)
    args = parser.parse_args(argv)
    args.handler(args)


if __name__ == '__main__':
    main()
```

- [ ] **Step 4: Run the unit tests**

Run: `python3 -m unittest scripts/memreport/test_memreport.py > /tmp/c1-t6-green.log 2>&1; tail -4 /tmp/c1-t6-green.log`

Expected: `Ran 21 tests` and `OK`.

- [ ] **Step 5: Run the driver end to end on a small real repository**

This is the check unit tests with a fake binary cannot make: the real binary, the real flags, a real cache.

```bash
cargo build --release --bin prism-mcp --features mcp > /tmp/c1-t6-build.log 2>&1; tail -1 /tmp/c1-t6-build.log
SMOKE=$(mktemp -d)
mkdir "$SMOKE/repo" && printf 'def helper():\n    return 1\n' > "$SMOKE/repo/util.py" \
  && printf 'from util import helper\n\ndef run():\n    return helper()\n' > "$SMOKE/repo/main.py"
cp -R scripts/memreport "$SMOKE/memreport"      # so the smoke workload never lands in the repo
python3 "$SMOKE/memreport/memreport.py" gen-workload --bin target/release/prism-mcp --corpus smoke="$SMOKE/repo" --functions 2
python3 "$SMOKE/memreport/memreport.py" run --bin target/release/prism-mcp --corpus smoke="$SMOKE/repo" --out "$SMOKE/run" --reps 2 --settle 0
python3 "$SMOKE/memreport/memreport.py" render --run "$SMOKE/run" --out "$SMOKE/readout.md"
python3 - "$SMOKE/run/summary.json" <<'EOF'
import json, sys
summary = json.load(open(sys.argv[1]))
entry = summary['corpora']['smoke']
assert entry['admissible'] == {'warm': 2, 'cold': 2, 'workload': 2, 'refresh': 2}, entry['admissible']
assert entry['inadmissible'] == [], entry['inadmissible']
assert entry['metrics']['steady']['n'] == 2
assert entry['attribution']['files'] == 2
print('DRIVER-SMOKE-OK')
EOF
cat "$SMOKE/readout.md"
```

Expected: `smoke: 8 requests`; eight run lines all `ok`; `DRIVER-SMOKE-OK`; a readout whose tables have one `smoke` row and no excluded runs. If any warm run is `inadmissible` for a cache miss, the priming run is not populating the cache the measured runs read: fix that before going on, since every later warm number depends on it.

- [ ] **Step 6: Write the README**

Create `scripts/memreport/README.md`:

````markdown
# memreport: where one prism-mcp index's memory goes

Measurement only. `prism-mcp` has a report mode (`PRISM_MCP_MEM_REPORT=1 … --eager`) that builds
the index in stages, optionally runs a query workload and one refresh, prints one JSON line to
stderr and exits without serving. This driver runs that mode per corpus and renders a readout.

```bash
cargo build --release --bin prism-mcp --features mcp
python3 scripts/memreport/memreport.py gen-workload --bin target/release/prism-mcp --corpus ruff=~/code/bench-repos/ruff
python3 scripts/memreport/memreport.py run --bin target/release/prism-mcp --corpus ruff=~/code/bench-repos/ruff --out ~/prism-evidence/mcp-memory/c1/run-1
python3 scripts/memreport/memreport.py render --run ~/prism-evidence/mcp-memory/c1/run-1 --out docs/analysis/readout.md
python3 -m unittest scripts/memreport/test_memreport.py
```

## What a run is

Four kinds per corpus, each repeated (default three times), one process at a time:

| Kind | Command adds | Measures |
|---|---|---|
| `warm` | `--cache-dir <run>/cache/<corpus>` after a priming run | load from the on-disk cache |
| `cold` | `--no-cache` | a full build |
| `workload` | the corpus workload | steady state after queries and a settle |
| `refresh` | `--mem-refresh` | the peak while two snapshots coexist |

## What is excluded, and why

A run is inadmissible, and left out of every statistic, when it timed out, exited non-zero, printed
no single report line, missed the cache when it was meant to be warm, or had workload errors. Each
of those means the run measured something other than what its kind names. Excluded runs are listed
in the readout; they are never retried silently.

## What the numbers are evidence of

- Stage footprints and peaks are measurements of this process on this host.
- Tree and string byte figures are **estimates** from exact counts. They rank candidates.
- The decision rule (200 MiB or 15 % on two corpora) decides whether an experiment is worth
  running. It does not decide what ships; each candidate's experiment measures its real saving.
- Freed memory can stay in the footprint, so nothing here is attributed by freeing and re-measuring.

## Workloads

`workloads/<corpus>.jsonl` is a fixed list of `tools/call` requests generated once per corpus
commit; `workloads/MANIFEST.json` records the commit, request count and hash. A baseline and a
candidate build must run the same file. Regenerate only when the corpus commit changes.
````

- [ ] **Step 7: Commit**

```bash
git add scripts/memreport/memreport.py scripts/memreport/test_memreport.py scripts/memreport/README.md
git commit -m "feat(scripts): memreport driver for the prism-mcp memory readout"
```

---

### Task 7: Verify the harness PR, then measure the five corpora

Steps 1–2 finish the harness PR (implementer). Steps 3–6 are the measurement, run by the controller on the merged binary so the readout's build identity is a commit on `main`.

**Files:**
- Create: `scripts/memreport/workloads/{slicing,ruff,excalidraw,prometheus,kubernetes}.jsonl`, `scripts/memreport/workloads/MANIFEST.json`
- Create: `docs/analysis/2026-10-prism-mcp-memory-readout.md`

**Interfaces:**
- Consumes: everything above.
- Produces: the readout that C2 planning starts from.

- [ ] **Step 1: Run the full verification set for the harness PR**

```bash
cargo fmt --check
cargo test --features mcp > /tmp/c1-full-mcp.log 2>&1; grep -E '^test result|FAILED|failed' /tmp/c1-full-mcp.log
cargo check > /tmp/c1-default-check.log 2>&1; tail -1 /tmp/c1-default-check.log
cargo test --doc --features mcp > /tmp/c1-doc.log 2>&1; grep -E '^test result' /tmp/c1-doc.log
cargo clippy --all-targets --features mcp -- -W clippy::all > /tmp/c1-clippy.log 2>&1
grep -E -A8 '^warning' /tmp/c1-clippy.log | grep -E 'src/mcp/mem_|src/mcp/session_stage_tests|src/bin/prism-mcp|tests/mcp/mem_report' | sort -u
python3 -m unittest scripts/memreport/test_memreport.py > /tmp/c1-py.log 2>&1; tail -3 /tmp/c1-py.log
python3 scripts/mutgate/mutgate.py > /tmp/c1-mutgate.log 2>&1; tail -5 /tmp/c1-mutgate.log
```

Expected: no formatting diff; every `test result` line `0 failed` (report the summed `passed`); no clippy warning in the files this slice created; `OK` from the Python tests; the mutation gate exits 0. State the gate's limit in the PR: it builds without `--features mcp` and so selects none of this slice's code.

- [ ] **Step 2: Open the harness PR**

Title: `feat(mcp): memory-report harness and driver (shared daemon C1)`

The body states: the lane goal; that serving behavior is unchanged with the variable unset (the `smoke_test` and, if S1 has merged, the golden transcripts pass untouched); the totals from Step 1; the end-to-end driver smoke from Task 6 Step 5 with its output; and what is not verified (no non-macOS run of `sample_tracks_a_touched_allocation` except CI's Linux run; the five-corpus measurement follows in a separate docs PR).

- [ ] **Step 3: Prepare the corpora and the evidence directory (controller, after merge)**

```bash
git -C ~/code/slicing fetch origin
git -C ~/code/slicing worktree add --detach ~/code/prism-c1-corpus origin/main
cd ~/code/prism-c1-corpus && cargo build --release --bin prism-mcp --features mcp > /tmp/c1-m-build.log 2>&1; tail -1 /tmp/c1-m-build.log
for r in ruff excalidraw prometheus kubernetes; do
  echo "$r $(git -C ~/code/bench-repos/$r rev-parse --short=12 HEAD) dirty=$(git -C ~/code/bench-repos/$r status --porcelain | wc -l | tr -d ' ')"
done
mkdir -p ~/prism-evidence/mcp-memory/c1
pgrep -fl prism-mcp | wc -l; sysctl -n vm.swapusage
```

Expected commits: ruff `44f6d1865fd2`, excalidraw `0642e72cfa2d`, prometheus `505095b64b43`, kubernetes `8d34ac5ef882`, all `dirty=0`. If one differs, use what is there and let the manifest record it; do not check out a different commit in a shared corpus. `~/code/prism-c1-corpus` is both the measured binary's source and the clean "slicing" corpus. Note the count of other `prism-mcp` processes and the swap in use: a host already swapping makes every footprint number suspect, so stop long-lived sessions first if swap is in use.

- [ ] **Step 4: Generate the workloads and run the measurement**

Declared budget: 3 hours wall for this step. Each run is capped at 1,800 s by the driver.

```bash
cd ~/code/prism-c1-corpus
BIN=target/release/prism-mcp
CORPORA="--corpus slicing=$HOME/code/prism-c1-corpus --corpus excalidraw=$HOME/code/bench-repos/excalidraw --corpus prometheus=$HOME/code/bench-repos/prometheus --corpus ruff=$HOME/code/bench-repos/ruff --corpus kubernetes=$HOME/code/bench-repos/kubernetes"
python3 scripts/memreport/memreport.py gen-workload --bin $BIN $CORPORA > ~/prism-evidence/mcp-memory/c1/gen-workload.log 2>&1; tail -6 ~/prism-evidence/mcp-memory/c1/gen-workload.log
python3 scripts/memreport/memreport.py run --bin $BIN $CORPORA --out ~/prism-evidence/mcp-memory/c1/run-1 > ~/prism-evidence/mcp-memory/c1/run-1.log 2>&1
tail -45 ~/prism-evidence/mcp-memory/c1/run-1.log
```

Run it in the background and read the log when it finishes. Kubernetes is last so a failure there costs nothing already measured. If workload generation for kubernetes exceeds its hour, or its runs time out, that is a result: it is recorded as excluded and the rule is evaluated on the corpora that completed. Do not raise the timeout and re-run to get a number; report the exclusion and let the owner decide whether kubernetes is worth a longer budget.

- [ ] **Step 5: Render the readout and check it against the raw reports**

```bash
python3 scripts/memreport/memreport.py render --run ~/prism-evidence/mcp-memory/c1/run-1 --out docs/analysis/2026-10-prism-mcp-memory-readout.md
python3 - ~/prism-evidence/mcp-memory/c1/run-1 <<'EOF'
import json, sys
from pathlib import Path
run = Path(sys.argv[1])
summary = json.loads((run / 'summary.json').read_text())
for name, entry in sorted(summary['corpora'].items()):
    raw = json.loads((run / 'raw' / name / 'workload-1.json').read_text())
    if raw['status'] != 'ok':
        print(name, 'workload-1', raw['status'], raw['reason']); continue
    stages = {s['name']: s for s in raw['report']['stages']}
    print(name, 'steady median MiB %.1f' % (entry['metrics']['steady']['median'] / 2**20),
          '| raw rep 1 footprint after_settle MiB %.1f' % (stages['after_settle']['phys_footprint_bytes'] / 2**20),
          '| workload errors', raw['report']['workload']['errors'])
EOF
```

Expected: for each corpus the summary's median steady state is within the min–max of its raw repetitions, and workload errors are 0. Compare the slicing row with the lane's earlier measurement (1,168 MB resident for an idle built index, 1,620 MB peak for a warm load): the same order of magnitude confirms the harness measures the same thing; a large gap is a finding to explain in the readout, not to smooth over.

Then add, by hand, a short section at the end of the readout titled **What this run does and does not show**, covering: which candidates advance and on which corpora; any excluded run and its reason; host swap state during the run; that tree and string figures are estimates; and the comparison with the 2026-10-09 slicing numbers.

- [ ] **Step 6: Commit the workloads and the readout, and put the raw evidence in custody**

```bash
cd ~/code/prism-c1-corpus && git switch -c docs/mcp-memory-readout
git add scripts/memreport/workloads docs/analysis/2026-10-prism-mcp-memory-readout.md
git commit -m "docs(analysis): five-corpus prism-mcp memory readout (shared daemon C1)"
git push -u origin docs/mcp-memory-readout
(cd ~/prism-evidence/mcp-memory/c1 && rm -rf run-1/cache && shasum -a 256 run-1/summary.json run-1.log gen-workload.log > SHA256SUMS && cat SHA256SUMS)
```

The cache directory is removed before custody: it is a rebuildable copy of each corpus's index, hundreds of megabytes each, and not evidence. Open the docs PR, link the readout, and state the verdict table in the PR body. C2 planning starts from that table.

---

## Self-review record

- **Spec coverage (§6.1):** report on stderr with per-stage resident size, `phys_footprint`, compressed bytes and `ru_maxrss` → Tasks 1, 4. Stages after `load_repo`, after the CPG load or build, after the call-edge index, after the workload → Tasks 3, 4. Workload through the in-process transport → Task 4 (`serve_session_with_freshness`). Four runs per corpus, three repetitions, min and median → Task 6. Component attribution (source bytes, tree nodes × per-node size, CPG node and edge counts, string duplication ratio) → Task 2. Five corpora with commit, dirty state, cache version, grammar fingerprint, build identity, allocator and host swap state → Tasks 4, 6, 7. Pinned experiment parameters → Global Constraints and the driver defaults. **§6.2** decision rule → Task 6 `evaluate`, Task 7 readout. **§12 row C1** readout under `docs/analysis/` → Task 7.
- **Deviations from the spec text, each deliberate:**
  - The spec's example command ends in `< /dev/null`, implying the server would then serve an empty stdin. Report mode does not serve at all and ignores stdin; the result is the same and there is no handshake path to keep working.
  - `--mem-workload` takes a file of requests rather than a seed list, and `--mem-emit-workload` generates it, so a baseline and a candidate build run byte-identical requests.
  - Per-stage peaks come from the lifetime peak sampled at each stage, not from a sampling thread: the stage in which the peak rose is the stage that set it, at no measurement overhead.
  - "The noise treatment (a candidate must beat the baseline's max by its own min)" is a C2 experiment rule. C1 reports min and max so that rule can be applied; it compares nothing.
- **Out of C1:** any reduction (C2); the fan-out latency gate (needs the shim, S5a).
