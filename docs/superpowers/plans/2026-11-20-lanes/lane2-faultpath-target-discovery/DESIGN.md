# Lane 2: faultpath target discovery — design

Status: Stage D design of record, 2026-11-24. The 631 row decisions are in `row-verdicts.jsonl`.
Stage D awaits both design reviewers. Tracking issue: #613. Research record:
`~/prism-evidence/lanes-2026-11-20/research/lane2-faultpath-targets.md`.

## 1. Intent

`prism targets` should list the places where a program calls an external dependency, whether or not any
review heuristic has a finding there, so that faultpath can inject faults at sites prism discovered instead
of sites someone typed into an overlay. The first population is bounded: Python, four call families (HTTP,
filesystem, parsing, configuration). The lane ends with one demonstrated pipeline on a real project: site
discovered by prism → resolved live by faultpath → reached by the project's tests → fault injected → verdict.

## 2. What exists today (verified on `origin/main` at 1179055f; faultpath at 8e5070a)

**Prism.**

- Every targets row comes from a finding. `kind: external_call` has exactly one source, the echo algorithm's
  `missing_error_handling` category (`crates/prism-review/src/targets/mapping.rs:193`), and echo only looks
  at in-repository callers of in-repository functions that the diff changed
  (`algorithms/echo_slice.rs:96-350`). A call to `requests.get` has no in-repository definition, so no echo
  finding can exist for it. Issue #613 is therefore a missing row source, not a classification bug.
- Probe (a 62-line file with 13 dependency calls, every line in the diff): 0 `external_call` rows; 61
  provenance rows of kind `other`.
- `targets/dependency_hint.rs` and `targets/dependency_identity.rs` already prove Python library identity
  for a call: the callee chain's root must be bound at the site by a visible import
  (`python_imported_module`, `dependency_identity.rs:122`), not shadowed locally, not a repository-local
  module, and in a library table (`PYTHON_LIBRARIES`, `:536`: requests, httpx, urllib3, aiohttp,
  urllib.request as `http`; pathlib and shutil as `filesystem`; and others). One receiver hop exists
  (`s = requests.Session(); s.send(..)`). Today this code only decorates rows echo already produced. Its
  shadowing checks recognise a fixed list of binding forms (plain assignment and a few patterns); a form not
  on the list counts as "no binding".
- Function bounds of existing rows come from the tree-sitter function node (`ParsedFile::
  function_node_spanning`, `node_line_range`). That node absorbs comment lines indented under the body, so
  its end line can lie past the function's last statement (probe: four of five test functions ended one to
  two lines late), and its start line is the first decorator line of a decorated function.
- `prism targets` requires `--diff`, and only the files the diff names are loaded
  (`load_review_inputs`, `src/api/review.rs:69`); a repository-wide run needs a synthetic diff that lists
  every line. `run_targets` always goes through the review runner, which builds the code property graph.
  The command writes schema 1.0 only; documents are checked against the schema file by tests, not at run
  time. Prism's copy of the 1.0 schema differs from the contract authority's in description text only (a
  known drift, not changed by this lane). A row id is the SHA-256 of nine fields: file, line, symbol,
  source algorithm, category, description, severity, related lines, related files (`targets/mod.rs:279`).
  `repo.sha` is `git rev-parse HEAD` of the repository when it is a git checkout (`main.rs` `repo_sha`).
- The whole-repository loader (`prism::repo_loader::load_repo`) skips directories named `.git`, `target`,
  `node_modules` and `vendor` at any depth, `build` and `dist` at the repository root, and every directory
  whose name starts with a dot. Files it cannot load are listed in `LoadedRepo::skipped`.

**Faultpath** (historical source review for this section). Stage D uses the recorded fixture oracle.

- A row is injectable only when `kind` is `external_call` or `resource_acquire`. Faultpath binds a row to
  one call by its line alone. Section 5.4 gives the exact line-anchor key and section 6.6 resolves equal keys.
  On `x = foo(a) + requests.get(u)` it binds `foo` irrespective of the inventory tables.
- Its site guard requires `site.function_start_line` and `function_end_line` to equal Python's `ast`
  `lineno` (the `def` line) and `end_lineno` of the innermost function whose body holds the call, and the
  whole call to lie within them. It accepts `site.symbol` only as the bare function name,
  `<innermost class>.<function>`, or that form prefixed with the module path; Python's `__qualname__`
  (`outer.<locals>.inner`, `Outer.Inner.m`) is refused (`stale_site_symbol`). A module-level call has no
  bounds and is rejected.
- Beyond those contract checks it has runtime limits of its own: a function with a decorator other than
  `staticmethod`/`classmethod`, a function whose name is defined twice in its scope, a call in a class body
  or a lambda, and comprehension bodies on Python 3.11 are refused as `unsupported_frame_guard`; a callee
  rooted in a call (`requests.Session().get(u)`) is a "dynamic callee"; nested functions get no probe;
  `with`-bound receivers are unresolved; an annotation-typed receiver needs a reached-call witness.
- It runs on CPython 3.11, or on 3.12 and 3.13 builds that match its qualification record; any other
  interpreter makes every resolver and guard call answer `unsupported_interpreter` or
  `unqualified_runtime`. The httpie pilot pins 3.11.11.
- **HTTP hints select faults.** Faultpath resolves the patch point from the source at `site.line`, and for
  an HTTP call `dependency_hint.callee` and `.kind` select which catalogued faults apply: entries whose
  patterns (`requests.*`, `httpx.*`, `urllib3.*`, `aiohttp.*`, `*.Session.*`, `*.get`, …) match the callee.
  With no hint an HTTP call gets no fault (`no-dependency-hint`). A wrong `kind` excludes the right faults,
  so an absent kind is better than a wrong one. A callee rooted in `urllib.request` matches no catalogued
  HTTP entry today.
- **Standard-library hints are identity assertions.** For a 1.1 document whose row has no hint, or a hint of
  kind `filesystem`, `parse`, `config` or `unknown`, the planner derives the call's canonical callee from
  the source (builtins spelled `builtins.<name>`) and looks it up in its own table of standard-library fault
  callees. It refuses the row with `dependency_identity_mismatch` (the row gets a probe and no fault) when
  the canonical callee is not in that table while the hint names one of those three kinds, when the hint's
  kind differs from the kind the table assigns, or when the hint's callee is not string-equal to the
  canonical callee. With no hint at all, a callee in the table gets its hint derived by the planner.
- `faultpath scan` already discovers standard-library filesystem, parsing and configuration call sites from
  the AST and writes targets schema 1.1 (`source_algorithm: faultpath.scan/1`, category
  `stdlib_fault_site.<kind>`). It does not cover HTTP or any third-party library. It skips every call that
  is not its line's anchor (`ambiguous_call_anchor`), every call in a file with a `from m import *`, and
  emits module-level calls as rows without bounds; each skip is a document warning of the form
  `<file>:<line>: <source callee>: <reason>`. It needs a clean tracked git repository and declared import
  roots. The recorded owner decision is "`faultpath scan` now, prism feeds the same contract later", with
  prism able to replace it "without any harness change".
- Schema 1.1 exists in the contract authority and in faultpath's vendored copy. It differs from 1.0 in three
  places: `schema_version` is `"1.1"`; `producer.tool` may be `prism` or `faultpath`; `dependency_hint.kind`
  adds `parse` and `config`. Faultpath's loader accepts exactly 1.0 or 1.1. Both schemas are closed: an
  unknown property fails validation. The 1.1 schema describes `site.symbol` as the name of the innermost
  function enclosing `line`.
- The httpie pilot (httpie at `2105caa49bae87c5809c274e407619a0de2639d1`) used an overlay with four rows:
  `httpie/client.py:114` (`requests_session.send`), `httpie/internal/update_warnings.py:44`
  (`requests.get`), `httpie/models.py:65` and `:68` (response body iterators). The appended rows carried no
  dependency hint; kind, exact bounds and symbol were what made them usable. Last measured result with the
  overlay (attempt 5, on an earlier faultpath revision and a pilot driver from a preparation branch): 14 of
  14 reachable keys reached, 12 caught, 2 not caught; 8 keys not reached because the suite mocks
  `requests.get` at `update_warnings.py:44`.
- On faultpath's current `main` the pilot census refuses a run without a reach document and a validated
  pilot configuration, keeps at most 25 targets, and fixes at most 3 faults per target; its only pilot
  driver starts from its own scan stage. The plain flow `faultpath plan` then `faultpath run` (with
  `--only-target`) needs none of these.
- Nothing in flight on faultpath touches discovery or the targets contract (call binding, custody, process
  containment, interpreter matrix).

## 3. Goals, non-goals, anti-goals

Goals:

1. A row source inside `prism targets` that is independent of findings: a **dependency-call inventory** for
   Python, HTTP family first, then the three standard-library families.
2. Rows faultpath accepts with no harness change: `kind: external_call`, exact function bounds, an accepted
   symbol, a hint whose `kind` is present only when library identity is proven, and, for the three
   standard-library families, the consumer's canonical callee and kind.
3. Every site the inventory recognises but cannot offer as injectable is listed with a reason, and every
   call it declines for unproven identity is counted per file and cause, so the output shows where coverage
   is lost.
4. Deterministic identity: the same source gives the same rows and ids on every run, machine, thread count,
   resolution mode and cache state, whatever other row sources ran.
5. The pipeline demonstrated on httpie without the overlay.

Non-goals: JavaScript, TypeScript and Go inventories; database, queue and process families; third-party
parsing libraries other than the three backports the consumer's standard-library table lists (`tomli`,
`importlib_metadata`, `defusedxml`), so no `yaml`; changing faultpath; a new contract version; fixing
provenance's noisy rows on `requests.*` lines (filed as a separate issue); fixing the end line of projected
finding rows (filed as a separate issue, section 10).

Anti-goals: rows whose `kind` rests on a name match alone (`x.get(..)` on an unknown receiver); output that
changes for existing users who do not ask for the inventory; a second, diverging copy of the library
identity proof; a line rule of prism's own that can drift from the consumer's.

## 4. Approaches considered

**A. A thirty-first slicing algorithm that emits a finding per dependency call**, mapped to `external_call`
like echo's. It fits the existing projection, but a finding per `open()` call would also appear in
`--format review`, JSON and SARIF, where it is noise; it would need rows in the algorithm-by-language
coverage matrix; and it keeps discovery tied to the diff. Rejected.

**B. A second row source inside the targets module**, merged with projected findings before dedupe and
sort. No effect on review output, reuses the row model, id rule, dedupe and schema validation, and reuses
the identity proof that already lives in the same module. **Chosen.**

**C. A core navigation query** (`prism nav dependency-calls`) that targets consumes. More reusable (lane 5
needs outbound HTTP call sites too) but it drags the identity proof and tables into the core crate and the
navigation `Evidence` model now. Not chosen; the inventory's core is written with no dependency on targets
types so it can be relocated by a move-only change if lane 5's pilot needs it.

**D. Leave discovery to faultpath: add `http` to `faultpath scan`.** Cheapest for HTTP alone, and remains
available to the owner. It keeps two discovery implementations, does not use prism's receiver typing and
scope-aware binding, and does not move toward the recorded decision that prism feeds the contract. Not
chosen; section 9, decision 2 records it as the fallback.

## 5. Architecture

```
parsed Python files ──► inventory::collect ──► InventoryOutput { sites, unsupported, unproven }
                                                      │
findings ──► project (unchanged) ──► rows ────────────┤ merge ► dedupe by id ► filters ► sort ► document
```

### 5.1 Row shape

An inventory site becomes an ordinary targets row:

| Field | Value |
|---|---|
| `kind` | `external_call` |
| `category` | `dependency_call.<family>`, family one of `http`, `filesystem`, `parse`, `config` |
| `source_algorithm` | `prism.inventory/1` |
| `expected` | `{ "property": "error_handled", "detail": "source callee: <callee text as written>" }` |
| `dependency_hint` | `{ "kind": "<family>", "callee": "<resolved dotted callee>" }`, for example `requests.Session.send` for `s.send(req)`; for the three standard-library families the resolved callee is the canonical spelling of section 5.3 (`builtins.open`, `pathlib.Path.read_text`) |
| `site` | `file`, `line` (first line of Python's callee expression), `symbol` (bare Python function name), `function_start_line`, `function_end_line`, `language: "python"`. Sections 5.4 and 6.6 define the source coordinates. |
| `confidence` | `exact` when identity is proven by imports and constructor bindings (root identity, routes R1 to R3); `nameonly` when a type annotation is part of the proof (routes R4, R5) |
| `tier` | `asserted` when the confidence is `exact` and the file's parse quality is `clean` (the schema's rule); `candidate` otherwise |
| `severity` | `info` |
| `description` | `<resolved dotted callee>: dependency call site` |
| `parse_quality` | the file's parse quality |

The tier rule is applied directly: `finding_confidence::classify` returns `unlabeled`/`candidate` for any
name that is not a slicing algorithm, so it is not called for inventory rows.

The hint carries the resolved callee because faultpath selects HTTP faults by matching the callee against
patterns such as `requests.*` and `*.Session.*` (`r.get` under `import requests as r` would match less), and
checks standard-library hints for string equality with its canonical callee. The text as written is kept in
`expected.detail`. This is the same convention `faultpath scan` uses.

The id uses the existing nine-field rule unchanged. With the fixed description and severity above, an
inventory row's id depends only on file, line, symbol, family and resolved callee.

### 5.2 Unsupported sites

**Principle.** A call whose callee is proven to belong to a table entry is an unsupported site only when no
consumer of the contract could bind a row to the right call or the right frame: the call has no enclosing
function, it runs in a lambda's or a class body's own frame, its line does not identify it (it is not its
line's anchor, or its callee is split over lines), or its function does not parse. Limits of one
consumer's runtime (decorators, comprehension frames on some interpreter versions, dynamic callees,
witnesses, nested-function probes, `with`-bound receivers) do not make a site unsupported: such sites stay
injectable rows, and the funnel (section 8) counts them under the consumer's own reason.

Such a call is emitted as a row with `kind: "other"`, `category: "dependency_call_unsupported"`,
`expected: { "property": "unknown", "detail": "<reason>" }` (or `"<reason>; <n> calls"` when n ≥ 2, below),
the same `dependency_hint` as a site row, `severity: info`,
`description: "<resolved dotted callee>: unsupported dependency call: <reason>"`, `site.file`, `site.line`
(first line of the callee), `site.symbol` (the bare name of the innermost `def` whose body contains the
call, absent when there is none) and no bounds. Faultpath drops kind `other`; a reader can count reasons.

**Grouping.** Unsupported calls are grouped by (file, line, symbol, resolved callee, reason); each group is
one row, and a group of n ≥ 2 calls carries `<reason>; <n> calls` in `expected.detail`. Because the reason
is in the description, two groups on one line differ in id; one injectable row and one unsupported row of
the same callee on one line differ in category.

Reasons in first-applicable order for a proven call. Section 6.6 defines the evaluation context and spans.

1. `parse_error_in_function`: the innermost `def` whose body contains the call has a parse error
   (tree-sitter `ERROR` or missing node) anywhere inside it.
2. `module_level`: no `def` body contains the call (module top level, a class body directly in the module,
   a decorator, default value or annotation of a top-level `def`).
3. `class_body_frame`: the innermost frame-introducing construct whose body contains the call is a class
   body (a class defined inside a function).
4. `lambda_frame`: that construct is a lambda.
5. `multiline_callee`: the callee expression does not fit on one line.
6. `not_line_anchor`: the call is not the line anchor of its callee's first line (section 5.4).

**Identity unproven.** Count one call only if an explicit tie to a table entry survives the candidate
test of section 6.5 and no route proves the identity. Untied calls give plain `none`. Try admissible
identities before diagnosing a failed spelling. An explicit write invalidating the chosen identity vetoes
that proof. Apply identity before the unsupported-site tests. For competing failures on a counted call
use the first applicable cause in this closed order.

| Cause | Evidence |
|---|---|
| `star_import` | A syntactic wildcard import anywhere in this file. |
| `shadowed_root` | A deciding binding hides a candidate library/builtin root, multiple bindings compete for that root, or a namespace write in 6.2 invalidates it. Receiver multiplicity has its own cause below. |
| `not_imported` | No binding occurrence for the candidate name is visible to lexical lookup, and builtin fallback does not apply. |
| `repository_local_module` | A relative import or the imported root's module/package in the repository. |
| `binding_unavailable` | The sole binding is type-checking-only at an executable use, or follows the use in the same local execution scope (section 6.2). |
| `member_rebound` | An explicit source write or delete invalidates a member on the proven imported path (section 6.2). |
| `receiver_not_unique` | More than one binding of the tied receiver name or more than one modelled write of the instance attribute. |
| `helper_unannotated` | A same-file helper tied by a returned constructor lacks an admissible return type. |
| `helper_not_admissible` | A tied helper fails the definition, visibility, decorator, generator or await conditions of section 6.4. |

The document carries one warning per file with counted calls:
`inventory: identity unproven: <file>: <n> (<cause>: <k>, …)`, causes in the table's order, zero counts
omitted. It also carries one warning per Python file the loader skipped,
`inventory: skipped: <file>: <loader reason>`, and one warning naming the excluded directory rules,
`inventory: excluded directories: .git, target, node_modules, vendor, venv, site-packages, dot directories,
build and dist at the root`.

### 5.3 Identity proof

Make the existing targets import-path and repository-local checks available at a call node.
Given the deciding import, `r.get` under `import requests as r` resolves to `requests.get`.
Inventory and echo use the same path construction. Inventory supplies the 6.2 binding census and
matches a **callee-level** table. Echo uses its existing admission policy and library-prefix table.

**Callee tables.**

- `http` uses the closed list of functions, constructors and per-class methods in section 6.1.
- `filesystem`, `parse`, `config`: exactly the consumer's standard-library fault-callee table, in its
  canonical spelling (the resolved callee, `dependency_hint.callee`) and with its kind (the family). Prism
  lists no other standard-library callee (a callee outside this table would be refused, not degraded) and
  every entry is listed. A change of the consumer's table is a change of this list, made in its own PR.
  - `filesystem`: `builtins.open`, `io.open`; `pathlib.Path.{open, read_text, read_bytes, stat, exists,
    is_file, is_dir, write_text, write_bytes, mkdir, touch, rename, replace, unlink, rmdir, symlink_to}`;
    `os.{stat, listdir, scandir, pathconf, mkdir, makedirs, remove, unlink, rename, replace, rmdir, write,
    fsync, symlink, link}`; `os.path.{getsize, getmtime}`; `shutil.{copy, copy2, copyfile, copytree,
    move}`; `tempfile.{mkstemp, mkdtemp, NamedTemporaryFile, TemporaryFile, TemporaryDirectory}`.
  - `parse`: `json.{load, loads}`; `tomllib.{load, loads}`; `tomli.{load, loads}`;
    `configparser.ConfigParser.{read, read_file, read_string}`;
    `configparser.RawConfigParser.{read, read_file, read_string}`; `xml.etree.ElementTree.{parse,
    fromstring}`; `xml.dom.minidom.{parse, parseString}`; `defusedxml.minidom.{parse, parseString}`;
    `pickle.{load, loads}`.
  - `config`: `os.environ.get`, `os.getenv`; `importlib.metadata.{version, metadata, distribution, files,
    entry_points}`; `importlib_metadata.{version, metadata, distribution, files, entry_points}`;
    `pathlib.Path.home`; `locale.{getpreferredencoding, getencoding}`.

  Builtins are spelled `builtins.<name>`, never bare. `Path.home()` is `config`, not `filesystem`, although
  prism's echo-decoration table maps `pathlib` to filesystem. `tempfile.gettempdir` is not in the table.

**Closed-world rule.** Resolve a name in its Python deciding scope. Require a single proving binding there,
or the bare builtin fallback. Count competing binding occurrences throughout that scope. Section 6.2
defines lookup, redirections and writes. Share import-path construction and repository-local checks with
the existing targets proof. The inventory uses its own deciding-scope census. Echo retains its existing
selection and admission policy.

**Receiver routes.** First try a proven root path in the table. For a method on an instance use the first
admissible route below. Verify the method in the chosen class's list. The constructor or annotation is
resolved where it is written. All five routes apply to the listed HTTP and standard-library classes.

| Route | Evidence | Grade |
|---|---|---|
| R1 | The receiver expression is the listed constructor call itself. | exact |
| R2 | A single name binding assigns the constructor result or takes the matching context-manager target (6.3). | exact |
| R3 | A single modelled store of the constructor result in the instance attribute of the owning class (6.3). | exact |
| R4 | A single parameter or local annotated assignment gives one admissible class (6.4). | nameonly |
| R5 | A single local assignment calls one visible same-file module helper with an admissible return annotation, with matching await form (6.4). | nameonly |

### 5.4 Site rules

- **Frame and enclosing function.** The enclosing function of a call is the innermost `def` or
  `async def` whose **body** contains the call. Decorator expressions, default values and evaluated
  annotations of a `def` belong to the scope around it; a class body and a lambda body are frames of their
  own (section 5.2, reasons 3 and 4); comprehensions and generator expressions do not stop the search (their
  sites belong to the enclosing `def`).
- **Symbol.** `site.symbol` is the bare name of the enclosing function (`m` for a method `C.m`, `inner` for
  a nested function). No qualified spelling is used anywhere.
- **Bounds.** `function_start_line` is the line of the definition's `def` keyword (of `async` for
  `async def`), never a decorator line: Python's `ast` `lineno`. `function_end_line` is the line on which
  the definition's last token ends, comments and blank lines excluded: Python's `ast` `end_lineno`. In
  tree-sitter terms, use the end of the last syntactic token, skipping trivia including continuations
  and comments (a multi-line string ends on its closing quote's row). Section 6.6 gives the delimiter
  and newline cases. The inventory computes both in
  `inventory/python.rs`; it does not use or change `function_node_spanning` or `node_line_range`.
- **Line anchor.** For a line L, the candidate calls are every call in the file whose callee expression's
  line span contains L. When an annotated assignment (`x: T = v`) belongs to a function's own scope,
  remove calls inside its annotation if any other kind of candidate exists. The
  **line anchor** minimises (absolute distance of callee end line from L, callee start byte column, negative
  call end byte column). The end column is compared independently of the call's end line. Equal keys use
  section 6.6's traversal order. On `foo(a) + requests.get(u)` the anchor is `foo`. On
  `requests.get(u).json()` it is the outer call. In a pair of same-line sibling calls the left one wins.
  On `requests.get(build(u))` it is `requests.get`.
- `line` is the first line of the semantic callee expression. It must fit on one line. Arguments can
  continue on later lines. Section 6.6 specifies grouping, newlines, byte columns and source text.
- Calls inside unevaluated annotations give plain `none`, including postponed annotations (section 6.6).

### 5.5 Command line and document

- `--inventory <name>` turns the row source on; the only name in this lane is `python-deps`.
  `--inventory-kinds http,filesystem,parse,config` narrows it (default: every family the build supports).
- **Which files.** The inventory reads the repository with the whole-repository loader the navigation
  commands use (`prism::repo_loader::load_repo`, skip rules in section 2), keeps the Python files, and also
  skips any path with a directory component named `venv` or `site-packages`. Test directories are included;
  a consumer filters them. `--files` (repository-relative paths, as today) narrows the set. The file set
  does not depend on the diff. Use the newline-normalised parse view of 6.6 for the inventory and
  the bounds example. Preserve the source bytes for the row's callee text.
- `--diff` stays required unless `--inventory` is given. An inventory-only run does not call the review
  runner and builds no code property graph. Its `diff` envelope follows the convention `faultpath scan`
  uses: `files` is the sorted list of inventoried files and `sha256` is the SHA-256 of that list's compact
  JSON encoding. `repo.sha` is filled exactly as for a projected document.
- With `--diff` and `--inventory` both given, findings are projected as today from the diff's files, and
  inventory rows for the whole repository are merged in before dedupe and sort.
- Other options on the inventory path:
  - `--algorithm`, `--scoped-cpg`, `--old-repo`, `--compile-commands`, `--cache-dir` and `--no-cache`
    configure the review runner. Given explicitly without `--diff` they are an error naming the option; with
    `--diff` they act on the projected findings as today.
  - `--min-confidence`, `--min-tier` and `--min-severity` filter inventory rows exactly as they filter
    projected rows: `--min-confidence exact` drops the `nameonly` rows of R4 and R5 in all four families,
    `--min-tier asserted` drops every `candidate` row, and a severity floor above `info` removes every
    inventory row, in which case the document carries the warning
    `inventory: <n> rows removed by --min-severity <floor>`.
  - `--resolution` does not change inventory rows; `producer.resolution_mode` records it as today.
  - `--strict` keeps its meaning (exit 3 when `errors` is non-empty); the inventory writes warnings, never
    errors.
- `--inventory-kinds` naming a family the build does not support is an error that lists the supported ones.
- `producer.algorithms` gains `prism.inventory/1` when the inventory ran.
- **Schema version.** A document produced with `--inventory` is written as schema 1.1. Tests check such
  documents against a vendored `docs/contracts/targets-1.1.schema.json`, which must be byte-identical to
  the contract authority's file. A document produced without `--inventory` stays 1.0, with output identical
  to today's apart from the fields every build changes (G-byte). No new contract version is created by this
  lane.

### 5.6 Code layout

```
crates/prism-review/src/targets/
  inventory/mod.rs          collect(), InventoryOptions, InventorySite, UnsupportedSite, family names
  inventory/python.rs       walk Python files: call nodes, frames, bounds, line anchor, site rules
  inventory/py_tables.rs    callee-level tables per family
  inventory/py_receiver.rs  targets-owned Python binding census (6.2) and routes R1 to R5 (6.3–6.4)
  inventory/rows.rs         InventorySite / UnsupportedSite → Target rows
  dependency_hint.rs        exposes call-node root identity, preserves finding-line resolution
  dependency_identity.rs    shared import paths and repository-local checks
  mod.rs                    inventory_document() and merge_inventory(); project() is unchanged
crates/prism-review/examples/inventory_bounds.rs   prints every function's bounds (gate G-bounds)
docs/contracts/targets-1.1.schema.json             vendored, byte-identical to the authority
```

`inventory/mod.rs`, `python.rs`, `py_tables.rs` and `py_receiver.rs` use no targets row types; only
`rows.rs` does.

## 6. Stage D amendment

This is the design of record for `tests/targets_inventory/rows.jsonl`. Join the sibling
`row-verdicts.jsonl` by `id`. Its `want` is the required result and its `rule` identifies the
deciding provision. It covers 279 HTTP root, 203 HTTP receiver and 149 standard-library rows.
Where a guard answer or absent qualification needs reconciliation, the rule field also cites 6.8
and identifies that row's observation. The preceding provision decides the want. Both design
reviewers must approve within two rounds. After two rejections use ORCHESTRATION section 8.

The matrix at `2a5667ee1c33` has SHA-256
`d05d3b36daf8c5f31daaa527ee582e6a8c8279feda5a3380294d04695356f71e`.
The supplied `oracle/site-oracle.jsonl` has SHA-256
`529fbcc28b90517347f5ca2500e51f821196209343074080d8212549f5fa378e`.

### 6.1 HTTP table and route retention

Each cell enumerates the permitted terminal names. Join the path and terminal with a dot to obtain
the hint. A class path identifies a constructor for the receiver proof. Calling the constructor alone
produces no HTTP row. A proven, explicitly class-qualified table member can also match by root identity.
There is no arity, literal-URL, await-at-method-use or fault-catalogue filter.
Every one of the 60 HTTP and 80 standard-library entries has a positive fixture verdict.

| Module or class path | Functions or methods |
|---|---|
| `requests` | `get, post, put, patch, delete, head, options, request` |
| `httpx` | `get, post, put, patch, delete, head, options, request, stream` |
| `urllib.request` | `urlopen, urlretrieve` |
| `urllib3` | `request` |
| `aiohttp` | `request` |
| `requests.Session` | `get, post, put, patch, delete, head, options, request, send` |
| `httpx.Client` | `get, post, put, patch, delete, head, options, request, send, stream` |
| `httpx.AsyncClient` | `get, post, put, patch, delete, head, options, request, send, stream` |
| `aiohttp.ClientSession` | `get, post, put, patch, delete, head, options, request` |
| `urllib3.PoolManager` | `request, urlopen` |

The lookup is exact after import resolution. For example, `requests.api.get`, `Session.stream`,
`urllib.parse` and response iterators have no entry. The `urllib.request.urlopen` census sites and
`urlretrieve` (tr2-12) remain discoverable even without a matching consumer fault. The other three
families use the canonical list in 5.3, including its three backports and the separate `Path.home` kind.

The stage M denominator includes every listed HTTP site, including unproven sites. Counts from
`census/CENSUS.md` give the following shares. The two R2 forms each account for one Black site.

| Tie | httpie (7) | Saleor (2) | Superset (25) | Black (2) |
|---|---:|---:|---:|---:|
| Root import or from-import | 6/7 | 2/2 | 12/25 | 0/2 |
| R1 direct constructor | 0/7 | 0/2 | 0/25 | 0/2 |
| R2 constructor name or with-target | 0/7 | 0/2 | 0/25 | 2/2 |
| R3 instance attribute | 0/7 | 0/2 | 0/25 | 0/2 |
| R4 annotation | 0/7 | 0/2 | 1/25 | 0/2 |
| R5 annotated helper | 1/7 | 0/2 | 0/25 | 0/2 |
| Other | 0/7 | 0/2 | 12/25 | 0/2 |

R4's 4% is a parameter in Superset. R5's 14.29% is the required `httpie/client.py:114` site.
Root identity covers the other overlay HTTP site at `update_warnings.py:44`. Only R1 and R3 as
whole routes qualify for the optional under-2%-in-every-corpus cut.

| Decision | Option and main cost | Choice |
|---|---|---|
| HTTP membership | Admit library prefixes or all verb names on every class. This would classify utilities, constructors or absent methods. | Declined |
| HTTP membership | Use only census hits. This drops tested APIs absent from these four projects. | Declined |
| HTTP membership | Use only patchable callees. This drops the measured urllib population. | Declined |
| HTTP membership | Maintain the explicit module and per-class lists above. Additions require an entry and a fixture. | Chosen |
| Routes | Drop R1. This removes h10 and introduces a family-specific exception for direct construction. | Declined |
| Routes | Drop R3. This removes h07 and the supported same-class store pattern. | Declined |
| Routes | Keep R1–R5. R1 also handles f01d/e and needs no name index. R3 reuses the target module's existing class-owner concept but requires an attribute-write census. | Chosen |
| Stdlib class routes (Q13) | Restrict to constructors. This loses tr3-24–26, sl1-65 and as1-27/28. | Declined |
| Stdlib class routes (Q13) | Use R1–R5 on `pathlib.Path`, `configparser.ConfigParser` and `configparser.RawConfigParser`. Annotation evidence costs a candidate-grade row and downstream witnessing. | Chosen |

### 6.2 Root identity and lexical scope

Use Python 3.11 binding and evaluation scopes. Build a per-file index in targets' `py_receiver.rs`.
A lexical scope has an identity even when another `def` or `class` has the same name. Record each
binding occurrence with its owner and its evidence. These rules govern lookup in a recovered tree as
well as in a clean file.

- A syntactic `from … import *` anywhere in this file vetoes every inventory identity proof,
  including an independent function-local import or builtin. Only calls tied by 6.5 are counted
  (e02/03, as5-01, n08). A wildcard in an imported sibling file or in literal text has no such effect.
- An unaliased `import a.b` binds `a` to the package. Append the source suffix to `a` once.
  `import a.b as x` binds `x` to `a.b`. A from-import binds the imported member, under its alias
  when present. This admits o01–04 and o08 and excludes sl4-05's doubled `request`. Identity describes
  the qualified call if reached. It does not require running imports to check which submodules are loaded.
  Check the actual imported root for an ancestor-local `root.py` or `root/__init__.py`, including the
  site's own file. Relative imports are local. An alias's spelling or a sibling module with no import
  is insufficient (as1-01, as2-04, as4-10). Bare builtin lookup needs no import of `builtins` (as5-15).
- The nearest binding scope decides. Count parameters, import items, function/class definitions, simple
  and chained stores, annotated and augmented stores, destructuring and starred targets, loop, context
  and exception targets (also the async forms and `except*`), assignment expressions, deletions and
  pattern captures. Count each target in a chain, even two occurrences of the same name (sl3-11).
  A capture in an OR pattern binds in the case's scope. A wildcard, value pattern, keyword label or
  class-pattern application supplies no such binding. Strings, comments and `__all__` create no imports.
  A simple annotation-only name in a function declares a local even without a stored value. A parenthesised
  annotation-only name does not (as4-19). An annotation-only class or module name stores no value (sl2-05).
- A function or lambda owns its body. Definition defaults, decorators, class bases/keywords and evaluated
  signature annotations use the defining environment. A method skips enclosing class namespaces when
  looking for a free name. Code immediately executing in a class can use its class namespace. Nested
  classes skip outer class namespaces in their bodies. Account for Python's implicit `__class__` cell
  and an annotated class's own `__annotations__` mapping (as1-02, as5-12/13).
- A comprehension's first iterable uses the surrounding scope. Its targets, later iterables, filters and
  result expression use its implicit scope and skip a surrounding class namespace. Its walrus binds the
  containing non-comprehension scope. A lambda's parameters and body-local walrus stay in the lambda.
  These are identity rules even where 5.4 assigns a comprehension site to its enclosing function.
- `global` and `nonlocal` redirect bindings and lookups. A declaration alone contributes zero writes.
  Include redirected imports and writes from other functions in their actual owner. An intervening
  function's global declaration affects its children's free lookup. A class declaration does not redirect
  a method's free lookup (sl1-01, sl2-01/02). Resolve nonlocal to the nearest enclosing function binder,
  skipping classes and unrelated closures (as2-16).
- Normalize identifier tokens to Python NFKC before scope or table comparison. Apply Python's
  class-private mangling to lookup keys in their lexical class context. Literal strings passed to
  reflection use their decoded values unchanged. A literal `"__session"` differs from the compiled
  attribute `_C__session` (sl2-17/18, as1-21). Section 6.6 defines the unmangled symbol and source text.
- Two import items count as two bindings, including comma-separated imports of the same package (sl1-03)
  or identical branch imports (sl3-01). One from-import with several *different* local names can establish
  each name, including a member bound before a later import failure (as4-23).
- For executable evidence, refuse a sole import inside the true arm of a proven `typing.TYPE_CHECKING`
  guard (including its import aliases), or a sole local value binding textually later than its use in the
  **same execution scope**. These give `binding_unavailable` (o11, sl3-02). This is a conservative
  source-order check, including loops. It does not order a deferred body against a module definition,
  another function's global setup or its enclosing function's initialisation. Other conditional imports
  are potential bindings. All competing occurrences still count. A type-only import can supply the
  static class in a string, postponed or unevaluated local annotation (as1-14). An eagerly evaluated
  signature still needs a runtime-visible class or wrapper at its definition.
- Count explicit writes/deletes through unshadowed `globals()[<literal name>]` as module bindings
  (as3-06). Replacing the module's `__builtins__` disqualifies bare builtin fallback (as4-20).
  On an otherwise proven imported path, a store/delete to that member or a prefix below the root
  invalidates the path (`member_rebound`, as1-05). Include an unshadowed builtin
  `setattr`/`delattr` with a literal member on that path. Compare the resolved object and member in
  the file, so an unrelated object's `get` does not invalidate `requests.get`.
  Other runtime namespace mutation, imports hidden in `exec` and arbitrary alias tracking are outside
  the proof. Never execute source to obtain a binding.

| Decision | Option and main cost | Choice |
|---|---|---|
| Census ownership | Reuse taint's private `Sites`. It totals spellings file-wide, treats declarations as flags, and would overlap the scheduled move-only extraction into `crates/prism-taint`. | Declined |
| Census ownership | Implement the scope index in targets. This costs a Python binding visitor. Reuse targets' import decomposition and local-module check, preserve the echo caller, and take no dependency on taint or its move sequence. | Chosen |
| Duplicate imports (Q12) | Coalesce equal package objects. This needs an equivalence rule for dotted, conditional and aliased imports. | Declined |
| Duplicate imports (Q12) | Count occurrences per deciding scope. A safely repeated import can be missed. | Chosen |
| Executable evidence | Accept any single textual import and ignore explicit writes to its members. This labels the later local import in sl3-02 and the replacement callable in as1-05 exact. | Declined |
| Executable evidence | Apply the bounded availability and write checks above. This can lose an earlier valid call in a file with a later mutation. General reaching-definitions analysis is outside the lane. | Chosen |
| Identifier comparison (Q9) | Use raw spelling for identity. This misses the actual call and shadow in sl1-13/14. | Declined |
| Identifier comparison (Q9) | Normalize identifiers and mangle lexical keys. This requires separate identity keys and source ranges. | Chosen |

### 6.3 Constructor routes R1 to R3

Resolve the constructor at its own expression and validate the selected class's method. Grouping
parentheses are transparent to identity. A mere `Session` spelling cannot prove a constructor.
An independent success can explain a library-looking receiver (sl2-21, as3-34).

- **R1** accepts exactly a call to the constructor as the method receiver, including imported aliases.
  Nested method results, `(x := C())`, `super()`, subscripts and context-entry calls do not qualify.
  The constructor call still competes for the line anchor (h10, sl2-16).
- **R2** accepts a single simple name assigned `C(...)` by an assignment statement, including an
  annotated assignment or a direct target of chained assignment. Unpacking and walrus targets count
  as bindings but do not establish this route. The name can live in the using function, an enclosing
  function or the same file's module. A module receiver with a redirected second write fails uniqueness (as4-11).
  A `with C(...) as name` target qualifies for a synchronous self-returning context
  (`requests.Session`, `httpx.Client`, `urllib3.PoolManager`, `pathlib.Path` on 3.11).
  For `async with`, admit `httpx.AsyncClient` and `aiohttp.ClientSession`.
  Count context items separately, including in parenthesised/multi-item forms. The bound object's
  identity persists after the suite (as2-13). Wrapping a context in another call does not qualify.
- **R3** accepts a direct `self.attr = C(...)` store (also annotated with a value or a direct chained
  target). Require literal `self` bound to the instance parameter of the owning method, or a lexical capture of that parameter. Count writes
  in methods of that exact class definition, including a nested body that captures the same `self`.
  A nested class/independent parameter owns a different object. Count plain, annotated-with-value,
  augmented, destructuring, loop/comprehension and with-target stores, deletes, and builtin
  `setattr(self, literal, …)` or `delattr(self, literal)`. Adjacent literal strings form one value.
  Count repeated method definitions within that class. An annotation without a value and a write to
  `self.attr.headers` leave the attribute binding intact (sl5-13, sl4-09).
  Class/descriptor defaults, `cls.attr`, attached methods, other-file subclasses, writes through an
  alias, a computed name or `__dict__` do not supply or invalidate this route (e07, tr3-20/21, as4-15).
  `exact` describes the bounded constructor proof. G-precision still checks the call against source.

| Decision | Option and main cost | Choice |
|---|---|---|
| R1/R2 | Follow arbitrary factories, aliases and context-manager return values. This needs return-flow or protocol inference. | Declined |
| R1/R2 | Admit the direct constructor and single name/context bindings above. Wrapped or computed receiver results are lost. | Chosen |
| R3 | Count only assignment statements. This misses literal deletion in as2-18. | Declined |
| R3 | Model every possible attribute mutation. This needs alias and cross-file analysis. | Declined |
| R3 | Count explicit same-owner attribute stores and literal setters/deleters. Mutations through aliases and overrides remain a precision limit. | Chosen |

### 6.4 Annotation and helper routes R4 and R5

For either route, resolve annotation identifiers at their **annotation site**. A signature uses the
defining environment (including a class defining a method). A local assignment uses its own scope.
A helper's body-local imports and its caller's imports cannot reinterpret its signature
(sl1-35/36/39, as1-16, as2-24). Static annotation evidence is usable with postponed annotations.

The closed grammar is one imported, listed class `C` (qualified, from-imported or aliased), optionally
nullable by `typing.Optional[C]`, `C | None` or `None | C`. Resolve the wrapper through imports,
including `from typing import Optional as Maybe`. Transparent parentheses and a statically decoded
string denoting the class or whole allowed expression are admitted, including adjacent literals
(as3-18). Parse at most one forward-string layer on any path with this grammar. No evaluation, forward
alias expansion or arbitrary subscription. Other unions, `typing.Union`, `Annotated`, generics,
casts, type comments, type aliases and subclasses provide no type proof.

- **R4** accepts a positional (including positional-only), keyword-only or ordinary instance parameter,
  or a function-local simple annotated assignment **with a value**, as the receiver name's sole
  binding. Varargs and kwargs bind collections. Ignore parameter default values as receiver evidence.
  R2 wins if the assignment itself proves a constructor, even against a different annotation
  (tr5-03, sl5-11). Otherwise R4 precedes R5 (sl2-22). A second write defeats the sole binding
  even if it uses the annotated constructor (as5-14).
- **R5** accepts a sole function-local `v = make(...)`, or `v = await make(...)` for an async helper.
  The callee is a bare lexical name resolving to exactly one module-scope function definition in the
  same file, including one under a module-level conditional. Its module binding must be unique, and no
  visible binding may hide it at the call. Unrelated same-spelling locals have no effect (tr4-03).
  A redirected global write, a shadowing parameter, a nested helper, a decorator, a generator or a
  mismatch between `await` and `async def` disqualifies it. A later module definition and a recursive
  call can qualify. Qualified calls, inline helper receivers and a split call/await cannot.
- Test generator status in the helper's **own** Python compiler scope, including unreachable yield,
  `yield from`, a nested definition's evaluated default, a nested class base and a yield flag in
  a local annotation. A nested generator body and an async comprehension alone do not make the helper
  a generator (tr5-04, sl1-40, sl3-16/17, sl4-13). Use the declared return annotation at
  `nameonly` even if the body returns a different object or code later mutates `__annotations__`
  (as1-27, as4-24). Section 6.5 defines the narrower use of a return expression for counting a loss.

| Decision | Option and main cost | Choice |
|---|---|---|
| Annotation grammar | Accept bare classes alone. This loses tested nullable, aliased and forward forms. | Declined |
| Annotation grammar | Resolve arbitrary typing constructs. Containers and unions can describe multiple receiver identities. | Declined |
| Annotation grammar | Admit the closed single-class grammar above. Other wrappers produce no R4/R5 proof. | Chosen |
| Helper visibility (Q3) | Reserve the spelling throughout the file. An unrelated local in tr4-03 would revoke a real module binding. | Declined |
| Helper visibility (Q3) | Require module uniqueness plus call-site visibility. This requires scope lookup and preserves the httpie helper. | Chosen |
| Helper reach | Follow bodies, imported factories or multiple return hops. This requires interprocedural inference. | Declined |
| Helper reach | Use one bare call and its declared return class at candidate grade. The declaration can be imprecise. | Chosen |

### 6.5 Warning eligibility and precedence

Walk actual call expressions. Exclude unevaluated calls (6.6). First try the table and receiver proofs.
A successful proof goes to the six shape tests in 5.2. Otherwise the following are the **only**
ties that make an identity failure count. Count at most once per call occurrence, before grouping.

1. A root/member spelling could name an entry, including bare `open`, or an explicit relevant import
   in the deciding scope supplies an alias or from-import of that entry. A wildcard alone identifies no
   bare `get` (n08). A wildcard plus an explicit import of `fetch` does identify a candidate (sl4-04).
   A repository-local import under an explicit alias counts too (sl5-04). A nearer unrelated binder
   hides an out-of-scope alias (as1-02). A table-spelled root hidden by an unrelated import/assignment
   is a candidate (tr2-40, as1-40). When an actual import into a known table library resolves to a
   different, off-table path, the call is simply outside the table (sl4-05, as1-25).
2. For a method of the candidate class, a visible binding of the receiver has a listed constructor form,
   a supported annotation form or a bare call tied to a same-file helper. This includes an otherwise
   matching constructor/annotation whose **own root proof fails**. Retain that root's cause at the method
   use (Q5). For example, with `s = requests.Session()` under a shadowed `requests`, `s.get(u)`
   counts once as `shadowed_root`. The constructor itself is outside the site table. A generic
   `make().get`, `dict.get`, or `s.close` on a requests session gives no tie.
3. A helper tie needs either a supported class spelling in its return annotation or a direct
   `return C(...)` in its own body with a listed constructor identity. Inspect same-file definitions on
   the bare name's lookup chain, including a module definition hidden at this call. Disregard unrelated
   function locals and returns of nested bodies. The returned-constructor witness explains n07,
   tr4-01/02, as1-17 and as2-23. A helper returning `{}` (sl1-43) supplies no tie. Such a return
   witness alone never types the receiver.

On ties use the **5.2 table order**, after attempting applicable proofs. Thus sl2-21 admits
`requests` as a constructor-typed receiver, while as1-18 takes `shadowed_root` over
`receiver_not_unique`. A non-import value binding, including `requests = import_module(...)`,
is `shadowed_root`. An absent binding is `not_imported`, even if a file of that name exists (as2-04).
A star import wins over both. An extra receiver binding wins over helper faults (tr4-01). Missing
admissible return annotation wins over a decorator on the same tied helper (tr4-02). Identity failure
wins over a frame or parse failure (as2-09). No new unsupported reason is added. The two added identity
causes are `binding_unavailable` and `member_rebound`.

Grouping a proven unsupported call uses the 5.2 key, including the **bare symbol string**, independently
of grade and source alias. Two calls in different functions with the same bare symbol can therefore group
on one line (as3-31). Sites, unsupported groups and unproven call occurrences are distinct counts.
The per-row matrix checks warning presence with count ≥1. The existing S2a aggregate test must also
assert exactly one warning for the fixture sl1-67/68/69, total 3, `shadowed_root: 2, not_imported: 1`
in that order, with no zero causes. as4-03/04 contribute two occurrences on a single line.

| Decision | Option and main cost | Choice |
|---|---|---|
| Failed proofs (Q5, Q11) | Count all method spellings. Dictionary lookups would dominate the warning. | Declined |
| Failed proofs (Q5, Q11) | Discard explicitly tied aliases and failed constructors. A known provenance failure would disappear from the counts. | Declined |
| Failed proofs (Q5, Q11) | Require the ties above, propagate the earliest applicable 5.2 cause and distinguish an absent root from a bound one. This requires carrying evidence through a failed route. | Chosen |
| Diagnostic vocabulary | Reuse one shadowing bucket for time-of-use and member writes. It cannot explain whether the import or its callable failed. | Declined |
| Diagnostic vocabulary | Keep the six unsupported reasons and seven prior unproven causes, adding the two named causes. The producer and matrix's allowed causes need two entries. | Chosen |
| Warning totals (Q10) | Use the per-row lower bound alone. It cannot detect a duplicate warning or double counting. | Declined |
| Warning totals (Q10) | Retain the row test and check the exact histogram in the existing aggregate test. It needs one multi-cause fixture assertion. | Chosen |

### 6.6 Call, frame and anchor coordinates

Use CPython 3.11 source coordinates. Parse a newline-normalised view (LF, CRLF and lone CR each end
one line). Keep a mapping back to the original bytes for `expected.detail`. A form feed inside a
comment is not a newline (as1-29/30). Compare zero-based **UTF-8 byte** columns, including the exclusive
end column on the call's own ending line. Neither NFKC nor display width changes source offsets.
tr6-02 selects the inner call because of the outer end column on the next line. The emoji in tr7-02
make the outer column larger in bytes. as2-32 has bounds 2–4 and its callee on line 3 with CR-only input.

The span is Python's `Call.func` span. Strip grouping solely around the entire callee, even when the
delimiters cross lines. In `(requests).get` the parentheses are internal to the attribute expression
and remain in its source slice (sl3-10). In a grouped `requests.get` on its own line, cite the name's
line, even when the outer grouping starts earlier. A newline inside `requests … .get` or
`Session(… ).get` makes that semantic callee multiline (u05, tr1-22, sl1-28). A multiline argument
list alone does not. All calls whose semantic callee covers L compete, including unlisted calls,
constructors, dynamic callees and calls in a statically discarded operand.

On exact equality of all three 5.4 keys, take the first candidate in Python AST breadth-first field
order (`ast.walk` on the pinned interpreter, preserving list order). Rust can assign that ordinal
from AST structure. Hash order, depth-first grammar visitation and the call's absolute end offset
are unsuitable tie-breaks. On as4-07 the outer `factory(requests.get(u))` and inner `factory`
tie at (0, 11, −35). The oracle binds the outer expression. The target `requests.get` loses.
This fixture distinguishes the selected call. It does not identify the consumer's internal algorithm.

Emit the bare, NFKC-normalized function identifier for the symbol, including `__fetch` as written,
without class mangling or qualification (as1-38, as2-10). Bounds use the `def`/`async` token and
the last syntactic token of its definition, including a closing delimiter, multiline string or trailing
semicolon on a continuation line. Exclude comments, whitespace and a bare continuation (sl2-15,
sl3-07). Check parse errors on the innermost owning definition's subtree, including its signature
and nested syntax. A clean nested definition can survive an error elsewhere in its enclosing
definition, and a nested error is inside the outer definition's subtree (sl1-24/25, sl2-14, as4-08/09).

Walk expressions, including actual f-string interpolations, assignment/deletion subscript operands
and match guards. A comment, literal text, pattern, implicit decorator application or a callable
passed as a value has no call to inventory. The owner is the innermost function **body** as in 5.4.
Choose between a class body and a lambda body by the innermost such frame since entering that
function. Neither a lambda default nor a nested definition's header runs in the body being defined.
A comprehension inside a class still uses 5.2's enclosing class-body test (as2-33).

For site eligibility, omit calls inside a function-local variable's annotation and all annotations
postponed by `from __future__ import annotations`, including signature and class annotations. Still
visit the evaluated default, RHS and evaluated parts of assignment targets (sl1-27, as4-22). Static
type spellings remain available to R4/R5. For the **binder's candidate set**, keep 5.4's specific
local annotated-assignment exception and fallback when every candidate is such a call. Elsewhere use
the syntactic key even for a postponed annotation. An unevaluated winner produces no row of its own.
as1-14 and as4-22 test the distinction between type evidence and executable expressions. The packet
has no consumer measurement for a postponed signature call competing with a default on the same line.

PEP 695 type statements and type-parameter scopes are outside the pinned 3.11.11 grammar. Accept loss
of sites whose proof requires those constructs. Treat an identifiable alias declaration as an opaque
binder, never as a listed class. There is no PEP 695 matrix row or interpreter-parity claim.

| Decision | Option and main cost | Choice |
|---|---|---|
| Equal key (Q1) | Refuse ties. This can drop a representable winner. | Declined |
| Equal key (Q1) | Compare absolute end offsets. This introduces a line-dependent key absent from the recorded rule. | Declined |
| Equal key (Q1) | Use stable Python AST order after the three key components. It requires preserving AST field order. | Chosen |
| Postponement (Q2) | Inventory all syntactic annotation calls. This invents executable sites. | Declined |
| Postponement (Q2) | Erase annotations entirely. This loses usable static receiver types. | Declined |
| Postponement (Q2) | Separate executable-site eligibility, line binding and static type lookup. Each uses its stated context. | Chosen |
| Grouping and coordinates (Q4, Q7, Q8) | Use tree-sitter grouping spans, LF rows or character columns directly. They can change line, bounds or winner. | Declined |
| Grouping and coordinates (Q4, Q7, Q8) | Use semantic spans, universal newlines and original UTF-8 byte columns. This requires a source-position map. | Chosen |
| Version scope (Q6) | Include PEP 695 now. It needs another interpreter oracle and a type-parameter scope model. | Declined |
| Version scope (Q6) | Bound this lane to the pinned grammar. New-syntax cases stay outside the measured population. | Chosen |

### 6.7 Accepted losses and G-parity

The verdict file is the per-ID loss register. Its 90 unsupported expectations represent groups, with
multiplicity in `want`. Ten lack a function body, 15 are in a class frame, nine in a lambda frame,
five have a multiline callee, 43 lose the line anchor, and eight have an error in the owning definition.
Keep their diagnostics. The contract cannot encode the missing frame, a second call on the line or
dependable bounds for that definition. Runtime refusals of otherwise representable rows belong in
the funnel.

The 146 counted omissions break down as 9 `star_import`, 58 `shadowed_root`, 11 `not_imported`,
7 `repository_local_module`, 2 `binding_unavailable`, 1 `member_rebound`, 40 `receiver_not_unique`,
4 `helper_unannotated` and 14 `helper_not_admissible`. Each row's cause identifies the missing evidence
in 5.2. Proving branch-specific values, trusting local shims or inferring undeclared returns would
admit sites outside the chosen identity proof.

The 67 plain `none` decisions are accounted for below. Some are negative controls, where there is
no executable table call to recover.

| Reason for plain `none` | Row IDs |
|---|---|
| No explicit call to the dependency. Subscripts, patterns, decorators applied implicitly, callables passed to higher-order functions, comments, literal text and non-executing annotation subtrees contribute no site. | f02, tr1-14, tr1-20, tr3-12, tr3-13, sl1-09, sl1-62, sl1-70, sl4-06, as1-29, as3-10, as4-21 |
| Constructor alone or a qualified member outside the closed family/class table. Extending the table or equating re-exports changes the published family boundary. | n01, n02, tr2-74, sl1-44, sl4-05, as1-25, as1-34, as1-35, as2-11, as3-05 |
| No visible tie at the use. The spelling belongs to an unknown parameter, wildcard member, unrelated helper, comprehension local or implicit class binding. Guessing from `get` would include dictionary calls. | n03, n08, sl1-43, sl3-12, as1-02, as2-15, as5-13 |
| A return, factory, descriptor, class attribute/default, different-file helper, attribute or callable alias, wrapper, operator, reflection, or intermediate result would have to be followed. R1–R5 stop at their stated expression or single binding. | sl1-42, sl5-09, sl5-10, sl5-12, sl5-14, as1-06, as1-11, as1-23, as1-33, as1-39, as2-14, as2-17, as2-25, as2-29, as3-03, as3-04, as3-21, as3-23, as3-26, as4-12, as4-13, as4-18, as5-07, as5-08, as5-11 |
| The annotation is on a different target, is outside a function-local R4 binding, or would require narrowing, alias expansion, cast, comment, subclass, container-element or wrapper interpretation. No unique receiver type is admitted by 6.4. | sl1-37, sl1-64, sl2-24, sl3-18, sl4-12, as1-12, as1-13, as1-15, as2-26, as3-14, as3-19, as4-16, as4-17 |

**G-parity has a narrower, closed list.** Apply it only to a standard-library injectable key present
in scan and missing in prism on the common file set. Record the key, source witness, class below and
prism outcome for each miss. A file-total warning alone cannot locate a miss. Every accepted miss stays
in the denominator and the 95% floor applies **per corpus**.

| Class | Required witness |
|---|---|
| P1. Competing or unavailable binding | Demonstrate an actual wildcard, competing deciding-scope occurrence, explicit namespace/member mutation, unavailable binding or non-unique receiver as specified by 6.2–6.5. Name the precise cause. |
| P2. Relative or repository-local identity | Show the relative import or the actual ancestor-local module/package of the imported root. This includes re-exports of real stdlib functions from a local shim. |
| P3. Indirect receiver or callable | Show one of these missing proof forms: assigned callable/module/object alias, constructor/default/wrapper or method/operator result, attribute/descriptor/container/subclass receiver, dynamic import/member lookup, indirect or cross-file helper, same-file helper missing a usable return annotation or failing another condition in 6.4, or a type form explicitly excluded in 6.4. Cite the relevant loss above or helper cause. |
| P4. Unrepresentable call or frame | Demonstrate the inner class/lambda frame, the multiline semantic callee, or the different winning line anchor under 5.4/6.6. Record the actual frame/span/winner. |
| P5. Unevaluated annotation | Show that the recorded call occurs exclusively in an unevaluated annotation under 6.6 on 3.11. |

A missing table member, missed ordinary import, wrong bounds/symbol/line, wrong canonical hint,
failure to parse valid supported Python, missing scan run or unexplained difference fails the gate.
`module_level` pairs with scan's bounds-less set. A genuine syntax error is reported with the scan's
file/parse evidence and grants no automatic exception for a missing injectable key. The P classes do
not excuse a false positive or lower the numerical floor. An unclassified miss goes to ORCHESTRATION
section 8.

| Decision | Option and main cost | Choice |
|---|---|---|
| Accepted loss | Infer values across files. This requires a new interprocedural proof. | Declined |
| Accepted loss | Keep the finite limits and publish every tied failure, unsupported group and plain negative above. Real indirect dependency calls can be missed. | Chosen |
| G-parity | Allow any unsupported or warning bucket. This could conceal a systematic defect. | Declined |
| G-parity | Pool the recall across corpora. A large corpus could mask a failing smaller corpus. | Declined |
| G-parity | Require P1–P5 with a site witness and retain each corpus's floor. New loss mechanisms need design escalation. | Chosen |

### 6.8 Oracle reconciliation and owner questions

Join on **(fixture, line)** and check the ID in the oracle's `rows`. Several IDs may share one binder
answer. There are 615 distinct lines for 631 IDs. In this table the counts are **row IDs**, using the
amended wants. “Syntax” means the oracle returned an error record with no bound call or guard answer.

| Want | accepted | unsupported_frame_guard | unsupported_multiline_callee | call_position_unavailable | site_call_unavailable | Syntax | IDs |
|---|---:|---:|---:|---:|---:|---:|---:|
| site | 311 | 14 | 0 | 0 | 0 | 3 | 328 |
| unsupported | 43 | 36 | 2 | 1 | 0 | 8 | 90 |
| none(unproven) | 135 | 10 | 0 | 0 | 0 | 1 | 146 |
| none | 54 | 3 | 0 | 4 | 6 | 0 | 67 |
| Total | 543 | 63 | 2 | 5 | 6 | 12 | 631 |

On all 603 parseable fixture lines a CPython 3.11.11 AST comparison reproduced the recorded bound
callee text or absence of a call, including the full-key tie. The other 12 lines have syntax errors.

- For all 325 parseable site IDs the binder selects the source callee. The enclosing AST function's
  bounds agree with the recorded bounds, and its name agrees with each explicit matrix symbol.
  Of the 14 site frame refusals, s03, c06, o20,
  tr1-23 and sl2-25 are comprehensions/generator expressions. s04, b04, tr1-24, sl5-08 and
  as4-01/02 have decorated or repeated definitions. c02, tr3-06 and tr3-07 have a nested header
  or lambda default evaluated in the surrounding scope. Keep the rows and tabulate the refusal.
- Of the 325, 177 have the same nonempty qualified name as the hint. All 135 receiver-route sites have
  an empty oracle qualifier. Compare their bound source text and record that empty field beside the
  constructed class path. For the other 13 empty qualifiers the source binding is still required.
  The five bare builtin sites are w06b, f01a, as1-31, as4-19 and as5-15. The eight imported roots
  are tr2-53, tr2-54, tr3-02, tr5-02, sl1-01, sl1-04, sl2-02 and sl2-04 (declarations, redirected
  imports, comprehension scope or private-name lookup). Preserve Python's identity. A blank is
  missing qualification evidence. For a census import-root row G-accept needs the consumer's
  nonempty matching qualified name. For a bare builtin it needs independently recorded canonical
  `builtins.open` from the consumer, including a planner derivation if available. Missing or
  conflicting evidence leaves that check unmet.
- o23a, sl1-24 and as4-09 have a clean innermost definition in a file the consumer cannot parse.
  Keep the sites. The input `consumer` label `unparseable_file` summarises the syntax error.
  The other nine syntax records are o23b, tr3-08/09, sl1-25, sl2-14, as1-10, as3-11 and as4-08
  (unsupported), and as2-09 (identity fails first). None provides a consumer verdict on the intended
  call. A recovered site on a census file still needs executed G-accept evidence.
- For unsupported rows an `accepted` answer describes the one **anchor** on the line. In 38 cases
  the row targets another occurrence, even if its text matches (u04b). In the five `multiline_callee`
  cases, u05 and tr1-22 actually bind the multiline target and the recorded guard accepts it.
  tr3-11, sl1-28 and as2-27 instead bind `consume`, `Session` or `Client`. Keep the 5.2 reason
  and group count in all five. For sl1-16 and sl2-11 the **winner** gets
  `unsupported_multiline_callee`. The target loses the anchor. In as3-08 the winner is `ignored`
  and the guard has no call position. The 36 frame refusals likewise describe the winner. Select
  unsupported reasons per intended occurrence before grouping, including when two rows share a line.
- For each plain or counted `none`, binding a source expression or qualifying its text supplies no
  evidence of table membership or of runtime library identity. Use that ID's proof or loss in 5.2/6.7
  for every guard cell. In particular, preserve the omissions even where `qualified` names a table
  entry despite a local module, shadow, wildcard or write. A position/frame refusal cannot turn an
  identity failure into an unsupported site. The four no-position plain negatives are unevaluated
  annotations. The six no-call negatives have no call at the line. The remaining frame refusals and
  accepted negatives follow the same per-ID rule.
- All 24 `faultpath-guard` wants and all 158 `python-semantics` wants stand. Of the 449 design
  wants, nine change: o11 and sl3-02 for runtime availability, as1-05 for a written member,
  as3-06 and as4-20 for namespace writes, as2-18 for a literal attribute delete, sl4-04 and sl5-04
  for explicit import-alias warning ties, and tr4-03 for visible helper scope. On all nine lines
  the recorded guard says `accepted`. For tr4-03 it binds `s.get` and supplies no class qualifier.
  Each amended result follows the source proof. The test implementer transcribes the verdicts after
  design approval, including `callee=requests.Session.get` for tr4-03's current `*` selector.

| Decision | Option and main cost | Choice |
|---|---|---|
| Guard disagreement | Make every guard refusal an unsupported site and every acceptance an injectable site. This drops representable frames and admits calls with no proven family. | Declined |
| Guard disagreement | Preserve 5.2/5.4 and compare source binding, qualification, guard and planning separately. Some real sites still lack consumer evidence for G-accept. | Chosen |
| Owner: schema / occurrence key | Add columns or an occurrence ID. This needs a new contract version and consumer work. | Owner option |
| Owner: schema / occurrence key | Use existing 1.1, bare symbols, grouped diagnostics and the current line key. Same-line losses remain. | Default |
| Consumer qualification and frames | Extend the qualifier or guard for these fixtures. This needs consumer-side validation. | Owner option |
| Consumer qualification and frames | Keep each source-derived row and report each missing qualification or refused frame. Some G-accept evidence may remain unavailable. | Default |
| urllib faults | Extend the HTTP catalogue to `urllib.request`. This requires consumer fault selection and delivery tests. | Owner option |
| urllib faults | List urllib sites and record planner outcomes under the current catalogue. Discovery may yield no applicable fault. | Default |
| Scan coexistence | Choose precedence or deduplication for scan and prism rows. Changing planning policy affects existing inputs. | Owner option |
| Scan coexistence | Present the parity comparison to the owner. Duplicate discoveries can persist meanwhile. | Default |
| Tooling pilot | Adapt the pilot driver to accept a foreign targets document. This requires tooling work and a qualified run. | Owner option |
| Tooling pilot | Keep the lane's plan/run G-live gate and hand the separate pilot run to the tooling owner. Pilot-level evidence comes later. | Default |

A missing oracle run or `unsupported_interpreter` / `unqualified_runtime` is an inadmissible probe.
Keep the rule-derived want and mark consumer evidence pending until a qualified run is available.
This matrix has no planner, scan, live injection or corpus gate receipt.

## 7. Gates

| Gate | Statement | Oracle |
|---|---|---|
| G-overlay | on httpie at the pinned commit, the inventory emits a row for each of the overlay's HTTP sites (`client.py:114`, `update_warnings.py:44`) with the overlay row's file, line, bounds and kind, and the bare function name as symbol | the archived overlay; faultpath's site guard |
| G-bounds | for every function of every census corpus, prism's (file, start line, end line, name) equals the `ast` listing of the stage M census: zero differences | Python `ast` under the pinned interpreter |
| G-accept | on every census corpus, for every injectable inventory row: (1) faultpath's loader accepts the document; (2) the call faultpath binds at `site.line` is the row's call: its callee text equals the row's source callee, and, import-root rows require the consumer's matching qualified name. Bare builtin rows require the consumer's matching canonical builtin identity (6.8). For receiver routes record both spellings and check the bound source callee; (3) the site guard, run with strict line checking, does not answer `site_call_unavailable`, `unsupported_multiline_callee`, `stale_function_bounds` or `stale_site_symbol`; (4) for a row whose resolved callee is in the consumer's tables (HTTP rows rooted in `requests`, `httpx`, `urllib3` or `aiohttp`; every standard-library row) and whose patch the planner resolves, the plan has no `dependency_identity_mismatch`, `no-dependency-hint` or `no-applicable-fault` note. Every other guard answer and planner note is tabulated per reason, not gated. The HTTP half runs in slices S2a and S2b, the standard-library half in S3 | faultpath's loader, call binder, site guard and planner, run read-only (the planner reads source only; no suite run) |
| G-parity | on every census corpus, restricted to the files of scan's `diff.files`: prism lists at least 95% of scan's injectable rows (rows with bounds) keyed by (file, line, function bounds, family, canonical callee); every miss has a source witness in the closed list P1–P5 of section 6.7; every row prism lists that scan does not is confirmed against the source, and none is a call that is not that library call. Prism's `module_level` rows are compared with scan's bounds-less rows, and prism's unsupported and unproven sets with scan's warnings. If scan cannot run, G-parity is not met | `faultpath scan` |
| G-precision | a reader checks HTTP rows against the source, stratified by route: every row of each of R1 to R5 and of root identity when a stratum has 50 rows or fewer across the census corpora, otherwise 50 drawn with a recorded seed; at most 1 row per stratum where the callee is not the listed library call; every stratum's size is reported | hand check |
| G-live | on httpie at the pinned commit, with a document produced with `--inventory-kinds http`, the plain flow `faultpath plan` then `faultpath run` with `--only-target` for the two HTTP overlay sites, under the pilot's interpreter, package pins, pytest options and deselections, after a no-fault run for the flake control: (i) `client.py:114`: the planner resolves the patch to the requests session's `send`, the probe is reached, and at least one fault entry is delivered and gets a verdict naming test ids; (ii) `update_warnings.py:44`: the planner resolves `requests.get`, and the run reports the site as not reached or shadowed because the suite mocks it. Per-fault verdicts are recorded beside attempt 5's as information, not as a condition | faultpath run |
| G-byte | without `--inventory`, `prism targets` output on the existing fixtures is identical between the merge-base binary and the head binary after removing `producer.version`, `producer.build_identity` and `repo.root` | fixture comparison |

`models.py:65` and `:68` in the overlay are `resource_acquire` rows for response-body iterators reached
through a method of an in-repository class. They need a typed receiver for a library *response* object,
which is outside this lane's routes. The lane reports whether the inventory finds them and does not gate on
them.

The pilot-level run (reach census, pilot configuration, the pilot driver) on prism's document is not this
lane's gate; it is handed to the tooling controller (decision 6).

## 8. What the lane reports at the end

A funnel table for every census corpus, static steps: dependency call sites the independent AST census
finds → inventory rows → injectable rows → accepted by faultpath's guard → planner gives at least one fault,
with the loss reason at each step (prism's unsupported reasons and identity-unproven causes; the consumer's
guard answers and planner notes). For httpie it continues with the live steps of G-live: reached by the
suite → injected → caught / not caught. This is the "where coverage is actually lost" output, and it
separates prism's losses (identity unproven, unsupported shape) from faultpath's and the suite's (runtime
limits, not reached, patch shadowed by a mock).

## 9. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Where the inventory lives | a row source inside targets (approach B) | moving to a nav query is a move-only change of four files | before lane 5's pilot |
| 2 | Families | **Confirmed by the owner, 2026-11-20:** HTTP first (slices S2a, S2b), then filesystem, parse, config with scan parity (slice S3) | stopping after S2b leaves standard-library discovery to `faultpath scan`; nothing built in S2a or S2b is wasted | after S2b merges |
| 3 | Schema version of an inventory document | 1.1, the already-promoted minor | emitting 1.0 would forbid the `parse` and `config` kinds | before S2a |
| 4 | Callee in the hint | resolved dotted callee (the consumer's canonical spelling for the standard-library families); text as written in `expected.detail` (issue #613 asked for source text in the hint). The id depends on the resolved callee through the description, so a later change of a canonical spelling changes those rows' ids once | swapping the two fields changes every inventory id once | before S2a merges |
| 5 | Unsupported sites | rows of kind `other` with a reason | moving them to warnings removes rows; consumers that count them change | before S2a merges |
| 6 | Who runs the live httpie proof | **changed by design review round 1:** the lane runs the plan-then-run proof (G-live, required for lane acceptance), using faultpath read-only, outputs in the evidence directory; the pilot-level re-run (reach census, pilot driver) is handed to the tooling controller with a note written at close-out | running the pilot-level re-run in this lane needs a driver that accepts a foreign targets document, which faultpath's `main` does not have | at S4 |
| 7 | How faultpath treats prism rows next to scan rows (prefer one, dedupe, trust level) | the lane supplies parity evidence to the owner, who decides whether to file or act on a consumer issue | — | owner |
| 8 | A line-independent site key (to follow a site across edits) | not added; faultpath binds to a pinned source and does not need it | a new field needs a new contract minor | when a consumer asks |

## 10. Risks and accepted limits

- **A second discovery implementation next to scan.** G-parity makes the overlap measurable; decision 7 is
  where the owner picks one. Until then both may feed faultpath and duplicate sites differ in id (different
  source algorithm), which faultpath already tolerates for overlay rows.
- **Receiver typing is shallow.** One constructor binding, one annotation, one helper hop. Sessions passed
  through several functions or stored in containers are missed. Only the explicit ties in 6.5 enter the identity-unproven counts.
- **The closed-world rule costs recall** where Python code rebinds a name it does not use at the call
  (`try`/`except ImportError` fallbacks, a parameter defaulted and then assigned). Faultpath's own binding
  check can report a different identity or no qualification. Section 6.8 defines how to compare them.
- **One call per line** is faultpath's binding: a table call that is not its line's anchor is listed as
  unsupported, not injected. The rule is ported, not approximated, so prism and faultpath agree on which
  call a line names.
- **Consumer runtime limits** (decorated functions, comprehension frames on 3.11, dynamic callees, nested
  functions, `with` receivers, annotation witnesses) leave some injectable rows unusable by faultpath today.
  They are counted in the funnel under the consumer's reason, and a lifted limit needs no prism change.
- **Sites a test suite mocks** are discovered but faultpath reports them as shadowed. That is correct and
  shows in the funnel.
- **Projected finding rows keep their end-line defect.** Rows projected from findings still take bounds from
  the tree-sitter function node (trailing comment lines, decorator start). Fixing that changes existing rows
  and breaks G-byte, so it is filed as a follow-up issue at close-out, not done here.
- **PR #841** edits `cli.rs` and `main.rs` for targets flags. This lane's CLI change is two options and one
  branch in `run_targets`; rebase over #841 if it merges first.

## Change log

- 2026-11-21. Folded architecture review round 1, including the plain plan/run live gate.
- 2026-11-24. Stage D fixed the HTTP table, R1–R5, binding ownership, warnings, coordinate rules and
  parity loss classes against 631 rows. The verdict ledger records 622 keeps and nine design changes.
  The supplied oracle covers 615 fixture lines. The two-reviewer Stage D gate is pending.
