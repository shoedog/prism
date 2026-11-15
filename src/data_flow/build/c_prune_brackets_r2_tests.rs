//! #829 chain A, round 2 (terra, sonnet, sol2, opus): pair rows of the inline cases `25_r2_*`
//! to `34_r2_*` (ids `g2<t|s|o>NN` = `G2-terra|sonnet|opus-NN`; TESTS.md "Chain A (round 2)"
//! maps them). Every row asserts the REQUIRED behaviour under the goal (the live pair is kept,
//! or the control pruning survives), never a specific `Off::` reason: the rule of DESIGN.md is
//! tentative. Tests named `boundary_*` are BOUNDARY rows (an input the loader or the macro
//! model cannot see: external macros, includes); the placement of each is decided later and a
//! row deferred to #813 will be marked `#[ignore = "#813"]`. Kinds: S live pair dropped on
//! base, C conservative (no liveness run), K control (green on base).
use super::c_prune_brackets_r3_support::pair_in;
use super::c_prune_brackets_tests::check_row;
use super::c_prune_oracle_rows::{judge, Want};

macro_rules! rows {
    ($file:expr => $($name:ident $id:expr),+ $(,)?) => {
        $(#[test] fn $name() { check_row($file, $id); })+
    };
}

// ---- external aliases and external block macros ----

// BOUNDARY S: A/B alias in-repo F/G from an outside header; EXT_APPLY forwards the name F.
rows!("25_r2_external_alias.c" =>
    boundary_row_g2_terra_01 "g2t01", boundary_row_g2_opus_06 "g2o06",
    row_g2_opus_06_in_repo_apply_control_keeps_the_pair "g2o06c");
rows!("25_r2_external_alias.cc" =>
    boundary_row_cpp_g2_terra_01 "g2t01", boundary_row_cpp_g2_opus_06 "g2o06",
    row_cpp_g2_opus_06_in_repo_apply_control_keeps_the_pair "g2o06c");

// S: Py_TRASHCAN_BEGIN_CONDITION (CPython 3.11, 3.12; compiled `41` by sonnet).
rows!("26_r2_external_names.c" => row_g2_sonnet_01 "g2s01");
rows!("26_r2_external_names.cc" => row_cpp_g2_sonnet_01 "g2s01");
// C: members of G2-sonnet-01 with no compile evidence (CPython 3.13 internal headers, MSVC).
rows!("26_r2_external_names.c" =>
    row_g2_sonnet_01a_critical_section_mut "g2s01a",
    row_g2_sonnet_01b_critical_section2_mut "g2s01b",
    row_g2_sonnet_01c_critical_section_sequence_fast "g2s01c",
    row_g2_sonnet_01d_suppress_iph "g2s01d");
// BOUNDARY S: NumPy aliases of the allow-threads pair.
rows!("26_r2_external_names.c" => boundary_row_g2_sonnet_02 "g2s02");
rows!("26_r2_external_names.cc" => boundary_row_cpp_g2_sonnet_02 "g2s02");
// BOUNDARY C: PostgreSQL PG_TRY family, unbraced (no liveness run).
rows!("26_r2_external_names.c" => boundary_row_g2_opus_09 "g2o09");

// S: an external statement-prefix macro turns `T *p = e;` into a store through the parameter.
rows!("26_r2_external_names.c" =>
    row_g2_sonnet_03a "g2s03a", row_g2_sonnet_03b "g2s03b", row_g2_sonnet_03c "g2s03c",
    row_g2_sonnet_03d "g2s03d", row_g2_sonnet_03e1 "g2s03e1", row_g2_sonnet_03e2 "g2s03e2");
// K: non-pointer declarations stay write-only drops; a real nested redeclaration still prunes.
rows!("26_r2_external_names.c" =>
    row_g2_sonnet_03k1_control "g2s03k1", row_g2_sonnet_03k2_control "g2s03k2",
    row_g2_sonnet_03k6_control "g2s03k6");
// K: C++ already keeps the pair (pins it); in-repo prefix macros keep it today.
rows!("26_r2_external_names.cc" => row_cpp_g2_sonnet_03k3_control "g2s03k3");
rows!("27_r2_inrepo_prefix.c" =>
    row_g2_sonnet_03k5a_control "g2s03k5a", row_g2_sonnet_03k5b_control "g2s03k5b");

// C: a listed system name is refused whatever the repository defines under it.
rows!("28_r2_listed_name_shim.c" => row_g2_sonnet_07 "g2s07");
rows!("27_r2_listed_name_shim.cc" => row_cpp_g2_sonnet_07 "g2s07");

// ---- the owner is inside an invocation, or the split appears after a rescan ----

rows!("29_r2_enclosing.c" => row_g2_opus_01 "g2o01");
rows!("29_r2_enclosing.cc" => row_cpp_g2_opus_01 "g2o01");
rows!("30_r2_enclosing_variadic.c" => row_g2_opus_01_variadic_control "g2o01k");
rows!("30_r2_enclosing_variadic.cc" => row_cpp_g2_opus_01_variadic_control "g2o01k");
rows!("31_r2_enclosing_file_scope.c" => row_g2_opus_01b "g2o01b");
rows!("31_r2_enclosing_file_scope.cc" => row_cpp_g2_opus_01b "g2o01b");
rows!("32_r2_strip_parenthesis.c" => row_g2_opus_02 "g2o02");
rows!("32_r2_strip_parenthesis.cc" => row_cpp_g2_opus_02 "g2o02");

// ---- census dialect hide (digit separators), control + unbalanced ----

rows!("33_r2_digit_separator.c" => row_g2_opus_03 "g2o03");
rows!("33_r2_digit_separator.cc" =>
    row_cpp_g2_opus_03 "g2o03", row_cpp_g2_opus_04_owner_side_hide "g2o04");
rows!("34_r2_control_unbalanced.cc" => row_cpp_g2_opus_07 "g2o07");

// ---- G2-sonnet-04: real block-macro definitions, as written in the system headers ----
//
// Excerpts (with their continuations, tabs, comments and nested `do`) of macros the audit of
// the headers on the author's machine found to open or close a block. Compared against the
// local macOS SDK `pthread.h`, CPython 3.12 `ceval.h` and `cpython/object.h` and CPython 3.14
// `cpython/critical_section.h`; the glibc and NumPy texts are reproduced from memory (those
// headers are not on this machine) and are approximate in whitespace. They arrive as
// a parsed header and as a census text, in front of a guard owner in both dialects. The
// required behaviour is the goal's: the live pair `x = src()` to the read after the pair is
// kept. glibc's push is already refused for rule D on base because `__sigsetjmp_cancel` names
// a returns-twice function (`control`), so that row is a green control. (The audit's
// header-wide list: CPython 3.11/3.12 allow-threads and trashcan, 3.13/3.14 critical sections,
// glibc and macOS pthread_cleanup, NumPy's aliases; file-scope `*_DECLS` and namespace macros
// are not function-scope and are not rows.)

const GLIBC_CLEANUP: &str = r####"# define pthread_cleanup_push(routine, arg) \
  do {									      \
    __pthread_unwind_buf_t __cancel_buf;				      \
    void (*__cancel_routine) (void *) = (routine);			      \
    void *__cancel_arg = (arg);						      \
    int __not_first_call = __sigsetjmp_cancel (__cancel_buf.__cancel_jmp_buf, 0);			      \
    if (__glibc_unlikely (__not_first_call))				      \
      {									      \
	__cancel_routine (__cancel_arg);				      \
	__pthread_unwind_next (&__cancel_buf);				      \
	/* NOTREACHED */						      \
      }									      \
									      \
    __pthread_register_cancel (&__cancel_buf);				      \
    do {
# define pthread_cleanup_pop(execute) \
      do { } while (0);/* Empty to allow label before pthread_cleanup_pop.  */\
    } while (0);							      \
    __pthread_unregister_cancel (&__cancel_buf);			      \
    if (execute)							      \
      __cancel_routine (__cancel_arg);					      \
  } while (0)
"####;

const MACOS_CLEANUP: &str = r####"#define pthread_cleanup_push(func, val) \
	{ \
		struct __darwin_pthread_handler_rec __handler; \
		pthread_t __self = pthread_self(); \
		__handler.__routine = func; \
		__handler.__arg = val; \
		__handler.__next = __self->__cleanup_stack; \
		__self->__cleanup_stack = &__handler;

#define pthread_cleanup_pop(execute) \
		/* Note: 'handler' must be in this same lexical context! */ \
		__self->__cleanup_stack = __handler.__next; \
		if (execute) (__handler.__routine)(__handler.__arg); \
	}
"####;

const CPYTHON_ALLOW_THREADS: &str = r####"#define Py_BEGIN_ALLOW_THREADS { \
                        PyThreadState *_save; \
                        _save = PyEval_SaveThread();
#define Py_BLOCK_THREADS        PyEval_RestoreThread(_save);
#define Py_UNBLOCK_THREADS      _save = PyEval_SaveThread();
#define Py_END_ALLOW_THREADS    PyEval_RestoreThread(_save); \
                 }
"####;

const CPYTHON_TRASHCAN: &str = r####"#define Py_TRASHCAN_BEGIN_CONDITION(op, cond) \
    do { \
        PyThreadState *_tstate = NULL; \
        /* If "cond" is false, then _tstate remains NULL and the deallocator \
         * is run normally without involving the trashcan */ \
        if (cond) { \
            _tstate = _PyThreadState_UncheckedGet(); \
            if (_PyTrash_begin(_tstate, _PyObject_CAST(op))) { \
                break; \
            } \
        }
        /* The body of the deallocator is here. */
#define Py_TRASHCAN_END \
        if (_tstate) { \
            _PyTrash_end(_tstate); \
        } \
    } while (0);
#define Py_TRASHCAN_BEGIN(op, dealloc) \
    Py_TRASHCAN_BEGIN_CONDITION((op), \
        _PyTrash_cond(_PyObject_CAST(op), (destructor)(dealloc)))
"####;

const CPYTHON_CRITICAL_SECTION: &str = r####"#define Py_BEGIN_CRITICAL_SECTION(op)                                   \
    {                                                                   \
        PyCriticalSection _py_cs;                                       \
        PyCriticalSection_Begin(&_py_cs, _PyObject_CAST(op))
#define Py_END_CRITICAL_SECTION()                                       \
        PyCriticalSection_End(&_py_cs);                                 \
    }
"####;

const CPYTHON_CRITICAL_SECTION_DEFAULT_BUILD: &str = r####"#ifndef Py_GIL_DISABLED
# define Py_BEGIN_CRITICAL_SECTION(op)      \
    {
# define Py_END_CRITICAL_SECTION()          \
    }
#endif
"####;

const NUMPY_ALIASES: &str = r####"#define Py_BEGIN_ALLOW_THREADS { \
                        PyThreadState *_save; \
                        _save = PyEval_SaveThread();
#define Py_END_ALLOW_THREADS    PyEval_RestoreThread(_save); \
                 }
#define NPY_BEGIN_ALLOW_THREADS Py_BEGIN_ALLOW_THREADS
#define NPY_END_ALLOW_THREADS Py_END_ALLOW_THREADS
"####;

fn excerpt_owner(open: &str, close: &str) -> String {
    format!(
        "int src(void);\nvoid sink(int);\nint cond;\nvoid f(void *h, void *op) {{\n    int x = src();\n    if (cond) {open};\n    x = 2;\n    {close};\n    sink(x + 1);\n}}\n"
    )
}

fn excerpt_rows() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        (
            "glibc pthread_cleanup",
            GLIBC_CLEANUP,
            "pthread_cleanup_push(h, 0)",
            "pthread_cleanup_pop(0)",
        ),
        (
            "macOS pthread_cleanup",
            MACOS_CLEANUP,
            "pthread_cleanup_push(h, 0)",
            "pthread_cleanup_pop(0)",
        ),
        (
            "CPython allow threads",
            CPYTHON_ALLOW_THREADS,
            "Py_BEGIN_ALLOW_THREADS",
            "Py_END_ALLOW_THREADS",
        ),
        (
            "CPython trashcan condition",
            CPYTHON_TRASHCAN,
            "Py_TRASHCAN_BEGIN_CONDITION(op, 1)",
            "Py_TRASHCAN_END",
        ),
        (
            "CPython trashcan wrapper",
            CPYTHON_TRASHCAN,
            "Py_TRASHCAN_BEGIN(op, h)",
            "Py_TRASHCAN_END",
        ),
        (
            "CPython critical section",
            CPYTHON_CRITICAL_SECTION,
            "Py_BEGIN_CRITICAL_SECTION(op)",
            "Py_END_CRITICAL_SECTION()",
        ),
        (
            "CPython 3.14 critical section, default build",
            CPYTHON_CRITICAL_SECTION_DEFAULT_BUILD,
            "Py_BEGIN_CRITICAL_SECTION(op)",
            "Py_END_CRITICAL_SECTION()",
        ),
        (
            "NumPy allow threads",
            NUMPY_ALIASES,
            "NPY_BEGIN_ALLOW_THREADS",
            "NPY_END_ALLOW_THREADS",
        ),
    ]
}

/// Every excerpt is judged (a failure of one does not hide the others). Owner lines: `int x =
/// src();` is line 5, `sink(x + 1);` is line 9.
fn excerpt_failures(census_text: bool) -> Vec<String> {
    let mut failed = Vec::new();
    for (what, excerpt, open, close) in excerpt_rows() {
        let owner = excerpt_owner(open, close);
        for (target, hdr) in [("a.c", "h.h"), ("a.cc", "h.hpp")] {
            let r = if census_text {
                pair_in(
                    &[(target, &owner)],
                    target,
                    &[("defs.inc", excerpt)],
                    None,
                    "x",
                    5,
                    9,
                )
            } else {
                pair_in(
                    &[(target, &owner), (hdr, excerpt)],
                    target,
                    &[],
                    None,
                    "x",
                    5,
                    9,
                )
            };
            if let Err(e) = judge(&format!("{what} ({target})"), Want::Kept, r) {
                failed.push(e);
            }
        }
    }
    failed
}

#[test]
fn g2_sonnet_04_real_block_macro_excerpts_in_a_parsed_header() {
    let failed = excerpt_failures(false);
    assert!(failed.is_empty(), "a live pair was dropped: {failed:#?}");
}

#[test]
fn g2_sonnet_04_real_block_macro_excerpts_as_a_census_text() {
    let failed = excerpt_failures(true);
    assert!(failed.is_empty(), "a live pair was dropped: {failed:#?}");
}
