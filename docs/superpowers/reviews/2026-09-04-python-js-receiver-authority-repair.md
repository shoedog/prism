# Receiver authority repair: audit and verification

Control: fetched `origin/main` = `10d82ca58387f030a863f75cb6f83ec2f1b9c662`.
Checkout `ea2965e0237335a1c9c5c147e3aee9168e5bb84b` had the identical tracked tree.
Verification ran with local uncommitted changes on `docs/python-js-nav-sequence-handoff`.
The owner subsequently authorized publication on `fix/python-js-receiver-authority`;
implementation `9a790419` is pushed and [PR #238](https://github.com/shoedog/prism/pull/238)
is open. Current custody is recorded in the matching handoff. All seven source/test
files matched the full-suite snapshot at publication; a fresh release/matrix repeat
passed 104/104. Publication changed documentation only after those full suites.

## Demonstrated WRONG findings and bounded repairs

| Failure population | Incorrect base result | Repair |
|---|---|---|
| JS enclosing constructor parameter/block, named class self (3) | Exact module `Foo.m`, though `new Foo` refers to another binding | Inspect enclosing callable bindings and class-expression self names |
| JS conditional `var`, iteration assignment, loop-carried write (3) | Exact `Foo.m` without an initialized/surviving constructor origin | Structured initialization proof, iteration writes, back-edge write window |
| Python current/enclosing constructor parameter, imported alias closure (3) | Exact local/imported `Foo.m` despite the shadowing binding | Current/enclosing function owner-root fence |
| Python conditional assignment, constructor/typed loop-carried writes (3) | Exact `Foo.m` despite uninitialized/reassigned receiver | Structured initialization proof and loop prefix/back-edge binding check |
| TS and TSX generic, enclosing generic/interface/alias/class, class-expression self (12) | Exact module `Foo.m` for an unrelated type spelling | Separate lexical type-name proof at the annotation |

Total: 24 concrete behavioral cases, across three accumulated regression tests.
Wrong edges are identified by target file/name/line, Exact confidence, and
ConstructorLocal or TypedParam kind. This rules out R6 NameOnly as their cause.
No unresolved bounded WRONG or SMELL was retained after round three. This is a
self-review, not an independent review or a claim of complete language semantics.

## Hypothesis, probe, result

1. Hypothesis: constructor spelling and source order are accepted without owner
   visibility or execution proof. Falsifier: shadow/conditional examples remain
   unrecovered. Probe: full `CallGraph::build` plus `resolve_call_site` matrices.
   Result: six JS and six Python cases retain wrong recovered metadata and Exact
   edges on base. Type-name equivalent: twelve TS/TSX cases reproduce TypedParam.
2. Alternative: correct extraction is being lost at downstream resolution.
   Discriminator: inspect stored receiver_type and recovered edge kind together.
   Result: wrong type evidence already exists at classification; fix the producer
   and invalidate cached metadata/topology.
3. First JS probe was inadmissible for Exact-edge evidence: same-name methods on
   one line collided in FunctionId. Distinct-line fixture reproduced all initial
   five Exact edges; the faulty probe was not used for the conclusion.
4. An intermediate patch loosened two existing Python qualified-annotation
   barriers and changed unsupported module-interface metadata. Exact-base controls
   passed (Python 5/5, TypeScript 1/1). Restore those compatibility contracts while
   adding the new enclosing/type-namespace fences; tests were not re-baselined.
5. Round two's named-class-self and Python loop REDs were repaired on the same
   artifact. Round three checked the final producer/consumer diff, reset and
   ended-scope controls, and full/subset/cache/incremental parity. Cap: three.

## Reproducible control

Archive `git archive 10d82ca...` into
`/private/tmp/prism-receiver-authority-y5TSfg/base`. Add only the new test modules.
Production prefixes of `src/ast.rs`, `src/resolution.rs`, `src/cpg_cache.rs`, and
`src/navigation/call_edge_cache.rs` were read back and compared with `git show`;
all four matched exactly. Test-module updates used the same source fixtures as
the candidate. Run with the same Cargo target directory and shell environment:

```text
CARGO_TARGET_DIR=/Users/wesleyjinks/code/slicing/target cargo test \
  --lib --test lang_python --test lang_javascript --test lang_typescript \
  receiver_ --no-fail-fast
```

`base-red.log`: library 111 passed/4 failed; JS 10/1; Python 52/1; TS 17/1.
Total 190 passed/7 failed. Failures are the three behavioral matrices, two version
pins, CPG cache/subset/incremental parity, and navigation sidecar behavior. All
preservation controls pass on base. This control is production-source identity,
not a Git checkout or a publishable artifact.

## Candidate verification

| Gate | Observed result | Evidence under `/private/tmp/prism-receiver-authority-y5TSfg/` |
|---|---|---|
| Full default `cargo test --no-fail-fast` | 3,713 passed / 0 failed / 1 ignored; 28 summaries | `default.log` |
| Full MCP `cargo test --features mcp --no-fail-fast` | 3,903 passed / 0 failed / 1 ignored; 30 summaries | `mcp.log` |
| Format / whitespace | PASS | `cargo fmt --all -- --check`; `git diff --check` |
| All-target MCP check | PASS, warnings emitted | `check.log` |
| Configured Clippy | PASS, warnings emitted | `clippy.log` |
| Release then Tier-A matrix | 104 ok / 0 regressions | `release-matrix.log`, `matrix.log` |
| Immediate release then Tier-A quick | Completed, exit 2: sole invalid reason corpus pin drift; oracle/SUT errors 0.000/0.000; matrix 104/104 | `release-quick.log`, `tier-a/run.json`, `tier-a/report.md` |

Exact quick invalid reason: `corpus_sha_drift: ea2965e02373 != pinned 20c8490591a3`.
The oracle was quiescent; the report records four stale adjudications. No pin,
baseline, or adjudication was changed. This is not a valid comparative corpus
acceptance result, and sampled metrics are not attributed to this change without
a paired corpus control. The generated run/report/snapshot were moved intact to
the evidence directory's `tier-a/` subdirectory; original retained files were not moved.

The ignored test is the pre-existing reserved `resolution_test::slice_elem_variant_reserved`.
Cache versions advance CPG 59→60 and navigation 28→29. Real CPG save/load,
direct-subset extraction, both incremental shadow transitions, and served
navigation-sidecar save/load test all four languages, including positive Exact
controls and negative absence. The old CPG version is rejected explicitly.

No full multi-corpus run was requested or run. These gates establish the bounded
behavior and compatibility, not corpus-wide Python/JS precision or recall.
General interprocedural writes, runtime mutation, structural dispatch, and new
cross-module JS/TS receiver recovery are outside this repair contract.

## Custody

Local recovery snapshots (not publication):

- RED: `red-checkpoint.tgz`, SHA-256
  `39c5c2828ba931a29e3ae6b4d8871ebf92a06a17cd023b6e2e533cc807b7ca97`.
- Implementation: `implementation-checkpoint.tgz`, SHA-256
  `ff0a0f4daf7bf21643f4dbb98ecf9d75bb78267a5720d400b4aa3d6e44bc14f4`.

The owner follow-up explicitly authorizes commit, push and PR creation. The root's original
`.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain preserved.
Final source/docs snapshot: `final-source.tgz`; complete verification capture:
`verification-logs.tgz`, both under the same evidence directory. SHA-256 values
are emitted by the final snapshot command in the session record.
