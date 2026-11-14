//! C++ and `.h` U/D matrix inputs that set repo-wide macro-census facts. They live as strings,
//! not under `tests/fixtures`, because the repository loader indexes that tree and a `#define`
//! there would make the census of any index of the prism repository refuse the same names
//! (#806 lesson). Rows for them stay in `tests/fixtures/dfg_prune/cpp/obligations.jsonl`
//! (`gen_obligations.py` reads the markers here too).

/// `(file name, source)`.
pub(super) const INLINE_CASES: &[(&str, &str)] = &[
    (
        "10_macros.cc",
        r####"// Census rows for C++ owners. `#define` lines are top-level, so every case shares one census.
int src();
void sink(int);
#define NEXT (x2 + 1)
#define READ_IT(v) ((v) + 1)
#define BIND(v) (keep_ptr = &(v))
#define PASTE(a, b) a##b
int *keep_ptr;

void cpp_macro_free_read_keep() {
    int x2 = src();       // @m1:D x2
    x2 = NEXT;            // @m1:U:keep:object_macro_hides_a_read
    sink(x2 + 1);
}

void cpp_macro_arg_keep() {
    int x4 = src();       // @m2:D x4
    x4 = 2;
    sink(READ_IT(x4));    // @m2:U:keep:name_passed_to_macro
}

void cpp_macro_address_keep() {
    int x5 = src();       // @m3:D x5
    BIND(x5);
    x5 = 1;
    sink(x5 + 1);         // @m3:U:keep:macro_takes_address
}

void cpp_macro_paste_off() {
    int x7 = src();       // @m4:D x7
    x7 = 1;
    sink(x7 + 1);         // @m4:U:keep:poisoned_macro_owner_off
    int PASTE(a, b) = 0;
}

void cpp_macro_unrelated_drop() {
    int x9 = src();       // @m5:D x9
    x9 = 1;
    sink(x9 + 1);         // @m5:U:drop:no_macro_involved
}
"####,
    ),
    (
        "11_owner_gates.cc",
        r####"// C++ owner gates G2 (directives in the body) and a control macro.
int src();
void sink(int);
#define JUMP goto done_label

void cpp_omp_owner_off() {
    int x = src();        // @g1:D x
    x = 1;
#pragma omp parallel
    sink(x + 1);          // @g1:U:keep:openmp_pragma_in_the_body
}

void cpp_define_in_body_off() {
    int x = src();        // @g2:D x
    x = 1;
#define LOCAL 1
    sink(x + LOCAL);      // @g2:U:keep:define_in_the_body
}

void cpp_control_macro_no_d() {
    int x = src();        // @g3:D x
    x = 1;
    sink(x + 1);          // @g3:U:keep:control_macro_turns_d_off
    JUMP;
done_label:
    return;
}

void cpp_clean_drop() {
    int x = src();        // @g4:D x
    x = 1;
    sink(x + 1);          // @g4:U:drop:no_gate
}
"####,
    ),
    (
        "12_header_macros.h",
        r####"// `.h` (C grammar, off since fold 1): the pairs are kept, macro or not.
int src(void);
void sink(int);
#define PEEK (hx + 1)

static inline int header_macro_free_name_keep(void) {
    int hx = src();       // @h1:D hx
    hx = PEEK;            // @h1:U:keep:object_macro_names_the_local
    return hx + 1;
}

static inline int header_clean_drop(void) {
    int hy = src();       // @h2:D hy
    hy = 1;
    return hy + 1;        // @h2:U:keep:dot_h_owner_is_off
}
"####,
    ),
    (
        "13_coroutine_macro.cc",
        r####"// A coroutine keyword spelled only inside an in-repo macro: the parser sees a call, so the
// owner is turned off by the macro census (fold 2, fix H). The control uses a macro without one.
int src();
void sink(int);
#define PAUSE() co_yield 0
#define OUTER() PAUSE()
#define PLAIN() sink(0)

void cpp_coroutine_macro_off() {
    int x = src();        // @co1:D x
    PAUSE();
    x = 1;
    sink(x + 1);          // @co1:U:keep:co_yield_hidden_in_a_macro
}

void cpp_coroutine_nested_macro_off() {
    int x = src();        // @co2:D x
    OUTER();
    x = 1;
    sink(x + 1);          // @co2:U:keep:co_yield_hidden_two_macros_deep
}

void cpp_plain_macro_drop() {
    int x = src();        // @co3:D x
    PLAIN();
    x = 1;
    sink(x + 1);          // @co3:U:drop:macro_without_a_coroutine_keyword
}
"####,
    ),
    (
        "14_keyword_macro.cc",
        r####"// An in-repo macro named like a C++ keyword rewrites the spelling the scalar proofs read:
// `int r = x;` is `IR r = x;`, a reference. An owner spelling the macro is off (fold 3).
using IR = long&;
long source();
#define int IR

long keyword_macro_owner_off() {
    long x = source();    // @km1:D x
    long saved = x;
    int r = x;
    x = 0;
    r = saved;
    return x + 1;         // @km1:U:keep:keyword_macro_int_rewrites_the_type_proof
}

long keyword_macro_unspelled_drop() {
    long y = source();    // @km2:D y
    y = 0;
    return y + 1;         // @km2:U:drop:owner_does_not_spell_the_macro
}
"####,
    ),
    (
        "15_phase_divergence.cc",
        r####"// The compiler's translation phases hide a line inside a comment that tree-sitter parses as a
// declaration (a trigraph `??/` is a backslash; GCC and clang splice a backslash followed by
// blanks). The phantom local would rebind `x`, so the owner is off (PhaseDivergence).
struct S { int v; S& operator=(int); };
int src();
void sink(int);

void phase_control_drop() {
    int x = src();        // @ph0:D x
    x = 1;
    sink(x + 1);          // @ph0:U:drop:no_phase_divergence_control
}

void phase_trigraph_const_ptr_keep(S x) {   // @ph1:D x
    // ??/
    const T* x;
    x = 1;                // @ph1:U:keep:trigraph_splice_hides_a_phantom_pointer_declaration
}

void phase_blank_backslash_const_ptr_keep(S x) {   // @ph2:D x
    // hidden \   
    const T* x;
    x = 1;                // @ph2:U:keep:backslash_blanks_newline_splice_hides_the_declaration
}

void phase_trigraph_scalar_keep(S x) {   // @ph3:D x
    // ??/
    int x;
    x = 1;                // @ph3:U:keep:trigraph_splice_hides_a_phantom_scalar_declaration
}

void phase_blank_backslash_scalar_keep(S x) {   // @ph4:D x
    // hidden \ 	
    int x;
    x = 1;                // @ph4:U:keep:backslash_tab_newline_splice_hides_the_declaration
}
"####,
    ),
    (
        "16_phase_splice_cr.cc",
        r####"// Fold 2: tree-sitter skips a bare backslash + newline as whitespace and ends a `//` comment
// only at LF; the compiler splices tokens across it and also ends a comment at a lone CR.
// The compiler's view is in the comments of each row; PhaseDivergence turns the owner off.
struct S { int v; S& operator=(int); };
int src();
void sink(int);

void splice_slash_star_keep(S x) {   // @sp1:D x
    int a = 1 /\
/*gap*/ 1; const T* x;
    ;
    x = 1;                // @sp1:U:keep:backslash_splice_makes_a_comment_only_the_compiler_sees
}

void splice_comment_pair_keep(S x) {   // @sp2:D x
    int y = 4 /\
*2; const T* x; int z = 3 *\
// c
1;
    x = 1;                // @sp2:U:keep:splices_inside_comment_delimiters
}

void splice_control_drop() {
    int x = src();        // @sp3:D x
    x = 1;
    sink(x + 1);          // @sp3:U:drop:control_without_any_splice_or_cr
}

int cr_comment_hides_if_keep(int c) {
    int x = src();        // @cr1:D x
    // note<CR> if (c)
    x = 2;
    return x + 1;         // @cr1:U:keep:lone_cr_ends_the_comment_so_the_compiler_sees_if_c_x_eq_2
}

int cr_control_conditional_keep(int c) {
    int x = src();        // @cr2:D x
    // note
    if (c)
        x = 2;
    return x + 1;         // @cr2:U:keep:compiler_view_of_the_previous_row_the_write_is_conditional
}

int cr_read_hidden_keep(int c) {
    int x = src();        // @cr3:D x
    if (c) x = 1; // n<CR> sink(x+0); // @cr3:U:keep:the_compiler_reads_x_here_tree_sitter_sees_a_comment
    return 0;
}

int cr_read_control_keep(int c) {
    int x = src();        // @cr4:D x
    if (c) x = 1;
    sink(x + 0);          // @cr4:U:keep:compiler_view_of_the_cr_row_the_read_is_live
    return 0;
}
"####,
    ),
    (
        "17_crlf_control.cc",
        r####"// <CRLF-FILE>
// CRLF line endings alone do not turn an owner off: this owner still prunes.
int src();
void sink(int);

void crlf_control_drop() {
    int x = src();        // @cl1:D x
    x = 1;
    sink(x + 1);          // @cl1:U:drop:crlf_only_owner_stays_on
}
"####,
    ),
    (
        "18_ucn_identifier.cc",
        r####"// Fold 3: `caf\u00e9` and `café` are one identifier to the compiler, two spellings to prism.
// An owner with a backslash-spelled identifier node is off (UcnIdentifier); a UCN only inside a
// string or character literal is not an identifier and leaves the owner on.
struct T {};
const T* source();
int src();
void sink(int);

const T* ucn_pointer_keep() {
    const T* café = source();   // @u1:D café
    café = caf\u00e9;           // @u1:U:keep:ucn_spelling_is_the_same_name_as_the_literal
    return café;
}

int ucn_scalar_keep() {
    int café = src();           // @u2:D café
    café = caf\u00e9;           // @u2:U:keep:ucn_spelling_is_the_same_name_as_the_literal
    return 0;
}

int ucn_consistent_control_drop() {
    int café = src();           // @u3:D café
    café = 1;
    sink(café + 1);             // @u3:U:drop:one_spelling_throughout
    return 0;
}

int ucn_in_string_only_drop() {
    int x = src();              // @u4:D x
    x = 1;
    sink(x + 1);                // @u4:U:drop:ucn_only_inside_literals
    const char* s = "caf\u00e9"; char c = '\u00e9';
    return s[0] + c;
}
"####,
    ),
    (
        "20_macro_brackets.cc",
        r####"// Bracket macros (#829), C++ owners. One census for the file.
int src();
void sink(int);
#define END }
#define BEGIN {
#define OPEN for (
#define CLOSE )
#define IF_BEGIN if (cond) {
#define IF_END }
#define WRAP(v) { sink(v); }
#define MSG "{ ( ["
struct S { int v; S& operator=(int i) { v = v * 10 + i; return *this; } };
int cond;
void sink2(const char *);

int cpp_brace_scope(S x) {
    {
        int* x = nullptr;
        END;
        x = 5;                // @x1:D x
        x = 1;                // @x1:U:keep:unbalanced_brace_macro_moves_scope
        BEGIN;
    }
    return x.v;
}

int cpp_paren_scope(S x) {
    OPEN; int* x = nullptr; CLOSE;
    x = 5;                    // @x2:D x
    x = 1;                    // @x2:U:keep:unbalanced_paren_macro_moves_scope
    return x.v;
}

void cpp_brace_control() {
    int x = src();            // @x3:D x
    IF_BEGIN;
    x = 2;
    IF_END;
    sink(x + 1);              // @x3:U:keep:unbalanced_macro_makes_the_write_conditional
}

template <typename T>
void cpp_template_owner(T t) {
    int x = src();            // @x4:D x
    IF_BEGIN;
    x = 2;
    IF_END;
    sink(x + 1);              // @x4:U:keep:template_owner_spells_unbalanced_macro
}

void cpp_balanced_still_prunes() {
    int x = src();            // @x5:D x
    WRAP(0);
    sink2(MSG);
    x = 2;
    sink(x + 1);              // @x5:U:drop:balanced_macros_do_not_turn_the_owner_off
}

void cpp_external_pthread_pair(void (*h)(void *)) {
    int x = src();            // @x6:D x
    pthread_cleanup_push(h, 0);
    x = 2;
    pthread_cleanup_pop(0);
    sink(x + 1);              // @x6:U:keep:external_block_macro_pair
}
"####,
    ),
    (
        "21_macro_args.cc",
        r####"// Macro argument lists (#829 part B), C++ owners. One census for the file.
int src();
void sink(int);
#define F(p, q) q p
#define CHECK(...) check_impl(__VA_ARGS__)
#define MAX(a, b) ((a) > (b) ? (a) : (b))
struct S { int v; S& operator=(int i) { v = v * 10 + i; return *this; } };
struct V { int a, b; };
bool check_impl(bool);
bool operator==(V, V);
int fn2(int, int);

int cpp_arg_split_scope(S x) {
    {
        int* x = nullptr; int b = 0;
        F([&](int){ int a, b; });
        x = 5;                // @y1:D x
        x = 1;                // @y1:U:keep:brace_held_comma_in_macro_arguments
    }(0);
    return x.v;
}

void cpp_init_list_in_macro(V v) {
    int x = src();            // @y2:D x
    CHECK(v == V{1, 2});
    x = 2;
    sink(x + 1);              // @y2:U:keep:initializer_list_comma_in_macro_arguments
}

void cpp_tight_commas_still_prune(int a, int b) {
    int x = src();            // @y3:D x
    sink(MAX(a, fn2(b, a)));
    x = 2;
    sink(x + 1);              // @y3:U:drop:commas_directly_inside_parentheses
}

void cpp_external_macro_still_prunes(V v) {
    int x = src();            // @y4:D x
    EXPECT_TRUE(v == V{1, 2});
    x = 2;
    sink(x + 1);              // @y4:U:drop:brace_held_comma_in_an_external_macro
}
"####,
    ),
    (
        "22_macro_wrappers.cc",
        r####"// In-repo macros that wrap listed external block macros, C++ owners (#829 round 1, G-terra-01,
// G-sol-01, G-sol-08). One census for the file.
int src();
void sink(int);
int cond;
#define P pthread_cleanup_push
#define Q pthread_cleanup_pop
#define ENTER(h, p) pthread_cleanup_push(h, p)
#define LEAVE(p) pthread_cleanup_pop(p)
#define E1(h, p) ENTER(h, p)
#define E2(h, p) E1(h, p)
#define L1(p) LEAVE(p)
#define L2(p) L1(p)
#define PY_IN(o) Py_BEGIN_CRITICAL_SECTION(o)
#define PY_OUT() Py_END_CRITICAL_SECTION()
#define PASS(pthread_cleanup_push) (pthread_cleanup_push)

void cpp_alias_object_like(void (*h)(void *)) {
    int x = src();            // @w1:D x
    if (cond) P(h, 0);
    x = 2;
    Q(0);
    sink(x + 1);              // @w1:U:keep:object_like_alias_of_a_listed_macro
}

void cpp_wrapper_function_like(void (*h)(void *)) {
    int x = src();            // @w2:D x
    if (cond) ENTER(h, 0);
    x = 2;
    LEAVE(0);
    sink(x + 1);              // @w2:U:keep:function_like_wrapper_of_a_listed_macro
}

void cpp_wrapper_multi_hop(void (*h)(void *)) {
    int x = src();            // @w3:D x
    if (cond) E2(h, 0);
    x = 2;
    L2(0);
    sink(x + 1);              // @w3:U:keep:multi_hop_wrapper_of_a_listed_macro
}

void cpp_wrapper_critical_section(void *op) {
    int x = src();            // @w4:D x
    if (cond) PY_IN(op);
    x = 2;
    PY_OUT();
    sink(x + 1);              // @w4:U:keep:wrapper_of_a_listed_cpython_macro
}

void cpp_parameter_shadow_control() {
    int x = src();            // @w6:D x
    int n = PASS(4);
    x = 2;
    sink(x + n);              // @w6:U:drop:a_parameter_spelled_like_a_listed_name_is_not_free
}
"####,
    ),
    (
        "23_macro_args_live.cc",
        r####"// Square-held splitting commas and held-comma controls, C++ owners (#829 round 1, G-sol-05,
// G-terra-05, G-terra-06). One census for the file.
int src();
void sink(int);
#define STR2(p, q) #p #q
#define S(a, b) #a "|" #b
#define ALIAS S
#define ONE(v) #v

void lambda_capture_plain() {
    int x = src();            // @y5:D x
    const char *s = STR2([a, b] { return 0; });
    x = 2;
    sink(x + 1);              // @y5:U:keep:square_held_comma_in_a_capture_list
}

void lambda_capture_reference() {
    int x = src();            // @y6:D x
    const char *s = STR2([&a, &b] { return 0; });
    x = 2;
    sink(x + 1);              // @y6:U:keep:square_held_comma_in_a_reference_capture
}

void lambda_capture_initialized() {
    int x = src();            // @y7:D x
    const char *s = STR2([a = 1, b = 2] { return 0; });
    x = 2;
    sink(x + 1);              // @y7:U:keep:square_held_comma_in_an_init_capture
}

void lambda_capture_mixed() {
    int x = src();            // @y8:D x
    const char *s = STR2([&a, b = 2] { return 0; });
    x = 2;
    sink(x + 1);              // @y8:U:keep:square_held_comma_in_a_mixed_capture
}

void alias_comment_square_split() {
    int x = src();            // @y9:D x
    ALIAS /*c*/ ([a, b]{});
    x = 2;
    sink(x + 1);              // @y9:U:keep:object_alias_comment_and_square_split
}

void subscript_comma_split(int *v) {
    int x = src();            // @y10:D x
    const char *s = STR2(v[0, 1]);
    x = 2;
    sink(x + 1);              // @y10:U:keep:comma_inside_a_subscript
}

void capture_comma_inside_parentheses(int a, int b, int c) {
    int x = src();            // @y11:D x
    ONE([a = (b, c)]{});
    x = 2;
    sink(x + 1);              // @y11:U:drop:innermost_bracket_is_a_parenthesis
}
"####,
    ),
    (
        "24_macro_splice.cc",
        r####"// A macro name assembled by a splice, C++ owner (#829 round 1, G-terra-10).
int src();
void sink(int);
#define OP\
EN {
#define CLOSE }
int cond;

void cpp_spliced_name() {
    int x = src();            // @s3:D x
    if (cond) OPEN /* comment */ ;
    x = 2;
    CLOSE;
    sink(x + 1);              // @s3:U:keep:macro_name_assembled_by_a_splice
}
"####,
    ),
    (
        "24_macro_splice_crlf.cc",
        r####"// <CRLF-FILE>
// A macro name assembled by a splice, C++ owner in a CRLF file (#829 round 1, G-terra-10).
int src();
void sink(int);
#define OP\
EN {
#define CLOSE }
int cond;

void cpp_spliced_name_crlf() {
    int x = src();            // @s4:D x
    if (cond) OPEN /* comment */ ;
    x = 2;
    CLOSE;
    sink(x + 1);              // @s4:U:keep:macro_name_assembled_by_a_splice
}
"####,
    ),
    (
        "25_r2_external_alias.cc",
        r####"// Chain A round 2 (#829), C++ owners: an external alias hides an in-repo macro
// (G2-terra-01, G2-opus-06; BOUNDARY). A, B and EXT_APPLY are never defined here. One census.
int src();
void sink(int);
#define F(p, q) q if (c) p
#define G(p, q) q p
#define APPLY(m, ...) m(__VA_ARGS__)

int cpp_external_alias_of_in_repo_macro(int c) {
    int x = 41;               // @g2t01:D x
    int b = 0;
    {
        A({ int a, b; });
        x = 2;
        B({ int a, b; });
        return x;             // @g2t01:U:keep:external_alias_exposes_an_in_repo_split
    }
}

int cpp_external_apply_forwards_an_in_repo_macro_name(int c) {
    int x = 41;               // @g2o06:D x
    int b = 0;
    {
        EXT_APPLY(F, { int a, b; });
        x = 2;
        EXT_APPLY(G, { int a, b; });
        return x;             // @g2o06:U:keep:external_variadic_apply_forwards_the_held_comma
    }
}

int cpp_in_repo_apply_forwards_an_in_repo_macro_name(int c) {
    int x = 41;               // @g2o06c:D x
    int b = 0;
    {
        APPLY(F, { int a, b; });
        x = 2;
        APPLY(G, { int a, b; });
        return x;             // @g2o06c:U:keep:in_repo_variadic_apply_forwards_the_held_comma
    }
}
"####,
    ),
    (
        "26_r2_external_names.cc",
        r####"// Chain A round 2 (#829), C++ owners: external block macros the repository never defines
// (G2-sonnet-01, G2-sonnet-02 BOUNDARY, G2-sonnet-03 control K3). No `#define` in this file.
int src();
void sink(int);
int g;

int cpp_trashcan_begin_condition(void *op, int c) {
    int x = 41;               // @g2s01:D x
    if (c) Py_TRASHCAN_BEGIN_CONDITION(op, 1);
    x = 2;
    Py_TRASHCAN_END;
    return x;                 // @g2s01:U:keep:unlisted_cpython_trashcan_macro
}

int cpp_numpy_allow_threads(int c) {
    int x = 41;               // @g2s02:D x
    if (c) NPY_BEGIN_ALLOW_THREADS;
    x = 2;
    NPY_END_ALLOW_THREADS;
    return x;                 // @g2s02:U:keep:numpy_alias_of_the_allow_threads_pair
}

int cpp_ext_prefix_deref_store(int *p, int c) {   // @g2s03k3:D p
    M *p = 2;                 // @g2s03k3:U:keep:cpp_escapes_names_of_non_keyword_typed_declarations
    return g;
}
"####,
    ),
    (
        "27_r2_listed_name_shim.cc",
        r####"// Chain A round 2 (#829), C++ owners: the repository defines a shim under a listed system
// name (G2-sonnet-07). One census for the file.
int src();
void sink(int);
int cond;
#define pthread_cleanup_push(r, a) do { (r)(a); } while (0)
#define pthread_cleanup_pop(e) ((void)(e))

void cpp_shimmed_listed_name(void (*h)(void *)) {
    int x = src();            // @g2s07:D x
    if (cond) pthread_cleanup_push(h, 0);
    x = 2;
    pthread_cleanup_pop(0);
    sink(x + 1);              // @g2s07:U:keep:listed_name_is_refused_even_when_the_repository_defines_it
}
"####,
    ),
    (
        "29_r2_enclosing.cc",
        r####"// Chain A round 2 (#829), C++: the owner sits inside an in-repo macro's argument group at
// file scope (G2-opus-01). One census for the file.
int b, e;
#define J(p, q, r) p; if (c) { q; } r
J(static int dummy;
int probe(int c) {
    int x = 41;               // @g2o01:D x
    int a, b;
    x = 2;
    int d, e;
    return x;                 // @g2o01:U:keep:owner_inside_an_enclosing_invocation
}
)
"####,
    ),
    (
        "30_r2_enclosing_variadic.cc",
        r####"// Chain A round 2 (#829), C++: control for G2-opus-01. A variadic forwarding macro rejoins
// the pieces with commas: the overwrite dominates, pruning must survive. One census.
int b, e;
#define K(...) __VA_ARGS__
K(static int dummy;
int probe_forwarded(int c) {
    int x = 41;               // @g2o01k:D x
    int a, b;
    x = 2;
    int d, e;
    return x;                 // @g2o01k:U:drop:variadic_forwarding_changes_nothing
}
)
"####,
    ),
    (
        "31_r2_enclosing_file_scope.cc",
        r####"// Chain A round 2 (#829), C++: the enclosing group is opened by a file-scope unbalanced
// macro (G2-opus-01b). One census for the file.
int b, e;
#define J(p, q, r) p; if (c) { q; } r
#define BEGIN_FNS J(
BEGIN_FNS static int dummy;
int probe_fs(int c) {
    int x = 41;               // @g2o01b:D x
    int a, b;
    x = 2;
    int d, e;
    return x;                 // @g2o01b:U:keep:owner_inside_a_group_opened_by_a_file_scope_macro
}
)
"####,
    ),
    (
        "32_r2_strip_parenthesis.cc",
        r####"// Chain A round 2 (#829), C++: a brace-held comma below an inner parenthesis splits after a
// rescan strips the parenthesis (G2-opus-02). One census for the file.
#define F(p, q) q if (c) p
#define G(p, q) q p
#define SF(v) F v
#define SG(v) G v

int cpp_stripped_parenthesis(int c) {
    int x = 41;               // @g2o02:D x
    int b = 0;
    {
        SF(({ int a, b; }));
        x = 2;
        SG(({ int a, b; }));
        return x;             // @g2o02:U:keep:rescan_strips_the_parenthesis_that_held_the_comma
    }
}
"####,
    ),
    (
        "33_r2_digit_separator.cc",
        r####"// Chain A round 2 (#829), C++: digit separators. The macro bodies are read in both
// dialects (G2-opus-03); the owner's argument group is read in both too (G2-opus-04, the C
// reading of `'0, b = 2'` is one character literal that hides a brace-held comma). One census.
#define OPEN if (c != 1'0) { (void)(2'0);
#define CLOSE (void)(3'0); } (void)(4'0);
#define F(p, q) q p
struct S { int v; S& operator=(int i) { v = v * 10 + i; return *this; } };

int cpp_digit_separator_hides_a_bracket(int c) {
    int x = 41;               // @g2o03:D x
    OPEN;
    x = 2;
    CLOSE;
    return x;                 // @g2o03:U:keep:c_reading_hides_the_brackets_of_the_c23_reading
}

int cpp_digit_separator_hides_a_held_comma(S x) {
    {
        int *x = nullptr; int b = 0;
        F([&](int){ int a = 1'0, b = 2'0; });
        x = 5;                // @g2o04:D x
        x = 1;                // @g2o04:U:keep:c_reading_hides_the_brace_held_comma_of_a_group
    }(0);
    return x.v;
}
"####,
    ),
    (
        "34_r2_control_unbalanced.cc",
        r####"// Chain A round 2 (#829), C++: a macro that is both control (goto) and unbalanced
// (G2-opus-07). Rule D is already off for a control macro; rule U must still keep the pair.
// Shape from jemalloc TEST_END. One census for the file.
#define END goto out; out: }
#define BEGIN {
struct S { int v; S& operator=(int i) { v = v * 10 + i; return *this; } };

int control_and_unbalanced(S x) {
    {
        int* x = nullptr;
        END;
        x = 5;                // @g2o07:D x
        x = 1;                // @g2o07:U:keep:control_and_unbalanced_macro_still_moves_the_scope
        BEGIN;
    }
    return x.v;
}
"####,
    ),
];

/// The harness view of an inline case: `<CR>` is a lone carriage return (a raw string cannot
/// hold one) and a source holding `<CRLF-FILE>` has every line end turned into CRLF.
pub(super) fn inline_source(s: &str) -> String {
    let crlf = s.contains("<CRLF-FILE>");
    let mut out = s.replace("<CRLF-FILE>", "").replace("<CR>", "\r");
    if crlf {
        out = out.replace('\n', "\r\n");
    }
    out
}
