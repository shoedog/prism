// C++ U/D obligations (#630 slice 6c): the scalar gate. No macros here (census facts live inline).
// A use is spelled `x + 1` where the row needs a drop: a bare `f(x)` argument is an escape.
int src();
void sink(int);
void take(int&);
int cond();
std::string mk();
void use_s(std::string);

// `size_t` is a typedef (or any class of that name), not a keyword built-in: fold 2 keeps it.
void size_t_keep() {
    size_t n = src();         // @st:D n
    n = 1;
    sink(n + 1);              // @st:U:keep:size_t_is_not_a_keyword_scalar
}

void sized_and_enum_drop() {
    unsigned long u = src();  // @se:D u
    u = 1;
    sink(u + 1);              // @se:U:drop:sized_type
    enum E2 f2 = src();       // @en:D f2
    f2 = E2(1);
    sink(f2 + 1);             // @en:U:drop:enum_type
}

void pointer_drop() {
    char *p = buf();          // @pd:D p
    p = other();
    sink(*p + 1);             // @pd:U:drop:pointer_declarator
}

void pointer_to_class_drop() {
    std::string *p = pbuf();  // @pc:D p
    p = pother();
    sink(p->size());          // @pc:U:drop:pointer_to_class_is_a_pointer
}

void auto_literal_drop() {
    auto a = 5;               // @al:D a
    a = 6;
    sink(a + 1);              // @al:U:drop:auto_with_literal
}

void auto_call_keep() {
    auto a = src();           // @ac:D a
    a = 6;
    sink(a + 1);              // @ac:U:keep:auto_from_call
}

void auto_ident_keep(int b) {
    auto a = b;               // @ai:D a
    a = 6;
    sink(a + 1);              // @ai:U:keep:auto_from_name
}

void class_type_keep() {
    std::string s = mk();     // @ct:D s
    s = mk();
    sink(s.size());           // @ct:U:keep:class_type
}

void typedef_type_keep() {
    Handle h = open_it();     // @ty:D h
    h = open_it();
    sink(h + 1);              // @ty:U:keep:typedef_name
}

void decltype_keep(int b) {
    decltype(b) d = src();    // @dt:D d
    d = 2;
    sink(d + 1);              // @dt:U:keep:decltype
}

void template_type_keep() {
    Vec<int> v = mkv();       // @tt:D v
    v = mkv();
    sink(v.size());           // @tt:U:keep:template_type
}

void method_receiver_keep() {
    std::string s = mk();     // @mr:D s
    int n = s.size();
    s = mk();
    s.append(n);
    sink(s.size());           // @mr:U:keep:receiver_mutation
}

void range_for_class_keep() {
    std::vector<int> v = mkv(); // @rf:D v
    v = mkv();
    for (int e : v) {         // @rf:U:keep:range_for_over_class
        sink(e + 1);
    }
}

void converting_ctor_keep() {
    int x = src();            // @cc:D x
    x = 1;
    Obj o = x;                // @cc:U:keep:converting_ctor_reference
    sink(o.v);
}

void reference_variable_keep() {
    int &r = ref_src();       // @rv:D r
    r = 1;
    sink(r + 1);              // @rv:U:keep:reference_declarator
}

void plain_control_drop() {
    int x = src();            // @pl:D x
    x = 1;
    sink(x + 1);              // @pl:U:drop:plain_int_control
}
