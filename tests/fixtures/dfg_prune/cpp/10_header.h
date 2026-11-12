// .h obligations (#630 slice 6c): C grammar, C++ rules (a .h may be included from C++).
int src(void);
void sink(int);
void f(int);

static inline int header_scalar_drop(void) {
    int x = src();            // @hs:D x
    x = 1;
    return x + 1;             // @hs:U:drop:scalar_in_a_header
}

static inline int header_cpp_semantics_keep(void) {
    int x = src();            // @hc:D x
    x = 1;
    f(x);
    x = 2;
    return x + 1;             // @hc:U:keep:bare_argument_could_be_a_cxx_reference
}

static inline int header_typedef_keep(void) {
    handle_t h = src();       // @ht:D h
    h = 1;
    return h + 1;             // @ht:U:keep:typedef_fails_the_scalar_gate
}

static inline int header_struct_keep(void) {
    struct S s = mk();        // @hu:D s
    s = mk();
    return s.v + 1;           // @hu:U:keep:struct_type
}
