> Current integration authority — 2026-11-12: This product line is integrated onto freshly fetched main `4f9e3a3eb8d2fc30c59d3741a494eafd6b1d3ac3` in `feat/review-crossfile-context-main-sync-20261112`. All old acceptance/test totals below are inherited historical receipts on their original bases, not validation of this integration. Before each next product slice, synchronize with current main and record the exact resulting base. Every delegate, regardless of provider, runs affected/dependent slice checks; run the full workspace suite before merge. This owner cadence supersedes older per-slice full-suite wording below. Keep required scoped gates and named Tier-A triggers. The twelve historical slices need one final-tree affected integration set, not twelve repeated runs. Preserve pinned historical controls and use the new main base separately for integration regression controls. No push/merge is authorized by this document.

# Post-S5 measurement protocol for #602

Status: prepared, not executed. Controller accepted /root/evidence_audit's read-only readiness audit. Historical artifacts are inherited evidence, not fresh runtime proof. This is a post-implementation comparison, not a revival of the waived entry gate.

## Custody and population

Base: `4c8620a2b130be19f242ff19d657869ccd12364e`. Bind final candidate commit, binary hashes/version, release configuration, platform, manifest/spec hashes and exact corpus commits. Restore root: `/private/tmp/prism-crossfile-602-evidence/restored/jsts-lane/602`.

Archive `prism-evidence-jsts-lane-2026-11-08.tar.zst`, SHA256 `68d11326ce3714aa7e48598d0b4fe198560955031d6e6deb61801eaa51c5e50c`, checksum verified by controller. Preserve original and restored historical outputs unchanged. New results get a fresh directory.

Auditor used git cat-file --batch-check: all 20 selected commits, all 20 parent references and pinned selection heads present in each of directus-directus, calcom-cal.com, apache-superset, saleor-saleor, miniflux-v2 and apache-answer. 222 distinct commit objects, none missing. Descendant blobs were not individually verified. Use isolated scratch corpus worktrees; never checkout or edit benchmark clones.

Restored M2 population: 120 matching done metadata records; 120 BASE specs (86,414 bytes); 360 BASE/CTX/CTX0 specs (3,898,066 bytes); 714 parseable compressed SARIF runs (1,008,160 bytes), each recorded rc 0; six CTX0 runs intentionally absent for three empty closures. Whole restored m2 directory is 13,805,984 bytes. All 60 adjudication keys match historical SARIF.

Do not run archived m2run.py unchanged: it hardcodes old paths/binary/memguard and resumes records marked done. timing.py has a historical one-off repair that must not touch restored evidence. Reuse original BASE specifications unchanged; do not regenerate broad static closures for new B.

## Arms and execution

Layer A has a separate retained performance budget from original #602: <=+5% review wall on restored fx/e1 with run/e1.json, fx/py with run/py-app.py-5.json, fx/go with run/go-main.go-9.json, and one ten-file Excalidraw diff. The old spec did not pin the Excalidraw files/diff or repetition protocol. Before timing, pin a new ten-file JSON/hash and isolated corpus identity (auditor observed bench-repos/excalidraw at0642e72cfa2d9a71198200e52f37399384610ee3), declare repetitions and alternate base4c8620/candidateOFF order under host quiet with identical argv/cache state. Record actual outputs and per-case statistics; profile separately. Run this after S2 and reconfirm on final candidate if later changes affect the default path. Do not waive A's budget because the old B entry gate was waived.

Build CLI precisely with `cargo build --release -p prism-review --bin prism`. Historical command grammar remains:

```
BIN --repo ISOLATED_TREE --diff ARCHIVED_BASE_JSON --algorithm taint \
    --resolution scoped --min-confidence nameonly --format sarif --no-cache
```

Run three paired arms on all 120 BASE specifications: preserved base binary; current OFF; current ON. S5 planned additions (verify names against implementation before running):

```
OFF: --taint-context off
ON:  --taint-context imports --taint-context-edges nameonly
     --taint-context-depth 2 --taint-context-max-files 64
```

Keep explicit scoped resolution: current default is nominal. Omit taint-source, taint-return-flow, scoped-cpg, files and cache-dir for this primary pass. Serialize processes under host quiet with deterministic rotating arm order. Record complete stdout/stderr, monotonic elapsed time and exit status. Timeout, malformed output and environment refusal are separate dispositions, never zero findings. Execution coordinator must declare retry cap before attempts; invalid probes earn no behavioral evidence.

## Analysis

Base versus current OFF checks unchanged findings/evidence/non-note payloads, permitting intended new notes. Current ON versus OFF measures added results. Match by category/file/line, then inspect supporting roots/paths before credit. Count notes, hint candidates, findings, bounds/skips and suppressed novel diff-only perturbations separately.

Historical labels are a regression panel, not current population precision. NEW 30: consensus yes J012/J045/J059 (3), consensus no 23, disputed 4. BASE 30: consensus yes J004/J032/J050/J056 (4), consensus no 25, disputed 1. Retention panel therefore has seven positives, 48 negatives and five disputed records. Match repo/full commit/category/repo-relative file/line, then verify same operation/proposition. A disappeared record is retention outcome, not a false positive. Changed propositions and unmatched findings need fresh adjudication; disputed records stay disputed unless re-reviewed.

Archive defect: labels/sol.jsonl line 12 has truncated invalid JSON for J012. Complete identical records appear twice in labels/sol-labels.log. Recover only in a new derived normalization with source-file/line provenance. Do not modify original or use silent regex parsing.

Draw a new fixed-seed sample from current incremental sinks and OFF sinks, up to 30 each, balanced across historical repository buckets with existing per-diff cap. Exact matching propositions may retain prior labels; independently adjudicate others. Strict useful credit needs concrete edited-data flow into a real sensitive operation's argument. A sanitized path may count; exploitability is not established. Report yes/no/unresolved separately; either-reviewer-yes is not strict credit.

The old in_exact_arm label refers to the finding-output filter, not the new Exact traversal gate. Historical language buckets are repository buckets: J004 is a JavaScript sink in the Go bucket. Report actual source/sink extensions too. Corpus has TS/TSX, Python, Go and some JavaScript but no JSX diff file; fixture coverage must address JSX separately.

Report successful/failed/bounded diffs, incremental sinks and affected diffs, strict useful examples, noise, per-repo and actual-language breakdown and concentration in top diffs. Timing: median/p95/max per arm and paired ON/OFF ratios; distinguish median ratios from ratios of medians. Include loaded files/bytes/rounds and bound hits. Any warm-cache pass is separately identified with per-binary cache directories. Old 348 NEW sinks and timing are context only, not current expected results.
