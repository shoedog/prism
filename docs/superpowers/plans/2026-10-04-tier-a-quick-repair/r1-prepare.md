# R1 reproduction inputs and commands

This repair is relative to `595430e5545b13529bd9da2412342457da597630`,
`feat/tier-a-quick-repair`. Controller owns commits. No acquisition is required.

Use the existing immutable Rust and Excalidraw inputs from prepare-inputs.md.
For Node, use the already acquired SecBench cache at
`~/prism-evidence/inputs/secbench-pkgs`, manifest SecBench commit
`5d362353550a8baa42bba34edd26e5fb86d41b60`.
The selected packages are `command-exists_1.2.2`, `find-process_1.4.4`,
`node-http-server_8.1.2`, `convict_6.0.0`, `algoliasearch-helper_3.6.0`.
They span process invocation/discovery, HTTP serving, configuration and search.
This is a vulnerable-package Node stratum, not a representative estimate for
all JavaScript projects. Build/doc scripts from Excalidraw are retained under
an explicit `--corpus excalidraw-js` for historical comparison.

Run the retained deterministic preparation script:

```sh
python3 ~/prism-evidence/meas/tiera/repair-r1/prepare-node.py
```

The script checks each cached tarball SHA against the cache manifest, copies
source to `~/.local/share/prism/corpora/secbench-node-r1/source`, omits tests,
docs/examples/benchmarks/dependencies, and writes a complete source manifest.
Expected manifest SHA256:
`6c9e7e9252d173b60f083dff36734e597b8121943fba0e1d6946a315e4b812d0`.
The committed `eval/manifests/secbench-node-r1.sha256` is the byte check for a
fresh reconstruction: 106 files, 1,144,361 bytes, including licenses and package
metadata. `repair-r1/node-provenance.json` preserves cache package/tarball pins.
Measurement excludes `*/dist/*` browser bundles/minified duplicates as well as
scripts; those bytes remain in the complete source manifest. This leaves 56 JS
source files in the Node runtime stratum. Do not delete them from the byte pin.
Native tsserver is pinned by validity to version 6.0.3; any node_modules directory
inside the source census invalidates the run, including a nested dependency tree.
Automatic package/type acquisition stays disabled.

From the checkout, rebuild before each run. Run from `eval/` with imports bound:

```sh
CARGO_NET_OFFLINE=true cargo build --release
cd eval
PYTHONPATH="$PWD" PYTHONDONTWRITEBYTECODE=1 CARGO_NET_OFFLINE=true \
UV_CACHE_DIR="$HOME/.local/share/prism/uv-cache" \
UV_PROJECT_ENVIRONMENT=/Users/wesleyjinks/code/slicing/eval/.venv \
uv run --offline --no-sync tier-a --lang ts,js --sample 12 \
  --allow-stale-sut --date r1-decision --out-dir <new-evidence-path>
```

`--sample N` is symbols per stratum, not a total. Quick defaults to three symbols
per eligible stratum and is a smoke check. The larger run uses up to 60 TS and
60 JS seeds. CIs are 95% Wilson binomial intervals over collapsed call-site
counts. They are conditional on this oracle frame/sample and do not account for
within-function clustering or stratum sampling design.

For the recorded three fresh quick gates, `repair-r1/run-three.sh` rebuilds
immediately before each invocation and invokes `run-fresh.py`. That wrapper
changes only the Rust corpus path to a manifest-exact external copy under
`repair-r1/rust-fresh/source`, removes that copy's generated target directory
before each invocation, and supplies a new SUT cache. It does not reuse an oracle
session. Every source file remains verified by the pinned manifest before/after.
The wrapper's explicit path override is recorded in each receipt.
Final labels are quick12, quick13 and quick14. Each receipt also hashes every
harness module, the syntax helper and corpora configuration before/after the run.
The final larger sample is decision4; earlier decision runs are retained but
superseded, including explicitly INVALID runs at the seed-addressability seam.
The external copy contains only files listed in the Rust source manifest, copied
from the canonical immutable Rust input; no Git metadata or build output is copied.

`frame` publishes hierarchy coverage and the entire Prism-named remainder, plus
an independent TypeScript syntax census. `member_sites` samples property callable
invocations and getter accesses using AST-derived UTF-16 token positions and
native definition queries. It reports conditional site detection recall; it
cannot estimate incoming-set precision. Nonconcrete/unsupported definitions and
errors are retained separately. A generic/interface property definition is not
silently assigned to one concrete callback implementation.
