# Gate pilot receipts (item 4), 2026-10-31
binary: the release build of the slicing repo (`target/release/prism`)
binary sha256: 226ff60f1ab6a8a0eca222c48eadd4f919db85478f13198724113b85d7244b60
binary mtime: Oct 30 22:00:57 2026
slicing main: 533f061a942a7cfefe32e678d68f431cc3025fb1
httpx: b5addb64f0161ff6bfe94c124ef76f6a1fba5254 (trial.json pins httpx b5addb64, cobra ad460ea8, excalidraw 0642e72c)
cobra: ad460ea8f249db69c943a365fb84f3a59042d54e (trial.json pins httpx b5addb64, cobra ad460ea8, excalidraw 0642e72c)
excalidraw: 0642e72cfa2d9a71198200e52f37399384610ee3 (trial.json pins httpx b5addb64, cobra ad460ea8, excalidraw 0642e72c)
gold: tracing-trial `gold/v2` in the private evidence store (frozen 2026-10-21; not published, so `pilot-report.json` carries counts, ceilings and verdicts but no gold units); ceiling rule: `evidence/directed-results.py` `rule_covered` (private store)
script: gate_pilot.py; raw CLI outputs (30 files) are in the private evidence store, not committed; `pilot-report.json` holds every reached line (P4) and the per-tier counts.

Re-running needs the private store and the pinned checkouts, and must not overwrite the committed receipt. Write to a scratch directory and compare the counts:

    export PRISM_TRIAL_DIR=<private store>/tracing-trial   # match.py, evidence/directed-results.py, gold/v2
    export PRISM_BENCH_REPOS=<dir with httpx/ cobra/ excalidraw/ at the pinned SHAs above>
    mkdir -p "$SCRATCH/gate-pilot-rerun"
    python3 -I gate_pilot.py <prism> "$SCRATCH/gate-pilot-cache" "$SCRATCH/gate-pilot-rerun/pilot-report.json"
    # then compare the covered/total/lines figures with the committed pilot-report.json

The script exits with a message when `PRISM_TRIAL_DIR` or `PRISM_BENCH_REPOS` is unset or does not contain the expected files. `PRISM_GOLD_DIR` (default `$PRISM_TRIAL_DIR/gold/v2`) overrides the gold location; `PRISM_PILOT_UNITS=1` adds the covered gold-unit ids to the output (keep that output private).


question | v7 bundle ceiling | T0 (one call per question-text seed) | T1 (+ one mechanical follow-up round)
G3 | 8/32 0.25 | 14/32 0.44 (120 lines) | 25/32 0.78 (209 lines)
J3 | 4/19 0.21 | 3/19 0.16 (6 lines)    | 3/19 0.16 (15 lines)
P3 | 7/29 0.24 | 5/29 0.17 (37 lines)   | 23/29 0.79 (227 lines)
P4 | 1/3 0.33  | 2/3 0.67 (8 lines)     | 2/3 0.67 (59 lines); 3/3 with a parameter-line remap (the DFG edge from the `stream` Def to its use is Exact; the Def is pinned to the signature line, while the remaining gold unit cites the parameter's own line)
calls: 9 slice, 28 taint-reaches, 2 callees; wall 13 s warm (excalidraw cold build 23 s)
Disclosure: the T1 follow-up rules (field-store -> grep field reads; callees d2 of the seed body -> seed their start lines) were written after the first exploratory probes showed the G3 field-store stop and the P3 inheritance stop; T0 is the strict recipe, fixed in the spec before the first probe (the only witness is the private store's file timestamps, not git history; see the top-level README). Caches were deleted after the run.
