# Controller feasibility probe (2026-09-24)

- Registry `dist.integrity` fetched from `https://registry.npmjs.org/<name>/<version>` for all six npm packages. The
  metadata SHA-256 values are in `registry-integrity.json`, and the metadata is preserved at
  `~/prism-evidence/gate-input-durability/registry-metadata/`.
- For @types/react 19.0.10, csstype 3.1.3, and typescript 5.9.3, the registry integrity is **byte-identical** to the
  Excalidraw `0642e72c` `yarn.lock` SRIs. That lock's SHA-256 is pinned in `docs/eval/post317-input-custody.md:29`.
- The react18 SRIs come from registry metadata:
  - @types/react 18.3.31: `sha512-vfEqpXTvwT91yhmwdfouStN2hSKwTvyRs8qpLfADyrq/kxDw0hZM7Wk9Ug1FELj8hIby+S/+kQCSRFF32nv2Qw==`
  - csstype 3.2.3: `sha512-z1HGKcYy2xA8AGQfwrn0PAy+PB7X/GSj3UVJW9qKyn43xWa+gl5nXmU4qqLMRzWVLFC8KusUX8T/0kCiOYpAIQ==`
  - @types/prop-types 15.7.15: `sha512-F6bEyamV9jKGAFBEmlQnesRPGOQqS2+Uwi0Em15xenOxHaf2hv6L8YCVn3rPdPJOiJfPiCnLIRyvwVaqMY3MIw==`
- Downloading all six tarballs, verifying each SRI, then running `tar -xzf … --strip-components=1` into
  `profiles/<p>/node_modules/<name>` gave these results. Each tree hash uses the exact formula in
  `verify-callable-authority.mjs:15-22`.

| Profile | Files | Tree hash | Result |
|---|---|---|---|
| react19 | 29 | `49c6c7a3cde29161a5af224dede5e4442295f9251ef4c694699341ed3682baad` | EQUALS pin |
| react18 | 24 | `7b8bbdc844cd38cbf691987a229858e4006273c3d3b3c4d8c8fef70267859b34` | EQUALS pin |

- TypeScript: `package/lib/typescript.js` SHA-256 is `3ae902c9…7675`, which EQUALS `COMPILER_HASH`.
- Tarball sizes are 3 KB – 4.4 MB, for about 4.9 MB in total.
- The owner chose "Re-plan simple v3" after spec v2 hit its 2-round cap non-converging (8 WRONG, then 8 WRONG).
