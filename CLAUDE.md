# CLAUDE.md — Instructions for AI Assistants Working on This Codebase

## Project Overview

Rust implementation of 30 code slicing algorithms for defect-focused automated
code review. Based on arXiv:2505.17928 plus the established program slicing
taxonomy and novel theoretical extensions.

## Build & Test

```bash
cargo build          # Build the project
cargo test           # Run all tests (unit + integration)
cargo fmt --check    # Check formatting (must pass before PR)
cargo run -- --help  # Show CLI usage
cargo run -- --list-algorithms  # List all 30 algorithms
cargo build --bin prism-mcp --features mcp  # Build the MCP stdio server
cargo test --features mcp                   # Run tests with MCP enabled
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
targets under default and MCP features. The accepted migration oracle has been retired;
`mutants/dfg-compaction.json` retains its normal-target mutation obligations.

## Accuracy Harness (Tier-A)

When a change touches call resolution, navigation queries, or CPG construction
(`src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`):

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
cargo fmt            # Fix formatting
cargo test           # All tests must pass
```

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

## Code Organization

### Core Modules (`src/`)

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
- `output/` — Output formatters: `mod.rs`, `navigation.rs`, `review.rs`, `review_compact.rs`, `mermaid.rs`, `sarif.rs` (SARIF 2.1 serializer), `sarif_rules.rs` (rule descriptions + pure mapping helpers), `sarif_model.rs` (serde model structs).
- `cli.rs` — The clap `Cli` derive tree (`Cli`, `ReviewArgs`, `Command`, `NavArgs`, `NavQuery`, `TargetsArgs`); `pub mod cli` in `lib.rs` so `tests/cli/readme_test.rs` can parse-only via `Cli::try_parse_from` without executing the binary.
- `main.rs` — CLI entry point: `main()`, `run_review`/`run_nav`/`run_targets`, and the format-arm dispatch; the clap grammar itself lives in `cli.rs`.
- `algorithms/` — All 30 slicing algorithms. Each is self-contained.
- `algorithms/taint_sinks/` — taint's flat sink matcher (#622 S1), a hybrid policy. JS/TS/TSX, Python and Go sinks are decided from the AST: the identifier must be the callee of a call (or the target of an assignment for DOM/location sinks) — `callee.rs` finds the call and reads receiver chains. For high-severity names (exec/spawn, query/execute/raw, system/popen/remove, send/write, eval/Function, Command, HTML, redirect) an UNKNOWN receiver keeps firing; only PROVEN non-sinks suppress: a regex literal/`new RegExp`/binding initialised with one, loggers, message channels and stdio, a collection or file-object binding (every binding form of the name counts toward one file-wide total: plain, annotated and tuple-unpacking assignment, `with .. as`, `for` targets including JS `for (x of ..)`, walrus and augmented assignment, JS destructuring and compound assignment; more than the one proving binding, or any rebinding of a parameter, proves nothing, so the receiver is unknown and the sink fires. A Python annotation counts by its base identifier only, generic arguments ignored, and a union only when every non-`None` alternative is a collection), a `def`/function declared in this file under the same name, zero-argument calls (`model.eval()`, `u.Query()`), request-typed Go receivers (SQL names only; framework-context `HTML` still fires, a 3-argument value call being tagged `go-html-render` and graded weak), imported Go packages other than the rule's (`cobra.Command`), a constant `Sprintf` format. Go container-named receivers (`cache`, `l`, `list`) suppress `Remove`/`Rename`-style file ops only without a declared parameter or `var` type, or with a declared container type (`*lru.Cache`, `*list.List`); a file-system type (`afero.Fs`, `*os.File`) fires, as does a local initialised by a file-package constructor (`afero.NewOsFs()`, `os.Open(..)`). Struct-field receivers (`s.cache`) are not typed and stay name-only. Python `query` fires when its SQL argument (positional, or a `query=`/`sql=`/`statement=` keyword value) is (after peeling parentheses) an f-string, a `%` operation, a `+` with a string/identifier operand, a `.format` call or a lowercase variable; attribute-only arithmetic (`User.a + User.b`) and `session.query(Model)` stay ORM builders. Go `if`/`for`/`switch` init statements and `range` clauses are scopes. Receiver allowlists (response chains, db handles, `child_process` bindings, import-path-resolved Go packages) are positive evidence only: they pick the rule and CWE tag, never gate. Bindings are resolved lexically (`js_scope.rs`, `py_scope.rs`, `go_scope.rs`), so a shadowing parameter or local never inherits a module-level fact. Python `.where`/`.filter` with a positional argument stays a (weak) SQL clause sink by owner decision. Other languages keep the whole `SINK_PATTERNS` substring list (`legacy.rs`) except `SHELL_ONLY` command names (gated to Bash) and `get_user` (exact, C/C++); library names such as `sqlite3`, `chmod`, `chown`, `curl`, `uci`, `mtd`, `fw_setenv` are not gated. JS structured `exec`/`execSync` sinks carry a semantic check (`js_exec_call_is_command`). Sink strength (#636): each hit carries `Strength`, and `SinkClassifier::line_strength` (`taint_sinks/strength.rs`) grades the `taint_sink` finding at emission. Weak requires proof; anything unproven stays Strong. A line is Weak only when every rule-recognised sink call on it (and on the calls enclosing it, for lines inside a multi-line call) is weak, no caller-supplied or wrapper sink name is on it, and no non-SQL structured pattern (SSRF, path, command, ... `taint_sinks/structured.rs`, every pattern, no flow path) matches it; JS/TS/TSX, Python and Go only, other languages are always Strong. Weak shapes: SQL calls (`js-db-call` for the SQL-text-in-argument-0 names only, never `createQueryBuilder`; `py-db-call`; `go-db-call`) whose query argument is constant text (a string literal without interpolation, or a name with exactly one binding in its function, or in the file for a module-level name, never written: Go `const`, `query := ".."`, `var query = ".."` (a multi-name `var a, b = x, y` pairs each name with its own initializer and gives none when the counts differ); JS single declarator with parameters, shorthand destructuring bindings and `catch` parameters counted as declarations in the strength proof only (the shared resolver keeps main's presence behaviour) and no write, so a later `var` redeclaration, a parameter of the same name or `:=` redeclaration is Strong); Python SQL (design A, a closed allow-list in `taint_sinks/sqla_tables.rs`, the grader in `sqla.rs`, `sqla_value.rs` and `sqla_chain.rs`; the old raw-SQL deny-list is gone): `execute`/`query` weak only when its argument proves as a statement (S), and a `.where`/`.filter`/`.filter_by` hit (`py-sql-clause`, the tag is shared with the strong `text(<non-literal>)` hit; strength is per hit) weak only when its own arguments and the whole method chain around it prove (maximal chain: methods applied to its result and methods of its receiver are on the `CHAIN` table with proven arguments; the root may be built elsewhere, a parameter, an attribute or an unknown call, but a library statement or a local is proven and a local bound more than once is Strong; a root that calls a same-file `def` (one binding, undecorated) or a module-level lambda, or `self.m`/`cls.m` resolved through the class and its same-file bases (a method defined more than once, conditionally, or overridden in a same-file subclass is Strong), proves only when every `return` passes the same chain rule, to depth 2, and every call argument is a value or a proven chain; recursion, a bare return, no return or another assigned callee is Strong; `sqla_helpers.rs`). Every argument has a position class: C clause (becomes SQL structure: only column references, literals, comparisons, boolean and arithmetic over those, and calls to table constructs and column methods), V value (a bound parameter), F from, S statement, L literal, T type, O loader; anything not on a table is unproven. A raw constructor (`text(<non-literal>)`, `literal_column`, `column`, `table`, `Column(<non-literal>)`, `SQL`, `RawSQL`, `.extra`, `.op`, `prefix_with`/`suffix_with`, `with_hint`/`with_statement_hint`/`from_statement`, `getattr(func, x)`, a `*list` or `**dict` splat in a C position) is never admitted, and a belt of raw-API names refuses a callee of that name whatever its receiver (`db.literal_column(x)`); the same belt is applied to the whole line (`SinkClassifier::py_line_has_raw`, strength only). Library identity: a name is SQLAlchemy/peewee/Django only when EVERY binding of that spelling in the file (`py_scope::Sites`, a census over assignment, annotated and augmented targets, unpacking, `for` and comprehension targets, `with`/`except .. as`, walrus, imports, `def`/`class`, parameters of functions and lambdas, `global`/`nonlocal`, match captures, `del`, type aliases) is an import of the same member of `sqlalchemy`(.sub) / `peewee` / `django.db.models`(.sub), an exact package boundary (`sqlalchemy_utils` is not `sqlalchemy`); the original member name selects the table entry, so an alias cannot hide `literal_column`. A local name resolves to exactly one binding in the nearest enclosing scope (function, lambda, comprehension, module; class bodies only for code directly in them), at most 8 hops deep (D3), else Strong (a proof that held is memoised by initializer, so a name used many times is proven once; the grader also has a fixed step budget, beyond which the answer is Strong); a name bound more than once, a loop target in a C/S/F position, an unannotated parameter in a C/S/F position and `self.x`/`cls.x` roots are Strong. D2: a never-rebound parameter annotated `Table`/`Select`/`Subquery`/`Alias`/`CTE`/`FromClause`/`TableClause`/`Join`/`type[..]` is a schema root, and one annotated `Select`/`Insert`/`Update`/`Delete`/`CompoundSelect` is a statement (`Executable` is not proof). A schema root is also an imported or star-imported name that no scope rebinds, a class, a module-level object, or a local bound once to a proven S/F. Django sub-grammar (D4): `Q(**V)`, `~Q`, `Q & Q`, `Q | Q`, `Exists(S)`, `Subquery(S)`, `OuterRef(L)`, `F(L)`, `Value(V)`, `exclude`/`using`/`values_list`; nothing else of `django.db.models` is admitted. Contract (D1): a value position binds the Python object it receives, so a SQL construct that reaches a V position from outside the function (a parameter, the return of a cross-module function, an attribute set elsewhere, a module imported from elsewhere) is out of model, as runtime mutation is for `Exact`; everything the function or file builds itself is checked: a local in a V position resolves to its initializer (a loop or `with` target is data, except that an iterable built in place, a display or comprehension, directly or through one local name that no mutator fills with a non-value, is still checked, D1b), a same-file function (or a `self.m`/`cls.m` method) is proven by its returns (two levels), an object the function stores to (`o.x = ..`, `o[k] = ..`, `setattr`, or a mutator method such as `append`/`update` with a non-value argument) is Strong, and a local container (a display, a comprehension, or a container-constructor or same-file-function call) is data only when every other occurrence of its name is an allow-listed one (`name.append|extend|add|insert|update|setdefault|appendleft|extendleft(values)`, the iterable of a `for`, an argument of `len`/`list`/`tuple`/`set`/`sorted`/`str`, a read-only truth test or `in` right operand; adders count only as whole statements and a container holding a nested display is Strong; `sqla_uses.rs`), any alias, other call, store, `del` or nested-scope occurrence being Strong, a `self`/`cls` attribute is checked against every assignment to it in the enclosing class (tuple targets, `setattr`, mutators on it) and a same-named property (D1a), a belt method on an opaque chain root is Strong on any line, a maximal-chain root bound by a conditional or boolean expression checks every branch while unpacking and walrus roots are Strong, an unknown call is admitted as data only when its callee and every argument are values (an import of a belt name or of a SQL-library member, or a module-qualified call rooted at one, is never data, even when file-wide identity trust failed), a listed column method needs its argument classes whatever the receiver, and an f-string in a V position is weak (`.ilike(f"%{q}%")`) while the same f-string in a C position is Strong; `tests/algo/taxonomy/taint_sink_strength_sqla_test.rs` pins the contract rows as Weak. Go's constant-query proof counts every tree-sitter-go binding or write node of the name (assignment, short var decl, var spec, `++`/`--`, `range`, `case q = <-ch` / `case q := <-ch`, a type-switch alias) and `&name`; a constant needs zero such bindings in the function, a variable exactly one; and reads a separate per-name initializer, so rule presence keeps the historical first-value pairing. Go `go-html-render` (3 arguments on a value receiver, weak only when the receiver's declared type is exactly `gin.Context` or `*gin.Context`, a package-qualified identity and not a substring; a status argument is no proof, so `plugin.Context`, an unknown `Ctx` and iris `HTML(format, a, b)` are Strong). A weak finding has severity `suggestion` instead of `warning`, so the existing mechanisms apply: `--format review` hides it under the default `warning` floor and `--review-min-severity suggestion` shows it, SARIF level is `note`, `--format json`/targets keep it (`targets[].id` hashes severity, so weak taint_sink target ids change once on upgrade). The tier is a view: the `taint_sink` set is unchanged. `--format review` adds `findings_below_floor` (per result and top level) counting hidden findings of severity `suggestion` or above, omitted when zero (`info` is never counted). Parameterised JS/Python `query`/`execute` calls whose taint rides only in bound parameters are already cleansed by the structured sanitizers and produce no finding, so they have no weak row. JS/TS/TSX also have import-resolved module identity (`js_modules.rs`, #637 A1): a lexically resolved import or `require` of `fs`/`fs/promises`/`fs-extra`/`graceful-fs` (`node:` forms, `.promises`, named and renamed imports) fires `js-fs` on the path-taking API, a single-binding `new NodeSSH()` local from `node-ssh` fires `execCommand`/`exec`, and `document.execCommand` on the unshadowed global document fires for `'insertHTML'` or a non-literal command; identity needs the innermost scope binding the name (`js_binding.rs`: params, loops, catch, hoisted `var`) to hold exactly one unwritten binding, type-only imports, a class expression's own name and `with` bodies prove nothing, and an fs hit needs a non-literal path argument; tables are positive evidence only, closed, with fields out of scope. Python module identity (#637 A2, `py_modules.rs`): closed per-module tables (paramiko `exec_command`, pexpect `spawn`/`run`/`PopenSpawn`, fabric/invoke `run`/`sudo`/`local`, jinja2 `from_string`; `jinja2.sandbox.SandboxedEnvironment` is weak) fire strong only when the callee is proven by an import visible at the use (every binding in its resolved scope is that import, no star import can replace it), or one hop from it (a single-binding local, a `with .. as` target, the first parameter of a fabric 2/invoke `@task`; a receiver that is itself a listed-class call counts; `from_string` needs a non-literal template); fields, annotations and rebinding prove nothing, and the tables never suppress. New sink shapes go in a rule table with matrix rows in `tests/algo/taxonomy/taint_sink_rules_*`, never as a bare name in `SINK_PATTERNS`; mutants in `mutants/taint-sink-rules.json`.
- `reasoning/` — Tier-2 reasoning layer: `taint_trace` consumer, `SeedSet`, output shaper.
- `api/` — `prism::api`: the stable, `#[non_exhaustive]` facade for embedding prism as a library (`build_info`, `review`, `run`, `nav`). See its module doc for the compatibility promise; `main.rs` is its first consumer.
- `targets/` — `prism targets`: projects findings into the instrumentation-targets contract (`mod.rs`: `project`/`TargetsMeta`/id/dedupe/warnings; `model.rs`: schema DTOs; `mapping.rs`: category table, hint parsers, language lowering). Schema at `docs/contracts/targets.schema.json`.
- `finding_confidence.rs` — Single source for per-finding confidence/tier/parse-quality classification (`classify`, `evidence_files`, `ParseQuality::min_over`), consumed by both `output/sarif.rs` and `targets/`.

### Test Structure (`tests/`)

```
tests/
├── algo/              # Algorithm-specific tests
│   ├── novel/         # 8 test files (absence, echo, membrane, provenance + lang variants)
│   ├── paper/         # 1 test file (paper algorithms 6-9)
│   ├── taxonomy/      # 5 test files (taint CVE/lang/sink, misc, misc_lang)
│   └── theoretical/   # 4 test files (angle, quantum, spiral, vertical)
├── ast/               # AST infrastructure tests (access_path, binding, cpg, dfg, field)
├── cli/               # CLI tests (algo, output, validation)
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
`cargo test --test algo_paper`).
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
- `taint.rs` → Forward taint propagation (uses `data_flow.rs`)
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
rendering in `src/output/onboarding.rs`. `onboard --out` is explicit create-new-only
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
`src/resolution_free.rs`; {C, C++}, {JS, TS, TSX}, every other language its own family) is
removal-only: the rung, kind and confidence come from the unfiltered pool, then targets in another
family than the caller are dropped, a pool left empty refuses with no later rung consulted, and a
filter never promotes (a multi-owner NameOnly pool left with one target stays NameOnly with the
same kind). Free-function pools filter inside their rungs; method rungs (R1, R3b, R4b, self and
receiver-variable owner lookups, recovered receivers, R6) share one admission on the public full
resolution (`resolve_call_site_full`). `resolve_call_site_candidates` is the unfiltered view.

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
The fallback cap counts U/D survivors, including duplicates. Per the owner decision, an owner
formerly capped before pruning can regain its surviving edges when pruning brings it within the cap; this includes a function just over 1,000,000 candidates that falls under it once Go blank candidates are gone and then emits its real edges (the same rule as #634: the cap counts survivors, owner-accepted).
The additive `dfg_pruned_write_only`, `dfg_pruned_scope`, `dfg_pruned_dominated`, and
`dfg_pruned_order` counters persist through cache hits and appear in both stats commands;
scope and order remain zero until their later slices ship. Prune counters count skipped
producer candidates before edge compaction and deduplication, counted once from the
first pass even when the owner is capped; replay does not count them again.

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
`module.exports = { f }`, `{ f: g }`, `module.exports.f = f`, `exports.f = f` (or a function expression member, named `g` with any `f`: #627), `module.exports = function g() {}` and `{ f: function g() {} }` (a named function expression written in place; the target is the indexed callable with the expression's own bytes, never a same-spelled declaration; anonymous, arrow, parenthesized and generator values, a duplicate or mutated export object, and any producer with an ESM export refuse these new forms; ESM importers of a CJS producer gain these facts too, as they have for declared exports),
and `f(..)` for `const|let|var f = require('./x').f`. The proof is `JsExportFacts::cjs_callables`/`cjs_module_callable`
(`src/ast/js_cjs_terminal.rs`), recorded only behind the export-object barrier and the terminal proof, in a file with no
ESM export; the route is `src/resolution_js_cjs.rs`, the binder `src/ast/js_cjs_module_calls.rs`. The declaration must be
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
An anonymous or module-level caller (#597) is admitted by its own route (`src/js_anonymous/cjs.rs`, evidence
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

CPG cache (currently v146) persists DataFlow labels and per-file RD statistics across cold, full-hit, and
partial-hit builds. The B8 rule that capture reads in deferred or nested callables become
`NameOnly(CfgIncomplete)` is **PROVISIONAL** pending callable-timing work.

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
`arguments` are not handled. See `docs/features/language-coverage/js-ts-known-gaps.md` §6.

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

`prism-mcp` is a local stdio MCP server behind the cargo `mcp` feature:

```bash
cargo run --bin prism-mcp --features mcp -- --repo /path/to/repo
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

Build, test, or lint MCP code with `--features mcp`; the default build keeps
the adapter disabled.

`Evidence` also has an additive optional `reasoning` field. It is omitted when
absent so existing navigation and diff-review output remains byte-compatible.

Transport layout: `src/mcp/framing.rs` holds the newline-delimited JSON framing as one generic
`LineTransport<R, W>`; stdio and the Unix-socket `UnixStreamTransport` are instantiations of it.
`src/mcp/transport.rs` splits a message into `handle_control` (lifecycle, `tools/list`, call
validation; needs no index) and `dispatch_tool_read` / `dispatch_tool_refresh` /
`dispatch_tool_auto_refresh` (shared versus exclusive access to the provider). Output settings are
an explicit `ConnectionSettings` (`src/mcp/settings.rs`), resolved from the environment once per
server, so an invalid `PRISM_MCP_*` output value warns once, at server start, not once per call.
`tests/fixtures/mcp/golden/` pins the in-process server's stdout; re-record with
`PRISM_MCP_BLESS_GOLDEN=1 cargo test --features mcp --test mcp golden_transcript_test` only for a
change that is meant to alter tool output, and say why in the commit.

Shared-mode building blocks (`src/mcp/shared/`, Unix only; the daemon and the shim that use them
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
(`src/mcp/settings.rs`), whose constructor refuses a result cap below the floor and a first-call
wait above 600 s. A change to what goes into the key or to the preamble's wire text bumps
`SHARED_PROTOCOL_VERSION`.

Shared daemon (`src/mcp/daemon/`, Unix only; `prism-mcp daemon --repo X [--cache-dir D]
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

Shim (`src/mcp/shim/`, Unix only; `prism-mcp --repo X --shared [--daemon-start-timeout S]
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

The operator's commands (`src/mcp/shim/`, clients of the same sockets and files; spec §4.7):
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
are `tests/mcp/launcher_test.rs`. The soak instrumentation is `scripts/soak/` (Python 3.9, standard
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

### Algorithm Dispatch

`src/algorithms/mod.rs` contains:
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
   Files already over 2,000 (13 today, e.g. `src/ast.rs`, `src/call_graph.rs`,
   `src/algorithms/taint.rs`) must not grow in a feature PR: small edits are fine, new
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

1. Create `src/algorithms/your_algo.rs` with a `pub fn slice(...)` function
2. Add variant to `SlicingAlgorithm` in `src/slice.rs` (both enum and `from_str`/`name`/`all`)
3. Add `pub mod your_algo;` in `src/algorithms/mod.rs`
4. Wire it up in the `run_slicing` dispatcher in `src/algorithms/mod.rs`
5. Add CLI flags in `src/main.rs` if it needs algorithm-specific config
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
- `petgraph` for graph data structures (CFG, CPG)
- `clap` for CLI
- `dirs` for prism-owned cache directory discovery
- rayon for parallel file parsing/extraction in the CPG build
- `serde`/`serde_json` for serialization
- `anyhow`/`thiserror` for error handling
- `build.rs` emits `GRAMMAR_FINGERPRINT` from `Cargo.lock` tree-sitter grammar versions and all vendored grammar inputs for cache invalidation
- `tempfile`, `assert_cmd`, `predicates` (dev-dependencies for testing)
