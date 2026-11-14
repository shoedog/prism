//! C U/D matrix inputs that set repo-wide macro-census facts. They live as strings, not under
//! `tests/fixtures`, because the repository loader indexes that tree and a `#define` there would
//! make the census of any index of the prism repository refuse the same names (#806 lesson).
//! Rows for them stay in `tests/fixtures/dfg_prune/c/obligations.jsonl`
//! (`gen_obligations.py` reads the markers here too).

/// `(file name, source)`.
pub(super) const INLINE_CASES: &[(&str, &str)] = &[
    (
        "10_macros.c",
        r####"// Census rows. `#define` lines are top-level, so every case below shares one census.
int src(void);
void sink(int);
#define NEXT (x2 + 1)
#define READ_IT(v) ((v) + 1)
#define FREE_WRITE (xw = 0)
#define BIND(v) (keep_ptr = &(v))
#define PASTE(a, b) a##b
#define JUMP goto done_label
#define NAMED_LOCAL nm
int *keep_ptr;

void macro_free_read_keep(void) {
    int x2 = src();       // @m1:D x2
    x2 = NEXT;            // @m1:U:keep:object_macro_hides_a_read
    sink(x2);
}

void macro_param_read_drop(void) {
    int x3 = src();       // @m2:D x3
    x3 = 2;
    sink(READ_IT(x3));    // @m2:U:keep:name_passed_to_macro
}

void macro_param_other_drop(void) {
    int x4 = src();       // @m3:D x4
    x4 = 2;
    sink(x4);             // @m3:U:drop:no_macro_involved
}

void macro_arg_touch_keep(void) {
    int x5 = src();       // @m4:D x5
    BIND(x5);
    x5 = 1;
    sink(x5);             // @m4:U:keep:macro_takes_address
}

void macro_free_write_keep(void) {
    int xw = src();       // @m5:D xw
    FREE_WRITE;
    xw = 1;
    sink(xw);             // @m5:U:keep:macro_free_name_write
}

void macro_control_off(void) {
    int x6 = src();       // @m6:D x6
    x6 = 1;
    sink(x6);             // @m6:U:keep:control_macro_owner_d_off
    JUMP;
done_label:
    return;
}

void macro_paste_poison(void) {
    int x7 = src();       // @m7:D x7
    x7 = 1;
    sink(x7);             // @m7:U:keep:poisoned_macro_owner_off
    int PASTE(a, b) = 0;
}

void macro_named_local_keep(void) {
    int NAMED_LOCAL = src(); // @m8:D NAMED_LOCAL
    NAMED_LOCAL = 1;
    sink(NAMED_LOCAL);    // @m8:U:keep:local_named_like_a_macro
}

void macro_typed_decl_not_dominating(void) {
    int x9 = src();       // @m9:D x9
    NEXT x9 = 0;
    sink(x9);             // @m9:U:keep:macro_typed_declaration
}

void define_in_body_off(void) {
    int xa = src();       // @m10:D xa
#define LOCALM 1
    xa = 1;
    sink(xa);             // @m10:U:keep:define_in_body_owner_off
}

void include_in_body_off(void) {
    int xb = src();       // @m11:D xb
#include <stdio.h>
    xb = 1;
    sink(xb);             // @m11:U:keep:include_in_body_owner_off
}
"####,
    ),
    (
        "11_incomplete.c",
        r####"// A `#define` the census cannot read turns every C owner off (`incomplete`).
int src(void);
void sink(int);
#define 123broken 1

void census_incomplete_off(void) {
    int x = src();        // @inc:D x
    x = 1;
    sink(x);              // @inc:U:keep:census_incomplete
}
"####,
    ),
    (
        "12_fold1_macros.c",
        r####"// Fold 1 census rows (#811 review round 1): macro spellings parsed as keywords or types,
// macros spelled after `->` and in parentheses.
long src(void);
void sink(long);
#define const x1 = saved1;
#define short x2 = saved2; int
#define stepper(v) step_impl(&(v))
struct It { void (*stepper)(long); };

void keyword_macro_keep(void) {
    long x1 = src();      // @k1:D x1
    long saved1 = x1;
    x1 = 0;
    const long y = 0;
    sink(x1);             // @k1:U:keep:keyword_spelled_macro_free_names
    sink(saved1 + y);
}

void type_keyword_macro_keep(void) {
    long x2 = src();      // @k2:D x2
    long saved2 = x2;
    x2 = 0;
    short y = 0;
    sink(x2);             // @k2:U:keep:type_keyword_spelled_macro_free_names
    sink(saved2 + y);
}

void keyword_macro_unrelated_drop(void) {
    long x9 = src();      // @k3:D x9
    x9 = 0;
    sink(x9);             // @k3:U:drop:no_macro_spelled_here
}

void arrow_macro_keep(struct It *it) {
    long a4 = src();      // @a1:D a4
    a4 = 1;
    it->stepper(a4);
    sink(a4);             // @a1:U:keep:macro_after_arrow_takes_address
}

void paren_name_escaped_keep(struct It *it) {
    long a5 = src();      // @a2:D a5
    a5 = 1;
    (stepper)(a5);
    sink(a5);             // @a2:U:keep:parenthesized_name_is_not_an_invocation_but_is_escaped
}
"####,
    ),
    (
        "13_fold2_macro_args.c",
        r####"// Identifiers between the parentheses after an in-repo macro name escape wherever the
// parser puts them: a type position, a declarator position, a call.
int src(void);
void sink(long);
void track(long *p);
long peek(void);
#define BIND2(v) pp = &(v); long
#define TRACKED(v) track(&v); long
#define DECLM(v) tmpv = (long)&(v), dd
long *pp;

void macro_type_arg_keep(void) {
    long x1 = src();      // @f1:D x1
    long saved1 = x1;
    long *p1 = 0;
    BIND2(x1) dummy1;
    x1 = 0;
    *p1 = saved1;
    sink(x1);             // @f1:U:keep:macro_type_specifier_argument
}

void macro_type_arg_track_keep(void) {
    long x2 = src();      // @f2:D x2
    TRACKED(x2) y2 = 0;
    x2 = peek();
    sink(x2 + y2);        // @f2:U:keep:macro_type_specifier_argument_tracked
}

void macro_declarator_arg_keep(void) {
    long x3 = src();      // @f3:D x3
    long DECLM(x3);
    x3 = peek();
    sink(x3);             // @f3:U:keep:macro_declarator_argument
}

void macro_args_control_drop(void) {
    long x4 = src();      // @f4:D x4
    x4 = 0;
    sink(x4);             // @f4:U:drop:no_macro_spelled_here
}
"####,
    ),
    (
        "11_phase_divergence.c",
        r####"// A trigraph `??/` is a backslash: the compiler splices the next line into the comment, the
// parser sees a declaration (PhaseDivergence turns the owner off, `.c` and C++ alike).
int src(void);
void sink(int);

void phase_control_drop(void) {
    int x = src();        // @ph0:D x
    x = 1;
    sink(x);              // @ph0:U:drop:no_phase_divergence_control
}

void phase_trigraph_scalar_keep(int x) {   // @ph1:D x
    // ??/
    int x;
    x = 1;                // @ph1:U:keep:trigraph_splice_hides_a_phantom_declaration
}
"####,
    ),
    (
        "12_phase_splice_cr.c",
        r####"// Fold 2 (`.c`): bare backslash splices and a lone CR, see 16_phase_splice_cr.cc.
int src(void);
void sink(int);

void splice_keep(int x) {   // @sp1:D x
    int a = 1 /\
/*gap*/ 1; int x;
    ;
    x = 1;                // @sp1:U:keep:backslash_splice_makes_a_comment_only_the_compiler_sees
}

void splice_control_drop(void) {
    int x = src();        // @sp3:D x
    x = 1;
    sink(x);              // @sp3:U:drop:control_without_any_splice_or_cr
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
"####,
    ),
    (
        "13_ucn_identifier.c",
        r####"// Fold 3 (`.c`): a backslash-spelled identifier turns the owner off.
int src(void);
void sink(int);

int ucn_scalar_keep(void) {
    int café = src();           // @u2:D café
    café = caf\u00e9;           // @u2:U:keep:ucn_spelling_is_the_same_name_as_the_literal
    return 0;
}

int ucn_consistent_control_drop(void) {
    int café = src();           // @u3:D café
    café = 1;
    sink(café);                 // @u3:U:drop:one_spelling_throughout
    return 0;
}
"####,
    ),
    (
        "20_macro_brackets.c",
        r####"// Bracket macros (#829). `#define` lines are top-level, so every case shares one census.
int src(void);
void sink(int);
void sink2(const char *);
#define END }
#define BEGIN {
#define LEAVE END
#define ENTER BEGIN
#define IF_BEGIN if (cond) {
#define IF_END }
#define OPEN_IF(c) if (c) {
#define RBD %>
#define LBD <%
#define P_OPEN for (
#define P_CLOSE )
#define LB [
#define RB ]
#define BOTH(s) BEGIN s; END
#define DUAL 0
#define DUAL {
#define WRAP(v) { sink(v); }
#define MSG "{ ( ["
#define CH '}'
#define CMT 1 /* { */ + 2 // (
int cond;

// Scope-shift shape (gate-only, see TESTS.md): no baseline pair exists, E01 pins the gate.
void brace_scope(int x) {
    int r = 0;
    {
        int *x = 0;
        END;
        r = x;
        BEGIN;
    }
    sink(r);
}

void brace_transitive(int x) {
    int r = 0;
    {
        int *x = 0;
        LEAVE;
        r = x;
        ENTER;
    }
    sink(r);
}

void brace_control(void) {
    int x = src();            // @b3:D x
    IF_BEGIN;
    x = 2;
    IF_END;
    sink(x);                  // @b3:U:keep:unbalanced_macro_makes_the_write_conditional
}

void brace_function_like(int c) {
    int x = src();            // @b4:D x
    OPEN_IF(c);
    x = 2;
    IF_END;
    sink(x);                  // @b4:U:keep:function_like_unbalanced_macro
}

void brace_digraph(int x) {
    int r = 0;
    {
        int *x = 0;
        RBD;
        r = x;
        LBD;
    }
    sink(r);
}

void paren_macro(int c) {
    int x = src();            // @b6:D x
    P_OPEN; c; P_CLOSE;
    x = 2;
    sink(x);                  // @b6:U:keep:unbalanced_paren_macro_owner_off
}

void bracket_macro(void) {
    int x = src();            // @b7:D x
    LB; RB;
    x = 2;
    sink(x);                  // @b7:U:keep:unbalanced_square_macro_owner_off
}

void balanced_wrapper_of_unbalanced(void) {
    int x = src();            // @b8:D x
    BOTH(sink(0));
    x = 2;
    sink(x);                  // @b8:U:keep:wrapper_names_unbalanced_macros
}

void one_definition_unbalanced(void) {
    int x = src();            // @b9:D x
    sink(DUAL);
    x = 2;
    sink(x);                  // @b9:U:keep:any_unbalanced_definition_taints_the_spelling
}

void balanced_macros_still_prune(void) {
    int x = src();            // @b10:D x
    WRAP(0);
    sink2(MSG);
    sink(CH);
    sink(CMT);
    x = 2;
    sink(x);                  // @b10:U:drop:brackets_in_literals_and_comments_do_not_count
}

void not_spelled_still_prunes(void) {
    int x = src();            // @b11:D x
    sink2("END BEGIN");       /* END BEGIN */
    x = 2;
    sink(x);                  // @b11:U:drop:macro_named_only_in_a_literal_or_comment
}

void external_pthread_pair(void (*h)(void *), int x) {
    int r = 0;
    {
        pthread_cleanup_push(h, 0);
        int *x = 0;
        pthread_cleanup_pop(0);
        r = x;
    }
    sink(r);
}

void external_cpython_pair(void *op, int x) {
    int r = 0;
    {
        Py_BEGIN_CRITICAL_SECTION(op);
        int *x = 0;
        Py_END_CRITICAL_SECTION();
        r = x;
    }
    sink(r);
}

// Guard shape of the same macro classes: the write between the pair is conditional for the
// compiler (`if (cond) {` from the macro) and unconditional for the parser.
void brace_if_guard(void) {
    int x = src();            // @b1:D x
    if (cond) BEGIN;
    x = 2;
    END;
    sink(x);                  // @b1:U:keep:unbalanced_brace_macro_guards_the_write
}

void brace_transitive_if_guard(void) {
    int x = src();            // @b2:D x
    if (cond) ENTER;
    x = 2;
    LEAVE;
    sink(x);                  // @b2:U:keep:macro_naming_an_unbalanced_macro
}

void brace_digraph_if_guard(void) {
    int x = src();            // @b5:D x
    if (cond) LBD;
    x = 2;
    RBD;
    sink(x);                  // @b5:U:keep:digraph_brace_macro
}

void external_pthread_if_guard(void (*h)(void *)) {
    int x = src();            // @b12:D x
    if (cond) pthread_cleanup_push(h, 0);
    x = 2;
    pthread_cleanup_pop(0);
    sink(x);                  // @b12:U:keep:external_block_macro_pair
}

void external_cpython_if_guard(void *op) {
    int x = src();            // @b13:D x
    if (cond) Py_BEGIN_CRITICAL_SECTION(op);
    x = 2;
    Py_END_CRITICAL_SECTION();
    sink(x);                  // @b13:U:keep:external_block_macro_pair
}
"####,
    ),
    (
        "21_macro_args.c",
        r####"// Macro argument lists and comma-carrying macros (#829 part B). One census for the file.
int src(void);
void sink(int);
struct P { int a, b; };
int fn(struct P p);
#define F(p, q) q p
#define ID(v) v
#define PAIR 1, 2
#define INIT {0, 0}
#define FWD(...) __VA_ARGS__
#define CALL(...) fn2(__VA_ARGS__)
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define USE(v) sink(v)
int fn2(int, int);

// Scope-shift shape (gate-only, see TESTS.md): no baseline pair exists, E01 pins the gate.
void arg_split_scope(int x) {
    int r = 0;
    {
        int *x = 0; int bb = 0;
        F({ int a, bb; })
        ;
        r = x;
    }
    sink(r);
}

void arg_split_chain(void) {
    int x = src();            // @a2:D x
    ID(F)({ int a, bb; });
    x = 2;
    sink(x);                  // @a2:U:keep:brace_held_comma_in_a_chained_group
}

void arg_split_compound_literal(void) {
    int x = src();            // @a3:D x
    USE(fn((struct P){1, 2}));
    x = 2;
    sink(x);                  // @a3:U:keep:brace_held_comma_nested_in_parentheses
}

void loose_top_level_comma(void) {
    int x = src();            // @a4:D x
    int arr[] = {PAIR};
    x = 2;
    sink(x + arr[0]);         // @a4:U:keep:macro_with_a_top_level_comma
}

void loose_brace_held_comma(void) {
    int x = src();            // @a5:D x
    int arr[2] = INIT;
    x = 2;
    sink(x + arr[0]);         // @a5:U:keep:macro_with_a_brace_held_comma
}

void loose_variadic_top(void) {
    int x = src();            // @a6:D x
    sink(fn2(FWD(1, 2)));
    x = 2;
    sink(x);                  // @a6:U:keep:variadic_arguments_outside_parentheses
}

void tight_commas_still_prune(int a, int b) {
    int x = src();            // @a7:D x
    sink(MAX(a, fn2(b, a)));
    sink(CALL(a, b));
    x = 2;
    sink(x);                  // @a7:U:drop:commas_directly_inside_parentheses
}

void non_macro_call_still_prunes(void) {
    int x = src();            // @a8:D x
    sink(fn((struct P){1, 2}));
    x = 2;
    sink(x);                  // @a8:U:drop:brace_held_comma_in_a_function_call
}
"####,
    ),
    (
        "22_macro_wrappers.c",
        r####"// In-repo macros that wrap listed external block macros (#829 round 1, G-terra-01, G-sol-01,
// G-sol-08, G-terra-07). One census for the file.
int src(void);
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
#define PY_NT Py_BEGIN_ALLOW_THREADS
#define PY_NT_END Py_END_ALLOW_THREADS
#define WHICH() pthread_cleanup_push
#define WHICH_POP() pthread_cleanup_pop
#define OPENV(v) {
#define PASS(pthread_cleanup_push) (pthread_cleanup_push)

void alias_object_like(void (*h)(void *)) {
    int x = src();            // @w1:D x
    if (cond) P(h, 0);
    x = 2;
    Q(0);
    sink(x);                  // @w1:U:keep:object_like_alias_of_a_listed_macro
}

void wrapper_function_like(void (*h)(void *)) {
    int x = src();            // @w2:D x
    if (cond) ENTER(h, 0);
    x = 2;
    LEAVE(0);
    sink(x);                  // @w2:U:keep:function_like_wrapper_of_a_listed_macro
}

void wrapper_multi_hop(void (*h)(void *)) {
    int x = src();            // @w3:D x
    if (cond) E2(h, 0);
    x = 2;
    L2(0);
    sink(x);                  // @w3:U:keep:multi_hop_wrapper_of_a_listed_macro
}

void wrapper_critical_section(void *op) {
    int x = src();            // @w4:D x
    if (cond) PY_IN(op);
    x = 2;
    PY_OUT();
    sink(x);                  // @w4:U:keep:wrapper_of_a_listed_cpython_macro
}

void wrapper_allow_threads(void) {
    int x = src();            // @w5:D x
    if (cond) PY_NT;
    x = 2;
    PY_NT_END;
    sink(x);                  // @w5:U:keep:object_like_alias_of_a_listed_cpython_macro
}

void wrapper_returns_a_callable_name(void (*h)(void *)) {
    int x = src();            // @w8:D x
    if (cond) WHICH()(h, 0);
    x = 2;
    WHICH_POP()(0);
    sink(x);                  // @w8:U:keep:wrapper_expanding_to_a_listed_macro_name
}

void parameter_shadow_control(void) {
    int x = src();            // @w6:D x
    int n = PASS(4);
    x = 2;
    sink(x + n);              // @w6:U:drop:a_parameter_spelled_like_a_listed_name_is_not_free
}

void parenthesized_unbalanced_name(void) {
    int x = src();            // @w7:D x
    (OPENV)(0);
    x = 2;
    sink(x);                  // @w7:U:keep:spelled_unbalanced_name_without_an_invocation
}
"####,
    ),
    (
        "23_macro_args_live.c",
        r####"// Live argument splits, rescans and held-comma controls (#829 round 1, G-sol-02/03/04/06/07,
// G-terra-02). One census for the file.
int src(void);
void sink(int);
struct P { int a, b; };
int M(struct P p) { return p.a; }
#define F(p, q) q if (c) p
#define G(p, q) q p
#define H(p) F(p)
#define J(p) G(p)
#define INIT { int a, b; }
#define AF F
#define AG G
#define ID(v) v
#define TS(p, q) q p
#define SZ(v) sizeof(v)
#define M(p, q) q p

int a1_live(int c) {
    int x = 41;               // @a1:D x
    int b = 0;
    {
        F({ int a, b; });
        x = 2;
        G({ int a, b; });
        return x;             // @a1:U:keep:live_brace_held_comma_moves_the_guard
    }
}

int a1_terra(int c) {
    int x = 1;                // @a1t:D x
    { {
    TS({ if (c) { 0, 0; } });
    x = 2;
    TS({ 0, 0; });
    return x;                 // @a1t:U:keep:live_brace_held_comma_moves_the_guard
    } }
}

int rescan_split(int c) {
    int x = 41;               // @a9:D x
    int b = 0;
    {
        H(INIT);
        x = 2;
        J(INIT);
        return x;             // @a9:U:keep:the_split_exists_only_after_rescan
    }
}

int split_through_an_object_alias(int c) {
    int x = 41;               // @a10:D x
    int b = 0;
    {
        AF({ int a, b; });
        x = 2;
        AG({ int a, b; });
        return x;             // @a10:U:keep:object_like_alias_of_a_splitting_macro
    }
}

int split_after_a_comment(int c) {
    int x = 41;               // @a11:D x
    int b = 0;
    {
        F /* c */ ({ int a, b; });
        x = 2;
        G /* c */ ({ int a, b; });
        return x;             // @a11:U:keep:comment_between_name_and_group
    }
}

int split_through_a_chained_group(int c) {
    int x = 41;               // @a12:D x
    int b = 0;
    {
        ID(F)({ int a, b; });
        x = 2;
        ID(G)({ int a, b; });
        return x;             // @a12:U:keep:argument_expanding_to_the_macro_name
    }
}

void held_comma_in_a_char_literal(void) {
    int x = src();            // @k1:D x
    int n = SZ((char[]){','});
    x = 2;
    sink(x + n);              // @k1:U:drop:a_comma_character_is_not_a_comma
}

void held_comma_inside_parentheses(void) {
    int x = src();            // @k2:D x
    int n = SZ((int[]){(1, 2)});
    x = 2;
    sink(x + n);              // @k2:U:drop:innermost_bracket_is_a_parenthesis
}

void inhibited_invocation(void) {
    int x = src();            // @k3:D x
    int n = (M)((struct P){1, 2});
    x = 2;
    sink(x + n);              // @k3:U:drop:parenthesized_name_is_not_an_invocation
}
"####,
    ),
    (
        "24_macro_splice_crlf.c",
        r####"// <CRLF-FILE>
// A macro name assembled by a splice, in a CRLF file (#829 round 1, G-terra-10).
int src(void);
void sink(int);
#define OP\
EN {
#define CLOSE }
int cond;

void spliced_name_crlf(void) {
    int x = src();            // @s1:D x
    if (cond) OPEN /* comment */ ;
    x = 2;
    CLOSE;
    sink(x);                  // @s1:U:keep:macro_name_assembled_by_a_splice
}
"####,
    ),
    (
        "24_macro_splice.c",
        r####"// A macro name assembled by a splice (#829 round 1, G-terra-10).
int src(void);
void sink(int);
#define OP\
EN {
#define CLOSE }
int cond;

void spliced_name(void) {
    int x = src();            // @s2:D x
    if (cond) OPEN /* comment */ ;
    x = 2;
    CLOSE;
    sink(x);                  // @s2:U:keep:macro_name_assembled_by_a_splice
}
"####,
    ),
    (
        "25_r2_external_alias.c",
        r####"// Chain A round 2 (#829): an external alias hides an in-repo macro. G2-terra-01 and
// G2-opus-06, both BOUNDARY. A, B and EXT_APPLY are never defined here: they stand for a header
// outside the repository (-include, -D, a system header). One census for the file.
int src(void);
void sink(int);
#define F(p, q) q if (c) p
#define G(p, q) q p
#define APPLY(m, ...) m(__VA_ARGS__)

int external_alias_of_in_repo_macro(int c) {
    int x = 41;               // @g2t01:D x
    int b = 0;
    {
        A({ int a, b; });
        x = 2;
        B({ int a, b; });
        return x;             // @g2t01:U:keep:external_alias_exposes_an_in_repo_split
    }
}

int external_apply_forwards_an_in_repo_macro_name(int c) {
    int x = 41;               // @g2o06:D x
    int b = 0;
    {
        EXT_APPLY(F, { int a, b; });
        x = 2;
        EXT_APPLY(G, { int a, b; });
        return x;             // @g2o06:U:keep:external_variadic_apply_forwards_the_held_comma
    }
}

int in_repo_apply_forwards_an_in_repo_macro_name(int c) {
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
        "26_r2_external_names.c",
        r####"// Chain A round 2 (#829): external block macros the repository never defines
// (G2-sonnet-01 and members, G2-sonnet-02 BOUNDARY, G2-sonnet-03, G2-opus-09 BOUNDARY).
// No `#define` anywhere in this file: every macro name below is external.
int src(void);
void sink(int);
int g;
typedef int T;
void use(int);

int trashcan_begin_condition(void *op, int c) {
    int x = 41;               // @g2s01:D x
    if (c) Py_TRASHCAN_BEGIN_CONDITION(op, 1);
    x = 2;
    Py_TRASHCAN_END;
    return x;                 // @g2s01:U:keep:unlisted_cpython_trashcan_macro
}

int cs_mut(void *m, int c) {
    int x = 41;               // @g2s01a:D x
    if (c) Py_BEGIN_CRITICAL_SECTION_MUT(m);
    x = 2;
    Py_END_CRITICAL_SECTION();
    return x;                 // @g2s01a:U:keep:cpython_313_critical_section_mut
}

int cs2_mut(void *m, void *m2, int c) {
    int x = 41;               // @g2s01b:D x
    if (c) Py_BEGIN_CRITICAL_SECTION2_MUT(m, m2);
    x = 2;
    Py_END_CRITICAL_SECTION2();
    return x;                 // @g2s01b:U:keep:cpython_313_critical_section2_mut
}

int cs_sequence_fast(void *seq, int c) {
    int x = 41;               // @g2s01c:D x
    if (c) Py_BEGIN_CRITICAL_SECTION_SEQUENCE_FAST(seq);
    x = 2;
    Py_END_CRITICAL_SECTION_SEQUENCE_FAST();
    return x;                 // @g2s01c:U:keep:cpython_313_critical_section_sequence_fast
}

int suppress_iph(int c) {
    int x = 41;               // @g2s01d:D x
    if (c) _Py_BEGIN_SUPPRESS_IPH;
    x = 2;
    _Py_END_SUPPRESS_IPH;
    return x;                 // @g2s01d:U:keep:cpython_msvc_suppress_iph_pair
}

int numpy_allow_threads(int c) {
    int x = 41;               // @g2s02:D x
    if (c) NPY_BEGIN_ALLOW_THREADS;
    x = 2;
    NPY_END_ALLOW_THREADS;
    return x;                 // @g2s02:U:keep:numpy_alias_of_the_allow_threads_pair
}

int ext_prefix_deref_store(int *p, int c) {   // @g2s03a:D p
    M *p = 2;                 // @g2s03a:U:keep:external_prefix_makes_a_declaration_a_store
    return g;
}

int ext_prefix_pointer_to_pointer(int **pp, int c) {   // @g2s03b:D pp
    M **pp = 0;               // @g2s03b:U:keep:external_prefix_pointer_to_pointer_store
    return g;
}

int ext_function_like_prefix(int *p, int c) {   // @g2s03c:D p
    M(c) *p = 2;              // @g2s03c:U:keep:external_function_like_prefix_store
    return g;
}

int ext_prefix_with_read(int *p, int *q, int c) {   // @g2s03d:D p
    M *p = *q;                // @g2s03d:U:keep:external_prefix_store_beside_a_read
    return g;
}

int ext_prefix_two_declarators(int *p, int *q, int c) {   // @g2s03e1:D p  @g2s03e2:D q
    M *p = 2, *q = 3;         // @g2s03e1:U:keep:external_prefix_first_declarator  @g2s03e2:U:keep:external_prefix_second_declarator
    return g;
}

int ext_prefix_plain_declaration(int x, int c) {   // @g2s03k1:D x
    M x = 2;                  // @g2s03k1:U:drop:non_pointer_declaration_is_a_write_only_line
    return g;
}

int ext_prefix_declaration_without_value(int x, int c) {   // @g2s03k2:D x
    M x;                      // @g2s03k2:U:drop:non_pointer_declaration_without_value_is_write_only
    return g;
}

int real_nested_redeclaration(int *q) {
    {
        T *p = 0;             // @g2s03k6:D p
        p = 0;                // @g2s03k6:U:drop:overwritten_inside_a_real_inner_declaration
        use(*p);              // @g2s03k6:U:drop:dominated_by_the_overwrite
    }
    return g;
}

int pg_try_unbraced(void) {
    int x = src();            // @g2o09:D x
    PG_TRY();
    x = 2;
    PG_CATCH();
    PG_END_TRY();
    sink(x);                  // @g2o09:U:keep:unlisted_postgres_try_family_unbraced
}
"####,
    ),
    (
        "27_r2_inrepo_prefix.c",
        r####"// Chain A round 2 (#829): in-repo statement-prefix macros keep the pair today
// (G2-sonnet-03, control K5). One census for the file.
int g;
#define IFC if (c)
#define NOTHING

int in_repo_if_prefix(int *p, int c) {   // @g2s03k5a:D p
    IFC *p = 2;               // @g2s03k5a:U:keep:in_repo_prefix_declaration_is_macro_typed
    return g;
}

int in_repo_empty_prefix(int *p, int c) {   // @g2s03k5b:D p
    NOTHING *p = 2;           // @g2s03k5b:U:keep:in_repo_empty_prefix_declaration_is_macro_typed
    return g;
}
"####,
    ),
    (
        "28_r2_listed_name_shim.c",
        r####"// Chain A round 2 (#829): the repository defines a shim under a listed system name
// (G2-sonnet-07). The system macro may be the one in effect (include order, #ifdef), so the
// listed spelling is conservative whatever the repository holds. One census for the file.
int src(void);
void sink(int);
int cond;
#define pthread_cleanup_push(r, a) do { (r)(a); } while (0)
#define pthread_cleanup_pop(e) ((void)(e))

void shimmed_listed_name(void (*h)(void *)) {
    int x = src();            // @g2s07:D x
    if (cond) pthread_cleanup_push(h, 0);
    x = 2;
    pthread_cleanup_pop(0);
    sink(x);                  // @g2s07:U:keep:listed_name_is_refused_even_when_the_repository_defines_it
}
"####,
    ),
    (
        "29_r2_enclosing.c",
        r####"// Chain A round 2 (#829): the owner sits inside an in-repo macro's argument group at file
// scope (G2-opus-01). tree-sitter recovers: `J(static int dummy;` and `)` are ERROR nodes and
// `probe` is a clean owner that spells no macro. One census for the file.
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
        "30_r2_enclosing_variadic.c",
        r####"// Chain A round 2 (#829): control for G2-opus-01. A variadic forwarding macro rejoins the
// pieces with commas, so the compiled program equals the written one and the overwrite
// dominates: pruning must survive. One census for the file.
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
        "31_r2_enclosing_file_scope.c",
        r####"// Chain A round 2 (#829): the enclosing group is opened by a file-scope unbalanced macro
// (G2-opus-01b). The owner does not spell BEGIN_FNS; its closing parenthesis is written out.
// One census for the file.
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
        "32_r2_strip_parenthesis.c",
        r####"// Chain A round 2 (#829): a brace-held comma below an inner parenthesis splits after a rescan
// strips the parenthesis (G2-opus-02). Parameter names differ from the owner's locals (a
// parameter of a spelled macro escapes the name). One census for the file.
#define F(p, q) q if (c) p
#define G(p, q) q p
#define SF(v) F v
#define SG(v) G v

int stripped_parenthesis(int c) {
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
        "33_r2_digit_separator.c",
        r####"// Chain A round 2 (#829): in the C reading (no digit separators) a pair of `'` swallows the
// bracket, in C23 and C++ it is exposed (G2-opus-03). The census verdict is the OR of the
// readings. One census for the file.
#define OPEN if (c != 1'0) { (void)(2'0);
#define CLOSE (void)(3'0); } (void)(4'0);

int digit_separator_hides_a_bracket(int c) {
    int x = 41;               // @g2o03:D x
    OPEN;
    x = 2;
    CLOSE;
    return x;                 // @g2o03:U:keep:c_reading_hides_the_brackets_of_the_c23_reading
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
