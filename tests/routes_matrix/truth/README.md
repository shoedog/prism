# Executed route truth

Run from the worktree root with Node 24 on `PATH` and the offline environments in place.

```sh
fixture=tests/routes_matrix
out=$(mktemp -d)
python3 -B -m unittest discover -s "$fixture/truth" -v
python3 -B "$fixture/truth/run_truth.py" --rows "$fixture/rows.jsonl" \
    --root "$fixture" --out "$out/truth.jsonl"
"$fixture/truth/check.sh"
```

Use `--only ID [ID ...]` to select rows. `ROUTES_TRUTH_VENV` accepts a venv
directory or its Python executable and defaults to
`~/prism-evidence/routes/truth/venv/bin/python`. `ROUTES_TRUTH_NODE` points to
`node_modules` and defaults to `~/prism-evidence/routes/truth/node/node_modules`.
The children check CPython 3.12, Flask 3.1.3, FastAPI 0.115.12, Starlette 0.46.2,
Node 24, and Express 4.22.3.

- `stage.py` checks and copies each row tree, stages the Node dependencies, and
  bootstraps the isolated Python child. The Python AST and Node Acorn import
  checks run inside their respective children before any fixture module loads.
- `flask_routes.py`, `fastapi_routes.py`, and `express_routes.mjs` hook native
  registration and read the resulting route tables. The Express adapter also
  checks actual module resolutions against the staged trees.
- `run_truth.py` runs one child per row with a ten-second limit and a fresh
  home. It emits one JSON line per selected executed row, including a refusal
  line and a failing exit status when observation fails.
- `check.sh` checks provenance and diffs a fresh run against the committed
  `truth.jsonl`. A hand row in a supported group needs a probe entry.
  `selfcheck/rows.jsonl` and `test_runner_selfchecks.py` exercise the tool
  independently of the real matrix.

`calls` records arguments, callback identities, and one-based site ordinals.
`served` links native routes to those call IDs and separates declared methods
from effective methods such as automatic HEAD or OPTIONS. `projection` carries
the observed parts of the listing shape. Compare a pinned `full` by equality
and a pinned `suffix` by checking the end of an observed full path. Receiver
bindings, root IDs, and static reachability reasons belong to the static
listing. A runtime-valued argument keeps its raw spelling when available.

Go and frameworks outside the adapters require hand truth. So do Express
mounts outside the literal-prefix layer grammar, ambiguous or unlocatable
Express route callbacks, and FastAPI subapplication or static mounts. Express
route regexes are preserved as raw patterns. Flask fixtures disable static
routes on apps and blueprints, FastAPI fixtures disable generated docs and
OpenAPI routes, and Express fixtures export an application without listening.
Use this harness with lane-authored fixtures.

The pinned environments and how they were built are in `ENV.md`; `ROUTES_TRUTH_VENV` and `ROUTES_TRUTH_NODE` select them. This is containment against ambient imports for lane-written fixtures, not a sandbox: a fixture can still run any standard-library code. `fixture_sha256` in each executed truth line ties it to the row directory.
