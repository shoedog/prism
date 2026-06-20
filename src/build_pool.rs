//! Shared rayon pool with a large per-worker stack for prism's recursive
//! tree-walks (file parsing and CPG construction).
//!
//! prism walks tree-sitter ASTs recursively in both `ParsedFile::parse`
//! (error-node counting) and the CPG build (DFG / call-graph / assemble), and
//! those walks go as deep as the AST nesting. rayon's default worker stack
//! (~2 MiB) overflows on pathological inputs — e.g. a `#if`-split 8192-element
//! C lookup-table initializer (vendored base64 tables in mypy's
//! `mypyc/lib-rt`), which aborts the whole process with a stack overflow.
//! Running the parse and build phases inside `build_pool().install(...)` gives
//! every nested `par_iter` worker a generous stack. The pool is built once
//! (lazily) and shared; its threads are idle when nothing is parsing/building.

use std::sync::OnceLock;

/// Per-worker stack for the shared pool: 256 MiB.
///
/// Virtual only — untouched pages never commit — so this is cheap; it just
/// raises the recursion ceiling far above the default ~2 MiB worker stack.
const STACK_BYTES: usize = 256 * 1024 * 1024;

/// The shared large-stack rayon pool. `install()` the recursive parse/build
/// work onto it so every nested `par_iter` worker has a generous stack.
pub(crate) fn build_pool() -> &'static rayon::ThreadPool {
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .stack_size(STACK_BYTES)
            .build()
            .expect("build prism large-stack rayon pool")
    })
}

/// Run `op` (and every `par_iter` it spawns) on the large-stack pool. Public so
/// process entry points — the CLI command dispatch and the MCP request handlers
/// — can wrap a whole command, putting all of prism's recursive AST walks
/// (parse, CPG build, live-type scan, and any future walker) on big-stack
/// workers in one place rather than wrapping each site. `op` runs on a pool
/// worker; the caller blocks until it returns.
pub fn install<R: Send>(op: impl FnOnce() -> R + Send) -> R {
    build_pool().install(op)
}
