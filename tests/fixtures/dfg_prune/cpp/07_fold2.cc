// C++ U/D obligations (#630 slice 6c, fold 2): every keep row has a baseline pair on the frozen
// producer and a value path the fold-1 head dropped (or, where noted, a guard that already
// held). Drop rows are controls: plain scalar paths still prune.
namespace N { int x; }
int g;
struct S { static int m; };
int S::m;
int source();
void sink(int);
int *rp;
int saved_v;
void keep_it(int& r) { rp = &r; saved_v = r; }
void restore_v() { *rp = saved_v; }
int **pp;
void keep_pp_ref(int *&r) { pp = &r; }
void restore_pp(int *c) { *pp = c; }
typedef int& IR;
struct Ret {
    Ret() {}
    Ret(int& r) { rp = &r; saved_v = r; }
    Ret& operator=(int& r) { rp = &r; saved_v = r; return *this; }
    int member;
};
Ret gl;
Ret& get(Ret& w) { return w; }
void qsave(int v) { N::x = v; }
void qsave_g(int v) { g = v; }
void qsave_m(int v) { S::m = v; }

// ---- fix A: the name of a qualified identifier is never an occurrence of a local ----
int qual_ns_keep() {
    int x = source();         // @qa:D x
    qsave(x + 0);
    x = 0;
    return N::x + 1;          // @qa:U:keep:namespace_qualified_use_is_not_the_local
}

int qual_global_keep() {
    int g = source();         // @qb:D g
    qsave_g(g + 0);
    g = 0;
    return ::g + 1;           // @qb:U:keep:global_qualified_use_is_not_the_local
}

int qual_static_member_keep() {
    int m = source();         // @qc:D m
    qsave_m(m + 0);
    m = 0;
    return S::m + 1;          // @qc:U:keep:static_member_use_is_not_the_local
}

int qual_write_guard_keep() {
    int x = source();         // @qd:D x
    N::x = 0;
    return x + 1;             // @qd:U:keep:qualified_write_is_not_a_write_of_the_local
}

int qual_control_drop() {
    int x = source();         // @qe:D x
    x = 0;
    return x + 1;             // @qe:U:drop:ordinary_local_control
}

// ---- controls: plain scalar paths still prune ----
int ctl_scalar_drop() {
    int x = source();         // @c1:D x
    x = 1;
    return x + 1;             // @c1:U:drop:plain_int
}

long ctl_long_drop() {
    long y = source();        // @c2:D y
    y = 2;
    return y + 1;             // @c2:U:drop:keyword_long
}

int *ctl_pointer_assign_drop(int *a, int *b) {
    int *p = a;               // @c3:D p
    p = b;
    return p;                 // @c3:U:drop:pointer_assignment
}

int ctl_assign_rhs_drop() {
    int y = source();         // @c4:D y
    int x = 0;
    x = y;
    y = 5;
    return y + x;             // @c4:U:drop:scalar_left_does_not_escape_the_right
}

unsigned long long ctl_sized_drop() {
    unsigned long long u = source();  // @c5:D u
    u = 7;
    return u + 1;             // @c5:U:drop:keyword_sized_type
}

// ---- fix C: a cast escapes its operand's lvalue roots unless the target is a built-in scalar ----
int cast_stmt_keep() {
    int x = source();         // @ca:D x
    (Ret)x;
    x = 0;
    restore_v();
    return x + 1;             // @ca:U:keep:cast_statement_to_class
}

int cast_typedef_ref_arg_keep() {
    int x = source();         // @cb:D x
    keep_it((IR)x);
    x = 0;
    restore_v();
    return x + 1;             // @cb:U:keep:cast_to_typedef_reference_argument
}

int cast_unknown_name_keep() {
    int x = source();         // @cc:D x
    keep_it((Unknown)x);
    x = 0;
    restore_v();
    return x + 1;             // @cc:U:keep:cast_to_an_unknown_name
}

int cast_operand_keep() {
    int x = source();         // @cd:D x
    Ret r = ((Ret)x, Ret());
    x = 0;
    restore_v();
    return x + 1;             // @cd:U:keep:cast_inside_a_comma_operand
}

int cast_static_cast_keep() {
    int x = source();         // @ce:D x
    keep_it(static_cast<IR>(x));
    x = 0;
    restore_v();
    return x + 1;             // @ce:U:keep:named_cast_guard
}

int cast_functional_keep() {
    int x = source();         // @cf:D x
    Ret r = Ret(x);
    x = 0;
    restore_v();
    return x + 1;             // @cf:U:keep:functional_cast_guard
}

int cast_brace_keep() {
    int x = source();         // @cg:D x
    Ret r = Ret{x};
    x = 0;
    restore_v();
    return x + 1;             // @cg:U:keep:brace_cast_guard
}

int cast_scalar_drop() {
    int x = source();         // @ch:D x
    int y = (int)x + 0;
    x = 0;
    return x + y;             // @ch:U:drop:scalar_cast_is_a_prvalue
}

// ---- fix D: a reference declarator, in every form, escapes its initializer ----
int ref_auto_direct_keep() {
    int x = source();         // @da:D x
    auto& r(x);
    keep_it(r);
    x = 0;
    restore_v();
    return x + 1;             // @da:U:keep:auto_reference_direct_init
}

int ref_multi_direct_keep() {
    int x = source();         // @db:D x
    int &r(x), &t(x);
    keep_it(r);
    x = 0;
    restore_v();
    return x + 1;             // @db:U:keep:multi_declarator_direct_init
}

int ref_brace_keep() {
    int x = source();         // @dc:D x
    int& r{x};
    keep_it(r);
    x = 0;
    restore_v();
    return x + 1;             // @dc:U:keep:reference_brace_init
}

int ref_switch_init_keep() {
    int x = source();         // @dd:D x
    switch (int& r(x); r) { default: keep_it(r); }
    x = 0;
    restore_v();
    return x + 1;             // @dd:U:keep:reference_in_a_switch_init
}

int *ref_to_pointer_keep(int *a, int *b) {
    int *p = a;               // @de:D p
    int *copy = p;
    int *&rr(p);
    keep_pp_ref(rr);
    p = b;
    restore_pp(copy);
    return p;                 // @de:U:keep:reference_to_pointer_direct_init
}

int ref_if_cond_keep() {
    int x = source();         // @df:D x
    if (int& r = x) { keep_it(r); }
    x = 0;
    restore_v();
    return x + 1;             // @df:U:keep:reference_declared_in_an_if_condition
}

// ---- fix E: assignment into a non-scalar left side escapes the right side ----
int assign_global_keep() {
    int x = source();         // @ea:D x
    gl = x;
    x = 0;
    restore_v();
    return x + 1;             // @ea:U:keep:global_class_left
}

int assign_comma_lvalue_keep(bool c) {
    Ret w;
    int x = source();         // @eb:D x
    (c, w) = x;
    x = 0;
    restore_v();
    return x + 1;             // @eb:U:keep:comma_lvalue_left
}

int assign_compound_cond_left_keep(bool c) {
    int x = source();         // @ec:D x
    int y = 0;
    (c ? x : y) += 1;
    x = 0;
    return x + 1;             // @ec:U:keep:compound_assignment_through_a_conditional_lvalue
}

// ---- fix F: names an unmodelled block-scope form introduces escape ----
int anon_union_block_keep() {
    int x = 0;
    {
        union { int x; int y; };
        x = source();         // @fa:D x
        int saved = y;
        x = 0;
        y = saved;
        int z = x + 1;        // @fa:U:keep:anonymous_union_members_alias
        return z;
    }
}

int anon_struct_in_union_keep() {
    int a = 0;
    {
        union { struct { int a; int b; }; long both; };
        a = source();         // @fb:D a
        long saved = both;
        a = 0;
        both = saved;
        int z = a + 1;        // @fb:U:keep:anonymous_struct_in_anonymous_union
        return z;
    }
}
