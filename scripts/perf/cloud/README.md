# Cloud measurement runner

Runs prism timing measurements (phase profile and paired main-versus-candidate deltas) inside a
Claude Code cloud session, so #437 perf measurements need not wait for a local quiet window.
Python 3 standard library only; the tests are hermetic (no network, no cargo).

## One-command cloud brief

Paste into a cloud session on `shoedog/prism` (replace `<ref>` and `<name>`; refs may be SHAs):

> Run `git fetch origin main` first, then
> `python3 scripts/perf/cloud/fetch_corpora.py --corpora T,X --work-dir /work`, then
> `python3 scripts/perf/cloud/measure.py --refs main,<ref> --corpora T,X --rounds 6 --threads 4 --out /work/out`.
> Copy `/work/out/RESULTS.md` and `/work/out/runs/` into `docs/eval/<name>/`, commit them on the
> session branch and push. Do not open a PR. Do not run prism on anything but the fetched corpora.

`fetch_corpora.py` finds the draft release `corpora-v1` of `shoedog/prism` over REST (`gh api`),
downloads the registry's assets, checks `SHA256SUMS`, extracts under `--work-dir` (default `/work`),
checks each corpus's manifest hash where `corpora.json` records one, and refuses on any mismatch.
`measure.py` builds each ref with `cargo build --release --locked --bin prism` (a ref equal to HEAD
reuses the checkout, others get a worktree under the work dir), gates each run on a quiet 1-minute
load average (default below 0.5, wait up to 300 s, the value used is recorded), rotates the ref
order per round, and writes `<out>/runs/` plus `<out>/RESULTS.md`. Use an even `--rounds` for two
refs so both run orders occur equally often. On a gate timeout, `--quiet-on-timeout record`
(default) records the load and flags `gate_failed`, then continues; use `--quiet-on-timeout fail`
to abort instead. The initial idle loadavg is recorded once before setup/builds. Finished runs
are checkpointed in RESULTS.md, marked PARTIAL with a reason on abort. Relax the gate with
`--quiet-threshold`; skip the build with `--prism-bin` (single ref) and use a local directory with
`--corpus-dir KEY=PATH`. Steal ticks come from `/proc/stat` and are recorded as `unavailable` where
it is missing.

The Binaries table records resolved commits and hashes. When local `<ref>` and `origin/<ref>`
differ, the runner uses `origin/<ref>` and records a warning. Reused worktrees must match the
requested commit. Tracked changes refuse a build unless `--allow-dirty` is explicit (recorded
in the environment). With `--prism-bin`, commit identity is `unknown (prism-bin)`.

## Caveats and limits

- `gh release ...` fails in a cloud session (the proxy refuses GraphQL, 403). Use REST: `gh api`.
- `/usr/bin/time` is absent; peak RSS and CPU come from `os.wait4` rusage of the child.
  Peak RSS is a floor that may include the parent's size.
- 4 vCPU, 15 GB RAM, no swap. Single-thread wall time varied 16-27% across idle runs. Cloud wall
  times are not comparable with local history: use phase shares, rankings and paired deltas only.
  Memory measurements stay local.
- A paired verdict `holds in every pair` needs at least 3 pairs and every pair to agree in sign;
  otherwise `mixed` (3+ pairs) or `insufficient pairs (n)`. Each verdict gives its pair count.
  Each metric includes median(paired delta) as a percentage of its base median, beside the
  base command-wall spread. Missing metric values reduce that metric's pair count; zero base
  medians have `n/a` percentages. Gate-failed runs remain included and visibly flagged.

## Adding a corpus (controller only)

Public corpora only; never a private corpus. Upload the archive and its line in `SHA256SUMS` to the
draft release `corpora-v1` (and update its `CORPORA.md`), then add an entry to `corpora.json`:
`{"KEY": {"asset": "<archive>.tar.gz", "root": "<path inside archive>", "manifest_sha256": "<optional>"}}`.
The manifest hash is sha256 of the sha256sum list of `find . -type f` under the root, sorted bytewise.
Extension point, not implemented: registry-sourced corpora (crates.io, proxy.golang.org, PyPI, Maven
are reachable from the session) could be added as another registry entry kind.
