# Offline SecBench measurement

Run from the repository root with the already acquired main binary. Its SHA256 must match the committed pin; harness HEAD and SUT HEAD are separate.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m eval.secbench --no-build   --binary /path/to/pinned/prism --out /path/to/new/run
```

The command inspects all 583 acquired packages, authenticates package/test-input/metadata bytes against input-pins.json, measures Prism, and emits 600 rows including the 17 acquisition exclusions. No corpus package, test input, server, callback or dependency is executed. Python uses the standard library; Node and the on-disk pinned TypeScript 5.9.3 compiler are required. Input test files are only read; do not create or extend their data values.

PRISM_EVIDENCE_ROOT and PRISM_TYPESCRIPT parameterize host defaults. CLI options include --inputs, --packages, --compiler, --out, --sut-repo, --binary, --no-build, --inspection, --workers and --timeout. --binary requires --no-build. Without --binary, --sut-repo must identify the exact main SUT checkout and an offline release build must reproduce the binary hash. Never silently refresh SUT, compiler, corpus or label pins. Use a new output directory for each run.

GT supports uniquely bound exported APIs and registered HTTP request formals. The conservative lexical resolver refuses shadowed/reassigned bindings; a separate pinned TypeScript checker accepts only one in-package callable signature. Setup constructor/configuration inputs and callback bodies receive no data credit. A function whose separately assigned metadata field is consumed as data has an explicit data role. HTTP mode ties the existing request test to a package bootstrap/API registration and a unique handler containing/reaching the declared target call. It seeds request parameter 0 directly, bypassing runtime ingress. Raw and normalized-handler clone counts are separate.

Target values retain their own occurrence lines, complete member/this paths and byte spans, alongside original metadata coordinates. Empty constant-only targets are unavailable. Every selected occurrence line is queried. Credit requires an exact formal Def and consumed target Use in the per-root witness; a synthetic zero-width Use is accepted only when the complete path has one same-name AST occurrence on that line. Line seeds may include unrelated roots; their witnesses and frontier reasons receive no data credit. Whole-object parameter seeds do not establish field-selective source soundness.

Outcome and mechanism are separate. Failed decisive witness/frontier/callees queries remain prism_error. Every error gets error_mechanism, with errors included in workstream opportunity counts only in an explicit sensitivity table. No selected Def means the break is at source binding. Downstream syntax screens remain hypotheses. B-destructure, B-rest-spread, B-default, B-arguments and ordinary B-plain-argument transfer are separate; plain transfer has no ws2 credit and default syntax alone is not a failure.

Nav queries share one package-local cache under the output directory. The default budget is 120 seconds per query, with two Rayon threads; each package cache is removed when its observations finish. Classic taint/chop are comparison channels and may still rebuild. Query timeouts are retained separately from capability failures.

Replay authenticates decisive raw bytes and the complete input population. It requires the same requested source and every target occurrence line; changing an endpoint line requires new observations. Historical R0 labels are optional, revision-bound and never automatically applied to the R1 mechanisms.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m eval.secbench.replay   --run /path/to/run --inspection /path/to/run/inspection.jsonl --out /path/to/replay
```

R1 follow-ups (use fresh paths):

```sh
python3 -m eval.secbench.conversions --run /path/to/run --out /path/to/conversions --binary /path/to/pinned/prism
python3 -m eval.secbench.report --run /path/to/run --baseline /path/to/R0/final --conversions /path/to/conversions --out /path/to/report
python3 -m eval.secbench.blind --run /path/to/run --sample /path/to/blind-sample.jsonl --reference /path/to/private-reference.jsonl
python3 -m eval.secbench.blind --reference /path/to/private-reference.jsonl --labels /path/to/independent-labels.jsonl
```

Conversions copy package source into scratch space, retain an unmodified control and patches, and re-query authenticated formal/target identities. Bare reads preserve member values; callback hoists use a pure identity call to preserve anonymous names and lexical scope; rest/legacy wrappers retain the engine-created array/object, signature, arity, lexical this and return. Reflective/ambiguous scopes are refused. Legacy normalization refuses multiple source parameters to avoid broadening input identity. Admission plans change only AST-identified local imports and the package entry locator, refuse observed runtime paths, and retain distributed originals. A maximum of four distinct barriers is recorded. Single-workstream and joint conversions, unresolved hypotheses and refusals stay separate. These are source counterfactuals, not shipped product changes or proof of runtime behavior.

The 40-entry blind sample has eight per class, with deterministic failure/exclusion/trace strata. It contains source/target and raw Prism excerpts without derived labels. Hide the separate reference file from independent labellers. Kappa requires later independent labels; this enriched sample cannot estimate population prevalence.

Tests:

```sh
TMPDIR=/private/tmp node --test eval/secbench/inspect.test.mjs eval/secbench/rewrite.test.mjs
TMPDIR=/private/tmp PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s eval/secbench -t .
```

Full project verification also runs cargo test --offline and the complete deterministic eval pytest suite. Live provider evaluations remain explicitly gated and are not requested by this lane. Historical R0 results remain in the original packet; R1 results are documented in docs/superpowers/plans/2026-10-04-secbench-ground-truth/R1.md.
