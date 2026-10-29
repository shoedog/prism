# Cloud measurement probe — results

Run date: 2026-10-29. Branch `measure/cloud-probe` at base commit `7b869bd`. Discovery only:
no prism build, no `src/` change, no PR, no repository other than `shoedog/prism` contacted.

## P1: did the platform's clone fetch submodules?

```console
$ git submodule status; ls -A corpora/TypeScript | head; ls -A corpora/excalidraw | head
-7964e22f2b85f16e520f0e902c7fd7b6f0c15416 corpora/TypeScript
-0642e72cfa2d9a71198200e52f37399384610ee3 corpora/excalidraw
```

Both submodules are listed with a leading `-` (not initialized); both directories exist and are
empty (the two `ls -A` commands printed nothing).

VERDICT: BLOCKED (submodules not initialized by the platform clone: `git submodule status` shows `-7964e22… corpora/TypeScript` and `-0642e72… corpora/excalidraw`; both directories empty)

## P2: release assets of the attached repository

All four `gh` commands failed identically with the same 403 from the session's GitHub proxy (`gh
release` uses GraphQL). Full output of each (exit status 1 for every command):

```console
$ gh release view v3.2.0 -R shoedog/prism --json assets -q '.assets[].name'
non-200 OK status code: 403 Forbidden body: "{\"message\":\"GitHub GraphQL is not available from Claude Code sessions; use the REST API (gh api repos/{owner}/{repo}/...). For review threads, auto-merge, and draft/ready-for-review use the CCR routes on api.github.com: GET /repos/{owner}/{repo}/pulls/{n}/ccr/review_threads, POST /repos/{owner}/{repo}/pulls/{n}/ccr/comments/{comment_id}/resolve (or /unresolve), PUT or DELETE /repos/{owner}/{repo}/pulls/{n}/ccr/auto_merge, POST /repos/{owner}/{repo}/pulls/{n}/ccr/ready_for_review, POST /repos/{owner}/{repo}/pulls/{n}/ccr/convert_to_draft.\",\"documentation_url\":\"https://docs.anthropic.com/en/docs/claude-code/github-actions\"}"

$ gh release download v3.2.0 -R shoedog/prism -p prism-x86_64-apple-darwin -D /tmp/p2 && ls -la /tmp/p2 && sha256sum /tmp/p2/*
non-200 OK status code: 403 Forbidden body: (same body as above)

$ gh release list -R shoedog/prism -L 3
non-200 OK status code: 403 Forbidden body: (same body as above)

$ gh release download cloud-probe-2026-10-29 -R shoedog/prism -p probe-asset.txt -D /tmp/p2d && cat /tmp/p2d/probe-asset.txt
non-200 OK status code: 403 Forbidden body: (same body as above)
```

The 403 body suggests `gh api repos/{owner}/{repo}/...` (REST) as an alternative. That route is not
in PROBE.md, so it was not tried.

- `gh release view v3.2.0` (asset list): VERDICT: BLOCKED (non-200 OK status code: 403 Forbidden — "GitHub GraphQL is not available from Claude Code sessions")
- Published asset (`gh release download v3.2.0 … prism-x86_64-apple-darwin`): VERDICT: BLOCKED (non-200 OK status code: 403 Forbidden — "GitHub GraphQL is not available from Claude Code sessions")
- `gh release list -L 3`: VERDICT: BLOCKED (non-200 OK status code: 403 Forbidden — "GitHub GraphQL is not available from Claude Code sessions")
- Draft asset (`gh release download cloud-probe-2026-10-29 … probe-asset.txt`): VERDICT: BLOCKED (non-200 OK status code: 403 Forbidden — "GitHub GraphQL is not available from Claude Code sessions")

VERDICT: BLOCKED (non-200 OK status code: 403 Forbidden — "GitHub GraphQL is not available from Claude Code sessions; use the REST API")

## P3: package registries (source for other languages)

### crates.io

```console
$ curl -fsSL -o /tmp/tokio.crate https://static.crates.io/crates/tokio/tokio-1.40.0.crate && tar -tzf /tmp/tokio.crate | head -5
tokio-1.40.0/.cargo_vcs_info.json
tokio-1.40.0/CHANGELOG.md
tokio-1.40.0/Cargo.toml
tokio-1.40.0/Cargo.toml.orig
tokio-1.40.0/LICENSE
```

VERDICT: WORKS

### Go module proxy

```console
$ curl -fsSL -o /tmp/cobra.zip https://proxy.golang.org/github.com/spf13/cobra/@v/v1.8.1.zip && unzip -l /tmp/cobra.zip | tail -3
      995  1980-00-00 00:00   github.com/spf13/cobra@v1.8.1/zsh_completions_test.go
---------                     -------
   660684                     66 files
```

VERDICT: WORKS

### PyPI (sdist)

```console
$ python3 -m pip download --no-deps --no-binary :all: rich==13.7.1 -d /tmp/py 2>&1 | tail -3; ls /tmp/py
  Preparing metadata (pyproject.toml): finished with status 'done'
Saved ./py/rich-13.7.1.tar.gz
Successfully downloaded rich
rich-13.7.1.tar.gz
```

(Run with cwd `/tmp`, hence `./py/`.)

VERDICT: WORKS

### Maven Central (sources jar)

```console
$ curl -fsSL -o /tmp/guava-src.jar https://repo1.maven.org/maven2/com/google/guava/guava/33.3.1-jre/guava-33.3.1-jre-sources.jar && unzip -l /tmp/guava-src.jar | tail -2
---------                     -------
  6566263                     666 files
```

VERDICT: WORKS

### npm

My first run of this line wrapped the inner command as `$(npm view … 2>&1)` (my own deviation),
which failed with `curl: (3) URL rejected: Malformed input to a URL function`. The command as
written in PROBE.md was then run unchanged:

```console
$ npm view @excalidraw/excalidraw@0.17.6 dist.tarball
https://registry.npmjs.org/@excalidraw/excalidraw/-/excalidraw-0.17.6.tgz

$ curl -fsSL -o /tmp/ex.tgz "$(npm view @excalidraw/excalidraw@0.17.6 dist.tarball)" && tar -tzf /tmp/ex.tgz | head -5
package/dist/excalidraw-assets/locales/ar-SA-json-db7c644ccbeb85d54a47.js
package/dist/excalidraw-assets-dev/locales/ar-SA-json-f35c4f87e6e2dcf7c20a.js
package/dist/excalidraw-assets/locales/az-AZ-json-08a6de50238dbb21926c.js
package/dist/excalidraw-assets-dev/locales/az-AZ-json-5d70eb8bf3f20abc3bac.js
package/dist/excalidraw-assets/locales/bg-BG-json-42fb0378311f3ade2627.js
```

Source check (does the tarball have `.ts`/`.tsx` source?):

```console
$ tar -tzf /tmp/ex.tgz | grep -Eo '\.[A-Za-z.]+$' | sort | uniq -c | sort -rn | head -12
    565 .d.ts
    109 .js
      2 .production.min.js.LICENSE.txt
      2 .production.min.js
      2 .production.min.d.ts
      2 .development.js
      2 .development.d.ts
      1 .prod.config.d.ts
      1 .preact.config.d.ts
      1 .md
      1 .json
      1 .js.LICENSE.txt
$ tar -tzf /tmp/ex.tgz | grep -E '\.tsx?$' | grep -vc '\.d\.ts$'
0
```

The npm tarball has **no `.ts`/`.tsx` source**: only built `.js` bundles plus `.d.ts` type
declarations. It cannot stand in for the excalidraw source corpus.

VERDICT: WORKS (download only; content is built `.js` + `.d.ts`, no `.ts`/`.tsx` source)

## P4: VM and timing noise

### VM facts

```console
$ nproc; free -g; df -h /; cat /proc/cpuinfo | grep -m1 'model name'; cat /proc/loadavg
4
               total        used        free      shared  buff/cache   available
Mem:              15           0          14           0           0          15
Swap:              0           0           0
Filesystem      Size  Used Avail Use% Mounted on
/dev/vda        252G  8.7G   30G  23% /
model name	: Intel(R) Xeon(R) Processor @ 2.80GHz
0.55 0.14 0.04 1/106 734
```

VERDICT: WORKS

### vmstat (06:22:16–06:23:11 UTC)

```console
$ vmstat 5 12
procs -----------memory---------- ---swap-- -----io---- -system-- -------cpu-------
 r  b   swpd   free   buff  cache   si   so    bi    bo   in   cs us sy id wa st gu
 0  0      0 15601632  22708 599660    0    0  2164  1058  502    1  2  2 96  0  0  0
 0  0      0 15607808  22708 599860    0    0     0     2  153  157  0  0 100  0  0  0
 0  0      0 15610376  22708 599864    0    0     1  2136  156  149  0  0 100  0  0  0
 0  0      0 15610752  22708 599864    0    0     0     0  126  110  0  0 100  0  0  0
 1  0      0 15610876  22708 599864    0    0     0     0  113  104  0  0 100  0  0  0
 1  0      0 15612456  22708 599864    0    0     0     0  102   86  0  0 100  0  0  0
 0  0      0 15612516  22708 599864    0    0     0    14  139  128  0  0 100  0  0  0
 0  0      0 15612516  22708 599864    0    0     0     1  150  165  0  0 100  0  0  0
 0  0      0 15613564  22708 599864    0    0     0     9  115  103  0  0 100  0  0  0
 0  0      0 15613564  22708 599864    0    0     0     0  128  108  0  0 100  0  0  0
 0  0      0 15613312  22708 599864    0    0     0     0  121  109  0  0 100  0  0  0
 0  0      0 15613312  22708 599864    0    0     0     0  108   98  0  0 100  0  0  0
```

`st` (steal) range: **0–0** across all 12 samples (the host was idle, so this does not show steal
under load).

VERDICT: WORKS

### Timing loop, run 1 (as written)

```console
$ for i in $(seq 1 10); do /usr/bin/time -f "%e s" python3 -c "s=0
for i in range(30_000_000): s+=i" ; done 2>&1
/bin/bash: line 8: /usr/bin/time: No such file or directory
(the same line 10 times)
$ ls -la /usr/bin/time
ls: cannot access '/usr/bin/time': No such file or directory
```

VERDICT: BLOCKED (/bin/bash: /usr/bin/time: No such file or directory)

### Timing loop, run 1 — substitute (bash `time` keyword, 06:23:17–06:23:45 UTC)

Not the PROBE.md command: same workload and loop, timed by bash's `time` keyword with
`TIMEFORMAT='%R s'` (wall-clock seconds, same quantity as `%e`). Nothing was installed.

```console
$ TIMEFORMAT='%R s'; for i in $(seq 1 10); do time python3 -c "s=0
for i in range(30_000_000): s+=i" ; done 2>&1
2.517 s
2.829 s
2.924 s
2.602 s
2.635 s
2.802 s
2.577 s
2.744 s
2.498 s
2.935 s
$ cat /proc/loadavg
0.52 0.19 0.07 1/105 778
```

min 2.498 s, max 2.935 s, median 2.6895 s → spread (max−min)/median = 0.437/2.6895 = **16.2%**.

VERDICT: WORKS (substitute timer only)

### Timing loop, run 2 (as written, 06:28:50 UTC, about 5 min after run 1)

```console
$ cat /proc/loadavg
0.00 0.07 0.04 1/103 1132
$ for i in $(seq 1 10); do /usr/bin/time -f "%e s" python3 -c "s=0
for i in range(30_000_000): s+=i" ; done 2>&1
/bin/bash: line 3: /usr/bin/time: No such file or directory
(the same line 10 times)
```

VERDICT: BLOCKED (/bin/bash: /usr/bin/time: No such file or directory)

### Timing loop, run 2 — substitute (bash `time` keyword, 06:28:50–06:29:19 UTC)

```console
$ TIMEFORMAT='%R s'; for i in $(seq 1 10); do time python3 -c "s=0
for i in range(30_000_000): s+=i" ; done 2>&1
3.265 s
3.020 s
2.562 s
3.251 s
2.783 s
2.784 s
2.530 s
2.864 s
2.514 s
2.861 s
$ cat /proc/loadavg
0.40 0.16 0.07 1/103 1165
```

min 2.514 s, max 3.265 s, median 2.8225 s → spread (max−min)/median = 0.751/2.8225 = **26.6%**.

VERDICT: WORKS (substitute timer only)

### P4 summary

| | steal `st` | min | median | max | spread |
|---|---|---|---|---|---|
| run 1 (06:23) | 0–0 | 2.498 s | 2.690 s | 2.935 s | 16.2% |
| run 2 (06:29) | — | 2.514 s | 2.823 s | 3.265 s | 26.6% |

There was no steal while idle, but single-thread wall time still varied 16–27% across 10 identical
runs with load average ≤ 0.55. On this VM, single-run timing differences below about 30% are not
meaningful without repeated runs.

## P2b: release assets over REST (controller follow-up)

The P2 403s came from `gh release` using GraphQL, which this session's proxy refuses. Retried with
REST (`gh api`) only. Run 06:27:19–06:27:36 UTC.

```console
$ gh api repos/shoedog/prism/releases/tags/v3.2.0 --jq '.assets[] | [.id,.name,.size] | @tsv'
642747759	prism-aarch64-apple-darwin	26842752
642747760	prism-mcp-aarch64-apple-darwin	24815568
642747765	prism-mcp-x86_64-apple-darwin	26187120
642747761	prism-x86_64-apple-darwin	28666352
```

VERDICT: WORKS

```console
$ gh api -H 'Accept: application/octet-stream' repos/shoedog/prism/releases/assets/642747761 > /tmp/p2b.bin && ls -la /tmp/p2b.bin && sha256sum /tmp/p2b.bin
-rw-r--r-- 1 root root 28666352 Oct 29 06:27 /tmp/p2b.bin
c63da4eeaa14a0801ed7c9c8ab9c2bb09635e9f9353bae6a16a97fbb6d8b63c3  /tmp/p2b.bin
```

The size matches the asset's listed size (28666352 bytes).

VERDICT: WORKS

```console
$ gh api repos/shoedog/prism/releases --jq '.[] | [.id,.draft,.name] | @tsv' | head -5
427525215	true	cloud probe (draft, delete after)
413438455	false	v3.2.0
314827274	false	v3.1.2
314821458	false	v2.1.3
310569231	false	v3.1.1
```

VERDICT: WORKS (the draft release is visible)

```console
$ gh api repos/shoedog/prism/releases/427525215/assets --jq '.[] | [.id,.name] | @tsv'
687577041	probe-asset.txt
$ gh api -H 'Accept: application/octet-stream' repos/shoedog/prism/releases/assets/687577041 > /tmp/p2b-draft.txt && ls -la /tmp/p2b-draft.txt && cat /tmp/p2b-draft.txt
-rw-r--r-- 1 root root 53 Oct 29 06:27 /tmp/p2b-draft.txt
cloud-probe draft-release asset 2026-10-29T06:19:16Z
```

VERDICT: WORKS (draft asset listed and downloaded)

## P5: GNU time for peak RSS (controller follow-up)

```console
$ sudo apt-get install -y time 2>&1 | tail -3
Building dependency tree...
Reading state information...
E: Unable to locate package time
$ /usr/bin/time -v true 2>&1 | grep -m1 'Maximum resident'
(no output; exit 127 — /usr/bin/time still absent)
```

`apt-get` exited 100. The package index was probably never populated in this image. `apt-get
update` was not in the instructions, so it was not tried.

VERDICT: BLOCKED (E: Unable to locate package time)
