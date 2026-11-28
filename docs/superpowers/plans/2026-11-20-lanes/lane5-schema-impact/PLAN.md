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
  schedules the two producer slices together and delivers both first reports in P4 (DESIGN 7.2).
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

Part 1 preserves the feasibility protocol and its historical expected values. The amended product rules,
inputs and gates are in Part 2 and DESIGN §§5, 7 and 9.

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
- [x] The owner approved both pairs and left their order open. DESIGN 7.2 records the applied schedule. The
      five-report rating is still unrecorded. It affects G4 and the historical recommendation.

---

# Part 2: pilot on both pairs

The owner approved both pairs and left their order open. The orchestrator's order is D-census → P1 → P2-R and P2 in parallel →
P3 (R configuration, then P) → P4 (first report on both) → P5 (P only) → P6 (replay and documentation for both).
The G4 owner rating is still unrecorded. The field-read measurement remains a P5 decision. After stage D review,
align the fixture expectations and regenerate the public `ids.tsv` before P1. D-census is a separate pre-implementation measurement.

Branch per task or slice from `main`, worktree `~/code/prism-boundary`. Every slice: tests first (RED),
implement, GREEN, the gates of ORCHESTRATION section 5 that apply (items 1 to 4, 8 and 9; no slice changes
the cache or the data-flow graph, so items 5 to 7 do not apply; the `CLAUDE.md` paragraph of item 10 is
written once, in P6), PR, two reviewers (cap 3 rounds).

P1 and P2-R were priced at 3–4 days. Allow about one extra day in P2 for registration evidence (4–5 total). P2-R starts with the table adapter. The memo's shorter
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
  src/producer/registrations.rs   P2 evidence-only reader
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

- [x] Set the final pilot rules against E-1 (170 rule, 47 accepted loss) and E-2 (288 rule, 18 accepted loss), both
  feasibility results, A1 and the provisional route-model contract. DESIGN 5.1–5.6, 7.2 and 9 govern the slice
  briefs.
- [x] Both seats rejected round 1. In round 2 Sol rejected and Opus approved. The design seat folded both sets,
  addressing EVIDENCE-INPUT and GRAMMAR as recurring classes. The targeted round 3 fold adds the directional
  capacity control, changes two bound expectations to candidate and specifies the fixture mapping controls.
  DESIGN records the dispositions. The 523 ledger rows comprise 458 rule and 65 accepted loss.
- [ ] After review, apply ORCHESTRATION sections 7–8 and the DESIGN **Alignment list**. Align the matrices'
  expectations and warnings with the reviewed design while retaining each row's semantic witness and recording
  accepted losses. Record a fresh base RED/GREEN for each amended row before enabling it. Regenerate `ids.tsv` next. This design change edits only documents.

### Regenerate ids.tsv

- [ ] Branch `eval/schema-impact-ids-regen`: update `eval/schema_impact/ids.py` and `schema.py` to the
      amended DESIGN 5.1, run `schema-impact ids` on the pinned schema, commit the new
      `feasibility/ids.tsv` to the evidence directory and its diff summary to the PR body. Tests of Task F-1
      updated where the amendment changed a rule. One reviewer. This file is P1's golden file.

### Task D-census (pre-implementation measurement, before P1)

- [ ] Pin the reference tool/build, configs and populations from DESIGN 7.2. Freeze the cut predicates, units
      and expected effects before counting. Measure P at `6744295f`. The authorized local R measurement seat
      uses the F5 pins and returns aggregates. Keep replay cases bound to their selected versions.
- [ ] Run every cut in the DESIGN 7.2 table individually on the applicable pinned inputs. For schema coverage,
      include direct YAML, media/body/status exclusions (including default), depth, tuple and assertion/ref
      limits, cycles, saturation, closed minima and conjunct remainders. For routes, include implicit methods,
      registrar/star/options, constrained dispatch, root/mount, prefixes/slashes, endpoint proof, fallback role
      and unplaced suffixes. For calls, include transport/tag/hook/wrapper gaps, defaults/helpers, writes and
      overflow, numeric and method/URL arm limits, missing export mappings or origin, other APIs, selectors
      without a distinct operation, and domain/score ties. Include saved inventories, lost reference locations,
      unavailable result definitions and field-read linkage in the report and field populations.
- [ ] Record one line per cut and pair with source scope, inspected denominator, affected count and rate,
      before/after outcomes on the same members, overlaps and deduplicated net loss. A checked zero cites the
      inspected population, an inapplicable population cites its basis, and unknown incidence stays pending.
      Use reference evidence and source inspection to apply the final rules before product code exists. Freeze
      the receipt before P1, refer material loss to the design seat, and reconcile later slice results against
      it. The declaration-only receipt is summarized in DESIGN 7.2. Prepare the producer evidence independently
      of replay outcomes.
- [ ] Before freezing the P02 media decisions, have an independent reader check all 33 original location
      keys against both historical producer trees without seeing later replay outcomes. For each side, reopen
      the schema and verify the applicable declaration. For omitted media, check admission and trace the cited
      Go source inventory through final wire media under DESIGN 5.1. Record the reader's decision and source
      spans or gap. Reconcile disagreements against those blobs. Freeze a location as cleared only when both
      sides pass and the independent check agrees.
      Keep each refusal and its reason. Account for all 33 original keys exactly once, then recount S04 and
      reconcile P02 with the owner-approved dispositions in P6.

### Slice P1: crate, element ids, schema reader, diff, documents (3 to 4 days)

Implement DESIGN 5.1, 5.2, 5.6 and 9. The reader consumes named-tree JSON and admitted reference/config blobs.
Represent a snapshot as a commit or a computed tree, with optional commit metadata, blob sources and dependency
receipts. Keep typed literal, parameter, optional and tail tokens through normalization. Structural list/map edges
differ from literal property names. An operation includes all source sites, explicit success keys, media/body roots
and per-region coverage. A field records typed path, branch and definition, comparable properties and stop
conditions. Index sorted element IDs, operations reaching each definition and declaration provenance. Provide
`index_schema`, `definitions_of`, `operations_reaching` and `diff_schemas`.

Preserve Swagger's trailing slash on `PUT /answer/admin/api/report/` under DESIGN §5.1. Equivalence to the
router's unslashed path under a stated rule decides the two report-route locations in P6. See P3 for the join.

The pilot element diff has the following `ChangeKind` variants. Return **all** applicable changes with the owning
operation, before/after element IDs, side-local sites, shapes and evidence. A status-root pairing can have
different IDs on the two sides. Keep the historical F4 selection as a separate evaluation filter.

```rust
pub enum ChangeKind {
    OpAdded, OpRemoved,
    RouteDeclarationChanged, ValidationChanged, HandlerReferenceChanged, HandlerCodeChanged,
    SuccessStatusAdded, SuccessStatusRemoved,
    FieldAdded, FieldRemoved, FieldTypeChanged, FieldFormatChanged, FieldItemTypeChanged,
    FieldRequiredAdded, FieldRequiredRemoved, EnumDeclarationChanged, EnumValuesAdded, EnumValuesRemoved,
}
```

A schema reader produces the applicable operation/status/field variants. The producer diff can add the
retained-edit variants when both source records exist. A client change is recorded separately as call
added/removed/retargeted or a one-sided observation with correspondence unknown. Preserve both sides of a mixed
enum change. Emit independent added and removed success-key rows, including bodyless declarations.
Group those rows as a status change only when DESIGN §5.1 proves exactly one outgoing and one incoming
root with equivalent JSON media. Compare their fields by relative path, retain both status-bearing IDs,
and emit no child edits for a root move alone. If §5.1 blocks pairing, retain the separate status rows
and record why body comparison was unchecked. An operation addition or removal owns its field details.

**Tests and gate**

- Exercise all 217 E-1 dispositions from DESIGN 9 against the amended expected IDs, warnings or scoped refusal.
  For source-language fixtures at P1 feed a literal declaration/token projection with its dialect, flags and
  authority to the common ID builder. P2/P2-R test extraction into those inputs. Never report a projection as an
  end-to-end framework scan. Compare the public pinned schema's IDs byte-for-byte with the regenerated golden. Port
  applicable F-1 controls.
- Include server overrides/order, strict slashes, admitted relative references, wildcard and optional widths,
  punctuation round trips, array/map recursion, cycles and depth, inherited and overridden body parameters,
  exact/range/default status and media evidence. Test exact and range status keys and list/wildcard route methods
  under the applicable format. Run the DESIGN 9 companion to `swagger-default-exclusion` with and without its
  scoped producer record. Keep an unread region distinct from a complete empty one.
- Test additions only, both directions for requiredness, omitted ↔ empty enum declaration (with zero member deltas)
  and member changes, format/item-only edits, a retained
  declaration edit, bodyless status loss, unique and ambiguous root replacement, unchanged definition rename, and
  formatting-only schema regeneration. Verify that missing evidence blocks the dependent change.
- Verify zero and residual capacity before literal and map edges. Add required `kept` to the one-slot object
  in DESIGN 5.1. Require its requiredness edit and loss of both the optional field and map descendant, then
  restore spare capacity. Test a closed minimum just feasible and one above the feasible key count. Apply
  each conjunct's false, omitted or schema-valued remainder to the other member's named and required-only
  keys, including incompatible values. Verify the checked-empty `t1-allof-closed-intersection` and partial
  coverage for unsupported applicators. Keep false optional, composition-only/root cycle and unaffected sibling
  controls. Reconcile P1 schema coverage counts with D-census.
- Add `maxProperties: 2` to `Record` in a control derived from `t1-directional-annotations`. Reserve only
  `secret` for requests and only `serial` for responses. Check the feasible request with `secret` and `label`
  and the feasible response with `serial` and `label`. Both roots must still emit all three field IDs with
  their directional requiredness, including `#/label` on each side and zero `FieldRemoved` changes.
- Publish versioned JSON schemas and golden bindings/impact documents per DESIGN 5.2 and A1 **Input documents and
  machine-readable form**. Validate serialized examples against the schemas, including unbound raw calls, coverage
  without a key, both element IDs on a paired status-root move, absent versus unknown sides, an admitted saved
  scan/inventory with zero matches and its missing-dependency control, four per-comparison match maps for a
  joint edit, configured provenance, mixed and client-only reviews, and null commit metadata on a computed tree.
  Reject a computed tree borrowing a commit date. Exercise tracked dirtiness separately from tree reads.
- [ ] Show the relevant RED on the exact base, then GREEN and the slice gates. Full-suite verification remains
  pending before merge to main.

### Slice P2: public producer bindings and registration evidence (4 to 5 days)

Run beside P2-R after P1. Use DESIGN 5.1–5.3 and 5.6. Extract P's admitted annotation and struct-field sites, from
the correct tree/blob, and preserve each authority and its own complete or partial regions. Keep annotations
distinct from registration proof, and schema sites distinct from handler/field sites. Compare decoded wire names
for struct tags. Preserve independent evidence when annotations and registrations disagree. P4 uses these producer
documents for the first report on P. Port the bounded literal registration reader from
`eval/schema_impact/producer_go.py` to the boundary crate. Read literal verb calls, final handler argument and
same-function literal group prefixes at each admitted tree. Freeze original method, path, handler and span,
root/suffix status and coverage. This reader populates comparison evidence only. The binding adapter list stays
annotation and struct fields. No cross-function Gin resolver or framework-entries change is required.

**Interfaces (produced):**

```rust
pub enum ProducerAdapter { SwaggoAnnotation, GoStructField }   // binding sources only
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
| g03 | annotation, blank line, `func`, including an agreeing registration | Missing annotation binding. Preserve the registration and withhold any stale warning from the gap alone. |
| g04 | annotation path absent from a complete schema inventory | Record `annotation_without_operation`. Test a linked, comparable registration contradicting the schema and a missing/partial registration. Only demonstrated schema/code disagreement earns `schema_stale_against_code`. |
| g05 | the same operation annotated on two functions | both as `candidate` |
| g06 | struct field tagged `json:"name,omitempty"` in `p.T` | every wire field whose `definition` is `p.T` and whose last segment is `name` is `bound` to it |
| g07 | field tagged `json:"-"` | nothing |
| g08 | untagged exported field | `unbound`, reason `untagged_field` |
| g09 | embedded struct whose fields the schema flattens into the outer definition (P: `UserChangeEmailSendCodeReq` embeds `UserVerifyEmailSendReq`; the schema lists `captcha_id` under the outer definition, and the embedded type is not a definition) | the promoted fields bind under every outer definition that lists them, with the site at the embedded struct's field; also under the embedded type when it is itself a definition; no binding for an element the schema does not have |
| g10 | generated file (`// Code generated … DO NOT EDIT.`) holding annotations | bindings kept; a note in `evidence` |
| g11 | two definitions with the same wire fields, one renamed | struct bindings follow the `definition` attribute; field ids unchanged |

**Gate.** On P, reach at least 98% annotation-bound operations (F1 observed 199 of 199). Account for every
contradiction in the original cross-check of 162 registrations. Preserve differing registration and annotation
evidence in the document. Reconcile all 162 study cross-checks against fresh registration records by site,
including uncovered and contradictory cases, and record the registration/declaration census per DESIGN 7.2.
Drop the annotation in an agreeing case and vary the method and path separately in linked drift cases. Assert
both authorities and zero bindings from the evidence reader.

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
Implement every policy field in DESIGN 5.3, including custom star sets, verb case, capture dialect and joins,
version defaults, router options and entry-option keys. Test identical star entries under all, finite and absent
policies, a missing/unknown option, internal-only, vhost competitors and a scoped mount. Run each through actual
extraction as well as the P1 projection. Inventory uses of each option on R before assessing the gate.
Keep registration and configuration evidence separate. A suffix never acquires a public address by string
similarity.

**Second source, provisional dependency.** The route-model lane's listing requires its S1 and later Hapi slice (PR
#882 §11.5). P2-R can produce both documents and measurements from the first source while this dependency is
pending. This exception reuses the prototype in the boundary crate while lane 12 owns framework extraction.
When the reviewed listing is available, replace the extraction input behind the interface and run the same
contract fixtures. Do not sum duplicate declarations from the two readers. If the upstream contract changes, keep
the first source and report the difference before accepting the listing.

| Input or condition | Required record or refusal |
|---|---|
| Proven path from a root, matching method/protocol, constraints and unique endpoint or certified delegate | `bound` with `proven` provenance. Keep the proof and endpoint location. |
| `receiver.status=router`, `receiver.proof=plugin_contract`, `paths[].status=suffix`, `root=null`, `reason=no_known_mount` | Config must identify exactly one `receiver.plugin` by defining file, object_byte and effective name where known. Compose its configured mount with `paths[].value` and `path_tokens`. Match exactly one compatible operation in the derived declaration index on tokens, method/protocol and constraints, and require a resolved endpoint or certified delegate. Label a resulting unique binding `configured`. Keep config source/revision/entry, mount, plugin identity, original suffix and `mount_refusal` (`hapi_registration_unproven` or `no_known_mount`). The source record remains a suffix. |
| Missing/ambiguous plugin or mount, dynamic leaf, protocol/constraint mismatch, competing endpoints or only a pre-callback | Preserve declaration, refusal and possible sites. Zero configured/proven endpoint credit. Duplicate method/path declarations keep each candidate site. |
| Static entry with an inline expression, named local handler, imported handler or unresolved expression | A direct function/arrow literal supplies its own endpoint span. A name needs a resolved target, a factory/delegate needs separate proof. Record a declaration site even on refusal. An imported validation object needs its own admitted dependency. |
| Literal method list, true any-method, colon capture, optional segment, fixed-width or unlimited wildcard | Follow DESIGN 5.1 tokens and IDs, including `op:* <path>`. Test matching with an exact-method tie, width mismatch and a missing mount. |
| Unsupported path/method, unknown spread, changed registrar shape, skipped file or an entry outside the scoped registrar | Record the reason and affected region. A missing extraction does not prove deletion. A proved removal in comparable inventories gives `OpRemoved`. |

**Gate.** Pass the neutral declaration, token, provenance, refusal and derived-diff rows. On R, under private
custody, compare the share of route entries extracted with method/path to F5 on the same population (at least
prototype share minus two percentage points). Report numerator, census denominator, exclusions and source for each
result. F5 observed 399 declared routes, 393 inline handlers, 118 proven validation objects and 124 more with
imported/unseen validation. Its resolved-handler baseline was zero. Publish counts for direct literals, resolved references, unresolved
expressions and proven versus configured bindings separately. The old handler threshold gives no positive evidence. The
provisional listing has zero root chains for about 400 entries, with 57 of 59 plugin prefixes supplied through
variables, and **configured yield is unmeasured** until this gate runs. The study's mount window begins in May
2023. Compare only revisions with admitted evidence for the mount applicable then. Run the private-name check
before any later publication. On the study population the design predicts up to 393 recovered inline endpoint
sites from the literal rule (393 if all reported inline functions are directly in handler position and admitted).
Measure this separately from uniquely mounted/compatible bindings. Record all P2-R cut counts from DESIGN 7.2,
including possible suffix-only negatives and the non-API fallback. Exercise the role selector by exact declaration
span and by numbered entry, including a stale selector. With only a mounted GET /api/a and an undesignated root
tail, GET /api/b gives `unbound/fallback_only`, partial role coverage and no target. A designated fallback is
overlap only. An ordinary tail in evidenced API scope remains eligible. Check the possible unplaced suffix override.

### Slice P3: consumer inventories and matching (4 to 5 days)

Implement DESIGN 5.1, 5.2, 5.4–5.6 and 9. Run R's configuration first, then P's. Implement one
`match_operation(request, authority)` for bind-time optional previews and for impact-time comparison. Its result
retains each input pair, primary/alternates/overlaps, reasons, source and completeness. A consumer scan takes the
admitted client tree, root, config sources and caller depth. A schema is optional for extraction. Return a
versioned `BindingsDocument` with every counted call's raw target, site and caller evidence plus per-file/per-entry
coverage. A saved preview freezes the actual inventory and its dependencies under DESIGN 5.2 even on an empty
match set. Version both public era configs and R's scoped config. Bind each hook export and default fetcher to
its source or mark defaults unconfigured, record document URL and producer origins, and specify wrapper
normalization and zero-based options positions. Test the Alignment list's numbered identity-prefix mapping
to each configured client's own fixture API after applying its declared prefix policy once. In the 37 listed
template-client configs, admit the mapping with its factory and settings dependencies and check all 47 listed
invocations, including `s20` and `s22-3`. With just the API mapping withheld, keep the observed site, prefix
and readable request components. Assert **Not checked** (`origin_unchecked`), zero confirmed targets and zero notifications.

For the P6 report-route locations, the router and client use `/answer/admin/api/report`, while P1 retains
Swagger's `/answer/admin/api/report/`. Prove equivalence for this route under a stated rule or explain the miss.

Carry-ins from the P1a review (#936): the checked ids spell a path-parameter capture `{}` (or `.{}`) whatever the
parameter schema admits. A schema may admit the empty string (`enum: [""]`), so P3's matcher admits an empty capture
from the parameter schema (for example `/reed/x` and `/reed/x.` under `x{}` and `x.{}`) while the ids stay unchanged;
the non-empty rule of DESIGN 5.1 is the default for a capture with no such schema evidence, and DESIGN's wording of
that bullet is reconciled in this slice.

CLI:
`prism-boundary bind --role consumer --api <name> --repo <tree> --config <toml> [--schema <json>] --caller-depth 2 --out <bindings.json>`.

**Rows.** Enable the data-driven test over all **306** E-2 rows in the slice's first change. Test the amended rule
or the exact scoped accepted loss for every row, plus F-3 controls where the rule is retained. Exercise
alias/baseUrl and barrel resolution, fetch and axios aliases/factories, bound invocations, hook object keys and
inline fetchers, single-assignment templates, finite endpoint tables, mixed segments, multiline cleanup and unknown
spreads. Verify dynamic-method precedence, literal shadows with overlapping and disjoint typed domains, prefix
provenance, origin ambiguity, variable-length route tokens and exact-method/wildcard ties. Independently test
unknown file/region coverage. Record excluded syntactic forms separately from counted unbound sites.
Check `url-concat-parenthesized` on numeric and opaque operands, and a readable GET alternative paired with
an opaque method on the same readable URL. Require the GET candidate with `dynamic_method`, or unbound with
that reason if GET has no match. Exercise an empty component against `{}`, `{?}` and `{*0+}` with the
separator policy fixed. At mapped `s05`, require bound `op:GET /mothbank/orchards/{}` with no unmatched arm.
The whole-component string variable is assumed non-empty under the syntactic path model.
At mapped `s09` and `s09-2`, the declared string domain gives `candidate/ambiguous_operation` with the two
candidate IDs in each Alignment list entry and zero notifications. Check the conditional overlap with the other
mixed declaration at each site under the suffix or prefix condition in that entry. Substitute
the literal `""` to bind only the generic route. Substitute `"wax"` to get `%2Fwax` and `wax-ridge-gold`,
respectively. At each site both routes match the two whole literals. With these primary counts equal, require
one affix point for the mixed route and zero for the generic route, giving a bound mixed route with the generic overlap. With a symbolic numeric
substitution, require the same binding by structural pattern equality. In `matching-tie-without-parameter-on-literal`,
both compatible GET declarations for `/press/copper/leaf` score two whole literals and zero affix points.
Require `candidate/ambiguous_operation` with both IDs.

| Row | Input | Required |
|---|---|---|
| c01 | client imported under another local name | recognised |
| c02 | client name rebound by a parameter or local | not a client call |
| c03 | a call of a member the configuration does not list (`request.upload(u)`); a client method passed as a hook's fetcher | the call: in the denominator, `unbound`, reason `unknown_client_method`; the fetcher argument: not a call site |
| c04 | hook key that is an array whose first element is a URL `const` | bound by the first element |
| c05 | Local initialized to `/reels`, conditionally assigned an opaque URL on a live arm | Mutable fixture gives `candidate/conditional_url`, GET /reels and an uncovered arm. The original const assignment throws on that arm, leaving only GET /reels at the call. |
| c06 | call in an anonymous callback | binding kept; `callers` may be empty |
| c07 | `fetch(url, { method: 'POST' })`, `fetch(url)`, `fetch(url, opts)` with opaque `opts` | POST; GET; `unbound`, reason `dynamic_method` |
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
the revised counting rule. Apply DESIGN 6.2 on each pair, locally under private custody on R. Draw 50 bound
joins when available (at most two wrong, with the existing one-time extension for an undecided sample). With
1–49 bound joins inspect all, require at least 80% decided rounded up and at most two wrong. Zero bound joins
or an exhausted undecided population is inconclusive. Print pair-specific counts by grade, basis and
reason. R's study baseline is 236 bound, 2 candidate and 78 unbound out of 316, with 83.9% recoverable patterns.
On the same 316 occurrences give the transition from each old grade, including retained configured joins out
of the original 236 (same site and same operation), lost/retargeted joins by reason, new joins, denominator
changes and net delta. Count product-only joins separately. The owner decision on minimum retained configured
yield defaults to **report, no floor** (DESIGN 8). P4 still requires a shared evidenced join on R. On both
pins reconcile actual per-cut counts to D-census in DESIGN 7.2. Include unknown origins/defaults, mixed branches, finite crossings, fallback-only, suffix competitors,
selector and other-API controls. Check their effects on negatives, totals and notifications.

### Slice P4: diff, impact JSON and Markdown (both pairs)

Include DESIGN 5.1, 5.2, 5.4–5.6 and REPORT-DESIGN.md (A1) in this slice's brief. Implement A1's product evidence,
JSON and rendering tables, applying DESIGN 5.1's media rule and citing the source for each JSON comparison.
The deliverable is the first product report on **both** pairs. `prism-boundary diff`
reads the two admitted schema or route-table sources for a service diff.
`prism-boundary impact --comparison <manifest> [--changed <elements.json>] --format json|md` selects input slots
and direction. The manifest references role documents, trees, mode/cutoff, config provenance and dependency
receipts. Keep the service-diff shorthand `--old <source> --new <source> --bindings <file>...` for identified
service pairs. Resolve the same slots and mark missing cutoff or configuration evidence unchecked in either form.
For service changes compare two service snapshots to one client. For client changes compare two clients to one
service. For joint changes check both actual pairs and both rollout orders. Schema files are required only for
schema claims. Validate a supplied change list against its admitted sources, expand definition IDs, and apply the
same coverage checks.

A later slice can sign value-level bounds at the owner's request, informed by the P1b census of unsigned value
keywords on 171 of 2,421 public golden fields and 57 of 600 OpenAPI 3.x fixture fields.

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
   separate evaluation companion. On the omitted-media companion from DESIGN 9, change the 200 field's type.
   With an admitted producer record on both sides, require the type change and both revision-specific media
   citations. Withhold one side's record and require a scoped **Not checked**. A historical run relying on
   a catalog created after cutoff must also print **Not checked**. Derive the same proof solely from admitted
   source on both sides and require the type change and citations again.
   Change or remove hindsight records after freezing and require byte-identical live Markdown and JSON.
4. **Three-report replay gate.** Before any product run for these subjects, produce and pin the public A1
   version-2 renders from prototype PR #886. Resolve `refs/pull/886/head` in `shoedog/prism` and pin the
   reachable renderer commit `028c7d1bf3200ced0a8fa53c7a4155c2eba45231` (renderer change `11f06c91`).
   Verify that exact commit before rendering and record the ref and resolved SHA. The public v2 report
   directory is `~/prism-evidence/boundary-impact/feasibility/reports-v2/public/`. Populate it using the
   pinned batch renderer on the ten P subjects or import its byte-verified renders. Pin full producer/renderer/input commits, input receipts, clock, config,
   scope, slots and SHA-256 of live Markdown and JSON in the manifest. The tracked memo reports and class index
   supply subject selection only. Select the first three by DESIGN 6.4 (counted classes in the stated order,
   one per commit, newest first within class). The index gives 01 `f93fb6b7`, 02 `7a036f21`, 03 `f723d120`.
   Freeze each reference occurrence's prototype ID, client tree, repository-relative path, blob, half-open
   byte span of the invocation, version, group, grade and evidence before seeing product output. If a render
   gives only a line or ordinal, resolve the invocation uniquely against the pinned source before freezing.
   Consolidate repeated presentations of the same occurrence and keep all its groups. An unresolved span or
   conflicting occurrence records for one key leave that comparison pending.
   Require at least one frozen reference occurrence and one evidenced shared location in each selected report. For each reference occurrence record
   reproduced with the same supported result, changed with independently checked evidence, or lost with measured
   refusal, cause and affected coverage. Report retained/changed/lost over the full population per subject.
   Keep the selected subjects fixed if one is empty or loses all shared coverage. Every product-only claim
   likewise needs its own evidence. Within each subject/comparison, join one-to-one on (client tree, path,
   blob, byte span). Keep both engine IDs in the receipt, null on the missing side. For a shared occurrence
   require equal target IDs, before/after grades and reasons on the same evidence. A changed result must
   identify the additional evidence or scope difference that accounts for it. Test a product-only call earlier
   in the same blob that shifts the shared call from ordinal 1 to 2. Pair the shared span once, preserve its
   two IDs and account for the earlier span as product-only, even if its ordinal equals the reference ID.
   Compare direction, versions, unknowns, actions,
   Markdown locations and appendix links against JSON. Empty shared coverage, an unexplained loss or a missing
   input/render keeps the gate pending.
5. **Admission and rendering.** Run A1's pre-freeze test from raw inputs with fixed query, clock and receipts.
   Vary/withhold unadmitted refs, config, saved schema, subject lists, restorations and later history. Compare
   the set of reports emitted and their bytes including unselected/restored subjects. Changing an admitted
   declaration must change its row. Independently pass the post-freeze byte-identity gate above. For the neutral
   GET→POST joint case assert both real matches and both rollout failures with separate site keys in all four
   comparison maps, then render directly from those frozen results.
6. Render the first report for each pair. R must show at least one evidenced producer endpoint and consumer
   occurrence joined on the same operation under an admitted scoped mount or root. Disclose configured versus
   proven basis. Publish aggregates only. Reconcile P4's saved-inventory and lost-location counts with D-census.

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
- `fields.result_defs(call, edges, callers) -> (definition_ids, coverage)`: identify the exact request expression
  by repo/tree, blob and byte span. Use its result definition or the uniquely evidenced assignment of that
  result. For a returned result, follow only evidenced return-to-caller-result edges for that invocation (one
  caller level). Link each definition to the occurrence, binding and typed path. A file/line or name alone
  cannot distinguish sibling expressions or later definitions. If the dump lacks spans or def-use identity,
  join to admitted AST/binding evidence that distinguishes them, otherwise record an unavailable seed and
  retain the call and its oracle reads in the measurement.
- `fields.reads_from_edges(edges, definition_ids, response_root, max_hops=6)`: follow value-preserving def-use
  edges forward from those definitions, respecting kills, reassignments and function boundaries. Submit unique
  (call occurrence, read occurrence at file/blob/span, typed path) records. Retain property, array and map
  steps, including the configured response root.
- `oracle/field_refs.mjs` returns TypeScript property-reference spans and typed paths for the selected type.
  Check their relation to the selected request. Pin oracle version and adjudicate coverage at the same
  occurrences. Type/spelling equality alone cannot establish attribution.

**Scoring.** Predeclare eligible response fields under the configured root, including list item fields below
`[]` or `list/[]`. Freeze the initial oracle set and the entire set of submitted reads for these calls.
Match one-to-one on the exact call occurrence, read occurrence and typed path. Adjudicate unmatched submissions
for incomplete oracle coverage using the source and value flow, recording each decision. Add verified oracle
omissions to the reference set before final scoring. All remaining unmatched submissions, including undecided
ones and same-named reads on unrelated values, count against precision. Recall is found / all reference reads.
Precision is found / **all submitted reads**. A zero denominator or an unverifiable reference population cannot
pass. Report raw and adjudicated totals and coverage per operation.

- [ ] In the clone's `ui/` directory, install the client dependencies with the package manager its lockfile
      names, scripts disabled (`--ignore-scripts`) and the lockfile frozen. This needs the network. If it
      fails, write "F3 oracle: reader-labelled" in the ledger; a Sonnet-5.5 reader then labels the exact call/read spans and typed paths under the same scoring contract.
- [ ] Test two definitions and two reads of `shade` on the same lines, only one from the chosen call. Include a
      second request, a killed edge, duplicate edge, list item, return-to-caller result and ambiguous seed.
      Assert occurrence/path sets. Deliberately submit the other object's same-named read and verify precision
      drops. Removing the exact seed must expose missing coverage and any missed reference reads.

- [ ] Build prism for the dump: `~/prism-evidence/tools/memguard.sh 8 cargo build --release` in the
      worktree; `target/release/prism --version` must show the worktree's `git rev-parse --short HEAD`.
- [ ] Dump edges once: `memguard.sh 8 target/release/prism nav --no-cache dfg-stats --repo <P clone>
      --edges > feasibility/edges.jsonl`.
- [ ] For the selected operations: oracle references from `field_refs.mjs` with the client's `tsconfig.json`
      (or the reader). Compare with `reads_from_edges`.
- [ ] `feasibility/F3.md`: per operation: fields, seeded and unseedable calls, oracle reads, all submissions, found, missed, unmatched and adjudications, recall and precision, and the
      three most common reasons for a miss (for example: value stored in component state, value passed
      through a hook). One reviewer on the eval PR.
- [ ] **Decision:** recall ≥50% and precision ≥90% with both denominators nonzero → continue with step 2.
      Otherwise P5 ends here: the pilot is operation-level with producer struct fields only; record the
      DESIGN section 7 row in the ledger.

**Step 2, product.** Add `prism::api::dfg_edges(session: &NavigationSession) -> impl Iterator<Item =
DfgEdgeRecord>` to the core crate (the records `nav dfg-stats --edges` prints), exported `#[doc(hidden)]`
outside the facade's compatibility promise, as `OwnerFailure` and `OwnerArgs` are in `src/api/mod.rs`,
with its own test in the core crate. Then `consumer/field_reads.rs`: step 1's procedure and scoring, in
Rust, matching reads to wire fields under the configured `response_root`. Rows come from F3's miss reasons;
each miss class is either a row that must pass or an accepted loss named in the README. **Gate:** on step 1's
operations, retain at least F3's found reads and precision ≥90% by the same submission rule.

### Slice P6: replay gate and documentation for both pairs (3 to 4 days)

Both producer adapters and both consumer configurations were scheduled before P4. P6 runs the product replay and
documents the final per-pair results. Keep the evaluation companion and measured history window distinct from the
A1 live review. R's configured mount must be admitted for each replayed version. Report the private replay only in
aggregates, without attributing lift where there were no controls.

**Public retention decision (2026-11-25).** The P02 check found `k = 0` of the 33 locations qualified. The gate
was returned to the owner under ORCHESTRATION §8. The owner set the public minimum at **153 of 161** checkable
locations (95%, rounded up) and kept all eight reader-counted recoveries mandatory. R has no field schema.
P3 handles both pairs' scoped API mappings.

Show the following accounting beside the public score. The 194 frozen predictions comprise 161 scored,
20 removed and 13 unchecked. The unchecked rows count toward 174 accounted locations, with 161 scored.

| Disposition | Locations | Treatment |
|---|---:|---|
| Scored | 161 | Retain at least 153. All eight reader-counted recoveries are mandatory. |
| Removed | 20 | Already stale at the original revision. Exclude from the score and the unchecked count. |
| Not checked | 13 | Historical JSON request media remains unverified. Include in accounting only. |

| Removed operation | Historical comparison | Locations | Reason |
|---|---|---:|---|
| PUT `/answer/admin/api/siteinfo/write` | `f05f1eb80d10` → `9efa9471bd84` | 15 | The router had already removed the route. Swagger later dropped the fields from `authorized_attachment_extensions` through `restrict_answer`. |
| GET `/answer/admin/api/siteinfo/interface` | `fed37f873d0a` → `2b1ad9479163` | 5 | Stale request-body annotation on a handler that reads no body. |

No request-side evidence was admitted for P02. DESIGN §5.1 requires media verification on both sides of a
historical comparison, with producer evidence at any revision where Swagger omits media. The 13 unchecked
locations are on PUT handlers that bind through `ShouldBind`, where Gin can select JSON, XML or form binding.
All operation suffixes below share the `/answer/admin/api` prefix.

| Operation | Unchecked request-field changes | Locations |
|---|---|---:|
| PUT `/siteinfo/interface` | Removed `logo`, `theme`, `default_avatar` (two rows) and `gravatar_base_url` | 5 |
| PUT `/setting/privileges` | Changed allowed values for `level` (two rows) | 2 |
| PUT `/setting/smtp` | Changed allowed values for `encryption` | 1 |
| PUT `/siteinfo/write` | Changed types of `recommend_tags` and `reserved_tags` | 2 |
| PUT `/siteinfo/general` | Removed `check_update` | 1 |
| PUT `/siteinfo/login` | Removed `login_required` and made `require_email_verification` required | 2 |

Frozen prediction IDs (change prefix `:pred:` number) of the 13 unchecked rows: `661d8a7f8fec:2`, `7085cfa7ed54:2`,
`8d421ae5d5ac:2`, `73cfbace7090:1`, `73cfbace7090:2` (interface); `61b10a854ff6:3`, `530812f81097:9` (privileges);
`530812f81097:2` (smtp); `530812f81097:5`, `530812f81097:6` (write); `3264fdd1d90f:2` (general); `3264fdd1d90f:6`,
`3b6f981b81ca:1` (login). The 20 removed are `f05f1eb80d10:20` to `:34` (write) and `fed37f873d0a:1` to `:5`
(interface). Outcome buckets (28/50/83 and the splits above) come from the frozen replay records of the P6 driver
(one record per change, `outcome.kind` per predicted location); P6 regenerates them and freezes the file before the
product run.
Carry the unchecked rows into P4/P6 by frozen change commit, operation, direction/status, field path and client
occurrence. Preserve the original field ID when known and identify the missing evidence.

**Prototype replay.** At `origin/main` commit `14c74b4cd39264f6cdff1b3eb07c88b8e234f463` (the owner-facing rerun receipt), the prototype
matched 172 of 174 accounted locations. With the 13 unchecked rows withheld by the media rule, 159 of 161
scored locations match. The floor allows eight misses in total. The 159/161 result already reflects the two
misses below, which the frozen reader labelled false predictions. They are outside the eight required recoveries.

| P6 acceptance row | Frozen locations | Required product disposition |
|---|---|---|
| Public report-route slash | `07d3d3525ba5c2248bafa7ea6812d811dff476b3:pred:1` (`flaged_content`) and `07d3d3525ba5c2248bafa7ea6812d811dff476b3:pred:2` (`flaged_type`), both field removals on PUT `/answer/admin/api/report/` | Check these two misses first. Recover each by proving equivalence under a stated rule to the router and client spelling, `/answer/admin/api/report`, or record why it remains a miss. Preserve the Swagger spelling in the source identity (P1/P3). |

- [ ] Add `--engine product` to the replay driver. Freeze the memo's original usable-change cohorts, pins,
      prediction rule and outcome observation window per pair. Require every reader-counted replay location
      again at its original (change commit, element, client revision and occurrence), reconcile lost and new
      predictions, and report the product denominator.
- [ ] Independently require **153/161 on P** and **60/63 on R** (95% of each scored set, rounded up, DESIGN §8).
      Freeze every reference location before the product run. P scores the 161 checkable locations and R scores
      its full frozen set. Join on the original change, element and client occurrence using P4's source key.
      Count each scored reference location still predicted for that change and element once. Report retained,
      lost and product-only counts. Only retained scored locations enter the numerator. A surviving call to the
      same operation earns field-location credit only when the product also predicts the original field change
      at that occurrence.
- [ ] Each of the three bucket shares must be within **5 percentage points** of its own pair's reference share,
      assessed independently, subject to the open public reference question below. Freeze the scored group's
      outcome shares before any product run. Use all predicted locations on each pair's frozen cohort as the
      share denominator, excluding the 20 removed and 13 unchecked on P. Include unresolved follow-ups as
      unknown and show counts and shares. A zero denominator or missing cohort fails.

| Pair and frozen population | Outcome reference counts and denominator | Comparison basis |
|---|---|---|
| P, F4, 37 usable first-parent commits at the public pin. 161 scored (frozen reference) | `same_commit` 28 (17.39%), `later` 50 (31.06%), `present_at_pin` 83 (51.55%) | Same-repository parent calls, first-parent touches through the pin, 616 controls separate. Recover all 8 counted F4 locations. |
| P, 194 original | `same_commit` 33 (17.01%), `later` 70 (36.08%), `present_at_pin` 91 (46.91%) | Original frozen shares for comparison. |
| P, 174 accounted after the 20 removals | `same_commit` 33, `later` 55, `present_at_pin` 86 | Includes all 13 unchecked locations. |
| R, F5, 42 usable route-change commits in the recorded mount window, predictions on 22 | `within_14_days` 62/63, `later` 1/63, `present_at_head` 0/63 | Date-aligned consumer revision, touches through its frozen head. Recover all 5 counted replay locations. No controls. |

The frozen replay accounts for 15 `later` and five `present_at_pin` among the removals, and five `same_commit`,
five `later` and three `present_at_pin` among the unchecked. The share of later client edits moves from 36.08%
to 31.06%, a drop of 5.02 points between the displayed percentages (5.03 from exact ratios), which exceeds the five-point tolerance against the original 194
shares. `same_commit` rises 0.38 points and `present_at_pin` rises 4.64 points between displayed percentages (4.65 from exact ratios).

**Open owner question with default (DESIGN §8).** Which reference and tolerance should P6 use for P?

Every share comparison uses exact count ratios (count over denominator), never the rounded displayed percentages;
the 5.02-point figures above are differences of displayed values. Under every option the product's share
denominator is all locations the product predicts on the frozen cohort (product-only ones included, unresolved
follow-ups as unknown), excluding the 20 removed and 13 unchecked rows; it can therefore differ from 161, and the
counts are compared as ratios over that denominator.

- **A (default).** Re-base the five-point band on the frozen scored-group shares (17.39%, 31.06%, 51.55%) and
  print the original 194-based shares and the deltas beside them in the report.
- **B.** Keep the original 194-based shares. A product that reproduces the scored group exactly fails the
  `later` check by construction, with a difference of 5.02 points between displayed percentages (5.03 from exact ratios).
- **C.** Keep the original reference and widen the tolerance to six points for `later` only.

Changing the choice later requires re-scoring only. No implementation depends on it. The latest cheap point
is before P6 sees product results.

Check the three gates independently. Under option A, P product buckets of 25 same-commit, 51 later, 82 present
and three unknown pass the share test over 161. On R, 58 within 14 days and five later fail the share test even if all
63 locations are retained. Retaining only the five reader-counted R locations, all within 14 days, passes the
share test and the reader-counted test but fails retention at 5/63. Adding unrelated product locations cannot
repair that shortfall.

The companion scores the historical selection and buckets. A1 renders the admitted change independently. Record
a gate miss with its cause even when the live review contains a new supported result.

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

### Design questions from the P1c product run (OWNER decided 2026-11-27: alignment B, standing limits C)

Decisions: (1) standing limits use option C (the blocking limit beside each change, a count by side and reason with root detail in the appendix); (2) revision alignment uses option B (the same commit or the next first-parent commit; 17 of 18 operation removals match under B against 10 of 18 under A, operation level only). Both are recorded in DESIGN 5.2. The reformatted report example is still the owner's to supply, and P4 layout waits for it. The text below is the question as asked.

Run: the P1c diff on the 37 usable producer revision pairs of the public pair (`docs/swagger.json` at the commit and
its first-parent parent; all 74 documents admitted; 105 operations added, 18 removed; 618 change entries).

1. For P2/P4 (reports): the diff carries 5,564 standing coverage limits on those pairs (`body_presence_unchecked` 4,423,
   `media_unverified` 918, `ref_siblings_unchecked` 109, `non_json_media` 114), against 618 change entries. A report
   must count and show standing limits separately from limits on changed elements, or they drown the changes.
   Open: the counting rule and where standing limits are shown (appendix count, per-root note, or both).
   Recorded default (**OWNER** to ratify; 5,564 standing limits vs 618 changes on the public pair): within a comparison basis, count once per P1c key `(side, scope, pointer, claim, reason)`. A limit linked to several changes counts once in the change-linked subtotal. Show standing coverage separately by side and reason, with complete root detail in the appendix and the blocking limit beside each affected change. P1d preserves keyless and unlinked records, side-local multiplicity and many-to-many claim links.
2. For P3/P6 (retention gate, 153 of 161): 7 of the prototype's 18 operation removals appear in the product's diff one
   revision later than in the prototype's replay, because the generated spec is regenerated one commit after the
   annotation commit (an eighth is the known trailing-slash miss). Open: how a prediction is aligned to a revision
   (same commit, or a stated window of N first-parent commits) must be stated BEFORE the product is scored against
   153 of 161.

## Acceptance for the lane

The memo records feasibility separately from owner ratings. D-census is due before P1. Product acceptance
requires the P2, P2-R, both P3 and both-pair P4 gates, P5's measured decision and P6's replay gate on the final
product head. Use P6's scored public set, report the removed and unchecked rows beside the score, and record
the disposition of both public report-route misses. Record each review outcome and every unbound reason and
count (P publicly, R under private custody). State the route source used, proven/configured counts, accepted
losses and remaining coverage. Run the full suite before merging product changes to main.
