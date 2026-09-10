# Bounded TypeScript erased-rvalue repair

Base: PR305 merge `70caaf6d43f9bbd1ecf47279af0db05ce02753f4`.
Reviewed and tested source: `0d71800bb7f481a4334667767ca80416ed33ce49`.
Subsequent closeout changes are documentation-only.

## Result and proof boundary

Supported erased import-type options no longer leak runtime identifier/path/span
occurrences through shared rvalue collectors, their condition/return consumers,
or the semantic-value-use classifier. For example:

```typescript
function owner() {
  type T = typeof import("./m", { with: { "resolution-mode": "import" } });
}
```

Before the fix, the single-line equivalent emits the erased `with` at `[51,55)`
through all three rvalue APIs. The captured base RED demonstrates the leakage;
the pinned compiler erases the entire type alias. The preceding call-name guard
was insufficient: call argument collection and enclosing value recursion enter
these subtrees independently.

The repair shares the existing TS/TSX type-context boundary with call extraction.
Each collector checks ancestry once at entry, then prunes local boundaries during
recursive descent. The semantic-use classifier checks those boundaries during its
existing ancestor walk. `as`/`satisfies` retain their runtime side using the actual
operator's byte boundary, including comments. No grammar, cache format, raw source
or tree changes. Existing source-byte cache identity covers the changed code.

Prism navigation supplied consumer hints but reported StaleIndex. Current-source
tracing confirmed DFG, reaching-definition, condition and semantic-use consumers;
this is not an exhaustive LSP references claim. That tracing drove the shared
collector repair rather than a call-query-only fix.

## Captured regressions and compiler authority

Same-worktree base TS suite: **88 passed**. Complete pre-fix RED: **2 private
failures, 9 integration failures, 1 runtime positive passed**. Focused GREEN:
**2 private +10 integration passed**; full TS suite now includes the existing88
and10 new integration tests. Initial bad-probe output is retained, not counted as
behavioral evidence: a const object declaration was an invalid positive control
for an API that recognizes assignment expressions, not variable declarators.
The corrected control uses an assignment; declaration behavior was not expanded.
Invalid cargo-filter and private helper compile attempts are also inadmissible.

Tests cover all three public APIs; selected lines; assignment/call/return/condition
nesting; direct erased and interior collector roots; manual/query parity; full
and subset DFG use maps; semantic-use refusal; TS/TSX; TS-only angle assertions;
commented casts; and mixed same-line erased/real options with UTF-8 offsets.
JS comparison controls preserve dynamic import, runtime `typeof import`, ordinary
object labels, runtime names/paths/spans and call names.

Pinned TypeScript5.9.3 validates **55 positive TS/TSX checks**, with **one expected
TSX angle-assertion rejection**, zero unexpected failures and zero skips. The
population is27 test fixture definitions plus one separately labeled compound
member example (28 definitions,27 unique bodies), checked in both dialects.
Strict ES2022/ESNext, JSX preserve, empty automatic types, a recorded ambient
prelude and virtual `./m` exporting `class X` establish fixture validity, not
real-project semantic completeness. The receipt binds exact source hashes,
UTF-16/UTF-8 compiler spans, diagnostics and emitted JavaScript. Erased options
disappear; runtime import counts and exact options text survive. Emitted code
was inspected, never executed.

## Verification and limitations

| Gate | Result |
|---|---|
| Rust default / MCP / MCP+detached-owner-audit | 4,095 /4,288 /4,311 passed; one ignored each |
| Observers / receiver helpers / authority profiles | 726 /18 /40 passed; no skips |
| Python evaluation suite | 940 passed; one deliberate live-adoption skip |
| Membership example | 12 passed |
| Format / diff / Clippy | pass /pass /completed with warnings, not warning-free |
| Tier-A matrix | 159 passed, zero regressions |
| Independent review | round1 of cap2 APPROVE; WRONG0, actionable SMELL0 |

Rust's ignore is `resolution_test::slice_elem_variant_reserved`. Python's skip
requires explicit `PRISM_RUN_LIVE_EVALS=1`; live-model evaluation was not enabled.
Full multi-corpus and real receiver-population/recall measurements were not run.

The eight-gate runner's aggregate status is **FAILED**, solely because its final
clean-check saw the concurrently generated Tier-A JSON/Markdown reports. Every
substantive gate passed and HEAD was unchanged. Both reports were moved into the
evidence archive; a separate `git diff --exit-code 0d71800b`, clean porcelain status
and exact HEAD check passed. `postflight-reconciliation.json` preserves this
correction without rewriting the original failed receipt or claiming a suite rerun.

Tier-A quick ran before review, then once more with the release binary stamped at
clean source HEAD0d71800b, without a stale-binary override. Both runs are **INVALID**,
not an accepted accuracy baseline: corpus differs from historical pin20c8490591a3;
C-method, C-name and U-method each have4/6 successful probes; oracle errors6/30
(0.20 >0.10), SUT errors0. Final matrix159 passed. Pinned `target-c-method` is a
flip candidate; `module-deps-feature-gated` and `load-repo-feature-gated` report
missing; ambiguous-symbol contract passes;16 adjudications remain pending.
No accuracy improvement/regression is attributed and no baseline was changed.

## Separate next candidate

`(runtime as TYPE).X` still reaches whole-member text serialization in the path
collectors. A compiler-valid import-type example emits `runtime.X`, but this
slice did not execute a native before/after probe of that compound case. Next:
bounded compiler/native proof and refusal requirements for asserted member paths
before normalization. Do not infer exhaustive erasure or DFG soundness here.

Duplicate/write/epoch barriers, executable ownership, closure admission, React.FC
scope and the unresolved `react-scripts` decision are unchanged. No private source
reads, real-source execution, dependency installs or registry publication.

## Durable evidence

Machine receipt: [verification JSON](2026-09-10-typescript-erased-rvalues-verification.json).
Raw directory: `/private/tmp/prism-erased-rvalue-avuSbL`.
Archive: `/private/tmp/prism-erased-rvalue-0d71800b-evidence.tgz`,870,899 bytes,
mode0600, gzip integrity checked; SHA256
`db794e7d3780dacadd6dabc571113e4a26c4807eb680e5c174f1b89bef9023ed`.
It includes RED/GREEN, all gate logs and original runner failure, reconciliation,
compiler verifier/receipt/readout, both Tier-A runs, review and source snapshot.
No source or evidence directory was deleted. Remote custody applies to committed
code/tests/readout/receipt; the raw archive is local, not a GitHub attachment.
