# LLM.md — Slicing: Code Slicing for Automated Code Review

## What This Project Does

A Rust implementation of 26 code slicing algorithms for extracting focused,
relevant code context from diffs. Given a diff and repository source, it
produces a concise slice — enough for an LLM to perform effective code review
without being overwhelmed by irrelevant code.

Based on ["Towards Practical Defect-Focused Automated Code Review"
(arXiv:2505.17928)](https://arxiv.org/abs/2505.17928) plus the established
program slicing taxonomy and novel theoretical extensions.

## All 26 Algorithms

### Paper Algorithms

| Algorithm | Strategy | Context Size | Best For |
|-----------|----------|-------------|----------|
| **OriginalDiff** | Raw diff lines only | Minimal | Token-constrained prompts |
| **ParentFunction** | Entire enclosing function | Medium | General context |
| **LeftFlow** | Backward data-flow from L-values | Focused | Defect detection (recommended) |
| **FullFlow** | LeftFlow + R-value forward tracing | Comprehensive | Deep analysis |

### Established Taxonomy

| Algorithm | Strategy | Best For |
|-----------|----------|----------|
| **ThinSlice** | Data deps only, no control flow | Most focused LLM input |
| **BarrierSlice** | Interprocedural with depth limits | Bounded cross-function analysis |
| **Chop** | All paths between source and sink | Security analysis, data flow bugs |
| **Taint** | Forward trace from untrusted values | Injection, XSS, command injection |
| **RelevantSlice** | LeftFlow + alternate branch paths | Missing else clauses, unhandled cases |
| **ConditionedSlice** | LeftFlow pruned by assumption | "What happens when x is null?" |
| **DeltaSlice** | Data flow diff between versions | Understanding behavioral changes |

### Theoretical Extensions

| Algorithm | Strategy | Best For |
|-----------|----------|----------|
| **SpiralSlice** | Adaptive-depth concentric rings | Progressive deepening |
| **CircularSlice** | Cross-function cycle detection | State management bugs, event cascades |
| **QuantumSlice** | Async state enumeration | Race conditions, unhandled states |
| **HorizontalSlice** | Peer pattern consistency | Omission detection ("these 3 handlers lack validation") |
| **VerticalSlice** | End-to-end feature path | Cross-layer bugs |
| **AngleSlice** | Cross-cutting concern trace | Error handling, logging, auth consistency |
| **3DSlice** | Risk scoring (structure * churn * size) | Review prioritization |

### Novel Extensions

| Algorithm | Strategy | Best For |
|-----------|----------|----------|
| **AbsenceSlice** | Missing counterpart detection | Resource leaks, unbalanced open/close, lock/unlock |
| **ResonanceSlice** | Git co-change frequency analysis | Finding files missing from a diff that usually change together |
| **SymmetrySlice** | Broken symmetry detection | serialize/deserialize, encode/decode consistency |
| **GradientSlice** | Continuous relevance scoring | Nuanced context with decaying relevance instead of binary |
| **ProvenanceSlice** | Data origin tracing | Identifying user_input, config, database, env_var origins |
| **PhantomSlice** | Recently deleted code surfacing | Detecting dependencies on removed code |
| **MembraneSlice** | Module boundary impact | API contract changes that break cross-file callers |
| **EchoSlice** | Ripple effect modeling | Callers missing error handling or null checks |

**Key finding from the paper:** LeftFlow often outperforms FullFlow because shorter,
focused context helps maintain LLM attention on the actual defect.

## Supported Languages

Python, JavaScript, TypeScript, Go, Java — all via tree-sitter AST parsing.

## How to Use the Output with an LLM

The sliced output is designed to be inserted into an LLM prompt for code review.
The format uses line-numbered, diff-marked lines:

```
+  12|    z = x + y        <- changed line
   13|    print(z)          <- context line
 ...|...                    <- gap (omitted lines)
   20|    return result     <- context line
```

### Recommended Prompt Structure

```
You are reviewing a code change. Below is the relevant code slice
extracted via static analysis. Lines prefixed with + are changed lines.
Other lines are context derived from data-flow analysis.

[SLICED CODE HERE]

Identify any defects, considering:
- Variable misuse or uninitialized values
- Off-by-one errors
- Resource leaks
- Null/undefined access
- Logic errors in control flow
```

### Algorithm Selection Guide for LLM Prompts

| Scenario | Algorithm | Why |
|----------|-----------|-----|
| General code review | `leftflow` | Best signal-to-noise ratio |
| Token budget is tight | `thin` | Minimal context, pure data chain |
| Security review | `taint` | Traces untrusted data to dangerous sinks |
| "Is this change safe?" | `relevant` | Shows what could break if a branch flips |
| "What does this affect?" | `fullflow` | Forward + backward tracing |
| Async/concurrent code | `quantum` | Enumerates possible interleavings |
| "Does this match the pattern?" | `horizontal` | Shows peer functions for comparison |
| "What's the full request path?" | `vertical` | Entry point to database |
| Error handling review | `angle --concern error_handling` | All error handling across files |
| Risk prioritization | `3d` | Ranks functions by coupling * churn * change size |
| Resource leak detection | `absence` | Finds open without close, lock without unlock |
| "Is this diff complete?" | `resonance` | Flags co-changing files missing from the diff |
| Symmetric pair consistency | `symmetry` | Checks serialize/deserialize, encode/decode pairs |
| Nuanced relevance ranking | `gradient` | Continuous scores instead of binary include/exclude |
| Data origin auditing | `provenance` | Classifies where each value originally came from |
| Deleted code dependencies | `phantom` | Surfaces recently removed code this change may need |
| API breakage detection | `membrane` | Shows all cross-file callers of changed functions |
| Downstream impact analysis | `echo` | Flags callers missing error handling for changed returns |

### Multi-Role Review (from the paper)

The paper achieves best results with a 3-reviewer + meta-reviewer pipeline:
1. Run 3 independent LLM reviewers on the same slice (temperature=0.1)
2. Each reviewer outputs up to 5 issues with severity scores (1-7)
3. A meta-reviewer aggregates and filters (keep issues scoring > 4)
4. A validator confirms issues against the actual diff

## CLI Usage

```bash
# Basic usage
slicing --repo ./my-project --diff changes.patch

# List all algorithms
slicing --list-algorithms

# Different algorithms
slicing --repo . --diff changes.patch -a thin
slicing --repo . --diff changes.patch -a taint
slicing --repo . --diff changes.patch -a spiral --spiral-max-ring 5
slicing --repo . --diff changes.patch -a chop --chop-source "src/api.py:42" --chop-sink "src/db.py:88"
slicing --repo . --diff changes.patch -a angle --concern error_handling
slicing --repo . --diff changes.patch -a barrier --barrier-depth 3
slicing --repo . --diff changes.patch -a conditioned --condition "user!=null"
slicing --repo . --diff changes.patch -a 3d --temporal-days 30

# Output formats
slicing --repo . --diff changes.patch -f json
slicing --repo . --diff changes.patch -f paper
```

### JSON Diff Input Format

```json
{
  "files": [
    {
      "file_path": "src/main.py",
      "modify_type": "Modified",
      "diff_lines": [10, 11, 12, 25]
    }
  ]
}
```

## Architecture

```
src/
├── main.rs                 # CLI entry point (clap)
├── lib.rs                  # Public API
├── diff.rs                 # Diff parsing (unified + JSON), DiffBlock output type
├── ast.rs                  # Tree-sitter AST wrapper with slicing-oriented queries
├── call_graph.rs           # Cross-file call graph (forward + reverse + cycle detection)
├── data_flow.rs            # Def-use chains, reachability, chopping, taint propagation
├── slice.rs                # SlicingAlgorithm enum (26 variants), SliceConfig, SliceResult
├── output.rs               # Output formatting (text, JSON, paper format)
├── algorithms/
│   ├── mod.rs              # Algorithm dispatcher
│   ├── original_diff.rs    # Paper: raw diff lines
│   ├── parent_function.rs  # Paper: enclosing function
│   ├── left_flow.rs        # Paper: backward L-value tracing
│   ├── full_flow.rs        # Paper: LeftFlow + R-value tracing
│   ├── thin_slice.rs       # Data deps only
│   ├── barrier_slice.rs    # Depth-limited interprocedural
│   ├── chop.rs             # Source-to-sink paths
│   ├── taint.rs            # Forward taint propagation
│   ├── relevant_slice.rs   # LeftFlow + alternate branches
│   ├── conditioned_slice.rs # Pruned by value assumption
│   ├── delta_slice.rs      # Two-version behavioral diff
│   ├── spiral_slice.rs     # Adaptive-depth rings
│   ├── circular_slice.rs   # Cross-function cycle detection
│   ├── quantum_slice.rs    # Async state enumeration
│   ├── horizontal_slice.rs # Peer pattern consistency
│   ├── vertical_slice.rs   # End-to-end feature path
│   ├── angle_slice.rs      # Cross-cutting concern trace
│   ├── threed_slice.rs     # Temporal-structural risk
│   ├── absence_slice.rs   # Missing counterpart detection
│   ├── resonance_slice.rs # Git co-change coupling
│   ├── symmetry_slice.rs  # Broken symmetry detection
│   ├── gradient_slice.rs  # Continuous relevance scoring
│   ├── provenance_slice.rs # Data origin tracing
│   ├── phantom_slice.rs   # Recently deleted code surfacing
│   ├── membrane_slice.rs  # Module boundary impact
│   └── echo_slice.rs      # Ripple effect modeling
└── languages/
    └── mod.rs              # Language detection, tree-sitter node type mappings
```

## Test Repositories

These actively maintained open source repositories are recommended for testing:

### Python
- **[pydantic/pydantic](https://github.com/pydantic/pydantic)** (~27k stars) — Validation pipelines
- **[Textualize/rich](https://github.com/Textualize/rich)** (~50k stars) — Rendering pipelines
- **[httpie/cli](https://github.com/httpie/cli)** (~35k stars) — Request building

### JavaScript
- **[date-fns/date-fns](https://github.com/date-fns/date-fns)** (~35k stars) — Composable pure functions
- **[pmndrs/zustand](https://github.com/pmndrs/zustand)** (~57k stars) — Middleware chains
- **[highlightjs/highlight.js](https://github.com/highlightjs/highlight.js)** (~25k stars) — State machine parser

### TypeScript
- **[colinhacks/zod](https://github.com/colinhacks/zod)** (~42k stars) — Schema composition
- **[trpc/trpc](https://github.com/trpc/trpc)** (~36k stars) — Middleware pipelines
- **[drizzle-team/drizzle-orm](https://github.com/drizzle-team/drizzle-orm)** (~33k stars) — Query builder chains

### Go
- **[charmbracelet/bubbletea](https://github.com/charmbracelet/bubbletea)** (~41k stars) — Elm Architecture
- **[spf13/cobra](https://github.com/spf13/cobra)** (~43k stars) — Command tree traversal
- **[spf13/viper](https://github.com/spf13/viper)** (~28k stars) — Multi-source config

### Java
- **[ben-manes/caffeine](https://github.com/ben-manes/caffeine)** (~16k stars) — Concurrent cache eviction
- **[FasterXML/jackson-databind](https://github.com/FasterXML/jackson-databind)** (~3.7k stars) — Serialization pipelines
- **[javaparser/javaparser](https://github.com/javaparser/javaparser)** (~5.4k stars) — Recursive descent parsing

### How to Test Against a Repository

```bash
git clone https://github.com/pydantic/pydantic /tmp/pydantic
cd /tmp/pydantic
git diff HEAD~1 > /tmp/recent.patch
slicing --repo /tmp/pydantic --diff /tmp/recent.patch -a leftflow

# Compare multiple algorithms
for algo in thin leftflow relevant spiral; do
  echo "=== $algo ==="
  slicing --repo /tmp/pydantic --diff /tmp/recent.patch -a $algo | wc -l
done
```

## Paper Reference

Lu, J., Jiang, L., Li, X., Fang, J., Zhang, F., Yang, L., & Zuo, C. (2025).
*Towards Practical Defect-Focused Automated Code Review.* arXiv:2505.17928.

| Algorithm | KBI (%) | FAR1 (%) | CPI1 |
|-----------|---------|----------|------|
| Original Diff | 23.70 | 94.84 | 5.71 |
| Parent Function | 31.85 | 95.66 | 5.52 |
| Left Flow | 37.04 | 94.36 | 9.77 |
| Full Flow | 39.26 | 94.57 | 9.67 |

KBI = Key Bug Inclusion rate. Higher is better.
