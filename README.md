[![CI](https://github.com/shoedog/prism/actions/workflows/ci.yml/badge.svg)](https://github.com/shoedog/prism/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/shoedog/prism/graph/badge.svg?token=C5JSSOQPWA)](https://codecov.io/github/shoedog/prism)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![Language Coverage](https://img.shields.io/badge/language_coverage-9_languages_%7C_95%25-green)

![Python](https://img.shields.io/badge/Python-88%25-green?logo=python&logoColor=white)
![JavaScript](https://img.shields.io/badge/JavaScript-94%25-green?logo=javascript&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-94%25-green?logo=typescript&logoColor=white)
![Go](https://img.shields.io/badge/Go-93%25-green?logo=go&logoColor=white)
![Java](https://img.shields.io/badge/Java-100%25-brightgreen?logo=openjdk&logoColor=white)
![C](https://img.shields.io/badge/C-100%25-brightgreen?logo=c&logoColor=white)
![C++](https://img.shields.io/badge/C%2B%2B-100%25-brightgreen?logo=cplusplus&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-91%25-green?logo=rust&logoColor=white)
![Lua](https://img.shields.io/badge/Lua-100%25-brightgreen?logo=lua&logoColor=white)

# slicing

A command-line tool that extracts focused, relevant code context from diffs for
code review. Instead of dumping an entire file or just the raw changed lines,
`slicing` uses static analysis to pull in exactly the surrounding code a
reviewer needs to understand a change — variable definitions, control flow,
called functions, return paths.

Implements 26 slicing algorithms spanning the paper
[Towards Practical Defect-Focused Automated Code Review](https://arxiv.org/abs/2505.17928),
the established program slicing taxonomy, and several novel theoretical
extensions including spiral, quantum, horizontal, vertical, angle, and 3D slices.

Supports **Python**, **JavaScript**, **TypeScript**, **Go**, **Java**, **C**, **C++**, **Rust**, and **Lua**.

---

## Install

Requires Rust 1.70+.

```bash
git clone <repo-url> && cd slicing
cargo build --release
```

The binary lands at `target/release/slicing`. Copy it somewhere on your `$PATH`
or run it directly.

---

## Quick start

```bash
# Generate a diff from any repo
cd /path/to/your/project
git diff HEAD~1 > /tmp/changes.patch

# Slice it
slicing --repo . --diff /tmp/changes.patch
```

That's it. The default algorithm (`leftflow`) traces data flow backward from
each changed line and prints the relevant slice to stdout.

List all 26 algorithms:

```bash
slicing --list-algorithms
```

---

## All 26 algorithms at a glance

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

---

## Usage by language

### Python

```bash
cd ~/projects/my-python-app
git diff main > /tmp/diff.patch

slicing --repo . --diff /tmp/diff.patch --algorithm leftflow
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
slicing --repo . --diff /tmp/diff.patch -a thin

# Taint: where do diff-line values end up?
slicing --repo . --diff /tmp/diff.patch -a taint

# Horizontal: find all handler functions that should match the changed one
slicing --repo . --diff /tmp/diff.patch -a horizontal

# Angle: trace error handling across the codebase
slicing --repo . --diff /tmp/diff.patch -a angle --concern error_handling
```

---

### JavaScript

```bash
cd ~/projects/my-js-app
git diff HEAD~3 > /tmp/diff.patch

slicing --repo . --diff /tmp/diff.patch
```

Recognized extensions: `.js`, `.mjs`, `.cjs`, `.jsx`

Handles `function` declarations, arrow functions (`=>`), method definitions,
generator functions, `const`/`let`/`var` declarations, and all standard control
flow.

**JS-specific algorithms worth trying:**

```bash
# Quantum: find async state races around await boundaries
slicing --repo . --diff /tmp/diff.patch -a quantum --quantum-var response

# Circular: detect event handler or state management cycles
slicing --repo . --diff /tmp/diff.patch -a circular

# Relevant: see alternate branches ("what if this condition was false?")
slicing --repo . --diff /tmp/diff.patch -a relevant
```

---

### TypeScript

```bash
cd ~/projects/my-ts-app
git diff feature-branch > /tmp/diff.patch

slicing --repo . --diff /tmp/diff.patch -a fullflow
```

Recognized extensions: `.ts`, `.tsx`

Same capabilities as JavaScript. Types are parsed but slicing focuses on
value-level data flow.

**TS-specific algorithms worth trying:**

```bash
# Barrier: trace callers/callees up to 3 levels, stop at framework internals
slicing --repo . --diff /tmp/diff.patch -a barrier --barrier-depth 3 --barrier-symbols "React.createElement,useEffect"

# Vertical: see the full request path from handler to database
slicing --repo . --diff /tmp/diff.patch -a vertical --layers "routes,services,models,db"
```

---

### Go

```bash
cd ~/projects/my-go-service
git diff HEAD~1 > /tmp/diff.patch

slicing --repo . --diff /tmp/diff.patch
```

Recognized extensions: `.go`

Handles `func` declarations, method declarations (with receivers),
`:=` short variable declarations, `for`/`range` loops, `if`/`switch`
statements, and `return` statements.

**Go-specific algorithms worth trying:**

```bash
# Quantum: detect goroutine races
slicing --repo . --diff /tmp/diff.patch -a quantum

# Chop: is there a data path from user input to this SQL query?
slicing --repo . --diff /tmp/diff.patch -a chop --chop-source "handlers/api.go:42" --chop-sink "db/query.go:88"

# 3D: which functions have the most risk (high coupling + high churn)?
slicing --repo . --diff /tmp/diff.patch -a 3d --temporal-days 30
```

---

### Java

```bash
cd ~/projects/my-java-project
git diff develop > /tmp/diff.patch

slicing --repo . --diff /tmp/diff.patch -a parentfunction
```

Recognized extensions: `.java`

Handles method declarations, constructor declarations,
`local_variable_declaration`, field declarations, enhanced for loops, try
statements, and standard control flow.

**Java-specific algorithms worth trying:**

```bash
# Spiral: start narrow and widen progressively
slicing --repo . --diff /tmp/diff.patch -a spiral --spiral-max-ring 5

# Conditioned: "what does the code do when this value is null?"
slicing --repo . --diff /tmp/diff.patch -a conditioned --condition "user!=null"

# Angle: trace authentication handling across layers
slicing --repo . --diff /tmp/diff.patch -a angle --concern auth

# Delta: what data-flow paths changed vs the previous version?
slicing --repo . --diff /tmp/diff.patch -a delta --old-repo /path/to/old/version
```

---

## Output formats

### Text (default)

Human-readable, line-numbered output with `+` marking changed lines and `...`
for gaps:

```bash
slicing --repo . --diff changes.patch --format text
```

### JSON

Machine-readable. Contains the full `SliceResult` with algorithm name, blocks,
line maps, and diff metadata:

```bash
slicing --repo . --diff changes.patch --format json
```

### Paper

Matches the `diff_outputs.json` format from the original paper:

```bash
slicing --repo . --diff changes.patch --format paper
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
| `--format`, `-f` | `text` | `text`, `json`, `paper` |
| `--list-algorithms` | | Print all algorithms and exit |
| `--max-branch-lines` | `5` | Max lines in a branch before summarizing |
| `--no-returns` | | Skip return statements in leftflow/fullflow |
| `--no-trace-callees` | | Skip callee bodies in fullflow |

### Algorithm-specific flags

| Flag | Algorithm | Description |
|---|---|---|
| `--barrier-depth N` | barrier | Max call depth (default: 2) |
| `--barrier-symbols a,b` | barrier | Functions to stop at |
| `--chop-source file:line` | chop | Source location |
| `--chop-sink file:line` | chop | Sink location |
| `--taint-source file:line` | taint | Explicit taint source (repeatable) |
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
slicing --repo . --diff changes.patch | pbcopy
slicing --repo . --diff changes.patch | llm review

# Save JSON for processing
slicing --repo . --diff changes.patch -f json > slice.json

# Filter by language
git diff main -- '*.py' > /tmp/py-only.patch
slicing --repo . --diff /tmp/py-only.patch

# Compare algorithms
for algo in thin leftflow fullflow relevant; do
  echo "=== $algo ==="
  slicing --repo . --diff changes.patch -a $algo | wc -l
done
```

---

## Language Coverage

Coverage percentages reflect how many language-specific patterns (destructuring, multi-return, optional chaining, etc.) are handled for each language. See `coverage/matrix.json` for the full matrix and `docs/cross-language-coverage.md` for the measurement methodology.

### Algorithm × Language

| Algorithm | Py | JS | TS | Go | Ja | C | C++ | Rs | Lua |
|---|---|---|---|---|---|---|---|---|---|
| absence_slice | ✅ | 🟡 | 🟡 | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ |
| angle_slice | ✅ | 🟡 | ❌ | ✅ | 🟡 | ❌ | ❌ | ❌ | ❌ |
| barrier_slice | ✅ | 🟡 | ❌ | 🟡 | 🟡 | ❌ | ❌ | ❌ | ❌ |
| chop | ✅ | 🟡 | ❌ | 🟡 | 🟡 | ❌ | ❌ | ❌ | ❌ |
| circular_slice | 🟡 | ❌ | 🟡 | 🟡 | ❌ | ❌ | ❌ | ❌ | ❌ |
| conditioned_slice | ✅ | 🟡 | ❌ | 🟡 | 🟡 | ❌ | ❌ | ❌ | ❌ |
| delta_slice | ✅ | ❌ | ❌ | 🟡 | ❌ | ❌ | ❌ | ❌ | ❌ |
| echo_slice | ✅ | 🟡 | ❌ | 🟡 | 🟡 | ✅ | ❌ | ❌ | ❌ |
| full_flow | ✅ | ✅ | ❌ | ✅ | ✅ | 🟡 | ❌ | ❌ | ❌ |
| gradient_slice | ✅ | ❌ | ❌ | 🟡 | ❌ | ❌ | ❌ | ❌ | ❌ |
| horizontal_slice | ✅ | 🟡 | ❌ | 🟡 | 🟡 | ❌ | ❌ | ❌ | ❌ |
| left_flow | ✅ | ✅ | 🟡 | ✅ | ✅ | 🟡 | ❌ | ❌ | ❌ |
| membrane_slice | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 | ✅ | ✅ | 🟡 | 🟡 |
| original_diff | ✅ | ✅ | 🟡 | ✅ | 🟡 | ❌ | ❌ | 🟡 | ❌ |
| parent_function | ✅ | ❌ | 🟡 | ✅ | 🟡 | ❌ | ❌ | 🟡 | 🟡 |
| phantom_slice | 🟡 | ❌ | ❌ | 🟡 | ❌ | ❌ | ❌ | ❌ | ❌ |
| provenance_slice | ✅ | ✅ | ❌ | ✅ | 🟡 | ✅ | ❌ | ✅ | ✅ |
| quantum_slice | ✅ | ✅ | ❌ | ✅ | 🟡 | ✅ | ❌ | 🟡 | 🟡 |
| relevant_slice | 🟡 | ❌ | ❌ | 🟡 | ❌ | ❌ | ❌ | ❌ | ❌ |
| resonance_slice | 🟡 | ❌ | ❌ | 🟡 | ❌ | ❌ | ❌ | ❌ | ❌ |
| spiral_slice | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| symmetry_slice | ✅ | ❌ | ❌ | 🟡 | ❌ | 🟡 | ❌ | ❌ | ❌ |
| taint | ✅ | ✅ | 🟡 | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| thin_slice | ✅ | ❌ | 🟡 | 🟡 | 🟡 | 🟡 | ❌ | 🟡 | 🟡 |
| threed_slice | ✅ | ❌ | ❌ | 🟡 | ❌ | ❌ | ❌ | ❌ | ❌ |
| vertical_slice | ✅ | ❌ | ❌ | 🟡 | ❌ | ❌ | ❌ | ❌ | ❌ |

✅ full (3+ tests) · 🟡 basic (1-2 tests) · ❌ none

---

## Limitations

- **Name-based variable tracking.** Variables matched by name within function
  scope. Same-named variables in nested scopes may cause extra context
  (conservative — false positives, not false negatives).

- **Partial cross-file analysis.** Called function signatures and bodies are
  traced, but full import resolution is not implemented.

- **No type information.** Tree-sitter provides syntax trees, not type-checked
  ASTs. Can't distinguish same-named variables across scopes without types.

- **Quantum slice is heuristic.** Async state enumeration uses pattern matching,
  not formal model checking. It identifies potential races, not proven ones.

- **3D slice requires git.** The temporal axis shells out to `git log`. Won't
  work outside a git repository.

- **Language coverage.** Python, JavaScript, TypeScript, Go, and Java. See
  CLAUDE.md for how to add new languages.
