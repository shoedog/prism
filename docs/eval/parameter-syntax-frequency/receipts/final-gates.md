# Parameter syntax-frequency final-gates receipt

**Candidate:** `f79bb95485940bcb1418a2510cd9ec73a65ca37f`  
**Tree:** `c5c48a54f2b4ba989c9c3ad7d2a4b4f36a5aca29`  
**Final source binding:** `/private/tmp/prism-post318-parameter-frequency/repair-freeze/source-manifest.md` SHA-256 `2eb75cd65acf7220279ef264540a3b45b5b47f6cbcf07fe8c7b8bf8eb2b23740`; final core acceptance `/private/tmp/prism-post318-parameter-frequency/review-round2/CORE-ACCEPTANCE.md` SHA-256 `69cfb7d4695c99899d2bd6c2bb910ce7534f7e9cd104c8901a2dd9ab73bbd676`.

| Frozen owned path | SHA-256 |
|---|---|
| `scripts/parameter-frequency/index.mjs` | `d1c77914d4db177af010e52efc2e308eb5bf6a9269ea9c50753dca5b69f66856` |
| `scripts/parameter-frequency/index.test.mjs` | `2d930123b847815f18aaf233c2250b5fb1b3afd398af3427803f89164d3cdc56` |
| `scripts/parameter-frequency/README.md` | `43e8374a8c60ecf60e72f01372a03824378ffbcd935abcbc5e3cd0bd9b3d8d13` |

## Fixed public population

Pinned TypeScript 5.9.3 SHA-256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675` and fixed manifest SHA-256 `f8ebbdd79ca01e5cdb675616b913f1fbb549d45378ab1047db84844a3bc8e696` passed `validate_inputs.py`: 414 members, 5,036,151 source bytes, five declaration-only members, and 62 test paths. The preflight log is `preflight.log` SHA-256 `73c10523e8095a8384eb163b0c6857dd940b7f5d21916968864c15ddca2832a8`.

Two fresh-output cold runs of:

```sh
node scripts/parameter-frequency/index.mjs \
  --root /private/tmp/prism-post317-measurement-inputs/source \
  --manifest /private/tmp/prism-post317-planning/p2/input-hash-manifest.json \
  --manifest-sha256 f8ebbdd79ca01e5cdb675616b913f1fbb549d45378ab1047db84844a3bc8e696 \
  --typescript /private/tmp/prism-post316-orchestration/public-inputs/typescript-5.9.3/package/package/lib/typescript.js \
  --out <fresh-output>
```

produced byte-identical 1,677,371-byte packets, each SHA-256 `e953121bbcdda492bb651e4ef8f27b9af3de7a0221ee2a98fcda611645a7896d`. `cold.json` and `repeat.json` are retained here.

Independent recomputation from every emitted file/callable/parameter row passed: 414 file rows, 4,600 unique parameter identities, five frequency rows, zero diagnostics, 437 excluded bodyless signatures, and 5,494 body-bearing callables. Receipt `recomputation.json` SHA-256 `6f379b664c857041effe613a129656da56b57e986fbacadaad4874fd554d5e02`.

| Stratum `[kind, declaration_only, test_path, parse_state]` | files | bodyless | callables | parameters |
|---|---:|---:|---:|---:|
| `Tsx,false,false,clean` | 195 | 220 | 2,175 | 2,154 |
| `Tsx,false,true,clean` | 46 | 3 | 1,245 | 203 |
| `TypeScript,false,false,clean` | 152 | 202 | 1,638 | 2,113 |
| `TypeScript,false,true,clean` | 16 | 0 | 436 | 130 |
| `TypeScript,true,false,clean` | 5 | 12 | 0 | 0 |

## Control and regression gates

| Gate | Result | Evidence |
|---|---|---|
| Parameter-frequency focused integrity/mutation controls | 21 pass / 0 fail / 0 skip | `parameter-frequency-focused.log` SHA-256 `1a9df02133083f114a90483137c1ba011916bf19ccd7cdc2a423391ae35593d1` |
| Full active Node population | 806 pass / 0 fail / 1 skip across 47 modules | `node-full-retry.log` SHA-256 `8eb84c49db653fa384908df1f2c1e3fee779eb82166b8dbdc23e2b9b640825ed`; `node-remaining.log` SHA-256 `7cd07e33990fd71f18ce1619697f9e392eff27dcf0487884a83ae2fe61345e6d`; exact inventory `node-complete-inventory.txt` SHA-256 `74b4953b3e5b00adb58625459b3391999cb02fb8744e9f7d55eda7bca11f98e1` |
| Default Rust | 29 suites, 4,559 pass / 0 fail / 1 ignored | `rust-default.log` SHA-256 `b07f48a0c1ac176de838d212018535881cc5c645a0f14eec12f8f16a91089fe3` |
| `cargo fmt --all -- --check` | pass | `fmt.log` SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `git diff --check` | pass | `diff-check.log` SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

The final Node population is 41 modules in the callable/grammar/new syntax-frequency subset plus six explicitly selected docs/eval modules. The first subset log is 749/0/1; the six remaining modules are 57/0/0; their aggregate is 806/0/1 and matches the preceding 785/0/1 population plus the 21 new syntax-frequency controls.

## Setup and exclusions

The first broad Node attempt is preserved in `node-full.log` SHA-256 `08e721ae3cb59d943e9a576eb9097046240a7c1c3ae9b375dade49e55065d4c4`: it is **INVALID**, 609 pass / 6 file-level assertion failures / 1 skip, because six modules required but did not receive explicit compiler/profile/native bindings. It supplies no behavioral conclusion. The one diagnosed retry used pinned compiler, profile archives, and an isolated helper built by `CARGO_TARGET_DIR=<receipt>/target cargo build --offline --example project_membership_census`; frozen helper SHA-256 `4ec8b8654b976cfc511a730a7694d7d686a2963a76a38fa83b7d2801dc0f0482`; validated retry preflight is `node-retry-preflight.log`.

The three-test historical `docs/eval/receiver-closure/audit-imported-props-source.test.mjs` module is excluded because all five required `PRISM_AUDIT_*` custody inputs, including `SITES`, are unavailable; it is outside the 47-module active inventory. The sole final Node skip is the grammar tamper-authentication branch, which expressly requires an already-acquired `PRISM_GRAMMAR_ARCHIVES` directory and does not download. The syntax-frequency scope does not trigger Tier-A, Python, native authority, TypeScript Program/type-checking, live adoption, or full multi-corpus gates. The public observation is compiler syntax only and never establishes parameter support, owner/slot/entry/call behavior, runtime demand, readiness, accuracy, or a production admission.
