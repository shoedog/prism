# Handoff — post-317 planning custody

**Written:** 2026-09-19 · **Provider:** codex · **Workspace:**
`/private/tmp/prism-post316-slice1` · **Branch:** `plan/post319-successor`

## Binding state

| Item | State | Binding evidence |
|---|---|---|
| Priority 1 exact caller reads | merged | PR #318 merge `e2d177d5144126a83159f41f1b2ee14e8eeb2b94`, tree `f21553d63fb99a3c267d44fc39b098c8fe8c7f0b`, matching published head `85d4b3c0090dfaf7b33f9eb0348fff80cbf784fb` |
| PR #318 CI | green | run `35466277259`; terminal receipt `/private/tmp/prism-post317-publication/pr318-ci-receipt.json` SHA-256 `27a03fc652fb75a5f626127643dc87baed46ba82c1e801e78881bd8079ad30c1` |
| Compiler syntax-frequency slice | merged / main-push CI pending | PR #319 merge `30e13053c9f3f9940b5526e20af0cb40f2a9fae4`, tree `961f698db1049cec42e7a8e15e716948082af9b6`; custody in `docs/eval/parameter-syntax-frequency/` |
| P2a | parked-design | `p2-split/P2A-BUDGET-STOP.md`; no further execution under its stopped contract |
| P2b | deferred | `p2-split/P2B-DEFERRED-SPEC.md` |
| P3 | parked | `parked-p3/README.md` |

## Active gate

The syntax-frequency slice is locally accepted, passed all five pre-merge CI
checks, and merged as PR #319. Separate main-push CI run `35473373053` is
pending. Successor planning has not started and remains gated on that run
turning green; `/root` retains CI interpretation and planning dispatch.

## Source mapping

- Reviewed Priority 1 source: `14e86083c2c754ed412d440742bd5763be388e42`,
  tree `a0411db6d858fe34806376ce41d2621853848d8b`.
- Published PR head: `85d4b3c0090dfaf7b33f9eb0348fff80cbf784fb`.
- Merge tree: `f21553d63fb99a3c267d44fc39b098c8fe8c7f0b`.
- Syntax-frequency published head: `1ddfbe147e8785f2d04d5fb7c28193de9cca302c`.
- Syntax-frequency merge commit/tree: `30e13053c9f3f9940b5526e20af0cb40f2a9fae4` /
  `961f698db1049cec42e7a8e15e716948082af9b6`.
- The byte-exact copied syntax-frequency files and their external source
  hashes are in `CUSTODY-MAP.md`; do not substitute current status text for
  those preserved artifacts.

## Stop conditions

- Do not treat the P1 Tier-A quick `INVALID` result as an accuracy claim.
- Do not restart P2a, execute P2b, or unpark P3 without separate controller authority.
- Do not begin successor planning before the separate PR #319 main-push CI is green.
