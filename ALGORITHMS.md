# Algorithm Reference

Per-algorithm operator's guide for all 30 slicing algorithms. The README has the cheat-sheet and CLI flag table; this is the deep reference for **what each algorithm answers, what it returns, and when its output is meaningful**.

If you only want to pick one, the default `leftflow` is the right starting point. The `review` preset (`-a review`) runs a curated subset that combines AST-only and CPG-based algorithms; `all` runs everything.

## How to read an entry

Each algorithm entry has these fields:

- **Flag** — the `-a <name>` token plus accepted aliases.
- **Question** — the one-sentence reviewer question this algorithm is meant to answer.
- **Mechanism** — the one- or two-sentence implementation strategy. Read this to know whether the algorithm's output is trustworthy for your context.
- **When to use** — concrete scenarios where this slice catches things the default doesn't.
- **CLI** — minimal invocation. Flags listed in the README's "Algorithm-specific flags" table.
- **Output** — `blocks` (line-level slice) plus `findings` (structured, with `severity` and `category`) where applicable. Severities are `info` (advisory), `suggestion` (low-priority), `warning` (review-warranted), or `concern` (likely real issue).
- **Limitations** — specific gotchas, language scope, and known false-negative / false-positive shapes. Read this *before* trusting the absence of a finding.
- **Source** — file path you can grep for the matchers, sink lists, and thresholds.

Algorithms that need a CPG (DFG + CFG + call graph) are marked **CPG-required**; the others run on parsed ASTs only. CPG-required algorithms cost more on a cold cache but free once the CPG is built — running 10 CPG-required algorithms on the same diff is barely more expensive than running one.

---

## Paper algorithms (arXiv:2505.17928)

The four baseline algorithms from the paper. Cheap, AST-only. Use these when you need a predictable, well-understood baseline.

### 1. OriginalDiff

**Flag:** `-a originaldiff` · `-a onlydiff`

**Question.** "What lines literally changed?"

**Mechanism.** Returns only the diff-touched lines, no expansion. The simplest possible slice.

**When to use.** Sanity check. Useful as a fallback when other algorithms produce confusing output.

**CLI.** `slicing --repo . --diff changes.patch -a originaldiff`

**Output.** One block per file. Every line in the block is marked `is_diff = true`. No findings.

**Limitations.** No context whatsoever. A reviewer can't tell from this slice whether the change touches a guard, an assignment, a return, or a comment.

**Source.** `src/algorithms/original_diff.rs`

### 2. ParentFunction

**Flag:** `-a parentfunction` · `-a function`

**Question.** "What's the full function this change is in?"

**Mechanism.** For each diff line, includes the entire enclosing function (signature through closing brace). Multiple diff lines in the same function produce one block. Lines not inside any function (top-level / global scope) get their own block.

**When to use.** When the diff line is meaningless without seeing the rest of the function (e.g. a single-line guard change in a 60-line method).

**CLI.** `slicing --repo . --diff changes.patch -a parentfunction`

**Output.** One block per (file, function) pair containing all lines from `func_start` to `func_end`. Diff-touched lines marked `is_diff = true`. No findings.

**Limitations.** Includes everything in the function, even unrelated code paths. Can be noisy for very large functions.

**Source.** `src/algorithms/parent_function.rs`

### 3. LeftFlow (default)

**Flag:** `-a leftflow` · `-a relevantcode` (default if `-a` is omitted)

**Question.** "What downstream code reads or branches on the values I changed?"

**Mechanism.** **CPG-required.** For each diff line, traces forward from L-values (assignment targets) through the data-flow graph; also includes condition variables, called functions' signatures, and return statements. Falls back to name-based AST matching when DFG is empty. Includes branch boundaries for branches up to `--max-branch-lines` (default 5).

**When to use.** The default for reasons: it's a good balance of focus and context. Use it when you're not sure which slice to pick.

**CLI.** `slicing --repo . --diff changes.patch -a leftflow`

**Output.** One block per (file, function) containing function signature + closing line + traced lines. Diff lines marked. No findings.

**Limitations.** Intraprocedural by default — only traces within the diff's enclosing function. Branch context is heuristic. Pure name-based fallback can over-include when the same identifier appears in nested scopes.

**Source.** `src/algorithms/left_flow.rs`

### 4. FullFlow

**Flag:** `-a fullflow` · `-a relevantcoderhs`

**Question.** "What does this change touch in *both* directions — what feeds it, what it feeds into?"

**Mechanism.** **CPG-required.** Extends LeftFlow with R-value tracing (variables read on the RHS of diff lines), full callee bodies (configurable via `--no-trace-callees`), and cross-file references when an R-value resolves to a function elsewhere.

**When to use.** When the change touches a value used by callers you also need to see, or when you want callee bodies inline rather than just signatures.

**CLI.** `slicing --repo . --diff changes.patch -a fullflow`

**Output.** Same shape as LeftFlow but blocks may include lines from multiple files (cross-file R-value references). No findings.

**Limitations.** Larger output than LeftFlow. The cross-file R-value heuristic resolves by exact name — it includes any function with the matching name across all parsed files.

**Source.** `src/algorithms/full_flow.rs`

---

## Established taxonomy

Standard program-slicing types. Use these when you have a specific question (security, two-version diff, focused data chain) that the paper algorithms don't directly answer.

### 5. ThinSlice

**Flag:** `-a thin`

**Question.** "What's the minimal data chain from this change — no control flow, no return statements, no branches?"

**Mechanism.** Like LeftFlow but strips out all control flow context. Pure data deps: assignment targets and references to those targets, nothing else.

**When to use.** When you want the most focused possible context for an LLM (or a human) to reason about a single value's flow without surrounding noise.

**CLI.** `slicing --repo . --diff changes.patch -a thin`

**Output.** One block per (file, function) with only the minimal data chain. No findings.

**Limitations.** Loses context that's often crucial for reasoning (e.g. which branch the value flows through). Not appropriate when the change touches a guard or branch condition.

**Source.** `src/algorithms/thin_slice.rs`

### 6. BarrierSlice

**Flag:** `-a barrier` · `-a barrierslice`

**Question.** "Trace this change N levels up and N levels down through the call graph, but stop at functions or modules I've marked as boundaries."

**Mechanism.** **CPG-required.** For each function touched by the diff, traces callers up and callees down to `BarrierConfig.max_depth` (default 2), pruning paths that cross any name in `barrier_symbols` or any path under any prefix in `barrier_modules`.

**When to use.** When you want interprocedural context but need to exclude noisy infrastructure (logging, metrics, framework internals) from the slice. The user supplies the boundary.

**CLI.** `slicing --repo . --diff changes.patch -a barrier --barrier-depth 3 --barrier-symbols "log_debug,metrics_emit" --barrier-modules "vendor/,third_party/"`

**Output.** One block per (diff-file, diff-function) pair. `file_line_map` includes the function body, in-budget caller/callee signatures (start + end lines), and per-call-site lines with `is_diff=true` only on actual diff-touched lines inside the originating function. **No findings** — pure scope expansion.

**Limitations.** Emits no findings — only scope. Common scope-noise sources: C/C++ overloads include all same-named definitions; virtual / dynamic-dispatch callees that don't match call-graph indirect-resolution Levels 1–4 are silently missed (false negative); duplicate names in non-C languages include every definition. Mitigate by widening `--barrier-modules` / `--barrier-symbols`.

**Source.** `src/algorithms/barrier_slice.rs`

### 7. Chop

**Flag:** `-a chop` · `-a chopping`

**Question.** "Is there a data-flow path from this source location to this sink location, and what statements are on it?"

**Mechanism.** **CPG-required.** Computes the intersection of forward-reachable-from-source and backward-reachable-from-sink in the DFG, additionally filtered by CFG reachability when CFG edges exist (`dfg_cfg_chop`).

**When to use.** Security analysis where you have a specific source (e.g. `request_handler.go:42`) and a specific sink (e.g. `db/exec.go:88`) and want to know whether one reaches the other.

**CLI.** `slicing --repo . --diff changes.patch -a chop --chop-source handlers/api.go:42 --chop-sink db/query.go:88`

**Output.** One block per file containing the intersection lines, with source and sink marked `is_diff=true` for highlighting. No findings.

**Limitations.** Requires explicit `--chop-source` and `--chop-sink`. CFG filter is intraprocedural; cross-function targets bypass the filter and rely on DFG only. Empty result means no path — but absence is conservative (could be a false negative for edges the indirect-call resolver missed).

**Source.** `src/algorithms/chop.rs`

### 8. Taint

**Flag:** `-a taint` · `-a taint_analysis`

**Question.** "Does any tainted value (diff line, IPC accessor, or explicit source) reach a known sink?"

**Mechanism.** **CPG-required.** Forward DFG propagation from taint sources, intersected with CFG reachability to prune dead code. Sources = diff lines (default) + GLib/D-Bus IPC accessors auto-detected in C/C++ files + explicit `--taint-source` locations. Sinks = a flat 200-entry list of patterns covering Python/JS/Go/Rust/Lua/Bash/C/C++/HCL plus dynamically-detected variadic format-string wrappers (1-hop only).

**When to use.** Security review of a diff that touches input handling, network IO, IPC, or anywhere user-controlled data might originate.

**CLI.** `slicing --repo . --diff vuln.patch -a taint --taint-source "src/handler.c:124"`

**Output.** Blocks containing every line on a tainted path. **Findings:**
  - `category: "taint_source"`, severity `info` — origin annotation per source.
  - `category: "taint_sink"`, severity `warning` — tainted value reaches a known sink.
  - `category: "unquoted_expansion"`, severity `warning` — Bash-only: `$VAR` inside a `command` not wrapped in a `string` node.

**Limitations.** Generic name/access-path DFG with no framework-aware modeling. Pydantic / dataclass validators are invisible (treated as opaque calls); decorator chains are not edges, so `@app.route` / `@retry` and similar wrappers do not connect dispatcher → handler; ORM session managers propagate through ordinary alias tracking but session/transaction lifecycles are not modeled; `await` is transparent (taint crosses iff the awaited callee resolves in the call graph); JS/TS Promise `.then(cb)` does not connect the resolved value to `cb`'s parameter. Sink list and source list are flat tables — `src/algorithms/taint.rs:21` and `:232`.

**Source.** `src/algorithms/taint.rs`

### 9. RelevantSlice

**Flag:** `-a relevant` · `-a relevantslice`

**Question.** "What would this code do if any of its branch conditions flipped?"

**Mechanism.** **CPG-required.** Starts with LeftFlow output, then for each Branch/Loop CFG node in the slice, walks alternate CFG successors forward up to `flip_depth` statements (default = `max(max_branch_lines, 3)`). Also runs an AST pass that detects missing `else` clauses (the "what if this `if` was false?" zone).

**When to use.** "One-flip-away" analysis — catching missing `else`, unhandled switch defaults, error returns that don't get checked.

**CLI.** `slicing --repo . --diff changes.patch -a relevant`

**Output.** Same shape as LeftFlow plus `flip_depth` lines from each alternate branch. No findings.

**Limitations.** Heuristic. Conservative on the AST side (only specific child node kinds are recognized as alternates). The CFG path is more precise but only fires if the CPG has CFG edges built.

**Source.** `src/algorithms/relevant_slice.rs`

### 10. ConditionedSlice

**Flag:** `-a conditioned` · `-a conditionedslice`

**Question.** "If I assume this predicate holds, what code is unreachable?"

**Mechanism.** **CPG-required.** Starts with LeftFlow, then prunes branches made unreachable by `--condition`. Uses AST pattern matching to identify branches whose condition is logically resolved by the assumption, then refines via CFG reachability so that lines only reachable through pruned branches are also removed (and merge-point lines reachable from other paths are kept).

**When to use.** "What does this code do when `user != null`?" or "What's the `error == 0` path?" — explore a specific execution scenario without other branches in the way.

**CLI.** `slicing --repo . --diff changes.patch -a conditioned --condition "user!=null"`

**Output.** LeftFlow-shape blocks with unreachable lines removed.

**Limitations.** Predicate parser handles `==`, `!=`, `>`, `<`, `>=`, `<=`, plus null-aware variants for `null`/`None`/`nil`. Only matches branches whose condition text contains the variable + operator + value substring; complex predicates like `if (a > 0 && b < 10)` are usually not pruned even when the assumption logically resolves them.

**Source.** `src/algorithms/conditioned_slice.rs`

### 11. DeltaSlice

**Flag:** `-a delta` · `-a deltaslice`

**Question.** "Which data-flow edges actually changed between the old version and the new version?"

**Mechanism.** **CPG-required.** Re-parses the changed files from `--old-repo`, builds an old CPG with the same type enrichment as the new one, then computes the symmetric difference of DFG edge sets. Lines on either side of any added or removed edge are returned.

**When to use.** When git diff alone over-reports change scope (whitespace, refactors that don't change behavior). Delta shows only structurally meaningful changes.

**CLI.** `slicing --repo . --diff changes.patch -a delta --old-repo /path/to/old/checkout`

**Output.** One block per file with lines participating in changed DFG edges; original diff lines marked. No findings.

**Limitations.** Requires `--old-repo` pointing to a parseable older checkout. Only compares DFG edges (data flow); doesn't surface CFG-only or call-graph-only changes. Edges are compared by (file, from_line, to_line) tuples — equivalent code that happens to produce different line offsets registers as a change.

**Source.** `src/algorithms/delta_slice.rs`

---

## Theoretical extensions

Slicing concepts that extend the established taxonomy. Most are heuristic; read each algorithm's Limitations carefully before relying on its findings.

### 12. SpiralSlice

**Flag:** `-a spiral` · `-a spiralslice`

**Question.** "Start narrow at the change point and widen outward — give me as much context as the change actually justifies."

**Mechanism.** **CPG-required.** Six concentric rings: (1) diff lines, (2) enclosing function, (3) LeftFlow + direct caller/callee signatures, (4) depth-2 callers/callees, (5) test files referencing changed functions, (6) shared utilities (files referenced from ≥2 changed files). Auto-stops when a ring adds < `auto_stop_threshold` (default 5%) new lines relative to the prior ring.

**When to use.** When you don't know how deep to look. Spiral picks a depth based on how much the slice grows at each ring.

**CLI.** `slicing --repo . --diff changes.patch -a spiral --spiral-max-ring 5`

**Output.** One block per file; each line is annotated internally with its ring level (1=diff line, 6=shared utility). No findings.

**Limitations.** Caller resolution is name-keyed across the parsed file set. Cross-file: yes (within parsed set); cross-package: yes if parsed; vendored / external: no (anything not parsed is invisible). Cross-language (FFI, subprocess, cgo, JNI): not modeled. Unresolved indirect dispatch silently terminates the chain — no marker is emitted.

**Source.** `src/algorithms/spiral_slice.rs`

### 13. CircularSlice

**Flag:** `-a circular` · `-a circularslice`

**Question.** "Is this change inside a call-graph or data-flow cycle?"

**Mechanism.** **CPG-required.** Uses petgraph's `tarjan_scc` to find call-graph cycles, then filters to cycles that include at least one diff function. Also detects DFG cycles within functions.

**When to use.** Mutual recursion, event handler cascades, state-management feedback loops. Anywhere unintended recursion or cyclic state mutation might be a bug.

**CLI.** `slicing --repo . --diff changes.patch -a circular`

**Output.** One block per cycle showing the function signatures and call sites that form the cycle. No findings.

**Limitations.** SCCs of size 1 (self-recursion) are not specifically called out; they appear as one-element cycles. Dynamic dispatch through unresolved indirect calls produces neither edges nor cycles, so cycles through (e.g.) virtual methods may be invisible.

**Source.** `src/algorithms/circular_slice.rs`

### 14. QuantumSlice

**Flag:** `-a quantum` · `-a quantumslice`

**Question.** "If this code is async, what states could the variable hold around the await/goroutine boundary?"

**Mechanism.** Heuristic per-language async detection (Python `await` + threading; JS `await` + `.then` + `setTimeout`; Go `go_statement` + `select`; Rust `tokio::spawn`; C/C++ `pthread_create` / `request_irq` / `signal` / `sigaction`; Lua `coroutine.*`; Bash `&` / `nohup`). For each variable on a diff line, enumerates assignment points and annotates whether they're before, after, or independent of an async boundary. C/C++ also detects functions registered as handlers via `signal`, `pthread_create`, `sigaction`, `request_irq`, `std::thread`.

**When to use.** When the diff touches an async/concurrent function and you want to know whether assignments could race.

**CLI.** `slicing --repo . --diff changes.patch -a quantum --quantum-var response`

**Output.** Block per (variable, async function) showing assignment lines and async-boundary lines. Only emitted when at least one state is async-dependent. No findings.

**Limitations.** Pattern matching, not formal model checking — identifies *potential* races, not proven ones. If `--quantum-var` isn't given, every identifier on the diff line is analyzed (high noise). Go-specific handling detects goroutine closure captures but not channels, mutexes, or atomics.

**Source.** `src/algorithms/quantum_slice.rs`

### 15. HorizontalSlice

**Flag:** `-a horizontal` · `-a horizontalslice`

**Question.** "What sibling functions look like the changed one — and would the same change apply there too?"

**Mechanism.** Identifies peers via `PeerPattern` ∈ {`Auto` (default — sniffs decorator above the function, falls back to directory siblings), `Decorator(@x)`, `ParentClass(name)`, `NamePattern(prefix*` / `*suffix` / `contains`), `DirectorySiblings`}. For each diff function, finds peers across all files matching the pattern.

**When to use.** Omission detection. "This handler has validation; do the others?" or "I added retry logic to one method — should the rest of this class have it too?"

**CLI.** `slicing --repo . --diff changes.patch -a horizontal --peer-pattern "decorator:@app.route"`

**Output.** Block per changed function containing the changed function plus the first 10 lines of each peer (preview). No findings.

**Limitations.** `Auto` mode picks the first decorator above the function and uses its base name (before `(`). If no decorator, falls back to "all functions in the same file" — very broad. `ParentClass` only matches when the function's tree-sitter parent is `class_body` or `class_declaration`.

**Source.** `src/algorithms/horizontal_slice.rs`

### 16. VerticalSlice

**Flag:** `-a vertical` · `-a verticalslice`

**Question.** "What's the end-to-end path from request entry to persistence for this feature?"

**Mechanism.** **CPG-required.** Detects layers via path keywords (`handler`, `controller`, `service`, `repository`, `dao`, `db`, etc.) or honors explicit `--layers`. For each diff function, walks callers up (depth 10) and callees down (depth 10), labels each with its layer, deduplicates by (file, function), and emits a block.

**When to use.** When you want to see "the request handler all the way down to the SQL query" for a single change.

**CLI.** `slicing --repo . --diff changes.patch -a vertical --layers "routes,services,models,db"`

**Output.** Per diff function, one block containing the entry/exit signature lines for every function in the path. No findings.

**Limitations.** Layer detection is path-keyword heuristics; if your project uses different terminology, pass `--layers` explicitly. Depth 10 is hardcoded — for deep stacks the slice can balloon. Path deduplication is by `(file, function)` so a function called by multiple ancestors shows once.

**Source.** `src/algorithms/vertical_slice.rs`

### 17. AngleSlice

**Flag:** `-a angle` · `-a angleslice`

**Question.** "How does this concern (errors, logging, auth, caching) cut across the codebase, and where else does it appear?"

**Mechanism.** Per-concern keyword sets: `error_handling`, `logging`, `authentication`, `caching`, plus `Custom(name, [keywords])`. For each diff line containing the concern, finds every line in every file containing any of the concern's keywords; includes enclosing function start/end lines.

**When to use.** "Did the auth check change here? Where else does auth happen?" — cross-cutting inventory.

**CLI.** `slicing --repo . --diff changes.patch -a angle --concern auth`

**Output.** One block per file containing concern hits; lines that overlap the diff are marked `is_diff=true`. No findings.

**Limitations.** Substring keyword matching on raw source — comments and string literals can match. Custom concerns split on `,` for keywords. The keyword lists in `src/algorithms/angle_slice.rs:36` are deliberately broad (e.g. logging includes `console.log`, `fmt.Print`, `LOG`, `debug`, `info`, `warn`) so expect false positives in projects that use those words for non-logging purposes.

**Source.** `src/algorithms/angle_slice.rs`

### 18. ThreeDSlice

**Flag:** `-a 3d` · `-a threed` · `-a threedslice`

**Question.** "Which functions are highest risk — combining structural coupling, recent churn, and change size?"

**Mechanism.** **CPG-required, requires git.** Three axes per function: structural coupling (callers + callees, depth 1), temporal activity (commits in `--temporal-days` window touching the file, default 90), and change complexity (diff lines inside the function). Risk score = `coupling × temporal × complexity` (each factor floored at 1). Also scores second-degree connected functions for context.

**When to use.** Triage. "Of the 12 functions touched, which 3 should I review first?"

**CLI.** `slicing --repo . --diff changes.patch -a 3d --temporal-days 30`

**Output.** Blocks sorted by risk score descending; each contains the function signature plus its diff lines. No findings.

**Limitations.** Requires `git` to be available and the working directory to be a git repo. Multiplicative scoring means any factor at zero zeros the score — which is intended (a function with no callers/callees and no recent churn isn't risky), but it means the absolute score is not comparable across runs.

**Source.** `src/algorithms/threed_slice.rs`

---

## Novel extensions

Diff-aware patterns that don't map cleanly onto the established taxonomy. These are where Prism does most of its higher-precision work.

### 19. AbsenceSlice

**Flag:** `-a absence` · `-a absenceslice`

**Question.** "Does this open / lock / allocate have a matching close / unlock / free in the same function?"

**Mechanism.** A flat ~50-pattern table of `(open_patterns, close_patterns, description)` covering generic, kernel/POSIX, Python, JS/TS, Go, Rust, Lua, Terraform, and Shell idioms — see `src/algorithms/absence_slice.rs:28` for the full list. For each diff line containing an open pattern, scans the enclosing function for a matching close. C/C++ functions with `goto` patterns get path-aware analysis: distinguishes normal-path close from error-path-only close, and detects double-close patterns (close inline + close in goto target).

**When to use.** Resource-leak review — files, locks, allocations, transactions, event subscriptions, IRQs. Especially valuable for kernel/firmware C code.

**CLI.** `slicing --repo . --diff changes.patch -a absence`

**Output.** Block per finding showing the function signature, the open call (highlighted), the closing line, return statements, and any goto labels. **Findings:**
  - `category: "missing_counterpart"`, severity `warning` — open with no close anywhere in function (and no language-level cleanup keyword like `defer`/`finally` or RAII type).
  - `category: "close_only_on_error_path"`, severity `info` — close only inside C/C++ goto label sections; normal-return path leaks.
  - `category: "missing_close_on_error_path"`, severity `warning` — forward `goto label` after the open, but the label's reachable section doesn't free the resource.
  - `category: "double_close"`, severity `warning` — close call inline before goto AND in the goto target's reachable section.

**Limitations.** Does **not** compare across peer functions (that's `peer`). Pattern table is open-coded — adding new pairs requires a code change. Goto-aware analysis is C/C++ only. RAII detection looks for textual keywords (`std::lock_guard`, `std::unique_ptr`, etc.) on any line in the function — not type-aware, so a comment containing those tokens would suppress the finding.

**Source.** `src/algorithms/absence_slice.rs`

### 20. ResonanceSlice

**Flag:** `-a resonance` · `-a resonanceslice`

**Question.** "Are there files that historically co-change with these but are missing from the diff?"

**Mechanism.** Requires git. Walks commits in the time window, tracks file co-occurrence per commit (skipping commits touching > 50 files to avoid merge / format storms). Only considers commits that touch at least one diff file. For each diff file, reports partners with co-change count ≥ `min_co_changes` (default 3) and ratio ≥ `min_ratio` (default 0.3).

**When to use.** Spot omissions. "Whenever `auth_handler.go` changes, `auth_test.go` usually changes too — is the test update missing?"

**CLI.** `slicing --repo . --diff changes.patch -a resonance`

**Output.** One block per missing partner: function signatures of the missing file (if parsed) plus the diff lines from the file that's coupled to it. No findings (each block IS the finding).

**Limitations.** The per-file commit counter only tracks commits that also touch a diff file — the ratio is conditional, not unconditional, which overstates coupling for shared utility files that change frequently on their own. Default 180-day window is hardcoded in `ResonanceConfig::default()` — change the source to override. Requires `git` + a git repo; throws an error otherwise.

**Source.** `src/algorithms/resonance_slice.rs`

### 21. SymmetrySlice

**Flag:** `-a symmetry` · `-a symmetryslice`

**Question.** "If I changed `serialize`, did I also change `deserialize`?"

**Mechanism.** A table of symmetric pairs (`serialize↔deserialize`, `encode↔decode`, `lock↔unlock`, `subscribe↔unsubscribe`, `to_json↔from_json`, etc. — see `src/algorithms/symmetry_slice.rs:24`). For each diff function whose name contains a left-side pattern, generates candidate counterpart names and searches all files. If the counterpart exists but isn't in the diff, emits a finding.

**When to use.** Format/encoding/lifecycle pair review — protocol changes, serialization version bumps, paired resource patterns.

**CLI.** `slicing --repo . --diff changes.patch -a symmetry`

**Output.** Block containing the changed function plus its counterpart's full body. **Finding:** `category: "broken_symmetry"`, severity `concern` — "X changed but Y was not."

**Limitations.** Counterpart name generation is naive substring replacement (e.g. `serializeUser` → `deserializeuser`, lowercased). Names with non-standard casing may not match. Only flags asymmetric changes — symmetric (both sides changed) doesn't produce a finding.

**Source.** `src/algorithms/symmetry_slice.rs`

### 22. GradientSlice

**Flag:** `-a gradient` · `-a gradientslice`

**Question.** "How relevant is each line to this change, on a continuous scale?"

**Mechanism.** **CPG-required.** Seeds diff lines at score 1.0, BFS through CPG `DataFlow` / `Call` / `Return` edges with score = `decay^hop` (default decay 0.6). Stops when score drops below `min_score` (default 0.1) or hop count exceeds `max_hops` (default 5).

**When to use.** When you want a *ranked* slice for an LLM to triage — no binary include/exclude. The consumer picks its own cutoff.

**CLI.** `slicing --repo . --diff changes.patch -a gradient`

**Output.** Lines sorted by score, grouped by file. Block-level the same as other algorithms; the score is internal (used for ordering). No findings.

**Limitations.** DataFlow edges are intraprocedural — gradient crosses function boundaries via `Call` / `Return` edges only. Configuration is hardcoded in `GradientConfig::default()`; no CLI knobs.

**Source.** `src/algorithms/gradient_slice.rs`

### 23. ProvenanceSlice

**Flag:** `-a provenance` · `-a provenanceslice`

**Question.** "Where did each variable on this line ultimately come from — and does that origin require special handling?"

**Mechanism.** **CPG-required.** For each variable on a diff line, walks `dfg.all_defs_of` and `dfg.backward_reachable` to find the ultimate definition site, then classifies the line via a set of keyword tables: user input, database, hardware (C/C++ embedded), config, env var, function param, constant, unknown. Python-specific: imports from non-web-framework modules (e.g. `from mylib import request`) are sanitized so `request` doesn't get classified as `UserInput`.

**When to use.** Trust-boundary analysis. "Is this variable user-controlled? Then it needs validation before X."

**CLI.** `slicing --repo . --diff changes.patch -a provenance`

**Output.** Block per variable showing the use site, origin site, and any intermediate definition lines. **Findings** (severity by origin):
  - `UserInput` / `Hardware` → `concern`.
  - `Database` / `ExternalCall` → `warning`.
  - `FunctionParam` / `EnvVar` / `Config` → `info`.
  - `Constant` / `Unknown` → no finding.
  Category: `untrusted_origin`.

**Limitations.** Keyword-based classification is conservative — `request` matches anywhere unless suppressed (Python web-framework suppression handles the most common false positives). `~`-prefixed patterns require a word boundary (`~form` doesn't match `transform`). HCL `var.` and `data.` are treated as user input and database respectively, which fits Terraform but may surprise users from other contexts.

**Source.** `src/algorithms/provenance_slice.rs`

### 24. PhantomSlice

**Flag:** `-a phantom` · `-a phantomslice` · `-a ghost`

**Question.** "Is this change about to depend on something that was deleted in recent commits?"

**Mechanism.** Requires git. Walks recent commits with `--diff-filter=D` (deletions), extracts function-name-shaped tokens from the deleted file content, then checks whether any identifier on a current diff line matches a recently-deleted name.

**When to use.** Catch the case where someone removed a utility and another developer is unknowingly relying on its absence still being correct. Most useful in active codebases with frequent deletions.

**CLI.** `slicing --repo . --diff changes.patch -a phantom`

**Output.** Block per matched deletion containing the diff lines that reference the deleted name. Block `modify_type` is `Deleted`. No findings (the block IS the signal).

**Limitations.** Function-name extraction is heuristic and per-language (`def`, `function`, `func`, Java accessors, C/C++ `[type] name(`). Identifier matching is exact and case-sensitive on tokens longer than 2 characters. A live, currently-existing function with the same name as a previously-deleted one will *also* match — phantom can't distinguish. Default look-back is 50 commits.

**Source.** `src/algorithms/phantom_slice.rs`

### 25. MembraneSlice

**Flag:** `-a membrane` · `-a membraneslice` · `-a boundary`

**Question.** "If I'm changing this exported function's contract, who across module boundaries calls it — and do they handle errors?"

**Mechanism.** **CPG-required.** For each diff function, finds callers in *other files* (cross-module by file boundary), respects C/C++ static linkage, includes the call site plus surrounding context, and scans the caller's body for ~80 error-handling patterns (try/catch, `if err`, `?`, `unwrap_or`, NULL checks, asserts, RAII smart pointers, optional/expected, etc.).

**When to use.** API-change review. "I'm changing this public function — which callers in other files need a heads-up, and which of them don't have error handling?"

**CLI.** `slicing --repo . --diff changes.patch -a membrane`

**Output.** Block per (changed function, cross-file caller) showing both. **Finding:** `category: "unprotected_caller"`, severity `concern` — call site without any error-handling pattern in the caller's body.

**Limitations.** "Cross-file" is the boundary, not "cross-module" in the package sense — files in the same package are still considered separate modules. Error-handling detection is keyword-based across the entire caller body, not just around the call site, so a try/catch elsewhere in the caller suppresses the finding even if the specific call isn't wrapped.

**Source.** `src/algorithms/membrane_slice.rs`

### 26. EchoSlice

**Flag:** `-a echo` · `-a echoslice` · `-a ripple`

**Question.** "If this change subtly altered behavior (return value, error semantics), do downstream callers handle the new behavior?"

**Mechanism.** **CPG-required.** For each diff function, classifies the change as touching `return` and/or `error` (raise/throw/`return err`/`return -1`/`return NULL` etc.). Walks callers (depth 2), grabs ~10 lines of context around each call site, and checks for ~50 safe-handling patterns. Emits a finding for callers without a safe pattern when the diff touched the relevant axis.

**When to use.** "I added a new error case — who needs to update their call sites?" or "I changed this from non-null to nullable — who needs a null check now?"

**CLI.** `slicing --repo . --diff changes.patch -a echo`

**Output.** Block per affected caller with the changed function signature plus the caller's body and the call site highlighted. **Finding:** `category: "missing_error_handling"`, severity `warning` — description lists which axes (return value, error handling) are not handled.

**Limitations.** Context window for safe-pattern detection is 3 lines before to 5 lines after the call site — far-away try/catch wrapping the whole function is missed. Pattern list is per-language but flat (no AST scoping).

**Source.** `src/algorithms/echo_slice.rs`

### 27. ContractSlice

**Flag:** `-a contract` · `-a contractslice`

**Question.** "Does this change weaken the function's implicit contract — pre-conditions, post-conditions, return shape?"

**Mechanism.** Two phases:
- **Phase 1+2 (always):** Extracts preconditions from guard clauses in the first 30% of the function body (any `if (cond)` whose body has an early exit — return/raise/throw/panic — or any assert/require). Classifies the constraint as `non-null`, `nil-check`, `non-empty`, `type-check`, `range-check`, `positive`, or `assertion`. Extracts postconditions by classifying every return value (null, bool, numeric, string, empty collection, Go error pair, expression) and labels the function as `always-non-null`, `consistent-type`, `nullable`, `non-null-or-throws`, `go-result-pair`, `void`, `always-bool`, or `mixed`.
- **Phase 3 (`--old-repo`):** Same extraction on the old version, then per-function diff. Classifies precondition changes as Removed (weakened), Added (strengthened), or Modified; postcondition changes as null-path-added (weakened), null-path-removed (strengthened), type-changed, or kind-changed (with weakening direction inferred from a transition table).

**When to use.** Refactor review. "Did this change accidentally remove a NULL guard? Did it add a new null return path?"

**CLI.** Without history: `slicing --repo . --diff changes.patch -a contract`. With history: add `--old-repo /path/to/old/checkout`.

**Output.** Block containing the function signature, the diff lines, and the guard clause lines. **Findings:**
  - `category: "contract"`, severity `info` — summary of all detected pre/postconditions for the function.
  - `category: "contract_postcondition"`, severity `info` — postcondition summary.
  - `category: "contract_violation"`, severity `warning` — a guard clause or return statement on a diff line.
  - `category: "contract_postcondition_new_null"`, severity `warning` — Phase 1+2: new `return None/null` on a diff line in a function that otherwise returns non-null.
  - With `--old-repo`: `contract_precondition_weakened` / `_strengthened`, `contract_postcondition_weakened` / `_strengthened`.

**Limitations.** Guard clause detection requires the guard to be in the first `max(30%, 5)` lines of the function. Postcondition classification of expressions (variables, function calls) reduces to "expression" — which means most real-world functions get classified as Mixed unless their returns are obvious literals. Phase 3 matches functions by name only — renaming a function looks like (delete, add) rather than rename.

**Source.** `src/algorithms/contract_slice.rs`

### 28. PeerConsistencySlice

**Flag:** `-a peer` · `-a peer_consistency`

**Question.** "Are there sibling functions in this file that share a first-parameter type and uniformly skip a NULL guard the rest do?"

**Mechanism.** **C/C++ only.** Clusters functions in the same file by first-parameter identifier name (proxy for type identity). For each cluster the diff touches: cluster size ≥ 3 required. Emits if (a) ≥ 80% of siblings dereference the first param via `p->`/`(*p)`/`fn(p, …)` and zero contain a NULL guard (`if (p)`, `if (!p)`, `if (p == NULL)`, `assert(p)`), or (b) ≥ 3 siblings guard the param and ≥ 1 dereferences without guarding (divergent).

**When to use.** C/C++ dispatcher-NULL bugs (see CVE-2025-61102). When you change one of N siblings that share a parameter shape, this surfaces the cluster so the reviewer can ask "do they all handle NULL the same way?"

**CLI.** `slicing --repo . --diff changes.patch -a peer`

**Output.** One block per (file, first-param-name) cluster — first 8 lines of every cluster member side-by-side. **Findings:**
  - `category: "peer_guard_divergence"`, severity `concern` (uniform-unguarded) or `warning` (divergent).

**Limitations.** C/C++ only. Same file only. Exact identifier-name equality only — `void f(struct vty *vty)` and `void g(struct vty *v)` cluster separately. Cluster thresholds (≥3 size, 80% deref ratio, 0 guards for uniform; ≥3 guards + ≥1 divergent for divergent) are hardcoded.

**Source.** `src/algorithms/peer_consistency_slice.rs`

### 29. CallbackDispatcherSlice

**Flag:** `-a callback` · `-a callback_dispatcher` · `-a dispatcher`

**Question.** "This function is stored into a struct field somewhere — who actually invokes that field, and do they pass NULL as an argument?"

**Mechanism.** **C/C++ only.** Two registration kinds: (1) `RegKind::Field` — direct struct-field assignment (`.field = func`, `obj.field = func`, `obj->field = func`, designated initializers); (2) `RegKind::CallArg` — function passed as an argument to a callee whose name (lower-cased) contains `register` or `_functab`, or equals `g_signal_connect`. For each `Field` registration, scans all C/C++ files for invocations like `x->field(args)` or `tab[i].field(args)`, checking whether any argument is a literal `NULL` / `nullptr` / `0`.

**When to use.** Catches what regular call-graph slicing misses — function-pointer-in-struct dispatch. Original motivation: FRR CVE-2025-61102, where `show_vty_*` is registered into `functab->show_opaque_info`, then `lib/log.c` invokes the field with `vty=NULL`.

**CLI.** `slicing --repo . --diff changes.patch -a callback`

**Output.** Block containing the diff function definition plus each registration site plus each invocation site. **Findings:**
  - `category: "callback_null_arg_dispatch"`, severity `concern` — registered + invocation site passes NULL.
  - `category: "callback_dispatcher_chain"`, severity `info` — registered + invoked, no NULL literal.
  - `category: "callback_registrar_call"`, severity `warning` — registered via call-arg path; dispatcher lives inside the registrar, can't see invocations.

**Limitations.** C/C++ only. Computed registrations (`tab[idx_for(name)] = func`) are silently invisible — no marker. Field-pattern path requires both registration AND at least one invocation, so registered-but-never-dispatched is filtered out (also no marker).

**Source.** `src/algorithms/callback_dispatcher_slice.rs`

### 30. PrimitiveSlice

**Flag:** `-a primitive` · `-a primitiveslice`

**Question.** "Does this change introduce or contain a known-bad security primitive shape — short hash, weak hash for identity, `shell=True` interpolation, disabled cert validation, hardcoded secret?"

**Mechanism.** Whole-file scan (not just diff lines) of every diff-touched file. Six rules, each producing a categorized finding with a `rule_id`. Rule 1b is two-pass: collects functions whose body is `<digest>.hexdigest()[:PARAM]`, then scans all diff-touched files for call sites that pass a literal int < 32 at PARAM's position (positional or keyword).

**When to use.** Lightweight security sweep. Most other algorithms answer "does data flow from X to Y?"; this answers "does this code contain a known-bad shape?" Many fingerprints are independent of data flow.

**CLI.** `slicing --repo . --diff changes.patch -a primitive`

**Output.** Block + finding per detection. **Findings** (categories are rule IDs):
  - `HASH_TRUNCATED_BELOW_128_BITS` — `<digest>.hexdigest()[:N]` with N < 32, or `<digest>.digest()[:N]` with N < 16.
  - `HASH_TRUNCATION_VIA_CALL` — two-pass: callee truncates, caller passes literal < 32. Includes the truncation site as `related_files` / `related_lines`.
  - `WEAK_HASH_FOR_IDENTITY` — `hashlib.md5(…)` / `hashlib.sha1(…)` assigned to a name matching `*_id`, `*_key`, `*_hash`, `*_token`, `cache*`, `session*`, `ident*`, `fingerprint*`.
  - `SHELL_TRUE_WITH_INTERPOLATION` — `subprocess.{run,Popen,call,check_call,check_output}` or `os.system(` with `shell=True` AND the command arg contains an f-string, `.format(`, or `%` interpolation.
  - `CERT_VALIDATION_DISABLED` — `verify=False`, `ssl.CERT_NONE`, `_create_unverified_context(`, `CURLOPT_SSL_VERIFYPEER, 0`, `rejectUnauthorized: false`, `InsecureSkipVerify: true`, etc.
  - `HARDCODED_SECRET` — assignment to a credential-shaped name with a non-placeholder string literal.
  Severity: `concern` if on a diff line or in a function that contains a diff line; `suggestion` otherwise. The reviewer is expected to calibrate from blast radius.

**Limitations.** Two-pass truncation rule (`HASH_TRUNCATION_VIA_CALL`) is Python-only. Most rules are Python-shaped; cert validation and weak hash identity rules have a few non-Python markers but are still primarily tuned for Python. No cross-procedural data flow — if the literal `12` is computed at runtime, the rule doesn't fire.

**Source.** `src/algorithms/primitive_slice.rs`

---

## Picking an algorithm

| Goal | Algorithm |
|------|-----------|
| Default — see what data the change touches | `leftflow` |
| Most focused (no control flow) | `thin` |
| Cross-version structural diff | `delta` (needs `--old-repo`) |
| Security sweep | `taint`, `provenance`, `primitive` |
| Resource leaks / paired ops | `absence`, `symmetry` |
| API contract changes | `contract`, `membrane`, `echo` |
| Dispatcher / callback bugs (C/C++) | `peer`, `callback` |
| Async / concurrency races | `quantum`, `circular` |
| Cross-cutting concern audit | `angle`, `horizontal`, `vertical` |
| Risk triage on big diffs | `3d` (needs git), `gradient` |
| Detect missing co-changes | `resonance` (needs git) |
| Detect deleted code dependencies | `phantom` (needs git) |
| Adaptive depth — let it decide | `spiral` |
| All non-git algorithms | `review` (preset suite) |
| Everything | `all` |

## Combining algorithms

Algorithms can be combined in a single run with comma-separated names: `-a "leftflow,taint,absence,contract"`. Each algorithm's blocks and findings are returned in the same `MultiSliceResult`, with per-algorithm `errors` captured if any individual algorithm fails. The CPG is built once and shared; running 5 CPG-required algorithms costs roughly the same as running one.

## Algorithms requiring CPG vs AST-only

Use `SlicingAlgorithm::needs_cpg()` (`src/slice.rs:209`) as the source of truth.

- **CPG-required** (DFG + CFG + call graph): `leftflow`, `fullflow`, `relevant`, `conditioned`, `barrier`, `chop`, `taint`, `delta`, `spiral`, `circular`, `vertical`, `3d`, `gradient`, `provenance`, `membrane`, `echo`.
- **AST-only**: `originaldiff`, `parentfunction`, `thin`, `quantum`, `horizontal`, `angle`, `absence`, `resonance`, `symmetry`, `phantom`, `contract`, `peer`, `callback`, `primitive`.

For test suites that exercise many algorithms, AST-only ones can be batched without CPG construction overhead.

## See also

- [`SLICING_METHODS.md`](SLICING_METHODS.md) — theoretical taxonomy and motivation for each algorithm class.
- [`docs/cpg-architecture.md`](docs/cpg-architecture.md) — how the unified CPG is built.
- [`docs/SPEC-ALGO-LANGUAGE-COVERAGE.md`](docs/SPEC-ALGO-LANGUAGE-COVERAGE.md) — algorithm × language test coverage spec.
- [`README.md`](README.md) — one-line algorithm summaries and CLI flag reference.
