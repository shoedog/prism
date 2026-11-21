# Lane 2: faultpath target discovery — design

Status: architecture final; the site-shape and receiver rules in section 6 are tentative until the test
matrix and enumeration (PLAN stages T and E) close. Tracking issue: #613. Research record:
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
  (`s = requests.Session(); s.send(..)`). Today this code only decorates rows echo already produced.
- `prism targets` requires `--diff`, and only the files the diff names are loaded
  (`load_review_inputs`, `src/api/review.rs:69`); a repository-wide run needs a synthetic diff that lists
  every line. `run_targets` always goes through the review runner, which builds the code property graph.
  The command writes schema 1.0 only; documents are checked against the schema file by tests, not at run
  time. Prism's copy of the 1.0 schema differs from the contract authority's in description text only (a
  known drift, not changed by this lane). A row id is the SHA-256 of nine fields: file, line, symbol,
  source algorithm, category, description, severity, related lines, related files (`targets/mod.rs:279`).

**Faultpath** (read, not run).

- A row is injectable only when `kind` is `external_call` or `resource_acquire`; `site.line` is the line
  where the callee expression starts, with that call the outermost call on the line;
  `site.function_start_line` and `function_end_line` equal the enclosing `def` exactly; `site.symbol` is a
  bare name, a qualified name or `module.qualname`. A module-level call has no bounds and is rejected.
- Faultpath resolves the patch point from the source at `site.line`; `dependency_hint.callee` and `.kind`
  only select which catalogued faults apply. With no hint at all it injects nothing. A wrong `kind` excludes
  the right faults, so an absent kind is better than a wrong one.
- `faultpath scan` already discovers standard-library filesystem, parsing and configuration call sites from
  the AST and writes targets schema 1.1 (`source_algorithm: faultpath.scan/1`, category
  `stdlib_fault_site.<kind>`). It does not cover HTTP or any third-party library. The recorded owner
  decision is "`faultpath scan` now, prism feeds the same contract later", with prism able to replace it
  "without any harness change".
- Schema 1.1 exists in the contract authority and in faultpath's vendored copy. It differs from 1.0 in three
  places: `schema_version` is `"1.1"`; `producer.tool` may be `prism` or `faultpath`; `dependency_hint.kind`
  adds `parse` and `config`. Faultpath's loader accepts exactly 1.0 or 1.1. Both schemas are closed: an
  unknown property fails validation.
- The httpie pilot (httpie at `2105caa49bae87c5809c274e407619a0de2639d1`) used an overlay with four rows:
  `httpie/client.py:114` (`requests_session.send`), `httpie/internal/update_warnings.py:44`
  (`requests.get`), `httpie/models.py:65` and `:68` (response body iterators). The appended rows carried no
  dependency hint; kind, exact bounds and symbol were what made them usable. Last measured result with the
  overlay: 14 of 14 reachable keys reached, 12 caught, 2 not caught.
- Nothing in flight on faultpath touches discovery or the targets contract (call binding, custody, process
  containment, interpreter matrix).

## 3. Goals, non-goals, anti-goals

Goals:

1. A row source inside `prism targets` that is independent of findings: a **dependency-call inventory** for
   Python, HTTP family first, then the three standard-library families.
2. Rows faultpath accepts with no harness change: `kind: external_call`, exact function bounds, an accepted
   symbol, a hint whose `kind` is present only when library identity is proven.
3. Every site the inventory recognises but cannot offer as injectable is listed with a reason, so the
   output shows where coverage is lost.
4. Deterministic identity: the same source gives the same rows and ids on every run, machine, thread count,
   resolution mode and cache state, whatever other row sources ran.
5. The pipeline demonstrated on httpie without the overlay.

Non-goals: JavaScript, TypeScript and Go inventories; database, queue and process families; third-party
parsing libraries (`yaml`); changing faultpath; a new contract version; fixing provenance's noisy rows on
`requests.*` lines (filed as a separate issue).

Anti-goals: rows whose `kind` rests on a name match alone (`x.get(..)` on an unknown receiver); output that
changes for existing users who do not ask for the inventory; a second, diverging copy of the library
identity proof.

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
parsed Python files ──► inventory::collect ──► InventoryOutput { sites, unsupported }
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
| `dependency_hint` | `{ "kind": "<family>", "callee": "<resolved dotted callee>" }`, for example `requests.Session.send` for `s.send(req)` |
| `site` | `file`, `line` (first line of the callee expression), `symbol` (Python qualified name of the enclosing function), `function_start_line`, `function_end_line`, `language: "python"` |
| `confidence` / `tier` | `exact` / `asserted` when identity is proven by imports and constructor bindings; `nameonly` / `candidate` when a type annotation is part of the proof |
| `severity` | `info` |
| `description` | `<resolved dotted callee>: dependency call site` |
| `parse_quality` | the file's parse quality |

The hint carries the resolved callee because faultpath selects faults by matching the callee against
patterns such as `requests.*` and `*.Session.*`; `r.get` under `import requests as r` would match less. The
text as written is kept in `expected.detail`. This is the same convention `faultpath scan` uses.

The id uses the existing nine-field rule unchanged. With the fixed description and severity above, an
inventory row's id depends only on file, line, symbol, family and resolved callee.

### 5.2 Unsupported sites

A call whose callee is proven to belong to a table entry but which cannot be offered as injectable is
emitted as a row with `kind: "other"`, `category: "dependency_call_unsupported"`,
`expected: { "property": "unknown", "detail": "<reason>" }`, the same `dependency_hint`, and
`severity: info`. Faultpath drops kind `other`; a reader can count reasons. Reasons (closed list, extended
only through the matrix): `module_level`, `multiline_callee`, `not_outermost_call_on_line`,
`several_table_calls_on_line`, `lambda_frame`, `parse_error_in_function`.

A call that only *resembles* a table entry produces no row. Two kinds of such calls are counted in a
document-level warning (`inventory: <n> calls skipped, identity unproven`): (a) a callee chain whose root is
spelled like a table library's import root but is shadowed, not imported, or names a repository-local
module; (b) a method from the callee tables called on a receiver that has at least one binding from a
listed constructor, or from a same-file helper that returns one, but fails its route's uniqueness or
annotation requirement. A method call on a receiver with no tie to a table library (`d.get(k)`) is neither
a row nor counted.

### 5.3 Identity proof

One proof, the existing one, extended. `dependency_hint.rs` and `dependency_identity.rs` are refactored so
that root identity is available for any call node (today the code starts from a finding's line and returns
only a kind): which import the callee chain's root is bound to at the site, and the dotted path that gives
(`r.get` under `import requests as r` is `requests.get`). The inventory and the echo-row decoration both use
it. The library-prefix table that decorates echo rows keeps its behaviour and is not consulted by the
inventory. The inventory decides the family from its own **callee-level** tables, because an inventory must
not list `requests.Session()` or `requests.utils.quote()` as HTTP calls.

Callee tables (final contents are fixed in stage D against the census):

- `http`: module functions `requests.{get,post,put,patch,delete,head,options,request}`,
  `httpx.{get,post,put,patch,delete,head,options,request,stream}`, `urllib.request.{urlopen,urlretrieve}`,
  `urllib3.request`, `aiohttp.request`; methods of `requests.Session`, `httpx.Client`, `httpx.AsyncClient`,
  `aiohttp.ClientSession`, `urllib3.PoolManager`: the verb methods plus `request`, `send`, `stream`,
  `urlopen` where the class has them.
- `filesystem`, `parse`, `config`: the standard-library callables `faultpath scan` lists (builtin `open`,
  `io.open`, `pathlib.Path` read and write methods, `os` and `shutil` file operations, `tempfile`
  constructors; `json.load(s)`, `tomllib` and `tomli` `load(s)`, `configparser` readers, the XML parsers,
  `pickle.load(s)`; `os.environ.get`, `os.getenv`, `importlib.metadata` readers). The table is written in
  prism from the standard library's documentation; agreement with scan is established by the differential
  gate (section 7), not by copying.

Receiver typing for method entries, in order; the first that applies decides:

| Route | Evidence | Grade |
|---|---|---|
| R1 direct | the receiver expression is itself a listed constructor call (`requests.Session().get(u)`) | exact |
| R2 local | the receiver is a local with exactly one binding in its function, initialised by a listed constructor call, or a `with <constructor>() as v` target | exact |
| R3 attribute | `self.x` with exactly one assignment in the class, from a listed constructor call | exact |
| R4 annotation | a parameter or local annotated with a listed class | nameonly |
| R5 helper | a single-binding local initialised by a call to a function defined exactly once at module level in the same file, never rebound there, whose return annotation names a listed class | nameonly |

Today's hint code has the plain-assignment forms of R2 and R3 (`x = Ctor()`, `self.x = Ctor()`). R1, the
`with` form of R2, R4 and R5 are new, and so is building the resolved callee for a method
(`<class path>.<method>`). The routes need no call graph: an inventory-only run builds none. R5 is what the
httpie site `requests_session.send(..)` needs (its session comes from a same-file helper with an annotated
return); a helper in another file is an accepted loss unless the census shows it matters. A receiver with
two bindings, a non-call initialiser, or an unannotated helper is unproven: no row.

### 5.4 Site rules

- The enclosing function is the nearest `def` or `async def`. `symbol` is its Python qualified name
  (`Class.method`, `outer.<locals>.inner`); the bounds are that definition's first line (the `def` line,
  not a decorator line) and last line.
- `line` is the first line of the callee expression. The call must be the outermost call whose callee
  starts on that line, the callee expression must fit on one line, and no second table call may share the
  line. Otherwise the site is unsupported with the matching reason.
- A call in a `lambda` is unsupported (`lambda_frame`). Calls in comprehensions, nested functions,
  decorators' argument lists, default-argument expressions and class bodies are settled by the matrix, with
  faultpath's site guard as the oracle (PLAN stage T).

### 5.5 Command line and document

- `--inventory <name>` turns the row source on; the only name in this lane is `python-deps`.
  `--inventory-kinds http,filesystem,parse,config` narrows it (default: every family the build supports).
- **Which files.** The inventory reads the repository with the whole-repository loader the navigation
  commands use (`prism::repo_loader::load_repo`, which skips `.git`, `target`, `node_modules` and `vendor`),
  keeps the Python files, and also skips any path with a component named `.venv`, `venv` or `site-packages`
  or starting with a dot. Test directories are included; a consumer filters them. `--files` (repository-
  relative paths, as today) narrows the set. The file set does not depend on the diff.
- `--diff` stays required unless `--inventory` is given. An inventory-only run does not call the review
  runner and builds no code property graph; giving `--algorithm` explicitly without `--diff` is an error.
  Its `diff` envelope follows the convention `faultpath scan` uses: `files` is the sorted list of
  inventoried files and `sha256` is the SHA-256 of that list's compact JSON encoding.
- With `--diff` and `--inventory` both given, findings are projected as today from the diff's files, and
  inventory rows for the whole repository are merged in before dedupe and sort.
- `--inventory-kinds` naming a family the build does not support is an error that lists the supported ones.
- `producer.algorithms` gains `prism.inventory/1` when the inventory ran.
- **Schema version.** A document produced with `--inventory` is written as schema 1.1. Tests check such
  documents against a vendored `docs/contracts/targets-1.1.schema.json`, which must be byte-identical to
  the contract authority's file. A document produced without `--inventory` stays 1.0 and byte-identical to
  today's output. No new contract version is created by this lane.

### 5.6 Code layout

```
crates/prism-review/src/targets/
  inventory/mod.rs          collect(), InventoryOptions, InventorySite, UnsupportedSite, family names
  inventory/python.rs       walk Python files: call nodes, enclosing definition, site rules
  inventory/py_tables.rs    callee-level tables per family
  inventory/py_receiver.rs  receiver routes R1 to R5
  inventory/rows.rs         InventorySite / UnsupportedSite → Target rows
  dependency_hint.rs        gains root_identity(parsed, call_node, ctx); resolve(parsed, line, ctx) uses it
  mod.rs                    inventory_document() and merge_inventory(); project() is unchanged
docs/contracts/targets-1.1.schema.json   vendored, byte-identical to the authority
```

`inventory/mod.rs`, `python.rs`, `py_tables.rs` and `py_receiver.rs` use no targets row types; only
`rows.rs` does.

## 6. Tentative rules that stage T and E may change

The callee tables' exact contents; routes R4 and R5's admission details (which annotation spellings count,
how many helper hops: one); the qualified-name spelling for nested functions; which syntactic contexts
count as an enclosing function; the unsupported-reason list. Each is pinned by matrix rows with a truth
label before stage D fixes it. Where faultpath's behaviour is the truth (does its guard accept this
symbol, these bounds, this line), the label is obtained by calling faultpath's own resolver on the fixture.
If faultpath cannot be installed, the defaults are: the symbol is Python's `__qualname__` spelling
(`Class.method`, `outer.<locals>.inner`); calls in nested functions and comprehensions are supported sites
of the nearest enclosing `def`; calls in decorator arguments, default-argument expressions and class bodies
are unsupported with reason `module_level` when no `def` encloses them. Those rows stay marked
`pending-faultpath` and are settled by gate G-accept.

## 7. Gates

| Gate | Statement | Oracle |
|---|---|---|
| G-overlay | on httpie at the pinned commit, the inventory emits a row for each of the overlay's HTTP sites (`client.py:114`, `update_warnings.py:44`) with the overlay row's file, line, bounds and kind, and a symbol faultpath's guard accepts | the archived overlay; faultpath's site guard |
| G-accept | every injectable inventory row on httpie loads in faultpath and passes its bounds and symbol checks: 0 `stale_function_bounds`, 0 `stale_site_symbol`, 0 `not_injectable_kind` | faultpath's loader and planner, run read-only |
| G-parity | for the three standard-library families on httpie and two more Python corpora, the set of (file, line, bounds, family, resolved callee) from prism equals the set from `faultpath scan`, except for differences that are each classified and listed | `faultpath scan` |
| G-precision | a reader checks 100 sampled HTTP rows across the census corpora against the source: at most 1 where the callee is not the listed library call | hand check |
| G-live | the httpie pilot re-run with prism's document and no overlay gives the two HTTP overlay sites the outcomes of the last overlay run, after the flake control: `client.py:114` reached and injected with the same verdicts; `update_warnings.py:44` kept by the census and reported as shadowed by the suite's mock, as before | faultpath pilot run |
| G-byte | without `--inventory`, `prism targets` output on the existing fixtures is byte-identical to main | fixture comparison |

`models.py:65` and `:68` in the overlay are `resource_acquire` rows for response-body iterators reached
through a method of an in-repository class. They need a typed receiver for a library *response* object,
which is outside this lane's routes. The lane reports whether the inventory finds them and does not gate on
them.

## 8. What the lane reports at the end

A funnel table for httpie: HTTP call sites an independent AST census finds → inventory rows → injectable
rows → accepted by faultpath → reached by the suite → injected → caught / not caught, with the loss reason
at each step. This is the "where coverage is actually lost" output, and it separates prism's losses
(identity unproven, unsupported shape) from faultpath's and the suite's (not reached, patch shadowed by a
mock).

## 9. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Where the inventory lives | a row source inside targets (approach B) | moving to a nav query is a move-only change of four files | before lane 5's pilot |
| 2 | Families | **Confirmed by the owner, 2026-11-20:** HTTP first (slice S2), then filesystem, parse, config with scan parity (slice S3) | stopping after S2 leaves standard-library discovery to `faultpath scan`; nothing built in S2 is wasted | after S2 merges |
| 3 | Schema version of an inventory document | 1.1, the already-promoted minor | emitting 1.0 would forbid the `parse` and `config` kinds | before S2 |
| 4 | Callee in the hint | resolved dotted callee; text as written in `expected.detail` (issue #613 asked for source text in the hint) | swapping the two fields changes every inventory id once | before S2 merges |
| 5 | Unsupported sites | rows of kind `other` with a reason | moving them to warnings removes rows; consumers that count them change | before S2 merges |
| 6 | Who runs the live httpie proof (G-live) | this lane's orchestrator, using faultpath read-only, outputs in the evidence directory, one working day at most | handing it to the tooling controller costs a handoff note | at S4 |
| 7 | How faultpath treats prism rows next to scan rows (prefer one, dedupe, trust level) | no change requested; an issue is filed in the faultpath repository with the parity numbers | — | owner |
| 8 | A line-independent site key (to follow a site across edits) | not added; faultpath binds to a pinned source and does not need it | a new field needs a new contract minor | when a consumer asks |

## 10. Risks and accepted limits

- **A second discovery implementation next to scan.** G-parity makes the overlap measurable; decision 7 is
  where the owner picks one. Until then both may feed faultpath and duplicate sites differ in id (different
  source algorithm), which faultpath already tolerates for overlay rows.
- **Receiver typing is shallow.** One constructor binding, one annotation, one helper hop. Sessions passed
  through several functions or stored in containers are missed; they are counted as identity-unproven.
- **One call per line** is faultpath's rule; chained or nested dependency calls on one line are listed as
  unsupported, not injected.
- **Sites a test suite mocks** are discovered but faultpath reports them as shadowed. That is correct and
  shows in the funnel.
- **PR #841** edits `cli.rs` and `main.rs` for targets flags. This lane's CLI change is two options and one
  branch in `run_targets`; rebase over #841 if it merges first.
