# Lane 6: provenance findings by proven identity — design

Status: architecture final; the per-language origin rules are tentative until the census, the labelled
sample and the test matrix (PLAN stages M, T, E). The plan files a tracking issue. Research record:
`~/prism-evidence/lanes-2026-11-20/research/lane6-provenance.md`. Design review round 1 (astra) folded.

## 1. Intent

The provenance algorithm tells a reviewer where a changed line's values come from: user input, a database,
configuration, the environment, a constant. It is in the default review suite and in the default
`prism targets` set. Today it decides by substring on whole source lines, and most of what it reports is
wrong: on a 62-line Python file it produced 26 findings at the highest severity, including "variable
'requests' has user_input origin" for the `requests` module and the same for `subprocess`, `check` and
`run`. The lane makes each finding rest on evidence: the variable's own definition, and an origin that is
proven by what the defining expression is.

## 2. What exists today (verified on `origin/main` at 1179055f)

| Fact | Reference |
|---|---|
| For every identifier on every diff line (module, attribute, callee, function and type names included), the algorithm looks up definition lines by name across the whole file, not by scope, classifies the raw text of a line against five substring tables, and emits one finding per identifier; the first definition that classifies wins. If no definition line classifies, the diff line's own class is given to every identifier on it. Constant and unknown origins emit nothing. | `crates/prism-review/src/algorithms/provenance_slice.rs` |
| The tables are not language-gated (Terraform and shell patterns run on Python), ignore comments and strings, and their "word boundary" is checked on the left only (`~form` matches `format`, `~fetch` matches `fetchConfig`). Precedence hides the true class: `db.query(` is `user_input` because `query` is tested before the database table. A constant is any line containing `= 1` or `= "`. | same file, `classify_line` |
| The `external_call` origin is never produced. The only precision mechanism is one import check that has no test. | `src/frameworks/origin.rs:19` |
| Finding: category `untrusted_origin`, description `variable 'X' has <origin> origin: <risk>`, severity `concern` for user input and hardware, `warning` for database, `info` for parameter, environment and configuration. No cap per line or per run. The targets projection recovers the origin by parsing the description, and attaches a `db` hint for every database origin. | `crates/prism-review/src/targets/mapping.rs:340` |
| Probes on three decoy-heavy fixtures (labelled by one rater): correct findings were 22% (Python), 31% (JavaScript) and 14% (Go); every real source read was caught, by pattern, except one-byte names, which are skipped (Go `q := r.URL.Query().Get("q")`). On two Python corpora about 3 to 5% of lines classify as user input, and a quarter to a third of sampled classified lines were comments, docstrings or strings. | research sections 6 and 7 |
| Identity facilities that exist: import-proven library tables for HTTP, database, queue, filesystem and process calls in Python, JavaScript/TypeScript and Go (`targets/dependency_identity.rs`, private to the targets module; matched by package prefix, and for Go and JS without the call path); request-source recognition for web frameworks (inside the taint module, private; Go's is declarative and public, and its seeds name a line only); scope-aware binding for Python, JavaScript and Go (inside `taint_sinks/`, private); public helpers for imports, scoped references and comment or string nodes. Nothing recognises environment or configuration reads precisely. Provenance and taint share only the `Origin` type. | research section 4; review r1 findings 5 to 7 |
| The graph the algorithm reads: module, package and class-body code has no variable nodes (callable owners only); a Python attribute read `obj.label` can carry a zero-width `Use(label)` fed by an `exact` edge from a same-named local; edges labelled `killed`, and edges from a definition that follows the use with no loop between (`cfg_incomplete`), stay in the graph as predecessors; a Go formal read only through a member has no Def node; a Java `T x = e;` Def is keyed by the whole declarator text. The review loader parses only the diff's files. | review r1 probes P1 to P7; `src/api/review.rs:78` |
| Tests: most provenance tests assert only that some block exists, which no precision change can affect. A handful pin substring-triggered classifications (`request.GET[...]` on a bare `request` name with no import; a repository-local `request = Request()` expected to give a `user_input` target) and would change. There are two negative tests and no mutation lane. | research section 3 |

## 3. Goals, non-goals, anti-goals

Goals:

1. Findings only for variables, looked up by scope, classified from the variable's defining expression.
2. For Python, JavaScript/TypeScript and Go, each origin class is assigned by proof: a request read on a
   recognised handler parameter; a listed origin-bearing call into an import-proven database, HTTP or
   configuration library, or on a handle obtained from one; an environment read by its built-in or imported
   name; a literal; a parameter.
3. The `external_call` origin is produced (a value returned by an HTTP client call), so the targets hint
   for it means something.
4. Other languages keep pattern classification with the structural defects fixed (variables only, scoped
   lookup, no comments or strings, gated tables, two-sided word boundaries).
5. Measured before and after on real diffs: finding volume by origin and severity; precision per origin
   class on a labelled sample of each build's findings, `info` included; and recall against a census of real
   source reads drawn from the diffs independently of both builds, each loss listed with its reason.

Non-goals: new origin classes; changing severities, the category or the description format (the targets
projection parses it); provenance across files; a cap or a ranking; removing provenance from any default.

Anti-goals: silently reporting less (every dropped class of finding is counted in the measurement); a
third copy of the import-identity proof; identity rules that only pass because fixtures were tuned to them.

## 4. Approaches considered

**A. Tighten the patterns only** (two-sided boundaries, language gating, skip comments). Removes a large
share of the noise cheaply and stays a text heuristic: `request` as a local name would still be user input.
Kept as the first slice and as the rule for languages without identity facilities; not sufficient alone.

**B. Classify the defining expression by proven identity** (chosen for Python, JavaScript/TypeScript, Go).
Uses facilities the code base already has for the same questions in taint and in targets.

**C. Derive provenance from the taint engine** (a variable is user input when a taint source reaches it).
Attractive as a single source of truth, but taint sources are seeded per diff and per framework handler,
and provenance also reports database, configuration and environment origins that taint has no notion of.
Not chosen; the request-read recogniser is shared instead.

**D. Remove provenance from the default suites until it is rebuilt.** One line, immediate relief, and a
visible behaviour change for every user. Left to the owner (section 8, decision 1); the lane does not
depend on it.

## 5. Architecture — *origin rules tentative*

### 5.1 What is examined

For each diff line, the identifier occurrences in a value-read position, found in the syntax tree: not a
callee, an attribute or member name, a module, a type, a declaration target or a keyword (stage D fixes the
node kinds per language). Each occurrence's binding is resolved lexically at the occurrence. An occurrence
that resolves to a file import is a module and is dropped; a local or parameter spelled like an import is
examined. A graph Use is taken for an occurrence only when its bytes are the occurrence's; a graph Use with
no occurrence (a zero-width anchor, an attribute spelling) is never examined. Findings stay one per name
per line, and the join below runs over every occurrence of that name on the line.

**Reaching definitions, graph route.** For an occurrence with a graph Use: its Def predecessors in the same
function that reach it on a live route. An edge labelled `killed` contributes nothing. A definition that
starts after the use contributes only when a loop of the function encloses both (loop-carried flow); with
no such loop it never does, whatever its label. Every other label (`exact`, `sameline`, `cfg_incomplete`,
`alias_unstable`, `call_nameonly`) contributes. The finding's evidence is the selected definition's own
edge, so its grade is that edge's label; an origin reached by propagation or by the syntax route is
`unlabeled`. A use whose predecessors are all excluded has no origin (traced `killed-only`).

**Reaching definitions, syntax route.** Where the graph has no Use for the occurrence, or the Use has no
predecessor at all, the definitions come from the occurrence's lexical binding in the same file: module,
package and class-body code (Python, JS/TS, Go), a name read in a function but bound only at module level
or in an enclosing function (a capture), a formal read only through a member, a Java declarator. In
straight-line code at one nesting level the last binding before the use is taken; bindings under a branch
or loop are all taken; for a module-level or captured name read inside a function, every binding of it in
its scope is taken. Nothing examined is silently skipped: each occurrence is reported or traced as
unknown with its reason (5.5).

Several definitions with different origins give the highest-severity one, then the earliest definition
(today the first definition that classifies wins).

### 5.2 Origin by proof (Python, JavaScript/TypeScript, Go)

| Origin | Proven when the defining expression is |
|---|---|
| `function_param` | a formal parameter that is not a recognised request parameter |
| `user_input` | a request accessor that the taint engine's recogniser lists, matched to the definition by position: the accessor's own bytes lie in the definition's value (for several targets, in the value slot paired with this target), and its receiver resolves at the accessor to the recognised handler parameter; a seed that names only a line is a hint, never a match. Or a command-line or standard-input source by its built-in or imported name (`sys.argv`, `input()`, `process.argv`, `os.Args`, `flag.*`). Two cases the recogniser does not cover are fixed at stage D: a framework's module-level request object proven by import but read outside a route-decorated function (Flask's `request` in a helper; tentative: user input), and browser sources read from an unshadowed global (`document.cookie`, `location`; tentative: user input) |
| `database` | a call listed as origin-bearing (a query or fetch result) whose callee is proven to be the library's module, or a method on a handle whose single definition is a listed constructor or transition of that library (connection, cursor, session; at most `HANDLE_HOP_DEPTH` hops) |
| `external_call` | a listed response-bearing call of an import-proven HTTP client (or built-in `fetch`), or a listed read on its response or session handle |
| `env_var` | `os.environ[...]`, `os.environ.get`, `os.getenv`, `process.env.X`, `os.Getenv`, `os.LookupEnv`, by built-in or import identity |
| `config` | a listed configuration read of an import-proven configuration or settings library; a file parser's result keeps its input's origin unless its input is a listed configuration source |
| `constant` | a literal |
| `unknown` | anything else; no finding |

Identity is per callable, not per package. Closed per-language tables (stage D) name each listed operation
and its value semantics: payload, handle constructor or transition, helper or metadata, control or error
result, output argument (`row.Scan(&name)` defines `name`). Only a payload, or an output argument listed
as one, carries the origin; for a multi-result call only the payload slot does (Go `v, err := ...` gives
`err` nothing). A call rooted in a proven package and absent from the table is `unknown`: helpers such as
`sqlite3.complete_statement`, `requests.utils.quote`, `http.StatusText`, `axios.isAxiosError`, `close()`
and constructors such as `flag.NewFlagSet` carry no origin.

Propagation: a value takes another variable's origin only through the forms stage D lists: a plain alias,
a member or element of that value, and a short closed list of named pass-through calls. A call's result
does not inherit an argument's origin by default (`number = fixed(raw)` is `unknown`), and neither does an
operand the language discards (JS `(raw, 7)`). At most `PROPAGATION_DEPTH` steps within the function; each
cutoff is traced with its reason. A name whose binding is not proven (shadowed, rebound, imported from a
repository-local module of the same name, or a Python import when no repository root is available) is
`unknown`.

### 5.3 Other languages

The existing tables stay, with five structural fixes that apply to every language: variables only (5.1);
scoped definition lookup; the defining value expression's text (the right-hand side, or the call an output
argument belongs to), never the whole statement, so the defined name itself cannot match; no text inside
comments or strings, except shell `$(..)` and backtick and HCL `${..}` substitutions, which are code; tables
gated by language; word boundaries on both sides, and an entry ending in `(` matches only a whole callee
token (`read(` does not match `spread(`). The fallback that hands a line's class to every identifier on it
is removed. It is today the only route by which some forms are classified at all (shell `read x` and
positional parameters); stage D gives each such form a defining-statement rule or lists it as an accepted
loss, and does the same for forms the two-sided boundary loses: Java JDBC (`executeQuery`), Rust and Lua
calls into the C API (`libc::getenv`), Bash `read -r value` as the definition of `value`.

### 5.4 Shared identity

The import-identity proof and library tables are used from where they are, made visible within the review
crate. Lane 2 extracts a call-level entry point from the same code (`root_identity`); this lane uses that
entry point and adds tables only for classes it lacks (configuration libraries, environment reads,
command-line sources, and the per-callable operation tables of 5.2). That entry point proves call nodes
only; environment and command-line reads are subscripts and member reads, so this lane adds a sibling for
non-call paths. The proof's site context needs a repository root: the review entry point already receives
it (`run_algorithm`) and passes it to provenance, so the local-module check sees files the diff does not
name. Where no root is available (the compatibility entry point), a Python import is not proven and no hint
is attached; the check is never run against the working directory.

Provenance adds site refusals on top of the shared proof (refusals, not a copy of it), each giving
`unknown` with a traced reason: a write to the root name anywhere in the file other than its import (JS
assignment, Python rebinding); a write that replaces the called member (`requests.get = ...`); a local
binding of the `require` loader; a catch, loop or match binder or a Go method receiver named like the root;
a named import or destructured binding whose original member the proof does not carry (`import {get as
read}`); a Go dot import, or a versioned path whose package name is not its last element (`pgx/v5`). The
same gaps affect targets hints on `main` today; they are filed for lane 2, which owns the module.

The request-read recogniser is exposed from the taint module by a function, without moving it; its seeds
are matched by position as 5.2 says, by a matcher in this lane. If lane 2 has not merged, this lane's first
slice that needs it carries the same small extraction; who owns that commit is the controller's to settle
(PLAN close-out).

### 5.5 Output

Category, severity mapping and description format are unchanged, so the targets projection and SARIF keep
working. The targets hint for a database origin is attached only when the origin was proven by library or
built-in identity. The projection sees only the description, so the rule is applied by language: Python,
JavaScript/TypeScript and Go never fall back to patterns, so a database or external-call origin there is
always identity-proven, and other languages get no hint; `external_call` origins get the `network` hint that
the mapping already defines. The number of findings changes; that is the point, and it is measured.

Trace (`PRISM_PROVENANCE_TRACE=1`, tab-separated on stderr, default output unchanged), enough to reconcile
every examined variable: each examined occurrence gives exactly one `prov-var` (reported: origin, route,
determining definition line and bytes, definition source `graph:<label>` or `syntax`, and for a propagated
origin the parent chain to the terminal proof) or one `prov-unknown` (examined, no finding: reason such as
`killed-only`, `unlisted-call`, `not-pass-through`, `depth`, `unproven-binding`, `local-module`, `no-repo-root`,
`constant`); every other identifier on a diff line gives one `prov-drop` with its role. Each request source
gives `prov-request-source` with its accessor bytes, the matched target or `-`, and the diff lines that read
the target's binding.

### 5.6 Where the code goes

```
crates/prism-review/src/algorithms/provenance_slice.rs      entry points; delegate
crates/prism-review/src/algorithms/provenance/mod.rs        reaching definitions, origin join, trace
crates/prism-review/src/algorithms/provenance/occurrence.rs value-read occurrences, roles, syntax route
crates/prism-review/src/algorithms/provenance/patterns.rs   the tables, gated and with two-sided boundaries
crates/prism-review/src/algorithms/provenance/identity.rs   origin by proof, site refusals, request matcher
crates/prism-review/src/algorithms/provenance/{python,js_ts,go}.rs   per-language defining-expression readers and tables
```

## 6. Measurement

**Population.** Real diffs, so that volume reflects use: the 120-diff population already used for issue
#602 (six repositories with full history), plus 60 Python-heavy diffs drawn the same way from two Python
corpora, plus 60 pattern-language diffs (15 each from C, C++, Java and Rust repositories already cloned for
benchmarks) for the structural slice's check. Lua, Bash and Terraform have no corpus. Provenance alone is
run on each diff with the base and the head build.

**Source census (recall).** Drawn before any head run and independently of both builds: 400 changed lines
with at least one value-read identifier, by fixed seed, stratified by family, with an oversampled stratum of
lines inside functions the taint recogniser marks as request handlers; inclusion weights are kept. Readers
label every read on each line: variable or not, its determining definition, its origin class (any class,
`info` included) or `insufficient_evidence`. Added to it: every taint request source whose target's binding
is read on a diff line, listed by syntax at the no-behaviour-change slice, before the first identity slice. Recall per origin class is the
weighted share of labelled source reads that the head reports with the same origin; the base build is
scored on the same keys, and retention of base-correct findings is reported beside it. Definition-only
changes (a source written on a changed line and read only on unchanged lines) are counted as their own
out-of-scope class.

**Precision samples.** From each build's findings at every severity, `info` included, stratified by the
finding file's language and origin class: at least 30 per cell, or the whole cell when smaller. Estimates
use the inclusion weights and give numerator, denominator and an interval per cell; a cell is never pooled
into a total that could hide it.

**Readers.** Two readers label each item independently, a third read resolves disagreements; per-rater,
adjudicated and unresolved counts are reported, and the results are stated as agreement with model
adjudication. Each item carries the use, the whole enclosing function, the file's imports and the
determining definition the build selected (for the census, the reader finds it in the function); the reader
names the origin it sees before seeing the claimed one; `insufficient_evidence` is an answer. The rubric
for the request or handle object itself, for payloads and for definition-only changes is fixed before
labelling; if stage D changes it, both samples are re-adjudicated under the final rubric.

**Cross-check.** Every request-source use listed for the census must carry a `user_input` finding on the
head, except the finite list stage D accepts.

## 7. Gates

| Gate | Threshold (default) |
|---|---|
| Census go | on the base sample, precision at `concern` and `warning` is below 60%, computed with the request or handle object itself counted right and counted wrong; at or above 60% both ways: stop and report; if the two straddle 60%: the owner decides |
| Precision | at least 85% right in every head cell (language × origin class, `info` included) with at least 30 findings; smaller cells are labelled in full and listed. A slice that changes default output for a language passes this on its own head for that language before it merges (PLAN stage I); stage G confirms |
| Recall | per origin class, at least 95% of the census's labelled source reads reported with the same origin; each loss listed with its trace reason; base retention reported beside it |
| Cross-check | section 6's cross-check holds, with no exception outside stage D's finite list |
| Hints | no targets hint on a row whose origin was not proven by library or built-in identity |
| Contract | the description format, category and severities are unchanged; the targets mapping tests pass unmodified except where a fixture's expectation was a substring artefact, each named in the PR |

For this lane the yield line of ORCHESTRATION section 7 is census recall per origin class; raw finding volume
is a separate ledger, expected to fall.

## 8. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Remove provenance from the default review suite and default targets set until the lane lands | no; defaults unchanged | one line each way | any time |
| 2 | Findings for names that are not variables | none (they are dropped and counted) | — | stage D |
| 3 | `info`-level findings (parameter, environment, configuration) | still emitted, now only when proven (constant and unknown emit nothing, as today) | suppressing them is a severity-floor choice the user already has | — |
| 4 | Languages with identity rules | Python, JavaScript/TypeScript, Go; others get the structural fixes only | each further language is its own slice | stage D |
| 5 | Precision and recall thresholds | section 7 | none | stage G |
| 6 | Precision check for pattern languages | C, C++, Java, Rust on the 60 pattern-language diffs; Lua, Bash, Terraform by matrix rows only | adding a corpus later is one population row set | slice S1 |

## 9. Risks and accepted limits

- **Recall can fall where identity is not proven** (a request object passed through a helper, a database
  handle built by a factory, an unlisted operation, a renamed handler parameter the recogniser does not
  accept). Those become `unknown`; the census recall gate and the listed losses make the cost visible, and
  the handle-hop and propagation depths are parameters the matrix fixes.
- **Graph defects are not relied on.** The zero-width attribute Use, the backward `cfg_incomplete` edges and
  the Java declarator key are main defects, filed with the review's probes and pinned by matrix rows; the
  occurrence filter and the later-definition rule make provenance independent of them.
- **Tests that encoded substring behaviour change.** Each is named in the PR with the reason; none is
  deleted without a replacement that states the intended behaviour.
- **The labelled samples are model-judged.** Two readers, a third read, the determining definition in each
  item and stratification reduce, not remove, that; the cross-check is the mechanical part.
- **Target ids change with each slice** that changes a finding's related lines, description or severity
  (target ids hash them); an intermediate consumer can see more than one change.
- **The real-diff population covers Python, JavaScript/TypeScript, Go, C, C++, Java and Rust.** Lua, Bash
  and Terraform are checked by matrix rows only.
- **The targets module is lane 2's.** This lane's edits there are additive (visibility, an optional field,
  an optional repository root), alter no existing result, and rebase over lane 2's first slice.
- **Out of model:** origins that cross files, values assembled through containers, reflection.

Recorded, not pursued:
- Repeated import and definition scans per use may cost time in default runs (review r1, no regression
  shown); memoize per file only if M-2's timings show it.
- The hint rule rests on the producer invariant: a synthetic or foreign provenance finding passed to the
  mapper gets a hint by language. The Hints gate checks the invariant end to end on the population.
