# CPG-build stack-overflow regression fixture

Vendored from the `base64` C library (via mypy's `mypyc/lib-rt`). `table_enc_12bit.h`
is an 8192-element `#if`-split lookup-table initializer whose deep AST drove prism's
recursive tree-walks past a rayon worker's default ~2 MiB stack (SIGABRT) — both in
the CPG build (DFG/call-graph/assemble) and in `ParsedFile::parse` (error-node
counting). `tables.c` is present so the parallel build schedules the table onto a
worker thread (it is marginal on the 8 MiB main thread).

Fix: both the repo parse (`src/repo_loader.rs`) and the CPG build (`src/cpg/build.rs`)
run on a shared large-stack rayon pool (`src/build_pool.rs::build_pool`). Regression
test: `src/cpg/tests.rs::cpg_build_survives_deep_c_initializer_without_stack_overflow`
(build path); the parse path is covered by the `nav_compat_test` dogfood tests, which
parse the whole prism repo (now including this fixture).
