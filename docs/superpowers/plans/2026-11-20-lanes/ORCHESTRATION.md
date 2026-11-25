# Orchestration protocol for the 2026-11-20 lanes

This file is the operating manual for the agent that orchestrates a lane (a Sonnet-5.5 session with the
Agent tool and a shell). Each lane has a `DESIGN.md` (what is built and why) and a `PLAN.md` (stages, slices,
gates). This file holds everything the four lanes share. Where a lane plan and this file disagree, the lane
plan wins for that lane.

Read in this order: this file, the lane's `DESIGN.md`, the lane's `PLAN.md`, then the repository `CLAUDE.md`.

## 1. What the orchestrator does and does not do

The orchestrator dispatches work, runs gates, reads results, folds review findings and merges. It does not
design. Every design choice in a lane is either made in `DESIGN.md`, listed there as a parameter that a
named measurement fills in, marked tentative and settled by the design seat at stage D, or listed as an
owner decision with a default.

If a situation is not covered by the lane plan:

1. A missing fact (a count, a file location, a behaviour on main): measure it or read the code. Record the
   result in the lane's evidence directory and continue.
2. A choice between designs: do not choose. Apply section 8 (design escalation).
3. A choice that is the owner's (section 9): apply the stated default, record it, and continue on work that
   does not depend on the answer.

## 2. Seats and models

| Seat | Model and effort | How it runs | Used for |
|---|---|---|---|
| Orchestrator | Sonnet-5.5, high | the main session | dispatch, gates, folds, merges, handoff |
| Implementer | Sonnet-5.5, high | Agent tool, `model: sonnet`, `effort: high`, in the lane worktree; may commit, never pushes | tests, code, folds |
| Census and measurement runner | Sonnet-5.5, medium | Agent tool | scripted counts, corpus differentials, oracle runs |
| Enumerator, tier 1 | Haiku-5.5, then Sonnet-5.5, then gpt-6.1-terra | Agent tool; terra by `codex exec` | variant rows for the test matrix |
| Enumerator, tier 2 | Opus-5.5 high, then gpt-6.1-sol high | Agent tool; sol by `codex exec` | variant rows the first tier missed |
| Reviewer A | Opus-5.5, xhigh | Agent tool, `model: opus`, `effort: xhigh`, read-only | spec and code review |
| Reviewer B | gpt-6.1-sol, xhigh | `codex exec -m gpt-6.1-sol -c model_reasoning_effort=xhigh -s read-only -C <review clone> "$(cat brief.md)" < /dev/null` (the `< /dev/null` is required: with stdin open a backgrounded codex waits forever) | spec and code review |
| Design seat | Fable-5.2 (Agent tool, `model: fable`, `effort: high`) or gpt-6-astra | stage D of every lane, and section 8 | drafts the design-of-record amendment from the full row set; re-plans after a rejected design |
| Cloud runner | Haiku-5.5 in a Claude Code cloud session | `~/prism-evidence/tools/cloud-tests.sh`, `cloud-probes.sh` | targeted tests at `RAYON_NUM_THREADS` 1 and 4, RED-on-base control, reviewer probes |

Rules for the seats:

- Call codex by its absolute path, `~/.local/share/mise/installs/node/lts/bin/codex`. The Homebrew binary
  that a non-interactive shell finds answers `model_not_found` for gpt-6.1-sol and still exits 0. After every
  launch, check the log for the `OpenAI Codex v` banner and for `model_not_found`.
- A codex seat that must run `cargo` or `prism` is launched as
  `~/prism-evidence/tools/memguard.sh 8 <codex command>`; inside the codex sandbox memguard cannot read
  process memory.
- The reviewer of a slice never shares a session with its implementer. A sol review runs in a fresh session.
- Test-only and docs-only changes get one reviewer (Sonnet-5.5, high, a careful-reading review). Everything
  that changes product behaviour gets both reviewers.
- If the OpenAI content filter stops a codex seat (lane 4 works on a security benchmark), re-brief once in
  neutral static-analysis terms ("benchmark accounting", "entry point", "target write"); if it stops again,
  give that seat's work to the Claude seat of the same tier (sol → Opus-5.5 xhigh, terra → Sonnet-5.5 high)
  and record the substitution in the lane ledger.

> **Run of 2026-11-24 (owner).** For the run that starts the lanes the owner assigned OpenAI seats: the
> implementer and the census runner are gpt-6.2-sol (xhigh) by `codex exec`; the design seat and reviewer B
> are gpt-6.2-astra (xhigh); reviewer A stays Opus-5.5 (xhigh); the orchestrator stays Sonnet-5.5. The
> orchestrator makes every commit and every push, because a codex seat's sandbox cannot write git metadata: a
> codex implementer leaves its changes in the working tree and reports. Enumerator tiers for that run, in
> order: the orchestrator's own pass, gpt-6.1-terra, gpt-6.2-sol, and gpt-6.2-astra last. Private code stays
> with Claude seats as section 9 says. The run's operating details (the job launcher, the build and
> measurement slots, the claims on shared files) are in the controller's runbook,
> `~/prism-evidence/lanes-run-2026-11-24/RUNBOOK.md`. Where this note and the table above disagree, the note
> applies to that run.

## 3. Lane lifecycle

Every lane runs these stages in order. A stage has an exit gate; the lane plan gives the gate's numbers.

| Stage | Output | Exit gate |
|---|---|---|
| M. Measure first | a census with counts per shape, on the named corpora, by a script kept in the evidence directory | the lane's go/no-go threshold. Below it, first list why each case missed: if the misses come from how the measurement matches things and not from what the product can do, go to section 8; otherwise stop the lane and write the result to its issue |
| T. Tests first | the variant matrix as fixtures with a truth label per row, plus a data-driven test where the code under test already compiles | every census class has a row and every row maps to a class; rows that state new behaviour were shown RED once on base (a row whose command does not exist yet is recorded as "command missing", which is not a behavioural RED: it is shown RED again on the binary of the slice that adds the command); the lane branch still compiles and CI is green (see the rule below) |
| E. Enumeration | more rows, found by the enumerator tiers run serially (section 6) | a full tier round adds no row |
| D. Design of record | `DESIGN.md` amended by the design seat: the rule set, stated against the complete row set, and the accepted-loss list; the implementer then updates the rows' expectations to match. Every slice section of `PLAN.md` that the amendment contradicts is amended in the same change, so that a slice brief, which quotes the plan, never carries a rule the design has replaced | both reviewers APPROVE the amended design (cap: 2 rounds, then section 8) |
| I. Implement | one slice per PR, in the order the plan gives | the slice's tests GREEN, gates of section 5 pass |
| R. Review | the reviewers the seats table assigns to the PR (both for a product change, one for a test-only or docs-only change) | no open MATERIAL WRONG finding (cap: 3 rounds per slice, then section 7) |
| G. Gate | corpus differential and the lane's acceptance numbers on the final head | numbers meet the plan's gate; regressions are listed in the PR body, never re-baselined away |
| H. Handoff | the lane ledger and handoff file refreshed, evidence summarised and offloaded | written at every stable point, not only at the end |

Stage M and stage T may reveal that a design parameter in `DESIGN.md` is wrong for the measured population.
That is expected. The census result overrides the parameter; record the change in the design's change log.
A census result that contradicts a design *premise* (the mechanism cannot work as described) goes to section 8.

Stage T must leave the branch compiling. A matrix is data (fixture files and a rows file) read by one
data-driven test with a constant that says which row groups are enabled; a slice enables its group in its
first commit and shows the test RED before implementing. Where the module under test does not exist yet,
stage T commits the data only and the slice adds the test. A stage-T commit never adds `#[ignore]` rows that
cannot compile, and never names an item that does not exist.

The stage D brief for the design seat contains the lane's tentative rules, the complete rows file with
truths, the census, and the accepted-loss candidates. It asks for rules stated against every row, and it
does not contain the orchestrator's opinion of which rule to choose.

Rules during stages T and E (owner rules, 2026-11-14):

- Rows state the required behaviour and a truth label. They never propose a rule or a fix.
- Accepted rows are not reviewed again in later rounds.
- A row that hits a documented model boundary is still written; it is marked `#[ignore]` with the issue
  number, or listed as an accepted loss in stage D.
- Corpora are samples. Enumerators also derive shapes from the language grammar and semantics.
- Rows whose truth comes from executing a lane-written fixture run contained as that lane's design states,
  and the orchestrator reads each new fixture before its first run. The runner checks its own containment at
  launch and refuses to run when the check fails.

## 4. Workspaces

For each lane, once (the lane plan names the worktree directory):

```bash
df -h /                                  # stop if less than 20 GiB is free
git -C ~/code/slicing fetch origin
git -C ~/code/slicing worktree add -b <branch> ~/code/prism-<lane> origin/main
cd ~/code/prism-<lane>
git sparse-checkout set --no-cone '/*' '!/eval/secbench/'    # not for a worktree whose task edits or runs the benchmark harness
```

The plan directory reaches `main` through PR #843. Until that PR merges, a lane worktree cut from `main` does not
carry the plans: they are read, and a stage D amendment is committed, on branch `plan/lanes-2026-11-20`
(the controller's runbook says how). After it merges, a lane worktree carries its own plan and a stage D
amendment is committed on the lane's branch and merged as a docs PR.

Branch names: `<type>/<lane>-<stage or slice>`, for example `test/targets-inventory-matrix`,
`feat/targets-inventory-s2`, `eval/schema-impact-feasibility`. One branch and one PR per slice; each new
slice branches from the updated `main`.

- One worktree per lane, one build per worktree; a second worktree only where the lane plan names it (two
  slices that must be built side by side). Delete `target/` when the lane pauses for more than a day.
- A codex reviewer works in a disposable clone of the pushed branch, never in the implementer's worktree.
- Every `cargo test`, test binary, `prism` corpus run and checker run goes through
  `~/prism-evidence/tools/memguard.sh 8 <command>`. If memguard refuses (exit 75), a measurement holds the
  host-quiet marker: run `python3 scripts/hostquiet/host_quiet.py wait` and retry. Never set
  `MEMGUARD_IGNORE_QUIET`. After a build that went through memguard, check `prism --version` shows the
  expected commit before using the binary.
- Timing-sensitive measurements take the host-quiet marker (`scripts/hostquiet/README.md`) and run one at a
  time. Counting censuses and edge differentials are not timing-sensitive and take no marker, but still run
  one at a time and through memguard.
- Never run the full local test suite when CI will run it. Kill a process by PID, never with `pkill -f`.
- Push the lane branch after every implementer wave: `git log origin/<branch>..HEAD | wc -l` must be 0 before
  any wait longer than a few minutes.

## 5. Gates on every product PR

1. `cargo fmt --all --check`.
2. Targeted tests for the touched code, at `RAYON_NUM_THREADS=1` and `4`, with a RED-on-base control for new
   tests. In the cloud: `~/prism-evidence/tools/cloud-tests.sh WORKTREE RESULT_BRANCH SPEC BASE OVERLAY`,
   where WORKTREE is the lane worktree with its head pushed, RESULT_BRANCH is a new name such as
   `test-results/<lane>-<slice>-<n>`, SPEC is a file with one `cargo test` argument list per line (for
   example `--test cli targets_inventory_test::`), BASE is the merge base with `main`, and OVERLAY is a file
   listing the new or changed test and fixture paths, one per line. Read `summary.json` on the result
   branch, then delete the result and input branches. Local runs through memguard are the fallback.
3. Every behaviour added or fixed has a test that fails on the pre-change code, and each new code path has a
   negative or edge row.
4. CI on the pushed head: wait until the full check set has appeared, then require every check to pass and
   `gh pr view --json mergeable` to be `MERGEABLE`. An empty check list is not green. A PR shown as
   `CONFLICTING` gets `origin/main` merged in first.
5. If the PR touches `src/call_graph*`, `src/navigation/`, `src/cpg/`, `src/ast*`: build release, then
   `cd eval && uv run tier-a --matrix-only --allow-stale-sut` before committing and
   `uv run tier-a --quick --allow-stale-sut` before review. Paste regressions into the PR body.
6. If the PR adds, removes or relabels data-flow nodes or edges: a corpus differential on the lane's corpora,
   comparing the `nav dfg-stats --edges` multiset and every `nav call-stats` counter (not only the DFG
   ones) between the merge base and the head. The PR body states the prediction made before the run and the
   result.
7. If the PR changes what is stored in the CPG or nav cache: bump the cache version constant in the same PR.
   Two open PRs that both bump it conflict; the second one rebases and takes the next number.
8. File size: `python3 scripts/file_size.py <changed files>`. New files aim for 600 weighted lines or fewer;
   no file may pass 2,000; a file already over 2,000 does not grow (new code goes in a new module).
9. Layering: nothing under `src/` names `prism_review`; `crates/prism-mcp/src/` does not name the algorithms.
10. Docs in the same PR: the behaviour paragraph in `CLAUDE.md`, and for JS/TS work the matching entry in
    `docs/features/language-coverage/js-ts-known-gaps.md`.
11. No mutation gate per PR (owner, 2026-10-31). Briefs tell implementers not to run `mutgate`.

Merge rule (standing): every reviewer assigned to the PR APPROVEs (both for a product change, the one
reviewer for a test-only or docs-only change) with no open MATERIAL WRONG, CI green on the final head, branch
up to date with `main` → `gh pr merge --squash --delete-branch`, then report the merge with the review record
and the CI result. Deferred findings are written to an issue before the merge.

## 6. Briefs

Every brief is self-contained: the worker sees nothing else.

**Implementer brief** contains: the slice's section of `PLAN.md` verbatim, and verbatim every `DESIGN.md`
section that the slice's text cites by number, both cut from the revision approved at stage D (the brief
names that commit and the revision of the rows file); the worktree path and branch; the
rows that must turn GREEN and the rows that must stay as they are; the gates of section 5 that apply; "do not
run mutgate; run tests through memguard; do not read `eval/secbench`; do not push" (when the task edits
or has to read the benchmark harness itself, the worker is a Claude seat, the worktree keeps that directory, and the
sentence becomes "read and edit the harness code under `eval/secbench`; never install, import or run
benchmark package code"); "commit after each green
step with a message that names the slice"; the report format (what changed, test totals, what was not
verified, anything outside scope that failed).

**Enumerator brief** contains: the goal of the matrix in one paragraph; the axes; the cumulative row list so
far; "find rows that are not covered; for each give the input, the required behaviour, the truth label and
the class; do not propose rules or fixes; say how you searched (grammar walk, corpus search, semantics)".
Tiers run serially, each model reading every earlier list. A tier repeats until a round adds nothing; then
the next tier starts. A model that added nothing in its last round is dropped from later rounds.

**Reviewer brief** contains: the goal the slice serves (so findings can be tagged MATERIAL or IMMATERIAL to
it); the diff range; the design sections that apply; and these requirements, verbatim:

> Review as a senior developer who wants this project to succeed in the long run. Be rigorous about real
> defects and do not invent any. Tag each finding WRONG (name the input or state and the incorrect result) or
> SMELL (no demonstrated incorrect behaviour), and MATERIAL or IMMATERIAL to the stated goal. For each
> finding give a recommended fix and the alternatives with their trade-offs, and say under which assumptions
> the finding would not apply. If a finding belongs to an open class, list every member you can identify, say
> how you searched, and say whether a dedicated enumeration pass is needed before the next fold. Review by
> reading. Do not re-run tests that CI ran. If you want a hand-broken variant tested, write it as a mutgate
> lane JSON file (`review-<pr>-<seat>-probes.json`, at most 10 probes) and return it.

The orchestrator runs returned probes with `~/prism-evidence/tools/cloud-probes.sh` and sends survivors back
to the reviewer for the final verdict. Reviewers on code that touches call resolution or data flow are told
to use the prism navigation tools on a pre-warmed index of the worktree and to end with a line
`prism: <n> queries`.

**Measurement brief** contains: the script path, the binary and its expected `--version` commit, the corpus
paths and revisions, the prediction to write down before the run, and where the raw output and the summary go.

## 7. Convergence

- Declare the cap before each loop: design review 2 rounds; slice review 3 rounds; enumeration ends when a
  full tier round is dry, and a tier that still adds rows of one class after 5 rounds is an open class: stop
  and go to section 8.
- When stage D changes what a row requires, the row is run again against the base build and its RED or GREEN
  record is replaced before the slice that enables it starts.
- A cap belongs to the blocker, not to the artifact. A renamed PR or a successor slice for the same blocker
  inherits the rounds already spent.
- At a cap, classify. Converging (each round has fewer, smaller, non-repeating findings): fold the valid
  fixes, continue, and say so in one line in the ledger. Open-class (each round finds new members of the
  same kind): stop folding, write the members as matrix rows, and go to section 8.
- Closed findings (input, incorrect result, bounded fix) get a targeted fix on the existing branch. Never
  restart a slice from scratch to escape review pressure.
- Before folding a cut that refuses or removes results, measure how often it fires on the lane's corpora in
  the same fold. A fold whose yield line drops materially is not merged; port the reference behaviour
  (the language's own rule) in preference to a blanket refusal.
- A WRONG verdict from an oracle that the lane believes is an oracle error is not waived by the orchestrator.
  It is written up (the input, the oracle's raw row, why it is believed wrong), the case is added to the
  oracle's self-check, and both reviewers see the raw row and confirm before it is reclassified. An override
  for a whole class of rows needs that proof for each row.
- When the orchestrator thinks a reviewer finding is wrong or not worth its cost, it does not decide. It
  records the finding, its own counter-argument and the options as an owner question (section 9), and
  continues with other slices.

## 8. Design escalation

Used when stage D is rejected twice, when a census contradicts a design premise, or at an open-class cap.

1. Write the problem only: goal, constraints, the failing rows, what is settled. Do not name a preferred
   option. Have a Sonnet-5.5 agent rewrite any measurements into a neutral factual summary.
2. Send it to the design seat (Fable-5.2, or gpt-6-astra when the Claude seat wrote the rejected design) and
   ask for its own options with pros and cons.
3. Keep the orchestrator's and the original design's options in a separate file. Pool both sets and ask the
   design seat to evaluate them, including combinations of options.
4. The result amends `DESIGN.md`. Both reviewers review the amendment once.
5. If they reject it again, or a requested cap extension is refused, the orchestrator stops the lane at that
   slice and reports to the owner: the rows, both designs and the reviewers' reasons. It does not try a third
   design.

## 9. Owner decisions

Each `DESIGN.md` has an "Owner decisions" table: the question, the default this plan applies, what the
default costs if the owner later chooses otherwise, and the latest point at which the choice can still be
changed cheaply. The orchestrator applies the default and keeps going. It asks the owner only when:

- a lane reaches its "latest point" for a decision and the owner has not confirmed the default;
- an action in the list below is next;
- a reviewer disagreement of section 7 blocks every remaining slice.

Owner-only actions, never taken on a default: changing a contract shared with another product (the targets
schema's version, any schema under the tooling repository's `contracts/`); committing to or opening pull
requests in another repository (faultpath, the tooling repository) beyond filing an issue; a release or a
tag; deleting evidence that exists in one copy; starting a stage that a lane plan marks "owner go-ahead".

Private code (the private corpus, and lane 5's private pair):

- The orchestrator may run measurements on it locally, with Claude seats only. Never in a cloud session and
  never through a codex seat.
- Outputs stay under `~/prism-evidence/` and are excluded from any offload of that lane's evidence.
- Its directory names, file paths, identifiers and code never appear in this public repository, an issue, a
  PR body or a commit message. Public text uses `private-corpus` (lane 5: `pilot-producer`,
  `pilot-consumer`) and aggregate counts only. The real locations are in
  `~/prism-evidence/lanes-2026-11-20/PRIVATE-BINDINGS.md`.
- Before every commit and every public post, search the staged diff or the body for the private names
  (`git diff --cached | grep -n -F -f <names file>` after `git add`; `git grep` misses untracked files).
- Briefs that tell a worker which directory not to open must say that the sentence is not to be copied into
  repository files.
- The sibling fault-injection product's source and the tooling repository are private too: tasks that read
  or run them use Claude seats; a codex seat that needs their behaviour is given recorded oracle outputs.

## 10. Evidence and handoff

- Lane evidence lives in `~/prism-evidence/<lane>/`: census scripts and outputs, measurement summaries,
  review records, the ledger (`LEDGER.md`, one line per event with the commit) and the handoff
  (`HANDOFF.md`, from `~/.claude/handoff-template.md`), refreshed at every stable point.
- What goes where: the design and plan live in this directory and are linked from the lane's issue; reviews
  are attached to their PR; measurement summaries go in the PR body; raw dumps stay in the evidence
  directory until the lane closes, then are summarised and offloaded.
- State what was verified and what was not in every done-claim. A probe that failed for its own reasons
  (wrong flag, zero tests selected, a refused guard) is not evidence; fix the probe first.
- Before blaming a failure on a change, run the pre-change build in the same environment and put the
  control's result next to the claim.

## 11. Running lanes side by side

Lanes 2 to 5 touch mostly different subsystems; lanes 6 to 12 overlap more with them and with each other.
Lanes may run in parallel, one orchestrator per lane, within these limits. Before a slice that touches a
shared file is dispatched, the orchestrator checks the other lanes' issues for an open PR on that file and
rebases after it; two lanes never have open PRs on the same shared file at once.

| Shared resource | Who touches it | Rule |
|---|---|---|
| CPG cache version constant | lanes 3, 4, 9 and 10, and the unrelated C/C++ data-flow lane (#818) | section 5 item 7: each PR takes the next free version at rebase time |
| `crates/prism-review/src/algorithms/taint*` and `taint_sinks/` | lane 4 (new sink), lane 3 (end-to-end check only), the cross-file taint PR #841 (merged 2026-11-23 as 9d1a7d06) | lane 4 adds new modules, a module line in `taint_sinks/mod.rs`, and edits the sink loop and emission block of `taint/run.rs`; lane 7 edits the same loop to attach rules (lane 4's sink slice follows lane 7's taint slice when lane 7 runs; if lane 4's merges first it writes its rule tag in the description and lane 7's taint slice migrates that producer and its reader); lane 6 adds one module (`taint/request_sources.rs`) and a module line; lane 8 implements its sink catalog over this code and extracts the sink loop of `taint/run.rs` into a function (the same loop lanes 4 and 7 edit: whichever merges later rebases). For the MCP server, lane 8 then moves nearly the whole taint module (about 18,300 of 19,146 lines, 51 of 53 files) into a new fourth crate, `crates/prism-taint`, below both binaries (owner, 2026-11-24; lane 8 slices I-9.1 to I-9.3): that move is a move-only PR sequence that no other lane's taint PR may overlap, so lanes 4, 6 and 7 finish their taint slices first or rebase after it; rebase over #841 when it merges; lane 3 changes no taint code |
| `src/cpg/object_literal_args.rs` | lane 3 | lane 4 reads its facts, does not edit it; lane 10 edits one path lookup there, in the slice that changes path spellings, after agreeing the change on lane 3's issue |
| Argument binding (`src/cpg/build.rs` step 5b, `src/cpg/callee_params.rs`) | lanes 3 and 9; lane 10 changes the path lookups | each lane adds modules and call lines; a lane rebases over the other's merged slice and re-runs its matrix |
| `src/cpg/trace.rs` and the reasoning layer | PR #841 (merged), then lane 8 | lanes 3, 9 and 10 do not edit it; what they need from it (descent rules, the check that a name-only binding edge is never crossed) is lane 8's, and the slices that depend on it wait for lane 8's name-only guard slice |
| `src/call_graph.rs`, `src/navigation/queries.rs` (over the size ceiling) | lane 9 (move-only splits first), lane 12 (calls only) | no lane adds logic there; move-only PRs are not combined with feature PRs |
| `SliceFinding` and its struct literals | lane 7, PR #841 (merged) | lane 7's field slice is cut from a `main` that contains PR #841 |
| The benchmark harness options (`eval/secbench`) | lanes 4, 8, 9 and 11 | one eval-only PR adds the pin and class options (lane 4's task); the others reuse it or do that task themselves if it has not merged, never a second variant |
| Measurement scripts | lane 11 builds the shared kit; lanes 3, 6, 8, 9, 10 measure | a lane whose measurement stage starts after the kit's comparator has merged uses the kit and adds only a classifier |
| `src/framework_entries/` | lane 12 | lanes 5 and 9 consume the route listing; they do not add route extraction of their own once lane 12 has started |
| `crates/prism-review/src/targets/`, `docs/contracts/targets.schema.json` and `docs/contracts/targets-1.1.schema.json` | lane 2 | no other lane edits them, except lane 6's additive edits (visibility, an optional repository root in the site context) after lane 2's first slice. The shared identity extraction has one owner: the lane whose slice starts first does it and records the commit on both issues; the other rebases onto it and never repeats it |
| The host | every measurement | section 4: one measurement process at a time across all lanes |
| `CLAUDE.md`, the known-gaps doc | every lane | each PR edits only its own paragraph; conflicts are resolved by rebasing |

Recommended order when only one orchestrator is available: lane 2 (smallest, unblocks a sibling product),
lane 3 stage M and lane 4 stage M together (both are censuses), then lane 3 implementation, then lane 4
implementation if its gate passes, with lane 5's feasibility study filling waits. Lane 5's pilot starts only
after its feasibility memo and an owner go-ahead.
