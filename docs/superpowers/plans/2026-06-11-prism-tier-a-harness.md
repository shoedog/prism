# Tier-A Accuracy Harness + Dev-Loop Test-Time Reduction — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Tier-A edge-level accuracy harness (`eval/`, 4 measurements, 5
corpora, adjudicated LSP-oracle comparison) and cut debug `cargo test` wall time from
~21 min to <8 min via 121→24 test-binary consolidation + profile tuning.

**Architecture:** WP2 first (per spec §5 — the link-count cut pays for itself across
WP1's Rust commits): per-directory umbrella test binaries applied in 4 compiling,
committed batches, then profile tuning, each independently measured. WP1 second: two
small Rust surfaces (`GIT_SHA` build identity, `nav functions`), then the Python harness
bottom-up — schemas → LSP client → adapters → measurements → reports — every layer
TDD'd against fakes/canned wire data; live oracles touched only in the final
orchestrator-run baseline task.

**Tech Stack:** Rust (existing workspace; clap, tree-sitter), Python 3.12 via uv
(stdlib-only runtime, pytest dev-dep), LSP servers (rust-analyzer, gopls, pyright) as
oracles, a2a-bridge/codex for containerized implementation.

**Spec:** `docs/superpowers/specs/2026-06-11-prism-tier-a-accuracy-harness-design.md`
(rev 4, owner-approved). Section references (§…) below point there.

**Execution model (spec §5):** containerized codex implements + verifies
(`cargo fmt`/`build`/`test --locked`; eval pytest only if the image has Python 3.12 +
pytest). Tasks marked **[ORCHESTRATOR]** run host-side (timing, live oracles, corpus
prep, baseline) — do not dispatch those to the container.

---

## Part A — WP2: test-suite wall-time reduction

### Task 1: [ORCHESTRATOR] Baseline timing (P1/P2, pre-WP2)

**Files:**
- Create: `docs/eval/wp2-timing.md`

- [ ] **Step 1: Measure P1 (clean) and P2 (dev-loop) on current main**

Run (machine otherwise idle; P2 immediately after P1 on its warmed target dir — spec §3.3):

```bash
cargo clean && /usr/bin/time -l cargo test 2>&1 | tail -25
touch src/lib.rs && /usr/bin/time -l cargo test 2>&1 | tail -25
```

Expected: P1 and P2 real/user/sys + max RSS captured; P2 ≈ the known ~21-minute number.

- [ ] **Step 2: Record the baseline point**

Create the directory (`mkdir -p docs/eval`) and write `docs/eval/wp2-timing.md`:

```markdown
# WP2 timing protocol results (spec §3.3)

Protocol: P1 = `cargo clean && /usr/bin/time -l cargo test`;
P2 = `touch src/lib.rs && /usr/bin/time -l cargo test`, run immediately after P1.
Single timed pair per point, machine otherwise idle. Gate: P2 < 8 min (G7).

| Point | P1 real | P1 user | P1 sys | P1 maxRSS | P2 real | P2 user | P2 sys | P2 maxRSS |
|---|---|---|---|---|---|---|---|---|
| pre-WP2 (main @ <sha>) | … | … | … | … | … | … | … | … |
| post-3.1 consolidation | | | | | | | | |
| post-3.1+3.2 profile | | | | | | | | |
```

(fill the pre-WP2 row with measured values and the commit SHA)

- [ ] **Step 3: Commit**

```bash
git add docs/eval/wp2-timing.md
git commit -m "docs(wp2): baseline P1/P2 timing per spec §3.3 protocol"
```

### Task 2: Consolidation batch 1 — `tests/ast` + `tests/algo/*` (48 → 5 targets)

**Files:**
- Create: `tests/ast/main.rs`, `tests/algo/novel/main.rs`, `tests/algo/taxonomy/main.rs`,
  `tests/algo/theoretical/main.rs`, `tests/algo/paper/main.rs`
- Modify: `Cargo.toml`, every `tests/ast/*.rs`, `tests/algo/**/*.rs` (header rewrite)

Background for whoever implements this: today every test file is its own crate with a
`#[path = "../common/mod.rs"] mod common; use common::*;` header. Under an umbrella
binary the files become modules of one crate, so `common` must be declared **once** in
`main.rs` and files must import it as `use crate::common::*;` (spec §3.1; the glob form
is load-bearing — 93 files call helpers unqualified, and there are **zero** inline
`common::` references outside those two header lines, verified). Module names are
file-derived, no `#[path]` aliasing (spec §3.1).

- [ ] **Step 1: Write the transform script (committed — Tasks 3–5 and any resumed
session need it from the tree)**

Save as `scripts/consolidate_tests.py` and include it in this task's commit:

```python
#!/usr/bin/env python3
"""Per-directory test consolidation (WP2, spec §3.1). Usage:
   python3 scripts/consolidate_tests.py tests/ast ast
   python3 scripts/consolidate_tests.py tests/mcp mcp --required-features mcp
Idempotence guard: refuses to run if <dir>/main.rs already exists.
"""
import re, sys, pathlib

d = pathlib.Path(sys.argv[1])
target = sys.argv[2]
req_feat = sys.argv[4] if "--required-features" in sys.argv else None

assert not (d / "main.rs").exists(), f"{d} already consolidated"
files = sorted(p for p in d.glob("*.rs") if p.name != "main.rs")
assert files, f"no test files under {d}"

header = re.compile(r'#\[path = "[^"]*common/mod\.rs"\]\s*\nmod common;\n')
uses_common = False
for p in files:
    s = p.read_text()
    s2 = header.sub("", s)
    s2 = s2.replace("use common::", "use crate::common::")
    if s2 != s:
        uses_common = True
        p.write_text(s2)

rel = "../" * (len(d.parts) - 1) + "common/mod.rs"
lines = []
if uses_common:
    lines.append(f'#[allow(dead_code)]\n#[path = "{rel}"]\nmod common;\n')
lines += [f"mod {p.stem};\n" for p in files]
(d / "main.rs").write_text("".join(lines))

c = pathlib.Path("Cargo.toml").read_text()
block = re.compile(
    r'\[\[test\]\]\nname = "[^"]+"\npath = "' + re.escape(str(d))
    + r'/[^"]+"\n(required-features = \[[^\]]*\]\n)?\n?'
)
n_removed = len(block.findall(c))
c = block.sub("", c)
entry = f'\n[[test]]\nname = "{target}"\npath = "{d}/main.rs"\n'
if req_feat:
    entry += f'required-features = ["{req_feat}"]\n'
pathlib.Path("Cargo.toml").write_text(c.rstrip() + "\n" + entry)
print(f"{target}: {len(files)} files, removed {n_removed} old [[test]] blocks")
```

Note: if the script's Cargo.toml regex removes a different count than the migration
table's "absorbs" column (spec §3.1 table), STOP and inspect — the `[[test]]` block
layout differs from the assumed `name`-then-`path` order for that entry; fix the block
by hand rather than loosening the regex.

- [ ] **Step 2: Run for batch-1 directories**

```bash
python3 scripts/consolidate_tests.py tests/ast ast
python3 scripts/consolidate_tests.py tests/algo/novel algo_novel
python3 scripts/consolidate_tests.py tests/algo/taxonomy algo_taxonomy
python3 scripts/consolidate_tests.py tests/algo/theoretical algo_theoretical
python3 scripts/consolidate_tests.py tests/algo/paper algo_paper
grep -c '^\[\[test\]\]' Cargo.toml
```

Expected: per-dir lines reporting 16/15/11/5/1 files and the same removed counts;
`[[test]]` count = 121 − 48 + 5 = **78**.

- [ ] **Step 3: Build and run the new targets**

```bash
cargo fmt
cargo test --test ast --test algo_novel --test algo_taxonomy --test algo_theoretical --test algo_paper
```

Expected: PASS, same test counts as before (spot-check one module filter:
`cargo test --test algo_taxonomy taint_cve_test::` runs the taint-CVE tests).
Common failure here is a name collision between a helper `fn` defined at file scope in
two sibling files — there are none known, but if one appears, rename the *private
helper* in one file (never a `#[test]` fn) and note it in the commit message.

- [ ] **Step 4: Commit**

```bash
git add -A tests/ast tests/algo Cargo.toml scripts/consolidate_tests.py
git commit -m "test(wp2): consolidate ast+algo test dirs into 5 umbrella targets (48->5)"
```

### Task 3: Consolidation batch 2 — `tests/lang/*` (37 → 11 targets)

**Files:**
- Create: `tests/lang/{c,cpp,go,java,javascript,lua,rust,terraform,tsx,typescript,bash}/main.rs`
- Modify: `Cargo.toml`, every `tests/lang/**/*.rs`

- [ ] **Step 1: Run the transform for all 11 language dirs**

```bash
for d in c cpp go java javascript lua rust terraform tsx typescript bash; do
  python3 scripts/consolidate_tests.py tests/lang/$d lang_$d
done
grep -c '^\[\[test\]\]' Cargo.toml
```

Expected: 6/2/3/2/5/3/3/3/4/3/3 files per dir (37 total); `[[test]]` count = 78 − 37 + 11 = **52**.

- [ ] **Step 2: Build and run**

```bash
cargo fmt && cargo test --test lang_c --test lang_cpp --test lang_go --test lang_java \
  --test lang_javascript --test lang_lua --test lang_rust --test lang_terraform \
  --test lang_tsx --test lang_typescript --test lang_bash
```

Expected: PASS, unchanged test counts.

- [ ] **Step 3: Commit**

```bash
git add -A tests/lang Cargo.toml
git commit -m "test(wp2): consolidate tests/lang into 11 umbrella targets (37->11)"
```

### Task 4: Consolidation batch 3 — `navigation`, `integration`, `frameworks`, `cli` (32 → 4 targets)

**Files:**
- Create: `tests/{navigation,integration,frameworks,cli}/main.rs`
- Modify: `Cargo.toml`, files in those dirs

- [ ] **Step 1: Run the transform**

```bash
for pair in "navigation navigation" "integration integration" "frameworks frameworks" "cli cli"; do
  set -- $pair; python3 scripts/consolidate_tests.py tests/$1 $2
done
grep -c '^\[\[test\]\]' Cargo.toml
```

Expected: 11/8/8/5 files; `[[test]]` count = 52 − 32 + 4 = **24**. (Four of those 24 are
still the old single-file targets — reasoning, output, mcp, infra; Task 5 converts them
to umbrellas 1:1 without changing the count.)

- [ ] **Step 2: Build and run — including the coverage matrix gate**

```bash
cargo fmt && cargo test --test navigation --test integration --test frameworks --test cli
cargo test --test integration coverage_test::
```

Expected: PASS. The coverage matrix's three path lists reference file paths that all
still exist (spec §3.1) — if `coverage_test::` fails, a file moved instead of being
re-registered; nothing in this plan moves files.

- [ ] **Step 3: Commit**

```bash
git add -A tests/navigation tests/integration tests/frameworks tests/cli Cargo.toml
git commit -m "test(wp2): consolidate navigation/integration/frameworks/cli (32->4)"
```

### Task 5: Consolidation batch 4 — singles + docs sweep (G6)

**Files:**
- Create: `tests/{reasoning,output,mcp,infra}/main.rs`
- Modify: `Cargo.toml`, `CLAUDE.md`, files in those dirs

- [ ] **Step 1: Consolidate the four single-file dirs (uniformity — filters work the same everywhere)**

```bash
python3 scripts/consolidate_tests.py tests/reasoning reasoning
python3 scripts/consolidate_tests.py tests/output output
python3 scripts/consolidate_tests.py tests/mcp mcp --required-features mcp
python3 scripts/consolidate_tests.py tests/infra infra
grep -c '^\[\[test\]\]' Cargo.toml
```

Expected: **24** — the spec §3.1 migration table, complete. Verify
`grep -A3 'name = "mcp"' Cargo.toml` still shows `required-features = ["mcp"]`.

- [ ] **Step 2: Full suite + sweep**

```bash
cargo fmt && cargo test && cargo test --features mcp --test mcp
grep -rn -- "--test " . --include="*.md" --include="*.yml" --include="*.yaml" \
  --include="*.json" --include="*.py" --include="*.sh" 2>/dev/null | grep -v "^./target"
grep -rEn "tests/(ast|algo|lang|navigation|integration|frameworks|cli|reasoning|output|mcp|infra)/[a-z_]+\.rs" scripts/ .github/ 2>/dev/null
```

Expected: full suite PASS. The first grep is **repo-wide** (G6): fix CLAUDE.md and
docs/MCP.md hits, and **fix `.claude/settings.local.json`** (it carries `--test`-pattern
permission entries that would silently stop matching — live config, not historical);
leave purely historical records (`STATUS-prism-cwe-phase*.md`,
`docs/prism-query-layer/*.md` review records) untouched and name them as exempt in the
commit message. The second grep (path-style) is expected clean —
`scripts/extract_tests.py` has zero `--test` references (verified).

- [ ] **Step 3: Rewrite CLAUDE.md's test-suite documentation**

In `CLAUDE.md`, replace the "Run specific test suites" block with:

```markdown
Run specific test suites (one umbrella binary per tests/ subdirectory; filter by
file-derived module name):
```bash
cargo test --test algo_paper                       # Paper algorithm tests
cargo test --test algo_taxonomy taint_cve_test::   # Taint CVE tests
cargo test --test lang_c algo_test::               # C language-specific tests
cargo test --test cli validation_test::            # CLI validation tests
cargo test --test integration core_test::          # Core integration tests
cargo test --test integration coverage_test::      # Coverage matrix
```
```

and in "Key Design Decisions" item 7, replace the sentence
"…and register each as a separate `[[test]]` target in `Cargo.toml`." with:
"…and register **one umbrella `[[test]]` target per `tests/` subdirectory** (its
`main.rs` declares the files as modules); individual files stay under 600 lines."
Also update the §"Language Coverage Matrix" run command to
`cargo test --test integration coverage_test::`, and the §"Adding a New Slicing
Algorithm"/"Adding a New Language" steps that mention registering `[[test]]` targets to
say "add a `mod <stem>;` line to that directory's `main.rs`".

- [ ] **Step 4: Re-run full suite 3× (G6 — concurrency topology changed)**

```bash
for i in 1 2 3; do cargo test 2>&1 | tail -3; done
```

Expected: 3 consecutive PASS runs (test execution is <2 min once built).

- [ ] **Step 5: Commit**

```bash
git add -A tests Cargo.toml CLAUDE.md
git commit -m "test(wp2): final consolidation batch + docs sweep — 121->24 targets (G6)"
```

### Task 6: Profile tuning (spec §3.2)

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add the profile**

Append to `Cargo.toml`:

```toml
[profile.dev]
debug = "line-tables-only"
```

- [ ] **Step 2: Verify build + that a backtrace genuinely keeps file:line**

A passing run emits no backtrace, so probe with a deliberate (uncommitted) failure:

```bash
cargo test --test infra 2>&1 | tail -3       # suite still green under the new profile
cat >> tests/infra/main.rs << 'EOF'
#[test]
fn wp2_backtrace_probe() {
    panic!("probe");
}
EOF
RUST_BACKTRACE=1 cargo test --test infra wp2_backtrace_probe 2>&1 | grep -E "tests/infra|src/" | head -5
git checkout tests/infra/main.rs            # revert the probe before committing
```

Expected: backtrace frames show `tests/infra/main.rs:NN` (file:line preserved under
`line-tables-only`); probe reverted.

- [ ] **Step 3: Commit (separate from 3.1 so its timing contribution is attributable)**

```bash
git add Cargo.toml
git commit -m "build(wp2): profile.dev debug=line-tables-only (spec §3.2)"
```

### Task 7: [ORCHESTRATOR] Post-WP2 timing + gate G7

- [ ] **Step 1: Measure post-3.1 (check out the Task 5 commit) and post-3.1+3.2 (Task 6 commit) with the Task 1 protocol; fill both rows of `docs/eval/wp2-timing.md`**
- [ ] **Step 2: Evaluate G7 (P2 < 8 min).** If missed: record honestly with attribution
      and add a "nextest/linker options move to proposed" note (spec §3.3) — do not waive.
- [ ] **Step 3: Commit**

```bash
git add docs/eval/wp2-timing.md
git commit -m "docs(wp2): post-consolidation + post-profile P1/P2 timing (gate G7)"
```

---

## Part B — WP1: Tier-A accuracy harness

### Task 8: Build identity — `GIT_SHA` + dirty flag in `prism --version` (spec §2.3)

**Files:**
- Modify: `build.rs`, `src/main.rs:36-40`
- Test: `tests/cli/version_test.rs` (+ `mod version_test;` in `tests/cli/main.rs`)

- [ ] **Step 1: Write the failing test**

`tests/cli/version_test.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_version_includes_git_sha_and_dirty_flag() {
    // Tier-A spec §2.3: CARGO_PKG_VERSION is constant across dev commits; the
    // harness's stale-SUT detection needs the build's git identity in --version.
    Command::cargo_bin("prism")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(
            predicate::str::is_match(r"^slicing \d+\.\d+\.\d+ \([0-9a-f]{12}(-dirty)?\)\n$")
                .unwrap(),
        );
}
```

Add `mod version_test;` to `tests/cli/main.rs`.

- [ ] **Step 2: Run it, verify it fails**

Run: `cargo test --test cli version_test::`
Expected: FAIL — stdout is `slicing 3.1.2` with no `(sha)` suffix.

- [ ] **Step 3: Implement**

Append to `build.rs` `main()` (after the GRAMMAR_FINGERPRINT print):

```rust
    // Build identity (Tier-A spec §2.3): GIT_SHA + dirty for stale-SUT detection.
    // rerun-if-changed on HEAD *and* the ref it points at, so the SHA can't go stale
    // across commits; dirty is best-effort between edits (harness HEAD-check backstops).
    println!("cargo:rerun-if-changed={manifest}/.git/HEAD");
    if let Ok(head) = std::fs::read_to_string(format!("{manifest}/.git/HEAD")) {
        if let Some(r) = head.trim().strip_prefix("ref: ") {
            println!("cargo:rerun-if-changed={manifest}/.git/{r}");
        }
    }
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&manifest)
            .output()
            .ok()
            .filter(|o| o.status.success())
    };
    let sha = git(&["rev-parse", "--short=12", "HEAD"])
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "nogit".repeat(2));
    let dirty = git(&["status", "--porcelain"])
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    println!(
        "cargo:rustc-env=GIT_SHA={sha}{}",
        if dirty { "-dirty" } else { "" }
    );
```

In `src/main.rs`, change the clap attribute `version = env!("CARGO_PKG_VERSION"),` to:

```rust
    version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_SHA"), ")"),
```

- [ ] **Step 4: Run the test, verify it passes**

Run: `cargo test --test cli version_test::`
Expected: PASS (in a clean checkout the suffix is `(abcdef123456)`; with local edits
`-dirty` — the regex accepts both; `nogitnogit` would fail the regex, which is correct:
the harness must not run against a non-git build).

- [ ] **Step 5: Commit**

```bash
git add build.rs src/main.rs tests/cli
git commit -m "feat(eval): embed GIT_SHA+dirty in --version for stale-SUT detection (§2.3)"
```

### Task 9: `prism nav functions` — inventory dump with dedup (spec §2.3)

**Files:**
- Create: `src/navigation/inventory.rs`
- Modify: `src/navigation/mod.rs` (add `pub mod inventory;`), `src/main.rs` (NavQuery variant + arm)
- Test: `tests/navigation/inventory_test.rs` (+ `mod inventory_test;` in `tests/navigation/main.rs`)

- [ ] **Step 1: Write the failing tests**

`tests/navigation/inventory_test.rs`:

```rust
use prism::navigation::inventory::functions_inventory;

#[test]
fn test_python_decorated_function_emits_one_record() {
    // §2.3 dedup: queries.rs captures BOTH (function_definition) and
    // (decorated_definition) for Python — without dedup this is two records.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.py"),
        "import functools\n\n@functools.cache\ndef handler(x):\n    return x\n",
    )
    .unwrap();
    let recs = functions_inventory(dir.path()).unwrap();
    assert_eq!(recs.len(), 1, "expected exactly one record, got {recs:?}");
    assert_eq!(recs[0].name.as_deref(), Some("handler"));
}

#[test]
fn test_sorted_with_resolved_kind_names() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), "fn alpha() {}\nfn beta() {}\n").unwrap();
    let recs = functions_inventory(dir.path()).unwrap();
    assert_eq!(recs.len(), 2);
    assert!(recs[0].start_line < recs[1].start_line, "sorted by (file, start_line)");
    assert_eq!(recs[0].kind, "function_item", "kind_id resolved to grammar kind name");
    assert_eq!(recs[0].name.as_deref(), Some("alpha"));
}
```

- [ ] **Step 2: Run, verify failure**

Run: `cargo test --test navigation inventory_test::`
Expected: FAIL — `prism::navigation::inventory` does not exist.

- [ ] **Step 3: Implement `src/navigation/inventory.rs`**

```rust
//! Whole-repo function inventory from the S1 FunctionTable (Tier-A spec §2.3).
//! Deliberately NOT the nav CPG index: CpgNode::Function carries no kind.
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FunctionRecord {
    pub file: String,
    pub name: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub kind: String,
}

pub fn functions_inventory(repo: &Path) -> anyhow::Result<Vec<FunctionRecord>> {
    let loaded = crate::repo_loader::load_repo(repo)?;
    let mut out = Vec::new();
    for (file, pf) in &loaded.files {
        let lang = pf.tree.language();
        let recs: Vec<FunctionRecord> = pf
            .functions()
            .iter()
            .map(|f| FunctionRecord {
                file: file.clone(),
                name: f.name.clone(),
                start_line: f.start_line,
                end_line: f.end_line,
                kind: lang
                    .node_kind_for_id(f.kind_id)
                    .unwrap_or("unknown")
                    .to_string(),
            })
            .collect();
        // §2.3 dedup: when records nest, keep the innermost — drop an outer record
        // when it shares the inner's name, or when it is a decorated_definition
        // wrapper (whose own name capture may be absent). Wrapper kinds never
        // survive over their inner definition.
        let mut keep = vec![true; recs.len()];
        for i in 0..recs.len() {
            for j in 0..recs.len() {
                if i == j {
                    continue;
                }
                let (outer, inner) = (&recs[i], &recs[j]);
                let contains = outer.start_line <= inner.start_line
                    && inner.end_line <= outer.end_line
                    && (outer.start_line, outer.end_line)
                        != (inner.start_line, inner.end_line);
                let same_name = outer.name.is_some() && outer.name == inner.name;
                let wrapper = outer.kind == "decorated_definition";
                if contains && (same_name || wrapper) {
                    keep[i] = false;
                }
            }
        }
        let mut it = keep.iter();
        let mut recs = recs;
        recs.retain(|_| *it.next().unwrap());
        out.extend(recs);
    }
    out.sort_by(|a, b| {
        (&a.file, a.start_line, a.end_line).cmp(&(&b.file, b.start_line, b.end_line))
    });
    out.dedup();
    Ok(out)
}
```

Add `pub mod inventory;` to `src/navigation/mod.rs`.

- [ ] **Step 4: Wire the CLI**

In `src/main.rs`, add to `enum NavQuery` (after `RepoMap`):

```rust
    /// Whole-repo function inventory from the FunctionTable (Tier-A spec §2.3).
    Functions {
        #[arg(long)]
        repo: std::path::PathBuf,
        #[arg(long, default_value = "json", value_parser = ["text", "json"])]
        format: String,
    },
```

and to the `run_nav` match:

```rust
        NavQuery::Functions { repo, format } => {
            let recs = prism::navigation::inventory::functions_inventory(repo)?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&recs)?);
            } else {
                for r in &recs {
                    println!(
                        "{}:{}-{} {} [{}]",
                        r.file,
                        r.start_line,
                        r.end_line,
                        r.name.as_deref().unwrap_or("<anon>"),
                        r.kind
                    );
                }
            }
            Ok(())
        }
```

Note this is a **plain JSON array**, not the Evidence envelope — it is an inventory
dump (spec §2.3); existing outputs are untouched (additive-only rule).

- [ ] **Step 5: Run tests + smoke the CLI**

```bash
cargo test --test navigation inventory_test::
cargo run --release -- nav functions --repo . --format json | head -12
```

Expected: tests PASS; CLI emits a sorted JSON array whose first records are from
the lexicographically first source file.

- [ ] **Step 6: Commit**

```bash
git add src/navigation src/main.rs tests/navigation
git commit -m "feat(nav): functions inventory subcommand with innermost-dedup (§2.3)"
```

### Task 10: `eval/` scaffold + schemas/normalization (spec §2.1)

**Files:**
- Create: `eval/pyproject.toml`, `eval/.python-version`, `eval/tier_a/__init__.py`,
  `eval/tier_a/model.py`, `eval/tier_a/interfaces.py`, `eval/tests/test_model.py`,
  `eval/README.md` (skeleton), `eval/.gitignore`

All eval tasks run from `eval/`: `cd eval && uv run pytest -q`. (Container fallback:
`python3 -m pytest -q` if Python 3.12 + pytest are present; otherwise eval pytest is a
host-side gate — spec §5.)

- [ ] **Step 1: Scaffold the project**

`eval/pyproject.toml`:

```toml
[project]
name = "prism-eval"
version = "0.1.0"
description = "Tier-A accuracy harness for prism (spec: docs/superpowers/specs/2026-06-11-prism-tier-a-accuracy-harness-design.md)"
requires-python = ">=3.12"
dependencies = []

[dependency-groups]
dev = ["pytest>=8"]

[project.scripts]
tier-a = "tier_a.cli:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["tier_a"]
```

`eval/.python-version`: `3.12`

`eval/.gitignore`:

```
__pycache__/
.pytest_cache/
runs/
```

`eval/README.md` skeleton (completed in Task 20):

```markdown
# prism-eval — Tier-A accuracy harness

> **Disambiguation (separation contract, spec §2.1):** this directory is prism's
> self-contained evaluation harness. It is **unrelated to `~/code/agent-eval`**
> (the review-agent harness for `~/code/agent-knowledge`). Nothing here imports
> from, writes to, or depends on agent-eval/agent-knowledge; corpus repos were
> one-time *copied*. If shared needs emerge, split to `~/code/prism-eval` —
> do not entangle the two.
```

`eval/tier_a/__init__.py`: empty.

- [ ] **Step 2: Write the failing schema tests**

`eval/tests/test_model.py`:

```python
from tier_a.model import (CallEdge, DefTarget, FunctionDef, Location,
                          from_lsp_line, match_by_selection, tie_break)


def fd(name, file, start, end, sel, kind="function", container=None):
    return FunctionDef(name=name, kind=kind, container=container,
                       location=Location(file, start, end), selection_line=sel)


def test_from_lsp_line_is_zero_to_one_based():
    assert from_lsp_line(0) == 1


def test_tie_break_smallest_span_then_start_then_file():
    a = fd("f", "b.rs", 10, 30, 10)
    b = fd("f", "b.rs", 12, 20, 12)   # smallest span wins
    c = fd("f", "a.rs", 12, 20, 12)   # same span: lower file wins over b
    assert tie_break([a, b]) is b
    assert tie_break([b, c]) is c


def test_match_by_selection_tolerates_doc_comment_offset():
    # LSP DocumentSymbol.range includes the doc comment (starts line 5);
    # tree-sitter's node starts at the fn keyword (line 9). Name-token line (11... )
    # falls inside prism's [9, 20] span -> match (spec §2.4).
    oracle = fd("build", "src/x.rs", 5, 20, 9)
    prism_rec = fd("build", "src/x.rs", 9, 20, 9, kind="function_item")
    assert match_by_selection(oracle, [prism_rec]) is prism_rec


def test_match_by_selection_requires_name_equality():
    oracle = fd("build", "src/x.rs", 5, 20, 9)
    other = fd("rebuild", "src/x.rs", 9, 20, 9)
    assert match_by_selection(oracle, [other]) is None
```

- [ ] **Step 3: Run, verify failure**

Run: `cd eval && uv run pytest -q`
Expected: FAIL — `tier_a.model` missing.

- [ ] **Step 4: Implement `eval/tier_a/model.py`**

```python
"""Schemas + normalization (spec §2.1). Adapters convert AT THEIR BOUNDARY;
comparison code never sees raw LSP or prism JSON. All lines 1-based inclusive,
files repo-relative POSIX."""
from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, order=True)
class Location:
    file: str
    start_line: int
    end_line: int


@dataclass(frozen=True)
class FunctionDef:
    name: str | None          # None = anonymous; excluded from matching (§2.1)
    kind: str                 # oracle-side semantic: function|method|constructor
    container: str | None     # enclosing symbol from hierarchical documentSymbol
    location: Location
    selection_line: int       # name-token line (LSP selectionRange; prism: start_line)


@dataclass(frozen=True)
class DefTarget:
    location: Location
    name: str | None
    kind: str | None


@dataclass(frozen=True)
class CallEdge:
    direction: str            # "caller" | "callee"
    seed: FunctionDef
    other_def: Location | None
    other_name: str | None
    call_site: Location


def from_lsp_line(line0: int) -> int:
    """LSP is 0-based; everything internal is 1-based."""
    return line0 + 1


def tie_break(cands: list[FunctionDef]) -> FunctionDef:
    """Deterministic pick: smallest span, then lowest start_line, then file (§2.1)."""
    return min(cands, key=lambda r: (r.location.end_line - r.location.start_line,
                                     r.location.start_line, r.location.file))


def match_by_selection(oracle_fd: FunctionDef,
                       prism_records: list[FunctionDef]) -> FunctionDef | None:
    """§2.4 matching primitive: name equality + oracle selection_line contained in
    the prism record's [start_line, end_line]. Anonymous never matches."""
    if oracle_fd.name is None:
        return None
    cands = [r for r in prism_records
             if r.name == oracle_fd.name
             and r.location.file == oracle_fd.location.file
             and r.location.start_line <= oracle_fd.selection_line <= r.location.end_line]
    return tie_break(cands) if cands else None
```

- [ ] **Step 5: Add `eval/tier_a/interfaces.py` — the spec-named swap seam (§2.1)**

```python
"""The two hard seams (spec §2.1, PEP 544). Sampling/comparison/adjudication/report
code depends ONLY on these protocols — multilspy, SCIP, a Rust rewrite, or a
non-prism SUT swap in behind them without touching anything downstream."""
from __future__ import annotations

from typing import Protocol, runtime_checkable

from .model import CallEdge, DefTarget, FunctionDef


@runtime_checkable
class Oracle(Protocol):
    def inventory(self, corpus_root: str) -> list[FunctionDef]: ...
    def callers(self, def_: FunctionDef) -> list[CallEdge]: ...
    def callees(self, def_: FunctionDef) -> list[CallEdge]: ...
    def definitions_at(self, file: str, line: int, character: int) -> list[DefTarget]: ...
    def version(self) -> str: ...
    def capability_probe(self) -> bool: ...


@runtime_checkable
class SystemUnderTest(Protocol):
    def inventory(self, corpus_root: str) -> list[FunctionDef]: ...
    def callers(self, corpus_root: str, def_: FunctionDef) -> list[CallEdge]: ...
    def callees(self, corpus_root: str, def_: FunctionDef) -> list[CallEdge]: ...
    def version(self) -> str: ...
```

- [ ] **Step 6: Run tests, verify pass**

Run: `cd eval && uv run pytest -q` — Expected: PASS (4 tests). Commit:

```bash
git add eval
git commit -m "feat(eval): scaffold uv project + §2.1 schemas/seams with tests"
```

### Task 11: stdlib LSP client + echo-server framing tests (spec §2.2)

**Files:**
- Create: `eval/tier_a/lsp_client.py`, `eval/tests/echo_server.py`, `eval/tests/test_lsp_client.py`

- [ ] **Step 1: Write the scripted echo server (test double)**

`eval/tests/echo_server.py`:

```python
"""Minimal JSON-RPC-over-stdio server for framing tests. Not an LSP."""
import json
import sys
import time


def read_msg(stdin):
    headers = {}
    while True:
        line = stdin.readline().decode()
        if line in ("\r\n", "\n", ""):
            break
        k, v = line.split(":", 1)
        headers[k.strip().lower()] = v.strip()
    n = int(headers["content-length"])
    return json.loads(stdin.read(n))


def write_msg(obj):
    body = json.dumps(obj).encode()
    sys.stdout.buffer.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
    sys.stdout.buffer.flush()


while True:
    msg = read_msg(sys.stdin.buffer)
    m, i = msg.get("method"), msg.get("id")
    if m == "initialize":
        write_msg({"jsonrpc": "2.0", "id": i, "result": {"capabilities": {}}})
    elif m == "test/echo":
        write_msg({"jsonrpc": "2.0", "id": i, "result": msg["params"]})
    elif m == "test/slow":
        time.sleep(2)
        write_msg({"jsonrpc": "2.0", "id": i, "result": "late"})
    elif m == "test/notifyme":
        write_msg({"jsonrpc": "2.0", "method": "test/notification", "params": {"k": 1}})
        write_msg({"jsonrpc": "2.0", "id": i, "result": "ok"})
    elif m == "test/error":
        write_msg({"jsonrpc": "2.0", "id": i, "error": {"code": -1, "message": "boom"}})
    elif m == "exit":
        break
```

- [ ] **Step 2: Write the failing client tests**

`eval/tests/test_lsp_client.py`:

```python
import sys
from pathlib import Path

import pytest
from tier_a.lsp_client import LspClient, LspServerError, LspTimeout

ECHO = [sys.executable, str(Path(__file__).parent / "echo_server.py")]


@pytest.fixture
def client():
    c = LspClient(ECHO, cwd=".", default_timeout=5.0)
    c.start()
    yield c
    c.stop()


def test_request_response_roundtrip(client):
    assert client.request("test/echo", {"x": [1, 2]}) == {"x": [1, 2]}


def test_concurrent_correlation(client):
    # interleaved ids must route to the right callers
    assert client.request("test/echo", {"n": 1}) == {"n": 1}
    assert client.request("test/echo", {"n": 2}) == {"n": 2}


def test_timeout_raises_and_client_survives(client):
    with pytest.raises(LspTimeout):
        client.request("test/slow", {}, timeout=0.2)
    assert client.request("test/echo", {"after": True}) == {"after": True}


def test_server_error_raises(client):
    with pytest.raises(LspServerError):
        client.request("test/error", {})


def test_notifications_are_captured(client):
    client.request("test/notifyme", {})
    notes = client.drain_notifications()
    assert any(n["method"] == "test/notification" for n in notes)
```

- [ ] **Step 3: Run, verify failure** — `cd eval && uv run pytest tests/test_lsp_client.py -q` → FAIL (module missing).

- [ ] **Step 4: Implement `eval/tier_a/lsp_client.py`**

```python
"""Stdlib JSON-RPC-over-stdio client (spec §2.2): Content-Length framing,
request/response correlation, per-request timeout, notification capture."""
from __future__ import annotations

import json
import subprocess
import threading


class LspError(Exception):
    pass


class LspTimeout(LspError):
    pass


class LspServerError(LspError):
    def __init__(self, err: dict):
        super().__init__(f"server error {err.get('code')}: {err.get('message')}")
        self.err = err


class LspClient:
    def __init__(self, cmd: list[str], cwd: str, default_timeout: float = 30.0,
                 root_uri: str | None = None):
        # root_uri: live LSP servers (rust-analyzer/gopls/pyright) need workspace
        # context for documentSymbol/callHierarchy; the echo-server tests pass None.
        self._cmd, self._cwd, self._timeout = cmd, cwd, default_timeout
        self._root_uri = root_uri
        self._proc: subprocess.Popen | None = None
        self._next_id = 0
        self._lock = threading.Lock()
        self._pending: dict[int, dict] = {}      # id -> {"event", "result"/"error"}
        self._notifications: list[dict] = []

    def start(self) -> None:
        self._proc = subprocess.Popen(
            self._cmd, cwd=self._cwd, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        threading.Thread(target=self._reader, daemon=True).start()
        params = {"processId": None, "rootUri": self._root_uri,
                  "capabilities": {"window": {"workDoneProgress": True}}}
        if self._root_uri:
            params["workspaceFolders"] = [{"uri": self._root_uri, "name": "corpus"}]
        self.server_info = self.request("initialize", params).get("serverInfo", {})
        self.notify("initialized", {})

    def stop(self) -> None:
        if self._proc and self._proc.poll() is None:
            try:
                self.notify("exit", {})
            except Exception:
                pass
            self._proc.wait(timeout=5)

    def _write(self, obj: dict) -> None:
        body = json.dumps(obj).encode()
        frame = f"Content-Length: {len(body)}\r\n\r\n".encode() + body
        with self._lock:
            self._proc.stdin.write(frame)
            self._proc.stdin.flush()

    def _reader(self) -> None:
        out = self._proc.stdout
        while True:
            headers = {}
            while True:
                line = out.readline()
                if not line:
                    return
                if line in (b"\r\n", b"\n"):
                    break
                k, v = line.decode().split(":", 1)
                headers[k.strip().lower()] = v.strip()
            msg = json.loads(out.read(int(headers["content-length"])))
            if "id" in msg and ("result" in msg or "error" in msg):
                slot = self._pending.get(msg["id"])
                if slot is not None:
                    slot["msg"] = msg
                    slot["event"].set()
            else:
                self._notifications.append(msg)

    def request(self, method: str, params: dict, timeout: float | None = None):
        with self._lock:
            self._next_id += 1
            rid = self._next_id
        slot = {"event": threading.Event(), "msg": None}
        self._pending[rid] = slot
        self._write({"jsonrpc": "2.0", "id": rid, "method": method, "params": params})
        if not slot["event"].wait(timeout or self._timeout):
            self._pending.pop(rid, None)
            raise LspTimeout(f"{method} timed out")
        msg = self._pending.pop(rid)["msg"]
        if "error" in msg:
            raise LspServerError(msg["error"])
        return msg["result"]

    def notify(self, method: str, params: dict) -> None:
        self._write({"jsonrpc": "2.0", "method": method, "params": params})

    def drain_notifications(self) -> list[dict]:
        out, self._notifications = self._notifications, []
        return out
```

- [ ] **Step 5: Run tests, verify pass, commit**

```bash
cd eval && uv run pytest -q
git add eval && git commit -m "feat(eval): stdlib LSP JSON-RPC client + echo-server framing tests"
```

### Task 12: Oracle adapters — response mapping + lifecycle (spec §2.2)

**Files:**
- Create: `eval/tier_a/oracles.py`, `eval/tests/test_oracles.py`

The adapters have two halves: **pure mapping functions** (LSP JSON → §2.1 dataclasses;
fully unit-tested against canned JSON, no servers) and **lifecycle** (spawn, readiness,
capability probe; exercised only in Task 21's live run). Server commands:
rust-analyzer → `["rust-analyzer"]`, gopls → `["gopls", "serve"]`,
pyright → `["pyright-langserver", "--stdio"]`.

- [ ] **Step 1: Write the failing mapping tests**

`eval/tests/test_oracles.py`:

```python
from tier_a.model import FunctionDef, Location
from tier_a.oracles import (map_document_symbols, map_incoming, map_outgoing,
                            enrich_definitions, uri_to_rel)


def test_uri_to_rel_posix():
    assert uri_to_rel("file:///repo/src/a%20b.rs", "/repo") == "src/a b.rs"
    assert uri_to_rel("file:///elsewhere/x.rs", "/repo") is None  # out_of_corpus


def test_map_document_symbols_hierarchical_kinds_container_selection():
    syms = [{
        "name": "Engine", "kind": 5,  # Class — not itself emitted
        "range": {"start": {"line": 9, "character": 0}, "end": {"line": 29, "character": 1}},
        "selectionRange": {"start": {"line": 9, "character": 6}, "end": {"line": 9, "character": 12}},
        "children": [{
            "name": "run", "kind": 6,  # Method
            "range": {"start": {"line": 11, "character": 4}, "end": {"line": 19, "character": 5}},
            "selectionRange": {"start": {"line": 11, "character": 8}, "end": {"line": 11, "character": 11}},
        }],
    }, {
        "name": "helper", "kind": 12,  # Function
        "range": {"start": {"line": 31, "character": 0}, "end": {"line": 33, "character": 1}},
        "selectionRange": {"start": {"line": 31, "character": 3}, "end": {"line": 31, "character": 9}},
    }]
    fds = map_document_symbols("src/x.py", syms)
    assert [f.name for f in fds] == ["run", "helper"]
    run = fds[0]
    assert (run.kind, run.container) == ("method", "Engine")
    assert run.location == Location("src/x.py", 12, 20)   # 0-based -> 1-based
    assert run.selection_line == 12
    assert fds[1].container is None


def test_map_incoming_one_edge_per_from_range():
    seed = FunctionDef("f", "function", None, Location("src/lib.rs", 5, 9), 5)
    items = [{
        "from": {"name": "caller_a", "uri": "file:///repo/src/m.rs",
                 "range": {"start": {"line": 99, "character": 0}, "end": {"line": 120, "character": 1}},
                 "selectionRange": {"start": {"line": 99, "character": 3}, "end": {"line": 99, "character": 11}}},
        "fromRanges": [
            {"start": {"line": 101, "character": 8}, "end": {"line": 101, "character": 9}},
            {"start": {"line": 110, "character": 8}, "end": {"line": 110, "character": 9}},
        ],
    }]
    edges = map_incoming(seed, items, root="/repo")
    assert len(edges) == 2
    assert edges[0].direction == "caller"
    assert edges[0].other_name == "caller_a"
    assert edges[0].other_def == Location("src/m.rs", 100, 121)
    assert {e.call_site.start_line for e in edges} == {102, 111}
    assert all(e.call_site.file == "src/m.rs" for e in edges)


def test_map_outgoing_call_sites_are_in_seed_file():
    seed = FunctionDef("f", "function", None, Location("src/lib.rs", 5, 30), 5)
    items = [{
        "to": {"name": "callee_x", "uri": "file:///repo/src/n.rs",
               "range": {"start": {"line": 3, "character": 0}, "end": {"line": 7, "character": 1}},
               "selectionRange": {"start": {"line": 3, "character": 3}, "end": {"line": 3, "character": 11}}},
        "fromRanges": [{"start": {"line": 12, "character": 4}, "end": {"line": 12, "character": 12}}],
    }]
    edges = map_outgoing(seed, items, root="/repo")
    assert len(edges) == 1
    assert edges[0].direction == "callee"
    assert edges[0].call_site == Location("src/lib.rs", 13, 13)
    assert edges[0].other_def == Location("src/n.rs", 4, 8)


def test_enrich_definitions_by_containment_smallest_span():
    inv = [FunctionDef("outer", "function", None, Location("src/p.rs", 1, 50), 1),
           FunctionDef("target", "method", "Edge", Location("src/p.rs", 10, 14), 10)]
    raw = [{"uri": "file:///repo/src/p.rs",
            "range": {"start": {"line": 11, "character": 4}, "end": {"line": 11, "character": 10}}}]
    [dt] = enrich_definitions(raw, inv, root="/repo")
    assert (dt.name, dt.kind) == ("target", "method")
```

- [ ] **Step 2: Run, verify failure** — `cd eval && uv run pytest tests/test_oracles.py -q` → FAIL.

- [ ] **Step 3: Implement the pure half of `eval/tier_a/oracles.py`**

```python
"""Oracle adapters (spec §2.2). Pure LSP-JSON->schema mapping (unit-tested) +
server lifecycle (live only). SymbolKind: Method=6, Constructor=9, Function=12."""
from __future__ import annotations

import os
import urllib.parse
from pathlib import PurePosixPath

from .model import CallEdge, DefTarget, FunctionDef, Location, from_lsp_line

SYMBOL_KIND = {6: "method", 9: "constructor", 12: "function"}


def uri_to_rel(uri: str, root: str) -> str | None:
    """file:// URI -> repo-relative POSIX path; None if outside root (§2.1)."""
    path = urllib.parse.unquote(urllib.parse.urlparse(uri).path)
    try:
        rel = os.path.relpath(path, root)
    except ValueError:
        return None
    if rel.startswith(".."):
        return None
    return str(PurePosixPath(rel))


def _loc(file: str, rng: dict) -> Location:
    return Location(file, from_lsp_line(rng["start"]["line"]),
                    from_lsp_line(rng["end"]["line"]))


def map_document_symbols(file: str, syms: list[dict]) -> list[FunctionDef]:
    out: list[FunctionDef] = []

    def walk(nodes: list[dict], container: str | None) -> None:
        for n in nodes:
            kind = SYMBOL_KIND.get(n.get("kind"))
            if kind and "selectionRange" in n:
                out.append(FunctionDef(
                    name=n.get("name"), kind=kind, container=container,
                    location=_loc(file, n["range"]),
                    selection_line=from_lsp_line(n["selectionRange"]["start"]["line"])))
            walk(n.get("children", []), n.get("name"))

    walk(syms, None)
    # Flat SymbolInformation fallback (no children/selectionRange): location.range +
    # containerName; selection_line degrades to the range start line.
    for n in syms:
        if "location" in n and SYMBOL_KIND.get(n.get("kind")):
            file2 = file  # caller passes per-file requests; uri already resolved
            loc = _loc(file2, n["location"]["range"])
            out.append(FunctionDef(name=n.get("name"), kind=SYMBOL_KIND[n["kind"]],
                                   container=n.get("containerName"),
                                   location=loc, selection_line=loc.start_line))
    return out


def map_incoming(seed: FunctionDef, items: list[dict], root: str) -> list[CallEdge]:
    edges = []
    for it in items:
        frm = it["from"]
        f = uri_to_rel(frm["uri"], root)
        if f is None:
            continue  # out_of_corpus; counted by the caller of this fn
        other = _loc(f, frm["range"])
        for r in it.get("fromRanges", []):
            edges.append(CallEdge("caller", seed, other, frm.get("name"), _loc(f, r)))
    return edges


def map_outgoing(seed: FunctionDef, items: list[dict], root: str) -> list[CallEdge]:
    edges = []
    for it in items:
        to = it["to"]
        f = uri_to_rel(to["uri"], root)
        if f is None:
            continue
        other = _loc(f, to["range"])
        for r in it.get("fromRanges", []):
            # outgoing fromRanges are positions in the SEED's body (LSP spec)
            edges.append(CallEdge("callee", seed, other, to.get("name"),
                                  _loc(seed.location.file, r)))
    return edges


def enrich_definitions(raw: list[dict], inventory: list[FunctionDef],
                       root: str) -> list[DefTarget]:
    """Location/LocationLink -> DefTarget, name via smallest containing inventory span."""
    out = []
    for d in raw:
        uri = d.get("uri") or d.get("targetUri")
        rng = d.get("range") or d.get("targetSelectionRange") or d.get("targetRange")
        f = uri_to_rel(uri, root)
        if f is None:
            continue
        loc = _loc(f, rng)
        within = [fd for fd in inventory
                  if fd.location.file == f
                  and fd.location.start_line <= loc.start_line <= fd.location.end_line]
        best = min(within, key=lambda fd: fd.location.end_line - fd.location.start_line,
                   default=None)
        out.append(DefTarget(loc, best.name if best else None,
                             best.kind if best else None))
    return out
```

- [ ] **Step 4: Implement the lifecycle half (same file; live-only, no unit tests)**

```python
PROBE_SOURCES = {
    "rust": ("probe_prism_eval.rs", "fn probe_callee() {}\nfn probe_caller() { probe_callee(); }\n"),
    "go": ("probe_prism_eval.go", "package main\n\nfunc probeCallee() {}\nfunc probeCaller() { probeCallee() }\n"),
    "python": ("probe_prism_eval.py", "def probe_callee():\n    pass\n\ndef probe_caller():\n    probe_callee()\n"),
}


class OracleError(Exception):
    """Any per-probe oracle failure (timeout, server error, bad response shape).
    The accounting layer converts these to oracle_error counts (spec §2.2) —
    they are NEVER converted into prism failures."""


class LspOracle:
    """Shared lifecycle: spawn, quiescence ($/progress drain), capability probe,
    per-method wrappers that raise OracleError on failure."""

    def __init__(self, cmd, root, lang, settle_s=2.0, quiescence_cap_s=300.0):
        import os as _os
        import urllib.parse as _up
        from .lsp_client import LspClient
        self.root, self.lang = root, lang
        self.not_quiescent = False
        self._cmd = cmd
        self.client = LspClient(cmd, cwd=root,
                                root_uri="file://" + _up.quote(_os.path.abspath(root)))
        self._settle, self._cap = settle_s, quiescence_cap_s

    def start(self):
        import time
        self.client.start()
        deadline = time.monotonic() + self._cap
        active: set = set()
        quiet_since = time.monotonic()
        while time.monotonic() < deadline:
            for n in self.client.drain_notifications():
                if n.get("method") == "$/progress":
                    tok = n["params"]["token"]
                    kind = n["params"]["value"].get("kind")
                    if kind == "begin":
                        active.add(tok)
                    elif kind == "end":
                        active.discard(tok)
                    quiet_since = time.monotonic()
            if not active and time.monotonic() - quiet_since >= self._settle:
                return
            time.sleep(0.1)
        self.not_quiescent = True  # proceed; report records oracle_not_quiescent

    def did_open(self, rel_path, text=None):
        p = os.path.join(self.root, rel_path)
        content = text if text is not None else open(p, encoding="utf-8", errors="replace").read()
        lang_id = {"rust": "rust", "go": "go", "python": "python"}[self.lang]
        self.client.notify("textDocument/didOpen", {"textDocument": {
            "uri": "file://" + urllib.parse.quote(p), "languageId": lang_id,
            "version": 1, "text": content}})

    def capability_probe(self) -> bool:
        """Overlay a 3-line probe file; prepareCallHierarchy + incomingCalls must
        work (spec §2.2). One retry against a real symbol is the caller's job."""
        name, text = PROBE_SOURCES[self.lang]
        self.did_open(name, text)
        uri = "file://" + urllib.parse.quote(os.path.join(self.root, name))
        # probe_callee's name token: line 0 (rust/go) / 0 (py), col aligned per source
        col = {"rust": 3, "go": 5, "python": 4}[self.lang]
        line = {"rust": 0, "go": 2, "python": 0}[self.lang]
        try:
            items = self.client.request("textDocument/prepareCallHierarchy",
                {"textDocument": {"uri": uri}, "position": {"line": line, "character": col}})
            if not items:
                return False
            calls = self.client.request("callHierarchy/incomingCalls", {"item": items[0]})
            return calls is not None and len(calls) >= 1
        except Exception:
            return False
```

Plus the callable per-method wrappers Task 20's runner consumes — every raw
`client.request` is funneled through `_req` so failures become `OracleError`, and
`version()` produces the report's oracle string:

```python
    def _req(self, method, params, timeout=None):
        from .lsp_client import LspError
        try:
            r = self.client.request(method, params, timeout=timeout)
        except LspError as e:
            raise OracleError(f"{method}: {e}") from e
        if r is None:
            raise OracleError(f"{method}: null result")
        return r

    def _uri(self, rel_path):
        return "file://" + urllib.parse.quote(os.path.abspath(
            os.path.join(self.root, rel_path)))

    def document_symbols(self, rel_path):
        from .oracles import map_document_symbols  # self-import safe at module level
        self.did_open(rel_path)
        syms = self._req("textDocument/documentSymbol",
                         {"textDocument": {"uri": self._uri(rel_path)}})
        return map_document_symbols(rel_path, syms)

    def _hierarchy_item(self, fd):
        items = self._req("textDocument/prepareCallHierarchy",
                          {"textDocument": {"uri": self._uri(fd.location.file)},
                           "position": {"line": fd.selection_line - 1, "character": 0}})
        if not items:
            raise OracleError(f"prepareCallHierarchy: no item for {fd.name}")
        return items[0]

    def callers(self, fd):
        items = self._req("callHierarchy/incomingCalls",
                          {"item": self._hierarchy_item(fd)})
        return map_incoming(fd, items, self.root)

    def callees(self, fd):
        items = self._req("callHierarchy/outgoingCalls",
                          {"item": self._hierarchy_item(fd)})
        return map_outgoing(fd, items, self.root)

    def definitions_at(self, inventory, rel_path, line, character):
        raw = self._req("textDocument/definition",
                        {"textDocument": {"uri": self._uri(rel_path)},
                         "position": {"line": line - 1, "character": character}})
        raw = raw if isinstance(raw, list) else [raw]
        return enrich_definitions(raw, inventory, self.root)

    def version(self):
        import subprocess as _sp
        info = getattr(self.client, "server_info", {}) or {}
        if info.get("name"):
            return f"{info['name']} {info.get('version', '?')}"
        try:
            out = _sp.run([self._cmd[0], "--version"], capture_output=True,
                          text=True, timeout=10).stdout.strip()
            return out.splitlines()[0] if out else self._cmd[0]
        except Exception:
            return self._cmd[0]
```

Note `prepareCallHierarchy` positions at `character: 0` of the selection line; if a
live server demands the exact name-token column, extend `FunctionDef` consumers to pass
the selectionRange character (the inventory snapshot already stores what the oracle
returned — record the adjustment in the run report). Wrapper-level tests: add a
`FakeRawClient` (dict-programmed `request`/`notify`/`drain_notifications`) to
`eval/tests/test_oracles.py` and assert (a) `callers()` raises `OracleError` when the
fake returns an LSP error, and (b) `_hierarchy_item` raises `OracleError` on an empty
prepare result — those two paths feed the §2.2 accounting and must not be guessed.

(rust-analyzer/gopls/pyright concrete classes are thin: command + any
server-specific `initializationOptions`; start with none and add only if the live
run in Task 21 requires them — record additions in the run report.)


- [ ] **Step 5: Run tests, verify pass, commit**

```bash
cd eval && uv run pytest -q
git add eval && git commit -m "feat(eval): oracle adapters — tested LSP mapping + lifecycle/probe (§2.2)"
```

- [ ] **Step 6 [ORCHESTRATOR]: early live pyright probe (de-risk the §2.2
precondition NOW, not at baseline time).** Run on the host as soon as this task lands:

```bash
npm i -g pyright
cd eval && uv run python -c "
from tier_a.oracles import LspOracle
o = LspOracle(['pyright-langserver', '--stdio'], 'fixtures/python/module_fn', 'python')
o.client.start(); print('capability_probe:', o.capability_probe())"
```

(If Task 18's fixtures aren't merged yet, point at any directory containing one small
`.py` file.) `True` → pyright call hierarchy works, the fallback is struck. `False` →
try `basedpyright`; if that also fails, **add a references-fallback task**
(oracle `callers()` via `textDocument/references` + caller-function containment per
spec §2.2) before Task 21 — do not let the baseline stall into unplanned work.

### Task 13: `PrismCli` SUT adapter (spec §2.3/§2.5 extraction table)

**Files:**
- Create: `eval/tier_a/sut.py`, `eval/tests/test_sut.py`,
  `eval/tests/fixtures/wire_functions_sample.json`,
  `eval/tests/fixtures/wire_callers_sample.json`, `eval/tests/fixtures/wire_callees_sample.json`

- [ ] **Step 1: Freeze real wire samples (ground truth for the extraction tests)**

```bash
cargo build --release
./target/release/prism nav functions --repo . --format json | head -40 > eval/tests/fixtures/wire_functions_sample.json
# make it valid JSON: regenerate without head if truncated — small repo dirs work:
./target/release/prism nav functions --repo tests/fixtures --format json > eval/tests/fixtures/wire_functions_sample.json
./target/release/prism nav callers --repo . --symbol load_repo --file src/repo_loader.rs --depth 1 --format json > eval/tests/fixtures/wire_callers_sample.json
./target/release/prism nav callees --repo . --symbol load_repo --file src/repo_loader.rs --depth 1 --format json > eval/tests/fixtures/wire_callees_sample.json
```

Inspect the callers sample once by eye and note the exact `why` entry shape for
`CalledBy`/`Calls` (serde enum representation). The extraction code below assumes
externally-tagged (`{"CalledBy": {"caller": ..., "call_site_line": ...}}`); **if the
frozen sample differs, adapt `_why()` to the observed shape and say so in the commit
message** — the frozen sample, not this plan, is the contract.

- [ ] **Step 2: Write the failing tests**

`eval/tests/test_sut.py`:

```python
import json
from pathlib import Path

from tier_a.model import FunctionDef, Location
from tier_a.sut import (extract_callers, extract_callees, extract_functions,
                        parse_version)

FIX = Path(__file__).parent / "fixtures"
SEED = FunctionDef("load_repo", "function", None,
                   Location("src/repo_loader.rs", 1, 1), 1)  # spans refined at runtime


def test_parse_version_sha_and_dirty():
    assert parse_version("slicing 3.1.2 (abcdef123456)\n") == ("abcdef123456", False)
    assert parse_version("slicing 3.1.2 (abcdef123456-dirty)\n") == ("abcdef123456", True)


def test_extract_functions_inventory_records():
    recs = extract_functions(json.loads((FIX / "wire_functions_sample.json").read_text()))
    assert recs, "sample must be non-empty"
    r = recs[0]
    assert r.location.start_line >= 1 and r.location.end_line >= r.location.start_line
    assert r.selection_line == r.location.start_line  # prism: selection = start (§2.1)


def test_extract_callers_per_site_edges():
    ev = json.loads((FIX / "wire_callers_sample.json").read_text())
    edges = extract_callers(SEED, ev)
    assert edges, "load_repo has known callers"
    e = edges[0]
    assert e.direction == "caller"
    # §2.5 table: other_def = items[].location (caller's span); call_site line from why
    assert e.other_def is not None
    assert e.other_def.start_line <= e.call_site.start_line <= e.other_def.end_line
    assert e.call_site.file == e.other_def.file


def test_extract_callees_unresolved_have_none_def():
    ev = json.loads((FIX / "wire_callees_sample.json").read_text())
    edges = extract_callees(SEED, ev)
    assert edges
    for e in edges:
        assert e.direction == "callee"
        assert e.call_site.file == SEED.location.file  # §2.5: callee sites in seed file
        if e.other_def is None:
            assert e.other_name  # unresolved still names the callee from why.Calls
```

- [ ] **Step 3: Run, verify failure**, then **Step 4: implement `eval/tier_a/sut.py`**

```python
"""PrismCli SystemUnderTest (spec §2.3 discovery, §2.5 extraction table)."""
from __future__ import annotations

import json
import os
import re
import subprocess

from .model import CallEdge, FunctionDef, Location


class SutError(Exception):
    pass


class SutStale(SutError):
    pass


def parse_version(out: str) -> tuple[str, bool]:
    m = re.search(r"\(([0-9a-f]{12})(-dirty)?\)", out)
    if not m:
        raise SutStale(f"no GIT_SHA in --version output: {out!r} (rebuild prism)")
    return m.group(1), bool(m.group(2))


def extract_functions(arr: list[dict]) -> list[FunctionDef]:
    return [FunctionDef(name=r["name"], kind=r["kind"], container=None,
                        location=Location(r["file"], r["start_line"], r["end_line"]),
                        selection_line=r["start_line"]) for r in arr]


def _why(item: dict, tag: str) -> dict | None:
    for w in item.get("why", []):
        if isinstance(w, dict) and tag in w:
            return w[tag]
    return None


def extract_callers(seed: FunctionDef, ev: dict) -> list[CallEdge]:
    edges = []
    for it in ev.get("items", []):
        cb = _why(it, "CalledBy")
        if cb is None:
            continue
        loc = it["location"]
        other = Location(loc["file"], loc["start_line"], loc["end_line"])
        site = Location(loc["file"], cb["call_site_line"], cb["call_site_line"])
        name = (it.get("symbol") or {}).get("name")
        edges.append(CallEdge("caller", seed, other, name, site))
    return edges


def extract_callees(seed: FunctionDef, ev: dict) -> list[CallEdge]:
    edges = []
    for it in ev.get("items", []):
        c = _why(it, "Calls")
        if c is None:
            continue
        site = Location(seed.location.file, c["call_site_line"], c["call_site_line"])
        if it.get("symbol"):
            loc = it["location"]
            other = Location(loc["file"], loc["start_line"], loc["end_line"])
        else:
            other = None  # §2.5: unresolved callee — counted separately, never a TP/FP
        edges.append(CallEdge("callee", seed, other, c.get("callee"), site))
    return edges


class PrismCli:
    def __init__(self, prism_repo: str, sut_bin: str | None = None,
                 allow_stale: bool = False):
        self.repo = prism_repo
        self.bin = (sut_bin or os.environ.get("PRISM_BIN")
                    or os.path.join(prism_repo, "target/release/prism"))
        self.allow_stale = allow_stale
        self.sha, self.dirty = self._check_freshness()

    def _check_freshness(self) -> tuple[str, bool]:
        out = subprocess.run([self.bin, "--version"], capture_output=True,
                             text=True, check=True).stdout
        sha, dirty = parse_version(out)
        head = subprocess.run(["git", "-C", self.repo, "rev-parse", "--short=12", "HEAD"],
                              capture_output=True, text=True, check=True).stdout.strip()
        if (sha != head or dirty) and not self.allow_stale:
            raise SutStale(f"binary {sha}{'-dirty' if dirty else ''} != HEAD {head}; "
                           "rebuild (cargo build --release) or pass --allow-stale-sut")
        return sha, dirty

    def _run(self, args: list[str]) -> dict | list:
        p = subprocess.run([self.bin, "nav", *args, "--format", "json"],
                           capture_output=True, text=True)
        if p.returncode != 0:
            raise SutError(f"prism nav {args[0]} failed ({p.returncode}): "
                           f"{p.stdout or p.stderr}")
        return json.loads(p.stdout)

    def inventory(self, corpus_root: str) -> list[FunctionDef]:
        return extract_functions(self._run(["functions", "--repo", corpus_root]))

    def callers(self, corpus_root: str, seed: FunctionDef) -> list[CallEdge]:
        loc = f"{seed.location.file}:{seed.location.start_line}"
        return extract_callers(seed, self._run(
            ["callers", "--repo", corpus_root, "--location", loc, "--depth", "1"]))

    def callees(self, corpus_root: str, seed: FunctionDef) -> list[CallEdge]:
        loc = f"{seed.location.file}:{seed.location.start_line}"
        return extract_callees(seed, self._run(
            ["callees", "--repo", corpus_root, "--location", loc, "--depth", "1"]))

    def callers_by_symbol(self, corpus_root: str, symbol: str,
                          file: str | None = None) -> list[CallEdge]:
        """Bare/with-file symbol seeding — the pathway for pinned probe #4
        (`ambiguous_symbol_error`): a bare common name MUST raise SutAmbiguous."""
        args = ["callers", "--repo", corpus_root, "--symbol", symbol, "--depth", "1"]
        if file:
            args += ["--file", file]
        seed = FunctionDef(symbol, "function", None, Location(file or "?", 1, 1), 1)
        return extract_callers(seed, self._run(args))

    def version(self) -> str:
        return f"prism {self.sha}{'-dirty' if self.dirty else ''}"
```

and teach `_run` to classify the safe-fail contract instead of collapsing every
nonzero exit into a crash:

```python
class SutAmbiguous(SutError):
    """prism's AmbiguousSymbol safe-fail — a CONTRACT, not a crash (§2.5 probe #4)."""


# inside _run, replace the bare `raise SutError(...)` with:
        if p.returncode != 0:
            blob = p.stdout or p.stderr
            if "AmbiguousSymbol" in blob:
                raise SutAmbiguous(blob)
            raise SutError(f"prism nav {args[0]} failed ({p.returncode}): {blob}")
```

with one more test in `eval/tests/test_sut.py` (the classifier is pure — feed it via a
monkeypatched `subprocess.run` returning returncode 3 and an `AmbiguousSymbol` JSON
error body; assert `SutAmbiguous` is raised, not `SutError`).

- [ ] **Step 5: Run tests, verify pass, commit**

```bash
cd eval && uv run pytest -q
git add eval && git commit -m "feat(eval): PrismCli SUT — discovery, freshness, §2.5 wire extraction"
```

### Task 14: Universe, inventory diff, strata, snapshot, sampling (spec §2.4/§2.5)

**Files:**
- Create: `eval/tier_a/corpus.py` (universe + snapshot), `eval/tier_a/strata.py`,
  `eval/tests/test_strata.py`, `eval/tests/test_corpus.py`

- [ ] **Step 1: Failing tests**

`eval/tests/test_strata.py`:

```python
from tier_a.model import FunctionDef, Location
from tier_a.strata import classify, inventory_diff, sample_strata


def fd(name, file, kind="function", container=None, start=1, end=5, sel=None):
    return FunctionDef(name, kind, container, Location(file, start, end),
                       sel if sel is not None else start)


def counts(fds):
    c = {}
    for f in fds:
        c[f.name] = c.get(f.name, 0) + 1
    return c


def test_precedence_collision_beats_scoped_and_unique():
    fds = [fd("run", "pkg/a/x.go", kind="method"),
           fd("run", "pkg/b/y.go"),
           fd("solo", "pkg/c/z.go"),
           fd("top", "main.go")]
    n = counts(fds)
    assert classify(fds[0], n, "go") == "C-method"
    assert classify(fds[1], n, "go") == "C-name"
    assert classify(fds[2], n, "go") == "Q-scoped"   # unique free fn, subdir package
    assert classify(fds[3], n, "go") == "U-free"


def test_python_q_scoped_requires_package_dir():
    # is_nested for python: any ancestor dir with __init__.py in the universe
    fds = [fd("f", "pkg/mod.py"), fd("g", "script.py")]
    n = counts(fds)
    assert classify(fds[0], n, "python", package_dirs={"pkg"}) == "Q-scoped"
    assert classify(fds[1], n, "python", package_dirs={"pkg"}) == "U-free"


def test_rust_unique_method_is_u_method_not_q():
    f = fd("only", "src/deep/mod.rs", kind="method")
    assert classify(f, {"only": 1}, "rust") == "U-method"


def test_inventory_diff_uses_selection_containment():
    oracle = [fd("a", "src/x.rs", start=5, end=20, sel=9)]   # doc-comment offset
    prism = [fd("a", "src/x.rs", start=9, end=20, sel=9)]
    d = inventory_diff(oracle, prism)
    assert d.matched and not d.prism_missing and not d.prism_extra


def test_sampling_is_deterministic_and_respects_shortfall():
    # all at src/lib.rs: crate root, so unique free fns land in U-free (not Q-scoped)
    fds = [fd(f"u{i}", "src/lib.rs", start=1 + 2 * i, end=2 + 2 * i) for i in range(20)]
    fds += [fd("dup", "src/a.rs"), fd("dup", "src/b.rs", start=9, end=12)]
    n = counts(fds)
    s1 = sample_strata(fds, n, "rust", seed=42, per_stratum=8)
    s2 = sample_strata(fds, n, "rust", seed=42, per_stratum=8)
    assert s1 == s2
    assert len(s1["C-name"]) == 2   # shortfall: takes all eligible
    assert len(s1["U-free"]) == 8


def test_filter_to_universe_drops_out_of_universe_prism_records():
    # §2.4: the SAME include/exclude filter applies to prism's inventory — without
    # this, prism's whole-repo walk floods prism_extra with tests/fixtures/ and
    # eval/fixtures/ records on the prism corpus (review M5).
    from tier_a.strata import filter_to_universe
    recs = [fd("a", "src/lib.rs"), fd("b", "tests/fixtures/x.py"),
            fd("c", "eval/fixtures/rust/free_fn_same_file/main.rs")]
    kept = filter_to_universe(recs, universe_files={"src/lib.rs"})
    assert [r.name for r in kept] == ["a"]
```

- [ ] **Step 2: Run, verify failure**, then **Step 3: implement `eval/tier_a/strata.py`**

```python
"""Strata (spec §2.5 precedence), M1 inventory diff (§2.4), seeded sampling."""
from __future__ import annotations

import random
from dataclasses import dataclass, field

from .model import FunctionDef, match_by_selection

STRATA = ("C-method", "C-name", "Q-scoped", "U-method", "U-free")


def is_nested(fd: FunctionDef, lang: str, package_dirs: set[str] | None = None) -> bool:
    f = fd.location.file
    if lang == "rust":
        # spec §2.5 + review m10: path-based; crate roots are the only non-nested files
        return f not in ("src/lib.rs", "src/main.rs")
    if lang == "go":
        return "/" in f
    if lang == "python":
        parts = f.split("/")[:-1]
        prefixes = {"/".join(parts[: i + 1]) for i in range(len(parts))}
        return bool(prefixes & (package_dirs or set()))
    raise ValueError(lang)


def classify(fd: FunctionDef, defs_per_name: dict, lang: str,
             package_dirs: set[str] | None = None) -> str:
    is_m = fd.kind in ("method", "constructor")
    if fd.name and defs_per_name.get(fd.name, 0) >= 2:
        return "C-method" if is_m else "C-name"
    if not is_m and is_nested(fd, lang, package_dirs):
        return "Q-scoped"
    return "U-method" if is_m else "U-free"


@dataclass
class InventoryDiff:
    matched: list = field(default_factory=list)        # (oracle_fd, prism_fd)
    prism_missing: list = field(default_factory=list)  # oracle-only
    prism_extra: list = field(default_factory=list)    # prism-only
    anon_oracle: int = 0
    anon_prism: int = 0


def inventory_diff(oracle: list[FunctionDef], prism: list[FunctionDef]) -> InventoryDiff:
    d = InventoryDiff()
    d.anon_oracle = sum(1 for f in oracle if f.name is None)
    d.anon_prism = sum(1 for f in prism if f.name is None)
    named_prism = [f for f in prism if f.name is not None]
    used: set[FunctionDef] = set()
    for ofd in oracle:
        if ofd.name is None:
            continue
        m = match_by_selection(ofd, [p for p in named_prism if p not in used])
        if m is None:
            d.prism_missing.append(ofd)
        else:
            used.add(m)
            d.matched.append((ofd, m))
    d.prism_extra = [p for p in named_prism if p not in used]
    return d


def filter_to_universe(records: list[FunctionDef],
                       universe_files: set[str]) -> list[FunctionDef]:
    """§2.4: apply the corpus universe filter to BOTH inventories. The runner MUST
    pass prism's `nav functions` output through this before inventory_diff."""
    return [r for r in records if r.location.file in universe_files]


def sample_strata(oracle: list[FunctionDef], defs_per_name: dict, lang: str,
                  seed: int, per_stratum: int = 8,
                  package_dirs: set[str] | None = None) -> dict[str, list[FunctionDef]]:
    byst: dict[str, list[FunctionDef]] = {s: [] for s in STRATA}
    for f in sorted((f for f in oracle if f.name),
                    key=lambda f: (f.location.file, f.location.start_line, f.name)):
        byst[classify(f, defs_per_name, lang, package_dirs)].append(f)
    rng = random.Random(seed)
    return {s: (v if len(v) <= per_stratum else rng.sample(v, per_stratum))
            for s, v in byst.items()}
```

`eval/tier_a/corpus.py` (universe walk + snapshot persistence; tests in
`eval/tests/test_corpus.py` cover include/exclude filtering and snapshot round-trip):

```python
"""Corpus file universe (§2.4) + oracle-inventory snapshots (§2.5/G3)."""
from __future__ import annotations

import dataclasses
import fnmatch
import hashlib
import json
import os
import subprocess
from pathlib import Path

from .model import FunctionDef, Location

EXTENSIONS = {"rust": [".rs"], "go": [".go"], "python": [".py"]}


def universe(root: str, lang: str, excludes: list[str]) -> list[str]:
    out = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d != ".git"]
        for fn in filenames:
            rel = os.path.relpath(os.path.join(dirpath, fn), root).replace(os.sep, "/")
            if not any(fn.endswith(e) for e in EXTENSIONS[lang]):
                continue
            if any(fnmatch.fnmatch(rel, g) for g in excludes):
                continue
            out.append(rel)
    return sorted(out)


def corpus_sha(root: str) -> str:
    return subprocess.run(["git", "-C", root, "rev-parse", "--short=12", "HEAD"],
                          capture_output=True, text=True, check=True).stdout.strip()


def corpus_dirty(root: str) -> bool:
    p = subprocess.run(["git", "-C", root, "status", "--porcelain"],
                       capture_output=True, text=True, check=True)
    return bool(p.stdout.strip())


def snapshot_path(snap_dir: str, corpus: str, sha: str) -> Path:
    return Path(snap_dir) / f"{corpus}-{sha}.json"


def save_snapshot(path: Path, inventory: list[FunctionDef]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps([dataclasses.asdict(f) for f in inventory],
                               indent=1, sort_keys=True))


def load_snapshot(path: Path) -> list[FunctionDef]:
    return [FunctionDef(name=r["name"], kind=r["kind"], container=r["container"],
                        location=Location(**r["location"]),
                        selection_line=r["selection_line"])
            for r in json.loads(path.read_text())]
```

`eval/tests/test_corpus.py`:

```python
from pathlib import Path

from tier_a.corpus import load_snapshot, save_snapshot, snapshot_path, universe
from tier_a.model import FunctionDef, Location


def test_universe_filters_extensions_and_excludes(tmp_path):
    (tmp_path / "src").mkdir()
    (tmp_path / "src/a.rs").write_text("")
    (tmp_path / "src/b.py").write_text("")
    (tmp_path / "vendor").mkdir()
    (tmp_path / "vendor/c.rs").write_text("")
    files = universe(str(tmp_path), "rust", excludes=["vendor/*"])
    assert files == ["src/a.rs"]


def test_snapshot_roundtrip(tmp_path):
    inv = [FunctionDef("f", "function", None, Location("src/a.rs", 1, 3), 1)]
    p = snapshot_path(str(tmp_path), "prism", "abc123def456")
    save_snapshot(p, inv)
    assert load_snapshot(p) == inv
```

- [ ] **Step 4: Run tests, verify pass, commit**

```bash
cd eval && uv run pytest -q
git add eval && git commit -m "feat(eval): universe/strata/snapshot/sampling — M1 + §2.5 strata (tested)"
```

### Task 15: M2 comparison + metrics (spec §2.5/§2.10)

**Files:**
- Create: `eval/tier_a/compare.py`, `eval/tier_a/metrics.py`, `eval/tier_a/pinned.py`,
  `eval/tests/test_compare.py`

- [ ] **Step 1: Failing tests**

`eval/tests/test_compare.py`:

```python
from tier_a.compare import caller_fn_sets, collapse_sites, site_compare
from tier_a.metrics import wilson
from tier_a.model import CallEdge, FunctionDef, Location


SEED = FunctionDef("f", "function", None, Location("src/s.rs", 1, 5), 1)


def edge(file, line, direction="caller", name="c", dstart=None, dend=None):
    d = Location(file, dstart or max(1, line - 2), dend or line + 2)
    return CallEdge(direction, SEED, d, name, Location(file, line, line))


def test_collapse_same_line_multicall():
    edges = [edge("a.rs", 10), edge("a.rs", 10), edge("a.rs", 12)]
    sites, collapsed = collapse_sites(edges)
    assert sites == {("a.rs", 10), ("a.rs", 12)}
    assert collapsed == 1


def test_site_compare_line_within_oracle_range_matches():
    # oracle fromRange spans a multi-line call 10..12; prism claims line 11
    prism = [edge("a.rs", 11)]
    oracle = [CallEdge("caller", SEED, Location("a.rs", 5, 20), "c",
                       Location("a.rs", 10, 12))]
    r = site_compare(prism, oracle)
    assert (len(r.tp), len(r.fp), len(r.fn)) == (1, 0, 0)


def test_site_compare_counts_fp_and_fn():
    prism = [edge("a.rs", 30)]
    oracle = [CallEdge("caller", SEED, Location("a.rs", 5, 20), "c",
                       Location("a.rs", 10, 10))]
    r = site_compare(prism, oracle)
    assert (len(r.tp), len(r.fp), len(r.fn)) == (0, 1, 1)


def test_caller_fn_sets_uses_module_level_bucket():
    inv = [FunctionDef("caller_fn", "function", None, Location("a.py", 5, 30), 5)]
    in_fn = edge("a.py", 10)
    at_module = edge("a.py", 50)
    fns = caller_fn_sets([in_fn, at_module], inv)
    assert fns == {("a.py", "caller_fn"), ("a.py", "<module_level>")}


def test_wilson_interval_brackets_point_estimate():
    p, lo, hi = wilson(9, 10)
    assert lo < 0.9 < hi and 0 <= lo and hi <= 1
```

- [ ] **Step 2: Run, verify failure**, then **Step 3: implement**

`eval/tier_a/compare.py`:

```python
"""M2 comparison (spec §2.5): site-level (primary) + caller-function level."""
from __future__ import annotations

from dataclasses import dataclass, field

from .model import CallEdge, FunctionDef


def collapse_sites(edges: list[CallEdge]) -> tuple[set, int]:
    """§2.5: same-line multi-calls collapse to one countable site, counted."""
    sites = [(e.call_site.file, e.call_site.start_line) for e in edges]
    return set(sites), len(sites) - len(set(sites))


@dataclass
class SiteResult:
    tp: set = field(default_factory=set)
    fp: set = field(default_factory=set)   # prism_only sites
    fn: set = field(default_factory=set)   # oracle_only sites
    collapsed: int = 0


def site_compare(prism: list[CallEdge], oracle: list[CallEdge]) -> SiteResult:
    r = SiteResult()
    psites, c1 = collapse_sites(prism)
    r.collapsed = c1
    matched_oracle = set()
    for f, line in psites:
        hit = next(((o.call_site.file, o.call_site.start_line) for o in oracle
                    if o.call_site.file == f
                    and o.call_site.start_line <= line <= o.call_site.end_line), None)
        if hit:
            r.tp.add((f, line))
            matched_oracle.add(hit)
        else:
            r.fp.add((f, line))
    osites, _ = collapse_sites(oracle)
    r.fn = {s for s in osites if s not in matched_oracle}
    return r


def caller_fn_sets(edges: list[CallEdge], inventory: list[FunctionDef]) -> set:
    """§2.5 coarse granularity; sites outside any inventoried fn -> module_level."""
    out = set()
    for e in edges:
        f, line = e.call_site.file, e.call_site.start_line
        within = [fd for fd in inventory if fd.location.file == f
                  and fd.location.start_line <= line <= fd.location.end_line]
        best = min(within, key=lambda fd: fd.location.end_line - fd.location.start_line,
                   default=None)
        out.add((f, best.name if best else "<module_level>"))
    return out
```

`eval/tier_a/metrics.py`:

```python
"""P/R + Wilson 95% (spec §2.10). Denominators per the §2.8 truth table —
the adjudication transforms are applied by adjudication.apply() before this."""
from __future__ import annotations

import math


def wilson(successes: int, n: int, z: float = 1.959964) -> tuple[float, float, float]:
    if n == 0:
        return (float("nan"), 0.0, 1.0)
    p = successes / n
    denom = 1 + z * z / n
    center = (p + z * z / (2 * n)) / denom
    half = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / denom
    return (p, max(0.0, center - half), min(1.0, center + half))


def precision_recall(tp: int, fp: int, fn: int) -> dict:
    return {
        "precision": wilson(tp, tp + fp),
        "recall": wilson(tp, tp + fn),
        "tp": tp, "fp": fp, "fn": fn,
    }
```

`eval/tier_a/pinned.py` (the four §2.5 pinned probes; resolved against the prism
corpus snapshot by name+file at run time):

```python
"""Pinned probes (spec §2.5) — fixed forever, excluded from stratum denominators."""
PINNED = [
    {"id": "target-c-method", "symbol": "target", "file": "src/algorithms/taint.rs",
     "expected": "known_fail",   # G1(b): reproduce raw P<=0.2 and R<=0.2, or flip-candidate
     "note": "prototype C-method total failure; S3 flip indicator"},
    {"id": "module-deps-feature-gated", "symbol": "module_deps",
     "file": "src/navigation/module_graph.rs", "expected": "oracle_miss_site",
     "oracle_miss_site": "src/mcp/tools.rs:162"},
    {"id": "load-repo-feature-gated", "symbol": "load_repo",
     "file": "src/repo_loader.rs", "expected": "oracle_miss_site",
     "oracle_miss_site": "src/mcp/session.rs:28"},
    {"id": "ambiguous-symbol-contract", "symbol": "slice", "file": None,
     "expected": "ambiguous_symbol_error",
     "note": "bare-symbol seed with no file MUST safe-fail (prototype contract)"},
]
```

- [ ] **Step 4: Run tests, verify pass, commit**

```bash
cd eval && uv run pytest -q
git add eval && git commit -m "feat(eval): M2 site/caller-fn comparison, Wilson metrics, pinned probes"
```

### Task 16: Adjudication store — truth table, keys, budget (spec §2.8)

**Files:**
- Create: `eval/tier_a/adjudication.py`, `eval/tests/test_adjudication.py`

The two known feature-gated records are **not** committed here — their `seed_def`
selection lines come from measurement 1's first live run (Task 21 commits them).
This task ships the loader/validator/transform with synthetic-record tests.

- [ ] **Step 1: Failing tests**

`eval/tests/test_adjudication.py`:

```python
import pytest
from tier_a.adjudication import (Adjudication, IllegalAdjudication, apply_verdicts,
                                 load_records, validate)


def rec(direction="prism_only", verdict="oracle_miss", site="src/a.rs:10",
        seed_def="src/s.rs:5", measurement="callers"):
    return Adjudication(corpus="prism", measurement=measurement, direction=direction,
                        seed_def=seed_def, site=site, verdict=verdict,
                        reason="r", adjudicated_by="t", date="2026-06-11")


def test_illegal_direction_verdict_combos_rejected():
    with pytest.raises(IllegalAdjudication):
        validate(rec(direction="prism_only", verdict="prism_fn"))
    with pytest.raises(IllegalAdjudication):
        validate(rec(direction="oracle_only", verdict="oracle_miss"))
    with pytest.raises(IllegalAdjudication):
        validate(rec(direction="oracle_only", verdict="prism_fp"))


def test_truth_table_transforms():
    # raw: 1 TP, prism_only sites {a,b,c,d}, oracle_only sites {e,f}
    fp = {("src/a.rs", 10), ("src/b.rs", 11), ("src/c.rs", 12), ("src/d.rs", 13)}
    fn = {("src/e.rs", 20), ("src/f.rs", 21)}
    records = [
        rec(site="src/a.rs:10", verdict="oracle_miss"),                       # -> TP
        rec(site="src/b.rs:11", verdict="prism_fp"),                          # stays FP
        rec(site="src/c.rs:12", verdict="oracle_artifact"),                   # excluded
        # src/d.rs:13 unadjudicated -> pending, excluded from corrected
        rec(site="src/e.rs:20", direction="oracle_only", verdict="prism_fn"), # stays FN
        # src/f.rs:21 unadjudicated -> pending
    ]
    out = apply_verdicts(tp=1, fp_sites=fp, fn_sites=fn, records=records,
                         corpus="prism", measurement="callers", seed_def="src/s.rs:5")
    assert (out.tp, out.fp, out.fn) == (2, 1, 1)
    assert out.pending == 2
    assert out.oracle_miss_count == 1
    assert out.excluded == 1


def test_ambiguous_and_alias_site_are_excluded_listed():
    # the two §2.6/§2.8 routing verdicts must land in `excluded`, not FP/FN
    fp = {("src/a.rs", 10), ("src/b.rs", 11)}
    records = [rec(site="src/a.rs:10", verdict="ambiguous"),
               rec(site="src/b.rs:11", verdict="alias_site")]
    out = apply_verdicts(tp=0, fp_sites=fp, fn_sites=set(), records=records,
                         corpus="prism", measurement="callers", seed_def="src/s.rs:5")
    assert (out.fp, out.excluded, out.pending) == (0, 2, 0)


def test_stale_records_flagged_not_deleted():
    records = [rec(site="src/gone.rs:99", verdict="prism_fp")]
    out = apply_verdicts(tp=0, fp_sites=set(), fn_sites=set(), records=records,
                         corpus="prism", measurement="callers", seed_def="src/s.rs:5")
    assert out.stale == 1 and out.fp == 0


def test_jsonl_roundtrip(tmp_path):
    p = tmp_path / "adj.jsonl"
    import dataclasses, json
    p.write_text(json.dumps(dataclasses.asdict(rec())) + "\n")
    [r] = load_records(p)
    assert r.verdict == "oracle_miss"
```

- [ ] **Step 2: Run, verify failure**, then **Step 3: implement `eval/tier_a/adjudication.py`**

```python
"""Adjudication store (spec §2.8): keyed records, legal-combo validation, the
metric-contribution truth table, stale/pending/budget accounting."""
from __future__ import annotations

import dataclasses
import json
from dataclasses import dataclass
from pathlib import Path

LEGAL = {
    "prism_only": {"oracle_miss", "prism_fp", "oracle_artifact", "ambiguous", "alias_site"},
    "oracle_only": {"prism_fn", "oracle_artifact", "ambiguous"},
}


class IllegalAdjudication(ValueError):
    pass


@dataclass(frozen=True)
class Adjudication:
    corpus: str
    measurement: str        # "callers" | "callees" | "m3"
    direction: str          # "prism_only" | "oracle_only"
    seed_def: str           # "file:selection_line" of the sampled symbol
    site: str               # "file:line" of the call site
    verdict: str
    reason: str
    adjudicated_by: str
    date: str


def validate(r: Adjudication) -> Adjudication:
    if r.direction not in LEGAL or r.verdict not in LEGAL[r.direction]:
        raise IllegalAdjudication(f"{r.direction} x {r.verdict} is not a legal combination")
    return r


def load_records(path: Path) -> list[Adjudication]:
    if not path.exists():
        return []
    return [validate(Adjudication(**json.loads(line)))
            for line in path.read_text().splitlines() if line.strip()]


@dataclass
class Corrected:
    tp: int = 0
    fp: int = 0
    fn: int = 0
    pending: int = 0
    excluded: int = 0
    oracle_miss_count: int = 0
    stale: int = 0


def _key(file: str, line: int) -> str:
    return f"{file}:{line}"


def apply_verdicts(tp: int, fp_sites: set, fn_sites: set,
                   records: list[Adjudication], corpus: str, measurement: str,
                   seed_def: str) -> Corrected:
    """The §2.8 truth table. fp_sites/fn_sites are (file, line) raw-diff sets."""
    out = Corrected(tp=tp)
    rel = {(r.direction, r.site): r for r in records
           if r.corpus == corpus and r.measurement == measurement
           and r.seed_def == seed_def}
    live_sites = ({("prism_only", _key(f, l)) for f, l in fp_sites}
                  | {("oracle_only", _key(f, l)) for f, l in fn_sites})
    out.stale = sum(1 for k in rel if k not in live_sites)
    for f, l in fp_sites:
        r = rel.get(("prism_only", _key(f, l)))
        if r is None:
            out.pending += 1
        elif r.verdict == "oracle_miss":
            out.tp += 1
            out.oracle_miss_count += 1
        elif r.verdict == "prism_fp":
            out.fp += 1
        else:                       # oracle_artifact | ambiguous | alias_site
            out.excluded += 1
    for f, l in fn_sites:
        r = rel.get(("oracle_only", _key(f, l)))
        if r is None:
            out.pending += 1
        elif r.verdict == "prism_fn":
            out.fn += 1
        else:
            out.excluded += 1
    return out
```

- [ ] **Step 4: Run tests, pass, commit**

```bash
cd eval && uv run pytest -q
git add eval && git commit -m "feat(eval): adjudication store + §2.8 truth table (tested)"
```

### Task 17: M3 definition spot-check (spec §2.6)

**Files:**
- Create: `eval/tier_a/spotcheck.py`, `eval/tests/test_spotcheck.py`

- [ ] **Step 1: Failing tests**

`eval/tests/test_spotcheck.py`:

```python
from tier_a.model import DefTarget, FunctionDef, Location
from tier_a.spotcheck import classify_site, find_call_position

SEED = FunctionDef("target", "method", "TaintSeed",
                   Location("src/algorithms/taint.rs", 1276, 1278), 1276)


def test_call_position_preferred_over_binding():
    # bare-first-occurrence would hit the LHS local and mint a false FP (§2.6)
    line = "    let target = edge.target();"
    assert find_call_position(line, "target") == line.index("edge.target") + len("edge.")


def test_name_absent_is_alias_site_not_fp():
    assert classify_site("    g()", "target", [], SEED) == "alias_site"


def test_any_matching_definition_confirms_tp():
    defs = [DefTarget(Location("src/algorithms/taint.rs", 1276, 1278), "target", "method")]
    assert classify_site("    seed.target()", "target", defs, SEED) == "confirmed_tp"


def test_all_other_named_definitions_confirm_fp():
    defs = [DefTarget(Location("src/petgraph_shim.rs", 40, 44), "edge_endpoint", "method")]
    assert classify_site("    e.target()", "target", defs, SEED) == "confirmed_fp"


def test_same_name_different_def_is_ambiguous():
    # oracle returned a trait/interface DECLARATION named like the seed (§2.6)
    defs = [DefTarget(Location("src/traits.rs", 10, 12), "target", "method")]
    assert classify_site("    s.target()", "target", defs, SEED) == "ambiguous"
```

- [ ] **Step 2: Run, verify failure**, then **Step 3: implement `eval/tier_a/spotcheck.py`**

```python
"""M3 site-level definition spot-check (spec §2.6 verdict table)."""
from __future__ import annotations

import re

from .model import DefTarget, FunctionDef


def _strip_strings_comments(line: str) -> str:
    line = re.sub(r'"(?:[^"\\]|\\.)*"', '""', line)
    line = re.sub(r"'(?:[^'\\]|\\.)*'", "''", line)
    return re.split(r"//|#", line, maxsplit=1)[0]


def find_call_position(line: str, name: str) -> int | None:
    """Column of `name` in CALL position: name( , .name( , ::name( ; fallback any
    occurrence; None if the token is absent entirely."""
    code = _strip_strings_comments(line)
    for m in re.finditer(rf"(?:(?<=\.)|(?<=::)|\b){re.escape(name)}\s*\(", code):
        return m.start()
    m = re.search(rf"\b{re.escape(name)}\b", code)
    return m.start() if m else None


def classify_site(line: str, seed_name: str, defs: list[DefTarget],
                  seed: FunctionDef) -> str:
    """§2.6 verdict table: confirmed_tp | confirmed_fp | ambiguous | alias_site."""
    if find_call_position(line, seed_name) is None:
        return "alias_site"          # -> adjudication, never auto-FP
    if not defs:
        return "ambiguous"           # oracle returned nothing usable
    for d in defs:
        if (d.name == seed.name and d.location.file == seed.location.file
                and d.location.start_line <= seed.selection_line <= d.location.end_line):
            return "confirmed_tp"
    if any(d.name == seed.name for d in defs):
        return "ambiguous"           # same name, different def: declaration vs impl
    return "confirmed_fp"            # all definitions name something else entirely
```

- [ ] **Step 4: Run tests, pass, commit**

```bash
cd eval && uv run pytest -q
git add eval && git commit -m "feat(eval): M3 spot-check — call-position finder + §2.6 verdicts"
```

### Task 18: Capability matrix — runner + all 29 fixtures (spec §2.7)

**Files:**
- Create: `eval/tier_a/matrix.py`, `eval/tests/test_matrix.py`,
  `eval/fixtures/{rust,go,python}/<capability>/...` (sources + `expected.toml`)

- [ ] **Step 1: Create the fixtures (run these three scripts from repo root, then `git add eval/fixtures`)**

Initial `status` values encode the documented gaps (cross-file receiver/method
resolution, dyn/interface dispatch, aliases, the decorator quirk); **Step 5 reconciles
them against reality** — the first run's observed truth wins, with flips called out.

Rust:

```bash
F=eval/fixtures/rust
mk() { mkdir -p "$F/$1"; }
mk free_fn_same_file && cat > $F/free_fn_same_file/main.rs << 'EOF'
fn helper() {}

fn run() {
    helper();
}
EOF
cat > $F/free_fn_same_file/expected.toml << 'EOF'
[case]
language = "rust"
capability = "free_fn_same_file"
status = "pass"
[seed]
symbol = "helper"
file = "main.rs"
line = 1
[[expect.callers]]
file = "main.rs"
line = 4
[expect]
exact = true
EOF
mk free_fn_cross_file_use && cat > $F/free_fn_cross_file_use/helpers.rs << 'EOF'
pub fn compute() {}
EOF
cat > $F/free_fn_cross_file_use/main.rs << 'EOF'
use helpers::compute;

fn run() {
    compute();
}
EOF
cat > $F/free_fn_cross_file_use/expected.toml << 'EOF'
[case]
language = "rust"
capability = "free_fn_cross_file_use"
status = "pass"
[seed]
symbol = "compute"
file = "helpers.rs"
line = 1
[[expect.callers]]
file = "main.rs"
line = 4
[expect]
exact = true
EOF
mk mod_qualified_free_fn && cat > $F/mod_qualified_free_fn/util.rs << 'EOF'
pub fn tick() {}
EOF
cat > $F/mod_qualified_free_fn/main.rs << 'EOF'
mod util;

fn run() {
    util::tick();
}
EOF
cat > $F/mod_qualified_free_fn/expected.toml << 'EOF'
[case]
language = "rust"
capability = "mod_qualified_free_fn"
status = "pass"
[seed]
symbol = "tick"
file = "util.rs"
line = 1
[[expect.callers]]
file = "main.rs"
line = 4
[expect]
exact = true
EOF
mk inherent_method_same_file && cat > $F/inherent_method_same_file/main.rs << 'EOF'
struct Engine;

impl Engine {
    fn start(&self) {}
}

fn run(e: Engine) {
    e.start();
}
EOF
cat > $F/inherent_method_same_file/expected.toml << 'EOF'
[case]
language = "rust"
capability = "inherent_method_same_file"
status = "pass"
[seed]
symbol = "start"
file = "main.rs"
line = 4
[[expect.callers]]
file = "main.rs"
line = 8
[expect]
exact = true
EOF
mk receiver_method_cross_file_stem_eq && cat > $F/receiver_method_cross_file_stem_eq/engine.rs << 'EOF'
pub struct Engine;

impl Engine {
    pub fn start(&self) {}
}
EOF
cat > $F/receiver_method_cross_file_stem_eq/main.rs << 'EOF'
mod engine;

fn run(e: engine::Engine) {
    e.start();
}
EOF
cat > $F/receiver_method_cross_file_stem_eq/expected.toml << 'EOF'
[case]
language = "rust"
capability = "receiver_method_cross_file_stem_eq"
status = "known_fail"
[seed]
symbol = "start"
file = "engine.rs"
line = 4
[[expect.callers]]
file = "main.rs"
line = 4
[expect]
exact = true
EOF
mk method_cross_file_type_ne_stem && cat > $F/method_cross_file_type_ne_stem/worker.rs << 'EOF'
pub struct Processor;

impl Processor {
    pub fn process(&self) {}
}
EOF
cat > $F/method_cross_file_type_ne_stem/main.rs << 'EOF'
mod worker;

fn run(p: worker::Processor) {
    p.process();
}
EOF
cat > $F/method_cross_file_type_ne_stem/expected.toml << 'EOF'
[case]
language = "rust"
capability = "method_cross_file_type_ne_stem"
status = "known_fail"
[seed]
symbol = "process"
file = "worker.rs"
line = 4
[[expect.callers]]
file = "main.rs"
line = 4
[expect]
exact = true
EOF
mk trait_static_dispatch && cat > $F/trait_static_dispatch/main.rs << 'EOF'
trait Runner {
    fn go(&self);
}

struct Fast;

impl Runner for Fast {
    fn go(&self) {}
}

fn run(f: Fast) {
    f.go();
}
EOF
cat > $F/trait_static_dispatch/expected.toml << 'EOF'
[case]
language = "rust"
capability = "trait_static_dispatch"
status = "pass"
[seed]
symbol = "go"
file = "main.rs"
line = 8
[[expect.callers]]
file = "main.rs"
line = 12
[expect]
exact = true
EOF
mk trait_dyn_dispatch && cat > $F/trait_dyn_dispatch/main.rs << 'EOF'
trait Runner {
    fn go(&self);
}

struct Fast;

impl Runner for Fast {
    fn go(&self) {}
}

fn run(r: &dyn Runner) {
    r.go();
}
EOF
cat > $F/trait_dyn_dispatch/expected.toml << 'EOF'
[case]
language = "rust"
capability = "trait_dyn_dispatch"
status = "known_fail"
[seed]
symbol = "go"
file = "main.rs"
line = 8
[[expect.callers]]
file = "main.rs"
line = 12
[expect]
exact = true
EOF
mk type_method_qualified && cat > $F/type_method_qualified/engine.rs << 'EOF'
pub struct Engine;

impl Engine {
    pub fn start_static() {}
}
EOF
cat > $F/type_method_qualified/main.rs << 'EOF'
mod engine;

fn run() {
    engine::Engine::start_static();
}
EOF
cat > $F/type_method_qualified/expected.toml << 'EOF'
[case]
language = "rust"
capability = "type_method_qualified"
status = "pass"
[seed]
symbol = "start_static"
file = "engine.rs"
line = 4
[[expect.callers]]
file = "main.rs"
line = 4
[expect]
exact = true
EOF
mk closure_call && cat > $F/closure_call/main.rs << 'EOF'
fn target_fn() {}

fn run() {
    let f = || target_fn();
    f();
}
EOF
cat > $F/closure_call/expected.toml << 'EOF'
[case]
language = "rust"
capability = "closure_call"
status = "pass"
[seed]
symbol = "target_fn"
file = "main.rs"
line = 1
[[expect.callers]]
file = "main.rs"
line = 4
[expect]
exact = true
EOF
mk common_name_collision && cat > $F/common_name_collision/alpha.rs << 'EOF'
pub fn process() {}
EOF
cat > $F/common_name_collision/beta.rs << 'EOF'
pub fn process() {}
EOF
cat > $F/common_name_collision/main.rs << 'EOF'
mod alpha;
mod beta;

fn run() {
    alpha::process();
}
EOF
cat > $F/common_name_collision/expected.toml << 'EOF'
[case]
language = "rust"
capability = "common_name_collision"
status = "known_fail"
[seed]
symbol = "process"
file = "alpha.rs"
line = 1
[[expect.callers]]
file = "main.rs"
line = 5
[expect]
exact = true
EOF
mk field_receiver_method && cat > $F/field_receiver_method/main.rs << 'EOF'
struct Inner;

impl Inner {
    fn poke(&self) {}
}

struct Outer {
    inner: Inner,
}

fn run(o: Outer) {
    o.inner.poke();
}
EOF
cat > $F/field_receiver_method/expected.toml << 'EOF'
[case]
language = "rust"
capability = "field_receiver_method"
status = "pass"
[seed]
symbol = "poke"
file = "main.rs"
line = 4
[[expect.callers]]
file = "main.rs"
line = 12
[expect]
exact = true
EOF
```

Go:

```bash
F=eval/fixtures/go
mkdir -p $F/same_pkg_free_fn && cat > $F/same_pkg_free_fn/main.go << 'EOF'
package main

func helper() {}

func run() {
	helper()
}
EOF
cat > $F/same_pkg_free_fn/expected.toml << 'EOF'
[case]
language = "go"
capability = "same_pkg_free_fn"
status = "pass"
[seed]
symbol = "helper"
file = "main.go"
line = 3
[[expect.callers]]
file = "main.go"
line = 6
[expect]
exact = true
EOF
mkdir -p $F/cross_pkg_qualified/util && cat > $F/cross_pkg_qualified/util/util.go << 'EOF'
package util

func Tick() {}
EOF
cat > $F/cross_pkg_qualified/main.go << 'EOF'
package main

import "example.com/m/util"

func run() {
	util.Tick()
}
EOF
cat > $F/cross_pkg_qualified/expected.toml << 'EOF'
[case]
language = "go"
capability = "cross_pkg_qualified"
status = "pass"
[seed]
symbol = "Tick"
file = "util/util.go"
line = 3
[[expect.callers]]
file = "main.go"
line = 6
[expect]
exact = true
EOF
mkdir -p $F/struct_method_same_file && cat > $F/struct_method_same_file/main.go << 'EOF'
package main

type Engine struct{}

func (e Engine) Start() {}

func run(e Engine) {
	e.Start()
}
EOF
cat > $F/struct_method_same_file/expected.toml << 'EOF'
[case]
language = "go"
capability = "struct_method_same_file"
status = "pass"
[seed]
symbol = "Start"
file = "main.go"
line = 5
[[expect.callers]]
file = "main.go"
line = 8
[expect]
exact = true
EOF
mkdir -p $F/struct_method_cross_file && cat > $F/struct_method_cross_file/engine.go << 'EOF'
package main

type Engine struct{}

func (e Engine) Start() {}
EOF
cat > $F/struct_method_cross_file/main.go << 'EOF'
package main

func run(e Engine) {
	e.Start()
}
EOF
cat > $F/struct_method_cross_file/expected.toml << 'EOF'
[case]
language = "go"
capability = "struct_method_cross_file"
status = "known_fail"
[seed]
symbol = "Start"
file = "engine.go"
line = 5
[[expect.callers]]
file = "main.go"
line = 4
[expect]
exact = true
EOF
mkdir -p $F/interface_dispatch && cat > $F/interface_dispatch/main.go << 'EOF'
package main

type Runner interface {
	Go()
}

type Fast struct{}

func (f Fast) Go() {}

func run(r Runner) {
	r.Go()
}
EOF
cat > $F/interface_dispatch/expected.toml << 'EOF'
[case]
language = "go"
capability = "interface_dispatch"
status = "known_fail"
[seed]
symbol = "Go"
file = "main.go"
line = 9
[[expect.callers]]
file = "main.go"
line = 12
[expect]
exact = true
EOF
mkdir -p $F/embedded_method && cat > $F/embedded_method/main.go << 'EOF'
package main

type Base struct{}

func (b Base) Ping() {}

type Wrap struct {
	Base
}

func run(w Wrap) {
	w.Ping()
}
EOF
cat > $F/embedded_method/expected.toml << 'EOF'
[case]
language = "go"
capability = "embedded_method"
status = "known_fail"
[seed]
symbol = "Ping"
file = "main.go"
line = 5
[[expect.callers]]
file = "main.go"
line = 12
[expect]
exact = true
EOF
mkdir -p $F/closure && cat > $F/closure/main.go << 'EOF'
package main

func target() {}

func run() {
	f := func() { target() }
	f()
}
EOF
cat > $F/closure/expected.toml << 'EOF'
[case]
language = "go"
capability = "closure"
status = "pass"
[seed]
symbol = "target"
file = "main.go"
line = 3
[[expect.callers]]
file = "main.go"
line = 6
[expect]
exact = true
EOF
mkdir -p $F/common_name_collision/alpha $F/common_name_collision/beta
cat > $F/common_name_collision/alpha/alpha.go << 'EOF'
package alpha

func Process() {}
EOF
cat > $F/common_name_collision/beta/beta.go << 'EOF'
package beta

func Process() {}
EOF
cat > $F/common_name_collision/main.go << 'EOF'
package main

import "example.com/m/alpha"

func run() {
	alpha.Process()
}
EOF
cat > $F/common_name_collision/expected.toml << 'EOF'
[case]
language = "go"
capability = "common_name_collision"
status = "known_fail"
[seed]
symbol = "Process"
file = "alpha/alpha.go"
line = 3
[[expect.callers]]
file = "main.go"
line = 6
[expect]
exact = true
EOF
```

Python:

```bash
F=eval/fixtures/python
mkdir -p $F/module_fn && cat > $F/module_fn/app.py << 'EOF'
def helper():
    pass

def run():
    helper()
EOF
cat > $F/module_fn/expected.toml << 'EOF'
[case]
language = "python"
capability = "module_fn"
status = "pass"
[seed]
symbol = "helper"
file = "app.py"
line = 1
[[expect.callers]]
file = "app.py"
line = 5
[expect]
exact = true
EOF
mkdir -p $F/import_module_call && cat > $F/import_module_call/util.py << 'EOF'
def tick():
    pass
EOF
cat > $F/import_module_call/app.py << 'EOF'
import util

def run():
    util.tick()
EOF
cat > $F/import_module_call/expected.toml << 'EOF'
[case]
language = "python"
capability = "import_module_call"
status = "pass"
[seed]
symbol = "tick"
file = "util.py"
line = 1
[[expect.callers]]
file = "app.py"
line = 4
[expect]
exact = true
EOF
mkdir -p $F/from_import_alias && cat > $F/from_import_alias/util.py << 'EOF'
def tick():
    pass
EOF
cat > $F/from_import_alias/app.py << 'EOF'
from util import tick as t

def run():
    t()
EOF
cat > $F/from_import_alias/expected.toml << 'EOF'
[case]
language = "python"
capability = "from_import_alias"
status = "known_fail"
[seed]
symbol = "tick"
file = "util.py"
line = 1
[[expect.callers]]
file = "app.py"
line = 4
[expect]
exact = true
EOF
mkdir -p $F/class_method_same_file && cat > $F/class_method_same_file/app.py << 'EOF'
class Engine:
    def start(self):
        pass

def run(e):
    e.start()
EOF
cat > $F/class_method_same_file/expected.toml << 'EOF'
[case]
language = "python"
capability = "class_method_same_file"
status = "pass"
[seed]
symbol = "start"
file = "app.py"
line = 2
[[expect.callers]]
file = "app.py"
line = 6
[expect]
exact = true
EOF
mkdir -p $F/instance_method_cross_file && cat > $F/instance_method_cross_file/engine.py << 'EOF'
class Engine:
    def start(self):
        pass
EOF
cat > $F/instance_method_cross_file/app.py << 'EOF'
from engine import Engine

def run():
    e = Engine()
    e.start()
EOF
cat > $F/instance_method_cross_file/expected.toml << 'EOF'
[case]
language = "python"
capability = "instance_method_cross_file"
status = "known_fail"
[seed]
symbol = "start"
file = "engine.py"
line = 2
[[expect.callers]]
file = "app.py"
line = 5
[expect]
exact = true
EOF
mkdir -p $F/inherited_override && cat > $F/inherited_override/app.py << 'EOF'
class Base:
    def go(self):
        pass

class Child(Base):
    def go(self):
        pass

def run(c):
    c.go()
EOF
cat > $F/inherited_override/expected.toml << 'EOF'
[case]
language = "python"
capability = "inherited_override"
status = "known_fail"
[seed]
symbol = "go"
file = "app.py"
line = 6
[[expect.callers]]
file = "app.py"
line = 10
[expect]
exact = true
EOF
mkdir -p $F/decorator_wrapped && cat > $F/decorator_wrapped/app.py << 'EOF'
import functools

@functools.cache
def handler(x):
    return x

def run():
    handler(1)
EOF
cat > $F/decorator_wrapped/expected.toml << 'EOF'
[case]
language = "python"
capability = "decorator_wrapped"
# B2's designated flip indicator (spec §2.7): the func_index (file,name) quirk
status = "known_fail"
[seed]
symbol = "handler"
file = "app.py"
line = 4
[[expect.callers]]
file = "app.py"
line = 8
[expect]
exact = true
EOF
mkdir -p $F/closure && cat > $F/closure/app.py << 'EOF'
def target():
    pass

def run():
    def inner():
        target()
    inner()
EOF
cat > $F/closure/expected.toml << 'EOF'
[case]
language = "python"
capability = "closure"
status = "pass"
[seed]
symbol = "target"
file = "app.py"
line = 1
[[expect.callers]]
file = "app.py"
line = 6
[expect]
exact = true
EOF
mkdir -p $F/common_name_collision && cat > $F/common_name_collision/alpha.py << 'EOF'
def process():
    pass
EOF
cat > $F/common_name_collision/beta.py << 'EOF'
def process():
    pass
EOF
cat > $F/common_name_collision/app.py << 'EOF'
import alpha

def run():
    alpha.process()
EOF
cat > $F/common_name_collision/expected.toml << 'EOF'
[case]
language = "python"
capability = "common_name_collision"
status = "known_fail"
[seed]
symbol = "process"
file = "alpha.py"
line = 1
[[expect.callers]]
file = "app.py"
line = 4
[expect]
exact = true
EOF
```

- [ ] **Step 2: Failing runner test**

`eval/tests/test_matrix.py`:

```python
from pathlib import Path

from tier_a.matrix import load_case, run_matrix
from tier_a.model import CallEdge, FunctionDef, Location

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures"


class FakeSut:
    """Resolves the rust free_fn_same_file case correctly; everything else empty."""

    def callers(self, root, seed):
        if seed.name == "helper" and root.endswith("free_fn_same_file"):
            return [CallEdge("caller", seed, Location("main.rs", 3, 5), "run",
                             Location("main.rs", 4, 4))]
        return []


def test_load_case_parses_expected_toml():
    case = load_case(FIXTURES / "rust" / "free_fn_same_file" / "expected.toml")
    assert case.capability == "free_fn_same_file"
    assert case.seed_symbol == "helper" and case.seed_line == 1
    assert case.expect_callers == {("main.rs", 4)}
    assert case.exact and case.status == "pass"


def test_run_matrix_statuses():
    results = run_matrix(FIXTURES, FakeSut(), languages=["rust"])
    by_cap = {r.capability: r for r in results}
    assert by_cap["free_fn_same_file"].outcome == "ok"
    # a `pass` case the FakeSut can't resolve -> regression
    assert by_cap["free_fn_cross_file_use"].outcome == "regression"
    # a `known_fail` case still failing -> expected_gap
    assert by_cap["trait_dyn_dispatch"].outcome == "expected_gap"


def test_run_matrix_flags_flip_candidates():
    class FlipSut(FakeSut):
        def callers(self, root, seed):
            if root.endswith("trait_dyn_dispatch"):
                return [CallEdge("caller", seed, Location("main.rs", 11, 13), "run",
                                 Location("main.rs", 12, 12))]
            return super().callers(root, seed)

    results = run_matrix(FIXTURES, FlipSut(), languages=["rust"])
    by_cap = {r.capability: r for r in results}
    assert by_cap["trait_dyn_dispatch"].outcome == "flip_candidate"
```

- [ ] **Step 3: Run, verify failure**, then **Step 4: implement `eval/tier_a/matrix.py`**

```python
"""Capability matrix runner (spec §2.7): by-construction ground truth, no LSP.
Outcomes: ok | regression (pass case failing -> fails the run) |
expected_gap | flip_candidate (known_fail now passing -> report, update status)."""
from __future__ import annotations

import tomllib
from dataclasses import dataclass
from pathlib import Path

from .model import FunctionDef, Location


@dataclass
class Case:
    path: Path
    language: str
    capability: str
    status: str
    seed_symbol: str
    seed_file: str
    seed_line: int
    expect_callers: set
    exact: bool


@dataclass
class CaseResult:
    capability: str
    language: str
    outcome: str
    got: set
    expected: set


def load_case(toml_path: Path) -> Case:
    d = tomllib.loads(toml_path.read_text())
    return Case(
        path=toml_path.parent,
        language=d["case"]["language"],
        capability=d["case"]["capability"],
        status=d["case"]["status"],
        seed_symbol=d["seed"]["symbol"],
        seed_file=d["seed"]["file"],
        seed_line=d["seed"]["line"],
        expect_callers={(c["file"], c["line"]) for c in d["expect"]["callers"]},
        exact=d["expect"].get("exact", True),
    )


def run_matrix(fixtures_root: Path, sut, languages: list[str]) -> list[CaseResult]:
    results = []
    for lang in languages:
        for toml_path in sorted((fixtures_root / lang).glob("*/expected.toml")):
            case = load_case(toml_path)
            seed = FunctionDef(case.seed_symbol, "function", None,
                               Location(case.seed_file, case.seed_line, case.seed_line),
                               case.seed_line)
            edges = sut.callers(str(case.path), seed)
            got = {(e.call_site.file, e.call_site.start_line) for e in edges}
            matched = got == case.expect_callers if case.exact \
                else case.expect_callers <= got
            if case.status == "pass":
                outcome = "ok" if matched else "regression"
            else:
                outcome = "flip_candidate" if matched else "expected_gap"
            results.append(CaseResult(case.capability, lang, outcome, got,
                                      case.expect_callers))
    return results
```

Also add the spec §2.12-named live self-test (skipped when the binary is absent) so
future fixture edits get an automated real-binary check, not just the one-off
reconciliation below — append to `eval/tests/test_matrix.py`:

```python
import os

import pytest

PRISM_BIN = os.environ.get("PRISM_BIN", str(Path(__file__).resolve().parents[2]
                                            / "target/release/prism"))


@pytest.mark.skipif(not os.path.exists(PRISM_BIN), reason="release prism binary absent")
def test_matrix_against_real_binary_has_no_regressions():
    from tier_a.sut import PrismCli
    sut = PrismCli(str(Path(__file__).resolve().parents[2]), sut_bin=PRISM_BIN,
                   allow_stale=True)   # self-test: freshness is Task 21's concern
    results = run_matrix(FIXTURES, sut, ["rust", "go", "python"])
    regressions = [r for r in results if r.outcome == "regression"]
    assert not regressions, f"matrix regressions: {regressions}"
```

- [ ] **Step 5: Run tests; then reconcile fixture statuses against the real binary**

*(Execution-model note: this step needs the release binary AND Python 3.12 + uv in one
environment. If the container lacks the Python side, this step — like the pytest suite
— falls back to the orchestrator host, per spec §5; the implementor then commits
fixture-status edits the orchestrator reports back.)*

```bash
cd eval && uv run pytest -q          # runner tests pass against FakeSut
cd .. && cargo build --release
# real reconciliation (PrismCli against each fixture dir):
cd eval && uv run python -c "
from pathlib import Path
from tier_a.matrix import run_matrix
from tier_a.sut import PrismCli
sut = PrismCli('..', allow_stale=False)
for r in run_matrix(Path('fixtures'), sut, ['rust', 'go', 'python']):
    print(f'{r.language}/{r.capability}: {r.outcome}  got={sorted(r.got)}')"
```

For every `regression`: the initial `status = "pass"` guess was wrong — flip it to
`known_fail`. For every `flip_candidate`: flip to `pass`. Re-run until the matrix
reports only `ok`/`expected_gap` (G5: statuses = observed truth at first commit; the
plan's initial guesses are hypotheses, the binary is the referee). Record any guess
that flipped in the commit message.

- [ ] **Step 6: Commit**

```bash
git add eval && git commit -m "feat(eval): capability matrix — runner + 29 fixtures, statuses reconciled (§2.7)"
```

### Task 19: Probe accounting + validity floors (`accounting.py`) (spec §2.2/§2.5)

**Files:**
- Create: `eval/tier_a/accounting.py`, `eval/tests/test_accounting.py`

The bookkeeping the gates stand on (review B2): per-probe outcome classification,
the §2.5 `inventory_miss` → all-oracle-edges-FN rule, error rates, and the §2.2
validity floors (failure-based; natural shortfall non-gating). A bug here silently
corrupts P/R — it gets the same TDD treatment as the metrics.

- [ ] **Step 1: Failing tests**

`eval/tests/test_accounting.py`:

```python
from tier_a.accounting import CorpusAccounting, evaluate_floors


def test_probe_outcomes_and_rates():
    acc = CorpusAccounting()
    acc.record("s1", "ok")
    acc.record("s2", "oracle_error")
    acc.record("s3", "sut_error")
    acc.record("s4", "inventory_miss")
    assert acc.oracle_error_rate() == 0.25
    assert acc.sut_error_rate() == 0.25
    assert acc.successful() == 1          # only fully-scored probes
    assert acc.inventory_misses == ["s4"]


def test_inventory_miss_scores_all_oracle_edges_as_fn():
    # §2.5: an unmatched sampled symbol is not seeded; its oracle edges are ALL FN
    acc = CorpusAccounting()
    fn_sites = acc.score_inventory_miss(oracle_sites={("a.rs", 1), ("a.rs", 9)})
    assert fn_sites == {("a.rs", 1), ("a.rs", 9)}


def test_floors_failure_based_not_population_based():
    # stratum floor: successful >= min(6, eligible) — natural shortfall non-gating
    ok, reasons = evaluate_floors(
        strata={"U-free": {"eligible": 3, "successful": 3},     # undersized but full
                "C-method": {"eligible": 8, "successful": 5}},  # failures ate probes
        oracle_error_rate=0.05, sut_error_rate=0.0,
        oracle_floor=0.10, sut_floor=0.05)
    assert not ok and any("C-method" in r for r in reasons)


def test_floors_corpus_rates():
    ok, reasons = evaluate_floors(
        strata={"U-free": {"eligible": 8, "successful": 8}},
        oracle_error_rate=0.30, sut_error_rate=0.06,
        oracle_floor=0.25, sut_floor=0.05)
    assert not ok and len(reasons) == 2   # both rate floors breached


def test_floors_pass():
    ok, reasons = evaluate_floors(
        strata={"U-free": {"eligible": 8, "successful": 7}},
        oracle_error_rate=0.05, sut_error_rate=0.0,
        oracle_floor=0.10, sut_floor=0.05)
    assert ok and reasons == []
```

- [ ] **Step 2: Run, verify failure**, then **Step 3: implement `eval/tier_a/accounting.py`**

```python
"""Probe accounting + validity floors (spec §2.2/§2.5, review B2)."""
from __future__ import annotations

from dataclasses import dataclass, field

OUTCOMES = ("ok", "oracle_error", "sut_error", "inventory_miss")


@dataclass
class CorpusAccounting:
    probes: dict[str, str] = field(default_factory=dict)   # probe id -> outcome
    inventory_misses: list[str] = field(default_factory=list)

    def record(self, probe_id: str, outcome: str) -> None:
        assert outcome in OUTCOMES, outcome
        self.probes[probe_id] = outcome
        if outcome == "inventory_miss":
            self.inventory_misses.append(probe_id)

    def _rate(self, outcome: str) -> float:
        return (sum(1 for o in self.probes.values() if o == outcome)
                / len(self.probes)) if self.probes else 0.0

    def oracle_error_rate(self) -> float:
        return self._rate("oracle_error")

    def sut_error_rate(self) -> float:
        return self._rate("sut_error")

    def successful(self) -> int:
        return sum(1 for o in self.probes.values() if o == "ok")

    def score_inventory_miss(self, oracle_sites: set) -> set:
        """§2.5: prism couldn't be seeded — every oracle site is a recall miss."""
        return set(oracle_sites)


def evaluate_floors(strata: dict, oracle_error_rate: float, sut_error_rate: float,
                    oracle_floor: float, sut_floor: float) -> tuple[bool, list[str]]:
    """§2.2 failure-based floors. Returns (baseline_valid, reasons)."""
    reasons = []
    for name, s in strata.items():
        need = min(6, s["eligible"])
        if s["successful"] < need:
            reasons.append(f"stratum {name}: {s['successful']}/{need} successful probes")
    if oracle_error_rate > oracle_floor:
        reasons.append(f"oracle_error_rate {oracle_error_rate:.2f} > {oracle_floor:.2f}")
    if sut_error_rate > sut_floor:
        reasons.append(f"sut_error_rate {sut_error_rate:.2f} > {sut_floor:.2f}")
    return (not reasons, reasons)
```

- [ ] **Step 4: Run tests, pass, commit**

```bash
cd eval && uv run pytest -q
git add eval && git commit -m "feat(eval): probe accounting + §2.2 validity floors (tested)"
```

### Task 20: corpora.toml, runner CLI, reports, README, agent guidance (spec §2.9–§2.11, G8)

**Files:**
- Create: `eval/corpora.toml`, `eval/tier_a/cli.py`, `eval/tier_a/report.py`,
  `eval/tests/test_report.py`
- Modify: `eval/README.md`, `CLAUDE.md`; Create: `AGENTS.md`

- [ ] **Step 1: `eval/corpora.toml`**

```toml
# Tier-A corpora (spec §2.9). SHAs pinned at first baseline run (Task 21).
# Selection principle: repeat-run friendliness — prism CPG cold-builds recur.
[defaults]
seed = 42
per_stratum = 8
quiescence_cap_s = 300
settle_s = 2.0
oracle_error_floor = { rust = 0.10, go = 0.10, python = 0.25 }
sut_error_floor = 0.05

[corpus.prism]
lang = "rust"
path = "."                       # the prism repo itself
oracle = "rust-analyzer"
excludes = ["tests/fixtures/*", "eval/fixtures/*"]
pinned_sha = ""                  # filled at first run

[corpus.tokio]
lang = "rust"
path = "~/code/bench-repos/tokio"
oracle = "rust-analyzer"
excludes = []
pinned_sha = ""

[corpus.caddy]
lang = "go"
path = "~/code/bench-repos/caddy"
oracle = "gopls"
excludes = []
pinned_sha = ""                  # copied from ~/code/agent-eval/cache/repos/caddy @ 77e9ce74

[corpus.flask]
lang = "python"
path = "~/code/bench-repos/flask"
oracle = "pyright"
excludes = ["examples/*"]
pinned_sha = ""

[corpus.click]
lang = "python"
path = "~/code/bench-repos/click"
oracle = "pyright"
excludes = ["examples/*"]
pinned_sha = ""
```

- [ ] **Step 2: Failing report test**

`eval/tests/test_report.py`:

```python
from tier_a.report import render_markdown

RUN = {
    "meta": {"corpus": "prism", "corpus_sha": "abc123def456", "corpus_dirty": False,
             "prism_sha": "abc123def456", "oracle": "rust-analyzer 1.94.0",
             "seed": 42, "harness_sha": "deadbeef", "date": "2026-06-11",
             "wall_s": {"m1": 1.0, "m2": 2.0, "m3": 0.5, "matrix": 0.2},
             "oracle_error_rate": 0.0, "sut_error_rate": 0.0,
             "baseline_invalid": False, "oracle_not_quiescent": False},
    "m2": {"callers": {"U-free": {"raw": {"precision": [1.0, 0.7, 1.0],
                                          "recall": [1.0, 0.7, 1.0],
                                          "tp": 10, "fp": 0, "fn": 0},
                                  "corrected": {"precision": [1.0, 0.7, 1.0],
                                                "recall": [1.0, 0.7, 1.0],
                                                "tp": 10, "fp": 0, "fn": 0},
                                  "pending": 0, "shortfall": 0}}},
}


def test_render_markdown_shows_wilson_and_metadata():
    md = render_markdown(RUN)
    assert "rust-analyzer 1.94.0" in md
    assert "1.00 [0.70–1.00]" in md       # point [lo–hi] formatting (§2.10)
    assert "abc123def456" in md
```

- [ ] **Step 3: Implement `eval/tier_a/report.py`** (JSON is just `json.dumps(run)`;
markdown renders meta block, per-stratum raw/corrected tables, pending-triage list,
matrix grid):

```python
"""Report rendering (spec §2.10): per-corpus md+json under docs/eval/tier-a/."""
from __future__ import annotations

import json
from pathlib import Path


def fmt_wilson(t: list | tuple) -> str:
    p, lo, hi = t
    return f"{p:.2f} [{lo:.2f}–{hi:.2f}]"


def render_markdown(run: dict) -> str:
    m = run["meta"]
    lines = [
        f"# Tier-A run — {m['corpus']} ({m['date']})",
        "",
        f"- corpus: `{m['corpus']}` @ `{m['corpus_sha']}`"
        + (" **dirty**" if m.get("corpus_dirty") else ""),
        f"- prism: `{m['prism_sha']}` · oracle: {m['oracle']} · seed: {m['seed']}"
        f" · harness: `{m['harness_sha']}`",
        f"- oracle_error_rate: {m['oracle_error_rate']:.3f} ·"
        f" sut_error_rate: {m['sut_error_rate']:.3f} ·"
        f" baseline_invalid: {m['baseline_invalid']} ·"
        f" oracle_not_quiescent: {m['oracle_not_quiescent']}",
        f"- wall (s): {m['wall_s']}",
        "",
    ]
    for direction, strata in run.get("m2", {}).items():
        lines += [f"## M2 {direction}", "",
                  "| stratum | raw P | raw R | corr P | corr R | tp/fp/fn | pending | shortfall |",
                  "|---|---|---|---|---|---|---|---|"]
        for s, d in strata.items():
            raw, cor = d["raw"], d["corrected"]
            lines.append(
                f"| {s} | {fmt_wilson(raw['precision'])} | {fmt_wilson(raw['recall'])} "
                f"| {fmt_wilson(cor['precision'])} | {fmt_wilson(cor['recall'])} "
                f"| {cor['tp']}/{cor['fp']}/{cor['fn']} | {d['pending']} | {d['shortfall']} |")
        lines.append("")
    for key, title in (("m1", "M1 inventory diff"), ("m3", "M3 spot-check"),
                       ("matrix", "Capability matrix"), ("pending", "Pending triage"),
                       ("pinned", "Pinned probes")):
        if key in run:
            lines += [f"## {title}", "", "```json",
                      json.dumps(run[key], indent=1, sort_keys=True), "```", ""]
    return "\n".join(lines)


def write_reports(run: dict, out_dir: Path) -> None:
    m = run["meta"]
    out_dir.mkdir(parents=True, exist_ok=True)
    base = out_dir / f"{m['date']}-{m['corpus']}"
    base.with_suffix(".json").write_text(json.dumps(run, indent=1, sort_keys=True))
    base.with_suffix(".md").write_text(render_markdown(run))
```

- [ ] **Step 4: Implement `eval/tier_a/cli.py`** — coded, not prose (review B2). The
metrics blocks are a **pure function of the run JSON's stored per-probe raw sites** —
that is the G3 replay property, and `compute_m2_from_probes` is that function.

```python
"""Runner CLI (spec §2.11). Glue over the tested layers. G3 replay property:
metrics blocks are a pure function of the run JSON's `probes` key."""
from __future__ import annotations

import argparse
import dataclasses
import json
import os
import sys
import tomllib
from pathlib import Path

from . import pinned as pinned_mod
from .accounting import CorpusAccounting, evaluate_floors
from .adjudication import apply_verdicts, load_records
from .compare import site_compare
from .corpus import (corpus_dirty, corpus_sha, load_snapshot, save_snapshot,
                     snapshot_path, universe)
from .matrix import run_matrix
from .metrics import precision_recall
from .model import CallEdge, FunctionDef, Location
from .report import write_reports
from .strata import filter_to_universe, inventory_diff, sample_strata
from .sut import PrismCli

EVAL_DIR = Path(__file__).resolve().parents[1]


def _edges(sites: list, direction: str) -> list[CallEdge]:
    """Rebuild minimal CallEdges from stored [file, start, end] site triples."""
    dummy = FunctionDef("_", "function", None, Location("_", 1, 1), 1)
    return [CallEdge(direction, dummy, None, None, Location(f, s, e))
            for f, s, e in sites]


def compute_m2_from_probes(probes: dict, adjudications: list) -> dict:
    """Pure: stored raw sites -> per-direction per-stratum raw+corrected metrics."""
    corpus = probes.get("_corpus", "?")
    out: dict = {}
    for direction in ("callers", "callees"):
        strata: dict = {}
        for pid, p in sorted(probes.items()):
            if pid == "_corpus" or p.get("outcome") != "ok" \
                    or p.get("direction") != direction:
                continue
            r = site_compare(_edges(p["prism_sites"], direction),
                             _edges(p["oracle_sites"], direction))
            s = strata.setdefault(p["stratum"], {"tp": 0, "fp": set(), "fn": set(),
                                                 "seed_of": {}})
            s["tp"] += len(r.tp)
            s["fp"] |= r.fp
            s["fn"] |= r.fn
            for site in r.fp | r.fn:
                s["seed_of"][site] = p["seed_def"]
        out[direction] = {}
        for name, s in sorted(strata.items()):
            raw = precision_recall(s["tp"], len(s["fp"]), len(s["fn"]))
            corr_tp, corr_fp, corr_fn, pending = s["tp"], 0, 0, 0
            by_seed: dict = {}
            for site, sd in s["seed_of"].items():
                grp = by_seed.setdefault(sd, {"fp": set(), "fn": set()})
                (grp["fp"] if site in s["fp"] else grp["fn"]).add(site)
            for sd, grp in sorted(by_seed.items()):
                c = apply_verdicts(tp=0, fp_sites=grp["fp"], fn_sites=grp["fn"],
                                   records=adjudications, corpus=corpus,
                                   measurement=direction, seed_def=sd)
                corr_tp += c.tp
                corr_fp += c.fp
                corr_fn += c.fn
                pending += c.pending
            out[direction][name] = {"raw": raw,
                                    "corrected": precision_recall(corr_tp, corr_fp,
                                                                  corr_fn),
                                    "pending": pending,
                                    "shortfall": 0}
    return out


def recompute_metrics_from_stored(stored: dict) -> dict:
    if "probes" not in stored:
        return stored
    adj = load_records(EVAL_DIR / "adjudications.jsonl")
    return {**stored, "m2": compute_m2_from_probes(stored["probes"], adj)}


def load_corpora() -> dict:
    cfg = tomllib.loads((EVAL_DIR / "corpora.toml").read_text())
    for c in cfg["corpus"].values():
        c["path"] = os.path.expanduser(c["path"])
    return cfg


def make_oracle(cfg: dict):
    from .oracles import LspOracle
    cmd = {"rust-analyzer": ["rust-analyzer"], "gopls": ["gopls", "serve"],
           "pyright": ["pyright-langserver", "--stdio"]}[cfg["oracle"]]
    return LspOracle(cmd, cfg["path"], cfg["lang"])


def run_corpus(name: str, cfg: dict, defaults: dict, args) -> dict:
    sut = PrismCli(str(EVAL_DIR.parent), sut_bin=args.sut_bin,
                   allow_stale=args.allow_stale_sut)
    sha = corpus_sha(cfg["path"])
    run: dict = {"meta": {"corpus": name, "corpus_sha": sha,
                          "corpus_dirty": corpus_dirty(cfg["path"]),
                          "prism_sha": sut.sha, "seed": defaults["seed"],
                          "date": args.date,
                          "harness_sha": corpus_sha(str(EVAL_DIR.parent)),
                          "oracle_not_quiescent": False, "wall_s": {}},
                 "probes": {"_corpus": name}}
    oracle = make_oracle(cfg)
    oracle.start()
    run["meta"]["oracle"] = oracle.version()
    run["meta"]["oracle_not_quiescent"] = oracle.not_quiescent
    if not oracle.capability_probe():
        run["meta"].update(baseline_invalid=True,
                           invalid_reasons=["oracle_unsupported"],
                           oracle_error_rate=1.0, sut_error_rate=0.0)
        return run
    files = universe(cfg["path"], cfg["lang"], cfg.get("excludes", []))
    acc = CorpusAccounting()
    oracle_inv = []
    for f in files:                       # M1: per-file; errors recorded, never fatal
        try:
            oracle_inv.extend(oracle.document_symbols(f))
        except Exception:
            acc.record(f"docsym:{f}", "oracle_error")
    prism_inv = filter_to_universe(sut.inventory(cfg["path"]), set(files))  # review M5
    diff = inventory_diff(oracle_inv, prism_inv)
    run["m1"] = {"matched": len(diff.matched),
                 "prism_missing": len(diff.prism_missing),
                 "prism_extra": len(diff.prism_extra),
                 "anon_oracle": diff.anon_oracle, "anon_prism": diff.anon_prism}
    sp = snapshot_path(str(EVAL_DIR / "snapshots"), name, sha)   # §2.5/G3 snapshot
    if sp.exists():
        snap = load_snapshot(sp)
    else:
        snap = oracle_inv
        save_snapshot(sp, snap)
    n_defs: dict = {}
    for fd in snap:
        if fd.name:
            n_defs[fd.name] = n_defs.get(fd.name, 0) + 1
    per = 3 if args.quick else defaults["per_stratum"]
    sample = sample_strata(snap, n_defs, cfg["lang"], defaults["seed"], per)
    prism_by_oracle = dict(diff.matched)
    strata_counts: dict = {}
    for stratum, fds in sample.items():
        strata_counts[stratum] = {"eligible": len(fds), "successful": 0}
        for fd in fds:
            sd = f"{fd.location.file}:{fd.selection_line}"
            pfd = prism_by_oracle.get(fd)
            for direction in ("callers", "callees"):
                pid = f"{direction}:{sd}"
                try:
                    osites = (oracle.callers(fd) if direction == "callers"
                              else oracle.callees(fd))
                except Exception:
                    acc.record(pid, "oracle_error")
                    continue
                if pfd is None:           # §2.5: inventory_miss -> all-oracle-FN probe
                    acc.record(pid, "inventory_miss")
                    psites = []
                else:
                    try:
                        psites = (sut.callers(cfg["path"], pfd)
                                  if direction == "callers"
                                  else sut.callees(cfg["path"], pfd))
                    except Exception:
                        acc.record(pid, "sut_error")
                        continue
                    acc.record(pid, "ok")
                    strata_counts[stratum]["successful"] += 1
                run["probes"][pid] = {
                    "outcome": "ok", "direction": direction, "stratum": stratum,
                    "seed_def": sd,
                    "prism_sites": [[e.call_site.file, e.call_site.start_line,
                                     e.call_site.end_line] for e in psites
                                    if direction == "callers"
                                    or e.other_def is not None],
                    "oracle_sites": [[e.call_site.file, e.call_site.start_line,
                                      e.call_site.end_line] for e in osites]}
    if name == "prism":
        run["pinned"] = pinned_mod.run_pinned(oracle, sut, snap, cfg["path"])
        run["matrix"] = [dataclasses.asdict(r) for r in
                         run_matrix(EVAL_DIR / "fixtures", sut,
                                    ["rust", "go", "python"])]
    run["meta"]["oracle_error_rate"] = acc.oracle_error_rate()
    run["meta"]["sut_error_rate"] = acc.sut_error_rate()
    ok, reasons = evaluate_floors(strata_counts, acc.oracle_error_rate(),
                                  acc.sut_error_rate(),
                                  defaults["oracle_error_floor"][cfg["lang"]],
                                  defaults["sut_error_floor"])
    run["meta"]["baseline_invalid"] = not ok
    run["meta"]["invalid_reasons"] = reasons
    adj = load_records(EVAL_DIR / "adjudications.jsonl")
    run["m2"] = compute_m2_from_probes(run["probes"], adj)
    return run


def main() -> int:
    ap = argparse.ArgumentParser(prog="tier-a")
    ap.add_argument("--corpus", default="prism")
    ap.add_argument("--quick", action="store_true")
    ap.add_argument("--matrix-only", action="store_true")
    ap.add_argument("--report-only")
    ap.add_argument("--sut-bin")
    ap.add_argument("--allow-stale-sut", action="store_true")
    ap.add_argument("--date", default=None)
    args = ap.parse_args()
    if args.date is None:
        import datetime
        args.date = datetime.date.today().isoformat()
    out_dir = EVAL_DIR.parent / "docs" / "eval" / "tier-a"
    if args.report_only:
        write_reports(recompute_metrics_from_stored(
            json.loads(Path(args.report_only).read_text())), out_dir)
        return 0
    if args.matrix_only:
        sut = PrismCli(str(EVAL_DIR.parent), sut_bin=args.sut_bin,
                       allow_stale=args.allow_stale_sut)
        results = run_matrix(EVAL_DIR / "fixtures", sut, ["rust", "go", "python"])
        for r in results:
            print(f"{r.language}/{r.capability}: {r.outcome}")
        return 1 if any(r.outcome == "regression" for r in results) else 0
    cfg = load_corpora()
    names = ["prism"] if args.quick else (
        list(cfg["corpus"]) if args.corpus == "all" else [args.corpus])
    rc = 0
    for name in names:
        run = run_corpus(name, cfg["corpus"][name], cfg["defaults"], args)
        (EVAL_DIR / "runs").mkdir(exist_ok=True)
        (EVAL_DIR / "runs" / f"{args.date}-{name}.json").write_text(
            json.dumps(run, indent=1, sort_keys=True, default=str))
        write_reports(run, out_dir)
        if run["meta"].get("baseline_invalid"):
            rc = 2
    return rc


if __name__ == "__main__":
    sys.exit(main())
```

(`pinned.run_pinned` is a thin driver added to `pinned.py` in this task: resolve each
pinned entry against the snapshot by `(symbol, file)`, run the oracle+SUT pair like an
M2 probe — except probe #4, which calls `sut.callers_by_symbol("slice")` and records
whether `SutAmbiguous` was raised — and return per-probe outcomes including the G1(b)
`known_fail`-vs-`flip_candidate` evaluation and the G2 `oracle_miss_site` rediscovery
booleans.)

The replay tests — **with a real `probes` fixture**, not only the identity branch
(append to `eval/tests/test_report.py`):

```python
def test_report_only_replay_recomputes_metrics_from_probes():
    import json
    from tier_a.cli import compute_m2_from_probes, recompute_metrics_from_stored
    probes = {
        "_corpus": "prism",
        "callers:src/s.rs:5": {
            "outcome": "ok", "direction": "callers", "stratum": "U-free",
            "seed_def": "src/s.rs:5",
            "prism_sites": [["src/a.rs", 10, 10], ["src/b.rs", 99, 99]],
            "oracle_sites": [["src/a.rs", 9, 11]]},
    }
    stored = {"meta": {"corpus": "prism"}, "probes": probes,
              "m2": compute_m2_from_probes(probes, [])}
    roundtrip = json.loads(json.dumps(stored, default=str))
    again = recompute_metrics_from_stored(roundtrip)
    m = again["m2"]["callers"]["U-free"]
    assert m["raw"]["tp"] == 1        # a.rs:10 falls in oracle range 9..11
    assert m["raw"]["fp"] == 1        # b.rs:99 is prism-only
    assert m["pending"] == 1          # the unadjudicated FP
    assert recompute_metrics_from_stored(roundtrip)["m2"] == again["m2"]  # stable


def test_report_only_without_probes_is_identity():
    from tier_a.cli import recompute_metrics_from_stored
    stored = {"meta": {"corpus": "x"}}
    assert recompute_metrics_from_stored(stored) == stored
```

- [ ] **Step 5: Complete `eval/README.md`** — sections: the separation-contract banner
(already present), oracle install (`rust-analyzer` on PATH;
`go install golang.org/x/tools/gopls@latest`; `npm i -g pyright`), corpus prep
(bench-repos copies/clones + SHAs + venv notes), SUT build
(`cargo build --release`) + discovery order (`--sut-bin` > `PRISM_BIN` >
`target/release/prism`), the five runner invocations from spec §2.11, adjudication
workflow (how to triage pending diffs into `adjudications.jsonl`, the §2.8 budget),
and the snapshot/baseline model (`eval/snapshots/`, `docs/eval/tier-a/`,
`baseline.md` updated only deliberately).

- [ ] **Step 6: Agent guidance (G8).** Append to `CLAUDE.md` (new section after
"Build & Test") and create `AGENTS.md` containing the same text:

````markdown
## Accuracy Harness (Tier-A)

When a change touches call resolution, navigation queries, or CPG construction
(`src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`):

```bash
cd eval && uv run tier-a --matrix-only   # seconds, no LSP — run before committing
cd eval && uv run tier-a --quick         # minutes, needs rust-analyzer — before review
```

Paste regressions/flip-candidates into the PR description rather than re-baselining.
Full multi-corpus runs (`uv run tier-a --corpus all`) are human-triggered; see
`eval/README.md`. The committed baseline lives in `docs/eval/tier-a/`.
````

- [ ] **Step 7: Run all eval tests, commit**

```bash
cd eval && uv run pytest -q
git add eval CLAUDE.md AGENTS.md
git commit -m "feat(eval): runner CLI, reports, corpora config, README + agent guidance (G8)"
```

### Task 21: [ORCHESTRATOR] Live baseline — corpora prep, probes, run, adjudicate, gates

- [ ] **Step 1: Prep corpora + oracles (one-time)**

```bash
go install golang.org/x/tools/gopls@latest
npm i -g pyright
# tokio: already at ~/code/bench-repos/tokio on this host, but do not depend on the
# accident (review B4) — clone if absent:
[ -d ~/code/bench-repos/tokio ] || git clone --depth 50 https://github.com/tokio-rs/tokio ~/code/bench-repos/tokio
cp -R ~/code/agent-eval/cache/repos/caddy ~/code/bench-repos/caddy
git clone --depth 50 https://github.com/pallets/flask ~/code/bench-repos/flask
git clone --depth 50 https://github.com/pallets/click ~/code/bench-repos/click
python3 -m venv ~/code/bench-repos/flask/.venv
~/code/bench-repos/flask/.venv/bin/pip install -e ~/code/bench-repos/flask
python3 -m venv ~/code/bench-repos/click/.venv
~/code/bench-repos/click/.venv/bin/pip install -e ~/code/bench-repos/click
cargo build --release
```

(pyright resolves imports against each corpus's populated venv — add a
`pyrightconfig.json` with `venvPath`/`venv` in the corpus root if auto-detection
misses; record whatever was needed in `eval/README.md`.)

Record **every** corpus's `git rev-parse --short=12 HEAD` into `eval/corpora.toml`
(`pinned_sha`) — including prism and tokio.

- [ ] **Step 2: pyright capability probe (spec §2.2 plan precondition)**

`cd eval && uv run tier-a --corpus flask` — if the probe reports
`oracle_unsupported`: install `basedpyright` and retry; if that also fails, switch the
Python oracle to the references-based fallback (spec §2.2) **before** scoring any
Python corpus, and record the decision in the run report and `eval/README.md`.

- [ ] **Step 3: Full baseline run**

```bash
cd eval && uv run tier-a --corpus all
```

Watch for: quiescence on tokio/caddy (first index is the slow one), floors
(`oracle_error_rate`, `sut_error_rate`), pinned-probe outcomes.

- [ ] **Step 4: Adjudicate with the owner (spec §2.8 budget)**

All Rust/Go pending diffs + ≤25 sampled pending diffs per Python corpus → append
records to `eval/adjudications.jsonl`, **including the two seeded feature-gated
records** with the real `seed_def` selection lines from this run
(`module_deps` → site `src/mcp/tools.rs:162`; `load_repo` → site
`src/mcp/session.rs:28`). Re-run `--report-only` on the stored runs to fold verdicts in.

- [ ] **Step 5: Evaluate gates and commit the baseline**

G1 (bimodality via U-strata corrected ≥0.95 + pinned `target` outcome), G2 (both
oracle-miss diffs rediscovered), G3 (snapshot determinism + replay), G4 (floors,
5 corpora), G5 (matrix statuses). Then:

```bash
git add eval/snapshots eval/adjudications.jsonl eval/corpora.toml docs/eval/tier-a/
git commit -m "feat(eval): Tier-A baseline — 5 corpora, adjudicated, gates G1-G5 evaluated"
```

Write `docs/eval/tier-a/baseline.md` (per-language summary, matrix grid, pending
counts, floor status) in the same commit. If any gate fails: record honestly in the
report + `docs/archive/plans/prism-query-layer/s1-followups.md`-style open-evidence note; do not waive (S1 row-C precedent).

- [ ] **Step 6: Update persistent memory** (Tier-A live → B2 trigger armed; S3 next,
measured against this baseline).

---

## Plan self-review notes (resolved inline)

- WP2 batch arithmetic verified: 48+37+32+4 = 121 absorbed; final count 24.
- Type consistency: `FunctionDef`/`CallEdge`/`Location`/`DefTarget` signatures match
  across Tasks 10–20; `PrismCli(prism_repo, sut_bin, allow_stale)` consistent between
  Tasks 13/18/20.
- rev 2 folds the merged plan-review
  (`docs/archive/review-artifacts/prism-query-layer/tier-a-plan-review-2026-06-11.md`): consolidation script
  committed under `scripts/`; cli.py fully coded with a probes-fixture replay test
  (B2) + new Task 19 accounting/floors layer; OracleError + lifecycle wrappers +
  `version()` on both seams (B3); tokio prep (B4); §2.4 universe filter applied to
  prism inventory (M5); early host-side pyright probe after Task 12 (M6);
  rootUri/workspaceFolders in initialize (M7); `mkdir -p docs/eval` (M8); real
  backtrace probe for the profile change (M9); ambiguous/alias_site verdict tests
  (m10); live skipif matrix test (m11); `callers_by_symbol` + `SutAmbiguous` (m12);
  repo-wide sweep incl. `.claude/settings.local.json` (m13); `interfaces.py` (m14);
  Task 18 Step 5 execution-model note (m15); spec §2.11 corpus-count fix (m16).
- Fixture line numbers were hand-counted; Task 18 Step 5's reconciliation run is the
  authoritative check (the binary is the referee for `status`, the source files for
  lines — if a line is off by one, fix `expected.toml` to match the source, which is
  the ground truth by construction).
- Deliberate scope notes: `cli.py` run-flow is specified as ordered integration of
  fully-coded, fully-tested parts rather than inlined (it is glue; its replay property
  is pinned by test). Oracle lifecycle (quiescence/probe) is live-only by design —
  exercised in Task 21, failure modes accounted per §2.2.
