# Implementation budget amendment — owner-approved 2026-09-23

The approved planning files (SPEC, IMPLEMENTOR, REVIEWER, SITE-MANIFEST, FINAL-PLANNING-ACCEPTANCE) are unchanged;
their hashes still bind. This amendment replaces **only** the §4 line budget. Every other contract clause stands.

## Trigger

The r0 implementer (gpt-5.6-terra xhigh, direct `codex exec`) stopped at the SPEC §4 early-stop gate. Stopping was
correct under the no-inflation rule. It preserved the WIP as commit `907ac3b6`; evidence is in
`/private/tmp/prism-native-gap-evidence/r0/`. The raw count was 357 physical lines, reachable only through 1,000+
character lines: the file set holds 48,228 bytes, and the longest line is 1,450 characters.

Controller measurement:

- **Rust worker.** `rustfmt --emit stdout` on the example gives 559 helper and 67 `#[cfg(test)]` executable lines,
  626 in total.
- **Node files.** The two files hold 12,889 (launcher) and 16,198 (tests) non-whitespace characters. At ordinary
  width that is roughly 230–320 and 295–405 lines respectively.
- **Honest totals.** About 790–880 helper, 360–470 tests, and 1,150–1,350 combined, against the planned
  450 / 450 / 900.

The planning budget could not hold the approved contract. That is a planning defect, not implementer inflation.

## Decision

The owner chose **"Amend budget, keep artifact"** over re-planning a smaller contract or parking the increment.
Consequences:

- New hard caps, in honest executable lines (non-blank and not comment-only):
  - **helper ≤ 900**
  - **tests ≤ 500**
  - **combined ≤ 1,400**
- Counting rules:
  - Rust is counted **after `cargo fmt`**.
  - JavaScript is counted with physical lines ≤ 100 columns. The only allowed exceptions are single string
    literals or fixture sources that cannot be split without changing tokens.
- Test code includes any `#[cfg(test)]` module.
- Early-stop threshold: 95% of either bucket cap.
- The existing artifact is kept, and the reflow pass must not change logic. Review then proceeds on the formatted
  artifact under the unchanged two-round implementation review cap.
- No contract, scope, owned-path, schema, or predicate change. The same no-restart and no-silent-inflation rules
  apply at the new caps.

## Second amendment: re-cap to measured size (owner-approved 2026-09-23)

The first amendment's caps rested on a bad controller estimate. It assumed roughly 40–55 non-whitespace characters
per reflowed JavaScript line; the verified reflow actually averages about 27. After the whitespace-only reflow,
both JavaScript files keep their non-whitespace streams byte-identical to `907ac3b6`, and neither has a line over
100 columns. Measured honest executable lines:

| File | Lines |
|---|---:|
| Rust worker helper | 559 |
| Rust worker `#[cfg(test)]` | 67 |
| `index.mjs` | 476 |
| `index.test.mjs` | 347 |

That gives **helper 1,035 / tests 414 / combined 1,449**, over the first amendment's helper and combined caps.
The owner chose **"Re-cap to measured, review"**. Final hard caps are:

- **helper ≤ 1,090**
- **tests ≤ 500**
- **combined ≤ 1,520**

Counting rules are unchanged, and the early-stop threshold stays at 95% of either bucket. Review fixes must stay
within these caps. A further overshoot is a stop and needs an owner decision, not a silent inflation.
