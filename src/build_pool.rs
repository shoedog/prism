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

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod per_item_tests;

/// Keep lumpy file/caller jobs independently stealable without changing their
/// indexed collection order. Call before adapters that discard indexing.
pub(crate) fn per_item<I>(items: I) -> impl rayon::iter::IndexedParallelIterator<Item = I::Item>
where
    I: rayon::iter::IntoParallelIterator,
    I::Iter: rayon::iter::IndexedParallelIterator,
{
    rayon::iter::IndexedParallelIterator::with_max_len(items.into_par_iter(), 1)
}

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
///
/// Where the caller blocks depends on who calls:
/// - a worker of the build pool runs `op` inline (rayon's own same-pool rule);
/// - an ordinary thread (no rayon pool) injects `op` and blocks on a latch;
/// - a worker of a *different* rayon pool hands the call to a scoped helper
///   thread, which injects `op` and blocks, while the worker waits in an
///   ordinary `join`. rayon's own cross-pool `install` (`rayon-core` 1.13
///   `Registry::in_worker_cross`) would instead make that worker wait with
///   `WorkerThread::wait_until`, which executes the worker's own pool's queued
///   jobs; a sibling job that needs a lock the caller holds across this call
///   then never returns control to the caller (issue #513). A joined helper
///   thread never runs anyone's queued jobs.
///
/// A panic in `op` propagates to the caller on every route.
///
/// # Contract for callers on another pool's worker
///
/// On the third route the calling worker is parked in a blocking join until
/// `op` returns, and `op` runs on the build pool. So `op` must not wait on
/// work that only the calling pool can run: for example, calling that same
/// pool's `install` (or `join`/`scope` on it) while all of its workers are
/// parked this way never completes. rayon's own cross-pool `install` would
/// complete that nested call by running it on the parked worker, at the price
/// of the hazard above; this function takes the other side of the trade. The
/// large-stack boundaries inside prism (graph constructors, the CPG build,
/// the loader) never call back into the caller's pool.
pub fn install<R: Send>(op: impl FnOnce() -> R + Send) -> R {
    #[cfg(test)]
    let op = {
        // P15a-fix2 (test-only): propagate the measuring thread's counter
        // generation onto the thread that runs `op`, so Go-provider
        // constructions inside the build are attributed to the live
        // MeasurementToken instead of being invisible to it. Captured here,
        // on the calling thread, so it survives the helper-thread hop.
        let generation = crate::type_providers::go::test_counters::current_generation();
        move || crate::type_providers::go::test_counters::inherit_generation(generation, op)
    };
    if in_foreign_pool_worker() {
        install_from_foreign_worker(op)
    } else {
        build_pool().install(op)
    }
}

/// True on a thread that is a worker of some rayon pool other than the build
/// pool (the global pool included). `current_thread_index` of the free
/// function is `Some` on any pool's worker; the pool's own method is `Some`
/// only on that pool's workers (`Registry::current_thread` compares registry ids).
fn in_foreign_pool_worker() -> bool {
    rayon::current_thread_index().is_some() && build_pool().current_thread_index().is_none()
}

fn install_from_foreign_worker<R: Send>(op: impl FnOnce() -> R + Send) -> R {
    std::thread::scope(|scope| {
        let helper = std::thread::Builder::new()
            .name("prism-build-pool-handoff".into())
            .spawn_scoped(scope, || build_pool().install(op))
            .expect("spawn prism build-pool handoff thread");
        match helper.join() {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    })
}
