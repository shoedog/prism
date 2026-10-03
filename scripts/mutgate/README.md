# mutgate: generated function-body mutant schemata

A pre-merge mutation gate. Eligible mutants become `match` arms inside their
enclosing function in a scratch copy, built together and selected with
`PRISM_MUTANT`. Location-sensitive schema failures require a clean text-mode
confirmation; other admissible schema kills stand directly. Production `src/`
is never edited.

## Defining a lane

Each `mutants/<lane>.json` has a flat `mutations` map:

```json
{
  "mutations": {
    "ID-slug": ["src/file.rs", "<original snippet>", "<mutated snippet>", "module::path::test_fn"]
  },
  "extra": {
    "ID-slug": [["src/other_file.rs", "<original>", "<mutated>"]]
  },
  "lib_test_prefixes": ["js_paths_snapshot::", "repo_loader::"]
}
```

- `mutations[id]` is `[file, original_snippet, mutated_snippet, test]`. `test`
  may be a single selector string or a nonempty list. Every selector must
  execute exactly one non-ignored test and pass on the unmutated baseline;
  at least one must fail on the mutant, and every mutant run must complete
  admissibly. Anchors are exact snippets; all occurrences are replaced.
- `extra[id]` lists additional `[file, original, mutated]` edits applied
  alongside the main one (e.g. a struct field plus its initializer).
- `lib_test_prefixes` routes a selector to the `--lib` test binary when its
  name starts with one of these prefixes; everything else runs against the
  lane's `--test integration` binary. There is no per-mutant target override.
- `intent_revisions[id]` records a deliberate change to a historical mutant's
  obligation and its reason. R4-04 retains its ID but now disables UTF-16 BOM
  decoding, with the existing native-decoding fixture, because r5's lossy
  decoding removed the former non-UTF-8-declines obligation.

## Commands

```bash
python3 scripts/mutgate/mutgate.py                      # full gate, every lane file
python3 scripts/mutgate/mutgate.py --since main                       # scoped: line-level
python3 scripts/mutgate/mutgate.py --since main --scope fn            # scoped: whole enclosing fn
python3 scripts/mutgate/mutgate.py --since main --scope file          # scoped: all mutants in changed files
python3 scripts/mutgate/mutgate.py --lane mutants/lane-p-tsconfig-paths.json --only I17-no-project-reference-cut
python3 scripts/mutgate/mutgate.py --plan-only             # print the schema plan, build nothing
```

Other flags: `--jobs N` (parallel test runs, default half the CPU count),
`--mode schema|text` (force text-mode, skipping schema rendering),
`--timeout SECS` (per-test, default 120; a timeout fails the gate),
`--out DIR` (summary location override).

## Exit codes and output

- Exit `0` only if every selected mutant is admissible and KILLED. SURVIVED,
  TIMEOUT and INADMISSIBLE each fail the gate. Timeouts never count as kills;
  this driver provides no timeout exemptions.
- Both modes compile unmutated source and preflight every selected selector.
  Schema mode also preflights the inactive schema tree. A red, missing,
  ignored or timed-out baseline makes that selector's mutants INADMISSIBLE.
- The summary (per-mutant verdict, timings, mode) is written to
  `$CARGO_TARGET_DIR/mutgate/report/summary.json` (default
  `./target/mutgate/report/summary.json`).
- Build state lives under `$CARGO_TARGET_DIR/mutgate/` (the scratch tree at
  `mutgate/tree`). Cargo artifacts are reused in `$CARGO_TARGET_DIR`; remove
  that build directory to reclaim them. Each text mutant starts from original
  source with only its own main and `extra` edits; no inactive schemas remain.

## Tiers

- Scoped round (`--since <ref>`): a local filter for one review round. Target
  well under 5 minutes. Line/fn scope selects overlapping anchors/functions;
  file scope selects all mutants in changed files. These do not trace callers
  or dependencies and can omit mutants affected by helper changes. Missing
  registry paths/anchors are selected even outside the diff, to fail visibly.
- Full gate: every lane mutant, at most 15 minutes, ideally 2–3 minutes with
  reusable Cargo artifacts. The full gate is the
  authority before merge; a green scoped result cannot replace it.

Round-2 certification of the current 93-ID lane: all 93 admissible kills in
both modes, zero ID-bound verdict differences; schema **90.00 s**, pure text
**513.64 s**, and a nonempty three-mutant synthetic fn-scope control **33.50 s**.
Of the 93, 89 ran as schemas, four demoted for struct-field edits, and zero
schema kills required text confirmation. See the [per-ID certification table](evidence/repair-r2/certification-table.md)
and [demotion table](evidence/repair-r2/demotion-table.md). These timings are
machine/cache dependent; the scoped measurement injects changed lines because
the production source diff is empty.

## Limits

- A mutant whose edit falls outside any function body, or whose unit fails
  to compile as a schema, demotes automatically to a one-off incremental
  text-mode build; this is normal, not an error.
- A function returning `impl Trait` is excluded from schema rendering
  (duplicating its body across match arms can fail RPIT's single-hidden-type
  inference when the body contains a closure or other anonymous-type
  expression) and runs any mutant touching it in text mode instead.
- Both original and mutated bodies demote for direct `line!`, `column!`,
  `file!`, `std::panic::Location`, `Location::caller`, and `#[track_caller]`
  (on the function or nested callees). `module_path!` is conservatively treated
  as an observer, as is `dbg!`, which prints a location. Plain `panic!`,
  `unwrap` and `expect` do not themselves make a body location-sensitive.
- The safe std macro allowlist is `vec`, `format`, `format_args`,
  `format_args_nl`, `write`, `writeln`, `print`, `println`, `eprint`, `eprintln`,
  `assert`, `assert_eq`, `assert_ne`, `debug_assert`, `debug_assert_eq`,
  `debug_assert_ne`, `matches`, `panic`, `unreachable`, `todo`, `unimplemented`,
  `concat`, `stringify`, `cfg`, `env`, `option_env`, `include_str`, `include_bytes`.
  Arguments are still scanned for observers. Local `macro_rules!` definitions
  are scanned across the crate, including transitive expansions and std-name
  shadowing; a known-safe local wrapper remains in schema mode. Same-name
  definitions are conservatively merged rather than resolving Rust scopes.
  New/changed nested definitions are inspected in the mutant too.
- The external safe list contains only `serde_json::json` (JSON construction
  without location APIs). All other external macros, including `anyhow::{anyhow,
  bail, ensure}`, `log::*`, `tracing::*`, and `tokio::*`, plus unresolved imported
  macros and `include!`, demote as `unaudited-macro`. Inspect an expansion before
  adding it to the safe list; recognition alone is insufficient.
- A schema kill needs text confirmation only when its **failure payload** has
  a numeric `.rs:line[:column]` location in one of that mutant's edited files,
  or its killing test uses location APIs or `should_panic(expected="… .rs: …")`.
  Rust's automatic panic diagnostic header is excluded: it reports provenance,
  rather than proving the test observed it. The full output is scanned before
  tail truncation. The test scan includes referenced constants and helpers
  (for example `AFTER = line!()`); same-name items are conservatively merged.
  The summary records confirmation counts and reasons, and preserves the
  schema observation separately from the confirmed verdict.
- These are lexical protections, not a Rust resolver or a general equivalence
  proof. Aliased location APIs, dynamic calls, proc/attribute macro expansions,
  external helpers or hooks that observe locations without exposing them in
  failure text can escape the targeted checks. Keep such mutants/tests in text
  mode. Each new lane must be certified by ID against pure text mode; this does
  not certify arbitrary future tests or external expansions.
- No perf or memory mutants: the schema's per-call string match on
  `PRISM_MUTANT` changes timing and allocation behavior, so this gate is
  unsuitable for timing- or memory-sensitive assertions. Keep those on their
  own driver (see `mutants.py` for the kernel lane's resource mutants).
