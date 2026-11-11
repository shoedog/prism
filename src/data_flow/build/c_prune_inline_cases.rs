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

void paren_macro_keep(struct It *it) {
    long a5 = src();      // @a2:D a5
    a5 = 1;
    (stepper)(a5);
    sink(a5);             // @a2:U:keep:parenthesized_macro_name
}
"####,
    ),
];
