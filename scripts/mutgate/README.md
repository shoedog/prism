# mutgate: authoritative text gate and advisory scoped checks

Before merge, run the full gate. Every authoritative verdict comes from an
isolated text mutant containing only its own main and `extra` edits. Source
baseline preflight must pass first. Schema mode never decides an authoritative
verdict. Production `src/` is never edited.

For each review round, use a fast advisory scoped run, or add `--authoritative`
when that round needs certainty about the selected mutants.

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
python3 scripts/mutgate/mutgate.py                              # authoritative full gate, pre-merge
python3 scripts/mutgate/mutgate.py --jobs 2                     # two isolated text workers
python3 scripts/mutgate/mutgate.py --since main --scope fn       # ADVISORY scoped round
python3 scripts/mutgate/mutgate.py --since main --scope line     # ADVISORY overlapping anchors
python3 scripts/mutgate/mutgate.py --since main --scope file     # ADVISORY changed files
python3 scripts/mutgate/mutgate.py --since main --scope fn --authoritative
python3 scripts/mutgate/mutgate.py --lane mutants/lane-p-tsconfig-paths.json --only I17-no-project-reference-cut
python3 scripts/mutgate/mutgate.py --plan-only                   # planner diagnostics, no verdicts/build
python3 -m unittest                                            # all driver regressions
```

`--jobs N` sets the number of isolated text workers and parallel baseline/schema
test runs (default **4**, positive integer). `--timeout SECS` limits each test
(default 120). `--out DIR` overrides the report directory. `--mode text` is an
alias for authoritative evaluation; `--mode schema` requires `--since` and
cannot be combined with `--authoritative`. `--lane` and `--only` restrict the
population, so a partial selection does not certify the full lane.

## Soundness contracts and exit codes

**Authoritative full gate:** no `--since` means pure text mode. Every mutant is
built separately from original source with only its own edits, with a private
scratch tree and private `CARGO_TARGET_DIR`. No schema bodies or helper are
injected, and no lexical observer detector is used to accept a verdict. An
unmutated-source build and preflight of every selector precede mutation, both
in the shared warm tree and each worker's own tree/target. Worker preflight
refuses relocation-induced test or compile failures, including observations
of `CARGO_MANIFEST_DIR`, rather than counting them as mutation kills.

**Advisory scoped mode:** `--since REV --scope line|fn|file` defaults to schema
mode. Eligible function-body edits are compiled together as `match` arms and
activated via `PRISM_MUTANT`. Output lines say **ADVISORY**, and summary JSON
contains `"authoritative": false`. Schema rendering changes source locations
and execution overhead. Lexical demotions and targeted text confirmations
remain useful heuristics, but cannot establish sound verdicts: macro-generated
helpers, aliases, location Debug output and caught payloads can falsely kill
equivalent mutants. Six compiled fixtures retain these counterexamples.
Even text fallbacks/confirmations do not make a schema run authoritative.

**Authoritative scoped mode:** add `--authoritative` (or `--mode text`) to
apply the text contract to the selected population. This gives certainty about
those registered mutants, not about all mutants affected by the diff.

Both modes exit **0** when every selected verdict is admissible and KILLED;
SURVIVED, TIMEOUT and INADMISSIBLE exit **1**. Advisory exit 0 is only an
advisory result. Every selector must run exactly one non-ignored test; all
selectors must be admissible and at least one must fail on the mutant. Timeouts
never count as kills. A red, missing, ignored or timed-out source baseline
makes its mutants INADMISSIBLE, preserving the selected denominator. Advisory
mode also preflights inactive schemas. A zero selection is explicitly reported
as `selected: 0`; it provides no mutation coverage.

Scopes are local filters: line scope selects overlapping anchors, fn scope
the enclosing function, and file scope all mutants in changed files. They do
not trace callers/dependencies and can omit effects of helper changes. Missing
registered paths/anchors are selected even outside the diff and fail visibly.
A green scoped run cannot replace the full pre-merge gate.

## Build state, resources and cleanup

The shared warm build lives in `$CARGO_TARGET_DIR` (default `./target`); its
baseline scratch tree is `mutgate/tree`. Each text worker seeds a private target
once from the shared warm target using APFS `cp -cR` when available, otherwise
real copies. Mutable Cargo artifacts are never hard-linked. Workers restore
source between mutants; their trees and targets are deleted after execution,
including handled failures. Only the shared warm cache and reports remain.
Disk grows with the active worker count rather than the number of mutants.

The default summary is `$CARGO_TARGET_DIR/mutgate/report/summary.json`, containing
mode, authority, selected IDs, baselines, verdicts, timings, seed methods and
sampled peak worker disk. `du` counts allocated blocks and can count shared APFS
clone extents repeatedly; it does not measure unique physical storage. Remove
the shared target when its warm cache is no longer needed; save reports first.

The warm lane-P target is **at most five minutes at four workers**. Measured
round-3 timings, peak disk/RSS, exact ID comparison against r2 pure text and
full test results are recorded in [repair-r3 evidence](evidence/repair-r3/README.md).
Earlier schema timings are historical advisory measurements, not a sound full
gate certification.

## Advisory planner limits

Edits outside function bodies, functions returning `impl Trait`, direct location
observers and unaudited macros demote to isolated text. Failed schema compilation
also demotes implicated units (three-build cap). Local macro expansion scans,
helper/constant test scans and failure payload scans remain lexical heuristics;
aliases, dynamic calls, external helpers and generated code can escape them.
The full authoritative gate does not depend on these protections. Performance
or memory-sensitive tests need a dedicated resource driver rather than schema
instrumentation (see `mutants.py`).
