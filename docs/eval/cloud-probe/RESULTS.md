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
