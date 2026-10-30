# Slice 1 final Rust gate receipt

- Frozen commit: `07ed5ceb6098f2b5a21f5c05008a2c823a5617d7`
- Frozen tree: `32be14a0eebecd8b936f94c289f21faeb0526716`
- Worktree: `/private/tmp/prism-post316-slice1`
- Source/test edits during gates: none; final `git status --short` was empty.

| Gate | Result | Log SHA-256 |
|---|---|---|
| `cargo test --offline --no-fail-fast` | GREEN; 29 suites, 4,518 passed / 0 failed / 1 ignored | `904bf86f4f9ad6297c7e64f5b2973c092424d3e17f81d404f42ecb320364230c` |
| `cargo test --offline --no-fail-fast --features mcp` | GREEN; 31 suites, 4,711 passed / 0 failed / 1 ignored | `ef5b3d287f1cd7ccbd423d0d3a2933b6e75a96beb25720794b0cd62fcda34ef6` |
| `PRISM_TYPESCRIPT=... PRISM_CALLABLE_PROFILES=... cargo test --offline --no-fail-fast --features 'mcp detached-owner-audit'` | GREEN; 31 suites, 4,734 passed / 0 failed / 1 ignored | `05c7e5e5a5c52b60a05924334f6f6a2a90629374d4a631448b27cc1c0db4d90e` |
| `PRISM_TYPESCRIPT=... PRISM_CALLABLE_PROFILES=... cargo test --offline --examples --features 'mcp detached-owner-audit'` | GREEN; 4 suites, 32 passed / 0 failed / 0 ignored | `5dd836782bde12b7d552d7abd20eb66dd48297d22175e928dcc1da6561272e96` |
| `cargo clippy --offline --all-targets --features mcp -- -W clippy::all` | GREEN exit 0; 179 warnings, 134 duplicates | `b4cdf0e60ae6a1268944bf2e7931378677923b7e66d6f5cbe6e65bbe5c05fa52` |
| `cargo fmt --all -- --check` | GREEN; empty output | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `git diff --check` | GREEN; empty output | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

The first widest-feature invocation omitted the mandatory compiler/profile environment and was inadmissible as behavior evidence: 23 tests refused with `NotPresent`, while 4,711 passed and 1 was ignored. Before the single environmental retry, `owner-audit-env-hypothesis.md` recorded the expected result, falsifier, and missing-profile alternative. The retry supplied:

- compiler: `/private/tmp/prism-post316-orchestration/public-inputs/typescript-5.9.3/package/package/lib/typescript.js`
- profiles: `/private/tmp/prism-post316-orchestration/public-inputs/callable-authority-profiles/profiles`

All 23 refusals became passes and no admitted behavior failure remained, so a same-environment base attribution run was not applicable. The invalid-run log is preserved as `full-mcp-owner-audit-invalid.log`; the accepted log is `full-mcp-owner-audit.log`.

The default log is `../full-default-final.log`; all other named logs are adjacent to this receipt.
