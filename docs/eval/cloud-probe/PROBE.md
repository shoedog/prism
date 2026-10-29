# Cloud measurement probe

Goal: find out which routes can bring pinned measurement corpora into this cloud session, and how noisy
this VM is for timing. This is discovery only. Do not change anything under `src/`, do not open a PR,
and do not build or run prism.

Rules:
- Only the routes listed below. Do not clone, fetch or download any GitHub repository other than
  this session's attached repository. If a step would need that, record "not attempted" and move on.
- Run each command in the foreground with a timeout of at most 5 minutes.
- Record every command and its full output (or the last 30 lines if longer) in
  `docs/eval/cloud-probe/RESULTS.md` under the step's heading. Each step ends with one verdict line:
  `VERDICT: WORKS`, `VERDICT: BLOCKED (<exact error>)` or `VERDICT: NOT ATTEMPTED (<reason>)`.
- After each step, commit and push RESULTS.md to this branch (`measure/cloud-probe`).

## P1: did the platform's clone fetch submodules?
This branch records two submodules, `corpora/TypeScript` and `corpora/excalidraw`. Do not run
`git submodule update` or any command that fetches them; only observe:
```bash
git submodule status; ls -A corpora/TypeScript | head; ls -A corpora/excalidraw | head
```
WORKS only if both directories contain files and `git submodule status` shows no leading `-`.

## P2: release assets of the attached repository
```bash
gh release view v3.2.0 -R shoedog/prism --json assets -q '.assets[].name'
gh release download v3.2.0 -R shoedog/prism -p prism-x86_64-apple-darwin -D /tmp/p2 && ls -la /tmp/p2 && sha256sum /tmp/p2/*
gh release list -R shoedog/prism -L 3
gh release download cloud-probe-2026-10-29 -R shoedog/prism -p probe-asset.txt -D /tmp/p2d && cat /tmp/p2d/probe-asset.txt
```
The last release is a draft. Record each command's verdict separately (published asset, draft asset).

## P3: package registries (source for other languages)
```bash
curl -fsSL -o /tmp/tokio.crate https://static.crates.io/crates/tokio/tokio-1.40.0.crate && tar -tzf /tmp/tokio.crate | head -5
curl -fsSL -o /tmp/cobra.zip https://proxy.golang.org/github.com/spf13/cobra/@v/v1.8.1.zip && unzip -l /tmp/cobra.zip | tail -3
python3 -m pip download --no-deps --no-binary :all: rich==13.7.1 -d /tmp/py 2>&1 | tail -3; ls /tmp/py
curl -fsSL -o /tmp/guava-src.jar https://repo1.maven.org/maven2/com/google/guava/guava/33.3.1-jre/guava-33.3.1-jre-sources.jar && unzip -l /tmp/guava-src.jar | tail -2
curl -fsSL -o /tmp/ex.tgz "$(npm view @excalidraw/excalidraw@0.17.6 dist.tarball)" && tar -tzf /tmp/ex.tgz | head -5
```
One verdict per registry. For npm, also record whether the tarball contains `.ts`/`.tsx` source or only built `.js`.

## P4: VM and timing noise
```bash
nproc; free -g; df -h /; cat /proc/cpuinfo | grep -m1 'model name'; cat /proc/loadavg
vmstat 5 12
for i in $(seq 1 10); do /usr/bin/time -f "%e s" python3 -c "s=0
for i in range(30_000_000): s+=i" ; done 2>&1
```
Report: the `st` (steal) column range, the 10 timings, and their spread ((max-min)/median).
Then repeat the 10-timing loop once more about 5 minutes later and report it the same way.

## Finish
Commit and push RESULTS.md. Your final message must contain the full RESULTS.md and the last commit SHA.
