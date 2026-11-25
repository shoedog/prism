# Lane 2: faultpath target discovery — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** `prism targets --inventory python-deps` lists Python dependency call sites (HTTP, then filesystem,
parsing, configuration) as rows faultpath can inject at, and faultpath plans and runs faults at the httpie
HTTP sites from those rows with no overlay.

**Architecture:** a second row source inside `crates/prism-review/src/targets/` (`inventory/`), AST-only,
merged with projected findings by a new function `merge_inventory` (`project()` is unchanged). It reuses and
extends the library-identity proof in `dependency_hint.rs` and `dependency_identity.rs`. Inventory documents
are written as targets schema 1.1.

**Tech stack:** Rust (prism-review crate, tree-sitter Python), `assert_cmd` CLI tests; Python 3.11.11 (a
`uv` environment, never the host's default `python3`) for every script of this lane: the census, the oracle
scripts, `parity/compare.py`, `e2e/funnel.py`; faultpath (read-only, from an exported copy, in the same
3.11.11 environment) as oracle.

**Spec:** `DESIGN.md` in this directory. Gate names (G-overlay, G-bounds, G-accept, G-parity, G-precision,
G-live, G-byte), routes R1 to R5, the closed-world rule, the line anchor, the frame, symbol and bounds rules,
the unsupported reasons, the identity-unproven causes, the callee tables and the row shape are defined
in DESIGN 5–6. `row-verdicts.jsonl` in this directory supplies the Stage D want and rule for every matrix ID.

## Global constraints

- Branch per slice from `main`; worktree `~/code/prism-targets`; evidence `~/prism-evidence/faultpath-targets/`.
- No change to any file in the faultpath or tooling repositories. Faultpath is used from an exported copy
  of its `origin/main` under the evidence directory. Every location of faultpath, its pilot profile, the
  tooling repository and the httpie pilot records is in `~/prism-evidence/lanes-2026-11-20/PRIVATE-BINDINGS.md`;
  nothing from that file is copied into this repository, an issue, a PR body or a commit message.
- **Faultpath tasks run on Claude seats only.** Every task that reads faultpath's source or runs it (M-1's
  export and install, the oracle scripts of T-1, G-accept, G-parity, G-live, the funnel) is done by the
  orchestrator or a Claude agent (implementer, measurement runner). Codex seats (reviewer B, terra and sol
  enumerators and the Stage D design seat) use the supplied oracle answers and DESIGN. Faultpath source
  and the private bindings file are outside their packet. The Stage D external packet is the census
  report and site lines, `enum/open-questions.md` and `oracle/site-oracle.jsonl`. The round-1 fold
  also uses the supplied reviews, focused enumeration and the original ledger copied into this directory.
- Without `--inventory`, `prism targets` output stays as today (G-byte).
- Row values are exactly those of DESIGN 5.1 and 5.2.
- `kind` in a hint only when library identity is proven (DESIGN 5.3: root identity, or a receiver route;
  R1/R2 use construction, R4/R5 require the annotation and the §6.4 value witness at grade `nameonly`).
  R3 ties go to diagnostics. Use the repository-wide static grades.
- The public facade keeps its signatures: `prism_review::api::project`, `TargetsDocument`, `TargetsMeta`
  are not changed. The inventory adds new functions beside them.
- The inventory is AST-only: no call graph, no code property graph, no review runner. It does not change
  `ParsedFile::function_node_spanning` or `node_line_range` (core, shared by every row source).
- PR #841 also edits `cli.rs` and `main.rs`; rebase over it if it merges first.
- httpie is pinned at `2105caa49bae87c5809c274e407619a0de2639d1`.

## Review focus

1. Test errors in the owning definition, including nested syntax and the signature. Proven calls with
   clean owning subtrees keep the file parse quality. sl2-14 diagnoses the outer call for a nested error.
   o23a, sl1-24 and as4-09 retain candidate tier at exact confidence. (S2a)
2. A decorated function: `function_start_line` is the `def` line, not the decorator line; a trailing comment
   line indented under the body does not move `function_end_line`. (S2a)
3. `await client.get(u)` inside `async def` with an `httpx.AsyncClient` or `aiohttp.ClientSession`
   receiver: listed like the synchronous form. (S2b)
4. A file path that the finding projection normalises (leading `./`, backslashes): inventory rows use the
   same normalised spelling, so ids agree across platforms. (S2a)
5. The same resolved callee on two lines of one function: two rows with different ids; a `--files` filter
   that excludes the file: no rows and the file absent from `diff.files`. (S2a)
6. Port DESIGN 5.4 and 6.6's candidate set and three-component byte-column key, then the Python AST
   ordinal for a full tie. Check the bound source occurrence (as4-07, tr6-02, tr7-02). (S2a)

---

## Stage M: census (1 day)

### Task M-1: workspace, corpora, interpreter

- [ ] Create the worktree and evidence directory (ORCHESTRATION section 4).
- [ ] Create the lane's interpreter: `uv venv --python 3.11.11 ~/prism-evidence/faultpath-targets/venv`.
      Every Python command of this lane runs with `~/prism-evidence/faultpath-targets/venv/bin/python`. If
      `uv` cannot provide 3.11.11, stop and record it in the ledger; no other interpreter is a substitute.
- [ ] `git clone https://github.com/httpie/cli ~/prism-evidence/faultpath-targets/corpus/httpie` and check
      out the pinned commit detached. If the network is unavailable, use the copy named in the private
      bindings file. Record the commit.
- [ ] Choose two more Python corpora from `~/code/bench-repos` that import `requests` or `httpx` in
      non-test code (count with `rg -l "^(import|from) (requests|httpx)" --glob '!**/test*'`); record the
      choice and the counts. `black` is the fourth corpus. These four are the **census corpora**. For each,
      record its import root (`.` or the directory, such as `src`, from which its top-level package is
      importable); G-accept and G-parity use that value.
- [ ] Export faultpath (Claude seat): fetch the faultpath checkout named in the private bindings file, export
      its `origin/main` with `git archive` into `~/prism-evidence/faultpath-targets/faultpath-main`, and
      install the exported tree into the lane's interpreter (`uv pip install --python <venv python> <tree>`).
      Record the commit. Check with a one-line call that faultpath's runtime check answers no refusal under
      this interpreter; a refusal is a failed install, not a result. If installation needs packages the
      machine cannot fetch, record that; the evidence and gate consequences are in DESIGN 6.8.

### Task M-2: independent census

**Files (evidence directory only):** `census/py_dep_census.py`, `census/README.md`.

- [ ] Write a Python `ast` script, independent of prism, that for each corpus lists every call whose callee
      chain or receiver can be tied to an HTTP library or to a callee of DESIGN 5.3's tables. For each: file,
      line, callee text, how the tie is established (direct import; alias; from-import; builtin; constructor
      local; `with` target; `self` attribute; parameter annotation; local annotation; helper with annotated
      return; helper without annotation; no tie but table method name on a receiver bound from a library
      call), and the site shape (in a function; module level; nested function; lambda; class body inside a
      function; comprehension; decorator argument; default argument; multi-line callee; not the line anchor
      of DESIGN 5.4; `await`).
- [ ] The same script lists every function definition of every corpus as (file, `lineno`, `end_lineno`,
      name) from `ast`, one JSON line each, in `census/functions-<corpus>.jsonl` (input of G-bounds).
- [ ] Run it on the corpora. Write `census/CENSUS.md`: counts per family, tie and shape for each corpus, and
      for httpie the classification of the four overlay sites.

**Exit gate:** the two HTTP overlay sites require a proven route and supported shape. M found root
identity at `update_warnings.py:44` and an annotated-helper tie at `client.py:114`. Its census record
cannot prove the return body. DESIGN 6.1 keeps R1 and defers positive R3. S2b tests the actual
helper against §6.4. Both overlay sites must still pass.

---

## Stage T and E: test matrix (2.5 days)

### Task T-1: fixtures, the rows file and the data-driven test

**Files:**
- Matrix data at repository root: `tests/targets_inventory/py/` (including multi-file fixtures) and
  `tests/targets_inventory/rows.jsonl`. Keep this tree outside the unrelated Python fixture walker.
- Create: `crates/prism-review/tests/cli/targets_inventory_matrix_test.rs` and its `mod` line in
  `crates/prism-review/tests/cli/main.rs` (after `mod targets_test;`)

**The rows file.** One JSON object per line with the fields:

| Field | Value |
|---|---|
| `id` | the row name below (`h01`, `w03`, …) |
| `group` | `http.root` (HTTP rows whose identity is root identity, and rows independent of receivers), `http.receiver` (rows that exercise routes R1 to R5), or `stdlib` (filesystem, parse and config rows) |
| `fixture` | fixture path relative to `tests/targets_inventory/py/` |
| `line` | the line the expectation is about |
| `callee` | the resolved dotted callee the expectation is about, or `*` for a `none` row that forbids any inventory row on the line |
| `class` | the census or enumeration class the row covers |
| `want` | `site(<family>,<resolved callee>,<grade>[,symbol=<name>][,bounds=<start>-<end>])`, `unsupported(<reason>,<count>)`, `none`, or `none(unproven:<cause>)` |
| `truth` | `python-semantics`, `faultpath-guard` (the want follows from a consumer rule DESIGN ports: line anchor, accepted symbol, canonical callee and kind) or `design` |
| `consumer` | for a site, the recorded guard result (including `unparseable_file` for a syntax error) or `pending` until measured. Empty otherwise. The oracle preserves each line's result for every row type. A guard observation supplies no planner result. |

**The test.** A command-line test, like the existing targets tests. A constant
`ENABLED_GROUPS: &[&str]` lists the groups that run; it is **empty** in this task. The test
`rows_file_is_well_formed` always runs: every line parses, every field is present, ids are unique, every
`want` matches the grammar above, every `group` is one of the three names, and every reason and cause named
in a `want` is in DESIGN 5.2's closed lists. The test `matrix_rows_match` returns early when
`ENABLED_GROUPS` is empty; otherwise it runs
`prism targets --repo tests/targets_inventory/py --inventory python-deps` once and, for each row
of an enabled group, compares the expected and actual multisets on the asserted fixture lines.
Record independent expected bare symbols (or absent) for unsupported IDs in the test expectations,
including `outer` for sl2-12 and `inner` for sl2-13. Match the following fields:

- `site(…)` requires one `external_call` with matching file, line, family, resolved callee and grade,
  plus the expected symbol and bounds when specified.
- `unsupported(reason,count)` requires one `other` with matching file, line, symbol (including absence),
  resolved callee, `dependency_call_unsupported`, and exact reason/count in `expected.detail`.
  Consume one actual row per positive or unsupported expectation. Compare multiplicities for the
  asserted keys and reject extras or missing occurrences. Deleting either of the two same-line groups
  in the matcher test must fail.
- `none` forbids inventory rows with that callee on the line (`*` forbids every inventory row there).
  Check absence against the full actual result, including consumed rows. For **each plain-none
  fixture** assert zero identity-unproven warnings unless a row for that
  fixture explicitly expects a counted omission. In a mixed fixture check the declared cause totals
  so that a plain-none row cannot silently add a warning.
- `none(unproven:<cause>)` makes the same absence assertion and requires a fixture warning with the
  named cause. Keep the exact total and ordered histogram test for sl1-67/68/69 in S2a.

A line may carry several rows of the rows file (one per expected document row). With `ENABLED_GROUPS` empty
the test passes on `main`; the branch compiles and CI is green.

Stage T seed examples are below. The complete matrix and Stage D verdicts govern the slice fixtures.

| Row | Group | Fixture line | Want |
|---|---|---|---|
| h01 | http.root | `import requests` … `requests.get(u)` in a function | site(http,requests.get,exact) |
| h02 | http.root | `import requests as r` … `r.post(u)` | site(http,requests.post,exact), source callee `r.post` |
| h03 | http.root | `from requests import get` … `get(u)` | site(http,requests.get,exact) |
| h04 | http.root | `from requests import get as fetch` … `fetch(u)` | site(http,requests.get,exact) |
| h05 | http.receiver | `s = requests.Session()` … `s.send(req)` | site(http,requests.Session.send,exact) |
| h06 | http.receiver | `with requests.Session() as s:` … `s.get(u)` | site(http,requests.Session.get,exact) |
| h07 | http.receiver | `self.s = requests.Session()` in `__init__`, `self.s.get(u)` in a method | none(unproven:receiver_value_unproven) |
| h08 | http.receiver | `def f(s: requests.Session): s.get(u)` | none(unproven:receiver_value_unproven) |
| h09 | http.receiver | `s = make()` then `s.send(req)`, where `make` is an admissible same-file annotated helper returning `requests.Session()` | site(http,requests.Session.send,nameonly) |
| h10 | http.receiver | `requests.Session().get(u)` | site(http,requests.Session.get,exact) |
| h11 | http.receiver | `async def f(): async with httpx.AsyncClient() as c: await c.get(u)` | site(http,httpx.AsyncClient.get,exact) |
| n01 | http.root | `s = requests.Session()` (the constructor call itself) | none |
| n02 | http.root | `requests.utils.quote(x)` | none |
| n03 | http.receiver | `d.get(k)` with `d` a parameter | none |
| n04 | http.root | `requests = Fake()` in the function, then `requests.get(u)` | none(unproven:shadowed_root) |
| n05 | http.root | a repository-local `requests.py` beside the file | none(unproven:repository_local_module) |
| n06 | http.receiver | `s = requests.Session()`, `s.get(u)`, then `s = other()` in one straight-line activation | site(http,requests.Session.get,exact) |
| n07 | http.receiver | `s = make()` where `make` has no return annotation | none(unproven:helper_unannotated) |
| n08 | http.root | `from requests import *` … `get(u)` | none |
| u01 | http.root | `R = requests.get(u)` at module level | unsupported(module_level,1) |
| u02 | http.root | `r = parse(requests.get(u))` | unsupported(not_line_anchor,1) |
| u03 | http.root | `requests.get(build(u))` | site(http,requests.get,exact) |
| u04 | http.root | `a, b = requests.get(u), requests.get(v)` | site(http,requests.get,exact) and unsupported(not_line_anchor,1) |
| u05 | http.root | callee split over two lines (`requests\n    .get(u)`) | unsupported(multiline_callee,1) |
| u06 | http.root | `f = lambda u: requests.get(u)` inside a function | unsupported(lambda_frame,1) |
| u07 | http.root | arguments spread over several lines, callee on the first | site, `line` = the callee's line |
| w01 | http.root | `x = foo(a) + requests.get(u)` | unsupported(not_line_anchor,1) |
| w02 | http.root | `if check(x) and requests.get(u).ok:` | unsupported(not_line_anchor,1) |
| w03 | http.root | `return wrap(a), requests.get(u)` | unsupported(not_line_anchor,1) |
| w04 | http.root | `log.info('x'); r = requests.get(u)` | unsupported(not_line_anchor,1) |
| w05 | http.root | `x = d[key(k)] or requests.get(u)` | unsupported(not_line_anchor,1) |
| w06a | http.root | `with open(p) as f, requests.get(u) as r:` | unsupported(not_line_anchor,1) for `requests.get` |
| w06b | stdlib | the same line | site(filesystem,builtins.open,exact) |
| w07 | http.root | `r = requests.get(u).json()` | unsupported(not_line_anchor,1) |
| w08 | http.root | `x = requests.get(u) + foo(a)` | site(http,requests.get,exact) |
| s01 | http.root | call in a method `C.method` | site(…,symbol=method,bounds=<the method's>) |
| s02 | http.root | call in a function nested in a function | site(…,symbol=inner,bounds=<inner's>) |
| s03 | http.root | call in a list comprehension inside a function | site of the enclosing function |
| s04 | http.root | decorated function | site(…,bounds=<`def` line>-<end>) |
| s05 | http.root | method `m` of a class nested in a class | site(…,symbol=m) |
| s06 | http.root | function `helper` nested in a method | site(…,symbol=helper) |
| s07 | http.root | method `m` of a class defined inside a function | site(…,symbol=m) |
| b01 | http.root | function whose body is followed by a comment line indented under it | site(…,bounds=<def>-<last statement line>) |
| b02 | http.root | blank line, then an indented comment after the body | site(…,bounds=<def>-<last statement line>) |
| b03 | http.root | nested function followed by an indented comment | site(…,bounds=<def>-<last statement line>) |
| b04 | http.root | function with a decorator spread over several lines | site(…,bounds=<def>-<last statement line>) |
| c01 | http.root | `def outer(): def inner(x=requests.get(u)): …` | site(…,symbol=outer,bounds=<outer's>) |
| c02 | http.root | `@deco(` / `    requests.get(u))` (the call on its own line) decorating a function nested in `outer` | site(…,symbol=outer,bounds=<outer's>) |
| c03 | http.root | the same decorator layout on a top-level function | unsupported(module_level,1) |
| c04 | http.root | `requests.get(u)` in the body of a class defined in a function | unsupported(class_body_frame,1) |
| c05 | http.root | `xs = [r for r in requests.get(u)]` inside a function (the comprehension's first iterable) | site of the enclosing function |
| c06 | http.root | `xs = [requests.get(x) for x in urls]` inside a function (the element expression) | site of the enclosing function |
| f01 | stdlib | `open(p)` / `json.loads(t)` / `os.getenv("X")` / `configparser.ConfigParser().read(p)` / `Path(p).read_text()`, one per line | site(filesystem,builtins.open,exact) / site(parse,json.loads,exact) / site(config,os.getenv,exact) / site(parse,configparser.ConfigParser.read,exact) / site(filesystem,pathlib.Path.read_text,exact) |
| f02 | stdlib | `os.environ["X"]` (a subscript, not a call) | none (`*`) |
| f03 | stdlib | `from pathlib import Path` … `Path.home()` | site(config,pathlib.Path.home,exact) |
| f04 | stdlib | `json.load(open(p))` | site(parse,json.load,exact) and, for `builtins.open`, unsupported(not_line_anchor,1) |

Truth labels: w01 to w08, u02, u04, s01, s05 to s07, f01, f03 and f04 are `faultpath-guard`; the bounds
rows b01 to b04, the frame rows c01 to c06, s02 to s04 and the `none` rows are `python-semantics`; the rest
`design`.

- [ ] Write the fixtures, `rows.jsonl` and the two tests. Confirm `rows_file_is_well_formed` passes and
      `matrix_rows_match` passes (empty `ENABLED_GROUPS`).
- [ ] Record the base command-missing result with `"http.root"` enabled locally, then restore the empty
      constant. Show behavioural RED on the implementing slice as ORCHESTRATION section 3 requires.
- [ ] Add `today_no_external_call_rows_for_dependency_calls` to
      `crates/prism-review/tests/cli/targets_test.rs`: it runs the current command on fixture h01 with a
      JSON diff spec naming every line (`{"files":[{"file_path":…,"modify_type":"Modified","diff_lines":[…]}]}`,
      the format of `tests/fixtures/targets/diff.json`) and asserts zero `external_call` rows. It passes on
      main and stays after S2a as the witness that a run without `--inventory` is unchanged.
- [ ] Fill the `consumer` column (needs faultpath; Claude seat) with a script in the evidence directory,
      `oracle/site_oracle.py`, run under the lane's interpreter. For each fixture and candidate row it
      records the actual bound call and callee spans in measurement evidence (§6.8). The supplied
      Stage D JSONL has text/bounds only. Its per-line records give the source text of the callee of the call
      faultpath binds at `site.line`, that callee qualified the consumer's way, and the site guard's answer
      with strict line checking (`accepted` or the reason). The private bindings file names the faultpath
      functions to call. The script exits non-zero, records nothing, and is fixed before any belief update
      when an answer is `unsupported_interpreter` or `unqualified_runtime`. For `faultpath-guard` rows, a
      bound callee that differs from the row's intended call is a disagreement with DESIGN 5.4: record it in
      the ledger and send it to stage D, never edit the want.

### Task T-2: enumeration

- [ ] Add these rows first (group `http.receiver` unless stated; truth `python-semantics`, e07 `design`).
      They follow the binding positions of DESIGN 6.2 and the R3 ownership boundary of 6.3.

| Row | Input | Want |
|---|---|---|
| e01 | `def fetch(u, session=None):` / `if session is None: session = requests.Session()` / `session.get(u)` | none(unproven:receiver_not_unique) (the parameter is a binding) |
| e02 | (stdlib) `from os import *` then `open(p, O_RDONLY)` in a function | none(unproven:star_import) |
| e03 | (http.root) `import requests` and `from compat import *`, then `requests.get(u)` | none(unproven:star_import) |
| e04a | (http.root) module `import requests`; in a function `with pool() as requests:` then `requests.get(u)` | none(unproven:shadowed_root) |
| e04b | (http.root) as e04a with `except E as requests:` | none(unproven:shadowed_root) |
| e05a–e | `s = requests.Session()`, then one of `with other() as s:`, `for s in pool:`, `(s := other())`, `s, t = a, b`, `except E as s:`; then `s.get(u)` | none(unproven:receiver_not_unique), each |
| e06 | `self.s = requests.Session()` in `__init__` and `self.s, self.t = other, None` in another method; `self.s.get(u)` | none(unproven:receiver_not_unique) |
| e07 | as h07, plus `obj.s = other()` in another module, `setattr(self, name, x)` with a computed name, and an override in a subclass in another file | none(unproven:receiver_value_unproven) (R3 deferred, DESIGN 6.3) |
| e08a | (http.root) `requests` imported at module level and assigned in another function under `global requests` | none(unproven:shadowed_root) |
| e08b | `s = requests.Session()` in `outer`, an inner function assigns it under `nonlocal s`, `s.get(u)` in `outer` | none(unproven:receiver_not_unique) |
| e08c | (http.root) `del requests` in the module, or a `match` arm `case requests:` in the calling function | none(unproven:shadowed_root) |
| e09a | `@contextmanager def make() -> requests.Session:` then `s = make()`, `s.get(u)` | none(unproven:helper_not_admissible) |
| e09b | `async def make() -> httpx.AsyncClient:` then `c = make()` (no `await`), `c.get(u)` | none(unproven:helper_not_admissible) |
| e09c | as e09b with `c = await make()` inside `async def`, then `await c.get(u)` | site(http,httpx.AsyncClient.get,nameonly) |
| e09d | as h09, with a parameter or local named `make` in the calling function | none(unproven:helper_not_admissible) |
| e10a | `def f(s: requests.Session): s = wrap(s); s.get(u)` | none(unproven:receiver_not_unique) |
| e10b | `s: requests.Session = other()` then `s.get(u)` | none(unproven:receiver_value_unproven) |
| e10c | `def f(s: Optional[requests.Session]): s.get(u)` | none(unproven:receiver_value_unproven) |
| e10d | `def f(s: "requests.Session"): s.get(u)` | none(unproven:receiver_value_unproven) |

- [ ] Run the enumerator tiers (ORCHESTRATION section 6) on `rows.jsonl`. Axes to hand them: import form;
      callee form; receiver route; binding count and binding kind of the receiver (the closed-world binding
      positions of DESIGN 6.2); enclosing frame (DESIGN 5.4); line layout and the line anchor; function end
      line (trailing comments, strings, continuation lines); shadowing source; family and canonical callee;
      Python syntax that changes scoping (`global`, `nonlocal`, walrus, `match` capture, class scope,
      conditional import, import inside the function, `try`/`except ImportError` fallbacks,
      `TYPE_CHECKING` imports). The census's shapes are rows too. Codex enumerators get the recorded
      `oracle/site-oracle.jsonl`, not faultpath's source.
- [ ] The implementer adds accepted rows after each tier, and the Claude seat refreshes the `consumer`
      column for new site rows. Stop when a full tier round is dry.

## Stage D: design of record (half a day)

- [x] DESIGN 6 pins the matrix, supplied oracle and sibling verdict ledger. Round 2 split after
      both reviewers rejected round 1. §6.8 records the pooled options, admission default and
      owner decisions, including the fixed multiline-policy conflict.
- [ ] One escalation review by each reviewer. Both must approve this amendment. On either
      rejection stop at this slice for the owner. No further fold or transcription.
- [ ] After both approvals verify all three digests in DESIGN 6 on the pre-transcription inputs.
      The implementer applies the 95 `change` verdicts by ID and preserves the 635 `keep` wants.
      The orchestrator materialises the 30 `add` records at `tests/targets_inventory/py/<fixture_path>` with the exact
      `fixture_source` bytes; set `fixture` to that relative path and copy its selector, line,
      class, group, truth and want. Preserve package neighbours for esc17 and esc18. Verify a
      unique ID-to-row join and the resulting 760 rows. Refresh the qualified oracle on the
      additions before claiming a consumer result, leaving the site observation pending meanwhile.
      On promoted sites (n06, tr4-03, sl1-03, wf27/38/96–98) use the site's canonical selector
      and the joined guard observation. Clear site-only consumer fields on omissions. Preserve
      targeted negative selectors on mixed lines. For as1-19 and as2-27 assert one surviving
      group member and a separate warning count, without forbidding the surviving member.
      Use all eleven ordered §5.2 causes including `receiver_value_unproven`. Run
      `rows_file_is_well_formed`.
      The empty → HTTP root → HTTP receiver → stdlib `ENABLED_GROUPS` sequence stays in force.

---

## Stage I: slices

### Slice S1: root identity for any call node (behaviour-preserving; half a day)

**Files:**
- Modify: `crates/prism-review/src/targets/dependency_hint.rs`, `dependency_identity.rs`, and the `dependency_hint` module visibility in `targets/mod.rs`
- Test: `crates/prism-review/src/targets/dependency_hint_tests.rs`

**Interfaces (produced):**

```rust
// SiteContext keeps its existing lifetime and fields.
pub(crate) struct SiteContext<'a> { /* existing fields */ }
/// What the root of a call's callee chain is bound to at the call site.
pub(crate) struct RootIdentity {
    pub source_callee: String,             // callee text as written, e.g. "r.get"
    pub resolved_path: Option<String>,     // dotted path through a proven import, e.g. "requests.get"; None when
                                           // the root is shadowed, unimported, or names a repository-local module
    pub root_is_library_spelling: bool,    // spelling evidence only. The inventory also checks actual scoped import
                                           // aliases and constructor/helper ties under DESIGN 6.5.
}
pub(crate) fn root_identity(parsed: &ParsedFile, call: tree_sitter::Node<'_>, ctx: &SiteContext<'_>) -> Option<RootIdentity>;
pub(super) fn sole_receiver_constructor(/* today's private helper, signature unchanged */);
```

| S1 visibility decision | Consequence |
|---|---|
| Chosen | Expose `SiteContext`, `RootIdentity` and `root_identity` as `pub(crate)` on a reachable crate path. Declare `pub(crate) mod dependency_hint` in `targets/mod.rs`. Compile a sibling-module import of all three via `crate::targets::dependency_hint`. |
| Declined | `pub(super)` or an inaccessible private module prevents the other lane from importing the three items. A public external API exceeds the required visibility. |

`resolve(parsed, line, ctx)` keeps its signature and its results; internally it may use `root_identity`.
Share the decomposition of Python imports and the repository-local lookup with the later inventory
census. Pass a root list to `repo_has_local_module`. The legacy finding-line binder supplies its
original site ancestors only (including packages). Inventory supplies the §6.2 root list and
applies its own builtin precedence before calling the filesystem helper. Check echo against its base. S2a can supply a deciding binding
from DESIGN 6.2 without inheriting the legacy binder's veto. Receiver routes start in S2b.

- [ ] Add tests `root_identity_reports_resolved_path_for_alias_import` (`import requests as r; r.get(u)` →
      `source_callee == "r.get"`, `resolved_path == Some("requests.get")`),
      `root_identity_reports_no_path_for_shadowed_root` and `…_for_repository_local_module`. They fail to
      compile: RED.
- [ ] Implement. The new tests and the whole `dependency_hint` test modules pass.
- [ ] Gate: `cargo test -p prism-review --test cli targets_test::` passes unchanged. One reviewer (refactor
      with tests).

### Slice S2a: inventory core, root-identity HTTP sites, command line, schema 1.1 (2 to 2.5 days)

**Files:**
- Create: `crates/prism-review/src/targets/inventory/{mod.rs,python.rs,py_tables.rs,py_receiver.rs,rows.rs}`
  (`py_receiver.rs` holds the targets-owned root/builtin census and invalidation checks of DESIGN 6.2
  in this slice. Use the existing targets import primitives. The taint module and crate move are independent.)
- Create: `crates/prism-review/examples/inventory_bounds.rs`
- Create: `docs/contracts/targets-1.1.schema.json` (copied byte for byte from the contract authority; the
  private bindings file gives its location and SHA-256)
- Create: `crates/prism-review/tests/cli/targets_inventory_test.rs`,
  `crates/prism-review/tests/cli/schema_support.rs` (the schema walker moved out of `targets_test.rs`, taking
  the schema file name as a parameter; `targets_test.rs` then uses it)
- Modify: `crates/prism-review/src/targets/mod.rs` (two new functions; `project` untouched),
  `crates/prism-review/src/api/` (re-export the new public items), `crates/prism-review/src/cli.rs`
  (`TargetsArgs`), `crates/prism-review/src/main.rs` (`run_targets`),
  `crates/prism-review/tests/cli/main.rs` (module lines),
  `crates/prism-review/tests/cli/targets_inventory_matrix_test.rs` (`ENABLED_GROUPS = ["http.root"]`),
  `CLAUDE.md` (the `targets/` paragraph), `README.md` (targets section)

**Interfaces (produced):**

```rust
// inventory/mod.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family { Http, Filesystem, Parse, Config }
impl Family { pub fn as_str(self) -> &'static str; pub fn parse(s: &str) -> Option<Family>;
              pub fn supported() -> &'static [Family]; }             // [Http] in S2a and S2b
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade { Exact, AnnotationTyped }
#[derive(Clone, Debug, Default)]
pub struct InventoryOptions { pub families: BTreeSet<Family> }       // empty = every supported family
pub struct InventorySite { pub file: String, pub line: usize, pub symbol: String,   // bare function name
    pub function_start_line: usize, pub function_end_line: usize, pub family: Family,
    pub resolved_callee: String, pub source_callee: String, pub grade: Grade }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnsupportedReason { ParseErrorInFunction, ModuleLevel, ClassBodyFrame, LambdaFrame,
    MultilineCallee, NotLineAnchor }                                   // DESIGN 5.2 order
impl UnsupportedReason { pub fn as_str(self) -> &'static str }        // snake_case names of DESIGN 5.2
pub struct UnsupportedSite { pub file: String, pub line: usize, pub symbol: Option<String>,
    pub family: Family, pub resolved_callee: String, pub reason: UnsupportedReason,
    pub grade: Grade }                                                 // one per occurrence until projection
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnprovenCause { StarImport, ShadowedRoot, NotImported, RepositoryLocalModule,
    BindingUnavailable, MemberRebound, ReceiverNotUnique, AnnotationConflict, HelperUnannotated, HelperNotAdmissible,
    ReceiverValueUnproven } // DESIGN 5.2 order
pub struct InventoryOutput { pub sites: Vec<InventorySite>, pub unsupported: Vec<UnsupportedSite>,
    pub parse_quality: BTreeMap<String, prism::finding_confidence::ParseQuality>, // every inventoried file, keyed as in sites
    pub unproven: BTreeMap<String, BTreeMap<UnprovenCause, usize>>,    // file → cause → count
    pub skipped: Vec<(String, String)>,                                // skipped Python file, loader reason
    pub files: Vec<String> }                                           // sorted, every inventoried file
pub struct FunctionBounds { pub name: String, pub start_line: usize, pub end_line: usize }
/// DESIGN 5.4 bounds of every function definition in the file, in source order (used by rows and by
/// the G-bounds example).
pub fn function_bounds(parsed: &prism::ast::ParsedFile) -> Vec<FunctionBounds>;
/// `files`: the Python files to inventory, keyed by repository-relative path. `repo_root` is needed by the
/// repository-local-module check.
pub fn collect(files: &BTreeMap<String, prism::ast::ParsedFile>, repo_root: &Path,
               opts: &InventoryOptions) -> InventoryOutput;
/// The Python files of DESIGN 5.5 ("Which files"): whole-repository load, exclusions, optional `--files`
/// filter; skipped Python files are returned for the warnings.
pub fn load_python_files(repo_root: &Path, only: Option<&BTreeSet<String>>)
    -> anyhow::Result<(BTreeMap<String, prism::ast::ParsedFile>, Vec<(String, String)>)>;

// targets/mod.rs
/// An inventory-only document (schema 1.1): no findings, `diff` built from `out.files`, `repo.sha` from
/// `meta.repo_sha`, the warnings of DESIGN 5.2.
pub fn inventory_document(meta: &TargetsMeta, out: &InventoryOutput) -> TargetsDocument;
/// Adds inventory rows to a projected document: sets schema 1.1, appends "prism.inventory/1" to
/// producer.algorithms, filters occurrences by confidence/tier, groups diagnostics, deduplicates row ids,
/// applies the severity floor (DESIGN 5.5/6.5), re-sorts by the existing key and appends the warnings of
/// DESIGN 5.2 and 5.5.
pub fn merge_inventory(doc: TargetsDocument, meta: &TargetsMeta, out: &InventoryOutput) -> TargetsDocument;
```

The family of a call comes from `py_tables.rs` applied to the resolved callee; `root_identity` supplies only
the root's path. The closed-world census of `py_receiver.rs` decides whether it is proven. Both projections get
file parse quality from `InventoryOutput` for every site and unsupported occurrence. Populate the map
at collection from the parser, including recovered files. A missing entry is an invariant failure.
Apply occurrence confidence and derived tier before grouping unsupported calls, aggregate by the full
DESIGN 5.2 key and serialize the surviving count and highest surviving grade (DESIGN 6.5). Site rules,
frames, bounds and the line anchor follow DESIGN 5.4/6.6 in `python.rs`. Use the same newline-normalised
parse and original-byte mapping for rows and the bounds example. The slice includes source-order and
type-only import refusals, alias ties, NFKC identity and the write checks in DESIGN 6.2. Index
canonical imported paths, bare builtin namespace members, literal mapping keys and one-hop assignment
aliases for vetoes. Distinguish slot capture from live member lookup; apply §6.2 ordering.
Join conditional class bindings with fallback in the evaluating namespace. For absolute imports
check non-package site ancestors, repository root and `src/` on disk, including outside `--files`.
Give absolute `builtins` builtin precedence. Tied relative imports have their explicit locality cause.
A sole table-library import resolving off-table supplies no root tie even when later than the call. Keep the legacy echo admission policy for G-byte. Implement narrow same-package import
coalescing and the sole table-import cause rule (sl3-02, as1-18, sl1-03, wf96–99).

Command line: `--inventory <NAME>` (value `python-deps`); `--inventory-kinds <LIST>`; `--diff` becomes
optional in clap. `run_targets` then enforces: no `--diff` and no `--inventory` → the same error text clap
gives today for a missing `--diff` (check `validation_test.rs` and `targets_test.rs` for a pinned message and
keep it passing); no `--diff` with `--inventory` → the inventory-only path (`load_python_files`, `collect`,
`inventory_document`, with `meta.repo_sha` from the existing `repo_sha`), which never calls the review
runner; an explicit review-runner option on that path (DESIGN 5.5's list, detected with clap's
`value_source`) → an error naming it; both given → today's path, then `load_python_files`, `collect`,
`merge_inventory`.

- [ ] Enable `"http.root"` in `ENABLED_GROUPS` as the first commit and show `matrix_rows_match` RED.
- [ ] CLI tests, RED first, in `targets_inventory_test.rs`:

```rust
#[test] fn inventory_only_run_needs_no_diff_and_writes_schema_1_1()   // exit 0; schema_version "1.1"; passes schema_support against targets-1.1.schema.json; producer.algorithms == ["prism.inventory/1"]; diff.files == sorted inventoried files
#[test] fn inventory_row_has_the_documented_shape()                   // fixture h01: kind external_call, category dependency_call.http, hint {kind http, callee requests.get}, expected.detail "source callee: requests.get", severity info, tier asserted, bounds, bare symbol
#[test] fn unsupported_site_is_an_other_row_with_a_reason()           // fixture u01: kind other, category dependency_call_unsupported, expected.detail "module_level", description with the reason, no bounds
#[test] fn unsupported_calls_of_one_callee_on_one_line_are_one_row()  // two non-anchor requests.get on one line: one row, detail "not_line_anchor; 2 calls"
#[test] fn identity_unproven_calls_are_counted_per_file_and_cause()   // n04 and n05, plus exact total 3 and causes 2/1 on the shared sl1-67/68/69 fixture (DESIGN 6.5)
#[test] fn inventory_only_document_carries_repo_sha_in_a_git_checkout() // a temporary git repository: repo.sha == git rev-parse HEAD
#[test] fn inventory_ids_are_stable_across_runs_and_thread_counts()   // two runs, RAYON_NUM_THREADS 1 and 4: identical bytes
#[test] fn inventory_merges_with_projected_findings()                 // --diff and --inventory: both row sources present, sorted by the existing key, no duplicate id
#[test] fn without_inventory_the_document_is_schema_1_0()             // existing fixture: schema_version "1.0", no prism.inventory rows
#[test] fn inventory_without_diff_rejects_review_runner_options()     // --algorithm echo, then --scoped-cpg, --old-repo, --cache-dir, each without --diff: error naming the option
#[test] fn inventory_respects_confidence_and_severity_filters()       // --min-severity warning: no inventory rows and the removal warning
#[test] fn inventory_kinds_rejects_an_unsupported_family()            // --inventory-kinds filesystem before S3: error naming the supported families
#[test] fn inventory_skips_venv_and_site_packages_directories()       // a fixture with venv/lib/x.py, lib/site-packages/y.py and pkg/x.py: only pkg/x.py in diff.files, and the excluded-directories warning
#[test] fn files_filter_narrows_the_inventory()                       // --files pkg/a.py: only that file's rows and only it in diff.files
#[test] fn vendored_1_1_schema_matches_its_recorded_hash()            // SHA-256 of docs/contracts/targets-1.1.schema.json equals a constant in the test
```

- [ ] Implement `py_tables.rs` (the HTTP entries of DESIGN 6.1), the root-identity and builtin part of `py_receiver.rs`,
      `python.rs`, `rows.rs`, `function_bounds`, `load_python_files`, the two document functions, the CLI
      wiring and `examples/inventory_bounds.rs` (arguments: a repository path; prints one JSON line per
      function of every inventoried file: file, start line, end line, name).
- [ ] Check the sibling `src/requests/__init__.py` collision from `app/main.py` with only the
      importing file selected. Test a package-local sibling (`httpie/output/formatters/json.py` pattern,
      esc17), bare and qualified `builtins` with a `src/builtins.py` decoy, and the controls in
      esc11–14/26. Confirm a later local off-table import is silent. Test direct false
      `TYPE_CHECKING`, the proven `typing_extensions` alias and a shadowing parameter (DESIGN 6.2).
      Test attribute and subscript annotated assignments in function scope: omit calls in their
      annotations while retaining evaluated target operands and RHS. Assert that the RHS wins in
      esc15/16 and the discarded annotation contributes no row. Test mismatched duplicate-text spans
      and an intentionally incorrect or absent route in the G-accept/G-precision evidence runner.
- [ ] `matrix_rows_match` GREEN for every `http.root` row.
- [ ] Review focus items 1, 2, 4, 5 and 6: one test each, named after the item.
- [ ] Gates of ORCHESTRATION section 5: items 1 to 4, 8, 9, 10 (no cache, data-flow or resolution change:
      5 to 7 do not apply). `main.rs` is near 1,000 weighted lines: put the inventory branch in a new
      function in a new file if it would pass that.
- [ ] Before S2a measurement, run the DESIGN 6.8 source preflight on all four census corpora for the
      eight empty-qualifier root shapes and bare builtins. Record counts including zero, occurrences,
      consumer qualifiers and any canonical builtin evidence. Check the M-1 declared roots against
      DESIGN 6.2's lookup set. Record ordered sibling or multiple roots, inactive `src/`, the candidate
      origin and actual precedence for each affected key. A default positive colliding with an active
      nonstandard root fails identity validation. An inactive `src/` cannot justify P2. Escalate missing/conflicting
      evidence to the owner under ORCHESTRATION §8 and leave the affected family gate unmet.
- [ ] Measure before review (measurement runner; G-accept steps on a Claude seat):
  - **G-bounds:** run the example on every census corpus and compare with `census/functions-<corpus>.jsonl`
    (`census/bounds_compare.py`); required difference: zero. A difference is a defect of `function_bounds`:
    add a matrix row, fix, re-run.
  - **G-byte:** run the merge-base binary and the head binary without `--inventory` on the two existing
    targets fixture directories; outputs must be identical after removing `producer.version`,
    `producer.build_identity` and `repo.root`.
  - Run the built binary with `--inventory python-deps` on every census corpus; record rows per family, per
    unsupported reason and per identity-unproven cause, and whether `update_warnings.py:44` is a row with the
    overlay's file, line, bounds and kind and the bare symbol (the S2a half of **G-overlay**).
  - **G-accept**, HTTP rows of this slice: `oracle/accept.py` (Claude seat) loads each document with
    faultpath's loader, runs `oracle/site_oracle.py` on every injectable row, runs `faultpath plan` with the
    corpus's import root, and checks DESIGN section 7's four conditions. Use a test-only sidecar
    keyed to revision, file, projected ID and occurrence. Emit the producer-selected route
    (`root`, `R1`, `R2`, `R3`, `R4`, `R5`) and discovery-time `Call.func` and whole-call spans.
    Capture the route at the successful resolver branch and associate its occurrence with the emitted
    ID at projection. Generating a route from the projected grade or callee would lose this evidence.
    Independently replay the ordered rules from source on that occurrence, then compare the route.
    Missing, duplicate or disagreeing route evidence fails condition (2) and prevents precision
    stratification. An emitted R3 row fails replay. Preserve the public targets schema. Extract the
    same spans from the actual consumer-bound occurrence, check both ranges and text, and fail condition (2) on missing or unequal evidence. Follow DESIGN 6.8 for bare
    builtins, empty qualifiers and recovered sites. Report any unmet identity/binding check. Tabulate
    other guard answers and planner notes per reason.
  - Put the tables in the PR body.
- [ ] Two reviewers. Merge. Comment on #613 with the httpie table.

### Slice S2b: receiver routes R1 to R5 (2 to 2.5 days)

**Files:**
- Modify: `crates/prism-review/src/targets/inventory/py_receiver.rs` (admission and diagnostics under the closed-world
  rule in DESIGN 6.2–6.5), `python.rs` (building `<class path>.<method>`),
  `crates/prism-review/tests/cli/targets_inventory_matrix_test.rs` (`ENABLED_GROUPS` gains
  `"http.receiver"`), `CLAUDE.md` (the `targets/` paragraph)
- Create (evidence directory): `precision/sample.py`, `precision/PRECISION.md`

- [ ] Enable `"http.receiver"` as the first commit; `matrix_rows_match` RED for its rows.
- [ ] Implement R1/R2, diagnostic R3 and constructor-backed R4/R5 (DESIGN 6.2–6.5).
      Check write timing at capture, construction, context entry and selected-member lookup,
      including `__new__`, `__init__`, the selected enter protocol and `__getattribute__` (esc06–10,
      esc20, esc25, esc28 and esc30).
      Verify wf26 versus wf27, fresh n06/wf38/esc29 versus persistent wf10/wf43/esc21, and alias
      writes in wf29–53. A field store never yields an R3 positive. Check setter, subclass and
      function-parameter cases e07, tr3-20, as1-20, wf40 and esc27, plus h07 as the recall cost.
      Distinguish parameter/default and literal conflicts from missing value evidence; test R4
      priority over R5, the unique unconditional return, a returned local with a single construction,
      and an alternate return or fallthrough (esc01–05/23/24). Preserve the earlier warning order.
      Count failed inline R1 and eligible receiver ties (wf74–95). Check mixed filters with esc22
      (two unfiltered, one exact/asserted, zero above info); as1-19 and as2-27 each have one surviving
      member plus one unproven occurrence. Require GREEN on every enabled row.
- [ ] Review focus item 3: one test.
- [ ] Gates of ORCHESTRATION section 5, as for S2a.
- [ ] Measure before review: re-run the S2a measurements (G-bounds unchanged, G-byte) on every census corpus;
      **G-overlay** in full (both HTTP overlay sites are rows with the overlay's file, line, bounds and kind
      and the bare symbol); **G-accept** for every HTTP row; **G-precision**: `precision/sample.py` draws the
      strata of DESIGN section 7 with a recorded seed, and a Sonnet-5.5 reader checks each drawn row against
      the source and writes `precision/PRECISION.md` (stratum sizes, rows checked, wrong rows).
      Use the S2a sidecar and gate replay for every HTTP occurrence before sampling. Report the
      empty R3 stratum and tied losses. Publish source-witnessed exceptions and count demonstrated
      mismatches under the original gate. At `httpie/client.py:114` record the actual return, constructor and
      relevant writes. If it lacks a witness, G-overlay is unmet and the owner decides the next step.
      Each wrong row becomes a matrix row and is fixed; the gate fails when a stratum has more than one wrong row.
- [ ] Two reviewers. Merge.

### Slice S3: standard-library families and scan parity (2.5 to 3 days)

**Files:**
- Modify: `crates/prism-review/src/targets/inventory/py_tables.rs` (DESIGN 5.3's standard-library table,
  verbatim), `py_receiver.rs` (constructors `pathlib.Path`, `configparser.ConfigParser`,
  `configparser.RawConfigParser`, with the same field deferral and annotation/value rules
  as HTTP under DESIGN 6.3–6.4), `inventory/mod.rs` (`Family::supported`),
  `crates/prism-review/tests/cli/targets_inventory_matrix_test.rs` (`ENABLED_GROUPS` gains `"stdlib"`)
- Create (evidence directory): `parity/compare.py`, `parity/PARITY.md`

- [ ] Enable `"stdlib"` in `ENABLED_GROUPS` and add the three families to `Family::supported()`: the matrix
      is RED for their rows. Add every canonical entry with its 5.3 family, including `Path.home` as
      config, and enable the same R4/R5 proofs for the three classes. GREEN.
- [ ] Add a unit test that the standard-library table in `py_tables.rs` equals DESIGN 5.3's list, entry for
      entry, with each entry's family. Test `async with Path(...) as p` alone gives plain none at
      `p.read_text()`. Check bare/qualified builtin mutation and the Path class member write across
      root, constructor, annotation and helper proofs. Check esc18/19 and the census-positive
      `json.loads` at `httpie/output/formatters/colors.py:183`.
- [ ] `parity/compare.py <prism.json> <scan.json>` (lane interpreter): restrict both sides to the files in the
      scan document's `diff.files`; key each injectable row by (file, line, function_start_line,
      function_end_line, family, resolved callee); compare prism's `module_level` unsupported rows with
      scan's rows without bounds; read scan's warnings (`<file>:<line>: <source callee>: <reason>`) as its
      unsupported and unproven set. Pair prism's `not_line_anchor` with scan's `ambiguous_call_anchor`
      at the site. Compare `star_import` with scan's `wildcard_import_binding_unknown` by file. An
      aggregate count cannot identify an individual missed site. Print recall per corpus (share of
      scan's injectable rows prism lists), both set differences and the pairings.
- [ ] Run `faultpath scan --kinds filesystem,parse,config` (Claude seat; each corpus on a clean checkout,
      with its import root) and `prism targets --inventory python-deps --inventory-kinds
      filesystem,parse,config` on every census corpus. Classify every difference in `parity/PARITY.md`:
      prism misses a site scan lists (name the exact P1–P5 class from DESIGN 6.7, its key, the source
      witness, independent replay of all applicable proof routes and shape, and prism outcome.
      P1 writes require a feasible order at the actual slot read. P2 requires a relative import or
      first importable origin under the active corpus roots. Keep every scan key in the denominator,
      including files prism omitted. A representable multiline
      scan key or differing anchor needs a matrix regression and fix or owner escalation); prism lists a site scan
      does not (confirm against the source); the two disagree on bounds, line or callee (decide by the
      source and faultpath's guard; a prism defect gets a matrix row and a fix). A table entry may be added
      in this slice only when DESIGN 5.3's list (the consumer's table) has it. A miss that belongs to no
      approved loss class, or whose fix would need a new route or site shape, goes to ORCHESTRATION section
      8; the measurement runner does not add routes.
- [ ] **G-parity** as DESIGN section 7 states it, on every census corpus. If scan cannot run, G-parity is not
      met: stop the slice before review and escalate to the owner.
- [ ] **G-accept**, standard-library half, on every census corpus (as in S2a).
- [ ] Two reviewers. Merge.

### Slice S4: the pipeline on httpie, funnel report, close-out (2 days)

**Files (evidence directory):** `e2e/targets-httpie-http.json`, `e2e/funnel.py`, `e2e/REPORT.md`,
`e2e/HANDOFF-tooling.md`.
**Files (repository):** `docs/features/targets-inventory.md` (what is listed, the families and tables, the
routes and the closed-world rule, the unsupported reasons and unproven causes, what is out of scope).

- [ ] Produce `e2e/targets-httpie-http.json` with the merged binary: `prism targets --repo <httpie>
      --inventory python-deps --inventory-kinds http --out …`; note the ids of the rows at `client.py:114`
      and `update_warnings.py:44`.
- [ ] **G-live** (Claude seat; one working day at most). Under the lane's interpreter with httpie installed
      from the pinned checkout and the package pins of faultpath's httpie pilot profile (location in the
      private bindings file):
  1. Flake control: run the suite once with no faults, with the profile's pytest options, deselections and
     test paths; record the failing tests; they are excluded from verdicts.
  2. `faultpath plan --targets e2e/targets-httpie-http.json --repo-root <httpie> --import-root .` with the
     profile's execution exclusions, `--only-target` for each of the two ids, `--out e2e/plan.json`.
  3. `faultpath run` with the same targets, inputs and `--only-target` values, `--expected-plan
     e2e/plan.json`, and `--test-cmd` running pytest with the profile's options and test paths; outputs in
     `e2e/`. Each test that newly fails under a fault gets one no-fault re-run before its verdict counts.
     No host-quiet marker.
  4. Check DESIGN section 7's conditions (i) and (ii). Tabulate the per-fault verdicts beside attempt 5's
     (private bindings file) as information.

  A refusal of `plan` or `run` for its own reasons (a missing input the profile defines, a wrong flag) is an
  inadmissible probe: fix the invocation and repeat within the day. If the conditions cannot be met within
  the day without changing faultpath, stop, write what was reached and what blocked in `e2e/REPORT.md`, and
  raise it to the owner: G-live is required, so the lane is not complete until it holds or the owner
  re-routes it.
- [ ] `e2e/funnel.py` joins the independent census, the prism documents, the G-accept tables and, for httpie,
      the G-live results into the funnel of DESIGN section 8, for every census corpus. `e2e/REPORT.md`
      holds the funnels, the loss reasons per step, and the httpie sites prism found beyond the overlay with
      their planner outcomes.
- [ ] Write `e2e/HANDOFF-tooling.md` for the tooling controller (decision 6): the merged prism commit, how to
      produce the document, the G-live result, and what a pilot-level run on prism's document still needs on
      faultpath's `main` (a reach document and a validated pilot configuration for the census; a driver that
      accepts a foreign targets document; the census's 25-target cap, which an all-family document can fill
      with standard-library rows ahead of HTTP rows; at most 3 faults per target). The note stays in the
      evidence directory; name it in the lane handoff and in the close-out report to the owner, who routes it
      (writing into the tooling repository is an owner-only action, ORCHESTRATION section 9). This lane does
      not run the pilot.
- [ ] Write `docs/features/targets-inventory.md`.
- [ ] Close #613 with the funnel. Give the owner the parity numbers and proposed scan/prism coexistence
      question for any faultpath issue (DESIGN decision 7). File prism issues for provenance noise on
      `requests.*` lines (the `request` substring match and `fetch` → database hint), for the families
      and languages left out, and for projected finding bounds. Projected rows take their bounds from
      the tree-sitter function node, which includes decorators and trailing indented comments and can
      cause `stale_function_bounds`. Inventory `function_bounds` supplies the reference behaviour.
- [ ] Refresh the handoff; delete the worktree's `target/`.

## Change log

- 2026-11-24. Folded Stage D round 1. T-1 now consumes complete group keys and checks silent
  fixtures. S1 exposes the shared crate interface. S2a–S3 carry parse quality and occurrence evidence,
  apply the revised identity rules and measure the stricter acceptance and parity gates.

- 2026-11-25. Folded the Stage D escalation. S1–S3 now specify the two callers’ import roots,
  value witnesses and field deferral, class fallback, per-slot ordering, verified route sidecars and
  parity evidence. The escalation review is pending before matrix transcription.

## Acceptance for the lane

G-byte, G-bounds, G-overlay, G-accept, G-parity and G-precision hold on the merged head; G-live holds; the
funnel report is posted on #613; the tooling handoff note is written; both reviewers approved S2a, S2b and
S3.
