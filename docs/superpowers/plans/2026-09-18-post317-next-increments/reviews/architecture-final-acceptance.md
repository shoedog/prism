# Post-317 architecture planning acceptance

**APPROVE — WRONG: 0 / SMELL: 0 remaining.**

This is verification of the controller-authorized targeted cap supplement for W1/W2/S1/S2, not a third broad planning review. Two planning rounds plus one explicitly disclosed, finite correction were used. Existing production is unchanged from merged baseline `9fb6c823e5fa8e6c07ace8c5b41abe9bc5acf090`, tree `9b4de2b021d0a53fe4d07e50599f97d6c685e9ee`; planning HEAD remains `ef11cde08377b1256c04e38de5d330e575bae527`. No production edits, new builds, tests or broad suite reruns were performed by this verification.

## Accepted frozen planning artifacts

Relative to `docs/superpowers/plans/2026-09-18-post317-next-increments/`:

| Artifact | SHA256 |
|---|---|
| `candidate-brief.md` | `dc7367430b98888f039237441438b1df2c41f653757905f8b4a4ec0cc8ca97fe` |
| `specs/01-exact-caller-read-occurrences.md` | `52f8648fa0c4c0f07817dc9985a379dabe9943600486cfcc8c1e270c22485b50` |
| `implementor/01-exact-caller-read-occurrences.md` | `614965657f30808dab03739930c12baef9a84d620084ee16e9048af0bb347fac` |
| `reviewer/01-exact-caller-read-occurrences.md` | `730486a6404b9efbc8e8131a4777bf006a686611e3b1fac0f15f527efcf0ec77` |
| Handoff `docs/superpowers/handoffs/2026-09-18-post317-next-increments.md` | `47c12046fb23b5f4d364e064a158a890e69b0d6ffc196967cbb72332d61f7a41` |

Final status-only handoff updates and receipt copying may record this acceptance without changing the implementation contract or reopening source review.

## Closed finite findings

- **W1 closed:** R01 now contains the authentic compact app/origin literals, filename labels outside the source, explicit no-trailing-newline convention and exact hashes. Independently extracted Markdown literals reproduce app SHA `5457bf7b...583f1`, origin SHA `8b721830...7fecb`, and value46–51/58–63/77–82. No cache regeneration or weakened byte oracle was needed.
- **W2 closed:** R02 now contains the authenticated parameter/local fixtures with real prior statement lines. Extracted literals reproduce 99-byte parameter SHA `286faa2b...ab10` and 85-byte local SHA `5c98fba3...6250`, including exact stated token offsets. Preparation's unchanged-base log contains all six dialect/case rows: existing first edge Exact, direct later edge absent; parameter later argument boundary Exact. Read and authenticated receipt SHA `2312e8ea236bfb0de6366e93f4d39b1b5299da9e485b40e4295da7171a0b7870`, log SHA `f8a357484b62702fbe5eee062b786bc3d3a633deb71b68b23e121fb4e7bbec14`, and harness SHA `4b7e707fff208838490516ba2bd8132455d4d944359e9c6381a1432faac9981e`. The log reports1selected1pass; this is baseline characterization, explicitly not the future behavioral RED.
- **S1 closed:** exact state is supplemental-only after independent classification and overlap checks; fact/cardinality expectations are unambiguous. Same-pair conflict preserves all legacy facts while refusing only that binding's expansion; unrelated-binding positive and representation-aware absent/wrong-key label controls are explicit. Direct deleted/untouched `remove_files`/merge parity is required. No impossible private-enum tamper detector is implied.
- **S2 closed:** pre-change public RED/preservation tests are separated from candidate-only internal fault injections. Production/total diff guardrails are700/1600changedlines, with controller re-slicing on overrun. Top-level classifier factoring may reuse one solve; reaching-subdirectory binding/kill production modules remain excluded. Actual `--sut-bin` binding and an immutable/locked quick corpus are explicit.

`git diff --check` is clean; `git diff merged-base -- src Cargo.toml Cargo.lock` is empty.

## Accepted contract and limits

Dispatch only the first bounded implementation: internal byte-keyed supplemental caller producer edges with independently classified confidence, deterministic CPG union, unchanged VarLocation/FlowEdge identity, legacy byte payloads/queries, kill/alias solver and owner inventory. Admit the one-line parameter NameOnly fixture and authenticated multiline/prior-local Exact cases under the conservative no-write/no-alias/no-shadow/control-flow gate. Preserve all refusal domains. Complete R01–R07, genuine96→97cache/nav53parity and full verification under a separate two-round implementation review cap.

Priority2 remains one observer-only census with independent cohort/public/private ledgers; Priority3 remains conditional fractional-indexing packet falsification. Both historical measurement populations are currently input-blocked. This planning acceptance does not establish corpus availability, authorize substitution of synthetic value measurements, prove the implementation works, or authorize publication/merge/adoption.

APPROVE PRIORITIES AS WRITTEN
