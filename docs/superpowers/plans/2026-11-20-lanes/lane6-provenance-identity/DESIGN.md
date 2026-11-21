# Lane 6: provenance findings by proven identity — design

Status: architecture final; the per-language origin rules are tentative until the census, the labelled
sample and the test matrix (PLAN stages M, T, E). The plan files a tracking issue. Research record:
`~/prism-evidence/lanes-2026-11-20/research/lane6-provenance.md`.

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
| For every identifier on every diff line (module, attribute, callee, function and type names included), the algorithm looks up definition lines by name across the whole file, not by scope, classifies the raw text of a line against five substring tables, and emits one finding per identifier. If no definition line classifies, the diff line's own class is given to every identifier on it. | `crates/prism-review/src/algorithms/provenance_slice.rs` |
| The tables are not language-gated (Terraform and shell patterns run on Python), ignore comments and strings, and their "word boundary" is checked on the left only (`~form` matches `format`, `~fetch` matches `fetchConfig`). Precedence hides the true class: `db.query(` is `user_input` because `query` is tested before the database table. A constant is any line containing `= 1` or `= "`. | same file, `classify_line` |
| The `external_call` origin is never produced. The only precision mechanism is one import check that has no test. | `src/frameworks/origin.rs:19` |
| Finding: category `untrusted_origin`, description `variable 'X' has <origin> origin: <risk>`, severity `concern` for user input and hardware, `warning` for database, `info` for parameter, environment and configuration. No cap per line or per run. The targets projection recovers the origin by parsing the description, and attaches a `db` hint for every database origin. | `crates/prism-review/src/targets/mapping.rs:340` |
| Probes on three decoy-heavy fixtures (labelled by one rater): correct findings were 22% (Python), 31% (JavaScript) and 14% (Go); every real source read was caught, by pattern. On two Python corpora about 3 to 5% of lines classify as user input, and a quarter to a third of sampled classified lines were comments, docstrings or strings. | research sections 6 and 7 |
| Identity facilities that exist: import-proven library tables for HTTP, database, queue, filesystem and process calls in Python, JavaScript/TypeScript and Go (`targets/dependency_identity.rs`, private to the targets module); request-source recognition for web frameworks (inside the taint module, private; Go's is declarative and public); scope-aware binding for Python, JavaScript and Go (inside `taint_sinks/`, private); public helpers for imports, scoped references and comment or string nodes. Nothing recognises environment or configuration reads precisely. Provenance and taint share only the `Origin` type. | research section 4 |
| Tests: most provenance tests assert only that some block exists, which no precision change can affect. A handful pin substring-triggered classifications (`request.GET[...]` on a bare `request` name with no import; a repository-local `request = Request()` expected to give a `user_input` target) and would change. There are two negative tests and no mutation lane. | research section 3 |

## 3. Goals, non-goals, anti-goals

Goals:

1. Findings only for variables, looked up by scope, classified from the variable's defining expression.
2. For Python, JavaScript/TypeScript and Go, each origin class is assigned by proof: a request read on a
   recognised handler parameter; a call into an import-proven database, HTTP or configuration library, or
   on a handle obtained from one; an environment read by its built-in or imported name; a literal; a
   parameter.
3. The `external_call` origin is produced (a value returned by an HTTP client call), so the targets hint
   for it means something.
4. Other languages keep pattern classification with the structural defects fixed (variables only, scoped
   lookup, no comments or strings, gated tables, two-sided word boundaries).
5. Measured before and after on real diffs: finding volume by origin and severity, and precision on a
   labelled sample, with every real source in the labelled set still reported.

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

For each diff line, the variables read on it: identifier uses that the data-flow graph records as uses of a
variable, not names in callee, attribute, module, type or declaration position. For each, the definitions
that reach that use in its own function (the reaching definitions the graph already has; where none is
available, the scope-aware reference search). The origin is decided from each reaching definition's
defining expression, as syntax. Several definitions with different origins give the highest-severity one,
as today.

### 5.2 Origin by proof (Python, JavaScript/TypeScript, Go)

| Origin | Proven when the defining expression is |
|---|---|
| `function_param` | a formal parameter that is not a recognised request parameter |
| `user_input` | a read of a request accessor on a recognised handler parameter (the recogniser the taint engine uses for framework sources), or of a command-line or standard-input source by its built-in or imported name (`sys.argv`, `input()`, `process.argv`, `os.Args`, `flag.*`). Two cases the taint recogniser does not cover are fixed at stage D: a framework's module-level request object proven by import but read outside a route-decorated function (Flask's `request` in a helper; tentative: user input), and browser sources read from an unshadowed global (`document.cookie`, `location`; tentative: user input) |
| `database` | a call whose callee root is an import-proven database library, or a method call on a handle whose single definition is such a call (one or two hops: connection, cursor) |
| `external_call` | a call into an import-proven HTTP client library, or a method on its response or session handle |
| `env_var` | `os.environ[...]`, `os.environ.get`, `os.getenv`, `process.env.X`, `os.Getenv`, `os.LookupEnv`, by built-in or import identity |
| `config` | a call into an import-proven configuration or settings library, or a read of a file-parsing call the tables list as configuration |
| `constant` | a literal |
| `unknown` | anything else; no finding above `info` |

A value computed from another variable takes that variable's origin, one assignment step at a time, within
the function, to a fixed depth. A name whose binding is not proven (shadowed, rebound, imported from a
repository-local module of the same name) is `unknown`.

### 5.3 Other languages

The existing tables stay, with five structural fixes that apply to every language: variables only; scoped
definition lookup; the defining statement's text, not any line of the same name; no text inside comments or
strings; tables gated by language; word boundaries on both sides. The fallback that hands a line's class to
every identifier on it is removed. It is today the only route by which some forms are classified at all
(shell `read x` and positional parameters); stage D gives each such form a defining-statement rule or lists
it as an accepted loss.

### 5.4 Shared identity

The import-identity proof and library tables are used from where they are, made visible within the review
crate. Lane 2 extracts a call-level entry point from the same code (`root_identity`); this lane uses that
entry point and adds tables only for classes it lacks (configuration libraries, environment reads,
command-line sources). That entry point proves call nodes only; environment and command-line reads are
subscripts and member reads, so this lane adds a sibling for non-call paths. The proof's site context needs
a repository root, which a review run does not always have; the root becomes optional there, and the
local-module check is skipped, never run against the working directory. The request-read recogniser is exposed from the taint module by a function, without
moving it. If lane 2 has not merged, this lane's first slice carries the same small extraction, and the two
lanes are told so.

### 5.5 Output

Category, severity mapping and description format are unchanged, so the targets projection and SARIF keep
working. The targets hint for a database origin is attached only when the origin was proven by library
identity. The projection sees only the description, so the rule is applied by language: Python,
JavaScript/TypeScript and Go never fall back to patterns, so a database or external-call origin there is
always identity-proven, and other languages get no hint; `external_call` origins get the `network` hint that the mapping already defines. The number of
findings changes; that is the point, and it is measured.

### 5.6 Where the code goes

```
crates/prism-review/src/algorithms/provenance_slice.rs      entry point; delegates
crates/prism-review/src/algorithms/provenance/mod.rs        variable selection, reaching definitions, origin join
crates/prism-review/src/algorithms/provenance/patterns.rs   the tables, gated and with two-sided boundaries
crates/prism-review/src/algorithms/provenance/identity.rs   origin by proof, shared across the three language families
crates/prism-review/src/algorithms/provenance/{python,js_ts,go}.rs   per-language defining-expression readers and tables
```

## 6. Measurement

**Population.** Real diffs, so that volume reflects use: the 120-diff population already used for issue
#602 (six repositories with full history), plus 60 Python-heavy diffs drawn the same way from two Python
corpora. Provenance alone is run on each diff with the base and the head build.

**Labelled sample.** From the base build's findings at `concern` and `warning`, 300 are sampled, stratified
by language and origin, and labelled by two readers independently (right origin, wrong origin, not a
variable); disagreements are resolved by a third read. The same is done for 200 of the head build's
findings. The labelled true positives of the base sample form the recall set.

**Cross-check.** Every line that the taint engine recognises as a framework request read in the population
must yield a `user_input` finding for the variable it defines, when that variable is used on a diff line.

## 7. Gates

| Gate | Threshold (default) |
|---|---|
| Census go | on the base sample, precision at `concern` and `warning` is below 60% (otherwise the premise is wrong: stop and report) |
| Precision | on the head sample, at least 85% right at `concern` and `warning`, per language family with at least 30 sampled findings |
| Recall | at least 95% of the recall set is still reported with the same origin; each loss is listed with its reason |
| Cross-check | section 6's cross-check holds with no exception that is not listed and explained |
| Hints | no targets hint on a row whose origin was not proven by library identity |
| Contract | the description format, category and severities are unchanged; the targets mapping tests pass unmodified except where a fixture's expectation was a substring artefact, each named in the PR |

## 8. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Remove provenance from the default review suite and default targets set until the lane lands | no; defaults unchanged | one line each way | any time |
| 2 | Findings for names that are not variables | none (they are dropped and counted) | — | stage D |
| 3 | `info`-level findings (parameter, environment, configuration, constant) | still emitted, now only when proven | suppressing them is a severity-floor choice the user already has | — |
| 4 | Languages with identity rules | Python, JavaScript/TypeScript, Go; others get the structural fixes only | each further language is its own slice | stage D |
| 5 | Precision and recall thresholds | section 7 | none | stage G |

## 9. Risks and accepted limits

- **Recall can fall where identity is not proven** (a request object passed through a helper, a database
  handle built by a factory). Those become `unknown`; the recall gate and the listed losses make the cost
  visible, and the handle-hop depth is a parameter the matrix fixes.
- **Tests that encoded substring behaviour change.** Each is named in the PR with the reason; none is
  deleted without a replacement that states the intended behaviour.
- **The labelled sample is small and human-judged.** Two readers and stratification reduce, not remove,
  that; the cross-check is the mechanical part.
- **Target ids change once.** The lines a provenance finding relates to change, and target ids hash them.
- **The real-diff population is Python, JavaScript/TypeScript and Go.** The structural fixes for other
  languages are checked by matrix rows only.
- **The targets module is lane 2's.** This lane's edits there are additive (visibility, an optional field),
  alter no existing result, and rebase over lane 2's first slice.
- **Out of model:** origins that cross files, values assembled through containers, reflection.
