# Slice 3 final Rust receipt

- Source: `440c6f4e2dc0bc24521d67b172fa51226d007716`, tree `cce3d1e19192f8b7085ca1145fa4825b287ca0e4`.
- Target: `/private/tmp/prism-post316-orchestration/slice3/target-rust-final`.
- Pinned widest env: `PRISM_TYPESCRIPT=/private/tmp/prism-post316-orchestration/public-inputs/typescript-5.9.3/package/package/lib/typescript.js`; `PRISM_CALLABLE_PROFILES=/private/tmp/prism-post316-orchestration/public-inputs/callable-authority-profiles/profiles`.

| Command | Result | Log SHA-256 |
|---|---|---|
| `cargo test --offline --no-fail-fast` | 4550/0/1 | `4be4e0116757705e8209eb92e0ba3ea5435719c0f5958778e1d5ee2ba8c42360` |
| `cargo test --offline --no-fail-fast --features mcp` | 4743/0/1 | `234f48154c9bd19126040efc0ca8b580f39a9938647c61fa98a34120f5752f8b` |
| widest `mcp detached-owner-audit` | 4766/0/1 | `0027393102447c708fe70c0d713be7ca567823a2a67aba9951e3ef3d41c6531f` |
| examples widest | 32/0/0 | `6b8f4ae05850f3602c25ba66a9e20c828e54834cbc2677a9a73fdb2246b06069` |
| clippy all targets/mcp | exit 0 | `e6033e518d83b8150e168238d09bcc1944c064f104cf4c7a2d79e20f230bf95c` |
| fmt/diff | clean | empty-output SHA |

Default→MCP adds feature-gated tests; MCP→widest adds detached owner-audit coverage. Raw logs remain in `slice3/implementation/logs/`; no large copies made.
