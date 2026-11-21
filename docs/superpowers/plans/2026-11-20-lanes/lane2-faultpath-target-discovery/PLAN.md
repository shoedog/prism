# Lane 2: faultpath target discovery — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** `prism targets --inventory python-deps` lists Python dependency call sites (HTTP, then filesystem,
parsing, configuration) as rows faultpath can inject at, and the httpie pilot runs on those rows with no
overlay.

**Architecture:** a second row source inside `crates/prism-review/src/targets/` (`inventory/`), AST-only,
merged with projected findings in `project()`. It reuses and extends the library-identity proof in
`dependency_hint.rs` and `dependency_identity.rs`. Inventory documents are written as targets schema 1.1.

**Tech stack:** Rust (prism-review crate, tree-sitter Python), `assert_cmd` CLI tests; Python 3 for the
census and funnel scripts; faultpath (read-only, from an exported copy) as oracle.

**Spec:** `DESIGN.md` in this directory. Gate names (G-overlay, G-accept, G-parity, G-precision, G-live,
G-byte), routes R1 to R5 and the row shape are defined there.

## Global constraints

- Branch per slice from `main`; worktree `~/code/prism-targets`; evidence `~/prism-evidence/faultpath-targets/`.
- No change to any file in the faultpath or tooling repositories. Faultpath is used from an exported copy
  of its `origin/main` under the evidence directory. Paths and run notes for faultpath and the httpie pilot
  are in `~/prism-evidence/lanes-2026-11-20/PRIVATE-BINDINGS.md`; nothing from that file is copied into this
  repository.
- Without `--inventory`, every existing `prism targets` output stays byte-identical (G-byte).
- Row values are exactly those of DESIGN 5.1 and 5.2: `source_algorithm` `prism.inventory/1`; categories
  `dependency_call.<family>` and `dependency_call_unsupported`; severity `info`; description
  `<resolved callee>: dependency call site`.
- `kind` in a hint only with proven identity; never from a name match.
- The public facade keeps its signatures: `prism_review::api::project`, `TargetsDocument`, `TargetsMeta`
  (new data goes in new `TargetsMeta` fields, which is `#[non_exhaustive]` and built from `Default`).
- PR #841 also edits `cli.rs` and `main.rs`; rebase over it if it merges first.
- httpie is pinned at `2105caa49bae87c5809c274e407619a0de2639d1`.

## Review focus

1. A function whose body has a parse error: its table calls become `parse_error_in_function` unsupported
   rows; other functions in the file are unaffected; no panic. (S2)
2. A decorated function: `function_start_line` is the `def` line, not the decorator line. (S2)
3. `await client.get(u)` inside `async def` with an `httpx.AsyncClient` or `aiohttp.ClientSession`
   receiver: listed like the synchronous form. (S2)
4. A file path that the finding projection normalises (leading `./`, backslashes): inventory rows use the
   same normalised spelling, so ids agree across platforms. (S2)
5. The same resolved callee on two lines of one function: two rows with different ids; a `--files` filter
   that excludes the file: no rows and the file absent from `diff.files`. (S2)

---

## Stage M: census (1 day)

### Task M-1: workspace and corpora

- [ ] Create the worktree and evidence directory (ORCHESTRATION section 4).
- [ ] `git clone https://github.com/httpie/cli ~/prism-evidence/faultpath-targets/corpus/httpie` and check
      out the pinned commit detached. If the network is unavailable, use the copy named in the private
      bindings file. Record the commit.
- [ ] Choose two more Python corpora from `~/code/bench-repos` that import `requests` or `httpx` in
      non-test code (count with `rg -l "^(import|from) (requests|httpx)" --glob '!**/test*'`); record the
      choice and the counts. `black` is the third standard-library corpus for G-parity.
- [ ] Export faultpath: `git -C ~/code/faultpath fetch origin` then
      `git -C ~/code/faultpath archive origin/main | tar -x -C ~/prism-evidence/faultpath-targets/faultpath-main`;
      create a virtual environment beside it and install the exported tree into it. Record the commit. If
      installation needs packages the machine cannot fetch, record that; tasks marked "needs faultpath" then
      fall back as each task states.

### Task M-2: independent census

**Files (evidence directory only):** `census/py_dep_census.py`, `census/README.md`.

- [ ] Write a Python `ast` script, independent of prism, that for each corpus lists every call whose callee
      chain or receiver can be tied to an HTTP library or to a standard-library callee of DESIGN 5.3. For
      each: file, line, callee text, how the tie is established (direct import; alias; from-import;
      constructor local; `with` target; `self` attribute; parameter annotation; local annotation; helper
      with annotated return; helper without annotation; no tie but table method name on a receiver bound
      from a library call), and the site shape (in a function; module level; nested function; lambda;
      comprehension; decorator argument; default argument; class body; multi-line callee; inner call of a
      line; several table calls on a line; `await`).
- [ ] Run it on the corpora. Write `census/CENSUS.md`: counts per family, tie and shape for each corpus, and
      for httpie the classification of the four overlay sites.

**Exit gate:** the two HTTP overlay sites fall under routes R1 to R5 and a supported shape. If one does
not, add the missing route or shape to DESIGN section 6 as a proposed parameter change and note it in the
ledger before stage T. Also record, per corpus, the share of HTTP call sites each route covers; a route
that covers under 2% everywhere may be dropped in stage D.

---

## Stage T and E: test matrix (2 days)

### Task T-1: fixtures and tests that record today's behaviour and the wanted rows

**Files:**
- Create: `crates/prism-review/src/targets/inventory_matrix_tests.rs` (declared from `targets/mod.rs` under
  `#[cfg(test)]`), `tests/fixtures/targets_inventory/py/` (one small file per row group)
- Create: `tests/fixtures/targets_inventory/rows.jsonl` (one line per row: `id`, `fixture`, `line`, `class`,
  `want`, `truth`)

`want` is one of `site(<family>,<resolved callee>,<grade>)`, `unsupported(<reason>)`, `none`. `truth` says
where the label comes from: `python-semantics`, `faultpath-guard` (obtained by calling faultpath's site
guard and resolver on the fixture from the exported copy; if faultpath could not be installed, the row is
written with `truth: pending-faultpath` and listed for S4), or `design`.

Seed rows (the implementer writes these first; stage E adds more):

| Row | Fixture line | Want |
|---|---|---|
| h01 | `import requests` … `requests.get(u)` in a function | site(http, requests.get, exact) |
| h02 | `import requests as r` … `r.post(u)` | site(http, requests.post, exact), source callee `r.post` |
| h03 | `from requests import get` … `get(u)` | site(http, requests.get, exact) |
| h04 | `from requests import get as fetch` … `fetch(u)` | site(http, requests.get, exact) |
| h05 | `s = requests.Session()` … `s.send(req)` | site(http, requests.Session.send, exact) |
| h06 | `with requests.Session() as s:` … `s.get(u)` | site(http, requests.Session.get, exact) |
| h07 | `self.s = requests.Session()` in `__init__`, `self.s.get(u)` in a method | site(http, requests.Session.get, exact) |
| h08 | `def f(s: requests.Session): s.get(u)` | site(http, requests.Session.get, nameonly) |
| h09 | `s = make()` where `def make() -> requests.Session:` in the same file | site(http, requests.Session.send, nameonly) |
| h10 | `requests.Session().get(u)` | site(http, requests.Session.get, exact) |
| h11 | `async def f(): async with httpx.AsyncClient() as c: await c.get(u)` | site(http, httpx.AsyncClient.get, exact) |
| n01 | `s = requests.Session()` (the constructor call itself) | none |
| n02 | `requests.utils.quote(x)` | none |
| n03 | `d.get(k)` with `d` a parameter | none |
| n04 | `requests = Fake()` in the function, then `requests.get(u)` | none (counted identity-unproven) |
| n05 | a repository-local `requests.py` beside the file | none (counted identity-unproven) |
| n06 | `s = requests.Session()` and later `s = other()` | none (counted identity-unproven) |
| n07 | `s = make()` where `make` has no return annotation | none (counted identity-unproven) |
| n08 | `from requests import *` … `get(u)` | none |
| u01 | `R = requests.get(u)` at module level | unsupported(module_level) |
| u02 | `parse(requests.get(u))` | unsupported(not_outermost_call_on_line) |
| u03 | `requests.get(build(u))` | site (outermost) |
| u04 | `a, b = requests.get(u), requests.get(v)` | two unsupported(several_table_calls_on_line) |
| u05 | callee split over two lines (`requests\n    .get(u)`) | unsupported(multiline_callee) |
| u06 | `f = lambda u: requests.get(u)` | unsupported(lambda_frame) |
| u07 | arguments spread over several lines, callee on the first | site, `line` = the callee's line |
| s01 | call in a method | symbol `Class.method`, bounds of the method |
| s02 | call in a nested function | symbol and bounds per `truth: faultpath-guard` |
| s03 | call in a list comprehension inside a function | per `truth: faultpath-guard` |
| s04 | decorated function | `function_start_line` = the `def` line |
| f01 | `open(p)` / `json.loads(t)` / `os.getenv("X")` / `configparser.ConfigParser().read(p)` / `Path(p).read_text()` | one site each, families filesystem / parse / config / parse / filesystem |
| f02 | `os.environ["X"]` (a subscript, not a call) | none |

- [ ] Write the fixtures, `rows.jsonl`, and a test `matrix_rows_match` that runs
      `inventory::collect` on each fixture and compares with `want`. It does not compile yet (no
      `inventory` module): that is the RED state. Also add `today_no_external_call_rows_for_dependency_calls`
      in `crates/prism-review/tests/cli/targets_test.rs`, which runs the current CLI on fixture h01 with an
      all-lines diff and asserts zero `external_call` rows; it passes on main and is deleted in S2 when the
      behaviour changes (its deletion is named in the S2 PR body).
- [ ] Obtain the `faultpath-guard` truths (needs faultpath) with a script in the evidence directory that
      loads each fixture and a candidate row into faultpath's resolver and site guard and prints the
      refusal reason or "accepted". Record the outputs beside `rows.jsonl`.

### Task T-2: enumeration

- [ ] Run the enumerator tiers (ORCHESTRATION section 6) on `rows.jsonl`. Axes to hand them: import form;
      callee form; receiver route; binding count and binding kind of the receiver; enclosing context; line
      layout; shadowing source; family; Python syntax that changes scoping (`global`, `nonlocal`, walrus,
      `match` capture, class scope, conditional import, import inside the function, `try`/`except
      ImportError` fallbacks, `TYPE_CHECKING` imports). The census's shapes are rows too.
- [ ] The implementer adds accepted rows after each tier. Stop when a full tier round is dry.

## Stage D: design of record (half a day)

- [ ] Amend DESIGN section 6: final tables, route admissions, symbol spelling, context rules, reason list,
      and the accepted-loss list (rows marked `none` that a stronger analysis could list). Both reviewers
      review the amendment and `rows.jsonl` (cap 2 rounds).

---

## Stage I: slices

### Slice S1: call-level identity entry point (behaviour-preserving; half a day)

**Files:**
- Modify: `crates/prism-review/src/targets/dependency_hint.rs`, `dependency_identity.rs`
- Test: `crates/prism-review/src/targets/dependency_hint_tests.rs`

**Interfaces (produced):**

```rust
/// Resolve one call node: callee text as written, the dotted path its root resolves to through imports
/// (None when identity is not proven), and the library-prefix kind used for echo rows.
pub(super) struct CallIdentity { pub source_callee: String, pub resolved_path: Option<String>, pub kind: Option<&'static str> }
pub(super) fn resolve_call(parsed: &ParsedFile, call: tree_sitter::Node<'_>, ctx: &SiteContext<'_>) -> Option<CallIdentity>;
```

`resolve(parsed, line, ctx)` keeps its signature and result and is rewritten to call `resolve_call` on the
candidate it selects.

- [ ] Add tests `resolve_call_reports_resolved_path_for_alias_import` (`import requests as r; r.get(u)` →
      `source_callee == "r.get"`, `resolved_path == Some("requests.get")`, `kind == Some("http")`) and
      `resolve_call_reports_no_path_for_shadowed_root`. Run: they fail to compile.
- [ ] Implement. Run the two tests and the whole `dependency_hint` test modules: all pass.
- [ ] Gate: `cargo test -p prism-review --test cli targets_test::` passes unchanged (G-byte for this slice).
      One reviewer (test and refactor only).

### Slice S2: inventory core, HTTP family, command line, schema 1.1 (3 days)

**Files:**
- Create: `crates/prism-review/src/targets/inventory/{mod.rs,python.rs,py_tables.rs,py_receiver.rs,rows.rs}`
- Create: `docs/contracts/targets-1.1.schema.json` (copied byte for byte from the contract authority; the
  private bindings file gives its location and SHA-256)
- Create: `crates/prism-review/tests/cli/targets_inventory_test.rs`, fixtures under
  `tests/fixtures/targets_inventory/`
- Modify: `crates/prism-review/src/targets/mod.rs` (merge, `schema_version` as a value, `TargetsMeta.inventory`),
  `crates/prism-review/src/cli.rs` (`TargetsArgs`), `crates/prism-review/src/main.rs` (`run_targets`),
  `crates/prism-review/tests/cli/main.rs` (module line), `CLAUDE.md`, `README.md` (targets section)

**Interfaces (produced):**

```rust
// inventory/mod.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family { Http, Filesystem, Parse, Config }
impl Family { pub fn as_str(self) -> &'static str; pub fn parse(s: &str) -> Option<Family>; }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade { Exact, AnnotationTyped }
#[derive(Clone, Debug, Default)]
pub struct InventoryOptions { pub families: BTreeSet<Family> }      // empty = every supported family
pub struct InventorySite { pub file: String, pub line: usize, pub symbol: String,
    pub function_start_line: usize, pub function_end_line: usize, pub family: Family,
    pub resolved_callee: String, pub source_callee: String, pub grade: Grade }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedReason { ModuleLevel, MultilineCallee, NotOutermostCallOnLine,
    SeveralTableCallsOnLine, LambdaFrame, ParseErrorInFunction }
impl UnsupportedReason { pub fn as_str(self) -> &'static str }       // snake_case names of DESIGN 5.2
pub struct UnsupportedSite { pub file: String, pub line: usize, pub symbol: Option<String>,
    pub family: Family, pub resolved_callee: String, pub source_callee: String, pub reason: UnsupportedReason }
pub struct InventoryOutput { pub sites: Vec<InventorySite>, pub unsupported: Vec<UnsupportedSite>,
    pub identity_unproven: usize, pub files: Vec<String> }           // files: sorted, every inventoried file
pub fn collect(files: &BTreeMap<String, prism::ast::ParsedFile>, opts: &InventoryOptions) -> InventoryOutput;

// targets/mod.rs
pub struct TargetsMeta { /* existing fields */ pub inventory: Option<InventoryOptions> }
```

Command line: `--inventory <NAME>` (value `python-deps`); `--inventory-kinds <LIST>`; `--diff` becomes
`required_unless_present = "inventory"`. Rules: `--inventory` without `--diff` forbids an explicit
`--algorithm` other than `none`; an inventory-only run builds no code property graph.

- [ ] Make `matrix_rows_match` compile against the interfaces (HTTP rows and the `none` and `unsupported`
      rows; rows of the three standard-library families are marked `#[ignore = "S3"]`). Confirm RED.
- [ ] Add CLI tests, RED first:

```rust
#[test] fn inventory_only_run_needs_no_diff_and_writes_schema_1_1()   // exit 0; schema_version "1.1"; validates against targets-1.1.schema.json; producer.algorithms == ["prism.inventory/1"]; diff.files == sorted inventoried files
#[test] fn inventory_row_has_the_documented_shape()                   // fixture h05: kind external_call, category dependency_call.http, hint {kind http, callee requests.Session.send}, expected.detail "source callee: s.send", severity info, bounds, symbol
#[test] fn unsupported_site_is_an_other_row_with_a_reason()           // fixture u01: kind other, category dependency_call_unsupported, expected.detail "module_level"
#[test] fn inventory_ids_are_stable_across_runs_and_thread_counts()   // two runs, RAYON_NUM_THREADS 1 and 4: identical bytes
#[test] fn inventory_merges_with_projected_findings()                 // --diff and --inventory: both row sources present, sorted by the existing key
#[test] fn without_inventory_the_document_is_schema_1_0()             // existing fixture: schema_version "1.0", no prism.inventory rows
#[test] fn inventory_without_diff_rejects_a_finding_algorithm()       // --inventory python-deps --algorithm echo, no --diff: clap error
#[test] fn vendored_1_1_schema_matches_its_recorded_hash()            // SHA-256 of docs/contracts/targets-1.1.schema.json equals a constant in the test
```

- [ ] Implement `py_tables.rs` (HTTP entries only), `py_receiver.rs` (R1 to R5 as fixed in stage D),
      `python.rs` (call walk, enclosing definition, site rules), `rows.rs`, then the merge in `mod.rs` and
      the CLI wiring. The inventory calls `dependency_hint::resolve_call` for identity; it adds no second
      import resolver.
- [ ] Review focus items 1 to 5: one test each, named after the item.
- [ ] Delete `today_no_external_call_rows_for_dependency_calls` and say so in the PR body.
- [ ] GREEN. Gates of ORCHESTRATION section 5: items 1 to 4, 8, 9, 10 (no cache, data-flow or resolution
      change: 5 to 7 do not apply).
- [ ] Measure before review: run the built binary on httpie; record rows per family and per unsupported
      reason, the identity-unproven count, and whether the two HTTP overlay sites are rows with the
      overlay's file, line, bounds and kind (**G-overlay**). Put the table in the PR body.
- [ ] Two reviewers. Merge. Comment on #613 with the httpie table.

### Slice S3: standard-library families and scan parity (2 to 3 days)

**Files:**
- Modify: `crates/prism-review/src/targets/inventory/py_tables.rs`, `py_receiver.rs` (constructors such as
  `configparser.ConfigParser`, `pathlib.Path`), `inventory_matrix_tests.rs` (un-ignore the S3 rows)
- Create (evidence directory): `parity/compare.py`

- [ ] Un-ignore the S3 matrix rows: RED. Add the three families' entries. GREEN.
- [ ] `parity/compare.py <prism.json> <scan.json>`: key each injectable row by (file, line,
      function_start_line, function_end_line, family, resolved callee); print the two set differences.
- [ ] Run `faultpath scan` (needs faultpath) and `prism targets --inventory python-deps --inventory-kinds
      filesystem,parse,config` on httpie, black and one more corpus, each on a clean checkout. Classify
      every difference in `parity/PARITY.md` as: prism misses a site scan lists (add the table entry or
      route, or record why not); prism lists a site scan does not (confirm against the source, or remove);
      the two disagree on bounds, line or callee (decide by the source and faultpath's guard).
- [ ] **G-parity:** after fixes, every remaining difference is listed with its class and reason, and none
      is "prism lists a site that is not that library call". If faultpath could not be installed, compare
      against the independent census of Task M-2 instead and mark G-parity as owed.
- [ ] **G-precision:** a Sonnet-5.5 reader checks 100 sampled HTTP rows across the census corpora against
      the source. At most 1 may be wrong; each wrong row becomes a matrix row and is fixed.
- [ ] Two reviewers. Merge.

### Slice S4: the pipeline on httpie, funnel report, close-out (2 days)

**Files (evidence directory):** `e2e/targets-httpie.json`, `e2e/funnel.py`, `e2e/REPORT.md`.
**Files (repository):** `docs/features/targets-inventory.md` (what is listed, the families and tables, the
routes, the unsupported reasons, what is out of scope).

- [ ] Produce `e2e/targets-httpie.json` with the merged binary: `prism targets --repo <httpie> --inventory
      python-deps --out …`.
- [ ] **G-accept** (needs faultpath): load the document with faultpath's loader; run its planning step for
      every injectable row without running the suite; tabulate refusal reasons. Required: no
      `stale_function_bounds`, `stale_site_symbol` or `not_injectable_kind`. Any such refusal is a prism
      defect: add a matrix row, fix, re-run.
- [ ] **G-live** (needs faultpath; one working day at most): follow the pilot run notes in the private
      bindings file to repeat the last httpie pilot attempt with `e2e/targets-httpie.json` in place of the
      overlaid document and no overlay. No host-quiet marker; one no-fault re-run for every newly failing
      test before counting a verdict. Required: both HTTP overlay sites are kept by the census and end
      with the outcomes of the last overlay run (`client.py:114` reached and injected, same verdicts;
      `update_warnings.py:44` reported as shadowed by the suite's mock). If the run cannot be reproduced within
      the day without changing faultpath, stop, write what was reached and what blocked, and leave G-live
      for the owner to route (DESIGN decision 6).
- [ ] `e2e/funnel.py` joins the independent census, the prism document, faultpath's census output and the
      run results into the funnel of DESIGN section 8. `e2e/REPORT.md` holds the funnel for httpie, the
      loss reasons per step, and the sites prism found beyond the overlay with their outcomes.
- [ ] Write `docs/features/targets-inventory.md`; update the `targets/` paragraph of `CLAUDE.md`.
- [ ] Close #613 with the funnel. File: (a) an issue in the faultpath repository proposing how prism rows
      and scan rows should coexist, with the parity numbers (DESIGN decision 7); (b) a prism issue for the
      provenance noise on `requests.*` lines (the `request` substring match and the `fetch` → database
      hint); (c) a prism issue listing the families and languages left out.
- [ ] Refresh the handoff; delete the worktree's `target/`.

## Acceptance for the lane

G-byte, G-overlay, G-parity and G-precision hold on the merged head; G-accept holds; G-live either holds or
is recorded as owed with the blocking reason; the funnel report is posted on #613; both reviewers approved
S2 and S3.
