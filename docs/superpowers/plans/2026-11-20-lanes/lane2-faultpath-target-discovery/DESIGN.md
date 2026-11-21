# Lane 2: faultpath target discovery — design

Status: architecture final; design review round 1 folded on 2026-11-21. The HTTP callee table and the
admission details of routes R4 and R5 (section 6) are tentative until the test matrix and enumeration (PLAN
stages T and E) close. Tracking issue: #613. Research record:
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

**Faultpath** (read, not run).

- A row is injectable only when `kind` is `external_call` or `resource_acquire`. Faultpath binds a row to
  one call by its line alone, with the rule section 5.4 states as the **line anchor**: among the calls whose
  callee covers the line, the leftmost-starting callee, and on a tie the call that ends last. It is not "the
  outermost call", and it knows nothing about dependency tables: on `x = foo(a) + requests.get(u)` it binds
  `foo`.
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
| `site` | `file`, `line` (first line of the callee expression), `symbol` (the bare name of the enclosing function, section 5.4), `function_start_line`, `function_end_line` (section 5.4), `language: "python"` |
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

Reasons (closed list, extended only through stage D), and the order in which they are tested for one call
(the first that applies is the call's reason):

1. `parse_error_in_function`: the innermost `def` whose body contains the call has a parse error
   (tree-sitter `ERROR` or missing node) anywhere inside it.
2. `module_level`: no `def` body contains the call (module top level, a class body directly in the module,
   a decorator, default value or annotation of a top-level `def`).
3. `class_body_frame`: the innermost frame-introducing construct whose body contains the call is a class
   body (a class defined inside a function).
4. `lambda_frame`: that construct is a lambda.
5. `multiline_callee`: the callee expression does not fit on one line.
6. `not_line_anchor`: the call is not the line anchor of its callee's first line (section 5.4).

**Identity unproven.** A call that only *resembles* a table entry produces no row. It is counted when (a)
its callee chain's root is spelled like a table library's import root, or like a builtin in the tables,
and root identity fails, or (b) it calls a method from the callee tables on a receiver that has at least
one binding from a listed constructor, or from a same-file helper whose return annotation names a listed
class, and the receiver route fails. A method call on a receiver with no tie to a table library (`d.get(k)`)
is neither a row nor counted. Each counted call gets the first cause that applies from this closed list:

| Cause | Meaning |
|---|---|
| `star_import` | the file has a `from m import *` in any scope |
| `shadowed_root` | the scope that binds the root name holds another binding of it (section 5.3, closed-world rule) |
| `not_imported` | no visible scope binds the root name and it is not a builtin in the tables |
| `repository_local_module` | the import binds a module or package that exists in the repository |
| `receiver_not_unique` | the receiver name (or `self` attribute) has more than one binding in its scope |
| `helper_unannotated` | the receiver's single binding is a call to a same-file helper with no return annotation naming a listed class |
| `helper_not_admissible` | the helper fails route R5's other conditions (decorated, a generator, an `async def` not awaited at the call, defined or bound more than once, or shadowed at the call) |

The document carries one warning per file with counted calls:
`inventory: identity unproven: <file>: <n> (<cause>: <k>, …)`, causes in the table's order, zero counts
omitted. It also carries one warning per Python file the loader skipped,
`inventory: skipped: <file>: <loader reason>`, and one warning naming the excluded directory rules,
`inventory: excluded directories: .git, target, node_modules, vendor, venv, site-packages, dot directories,
build and dist at the root`.

### 5.3 Identity proof

One proof, the existing one, extended. `dependency_hint.rs` and `dependency_identity.rs` are refactored so
that root identity is available for any call node (today the code starts from a finding's line and returns
only a kind): which import the callee chain's root is bound to at the site, and the dotted path that gives
(`r.get` under `import requests as r` is `requests.get`). The inventory and the echo-row decoration both use
it. The library-prefix table that decorates echo rows keeps its behaviour and is not consulted by the
inventory. The inventory decides the family from its own **callee-level** tables, because an inventory must
not list `requests.Session()` or `requests.utils.quote()` as HTTP calls.

**Callee tables.**

- `http` (final contents fixed in stage D against the census): module functions
  `requests.{get,post,put,patch,delete,head,options,request}`,
  `httpx.{get,post,put,patch,delete,head,options,request,stream}`, `urllib.request.{urlopen,urlretrieve}`,
  `urllib3.request`, `aiohttp.request`; methods of `requests.Session`, `httpx.Client`, `httpx.AsyncClient`,
  `aiohttp.ClientSession`, `urllib3.PoolManager`: the verb methods plus `request`, `send`, `stream`,
  `urlopen` where the class has them. The resolved callee of a method is `<class path>.<method>` with the
  class path as listed here. Rows rooted in `urllib.request` are listed although no catalogued HTTP fault
  matches them today; the funnel counts them.
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

**Closed-world rule.** Every identity proof (root identity, the builtin proof, routes R2 to R5) holds only
when *every* binding of the name in the scope that decides it is the proving one. The deciding scope is
Python's: the nearest enclosing function scope that binds the name (class bodies count only for code
directly in them), else the module; a `global` declaration moves the name to the module scope, and then an
assignment to it in any function counts as a module binding; `nonlocal` moves it to the enclosing function.
Binding positions, all of which count: assignment targets (plain, annotated with a value, augmented, tuple,
list and starred unpacking), `for` and comprehension targets, `with … as`, `except … as`, walrus, `import`
and `from … import` (an `as` name included), `def` and `class` names, parameters, `del`, `match` captures
(`case x`, `case [x]`, `as x`, `**rest`), and `type` alias statements. A `from m import *` anywhere in the
file can bind any name: such a file proves no root identity and no builtin (cause `star_import`). Examples:
`try: import requests` / `except ImportError: requests = None` is two module bindings, so `requests.get(u)`
is unproven (`shadowed_root`); `open(p)` is `builtins.open` only when no visible scope binds `open`.

For a `self.x` receiver (route R3) the bindings are every assignment to `self.x` in the methods of the class
body: plain, annotated, augmented and tuple targets, `for self.x in`, `with … as self.x`, `del self.x`, and
`setattr(self, "x", …)` with that literal name. Out of model, as for `Exact` elsewhere: assignments to the
attribute from outside the class body, `setattr` with a computed name, writes through `__dict__`, and
assignments in subclasses in other files. A site that relies on them still gets an `exact` row.

**Receiver routes** for method entries, in order; the first that applies decides:

| Route | Evidence | Grade |
|---|---|---|
| R1 direct | the receiver expression is itself a listed constructor call (`requests.Session().get(u)`) | exact |
| R2 local | the receiver is a local whose only binding in its scope (closed-world rule) is an assignment from a listed constructor call or a `with <constructor>() as v` target | exact |
| R3 attribute | `self.x` whose only binding in the class (closed-world rule) is an assignment from a listed constructor call | exact |
| R4 annotation | a parameter or local whose only binding in its scope is that parameter or one annotated assignment, annotated with a listed class (also `Optional[C]`, `C \| None` and the string form `"C"`) | nameonly |
| R5 helper | a local whose only binding is `v = make(…)` (or `v = await make(…)` for an `async def` helper), where `make` is a function defined exactly once at module level in the same file, bound nowhere else in the file, not shadowed in the calling function's visible scopes, undecorated, not a generator, and whose return annotation names a listed class (the same spellings as R4) | nameonly |

Today's hint code has the plain-assignment forms of R2 and R3 (`x = Ctor()`, `self.x = Ctor()`) and checks a
fixed list of binding forms; the closed-world rule replaces that list for the inventory. R1, the `with`
form of R2, R4 and R5 are new, and so is building the resolved callee for a method
(`<class path>.<method>`). The routes need no call graph: an inventory-only run builds none. R5 is what the
httpie site `requests_session.send(..)` needs (its session comes from a same-file helper with an annotated
return); a helper in another file is an accepted loss unless the census shows it matters. A receiver with
two bindings, a non-call initialiser, or a helper that fails R5 is unproven: no row, counted (section 5.2).

R1, the `with` form of R2 and R4 are not usable by faultpath today (dynamic callee, unresolved `with`
receiver, witness required). They still serve discovery and are counted in the funnel; stage M's rule
(drop a route that covers under 2% of HTTP sites in every census corpus) is the control on their cost.

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
  tree-sitter terms, the end row of the last leaf inside the `function_definition` node that is not a
  `comment` (a multi-line string ends on its closing quote's row). The inventory computes both in
  `inventory/python.rs`; it does not use or change `function_node_spanning` or `node_line_range`.
- **Line anchor.** For a line L, the candidate calls are every call in the file whose callee expression's
  line span contains L. A call inside the annotation of an annotated assignment (`x: T = v`) in a function
  body never runs and is not a candidate, unless every candidate is such a call. The **line anchor** of L is
  the candidate with the smallest key (|end line of its callee − L|, byte column where its callee starts,
  minus the byte column where the call ends); the end column is compared without its line. In words: the
  callee that starts leftmost on L, and on a tie the call that ends last. A table call is injectable only
  when it is the line anchor of the first line of its callee, whatever else is on the line; otherwise it is
  unsupported with reason `not_line_anchor`. Examples: on `x = foo(a) + requests.get(u)` the anchor is
  `foo`; on `r = requests.get(u).json()` it is the outer `.json` call (same start column, ends later); on
  `a, b = requests.get(u), requests.get(v)` it is the first `requests.get`; on `requests.get(build(u))` it
  is `requests.get`.
- `line` is the first line of the callee expression; the callee must fit on one line (reason
  `multiline_callee`). Arguments may continue on later lines.
- A call inside an unevaluated annotation (above) gets no row and is not counted.

### 5.5 Command line and document

- `--inventory <name>` turns the row source on; the only name in this lane is `python-deps`.
  `--inventory-kinds http,filesystem,parse,config` narrows it (default: every family the build supports).
- **Which files.** The inventory reads the repository with the whole-repository loader the navigation
  commands use (`prism::repo_loader::load_repo`, skip rules in section 2), keeps the Python files, and also
  skips any path with a directory component named `venv` or `site-packages`. Test directories are included;
  a consumer filters them. `--files` (repository-relative paths, as today) narrows the set. The file set
  does not depend on the diff.
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
    projected rows: `--min-confidence exact` drops the `nameonly` rows of routes R4 and R5,
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
  inventory/py_receiver.rs  receiver routes R1 to R5 under the closed-world rule
  inventory/rows.rs         InventorySite / UnsupportedSite → Target rows
  dependency_hint.rs        gains root_identity(parsed, call_node, ctx); resolve(parsed, line, ctx) uses it
  mod.rs                    inventory_document() and merge_inventory(); project() is unchanged
crates/prism-review/examples/inventory_bounds.rs   prints every function's bounds (gate G-bounds)
docs/contracts/targets-1.1.schema.json             vendored, byte-identical to the authority
```

`inventory/mod.rs`, `python.rs`, `py_tables.rs` and `py_receiver.rs` use no targets row types; only
`rows.rs` does.

## 6. Tentative rules that stage T and E may change

The HTTP callee table's exact contents; the admission details of routes R4 and R5 (annotation spellings
beyond those section 5.3 lists, one helper hop); whether `py_receiver.rs` builds on the taint lane's Python
binding census (`crates/prism-review/src/algorithms/taint_sinks/py_scope.rs`, `Sites`, private to that
module today) or implements the closed-world rule in the targets module (stage D decides); the
unsupported-reason list and the identity-unproven cause list (stage D may add entries, never remove one);
the loss classes that G-parity accepts. Each is pinned by matrix rows with a truth label before stage D
fixes it. The frame, symbol, bounds and line-anchor rules of section 5.4, the principle of section 5.2 and
the standard-library table of section 5.3 are not tentative.

Where the consumer's behaviour is the truth (does its guard accept this symbol, these bounds; which call
does it bind at this line; which canonical callee and kind does it derive), the label is obtained by
calling faultpath on the fixture, under the interpreter the plan pins. An answer of
`unsupported_interpreter` or `unqualified_runtime` is a failed probe, never a truth. If faultpath cannot be
installed, the rows keep the wants that the rules above give, their consumer column stays `pending`, and
the gates that need faultpath (G-accept, G-parity, G-live) are not met until it can be.

## 7. Gates

| Gate | Statement | Oracle |
|---|---|---|
| G-overlay | on httpie at the pinned commit, the inventory emits a row for each of the overlay's HTTP sites (`client.py:114`, `update_warnings.py:44`) with the overlay row's file, line, bounds and kind, and the bare function name as symbol | the archived overlay; faultpath's site guard |
| G-bounds | for every function of every census corpus, prism's (file, start line, end line, name) equals the `ast` listing of the stage M census: zero differences | Python `ast` under the pinned interpreter |
| G-accept | on every census corpus, for every injectable inventory row: (1) faultpath's loader accepts the document; (2) the call faultpath binds at `site.line` is the row's call: its callee text equals the row's source callee, and, for a row whose callee root is an import or a builtin, its callee qualified through the file's imports equals the row's resolved callee (for receiver-route rows the consumer's spelling is recorded beside the row's, not compared); (3) the site guard, run with strict line checking, does not answer `site_call_unavailable`, `unsupported_multiline_callee`, `stale_function_bounds` or `stale_site_symbol`; (4) for a row whose resolved callee is in the consumer's tables (HTTP rows rooted in `requests`, `httpx`, `urllib3` or `aiohttp`; every standard-library row) and whose patch the planner resolves, the plan has no `dependency_identity_mismatch`, `no-dependency-hint` or `no-applicable-fault` note. Every other guard answer and planner note is tabulated per reason, not gated. The HTTP half runs in slices S2a and S2b, the standard-library half in S3 | faultpath's loader, call binder, site guard and planner, run read-only (the planner reads source only; no suite run) |
| G-parity | on every census corpus, restricted to the files of scan's `diff.files`: prism lists at least 95% of scan's injectable rows (rows with bounds) keyed by (file, line, function bounds, family, canonical callee); every miss belongs to a loss class approved in stage D; every row prism lists that scan does not is confirmed against the source, and none is a call that is not that library call. Prism's `module_level` rows are compared with scan's bounds-less rows, and prism's unsupported and unproven sets with scan's warnings. If scan cannot run, G-parity is not met | `faultpath scan` |
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
| 7 | How faultpath treats prism rows next to scan rows (prefer one, dedupe, trust level) | no change requested; an issue is filed in the faultpath repository with the parity numbers | — | owner |
| 8 | A line-independent site key (to follow a site across edits) | not added; faultpath binds to a pinned source and does not need it | a new field needs a new contract minor | when a consumer asks |

## 10. Risks and accepted limits

- **A second discovery implementation next to scan.** G-parity makes the overlap measurable; decision 7 is
  where the owner picks one. Until then both may feed faultpath and duplicate sites differ in id (different
  source algorithm), which faultpath already tolerates for overlay rows.
- **Receiver typing is shallow.** One constructor binding, one annotation, one helper hop. Sessions passed
  through several functions or stored in containers are missed; they are counted as identity-unproven.
- **The closed-world rule costs recall** where Python code rebinds a name it does not use at the call
  (`try`/`except ImportError` fallbacks, a parameter defaulted and then assigned). Faultpath's own binding
  check refuses the same shapes; the counts show in the per-file warnings.
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
