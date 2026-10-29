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
