# Lane 5: schema-aware service impact — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** measure whether schema operations and fields can be bound to producer handlers and consumer call
sites on a real pair, which integration inconsistencies stand today, and whether history shows the report
would have helped; and, on an owner go-ahead, build a pilot that reports the code locations a schema change
affects.

**Architecture:** the feasibility study is a Python prototype under `eval/schema_impact/` plus a history replay;
the pilot is a separate workspace crate `crates/prism-boundary` that reads prism's parsed files and call
graph and writes two JSON documents (bindings per repository, impact per change).

**Tech stack:** Python 3.12 and uv (the existing `eval` project), tree-sitter Python bindings, TypeScript
5.9.3 as the field-read oracle (slice P5); Rust, `serde_json`, `toml`, `tree-sitter`, `clap`, the `prism` core
library for the pilot.

**Spec:** `DESIGN.md` in this directory. Sections named below (F1 to F8, G1 to G4, 5.1 to 5.7, 6.1 to 6.5,
7.1, 7.2) are its. Every term in italics in this plan (*usable*, *predicted location*, *touched*, *outcome*,
*late fix*, *control sample*, *lift*, *standing mismatch*, the adjudication and mismatch classes, the reader
tasks R1 to R3) is defined in DESIGN 6.1 and 6.2; grades (`bound`, `candidate`, `unbound`) in DESIGN 5.4 and
5.5. Do not restate or reinterpret them in a brief: quote them.

## Global constraints

- No product-crate change in Part 1. The owner approved the pilot on both pairs without ordering them. Part 2
  applies the private-first variant with P2 beside P2-R (DESIGN 7.2).
- Corpus P is `bench-repos/apache-answer` at `6744295f`. Work on a clone under the evidence directory, never
  in `~/code/bench-repos`.
- Pair R is private: ORCHESTRATION section 9 applies to every step that touches it (Claude seats only, never
  a cloud session or a codex seat). Its paths, framework details and client configuration are read from
  `~/prism-evidence/lanes-2026-11-20/PRIVATE-BINDINGS.md`. Every R output goes under
  `~/prism-evidence/boundary-impact/private/`. Public text says `pilot-producer` and `pilot-consumer` and
  carries aggregate counts and class names only. Rows or fixtures derived from R's code shapes are written as
  neutral synthetic code with invented names.
- Binding grades are `bound`, `candidate`, `unbound`. The words `Exact` and `NameOnly` are not used for
  bindings.
- Element IDs and slash, capture, wildcard, authority and field-pointer rules are in DESIGN 5.1. Apply the source
  dialect and preserve typed tokens. A field belongs to its request or status-bearing response root.
- Clients are identified by resolved file (DESIGN 5.4), never by import spelling.
- The pilot reads JSON schema files only.
- Evidence directory: `~/prism-evidence/boundary-impact/` (`LEDGER.md`, `HANDOFF.md`, `feasibility/`,
  `private/`).

## Review focus

Inputs the design implies but no single task's happy path exercises. Each is pinned by a test in the task
named.

1. Two schema paths that normalise to one path (`/a/{id}` and `/a/{name}`): with disjoint methods both
   operations exist and nothing is warned; with the same method the first sorted original path is kept and
   `duplicate_operation` is warned, never merged silently. (Task F-1 `schema.py`; pilot P1.)
2. A recursive `$ref` (a definition that contains itself): expansion stops at the cycle, the field holding
   the reference is listed once with `recursive`, and `definitions_of` terminates. (F-1; P1.)
3. A client call inside an anonymous callback or a hook body: the binding is still recorded, with an empty
   or anonymous caller list, not dropped. (F-3; P3.)
4. A URL whose only dynamic part is the query string (`/x?title=${t}`): classed as a literal path after the
   cut, not as dynamic. (F-3; P3.)
5. A handler with two `@Router` lines, and a `@Router` comment separated from the function by a blank line:
   the first binds two operations to one function; the second binds nothing (it is not the function's doc
   comment). (F-2; P2.)
6. A wire-neutral definition rename produces no change. (F-1; P1; F-5.)
7. A client parameter facing a schema literal is never `bound`. (F-3; P3.)
8. A replay parent whose client import uses the older spelling is bound with the era configuration, and a
   parent with no import resolving to the client is skipped as `no_client_import`, never counted as zero
   predictions. (F-5.)
9. A regress-and-restore pair (2a99d234, a5edc4fc) contributes no usable change. (F-5.)

---

# Part 1: feasibility study (about 7 days)

Branch `eval/schema-impact-feasibility`, worktree `~/code/prism-boundary` (removed after Task F-8 merges;
Part 2 creates it again). Gates for Part 1 tasks: `cd eval && uv run --group schema-impact pytest
tests/schema_impact -q`, the private-name check before every commit, and CI on the PR. Items 5 to 9 of
ORCHESTRATION section 5 concern Rust product code and do not apply.

Task order and estimates: F-0 (0.25 day), F-1 (1), F-2 (0.5), F-3 (1), F-9 (0.5), F-5 (1.5), F-6 (2),
F-8 (0.5). F-6 comes after F-5 because its cross-repository replay uses F-5's follow and adjudication code;
its other steps may start as soon as F-3 has merged into the branch. Task F-4 moved to slice P5; Task F-7 is dropped. Task ids are kept stable so earlier references
stay readable; measure ids (F1 to F8) are DESIGN's and differ from task ids. Mapping: Task F-2 measures
F1, F-3 F2, F-4 F3 (in slice P5), F-5 F4, F-6 F5, F-8 F6, F-9 F8.

### Task F-0: workspace, corpus, issue

**Files:** none in the repository.

- [ ] Create the worktree and evidence directory (ORCHESTRATION section 4).
- [ ] `git clone ~/code/bench-repos/apache-answer ~/prism-evidence/boundary-impact/corpus/apache-answer`
      and `git -C … checkout --detach 6744295f`. Record the commit in `LEDGER.md`. All schema work uses
      `docs/swagger.json` (the YAML twin is not read); record that the two files list the same number of
      paths at the pinned commit.
- [ ] File a tracking issue in the prism repository titled "Schema-aware service impact: feasibility study",
      linking this directory. Record its number in `LEDGER.md`; later tasks reference it.

### Task F-1: prototype package, schema reader, ids

**Files:**
- Create: `eval/schema_impact/__init__.py`, `eval/schema_impact/cli.py`, `eval/schema_impact/ids.py`,
  `eval/schema_impact/schema.py`
- Create: `eval/tests/schema_impact/test_si_ids.py`, `eval/tests/schema_impact/test_si_schema.py`,
  `eval/tests/schema_impact/fixtures/` (`mini.swagger.json`, `recursive.json`, `collision.json`,
  `duplicate.json`, `diff_old.json`, `diff_new.json`, `rename_old.json`, `rename_new.json`,
  `allof_ab.json`, `allof_ba.json`; test file names
  must stay unique across `eval/tests/`, which has no package files; the existing `test_boundary.py` belongs
  to another tool)
- Modify: `eval/pyproject.toml` (dependency group `schema-impact = ["pyyaml>=6", "tree-sitter>=0.23",
  "tree-sitter-go", "tree-sitter-typescript", "tree-sitter-javascript"]`; script
  `schema-impact = "schema_impact.cli:main"`; add `"schema_impact"` to the wheel packages), `eval/uv.lock`
  (regenerated; commit it)

**Interfaces (produced):**
- `ids.normalize_path(raw: str, base: str = "") -> str`
- `ids.op_id(method: str, path: str, base: str = "") -> str`
- `ids.field_id(root: str, pointer: list[str]) -> str` where `root` is `req:<METHOD> <path>` or
  `resp:<METHOD> <path>:<status>`
- `schema.load(path_or_bytes) -> SchemaIndex` with `.operations: dict[str, Operation]` (`Operation.source_path`,
  `.operation_id`), `.fields: dict[str, Field]` (`Field.definition`, `.recursive`, `.shape`),
  `.definitions_of(op_id) -> set[str]`, `.response_fields(op_id, response_root: str) -> list[str]` (the
  property names directly under `#/<response_root>` of the operation's first 2xx response in sorted order;
  for a root that is an array, under `#/<response_root>/[]`), `.sha256`, `.info_version`,
  `.warnings: list[str]`
- `schema.diff(old: SchemaIndex, new: SchemaIndex) -> list[Change]` where `Change.kind` is one of
  `op_removed`, `op_added`, `field_removed`, `field_added`, `field_type_changed`, `field_required_added`,
  `enum_values_changed`, keyed by wire-tree field id. A field of a removed operation is not listed again.
  There is no rename kind; a wire-neutral rename yields nothing.
- CLI: `schema-impact ids <schema.json>` prints `ids.tsv` (one id per line, operations then fields, sorted).

All id rules are DESIGN 5.1 (operations table, wire-tree table, cycle and depth cut, 2xx, request body);
shape signatures are DESIGN 5.6.

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
    assert field_id("resp:GET /answer/admin/api/api-key/all:200", ["data", "[]", "access_key"]) == \
        "field:resp:GET /answer/admin/api/api-key/all:200#/data/[]/access_key"
    assert field_id("req:POST /a", ["x"]) == "field:req:POST /a#/x"

def test_load_mini_swagger():
    idx = load(FIXTURE)       # 3 paths, 4 operations plus a `parameters` key that is not an operation;
                              # GET /answer/api/v1/question/{id} has a 200 (allOf envelope) and a 400 response
    assert len(idx.operations) == 4
    assert "op:GET /answer/api/v1/question/{}" in idx.operations
    title = idx.fields["field:resp:GET /answer/api/v1/question/{}:200#/data/title"]
    assert title.definition == "schema.QuestionInfo"
    assert "field:resp:GET /answer/api/v1/question/{}:200#/code" in idx.fields
    assert not any(":400#" in k for k in idx.fields)                 # only 2xx responses are read
    assert "schema.QuestionInfo" in idx.definitions_of("op:GET /answer/api/v1/question/{}")
    assert idx.response_fields("op:GET /answer/api/v1/question/{}", "data") == ["id", "title"]

def test_recursive_definition_terminates():
    idx = load(RECURSIVE_FIXTURE)             # GET /n returns Node { children: [ $ref Node ] }
    assert idx.fields["field:resp:GET /n:200#/children"].recursive == "Node"
    assert not any(k.startswith("field:resp:GET /n:200#/children/[]/") for k in idx.fields)
    assert idx.definitions_of("op:GET /n") == {"Node"}

def test_colliding_paths_with_disjoint_methods_are_both_kept():
    idx = load(COLLISION_FIXTURE)             # /a/{id} with get, /a/{name} with post
    assert {"op:GET /a/{}", "op:POST /a/{}"} <= set(idx.operations)
    assert not any("duplicate" in w for w in idx.warnings)

def test_true_duplicate_keeps_the_first_sorted_and_warns():
    idx = load(DUPLICATE_FIXTURE)             # /a/{id} and /a/{name}, both with get
    assert [o for o in idx.operations if o.startswith("op:GET /a/")] == ["op:GET /a/{}"]
    assert idx.operations["op:GET /a/{}"].source_path == "/a/{id}"
    assert any("duplicate_operation" in w for w in idx.warnings)

def test_diff_kinds():
    kinds = {c.kind for c in diff(load(OLD), load(NEW))}
    assert kinds == {"op_removed", "field_removed", "field_type_changed"}

def test_wire_neutral_rename_has_no_change():
    assert diff(load(RENAME_OLD), load(RENAME_NEW)) == []   # schema.A {x, y} renamed schema.B, same wire tree

def test_allof_member_order_and_property_wrapper():
    a, b = load(ALLOF_AB), load(ALLOF_BA)     # GET /b: two allOf members both declare `mode` (enums differ),
                                              # in both orders; plus `level: {description, allOf: [{$ref: Level}]}`
    assert diff(a, b) == []
    assert any("allof_overlap" in w for w in a.warnings)
    assert "field:resp:GET /b:200#/level" in a.fields
```

- [ ] Run `cd eval && uv run --group schema-impact pytest tests/schema_impact -q`; expect import failures.
- [ ] Implement the three modules. The reader accepts Swagger 2.0 and OpenAPI 3 in JSON (YAML too, for
      convenience); `$ref` resolution is local to the file. The OpenAPI 3 base rule is provisional
      (DESIGN 5.1).
- [ ] Run the tests; expect all to pass. Write `feasibility/ids.tsv` for the pinned schema. It is
      provisional: stage D may amend DESIGN 5.1, and the pilot's golden file is the one regenerated after
      stage D (Part 2, "Regenerate ids.tsv"). Commit (`eval(schema-impact): schema reader and element ids`).

### Task F-2: producer extraction and measure F1

**Files:**
- Create: `eval/schema_impact/producer_go.py`, `eval/schema_impact/outbound_http.py`,
  `eval/tests/schema_impact/test_si_producer_go.py`, `eval/tests/schema_impact/fixtures/go_mini/` (three
  `.go` files)

**Interfaces (produced):**
- `producer_go.annotations(repo: Path) -> list[Binding]`: one per `@Router <path> [<method>]` line in the doc
  comment of a function or method declaration, in every `.go` file that is not a `_test.go` file.
  `Binding(element, file, line, function, receiver_type, via="swaggo_annotation")`; `function` is the bare
  function or method name. A doc comment is the run of `//` lines directly above the declaration with no
  blank line between; a bare `//` line does not end it. A variant
  `producer_go.annotations_from_sources(files: dict[str, str])` takes file texts (Task F-5 feeds it from
  `git show`).
- `producer_go.registrations(repo: Path) -> list[Registration]`: calls `<x>.<METHOD>("<literal>", <handler>)`
  with METHOD one of GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS;
  `Registration(method, path_suffix, handler_name, file, line)` where `handler_name` is the last identifier
  of the handler expression (`a.langController.GetLangMapping` → `GetLangMapping`) and colon parameters in
  the suffix become `{}`. Calls of `Any`, `Handle` and `Match` are listed as `unextracted_registration` with
  their count.
- `outbound_http.count(repo: Path) -> dict`: outbound HTTP call sites in non-test Go and Python files, by
  callee, from these regular expressions (an approximation; the memo says so): Go `http.Get(`, `http.Post(`,
  `http.PostForm(`, `http.Head(`, `http.NewRequest(`, `http.NewRequestWithContext(`, and `.Do(` on a
  receiver whose name contains `client` in any case; Python `requests.<verb>(`, `requests.request(`,
  `httpx.<verb>(`, `httpx.Client(`, `httpx.AsyncClient(`, `urllib.request.urlopen(`,
  `aiohttp.ClientSession(`.
- CLI: `schema-impact producer --repo <P> --schema <file> --out <dir>` writes `producer.jsonl`, `f1.json`
  and `outbound-http.json`.

- [ ] **Write the failing tests** on the fixture: a method with one annotation; a function with two
      annotations (two bindings, same function); an annotation followed by a blank line before `func`
      (no binding); an annotation block containing a bare `//` line (still one block); a registration
      `g.GET("/x", c.Handler)` (suffix `/x`, handler name `Handler`); a registration `g.GET("/x/:id", h)`
      (suffix `/x/{}`); a `g.Any("/y", h)` (listed as unextracted); one `http.Get(` call counted by
      `outbound_http.count`.
- [ ] Run; expect failure. Implement. Run; expect pass. Commit.
- [ ] Write the prediction for F1 and G1 in `feasibility/F1.md`, then run on P. `f1.json` holds, over
      **all** schema operations: bound to exactly one function; to none; to several; annotations naming no
      schema operation (the sanity number of DESIGN F1). Then the drift: the cross-check (for each
      registration, the schema operations with the same method whose path ends with the registration's
      suffix; where exactly one such operation exists and it has exactly one annotation binding, whether
      `handler_name` equals that binding's `function`), with the population size and the agreement; and the
      residue rows of DESIGN 6.1: standing-mismatch lists 2 and 3, the annotation-versus-registration part of
      list 4, and the cross-check disagreements.
- [ ] Classify every residue row with reader task R2 (one Sonnet-5.5 seat, medium; DESIGN 6.2). Record the
      labels in `f1.json` and the G1 figure (DESIGN 6.3) in `feasibility/F1.md`, with the statement that G1
      has no bearing on a code-first producer.
- [ ] Record P's `outbound-http.json` totals in `feasibility/F1.md`.

### Task F-3: consumer extraction and measure F2

**Files:**
- Create: `eval/schema_impact/consumer_ts.py`, `eval/schema_impact/match.py`, `eval/schema_impact/config.py`,
  `eval/tests/schema_impact/test_si_consumer_ts.py`, `eval/tests/schema_impact/test_si_match.py`,
  `eval/tests/schema_impact/fixtures/ts_mini/` (a wrapper module with verb methods and an `instance`
  member, a barrel `index.ts` re-exporting it, a fixed-method function client, a services file, a component
  using a hook), `eval/tests/schema_impact/fixtures/match_matrix.tsv`,
  `eval/schema_impact/configs/apache-answer.toml`, `eval/schema_impact/configs/apache-answer-2022.toml`

**Interfaces (produced):**
- `config.load(path) -> Config` with `.aliases` and `.clients: list[ClientSpec]` (the TOML of DESIGN 5.4:
  `file`, `export`, `methods` including member paths, `call` for function clients, `hook` and `fetcher_arg`,
  `url_arg`, `strip_prefix`, `add_prefix`, `response_root`).
- `consumer_ts.calls(repo: Path, config: Config, root: str) -> list[ClientCall]` with
  `ClientCall(file, line, method, url_text, pattern, url_class, reason, response_type, enclosing_function)`;
  `url_class` is `literal`, `template`, `dynamic` or `conditional`; `reason` is set for calls that cannot
  be bound before matching (`unknown_client_method`, `non_string_key`, `unconfigured_client_form`,
  `dynamic_method`, `conditional_url`, `dynamic_url`); `response_type` is the text of an explicit generic
  argument (`request.get<Type.X>(…)`, `useSWR<Type.X>(…)`) or `None`. The set of calls is the denominator of
  DESIGN 5.4 (counting rule), including `fetch` and `axios` calls.
- `consumer_ts.pattern(expr_text: str, consts: dict[str, str] | None = None) -> (str | None, url_class)`:
  the URL pattern rules of DESIGN 5.4 (const replacement, whole-segment `{}`, query cut, conditional arms,
  array keys), followed by the prefix map and the 5.1 path normalisation where a client spec applies.
- `match.bind(call: ClientCall, idx: SchemaIndex) -> (grade, list[op_id], reason)`: the matching rule of
  DESIGN 5.4.
- CLI: `schema-impact consumer --repo <P> --root ui/src --schema <file> --config <toml> --out <dir>` writes
  `consumer.jsonl`, `f2.json`, and `f2-sample.md`.

- [ ] **Write the failing tests.**

```python
def test_url_classes():
    consts = {"apiUrl": "/answer/api/v1/personal/user/info",
              "qaUrl": "/answer/api/v1/personal/qa/top?username="}
    assert pattern("'/answer/api/v1/file'") == ("/answer/api/v1/file", "literal")
    assert pattern("`/answer/api/v1/question/${id}`") == ("/answer/api/v1/question/{}", "template")
    assert pattern("`/answer/api/v1/question/similar?title=${title}`") == ("/answer/api/v1/question/similar", "literal")
    assert pattern("`/a/item-${id}`")[1] == "dynamic"
    assert pattern("`${base}/x`")[1] == "dynamic"                  # base is no single-assignment literal const
    assert pattern("`${apiUrl}?username=${u}`", consts) == ("/answer/api/v1/personal/user/info", "literal")
    assert pattern("`${qaUrl}${u}`", consts) == ("/answer/api/v1/personal/qa/top", "literal")
    assert pattern("'/a/' + id") == ("/a/{}", "template")
    assert pattern("id ? `/a/${id}` : null") == ("/a/{}", "template")
    assert pattern("c ? '/a' : '/b'")[1] == "conditional"
    assert pattern("[apiUrl, { params: { action } }]", consts) == ("/answer/api/v1/personal/user/info", "literal")

def test_match_scoring():
    idx = index_with("op:GET /user/{}", "op:GET /user/me", "op:POST /user/{}")
    assert bind(call("GET", "/user/{}"), idx)[:2] == ("bound", ["op:GET /user/{}"])
    assert bind(call("GET", "/user/me"), idx)[:2] == ("bound", ["op:GET /user/me"])
    assert bind(call("DELETE", "/user/{}"), idx)[0] == "unbound"

def test_client_parameter_on_schema_literal_is_never_bound():
    assert bind(call("GET", "/user/{}"), index_with("op:GET /user/me")) == \
        ("candidate", ["op:GET /user/me"], "client_parameter_on_literal")

def test_client_literal_on_schema_parameter_binds_when_unique():
    idx = index_with("op:GET /user/{}", "op:GET /user/me")
    assert bind(call("GET", "/user/you"), idx)[:2] == ("bound", ["op:GET /user/{}"])

def test_match_matrix():                     # every row of fixtures/match_matrix.tsv
    for row in rows(MATCH_MATRIX):
        assert bind(call(row.method, row.pattern), index_with(*row.operations)) == row.expected, row.id

def test_calls_fixture():
    calls = consumer_ts.calls(FIXTURE, config.load(CONFIG), "src")
    assert {c.url_class for c in calls} >= {"literal", "template", "dynamic"}
    assert any(c.file.endswith("Widget.tsx") for c in calls)      # a call inside a hook callback is kept
    assert not any(c.line == SHADOWED_LINE for c in calls)        # `request` rebound to a parameter: not a client
    assert any(c.line == CONST_URL_LINE and c.pattern == "/a/list" for c in calls)   # const followed one step
    assert sum(1 for c in calls if c.line == HOOK_LINE) == 1      # hook site counted once
    assert not any(c.line == HOOK_INLINE_FETCHER_LINE for c in calls)                # inner fetcher call not a site
    assert any(c.line == POST_FETCHER_HOOK_LINE and c.method == "POST" for c in calls)   # fetcher fixes POST
    assert any(c.line == INSTANCE_PUT_LINE and c.method == "PUT" for c in calls)     # member path `instance.put`
    assert any(c.line == BARREL_LINE for c in calls)              # imported through the barrel
    assert any(c.line == RELATIVE_LINE for c in calls)            # imported by a relative path
    assert any(c.line == FIXED_METHOD_LINE and c.method == "POST" for c in calls)    # function client
    assert any(c.line == UNKNOWN_METHOD_LINE and c.reason == "unknown_client_method" for c in calls)
    assert any(c.line == FETCH_DYNAMIC_INIT_LINE and c.reason == "dynamic_method" for c in calls)
    assert any(c.response_type == "Type.Item" for c in calls)
```

- [ ] Write `fixtures/match_matrix.tsv` (columns: `id`, `operations`, `method`, `pattern`, `grade`,
      `operation_ids`, `reason`, `class`) with at least these rows: literal on equal literal; client `{}` on
      schema `{}`; client literal on schema `{}` as the unique match; literal beats `{}` when both exist;
      client `{}` facing only a schema literal (`candidate`, `client_parameter_on_literal`); client `{}`
      facing two literals (`/vote/{}` against `vote/up` and `vote/down`: both `candidate`); an interpolated
      version segment with one version and with two; different segment counts (`unbound`); the same path
      under another method (`unbound`, `no_operation`); a prefix map that strips and then adds; a client URL
      with a trailing slash and with a doubled slash (normalised before matching).
- [ ] Run; expect failure. Implement. Run; expect pass. Commit.
- [ ] Write `configs/apache-answer.toml` from the client's source (DESIGN 5.4 grammar): the default export of
      `src/utils/request.ts` with the verb methods that class defines (read them; do not assume `patch`) and
      the `instance.<verb>` member paths with the same `response_root` (they pass through the same response
      interceptor); `response_root` set to what the wrapper's response handling returns to callers; the
      `requestAi` function client with its fixed method; the `useSWR` hook with `url_arg = 0` and
      `fetcher_arg = 1`; the alias table `{ "@/" = "src/" }`. Write `configs/apache-answer-2022.toml` the
      same way from the client at the first parent of 2b1ad947, with the alias table of DESIGN 6.1 (era
      configuration). Any other client module found is added when the grammar expresses it, or its calls
      are counted as `unconfigured_client_form`.
- [ ] Write the prediction for F2, then run on P. `f2.json` holds: the denominator; counts per `url_class` and
      per pre-match reason; unique, several, none; the reasons for none; and a statement that every unique
      match on P is a string equality (P has no path parameters).
- [ ] `f2-sample.md`: sort the unique matches by (file, line), then take 50 with `random.Random(20261120)`
      (all of them if fewer). Run reader task R1 (one Sonnet-5.5 seat, medium; DESIGN 6.2). If the sample is
      not *decided* (DESIGN 6.2), extend it once as that section says. Record the tally, with the decided
      count, in `feasibility/F2.md` with the matching-matrix result.

### Task F-4: moved to slice P5

Measure F3 and this task's procedure now run at the start of slice P5 (Part 2, "Slice P5, step 1").

### Task F-9: standing mismatches and measure F8

**Files:**
- Create: `eval/schema_impact/mismatch.py`, `eval/tests/schema_impact/test_si_mismatch.py`

**Interfaces (produced):**
- `mismatch.standing(producer_out: Path, consumer_out: Path, idx: SchemaIndex) -> dict[str, list[Row]]`:
  the four standing-mismatch lists of DESIGN 6.1, from Task F-2's and F-3's outputs.
- CLI: `schema-impact mismatch --producer <dir> --consumer <dir> --schema <file> --out <dir>` writes
  `f8.json`.

- [ ] Unit-test on small hand-written producer and consumer outputs: one row of each list, and a client
      call whose path matches an operation only under another method (list 4, not list 1). Implement.
      Commit.
- [ ] Write the prediction, then run on P at the pin. Classify every row with reader task R2. Write
      `feasibility/F8.md`: each list with its rows and labels, the class counts, and the counted rows.

### Task F-6: pair R and measure F5 (private)

**Files:**
- Create: `eval/schema_impact/producer_route_table.py`, `eval/tests/schema_impact/test_si_route_table.py`,
  `eval/tests/schema_impact/fixtures/route_table_mini/` (public, neutral synthetic code with invented names)
- Private, not committed: the R producer and client configurations, the client-shape list and every output,
  under `~/prism-evidence/boundary-impact/private/`

- [ ] Skip this task, and write "F5 owed" in the ledger, if the private bindings file does not list pair R.
      The memo then reports G2 as "pass on literal paths only", G4 as not measured, and the pilot-order
      section without R's figures.
- [ ] `producer_route_table.routes(repo, spec) -> list[Route]` with `spec` giving the registration callee
      name, the method, path and handler keys, the validation-object keys and the mount (DESIGN 5.3):
      object literals with string-valued method and path keys inside an array argument of a call of the
      configured callee. `Route(method, path, handler_text, handler_site, has_validation, file, line)`;
      `handler_site` is the function the handler expression names when it resolves to a function declared
      in the same file or imported by name from a repository file, else `None`. Unit-test on the public
      fixture (three routes, one with a colon parameter, one with an unresolvable handler, one with a
      validation key, one entry outside a registration call that is ignored).
- [ ] Enumerate the client call shapes of `pilot-consumer` before computing anything (DESIGN 5.4 lists what
      the grammar expresses): Claude seats only, serially, Haiku-5.5, then Sonnet-5.5, then Opus-5.5 high,
      each reading the cumulative list, a tier repeating until a round adds nothing. Each entry: an example
      location, an approximate count, and whether the grammar expresses it. Write the private TOML for every
      expressible shape; the others are counted as `unconfigured_client_form`.
- [ ] At the two current heads: run `routes` on `pilot-producer` (the route set is the schema source,
      DESIGN 5.3); run `consumer_ts.calls` on `pilot-consumer`; bind against the route set. Record the
      denominator, per-class counts, bound, candidate and unbound by reason, and G4's pattern share.
- [ ] The standing join: the four lists of DESIGN 6.1 in their R form. Reader task R2 on every row of list 4
      and up to 30 rows of list 1 (DESIGN 6.2).
- [ ] Reader task R1 on 50 `bound` joins (sorted by (file, line), drawn with `random.Random(20261120)`),
      extended once if not *decided* (DESIGN 6.2).
- [ ] The two counts: route entries whose `handler_site` is not `None`; route entries with a validation
      object. `outbound_http.count` on both repositories.
- [ ] The cross-repository replay of DESIGN 6.1 with `replay.replay_cross` (Task F-5 provides the follow and
      adjudication machinery; run this step after F-5's unit tests pass). Reader task R3 on every
      `within_14_days`, `later` and `present_at_head` location up to the cap. Render five reports chosen by
      the rule of DESIGN 6.4 into `private/reports/`; record how many exist (G4 fails with "R payoff
      unmeasured" when none does).
- [ ] Private `F5.md`: everything above. Public memo input (`feasibility/F5-public.md`): aggregate counts,
      percentages and class names only, G2's R figure, G4's figures, the counted locations and rows.
- [ ] Run the private-name check of ORCHESTRATION section 9 on the staged diff before committing the public
      extractor and fixture.

### Task F-5: history replay and measure F4

**Files:**
- Create: `eval/schema_impact/replay.py`, `eval/schema_impact/render.py`,
  `eval/tests/schema_impact/test_si_replay.py`

**Interfaces (produced):**
- `replay.first_parents(repo, pin) -> list[str]`: first-parent commits, oldest first.
- `replay.operation_sets(repo, commits) -> dict[str, set[str]]`: the *operation set* of every commit,
  computed incrementally: the full set at the first commit, then for each commit only the `.go` files its
  diff against its first parent changes are re-read with `git show` and passed to
  `producer_go.annotations_from_sources`.
- `replay.changes(repo, commit, op_sets, schema_path) -> list[Change]`: the *element changes* of the commit
  (DESIGN 6.1).
- `replay.usable(changes_by_commit) -> dict[str, list[Change]]`: applies the reverted-change and restoration
  rule (10 first-parent commits) and keeps the *usable* kinds.
- `replay.select_config(repo, p0, configs_dir) -> Path`: the *era configuration*.
- `replay.follow(repo, p0, lines: list[(file, line)], pin) -> Outcome(kind, touched_in, lag_commits,
  lag_days)`: the *touched* procedure over a location's *followed lines* and its *outcome* (`same_commit`,
  `later`, `present_at_pin`).
- `replay.replay_one(repo, commit, usable_changes, config, root, pin) -> ReplayResult` with `changes`,
  `predicted` (each with its outcome and `field_spelled`), `control` (the *control sample* with outcomes),
  `skipped_reason` (`config_missing`, `no_client_import`).
- `replay.replay_cross(producer, consumer, route_spec, config, root) -> list[ReplayResult]`: DESIGN 6.1's
  cross-repository replay, for Task F-6.
- `render.report(result, labels) -> str` (Markdown: the change, producer locations, consumer locations with
  outcome and adjudicated class, what happened next in history).
- CLI: `schema-impact replay --repo <P clone> --schema docs/swagger.json --configs <dir> --root ui/src
  --pin 6744295f --out <dir>`.

- [ ] Unit-test `replay` on small repositories built in the test:
      (1) commit 1 has an annotated handler, a schema and a client call; commit 2 removes the annotation;
      commit 3 removes the client call: one predicted location, outcome `later`, lag 1;
      (2) the client call is removed in commit 2 itself: outcome `same_commit`;
      (3) the client call is never removed: `present_at_pin`;
      (4) an edit above the client call between commit 2 and the pin moves its line: still `present_at_pin`
      (lines are followed, not compared by number);
      (5) commit 2 removes the annotation and commit 3 restores it: nothing usable in either;
      (6) a schema change that only renames a definition: no change;
      (7) a parent whose import uses an alias only the older configuration maps: predictions made with that
      configuration; a parent with no import resolving to the client file: `skipped_reason =
      "no_client_import"`;
      (8) the control sample is drawn with the seed, at least five sites, and followed;
      (9) a field removed in commit 2, whose client call sits in a wrapper file and whose field is spelled
      only in a form that imports the wrapper function, and commit 3 edits only the form: outcome `later`.
      Implement. Commit.
- [ ] Count usable commits (no extraction) and record the number, split into operation and field changes.
- [ ] Write the prediction, then replay every usable commit. Each replayed commit uses a detached scratch
      worktree of the clone at the first parent, removed afterwards. Record the replay's wall time (F6).
- [ ] Reader task R3 on every `later` and `present_at_pin` location up to the adjudication cap (DESIGN 6.1).
- [ ] `feasibility/F4.md`: first-parent commits examined; usable, by kind; skipped by reason; elements with
      a non-empty prediction; outcomes; late fixes with commit ids; lift per commit and pooled; adjudicated
      class counts and the counted locations with commit ids; and ten rendered reports
      (`feasibility/reports/01.md` … `10.md`) chosen by the rule of DESIGN 6.4.

### Task F-7: dropped

Measure F7 (an external differ check) is dropped; it changes no decision.

### Task F-8: memo, review, stop

**Files:**
- Create: `docs/superpowers/plans/2026-11-20-lanes/lane5-schema-impact/FEASIBILITY.md`

- [ ] Measure F6: non-blank, non-comment lines per prototype module (producer: `producer_go.py`,
      `producer_route_table.py`; consumer: `consumer_ts.py`, `config.py`, `match.py`; schema: `schema.py`,
      `ids.py`; `mismatch.py`; `replay.py`, `render.py`); wall time on P of `schema-impact producer`,
      `consumer` and `mismatch` (run once each under `/usr/bin/time`) and of the replay (Task F-5's ledger
      entry); reader-seat runs per task from the ledger. Write `feasibility/F6.md`.
- [ ] Write the memo per DESIGN 6.4: the measures table (with the prism build's commit where one was used),
      gates G1 to G4 with pass or fail and the pair each figure comes from, the F8 lists and R's class
      counts, the ten reports (linked from the evidence directory; quote three in full), the recommended
      adapters, a day estimate per pilot slice revised from F6 with DESIGN 7.2's table brought up to date,
      and the pilot-order evidence section with the recommendation its default rule gives. If G3 has fewer
      than 2 counted locations or rows, write "owner rating pending" beside it.
- [ ] Ask the owner, in the session and never on the issue, to read the five private reports at their local
      paths and say for each whether it "would have helped the review of the producer change"; record the
      ratings privately and the count in the memo. G4, the G3 owner arm and the pilot-order rule use that
      count. If the study passed 8 days, the memo names the work cut by DESIGN 6.5's cut line.
- [ ] Write `~/prism-evidence/boundary-impact/roadmap-corrections.md` for the owner: the statements in the
      tooling roadmap's boundary and language-status sections that the research record (section C) found
      stale, each with the current fact and its code reference. The tooling repository itself is not edited
      by this lane.
- [ ] Open one PR with `eval/schema_impact/`, its tests, `eval/uv.lock` and the memo. One reviewer
      (Sonnet-5.5, high, careful reading: do the numbers in the memo match the JSON outputs; do the tests
      pin Review focus items 1 to 9; does any private name appear). Fold, merge on approval and green CI.
- [ ] Post the gate table, the recommendation, the pilot-order section and the ten public reports on the
      tracking issue (never the private reports). Refresh `HANDOFF.md`. Remove the worktree.
- [ ] **Stop. The pilot needs the owner's go-ahead** (DESIGN decision 4), and the go-ahead names the order
      (DESIGN decision 2). Do not start Part 2 on a default.

---

# Part 2: pilot on both pairs

The owner approved both pairs and left their order open. The orchestrator's order is P1 → P2-R and P2 in parallel →
P3 (R configuration, then P) → P4 (first report on both) → P5 (P only) → P6 (replay and documentation for both).
The G4 owner rating is still unrecorded. The field-read measurement remains a P5 decision. After stage D review,
align the fixture expectations and regenerate the public `ids.tsv` before P1.

Branch per task or slice from `main`, worktree `~/code/prism-boundary`. Every slice: tests first (RED),
implement, GREEN, the gates of ORCHESTRATION section 5 that apply (items 1 to 4, 8 and 9; no slice changes
the cache or the data-flow graph, so items 5 to 7 do not apply; the `CLAUDE.md` paragraph of item 10 is
written once, in P6), PR, two reviewers (cap 3 rounds).

P1 and the two producer slices were each priced at 3–4 days. P2-R starts with the table adapter. The memo's shorter
estimate assumed a ready listing. P3 was repriced at 4–5 days. P4 was priced at 3–4 days before A1. P5 takes 1 day
for a negative measurement or 5–6 with implementation, P6 3–4. These are slice estimates.

### File map

```
crates/prism-boundary/
  Cargo.toml                 depends on prism (path), serde, serde_json, toml, tree-sitter, clap, anyhow, sha2
  README.md                  usage, the documents, the facade candidates it uses
  schemas/boundary-bindings.schema.json
  schemas/boundary-impact.schema.json
  src/lib.rs                 public API re-exports
  src/main.rs                clap: bind | diff | impact
  src/element.rs             ElementId, FieldRoot, normalize_path_template
  src/model.rs               serde DTOs: BindingsDocument, Binding, Unbound, ImpactDocument, ElementChange, BindingStatus
  src/revision.rs            snapshot/tree, cutoff receipts, schema provenance, byte and structural hashes
  src/schema/mod.rs          SchemaIndex, Operation, Field
  src/schema/openapi.rs      Swagger 2.0 and OpenAPI 3 JSON reader, wire trees
  src/schema/route_table.rs  route-table schema source (P2-R, alongside public P2)
  src/schema/diff.rs         structural diff
  src/producer/mod.rs        bind_producer, adapter selection
  src/producer/swaggo.rs     annotation adapter
  src/producer/go_struct_fields.rs
  src/producer/route_table.rs
  src/consumer/mod.rs        bind_consumer
  src/consumer/config.rs     prism-boundary.toml
  src/consumer/resolve.rs    import → defining export (aliases, baseUrl, relative paths and re-exports)
  src/consumer/client_calls.rs
  src/consumer/url_pattern.rs
  src/consumer/matcher.rs
  src/consumer/field_reads.rs        (only if P5's measurement is positive)
  src/impact.rs              admit slots, diff and rematch pairs, freeze A1 JSON and render Markdown
  tests/                     one integration test file per slice, fixtures under tests/fixtures/
```

Root `Cargo.toml`: add the member; do not add it to `default-members`.

### Task E-1: element-identity matrix (stages T and E)

- [x] E-1 stages T and E delivered **217 rows** in `crates/prism-boundary/tests/fixtures/ids_matrix/rows.tsv` with
  IDs, fixtures, expected IDs and warnings, truth and class. The enumerators were still finding wire-tree cases.
  DESIGN 5.1 supplies the generator and section 9 partitions every row. P1 owns the identity test.

### Task E-2: client-recognition and matching matrix (stages T and E)

- [x] E-2 stages T and E delivered **306 rows** in `crates/prism-boundary/tests/fixtures/consumer_matrix/rows.tsv`
  with fixtures, config, grade, IDs, reason, truth and class, including neutral shapes s01–s45. URL-pattern and
  method enumeration was still growing. DESIGN 5.4 supplies the generator and section 9 partitions every row. P3
  owns the consumer test.

### Stage D: design of record

- [x] Draft the pilot rules against E-1 (165 rule, 52 accepted loss) and E-2 (288 rule, 18 accepted loss), both
  feasibility results, A1 and the provisional route-model contract. DESIGN 5.1–5.6, 7.2 and 9 govern the slice
  briefs.
- [ ] Both reviewers review the amendment, cap two rounds, then ORCHESTRATION section 8. Align the matrices'
  expectations and warnings with the reviewed design while retaining each row's semantic witness and recording
  accepted losses. Regenerate `ids.tsv` next. This design change edits only documents.

### Regenerate ids.tsv

- [ ] Branch `eval/schema-impact-ids-regen`: update `eval/schema_impact/ids.py` and `schema.py` to the
      amended DESIGN 5.1, run `schema-impact ids` on the pinned schema, commit the new
      `feasibility/ids.tsv` to the evidence directory and its diff summary to the PR body. Tests of Task F-1
      updated where the amendment changed a rule. One reviewer. This file is P1's golden file.

### Slice P1: crate, element ids, schema reader, diff, documents (3 to 4 days)

Implement DESIGN 5.1, 5.2, 5.6 and 9. The reader consumes named-tree JSON and admitted reference/config blobs.
Represent a snapshot as a commit or a computed tree, with optional commit metadata, blob sources and dependency
receipts. Keep typed literal, parameter, optional and tail tokens through normalization. Structural list/map edges
differ from literal property names. An operation includes all source sites, explicit success keys, media/body roots
and per-region coverage. A field records typed path, branch and definition, comparable properties and stop
conditions. Index sorted element IDs, operations reaching each definition and declaration provenance. Provide
`index_schema`, `definitions_of`, `operations_reaching` and `diff_schemas`.

The pilot element diff has the following `ChangeKind` variants. Return **all** applicable changes with the owning
operation, before/after element IDs, side-local sites, shapes and evidence. A status-root pairing can have
different IDs on the two sides. Keep the historical F4 selection as a separate evaluation filter.

```rust
pub enum ChangeKind {
    OpAdded, OpRemoved,
    RouteDeclarationChanged, ValidationChanged, HandlerReferenceChanged, HandlerCodeChanged,
    SuccessStatusAdded, SuccessStatusRemoved,
    FieldAdded, FieldRemoved, FieldTypeChanged, FieldFormatChanged, FieldItemTypeChanged,
    FieldRequiredAdded, FieldRequiredRemoved, EnumValuesAdded, EnumValuesRemoved,
}
```

A schema reader produces the applicable operation/status/field variants. The producer diff can add the
retained-edit variants when both source records exist. A client change is recorded separately as call
added/removed/retargeted or a one-sided observation with correspondence unknown. Preserve both sides of a mixed
enum change. Success-key rows include bodyless declarations. Group a unique comparable replacement for body diffing
by relative path and suppress child churn caused only by the root move. For ambiguous or overlapping ranges retain
the separate status rows and mark body comparison unchecked. An op addition/removal owns its field details.

**Tests and gate**

- Exercise all 217 E-1 dispositions from DESIGN 9 against the amended expected IDs, warnings or scoped refusal.
  For source-language fixtures at P1 feed a literal declaration/token projection with its dialect, flags and
  authority to the common ID builder. P2/P2-R test extraction into those inputs. Never report a projection as an
  end-to-end framework scan. Compare the public pinned schema's IDs byte-for-byte with the regenerated golden. Port
  applicable F-1 controls.
- Include server overrides/order, strict slashes, admitted relative references, wildcard and optional widths,
  punctuation round trips, array/map recursion, cycles and depth, inherited and overridden body parameters,
  exact/range/default status and media evidence. Test exact and range status keys and list/wildcard route methods
  under the applicable format. Keep an unread region distinct from a complete empty one.
- Test additions only, both directions for requiredness and value lists, format/item-only edits, a retained
  declaration edit, bodyless status loss, unique and ambiguous root replacement, unchanged definition rename, and
  formatting-only schema regeneration. Verify that missing evidence blocks the dependent change.
- Publish versioned JSON schemas and golden bindings/impact documents per DESIGN 5.2 and A1 **Input documents and
  machine-readable form**. Validate serialized examples against the schemas, including unbound raw calls, coverage
  without a key, configured provenance, mixed and client-only reviews, and null commit metadata on a computed tree.
  Reject a computed tree borrowing a commit date. Exercise tracked dirtiness separately from tree reads.
- [ ] Show the relevant RED on the exact base, then GREEN and the slice gates. Full-suite verification remains
  pending before merge to main.

### Slice P2: public producer bindings (3 to 4 days)

Run beside P2-R after P1. Use DESIGN 5.1–5.3 and 5.6. Extract P's admitted annotation and struct-field sites, from
the correct tree/blob, and preserve each authority and its own complete or partial regions. Keep annotations
distinct from registration proof, and schema sites distinct from handler/field sites. Compare decoded wire names
for struct tags. Preserve independent evidence when annotations and registrations disagree. P4 uses these producer
documents for the first report on P.

**Interfaces (produced):**

```rust
pub enum ProducerAdapter { SwaggoAnnotation, GoStructField }   // P2's public adapters
pub struct ProducerOptions { pub adapters: Vec<ProducerAdapter> }       // default: [SwaggoAnnotation, GoStructField]
pub fn bind_producer(session: &prism::navigation::NavigationSession, schema: &SchemaIndex,
                     snapshot: &SnapshotInput, opts: &ProducerOptions) -> BindingsDocument;
```

CLI: `prism-boundary bind --role producer --api <name> --repo <dir> --schema <file.json> --out <bindings.json>`.

**Seed rows** (stage T adds F1's residue rows; stage E runs the tiers):

| Row | Input | Required |
|---|---|---|
| g01 | one `@Router` line in a method's doc comment | one `bound` binding to that method |
| g02 | two `@Router` lines on one function | two bindings, same function |
| g03 | annotation, blank line, `func` | no binding; the operation is `operation_without_producer` |
| g04 | annotation path not in the schema | listed under `unbound` as `annotation_without_operation`; warning `schema_stale_against_code` |
| g05 | the same operation annotated on two functions | both as `candidate` |
| g06 | struct field tagged `json:"name,omitempty"` in `p.T` | every wire field whose `definition` is `p.T` and whose last segment is `name` is `bound` to it |
| g07 | field tagged `json:"-"` | nothing |
| g08 | untagged exported field | `unbound`, reason `untagged_field` |
| g09 | embedded struct whose fields the schema flattens into the outer definition (P: `UserChangeEmailSendCodeReq` embeds `UserVerifyEmailSendReq`; the schema lists `captcha_id` under the outer definition, and the embedded type is not a definition) | the promoted fields bind under every outer definition that lists them, with the site at the embedded struct's field; also under the embedded type when it is itself a definition; no binding for an element the schema does not have |
| g10 | generated file (`// Code generated … DO NOT EDIT.`) holding annotations | bindings kept; a note in `evidence` |
| g11 | two definitions with the same wire fields, one renamed | struct bindings follow the `definition` attribute; field ids unchanged |

**Gate.** On P, reach at least 98% annotation-bound operations (F1 observed 199 of 199). Account for every
contradiction in the original cross-check of 162 registrations. Preserve differing registration and annotation
evidence in the document. Neither a missing registration nor agreement on a suffix certifies a runtime handler.

### Slice P2-R: route-table producer and schema source (beside P2)

Implement DESIGN 5.1–5.3, 5.5 and 5.6 after P1. Start with the **prototype-derived route-table adapter**. It emits
operation declarations and source/coverage records from admitted route entries and spec at each revision. Put it
behind a route-source interface returning plugin identity where applicable, original method/path/tokens,
root/suffix evidence, constraints, declaration/endpoint sites, validation evidence and per-file/entry diagnostics.
The derived index has `schema.source.derived = "route_table"`, a structural `content_hash`, and no schema-file byte
hash. Keep declaration-site, handler-reference and handler-function coverage separate, especially for inline
handlers and imported validators. This source supplies operation-level impact.

CLI: `prism-boundary bind --role producer --api <name> --repo <tree> --route-table <spec> --out <bindings.json>`.
Compare two admitted route sets using the same interface as a schema-file source. Bind the spec and its plugin
selector, mount, method/path/handler/validation keys, resolver/lookup containers and era to the selected revisions.
Keep registration and configuration evidence separate. A suffix never acquires a public address by string
similarity.

**Second source, provisional dependency.** The route-model lane's listing requires its S1 and later Hapi slice (PR
#882 §11.5). P2-R can produce both documents and measurements from the first source while this dependency is
pending. When the reviewed listing is available, replace the extraction input behind the interface and run the same
contract fixtures. Do not sum duplicate declarations from the two readers. If the upstream contract changes, keep
the first source and report the difference before accepting the listing.

| Input or condition | Required record or refusal |
|---|---|
| Proven path from a root, matching method/protocol, constraints and unique endpoint or certified delegate | `bound` with `proven` provenance. Keep the proof and endpoint location. |
| `receiver.status=router`, `receiver.proof=plugin_contract`, `paths[].status=suffix`, `root=null`, `reason=no_known_mount` | Config must identify exactly one `receiver.plugin` by defining file, object_byte and effective name where known. Compose its configured mount with `paths[].value` and `path_tokens`. Match exactly one compatible operation on tokens, method/protocol and constraints, and require a resolved endpoint or certified delegate. Label a resulting unique binding `configured`. Keep config source/revision/entry, mount, plugin identity, original suffix and `mount_refusal` (`hapi_registration_unproven` or `no_known_mount`). The source record remains a suffix. |
| Missing/ambiguous plugin or mount, dynamic leaf, protocol/constraint mismatch, competing endpoints or only a pre-callback | Preserve declaration, refusal and possible sites. Zero configured/proven endpoint credit. Duplicate method/path declarations keep each candidate site. |
| Static entry with an inline expression, named local handler, imported handler or unresolved expression | A declaration site for each. Claim a handler site only from resolved endpoint evidence. An imported validation object needs its own admitted dependency. |
| Literal method list, true any-method, colon capture, optional segment, fixed-width or unlimited wildcard | Follow DESIGN 5.1 tokens and IDs, including `op:* <path>`. Test matching with an exact-method tie, width mismatch and a missing mount. |
| Unsupported path/method, unknown spread, changed registrar shape, skipped file or an entry outside the scoped registrar | Record the reason and affected region. A missing extraction does not prove deletion. A proved removal in comparable inventories gives `OpRemoved`. |

**Gate.** Pass the neutral declaration, token, provenance, refusal and derived-diff rows. On R, under private
custody, compare the share of route entries extracted with method/path to F5 on the same population (at least
prototype share minus two percentage points). Report numerator, census denominator, exclusions and source for each
result. F5 observed 399 declared routes, 393 inline handlers, 118 proven validation objects and 124 more with
imported/unseen validation. Its resolved-handler baseline was zero. Publish resolved endpoints, unresolved/inline
sites and proven versus configured bindings separately. The old handler threshold gives no positive evidence. The
provisional listing has zero root chains for about 400 entries, with 57 of 59 plugin prefixes supplied through
variables, and **configured yield is unmeasured** until this gate runs. The study's mount window begins in May
2023. Compare only revisions with admitted evidence for the mount applicable then. Run the private-name check
before any later publication.

### Slice P3: consumer inventories and matching (4 to 5 days)

Implement DESIGN 5.1, 5.2, 5.4–5.6 and 9. Run R's configuration first, then P's. Implement one
`match_operation(request, authority)` for bind-time optional previews and for impact-time comparison. Its result
retains each input pair, primary/alternates/overlaps, reasons, source and completeness. A consumer scan takes the
admitted client tree, root, config sources and caller depth. A schema is optional for extraction. Return a
versioned `BindingsDocument` with every counted call's raw target, site and caller evidence plus per-file/per-entry
coverage. A saved preview records the exact schema inventory used.

CLI:
`prism-boundary bind --role consumer --api <name> --repo <tree> --config <toml> [--schema <json>] --caller-depth 2 --out <bindings.json>`.

**Rows.** Enable the data-driven test over all **306** E-2 rows in the slice's first change. Test the amended rule
or the exact scoped accepted loss for every row, plus F-3 controls where the rule is retained. Exercise
alias/baseUrl and barrel resolution, fetch and axios aliases/factories, bound invocations, hook object keys and
inline fetchers, single-assignment templates, finite endpoint tables, mixed segments, multiline cleanup and unknown
spreads. Verify dynamic-method precedence, literal shadows with overlapping and disjoint typed domains, prefix
provenance, origin ambiguity, variable-length route tokens and exact-method/wildcard ties. Independently test
unknown file/region coverage. Record excluded syntactic forms separately from counted unbound sites.

| Row | Input | Required |
|---|---|---|
| c01 | client imported under another local name | recognised |
| c02 | client name rebound by a parameter or local | not a client call |
| c03 | a call of a member the configuration does not list (`request.upload(u)`); a client method passed as a hook's fetcher | the call: in the denominator, `unbound`, reason `unknown_client_method`; the fetcher argument: not a call site |
| c04 | hook key that is an array whose first element is a URL `const` | bound by the first element |
| c05 | URL `const` assigned twice | not followed; `dynamic` |
| c06 | call in an anonymous callback | binding kept; `callers` may be empty |
| c07 | `fetch(url, { method: 'POST' })`, `fetch(url)`, `fetch(url, opts)` with non-literal `opts` | POST; GET; `unbound`, reason `dynamic_method` |
| c08 | two operations tie on score | both `candidate` |
| c09 | `strip_prefix` and `add_prefix` set | stripped, then added, before matching |
| c10 | call bound, enclosing function called from two components | both in `callers` at depth 2 |
| c11 | client parameter facing only one feasible literal | `candidate`, reason `client_parameter_on_literal`. A disjoint declared domain cannot match |
| c12 | hook whose fetcher is `request.instance.post` | method POST |
| c13 | client imported through a barrel and through a relative path | both recognised |
| c14 | `${apiUrl}?q=${x}` with `apiUrl` a module-level literal `const` | literal path of the `const` |
| c15 | a client shape the grammar cannot express | in the denominator, `unbound`, reason `unconfigured_client_form` |

**Gate.** All 306 row dispositions are accounted for, including reason and denominator checks for the losses. On P,
the bound share must be at least 75% (F2's 77% minus two percentage points). Report any change in denominator under
the revised counting rule. On P and, locally under private custody, on R, allow at most two wrong in a fresh
**decided** 50-row sample of bound joins per pair (DESIGN 6.2). Print pair-specific counts by grade, basis and
reason. R's study baseline is 236 bound, 2 candidate and 78 unbound out of 316, with 83.9% recoverable patterns.
Report the product's own result alongside this baseline.

### Slice P4: diff, impact JSON and Markdown (both pairs)

Include DESIGN 5.1, 5.2, 5.4–5.6 and REPORT-DESIGN.md (A1) in this slice's brief. Implement A1's product evidence,
JSON and rendering tables. The deliverable is the first product report on **both** pairs. `prism-boundary diff`
reads the two admitted schema or route-table sources for a service diff.
`prism-boundary impact --comparison <manifest> [--changed <elements.json>] --format json|md` selects input slots
and direction. The manifest references role documents, trees, mode/cutoff, config provenance and dependency
receipts. Keep the service-diff shorthand `--old <source> --new <source> --bindings <file>...` for identified
service pairs. Resolve the same slots and mark missing cutoff or configuration evidence unchecked in either form.
For service changes compare two service snapshots to one client. For client changes compare two clients to one
service. For joint changes check both actual pairs and both rollout orders. Schema files are required only for
schema claims. Validate a supplied change list against its admitted sources, expand definition IDs, and apply the
same coverage checks.

**Comparison and rendering gates**

1. Rematch **every** admitted raw call, including previously unbound ones. Assert API, role, tree, scope,
   configuration and dependencies per slot. Exercise live, historical and selected-version modes, a missing ref or
   saved inventory, wrong revision, dirty substitutes, equal timestamps, target-to-merge in both repository setups,
   a receipt-bound computed tree with exact-blob links, and conflict without a result. Verify saved-current/stale,
   present-with-no-saved-match, established absence and unchecked alongside fresh matches. Form `unobserved` only
   from complete admitted membership, including producer bindings.
2. Golden JSON must validate against the A1 impact schema. Exercise additions/removals, status-only and mixed
   field edits, retained declaration/handler edits, client edits with and without correspondence, wildcard ties,
   lower overlaps and prefix hints. Remove each required evidence record in turn and mark a file or entry unknown
   on either side. Verify all affected cells, totals, notifications and empty states. On R show configured and
   proven results separately and fields unchecked without a schema.
3. The product's `impact --format md` renders that frozen JSON as the A1 review: plain-language result and action,
   versions and coverage, one group per operation with service evidence first, both client matches, callers,
   limits, standing differences, bounded preview and complete linked Markdown appendix. Check the exact source/side
   of each link, all preview limits including lists inside cells, and one count per comparable site. Produce a
   separate evaluation companion. Change or remove hindsight records after freezing and require byte-identical live
   Markdown and JSON.
4. **Three-report replay gate.** Freeze three of the ten public replay subjects, their version-2 A1 prototype
   reports, comparison modes and input receipts before running the product. Compare the product's Markdown and
   backing JSON with those **version-2** reports for the same subjects and slots. Require the same evidenced
   consumer locations on shared coverage, supported before/after results, direction, version basis, unknowns,
   actions and appendix links. A product-only claim (e.g. resolved callers or wildcard matching) needs its
   additional evidence and an explicit reconciliation. A missing version-2 reference keeps this gate pending. The
   original memo renderings are historical evidence.
5. Render one first report per pair at admitted versions. Keep R's artifacts local. Publish only aggregates.
   Verify client-first, service-first and joint directions on neutral fixtures even when the three public subjects
   exercise only service edits.

### Slice P5: field level (5 to 6 days; 1 day if step 1 is negative)

Run P5 on P only. P4 supplies call/operation links for both pairs. On R the route source has no committed field
schema. On P use DESIGN 5.1, 5.2 and 5.4 and A1's JSON-media, status and per-path coverage rules. A matched
operation, a field spelled in source and a linked field read are separate evidence records. If the measurement
fails, each affected report still states that client field use was not checked.

**Step 1, Task F-4 (measure F3), run first.** Branch `eval/schema-impact-field-reads`.

**Files:**
- Create: `eval/schema_impact/fields.py`, `eval/schema_impact/oracle/field_refs.mjs`,
  `eval/tests/schema_impact/test_si_fields.py`

**Interfaces (produced):**
- `fields.select_operations(idx, calls, n=12, seed=20261120) -> list[op_id]`: eligible operations have a
  uniquely bound client call with a `response_type` that names a type (not an inline type), that named type
  is the `response_type` of no other bound call, and the response under `response_root` is object-valued.
  Classes: *list* (GET whose response under the root is an array or has an array property named `list`),
  *detail* (other GET), *mutation* (POST, PUT, DELETE). Sort eligible operations by id, shuffle with the
  seed, take up to 4 per class, and fill a short class from the others. Record the class sizes and the
  number of operations excluded as inline or shared.
- `fields.start_lines(call, callers_json) -> list[(file, line)]`: the call's own line, and, when the call is
  the returned value of its enclosing function (a `return` operand or an arrow body), the line of every
  call site of that function that `prism nav callers --symbol <function> --file <file> --format json`
  reports (depth 1).
- `fields.reads_from_edges(edges_jsonl, start_lines, response_root, max_hops=6) -> set[(file, line, path)]`:
  start from every edge endpoint that is a definition on a start line; follow edges forward, of any label
  except `doubt: "killed"` (the definition does not reach that use), across functions, up to `max_hops`; collect use endpoints whose access path has at least one field,
  rendered `f1.f2` from the record's `path` object (base dropped). `response_root` is the configured root:
  empty when the client already returns the part under it.
- `oracle/field_refs.mjs <tsconfig> <type name> <property>`: prints `file:line` of every reference to that
  property, with the TypeScript at `~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3`.

**Scoring.** For a *detail* or *mutation* operation, the scored fields are `idx.response_fields(op,
response_root)`; for a *list* operation, the property names one level below the array (under
`#/<root>/[]` or `#/<root>/list/[]`). *Found*: an oracle reference and a prism read at the same file and line
whose path starts with that field (after the array step for list operations). *Missed*: an oracle reference
with no such read. *Wrong*: a prism read whose path, after the same step, does not start with one of the
fields.

- [ ] In the clone's `ui/` directory, install the client dependencies with the package manager its lockfile
      names, scripts disabled (`--ignore-scripts`) and the lockfile frozen. This needs the network. If it
      fails, write "F3 oracle: reader-labelled" in the ledger; a Sonnet-5.5 reader then lists the reads of
      each field by reading the call's file and its callers.
- [ ] Unit-test `reads_from_edges` on a hand-written JSONL in the shape `nav dfg-stats --edges` prints
      (`{"from":{"file","line","path":{"base","fields"},"access"},"to":{…},"confidence":…,"doubt":…}`): a
      call-line definition, a reassignment, two member reads, one cross-function edge, one unrelated edge,
      one read reached only by a `killed` edge; expect the member paths, without the killed one. Implement. Commit.
- [ ] Build prism for the dump: `~/prism-evidence/tools/memguard.sh 8 cargo build --release` in the
      worktree; `target/release/prism --version` must show the worktree's `git rev-parse --short HEAD`.
- [ ] Dump edges once: `memguard.sh 8 target/release/prism nav --no-cache dfg-stats --repo <P clone>
      --edges > feasibility/edges.jsonl`.
- [ ] For the selected operations: oracle references from `field_refs.mjs` with the client's `tsconfig.json`
      (or the reader). Compare with `reads_from_edges`.
- [ ] `feasibility/F3.md`: per operation: fields, oracle reads, found, missed, wrong; the totals; and the
      three most common reasons for a miss (for example: value stored in component state, value passed
      through a hook). One reviewer on the eval PR.
- [ ] **Decision:** at least 50% of oracle reads found with at most 10% wrong → continue with step 2.
      Otherwise P5 ends here: the pilot is operation-level with producer struct fields only; record the
      DESIGN section 7 row in the ledger.

**Step 2, product.** Add `prism::api::dfg_edges(session: &NavigationSession) -> impl Iterator<Item =
DfgEdgeRecord>` to the core crate (the records `nav dfg-stats --edges` prints), exported `#[doc(hidden)]`
outside the facade's compatibility promise, as `OwnerFailure` and `OwnerArgs` are in `src/api/mod.rs`,
with its own test in the core crate. Then `consumer/field_reads.rs`: step 1's procedure and scoring, in
Rust, matching reads to wire fields under the configured `response_root`. Rows come from F3's miss reasons;
each miss class is either a row that must pass or an accepted loss named in the README. **Gate:** on step 1's
operations, found reads at least F3's figure, wrong reads at most 10%.

### Slice P6: replay gate and documentation for both pairs (3 to 4 days)

Both producer adapters and both consumer configurations were scheduled before P4. P6 runs the product replay and
documents the final per-pair results. Keep the evaluation companion and measured history window distinct from the
A1 live review. R's configured mount must be admitted for each replayed version. Report the private replay only in
aggregates, without attributing lift where there were no controls.

- [ ] `eval/schema_impact/replay.py` gains `--engine product` (runs `prism-boundary bind`, `diff` and `impact`
      instead of the prototype extractors). **Gate:** every location the memo counted is predicted again,
      and the outcome shares (`same_commit`, `later`, `present_at_pin`) are within 5 points of the memo's. Score the memo's historical cohort under its recorded F4 definitions in the evaluation companion. A1 live review includes all supported additions and edits. Record any lost location with its cause and keep a missed gate visible.
- [ ] Once that gate passes, remove the prototype's extractors, matcher and schema reader
      (`producer_go.py`, `producer_route_table.py`, `consumer_ts.py`, `match.py`, `mismatch.py`, `config.py`,
      `schema.py`, `ids.py`, `fields.py` and their tests) in the same PR; keep the replay driver and the
      renderer.
- [ ] `crates/prism-boundary/README.md`, a short user guide `docs/boundary-impact.md`, a paragraph in
      `CLAUDE.md` (crate, documents, grades, what is out of model).
- [ ] Re-run the P2, P2-R and P3 gates for their respective pairs, with configured/proven counts, denominators,
  unbound reasons and coverage. Document the P4 version-2 report comparison and P5 measurement outcome. Keep
  private evidence local.
- [ ] Close the tracking issue with the acceptance numbers; file follow-ups for protobuf, federation, MCP
      exposure and a Go or Python service-consumer adapter as separate issues, not started.

## Acceptance for the lane

The memo records feasibility separately from owner ratings. Product acceptance requires the P2, P2-R, both P3 and
both-pair P4 gates, P5's measured decision and P6's replay gate on the final product head. Record each review
outcome and every unbound reason and count (P publicly, R under private custody). State the route source used,
proven/configured counts, accepted losses and remaining coverage. Run the full suite before merging product changes
to main.
