// C++ U/D obligations (#630 slice 6c, fold 1): every row has a baseline pair on the frozen
// producer and a value path the first 6c head dropped.
enum E { Zero, Seed };
namespace N { int x; }
int src();
int source();
void sink(int);
void take(int&);
void save();
void restore();
void may();
struct Task {};
int element[1];
E *retained;
E saved_e;
int *begin(E& e) { retained = &e; saved_e = e; return element; }
int *end(E&) { return element + 1; }
void restore_e() { *retained = saved_e; }

// ---- fix 1: an address-of site escapes every lvalue root of its operand ----
int addr_conditional_keep(bool c) {
    int x = source();         // @a1:D x
    int saved = x + 0;
    int *p = &(c ? x : x);
    x = 0;
    *p = saved;
    return x + 1;             // @a1:U:keep:address_of_conditional_lvalue
}

int addr_comma_keep(bool c) {
    int x = source();         // @a2:D x
    int saved = x + 0;
    int *p = &(c, x);
    x = 0;
    *p = saved;
    return x + 1;             // @a2:U:keep:address_of_comma_right_operand
}

int addr_assign_keep() {
    int x = source();         // @a3:D x
    int saved = x + 0;
    int *p = &(x = 1);
    x = 0;
    *p = saved;
    return x + 1;             // @a3:U:keep:address_of_assignment_target
}

int addr_update_keep() {
    int x = source();         // @a4:D x
    int saved = x + 0;
    int *p = &++x;
    x = 0;
    *p = saved;
    return x + 1;             // @a4:U:keep:address_of_prefix_update
}

int addr_cast_keep() {
    int x = source();         // @a5:D x
    int saved = x + 0;
    int *p = &(int&)(x);
    x = 0;
    *p = saved;
    return x + 1;             // @a5:U:keep:address_of_cast_to_reference
}

// The call-argument side of the same root analysis (already an escape before fold 1).
void call_arg_conditional_keep(bool c) {
    int x = source();         // @c1:D x
    take(c ? x : x);
    x = 0;
    sink(x + 1);              // @c1:U:keep:conditional_lvalue_call_argument
}

void call_arg_assign_keep() {
    int x = source();         // @c2:D x
    take((x = 1));
    x = 0;
    sink(x + 1);              // @c2:U:keep:parenthesized_assignment_call_argument
}

void call_arg_update_keep() {
    int x = source();         // @c3:D x
    take(++x);
    x = 0;
    sink(x + 1);              // @c3:U:keep:prefix_update_call_argument
}

// ---- fix 2: structured bindings under a wrapper bind the names, never an outer one ----
int structured_binding_ref_shadow_keep() {
    int x = 0;
    int a[2];
    {
        auto &[x, y] = a;
        x = source();         // @s1:D x
        int saved = x + 0;
        x = 0;
        a[0] = saved;
        return x + 1;         // @s1:U:keep:reference_structured_binding_shadows_the_outer_name
    }
}

int structured_binding_rref_shadow_keep() {
    int x = 0;
    int a[2];
    {
        auto &&[x, y] = a;
        x = source();         // @s2:D x
        int saved = x + 0;
        x = 0;
        a[0] = saved;
        return x + 1;         // @s2:U:keep:rvalue_reference_structured_binding_shadows
    }
}

int structured_binding_range_for_keep(int (*arrs)[2], int n) {
    int x = 0;
    for (auto &[x, y] : arrs) {
        x = source();         // @s3:D x
        int saved = x + 0;
        x = 0;
        sink(saved);
        return x + 1;         // @s3:U:keep:range_for_structured_binding_shadows
    }
    return n;
}

// ---- fix 3: block-scope using-declarations and directives ----
void using_decl_shadow_keep() {
    int x = src();
    {
        using N::x;
        x = 1;                // @u1:D x
        save();
        x = 2;
        restore();
        int z = x + 1;        // @u1:U:keep:using_declaration_names_the_global
        sink(z);
    }
}

void using_directive_owner_off() {
    int x = src();            // @u2:D x
    x = 1;
    {
        using namespace N;
        sink(x + 1);          // @u2:U:keep:using_directive_turns_the_owner_off
    }
}

// ---- fix 4: a lambda with any capture reads its captures at creation ----
int lambda_value_capture_creation_keep() {
    int x = source();         // @l1:D x
    x = [=] {                 // @l1:U:keep:default_value_capture_reads_x_at_creation
        return x + 1;
    }();
    return x;
}

int lambda_explicit_capture_keep() {
    int x = source();         // @l2:D x
    x =                       // @l2:U:keep:explicit_value_capture_reads_x_at_creation
        [x] {
        return x + 1;
    }();
    return x;
}

int lambda_init_capture_keep() {
    int x = source();         // @l3:D x
    x =                       // @l3:U:keep:init_capture_reads_x_at_creation
        [y = x] {
        return y + 1;
    }();
    return x;
}

int lambda_empty_capture_drop() {
    int x = source();         // @l4:D x
    x = 1;
    auto l = [] { return 1; };
    return (x + 1) + l();     // @l4:U:drop:empty_capture_list_reads_no_local
}

// ---- fix 5: a coroutine owner is off ----
Task coroutine_param_keep(int x) {  // @k1:D x
    x = 0;
    restore();
    sink(x + 1);              // @k1:U:keep:coroutine_owner_is_off_promise_may_bind_the_parameter
    co_return;
}

Task co_yield_ref_keep() {
    int x = src();            // @k2:D x
    co_yield x;
    x = 0;
    co_yield x;
    int z = x + 1;            // @k2:U:keep:coroutine_owner_is_off_yield_binds_the_operand
    sink(z);
}

// ---- fix 6: the range of a range-for escapes whatever the loop variable ----
int range_for_enum_value_keep() {
    enum E x = Seed;          // @r1:D x
    for (int e : x) {}
    x = Zero;
    restore_e();
    return x + 1;             // @r1:U:keep:range_for_range_escapes_by_value_loop_variable
}

// ---- negative rows: the new escapes and gates are by name or owner, nothing wider ----
namespace N { int y; }
void using_decl_other_name_drop() {
    int x = src();            // @n1:D x
    x = 1;
    using N::y;
    sink(x + 1 + y);          // @n1:U:drop:using_declaration_escapes_only_its_own_name
}

void addr_of_other_name_drop(bool c) {
    int x = src();            // @n2:D x
    int y = 0;
    int *p = &(c ? y : y);
    x = 1;
    sink(x + 1 + *p);         // @n2:U:drop:address_of_escapes_the_roots_of_its_operand_only
}

void range_for_other_name_drop(int *arr) {
    int x = src();            // @n3:D x
    x = 1;
    for (int e : arr) { sink(e); }
    sink(x + 1);              // @n3:U:drop:range_escape_is_by_the_range_name
}

void coroutine_only_in_a_lambda_drop() {
    int x = src();            // @n4:D x
    x = 1;
    auto l = [] { co_return; };
    sink(x + 1);              // @n4:U:drop:a_lambda_is_its_own_callable
}
