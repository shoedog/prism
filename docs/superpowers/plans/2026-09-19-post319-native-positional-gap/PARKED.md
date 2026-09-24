# Native positional-gap characterization observer — PARK RECORD (2026-09-23), RESUMED 2026-09-23

> **RESUMED.** On 2026-09-23 the owner granted a test cap of 740 (fourth amendment in BUDGET-AMENDMENT.md) and resumed
> the lane. The review cap is r4 plus at most r5. D1 and D2 below were folded at `5fc71b0e`.
>
> - Review r4: kimi APPROVE; sol FIX, 1 WRONG / 2 SMELL (stage-collision foreign delete), being folded.
> - r5 is the final round. The live state is in the lane handoff.
>
> The text below is the historical park record.

The increment was parked at `af6368c8` because the hard-final third review round did not approve.
## Review record

| Round | Subject | sol (gpt-5.6-sol xhigh, executed) | kimi-k3 (static) |
|---|---|---|---|
| r1 | `e5a3e7f4` | FIX 7 WRONG / 2 SMELL (W7 was a controller stale-cap error) | APPROVE, 7 SMELL |
| r2 | `ca9dcc9c` | FIX 3 WRONG / 2 SMELL (classified open-class) | APPROVE, 6 SMELL |
| r3 (owner-authorized hard-final) | `af6368c8` | **FIX 1 WRONG / 4 SMELL** | no verdict: the opencode turn ended mid-run (inadmissible) |

WRONG findings per round went 6, 3, then 1. Every r1 and r2 WRONG is CLOSED per sol r3. The inherited mutant runner
is 19/19 killed.

The r3 WRONG is **test coverage only**. The validators themselves trace correct, and each survivor was reproduced
against a passing unmutated control in the same environment. Four new bounded mutants stay green:

1. **`ordinary()` direct-comment allowance removed.**
   - Input: `later /*ok*/: string`.
   - Wrong result: eligibility becomes `selection_native_shape_mismatch` and the packet defers.
   - Correct result: eligible, with `next_action` `bounded_entry_and_call_proof`.
2. **Launcher `..` refusal removed.** `../outside.js` is read from outside the root.
3. **Top-level manifest exact-key check relaxed.** A manifest with an extra key is accepted.
4. **Canonical-output equality check removed.** A pretty-printed worker packet is accepted.

Evidence is at `/Users/wesleyjinks/prism-evidence/native-positional-gap/review-r3/`:

- `REVIEW-r3-sol.md`
- `r3-own-mutation-results.json`
- `r3-node-mutation-results.json`
- `r3-launcher-mutant-probes.json`
- `r3-comment-mutant-probe.json`

## Deferred work

### D1 — close the r3 coverage gap (Important; blocks core approval)

- **Why deferred:** the owner-authorized review rounds are exhausted.
- **Constraint:** the test bucket is exactly at its 700-line cap (strict count 1,073 / 700 / 1,773 against caps of
  1,100 / 700 / 1,800). The fix needs either row compression or a fourth budget amendment.
- **Impact:** without it, four wrong implementations stay green. No shipped behavior is wrong today. This is an
  opt-in research tool that has never run on public input.
- **Fix sketch:**
  - Add one TS/TSX complete-record row that holds both a list comment and a wrapper comment:
    `({x}: {x:number}, /*a*/ later /*ok*/: string)`.
  - Add launcher table rows for:
    - `..`, absolute, and backslash paths
    - an extra manifest key
    - a valid but noncanonical worker packet, via the authenticated test child
    - an unreferenced member
    - a native hash mismatch
  - Re-run the 19 inherited mutants plus the four r3 mutants. Estimated +25–40 test lines, so either compress
    existing table rows or ask for a test cap of about 740.

### D2 — r3 SMELL 1–3 hardening (Low)

- **Why deferred:** the fixes are non-blocking.
- **Impact:** diagnostics and robustness only.
- **Fix sketch:**
  - **S1 — orphan stage file.** Wrap the stage write so creation sits inside `try/finally`. Clean up best-effort
    without masking the primary error.
  - **S2 — signal reported as timeout.** Report `result.signal` distinctly from a timeout; today a crash is
    labeled a timeout.
  - **S3 — `runNative` trusts its caps.** Pass caps through `limits()`, or reject zero or raised caps, inside
    exported `runNative`.

### D3 — public observation (blocked by D1)

- **What it is:** once D1 gets core approval, the spec's §7 steps 3–5 remain:
  - two cold runs over the exact 12 selectors
  - independent reconciliation of every row
  - the readout (bounded next proof, or defer with reason)
  - `docs/eval/native-positional-gap/{site-manifest.json,receipt.json,readout.md}`
- **Status:** the 11 selected inputs are preserved and re-hashed at
  `/Users/wesleyjinks/prism-evidence/native-positional-gap/public-inputs-selected/`, so they survive the `/tmp`
  cleaner.

### D4 — test environment inputs (Low)

- **Problem:** the pinned react18/react19 profile layouts, needed by 6 callable-observation Node modules, were
  deleted by the macOS daily `/tmp` cleaner. The repository does not document how to re-acquire them.
- **Partial restore:** TypeScript 5.9.3 was restored from the npm registry, and its `typescript.js` SHA-256
  `3ae902c9…` matches the pin. It lives under `…/prism-evidence/native-positional-gap/gate-inputs/`.
- **Next step:** document the profile acquisition (the PR259 layouts) and keep pinned inputs outside `/tmp`.

## Gates on the parked subject `af6368c8` (not an acceptance)

| Gate | Result |
|---|---|
| Example tests | 1/0 |
| Focused Node suite | 11 pass / 0 fail / 1 RED skip |
| RED | Admissible: fails only on alias `source_ordinal` 1 vs 2 |
| `fmt`, `git diff --check` | Clean |
| Example clippy | 0 diagnostics |
| Default Rust | 4,559 pass / 0 fail / 1 ignored |
| Active Node population | 42 of 49 modules: 707 pass / 0 fail / 3 skip |

The three Node skips are the RED adapter plus two grammar-verifier skips pinned to Node 26.0.0 / macOS arm64.

Seven modules were excluded, and none counts as passing:

- `audit-imported-props-source`: its historical `PRISM_AUDIT_*` inputs are unavailable.
- `verify-callable-authority` and five `callable-observations` modules (index, module-search, nested, props-class,
  umd-qualifier): their profile inputs were purged (D4).

Logs are in `…/prism-evidence/native-positional-gap/gates-af6368c8/`.
