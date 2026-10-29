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
