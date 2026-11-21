# Lane 5: schema-aware service impact — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** measure whether schema operations and fields can be bound to producer handlers and consumer call
sites on a real pair, and, on an owner go-ahead, build a pilot that reports the code locations a schema
change affects.

**Architecture:** the feasibility study is a Python prototype under `eval/schema_impact/` plus a history replay;
the pilot is a separate workspace crate `crates/prism-boundary` that reads prism's parsed files and call
graph and writes two JSON documents (bindings per repository, impact per change).

**Tech stack:** Python 3.12 and uv (the existing `eval` project), tree-sitter Python bindings, TypeScript
5.9.3 as the field-read oracle; Rust, `serde_json`, `clap`, the `prism` core library for the pilot.

**Spec:** `DESIGN.md` in this directory. Sections named below (F1 to F7, G1 to G4, 5.1 to 5.7) are its.

## Global constraints

- No product-crate change in Part 1. Part 2 starts only after the owner go-ahead (DESIGN decision 4).
- Corpus P is `bench-repos/apache-answer` at `6744295f`. Work on a clone under the evidence directory, never
  in `~/code/bench-repos`.
- Pair R is private: ORCHESTRATION section 9 applies to every step that touches it. Its paths and client
  configuration are read from `~/prism-evidence/lanes-2026-11-20/PRIVATE-BINDINGS.md`.
- Binding grades are `bound`, `candidate`, `unbound`. The words `Exact` and `NameOnly` are not used for
  bindings.
- Element ids follow DESIGN 5.1 exactly: `op:<METHOD> <path>` with `{}` parameters and no trailing slash;
  `field:<definition>#/<property path>`.
- The pilot reads JSON schema files only.
- Evidence directory: `~/prism-evidence/boundary-impact/` (`LEDGER.md`, `HANDOFF.md`, `feasibility/`,
  `private/`).

## Review focus

Inputs the design implies but no single task's happy path exercises. Each is pinned by a test in the task
named.

1. Two schema paths that normalise to the same id (`/a/{id}` and `/a/{name}`): keep the first, emit a
   warning, never merge silently. (F1 task `schema.py`; pilot P1.)
2. A recursive `$ref` (a definition that contains itself): `definitions_of` terminates and each field is
   listed once under its own definition. (F-1; pilot P1.)
3. A client call inside an anonymous callback or a hook body: the binding is still recorded, with an empty
   or anonymous caller list, not dropped. (F3 task; P3.)
4. A URL whose only dynamic part is the query string (`/x?title=${t}`): classed as a literal path after the
   cut, not as dynamic. (F3 task; P3.)
5. A handler with two `@Router` lines, and a `@Router` comment separated from the function by a blank line:
   the first binds two operations to one function; the second binds nothing (it is not the function's doc
   comment). (F2 task; P2.)

---

# Part 1: feasibility study (3 to 5 days)

Branch `eval/schema-impact-feasibility`, worktree `~/code/prism-boundary` (removed after Task F-8 merges;
Part 2 creates it again). Gates for Part 1 tasks: `cd eval && uv run --group schema-impact pytest
tests/schema_impact -q`, the private-name check before every commit, and CI on the PR. Items 5 to 9 of
ORCHESTRATION section 5 concern Rust product code and do not apply. Task order: F-0, F-1, then F-2 and F-3
in either order, then F-5 (needs F-1 and F-3 only), F-4 (needs the build), F-6, F-7, F-8.

### Task F-0: workspace, corpus, issue

**Files:** none in the repository.

- [ ] Create the worktree and evidence directory (ORCHESTRATION section 4).
- [ ] `git clone ~/code/bench-repos/apache-answer ~/prism-evidence/boundary-impact/corpus/apache-answer`
      and `git -C … checkout --detach 6744295f`. Record the commit in `LEDGER.md`. All schema work uses
      `docs/swagger.json` (the YAML twin is not read); record that the two files list the same number of
      paths at the pinned commit.
- [ ] Client dependencies for the F3 oracle: in the clone's `ui/` directory, install with the package
      manager its lockfile names, scripts disabled (`--ignore-scripts`) and the lockfile frozen. This needs
      the network. If it fails, write "F3 oracle: reader-labelled" in the ledger; Task F-4 then uses a
      reader for every operation.
- [ ] File a tracking issue in the prism repository titled "Schema-aware service impact: feasibility study",
      linking this directory. Record its number in `LEDGER.md`; later tasks reference it.

### Task F-1: prototype package, schema reader, ids

**Files:**
- Create: `eval/schema_impact/__init__.py`, `eval/schema_impact/cli.py`, `eval/schema_impact/ids.py`,
  `eval/schema_impact/schema.py`
- Create: `eval/tests/schema_impact/test_si_ids.py`, `eval/tests/schema_impact/test_si_schema.py`,
  `eval/tests/schema_impact/fixtures/mini.swagger.json` (test file names must stay unique across
  `eval/tests/`, which has no package files; the existing `test_boundary.py` belongs to another tool)
- Modify: `eval/pyproject.toml` (dependency group `schema-impact = ["pyyaml>=6", "tree-sitter>=0.23",
  "tree-sitter-go", "tree-sitter-typescript", "tree-sitter-javascript"]`; script
  `schema-impact = "schema_impact.cli:main"`; add `"schema_impact"` to the wheel packages), `eval/uv.lock`
  (regenerated; commit it)

**Interfaces (produced):**
- `ids.normalize_path(raw: str, base: str = "") -> str`
- `ids.op_id(method: str, path: str, base: str = "") -> str`
- `ids.field_id(owner: str, props: list[str]) -> str`
- `schema.load(path_or_bytes) -> SchemaIndex` with `.operations: dict[str, Operation]`, `.fields: dict[str, Field]`,
  `.definitions_of(op_id) -> set[str]`, `.response_fields(op_id, response_root: str) -> list[str]` (the
  top-level property names the client receives), `.sha256`, `.info_version`, `.warnings: list[str]`
- `schema.diff(old: SchemaIndex, new: SchemaIndex) -> list[Change]` where `Change.kind` is one of
  `op_removed`, `op_added`, `field_removed`, `field_added`, `field_type_changed`, `field_required_added`,
  `enum_values_changed`. There is no rename kind: a rename appears as a removal and an addition.
- CLI: `schema-impact ids <schema.json>` prints `ids.tsv` (one id per line, operations then fields, sorted).

All id rules are the two tables of DESIGN 5.1.

- [ ] **Write the failing tests.**

```python
def test_normalize_path():
    assert normalize_path("/answer/api/v1/question/{id}/") == "/answer/api/v1/question/{}"
    assert normalize_path("question/{id}", base="/answer/api/v1") == "/answer/api/v1/question/{}"
    assert normalize_path("/", base="/") == "/"
    assert normalize_path("/a//b", base="") == "/a/b"
    assert normalize_path("/f/{id}.json") == "/f/{}.json"

def test_op_id():
    assert op_id("get", "/a/{x}/b") == "op:GET /a/{}/b"

def test_field_ids():
    assert field_id("schema.GetAPIKeyResp", ["access_key"]) == "field:schema.GetAPIKeyResp#/access_key"
    assert field_id("A", ["items", "[]", "x"]) == "field:A#/items/[]/x"

def test_load_mini_swagger():
    idx = load(FIXTURE)                       # 3 paths, 4 operations plus a `parameters` key that is not an operation
    assert len(idx.operations) == 4
    assert "op:GET /answer/api/v1/question/{}" in idx.operations
    assert "field:schema.QuestionInfo#/title" in idx.fields
    assert "schema.QuestionInfo" in idx.definitions_of("op:GET /answer/api/v1/question/{}")   # through an allOf envelope
    assert "field:resp:GET /answer/api/v1/question/{}:200#/data" in idx.fields
    assert idx.response_fields("op:GET /answer/api/v1/question/{}", "data") == ["id", "title"]

def test_recursive_definition_terminates():
    idx = load(RECURSIVE_FIXTURE)             # Node { children: [ $ref Node ] }
    assert "field:Node#/children" in idx.fields and idx.definitions_of("op:GET /n") == {"Node"}

def test_duplicate_normalized_path_keeps_the_first_sorted_and_warns():
    idx = load(DUPLICATE_FIXTURE)             # /a/{id} and /a/{name}
    assert [o for o in idx.operations if o.startswith("op:GET /a/")] == ["op:GET /a/{}"]
    assert idx.operations["op:GET /a/{}"].source_path == "/a/{id}"
    assert any("duplicate" in w for w in idx.warnings)

def test_diff_kinds():
    kinds = {c.kind for c in diff(load(OLD), load(NEW))}
    assert kinds == {"op_removed", "field_removed", "field_type_changed"}
```

- [ ] Run `cd eval && uv run --group schema-impact pytest tests/schema_impact -q`; expect import failures.
- [ ] Implement the three modules. The reader accepts Swagger 2.0 and OpenAPI 3 in JSON (YAML too, for
      convenience); `$ref` resolution is local to the file.
- [ ] Run the tests; expect all to pass. Write `feasibility/ids.tsv` for the pinned schema (it becomes the
      pilot's golden file). Commit (`eval(schema-impact): schema reader and element ids`).

### Task F-2: producer extraction and measure F1

**Files:**
- Create: `eval/schema_impact/producer_go.py`, `eval/tests/schema_impact/test_si_producer_go.py`,
  `eval/tests/schema_impact/fixtures/go_mini/` (three `.go` files)

**Interfaces (produced):**
- `producer_go.annotations(repo: Path) -> list[Binding]`: one per `@Router <path> [<method>]` line in the doc
  comment of a function or method declaration, in every `.go` file that is not a `_test.go` file.
  `Binding(element, file, line, function, receiver_type, via="swaggo_annotation")`; `function` is the bare
  function or method name. A doc comment is the run of `//` lines directly above the declaration with no
  blank line between; a bare `//` line does not end it.
- `producer_go.registrations(repo: Path) -> list[Registration]`: calls `<x>.<METHOD>("<literal>", <handler>)`
  with METHOD one of GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS;
  `Registration(method, path_suffix, handler_name, file, line)` where `handler_name` is the last identifier
  of the handler expression (`a.langController.GetLangMapping` → `GetLangMapping`).
- CLI: `schema-impact producer --repo <P> --schema <file> --out <dir>` writes `producer.jsonl` and `f1.json`.

- [ ] **Write the failing tests** on the fixture: a method with one annotation; a function with two
      annotations (two bindings, same function); an annotation followed by a blank line before `func`
      (no binding); an annotation block containing a bare `//` line (still one block); a registration
      `g.GET("/x", c.Handler)` (suffix `/x`, handler name `Handler`).
- [ ] Run; expect failure. Implement. Run; expect pass. Commit.
- [ ] Run on P. `f1.json` holds, over **all** schema operations: bound to exactly one function; to none; to
      several; annotations naming no schema operation. And the cross-check: for each registration, the
      schema operations with the same method whose path ends with the registration's suffix; where exactly
      one such operation exists and it has exactly one annotation binding, whether `handler_name` equals
      that binding's `function`. Report the size of that cross-check population as well as the agreement.
- [ ] Record the F1 table in `feasibility/F1.md` with the prediction written before the run.

### Task F-3: consumer extraction and measure F2

**Files:**
- Create: `eval/schema_impact/consumer_ts.py`, `eval/schema_impact/match.py`, `eval/schema_impact/config.py`,
  `eval/tests/schema_impact/test_si_consumer_ts.py`, `eval/tests/schema_impact/test_si_match.py`,
  `eval/tests/schema_impact/fixtures/ts_mini/` (a wrapper module, a services file, a component using a hook),
  `eval/schema_impact/configs/apache-answer.toml`

**Interfaces (produced):**
- `config.load(path) -> list[ClientSpec]` (the TOML of DESIGN 5.4, including `response_root`).
- `consumer_ts.calls(repo: Path, clients: list[ClientSpec], root: str) -> list[ClientCall]` with
  `ClientCall(file, line, method, url_text, pattern, url_class, response_type, enclosing_function)`;
  `url_class` is `literal`, `template` or `dynamic`; `response_type` is the text of an explicit generic
  argument (`request.get<Type.X>(…)`, `useSWR<Type.X>(…)`) or `None`.
- `match.bind(call: ClientCall, idx: SchemaIndex) -> (grade, list[op_id], reason)`.
- CLI: `schema-impact consumer --repo <P> --root ui/src --schema <file> --config <toml> --out <dir>` writes
  `consumer.jsonl`, `f2.json`, and `f2-sample.md`.

**Counting rule (the F2 denominator).** A client call site is a call expression in a non-test file under
`--root` whose callee is a configured client method (the receiver bound to an import of the configured
module and never rebound in the file) or a configured hook. For a hook, the hook call is the site; a client
call inside the hook's inline fetcher, and a client method passed as the fetcher, are not sites. Test files
are those matching `*.test.*`, `*.spec.*` or under `__tests__`.

- [ ] **Write the failing tests.**

```python
def test_url_classes():
    assert pattern("'/answer/api/v1/file'") == ("/answer/api/v1/file", "literal")
    assert pattern("`/answer/api/v1/question/${id}`") == ("/answer/api/v1/question/{}", "template")
    assert pattern("`/answer/api/v1/question/similar?title=${title}`") == ("/answer/api/v1/question/similar", "literal")
    assert pattern("`/a/item-${id}`")[1] == "dynamic"
    assert pattern("`${base}/x`")[1] == "dynamic"
    assert pattern("'/a/' + id") == ("/a/{}", "template")
    assert pattern("id ? `/a/${id}` : null") == ("/a/{}", "template")

def test_match_scoring():
    idx = index_with("op:GET /user/{}", "op:GET /user/me", "op:POST /user/{}")
    assert bind(call("GET", "/user/{}"), idx)[:2] == ("bound", ["op:GET /user/{}"])
    assert bind(call("GET", "/user/me"), idx)[:2] == ("bound", ["op:GET /user/me"])
    assert bind(call("DELETE", "/user/{}"), idx)[0] == "unbound"

def test_calls_fixture():
    calls = consumer_ts.calls(FIXTURE, load(CONFIG), "src")
    assert {c.url_class for c in calls} == {"literal", "template", "dynamic"}
    assert any(c.file.endswith("Widget.tsx") for c in calls)      # a call inside a hook callback is kept
    assert not any(c.line == SHADOWED_LINE for c in calls)        # `request` rebound to a parameter: not a client
    assert any(c.line == CONST_URL_LINE and c.pattern == "/a/list" for c in calls)   # const followed one step
    assert sum(1 for c in calls if c.line == HOOK_LINE) == 1      # hook site counted once
    assert not any(c.line == HOOK_INLINE_FETCHER_LINE for c in calls)                # inner fetcher call not a site
    assert any(c.response_type == "Type.Item" for c in calls)
```

- [ ] Run; expect failure. Implement. Run; expect pass. Commit.
- [ ] Write `configs/apache-answer.toml` from the client's source: the default export of its request
      module with the verb methods that class defines (read them; do not assume `patch`), `response_root`
      set to what the wrapper's response handling returns to callers, and the `useSWR` hook with
      `url_arg = 0`. If a second client module exists, add it when it is a verb wrapper, or record it and
      its call count as excluded when it is not.
- [ ] Run on P. `f2.json` holds: client call sites; counts per `url_class`; unique, several, none; the
      reasons for none.
- [ ] `f2-sample.md`: sort the unique matches by (file, line), then take 50 with `random.Random(20261120)`
      (all of them if fewer). Each row shows the call's source line, the operation id and the schema's
      summary line for it. Dispatch one Sonnet-5.5 reader (medium): "does this call address this operation?
      yes / no / cannot tell". G2 counts "no" answers; "cannot tell" answers are reported and left out.
      Record the tally in `feasibility/F2.md`.

### Task F-4: field reads and measure F3

**Files:**
- Create: `eval/schema_impact/fields.py`, `eval/schema_impact/oracle/field_refs.mjs`,
  `eval/tests/schema_impact/test_si_fields.py`

**Interfaces (produced):**
- `fields.select_operations(idx, calls, n=12, seed=20261120) -> list[op_id]`: eligible operations have a
  uniquely bound client call with a `response_type` and an object-valued response under `response_root`.
  Classes: *list* (GET whose response under the root is an array or has an array property named `list`),
  *detail* (other GET), *mutation* (POST, PUT, DELETE). Sort eligible operations by id, shuffle with the
  seed, take up to 4 per class, and fill a short class from the others. Record the class sizes.
- `fields.start_lines(call, callers_json) -> list[(file, line)]`: the call's own line, and, when the call is
  the returned value of its enclosing function (a `return` operand or an arrow body), the line of every
  call site of that function that `prism nav callers --symbol <function> --file <file> --format json`
  reports (depth 1).
- `fields.reads_from_edges(edges_jsonl, start_lines, response_root, max_hops=6) -> set[(file, line, path)]`:
  start from every edge endpoint that is a definition on a start line; follow edges forward, of any label,
  across functions, up to `max_hops`; collect use endpoints whose access path has at least one field,
  rendered `f1.f2` from the record's `path` object (base dropped). `response_root` is the configured root:
  empty when the client already returns the part under it.
- `oracle/field_refs.mjs <tsconfig> <type name> <property>`: prints `file:line` of every reference to that
  property, with the TypeScript at `~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3`.

**Scoring.** For an operation, the fields are `idx.response_fields(op, response_root)`. *Found*: an oracle
reference and a prism read at the same file and line whose path starts with that field. *Missed*: an oracle
reference with no such read. *Wrong*: a prism read whose first path component is not one of the fields.
Only top-level fields are scored.

- [ ] Unit-test `reads_from_edges` on a hand-written JSONL in the shape `nav dfg-stats --edges` prints
      (`{"from":{"file","line","path":{"base","fields"},"access"},"to":{…},"confidence":…}`): a call-line
      definition, a reassignment, two member reads, one cross-function edge, one unrelated edge; expect the
      member paths. Implement. Commit.
- [ ] Build prism for the dump: `~/prism-evidence/tools/memguard.sh 8 cargo build --release` in the
      worktree; `target/release/prism --version` must show the worktree's `git rev-parse --short HEAD`.
- [ ] Dump edges once: `memguard.sh 8 target/release/prism nav --no-cache dfg-stats --repo <P clone>
      --edges > feasibility/edges.jsonl`.
- [ ] For the selected operations: oracle references from `field_refs.mjs` with the client's `tsconfig.json`
      (or, if Task F-0 could not install the dependencies, a Sonnet-5.5 reader lists the reads of each
      field by reading the call's file and its callers). Compare with `reads_from_edges`.
- [ ] `feasibility/F3.md`: per operation: fields, oracle reads, found, missed, wrong; the totals; and the
      three most common reasons for a miss (for example: value stored in component state, value passed
      through a hook).

### Task F-5: history replay and measure F4

**Files:**
- Create: `eval/schema_impact/replay.py`, `eval/schema_impact/render.py`,
  `eval/tests/schema_impact/test_si_replay.py`

**Interfaces (produced):**
- `replay.schema_commits(repo, schema_path) -> list[str]`: first-parent commits that changed the file,
  newest first (`git log --first-parent --format=%H -- <path>`).
- `replay.usable(repo, commit, schema_path) -> list[Change]`: the diff between the first parent's and the
  commit's schema, kept when it has at least one `op_removed`, `field_removed`, `field_type_changed`,
  `field_required_added` or `enum_values_changed`; empty otherwise. A schema that fails to load at either
  side gives an empty list and a logged reason.
- `replay.replay_one(repo, commit, schema_path, config, root, window=10) -> ReplayResult` with `changes`,
  `predicted`, `touched_same_commit`, `touched_later`, `late_fixes`, `skipped_reason`.
- `render.report(result) -> str` (Markdown: the change, producer locations, consumer locations, what
  happened next in history).
- CLI: `schema-impact replay --repo <P clone> --schema docs/swagger.json --config <toml> --root ui/src
  --limit 60 --out <dir>`.

**Definitions.**
- *Predicted* (computed at the first parent, by the prototype extractors only; prism is not run, so no
  callers): for a removed operation, every uniquely bound client call site of it; for a field change in
  owner `D`, every uniquely bound call site of an operation with `D` in `definitions_of`, kept only when the
  file containing the call, or a file that imports the call's enclosing function by name, has a line
  matching the field name as a property (`.name`, `name:`, `"name"`, `'name'`, or `{ name` in a
  destructuring).
- *Touched in commit X*: X's diff for the predicted file has a hunk whose old-side range comes within 3
  lines of the call's line, or, for a field, whose removed or added lines match the field-name pattern.
- *Touched in the same commit*: X is the schema commit. *Touched later*: X is one of the next `window`
  first-parent commits after it.
- *Late fix*: a predicted location touched later and not touched in the same commit.
- A commit whose configured client module does not exist at the parent is skipped with
  `skipped_reason = "config_missing"`.

- [ ] Unit-test `replay_one` on a three-commit repository built in the test (commit 1: schema and client;
      commit 2: schema removes an operation; commit 3: client stops calling it): one predicted location,
      not touched in the same commit, one late fix. A second case where the client is updated in commit 2:
      zero late fixes. Implement. Commit.
- [ ] First count usable commits (`usable` over `schema_commits`, no extraction) and record the number.
- [ ] Run the replay on the newest usable commits until 60 schema commits have been examined or history
      ends. Each replayed commit uses a detached scratch worktree of the clone at the first parent, removed
      afterwards.
- [ ] `feasibility/F4.md`: schema commits examined; usable; skipped by reason; elements with a non-empty
      prediction; share of predicted locations touched in the window; late fixes with commit ids; and ten
      rendered reports (`feasibility/reports/01.md` … `10.md`), late fixes first, then the largest
      predictions.

### Task F-6: pair R and measure F5 (private)

**Files:**
- Create: `eval/schema_impact/producer_route_table.py`,
  `eval/tests/schema_impact/test_si_route_table.py` (public fixture only)
- Private, not committed: the R configuration and every output, under `~/prism-evidence/boundary-impact/private/`

- [ ] Skip this task, and write "F5 owed" in the ledger, if the private bindings file does not list pair R.
- [ ] `producer_route_table.routes(repo) -> list[Route]`: object literals with string `method` and `path`
      properties inside an array passed to a `.route(` call. Unit-test on a public three-route fixture.
- [ ] Run it on `pilot-producer`; run `consumer_ts.calls` on `pilot-consumer` with a private TOML describing
      its client object; bind calls against the route set (the route table is the schema source, DESIGN 5.3).
- [ ] Private `F5.md`: route entries, extracted with method and path; client calls per URL class; unique,
      several, none; top reasons for none. Public memo: the three percentages only.
- [ ] Run the private-name check of ORCHESTRATION section 9 on the staged diff before committing the public
      extractor and fixture.

### Task F-7: external differ (optional, 15 minutes)

- [ ] Try to fetch an `oasdiff` release binary into the evidence directory (no system package change). If
      it runs, compare its changed-element list with `schema.diff` on five replay commits and note
      disagreements in `feasibility/F7.md`. If it is not running within 15 minutes, write that and stop.

### Task F-8: memo, review, stop

**Files:**
- Create: `docs/superpowers/plans/2026-11-20-lanes/lane5-schema-impact/FEASIBILITY.md`

- [ ] Write the memo per DESIGN section 6: the measures table (with the prism build's commit), gates G1 to
      G4 with pass or fail, the ten reports (linked from the evidence directory; quote three in full), the
      recommended pilot pair and adapters per the table in DESIGN section 7, and a day estimate per pilot
      slice revised from F6. If G3 has fewer than 2 late fixes, write "owner rating pending" beside it.
- [ ] Write `~/prism-evidence/boundary-impact/roadmap-corrections.md` for the owner: the statements in the
      tooling roadmap's boundary and language-status sections that the research record (section C) found
      stale, each with the current fact and its code reference. The tooling repository itself is not edited
      by this lane.
- [ ] Open one PR with `eval/schema_impact/`, its tests, `eval/uv.lock` and the memo. One reviewer
      (Sonnet-5.5, high, careful reading: do the numbers in the memo match the JSON outputs; do the tests
      pin Review focus items 1, 3, 4, 5). Fold, merge on approval and green CI.
- [ ] Post the gate table, the recommendation and the ten reports on the tracking issue. Refresh
      `HANDOFF.md`. Remove the worktree.
- [ ] **Stop. The pilot needs the owner's go-ahead** (DESIGN decision 4). Do not start Part 2 on a default.

---

# Part 2: pilot (15 to 25 days; provisional until the memo)

Before dispatching P1, re-read `FEASIBILITY.md` and apply DESIGN section 7's table: it may drop P5's
consumer half, add an adapter to P2, or reorder for pair R. Record the applied row in the ledger.

Branch per slice from `main`, worktree `~/code/prism-boundary`. Lifecycle stages T and E of
ORCHESTRATION section 3 apply to P2 and P3 (their rows come from F1 and F2's "none" and "several" reasons
plus the enumerator tiers); P1 and P4 are specified closely enough to go straight to implementation.

### File map

```
crates/prism-boundary/
  Cargo.toml                 depends on prism (path), serde, serde_json, clap, anyhow, sha2
  README.md                  usage, the documents, the facade candidates it uses
  src/lib.rs                 public API re-exports
  src/main.rs                clap: bind | diff | impact
  src/element.rs             ElementId, normalize_path_template
  src/model.rs               serde DTOs: BindingsDocument, Binding, Unbound, ImpactDocument, ElementChange
  src/revision.rs            repo rev and dirty flag (git), schema sha256
  src/schema/mod.rs          SchemaIndex, Operation, Field
  src/schema/openapi.rs      Swagger 2.0 and OpenAPI 3 JSON reader
  src/schema/diff.rs         structural diff
  src/producer/mod.rs        bind_producer, adapter selection
  src/producer/swaggo.rs     annotation adapter
  src/producer/go_struct_fields.rs
  src/consumer/mod.rs        bind_consumer
  src/consumer/config.rs     prism-boundary.toml
  src/consumer/client_calls.rs
  src/consumer/url_pattern.rs
  src/consumer/matcher.rs
  src/consumer/field_reads.rs        (only if F3 is positive)
  src/impact.rs              join and warnings
  tests/                     one integration test file per slice, fixtures under tests/fixtures/
docs/contracts/boundary-bindings.schema.json
docs/contracts/boundary-impact.schema.json
```

Root `Cargo.toml`: add the member; do not add it to `default-members`.

### Slice P1: crate, element ids, schema reader, diff, documents (2 to 3 days)

**Interfaces (produced):**

```rust
pub struct ElementId(String);                       // Display, Ord, Serialize as string
impl ElementId {
    pub fn operation(method: &str, path_template: &str, base: &str) -> ElementId;
    pub fn field(definition: &str, properties: &[&str]) -> ElementId;
}
pub fn normalize_path_template(raw: &str) -> String;
pub enum SchemaFormat { Swagger2, OpenApi3 }
pub struct SchemaIndex { pub format: SchemaFormat, pub sha256: String, pub info_version: Option<String>,
    pub operations: BTreeMap<ElementId, Operation>, pub fields: BTreeMap<ElementId, Field>, pub warnings: Vec<String> }
impl SchemaIndex { pub fn definitions_of(&self, op: &ElementId) -> BTreeSet<String>;
                   pub fn operations_reaching(&self, definition: &str) -> BTreeSet<ElementId>; }
pub fn index_schema(bytes: &[u8]) -> anyhow::Result<SchemaIndex>;
pub enum ChangeKind { OpRemoved, OpAdded, FieldRemoved, FieldAdded, FieldTypeChanged, FieldRequiredAdded, EnumValuesChanged }
pub struct ElementChange { pub element: ElementId, pub kind: ChangeKind }
pub fn diff_schemas(old: &SchemaIndex, new: &SchemaIndex) -> Vec<ElementChange>;
```

**Tests** (`tests/schema_test.rs`; fixtures are two trimmed real revisions of P's `swagger.json`, five
operations each): the Python tests of Task F-1 ported one for one, plus
`ids_match_the_prototype_golden_file` (the ids of the pinned schema equal `feasibility/ids.tsv`, copied to
`tests/fixtures/ids.tsv`, byte for byte), `openapi3_first_server_path_is_prefixed`,
`addition_only_change_has_no_removed_or_changed_kind`, `documents_validate_against_contract_schemas`
(serialise one of each document and check it with the hand-written schema walker the targets contract tests
use, copied into this crate's tests; the two new schema files use only the keywords that walker supports:
`type`, `required`, `properties`, `additionalProperties: false`, `enum`, `const`, `items`).

- [ ] Tests first (RED: crate does not exist), implement, GREEN, gates of ORCHESTRATION section 5
      (items 1 to 4, 8, 9; no cache or data-flow change, so 5 to 7 do not apply), PR, two reviewers.

### Slice P2: producer bindings (3 to 4 days)

**Interfaces (produced):**

```rust
pub struct ProducerOptions { pub adapters: Vec<ProducerAdapter> }       // default: [SwaggoAnnotation, GoStructField]
pub fn bind_producer(session: &prism::navigation::NavigationSession, schema: &SchemaIndex,
                     repo: &RepoIdentity, opts: &ProducerOptions) -> BindingsDocument;
```

CLI: `prism-boundary bind --role producer --repo <dir> --schema <file.json> --out <bindings.json>`.

**Seed rows** (stage T adds F1's residue; stage E runs the tiers):

| Row | Input | Required |
|---|---|---|
| g01 | one `@Router` line in a method's doc comment | one `bound` binding to that method |
| g02 | two `@Router` lines on one function | two bindings, same function |
| g03 | annotation, blank line, `func` | no binding; the operation is `operation_without_producer` |
| g04 | annotation path not in the schema | listed under `unbound` as `annotation_without_operation` |
| g05 | the same operation annotated on two functions | both as `candidate` |
| g06 | struct field tagged `json:"name,omitempty"` | `field:<pkg>.<T>#/name` bound |
| g07 | field tagged `json:"-"` | nothing |
| g08 | untagged exported field | `unbound`, reason `untagged_field` |
| g09 | embedded struct | fields bind under the embedded type's definition, not the outer one |
| g10 | generated file (`// Code generated … DO NOT EDIT.`) holding annotations | bindings kept; a note in `evidence` |

**Gate:** on P, bound operations at least F1's prototype figure minus 2 points; zero `bound` bindings that
the F1 cross-check contradicts without a recorded explanation.

### Slice P3: consumer bindings (3 to 4 days)

**Interfaces (produced):**

```rust
pub struct ClientSpec { /* module, export, methods, hook, url_arg, base_path, response_root */ }
pub fn load_config(path: &Path) -> anyhow::Result<Vec<ClientSpec>>;
pub enum UrlClass { Literal, Template, Dynamic }
pub fn url_pattern(parsed: &prism::ast::ParsedFile, arg: tree_sitter::Node) -> (Option<String>, UrlClass);
pub fn match_operation(method: &str, pattern: &str, schema: &SchemaIndex) -> MatchResult;   // Bound(id) | Candidates(ids) | Unbound(reason)
pub fn bind_consumer(session: &prism::navigation::NavigationSession, schema: &SchemaIndex,
                     repo: &RepoIdentity, clients: &[ClientSpec], caller_depth: usize) -> BindingsDocument;
```

CLI: `prism-boundary bind --role consumer --repo <dir> --schema <file.json> --config <toml> --caller-depth 2 --out <bindings.json>`.

**Seed rows:** every assertion of Task F-3's tests, plus:

| Row | Input | Required |
|---|---|---|
| c01 | client imported under another local name | recognised |
| c02 | client name rebound by a parameter or local | not a client call |
| c03 | a call of a method the configuration does not list (`request.upload(u)`); a client method passed as a hook's fetcher | the call: `unbound`, reason `unknown_client_method`; the fetcher argument: not a call site |
| c04 | hook key that is an array | `unbound`, reason `non_string_key` |
| c05 | URL `const` assigned twice | not followed; `dynamic` |
| c06 | call in an anonymous callback | binding kept; `callers` may be empty |
| c07 | `fetch(url, { method: 'POST' })` and `fetch(url)` | POST and GET |
| c08 | two operations tie on score | both `candidate` |
| c09 | `base_path` set | prefixed before matching |
| c10 | call bound, enclosing function called from two components | both in `callers` at depth 2 |

**Gate:** on P, unique matches at least F2's figure minus 2 points; a fresh 50-row sample of `bound`
bindings has at most 2 wrong. Tier-A does not apply (no resolution change).

### Slice P4: diff and impact commands (3 days)

**Interfaces (produced):**

```rust
pub fn impact(changes: &[ElementChange], old: &SchemaIdentity, new: &SchemaIdentity,
              bindings: &[BindingsDocument]) -> ImpactDocument;
```

CLI: `prism-boundary diff --old <a.json> --new <b.json>`;
`prism-boundary impact (--old <a.json> --new <b.json> | --changed <elements.json>) --bindings <file>... --format json|md`.

**Rows:** removed operation with a bound consumer → listed with its callers; changed field → every
operation reaching its definition, their consumer bindings marked `scope: "operation"`, and field-read
bindings marked `scope: "field"` when present; a bindings document computed against another schema hash →
`schema_mismatch` warning naming both hashes; `repo.dirty` → `rev_dirty` warning; a changed element with no
binding in any document → listed under `unobserved`, never omitted; `candidate` bindings appear in a
separate list from `bound`.

**Gate:** the Markdown output for three of the memo's ten replay changes names the same consumer locations
as the prototype's reports.

### Slice P5: field level (4 to 5 days; consumer half only if F3 was positive)

Producer struct fields ship in P2. This slice first adds a facade function to the core crate,
`prism::api::dfg_edges(session: &NavigationSession) -> impl Iterator<Item = DfgEdgeRecord>` (the records
`nav dfg-stats --edges` prints; additive, under the facade's compatibility promise, with its own test in the
core crate), because the graph is not public outside it. It then adds `consumer/field_reads.rs`: the
procedure and scoring of Task F-4, in Rust, with the configured `response_root`. Rows come from F3's miss reasons; each miss class is either a row that
must pass or an accepted loss named in the README. **Gate:** on F3's 12 operations, found reads at least
F3's figure, wrong reads at most 10%.

### Slice P6: replay gate, documentation, pair R (3 to 4 days)

- [ ] `eval/schema_impact/replay.py` gains `--engine product` (runs `prism-boundary bind` and `impact` instead of
      the prototype extractors). **Gate:** late fixes found ≥ the memo's count; predicted-location touch rate
      within 5 points of the memo's.
- [ ] `crates/prism-boundary/README.md`, a short user guide `docs/boundary-impact.md`, a paragraph in
      `CLAUDE.md` (crate, documents, grades, what is out of model).
- [ ] If the owner chose it: the `route_table` producer adapter and route-table schema source for pair R,
      with public fixtures only; R's numbers recorded privately; the private-name check before every commit.
- [ ] Close the tracking issue with the acceptance numbers; file follow-ups for protobuf, federation and
      MCP exposure as separate issues, not started.

## Acceptance for the lane

Part 1 is done when the memo is merged and posted. Part 2 is done when, on P at the pinned revision: the
P2 and P3 gates hold on the final head, the P6 replay gate holds, both reviewers approved each slice, and
the README lists every unbound reason with its count on P.
