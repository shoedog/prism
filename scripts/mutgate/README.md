# mutgate: generated function-body mutant schemata

A pre-merge mutation gate. It applies every lane's mutants as `match` arms
inside the enclosing function body, in a scratch copy of the tree, builds
once, and runs each mutant's test with `PRISM_MUTANT` set to pick its arm.
Production `src/` is never edited.

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
  may be a single selector string or a list of selectors (all must kill it).
  `original_snippet` must be the unique, exact text of the anchor in `file`.
- `extra[id]` lists additional `[file, original, mutated]` edits applied
  alongside the main one (e.g. a struct field plus its initializer).
- `lib_test_prefixes` routes a selector to the `--lib` test binary when its
  name starts with one of these prefixes; everything else runs against the
  lane's `--test integration` binary (override per-mutant with
  `"test_target": "lib"` next to that mutation if a single lane mixes both
  in ways the prefix list can't express).

## Commands

```bash
python3 scripts/mutgate/mutgate.py                      # full gate, every lane file
python3 scripts/mutgate/mutgate.py --since main                       # scoped: line-level
python3 scripts/mutgate/mutgate.py --since main --scope fn            # scoped: whole enclosing fn
python3 scripts/mutgate/mutgate.py --lane mutants/lane-p-tsconfig-paths.json --only I17-no-project-reference-cut
python3 scripts/mutgate/mutgate.py --plan-only             # print the schema plan, build nothing
```

Other flags: `--jobs N` (parallel test runs, default half the CPU count),
`--mode schema|text` (force text-mode, skipping schema rendering),
`--timeout SECS` (per-test, default 120; a timeout counts as killed),
`--out DIR` (summary location override).

## Exit codes and output

- Exit `0` only if every selected mutant is killed; exit `1` otherwise.
- The summary (per-mutant verdict, timings, mode) is written to
  `$CARGO_TARGET_DIR/mutgate/report/summary.json` (default
  `./target/mutgate/report/summary.json`).
- Build state lives under `$CARGO_TARGET_DIR/mutgate/` (the scratch tree at
  `mutgate/tree`, reused across rounds for incremental builds). Reclaim disk
  with `rm -rf $CARGO_TARGET_DIR/mutgate`.

## Tiers

- Scoped round (`--since <ref>`): seconds, for one review round. Target
  5–10 minutes.
- Full gate: every lane mutant. Target 15 minutes or less, hard cap 30
  minutes, before merge.

## Limits

- A mutant whose edit falls outside any function body, or whose unit fails
  to compile as a schema, demotes automatically to a one-off incremental
  text-mode build; this is normal, not an error.
- A function returning `impl Trait` is excluded from schema rendering
  (duplicating its body across match arms can fail RPIT's single-hidden-type
  inference when the body contains a closure or other anonymous-type
  expression) and runs any mutant touching it in text mode instead.
- No perf or memory mutants: the schema's per-call string match on
  `PRISM_MUTANT` changes timing and allocation behavior, so this gate is
  unsuitable for timing- or memory-sensitive assertions. Keep those on their
  own driver (see `mutants.py` for the kernel lane's resource mutants).
