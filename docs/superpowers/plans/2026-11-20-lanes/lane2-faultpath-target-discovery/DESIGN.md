# Lane 2: faultpath target discovery — design

**Status (2026-11-25).** The owner directed the r4 fold and selected K for declaration-only standard-library receivers. Both d3 reviewers rejected `ffcbe8fa`. Both reviewers will review this owner-directed revision once. Tracking issue #613.

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
   symbol and a hint from an admitted static identity route. For the three standard-library families use
   the consumer's canonical callee and kind, including eligible declarations at `nameonly` (§6.4).
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
| `confidence` | `exact` when identity is proven by imports and constructor bindings (root identity, R1 or R2). R4/R5 use `nameonly` under §6.4, including admissible declaration-only standard-library receivers (§6.10). R3 supplies diagnostics only |
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
| `shadowed_root` | A non-import binder or unrelated absolute import hides a table spelling, competing root bindings survive Q12, class lookup has competing outcomes, or a namespace write invalidates the root. A sole tied relative import goes to `repository_local_module`. Resolve a sole import of a candidate table path under the locality and availability cells below (§6.2). |
| `not_imported` | No binding occurrence for the candidate name is visible to lexical lookup, and builtin fallback does not apply. |
| `repository_local_module` | A relative import or a module/package for the imported root found by the §6.2 lookup. |
| `binding_unavailable` | The sole effective binding is type-only at an executable use, or has no available occurrence before this use in the same local execution scope (§6.2). |
| `member_rebound` | A modelled write or delete hits a member or prefix required by the chosen callable, including bare builtins and receiver methods (§6.2). |
| `receiver_not_unique` | More than one binding of the tied receiver name or more than one modelled write of the instance attribute. |
| `annotation_conflict` | The declaration conflicts with value evidence and no independent sole construction resolves the method (§6.4). |
| `helper_unannotated` | A same-file helper tied by a returned constructor lacks an admissible return type. |
| `helper_not_admissible` | A tied helper fails the definition, visibility, decorator, generator or await conditions of section 6.4. |
| `receiver_value_unproven` | A tied R3 field lacks an admitted instance proof, a tied HTTP declaration lacks its §6.4 construction witness, or a returned local fails the escape or rebinding check. Apply after the earlier causes. An admissible standard-library declaration supplies a `nameonly` route. |

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
or the bare builtin fallback. Count competing binding occurrences throughout that scope, applying the narrow Q12 equivalence in §6.2. Section 6.2
defines lookup, redirections and writes. Share import-path construction and repository-local checks with
the existing targets proof. The inventory uses its own deciding-scope census. Echo retains its existing
selection and admission policy.

**Receiver routes.** First try a proven root path in the table. For a method on an instance use the first
admissible route below. Verify the method in the chosen class's list. The constructor or annotation is
resolved where it is written. In the R4/R5 summary below, the construction requirement and the
parameter-only diagnostic apply to HTTP. §6.4 governs constructed-class precedence for all families
and admits eligible declaration-only standard-library receivers at `nameonly` (§6.10).

| Route | Evidence | Grade |
|---|---|---|
| R1 | The receiver expression is the listed constructor call itself. | exact |
| R2 | A single name binding assigns the constructor result or takes the matching context-manager target (6.3). | exact |
| R3 | A constructor store on the owning instance field ties a diagnostic. Positive field inference is deferred (§6.3). | no row |
| R4 | Local annotated assignment with the matching, constructor-backed helper result in §6.4. A parameter alone ties a diagnostic. | nameonly |
| R5 | A single local assignment calls one visible same-file module helper with an admissible return annotation, matching await form and constructor-backed return (§6.4). | nameonly |

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
    inventory row. For grouped diagnostics, apply confidence and tier to occurrences before grouping (§6.5).
    A severity floor above `info` counts the surviving site rows and diagnostic groups removed in the warning
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

The sibling `row-verdicts.jsonl` records the owner-directed r4 `want` and governing `rule` for 772 IDs.
Of these, 738 join the materialised matrix (644 keep, 94 change). The 34 `add` records supply fixture source.
There are 305 HTTP root, 287 HTTP receiver and 180 standard-library IDs. §6.8 reconciles the 738 IDs
with the supplied oracle. §6.9 checks the pins and Superset shapes. §§6.10–6.11 record the owner's
decision and its eleven ledger substitutions.

| Pinned input or verdict ledger | SHA-256 |
|---|---|
| `tests/targets_inventory/rows.jsonl` at `9e43bbec` | `f3f8927ee594cf0fb8737b6df97a0aa0e4f5391d62bed91afbaef7e040cdaafd` |
| Supplied `oracle/site-oracle.jsonl` (722 fixture lines) | `ee80f583ec150ea52ebe70899703ad4beaaaa31a73d56b2c98beab0e92922b32` |
| Sibling `row-verdicts.jsonl` (K, 772 IDs) | `f49c094cc597689da02c6a88518cdf8e78d1b8c134d22af89c5c903a8e264159` |

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

The stage M denominator includes every listed HTTP site, including unproven sites. The four
`census/sites-*.jsonl` files give the following shares. The two R2 forms each account for one Black site.

| Tie | httpie (7) | Saleor (2) | Superset (25) | Black (2) |
|---|---:|---:|---:|---:|
| Root import or from-import | 6/7 | 2/2 | 12/25 | 0/2 |
| R1 direct constructor | 0/7 | 0/2 | 0/25 | 0/2 |
| R2 constructor name or with-target | 0/7 | 0/2 | 0/25 | 2/2 |
| R3 instance attribute | 0/7 | 0/2 | 0/25 | 0/2 |
| R4 annotation | 0/7 | 0/2 | 1/25 | 0/2 |
| R5 annotated helper | 1/7 | 0/2 | 0/25 | 0/2 |
| Other | 0/7 | 0/2 | 12/25 | 0/2 |

The Superset R4 parameter accounts for 4% and lacks an admissible value witness. The httpie R5 tie
at `httpie/client.py:114` accounts for 14.29%. In the pinned source, `build_requests_session` constructs at line 161, calls `mount` at 173, 174 and 179, then returns that local at 184. The call at 68 binds it in `collect_messages`. The `cookies` store and `prepare_request` and `merge_environment_settings` calls leave the indexed `send` slots intact. §6.4 gives R5 `requests.Session.send` at `nameonly`. S2b still owes the emitted-row and consumer receipts.
Root identity covers the other overlay HTTP site at `update_warnings.py:44`. Only R1 and R3 as
whole routes qualify for the optional under-2%-in-every-corpus cut.

| Decision | Option and main cost | Choice |
|---|---|---|
| HTTP membership | Admit library prefixes or all verb names on every class. This would classify utilities, constructors or absent methods. | Declined |
| HTTP membership | Use only census hits. This drops tested APIs absent from these four projects. | Declined |
| HTTP membership | Use only patchable callees. This drops the measured urllib population. | Declined |
| HTTP membership | Maintain the explicit module and per-class lists above. Additions require an entry and a fixture. | Chosen |
| Routes | Drop R1. This removes h10 and introduces a family-specific exception for direct construction. | Declined |
| Routes | Defer positive R3. Even the unconditional field pattern in h07 loses its hint. No Stage M HTTP site uses it. Retain tied field diagnostics. | Chosen in escalation |
| HTTP routes | Keep R1/R2 and constructor-backed R4/R5. A default or parameter type cannot establish the supplied value. | Chosen in escalation |
| Stdlib class routes (Q13) | Use construction when witnessed for `pathlib.Path`, `configparser.ConfigParser` and `configparser.RawConfigParser`. R4/R5 also admit the sole eligible declaration at `nameonly` (§§6.4, 6.10). | K, chosen by owner |

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
  For absolute imports check the actual imported top-level name for `root.py` or
  `root/__init__.py`. Inventory passes a deduplicated list to `repo_has_local_module`: the repo
  root, conventional `<repo>/src` if present, and site ancestors below the repo root that lack
  `__init__.py`. Inspect the filesystem even outside `--files`. A sibling of the importer inside
  a regular package does not enter the absolute search through that package directory.
  Hence `import json` at `httpie/output/formatters/colors.py:183` resolves `json.loads` to the
  `parse` table entry at `exact` despite `httpie/output/formatters/json.py`, as the census records.
  For absolute `builtins`, CPython's builtin module takes precedence, including qualified,
  aliased and from-import uses. Lexical rebinding, relative imports and member writes still apply.
  For the pinned CPython 3.11 release startup, an absolute import of `os` or `io` uses
  its preloaded or frozen module ahead of filesystem lookup. `os.path` uses the platform
  path module installed by that `os` (including aliased and from-import forms).
  A same-named local file alone gives no locality veto. Apply this exception only to these
  startup identities on that profile. On a different startup configuration require origin evidence
  for this precedence. Relative imports, lexical replacement and §6.2 slot writes still count.
  Bare builtin fallback requires no import. Echo passes its original ancestors (including packages)
  and keeps its original admission policy. The common helper searches exactly the caller's roots.
  The corpus's actual import roots determine P2 credit (§6.7).
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
- For immediate class lookup, consider the class namespace **at this expression**. An earlier
  unconditional simple store/import establishes the local slot. A missing key (including after
  deletion) takes Python's fallback. For a name bound by the class block this is globals then
  builtins, even inside a function. A genuine free variable can use its enclosing cell.
  Preserve explicit global/nonlocal declarations. An annotation without a value leaves the key
  absent. For a conditional store, import or delete, retain both reachable slot states, including
  absence. Propagate simple straight-line statements and union arm exits; when availability is
  uncertain include absence and every possible local value. A populated class slot hides fallback
  only on that path. Accept only if every possible lookup proves the same canonical identity and
  passes locality and write checks. Different or unresolved alternatives on a tied spelling give
  `shadowed_root` (wildcard still wins). This join handles one conditional import and its fallback;
  it does not coalesce repeated conditional imports under Q12.
  Apply the rule to expressions evaluated in the class, including inner class headers, defaults,
  decorators and comprehension first iterables. Methods and comprehension bodies keep their own
  lookup. See esc11–13 and esc26, with as3-01, sl2-05 and tr7-01 as straight-line controls.
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
- **Q12.** Keep each import occurrence in the index. Collapse a set for the root proof only when
  every binding is an absolute, unaliased `import p` or `import p.suffix` of the same top-level
  package, directly in the same module or function suite (including comma-separated items).
  Each stores the same package under `p`. At an immediate use at least one must already be
  available. Any conditional arm, alias, from-import, redirected write or other binder in a repeated
  set prevents coalescing (sl3-01, o10, as1-24, wf99). A single from-import can establish each of
  several distinct names, including a member captured before a later import failure (as4-23).
- For executable evidence refuse a sole import in the true arm of a proven type-only guard.
  Recognize a direct test of `typing.TYPE_CHECKING` or `typing_extensions.TYPE_CHECKING`, including
  verified import aliases, and a direct test of the name `TYPE_CHECKING` whose sole, available
  unconditional binding in the deciding scope is the literal `False`. Resolve the guard where
  written and check module origin and member writes. Other values, same-spelled parameters,
  computed tests and unproven wrappers use the ordinary conditional-import rule.
  Also refuse a sole local value binding textually later than its use in the **same execution scope**.
  These give `binding_unavailable` (o11, sl3-02). This source-order check includes loops. Do not
  order a deferred body against module or enclosing-function initialisation or another function's
  global setup. Other conditional imports are potential bindings subject to the class fallback rule. A type-only import can supply the
  static class in a string, postponed or unevaluated local annotation (as1-14). An eagerly evaluated
  signature still needs a runtime-visible class or wrapper at its definition.

**Same-file writes (Sol 2–3, Opus W1).** For each proposed proof, build the paths it actually reads.
Compare stores and deletes to these paths using binding owners and canonical import paths. A hit on
a required slot invalidates proof only if it can affect the read of **that slot**.
Resolve each write at its own binding site. Check this before grade and shape. A sibling member
alone supplies no veto. Use the following bounded ordering rules.

1. A from-import reads the export slot at the import statement and stores the object in a new
   binding. A later replacement of the export cannot alter that completed capture. Check subsequent
   writes to the captured name and to slots read *on* the captured object separately. A captured
   class still uses its current construction protocol and instance method. A captured module still
   uses its current members. Preserve the capture point and object prefix across aliases. wf26/27,
   esc19/20/28 distinguish these cases.
2. In straight-line module initialisation, order direct suite statements within that initialisation.
   In a function an available local import, constructed value or qualifying with-target binds a
   fresh local name for this invocation (including a §6.4 helper result). A later reassignment or
   deletion of that name cannot reach an earlier read when its binding dominates the read, both are in that activation, and no back-edge from the
   write reaches the read without a fresh binding. The simple proof admits sequential statements
   of a loop-free suite, including the direct body of a qualifying with for its own fresh target.
   Apply the same ordering to writes on the freshly constructed instance when it is unescaped
   (its only uses up to the call are the tracked binding and member read). Apply this exception before
   counting competing occurrences (n06, wf38, esc29). A later write to the referenced module
   itself still has the persistent lifetime in rule 3. A local declaration such as `del open` still affects lexical lookup
   throughout the function (wf25).
3. A persistent object can carry a write across invocations, even a later statement in the same
   function. Test the lifetime of the **written slot**. A local capture of a module member can
   see a prior invocation's write to the module (esc21, wf10). An R3 field persists (wf43).
   Treat branches, loops, deferred bodies, captures, redirections and unknown order conservatively
   when the simple proof above cannot exclude reach. Record the uncertain edge. This veto alone
   cannot establish a parity exception.

| Slot and time of read | Required checks |
|---|---|
| Root path | Check each imported slot at import/capture or live lookup as appropriate. A class-qualified `C.m` root reads the class member directly. It never constructs an instance or enters a context (esc25). |
| R1/R2 | Check the chosen constructor binding, `__new__` and `__init__` at construction (esc06/30). Apply the instance lookup below to the method. A qualifying context adds its entry protocol and, when exit can affect the read, its exit protocol (as2-13, esc07/08). |
| R3 | Apply the name, field and method diagnostics, then withhold the field hint (§6.3). |
| R4/R5 | Carry object identity from the witnessed construction through the returned temporary and the caller binding. Apply instance lookup at the call and check the annotation at its defining site. For declaration-only standard-library receivers, check the declared class, receiver binding and selected lookup. Construction slots apply only to a witnessed construction. |

**Lookup slots.** For each attribute actually selected, use the lookup order of the pinned library
class. For class `C` and name `m`, the defining class is the first class in the unmodified
`C.__mro__` whose own namespace contains `m`. Every preceding class, including intermediate bases
in multiple inheritance, can acquire an override. Carry qualified class identities in the static
table's lookup metadata, resolving import aliases at the write. The lookup owner can differ from
the receiver in the hint. For example, `ConfigParser.read` searches through `RawConfigParser.read`,
and `PoolManager.request` reaches urllib3's `RequestMethods.request` on versions that inherit it.
Apply the same owner search to protocol methods. Bind the class and owner paths to the supported
library layout when specifying that metadata. The scanner does no target import or method execution.

| Slot or event | Same-file write that invalidates this lookup, subject to §6.2 ordering |
|---|---|
| Live module attribute or from-import capture | Check the module's `__class__` and member/key at each module hop, e.g. `requests.__class__ = M` before `requests.get`. A completed from-import severs later exporter replacements (wf27). |
| Receiver's effective class | `s.__class__ = Other` before lookup affects `s.m` even when the replacement is layout-compatible. |
| Receiver's namespace | Replacing the whole `s.__dict__` invalidates instance lookup. A literal key write in that dictionary or `vars(s)` affects the matching member. A write solely to a sibling key such as `cookies` leaves `send` available. |
| Instance lookup protocol | Check `__getattribute__` on the class lookup chain, including an inherited implementation (esc09). A class-qualified root `C.m` skips the instance and this instance protocol (esc25). |
| Selected class member | For each class `B` from `C` through the defining class inclusive, check `B.m`, including literal setters/deleters and admitted aliases. Overwriting `RawConfigParser.read` invalidates `ConfigParser().read`. Class-qualified reads also traverse the class-member chain. |
| Inheritance | Check `B.__bases__` for the classes whose ancestry supplies the search order, including `C.__bases__`. A reachable replacement can change lookup without writing `m`. |
| Synchronous entry and exit | Check the manager class's `__enter__` at entry. Check the captured `__exit__` when that exit can execute on the tracked object before the method read (e.g. after the suite). A replacement exit that installs `self.get = {}.get` can affect a post-suite `s.get`. |
| Asynchronous entry and exit | Use `__aenter__` and `__aexit__` with the same reach test on the matching async manager. A read before its first exit has no exit dependency. |

Apply the receiver class, namespace, member, lookup-protocol and inheritance checks to direct,
named, context-bound and helper-backed instances. A fresh unnamed R1 object has no prior named
instance store. Check class slots even then. Check matching context effects on a tracked alias
or returned local only when it participated in that context. For both protocols the exit callable
is selected at entry and its effects occur on exit. Order a write to that callable against its
capture, and the exit against the selected read, including any viable back-edge on the same object.
An ordinary construction has no context dependencies (esc10). For a plain module root use the
module checks. Use the existing literal and one-hop write forms below for all these slots.

All protocol checks use the same resolved write forms and one-hop aliases as selected methods. A matching protocol write gives `member_rebound`.
Writes on other members or protocols not used by this route do not veto it. These are local
inventory admission tests on explicit syntax. The repository-wide static-binding grades in
`CLAUDE.md` retain their meaning. No inventory hint certifies runtime dispatch.

| Read by the proof | Effect of a matching write |
|---|---|
| Deciding module name (including a literal `globals()["name"]` key), or the module's reserved `__builtins__` binding for bare fallback | Count a competing root binder as `shadowed_root`. A new binding for a named receiver participates in receiver uniqueness. |
| Imported member or prefix, at its read/capture point, the builtin member behind both `open` and `b.open`, or the class/constructor and selected class method behind R1–R5 | `member_rebound`, including an imported class alias writing `Class.get` while the read uses `requests.Session`. |
| R2/R4/R5 receiver in its deciding scope, or the exact-class R3 field | A competing write reaching the receiver name or R3 field gives `receiver_not_unique`. A write to its selected method gives `member_rebound`, including `self.client.get = v`, `del s.get` and literal setters. Compare owners across captures and redirections. |

Recognize attribute store/delete targets (including augmented stores and binding targets), unshadowed
builtin `setattr`/`delattr` with a literal member, and stores/deletes of a literal key in
`obj.__dict__` or unshadowed `vars(obj)`. For bare builtins, include module-owned or implicit
`__builtins__.X` and `__builtins__["X"]`. A shadowing function local of that name is a different
object. An unshadowed `globals()["root"]` used as the base of a member write identifies that
module-owned root. Use Python's lexical mangling for attribute tokens and decoded keys for strings.
Accept import aliases. For **write detection only**, also recognize a simple name's single assignment
`alias = object` where the RHS directly names a tracked module, class, receiver, R3 field or owning
`self` in the same file. Key the alias by lexical binding and the RHS by its own lookup. This one hop
handles as4-15 and wf05/36/41/45/53. It supplies no new positive call route.

The following mechanisms are outside the inventory write index. A surviving exact grade describes the modelled
binding. A source-disproved HTTP positive still counts as an error in G-precision.

1. Other-file writes, attached methods, subclass overrides and descriptor dispatch, including the
   supplied subclass of as1-20. R3 records a field tie and withholds its hint for every owner.
2. A write through another receiver whose identity would require argument flow, a second assignment
   alias hop, a container lookup or equality of runtime objects. This includes `other.client` in the
   owning class or its methods and `owner.session` in wf40. A qualifying alias of the actual `self`
   parameter is covered even inside a class body that captures it.
3. Computed keys or member names, a shadowed reflection helper, arbitrary mapping methods, and
   namespace mutations beyond the literal forms above (wf12, e07, tr3-20). Do not execute source.
4. Runtime import hooks, `exec`, changes to `sys.modules` and mutations of a helper's runtime
   `__annotations__` (as4-24). R5 reads the source declaration. The selected protocol list above
   bounds inventory checks. Arbitrary metaclass behavior, unlisted descriptors and effects inside
   called code have no general AST proof here. Record any evidenced mismatch at the product gates.

| Explicit-write decision | Alternative and consequence |
|---|---|
| Chosen | Compare the selected name, field, class and callable paths. wf01–53 cover root writes (01–12), builtin namespaces (13–25), from-import capture (26–28), constructor/class prefixes (29–34), receiver names, fields and methods (35–47) and class methods (48–53). The ledger identifies the deciding slot for each row. |
| Declined | Check only the imported spelling. Bare builtins and inferred methods can point at the replacement. |
| Declined | Veto every write on any similarly named receiver. Sibling members such as wf03 lose valid rows. Unknown receiver aliasing in wf40 motivates deferring field hints. |

| Repository-local lookup | Alternative and consequence |
|---|---|
| Chosen | Non-package site ancestors, repository root and conventional `src/`, with absolute builtin and pinned startup-module precedence. `repo/app/main.py` importing `requests` with `repo/src/requests/__init__.py` gives `repository_local_module`, even under `--files app/main.py`. No installation lookup or new flag. |
| Declined | Site ancestors alone miss that package. Treating every directory as an import root would let an unrelated nested fixture or vendor tree hide the external library. |
| Declined | Refuse every absolute import without a declared loader search path. The current interface has no such declaration, so ordinary external imports would lose their proof. |
| OWNER OPTION | Accept repeatable declared import roots for sibling or multiple nonstandard source trees. These layouts are outside the default guarantee. Where a corpus declares such a root, compare the actual origin before crediting any affected identity. Preserve even an erroneous emitted row in the raw inventory. A collision with the active local origin fails identity validation and goes to the owner. The default CLI cannot take the missing root. |

`saleor/settings.py` imports both `os` and `os.path`. Its Stage M ledger contains 122 tied shadowed calls, 121 module-level and
one in a function. The other shadow counts are Superset 15, httpie 1 and Black 1, all in functions,
with no repeat-import case identified. Coalescing the supported same-package form corrects the root
diagnosis even for module-level calls, whose next test is shape. These are census counts before the
change. Corpus recall under the amendment awaits measurement.

| Decision | Option and main cost | Choice |
|---|---|---|
| Census ownership | Reuse taint's private `Sites`. It totals spellings file-wide, treats declarations as flags, and would overlap the scheduled move-only extraction into `crates/prism-taint`. | Declined |
| Census ownership | Implement the scope index in targets. This costs a Python binding visitor. Reuse targets' import decomposition and local-module check, preserve the echo caller, and take no dependency on taint or its move sequence. | Chosen |
| Conditional class lookup | Refuse any use with a conditional class binding. It would lose the equal local/fallback identity in esc13, including the resulting class-frame diagnostic. | Declined |
| Conditional class lookup | Join the reachable class slot values and the fallback when absent. All outcomes must agree. esc11/12/26 retain the shadowing cause. | Chosen |
| Duplicate imports (Q12) | Count every occurrence independently. This loses the 122 indexed Saleor calls at identity. | Declined |
| Duplicate imports (Q12) | Coalesce the unconditional unaliased same-package form above (sl1-03, wf96–98). Preserve competing-branch and competing-origin refusals. | Chosen |
| Duplicate imports (Q12) | Equate conditional arms and imports under arbitrary aliases. This needs further availability and value-equivalence analysis. | Declined |
| Executable evidence | Accept any single textual import and ignore explicit writes to its members. This labels the later local import in sl3-02 and the replacement callable in as1-05 exact. | Declined |
| Executable evidence | Apply the bounded availability and write checks above. This can lose an earlier valid call in a file with a later mutation. General reaching-definitions analysis is outside the lane. | Chosen |
| Identifier comparison (Q9) | Use raw spelling for identity. This misses the actual call and shadow in sl1-13/14. | Declined |
| Identifier comparison (Q9) | Normalize identifiers and mangle lexical keys. This requires separate identity keys and source ranges. | Chosen |

### 6.3 Constructor routes R1 to R3

Resolve the constructor at its own expression and validate the selected class's method. Grouping
parentheses are transparent to identity. A mere `Session` spelling cannot prove a constructor.
An independent constructor proof can explain a library-looking receiver (sl2-21). A parameter declaration alone fails in as3-34.

- **R1** accepts exactly a call to the constructor as the method receiver, including imported aliases.
  Nested method results, `(x := C())`, `super()`, subscripts and context-entry calls do not qualify.
  The constructor call still competes for the line anchor (h10, sl2-16).
- **R2** accepts a single effective binding at the read (§6.2) of a simple name assigned `C(...)` by an assignment statement, including an
  annotated assignment or a direct target of chained assignment. Unpacking and walrus targets count
  as bindings but do not establish this route. The name can live in the using function, an enclosing
  function or the same file's module. A module receiver with a redirected second write fails uniqueness (as4-11).
  A `with C(...) as name` target qualifies for a synchronous self-returning context
  (`requests.Session`, `httpx.Client`, `urllib3.PoolManager`, `pathlib.Path` on 3.11).
  For `async with`, admit `httpx.AsyncClient` and `aiohttp.ClientSession`.
  Count context items separately, including in parenthesised/multi-item forms. The bound object's
  identity persists after the suite subject to the exit and lookup checks (as2-13). Receiver-only setup such as `s.mount(...)` and writes to unrelated members leave the selected R2 route available (§6.2). Wrapping a context in another call does not qualify.
- **R3** recognizes a diagnostic tie from a direct `self.attr = C(...)` store (also annotated with a value or a direct chained
  target). Require literal `self` bound to the instance parameter of the owning method, or a lexical capture of that parameter. Count writes
  in methods of that exact class definition, including a nested body that captures the same `self`.
  A nested class's own instance parameter belongs to that other class. Count plain, annotated-with-value,
  augmented, destructuring, loop/comprehension and with-target stores, deletes, and builtin
  `setattr(self, literal, …)` or `delattr(self, literal)`. Adjacent literal strings form one value.
  Count repeated method definitions within that class. An annotation without a value and a write to
  `self.attr.headers` leave the attribute binding intact (sl5-13, sl4-09).
  Apply the §6.2 write rule to this field and its selected method, including a one-hop alias of
  the owning self and literal instance-dictionary writes (as4-15, tr3-21). Owners and write
  forms beyond that rule have the numbered exclusions in §6.2. Positive R3 is deferred for the
  entire release, including adjacent store/use. On a tied field keep the first applicable §5.2
  cause, otherwise count `receiver_value_unproven`. This covers parameterised setters (e07,
  tr3-20), subclass-supplied values (as1-20, esc27) and a function receiving the owner (wf40).
  No caller enumeration or assumption that an instance has run this class’s initializer is required.

| Decision | Option and main cost | Choice |
|---|---|---|
| R1/R2 | Follow arbitrary factories, aliases and context-manager return values. This needs return-flow or protocol inference. | Declined |
| R1/R2 | Admit the direct constructor and single name/context bindings above. Wrapped or computed receiver results are lost. | Chosen |
| R3 | Count only assignment statements. This misses literal deletion in as2-18. | Declined |
| R3 | Model every possible attribute mutation. This needs alias and cross-file analysis. | Declined |
| R3 | Keep explicit same-owner field diagnostics. The consumer gets no positive field hint, including for h07. | Chosen |

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

- **R4** tests a positional (including positional-only), keyword-only or ordinary instance parameter,
  or a simple annotated assignment **with a value**, as the receiver name's sole
  binding. The assignment is function-local, or, for a listed standard-library class only, a same-file module name visible at the read (pin06, sl1-64). A bare module annotation stores no value. Varargs and kwargs bind collections. An unannotated default alone supplies no type.
  R2 wins when the assignment itself proves a constructor, even against a different annotation
  (tr5-03, sl5-11, wf60/62/65/68). Use that class's table. Otherwise consider R4 before R5,
  subject to the conflict, construction and declaration rules below. A second write defeats the sole binding
  even if it uses the annotated constructor (as5-14).
- **R5** tests a sole function-local `v = make(...)`, or `v = await make(...)` for an async helper.
  An annotated simple assignment offers its RHS to R5 if R4 has no admitted annotation (wf64/66).
  The callee is a bare lexical name resolving to exactly one module-scope function definition in the
  same file, including one under a module-level conditional. Its module binding must be unique, and no
  visible binding may hide it at the call. Unrelated same-spelling locals have no effect (tr4-03).
  A redirected global write, a shadowing parameter, a nested helper, a decorator, a generator or a
  mismatch between `await` and `async def` disqualifies it. A later module definition or a recursive call can resolve the helper name. The value test below applies. Qualified calls, inline helper receivers and a split call/await cannot.
- Test generator status in the helper's **own** Python compiler scope, including unreachable yield,
  `yield from`, a nested definition's evaluated default, a nested class base and a yield flag in
  a local annotation. A nested generator body and an async comprehension alone do not make the helper
  a generator (tr5-04, sl1-40, sl3-16/17, sl4-13). Read the source return annotation, including
  when code later mutates `__annotations__` (as4-24).

**Conflict test.** At a parameter, inspect the default in its defining environment. At a local,
inspect the initializer. A directly constructed listed class is a witness. For a bare call to a
visible, otherwise admissible same-file helper (with matching await), inspect its admitted return
class, if any, and direct `return C(...)` expressions in that helper's own body. For R5 itself,
compare its declaration with those same return expressions. Resolve each constructor at the return
site, including body-local imports (as2-24). Stop at that helper. Record a different proven listed
class as a declaration conflict, including one of several direct returns. Apply the precedence below before counting a warning. No body execution or second helper hop.
A literal or display known to construct a non-instance also contradicts a listed class (dict, list,
set or tuple display, string, bytes or numeric literal, boolean or None). For nullable syntax,
None is consistent but supplies no instance witness. An absent default, opaque value or unanalysed
call supplies no contradiction. For helpers, inspect returns only in their own compiler scope.

**Construction takes precedence.** R1/R2 keep their existing priority and grade. For an eligible
R4 local or R5 binding, an independently resolved listed class from the helper's sole final
construction witness below determines the receiver. Resolve the annotation at its declaration and
the constructor at its own expression. Check the selected method on the **constructed** class.
If listed, use its family and canonical path at `nameonly` (R4 for an eligible annotated local,
otherwise R5). Record a disagreement as declaration location/class and construction location/class
in the route evidence. No identity-unproven occurrence is counted for a proved call. When only the
contradicted declaration lists the method, withhold the hint and count `annotation_conflict`.
An off-table method of both classes gives plain `none` absent an independent tie.

| Former conflict rows | Constructed callee emitted at `nameonly` | Conflicting declaration in route evidence |
|---|---|---|
| sl2-22, as2-24, wf61, wf63, wf67, wf69, wf70, wf73 | `http, requests.Session.get` | `httpx.Client` (in as2-24 the helper's body-local import resolves the constructor to `requests.Session`) |
| as1-27 | `parse, configparser.ConfigParser.read` | `configparser.RawConfigParser` |
| wf72 | `parse, configparser.ConfigParser.read_file` | `configparser.RawConfigParser` |

The five overridable defaults (as2-19, wf54/55/58/59) and three known non-instance cases
(esc01/02/24, including an alternative dictionary return) still give `annotation_conflict`.
Without one independently witnessed construction, apply the existing conflict veto to all tied
annotation routes. R2 precedence, nullable syntax, visibility, helper eligibility and earlier
causes remain as above. P3 cannot excuse a miss for an independently proved listed method.

**Positive value test.** For HTTP and for a construction-backed standard-library result, R4
requires an eligible local assignment to an admissible annotated helper. R5 already selects that
helper. An overridable parameter default gives no positive witness. The helper must have exactly
one return in its own compiler scope, the final unconditional statement of its top-level suite.
It either returns the resolved constructor call directly or returns a simple local `v` subject to
all of these checks:

1. A single unconditional simple `v = C(...)` in that suite precedes the return. Resolve and
   check the same constructor and lookup slots as R2, carrying the object into the caller.
2. Apart from the binding and final return, each occurrence of this binding of `v` must be the
   base of a member read or receiver call (`v.attr`, `v.m(...)`), or a store to an unrelated
   member. Such occurrences may appear in a branch or loop. Inspect **every** argument, target and
   nested expression: `v.m(v)` still passes the object. A rebind, `del v`, alias `x = v`,
   argument or container element equal to `v`, or capture by a nested scope defeats this witness
   (`receiver_value_unproven` absent an earlier cause). Resolve bindings so a different local with
   the same spelling has no effect.
3. Apply §6.2 to recognized write forms before the escape diagnostic. A reachable write through
   the temporary to the selected method, lookup namespace or other required slot gives
   `member_rebound`, including a literal setter or dictionary write. A modeled call that mutates
   a required slot cannot use the receiver-call allowance to bypass the veto. A receiver call
   alone has the same treatment as in R2. Effects hidden in called code remain a limit of §6.2.

Prefixes may raise. An alternative return, fallthrough, second factory hop or opaque result gives
no construction proof. The public fixture pin01 and the source at `httpie/client.py:114` satisfy
the returned-local rule. The three `mount` receivers, including the loop receiver, read the same
binding. esc31–34 exercise receiver-only setup, passing the local, aliasing it and replacing `send`.

**Declaration-only standard-library receivers (K).** After checking stronger routes, use the
declared class and its listed method at `nameonly` for `filesystem`, `parse` and `config`. R4
accepts the sole annotated parameter or value-bearing simple assignment above, including a
visible same-file module binding. R5 accepts a sole function-local binding to the result of an
otherwise admissible annotated helper, including an opaque return (as1-28). Require the §6.2 binding, availability and
lookup checks. A known contradiction, failed returned-local escape or rebinding check, or
inadmissible helper still takes its §5.2 cause. Apply the ordinary shape and anchor rules to the
admitted occurrence. HTTP R4/R5 require the construction witness above. R3 remains diagnostic.

| Annotation evidence | r4 disposition |
|---|---|
| Matching helper return, retain `nameonly` | h09, e09c, tr2-57/58, tr3-26, tr4-03, tr5-04, sl1-38/39/41, sl3-17, as3-17/22, as4-24, as5-06, wf64/66 |
| HTTP parameter or opaque initializer | Withhold the hint for h08, e10b/c/d, tr1-12, sl1-35/36, sl2-25, sl4-11, as1-14/16, as2-20/21, as3-18/34. The hidden table-spelled root in as3-34 takes `shadowed_root`. |
| Standard-library declaration | tr3-24/25, sl1-64/65, as5-10 and pin03–06 yield R4 sites at `nameonly`. |
| Helper returns | as1-28 admits the opaque `Path` result at `nameonly`. as3-16 lacks the required HTTP construction witness. |
| Typed unsupported occurrences | sl4-14 admits `pathlib.Path.stat` at identity, then loses the anchor to `int`. as1-19 and as2-27 each retain one constructor occurrence and count the HTTP parameter separately. |

| Decision | Option and main cost | Choice |
|---|---|---|
| Annotation grammar | Accept bare classes alone. This loses tested nullable, aliased and forward forms. | Declined |
| Annotation grammar | Resolve arbitrary typing constructs. Containers and unions can describe multiple receiver identities. | Declined |
| Annotation grammar | Admit the closed single-class grammar above. Other wrappers produce no R4/R5 proof. | Chosen |
| Helper visibility (Q3) | Reserve the spelling throughout the file. An unrelated local in tr4-03 would revoke a real module binding. | Declined |
| Helper visibility (Q3) | Require module uniqueness plus call-site visibility, then check the returned value. The census alone cannot certify the httpie helper. | Chosen |
| Helper reach | Follow bodies, imported factories or multiple return hops. This requires interprocedural inference. | Declined |
| Helper reach | Use one bare call to a visible annotated helper. For HTTP require the construction witness. For an eligible standard-library helper the declaration can supply the canonical class at `nameonly` (as1-28). | Chosen |

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
   different, off-table path, the call supplies no root tie regardless of the import's availability
   (sl4-05, as1-25, esc14). A later local `import urllib3 as requests` therefore gives plain `none`
   at `requests.get` if no independent receiver tie applies. Apply this same eligibility test to a constructor or annotation **before** using its spelling for any receiver tie. With that import, `requests.Session().get(u)` resolves the constructor path to off-table `urllib3.Session` and gives plain `none`. A sole later remap is equally silent. An independent tie, a genuine unrelated binder, or a listed remapped constructor still follows the ordinary rules.
2. For a listed method, an inline R1 receiver `C(...).method` is tied by the constructor spelling
   or a visible explicit constructor import, even when root proof fails. Propagate its §5.2 cause
   to the method. The constructor alone supplies no site. For a named receiver use the binding
   positions in §§6.3–6.4, including an R4 declared parameter or value-bearing local assignment, or a visible same-file module assignment of that form for a standard-library class.
   Require a listed method and a constructor, annotation or helper tie. Waive uniqueness and defer
   the positive value test to proof; neither is required to establish the diagnostic tie. For R3 require a qualifying literal construction store on the owning
   self field, again allowing competing writes for diagnosis. Check candidate class/method membership
   and then propagate the constructor or annotation root failure (Q5). A context must have the
   admitted constructor and sync/async pairing even to tie: `async with Path(...) as p` alone gives
   plain `none` at `p.read_text()`. A generic `make().get`, `dict.get` or Session's `close` gives no tie.
3. In an eligible R5 assignment position a bare helper needs a supported class spelling in its return
   annotation or a direct `return C(...)` in its own body with a listed constructor identity. Inspect
   same-file definitions on the bare name's lookup chain, including a module definition hidden at the
   call. The helper may fail the definition, decorator, generator, await or return-annotation checks.
   Disregard unrelated locals and nested bodies. The return witness explains n07, tr4-01/02, as1-17
   and as2-23. A helper returning `{}` (sl1-43) supplies no tie. A visible helper in an unpacking,
   with-item, parameter default, module or class assignment supplies no R5 tie.

For wf74 an annotation-only local declares a local name and stores no value. It has no R4 assignment
with a value, so the call is silent even though Python would raise on lookup. Constructor/helper
spellings inside rejected positions do not propagate a root or helper cause (wf81/84/95). A module
constructor assignment is eligible for R2 (wf92), a module helper assignment is outside R5 (wf93).
The standard-library module annotation in sl1-64 supplies the §6.4 R4 `pathlib.Path.exists` site at `nameonly`. The position restrictions cover as2-26, sl5-10 and the tuple, walrus and loop cases in wf75–91.

| Warning tie decision | Alternative and consequence |
|---|---|
| Chosen | Count failed inline R1 and eligible binding positions, retaining root causes and waiving uniqueness for diagnostics. A wrong context protocol or assignment position is silent. |
| Declined | Count a constructor or helper anywhere inside a binding. This ties the default in sl5-10 and the class-body helper in wf95 to a receiver with no admissible proof or cause. |

| Deciding import and cause | Disposition |
|---|---|
| Chosen | A sole tied relative import goes to `repository_local_module` after any wildcard veto (o09, as2-28, as3-25). It cannot itself satisfy the unrelated-import `shadowed_root` predicate. Multiple competing binders remain subject to their own earlier cause. |
| Chosen | A sole absolute table-library import that maps the call to a listed path supplies a candidate of **that** library. Hiding an outer import is normal lookup. Failure of its availability gives `binding_unavailable` (sl3-02), after any earlier applicable wildcard or repository-local veto. An off-table path under a table library supplies no root tie even before the import is available. Independent receiver routes still run. |
| Declined | Count any hidden outer binding as shadowing. That misdiagnoses the later local import in sl3-02. A deciding unrelated binder or multiple non-equivalent binders still gives `shadowed_root` on a table spelling (as1-18). |

On ties use the **5.2 table order**, after attempting applicable proofs. Thus sl2-21 admits
`requests` as a constructor-typed receiver, while as1-18 takes `shadowed_root` over
`receiver_not_unique`. A non-import value binding, including `requests = import_module(...)`,
is `shadowed_root`. An absent binding is `not_imported`, even if a file of that name exists (as2-04).
A star import wins over both. An extra receiver binding wins over helper faults (tr4-01). Missing
admissible return annotation wins over a decorator on the same tied helper (tr4-02). Identity failure
wins over a frame or parse failure (as2-09). The unsupported reasons retain the §5.2 order.
For a tied annotation conflict, competing receiver writes take precedence, followed by
`annotation_conflict`, the two helper causes, then `receiver_value_unproven`.

Grouping a proven unsupported call uses the 5.2 key, including the **bare symbol string**, independently
of grade and source alias. Keep each occurrence's grade and file parse quality through confidence
and tier filtering. Group surviving occurrences, recompute their count and use the highest surviving
confidence and its corresponding §5.1 tier for the group. Drop empty groups. esc22 contributes two
without a filter and one exact occurrence under `--min-confidence exact`. In as1-19 and as2-27
one constructor occurrence survives both settings. Each fixture also counts one `receiver_value_unproven`
for its parameter, independent of confidence filtering. Apply severity to these
rows and count removed groups once. Diagnostic warning counts for unproven identity have no grade.
Two calls in different functions with the same bare symbol can therefore group
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
| Diagnostic vocabulary | Keep six unsupported reasons. T-1 and the producer use the eleven ordered §5.2 causes, ending in `receiver_value_unproven`. | Chosen |
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

For site eligibility, omit calls in the annotation of **any annotated assignment in a function's
own scope**, including name, attribute and subscript targets. Thus `self.x: requests.get(u) = None`
in a method contributes plain `none`. Also omit calls in annotations postponed by
`from __future__ import annotations`, including signature and class annotations. Still visit the
evaluated default, RHS and evaluated parts of assignment targets (sl1-27, as4-22). Static
type spellings remain available to R4/R5. For the **binder's candidate set** apply §5.4 to every
function-scope `AnnAssign.annotation`, whether the target is a name, attribute or subscript.
If any other candidate covers the line, remove the annotation's calls, leaving evaluated target
operands and RHS calls in competition. Otherwise keep the annotation candidates for binding;
none can emit an executable site. In esc15/16 only `requests.post` on the RHS can win. Elsewhere use
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

The K ledger has 338 site IDs. The remaining IDs form the per-ID loss register. Including the proposed fixtures, 91 unsupported expectations represent groups, with
multiplicity in `want`. Ten lack a function body, 15 are in a class frame, nine in a lambda frame,
five have a multiline callee, 44 lose the line anchor, and eight have an error in the owning definition.
Keep their diagnostics. The frame, competing occurrence and parse-error cases lack a usable
frame, line or bounds. §6.8 records the two bindable multiline exceptions for the owner. Runtime
refusals of otherwise representable rows belong in the funnel.

The 253 counted-omission verdicts comprise 9 `star_import`, 67 `shadowed_root`, 11 `not_imported`,
7 `repository_local_module`, 2 `binding_unavailable`, 47 `member_rebound`, 46 `receiver_not_unique`,
8 `annotation_conflict`, 5 `helper_unannotated`, 14 `helper_not_admissible` and 37 `receiver_value_unproven`.
These are IDs. The mixed groups in §6.5 also have omitted occurrences. Each row's cause identifies the missing evidence
in 5.2. Proving branch-specific values, trusting local shims or inferring undeclared returns would
admit sites outside the chosen identity proof.

The 90 plain `none` decisions are accounted for below. Some are negative controls, where there is
no executable table call to recover.

| Reason for plain `none` | Row IDs |
|---|---|
| No explicit call to the dependency. Subscripts, patterns, decorators applied implicitly, callables passed to higher-order functions, comments, literal text and non-executing annotation subtrees contribute no site. | f02, tr1-14, tr1-20, tr3-12, tr3-13, sl1-09, sl1-62, sl1-70, sl4-06, as1-29, as3-10, as4-21 |
| Constructor alone or a qualified member outside the closed family/class table. Extending the table or equating re-exports changes the published family boundary. | n01, n02, tr2-74, sl1-44, sl4-05, as1-25, as1-34, as1-35, as2-11, as3-05, esc14 |
| No visible tie at the use. The spelling belongs to an unknown parameter, wildcard member, unrelated helper, comprehension local or implicit class binding. Guessing from `get` would include dictionary calls. | n03, n08, sl1-43, sl3-12, as1-02, as2-15, as5-13 |
| A return, factory, descriptor, class attribute/default, different-file helper, attribute or callable alias, wrapper, operator, reflection, or intermediate result would have to be followed. R1–R5 stop at their stated expression or single binding. | sl1-42, sl5-09, sl5-10, sl5-12, sl5-14, as1-06, as1-11, as1-23, as1-33, as1-39, as2-14, as2-17, as2-25, as2-29, as3-03, as3-04, as3-21, as3-23, as3-26, as4-12, as4-13, as4-18, as5-07, as5-08, as5-11 |
| The annotation is on a different target, is outside an eligible R4 binding, or would require narrowing, alias expansion, cast, comment, subclass, container-element or wrapper interpretation. No unique receiver type is admitted by 6.4. | sl1-37, sl2-24, sl3-18, sl4-12, as1-12, as1-13, as1-15, as2-26, as3-14, as3-19, as4-16, as4-17 |
| Focused binding-position negatives (§6.5). An excluded annotation wrapper, valueless binding, unpacking, walrus, context, class or other inadmissible owner/target supplies no receiver tie. | wf56/57, wf74–91, wf93–95 |

**G-parity has a narrower, closed list.** Apply it only to a standard-library injectable key present
in scan and missing in prism on **scan's entire `diff.files`**, including files prism omitted.
Independently replay root identity, bare fallback, every applicable receiver route and the shape
rule on the same source occurrence. A positive injectable proof disqualifies **every** loss class.
Record the attempted routes, shape, key, source witness, class below and
prism outcome for each miss. A file-total warning alone cannot locate a miss. Compute the 95% floor
for each corpus as matched keys divided by **all** injectable scan keys. Every missed key stays in
the denominator. A missing key admitted by K remains unresolved.

| Class | Required witness |
|---|---|
| P1. Competing or unavailable binding | Demonstrate an actual wildcard, competing deciding-scope occurrence, explicit namespace/member mutation, unavailable binding or non-unique receiver as specified by 6.2–6.5. Name the precise cause and exclude a successful alternative (e.g. R2 in sl2-21). For a write, record the binding owner, object/slot, capture/lookup point and a feasible execution order from write to read (including a prior invocation for persistent state). A file-wide hit, unknown order alone, wf27's severed exporter link or wf38's later local delete cannot earn P1. For a competing binder, require that it can reach this read with a different value. Exclude a parameter killed by a dominating construction (as5-14), a dead write over an empty literal iterable (tr3-31), and a join where all reaching imports have the same available origin (sl3-01). If a standard-library scan miss has one of these shapes, it remains unresolved and counts against the per-corpus floor. A producer refusal alone earns no loss classification. |
| P2. Relative or repository-local identity | Show a relative import, or identify the competing file/package and show its name is importable first under the corpus's recorded active roots and applicable precedence. A file under an inactive `src/` or a regular package sibling alone earns no P2. Apply §6.2 precedence for absolute `builtins`, `os`, `os.path` and `io` on the pinned startup profile. This includes genuine local shims re-exporting a stdlib function. |
| P3. Indirect receiver or callable | Show the excluded expression or binding at this occurrence. Eligible witnesses are an assigned callable/module/object alias, constructor in an inadmissible binding position, wrapper or method/operator result requiring inference, R3 field, descriptor/container/subclass receiver, dynamic import/member lookup, indirect or cross-file helper, same-file helper failing §6.4 eligibility, or an excluded type form. Exclude all admitted routes, including R1/R2, the independently constructed class and K's eligible parameter, value-bearing assignment or helper declaration. The operator in pin05's initializer leaves its R4 proof intact. |
| P4. Unrepresentable call or frame | Demonstrate an inner class or lambda body frame and record its owner. Scan already skips non-anchors. A different winner or a representable multiline scan key gets a matrix regression and fix, or owner escalation under §6.8. Neither earns P4. |
| P5. Unevaluated annotation | Show that the recorded call occurs exclusively in an unevaluated annotation under 6.6 on 3.11. |

A missing table member, missed ordinary import, wrong bounds/symbol/line, wrong canonical hint,
failure to parse valid supported Python, missing scan run or unexplained difference fails the gate.
`module_level` pairs with scan's bounds-less set. A genuine syntax error is reported with the scan's
file/parse evidence and grants no automatic exception for a missing injectable key. The P classes do
not excuse a false positive or lower the numerical floor. An unclassified miss goes to ORCHESTRATION
section 8.

| Decision | Option and main cost | Choice |
|---|---|---|
| P1 ordering | Exclude killed, dead and same-origin competitors from P1. This preserves the pinned producer refusals and makes a scan miss visible for resolution. Extending production dominance and branch equivalence would change as5-14, tr3-31 and sl3-01 beyond the parity fix. | Chosen |
| Accepted loss | Infer values across files. This requires a new interprocedural proof. | Declined |
| Accepted loss | Keep the finite limits and publish every tied failure, unsupported group and plain negative above. Real indirect dependency calls can be missed. | Chosen |
| G-parity | Allow any unsupported or warning bucket. This could conceal a systematic defect. | Declined |
| G-parity | Pool the recall across corpora. A large corpus could mask a failing smaller corpus. | Declined |
| G-parity | Require P1–P5 with a site witness and retain each corpus's floor. New loss mechanisms need design escalation. | Chosen |

### 6.8 Oracle reconciliation and owner questions

Join on **(fixture, line)** and check the ID in the oracle's `rows`. Several IDs may share one binder
answer. The supplied oracle covers 722 distinct fixture lines for the 738 materialised IDs. This
join uses their K wants. The 34 proposed IDs lack observations and are excluded from this table. “Syntax” denotes an error record without a bound call or guard answer.

| Want | accepted | unsupported_frame_guard | unsupported_multiline_callee | call_position_unavailable | site_call_unavailable | Syntax | IDs |
|---|---:|---:|---:|---:|---:|---:|---:|
| site | 310 | 13 | 0 | 0 | 0 | 3 | 326 |
| unsupported | 43 | 35 | 2 | 1 | 0 | 8 | 89 |
| none(unproven) | 221 | 12 | 0 | 0 | 0 | 1 | 234 |
| none | 73 | 6 | 0 | 4 | 6 | 0 | 89 |
| Total | 647 | 66 | 2 | 5 | 6 | 12 | 738 |

The pre-fold 3.11.11 AST comparison reproduced the recorded bound callee text or absence on the
603 parseable lines in the original 615-line packet. The expanded join adds 107 parseable lines, including the eight new pin lines. Each of those eight has a supplied `accepted` guard answer. Only pin02 has a nonempty qualifier.
The oracle supplies texts and bounds but lacks occurrence spans. Unique source text can identify an
occurrence in a fixture. Duplicate text by itself cannot certify G-accept (u04a/b).

- Of the 326 site IDs, 323 have a binder/guard record and three have a whole-file syntax error.
  Of the 13 site frame refusals, s03, c06, o20 and tr1-23 are comprehensions/generator expressions. s04, b04, tr1-24, sl5-08 and
  as4-01/02 have decorated or repeated definitions. c02, tr3-06 and tr3-07 have a nested header
  or lambda default evaluated in the surrounding scope. Keep the rows and tabulate the refusal.
- Of the 323, 185 have the same nonempty qualified name as the hint. All 125 parseable receiver-route
  sites have an empty qualifier. Compare occurrence spans in G-accept and record that field beside the
  constructed or declared class path. The other 13 empty qualifiers require the source binding.
  The five bare builtin sites are w06b, f01a, as1-31, as4-19 and as5-15. The eight imported roots
  are tr2-53, tr2-54, tr3-02, tr5-02, sl1-01, sl1-04, sl2-02 and sl2-04 (declarations, redirected
  imports, comprehension scope or private-name lookup). Preserve Python's identity. A blank is
  missing qualification evidence. For a census import-root row G-accept needs the consumer's
  nonempty matching qualified name. For a bare builtin it needs independently recorded canonical
  `builtins.open` from the consumer, including a planner derivation if available. Missing or
  conflicting evidence leaves that check unmet.

  Preflight from the census site ledgers (httpie, Saleor, Superset, Black order): bare `open` has
  31, 7, 58, 71 candidates. HTTP import roots have 6, 2, 12, 0. No import-root callee text starts
  with a private `__` spelling. The recorded comprehension shape has 45 import roots in Superset,
  all stdlib. A prior comprehension can affect a site outside its body, and these records omit lexical
  binders. Before S2a measurement the runner must scan the four source corpora for **all eight fixture
  patterns**: direct global/nonlocal declarations (tr2-53/54), a separate comprehension binder or
  first iterable (tr3-02/tr5-02), an intervening global declaration or redirected import (sl1-01/04),
  a method skipping a class namespace (sl2-02) and private-name mangling (sl2-04). Also enumerate
  bare builtins. Record per-corpus file, line, span, pattern and counts including zero, then run the
  qualified binder on candidates. For an empty root qualifier or a builtin without consumer canonical
  evidence, retain the source-derived want, mark condition (2) unmet at the applicable family gate,
  and take the receipts to the owner under ORCHESTRATION §8. Changing the gate or the consumer
  requires an owner decision.
- o23a, sl1-24 and as4-09 have a clean innermost definition in a file the consumer cannot parse.
  Keep the sites. The input `consumer` label `unparseable_file` summarises the syntax error.
  The other nine syntax records are o23b, tr3-08/09, sl1-25, sl2-14, as1-10, as3-11 and as4-08
  (unsupported), and as2-09 (identity fails first). None provides a consumer verdict on the intended
  call. A recovered site on a census file still needs executed G-accept evidence.
- For unsupported rows an `accepted` answer describes the one **anchor** on the line. The 38 accepted
  `not_line_anchor` IDs target another occurrence, even if its text matches (u04b). In sl4-14 the
  binder selects `int` while the diagnostic selects `pathlib.Path.stat`. In the five `multiline_callee`
  cases, u05 and tr1-22 actually bind the multiline target and the recorded guard accepts it.
  tr3-11, sl1-28 and as2-27 instead bind `consume`, `Session` or `Client`. Keep the 5.2 reason
  and group count in all five. For sl1-16 and sl2-11 the **winner** gets
  `unsupported_multiline_callee`. The target loses the anchor. In as3-08 the winner is `ignored`
  and the guard has no call position. The 35 frame refusals likewise describe the winner. Select
  unsupported reasons per intended occurrence before grouping, including when two rows share a line.
- For each plain or counted `none`, binding a source expression or qualifying its text supplies no
  evidence of table membership or of runtime library identity. Use that ID's proof or loss in 5.2/6.7
  for every guard cell. In particular, preserve the omissions even where `qualified` names a table
  entry despite a local module, shadow, wildcard or write. A position/frame refusal cannot turn an
  identity failure into an unsupported site. The four no-position plain negatives are unevaluated
  annotations. The six no-call negatives have no call at the line. The remaining frame refusals and
  accepted negatives follow the same per-ID rule.
- Against the pinned matrix, 83 design wants and eleven Python-semantics wants change. All 24
  faultpath-guard wants stand. The ledger distinguishes the 34 pending fixtures from the 94 changes
  to existing IDs. Promoted `*` selectors take the callee from the site want. Consumer evidence for
  the additions requires a qualified oracle run.

| Gate evidence decision | Chosen | Alternative and consequence |
|---|---|---|
| Occurrence identity (G-accept) | On identical source bytes compare the actual Prism discovery with the actual consumer-bound call. Record file, four-coordinate `Call.func` and whole-`Call` spans (start/end lines and UTF-8 byte columns, §6.6), and source text from each. Require equality. A test-only sidecar carries the producer-selected route (`root`, `R1`…`R5`) and both discovery spans before projection. Join by revision, file, row ID and occurrence. Replay the ordered source rules independently on that occurrence. Require one matching route as well as matching spans. Missing, duplicate or disagreeing route evidence fails G-accept (2) and leaves G-precision stratification unmet. Missing or unequal spans fail condition (2). | Matching only text, line, qualification or guard can confuse the two calls in `[s.get(x) for s in items], s.get(y)`. Reconstructing both spans from one winner repeats the same error. |
| Unsupported group matching (T-1) | Match full group keys, symbol, reason and count. Consume each actual row at most once. | Independent existence checks allow either sl2-12 or sl2-13 to satisfy both. |
| Parse quality (S2a) | Carry file parse quality into both projections. Recovered sites with clean owning subtrees retain the file's quality and candidate tier. | Inferring quality from a clean owner would mark o23a, sl1-24 and as4-09 asserted. |
| Unsupported filters | Filter occurrences then group (§6.5). | Filtering a mixed group as a unit would drop or retain both calls for esc22 under an exact floor. |
| Parity | Require independent proof and shape replay. Keep omitted files in scan's denominator (§6.7). | A failed spelling or the label “constructor” alone would excuse admitted R2 or R1 calls. |
| Ledger custody | Use the sibling ledger and three digests in §6. | An external unversioned pointer leaves the design and test seats without a common input. |

| Decision | Option and main cost | Choice |
|---|---|---|
| Guard disagreement | Make every guard refusal an unsupported site and every acceptance an injectable site. This drops representable frames and admits calls with no proven family. | Declined |
| Guard disagreement | Preserve 5.2/5.4 and compare source binding, qualification, guard and planning separately. Some real sites still lack consumer evidence for G-accept. | Chosen |
| Multiline attribute callee | Allow a multiline attribute callee at its first line when the consumer binds that exact occurrence and frame. u05 and tr1-22 are accepted with `requests.get`. This changes the fixed §5.4 rule and their wants. | OWNER OPTION |
| Multiline attribute callee | Keep the single-line restriction as an explicit product limit independent of bindability. This requires owner reconciliation of the §5.2 principle. Pending that decision, the two `multiline_callee` verdicts stand. Stage M found zero multiline sites across four corpora. A scan miss here cannot earn P4. | OWNER OPTION |
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

#### Escalation decision after round 2 (historical, rejected at d3)

At `ffcbe8fa`, the escalation amendment required a
bounded value witness on an annotation route. `CLAUDE.md` defines Exact by static binding and
places runtime module mutation outside every language's grade. The §6.2 same-file veto affects
inventory admission only. The public grades and both overlay requirements stand. The owner
decides any change to either contract.

| Pooled alternatives | Decision for this release | Cost or boundary |
|---|---|---|
| A / O1, retain all routes and disclose runtime exceptions | Decline admission on declaration alone. Retain O1 disclosure for residual cases. | Disclosure leaves a source-disproved dictionary hint eligible for planning. |
| B + partial O3, positive construction witness and defer field inference | **Choose.** R1/R2 keep their grades, R3 is diagnostic only, R4/R5 need §6.4. | This loses h07 and even adjacent field uses, the Superset HTTP parameter and opaque helpers. A fresh, unescaped owner proof could recover some R3 cases but requires tracking the owner at the use. |
| C / O2, bounded contradiction checks on otherwise static routes | **Choose** literal conflicts, selected protocol slots and existing field-write diagnostics. Decline caller/setter/subclass enumeration as a positive proof. | An unsampled caller can replace a parameter; an inherited method can read a different field. Adding another setter pattern cannot close that class. |
| O3, drop all annotation routes as well | Decline. | Would also lose h09 and other helpers returning the matching constructor. |
| O4, grade R3/R4/R5 all `nameonly` | **OWNER OPTION**, default to admission above and existing grades. | A lower grade still exports an incorrect callee to an unfiltered consumer. |
| D / O5, separate declaration-only candidates, enforce receiver types or redefine receiver grades/G-overlay | **OWNER OPTION**, default to no candidate hint without the witness. Preserve schema 1.1, the grade contract and G-overlay. | Requires an owner decision on representation, consumers and gate credit. A measured overlay failure stops S2b for that decision. |
| E / O7, order writes per slot and capture | **Choose** §6.2. | Needs binding owner, capture point, read phase and object lifetime; unknown order can still cost a site. |
| F / O6, blanket veto with separate loss reporting | Decline. | Loses wf27, n06 and wf38. Counting such losses in parity would require an owner exception. |
| G, reaching definitions and cross-call summaries | Decline. | Branch joins, aliases, recursion and unknown calls exceed this AST visitor. P1 still requires a feasible witness. |
| H / modified O8, conventional roots and package/builtin precedence | **Choose** §6.2, with active-root evidence for P2. | Inactive `src/` may suppress a real call. A sibling root outside the default lookup may produce a wrong hint. Neither earns gate credit from a filename. |
| I / O9, ordered declared roots including multiple launch contexts | **OWNER OPTION**, default to no new CLI input. | Needs a contract for root order, unknown roots and disagreeing entry points. Record all affected keys and withhold gate credit until resolved. |
| J / O10, search every directory, or require a declared origin universally | Decline both. | Unrelated local fixtures would suppress genuine calls; requiring a launch declaration would withhold ordinary imports under the current interface. |

For S2b publish the per-route positives and tied losses, including deferred R3, exposed parameter
defaults, opaque returns, and evidenced exceptions to the write model (e.g. wf12 if its parameter
is the real builtin). Record the occurrence, source witness and relevant exclusion. An observed
wrong hint counts under G-precision regardless of grade or exclusion. No rule assumes that an
absence of indexed writes proves runtime immutability. G-accept tests binding to the intended
occurrence independently.

A missing oracle run or `unsupported_interpreter` / `unqualified_runtime` is an inadmissible probe.
Keep the rule-derived want and mark consumer evidence pending until a qualified run is available.
This matrix has no planner, scan, live injection or corpus gate receipt.

### 6.9 Pinned-row results

Each row checks one r4 rule with the full K design applied to the eight pinned occurrences.
`E` and `N` are sites at `exact` and `nameonly`. `V` is `none(unproven:receiver_value_unproven)`
and `M` is `none(unproven:member_rebound)`. The HTTP columns select `requests.Session.send`
and `requests.get`. The six filesystem columns select `pathlib.Path.read_text`, `exists`, `mkdir`,
`touch`, `read_text` and `is_file`, respectively. Pin04 also selects `write_text` at `nameonly`
on line 6. Pin08 also selects `exists` at `exact` on line 9.

| Rule | pin01 | pin02 | pin03 | pin04 | pin05 | pin06 | pin07 | pin08 | Superset shape check |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| H · helper setup | N | E | N | N | N | N | E | E | H applies to the annotated helper. The parameter and operator-initialized Path shapes in pin03–06 need only R4. The census helper tie needs an eligible local binding for R5. |
| L · lookup slots | N | E | N | N | N | N | E | E | L checks the receiver, class and selected method. The local and module initializers in pin05/06 have no write to the selected lookup. Pin07/08 also pass their constructor-slot checks. |
| C · construction | N | E | N | N | N | N | E | E | C prefers a construction when present. Pin07/08 establish it directly or by a local assignment. The operators in pin05/06 supply no independent construction of the receiver. |
| P · P1 evidence | N | E | N | N | N | N | E | E | P requires a feasible different reaching value for a competing binder. The two census-unproven Path parameter records need that witness for P1 on any scan miss. A sole admissible Path declaration gives a positive route. |
| T · remapped ties | N | E | N | N | N | N | E | E | T resolves the imported class before testing membership. All six Path pins import pathlib.Path without an off-table remap. |
| J · class lookup | N | E | N | N | N | N | E | E | J checks class-scope fallback where it applies. The Path pins read a parameter, local, module name or direct constructor in a function. None depends on a conditional class slot. |
| F · startup precedence | N | E | N | N | N | N | E | E | F checks startup precedence for builtins, os, os.path and io. The Superset Path shapes use ordinary pathlib import resolution. |
| K · keep | N | E | N | N | N | N | E | E | K gives the parameter, local and module shapes R4. A same-file helper declaration can supply R5 under §6.4. Direct and locally bound constructors keep their exact grades. |

| Rule | esc31 | esc32 | esc33 | esc34 |
| --- | --- | --- | --- | --- |
| H · helper setup | N | V | V | M |
| L · lookup slots | N | V | V | M |
| C · construction | N | V | V | M |
| P · P1 evidence | N | V | V | M |
| T · remapped ties | N | V | V | M |
| J · class lookup | N | V | V | M |
| F · startup precedence | N | V | V | M |
| K · keep | N | V | V | M |

| Rule | Discriminating controls and result |
|---|---|
| H | pin01 and esc31 produce the overlay callee. esc23 keeps the simple fresh-local case. esc32/33 withhold for argument/alias escape and esc34 counts the selected-member write. The former blanket ban on setup fails pin01. Unrestricted uses fail esc32–34. Both are declined. |
| L | wf47 rejects the selected dictionary key. wf03 keeps a sibling write. esc06–09/20/30 withhold on used protocols or methods, esc10/25/28 keep their exact results, and as2-13 keeps the clean post-exit site. The new slot examples and before/after-exit controls are specified in §6.2. No existing want changes for L. |
| C | Exactly ten conflict IDs move to the constructed canonical callee in §6.4. The eight other conflict IDs retain their cause. No R3 field gains a hint and neither declaration relabels a proved R2. |
| P | as5-14 and tr3-31 keep `receiver_not_unique`, sl3-01 keeps `shadowed_root`. All three lose P1 eligibility for their source-proven misses. Extending production ordering to promote them is declined. |
| T | Under `import urllib3 as requests`, the remapped `Session` constructor cannot originate a tie. esc14 stays silent, sl3-02 keeps `binding_unavailable` on its on-table remap, and sl2-21 retains its independent R2. |
| J | esc13 keeps `unsupported(class_body_frame,1)` because the conditional import and fallback agree. esc11/12/26 keep `shadowed_root`. Blanket refusal fails esc13 and is declined. |
| F | o15/16/18, sl2-26, wf96 and esc18 keep exact identities. as2-28/as3-25 retain their relative-import cause. The d3 startup-precedence finding identifies zero affected census sites. |
| K | Pin03–06 and sl1-64 admit the declared `Path` at `nameonly`. sl4-14 admits the declared method but loses its anchor. §6.11 gives the exact wants and selectors. |

The Superset census supplies the corresponding source locations. Parameter examples include
`config_path.read_text` (scripts/extract_feature_flags.py:60) and `pot_path.write_text`
(scripts/translations/apply_do_not_translate.py:91). The local and module examples include
`frontend_dist.mkdir` (superset-extensions-cli/tests/test_cli_build.py:205) and `RELOAD_TRIGGER.touch`
(superset/extensions/local_extensions_watcher.py:141). Constructor controls include the direct
`Path(...).read_text` in tests/unit_tests/mcp_service/chart/test_gantt_chart.py:101 and the
local `path_object.is_file` in superset/cli/importexport.py:335. The single helper-tied key is
`examples_dir.exists` in superset/examples/data_loading.py:120. The census records the tie and
shape. §6.10 states the source assumptions needed to project these keys onto the scan denominator.

The declared-root layouts and the bindable multiline winners u05/tr1-22 remain owner options
in §§6.2 and 6.8. Their operational rules have no r4 delta. The status correction has no
producer or parity predicate.

<details>
<summary>Per-rule results for all 760 inherited IDs, including every R3/R4/R5, write and binding control</summary>

Each column is one ID. The reference is its exact `want` at `ffcbe8fa` (ledger SHA-256
`25d5ba510c6ec58859997756e317547badee15e10d0cc796ffdeaddb8c93871d`). The reference row
shows its kind. `U` means unsupported, `Ø` means silent, `A` means annotation conflict and
`D` means another counted cause. In each rule row, `=` preserves the full reference want,
including callee, grade, cause, multiplicity, symbol and bounds. Each differing cell gives the
kind of that rule's replacement. The sibling ledger gives its full want. These changes compose
without overlap. Rule letters sharing a row have the same result for every ID in the panel.
P1 classifies scan differences and has no producer-want delta.

| Rule | h01 | h02 | h03 | h04 | h05 | h06 | h07 | h08 | h09 | h10 | h11 | n01 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | E | E | E | V | V | N | E | E | Ø |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | n02 | n03 | n04 | n05 | n06 | n07 | n08 | u01 | u02 | u03 | u04a | u04b |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | Ø | Ø | D | D | E | D | Ø | U | U | E | E | U |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | u05 | u06 | u07 | w01 | w02 | w03 | w04 | w05 | w06a | w06b | w07 | w08 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | U | U | E | U | U | U | U | U | U | E | U | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | s01 | s02 | s03 | s04 | s05 | s06 | s07 | b01 | b02 | b03 | b04 | c01 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | E | E | E | E | E | E | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | c02 | c03 | c04 | c05 | c06 | f01a | f01b | f01c | f01d | f01e | f02 | f03 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | U | U | E | E | E | E | E | E | E | Ø | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | f04a | f04b | e01 | e02 | e03 | e04a | e04b | e05a | e05b | e05c | e05d | e05e |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | U | D | D | D | D | D | D | D | D | D | D |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | e06 | e07 | e08a | e08b | e08c | e09a | e09b | e09c | e09d | e10a | e10b | e10c |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | V | D | D | D | D | D | N | D | D | V | V |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | e10d | o01 | o02 | o03 | o04 | o05 | o06 | o07 | o08 | o09 | o10 | o11 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | V | E | E | E | E | E | E | E | E | D | D | D |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | o12 | o13 | o14 | o15 | o16 | o17 | o18 | o19 | o20 | o21 | o22 | o23a |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | U | E | E | E | E | E | D | E | U | U | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | o23b | o24a | o24b | o25 | o26 | tr1-01 | tr1-02 | tr1-03 | tr1-04 | tr1-06 | tr1-07 | tr1-08 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | U | E | E | U | D | D | D | D | D | D | D | D |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr1-09 | tr1-10 | tr1-11 | tr1-12 | tr1-13 | tr1-14 | tr1-15 | tr1-16 | tr1-17 | tr1-18 | tr1-19 | tr1-20 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | D | D | V | D | Ø | E | U | E | E | E | Ø |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr1-21 | tr1-22 | tr1-23 | tr1-24 | tr1-25 | tr1-26 | tr1-27 | tr1-28 | tr1-29 | tr1-30 | tr1-31 | tr1-32 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | U | E | E | E | E | E | E | E | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr1-33 | tr1-34 | tr1-35 | tr1-36 | tr1-37 | tr1-38 | tr1-39 | tr1-40 | tr1-41 | tr1-42 | tr1-43 | tr1-44 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | E | E | E | E | E | E | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr1-45 | tr1-46 | tr1-47 | tr1-48 | tr1-49 | tr1-50 | tr1-51 | tr1-52 | tr1-53 | tr1-54 | tr1-55 | tr1-56 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | E | E | E | E | E | E | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr1-57 | tr1-58 | tr1-59 | tr1-60 | tr1-61 | tr1-62 | tr1-63 | tr1-64 | tr1-65 | tr1-66 | tr1-67 | tr1-68 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | E | E | E | E | E | E | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr1-69 | tr1-70 | tr1-71 | tr1-72 | tr1-73 | tr1-74 | tr1-75 | tr1-76 | tr1-77 | tr1-78 | tr1-79 | tr1-80 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | E | E | E | E | E | E | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr1-81 | tr1-82 | tr1-83 | tr1-84 | tr1-85 | tr1-86 | tr1-87 | tr1-88 | tr1-89 | tr1-90 | tr1-91 | tr1-92 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | E | E | E | E | E | E | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr1-93 | tr1-94 | tr1-95 | tr1-96 | tr1-97 | tr1-98 | tr1-99 | tr1-05 | tr2-01 | tr2-02 | tr2-03 | tr2-04 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | E | E | E | E | D | E | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr2-05 | tr2-06 | tr2-07 | tr2-08 | tr2-09 | tr2-10 | tr2-11 | tr2-12 | tr2-13 | tr2-14 | tr2-15 | tr2-16 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | E | E | E | E | E | E | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr2-17 | tr2-18 | tr2-19 | tr2-20 | tr2-21 | tr2-22 | tr2-23 | tr2-24 | tr2-25 | tr2-26 | tr2-27 | tr2-28 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | E | E | E | E | E | E | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr2-29 | tr2-30 | tr2-31 | tr2-32 | tr2-33 | tr2-34 | tr2-35 | tr2-36 | tr2-37 | tr2-38 | tr2-39 | tr2-40 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | D | D | D | D | D | D | D | D | D |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr2-41 | tr2-42 | tr2-43 | tr2-44 | tr2-45 | tr2-46 | tr2-47 | tr2-48 | tr2-49 | tr2-50 | tr2-51 | tr2-52 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | D | D | D | D | D | D | D | D | D | D | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr2-53 | tr2-54 | tr2-55 | tr2-56 | tr2-57 | tr2-58 | tr2-59 | tr2-60 | tr2-61 | tr2-62 | tr2-63 | tr2-64 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | D | N | N | D | D | U | U | U | U |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr2-65 | tr2-66 | tr2-67 | tr2-68 | tr2-69 | tr2-70 | tr2-71 | tr2-72 | tr2-73 | tr2-74 | tr2-75 | tr2-76 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | U | E | E | E | U | D | E | D | E | Ø | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr2-77 | tr3-01 | tr3-02 | tr3-03 | tr3-04 | tr3-05 | tr3-06 | tr3-07 | tr3-08 | tr3-09 | tr3-10 | tr3-11 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | D | E | D | D | E | E | E | U | U | U | U |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr3-12 | tr3-13 | tr3-14 | tr3-15 | tr3-16 | tr3-17 | tr3-18 | tr3-19 | tr3-20 | tr3-21 | tr3-22 | tr3-23 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | Ø | Ø | E | E | E | D | D | E | V | D | D | D |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr3-24 | tr3-25 | tr3-26 | tr3-27 | tr3-28 | tr3-29 | tr3-30 | tr3-31 | tr3-32 | tr3-33 | tr4-01 | tr4-02 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | V | V | N | E | E | E | E | D | E | U | D | D |
| H / L / C / P / T / J / F | = | = | = | = | = | = | = | = | = | = | = | = |
| K | N | N | = | = | = | = | = | = | = | = | = | = |

| Rule | tr4-03 | tr4-04 | tr4-05 | tr4-06 | tr4-07 | tr4-08 | tr4-09 | tr4-10 | tr4-11 | tr5-01 | tr5-02 | tr5-03 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | N | U | D | E | U | U | U | D | D | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | tr5-04 | tr6-01 | tr6-02 | tr7-01 | tr7-02 | sl1-01 | sl1-02 | sl1-03 | sl1-04 | sl1-05 | sl1-06 | sl1-07 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | N | D | E | U | U | E | U | E | E | D | D | D |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | sl1-08 | sl1-09 | sl1-10 | sl1-11 | sl1-12 | sl1-13 | sl1-14 | sl1-15 | sl1-16 | sl1-17 | sl1-18 | sl1-19 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | Ø | E | D | D | E | D | E | U | U | U | U |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | sl1-20 | sl1-21 | sl1-22 | sl1-23 | sl1-24 | sl1-25 | sl1-26 | sl1-27 | sl1-28 | sl1-29 | sl1-30 | sl1-31 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | U | U | U | U | E | U | E | E | U | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | sl1-32 | sl1-33 | sl1-34 | sl1-35 | sl1-36 | sl1-37 | sl1-38 | sl1-39 | sl1-40 | sl1-41 | sl1-42 | sl1-43 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | V | V | V | V | Ø | N | N | D | N | Ø | Ø |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | sl1-44 | sl1-45 | sl1-46 | sl1-47 | sl1-48 | sl1-49 | sl1-50 | sl1-51 | sl1-52 | sl1-53 | sl1-54 | sl1-55 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | Ø | E | E | E | E | E | E | E | E | E | E | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | sl1-56 | sl1-57 | sl1-58 | sl1-59 | sl1-60 | sl1-61 | sl1-62 | sl1-63 | sl1-64 | sl1-65 | sl1-66 | sl1-67 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | E | E | D | Ø | E | Ø | V | E | D |
| H / L / C / P / T / J / F | = | = | = | = | = | = | = | = | = | = | = | = |
| K | = | = | = | = | = | = | = | = | N | N | = | = |

| Rule | sl1-68 | sl1-69 | sl1-70 | sl2-01 | sl2-02 | sl2-03 | sl2-04 | sl2-05 | sl2-06 | sl2-07 | sl2-08 | sl2-09 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | D | Ø | U | E | D | E | U | E | E | D | D |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | sl2-10 | sl2-11 | sl2-12 | sl2-13 | sl2-14 | sl2-15 | sl2-16 | sl2-17 | sl2-18 | sl2-19 | sl2-20 | sl2-21 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | U | U | U | U | E | U | D | V | D | V | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | sl2-22 | sl2-23 | sl2-24 | sl2-25 | sl2-26 | sl3-01 | sl3-02 | sl3-03 | sl3-04 | sl3-05 | sl3-06 | sl3-07 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | A | D | Ø | V | E | D | D | D | U | U | D | E |
| H / L / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |
| C | N | = | = | = | = | = | = | = | = | = | = | = |

| Rule | sl3-09 | sl3-10 | sl3-11 | sl3-12 | sl3-13 | sl3-14 | sl3-15 | sl3-16 | sl3-17 | sl3-18 | sl3-19 | sl3-20 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | E | D | Ø | E | D | V | D | N | Ø | D | E |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | sl4-01 | sl4-02 | sl4-03 | sl4-04 | sl4-05 | sl4-06 | sl4-07 | sl4-08 | sl4-09 | sl4-10 | sl4-11 | sl4-12 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | D | U | D | Ø | Ø | U | E | V | D | V | Ø |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | sl4-13 | sl4-14 | sl5-01 | sl5-02 | sl5-03 | sl5-04 | sl5-05 | sl5-06 | sl5-07 | sl5-08 | sl5-09 | sl5-10 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | V | E | U | D | D | U | U | E | E | Ø | Ø |
| H / L / C / P / T / J / F | = | = | = | = | = | = | = | = | = | = | = | = |
| K | = | U | = | = | = | = | = | = | = | = | = | = |

| Rule | sl5-11 | sl5-12 | sl5-13 | sl5-14 | as1-01 | as1-02 | as1-03 | as1-04 | as1-05 | as1-06 | as1-07 | as1-08 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | Ø | V | Ø | E | Ø | D | E | M | Ø | E | U |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | as1-09 | as1-11 | as1-12 | as1-13 | as1-14 | as1-15 | as1-16 | as1-17 | as1-18 | as1-19 | as1-20 | as1-21 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | Ø | Ø | Ø | V | Ø | V | D | D | U | V | V |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | as1-22 | as1-23 | as1-24 | as1-25 | as1-26 | as1-27 | as1-28 | as1-29 | as1-30 | as1-31 | as1-32 | as1-33 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | Ø | D | Ø | E | A | V | Ø | E | E | U | Ø |
| H / L / P / T / J / F | = | = | = | = | = | = | = | = | = | = | = | = |
| C | = | = | = | = | = | N | = | = | = | = | = | = |
| K | = | = | = | = | = | = | N | = | = | = | = | = |

| Rule | as1-34 | as1-35 | as1-36 | as1-37 | as1-38 | as1-39 | as1-40 | as1-41 | as1-10 | as2-01 | as2-02 | as2-03 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | Ø | Ø | D | D | E | Ø | D | E | U | E | D | D |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | as2-04 | as2-05 | as2-06 | as2-07 | as2-08 | as2-09 | as2-10 | as2-11 | as2-12 | as2-13 | as2-14 | as2-15 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | D | E | U | U | D | E | Ø | E | E | Ø | Ø |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | as2-16 | as2-17 | as2-18 | as2-19 | as2-20 | as2-21 | as2-22 | as2-23 | as2-24 | as2-25 | as2-26 | as2-27 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | Ø | D | A | V | V | D | D | A | Ø | Ø | U |
| H / L / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |
| C | = | = | = | = | = | = | = | = | N | = | = | = |

| Rule | as2-28 | as2-29 | as2-30 | as2-31 | as2-32 | as2-33 | as3-01 | as3-02 | as3-03 | as3-04 | as3-05 | as3-06 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | Ø | U | U | E | U | U | D | Ø | Ø | Ø | D |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | as3-07 | as3-08 | as3-09 | as3-10 | as3-11 | as3-12 | as3-13 | as3-14 | as3-15 | as3-16 | as3-17 | as3-18 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | U | U | Ø | U | V | V | Ø | D | V | N | V |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | as3-19 | as3-20 | as3-21 | as3-22 | as3-23 | as3-24 | as3-25 | as3-26 | as3-27 | as3-28 | as3-29 | as3-30 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | Ø | D | Ø | N | Ø | D | D | Ø | E | U | U | U |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | as3-31 | as3-32 | as3-33 | as3-34 | as4-01 | as4-02 | as4-03 | as4-04 | as4-05 | as4-06 | as4-07 | as4-08 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | U | E | U | D | E | E | D | D | E | U | U | U |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | as4-09 | as4-10 | as4-11 | as4-12 | as4-13 | as4-14 | as4-15 | as4-16 | as4-17 | as4-18 | as4-19 | as4-20 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | D | D | Ø | Ø | V | D | Ø | Ø | Ø | E | D |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | as4-21 | as4-22 | as4-23 | as4-24 | as5-01 | as5-02 | as5-03 | as5-04 | as5-05 | as5-06 | as5-07 | as5-08 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | Ø | E | E | N | D | E | U | E | D | N | Ø | Ø |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | as5-09 | as5-10 | as5-11 | as5-12 | as5-13 | as5-14 | as5-15 | wf01 | wf02 | wf03 | wf04 | wf05 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | V | V | Ø | U | Ø | D | E | M | M | E | M | M |
| H / L / C / P / T / J / F | = | = | = | = | = | = | = | = | = | = | = | = |
| K | = | N | = | = | = | = | = | = | = | = | = | = |

| Rule | wf06 | wf07 | wf08 | wf09 | wf10 | wf11 | wf12 | wf13 | wf14 | wf15 | wf16 | wf17 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | M | M | D | M | M | M | E | M | M | M | M | M |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | wf18 | wf19 | wf20 | wf21 | wf22 | wf23 | wf24 | wf25 | wf26 | wf27 | wf28 | wf29 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | M | M | M | D | M | M | D | D | M | E | D | M |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | wf30 | wf31 | wf32 | wf33 | wf34 | wf35 | wf36 | wf37 | wf38 | wf39 | wf40 | wf41 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | M | M | M | M | M | M | M | M | E | D | V | D |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | wf42 | wf43 | wf44 | wf45 | wf46 | wf47 | wf48 | wf49 | wf50 | wf51 | wf52 | wf53 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | D | D | M | M | D | M | M | M | M | M | M | M |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | wf54 | wf55 | wf56 | wf57 | wf58 | wf59 | wf60 | wf61 | wf62 | wf63 | wf64 | wf65 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | A | A | Ø | Ø | A | A | E | A | E | A | N | E |
| H / L / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |
| C | = | = | = | = | = | = | = | N | = | N | = | = |

| Rule | wf66 | wf67 | wf68 | wf69 | wf70 | wf71 | wf72 | wf73 | wf74 | wf75 | wf76 | wf77 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | N | A | E | A | A | D | A | A | Ø | Ø | Ø | Ø |
| H / L / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |
| C | = | N | = | N | N | = | N | N | = | = | = | = |

| Rule | wf78 | wf79 | wf80 | wf81 | wf82 | wf83 | wf84 | wf85 | wf86 | wf87 | wf88 | wf89 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | Ø | Ø | Ø | Ø | Ø | Ø | Ø | Ø | Ø | Ø | Ø | Ø |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | wf90 | wf91 | wf92 | wf93 | wf94 | wf95 | wf96 | wf97 | wf98 | wf99 | esc01 | esc02 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | Ø | Ø | E | Ø | Ø | Ø | E | E | E | D | A | A |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | esc03 | esc04 | esc05 | esc06 | esc07 | esc08 | esc09 | esc10 | esc11 | esc12 | esc13 | esc14 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | V | N | V | M | M | M | M | E | D | D | U | Ø |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | esc15 | esc16 | esc17 | esc18 | esc19 | esc20 | esc21 | esc22 | esc23 | esc24 | esc25 | esc26 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Reference | E | E | E | E | E | M | M | U | N | A | E | D |
| H / L / C / P / T / J / F / K | = | = | = | = | = | = | = | = | = | = | = | = |

| Rule | esc27 | esc28 | esc29 | esc30 |
| --- | --- | --- | --- | --- |
| Reference | V | E | E | M |
| H / L / C / P / T / J / F / K | = | = | = | = |

</details>

### 6.10 Owner decision K and projected standard-library parity

Use the §6.4 declaration-only route at `nameonly` for `filesystem`, `parse` and `config`. The hint
uses the declared class's canonical method. This candidate reaches unfiltered consumers even though
a caller can supply a different value. The sole binding, lookup, contradiction, escape and helper
checks still apply. For example, a module name with a value-bearing annotation can qualify through
R4. An unannotated module assignment calling a helper falls outside R5. After identity, apply the
ordinary §5.2 and §5.4 shape rules, including unsupported grouping when the call loses the anchor.
Decision 9 records the owner's direction and the cost of the declined W alternative.

#### Census projection

From each `census/sites-*.jsonl`, select `proven == true`, one of the three standard-library
families, and an enclosing function. Exclude `module_level`, `class_body_in_function`, `lambda`,
`multiline_callee` and `not_line_anchor`. Count unique (file, line, enclosing function, family,
resolved callee) keys. Each selected occurrence has a distinct key in these ledgers. Bounds are
absent. For this projection, assume scan lists these keys, every selected annotation tie satisfies
§6.4 as a declaration-only receiver, and all other selected keys match. The tie label alone
cannot establish a helper's binding position or a local initializer's construction.

| Corpus | Selected keys | Parameter / local / helper annotation ties | K matched / all keys |
|---|---:|---:|---:|
| Superset | 222 | 21 / 16 / 1 | 222/222 (100%) |
| Black | 77 | 2 / 0 / 3 | 77/77 (100%) |
| httpie | 75 | 1 / 0 / 0 | 75/75 (100%) |
| Saleor | 335 | 0 / 0 / 0 | 335/335 (100%) |

Under these assumptions K retains all 38, five, one and zero declaration-tied keys, respectively.
Two further Superset parameter records have census cause `receiver_not_unique` and fall outside
the proven-key selection. HTTP ties, including `client.py:114`, belong to the HTTP gates.
S3 measures actual per-corpus recall against every injectable scan key in scan's files, using
scan's function bounds. The 95% floor uses that full denominator.

| Ledger count | K (772 IDs) |
|---|---:|
| Sites | 338 |
| Unsupported groups | 91 |
| Plain `none` | 90 |
| Counted omissions | 253 |
| `star_import` | 9 |
| `shadowed_root` | 67 |
| `not_imported` | 11 |
| `repository_local_module` | 7 |
| `binding_unavailable` | 2 |
| `member_rebound` | 47 |
| `receiver_not_unique` | 46 |
| `annotation_conflict` | 8 |
| `helper_unannotated` | 5 |
| `helper_not_admissible` | 14 |
| `receiver_value_unproven` | 37 |

These counts tally one primary `want` per ID. The additional omitted HTTP occurrences on the
two mixed lines in §6.5 remain separate runtime warning assertions.

### 6.11 Ledger substitutions adopted under K

These eleven records replace the declaration-only omissions in `becb5c6d`. The verdict compares
the K want with the pinned matrix. Use the site want's canonical callee for the `sl1-64` selector.
The remaining 761 ledger lines match that revision byte for byte.

| ID | K want | K verdict |
|---|---|---|
| tr3-24 | `site(filesystem,pathlib.Path.read_text,nameonly)` | keep |
| tr3-25 | `site(parse,configparser.ConfigParser.read,nameonly)` | keep |
| sl1-64 | `site(filesystem,pathlib.Path.exists,nameonly)` | change |
| sl1-65 | `site(config,pathlib.Path.home,nameonly)` | keep |
| sl4-14 | `unsupported(not_line_anchor,1)` (selector `pathlib.Path.stat`) | keep |
| as1-28 | `site(filesystem,pathlib.Path.exists,nameonly,symbol=read,bounds=6-8)` | keep |
| as5-10 | `site(config,pathlib.Path.home,nameonly)` | keep |
| pin03 | `site(filesystem,pathlib.Path.read_text,nameonly)` | change |
| pin04 | `site(filesystem,pathlib.Path.exists,nameonly)` | change |
| pin05 | `site(filesystem,pathlib.Path.mkdir,nameonly)` | change |
| pin06 | `site(filesystem,pathlib.Path.touch,nameonly)` | change |

K gives 644 keep, 94 change and 34 add. `as5-11` (the result of `seed.home()`) stays untied,
and `wf50/51` still count the selected member deletion. This choice changes neither HTTP
declarations nor constructed-class precedence.

## 7. Gates

| Gate | Statement | Oracle |
|---|---|---|
| G-overlay | on httpie at the pinned commit, the inventory emits a row for each of the overlay's HTTP sites (`client.py:114`, `update_warnings.py:44`) with the overlay row's file, line, bounds and kind, and the bare function name as symbol | the archived overlay; faultpath's site guard |
| G-bounds | for every function of every census corpus, prism's (file, start line, end line, name) equals the `ast` listing of the stage M census: zero differences | Python `ast` under the pinned interpreter |
| G-accept | on every census corpus, for every injectable inventory row: (1) faultpath's loader accepts the document; (2) the call faultpath binds at `site.line` is the row's call: the discovery and binder spans (callee and whole call, §6.8) and their source texts agree, and independent source replay agrees with the producer-selected route recorded in the sidecar. Import-root rows require the consumer's matching qualified name. Bare builtin rows require the consumer's matching canonical builtin identity (6.8). For receiver routes record both spellings and check the same occurrence spans; (3) the site guard, run with strict line checking, does not answer `site_call_unavailable`, `unsupported_multiline_callee`, `stale_function_bounds` or `stale_site_symbol`; (4) for a row whose resolved callee is in the consumer's tables (HTTP rows rooted in `requests`, `httpx`, `urllib3` or `aiohttp`; every standard-library row) and whose patch the planner resolves, the plan has no `dependency_identity_mismatch`, `no-dependency-hint` or `no-applicable-fault` note. Every other guard answer and planner note is tabulated per reason, not gated. The HTTP half runs in slices S2a and S2b, the standard-library half in S3 | faultpath's loader, call binder, site guard and planner, run read-only (the planner reads source only; no suite run) |
| G-parity | For each census corpus, prism must match at least 95% of **all** injectable standard-library scan keys. Form the denominator from rows with bounds in scan's entire `diff.files`, including files prism omitted. Match on (file, line, function bounds, family, canonical callee). Every miss has a source witness in the closed list P1–P5 of section 6.7; every row prism lists that scan does not is confirmed against the source, and none is a call that is not that library call. Prism's `module_level` rows are compared with scan's bounds-less rows, and prism's unsupported and unproven sets with scan's warnings. If scan cannot run, G-parity is not met | `faultpath scan` |
| G-precision | a reader checks HTTP rows against the source, stratified by the sidecar route verified by independent source replay (§6.8): every row of each of R1 to R5 and of root identity when a stratum has 50 rows or fewer across the census corpora, otherwise 50 drawn with a recorded seed; at most 1 row per stratum where the callee is not the listed library call; every stratum's size is reported | hand check |
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
| 9 | Declaration-only standard-library receivers and the G-parity floor | **K chosen, W declined (2026-11-25).** The owner directed “lane 2: yes and fold” and specified keeping these receivers at `nameonly` for scanner parity. The 95% floor uses every injectable scan key in each corpus. | W would withhold the 38 eligible annotation-tied Superset keys in the §6.10 projection, giving **184/222 (82.9%)** raw recall. Its proposed denominator exemption was declined. | Decided in r4 |

## 10. Risks and accepted limits

- **A second discovery implementation next to scan.** G-parity makes the overlap measurable; decision 7 is
  where the owner picks one. Until then both may feed faultpath and duplicate sites differ in id (different
  source algorithm), which faultpath already tolerates for overlay rows.
- **Receiver typing is shallow.** R3 field hints are deferred. §6.4 prefers a sole witnessed construction. §6.10 admits eligible standard-library declarations at `nameonly`. A supplied value can differ from its declaration. Sessions passed
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
  The supplied oracle covered 615 fixture lines. Both reviewers subsequently rejected round 1.
- 2026-11-24. Folded round 1 on the existing amendment. Added 99 focused verdicts, the write
  invalidation and annotation-conflict rules, warning eligibility, repository lookup roots and the
  measured Q12 decision. Tightened the matcher, projection and gate evidence. §6.8 records the
  multiline conflict for the owner. This round pinned 730 verdicts and 714 oracle lines.
- 2026-11-25. Round 2 split (A approved, B rejected). Pooled the independent and held options in
  §6.8. Deferred R3 hints, required value evidence for R4/R5, and specified protocol reads, capture
  order, class fallback and import precedence. Updated the route evidence and parity witnesses.
  This escalation changes 44 existing wants relative to round 2 and proposes 30 fixtures. One joint
  review round remained at that revision. Both reviewers subsequently rejected it.
- 2026-11-25. Proposed the d3 closed-item fold to the owner. §§6.2–6.7 cover receiver-only setup, lookup-slot invalidation, constructed-class precedence, P1 evidence, remapped ties, class fallback and startup precedence. §§6.9–6.11 check all 772 proposed decisions and present the K/W recall and ledger deltas. The eight pins are materialised. esc31–34 are proposed. That proposal awaited the owner’s choice.
- 2026-11-25. The owner directed the r4 fold with K. §§6.4 and 6.7 admit eligible standard-library declarations at `nameonly` and retain the full per-corpus parity floor. §§6.8–6.11 reconcile the K ledger, pins, oracle and census. Both reviewers review this revision once.
