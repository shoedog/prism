[![CI](https://github.com/shoedog/prism/actions/workflows/ci.yml/badge.svg)](https://github.com/shoedog/prism/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/shoedog/prism/graph/badge.svg?token=C5JSSOQPWA)](https://codecov.io/github/shoedog/prism)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

<!-- COVERAGE_BADGES_START -->
**Language feature coverage** · [details](#language-feature-coverage)

![Python](https://img.shields.io/badge/Python-88%25-green?logo=python&logoColor=white)
![JavaScript](https://img.shields.io/badge/JavaScript-94%25-green?logo=javascript&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-94%25-green?logo=typescript&logoColor=white)
![Go](https://img.shields.io/badge/Go-93%25-green?logo=go&logoColor=white)
![Java](https://img.shields.io/badge/Java-100%25-brightgreen?logo=openjdk&logoColor=white)
![C](https://img.shields.io/badge/C-100%25-brightgreen?logo=c&logoColor=white)
![C++](https://img.shields.io/badge/C%2B%2B-100%25-brightgreen?logo=cplusplus&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-91%25-green?logo=rust&logoColor=white)
![Lua](https://img.shields.io/badge/Lua-100%25-brightgreen?logo=lua&logoColor=white)
![Terraform](https://img.shields.io/badge/Terraform-100%25-brightgreen?logo=terraform&logoColor=white)
![Bash](https://img.shields.io/badge/Bash-100%25-brightgreen?logo=gnubash&logoColor=white)

**Algorithm test coverage** · [details](#algorithm--language)

![Python algo](https://img.shields.io/badge/Python-93%25-green?logo=python&logoColor=white)
![JavaScript algo](https://img.shields.io/badge/JavaScript-93%25-green?logo=javascript&logoColor=white)
![TypeScript algo](https://img.shields.io/badge/TypeScript-90%25-green?logo=typescript&logoColor=white)
![Go algo](https://img.shields.io/badge/Go-93%25-green?logo=go&logoColor=white)
![Java algo](https://img.shields.io/badge/Java-90%25-green?logo=openjdk&logoColor=white)
![C algo](https://img.shields.io/badge/C-100%25-brightgreen?logo=c&logoColor=white)
![C++ algo](https://img.shields.io/badge/C%2B%2B-96%25-brightgreen?logo=cplusplus&logoColor=white)
![Rust algo](https://img.shields.io/badge/Rust-90%25-green?logo=rust&logoColor=white)
![Lua algo](https://img.shields.io/badge/Lua-90%25-green?logo=lua&logoColor=white)
![Terraform algo](https://img.shields.io/badge/Terraform-90%25-green?logo=terraform&logoColor=white)
![Bash algo](https://img.shields.io/badge/Bash-90%25-green?logo=gnubash&logoColor=white)
<!-- COVERAGE_BADGES_END -->

# Prism

Prism is a Code Property Graph (CPG) engine for diff-aware program slicing.
It builds a unified graph — call graph, data flow graph, and control flow
graph — from source code using [tree-sitter](https://tree-sitter.github.io/)
and [petgraph](https://docs.rs/petgraph/), then runs 30 slicing algorithms
against diffs to extract exactly the context a code reviewer needs.

**The problem:** Raw diffs show *what* changed but not *why it matters*.
A 5-line change to a validation function might affect 40 callers across 12
files. A new parameter might break a taint path from user input to a SQL
query. Prism answers these questions statically and fast — typically under
3 seconds for a 50-file repository.

**How it works:**

1. Parse all source files into ASTs (tree-sitter — 11 languages, 12 grammar variants; TSX parsed separately from TypeScript)
2. Build a unified CPG: call graph + data flow graph + control flow graph
3. Enrich with type information (per-language providers, RTA for dispatch)
4. Accept a diff (unified patch or JSON)
5. Run selected algorithms against the diff, querying the CPG
6. Output focused slices: the exact code context relevant to each change

**Use cases:**
- Power automated code review agents with structured program analysis
- Pre-review triage: identify which changes are high-risk before human review
- Security analysis: trace taint paths from untrusted input to sensitive sinks
- Firmware/embedded review: detect missing resource cleanup, absent error handling
- Refactoring safety: verify that signature changes don't break callers

Implements 30 slicing algorithms spanning the paper
[Towards Practical Defect-Focused Automated Code Review](https://arxiv.org/abs/2505.17928),
the established program slicing taxonomy, and several novel theoretical
extensions including spiral, quantum, horizontal, vertical, angle, and 3D slices.

Supports **Python**, **JavaScript**, **TypeScript**, **Go**, **Java**, **C**, **C++**, **Rust**, **Lua**, **Terraform/HCL**, and **Bash**.

---

## Install

Requires Rust 1.70+.

```bash
git clone <repo-url> && cd prism
cargo build --release
```

The binary lands at `target/release/prism`. Copy it somewhere on your `$PATH`
or run it directly.

---

## Quick start

```bash
# Generate a diff from any repo
cd /path/to/your/project
git diff HEAD~1 > /tmp/changes.patch

# Slice it
prism --repo . --diff /tmp/changes.patch
```

That's it. The default algorithm (`leftflow`) traces data flow backward from
each changed line and prints the relevant slice to stdout.

List all 30 algorithms:

```bash
prism --list-algorithms
```

---

## MCP server — whole-repo navigation for coding agents

Beyond the diff-slicing CLI, Prism ships **`prism-mcp`**, a local **stdio MCP server** that gives
MCP-capable coding agents (Claude Code, Codex, Kiro, …) read-only, whole-repo code navigation — who
calls a symbol, what it calls, what breaks if you change it, the module dependency graph.

```bash
cargo build --release --bin prism-mcp --features mcp          # build (behind the `mcp` feature)
```

**Claude Code (plugin — recommended):** installs the skills *and* wires the MCP to your current project:

```text
/plugin marketplace add shoedog/prism
/plugin install prism@prism-dev
```

**Or wire it manually / on Codex/Kiro:**

```bash
claude mcp add --transport stdio prism \
  -- /abs/path/to/prism/target/release/prism-mcp --repo /abs/path/to/your/repo
```

**See [`docs/MCP.md`](docs/MCP.md)** for the full guide: the plugin, Codex/Kiro config, all nine
tools, cache warming, the gotchas, and the bundled **skills** ([`skills/`](skills/)) that teach an agent
*how* to use the connection. One server instance navigates one repo.

`prism-mcp` registers **nine tools**: seven read-only navigation tools (`nav_nodes_at`,
`nav_symbol_spans`, `nav_callers`, `nav_callees`, `nav_ego_graph`, `nav_module_deps`,
`nav_repo_map`), one read-only reasoning tool (`taint_reaches`), and one non-destructive local-state
tool (`refresh_index`).

### `prism nav` — the same navigation, no MCP client needed

The seven `nav_*` queries are also a direct, first-class CLI subcommand — useful for scripting or when
there's no MCP client in the loop:

```bash
prism nav callers --repo . --symbol handle_request
prism nav symbol-spans --repo . --symbol handle_request --file src/server.rs --format json
prism nav repo-map --repo . --format json
prism nav onboard --repo . --out prism-project-overview.md
```

Twelve subcommands in total: `nodes-at`, `symbol-spans`, `callers`, `callees`, `ego`, `module-deps`,
`repo-map`, `onboard`, `call-stats`, `interface-manifest`, `functions`, `taint-reaches`.
`--no-cache`/`--cache-dir` on `nav` gate the whole-repo navigation cache — a separate cache from the
diff-review CPG cache (see [CPG Caching](#cpg-caching)).

`onboard` packages one cached navigation build into a bounded project-orientation report: inventory,
language counts, module connectivity, call-resolution totals, warnings, and suggested follow-up queries.
Markdown is the default and JSON is available with `--format json`. It writes to stdout unless `--out`
names a new file; an existing output target is refused and never overwritten.

---

## All 30 algorithms at a glance

For a per-algorithm operator's guide — what each one answers, when its output is meaningful, exact finding categories and severities, and known limitations — see [`ALGORITHMS.md`](ALGORITHMS.md). The table below is the cheat sheet.

### Paper algorithms (arXiv:2505.17928)

| Algorithm | Flag | What it includes |
|---|---|---|
| **OriginalDiff** | `-a originaldiff` | Only the changed lines |
| **ParentFunction** | `-a parentfunction` | Entire enclosing function |
| **LeftFlow** | `-a leftflow` | Backward data-flow from assignments (default) |
| **FullFlow** | `-a fullflow` | LeftFlow + forward R-value tracing |

### Established taxonomy

| Algorithm | Flag | What it does |
|---|---|---|
| **ThinSlice** | `-a thin` | Data deps only — no control flow, no returns. Most focused |
| **BarrierSlice** | `-a barrier` | Interprocedural with depth limits and barriers |
| **Chop** | `-a chop` | All data-flow paths between a source and sink |
| **Taint** | `-a taint` | Forward propagation of untrusted values to sinks |
| **RelevantSlice** | `-a relevant` | LeftFlow + alternate branch paths ("one flip from a bug") |
| **ConditionedSlice** | `-a conditioned` | LeftFlow pruned by a value assumption |
| **DeltaSlice** | `-a delta` | Behavioral diff between two program versions |

### Theoretical extensions

| Algorithm | Flag | What it does |
|---|---|---|
| **SpiralSlice** | `-a spiral` | Adaptive-depth through concentric rings (1-6) |
| **CircularSlice** | `-a circular` | Detects data-flow cycles across function boundaries |
| **QuantumSlice** | `-a quantum` | Enumerates concurrent states around async boundaries |
| **HorizontalSlice** | `-a horizontal` | Finds peer constructs that should follow the same pattern |
| **VerticalSlice** | `-a vertical` | End-to-end feature path across architectural layers |
| **AngleSlice** | `-a angle` | Cross-cutting concern trace (errors, logging, auth) |
| **3DSlice** | `-a 3d` | Risk scoring: structural coupling * git churn * change size |

### Novel extensions

| Algorithm | Flag | What it does |
|---|---|---|
| **AbsenceSlice** | `-a absence` | Detects missing counterparts: open without close, lock without unlock |
| **ResonanceSlice** | `-a resonance` | Flags files that usually co-change in git but are missing from the diff |
| **SymmetrySlice** | `-a symmetry` | Detects broken symmetric pairs: serialize/deserialize, encode/decode |
| **GradientSlice** | `-a gradient` | Continuous relevance scoring (decaying) instead of binary include/exclude |
| **ProvenanceSlice** | `-a provenance` | Traces data origin (user_input, config, database, env_var, etc.) |
| **PhantomSlice** | `-a phantom` | Surfaces recently deleted code this change might depend on |
| **MembraneSlice** | `-a membrane` | Shows cross-file callers of changed API functions |
| **EchoSlice** | `-a echo` | Ripple effect: flags callers missing error handling or null checks |
| **ContractSlice** | `-a contract` | Extracts implicit behavioral contracts and flags violations |
| **PeerConsistencySlice** | `-a peer` | Sibling-function NULL-guard divergence (C/C++ clusters sharing a first-parameter name) |
| **CallbackDispatcherSlice** | `-a callback` | Resolves function-pointer-in-struct registrations to their dispatch sites; flags NULL-argument dispatches |
| **PrimitiveSlice** | `-a primitive` | Deterministic security-primitive fingerprints: hash truncation (incl. two-pass via call), weak-hash-for-identity, shell-injection, TLS-disabled, hardcoded secrets |

---

## Usage by language

### Python

```bash
cd ~/projects/my-python-app
git diff main > /tmp/diff.patch

prism --repo . --diff /tmp/diff.patch --algorithm leftflow
```

Recognized extensions: `.py`

Handles `def` functions, decorated functions (`@decorator`), assignments,
augmented assignments (`+=`), `if`/`for`/`while` conditions, and `return`
statements.

**Example** — you changed line 12 (`total = x + y`):

```
# Block 0 [M] src/calc.py
    6|def calculate(x, y):
+  12|    total = x + y
   14|    if total > 10:
   15|        result = total * 2
   19|    return result
```

The slicer traced `total` into the `if` condition and `result`.

**Python-specific algorithms worth trying:**

```bash
# Thin slice: just the data chain, no control flow noise
prism --repo . --diff /tmp/diff.patch -a thin

# Taint: where do diff-line values end up?
prism --repo . --diff /tmp/diff.patch -a taint

# Horizontal: find all handler functions that should match the changed one
prism --repo . --diff /tmp/diff.patch -a horizontal

# Angle: trace error handling across the codebase
prism --repo . --diff /tmp/diff.patch -a angle --concern error_handling
```

---

### JavaScript

```bash
cd ~/projects/my-js-app
git diff HEAD~3 > /tmp/diff.patch

prism --repo . --diff /tmp/diff.patch
```

Recognized extensions: `.js`, `.mjs`, `.cjs`, `.jsx`

Handles `function` declarations, arrow functions (`=>`), method definitions,
generator functions, `const`/`let`/`var` declarations, and all standard control
flow.

**JS-specific algorithms worth trying:**

```bash
# Quantum: find async state races around await boundaries
prism --repo . --diff /tmp/diff.patch -a quantum --quantum-var response

# Circular: detect event handler or state management cycles
prism --repo . --diff /tmp/diff.patch -a circular

# Relevant: see alternate branches ("what if this condition was false?")
prism --repo . --diff /tmp/diff.patch -a relevant
```

---

### TypeScript

```bash
cd ~/projects/my-ts-app
git diff feature-branch > /tmp/diff.patch

prism --repo . --diff /tmp/diff.patch -a fullflow
```

Recognized extensions: `.ts`, `.tsx`

Same capabilities as JavaScript. Types are parsed but slicing focuses on
value-level data flow.

**TS-specific algorithms worth trying:**

```bash
# Barrier: trace callers/callees up to 3 levels, stop at framework internals
prism --repo . --diff /tmp/diff.patch -a barrier --barrier-depth 3 --barrier-symbols "React.createElement,useEffect"

# Vertical: see the full request path from handler to database
prism --repo . --diff /tmp/diff.patch -a vertical --layers "routes,services,models,db"
```

---

### Go

```bash
cd ~/projects/my-go-service
git diff HEAD~1 > /tmp/diff.patch

prism --repo . --diff /tmp/diff.patch
```

Recognized extensions: `.go`

Handles `func` declarations, method declarations (with receivers),
`:=` short variable declarations, `for`/`range` loops, `if`/`switch`
statements, and `return` statements.

**Go-specific algorithms worth trying:**

```bash
# Quantum: detect goroutine races
prism --repo . --diff /tmp/diff.patch -a quantum

# Chop: is there a data path from user input to this SQL query?
prism --repo . --diff /tmp/diff.patch -a chop --chop-source "handlers/api.go:42" --chop-sink "db/query.go:88"

# 3D: which functions have the most risk (high coupling + high churn)?
prism --repo . --diff /tmp/diff.patch -a 3d --temporal-days 30
```

---

### Java

```bash
cd ~/projects/my-java-project
git diff develop > /tmp/diff.patch

prism --repo . --diff /tmp/diff.patch -a parentfunction
```

Recognized extensions: `.java`

Handles method declarations, constructor declarations,
`local_variable_declaration`, field declarations, enhanced for loops, try
statements, and standard control flow.

**Java-specific algorithms worth trying:**

```bash
# Spiral: start narrow and widen progressively
prism --repo . --diff /tmp/diff.patch -a spiral --spiral-max-ring 5

# Conditioned: "what does the code do when this value is null?"
prism --repo . --diff /tmp/diff.patch -a conditioned --condition "user!=null"

# Angle: trace authentication handling across layers
prism --repo . --diff /tmp/diff.patch -a angle --concern auth

# Delta: what data-flow paths changed vs the previous version?
prism --repo . --diff /tmp/diff.patch -a delta --old-repo /path/to/old/version
```

---

## Output formats

`--format`/`-f` selects the output shape. Seven values:

| Format | Flag value | Description |
|---|---|---|
| Text (default) | `text` | Human-readable, line-numbered output with `+` marking changed lines and `...` for gaps |
| JSON | `json` | Full `SliceResult`/`MultiReviewOutput` — algorithm name, blocks, line maps, diff metadata |
| Paper | `paper` | Matches the `diff_outputs.json` format from the original paper |
| Review | `review` | Compact, findings-first JSON for code-review agents (severity floor + block retention; see `--review-*` flags) |
| Callers | `callers` | Raw call graph for the diff's changed functions — no algorithm runs |
| Mermaid | `mermaid` | Mermaid flowchart diagrams of the slice graph |
| SARIF | `sarif` | [SARIF](https://sarifweb.azurewebsites.net/) 2.1 — upload as GitHub code-scanning annotations |

```bash
prism --repo . --diff changes.patch --format text
prism --repo . --diff changes.patch --format json
prism --repo . --diff changes.patch --format paper
prism --repo . --diff changes.patch --algorithm review --format review
prism --repo . --diff changes.patch --format callers
prism --repo . --diff changes.patch --format mermaid
```

### SARIF and GitHub code scanning

`--format sarif` maps findings to [SARIF 2.1](https://sarifweb.azurewebsites.net/) results —
one rule per `<algorithm>/<category>` pair, severity/confidence/tier/parse-quality carried as
result `properties`. Byte-identical for the same input; safe to diff across runs.

```bash
prism --repo . --diff change.patch --algorithm review --format sarif > prism.sarif
```

Upload it as a GitHub code-scanning annotation from a workflow:

```yaml
- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: prism.sarif
```

---

## Diff input formats

### Unified diff (from git)

```bash
git diff > changes.patch
git diff HEAD~5 HEAD -- src/ > changes.patch
git show abc123 --format="" > changes.patch
```

### JSON

```json
{
  "files": [
    {
      "file_path": "src/handler.py",
      "modify_type": "Modified",
      "diff_lines": [42, 43, 44, 78]
    }
  ]
}
```

---

## Options reference

### Universal flags

| Flag | Default | Description |
|---|---|---|
| `--repo`, `-r` | (required) | Path to the repository root |
| `--diff`, `-d` | (required) | Path to unified diff or JSON diff file |
| `--algorithm`, `-a` | `leftflow` | Algorithm name (see `--list-algorithms`) |
| `--format`, `-f` | `text` | `text`, `json`, `paper`, `review`, `callers`, `mermaid`, `sarif` |
| `--list-algorithms` | | Print all algorithms and exit |
| `--max-branch-lines` | `5` | Max lines in a branch before summarizing |
| `--no-returns` | | Skip return statements in leftflow/fullflow |
| `--no-trace-callees` | | Skip callee bodies in fullflow |
| `--files a,b` | (all diff files) | Only process these files from the diff |
| `--scoped-cpg` | off | Build the CPG from only diff-changed files + direct callers/callees |
| `--compile-commands path` | | `compile_commands.json` for C/C++ type enrichment |
| `--cache-dir path` | | Cache the CPG to this directory (see [CPG Caching](#cpg-caching)) |
| `--no-cache` | off | Ignore any existing CPG cache and force a full rebuild |
| `--caller-depth N` | `5` | Max traversal depth for `--format callers` |
| `--diagram-node-cap N` | `40` | Max nodes per Mermaid diagram before truncation (must be >= 4) |
| `--strict-diagrams` | off | Exit non-zero if any bug-class diagram warning is produced |
| `--review-min-severity` | `warning` | `--format review` only: severity floor (`info`, `suggestion`, `warning`, `concern`) |
| `--review-full-slices` | off | `--format review` only: keep every block, not just ones with a retained finding |
| `--review-no-diagrams` | off | `--format review` only: omit diagram payloads |
| `--python-version`, `--go-version`, `--node-version`, `--typescript-version`, `--java-version`, `--rust-version` | | Target language version (stored, informational) |

### Algorithm-specific flags

| Flag | Algorithm | Description |
|---|---|---|
| `--barrier-depth N` | barrier | Max call depth (default: 2) |
| `--barrier-symbols a,b` | barrier | Functions to stop at |
| `--chop-source file:line` | chop | Source location |
| `--chop-sink file:line` | chop | Sink location |
| `--taint-source file:line` | taint | Explicit taint source (repeatable) |
| `--taint-return-flow` | taint | Follow singleton-Exact callee return values to caller LHSs |
| `--condition "var==val"` | conditioned | Value assumption predicate |
| `--old-repo path` | delta | Path to old version of repo |
| `--spiral-max-ring N` | spiral | Maximum ring level 1-6 (default: 4) |
| `--quantum-var name` | quantum | Target variable to analyze |
| `--peer-pattern pat` | horizontal | `decorator:@X`, `name:prefix*`, `class:Name` |
| `--layers a,b,c` | vertical | Explicit layer names (highest to lowest) |
| `--concern name` | angle | `error_handling`, `logging`, `auth`, `caching`, or custom keywords |
| `--temporal-days N` | 3d | Git history window in days (default: 90) |
| | absence | No additional flags |
| | resonance | No additional flags (requires git history) |
| | symmetry | No additional flags |
| | gradient | No additional flags |
| | provenance | No additional flags |
| | phantom | No additional flags (requires git history) |
| | membrane | No additional flags |
| | echo | No additional flags |

---

## Piping into other tools

```bash
# Feed into an LLM for review
prism --repo . --diff changes.patch | pbcopy
prism --repo . --diff changes.patch | llm review

# Save JSON for processing
prism --repo . --diff changes.patch -f json > slice.json

# Filter by language
git diff main -- '*.py' > /tmp/py-only.patch
prism --repo . --diff /tmp/py-only.patch

# Compare algorithms
for algo in thin leftflow fullflow relevant; do
  echo "=== $algo ==="
  prism --repo . --diff changes.patch -a $algo | wc -l
done
```

---

## `prism targets`

`prism targets` projects prism's findings into a stable, closed-schema JSON document of
**instrumentation sites** — for a runtime harness that wants to fault-inject, watch, or verify
specific call/resource/contract sites a diff touched, rather than parse prism's native finding
shape itself.

```bash
# Default five finding-producing algorithms (echo, absence, contract, provenance, membrane)
prism targets --repo . --diff change.patch

# Explicit algorithm subset, filtered, strict, written to a file
prism targets --repo . --diff change.patch --algorithm echo,absence --min-severity info --min-tier candidate --strict --out targets.json
```

**Acceptance table** (evaluated before the repo is even read):

| `--algorithm` | Result |
|---|---|
| `echo`, `absence`, `contract`, `provenance`, `membrane`, `taint`, `symmetry`, `peer_consistency`, `callback_dispatcher`, `primitive` | Accepted — projected into targets |
| `angle`, `delta` | Accepted, but produce no findings at this version (`delta` also requires `--old-repo`, else exit 1) |
| `chop`, `conditioned` | Rejected (exit 1) — these need `--chop-source`/`--chop-sink`/`--condition`; use the top-level `prism` command instead |
| anything else (`review`, `all`, other slice-only algorithms) | Rejected (exit 1) — "produces slice blocks, not findings" |

`--strict` exits **3** when one or more requested algorithms failed (non-empty `errors[]`);
without it, failures are still recorded in `errors[]` but the exit code stays 0. `--min-severity`
and `--min-tier` filter the emitted targets; `--out <file>` writes the document to a file instead
of stdout.

The document is validated against
[`docs/contracts/targets.schema.json`](docs/contracts/targets.schema.json) (JSON Schema Draft
2020-12; `schema_version` is the const `"1.0"`). Excerpt (fields elided — see the schema for the
full required-field list per object):

```json
{
  "schema_version": "1.0",
  "producer": { "tool": "prism", "algorithms": ["echo", "absence", "contract", "provenance", "membrane"] },
  "targets": [{ "kind": "resource_acquire", "source_algorithm": "absence", "severity": "warning" }]
}
```

---

## Library use (`prism::api`)

Beyond the CLI, `prism::api` is a stable Rust facade for embedding prism in another tool (an
analyzer, a CI check, a runtime harness) without depending on prism's internal module graph.

> Within a major version, every item of `prism::api` keeps its name and signature; a removal or signature change is preceded by a `#[deprecated]` release. Every struct and enum reachable through `prism::api` — defined in `api`, `finding_confidence`, `targets`, or `output::sarif` — is `#[non_exhaustive]`: construct with `new`/`Default`/builders and assign public fields, never with a struct literal or exhaustive `match` (`SarifInputs` has a builder; `TargetsMeta` is `Default` + field assignment; `TargetsDocument` and its nested types are produced by `project` and read/deserialized by consumers). Types from other modules that appear in `prism::api` signatures (`ParsedFile`, `TypeDatabase`, `CpgContext`, `SliceConfig`, `SlicingAlgorithm`, `SliceResult`, `SliceFinding`, `NavigationSession`, `Evidence`, `QueryError`, `DiffInput`, `Language`, `LanguageVersion`) are **stable as handles**: you may obtain them from `prism::api`, pass them back into `prism::api`, and read the fields the `prism::api` docs name; their other fields and methods are internal and may change. Everything else in the crate is internal. Output formats are versioned by their own fields: multi-run `json`/`review` carry `version: "1.0"`; single-run `json`/`review` shapes are unversioned and pinned by tests; SARIF carries `properties.mapping_version`; targets carries `schema_version`.

This example is also a running doc-test on `prism::api::review` (`cargo test --doc api`) — the
sample below is verified on every test run, not just written prose:

```rust
use prism::api::{review, AlgorithmParams, ReviewOptions};
use prism::slice::{SliceConfig, SlicingAlgorithm};
use std::fs;
use tempfile::TempDir;

// `TempDir` removes its directory on drop — including on an assertion panic below — so
// nothing leaks even if this doc-test fails.
let repo = TempDir::new()?;
fs::write(repo.path().join("a.py"), "def read():\n    f = open(\"x\")\n    return f\n")?;

let diff_json = r#"{"files":[{"file_path":"a.py","modify_type":"Modified","diff_lines":[2]}]}"#;
let outcome = review(
    &ReviewOptions::new(repo.path()),
    diff_json,
    &[SlicingAlgorithm::AbsenceSlice],
    &SliceConfig::default(),
    &AlgorithmParams::default(),
)?;

for f in &outcome.run.findings {
    println!("{}:{} {} {}", f.file, f.line, f.algorithm, f.description);
}
assert!(!outcome.run.findings.is_empty());
assert_eq!(
    outcome.run.findings[0].category.as_deref(),
    Some("missing_counterpart")
);
```

`review()` is the one-shot entry point; `load_review_inputs`/`build_context`/`run_review`/
`run_algorithm` expose the same pipeline in two phases for callers that want to build once and
run many algorithms. `nav_session`/`callers`/`callees` expose whole-repo navigation the same way
`prism nav` does. See the [`prism::api` module docs](src/api/mod.rs) for the full surface.

---

## Architecture

```
Source files ──→ tree-sitter ──→ AST per file
                                    │
                  ┌─────────────────┼─────────────────┐
                  ▼                 ▼                  ▼
            Call Graph        Data Flow Graph    Control Flow Graph
          (import-aware,     (field-sensitive,    (goto-aware for C,
           cross-file)       interprocedural)      exception paths)
                  │                 │                  │
                  └────────┬────────┘──────────────────┘
                           ▼
                    Type Registry (RTA)
              C++ · Go · TS · Java · Rust · Python
                           │
                           ▼
                   Unified CPG (petgraph)
                           │
                  ┌────────┼────────┐
                  ▼        ▼        ▼
            Diff → Algorithm → SliceResult
                    dispatch
         data/control-flow algorithms share this CPG;
         simpler algorithms run on the AST only
```

**Key design decisions:**

- **petgraph as the graph backend** — the CPG is a single directed graph with
  typed edges (call, data flow, CFG) and typed nodes (function, variable,
  statement). All algorithms query the same graph structure.

- **tree-sitter for parsing** — zero-dependency, incremental, supports 11
  languages (12 tree-sitter grammar variants — TSX parsed separately from
  TypeScript) with a single unified AST query interface. No language-specific
  parsers to maintain.

- **CPG built once, queried many times** — the data/control-flow algorithms
  run graph traversals over the shared CPG (simpler algorithms are AST-only;
  see `SlicingAlgorithm::needs_cpg()`), so running many CPG-required
  algorithms together costs ~10% more than running one, because the expensive
  step (CPG construction) is shared.

- **Diff-aware by default** — algorithms receive the diff as input and focus
  analysis on changed code. Unchanged code is included only when it's
  reachable from a change (caller, data flow source, control flow predecessor).

- **Framework-aware route detection** — `src/frameworks/` recognizes 11 web/API
  frameworks (Go: gin, net/http, gorilla/mux; JS/TS: Express, Fastify, NestJS,
  Koa; Python: Flask, FastAPI, Django, DRF) to find route-entry points, feeding
  algorithms like `membrane`/`echo` that reason about cross-boundary callers.

---

## Type System

Prism includes a multi-language type system for resolving virtual/dynamic
dispatch during call graph construction. Without type information, a call to
`animal.speak()` could resolve to any `speak()` function in the codebase.
With type information, Prism narrows this to concrete implementations using
Rapid Type Analysis (RTA).

| Language   | Provider          | Dispatch Resolution                    |
|------------|-------------------|----------------------------------------|
| C++        | CppTypeProvider   | vtable, templates, qualified names     |
| Go         | GoTypeProvider    | Interface satisfaction, embedded types  |
| TypeScript | TypeScriptTypeProvider | Class hierarchy, interface implements |
| Java       | JavaTypeProvider  | Class hierarchy, interface implements   |
| Rust       | RustTypeProvider  | Trait impls, inherent methods           |
| Python     | PythonTypeProvider| Type annotations, class hierarchy       |

Languages without a dedicated provider fall back to name-based call resolution,
which is conservative (may include false callees) but never misses real ones.

---

## CPG Caching

Building the CPG is the most expensive operation (~1-3 seconds for a 50-file
repo). For repeated analysis of the same repository (e.g., reviewing multiple
PRs), Prism caches the serialized CPG to disk:

```bash
# First run: builds CPG, writes cache
prism --repo . --diff changes.patch --cache-dir ~/.cache/prism

# Second run: loads from cache, rebuilds only changed files
prism --repo . --diff new-changes.patch --cache-dir ~/.cache/prism
```

Cache behavior:

- **Full hit:** All files unchanged → load entire CPG from cache (~50ms)
- **Partial hit:** Some files changed → load cached data for unchanged files,
  rebuild only changed files, merge results
- **Miss:** Cache stale or absent → full rebuild

The type registry is rebuilt from source files on every run (not cached),
so adding a new language provider never invalidates the cache.

**Scope — this is one of two, separate caches.** `--cache-dir` above is the
diff-review CPG cache: it's opt-in and covers only the files referenced in
the *current diff*, so it's effectively per-diff/per-MR — a different diff
touching different files misses and triggers a full rebuild. `prism nav`
(whole-repo navigation) uses a second, separate cache: a prism-owned,
per-repo store under `dirs::cache_dir()/prism/nav/<hash(canonical repo
root)>/`, gated by `nav`'s own `--cache-dir`/`--no-cache` (see [`prism
nav`](#prism-nav--the-same-navigation-no-mcp-client-needed)). The two never
share state.

---

## Security Analysis

Prism's taint and absence algorithms are designed for catching real
vulnerabilities in C/firmware code:

```bash
# Trace taint from network input to dangerous sinks
prism --repo . --diff vuln.patch --algorithm taint

# Check for missing resource cleanup (malloc/free, lock/unlock)
prism --repo . --diff vuln.patch --algorithm absence

# Detect weakened preconditions (removed NULL checks, guard clauses)
prism --repo . --diff vuln.patch --algorithm contract
```

Prism has been validated against real CVE patterns including buffer overflows,
use-after-free/double-free, NULL dereferences, and command injection. See the
`cve_*.c` fixtures under `tests/fixtures/c/` (exercised by
`tests/lang/c/cve_test.rs` and `tests/lang/c/cve_fixture_test.rs`), plus the
sanitizer suites under `tests/fixtures/sanitizer-suite-{go,js-ts,python}/` and
the accuracy harness in `eval/fixtures/`.

---

## Language Coverage

Two metrics track cross-language support. See `coverage/matrix.json` for the full matrix and `docs/features/language-coverage/cross-language-coverage.md` for the measurement methodology. Run `python3 scripts/generate_coverage_badges.py` after changing the matrix to update badges and tables.

### Language Feature Coverage

Measures how many language-specific patterns (destructuring, multi-return, optional chaining, etc.) Prism handles for each language. This reflects DFG/alias/AccessPath completeness — whether the infrastructure correctly models the language's idioms.

<!-- COVERAGE_FEATURE_TABLE_START -->
| Language | Features | Coverage | Gaps |
|----------|----------|----------|------|
| Python | 16/18 | 88% | `for_range_multi`, `comprehension_taint` |
| JavaScript | 16/17 | 94% | `spread_field_provenance` |
| TypeScript | 16/17 | 94% | `spread_field_provenance` |
| Go | 14/15 | 93% | `for_range_multi` |
| Java | 12/12 | 100% | — |
| C | 12/12 | 100% | — |
| C++ | 14/14 | 100% | — |
| Rust | 11/12 | 91% | `question_mark_operator` |
| Lua | 10/10 | 100% | — |
| Terraform | 2/2 | 100% | — |
| Bash | 5/5 | 100% | — |
<!-- COVERAGE_FEATURE_TABLE_END -->

### Algorithm × Language

Every algorithm is tested against every supported language (330/330 cells). Some languages have deeper behavioral tests (✅ full) while others have basic smoke tests (🟡 basic) that verify the algorithm runs correctly on that language's syntax.

<!-- COVERAGE_TABLE_START -->
| Algorithm | Py | JS | TS | Go | Ja | C | C++ | Rs | Lua | TF | Sh |
|---|---|---|---|---|---|---|---|---|---|---|---|
| absence_slice |  ✅ | ✅ | 🟡 | ✅ | 🟡 | ✅ | 🟡 | ✅ | 🟡 | ✅ | 🟡 |
| angle_slice |  ✅ | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| barrier_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| callback_dispatcher_slice |  ❌ | ❌ | ❌ | ❌ | ❌ | 🟡 | 🟡 | ❌ | ❌ | ❌ | ❌ |
| chop |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| circular_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| conditioned_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| contract_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| delta_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| echo_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | ✅ | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| full_flow |  ✅ | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| gradient_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| horizontal_slice |  ✅ | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| left_flow |  🟡 | 🟡 | ✅ | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| membrane_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | ✅ | ✅ | 🟡 | 🟡 | 🟡 | 🟡 |
| original_diff |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| parent_function |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| peer_consistency_slice |  ❌ | ❌ | ❌ | ❌ | ❌ | 🟡 | 🟡 | ❌ | ❌ | ❌ | ❌ |
| phantom_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| primitive_slice |  🟡 | 🟡 | ❌ | 🟡 | ❌ | 🟡 | ❌ | ❌ | ❌ | ❌ | ❌ |
| provenance_slice |  ✅ | ✅ | 🟡 | 🟡 | 🟡 | ✅ | 🟡 | ✅ | ✅ | ✅ | 🟡 |
| quantum_slice |  ✅ | ✅ | 🟡 | 🟡 | 🟡 | ✅ | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| relevant_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| resonance_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| spiral_slice |  ✅ | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| symmetry_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| taint |  ✅ | ✅ | 🟡 | ✅ | 🟡 | ✅ | 🟡 | ✅ | ✅ | ✅ | ✅ |
| thin_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| threed_slice |  🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| vertical_slice |  ✅ | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |

✅ full (3+ tests) · 🟡 basic (1-2 tests) · ❌ none
<!-- COVERAGE_TABLE_END -->

✅ full (3+ tests) · 🟡 basic (1-2 tests) · ❌ none

---

## Limitations

- **Name-based variable tracking.** Variables matched by name within function
  scope. Same-named variables in nested scopes may cause extra context
  (conservative — false positives, not false negatives).

- **Quantum slice is heuristic.** Async state enumeration uses pattern matching,
  not formal model checking. It identifies potential races, not proven ones.

- **3D slice requires git.** The temporal axis shells out to `git log`. Won't
  work outside a git repository.

---

## Contributing

Issues and pull requests welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
