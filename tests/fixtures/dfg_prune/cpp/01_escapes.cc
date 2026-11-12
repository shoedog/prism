// C++ U/D obligations (#630 slice 6c): the E3 escape set.
int src();
void sink(int);
void take(int&);
void takep(int*);
int cond();

void bare_arg_escape_keep() {
    int x = src();            // @ba:D x
    x = 1;
    take(x);
    x = 2;
    sink(x + 1);              // @ba:U:keep:bare_call_argument
}

void paren_arg_escape_keep() {
    int x = src();            // @pa:D x
    x = 1;
    take((x));
    sink(x + 1);              // @pa:U:keep:parenthesized_argument
}

void cond_arg_escape_keep(int y) {
    int x = src();            // @cq:D x
    x = 1;
    take(cond() ? x : y);
    sink(x + 1);              // @cq:U:keep:conditional_lvalue_argument
}

void new_arg_escape_keep() {
    int x = src();            // @na:D x
    x = 1;
    int *p = new int(x);
    sink(x + 1);              // @na:U:keep:new_argument
}

void brace_init_escape_keep() {
    int x = src();            // @bi:D x
    x = 1;
    Pair q{x, 2};
    sink(x + 1);              // @bi:U:keep:brace_initializer
}

void direct_init_escape_keep() {
    int x = src();            // @di:D x
    x = 1;
    Obj o(x);
    sink(x + 1);              // @di:U:keep:direct_init_argument
}

void functional_cast_keep() {
    int x = src();            // @fc:D x
    x = 1;
    Obj o = Obj(x);
    sink(x + 1);              // @fc:U:keep:call_expression_argument
}

void stream_extract_keep(In &in) {
    int x = src();            // @sx:D x
    x = 1;
    in >> x;
    sink(x + 1);              // @sx:U:keep:shift_right_operand
}

void stream_insert_drop(Out &out) {
    int x = src();            // @si:D x
    x = 1;
    out << x;
    sink(x + 1);              // @si:U:drop:shift_left_is_not_an_escape_x3
}

void ref_init_keep_U() {
    int x = src();            // @ri:D x
    int &r = x;
    sink(r + 1);
    x = 0;                    // @ri:U:keep:reference_initializer
}

void ref_init_keep_D() {
    int x = src();            // @rd:D x
    x = 1;
    const int &r = x;
    sink(r + 1);
    sink(x + 2);              // @rd:U:keep:reference_initializer
}

void structured_binding_keep() {
    int x = src();            // @sb:D x
    x = 1;
    auto [p, q] = x;
    sink(x + 1);              // @sb:U:keep:structured_binding_initializer
}

void ref_init_capture_keep() {
    int x = src();            // @rc:D x
    x = 1;
    auto l = [&r = x]() { r = 5; };
    l();
    sink(x + 1);              // @rc:U:keep:reference_init_capture
}

void ref_capture_keep() {
    int x = src();            // @rf:D x
    x = 1;
    auto l = [&x]() { x = 5; };
    l();
    sink(x + 1);              // @rf:U:keep:reference_capture
}

void default_capture_keep() {
    int x = src();            // @dc:D x
    x = 1;
    auto l = [&]() { x = 5; };
    l();
    sink(x + 1);              // @dc:U:keep:default_reference_capture
}

void value_capture_keep() {
    int x = src();            // @vc:D x
    x = 1;
    auto l = [x]() { return x; };
    sink(x + 1);              // @vc:U:keep:any_capture_reads_x_at_creation
}

void address_of_keep() {
    int x = src();            // @ad:D x
    x = 1;
    takep(&x);
    sink(x + 1);              // @ad:U:keep:address_taken
}

void range_for_ref_keep() {
    int x = src();            // @fr:D x
    x = 1;
    for (auto &e : x) { sink(e); }
    sink(x + 1);              // @fr:U:keep:range_for_reference_over_x
}

void unrelated_name_drop() {
    int x = src();            // @un:D x
    int y = src();
    take(y);
    x = 1;
    sink(x + 1);              // @un:U:drop:escape_is_by_name_of_y_only
}

struct Ctor {
    int m;
    Ctor(int a);
};
Ctor::Ctor(int a) : m(a) {  // @mi:D a
    a = 1;
    sink(a + 1);              // @mi:U:keep:mem_initializer_argument
}

void method_scope_member_keep();
void Cls::meth() {
    m_ = src();               // @mu:D m_
    m_ = 1;
    sink(m_ + 1);             // @mu:U:keep:unqualified_member_is_not_a_local
}

void this_field_keep() {
    this->f = src();          // @tf:D this.f
    this->f = 1;
    sink(this->f + 1);        // @tf:U:keep:field_path_stays_off
}
