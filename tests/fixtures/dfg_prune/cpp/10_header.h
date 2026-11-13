// .h obligations (#630 slice 6c, fold 1): a `.h` is parsed as C (#529) but may hold C++, and the
// C grammar parses C++ shapes such as `T& r = (x);` without an error, so `.h` owners are off.
int src(void);
void sink(int);
void f(int);

static inline int header_scalar_drop(void) {
    int x = src();            // @hs:D x
    x = 1;
    return x + 1;             // @hs:U:keep:dot_h_owner_is_off
}

static inline int header_cpp_semantics_keep(void) {
    int x = src();            // @hc:D x
    x = 1;
    f(x);
    x = 2;
    return x + 1;             // @hc:U:keep:dot_h_owner_is_off_bare_argument
}

static inline int header_typedef_keep(void) {
    handle_t h = src();       // @ht:D h
    h = 1;
    return h + 1;             // @ht:U:keep:dot_h_owner_is_off_typedef
}

static inline int header_struct_keep(void) {
    struct S s = mk();        // @hu:D s
    s = mk();
    return s.v + 1;           // @hu:U:keep:dot_h_owner_is_off_struct
}

typedef int T;
void save(T& r);
void restore(T& r);
static inline void header_typedef_ref_paren_keep(void) {
    int x = src();            // @hr:D x
    T& r = (x);
    save(r);
    x = 0;
    restore(r);
    int z = x + 1;            // @hr:U:keep:dot_h_is_off_the_c_grammar_hides_the_reference
    sink(z);
}
