# Recovered public input: Excalidraw 0642e72c (2026-09-24)

**Why.** The original custody root `/private/tmp/prism-post317-measurement-inputs/` was purged by the macOS daily
`/tmp` cleaner: 0 of the 11 selected files remained, and the archive and manifests were gone. The input was recovered
here, which is durable (outside `/tmp`).

**Authority.** `docs/eval/post317-input-custody.md` on `main`, and
`docs/superpowers/plans/2026-09-18-post317-next-increments/inputs/02-excalidraw-input-hash-manifest.json`
(SHA-256 `f8ebbdd7…c8e696`).

| Check | Result |
|---|---|
| Archive re-downloaded from `https://api.github.com/repos/excalidraw/excalidraw/tarball/0642e72cfa2d9a71198200e52f37399384610ee3` | SHA-256 **`0a5a136c1330e87559767a7768b053dab74f7bba336e0fd7ed429377022b141d`** (exact pin match), 35,599,041 bytes |
| Safe extraction (validated before extraction) | 1,369 entries, 1,229 regular files, 140 dirs, 54,407,283 expanded bytes, one root (`excalidraw-excalidraw-0642e72`), 0 links, no absolute or `..` paths. Matches the pinned counts |
| Control files | 5/5 match the pinned SHA-256 values: `package.json`, `tsconfig.json`, `yarn.lock`, `packages/fractional-indexing/tsconfig.json`, `packages/tsconfig.base.json` |
| 414-member census population | 414/414 match SHA-256 and bytes; 5,036,151 bytes total |
| 11 native-gap site members (`SITE-MANIFEST.json`, `789352a5…`) | 11/11 match; also 11/11 equal the git blobs at the pinned commit in `~/code/bench-repos/excalidraw` |
| Full-manifest aggregate `afb0c3e9…` | **Not reproduced.** Its exact serialization is not recorded, and the original file was purged. Subsumed by the exact archive-hash match. A recomputed per-file manifest (`<sha256>  <path>`, C-sorted) is `source-file-manifest.sha256`, SHA-256 `1a6c4397bfacafc0dc2651d1f65fe4dd7afed60a0fa21f8f928061b9e20968f0` |

- Source root for tools: `/Users/wesleyjinks/prism-evidence/inputs/excalidraw-0642e72c/source`
- The selected-file subset is also at `/Users/wesleyjinks/prism-evidence/native-positional-gap/public-inputs-selected/`.
