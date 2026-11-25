# C3 inventory verification

The worktree was at `6e1357f911b5aefb75092527309e1e9d2c451c5c` on
`eval/subscript-recall-inventory`. PR #903 is in that commit. The local
`origin/main` ref advanced separately during the run.

| Check | Command or evidence | Result |
|---|---|---|
| Tests-first collection | From `scripts/subscript_recall/`, `python3 -B -m unittest` | The first run stopped at the missing `inventory` import. |
| Executed regression controls | `python3 -B -m unittest` in the same directory | Before fixes, controls exposed a JSX tail refused as a barrier, nested-child replacement at the wrong depth, an admitted direct-eval root and unpacking operand, and branches incorrectly taken after literal tests. |
| Final unit suite | From `scripts/subscript_recall/`, `python3 -B -m unittest` | 38 passed. Covers all five parser modes, UTF-8 spans, lexical and endpoint refusals, ordered lifetime and reads, branch/loop/switch/select phases, cleanup barriers, and all three caps. |
| Weighted size | From the worktree root, `python3 -B scripts/file_size.py scripts/subscript_recall/*.py` | Nine Python files. Largest 407 weighted lines. |
| Sample and whole frame | Four shard commands and two merges in [the run README](../../.audit-out/README.md) | Exact 180/180 frozen sample keys and 1,110/1,110 frame keys. One frame key is recorded as a Go parse recovery. All shared sampled summary fields agree with the full-frame rows. |
| Repeatability | Replay all four shard commands with `PYTHONHASHSEED=71`, merge and `cmp` each artifact against the original | Both files byte-identical. |
| Span and cap reconciliation | Independent `python3 -B` JSON audit | All detailed spans were nonempty, in range, matched their source line and decoded as UTF-8. Raw counts, surviving pair rows, fan-in, owner counts and would-add rows reconciled. |
| Source revision | `git --no-optional-locks rev-parse HEAD` and `git --no-optional-locks ls-tree -rz HEAD` in the five Git-backed source roots | 563 analyzed file bytes matched pinned blob IDs. The other 36 files were hashed from the supplied X export. |

Raw graph locations and multiplicity, assembled endpoint bytes and owner mapping,
occupied pairs, product RD availability, public CFG and query reach stay unverified.
The corpus run uses direct source reads and existing census outputs. Cargo, the
full Prism test suite and Tier-A remain pending before merge to main. The brief
covers the measurement code and its unittest suite. No product code was edited.
