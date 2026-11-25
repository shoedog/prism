# Truth environments

The executed truth was produced with these pinned, offline environments (the paths default to `~/prism-evidence/routes/truth/`; set `ROUTES_TRUTH_VENV` and `ROUTES_TRUTH_NODE` to other locations).

- Python: CPython 3.12 venv, `uv pip install --offline flask==3.1.3 fastapi==0.115.12`; resolved versions:
    Package           Version
    ----------------- --------
    annotated-types   0.8.0
    anyio             4.16.0
    blinker           1.9.0
    click             8.5.0
    fastapi           0.115.12
    flask             3.1.3
    idna              3.20
    itsdangerous      2.2.0
    jinja2            3.1.6
    markupsafe        3.0.4
    pydantic          2.12.5
    pydantic-core     2.41.5
    starlette         0.46.2
    typing-extensions 4.16.0
    typing-inspection 0.4.4
    werkzeug          3.1.9
- Node: v24 and `npm install --offline express@4.22.3` into a directory whose `node_modules` is `ROUTES_TRUTH_NODE`; resolved: express 4.22.3 (67 packages; `npm ls` lists them).
- Go, gin, chi, net/http, Hapi, Django and NestJS rows are hand-labelled (no executed truth).
- Each executed truth entry records `fixture_sha256`, a hash of its row directory, which `routes_matrix_test` recomputes: a fixture edited without regenerating truth fails the test.
- This runner is containment against ambient imports for lane-written fixtures only; it is not a sandbox.
