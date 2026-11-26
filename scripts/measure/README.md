# Measurement kit

Run from `scripts/` with Python 3.9 or later. The kit uses the Python
standard library, Git for Git-backed pins, and the repository memory guard.

```sh
python3 -m measure corpora list
python3 -m measure corpora verify EX T --corpora-root /path/to/clones
python3 -m measure run --binary /path/to/prism --corpus EX \
  --corpora-root /path/to/clones --out /path/to/new-run \
  --expect-version 1234567 --prediction /path/to/prediction.md
```

Use `--manifest FILE` before the subcommand for a different registry. Corpus
lookup tries each `--corpora-root` in order, then `PRISM_MEASURE_CORPORA`
(a path list separated by the host's path separator), then the kit checkout.
Within each location the registry supplies ordered checkout paths and a query
root. The first existing query root must pass admission. Git pins require the
registered commit, a clean query root and a match to the raw pinned blobs.
Filter-converted bytes, hard links, gitlinks and links that escape the root,
enter `.git` or form cycles are refused. Extracted trees require a complete
`tree_sha256` for that query root, including literal symlink targets. The
perf-cloud manifest is an additional check when supplied. `RS` and `R`
select the same ripgrep registration. `GIN` selects gin and `G` selects guava.

Register each `SB.<class>.<entry>` with the acquired package's own tree digest
and package path. The registry ships the public Git pins only: it holds no `SB`
entries, so `run --corpus SB.<class>.<entry>` against it exits 2 (unknown key)
until S6b registers digests computed from the authenticated acquired trees (S1 verifies any
per-package registration now; default `SB` registration follows authenticated acquisition).
A manifest supplied with `--manifest` can register them earlier. The source-only
`R4B` fixture is registered in S2, and package and extraction registrations
require digests from the actual acquired trees. An extracted tree with a
perf-cloud `manifest_sha256` must hold no `.git` file or directory, because the
reference checksum counts them and the section 2 digest does not.

`run` requires an absent or empty output directory outside the query root.
It retains a private source snapshot without Git administrative files or
sibling source directories. The receipt binds the specified file and link
bytes and also records a separate fingerprint of directories, file modes and
link resolution state. Checks before and after execution detect persistent
changes. In-between changes that are completely restored can escape these
checks. Run receipts record execution evidence. Product credit and
qualification of ambient inputs belong to the consuming gate.

By default the guard uses an 8 GiB RSS limit. `--no-bytes` disables the edge
byte switch, `--sites` adds the call-site dump, and repeatable
`--trace-env NAME=VALUE` applies only to the edge command. Version pins match
the version token or at least seven hex digits of the reported commit. A
build whose commit token carries a `-dirty` suffix cannot match a commit pin;
pin its version token or build from a clean tree. The receipt also records the
guard's SHA-256, the SHA-256 of the host-quiet helper at `../hostquiet/host_quiet.py` relative to the guard file (null when absent or not a regular file; other dependencies of a custom guard are not bound, see open-n3-21) and
`kit_dirty`, which marks Git-visible edits to the kit (tracked changes and untracked
non-ignored files under the kit directories) and, as a fallback for tracked edits that
`assume-unchanged` or `skip-worktree` index flags hide, any tracked kit path that cannot be shown to match the
recorded commit's blobs (byte or symlink-target difference, missing file, type mismatch or comparison error counts as dirty). Mode-only working-tree edits hidden from `git status` by those flags and ignored untracked files are not
seen (known-s1-2). The guard and its helper
are rechecked before each launch and after the last command; a change fails the
run (exit 4; 75 when a later attempt reports host quiet after an analysis has launched); once an analysis has launched the failed receipt is retained, before that there is none. The guard must be a readable
executable regular file. If a command changes or deletes an artifact after
capture, the failed receipt lists `artifact_drift` entries with the captured and
retained digests (retained is null when the file is gone) and publishes the
surviving files.
See `run --help` for the full interface.

Successful runs return 0. Usage and occupied output errors return 2, corpus
and version pin refusals return 3, and command, guard or input failures return
4. A live host-quiet marker reported by the guard returns 75. A binary's own
exit 75 is a command failure. Finalized runs keep output digests, the raw
guard exit and structured status for each attempt, and the prediction digest
captured before version. A refusal before an analysis launches leaves no
measurement receipt. Once analysis starts, a failure retains partial
artifacts and publishes the failed receipt last.

A worker still running in the command's process group after the command exits is
terminated and reported as `guard_error` (exit 1), never as completion. Standalone
use has the same effect: the guard terminates the surviving group and exits 1
even though the command itself succeeded, so a command that deliberately leaves
a background worker behind must not be run under it.

The guard also runs directly as `scripts/memguard/memguard.sh LIMIT COMMAND ...`
from the repository root. `--status-fd FD` precedes the limit when a caller
supplies a dedicated status descriptor. The kill log uses `MEMGUARD_LOG`, then
`XDG_STATE_HOME/prism/memguard-kills.log` or the corresponding path under
`~/.local/state`.

Run the hermetic tests from the repository root.

```sh
python3 -m unittest discover -s scripts/measure -t scripts -p 'test_*.py'
python3 -m unittest discover -s scripts/memguard -p 'test_*.py'
```

## Known limits of S1 (accept-list)

Round 3 of the S1 review (sol) found four more instances of one class: edge cases around trusting the guard, its quiet helper, the kit's own files and the product's staged artifacts when those are special files or edited by an adversarial party. Opus approved the S1 head before these. The boundary: S1 binds regular-file bytes of the executable, guard and helper (rechecked at each launch and at completion), the corpus snapshot, and the staged artifacts; it detects ordinary replacement, rewrite and deletion. It does not defend against a guard, helper, kit file or product that is itself adversarial (FIFO or device substitution, mode-only edits under suppressing index flags). Those four instances are the rows `known-s1-1..4` in FORMATS-OPEN-ROWS.md, each with an owner. A consumer that must resist an adversarial product or guard must wait for those rows.
