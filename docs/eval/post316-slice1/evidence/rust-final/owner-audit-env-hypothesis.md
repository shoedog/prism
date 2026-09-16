# Owner-audit gate environmental retry

- Observation: the first widest-feature run selected 31 suites and reported 4,711 passes, 23 failures, and 1 ignored test. Every failure panicked before its behavior assertion because the required explicit pinned compiler environment variable was absent (`NotPresent`). The run is inadmissible as ownership-behavior evidence.
- Hypothesis: the 23 failures are entirely an invocation-environment refusal. Supplying the restored pinned TypeScript 5.9.3 compiler and callable-profile root will make the same full gate run its assertions and pass.
- Expected if true: all 23 formerly refusing tests pass in the rerun and no behavior failure remains.
- Falsifier: any rerun failure after the test accepts the explicit compiler/profile inputs.
- Alternative: the compiler variable is accepted but the callable-profile root is absent or mismatched, producing a later authority-input refusal. The full rerun output separates that alternative from a candidate behavior failure.
- Retry cap: one diagnosed environmental retry for this gate class.

## Result

The environment hypothesis was supported and the missing-profile alternative was falsified. With the pinned compiler and profile root supplied, the same 31-suite gate passed 4,734 tests, failed 0, and ignored 1. The 23 former refusals all became passes; no same-environment base attribution was needed because no admitted behavior failure remained.
