# CLAUDE.md — Instructions for AI Assistants Working on This Codebase

## Project Overview

Rust implementation of 30 code slicing algorithms for defect-focused automated
code review. Based on arXiv:2505.17928 plus the established program slicing
taxonomy and novel theoretical extensions.

## Build & Test

```bash
cargo build          # Build the project
cargo test           # Run all tests (unit + integration) of the default members: prism (core) and prism-review
cargo fmt --all --check  # Check formatting of every workspace crate (must pass before PR)
cargo run -- --help  # Show CLI usage
cargo run -- --list-algorithms  # List all 30 algorithms
cargo build -p prism-mcp --bin prism-mcp    # Build the MCP stdio server
cargo test --workspace                      # Run tests for prism, prism-review and prism-mcp
```

### Mutation gate
Locally, mutgate waits for live host-quiet markers before starting; `--plan-only`, `--no-quiet-wait` or `HOST_QUIET_IGNORE=1` skips waiting (see `scripts/hostquiet/README.md`).
```bash
python3 scripts/mutgate/mutgate.py --since main --scope fn    # fast ADVISORY check each review round
python3 scripts/mutgate/mutgate.py --since main --scope fn --authoritative  # pre-merge default, plus lanes the PR adds
python3 scripts/mutgate/mutgate.py --lane mutants/<lane>.json # a lane the PR adds or changes in substance, or a dependent lane
python3 scripts/mutgate/mutgate.py                            # full authoritative gate (tier 3 only); default 4 workers
```
Pre-merge gate, on the final head (tiered):
1. **Every code PR:** `--since <merge-base> --scope fn --authoritative` (a stale local `main` ref selects
   extra mutants), plus each lane the PR adds. The `--since` run also selects registered mutants whose anchors
   no longer match, even outside the diff. A lane the PR re-anchors onto code the PR itself moved needs no run
   of its own when `--since <merge-base> --scope fn --only <ids> --plan-only` selects every re-anchored id;
   otherwise (e.g. repairing an anchor broken earlier on main) run those ids. A lane edit that only removes
   mutants needs no run. A lane whose mutant snippets or tests the PR changes in substance (anything beyond
   re-anchoring or removal) still runs.
2. **Dependent lanes, on a named trigger:** a change to resolution results that other tests rely on
   (especially removing Exacts), or a change to where code executes or to test oracles, thread-local
   switches or `cfg(test)` seams. Add only the lanes of dependent code that covers the changed language or
   subsystem (find it with callers/grep) and name the trigger in the PR. A change counts as one language only
   when the changed code runs only for that language; shared code (the resolution ladder, free-function
   pools, cache format) counts as every language that reaches it.
3. **Full gate:** only for cross-cutting infrastructure whose dependents cannot be listed (build pool and
   execution context, shared test helpers, the mutgate driver, CPG cache format, wide `cfg(test)` plumbing,
   PRs spanning many subsystems), plus a periodic sweep of main in a quiet window.
Scopes do not trace dependency effects; tier 2 and the periodic full sweep cover that gap.
Gates run by default in a Claude Code cloud session, and its verdicts count for every tier; a local run is
the fallback when the cloud route is unavailable (host-quiet waits and quiet windows apply to local runs only).
Full and authoritative scoped verdicts use isolated text builds with only each
mutant's own edits and green source baselines in the shared and worker contexts;
TIMEOUT/INADMISSIBLE fail.
Scoped schema results are labeled ADVISORY (`authoritative: false`), can falsely
kill equivalent location-observing mutants, and cannot certify a pre-merge gate.
Exit 0 means all selected verdicts are
KILLED under the stated contract. Warm lane-P target: at most 5 minutes at 4 jobs.
Details (lane format, cleanup, measurements, limits): `scripts/mutgate/README.md`.
DFG compact regression witnesses run in the ordinary library, navigation and output
targets, and in the prism-mcp crate (`cargo test -p prism-mcp`). The accepted migration oracle has been retired;
`mutants/dfg-compaction.json` retains its normal-target mutation obligations.

## Accuracy Harness (Tier-A)

When a change touches call resolution, navigation queries, or CPG construction
(`src/call_graph.rs`, `src/call_graph/`, `src/navigation/`, `src/cpg/`, `src/ast.rs`, `src/ast/`):

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut   # seconds, no LSP — run before committing
cd eval && uv run tier-a --quick --allow-stale-sut         # minutes, needs rust-analyzer — before review
```

Use `--allow-stale-sut` only with an immediate preceding rebuild in the same
worktree; it is for the normal dirty pre-commit state, not stale binaries.
Paste regressions/flip-candidates into the PR description rather than re-baselining.
Full multi-corpus runs (`uv run tier-a --corpus all`) are human-triggered; see
`eval/README.md`. The committed baseline lives in `docs/eval/tier-a/`.

### Before Creating a PR

Always run these checks before committing/pushing:
```bash
cargo fmt --all      # Fix formatting
cargo test           # All tests must pass
```

Run specific test suites (one umbrella binary per tests/ subdirectory; filter by
file-derived module name):
```bash
cargo test --test algo_paper                       # Paper algorithm tests
cargo test --test algo_taxonomy taint_cve_test::   # Taint CVE tests
cargo test --test lang_c algo_test::               # C language-specific tests
cargo test --test cli validation_test::            # CLI validation tests (prism-review crate)
cargo test --test integration core_test::          # Core integration tests
cargo test --test integration coverage_test::      # Coverage matrix
```

## Code Organization

### Core Modules (`src/`, the `prism` library)

- `lib.rs` — Public API; re-exports all modules.
- `ast.rs` — Tree-sitter AST wrapper (`ParsedFile`). All tree-sitter interaction goes through here.
- `cfg.rs` — Control Flow Graph construction from tree-sitter AST (intraprocedural CFG edges).
- `cpg.rs` — Code Property Graph (`CpgContext`): unified graph merging AST, DFG, call graph, and CFG. This is the main interface that algorithms use.
- `call_graph.rs` — Cross-file call graph with forward/reverse edges and cycle detection.
- `data_flow.rs` — Def-use chains, reachability queries, chopping, taint propagation.
- `access_path.rs` — Structured variable access paths for field-sensitive data flow analysis (e.g., `x`, `dev->name`, `self.config.timeout`).
- `type_db.rs` — Optional C/C++ type enrichment from `compile_commands.json` + clang. Provides struct definitions, field types, typedefs, and class hierarchy.
- `languages/mod.rs` — Language-specific node type mappings. Add new languages here.
- `diff.rs` — Diff parsing and `DiffInput`/`DiffInfo` types.
- `slice.rs` — `SlicingAlgorithm` enum (30 variants), `SliceConfig`, and `SliceResult`.
- `reasoning/` — Tier-2 reasoning layer: `taint_trace` consumer, `SeedSet`, output shaper.
- `api/` — `prism::api`: the stable, `#[non_exhaustive]` core facade of the `prism` library (`build_info`, `nav_session`/`callers`/`callees`, `OwnerOptions`, `load_review_inputs`, `build_context`, `ReviewOptions`, the finding-confidence items, `with_build_pool`). The review runner, SARIF and targets items are not in it: they live in `prism_review::api` (see "Review crate" below). See the module doc for the compatibility promise.
- `finding_confidence.rs` — Single source for per-finding confidence/tier/parse-quality classification (`classify`, `evidence_files`, `ParseQuality::min_over`), consumed by both prism-review's `output/sarif.rs` and `targets/`.

### Review crate (`crates/prism-review/src/`)

The review layer is the workspace crate `crates/prism-review` (package `prism-review`, library `prism_review`, binary `prism`; #734, the 4.0.0 split). It depends on the `prism` core library; the core never depends on it. Its tests are `crates/prism-review/tests/cli/` (the `cli` umbrella target) plus its inline `src/` unit tests.

- `output/` — Output formatters: `mod.rs`, `review.rs`, `review_compact.rs`, `mermaid.rs`, `sarif.rs` (SARIF 2.1 serializer), `sarif_rules.rs` (rule descriptions + pure mapping helpers), `sarif_model.rs` (serde model structs). Nav evidence rendering lives in core (`src/navigation/render.rs`) and is re-exported as `output::navigation`, so `output::navigation::` paths resolve.
- `cli.rs` — The clap `Cli` derive tree (`Cli`, `ReviewArgs`, `Command`, `NavArgs`, `NavQuery`, `TargetsArgs`); `pub mod cli` in `lib.rs` so `crates/prism-review/tests/cli/readme_test.rs` can parse-only via `Cli::try_parse_from` without executing the binary.
- `main.rs` — CLI entry point: `main()`, `run_review`/`run_nav`/`run_targets`, and the format-arm dispatch; the clap grammar itself lives in `cli.rs`.
- `algorithms/` — All 30 slicing algorithms. Each is self-contained.
- `algorithms/taint_sinks/` — taint's flat sink matcher (#622 S1), a hybrid policy. JS/TS/TSX, Python and Go sinks are decided from the AST: the identifier must be the callee of a call (or the target of an assignment for DOM/location sinks) — `callee.rs` finds the call and reads receiver chains. For high-severity names (exec/spawn, query/execute/raw, system/popen/remove, send/write, eval/Function, Command, HTML, redirect) an UNKNOWN receiver keeps firing; only PROVEN non-sinks suppress: a regex literal/`new RegExp`/binding initialised with one, loggers, message channels and stdio, a collection or file-object binding (every binding form of the name counts toward one file-wide total: plain, annotated and tuple-unpacking assignment, `with .. as`, `for` targets including JS `for (x of ..)`, walrus and augmented assignment, JS destructuring and compound assignment; more than the one proving binding, or any rebinding of a parameter, proves nothing, so the receiver is unknown and the sink fires. A Python annotation counts by its base identifier only, generic arguments ignored, and a union only when every non-`None` alternative is a collection), a `def`/function declared in this file under the same name, zero-argument calls (`model.eval()`, `u.Query()`), request-typed Go receivers (SQL names only; framework-context `HTML` still fires, a 3-argument value call being tagged `go-html-render` and graded weak), imported Go packages other than the rule's (`cobra.Command`), a constant `Sprintf` format. Go container-named receivers (`cache`, `l`, `list`) suppress `Remove`/`Rename`-style file ops only without a declared parameter or `var` type, or with a declared container type (`*lru.Cache`, `*list.List`); a file-system type (`afero.Fs`, `*os.File`) fires, as does a local initialised by a file-package constructor (`afero.NewOsFs()`, `os.Open(..)`). Struct-field receivers (`s.cache`) are not typed and stay name-only. Python `query` fires when its SQL argument (positional, or a `query=`/`sql=`/`statement=` keyword value) is (after peeling parentheses) an f-string, a `%` operation, a `+` with a string/identifier operand, a `.format` call or a lowercase variable; attribute-only arithmetic (`User.a + User.b`) and `session.query(Model)` stay ORM builders. Go `if`/`for`/`switch` init statements and `range` clauses are scopes. Receiver allowlists (response chains, db handles, `child_process` bindings, import-path-resolved Go packages) are positive evidence only: they pick the rule and CWE tag, never gate. Bindings are resolved lexically (`js_scope.rs`, `py_scope.rs`, `go_scope.rs`), so a shadowing parameter or local never inherits a module-level fact. Python `.where`/`.filter` with a positional argument stays a (weak) SQL clause sink by owner decision. Other languages keep the whole `SINK_PATTERNS` substring list (`legacy.rs`) except `SHELL_ONLY` command names (gated to Bash) and `get_user` (exact, C/C++); library names such as `sqlite3`, `chmod`, `chown`, `curl`, `uci`, `mtd`, `fw_setenv` are not gated. JS structured `exec`/`execSync` sinks carry a semantic check (`js_exec_call_is_command`). Sink strength (#636): each hit carries `Strength`, and `SinkClassifier::line_strength` (`taint_sinks/strength.rs`) grades the `taint_sink` finding at emission. Weak requires proof; anything unproven stays Strong. A line is Weak only when every rule-recognised sink call on it (and on the calls enclosing it, for lines inside a multi-line call) is weak, no caller-supplied or wrapper sink name is on it, and no non-SQL structured pattern (SSRF, path, command, ... `taint_sinks/structured.rs`, every pattern, no flow path) matches it; JS/TS/TSX, Python and Go only, other languages are always Strong. Weak shapes: SQL calls (`js-db-call` for the SQL-text-in-argument-0 names only, never `createQueryBuilder`; `py-db-call`; `go-db-call`) whose query argument is constant text (a string literal without interpolation, or a name with exactly one binding in its function, or in the file for a module-level name, never written: Go `const`, `query := ".."`, `var query = ".."` (a multi-name `var a, b = x, y` pairs each name with its own initializer and gives none when the counts differ); JS single declarator with parameters, shorthand destructuring bindings and `catch` parameters counted as declarations in the strength proof only (the shared resolver keeps main's presence behaviour) and no write, so a later `var` redeclaration, a parameter of the same name or `:=` redeclaration is Strong); Python SQL (design A, a closed allow-list in `taint_sinks/sqla_tables.rs`, the grader in `sqla.rs`, `sqla_value.rs` and `sqla_chain.rs`; the old raw-SQL deny-list is gone): `execute`/`query` weak only when its argument proves as a statement (S), and a `.where`/`.filter`/`.filter_by` hit (`py-sql-clause`, the tag is shared with the strong `text(<non-literal>)` hit; strength is per hit) weak only when its own arguments and the whole method chain around it prove (maximal chain: methods applied to its result and methods of its receiver are on the `CHAIN` table with proven arguments; the root may be built elsewhere, a parameter, an attribute or an unknown call, but a library statement or a local is proven and a local bound more than once is Strong; a root that calls a same-file `def` (one binding, undecorated) or a module-level lambda, or `self.m`/`cls.m` resolved through the class and its same-file bases (a method defined more than once, conditionally, or overridden in a same-file subclass is Strong), proves only when every `return` passes the same chain rule, to depth 2, and every call argument is a value or a proven chain; recursion, a bare return, no return or another assigned callee is Strong; `sqla_helpers.rs`). Every argument has a position class: C clause (becomes SQL structure: only column references, literals, comparisons, boolean and arithmetic over those, and calls to table constructs and column methods), V value (a bound parameter), F from, S statement, L literal, T type, O loader; anything not on a table is unproven. A raw constructor (`text(<non-literal>)`, `literal_column`, `column`, `table`, `Column(<non-literal>)`, `SQL`, `RawSQL`, `.extra`, `.op`, `prefix_with`/`suffix_with`, `with_hint`/`with_statement_hint`/`from_statement`, `getattr(func, x)`, a `*list` or `**dict` splat in a C position) is never admitted, and a belt of raw-API names refuses a callee of that name whatever its receiver (`db.literal_column(x)`); the same belt is applied to the whole line (`SinkClassifier::py_line_has_raw`, strength only). Library identity: a name is SQLAlchemy/peewee/Django only when EVERY binding of that spelling in the file (`py_scope::Sites`, a census over assignment, annotated and augmented targets, unpacking, `for` and comprehension targets, `with`/`except .. as`, walrus, imports, `def`/`class`, parameters of functions and lambdas, `global`/`nonlocal`, match captures, `del`, type aliases) is an import of the same member of `sqlalchemy`(.sub) / `peewee` / `django.db.models`(.sub), an exact package boundary (`sqlalchemy_utils` is not `sqlalchemy`); the original member name selects the table entry, so an alias cannot hide `literal_column`. A local name resolves to exactly one binding in the nearest enclosing scope (function, lambda, comprehension, module; class bodies only for code directly in them), at most 8 hops deep (D3), else Strong (a proof that held is memoised by initializer, so a name used many times is proven once; the grader also has a fixed step budget, beyond which the answer is Strong); a name bound more than once, a loop target in a C/S/F position, an unannotated parameter in a C/S/F position and `self.x`/`cls.x` roots are Strong. D2: a never-rebound parameter annotated `Table`/`Select`/`Subquery`/`Alias`/`CTE`/`FromClause`/`TableClause`/`Join`/`type[..]` is a schema root, and one annotated `Select`/`Insert`/`Update`/`Delete`/`CompoundSelect` is a statement (`Executable` is not proof). A schema root is also an imported or star-imported name that no scope rebinds, a class, a module-level object, or a local bound once to a proven S/F. Django sub-grammar (D4): `Q(**V)`, `~Q`, `Q & Q`, `Q | Q`, `Exists(S)`, `Subquery(S)`, `OuterRef(L)`, `F(L)`, `Value(V)`, `exclude`/`using`/`values_list`; nothing else of `django.db.models` is admitted. Contract (D1): a value position binds the Python object it receives, so a SQL construct that reaches a V position from outside the function (a parameter, the return of a cross-module function, an attribute set elsewhere, a module imported from elsewhere) is out of model, as runtime mutation is for `Exact`; everything the function or file builds itself is checked: a local in a V position resolves to its initializer (a loop or `with` target is data, except that an iterable built in place, a display or comprehension, directly or through one local name that no mutator fills with a non-value, is still checked, D1b), a same-file function (or a `self.m`/`cls.m` method) is proven by its returns (two levels), an object the function stores to (`o.x = ..`, `o[k] = ..`, `setattr`, or a mutator method such as `append`/`update` with a non-value argument) is Strong, and a local container (a display, a comprehension, or a container-constructor or same-file-function call) is data only when every other occurrence of its name is an allow-listed one (`name.append|extend|add|insert|update|setdefault|appendleft|extendleft(values)`, the iterable of a `for`, an argument of `len`/`list`/`tuple`/`set`/`sorted`/`str`, a read-only truth test or `in` right operand; adders count only as whole statements and a container holding a nested display is Strong; `sqla_uses.rs`), any alias, other call, store, `del` or nested-scope occurrence being Strong, a `self`/`cls` attribute is checked against every assignment to it in the enclosing class (tuple targets, `setattr`, mutators on it) and a same-named property (D1a), a belt method on an opaque chain root is Strong on any line, a maximal-chain root bound by a conditional or boolean expression checks every branch while unpacking and walrus roots are Strong, an unknown call is admitted as data only when its callee and every argument are values (an import of a belt name or of a SQL-library member, or a module-qualified call rooted at one, is never data, even when file-wide identity trust failed), a listed column method needs its argument classes whatever the receiver, and an f-string in a V position is weak (`.ilike(f"%{q}%")`) while the same f-string in a C position is Strong; `tests/algo/taxonomy/taint_sink_strength_sqla_test.rs` pins the contract rows as Weak. Go's constant-query proof counts every tree-sitter-go binding or write node of the name (assignment, short var decl, var spec, `++`/`--`, `range`, `case q = <-ch` / `case q := <-ch`, a type-switch alias) and `&name`; a constant needs zero such bindings in the function, a variable exactly one; and reads a separate per-name initializer, so rule presence keeps the historical first-value pairing. Go `go-html-render` (3 arguments on a value receiver, weak only when the receiver's declared type is exactly `gin.Context` or `*gin.Context`, a package-qualified identity and not a substring; a status argument is no proof, so `plugin.Context`, an unknown `Ctx` and iris `HTML(format, a, b)` are Strong). A weak finding has severity `suggestion` instead of `warning`, so the existing mechanisms apply: `--format review` hides it under the default `warning` floor and `--review-min-severity suggestion` shows it, SARIF level is `note`, `--format json`/targets keep it (`targets[].id` hashes severity, so weak taint_sink target ids change once on upgrade). The tier is a view: the `taint_sink` set is unchanged. `--format review` adds `findings_below_floor` (per result and top level) counting hidden findings of severity `suggestion` or above, omitted when zero (`info` is never counted). Parameterised JS/Python `query`/`execute` calls whose taint rides only in bound parameters are already cleansed by the structured sanitizers and produce no finding, so they have no weak row. JS/TS/TSX also have import-resolved module identity (`js_modules.rs`, #637 A1): a lexically resolved import or `require` of `fs`/`fs/promises`/`fs-extra`/`graceful-fs` (`node:` forms, `.promises`, named and renamed imports) fires `js-fs` on the path-taking API, a single-binding `new NodeSSH()` local from `node-ssh` fires `execCommand`/`exec`, and `document.execCommand` on the unshadowed global document fires for `'insertHTML'` or a non-literal command; identity needs the innermost scope binding the name (`js_binding.rs`: params, loops, catch, hoisted `var`) to hold exactly one unwritten binding, type-only imports, a class expression's own name and `with` bodies prove nothing, and an fs hit needs a non-literal path argument; tables are positive evidence only, closed, with fields out of scope. Python module identity (#637 A2, `py_modules.rs`): closed per-module tables (paramiko `exec_command`, pexpect `spawn`/`run`/`PopenSpawn`, fabric/invoke `run`/`sudo`/`local`, jinja2 `from_string`; `jinja2.sandbox.SandboxedEnvironment` is weak) fire strong only when the callee is proven by an import visible at the use (every binding in its resolved scope is that import, no star import can replace it), or one hop from it (a single-binding local, a `with .. as` target, the first parameter of a fabric 2/invoke `@task`; a receiver that is itself a listed-class call counts; `from_string` needs a non-literal template); fields, annotations and rebinding prove nothing, and the tables never suppress. New sink shapes go in a rule table with matrix rows in `tests/algo/taxonomy/taint_sink_rules_*`, never as a bare name in `SINK_PATTERNS`; mutants in `mutants/taint-sink-rules.json`.
- `api/` — `prism_review::api`: the review facade. It re-exports all of `prism::api` and adds the algorithm runner (`review`, `run_review`, `run_algorithm`, `parse_algorithms`, `AlgorithmParams`, `ReviewOutcome`, `ReviewRun`, `annotate_finding_parse_quality`, `filter_result_findings`), SARIF (`to_sarif`, `SarifInputs`) and targets (`project`, `TargetsDocument`, `TargetsMeta`). Both facades carry the same `#[non_exhaustive]` and compatibility promise (within a major version every item keeps its name and signature; removal or signature change is preceded by a `#[deprecated]` release); 4.0.0 made the split and moved the review items to `prism_review::api`. `main.rs` is its first consumer.
- `targets/` — `prism targets`: projects findings into the instrumentation-targets contract (`mod.rs`: `project`/`TargetsMeta`/id/dedupe/warnings; `model.rs`: schema DTOs; `mapping.rs`: category table, hint parsers, language lowering). Schema at `docs/contracts/targets.schema.json`.

### Test Structure (`tests/`)

```
tests/
├── algo/              # Algorithm-specific tests
│   ├── novel/         # 8 test files (absence, echo, membrane, provenance + lang variants)
│   ├── paper/         # 1 test file (paper algorithms 6-9)
│   ├── taxonomy/      # 5 test files (taint CVE/lang/sink, misc, misc_lang)
│   └── theoretical/   # 4 test files (angle, quantum, spiral, vertical)
├── ast/               # AST infrastructure tests (access_path, binding, cpg, dfg, field)
├── common/            # Shared test helpers and fixture generators
│   └── mod.rs         # Re-exports core types + fixture generators per language
├── fixtures/          # Test data files (bash, c, python, terraform)
├── integration/       # Integration tests (call_graph, core, coverage)
└── lang/              # Language-specific tests
    ├── c/             # algo, complex, cve
    ├── cpp/
    ├── go/            # algo, advanced, lang
    ├── javascript/    # algo, destructuring, lang
    ├── lua/
    ├── rust/
    └── typescript/    # typescript, lang
```

Cargo.toml defines one umbrella test target per `tests/` subdirectory (e.g.,
`cargo test --test algo_paper`). The CLI tests (algo, output, validation, `readme_test`, ...) are the
`cli` umbrella target of the `prism-review` crate, in `crates/prism-review/tests/cli/`; the root
workspace's `default-members` include `prism-review`, so `cargo test --test cli validation_test::`
still works from the root (`cargo test -p prism-review --test cli ...` names it explicitly).
Shared test helpers in `tests/common/mod.rs` provide fixture generators like
`make_python_test()`, `make_javascript_test()`, etc.

### Language Coverage Matrix

`tests/integration/coverage_test.rs` contains a hardcoded list of test file
paths (`all_test_files`) that it scans for `fn test_*` names to build an
algorithm × language coverage matrix. **This list appears 3 times** — once in
each of `test_algorithm_language_matrix`, `test_language_coverage_minimum`, and
`test_coverage_matrix_validation`. When adding or renaming test files, all 3
copies must be updated or the matrix will under-report coverage. Run
`cargo test --test integration coverage_test::` to verify.

## Algorithm Implementation Map

### Paper (arXiv:2505.17928)
- `original_diff.rs` → Algorithm 6 (AnalysisOnlydiff)
- `parent_function.rs` → Algorithm 7 (AnalysisFunction)
- `left_flow.rs` → Algorithm 8 (AnalysisRelevantCode)
- `full_flow.rs` → Algorithm 9 (AnalysisRelevantCodeRHS)

### Established Taxonomy (SLICING_METHODS.md §4)
- `thin_slice.rs` → Data deps only, no control flow
- `barrier_slice.rs` → Depth-limited interprocedural (uses `call_graph.rs`)
- `chop.rs` → Source-to-sink paths (uses `data_flow.rs`)
- `taint.rs` → Forward taint propagation (uses `data_flow.rs`); the entry re-exports `slice` from `taint/run.rs`, and the pipeline lives in `taint/{run,report,flat,seeds,sink_match,cleansing}.rs` with per-language code under `taint/{js_ts,python,go}/` (#707)
- `relevant_slice.rs` → LeftFlow + alternate branch paths
- `conditioned_slice.rs` → LeftFlow pruned by value predicate
- `delta_slice.rs` → Two-version data-flow diff

### Theoretical Extensions (SLICING_METHODS.md §5)
- `spiral_slice.rs` → Adaptive-depth rings (composes other algorithms)
- `circular_slice.rs` → Cross-function cycle detection (uses `call_graph.rs` + `data_flow.rs`)
- `quantum_slice.rs` → Async state enumeration (language-specific pattern detection)
- `horizontal_slice.rs` → Peer pattern consistency (decorator/name/class matching)
- `vertical_slice.rs` → End-to-end feature path (uses `call_graph.rs` + layer heuristics)
- `angle_slice.rs` → Cross-cutting concern trace (keyword pattern matching)
- `threed_slice.rs` → Temporal-structural risk (uses `call_graph.rs` + git)

### Novel Extensions
- `absence_slice.rs` → Missing counterpart detection (open/close, lock/unlock)
- `resonance_slice.rs` → Git co-change coupling (requires git history)
- `symmetry_slice.rs` → Broken symmetry detection (serialize/deserialize, encode/decode)
- `gradient_slice.rs` → Continuous relevance scoring (decaying scores)
- `provenance_slice.rs` → Data origin tracing (user_input, config, database, etc.)
- `phantom_slice.rs` → Recently deleted code surfacing (requires git history)
- `membrane_slice.rs` → Module boundary impact (cross-file callers of changed APIs)
- `echo_slice.rs` → Ripple effect modeling (callers missing error handling)
- `contract_slice.rs` → Implicit behavioral contract extraction and violation detection (guard clauses, postconditions)
- `peer_consistency_slice.rs` → Peer-signature guard divergence — sibling functions sharing a first-parameter name where some/all lack a NULL guard
- `callback_dispatcher_slice.rs` → Resolve function-pointer-in-struct registrations to their dispatcher invocation sites; flags NULL argument passing
- `primitive_slice.rs` → Deterministic security-primitive fingerprint sweep (hash truncation, weak-hash-for-identity, shell=True interpolation, disabled cert validation, hardcoded secrets)

## Architecture

### CpgContext (Code Property Graph)

The modern architecture centers on `CpgContext`, which bundles:
- `cpg: CodePropertyGraph` — unified graph merging AST, DFG, call graph, and CFG
- `files: &BTreeMap<String, ParsedFile>` — parsed ASTs
- `type_db: Option<&TypeDatabase>` — optional C/C++ type enrichment

Whole-repo `prism nav` indexes use a prism-owned per-repo cache at
`dirs::cache_dir()/prism/nav/<hash(canonical repo root)>/`, with `--no-cache`
and `--cache-dir` gating that navigation store specifically.
Navigation queries include `nodes-at`, `symbol-spans`, `callers`, `callees`, `ego`,
`module-deps`, `repo-map`, `dfg-stats`, and the CLI-only `onboard` report.
`module-deps`/`repo-map` live in `src/navigation/module_graph.rs`; onboarding report
construction lives in `src/navigation/onboarding.rs` and deterministic Markdown/JSON
rendering in `crates/prism-review/src/output/onboarding.rs`. `onboard --out` is explicit create-new-only
output and must never overwrite an existing file.
Navigation and CPG call resolution share one ordered ladder (S3),
`CallGraph::resolve_call_site` in `src/resolution.rs`, which returns each callee
with a `ResolutionConfidence` (`Exact` | `NameOnly`) and a `ResolutionKind`. The
rungs (R1–R7): qualified `T::m`/`mod::T::m` and `Self::`/`self.`/receiver-var
calls bind via a per-`(owner, name)` method index (Rust trait impls are dual-keyed
under the trait, demoted when >1 impl); authoritative Rust `Self::` calls reach
the dedicated Self allow-list. The caller must be a direct method of a module-level
impl, whose raw self type resolves to the selected in-repo type scope (never a
trait-only key for an alias or external self type), with no nested named item between the method and the call. Closures, async
blocks and inline const blocks retain Self. The singleton target must always be inherent, with an unambiguous syntax identity
checked before the same-impl shortcut. Otherwise it must have identical whitespace-normalized
self-type syntax (including generic arguments). If either impl has generic
parameters or a where clause, both generics and where clauses must also match
textually. Normalization compares syntax-token sequences, preserving token
boundaries and literal contents. Cross-impl generic/composite self types or
constrained headers additionally require the same lexical module (same file and
enclosing module nodes); plain nominal paths retain cross-file support. This is
a conservative restriction because equal argument/bound text can name different
types in different modules. All trait targets (including same-impl targets), local impls, missing syntax/owner evidence,
trait-default callers, any in-repo trait default with the callee name, and pools
with zero or multiple candidates drop as UnknownName; there is no NameOnly fan-out.
The target must be accessible: public methods pass, pub(crate) requires the same
crate root, and private or other restricted forms require the caller's module to
be the defining module or a descendant (including out-of-line modules). Other
pub(super)/pub(in ...) cases conservatively refuse. A same-named variant on any
in-repo enum with the self type's name vetoes the edge; variants on differently
named enums do not. Enum and variant names canonicalize raw identifier spelling.
Only new Self edges (main gives UnknownName) refuse when the caller's modeled
crate or a reachable in-repo dependency contains a non-ASCII Rust identifier token
(references and macro tokens count; literals/comments do not); unassigned files
count toward every possible crate. Main's outcomes and source-spelled FunctionIds
are preserved; main's identifier byte comparison belongs to #528.
Trait-impl callers may still call inherent targets.
Ambiguous impl metadata and macro-token calls without a parsed call expression
also refuse.
External, std, derive and macro-generated impls remain outside the existing owner
index model. Rust std/core/alloc paths, scope-selected imports/aliases and known external crate paths filter each owner/function candidate against declarations in that target's own crate using a graph-cached lookup that excludes generic barriers, only with complete scope facts and external-prefix proof; lexical in-repo roots and receiver annotations take precedence, surviving targets retain main's kind/confidence, and missing facts, poisoned roots, macro-hidden declarations, facade re-exports, chained imports and external globs keep main's behavior; the eight-corpus audit identifies no correct-edge losses among 88 Exact/204 total removals. Accepted cost: a crate that declares a type or trait with the std name loses correct edges to its own impls of or for the std item (for example `std::error::Error::source(e)` in a crate that declares `enum Error` and implements `std::error::Error` loses its correct NameOnly candidates; the `ExitCode` fixture is one instance). The filter also fails open, keeping wrong edges, when the `std` root lookup is Poisoned by an unexpanded glob or macro, or when the target file is attached to no crate because its `mod` is macro-hidden. Mixed-edition workspaces and `extern crate self` supply no external proof.
A Rust `pub(in path)` item is visible to call sites inside the named ancestor module: after population `rust_populator/restrict.rs` places the path (`crate`/`self`/`super` anchor, each segment naming the next module on the item's own module chain) and sets the binding's restrict; a path it cannot place (not anchored, not an ancestor, through a re-export, `pub(in ..) use`) stays unknown and refuses as before.
A Rust self import (`use p::{self}`, `use p::{self as m}`) binds the module in the type namespace only (Rust Reference, "self imports"), and `{self as m}` binds `m` to `p` (the trailing `self` marker is not a path segment; a leading `self::` anchor is kept). A `crate`/`self`/`super::super` prefix written before a macro-argument call (`assert!(super::super::f(x))`) is kept in `CallSite::path_anchor` and read only by the authoritative route (`resolution/rust/path_anchor.rs`); `callee_name`, the span and every other consumer see the chain from its first identifier as before. Qualified paths start at the enclosing module and do not see block-scope imports or modules (#768); the newly admitted forms refuse where that matters: a `{self as m}` path with a block-scope `m` between the site and its module.

A Rust `#[path = "x.rs"] mod m;` resolves relative to the DECLARING FILE'S directory, not its `<stem>/` directory (Rust Reference, "Modules", the `path` attribute; #765): `src/a.rs` mounts `src/x.rs`, `src/a/mod.rs` and a crate root mount beside themselves. Inside an inline `mod` block the path is relative to that module's directory (the file's directory for a mod-rs file, `<stem>/` for a non-mod-rs file, plus the inline names); an inline module's own `#[path = "p"]` is the directory its children live in, relative to the file's directory. A `#[path]`-loaded file is treated like a mod-rs file, so its children are its siblings. `.`/`..` segments are normalized, and an absolute or root-escaping path names no file; `#[cfg_attr(.., path = ..)]` is not read (the default `m.rs` location is used). A module declared inside a block (a fn body) drops the pending `<stem>/` of a non-mod-rs file: `fn f() { mod i { #[path = "x.rs"] mod m; } }` in `src/a/b.rs` is `src/a/i/x.rs`; an inline module whose own `#[path]` is absolute or climbs out of the repository leaves its children nothing to find. A file loaded by more than one `mod` item (a plain `mod b;` and `#[path = "b.rs"] mod b;`) is populated once per OCCURRENCE with its own parent and child directory (an active-file stack stops `#[path]` cycles); a site inside such a file (`ScopeGraph::multi_occurrence_files`) cannot be tied to one occurrence and keeps only an unqualified callee whose targets are in its own file (a `crate::`/`super::`/module path can mean a different item per occurrence). Crate roots by convention are the package-level `tests/*.rs`, `benches/*.rs`, `examples/*.rs` (never below a `src/` segment, which holds modules) plus `src/lib.rs`, `src/main.rs`, `src/bin/*`; a real `tests/probe.rs` that is also `#[path]`-mounted is two occurrences. Because the placement is rustc's, no resolution rung guards against a misplaced mount.

A Rust local is in scope only after the statement that binds it: `let report = report(2);`, `for x in x(..)`, `if let Some(x) = x(..)` and a `let .. else` block call the fn, not the new local (#765 PR D). A std expression macro that can introduce no item or binding (`env! option_env! concat! include_str! include_bytes! cfg! line! column! file! module_path! stringify! matches! compile_error!`, unqualified or `std::`/`core::`/`alloc::`) does not poison the lookup of the names after it, unless the repo can bind the spelling to another macro: a `macro_rules!` definition, or a `use` item that can bring a macro under it (a path rooted outside the repo other than the std item of that name, `use other::env;` and `use ext::anything as env;` included, or an in-repo `as <spelling>` alias of an in-repo macro, chains followed; an unaliased in-repo `use super::write;` of a fn and an in-repo alias of a fn taint nothing), or a `#[macro_use] extern crate` of a non-std crate anywhere (an unqualified spelling then; `std::env!` stays trusted). A glob import of a macro named like a prelude macro is rustc error E0659 and cannot occur in a compiling crate, so `use ext::*;` taints nothing. This census applies to the E2 list only: `TRANSPARENT_ARG_MACROS` keeps main's `macro_rules!`-names-only shadow set (a `use pretty_assertions::assert_eq;` must not stop call extraction repo-wide; main's alias hole on that list is #773). `thread_local!`, `include!`, user macros, imported or aliased names and every item-position macro still poison (an item-position `thread_local!` poisoning the fns after it is the largest recall loss, #771). The list is `NON_BINDING_EXPR_MACROS` (`call_graph/rust/macro_args.rs`), separate from `TRANSPARENT_ARG_MACROS` so call extraction does not move. Out of model (owner 2026-11-08, #772): three import-provenance forms under which a user macro reaches a listed spelling that the census still trusts: an external macro re-exported under another name and then aliased to the spelling (`pub use ext::make as bridge;` then `use crate::a::bridge as env;`), a named in-repo import of the spelling from a module whose only source for it is an external glob re-export (`pub use ext::*;` then `use crate::a::env;`), and an external crate whose name also occurs as an unrelated in-repo module spelling (`mod unrelated { pub mod ext {} }` then `use ext::env;`); none occurs on prism's tree or the Tier-A corpora.
Import-qualified `pkg.f()` narrows by file
stem or Go package directory with no permissive fall-through; `Class.m()` binds
when the qualifier is itself an owner key; unqualified calls prefer a local free
definition, then Java/C++ implicit-`this`, then cross-file free functions
(methods excluded — a method needs a receiver); unknown-receiver `x.m()` uses
**P6-lite** syntactic receiver typing behind a swappable `ReceiverClassifier` seam
(`resolution.rs`, `legacy ↔ expanded`): Rust/Go typed params + constructor locals,
plus Phase-IP PR-2's Go type-assertion (`x.(Module).M()`) and `var`-declared
(`var r Runner`) receivers, with a std-wrapper peel list and shadow-bail; recovered
receivers route through the existing `owner_lookup → interface_impls → drop` ladder
(recover-and-route — recovery is syntactic, routing decides interface-vs-concrete).
It otherwise demotes a single in-repo owner / drops a multi-owner collision (the
precision floor); module-qualified free functions fall back to file-stem matching.
The relative candidate list `js_ts_relative_module_candidates` serves import-member, re-export and
namespace resolution and the class-identity consumers (receiver typing through an imported class,
imported props types). It keeps a specifier's existing list when it has one; otherwise a literal `.js`
path that is not indexed finds the same-stem `.ts` then `.tsx`, `.`/`..` find the directory index, and
`//`, `/./`, `/../` segments are normalized within the repo root. Import-forwarding uses it for
`./`-spelled specifiers and refuses the bare `.`/`..` forms; the tsconfig `paths` lane keeps its own
rules; a namespace import whose module only this fallback finds and whose proof fails stays NameOnly.
Language-family admission (`src/languages/family.rs`, `ResolutionOutcome::retain_family` in
`src/resolution/free.rs`; {C, C++}, {JS, TS, TSX}, every other language its own family) is
removal-only: the rung, kind and confidence come from the unfiltered pool, then targets in another
family than the caller are dropped, a pool left empty refuses with no later rung consulted, and a
filter never promotes (a multi-owner NameOnly pool left with one target stays NameOnly with the
same kind). Free-function pools filter inside their rungs; method rungs (R1, R3b, R4b, self and
receiver-variable owner lookups, recovered receivers, R6) share one admission on the public full
resolution (`resolve_call_site_full`). `resolve_call_site_candidates` is the unfiltered view. Every binary root of a workspace member with in-repo `[dependencies]` (explicit `[[bin]]` paths, else `src/main.rs`, `src/bin/*.rs`, `src/bin/*/main.rs`; bin-only members included) is keyed with those dependencies under their in-source names (`Builder::finish`, `RustCrateConfig::member_bin_roots`, enumeration shared with `binary_library_deps` in `repo_loader/bin_roots.rs`), so `prism::m::f()` in `crates/prism-review/src/main.rs` resolves like the same path in a library file; a dependency named like the member's own library is keyed for neither, `build.rs`/`tests/`/`examples`/`benches` roots stay unkeyed (they also see unparsed build/dev dependencies), `autobins = false`, a `[[bin]]` with its own `edition` and the 2018+/uniform-edition gate still exclude, and a result in such a dependency refuses when a block or callable scope binds the path's first segment in the type namespace (`resolution/rust/bin_dep_guard.rs`; #768). `#[path]` placement for these entries is rustc's (#765), with no guard of their own. The `T::m` owner-pool fail-open in `rust_authoritative_nonself` is inherited and unchanged.

Navigation maps confidence to `score` (Exact 1.0 / NameOnly 0.6) with a
`Reason::Resolution` and a `Collision` warning when same-name receiver sites are
dropped. Phase-IP type-confirmed dispatch has **shipped** Go embedding promotion
(#95) and Go interface satisfaction (#96); remaining gaps (spec §2.4): Python
inheritance, field/return-typed receivers (the S3.1 struct-field-index candidate),
cross-package concrete-asserted keys, and package-level `var` receivers.
`prism nav call-stats --repo <dir>` reports the resolution-kind histogram and drop
classification; `prism nav interface-manifest --repo <dir>` emits the PR-2 in-scope
interface-dispatch manifest (the §8a denominator for the precision gate report).

`prism nav dfg-stats --repo <dir>` reports `dfg_label_exact`,
`dfg_label_loop_carried`, five mutually exclusive `dfg_label_nameonly_*` doubt counters, and the
two reaching-definition availability counters. RD-unavailable functions with more than
1,000,000 prospective raw legacy emissions surviving U/D emit no local edges while retaining defs/uses;
`dfg_fallback_functions_capped` counts omitted functions and `dfg_fallback_edges_capped`
counts their surviving raw emissions, duplicates included. Omission also removes the capped owner's
contribution to shared label keys of same-name/same-start-line owners and derived edges,
such as the #594 object-literal shortcut into a capped callee. Loop-carried is a subset of Exact, so labeled-edge
count is Exact plus the five NameOnly counters. `--edges` emits sorted JSONL with the doubt
vocabulary `killed`, `sameline`, `cfg_incomplete`, `alias_unstable`, and `call_nameonly`;
`kill_line` is present only for `killed`; on an object-literal shortcut edge (below) it is a line of the
`to` endpoint's function, the callee. The same counters are nested under `call-stats.dfg_labels`.

Go eager aggregate reads also receive genuine field-write Def → ancestor Use edges.
This separate contribution pass shares the owner's CFG, binding facts and DefIds;
only its own lane applies root/prefix/same-field kills. Sibling fields stay independent.
Receivers, captures, same-line pairs, nested-callable writes, indices and alias twins
are excluded. Available RD is required. `dfg_recall_functions_over_rd_cap` and
`dfg_recall_functions_without_cfg` count affected owners skipped for RD refusal.
Available owners count raw surviving candidates, including duplicates, before allocating
edges or supplemental Uses. Over 1,000,000 omits the entire addition while keeping ordinary
edges; `dfg_recall_functions_capped` and `dfg_recall_candidates_capped` count these omissions.
`dfg_recall_candidates` includes admitted and addition-capped candidates, and
`dfg_recall_max_candidates` is the largest per-owner count. RD-unavailable survivors
cannot be counted, so they contribute only to their owner skip counter.
These counters appear in dfg-stats and call-stats.dfg_labels and persist through cache hits.
The lane ignores Go return/break to defer dispatch. Range, select-receive and type-switch rebinding, including selector and dereference header targets, kill the comparable contributions of the root (range on iteration entry only). Package-level roots apply overwrite kills. A use before its def is emitted only with an Exact cyclic route; uncertain backward pairs are dropped (a recall loss, e.g. loop-carried flow of package-level roots). Other uncertain routes carry cfg_incomplete.

Go recall S2 (#639) adds three consumer shapes to the same lane, still `Def(p.f…) → Use(strict prefix)`, counted in the same per-owner
1,000,000 budget (one survivor iterator). Receiver: `R.M(..)` with a static path `R` (`p`, `p.a`, `*p`,
parenthesized) is a consumer of `R`, a real Use is created at the receiver span only when an edge is admitted into it; in
`R.M().N()` only `M`'s receiver counts. The receiver of an outermost `defer R.M()` / `go R.M()` call is evaluated at the statement
(Go copies a value receiver and fixes a pointer there) and may read `R`'s fields at exit or concurrently, so it sees only
contributions that reach the statement (evaluation point within its node), graded `cfg_incomplete`; writes after the
statement give no edge (owner decision, fold 3; recovering them for proven pointer receivers is issue #675); an inner call of a chain is evaluated at the statement and keeps the ordinary rule. Refused: `M` is a func-typed struct field of the same Go
package (a func-typed field call is a field read; fold 1 moved this from a same-file name check to a repository-wide one: each
admitted receiver edge is recorded with its selector in the file's recall ledger (`RecallStats`, cached with the file's DFG), and
at graph assembly `cpg/go_package_fields.rs` removes or restores edges against the package's fields, so cold, full-hit and
partial-hit builds agree, including when only the declaring sibling file changes; counter `dfg_recall_receiver_field_refused`,
in dfg-stats and call-stats `dfg_labels`). Every `field_declaration` of the package files counts, wherever it sits: named
top-level types, anonymous struct types and function-local type declarations (fail closed: a local table-struct func field named
like a method refuses that method's receiver edges too). A scoped review build assembles only its in-scope files but reads the
package fields from the whole repository's file map (`CodePropertyGraph::build_scoped_subset`), so its refusals equal a full build's. Package = directory + `package` clause, so an external `foo_test` package does not see
`foo`'s fields while internal `_test.go` files do, but only for call sites in `_test.go` files (#677): a non-test file is compiled without the package's `_test.go` files, so a field or type declared there refuses and resolves for `_test.go` call sites only, while fields declared in non-test files refuse in every file. A field refuses its name when its type may be callable: `func(..)`, a qualified
`pkg.T`, a package-local type resolving (through aliases, depth 8) to a func type, or one that cannot be classified; basic, slice,
map, chan, array, pointer, struct and interface types do not refuse, and a method with the same name does not matter (a field
and a method of that name in one package refuse, failing closed). Gap: an embedded type's own fields are not followed. Receiver
Uses that exist only as recall endpoints are lane-local: they get a graph node for the recall edge but are left out of the
Step 5b/5c lookup views (`cpg/recall_hidden.rs`; the stored, public `var_index` stays complete because a cache load rebuilds it from every
node, so cold, full-hit and partial-hit builds answer lookups alike) and Step 5c's input selection skips them (return-expression
inputs and caller-RHS suppression), so no pass binds them to a callee; the call-stats counters outside `dfg_labels` equal a
recall-disabled build (real
receiver-to-method flow is a possible later slice); a Use shared with an ordinary same-line same-path Use is one node and stays
visible. Also refused: a method value without a call, an import-qualifier root the owner does not redeclare, a
root without a proven binding (flat fallback or unresolved), a sibling receiver (`p.g = x; p.f.M()`), and the same path
(`p.f = x; p.f.M()`, left to main). Capture read: a consumer of `p` or a strict static prefix inside a function literal at any depth is admitted, always
`cfg_incomplete`, by the run-site rule (owner decision, `go_literal_mode.rs`): an immediately invoked literal and a literal passed
directly as a call argument outside `go`/`defer` see only contributions that reach the creation point; a literal bound once by
`f := func..` / `var f = func..` whose name is only ever called as `f(..)` (outside any literal and `go`/`defer`) sees those that
reach some call; an escaping literal (stored, returned, sent, assigned elsewhere, under `go`/`defer`, or `f` used other than as a
callee) keeps live-at-or-after-creation, propagated forward from the creation point so only contributions on paths through it count (a write on a branch bypassing the literal that joins after it gives no edge). Liveness is computed at the literal's evaluation point inside its collapsed node
(`reaching/route.rs`), not from the node's incoming set, so a root, prefix or same-field write earlier on the creation line kills
the contribution while one after creation does not. Cost: real flow into a callback a callee stores and runs later is lost.
Refused when a closure parameter or local declares the root name (flow-insensitive), or when the outermost literal writes the
root, `*root` or any prefix of the path, by assignment, `:=`, `var`, `++`, or `range`/select-receive rebinding.
Writes inside literals are never sources. Same-line: order is evaluation order, not bytes, from each item's statement and its chain of
enclosing branches and arms (`route::order`): the right side of a simple statement runs before its write (`p.f = g(p)` and
`p.f, y = x, p` give nothing); across `;` statements and header init → condition → body the earlier one writes first, label
`sameline`; arms of one `if`/`switch`/`select` never both run (`if c { p.f = x } else { use(p) }` gives nothing); a reverse read
gives nothing, even in a loop, except a `for` post assignment reaching its own condition or body, only when THIS loop's post can
run and return to its header through nodes of the loop's own lines (an enclosing loop's cycle does not count, and a loop whose
header, body and back edge share one node proves nothing; `inc_statement` `p.f++` is not a Def in main, so it is no source); a
comparable write between them (or in the same statement) kills; a kill strictly earlier than the source on its node (`p = q; p.f = x`) no longer hides the source.
A single-statement owner has no CFG edges and so no recall (existing NoCfg rule).

Go producer defs (#694 slice A, `src/ast/go_header_defs.rs`): range and receive `:=` targets, type-switch aliases and every `var`/`const` name get a def at their identifier (the first name of the first direct spec always did); function-literal formals and named results get one too (#694 slice B: the owner's own formals always did; `go_header_defs.rs` adds each literal's formals and the named results of any function or literal, def line = the identifier's own line, an entry zero value for a result, nothing for `_` or an unnamed result). Header targets, formals and results and aliases mint no alias twin, and rules U and D classify them as plain writes that are never dominating ones (`parameter_declaration` in `lexical_prune.rs`; a formal inside a literal is nested, yet still a declare-only write), so a header or signature line that only declares a name is write-only and a use after the loop or select still sees the outer def.

For Go, the producer omits def→use pairs only when a lexical proof establishes a plain
write-only use (U) or a dominating unconditional write in a single-entry region (D).
Go paths whose base is exactly `_` contribute no DataFlow definitions, uses, edges or alias-twin definitions; RHS reads remain, and other languages retain `_` as an ordinary name. Kept edges can change label where a removed blank twin had collapsed a line (`r, _, err := f()` with `_ = err` elsewhere), besides RD availability; where that line is an `if`/`switch` header whose initializer declares the name, the restored label can be a false `exact` (line-granular header RD, #662). Step 5c treats a Go `_` lvalue as a positional hole: no ReturnFlow into it, the other slots keep theirs, and an all-blank site counts in `return_flow_skipped_blank_lhs`.
U removes even `exact` pairs into pure writes and mints no anchor Use on a write-only line.
A Go statement label between that write and the use disables that D proof. D preserves
nested callable definitions and reads. Field paths are compared as fixed storage; pointer identity
across reassignment of a path's base is out of model (design §2.2).
Uncertain cases keep the pair. RD labels only the surviving edges.
Go goto, labeled break and continue now have CFG edges; this also closes the missing
unlabeled Go break/continue edges. Go expression/type switches and selects dispatch
through case-value/communication anchors, normal arms exit without sibling fallthrough,
and else-if false branches enter the nested condition. Blocking selects have no unselected exit.
The fallback cap counts U/S/D survivors, including duplicates. Per the owner decision, an owner
formerly capped before pruning can regain its surviving edges when pruning brings it within the cap; this includes a function just over 1,000,000 candidates that falls under it once Go blank candidates are gone and then emits its real edges (the same rule as #634: the cap counts survivors, owner-accepted).
Go rule S (#630 slice 3, `src/data_flow/build/scope_prune.rs`) drops a (def, use) pair when the def and every read of the name on the use line resolve to different explicit declarations (`BindingFacts::binding_of_def`/`binding_at`, built once per Go owner in the producer from `rd_defs`; RD keeps its own instance because its use-byte table is built from the emitted edges). It runs before D and counts `dfg_pruned_scope`; non-Go languages are untouched. Everything unproven keeps the pair (only the lexical drop below is exempt from these gates; on the binding path a declare-only def bypasses just the address-taken and storage-access gates, and a non-`multi` or unclassified name still keeps): a `FlatFallback`/non-declaration binding, any unclassified binding construct for the name in the function (`unclassified_binding_lines`), an alias twin, a def with fields, an owner with parse errors. A different declaration is not different storage (#687), so S drops only when the def writes the variable itself (its span is a plain identifier in an `=`/`:=`/`var`/`const`/parameter/range/receive/`++`/`--` position, not an operand of `*`, indexing, slicing or selection), no occurrence of the name on the def's line is such an operand and all coalesced same-line defs of that path bind the same declaration and are each a plain variable write by their own span (so a multi-line `*`/newline/`x` lvalue recorded at the line of the `*` keeps the pair), and no use-line occurrence that resolves elsewhere is itself a storage access (`*x`, `x[i]`, `x.f`, `x.M()`, `(x)`). A name whose address is taken or whose array is sliced anywhere in the owner (`&x`, `&(x)`, `&x.f`, `&x[i]`, `x[:]`, by name, so for both the def's and the use's binding) keeps every pair of an ordinary def. Function-literal formals and named results have producer defs (#694 slice B), so S drops an outer def's pair into their body uses (`v := 1; func(v int){use(v)}(c)`), except where the value arrives through something prism does not connect and the name coincidence is the only carrier. A declare-only def (a literal formal or a named result: the entry value of a fresh variable, which writes no storage) is judged in two ways. The lexical drop bypasses every gate: a pair into a use line whose every occurrence of the name lies outside the declaring callable's body drops (`dfg_pruned_scope`, no binding proof; a def coalesced with another def of the path on its line is not judged by this lone-def drop, but each raw signature span is judged on its own, see `raw_signature_outside_scope` below). On the binding path (a clean, `multi` name) it bypasses only the address-taken and storage-access gates, and only when every span of the path coalesced onto its line is itself a plain write (`func(v *int) { *v = 1 }(v)` keeps the store's edge); a line holding only declaring occurrences of the name drops. This can drop an edge main never had (a def main did not carry) even where another declaration shares the storage under the spelling (`func f() (n int) { p := &n; func(n *int){ use(*n) }(p); return }`). The owner's own grouped parameters (`a, b int`) still read only the first name as declaring. Slice B keeps the pairs below (`scope_params.rs`, per declaration; documented limits, nothing filed), because there the real value arrives through something prism does not connect and the name coincidence is the only carrier; it is not a claim that every dropped pair has no other path (a callee that stores a callback, below, is the known exception): (1) a formal of a literal that is not a discarded-result callback (called in place, `go`/`defer` included, assigned, stored in a field, slice or map, returned, spread, or the argument of a conversion, `append`, a wrapper whose result is used, an in-place literal callee or a callee bound in the owner) keeps every ordinary def of the spelling, because arguments reach it indirectly (`arg := v + 1; func(v int){..}(arg)`) or from any later call scope, and no edge binds an argument to a literal formal (left for later); (2) a formal of a literal that is an argument of a call whose result is discarded (a whole statement, or under `go`/`defer`) and whose callee is neither a literal nor an owner-bound name (`sort.Slice`, `t.Run`, `filepath.Walk`, `x.Visit` as statements) keeps only defs whose variable is in scope where the literal is created (a sibling literal's formal never keeps), but only in an owner with no other `func_literal` that has a formal able to accept a function value (a function, interface including `any`/`interface{..}`, named or qualified type, or a variadic of these; pointer, slice, map, array, chan, struct and predeclared basic types cannot): only such a literal can hold the callback and call it with its own values, so the gate needs no alias analysis (`fs[0](func..)`, `s.each = func(cb ..){..}; s.each(func..)` and `g := s; g.each(func..)` keep), and in a gated owner a def whose variable is in scope at creation still keeps; the disclosed limit is a callee that stores the callback (`s.register(func(v int){..})`, then `s.cb(v)` in a narrower scope); (3) a named result read inside a callable nested in the one that declares it (`return e` assigns the result, and a deferred literal reads it afterwards); (4) a def in another callable than the read's (`parity_keep`), for a name that a function-literal formal or a named result of the owner declares, when the read is not inside the def's callable and either the read's callable strictly encloses the def's (the owner counts as a callable) or the read binds anything but a formal or named result of the read's own callable (a captured variable, a `:=`/`var`/range local of the read's callable, or a non-declaration binding): main blanket-kept exactly these names, so this is a pair main kept and adds no edge main lacks (a literal's value reaching the owner through its call result, a deferred literal that sets the carrier's named result, a literal bound inside another literal, an IIFE result read in a sibling literal through the enclosing callable's variable; two sibling literals that each declare their own result stay dropped, and a sibling local written by a dominating `:=` loses the pair exactly as on main, to the reference search's shadow filter (`is_shadowed_at`), not to rule D; an entry-only def never keeps this way: a line whose every coalesced span of the path is a signature name, while an ordinary write sharing the line with a signature name takes the ordinary checks and keeps its real path). Rules U and D are unchanged by this keep, and O is described below. A named-result read of another callable (`Form::Result`, item 3) is likewise never fed by an entry-only def (an outer result's entry value is no value of the nested literal's result). Each raw def span is judged on its own: the lexical scope drop applies to a literal formal or result span even when an ordinary def of the same path shares its line, so a coalesced def survives only if some raw span survives every rule (`raw_signature_outside_scope`; `err = func() (err int) {..}()` no longer gives the signature span an edge to an earlier read of the outer `err`). The callback gate trusts a predeclared basic type name (`int`, `string`, ..) as basic only when the owner's file declares no type of that name (`type_spec`, alias or a type parameter, at any depth; read once per file); a declaration in another file of the package is not seen (a disclosed limit). Ordinary-name return flow out of a literal is not modelled (as on main): `err := func() error { err := do(); return err }()` gives no edge from the inner def to a later read of the outer `err`, because the name is no formal or result and the carrier cannot be told apart from a shadowing local by position and spelling; the successor slice must identify the carrier by binding. Range `:=` and receive `:=` targets, type-switch aliases and every `var`/`const` name now have producer defs (#694 slice A, `src/ast/go_header_defs.rs`: one def per target at its identifier, the right-hand side's reads on the same line, a multi-value call feeding every name; implicit const repetition gets the def only, a constant expression reads only constants), so the value reaches the body through the new def and S drops the outer pair (`v := c; switch v := v.(type)`, `for _, v := range v`, `var _, v = 0, v`). A type-switch alias is ONE def at the header identifier, attached by `BindingFacts` to the first clause's declaration and grouped with the alias's per-clause declarations (`def_reaches_declaration`): S and RD treat a use resolving to any clause's declaration as reached by it, and a use outside the switch as a different declaration. A split header or spec (any token of its right-hand side on a row other than a declared name's: the range expression, the received channel, the asserted expression, the spec's values; `scope_split.rs`) keeps main's behaviour: rules S and O keep every pair of a name it declares (defs main had; a header target's own new def only where it is the variable the split header shadows), rule D never takes a split `var`/`const` spec as a dominating write, and rule U is unchanged (the declare-only anchor line holds no read, and the kept pair carries the outer value); connecting reads across the statement span is #728. Because S takes pairs before D, `dfg_pruned_dominated` falls on upgrade for Go owners where S now removes them. Names introduced by `:=`/`var`/`const`/parameters are writes, not reads; a line with no read keeps the pair (U's job). Two occurrences in different scopes keep the pair if either keeps it. Header-line pairs that RD labelled a false `exact` (#662) are dropped when no read of the def's binding exists on the line.

Python rules U and D (#630 slice 5, `src/data_flow/build/python_prune.rs`, driven by `lexical_prune.rs`; counted in `dfg_pruned_write_only`/`dfg_pruned_dominated`): enabled for a `function_definition` owner without parse errors that cannot reach a builtin that reads locals by name at run time (`eval`, `exec`, `locals`, `vars`, `globals`, and Python 2's `execfile`): U and D are off for every owner of a file that anywhere names a reader as a bare identifier (a direct call `eval(..)` included; the member of an attribute, `model.eval()`, and a keyword argument's name, `f(eval=True)`, are not the builtin), mentions `builtins`, `__builtin__` (Python 2), `__builtins__`, `__import__` or `__self__` (any position, or as a string's whole content), has a string spelling a reader (`getattr(ns, "eval")`), or has any non-ASCII identifier (Python NFKC-normalizes identifiers, so `ｅｖａｌ` is `eval`). Every alias spelled in the file's source needs one of these, so no alias form has to be enumerated (`python_prune::file_hidden_hazard`, one walk per file). `x = eval("x + 1")`, `locals()["x"]` and `"{x}".format(**locals())` read a name prism does not see, so U would take the line as write-only. Out of model: a reader carried in from another module, the builtins module obtained at run time without naming it (`inspect.getmodule(len)`, `sys.modules[computed]`), names or code built from or held inside strings (`"buil" + "tins"`, `"\x65val"`, `exec("ev = eval")`, `pydoc.locate("builtins.eval")`), frame introspection (`sys._getframe`, pandas `query`/`eval` `@name`, `numexpr`), zero-argument `super()` reading the first local and Python 2 semantics beyond `__builtin__`/`execfile` (the `exec` statement, `input()`); precise handling is #805). A write is a whole `expression_statement` holding one `assignment` (single, tuple/list or starred target, annotated with a value) or `augmented_assignment` whose target is exactly the path (identifier or `a.b` attribute chain; D proves identifier paths only, see below); chained inner targets, walrus, `for`/`with`/`except` targets, imports and `x: int` are no write. U (plain `=` only): a use line whose every occurrence of the name is inside a plain lvalue span and has no real read (`x = x + 1`, `x += 1`, `x[i] = 1`, `x.f = 1` are not write-only for `x`). D: regions are `block` only (single entry, no goto), the dominating write is a direct child before the use's statement, defs between it and that statement's end are kept. Comprehensions and `class` bodies count as nested scopes (defs and uses inside are kept), as do `def`/`lambda`; names declared `global`/`nonlocal` anywhere in the owner are never dropped by D; `del`, loop and `with ... as` targets are never the dominating write. D applies to an attribute path only when its base is `self` or `cls`, that base is a parameter of the owner, and it is never rebound anywhere in the owner, nested callables included (assignment, augmented assignment, `del`, `for`/`with`/`except .. as`/walrus/match-capture targets, unpacking, `global`/`nonlocal`, a same-named parameter of a nested def or lambda); every other attribute path keeps (`o.x` with `o` a parameter, a rebound base, a module-level or local `self`). Contract: `self.<path>`/`cls.<path>` are compared as fixed storage, so properties, descriptors, `__setattr__`/`__getattr__`/`__getattribute__`, `__slots__` tricks and runtime mutation are out of model, as for Exact. For such a path a write of a prefix of it (`self.x = ..` after `self.x.y = ..`) or of the path itself kills; a sibling field write does not. U is unchanged. Same-line pairs are judged per raw def span (`LexicalFacts::keep_def` gets each lvalue's own bytes, at the direct and the alias-twin emission site, while the emitted location stays the coalesced one), so `x = 1; x = 2` keeps the second def's edge. This is shared with Go: same-line pairs gain edges in both languages (and Go `dfg_pruned_dominated` can move on non-gofmt `;` lines), and because the fallback cap counts survivors an RD-unavailable owner exactly at the cap can cross it and then emits no local edges (owner-accepted under #634/#630; pinned by `same_line_fold2_tests.rs`). A decorated function is not enabled (the owner node is `decorated_definition`, not `function_definition`). Per the #634 rule the survivor-based fallback cap lets a formerly capped Python owner regain its surviving edges when pruning brings it under the cap, so removal-only is not strict for such owners.
Rust rules U and D (#630 slice 5, `src/data_flow/build/rust_prune.rs`, driven by `lexical_prune.rs`; counted in `dfg_pruned_write_only`/`dfg_pruned_dominated`): enabled for a `function_item` owner without parse errors (methods in `impl`/`trait` blocks are `function_item`s; closures, `async`/`gen` blocks and nested `fn` items are nested callables, so a use inside one gets no proof and D keeps the defs inside one, a nested `fn` item's lets included, as the owner's byte range holds them; U can still drop a nested def's edge into an outer write-only line, and a nested `fn` is also analyzed as its own owner). A write is a statement that is a direct child of a `block`: `let x = ..;` with an identifier pattern, a value and no `else`, or an `expression_statement` holding one `=` or compound assignment (`compound_assignment_expr`) whose left side is exactly the path (an identifier or a chain of plain fields, `s.f`; indexing, dereference, tuple indices and calls prove nothing). A statement preceded by an attribute (`#[cfg(..)]`) is no write, since it may not exist. Not writes: `let x;`, tuple/struct/slice/`ref` patterns, `let .. else`, `if let`/`while let`/`match`/closure-parameter bindings, `for` targets, assignments nested in a larger expression (`y = (x = 1)`, a match-arm value, a tail expression) and anything inside a macro invocation (a `token_tree`, so a macro write is never a dominating write, while a macro read such as `println!("{}", x)` is a use). U (plain `=` and `let` only): a use line whose every occurrence of the name is inside a plain lvalue span and has no real read (`x = x + 1`, `x += 1`, `v[i] = 1`, `s.f = 1` for `s`, `*x = 1` are not write-only for `x`). D: the only region is `block` (function body, `if`/`else`, loop, plain, `unsafe`, labeled `'a: {}` and match-arm blocks; `break 'a` only leaves forward, `?`/`return` leave the function, and a loop re-enters its body at the top, so none is an entry breaker); the dominating write is the last direct child write before the use's statement, defs between it and that statement's end are kept. Shadowing of a plain name needs no binding facts: a dominating `let x` is a new binding that every later use in the region reads, an inner `let x` is a def kept inside the range or in a block that does not contain the use, and a dropped def is one the dominating statement overwrote on every path; rule S does not run for Rust yet. Field paths compare identical paths only (`s.f` never dominates `s` nor the reverse; a prefix write does not kill), and D proves identifier paths only: a field path `b.f..` never gets D (a `&mut`/`Box` base or a strict prefix of the path can be re-pointed and restored, moved out and back, or swapped through `mem::replace(&mut s.g, ..)`, a `&mut &mut S` formal or a dereference, so identical spelling is not identical storage); U stays for field paths. `self` is not an `identifier` node in tree-sitter-rust, so `self.*` paths are never proven by U or D (recall only; supporting `mut self` needs its own slice). U and D are both off for an owner containing a `macro_rules!` definition anywhere, nested scopes included (a macro defined inside the owner reads a local by definition-site hygiene while the line-anchored use sits at the invocation: `let mut x = 1; macro_rules! bump { () => { x + 1 }; } x = bump!();` reads `x` on a line with no `x` token), for an owner with the identifier `macro_rules` in any token tree (`identity! { macro_rules! bump { .. } }`, `install!(macro_rules, x)`) and for one that invokes `paste!`/`concat_idents!` (belt B2: they build names from fragments). Macro hygiene is closed by a repo-wide census of macro verdicts (fold 3, `call_graph/rust/macro_args/census.rs`, `RustMacroCensus`, one verdict per invocation spelling): Allowed means the expansion holds no `macro_rules!` and invokes nothing that does. A std spelling of the closed list (`TRANSPARENT_ARG_MACROS`, `NON_BINDING_EXPR_MACROS`, `pin`, `thread_local`; `include!`, `concat_idents!`, `asm!` absent) is Allowed unqualified or as `std::`/`core::`/`alloc::`, unless the E2 shadow rules withhold it (a `macro_rules!` of that name, an external-rooted `use` of the spelling, an in-repo `as` alias of an in-repo macro, and for an unqualified spelling a `#[macro_use] extern crate` of a non-std crate). An in-repo `macro_rules!` spelling (unqualified, or `crate::`/`self::`/`super::`/`$crate::`) is Allowed only when EVERY definition of it is non-generating, a least fixpoint: a definition generates when its right-hand sides hold the identifier `macro_rules`, a metavariable followed by `!` (`$m!(..)`, `$k! name {..}`), a definition that did not parse, or an invocation of a Refused spelling (a cycle of clean macros stays clean). Every other spelling (a qualified external path, a `mod`-rooted path, an unknown name, proc macros included) is Refused: fail closed. Fold 4 closes more of the census inventory: comments inside token soup are dropped before invocation, path and metavariable recognition (`install_get /* c */ !(x)`, in an owner or a definition body); every `macro_rules ! NAME` spelled in ANY token tree (module level, `cfg_if!` bodies, inside owners) or in a definition's right-hand side adds NAME to the census's defined AND generating sets (the rules are not parsed: fail closed), so a soup-defined generator cannot hide behind a clean same-spelling definition or a std spelling; `r#` is stripped from import spellings before the relevance and alias checks (`use ext::m as r#assert;` withholds `assert`) and from the DFG path name when matching refused (and mutably borrowed) names; and comment siblings are skipped when reading `#[macro_use]` on an `extern crate`. Per owner (`rust_prune::scan_macros`, every `macro_invocation` at any depth, nested callables and items included), the invocation's own path and every `ident ! (..)` spelled in its token tree (a keyword-spelled one such as `gen!` too) are judged; one Refused spelling gives every identifier token of that invocation neither U nor D in the owner (`LexicalFacts::rust_refused`; `identity!(install_get!(x))` sees `install_get`), so a name passed to `json!`, a logger or an unknown macro keeps its pairs (recall cost) while a name passed to `println!` or a clean in-repo macro prunes as before. This retires fold 2's token-tree use-line refusal (`in_macro_tokens`). An implicit format capture (`{x}`, `{x:?}`, `{x:spec}`, and `name$` width/precision arguments; `{{`, positional `{0}`, `{}` and `{:?}` name nothing) in a string literal token of ANY macro's token tree is a read of the name for pruning only (no Use node, no edge): it is an occurrence on the lines of EVERY enclosing statement up to the owner (each ancestor that is a child of a block: a `format!(` whose literal sits on a later line than the assignment, inside an `if let` block, a block-bodied closure or a braced match arm, still blocks U on the write's line), so it blocks U and D must dominate it too (`x = format!("{x}b")` keeps main's edge, `x = format!("{{x}}")` still drops). The format string is decoded first (cooked escapes such as `\x7b` and `\u{7b}`; a raw literal stays raw) and the spec is parsed by the format grammar, so a named width or precision after fill, align, sign, `#` and `0` (`{:0x$}`, `{:>+#0x$.y$}`) is a capture while `{:5}` names nothing. The census is built from the whole repository's file map (a scoped build reads `all_files`), its digest is persisted on the DFG storage (`CompactDfg::rust_macro_census_digest`), and a partial cache hit that finds a different fresh digest rebuilds in full, next to the macro-shadow guard in `build_incremental_with_scope_graph_inputs`. Residuals, disclosed and not fixed: an attribute proc macro on the owner or an enclosing item (`#[tokio::main]`, `#[async_trait]`, `#[instrument]`) rewrites the body, so no per-line premise holds (the same class as a macro-expanded write); an identifier assembled from fragments by an external name-building macro under a spelling other than `paste`/`concat_idents`, or produced by a proc macro with a call-site span from nothing (`quote!(x)` in a `#[proc_macro]` makes `x = read_x!() + 1` read the local with no token); an external generator brought under a spelling by a glob import (`use ext::*;`) when an unrelated clean in-repo definition of that spelling sits out of textual scope (the census trusts the in-repo definition; the code fix would cost recall for every `proptest::prelude::*`-style glob, so it sits beside the #772 family); and the #772-style collision E2 also accepts (an external crate name that is also an unrelated in-repo module spelling, since only `crate`/`self`/`super`/`$crate` count as in-repo qualifiers here, a `mod`-rooted macro path is Refused). D also refuses a name mutably borrowed anywhere in the owner (`rust_prune::mutably_borrowed`: `&mut x`, `&mut x.f`, `&mut *x`, `&raw mut x`, a `ref mut` binding of a place rooted at `x` in `let`/`if let`/`match`/`for`, and in a macro token tree every identifier beside a `mut` token such as `dbg!(swap(&mut x, ..))`; nested callables included), the analogue of Go rule S's address-taken set. Out of model, as in Go design §2.2 and as for Exact: a writer prism cannot see, namely a `&mut self` method called with auto-ref (`impl Reset for Box<S>`), `clone_from`, interior mutability (`Cell`, `RefCell`), `unsafe` raw-pointer writes, and a macro that expands to a write or a `let` of the name (and the three residuals above). Such a writer can bring back the value of a dropped def (`swap(&mut x, &mut y); x = 2; swap(..)` is refused by the borrow census, `x.clone_from(&saved)` is not seen), and that loss is accepted. Same-line pairs are judged per raw def span, as for Python (`let mut x = 1; x = 2;` keeps the second def's edge). Per the #634 rule the survivor-based fallback cap lets a formerly capped Rust owner regain its surviving edges when pruning brings it under the cap, so removal-only is not strict for such owners (disclosed, as for Go, Python and JS; not changed).

JS/TS/TSX rules U and D (#630 slice 5, `src/data_flow/build/js_prune.rs`, driven by `lexical_prune.rs`; counted in `dfg_pruned_write_only`/`dfg_pruned_dominated`): enabled for a named owner (`function_declaration`, method, and an arrow or function expression with an inferred name) without parse errors. Synthetic `<anon@..>` owners carry exact read spans and stay off, and module level has no owner. U and D are both off for an owner that has a direct eval (`js_ts_direct_eval_anywhere`: parentheses, TS wrappers, escaped spelling), possibly mapped `arguments` reachable from the owner (`refusals::mapped_arguments_reachable`, the label code's walk: the owner or an enclosing non-arrow callable with formals whose `arguments` is mapped, reached directly, through nested arrows, an alias or a computed key, escaped spellings decoded, so an arrow owner is judged by the callables around it; or by the owner's own `js_ts_mapped_arguments_possible`, which also covers a program-level arrow that spells `arguments` under a CommonJS wrapper; a read hazard for U, `a = arguments[0]`, and a write hazard for D on a stable base), a `with` statement, an enclosing `with` body (`inside_with_body`), an escaped identifier spelling anywhere (the S1b-1 census `precondition`) or a parse error; strict code, modules and non-simple parameter lists that mention `arguments` keep both rules (`owner_enabled`). Regions are `statement_block` (a namespace body is not one), `switch_case` and `switch_default` (fallthrough enters at the case's top), class static blocks (their body is a block) and `finally` bodies (a block). A write is plain for U when it is an `=` assignment (statement or nested) or a declarator name (initialised or not); `+=` etc., `x++`, destructuring, member and computed targets are no write for the name, so `x += 1`, `x = x + 1`, `[a, b] = [b, a]`, `x[i] = 1`, `x.f = 1` keep. A dominating write for D is an `expression_statement` holding `=` or a non-logical augmented assignment of exactly the path, or a lexical/`var` declaration with one initialised plain-identifier declarator, as a direct child of the region (a `case x = 5:` label assignment is not one: it is skipped on fallthrough, and `window` admits only these three statement kinds) (`let x: T = v`, `as`/`!`/`satisfies` on the right side included); `var x;`, `let x;`, `||=`/`&&=`/`??=`, `a = b || (x = 1)`, comma or conditional operands, `for` heads, `catch (x)`, `x++`/`--` and anything in a nested callable are never d*. Class field initialisers (`public_field_definition`/`field_definition`) are nested scopes like callables. Hoisted function declarations (Annex B, same name), `var` redeclarations and labels need no switch: D drops only defs a dominating write provably overwrote, so an unmodelled earlier writer cannot resurrect one, and a label only targets enclosing statements. Generators and `async` resume in place, so a region keeps one entry. Field D runs on single-field paths `base.f` only (no prefix kills, unlike Python): an intermediate object of a path of two or more fields is a reference the owner can swap and restore (`o.a = {}; ..; o.a = old`), so those keep. The base must be stable: `this`, or a name bound exactly once in the owner as one of its own formals or an initialised declarator whose scope is the owner itself (not inside a nested callable or class static block, and a `let`/`const` only directly in the owner's body block) and never written anywhere in it (nested callables included); any other base (a rebound name, an outer variable, a name bound only in a nested scope) keeps. Contract: getters, setters, Proxies and runtime mutation are out of model, as for Exact. U also drops a pair into a line whose only occurrences of the name are `var x;`/`let x;` or write targets, so anchor uses on such lines are no longer minted (CPG use nodes and the `edges_before` multiset shrink; `edges_after` is a subset of `edges_before`, labels unchanged). The census, D2-c, D3, refusal-class and #594 passes see the pruned edge set: a def D removed gives the object-literal shortcut nothing to follow past the dominating write. Per the #634 rule the survivor-based fallback cap lets a formerly capped JS owner regain its surviving edges when pruning brings it under the cap, so removal-only is not strict for such owners (owner-accepted, as for Python and Go).

JS/TS/TSX try joins (#802, `EdgeOrigin::JsTryJoin`, `src/cfg.rs` `push_try_join`): the generic try builder's three edge roles (try header -> catch, try header -> finally, first catch statement -> finally) carry explicit incomplete-join provenance for the JS family only (`Language::is_js_family()`; Java shares the builder and keeps `Structured`), whatever their endpoint statement kinds, independently of ownership tracing. Every other copy of the same line pair in that function (a sequential or structured duplicate) is tagged at drain, because RD keeps duplicates and accepts any unflagged one. Also tagged: an edge emitted by the then-arm completion builder from the header line of a compound statement (`if`, loop, `switch`, `try`, `with`, label) inside a try body or catch clause to a handler or finalizer entry (body -> catch or finally entry, catch -> finally entry): the branch-completion builders draw an arm's exit from the nested compound header that ends it (`catch { if (a) { if (b) { x = 1; } else { x = 2; } } else { x = 3; } } finally { f(); }` drew `7 -> finally`, skipping every overwrite); edges from simple statements into an entry, and a header's own false-branch edge, stay as they were (a one-line `if (c) y = f();` ending a try body has its arm exit on the header line and is retagged: accepted cost, `projectServiceStateLogger.ts` in TypeScript/src). `is_incomplete_join` keeps its source-kind check (Python and Java labels do not move); a route that crosses such an edge never proves `exact`. Geometry is unchanged: the edge set, order, statement universe, defs and kills are as before, so labels only move toward `cfg_incomplete` (`lf.js`: the catch overwrites `x` on both arms and the catch -> finally edge used to bypass them, `x@6->x@14` was `exact`; `try { x = a(); } catch (e) {\n sink(x); }`: the header -> catch role was the only route, `x@3->4` was `exact`). Accepted cost: a legitimate exact whose only modelled route crosses a flagged edge becomes `cfg_incomplete`, including through the header -> finally retag (a normally completing catch with a preserving finalizer; a `for..of` exit landing on a catch's `return null` line, then catch -> finally), and a `killed` label can become `cfg_incomplete` where the kill sat on the unflagged route (`try {..} finally { x = 1; }` after a def). Matching is by LINE PAIR: a pair seeded by an arm completion retags every copy, so a loop exit or a header's legitimate false edge that shares the pair (an entry-line collision such as `sink(x); } catch (e) { handle(); }` on one line after an inner `if`) is retagged too. Residuals, not closed: a nested-if exit that does not enter the handler or finalizer (#796 family; `x@3->12` with no try is wrong on main too); a loop or do-while completing into the finalizer (#693/#697, wrong `exact` also without any try; #785 fixes admitted loops); finalizer-created defs and pending completions (#802). The model has no pending completion, so a finalizer-created def still reaches the statement after the try as `exact` (`try { return; } finally { x = 1; } sink(x);`, `x@6->x@8`), as does any def on a route that leaves the finalizer for a normal successor after a pending abrupt completion; only routes that START at a def and cross a try edge are restricted. Pending-completion modelling is separate work (#802, #673).

The additive `dfg_pruned_write_only`, `dfg_pruned_scope`, `dfg_pruned_dominated`, and
`dfg_pruned_order` counters persist through cache hits and appear in both stats commands;
order counts Go rule O (below), scope Go rule S. Prune counters count skipped
producer candidates before edge compaction and deduplication, counted once from the
first pass even when the owner is capped; replay does not count them again.

Alias twins (alias-twin lane, `src/data_flow/build/alias_occurrence/`): the producer's flow-insensitive alias map mints a fabricated "twin" Def of the alias's target at every lvalue occurrence of the alias name. Per language, in slices, an occurrence that only declares, initialises or reassigns the alias variable (`binding-write`) mints no twin (registration, fallback anchors and edge emission read one per-owner `TwinPolicy`, keyed by span start byte); real writes through the alias (`storage-write`: `u.f = v`, `u[i] = v`) and every form without evidence (`unsupported`) keep today's behaviour, and alias discovery (the map, `raw_aliases`, supplemental exact reads) is unchanged, so no relation is rejected or resurrected (the stale-relation invalidation policy binds the first slice that rejects candidates). The decision is per (owner, occurrence). Refusal R1 keeps the twin of every binding write in an owner holding a method-name or other-object member-name token spelled like the alias's target root (the occurrence extraction cannot tell such a token from the variable; removing the twin would promote edges through it to `Exact`). Java is the first language (rows `J-01..J-06`): declarations (J-02, J-03) are always binding writes, while an assignment (J-01) is one only when a local or parameter declaration of the name is visible at the occurrence (block, `for` init, enhanced-`for`, catch, try-with-resources, method and explicitly typed lambda parameters; a class body, an initialiser or a callable boundary ends the walk, inferred lambda parameters carry no evidence); fields, implicit fields, a same-named local in another block, anonymous-class members and read positions (call arguments, operands, initialisers: an extraction defect owned by its own slice) stay unsupported. The classifier never reads a type name. Go, Rust and Python follow (slice 2a, rows `G-01/G-02`, `R-01/R-03`, `Y-01..Y-03`): a name written by `=`, `:=`, `var`, a range clause or a receive (Go), by `=` or a `let`/`mut`/`ref`/tuple/closure/fn-parameter pattern (Rust) or by `=`, a pattern leaf, a walrus or an `as` target (Python) is a binding write; `*u = ..` (Go, Rust), attribute and subscript targets are not, nor are selector/index targets (Go, Rust, kept as unsupported without declaration evidence), Rust compound assignment (`R-02`, always unsupported: more conservative than the oracle), Python `u += rhs` (`Y-02`), struct/tuple-struct pattern leaves and destructuring-assignment leaves (a plain identifier directly inside `if let` or `for` has no span; a `mut`/`ref`/tuple pattern leaf in them is a binding), an identifier the file declares as a const/static/struct/variant or imports by an explicit `use` leaf or alias (`R-06`), comprehension targets and Go `const`/type-switch forms. R1 collects selector field and method names, keyed literal keys and labels (Go), field and method names, scoped path segments and macro tokens after `.`/`::` or before `:` (Rust, plus named macro arguments `name = ..` and the roots of `&mut x` and `ref mut` borrows), attribute and keyword-argument names (Python). Residuals: glob-imported and macro-made constants, interior mutability and call mutation, Go `append(u[:0], v)` on a shared backing array, Python `import o` (not a def), Rust writes the DFG cannot see beyond the `&mut`/`ref mut` refusal (autoref `&mut self` method calls, closures capturing by unique borrow, struct-pattern shorthand `S { ref mut f }`), and an inherited extraction defect (a zero-width anchor on a write-only target is recorded as a Use; clap `examples/git.rs`, Refs #815). C and C++ follow (slice 2b, rows `C-01..C-10`, `X-READ`, `X-DECL`, `c_family.rs`, `c_family_hazards.rs`): a name written by `=`, a compound operator or `++`/`--`, a declaration or parameter name and declarator syntax are binding writes (C always; C++ only with positive evidence: a pointer declarator or a keyword-only builtin type prefix visible at the occurrence, while a `&` in the prefix is a reference and a storage write, and typedef/`using` names including `size_t`, `auto`, class and dependent types, members and undeclared names stay unsupported); `*u = ..`, `u->f`, `(*u).f`, `u[i]` through a pointer or array parameter and `*p++ = v` (`C-10`) are storage writes, `u.f` is unsupported except through a C++ reference, and a read recorded as an lvalue span (`int r = bar(u)`) stays unsupported. A `.h` header (parsed as C) takes the C++ rules, and a C++ `for`-initialiser declaration gives no builtin-keyword evidence (its prefix starts at the `for`), so a later assignment to a `for`-declared builtin keeps its twin while a pointer declarator in it still is evidence and the declaration itself is a binding write (both more conservative than the table). Parameters of prototypes, function-pointer types and abstract declarators are never visible evidence: only the parameter list of a definition's innermost `function_declarator` (the one whose declarator is the function's name, never one nested in a returned function type) or a lambda's counts. The refusals keep a target's twins when the owner holds a token spelled like it that is not the variable (struct fields, designated initialisers, labels, file-level and local enumerators and macros, macro parameters, type, namespace and qualified-path names, template arguments) or can write it in a way the DFG does not model: `&o` in any form, `o++`/`--o`, an array, an undeclared target passed to a call, any identifier among the arguments of a macro-looking callee (all-caps or `#define`d in the file), a bare target argument of a callee that is neither declared in the file nor a library name (`Py_CLEAR(o)`); an occurrence whose own statement holds a parse error is unsupported and every identifier inside an `ERROR` node is a refused spelling; for C++ also every call argument (a bare pointer included: no by-value exemption), a method called on it, the right operand of `>>`, a reference bound to part of it, a hazard on a proven reference alias (`int& r = o; ++r;`, also through a file-scope or class-member reference whose initialiser names the target), a reference declarator anywhere in a chain or with any initialiser form other than the plain `T& r = o;` (`int *&r = o`, `int& r{o}`, `c ? o : q`: every identifier the initialiser can yield), an address-of an operand without a single root (`&(c ? o : q)`) and by-reference lambda captures including their initialisers (`[&x = o]`); a macro argument in parentheses (`clear((o))`) counts as bare. Residuals: header macros that assign a target nested in an expression, array fields decaying inside a call argument, callee mutation through a pointer or global (library `memcpy(o, ..)` writes the pointee), a typedef'd array target (`jmp_buf`, `va_list`), GNU `asm` output operands, a copy constructor that writes its source, and the X-READ extraction defect (an `Exact` edge from the read of one initialiser to the next, present in main without any alias; the twin's same-line collapse used to doubt such edges). None of the six enabled languages produces fallback anchors. Specification and gates: the slice-0 classifier table and `GATE.md` of the provenance survey; `PRISM_EDGES_BYTES=1` adds occurrence bytes to `nav dfg-stats --edges` endpoints for the gate's byte-level edge ledger (off by default, default output unchanged). The legacy frozen producer (`legacy_producer.rs`, test-only) is a reference of main at cc78c2b1 and is deliberately not updated.

Go rule O (#630 slice 4, `src/data_flow/build/order_prune.rs`) drops a (def, use) pair when the def's line is after the use's line, both are direct (neither inside a nested callable of the owner), no `for` node of the owner contains both, and the owner has no `goto` or labeled statement (any form of loop counts, and the outermost loop around the def decides; function literals, including those passed to `defer`/`go`, are nested callables, while a `defer`/`go` call's arguments are evaluated at the statement). The use is line-anchored, so every non-declaring occurrence of the name on the use line must satisfy this (any one that keeps, keeps the pair); same-line pairs are always kept. Re-entry: the name must resolve at the def and at every use-line occurrence to the same explicit declaration of the owner itself (`BindingFacts`; package-level, captured, `FlatFallback` and unclassified-binding names keep), the def must write the variable itself by rule S's plain-write check (field paths, `*x = ..`, indexing and same-line coalescing keep), and a name in rule S's address-taken set keeps (`&x`, `x[:]`; named results and literal formals are ordinary locals for O since #694 slice B: `use(r); r = 1` drops the earlier pair, while a read of `r` inside a deferred literal is nested and keeps, and the later write still reaches it). A use-line occurrence that is a storage access through the name (`*x`, `x[i]`, `x.f`, a method call or method value `x.M`, which a pointer-receiver method or a deferred `defer x.M()` reads later) keeps. A use that is itself a declaring name (a parameter-list anchor) keeps. O runs after S and before D, so pairs D would also drop now count in `dfg_pruned_order`; D still handles the cases where a dominating earlier def exists and O the ones where none does. Facts are lazy: nothing is built for an owner until a pair with the def after the use line reaches it.

C/C++ unlabeled `break`/`continue` have CFG edges (`src/cfg/c_jumps.rs`, #645): `break` goes to the structural exit of the innermost enclosing loop or `switch` (the next statement, or at the end of an enclosing loop body that loop's continue target; function-final gives none), `continue` to the innermost loop's header line, or a `do`'s condition line, skipping a `switch`; nested callables are never crossed. A `switch` dispatches header → each `case`'s first statement (an empty case reaches the next one) and, without `default`, header → exit, while real fallthrough keeps riding the sequential edge; a switch with a `case` outside the body's direct children (Duff's device) gets no dispatch. A `do`..`while` condition on its own line is a CFG statement (`statement_traversal` admits it for C/C++ and, under the JS/TS/TSX rule below, the JS family): body end → condition → first body statement (when the body's entry is Unknown, e.g. a leading preprocessor wrapper, condition → the `do` header line when that line holds no other statement, whose sequential edge enters the body; otherwise no edge) or loop exit, and the fabricated `do` header → exit edge and the body-last → `do` header back edge are dropped for EVERY admitted `do` (a kept back edge would bypass the condition's writes). `c_jumps` adds completion edges by an explicit rule list (`Cx::completions`): an expression statement or declaration completes at its own line; a block or case through its last statement; both `if` arms (the `if` itself without `else`); a nested loop header; a nested `do`'s condition; a labeled statement's inner statement; a `switch` through its last case and, only without `default` (and not Duff), through its header; `try`, preprocessor wrappers, attributed statements and unknown kinds supply no completion (a recall loss, never a bypass). The admission is one predicate, `ParsedFile::do_condition_line`, shared by the collectors, their oracle copies, `c_jumps` and `cfg.rs`: a one-line `do { .. } while (c);`, a condition sharing a line with any body statement (range-for included), or with ANY statement outside the `do` in the owning callable's statement universe (`} while (c); return r;`, also behind an unbraced parent, a closing brace or a label) keeps main's edges. All edges are `Structured`; statements sharing a line stay deduplicated (#549).

JS/TS/TSX loops (#693, #697; `src/cfg/js_loops.rs`, run after every other builder of a JS-family owner): the sequential and `if` edges join a statement to the next line of the sorted universe, so a loop body's last statement also reached the code after the loop without passing the header and a header write (a `for` update, a `while` condition) never killed a body def; a `do` condition shared the `do` header's node. The unit is the loop TREE (an outermost loop of the owner, never inside a nested callable, with every loop nested in it), REWIRED all or nothing: every loop's continue point (the header line, or an admitted `do`'s condition line), exit (`after`, computed from syntax, never from the sorted list), body completions (an explicit rule list like C's: atomic statements at their own line, a block through its last statement, both `if` arms or the `if` itself without `else`, an empty arm at the construct that entered it, a nested loop's header, a nested `do`'s condition, a labeled or `with` statement's inner statement, `try` through its `finally` (else its body and `catch`; nowhere when neither completes; an empty `finally` is transparent), a `switch` through its last case and, without `default`, its header) and a `do`'s body entry must be known, and the tree must hold no form the module cannot route soundly: a `break`/`continue` that leaves a `try` body or `catch` whose `finally` does not enclose the jump's target (a direct edge would skip the `finally`'s writes: a wrong `exact`), and a `for` whose initializer replay hazard holds (`src/cfg/js_loops/for_init.rs`, slice 4, design §5; the initializer runs once but shares the header node, so a back route would re-run its kill). Per eager body definition D, the hazard is: some initializer write can affect D (its static path is a prefix of D's, `[]` matching any component, compared both precisely and as RD records it, cut at the first `[`; `delete o.x` is a write, `delete name` is not; a shape without a static path falls back to the identifiers it hangs off, never an index or call argument) AND no repeated write necessarily affects D. A repeated write excuses the replay only when it is an exact static path, a prefix of D, a MUST-write of the test or update (reached only through parentheses, comma sequences, the right side of an assignment, non-short-circuit operands and the left of `&&`/`||`/`??`; a call, conditional, callable, pattern or logical assignment stops it) and its root names one binding in the loop (declared nowhere in it, or only in the initializer; a `let`/`const` header name redeclared in the body is another binding, derived from declarator NAME positions only, so a write inside an initial value is still a write). No solver is asked. A tree is rewired only when the file's ONE loop-tree plan admits it (`src/cfg/js_loops/plan.rs`, `placement.rs`, #693/#697 option B slice 2; a pure function of the parsed file, computed once, asked by the `do` condition admission, the edge rewrite and reaching definitions, so no `do` condition is admitted without its tree's rewrite): (1) main's own routing above accepts it (`Cx::tree_checked`), (2) the PLACEMENT rule holds, and (3) every CFG record of the owner that intersects it has a disposition of `OwnedReplaceable`, `Independent` or `Foreign`. Placement: reaching definitions places every def and use on the statement node its LINE resolves to through `StatementLookup` (`cpg::placed_lines`: the shortest retained span holding the line), so the rewrite, which is built on the head's line node, is representable only when every header execution occurrence (a `for` initializer, test and update, a `while` test, an iteration binding and iterated expression, a `do` condition) resolves to the head node (a `do`'s dedicated condition node) and no body occurrence does; an occurrence is a named syntax node of the header or body at its start line, a nested callable EXPRESSION is one occurrence, a hoisted function DECLARATION is none (it executes nothing where it stands; slice 3 removed the false positive that refused such a tree), and a check that cannot be evaluated is a violation (fail closed). `Foreign` records are those drawn for a callable nested in the owner or by another callable's build: `Inert` when the owner's RD does not select them (an endpoint line is no statement of the owner), `Admitted` when it does and they are independent of every loop of the tree (or are the very line pair the tree's own rewrite emits, a duplicate); a selected foreign record that is not independent (an enclosing callable's redrawn legacy `LoopBack` for a loop inside a nested callable, the usual TypeScript case) is `Unresolved(ForeignAdmitted)`. Slice 3 (rule (b)): when the nested callable's own tree is admitted, the ENCLOSING owner drops its copies of exactly the records that tree owns (`js_loops/replace.rs::drop_nested_copies`, through the nested owner's `Authority`), so the nested `do`'s back edge no longer survives next to its replacement (a wrong `exact` on 874ce2c3) and the class shrinks to a nested callable's constructs no admitted tree owns. The dispositions are read off a build, which depends on the universe, which depends on the admitted set, so the plan is a fixpoint: each round builds the looped callables (and those sharing a line with or enclosing them) with exactly the admitted trees assumed admitted, removes the trees that fail, and stops at the first round that removes none, so the survivors were verified against the build that stands (the assumption is keyed by file, `ASSUMED`). FINAL REFUSAL POLICY (owner decision, O1final): an owner with ANY outermost tree the plan refuses is refused WHOLE and keeps EXACTLY main's statement universe, edges, labels and RD availability (the fallback cap included): `js_loops::build_inner` returns before any rewrite, wrapper delegation or foreign-record drop, so the owner's CFG is main's, and reaching definitions has no refusal path of its own (the interim callable-wide `cfg_incomplete`, `CensusFacts::refuse_loop_plan` and the all-admitted availability reference are gone). Its admissible sibling trees are refused with it (`Why::OwnerSibling`, counter `refused.owner_sibling`), decided inside the plan fixpoint, so a rewired tree never sits in an owner whose other trees are not; a nested callable keeps its own admission. A wrong `exact` that main draws in a refused owner stands (an open main residual, never worse than main). Disclosed main residual (follow-up candidate: drop tree-straddling foreign edges in the nested owner): an enclosing owner's own sequential edge joining two lines that are also statements of a nested callable skips that callable's loop, so `z@14->z@18` is `exact` on main and here although its header always writes `z` (`loop_plan_tests::a_tree_straddling_foreign_edge_in_the_enclosing_owner_is_a_main_residual`). A tree main's own policy refuses (`Legacy`: a `for` initializer hazard, a jump through a `finally`, an unrouted form) is refused the same way; `legacy_o1` (a failed placement or an unresolved record of it under the final admission) is kept for the trace and the counters. The counters `dfg_loop_plan` (per file, persisted: `trees`, `admitted`, `refused.legacy.<for_init|finally|unrouted>`, `refused.placement`, `refused.unresolved.<kind>`, `refused.foreign_admitted`, `legacy_o1.placement|unresolved.<kind>`, `overlap.<placement|unresolved|foreign_admitted>`, `owners_refused`) are in `nav dfg-stats`/`call-stats`; `PRISM_CFG_OWNERSHIP_TRACE=1` prints the verdicts as `own-plan` lines. A tree that is not rewired keeps main's edges and main's statement universe, its `do` conditions included. In a rewired tree the edges are replaced BY RECORD (slice 3, `js_loops/replace.rs`, no line-span rule): a record of the owner's build is dropped only when an admitted tree owns it (`js_disposition::Authority::owns`: `OwnedReplaceable`, the tree's own back edge, exit, a body completion that bypasses the loop, or a delegated then-end), records of other trees, of statements outside the tree and of unmodelled emitters are unchanged, and the tree adds from syntax: body completion -> continue point, header -> `after(loop)` (a nested loop's exit is the outer continue point), a `do` condition -> body entry; a `for`..`in`/`of` body and a `continue` of one also complete into `after(loop)` because its head writes on entering an iteration, never on the exhaustion. `break` (unlabeled, to the innermost loop or `switch`), labeled `break`, `continue` and labeled `continue` inside a rewired tree get edges to their target (main had none; a `break` out of a `switch` inside a rewired tree reaches the continue point); a jump whose target cannot be resolved gets none. A `do` condition is a CFG statement under the same geometry as C/C++ (`do_geometry_line`: past every statement of the body, no statement after the `do` on its line) and, for the JS family, only when its tree is rewired (`do_condition_line`, through `js_loops::do_tree_rewirable`, so the universe and the CFG always agree); a one-line `do`, a condition sharing a line with a body statement or with a following statement keeps main's edges (residual, pinned by `tests/fixtures/js_loops/variant-matrix.tsv`), and the `do` header -> exit and body-last -> `do` header edges are dropped for an admitted `do`. All added edges are `Structured`. Counters can move by construction: a two-line `do` gains its condition as a node, and a one-line owner whose loop holds a `continue` would gain its first CFG edge (a self-loop) but its loop shares every line with its body, so the plan refuses it and main's (empty) edge set and availability stand. Residuals: a tree holding a statement kind with no completion rule, an unadmitted `do`, a finally-crossing jump or a `for` initializer hazard keeps main's edges and labels (its owner is refused whole; the finally-crossing jump and initializer forms are tracked as gaps: a precise route through the `finally` and a header-phase split are the alternatives); a `switch` `break` outside any loop tree has no edge (as main); a header continuation line of a multi-line `for` is its own statement line; `catch` blocks after a loop are reached only through main's try -> catch edge. `PRISM_RD_CENSUS_TRACE=1` names each moved edge `loop-exit` or `do-cond` (the edge's lines touch an admitted `do`) from a control pass under main's loop CFG and universe (`js_loops::with_main_loop_cfg`, `reaching/loop_control.rs`, whose per-file CFG cache is keyed by path and a hash of the source), and `rd-census-owner<TAB>loop-exit` each owner whose RD availability flipped. An `if` arm that ENDS in a wrapper (a label, `try`, `switch`, a nested `if`) holding a loop of the tree, or leaving a loop of the tree from inside its body, is delegated to the tree (slice 3): main's then-end edge leaves the wrapper's ENTRY line, a bypass of the loop, so the tree drops it and emits the wrapper's own normal completions (`Cx::wrapper_completion`, the closed `completions` rule list) to what follows the arm (`after`); a wrapper whose completion is not resolvable (a form with no rule, an unknown target, a `break`/`continue` in it that the tree does not route, or two trees of the owner inside it) stays `Unresolved(WrapperEntryThenEnd)` and refuses the tree. TRY-FLOW BOUNDARY (T2, option A3; `src/cfg/js_loops/tryflow.rs`; supersedes fold 1's catch-shape rule): generic JS try construction draws header -> catch, header -> finally and first-catch-statement -> finally as `Structured`, and a finalizer's completion is not tied to the completion that entered it, so no completion rule is trusted around a `try`. An owner with loop trees is therefore REFUSED WHOLE (every tree, legacy ones too, main's geometry, labels and availability stand, decided inside the plan fixpoint) when (a) its own executable syntax contains a `try_statement` (recursive scan independent of the completion rules that follows EXECUTION regions: the owner's body and its own parameter defaults, plus the eager parts of the callables nested in it (a method's computed key, decorators and PARAMETER decorators, including TS parameter properties), and of class fields (names, computed keys, decorators and the initializers of `static` fields), with any class static block reached through them; nested callable bodies, their parameter defaults and instance field initializers (per instance, deferred) excluded; class static blocks, `with`, labels, `switch` and bare blocks included) or (b) its selected statement-line universe lies on a line of ANY try region of the file (nested callable bodies and foreign projections included; a line interval is a refusal check, never deletion authority). A nested callable's try with no line in the owner's universe leaves the owner admitted. Rule (b) also refuses a loop owner that merely sits inside another function's try body (its statement lines lie on that try region; 1 excalidraw owner). Refusing a callee also turns the cross-file argument-binding labels INTO it `cfg_incomplete` (`{ exportingFrame }` -> `opts@312` of a refused `exportToSvg`: the 3 excalidraw part-(ii) keys, lost correct exacts). Disclosed main residual, not an A3 case: an always-throwing eager block with NO try (`[class { static { throw 1; } }]() {}` in a loop, or the same in a parameter decorator, probes `ckey_notry` and `pdec_notry`) leaves `x@2->x@11` `exact` on main and here (pinned in `tryflow_tests`); exception flow without try syntax is outside the explicit-try contract. `Why::TryFlow`, counters `refused.try_flow`, `overlap.try_flow`, `legacy_o1.try_flow`. Limit: A3 contains EXPLICIT try flow only; `await`, `yield` and iterator close without try syntax are out of model (the pending-completion model is deferred). Residual, on main, not fixed here: JS `switch` has no case-dispatch edges, so after a `break` inside a delegated wrapper the `default` arm reads `killed` where the value reaches (never `exact`), beside the switch false kill. Inherited, not loop-owned: the `if`'s own skip branch edge, drawn to the next line of the sorted universe, can still reach the OTHER arm of an enclosing `if` (a wrong `exact` main has with no loop at all). JS-family CFG edges also carry transient ownership records (#693/#697 option B; slice 1 recorded them observationally, slice 3 consumes them for the replacement and nothing else, never a flag, label or the cache): `cfg/js_ownership.rs` records, in lockstep with the edge list, each edge's producer (sequential, branch entry, then-end, loop back, loop exit, `js_loops` replacement, jump, try, switch), role, syntax anchors (byte ranges and node kinds of the source, target and the construct whose completion it depends on) and an ownership state (`Known`, `Unknown`/`Ambiguous` with a reason: legacy emitter, wrapper-entry then-end, nested callable, shared line); the sequential builder's statements are resolved against the pre-dedup universe (`statement_spans_before_dedup`). `cfg/js_disposition.rs` lists, per loop tree, every record touching its lines or enclosed by an anchor (surviving, dropped by the current rewrite, or suppressed as a duplicate) as `OwnedReplaceable`, `Independent`, `Unresolved` (shared boundary line, head collapse, wrapper-entry then-end, non-completion crossing, ...) or `Foreign` (drawn for a nested callable; never a reason to refuse), where a replacement edge is owned only by the tree that emitted it (`EdgeRecord::tree`), flags `unauthorized_drop` and `surviving_bypass` against the current rewrite, and inventories the #700 mechanisms (statements dropped by start-line dedup, retained statements covered by a shorter span); `PRISM_CFG_OWNERSHIP_TRACE=1` prints it to stderr as `own-*` lines (results unchanged), and, for the standing build only (never a plan pass), one `own-drop` line per record the rewrite dropped (`own` the owner's, `nested` an enclosing owner's copy for a nested callable's tree) and one `own-emit` line per edge it emitted: the accounting of the corpus gate (every removed edge is a dropped record, every added edge an emission).

Finding confidence is `exact | nameonly | unlabeled`; tier is `asserted` only for Exact evidence
whose evidence-bearing files all parse cleanly, otherwise `candidate`. `asserted` grades the
evidence path, not the heuristic's truth.

`Exact` is a static-binding grade: the site's name resolves, by the language's declaration, scope, import
and export syntax as prism models it, to exactly one callable. It does not assert runtime truth. Runtime
mutation of module objects (monkey-patching, reflective writes, `eval`, host globals, re-acquisition through
`require`/`import()`), from the same file or another, is out of model for every rung and every language
(replan probes, 2026-09-25). Producer-side mutation is modeled only where mutation is the construction
idiom (CJS `module.exports`, `src/ast/js_cjs_export_barriers.rs`). A slice that wants to model runtime
mutation proposes it as a new contract, not as a precision fix to an existing rung.
Rust Exact does not depend on build configuration: a site is Exact only if, in every configuration in
which the site itself compiles, it binds the same callable; conditions implied by the caller's own `#[cfg]`
context are assumed true. Rungs do not evaluate `#[cfg]`/`#[cfg_attr]`: every cfg-gated item is a candidate,
and cfg alternatives that meet in one pool demote or refuse. A cfg-gated target whose competitor the rung
cannot see stays a documented gap (#527 measured 0 cfg-caused wrong Exacts in 57 adjudicated tokio sites;
fencing failed its cost bound). Tier-A's rust-analyzer oracle uses default features and the host target.
Known JS/TS, React and Node resolution gaps, with options and tracking issues, are listed in
`docs/features/language-coverage/js-ts-known-gaps.md`; update it when a slice closes or finds one.

A JS/TS local alias is followed when a bounded chain
of unique, unwritten `const` bindings reaches one callable, an import, or a namespace-import
member proven per export (a source-backed span, a relative module); that route is binding
evidence, spelling alone is not. An alias that is not followed, or whose terminal is not proven,
keeps the earlier resolution. A JS/TS `const x = H(...)`, `const { k } = H(...)` or `const [a, x] = H(...)`
(`H` a bare identifier proven to be exactly one in-repo function, by the same routes) is followed one
returned-value step: when every value-returning statement of `H` selects the same callable from an object
or array literal (spread-after-key, computed key, accessor, duplicate key, async/generator, a parameter or a
call result all refuse), the site binds that callable as seen from `H`'s file; a site the fact does not
prove (not exactly one Exact target) has exactly the earlier records, and no edge is removed.

A JS/TS call through a top-level relative `require` binds Exact (kind `import_member`, a followed binding) when it names a
callable the producer's CommonJS exports prove: `m(..)` for `const|let|var m = require('./x')` with `module.exports = f`
(`f` a declared function, or a `const|let|var` arrow or function expression), `m.f(..)` and `require('./x').f(..)` for
`module.exports = { f }`, `{ f: g }`, `module.exports.f = f`, `exports.f = f` (or a function expression member, named `g` with any `f`: #627), `module.exports = function g() {}` and `{ f: function g() {} }` (a named function expression written in place; the target is the indexed callable with the expression's own bytes, never a same-spelled declaration; anonymous, arrow, parenthesized and generator values, a duplicate or mutated export object, and any producer with an ESM export refuse these forms; ESM importers of a CJS producer gain these facts too, as they have for declared exports),
and `f(..)` for `const|let|var f = require('./x').f`. The proof is `JsExportFacts::cjs_callables`/`cjs_module_callable`
(`src/ast/js_cjs_terminal.rs`), recorded only behind the export-object barrier and the terminal proof, in a file with no
ESM export (a file with a program-level `export`, TS `export =` included, is an ES module: its `exports`/`module` writes are no export claim for any consumer form, require, destructured, `.f`, ESM import or namespace, #664; an `export` only inside a namespace does not count); the route is `src/resolution/js_ts/cjs.rs`, the binder `src/ast/js_cjs_module_calls.rs`. The declaration must be
a direct statement of the program, unwritten and unshadowed, with one plain relative string literal and a `require`
proved unshadowed at its own occurrence (no declaration, no write, no enclosing `with` body); a member site `m.f(..)` is
also refused when the consumer file writes `m.f`, any `m[..]`, or passes `m` first to `Object.assign`/`defineProperty`/
`defineProperties`/`setPrototypeOf` or `Reflect.set`/`defineProperty`/`deleteProperty`/`setPrototypeOf` (file-wide, by
spelling: a consumer-side stub; this is a partial guard, and other mutation routes such as an alias `n = m; n.f = …`,
`with (m)`, re-acquisition through `require`, reflective or computed mutator calls, or passing `m` to a function stay
under the runtime-mutation rule above); anything else, anonymous exports, class
exports, `exports.default` (not the module value), ESM producers and `new m()` keeps the earlier outcome with no new
edge. A proven site replaces earlier NameOnly or indirect records at that site, as slice L does (an R6 single-owner
method of an unrelated class becomes the Exact export; textual indirect records for the same member stand down for the one
Source record).
An anonymous or module-level caller (#597) is admitted by its own route (`src/resolution/js_ts/anonymous/cjs.rs`, evidence
`src/ast/js_cjs_anonymous.rs`, recorded in the proof index) only for a binding that is a direct top-level `const` or
`var` statement of the program (owner 2026-11-04 admitted `var`; `let` stays out), declared once in the resolved
program scope (hoisted declarations count: a second declaration of the name at program scope, or a `var` of it in a
nested block, loop, `try`, `switch` or labeled statement, is the binder's `duplicate_declaration`; a declaration inside
a nested function does not), with no write resolving to that binding (compound assignment, `++`/`--`, destructuring
targets, bare `delete`, `for (m in/of ..)` heads and writes in nested functions make the binder answer `MayCall`; a
write to a different binding of the same spelling does not count), and with the binding walk from the site reaching
the program scope without another declaration of the name. This is the binder's per-binding proof, distinct from the
file-wide consumer-stub guard on `m.f(..)` above. A declarator in a multi-declarator statement
(`var a = 1, m = require('./x')`) is judged on its own. The shapes are `m(..)`, `m.f(..)`, `f(..)` for
`const f = require('./x').f`, and `f(..)` for `const { f } = require('./x')` / `{ k: f }` (the existing flat-pattern
admission, `js_ts_object_pattern_key`: no rest, default, nested or string-key entry, and every bound name unique in
the pattern). The producer proof is the one above, plus the anonymous boundary's supported-file-environment check. The
route answers only with a certificate; whenever it cannot issue one it returns nothing and the site keeps its earlier
outcome. `let`, inline `require('./x').f(..)` and everything else keep the earlier outcome. Named callers' resolution
route and outcomes are unchanged (the proof index does extract the evidence for every JS/TS site). A module-level call
that runs before its declaration (`var a = m(1), m = require('./x')`) is Exact under the static-binding contract: the
binding never holds another callable.

`--resolution nominal` is the default and reports CPG-derived findings as `unlabeled/candidate`;
`--resolution scoped` reports retained evidence labels. `--min-confidence exact` keeps only Exact
findings. Its default, `nameonly`, retains all three confidence values, including ungraded Unlabeled
findings. The filter is supported only by finding-bearing `json`, `review`, `sarif`, and `targets`;
the CLI rejects it for `text`, `paper`, `mermaid`, and `callers`.

CPG cache (currently v182) persists DataFlow labels and per-file RD statistics across cold, full-hit, and
partial-hit builds. The B8 rule that capture reads in deferred or nested callables become
`NameOnly(CfgIncomplete)` is **PROVISIONAL** pending callable-timing work.

The C/C++ macro census (`src/call_graph/c/macro_census.rs`, `CMacroCensus`, #630 slice 6a) records per in-repo `#define` its parameters, transitive free identifiers and `control`/`poisoned` flags; slice 6b consumes it. Its digest is persisted with the cached DFG (`c_macro_census_digest`), and a partial cache hit that finds a different digest rebuilds in full. Its `incomplete` flag is repo-wide: any `#define` line the textual pass cannot read into a name, an unreadable macro parameter, an unterminated comment, an uncertain raw string, or a census input the loader lost (unparsable or symlinked C/C++, an unreadable file or directory, an oversize file with `define`; stamped on every parsed C/C++ `ParsedFile`) means slice 6b disables rules U and D for ALL C/C++. Definition lines are the union of comment-stripped and unstripped text after phase-1 normalization (BOM, CRLF/CR, trigraphs, splices with any blank), and one identifier rule (ASCII, non-ASCII bytes, UCNs; a UCN or non-ASCII name or parameter poisons the macro) serves the name, parameter and body readers. Fold 3 also sets `incomplete` for: any directive line holding a non-ASCII byte or UCN outside string/char literals and comments (either dialect pass; any non-ASCII byte when a pass cannot read the line's literal boundaries) or a trigraph anywhere (an exotic-lexing belt); a define-looking line no pass can read; any `Unreadable` loader skip (a directory too), ParseFailed/Symlink skips of C/C++ files, and the census-input losses below; and `prism review` when the repository lists a C/C++ file it did not parse or holds another unread census input. Census inputs follow the hybrid rule (slice 6b; "every file is a header" measured incomplete on redis and cpython and cost +14% on prism's own tree): (1) parsed C/C++ files use the full census lexer; (2) include discovery is the primary input: the `#include`/`#include_next`/`#import` targets (quoted or angle) of every parsed C/C++ file, found on comment-kept and comment-stripped variants in both dialects, splice modes and trigraph modes, and transitively of every census text, are normalized (`.` and empty segments dropped, `..` resolved lexically, `\` read as `/`) and matched against EVERY in-repo directory (a repository file whose path equals the target or ends with `/<target>`, quoted targets too so `-I` directories are covered); a leading `..` that cannot be resolved is dropped for quoted and angle targets alike (the include directory can be anywhere in the repository, so the match is by suffix); every resolved target that is not a parsed C/C++ file is read as census text whatever its extension or language (`macros.py` included); (3) the C-like-extension net: every in-repo file with a C-like extension (`.c .h .cc .cpp .cxx .hh .hpp .hxx .h++ .c++ .inl .inc .ipp .tcc .tpp .txx .def .cu .cuh .x`, any case) that is not a parsed C/C++ file is read as census text even when nothing visibly includes it (computed includes, `.git/` at any depth and `eval/secbench` excluded; hidden and gitignored directories included); (4) census texts use a PERMISSIVE scan (`collect_permissive`): after phase-1 normalization (BOM, CR/CRLF, trigraphs, lenient and strict splices, unioned; a NUL-stripped variant for BOM-less UTF-16) any `#`/`%:`/`??=` + blanks + `define`/`undef` (read on the comment-kept and a comment-erased variant, `/* .. */` incl. across newlines and `// ..` erased best-effort, so `#/**/define`, `#define/**/N` and `#include /**/ <x>` work in a text input and in transitive include discovery) + an identifier name anywhere on a line is a definition whose body's identifier-like runs are free names, `##` poisons, the control spellings set control, a non-identifier name is no fact, there is no comment or string tracking (a define in a comment counts), and such a file NEVER sets `incomplete` from lexer state; a non-UTF-8 file is decoded lossily and UTF-16 with a BOM as UTF-16; only an unreadable file or directory, a symlink that points outside the repository or cannot be read, a symlinked directory pointing outside the root or into `.git`/`eval/secbench` (an in-repo directory link is followed: an include target is walked segment by segment from every directory, a directory link entered at its target, so `#include "link/x.h"`, a link to the root, a link inside another link's target and a link cycle all resolve, bounded by the segment count), or a non-C input over 64 MiB that contains ANY directive leader byte sequence (`#`, `%:`, `??=`, also NUL-interleaved; scanned by streaming; a file with no leader holds no directives and is skipped), is `incomplete`. A symlinked file with a C-like extension, or one an include resolves to, is read through the link when its target lies inside the repository (a C-like symlink is also a loader loss, `is_loss`). A header with an unusual extension reached only through a form prism cannot resolve (a computed include, a generated header absent from the repository) acts like an external header, the same boundary as D2 (precise follow-ups #813, residual lexing shapes #812). The walk reads only parsed files, resolved targets and C-like files (a directory listing finds them); a repository with no parsed C/C++ file does no census walk (load or freshness). The cache identity is ONE aggregate `c-census:aggregate` entry of `LoadedRepo::manifest_hashes` (digest of the facts the texts yield, the files read, the contributing files, the lost inputs), so an unrelated edit leaves the topology alone and a new, edited-with-different-facts or removed define-bearing input moves it. The freshness bootstrap recomputes the aggregate (and compares the kept texts with the disk through the census's own decoding), so a new define-bearing C-like file or a newly resolvable target marks the session stale and an unrelated edit does not; census identity stays apart from `scope_graph_inputs`. Review mode fails closed (`c-census:review-incomplete`) when the repository holds an unparsed C/C++ file or any other input that yields a fact or a loss. A delimited (`\u{..}`, `\U{..}`) or named (`\N{..}`) UCN counts as a UCN everywhere a UCN does (hazardous on define/undef lines, a poisoning non-ASCII spelling in a name), while a bare `\u`, `\U` or `\N` not followed by hex digits or `{` is not one. Their hashes and the skipped-file aggregate are `c-census:` topology entries, so a full cache hit cannot replay a stale census. Each pass also runs in C dialect (no raw strings or digit separators), and the results are unioned. Soft poison is cleared only when the same source definition was read cleanly by another pass. Control is any identifier containing `setjmp`, `vfork`, `getcontext` or `savectx`, plus `goto`, `case`, `default` and the asm spellings.

C rules U and D (#630 slice 6b, `src/data_flow/build/c_prune.rs`, driven by `lexical_prune.rs` through explicit `Language::C` dispatch; counted in `dfg_pruned_write_only`/`dfg_pruned_dominated`; design of record `docs/superpowers/plans/2026-11-11-630-s6-cpp-ud/DESIGN.md` §5.1): `function_definition` owners of `.c` files only (`.h`, C++ and every field path stay off: both rules drop identifier paths only). An owner is off for a parse error, a nested function, a census with `incomplete` (every C/C++ owner, repo-wide), a `#define`/`#undef`/`#include` or `#pragma omp|acc`/`_Pragma` in the body (G2, a textual belt over the owner's logical lines: continuations spliced and comments stripped first, in both dialects, so `#pragma /*gap*/ omp` and a continued `#pragma \` + `omp` are caught), asm or SEH (G3; a Microsoft-extension node `ms_*`, i.e. `__declspec`, calling conventions, `__restrict`, pointer modifiers, is refused whole as `G3Ms` and counted apart in the ledger), a returns-twice name (`setjmp` family, `vfork`, `getcontext`, G4) and an invoked in-repo macro marked `poisoned` (G5); D alone is also off when a `goto` names a label that is not a visible `labeled_statement` of the owner or an in-repo macro marked `control` is invoked (G6). Every use-line occurrence (and, for D, the write's target) must resolve lexically to ONE eligible declaration (E1: parameters, or locals declared before the occurrence in an enclosing block, `for` initializer or switch body, innermost scope first; unresolved, ambiguous and K&R-style names keep; a declaration that is not a direct child of a block, a direct `case` or a `for` header (under `#if`/`#ifdef`/`#else`/`#elif`, a label or a nested `case`) is invisible to the resolver, so every name it declares joins the owner-wide escaped set; arrays, function declarators and declarations whose specifiers name an in-repo macro are ineligible), with no `static`/`extern`/`volatile`/`_Atomic`/thread-local storage (E2), and the spelling must not have escaped anywhere in the owner by name (E3: `&x`, `&(x)`, `&x.f`, `&x[i]`, any identifier between the parentheses that follow an in-repo macro name or a `va_*` builtin, found in the owner's normalized token stream whatever node the parser made of it (a call argument, a `macro_type_specifier` type as in `BIND(x) dummy;`, a declarator parameter as in `long M(x);`), plus the callee-field forms `a->M(x)` and `a.M(x)` and the parenthesized name `(M)(x)` (which is not a macro invocation, since a function-like macro expands only before `(`, but is conservatively escaped like one), and every `(` group that directly follows an invoked in-repo macro's argument list, repeated for chains (`ID(TRACKED)(x)` is `TRACKED(x)`; punctuation is not a token of the stream, so a group a `;` or operator separates is read as a chain too, which only escapes more), the transitive free identifiers and parameters of every in-repo macro spelled in the owner (read from the owner's preprocessing identifier tokens, whatever node kind the parser gave them: `#define const x = saved;` makes `const` a macro use, and every census fact applies: free names, parameters, `poisoned`, `control`), and the spelling itself being an in-repo macro name). E4 (D): a non-pointer declarator whose name has any member or subscript access in the owner keeps (a sub-object write after a whole write carries the old value). A write is plain for U when its span is exactly an identifier that is the `left` of an `=` assignment, or the declarator identifier (through pointer and parenthesized declarators) of a declaration or `init_declarator`, never an initializer value (`int y = x;` records `x` as a span but is no write of `x`). A dominating write for D is plain and is a direct child of a region: an `expression_statement` holding the assignment, or a `declaration` with an initializer; compound assignment, comma and conditional operands, unbraced arms, `for` headers and anything under `#if` never dominate. Regions are a `compound_statement` that is not a switch body and a `case_statement` that is a direct child of a switch body (statements before the first case have no region). D also refuses a window in which a `labeled_statement`, or a `case_statement` that is not a direct child of a switch body, starts in `[W.start, child_end)` at any depth (a goto or Duff entry); direct cases of a nested switch do not refuse. The D candidate must pass a variable-write certificate: its raw span is an identifier spelled as the base in the `left` of an assignment (any operator), a declarator, or a parameter, and resolves to the window's own declaration; dereference stores (`*p++ = c` is recorded as a def of `p`), subscript and update spans, initializer-value spans and alias twins are always kept. Out of model (disclosed, owner D2): a macro defined outside the repository (system or third-party headers, `-D`, `-include`) acts as a function: it reads and writes only what its arguments spell, takes an address only where an argument spells `&`, and hides no jump target, `case` label or returns-twice call; a `.c` file compiled as C++ gets the C rules. Per the #634 rule the survivor-based fallback cap lets a formerly capped C owner regain its surviving edges when pruning brings it under the cap, so removal-only is not strict for such owners (disclosed, as for Go, Python, Rust and JS; not changed). Z04 and Z05: the census digest rows are 6a's; the formerly-capped row is `c_prune_formerly_capped_owner_regains_survivors_per_634`, which pins the #634 behaviour rather than forbidding it. `PRISM_C_PRUNE_TRACE=1` prints one `c-prune-owner` line per C owner with its verdict. Tests: `c_prune_tests.rs` over `tests/fixtures/dfg_prune/c/` (rows in `obligations.jsonl`, generated by `gen_obligations.py` from the markers in the fixtures and in `c_prune_inline_cases.rs`; inputs that set repo-wide census facts, `#define` included, live inline, never under `tests/fixtures`). Macro facts come from the census's hybrid inputs (include discovery plus the C-like-extension net, permissive text scan); a header with an unusual extension reached only through a form prism cannot resolve (a computed include, a generated header absent from the repository) acts like an external header, the same boundary as D2 (#813 precise follow-ups, #812 residual lexing shapes).

JS/TS/TSX write census (#660 S1b-1, `src/cpg/reaching/census_defs.rs` over `src/ast/js_write_census/`; design
`docs/superpowers/plans/2026-11-05-660-js-def-census/DESIGN.md`): a write in the owner's body that the producer records no
def for (update, pattern default, member target in a pattern, `delete`, wrapped or member `for` head, eager class position)
is a kill-only def at its statement node, for bases the owner defines. It gens and kills by `affects` (a prefix write kills
the paths below it), never emits an edge, is never a candidate and never counts in a collapse group; label-only. Only the
solver runs it (one walk per owner; the cap preflight is main's, on recorded defs). The census never changes availability, the
def cap or any counter: a completed list that would trip the one-node distinct-bases belt or the 2,048 cap runs RD on the
recorded defs and labels the owner `NameOnly(CfgIncomplete)` (`owner-refuse:one-node|def-cap`). Refused the same way: an owner with an escaped identifier spelling, a parse error
or a refused target kind (no kill-only defs added); a base whose
eager site maps to no statement node (class or enum declaration lines, or a statement lost behind another on its line under a compound statement) is refused for its own edges. The one-node `first_hazard` guard is deleted (#660 S1d-2): `single_node_cfg_is_complete` is JS family, one statement line and distinct recorded bases, and its six hazards (update, pattern default, hoisted declaration, `arguments`, eval, nested-callable default) are labelled by the census, D3, the S1d-1 refusal classes and the S1a/D2-c capture rules, so an owner the guard over-refused is now `Exact` when the hazard is harmless (strict or non-simple `arguments`, an update or pattern default of another name). Availability counters move (`dfg_rd_functions_without_cfg` falls by the flipped owners); edge sets are unchanged. An owner newly admitted to RD is exempt from the fallback edge cap like every RD owner, so fallback-cap counters can fall (and edges appear) for owners the guard used to refuse (zero on the three corpora). A member write in the owner's own parameter list refuses (`own-params`) the edges from earlier defs to the uses it affects. The trace names those owners (`guard-deleted:<a-f>`, from the old predicate kept as the trace's control in `reaching/guard_control.rs`). An expression-bodied arrow whose body is itself a callable (`k => () => { .. }`, a function or generator expression) or a ternary takes the one synthetic body node whatever its located count (#759, `cfg::js_single::arrow_body_takes_one_node`): `statements_in_function` had reported the nested callable's own block statements, or the calls of a ternary's mutually exclusive arms, as the arrow's, so the census never saw the callable's writes (the outer owner left `t@3->t@5` `exact` beside the inner owner's `killed`) and a def in one arm reached a use in the other as `exact`. The nested body is now a deferred region of the outer owner (S1a, D2-c), its labels `sameline` like any one-node owner, or `cfg_incomplete` where the distinct-bases belt refuses the node; `||`, `&&` and `??` bodies keep their universe (left to right: a skipped def can only over-kill, never give a wrong `exact`), and so does any other body holding two or more located statements. A belt-refused owner is RD-unavailable, so it is subject to the fallback edge cap (1,000,000 raw emissions) that main's multi-statement universe was exempt from: above the cap its edges are dropped (pinned by `arrow_body_cap_tests`). A ternary body also costs correct `exact`s when no arm writes: a formal on the body's first line becomes `sameline`, and a def in the condition makes the belt refuse the owner (`cfg_incomplete`, then the cap applies); neither occurs on excalidraw, express or TypeScript src (rows `h01`, `h02`). An array body holding a nested callable and two or more located calls keeps the wrong outer `exact` (pinned as row `g01`, design issue #784). The trace names the moved edges `arrow-body-node` (control: `reaching/arrow_control.rs`, main's universe behind `with_main_arrow_universe`, solved only when the trace is on).
`PRISM_RD_CENSUS_TRACE=1` names each moved edge (`kill:<form>`, `owner-refuse:..`, `site-unmapped`, `d3-use|def|loop|unknown`, `d2c`) from a census-free control pass.
Recorded defs kill by `affects` too in JS/TS/TSX owners (#674 G1/G2, `def_buckets.rs`, flags in `affects_writers.rs`): a recorded
write kills every recorded def of its base at or below its own path, `[]` matching any component either way (`o = {}` kills `o.x`;
`o["x"] = 2` kills `o.x`), by a per-base trie so work follows the affected defs. One rule (`affects_writers.rs`): a recorded write kills this way only when its recorded path equals, base and every field, the census's precise identity for the same write site (matched by target byte, `Completion::covered`), its `this` is not rebound, its base is not a catch parameter or block/`switch`-level function or class declaration, and it is not inside a nested callable or instance field initializer; every other recorded write (truncated `o[k].x`, calls or tagged templates in the chain, asserted `this`, text-split keys, formals, destructured and loop-head names; a plain `let`/`const`/`var o = ..` declarator is eligible) and every other language keeps path equality. Same-node writes kill nothing. Labels only move toward NameOnly, usually
`nameonly/killed`, and `exact -> cfg_incomplete` when a new kill removes the last unflagged route; a `kill_line` can move to the
lowest kill, a `[]` may-kill included. `kill:affects` names each such moved edge in the trace (from a control that keeps the census).
Captured writes (D2-c, #660 S1c, `reaching/d2c.rs`, `js_write_census/captured.rs`): a write in a nested callable (parameter list or body) or an instance field initializer that no declaration inside that unit provably scopes (`var`/body function/parameter/named-function-expression name of the callable, `let`/`const`/`class`/catch by block, `for` head, `switch` body or clause, a block function its block only; a body declaration never binds a parameter default) turns every `exact` or `sameline` edge into `NameOnly(CfgIncomplete)` when its candidate def is the written binding (the innermost declaration by scope alone, TDZ aside, so a closure writes the later `let` it names, and a parameter's scope excludes its decorators, a switch-local `let` has the switch's scope, and a def binds the written declaration unless a narrower same-name declaration nested in it does; any binding when unresolved), `affects(write, use)` holds and the use lies outside the unit. No order or creation proof (recovery #667); a killed or already incomplete label is kept.
Same-node order (D3, #660 S1b-2, `reaching/d3.rs`): an `exact` edge D->U whose use node (or def node) holds another write of
an `affects`-related path that the S0 `precedes` relation does not prove after U (before D) is `NameOnly(SameLine)`; any
loop, `unknown` or deferred kind on a chain, an unplaced position or two occurrences that disagree refuses, and only a node's
single `RW`/`delete` operand read is excused. The DFG keeps one use per path and line, so every read of the path on the line
counts (found as S1a places a region read; which node a position is in is reaching definitions' answer alone, #696: a write is in the node `StatementLookup` gives its line, a read in the node of the use's line, and the census's span placement only orders two positions inside that node, so a read in a later statement of the line refuses and an unmappable counting write refuses its base). Label-only; a header def with a same-node increment (`for (let i..; i++)`) and a
repeated context are the accepted cost (recovery #670).
Refusal classes (#660 S1d-1, `reaching/refusal_classes.rs` over `js_write_census/refusals.rs`): writes not spelled as a target move an `exact` or `sameline` label, never a `NameOnly` one, and add no edge, def or counter. A direct `eval` anywhere in the owner and a sloppy mapped `arguments` (the formals of the owner and of every enclosing non-arrow callable, by name), a block-level function declaration in sloppy code (Annex B, a formal's name excepted), a `var` of a catch parameter's name inside `catch (a)` (G6a), a write to a function expression's or a class's own name from inside it (G6b, no declaration scoping it), a use or def inside a `with` body and an owner defined inside one become `NameOnly(CfgIncomplete)`; a formal replaced at entry by a hoisted function declaration (through labels only) or written by a later default in the own parameter list is `NameOnly(Killed)` from its own def. Which environment a position is in (parameter list, body, or eager: computed key, decorator) is one rule, `js_write_census/position.rs`. Trace names `eval`, `arguments`, `annexb`, `catch-var`, `immutable-name`, `with`, `dead-formal`.

JS/TS/TSX parameter Defs: a formal that is read bare gets a Def at its identifier. A *member-only*
formal (no bare read, including a bare read inside a member object such as `wrap(p).x`; at least one
static non-computed `p.x`/`p?.x` read outside erased TS type positions; callable free of ERROR/MISSING
nodes and of `\`-escaped identifiers; not a seam binding) also gets a base Def, with
`Def(p) -> Use(p.<path>)` projection edges to the outermost member path of each read (prefix reads
included). One body walk (by bytes: the parameter list and its defaults are never projected,
declaration-line body reads are) admits only bound, evaluated reads outside `with` bodies, and builds
each Use at its exact span. Edges only start at the base Def, never between two field paths, so a field
seed (`req.body`) never reaches a sibling (`req.headers`); FullFlow's backward-then-forward walk does
not expand from a projection-only base Def (`CodePropertyGraph::dfg_is_projection_only_def`). The
accepted cost is that a seed on the whole parameter reaches every `p.<field>` read. A member write
bound to the formal suppresses its path for the whole callable (flow-insensitive): plain, compound and
logical assignment, `++`/`--`, `delete`, array/object/rest pattern targets, `for (p.x of/in ...)`, and
the same inside nested callables; a write to `p`, to a prefix of the path, or below it (`p.x.y = 1`
also suppresses `p.x`) all count. `p[k]` counts as a bare read; `this.x`, destructured formals and
`arguments` are not handled. A bare read in a default of the own parameter list (`u = o`, `u = g(o)`,
`{ k = o } = {}`, a computed pattern key; a nested callable's read counts, as in the body) makes the
formal mixed, like the same read in the body (#761; `o[k] = v` written in the list is not a read, it
stays `own-params`' domain). A default that is a plain identifier, or a destructuring pattern with an
identifier value (`[u] = o`, `{ a: u } = o`; the body's pattern extractors, owner kind by owner kind), is
the body alias `const u = o` placed at the parameter: the alias binding gets its def and the alias twin
is minted by the same machinery (named owners: the alias map; anonymous owners: the alias def's own
span), every one of them on the owner's first line with the identifier's bytes (where main puts every
formal's def). So a write through `u` is a write of `o.<path>` and the formal's edge is not `exact`
across it, with the body's limits: the alias twin never kills an earlier direct def of the path
(#783), wrapped aliases, a conditional rebind, a deferred nested-callable write and a literal
right-hand side are not seen, and an object-pattern alias leaves the formal's own edge `exact` (the
field twin shares its node). A self or forward reference (`u = u`, `a = b, b`: a TDZ error) and an
alias of a refused formal (seam, eval, `arguments`) are not aliases. See the known-gaps doc for the
residual list. `PRISM_RD_CENSUS_TRACE=1` names the owners as
`rd-census-owner<TAB>param-alias|param-bare<TAB>file<TAB>owner<TAB>first<TAB>last`. See
`docs/features/language-coverage/js-ts-known-gaps.md` §6.

An object-literal argument `f({ k: v })` to an Exact-singleton call binds field-selectively
(`src/cpg/object_literal_args.rs`, #594): each leaf `v` (identifier or member path; nested literals
flatten to `k.j`) gets `Use(v) -> Use(p.<read>)` edges to the formal's reads that the definition
already reaches, for reads whose static chain is a prefix of, or extends, the leaf's key path (so
`{ a: x }` feeds `p.a`, `p.a.b` and bare `p`, never `p.b`; a mixed formal's member reads are base `p`
Uses and are classified from the syntax, with the member-only lane's flow-insensitive write
suppression applied: a read whose path is written, or has a written prefix or a deeper write, is not
fed, and a leaf at or below a written path feeds no read). The label is the call's label joined with
the definition edge's. A spread or computed key refuses every key written before it; for duplicate keys the last
property wins, and accessors, methods and `__proto__` are never bound. The spread's own source,
non-data values (calls, arithmetic, arrays), a spread argument before the literal, NameOnly or
multi-target calls, and destructured, rest or bare-arrow formals bind nothing. A mixed formal's reads
in a declarator initialiser, a condition, a `for-in` head or an `in` test have zero-width base Uses and
are not fed (known gap).

### MCP Adapter

> **User-facing install/usage guide:** [`docs/MCP.md`](docs/MCP.md) — `claude mcp add` / Codex / Kiro
> wiring, cache warming, the gotchas, and the bundled agent **skills** in [`skills/`](skills/)
> (`prism-code-navigation`, `prism-code-slicing`). This section is the architecture notes.

`prism-mcp` is a local stdio MCP server, the `crates/prism-mcp` workspace member (library `prism_mcp`,
binary `prism-mcp`, integration tests `crates/prism-mcp/tests/mcp`). It depends on `prism` as a library,
and the `mcp` cargo feature no longer exists (#731). Run it with:

```bash
cargo run -p prism-mcp --bin prism-mcp -- --repo /path/to/repo
```

On macOS it re-execs before dispatch with `MallocSpaceEfficient=1` unless the caller already set it or `PRISM_MCP_MALLOC_SPACE_EFFICIENT=0` opts out; see the environment table in `docs/MCP.md`.

The `macos-footprint` CI job runs allocator tests and an ignored idle-unloaded footprint guard; see `docs/MCP.md` for the local command.

Startup modes: by default the repository index is built on the first valid `tools/call`
(`initialize`, `ping`, and `tools/list` never start it, so a client that only handshakes costs
a few MB); `--warm-at-startup` starts the background build at process start; `--eager` builds
before answering `initialize`. See `docs/MCP.md`. `--shared` (opt-in, Unix) serves through the
shared per-repository daemon instead of an index of the process's own (the shim, below).

The server exposes nine tools: seven read-only navigation tools (the six graph/evidence
queries return Prism `Evidence` JSON; `nav_symbol_spans` returns a dedicated coordinate
result), one read-only reasoning tool `taint_reaches` (also Evidence), and one
non-destructive local-state-changing tool `refresh_index` (returns a refresh summary).

- `nav_nodes_at` — evidence for a repository file and 1-indexed line.
- `nav_symbol_spans` — exact read-only callable coordinates without source text.
- `nav_callers` — incoming callers for a symbol or location seed.
- `nav_callees` — outgoing callees for a symbol or location seed.
- `nav_ego_graph` — local graph around a symbol or location seed.
- `nav_module_deps` — outbound module dependencies for one file.
- `nav_repo_map` — whole-repository module dependency graph.
- `taint_reaches` — forward taint reachability from a seed.
- `refresh_index` — re-indexes the repo snapshot for this server session.

Build, test, or lint MCP code with `-p prism-mcp` (or `--workspace`); a bare root
command covers the root package only.

`Evidence` also has an additive optional `reasoning` field. It is omitted when
absent so existing navigation and diff-review output remains byte-compatible.

Transport layout: `crates/prism-mcp/src/framing.rs` holds the newline-delimited JSON framing as one generic
`LineTransport<R, W>`; stdio and the Unix-socket `UnixStreamTransport` are instantiations of it.
`crates/prism-mcp/src/transport.rs` splits a message into `handle_control` (lifecycle, `tools/list`, call
validation; needs no index) and `dispatch_tool_read` / `dispatch_tool_refresh` /
`dispatch_tool_auto_refresh` (shared versus exclusive access to the provider). Output settings are
an explicit `ConnectionSettings` (`crates/prism-mcp/src/settings.rs`), resolved from the environment once per
server, so an invalid `PRISM_MCP_*` output value warns once, at server start, not once per call.
`tests/fixtures/mcp/golden/` pins the in-process server's stdout; re-record with
`PRISM_MCP_BLESS_GOLDEN=1 cargo test -p prism-mcp --test mcp golden_transcript_test` only for a
change that is meant to alter tool output, and say why in the commit.

Shared-mode building blocks (`crates/prism-mcp/src/shared/`, Unix only; the daemon and the shim that use them
are later slices): `identity.rs` computes the daemon identity key, a SHA-256 over the canonical
repo root, its device and inode, the build identity, the shim–daemon protocol version, the
canonical cache base and the refresh policy; `cache_base.rs` gives `CacheMode::Default` and an
explicit `--cache-dir` one canonical spelling before the directory exists; `runtime_dir.rs`
resolves `$PRISM_MCP_RUNTIME_DIR`, else the user runtime directory, else the temp directory,
joined with `prism-mcpd`, and validates it on every use (a real directory, owned by the current
user, mode 0700, no access control list on macOS, and nothing on its resolution that another user
could replace by owner, mode bits or list, which `dir_chain.rs` walks; this is hardening, and the
guarantee between users is the peer-uid check the daemon and the shim make); `paths.rs` names a daemon's files by the key's first 16 hex digits and enforces
the 103-byte socket path limit; `preamble.rs` and `preamble_reader.rs` encode, decode and read the
one-line `HELLO` / `READY` / `RETIRING` / `BUSY` / `MISMATCH` exchange (4 KiB bound, one accepted
spelling per message). Settings a client sends in the preamble are a `ClientSettings`
(`crates/prism-mcp/src/settings.rs`), whose constructor refuses a result cap below the floor and a first-call
wait above 600 s. A change to what goes into the key or to the preamble's wire text bumps
`SHARED_PROTOCOL_VERSION`.

Shared daemon (`crates/prism-mcp/src/daemon/`, Unix only; `prism-mcp daemon --repo X [--cache-dir D]
[--refresh-policy P] [--warm-at-startup] [--idle-ttl S] [--max-clients N] [--key HEX]
[--index-idle-secs S] [--index-idle-pressure-relief]`, internal,
started by `prism-mcp --shared`): READY/STATUS expose effective `index_idle_timeout_secs`, `index_idle_pressure_relief` and the distinct `unloaded` build state under protocol/control version 2.
`coordinator.rs` holds `DaemonState`, which is a control plane
(`Mutex<ControlState>` and one `Condvar`: the build phase `Idle | Building | Ready | Failed`, the
attempt count, the parked calls, the generation of the provider in the slot) and a provider slot
(`RwLock<Option<SessionProvider>>`). `ensure_serving` (production; `ensure_ready` is test-only) picks its build attempt once and waits for
that attempt only, up to the connection's `ConnectionBudget` (`budget.rs`: one deadline per
connection per attempt, and the outcome of the attempt it last joined); it is answered from that
attempt's own `AttemptOutcome`, written once, whatever the phase has become since. A connection
that stopped waiting before its attempt failed hears that failure on its next call before
anything is started (the in-process server's `warming`, `build_failed`, retry). A failed attempt
is retried by the next call from a connection with no unheard failure, once however many calls
arrive, and the build thread puts the provider in the slot and records its generation
before it announces `Ready`. The generation is recorded under the slot's write guard at
publication and after every exclusive access, so it is the slot's whatever the phase. Lock order: code holding the control lock
does not touch the slot; code holding the slot may take the control lock. A panic in a query or
a refresh (`read`, `write`), on the build thread, or in the control plane (`ensure_ready`,
`status`, the `--warm-at-startup` start), and a lock found poisoned, end the daemon through
`Parts::on_fatal` (`fatal.rs`: a `FatalOnUnwind` guard made right after each entry takes its
lock; production `abort_daemon` writes one line to stderr and
aborts, so every attached client sees its connection end); nothing is read through a poisoned
lock, and a test installs a `FatalLog` instead of the abort. Time is the injected
`Clock` (`clock.rs`), so tests move a manual clock and never sleep to order events.
`control.rs` holds the types the control lock guards (`BuildPhase`, `ControlState`,
`AttemptOutcome`, `BuildStatus`) and `Answer`, what `ensure_ready` tells a call: `Ready`,
`Warming`, `Failed` with the call's own attempt's error, or `Superseded` when that attempt
failed and a later attempt is already building; when a later attempt has already published the
index, a call whose attempt failed is answered `Ready` and served (the pure rule `answer_for`).
`connection.rs` serves one admitted connection (`serve_admitted`, which first sets the stream
blocking with no read timeout): `transport::serve_loop`, the
frame loop the in-process server also runs, then `handle_control` with no lock, then for a tool
call `ensure_ready` followed by `dispatch_tool_read` under the slot's read lock, or
`dispatch_tool_refresh` / `dispatch_tool_auto_refresh` under its write lock. Nothing is served
from the slot before `ensure_ready` has answered `Ready`. A query sees the whole provider, so
stale evidence a refresh recorded outranks a clean stamp probe for every client. Before the
index is ready a call gets the in-process server's `warming` or `build_failed` result; a
`Superseded` call gets the daemon's own retryable `indexing` status (retry in
`INDEXING_RETRY_AFTER_SECS`; ignore it if a retry already succeeded), which the in-process
server never sends. Under an automatic refresh policy a query that found the index stale asks
the read's question again once it holds the write lock: one that now finds the index current
(another client refreshed in between) is answered as a read, so clients that race on one stale
index refresh it once, and one that still finds it stale (sticky evidence included) refreshes.
The query after a refresh runs under the same write lock, so a response and the refresh
generation it reports are one snapshot's. No lock is held while a response is written: a client
that stops reading stalls only its own connection. A panic anywhere else on a connection's
thread (framing, the readiness answers, writing a response) ends that connection only:
`serve_bounded` turns it into an error and the daemon goes on.
Admission and retirement (`admission.rs`) are entries of the control plane: each takes the lock
through `DaemonState::control_plane`, whose `ControlPlane` makes the panic guard right after the
lock. Pending and admitted connections are counted together against `--max-clients`; the idle
time counts from the moment both reach zero (or from the first decision, for a daemon nobody
reaches); the drain decision is taken once (idle TTL passed with no build running, or forced)
and the socket's unlink runs inside it, so nothing is admitted after it. A warm daemon's
connection starts holding the startup attempt (`budget_for`), so a startup build that failed
before its first call is reported as the in-process server reports it. `files.rs` owns a key's
files: the owner lock (`flock`, held for the daemon's life; a second claimant is told `Taken` and
touches none of the owner's files), the socket (a stale one is replaced by the owner, and the
owner removes it only while it is still the one it bound), and the pid file; every file but the
socket is opened through the validated runtime directory's descriptor with `O_NOFOLLOW`, and lock
files are created with `O_EXCL` and never removed.
`server.rs` is the accept loop, one thread polling the listener and every pending connection:
a peer of another uid (`peer.rs`) is dropped before any reply and never counted; a pending
connection's `HELLO` is read from a non-blocking socket against its own deadline (10 s) and
answered `READY` (then a worker thread runs `serve_admitted` and releases the admission when it
returns), `MISMATCH` (another identity or protocol version), `BUSY` (over `--max-clients`), or
nothing (a refused line). The root's device and inode and the daemon's own files are checked on
every admission and every 5 s; a change, SIGTERM/SIGINT, or the idle retirement drains the
daemon: socket unlinked under the control lock, `RETIRING` to everything still pending or queued,
pid file removed, owner lock released, exit 0. `run.rs` is `run_daemon`: resolve the identity
(refusing a `--key` that differs), own the key or exit 0 silently, bind, write the pid file,
serve. `log.rs` caps the daemon's own lines (log truncated at start over 1 MiB, capped at 8 MiB).
While its builder runs the daemon leaves `<key16>.build` (`files.rs::BuildMarker`, set and cleared
around the builder by `run.rs::marked`, removed at release); a daemon that dies during its build
leaves it. The marker shares a fence with `DaemonFiles`: release (or drop) sets it under its lock
before the owner lock is released, so a released daemon's late `set` or `clear` does nothing.
An operator's control line (`shared/control.rs`; spec §4.7) is answered in place of a `HELLO`:
`STATUS {"control":1}`, or `SHUTDOWN` naming one instance by identity, pid and start time (the
exchange has a version of its own, not in the identity key, so an operator tool can ask a daemon
of another build). The daemon answers one `STATUS` line (instance, root, build identity, clients,
build state, idle time, whether a shutdown is requested), or `RETIRING`/`BUSY`, and closes; the
connection is never admitted, and `admission.rs::control_exchange` takes it out of the pending
count without restarting the idle time (`quiet_since`), so a probe does not keep an idle daemon
up; a control line whose read completes across its handshake deadline is dropped the same way, unanswered
and with nothing recorded. A connection that ends before its line is whole, or whose line is
neither a control line nor a `HELLO` (which is what a daemon built before this exchange makes of a
`STATUS`), is treated as a client's and restarts the idle time. A `SHUTDOWN` for this very
instance makes the daemon drain once nothing is admitted or pending, whatever the idle TTL or a
running build (`StopReason::Shutdown`); until then it goes on admitting new clients, and it never
disconnects one.

Shim (`crates/prism-mcp/src/shim/`, Unix only; `prism-mcp --repo X --shared [--daemon-start-timeout S]
[--idle-ttl S] [--max-clients N]`; `--shared` with `--no-cache`, `--eager` or `--owner-*` is a CLI
error, and `--standalone`, the default, changes nothing): `connect.rs` makes one attempt to
reach a daemon: connect without blocking (`socket.rs`; a full listen backlog fails at once), check
the peer's uid (`daemon::peer`; a peer of another uid is refused before anything is sent), send
`HELLO`, and read the one reply line with every byte's read bounded by the time left to the shim's
one deadline and no line accepted after it. `READY` for this key reaches it (the socket's timeouts
are cleared); `RETIRING`, an end of stream or a reset before a reply, a failed connect, or no reply
by the deadline are transient; `BUSY`, `MISMATCH`, `READY` for another identity and an unreadable
line are refused (fail closed). `arbitrate.rs` is the start protocol: the socket first; then the
start lock (`<key16>.start`); under it the socket again, then the owner lock: held means a live
daemon (poll every 10 ms; never touch its files), free means none (holding it, remove the stale
socket, pid file and build marker, release it, start a daemon through the `Starter` seam). While a
started daemon neither answers nor ends the owner lock is not looked at again; one that ends 0
before `READY` lost the key (look again), one that ends otherwise fails the start. A free key with
a build marker is a daemon that died during its build: the first shim to see it records the
sighting in the marker, and the key is not restarted for 60 s from that sighting (the restart bound
for a build that keeps dying, however long it runs). `key_files.rs` opens the key's files through
the runtime directory's descriptor with `daemon::files`' functions. `spawn.rs` starts `prism-mcp daemon` from the
same executable with the shim's canonical root, cache base, refresh policy, full key and resolved
`--index-idle-secs S` (including zero), plus `--index-idle-pressure-relief` when requested (and
`--warm-at-startup`, `--idle-ttl`, `--max-clients` when the shim has them), in a session of its own
(`setsid`), from `/`, standard input null, output appended (`O_APPEND`) to `<key16>.log`, without
`PRISM_NAV_EDGE_CACHE_LOAD_DIRTY`. `diagnostic.rs` writes the one fail-closed line (reason, key,
runtime dir, socket, pid file contents, log, and the `--standalone` invocation carrying the selected
idle seconds and relief intent, named for failures
before the key is known too; control characters escaped); exit 1. `pump.rs` carries the session
byte for byte: the client's end of input half-closes the socket and the daemon's last responses are
still delivered (exit 0); a daemon that closes first ends the session (exit 1) and the shim never
reconnects. `run.rs` is `run_shim`; on `READY` it writes `prism-mcp: shared daemon pid N (started
T, generation G) at <socket>; index STATE; index_idle_timeout_secs=S; index_idle_pressure_relief=B`
to stderr. A differing applicable request adds one mismatch line naming effective/requested
seconds (and relief when it differs), keeping first-spawner policy; inapplicable relief gets the unused note.

The operator's commands (`crates/prism-mcp/src/shim/`, clients of the same sockets and files; spec §4.7):
`prism-mcp daemon-status [--repo X [--cache-dir D] [--refresh-policy P]] [--runtime-dir BASE]
[--timeout S]` (`status.rs`) lists every key in the runtime directory: a daemon that answers the
control exchange (`control.rs`) with its status; one that does not (its connect error or why it
gave no reply), through its locks (the owner lock only under the start lock, each for a moment,
`under_locks`); what a dead daemon left, with its build marker's restart time (the marker is read
without waiting, a regular file only, 256 bytes at most: `KeyDir::read_build_marker`, which the
shim uses too); keys with nothing but lock files and logs in one count line; a key that cannot be
looked at on a line of its own, with exit 1 (only a missing file is absent: `present`). It never removes anything. `prism-mcp daemon-shutdown --repo X [--cache-dir D] [--refresh-policy P]
[--runtime-dir BASE] [--pid N --start-time T] [--timeout S]` (`shutdown.rs`) asks the instance a
status exchange has just shown (or the one named) to drain, and when it had no client waits for
its pid file to go; a socket that refuses connections is no daemon only when nobody holds the
owner lock; exit 0 when that instance no longer serves the key, or has accepted and drains after
its clients; exit 1 when it could not ask, the reply to its request was lost (the request may
have been recorded), or it did not see the drain. `--runtime-dir` is the
base, as `PRISM_MCP_RUNTIME_DIR` gives it.

Opting in and the soak: user documentation of shared mode is `docs/MCP.md` ("Shared daemon"). The
plugin's launcher (`scripts/prism-mcp-launch.sh`) adds `--shared` when `PRISM_MCP_SHARED=1` (unset,
empty or `0`: nothing; any other value: exit 2; an explicit `--shared`/`--standalone` wins); its tests
are `crates/prism-mcp/tests/mcp/launcher_test.rs`. The soak instrumentation is `scripts/soak/` (Python 3.9, standard
library; CI runs its hermetic tests in the build-checks job): `fanout.py` (N clients of one corpus
in shared or standalone mode, per-call latency, a pressure observer that stops the run at a
critical level, one required `refresh_index` and its overlap with other clients' calls, `compare`
under the pinned rule; also the spec §6.2 fan-out gate), `soak.py` (`sample` appends the prism-mcp
processes, whether each footprint was measured, and the host's swap, pressure and compressor;
`probe` checks that a launcher's command line starts a server; `--window-file` keeps a window's
start across resumes), `soak_judge.py` (`soak.py report --phase round|week`: each rule PASS, FAIL
or UNKNOWN when its evidence was not collected, a phase INADMISSIBLE without observed use: a usage
day is a day daemons spent CPU, and a round names its checkouts with `--round-repo` and needs a
reached daemon in every client log) and `prism-mcp-logged.sh` (logs a client's prism-mcp standard
error with time and pid). Run their tests with `python3 -m unittest discover -s scripts/soak -p
'test_*.py'`; `PRISM_SOAK_BIN=<prism-mcp>` adds a live run against the binary.

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
2. **Graph-based** (require the CPG; source of truth `SlicingAlgorithm::needs_cpg()`, src/slice.rs:210): `left_flow`, `full_flow`, `relevant_slice`, `conditioned_slice`, `barrier_slice`, `chop`, `taint`, `delta_slice`, `spiral_slice`, `circular_slice`, `vertical_slice`, `threed_slice`, `gradient_slice`, `provenance_slice`, `membrane_slice`, `echo_slice`

### Layering and file layout (#701)

The workspace is three crates: `prism` (the core library, root `src/`), `crates/prism-review` (the review
layer, library `prism_review` plus the `prism` binary) and `crates/prism-mcp`; `default-members` are the root
and `prism-review` (`prism-mcp` is opt-in with `-p prism-mcp` or `--workspace`). The layering boundary is
Cargo: the `prism` library cannot name `prism_review` (no dependency) or `algorithms` (the module is not there);
its `#[cfg(test)]` code could name `prism_review` through the root's dev-dependency, which is what
`tests/integration/layering_test.rs` closes: it fails on any non-comment line
under `src/` that contains `prism_review`, test modules and `*_tests.rs` files included (the dev-dependency is
for integration tests only and must not leak into core source), and it
scans `crates/prism-mcp/src/` for `crate::algorithms`, `algorithms::`, `prism::algorithms` and
`prism_review`, exempting only its test files by name (`*_tests.rs`, `*_test.rs`, `tests.rs`, any
`/tests/` component); `//` comment lines are skipped, and there is deliberately no `#[cfg(test)]`
exemption. Large files are split by
generated move-only PRs with byte-identity checks: `src/ast.rs` is the entry (types, `mod` lines,
re-exports, a few core methods) over `src/ast/{js_ts,go,python}/` and concern files
(`imports`, `parameters`, `assignments`, `lvalues`, `rvalues`, `calls`, `control_flow`, ...);
`crates/prism-review/src/algorithms/taint.rs` likewise over `taint/`; `src/call_graph/*_tests.rs` hold the former
inline test modules. Language-specific code stays per layer (`<layer>/<lang>/`), never in a
per-language crate. The former root language modules now live in their layers:
`resolution/{go,js_ts,rust}/`, `call_graph/{go,js_ts,rust}/`, `type_providers/go/type_alias.rs`,
`languages/go/statements.rs` (`ast` imports `languages`, so the Go statement view lives in the lower
layer), `ast/js_ts/{exports,returns}.rs`, `frameworks/js_ts/react_hooks.rs` and `data_flow/terraform.rs`.
A layer's own files live in that layer's directory (`src/ast/`, `src/call_graph/`, `src/anon_owner/`,
`src/repo_loader/`, `src/resolution/`), not as layer-prefixed files in the `src/` root; a test file keeps a
`*_tests.rs` (or `tests.rs`) name.

### Algorithm Dispatch

`crates/prism-review/src/algorithms/mod.rs` contains:
- `run_slicing(ctx: &CpgContext, diff: &DiffInput, config: &SliceConfig)` — main dispatcher
- `run_slicing_compat(...)` — backward-compatible wrapper that builds `CpgContext` automatically
- `check_parse_warnings(...)` — reports tree-sitter parse errors (>10% warn, >30% skip)

## Key Design Decisions

1. **Tree-sitter for multi-language AST parsing.** The original paper used
   cppcheck (C++ only). We use tree-sitter to support 11 languages.

2. **Name-based variable tracking** instead of cppcheck's `varId` system.

3. **BTreeMap/BTreeSet everywhere** for deterministic, sorted output.

4. **Shared infrastructure:** `call_graph.rs`, `data_flow.rs`, `cfg.rs`, and
   `cpg.rs` are reused across multiple algorithms. Build them once via
   `CpgContext`, pass to algorithms.

5. **Algorithm-specific configs** live in each algorithm's module (e.g.,
   `BarrierConfig`, `TaintConfig`, `SpiralConfig`), not in the central
   `SliceConfig`, to keep the core config lean.

6. **Field-sensitive analysis** via `access_path.rs` tracks structured paths
   (e.g., `self.config.timeout`) rather than just variable names.

7. **File size: weighted lines, a 2,000 ceiling.** Size is measured in weighted
   lines — code 1, comment-only 0.5, blank 0 — by `python3 scripts/file_size.py FILE...`
   (`--over N` lists only files above N). New files aim for 600 or fewer. Over 1,000 a
   file should be split along a real responsibility boundary, and a PR that leaves a
   file over 1,000 says why. **2,000 is a hard ceiling: no PR may take a file past it.**
   Files already over 2,000 (7 today, e.g. `src/call_graph.rs`, `src/resolution.rs`,
   `src/type_providers/go.rs`) must not grow in a feature PR: small edits are fine, new
   code goes in new modules. Such a file is split in its own move-only PR (no behaviour
   change, reviewed as a pure move) before the work that needs it, never as a side effect
   of a feature PR. For tests, group by category (e.g., `algo_test.rs`,
   `advanced_test.rs`, `lang_test.rs`) and register **one umbrella `[[test]]` target per
   `tests/` subdirectory** (its `main.rs` declares the files as modules).

## Supported Languages

11 languages (12 tree-sitter grammar variants — TSX is parsed separately from
TypeScript):
Python, JavaScript, TypeScript, Go, Java, C, C++, Rust, Lua, Terraform/HCL, Bash.

## CLI Usage

```bash
# Single algorithm
cargo run -- --repo /path/to/repo --diff diff.patch --algorithm leftflow

# Multiple algorithms
cargo run -- --repo /path/to/repo --diff diff.patch --algorithm "leftflow,fullflow,taint"

# Preset suites
cargo run -- --repo /path/to/repo --diff diff.patch --algorithm review  # review suite
cargo run -- --repo /path/to/repo --diff diff.patch --algorithm all     # all 30

# Output formats: text (default), json, paper, review, callers, mermaid, sarif
cargo run -- --repo /path/to/repo --diff diff.patch --format json

# Navigation cache controls: these gate the nav store, not review CPG caching
cargo run -- nav --no-cache callers --repo /path/to/repo --symbol run
cargo run -- nav --cache-dir /tmp/prism-nav callers --repo /path/to/repo --symbol run
cargo run -- nav module-deps --repo /path/to/repo --file src/main.rs --format json
cargo run -- nav repo-map --repo /path/to/repo --format json
cargo run -- nav dfg-stats --repo /path/to/repo
cargo run -- nav dfg-stats --repo /path/to/repo --edges
```

Key algorithm-specific flags:
- `--barrier-depth`, `--barrier-symbols` (BarrierSlice)
- `--chop-source`, `--chop-sink` (Chop)
- `--taint-source` (Taint, repeatable)
- `--condition` (ConditionedSlice)
- `--old-repo` (DeltaSlice)
- `--spiral-max-ring` (SpiralSlice)
- `--quantum-var` (QuantumSlice)
- `--peer-pattern` (HorizontalSlice)
- `--layers` (VerticalSlice)
- `--concern` (AngleSlice)
- `--temporal-days` (ThreeDSlice, ResonanceSlice)
- `--compile-commands` (C/C++ type enrichment)

## Adding a New Language

1. Add the tree-sitter grammar crate to `Cargo.toml`
2. Add a variant to `Language` enum in `src/languages/mod.rs`
3. Implement all the node type methods for the new language
4. Add a fixture generator in `tests/common/mod.rs`
5. Add language-specific tests in `tests/lang/` and add a `mod <stem>;` line to
   that directory's `main.rs`
6. Add the new test file paths to the `all_test_files` arrays in
   `tests/integration/coverage_test.rs` (there are 3 copies — one each in
   `test_algorithm_language_matrix`, `test_language_coverage_minimum`, and
   `test_coverage_matrix_validation`)

## Adding a New Slicing Algorithm

1. Create `crates/prism-review/src/algorithms/your_algo.rs` with a `pub fn slice(...)` function
2. Add variant to `SlicingAlgorithm` in `src/slice.rs` (both enum and `from_str`/`name`/`all`)
3. Add `pub mod your_algo;` in `crates/prism-review/src/algorithms/mod.rs`
4. Wire it up in the `run_slicing` dispatcher in `crates/prism-review/src/algorithms/mod.rs`
5. Add CLI flags in `crates/prism-review/src/cli.rs` (the clap grammar) and `crates/prism-review/src/main.rs` if it needs algorithm-specific config
6. Add tests in `tests/algo/` (appropriate subcategory) and add a `mod <stem>;`
   line to that directory's `main.rs`
7. Add the algorithm to the `algorithms` list in `test_algorithm_language_matrix`
   and `test_language_coverage_minimum` in `tests/integration/coverage_test.rs`

## Common Patterns

- **Line numbers are 1-indexed** throughout. Tree-sitter uses 0-indexed rows;
  conversion happens in `ast.rs`.
- **`DiffBlock.file_line_map`** maps `filename → (line_number → is_diff_line)`.
- **Cross-file references**: Many algorithms include lines from multiple files.
  These appear as additional entries in `file_line_map`.
- **Algorithms that need call graph or data flow** receive them via `CpgContext`.
  The graph is built once and shared across algorithm invocations.
- **Finding severity has four values**: `info`, `suggestion`, `warning`, `concern`
  (`SliceFinding.severity`, `src/slice.rs`; ranked in that order by
  `output::severity_rank`). `--format review`'s default floor is `warning`;
  SARIF maps unknown severities to `error` (louder, never quieter); targets
  maps unknown severities to `concern` and records a warning.

## Dependencies

- `tree-sitter` + 11 language grammar crates (12 parsed language variants) for AST parsing; TypeScript 0.23.2 and Go 0.25.0 + PR #193 are source-vendored (see each `vendor/tree-sitter-*/PRISM.md`)
- `tikv-jemallocator` (Linux only) is the `prism` CLI's global allocator (`crates/prism-review/src/main.rs`; #437); `prism-mcp` stays on the system allocator because its sampler reads glibc `mallinfo2`
- `petgraph` for graph data structures (CFG, CPG)
- `clap` for CLI
- `dirs` for prism-owned cache directory discovery
- rayon for parallel file parsing/extraction in the CPG build
- `serde`/`serde_json` for serialization
- `anyhow`/`thiserror` for error handling
- `build.rs` emits `GRAMMAR_FINGERPRINT` from `Cargo.lock` tree-sitter grammar versions and all vendored grammar inputs for cache invalidation
- `tempfile`, `assert_cmd`, `predicates` (dev-dependencies for testing)
