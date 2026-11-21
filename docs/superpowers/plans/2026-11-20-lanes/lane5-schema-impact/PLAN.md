# Lane 5: schema-aware service impact — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** measure whether schema operations and fields can be bound to producer handlers and consumer call
sites on a real pair, and, on an owner go-ahead, build a pilot that reports the code locations a schema
change affects.

**Architecture:** the feasibility study is a Python prototype under `eval/boundary/` plus a history replay;
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
2. A recursive `$ref` (a definition that contains itself): flattening terminates and each field is listed
   once. (P1.)
3. A client call inside an anonymous callback or a hook body: the binding is still recorded, with an empty
   or anonymous caller list, not dropped. (F3 task; P3.)
4. A URL whose only dynamic part is the query string (`/x?title=${t}`): classed as a literal path after the
   cut, not as dynamic. (F3 task; P3.)
5. A handler with two `@Router` lines, and a `@Router` comment separated from the function by a blank line:
   the first binds two operations to one function; the second binds nothing (it is not the function's doc
   comment). (F2 task; P2.)

---

# Part 1: feasibility study (3 to 5 days)

Branch `eval/boundary-feasibility`, worktree `~/code/prism-boundary`.

### Task F-0: workspace, corpus, issue

**Files:** none in the repository.

- [ ] Create the worktree and evidence directory (ORCHESTRATION section 4).
- [ ] `git clone ~/code/bench-repos/apache-answer ~/prism-evidence/boundary-impact/corpus/apache-answer`
      and `git -C … checkout --detach 6744295f`. Record the commit in `LEDGER.md`.
- [ ] File a tracking issue in the prism repository titled "Schema-aware service impact: feasibility study",
      linking this directory. Record its number in `LEDGER.md`; later tasks reference it.
- [ ] Build prism once for the data-flow dump used by Task F-4:
      `~/prism-evidence/tools/memguard.sh 8 cargo build --release` in the worktree, then check
      `target/release/prism --version` names the worktree's commit.

### Task F-1: prototype package, schema reader, ids

**Files:**
- Create: `eval/boundary/__init__.py`, `eval/boundary/cli.py`, `eval/boundary/ids.py`, `eval/boundary/schema.py`
- Create: `eval/tests/boundary/test_ids.py`, `eval/tests/boundary/test_schema.py`, `eval/tests/boundary/fixtures/mini.swagger.json`
- Modify: `eval/pyproject.toml` (dependency group `boundary = ["pyyaml>=6", "tree-sitter>=0.23", "tree-sitter-go", "tree-sitter-typescript", "tree-sitter-javascript"]`; script `boundary = "boundary.cli:main"`; add `"boundary"` to the wheel packages)

**Interfaces (produced):**
- `ids.normalize_path(raw: str, base: str = "") -> str`
- `ids.op_id(method: str, path: str, base: str = "") -> str`
- `ids.field_id(definition: str, props: list[str]) -> str`
- `schema.load(path_or_bytes) -> SchemaIndex` with `.operations: dict[str, Operation]`, `.fields: dict[str, Field]`,
  `.definitions_of(op_id) -> set[str]`, `.sha256`, `.info_version`, `.warnings: list[str]`
- `schema.diff(old: SchemaIndex, new: SchemaIndex) -> list[Change]` where `Change.kind` is one of
  `op_removed`, `op_added`, `field_removed`, `field_added`, `field_type_changed`, `field_required_added`,
  `enum_values_changed`

- [ ] **Write the failing tests.**

```python
def test_normalize_path():
    assert normalize_path("/answer/api/v1/question/{id}/") == "/answer/api/v1/question/{}"
    assert normalize_path("question/{id}", base="/answer/api/v1") == "/answer/api/v1/question/{}"

def test_op_id():
    assert op_id("get", "/a/{x}/b") == "op:GET /a/{}/b"

def test_field_id():
    assert field_id("schema.GetAPIKeyResp", ["access_key"]) == "field:schema.GetAPIKeyResp#/access_key"

def test_load_mini_swagger():
    idx = load(FIXTURE)                       # 3 paths, 4 operations, basePath "/answer"
    assert "op:GET /answer/api/v1/question/{}" in idx.operations
    assert "field:schema.QuestionInfo#/title" in idx.fields
    assert "schema.QuestionInfo" in idx.definitions_of("op:GET /answer/api/v1/question/{}")   # through an allOf envelope

def test_duplicate_normalized_path_warns():
    idx = load(DUPLICATE_FIXTURE)             # /a/{id} and /a/{name}
    assert len([o for o in idx.operations if o.startswith("op:GET /a/")]) == 1
    assert any("duplicate" in w for w in idx.warnings)

def test_diff_kinds():
    kinds = {c.kind for c in diff(load(OLD), load(NEW))}
    assert kinds == {"op_removed", "field_removed", "field_type_changed"}
```

- [ ] Run `cd eval && uv run --group boundary pytest tests/boundary -q`; expect import failures.
- [ ] Implement the three modules. The reader accepts Swagger 2.0 and OpenAPI 3 in JSON or YAML; `$ref`
      resolution is local to the file; `allOf` members are flattened into the referring schema; a recursive
      reference is expanded once.
- [ ] Run the tests; expect all to pass. Commit (`eval(boundary): schema reader and element ids`).

### Task F-2: producer extraction and measure F1

**Files:**
- Create: `eval/boundary/producer_go.py`, `eval/tests/boundary/test_producer_go.py`, `eval/tests/boundary/fixtures/go_mini/` (three `.go` files)

**Interfaces (produced):**
- `producer_go.annotations(repo: Path) -> list[Binding]`: one per `@Router <path> [<method>]` line in the doc
  comment of a function or method declaration. `Binding(element, file, line, function, via="swaggo_annotation")`.
- `producer_go.registrations(repo: Path) -> list[Registration]`: gin calls `<x>.<METHOD>("<lit>", <handler>)`
  with `Registration(method, path_suffix, handler_text, file, line)`.
- CLI: `boundary producer --repo <P> --schema <file> --out <dir>` writes `producer.jsonl` and `f1.json`.

- [ ] **Write the failing tests** on the fixture: a method with one annotation; a function with two
      annotations (two bindings, same function); an annotation followed by a blank line before `func`
      (no binding); a registration `g.GET("/x", c.Handler)` (suffix `/x`, handler `c.Handler`).
- [ ] Run; expect failure. Implement (annotations by scanning comment blocks adjacent to declarations found
      with tree-sitter-go; registrations by call-expression shape). Run; expect pass. Commit.
- [ ] Run on P. `f1.json` holds: operations in the schema; bound to exactly one function; to none; to
      several; annotations naming no schema operation; and the cross-check (for operations whose path ends
      with a registration's suffix and whose method agrees: share where the registration's handler names the
      annotated function).
- [ ] Record the F1 table in `feasibility/F1.md` with the prediction written before the run.

### Task F-3: consumer extraction and measure F2

**Files:**
- Create: `eval/boundary/consumer_ts.py`, `eval/boundary/match.py`, `eval/boundary/config.py`,
  `eval/tests/boundary/test_consumer_ts.py`, `eval/tests/boundary/test_match.py`,
  `eval/tests/boundary/fixtures/ts_mini/` (a wrapper module, a services file, a component using a hook),
  `eval/boundary/configs/apache-answer.toml`

**Interfaces (produced):**
- `config.load(path) -> list[ClientSpec]` (the TOML of DESIGN 5.4).
- `consumer_ts.calls(repo: Path, clients: list[ClientSpec]) -> list[ClientCall]` with
  `ClientCall(file, line, method, url_text, pattern, url_class)`; `url_class` is `literal`, `template` or `dynamic`.
- `match.bind(call: ClientCall, idx: SchemaIndex) -> (grade, list[op_id], reason)`.
- CLI: `boundary consumer --repo <P> --schema <file> --config <toml> --out <dir>` writes `consumer.jsonl`,
  `f2.json`, and `f2-sample.md` (50 unique matches chosen with `random.Random(20261120)`).

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
    calls = consumer_ts.calls(FIXTURE, load(CONFIG))
    assert {c.url_class for c in calls} == {"literal", "template", "dynamic"}
    assert any(c.file.endswith("Widget.tsx") for c in calls)      # a call inside a hook callback is kept
    assert not any(c.line == SHADOWED_LINE for c in calls)        # `request` rebound to a parameter: not a client
    assert any(c.line == CONST_URL_LINE and c.pattern == "/a/list" for c in calls)   # const followed one step
```

- [ ] Run; expect failure. Implement. Run; expect pass. Commit.
- [ ] Write `configs/apache-answer.toml`: the `request` default export of the client's request module with
      `get`, `post`, `put`, `delete`, `patch`; the `useSWR` hook with `url_arg = 0`.
- [ ] Run on P. `f2.json` holds: client call sites; counts per `url_class`; unique, several, none; the
      reasons for none.
- [ ] Dispatch one Sonnet-5.5 reader (medium) with `f2-sample.md`: for each row, "does this call address this
      operation? yes / no / cannot tell", reading the call's file and the schema entry. Record the tally in
      `feasibility/F2.md`.

### Task F-4: field reads and measure F3

**Files:**
- Create: `eval/boundary/fields.py`, `eval/boundary/oracle/field_refs.mjs`, `eval/tests/boundary/test_fields.py`

**Interfaces (produced):**
- `fields.select_operations(idx, calls, n=12, seed=20261120) -> list[op_id]` (4 list, 4 detail, 4 mutation
  operations that have at least one bound client call and a typed response definition).
- `fields.reads_from_edges(edges_jsonl: Path, call: ClientCall, envelope: str) -> set[str]`: member paths read
  on values reachable forward from the definitions on the call's line, from a `prism nav dfg-stats --edges`
  dump.
- `oracle/field_refs.mjs <tsconfig> <file> <interface> <property>`: prints `file:line` of every reference,
  using the TypeScript at `~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3`.

- [ ] Write a unit test for `reads_from_edges` on a hand-written six-edge JSONL (a call-line definition, a
      reassignment, two member reads, one unrelated edge); expect the two member paths. Implement. Commit.
- [ ] Dump edges once: `memguard.sh 8 target/release/prism nav dfg-stats --repo <P clone> --edges > feasibility/edges.jsonl`.
- [ ] For the 12 operations: oracle reads from `field_refs.mjs` where the service function's return is
      typed; where it is not, a Sonnet-5.5 reader labels reads by hand. Compare with `reads_from_edges`.
- [ ] `feasibility/F3.md`: per operation, oracle reads, found, missed, wrong; the totals; and the three most
      common reasons for a miss (for example: value stored in component state, value passed through a hook).

### Task F-5: history replay and measure F4

**Files:**
- Create: `eval/boundary/replay.py`, `eval/boundary/render.py`, `eval/tests/boundary/test_replay.py`

**Interfaces (produced):**
- `replay.schema_commits(repo, schema_path) -> list[str]` (newest first).
- `replay.replay_one(repo, commit, schema_path, config, window=10) -> ReplayResult` with `changes`,
  `predicted` (consumer locations at the parent commit bound to a removed, renamed or type-changed element),
  `touched_same_commit`, `touched_later` (locations whose file has a changed hunk mentioning the element's
  path literal or field name in the commit itself, or in one of the next `window` commits), `late_fixes`.
- `render.report(result) -> str` (Markdown: the change, producer locations, consumer locations, what
  happened next in history).
- CLI: `boundary replay --repo <P clone> --schema docs/swagger.yaml --config <toml> --limit 60 --out <dir>`.

- [ ] Unit-test `replay_one` on a three-commit fixture repository built in the test (commit 1: schema and
      client; commit 2: schema removes an operation; commit 3: client stops calling it); expect one predicted
      location and one late fix. Implement. Commit.
- [ ] Run on P. Each replayed commit uses a detached scratch worktree of the clone at the parent commit,
      removed afterwards. Keep going until 60 schema commits are examined or history ends.
- [ ] `feasibility/F4.md`: schema commits examined; usable (at least one removed, renamed or type-changed
      element); elements with a non-empty prediction; share of predicted locations touched in the window;
      late fixes with commit ids; and ten rendered reports (`feasibility/reports/01.md` … `10.md`), choosing
      late fixes first, then the largest predictions.

### Task F-6: pair R and measure F5 (private)

**Files:**
- Create: `eval/boundary/producer_route_table.py`, `eval/tests/boundary/test_route_table.py` (public fixture only)
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

- [ ] Try to obtain one OpenAPI differ without changing system packages (a release binary into the evidence
      directory). If it runs, compare its changed-element list with `schema.diff` on five replay commits and
      note disagreements in `feasibility/F7.md`. If it does not install in 15 minutes, write that and stop.

### Task F-8: memo, review, stop

**Files:**
- Create: `docs/superpowers/plans/2026-11-20-lanes/lane5-schema-impact/FEASIBILITY.md`

- [ ] Write the memo per DESIGN section 6: the measures table, gates G1 to G4 with pass or fail, the ten
      reports (linked from the evidence directory; quote three in full), the recommended pilot pair and
      adapters per the table in DESIGN section 7, and a day estimate per pilot slice revised from F6.
- [ ] Open one PR with `eval/boundary/`, its tests and the memo. One reviewer (Sonnet-5.5, high, careful
      reading: do the numbers in the memo match the JSON outputs; do the tests pin Review focus items 1, 3,
      4, 5). Fold, merge on approval and green CI.
- [ ] Post the gate table and the recommendation on the tracking issue. Refresh `HANDOFF.md`.
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
`recursive_ref_terminates_and_lists_each_field_once`, `openapi3_first_server_path_is_prefixed`,
`addition_only_change_has_no_removed_or_changed_kind`, `documents_validate_against_contract_schemas`
(serialise one of each document and validate with the JSON schemas, as the targets contract test does).

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
pub struct ClientSpec { /* module, export, methods, hook, url_arg, base_path, envelope */ }
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
| c03 | a method the configuration does not list (`request.instance.get`) | `unbound`, reason `unknown_client_method` |
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

Producer struct fields ship in P2. This slice adds `consumer/field_reads.rs`: from each bound call, forward
over prism's data-flow edges (the same edge set `nav dfg-stats --edges` prints, read through the library)
to member reads; strip the configured envelope; match the remaining path to the fields of the definitions of
the operation's success response. Rows come from F3's miss reasons; each miss class is either a row that
must pass or an accepted loss named in the README. **Gate:** on F3's 12 operations, found reads at least
F3's figure, wrong reads at most 10%.

### Slice P6: replay gate, documentation, pair R (3 to 4 days)

- [ ] `eval/boundary/replay.py` gains `--engine product` (runs `prism-boundary bind` and `impact` instead of
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
