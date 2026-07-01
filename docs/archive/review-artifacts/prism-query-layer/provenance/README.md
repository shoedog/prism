# Provenance — Prism Navigation Layer (Tier 1) design

How the committed spec
(`docs/superpowers/specs/2026-06-07-prism-navigation-layer-design.md`) was
produced: a firewalled clean-room design plus three adversarial review rounds,
run via `~/code/a2a-bridge` (codex `gpt-5.5` rigor + claude soundness). These
files are point-in-time artifacts kept for design history — the canonical spec is
authoritative.

## Process

1. **Brainstorm + pressure-test** (in-session): validated the direction —
   differentiated reasoning over commodity navigation, library-first + CLI/MCP,
   single-repo, on-demand/Tier-1 split, preserve diff-review.
2. **Original full spec** — the in-session Claude spec covering Tier 1 *and* the
   reasoning layer (`00`). The reasoning layer was then split out as a follow-on
   initiative; the clean room was scoped to Tier 1.
3. **Clean-room codex design** — codex got the brief (`01`) + read-only repo
   access, never the Claude spec, and produced an independent design (`02`).
4. **Fold + 3 review rounds** — fold codex's design with the Claude spec, then
   `spec-review` (codex rigor + claude soundness → synth) each round:
   - `03` folded v1 → `04` review R1 (4 blockers)
   - `05` folded v2 → `06` review R2 (1 blocker: surfaced "Option C")
   - `07` folded v3 → `08` review R3 (1 blocker: a lifetime typo)
   - folded v4 = the **committed spec** (not duplicated here).

## Files

| File | What |
|---|---|
| `00-claude-original-full-spec.md` | Original in-session spec (Tier 1 + reasoning layer, pre-split) |
| `01-cleanroom-brief.md` | Problem statement given to codex (Tier-1 scope, discovered facts, no solution) |
| `02-codex-cleanroom-design.md` | codex's independent clean-room design + 12-item gap register |
| `03-folded-spec-v1.md` | First fold (codex ⊗ Claude) |
| `04-spec-review-round1.md` | Review R1 synthesis — 4 blockers, 6 major |
| `05-folded-spec-v2.md` | Hardened against R1 |
| `06-spec-review-round2.md` | Review R2 — found "Option C" (additive, zero core edits) |
| `07-folded-spec-v3.md` | Adopted Option C + R2 fixes |
| `08-spec-review-round3.md` | Review R3 — lifetime contradiction + clarifications |
| *(v4)* | → `docs/superpowers/specs/2026-06-07-prism-navigation-layer-design.md` |

See the committed spec's §0 disposition table for every finding → resolution.
