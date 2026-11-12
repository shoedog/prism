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
        r####"// `.h` (C grammar, C++ rules): a macro naming the local keeps its pairs.
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
    return hy + 1;        // @h2:U:drop:scalar_without_macro
}
"####,
    ),
];
