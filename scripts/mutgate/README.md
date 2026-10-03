# mutgate: generated function-body mutant schemata

A pre-merge mutation gate. Eligible mutants become `match` arms inside their
enclosing function in a scratch copy, built together and selected with
`PRISM_MUTANT`. Every schema kill must also kill in a clean text-mode build
before the gate reports KILLED. Production `src/` is never edited.

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
  under 5–10 minutes. Line/fn scope selects overlapping anchors/functions;
  file scope selects all mutants in changed files. These do not trace callers
  or dependencies and can omit mutants affected by helper changes. Missing
  registry paths/anchors are selected even outside the diff, to fail visibly.
- Full gate: every lane mutant, at most 15 minutes. The full gate is the
  authority before merge; a green scoped result cannot replace it.

## Limits

- A mutant whose edit falls outside any function body, or whose unit fails
  to compile as a schema, demotes automatically to a one-off incremental
  text-mode build; this is normal, not an error.
- A function returning `impl Trait` is excluded from schema rendering
  (duplicating its body across match arms can fail RPIT's single-hidden-type
  inference when the body contains a closure or other anonymous-type
  expression) and runs any mutant touching it in text mode instead.
- Bodies containing macro invocations (including unknown wrappers), location
  queries, `unwrap`/`expect`, or a `#[track_caller]` attribute stay in text mode.
  Both original and mutated bodies are checked. Text mode preserves the
  locations of the ordinary text edit; duplication cannot preserve them.
  Observations hidden behind calls or expansions elsewhere are protected by
  mandatory text confirmation. Schema failures awaiting confirmation are
  observations, not accepted kills; the summary retains them separately.
- No perf or memory mutants: the schema's per-call string match on
  `PRISM_MUTANT` changes timing and allocation behavior, so this gate is
  unsuitable for timing- or memory-sensitive assertions. Keep those on their
  own driver (see `mutants.py` for the kernel lane's resource mutants).
