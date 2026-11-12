// C++ U/D obligations (#630 slice 6c): E1 bindings, regions, owner gates.
int src();
void sink(int);
void may();
int cond();

void catch_param_distinct_keep() {
    int e = src();            // @cp:D e
    e = 1;
    try { may(); } catch (int e) {
        sink(e + 1);          // @cp:U:keep:catch_parameter_is_another_variable
    }
}

void cond_decl_keep() {
    if (int v = src()) {      // @cd:D v
        v = 1;
        sink(v + 1);          // @cd:U:keep:condition_declaration_is_not_a_write
    }
}

void init_stmt_drop() {
    if (int v = src(); cond()) {  // @is:D v
        v = 1;
        sink(v + 1);          // @is:U:drop:init_statement_declaration_is_a_write
    }
}

void range_for_shadow_keep(int *arr) {
    int k = src();            // @rk:D k
    k = 1;
    for (int k : arr) {
        sink(k + 1);          // @rk:U:keep:use_binds_the_range_for_variable
    }
}

void shadowed_in_condition_keep() {
    int v = src();            // @sh:D v
    v = 1;
    if (int v = other()) {
        sink(v + 1);          // @sh:U:keep:use_binds_the_condition_declaration
    }
}

void lambda_def_kept() {
    auto l = []() {
        int y = src();        // @ld:D y
        y = 1;
        sink(y + 1);          // @ld:U:keep:definition_inside_a_lambda
    };
    l();
}

void try_catch_drop() {
    int x = src();            // @tc:D x
    try {
        x = 1;
        sink(x + 1);          // @tc:U:drop:write_dominates_inside_the_try_body
    } catch (...) {
    }
}

void try_body_def_keep() {
    int x = src();
    x = 1;
    try {
        x = src();            // @tb:D x
    } catch (...) {
        sink(x + 1);          // @tb:U:keep:definition_in_the_try_body_reaches_the_handler
    }
}

void coroutine_drop() {
    int x = src();            // @co:D x
    x = 1;
    co_await tick();
    sink(x + 1);              // @co:U:drop:suspension_keeps_locals
}

void local_class_method_keep() {
    int x = src();            // @lc:D x
    x = 1;
    struct L { void m() {} };
    sink(x + 1);              // @lc:U:keep:local_class_owner_is_off
}

void goto_cleanup_tail_drop() {
    int rc = src();           // @gt:D rc
    if (cond()) goto out;
    rc = 1;
    sink(rc + 1);             // @gt:U:drop:label_after_the_use
out:
    return;
}

void label_in_window_keep() {
    int x = src();            // @lw:D x
    x = 1;
    if (cond()) goto mid;
    sink(1);
mid:
    sink(x + 1);              // @lw:U:keep:label_between_write_and_use
}

void compound_assign_keep() {
    int x = src();            // @ca:D x
    x += 1;
    sink(x + 1);              // @ca:U:keep:compound_assignment_reads_x
}

void switch_case_drop(int k) {
    int x = src();            // @sw:D x
    switch (k) {
    case 1:
        x = 1;
        sink(x + 1);          // @sw:U:drop:write_dominates_inside_the_case
        break;
    }
}

void asm_off() {
    int x = src();            // @as:D x
    x = 1;
    asm volatile("nop");
    sink(x + 1);              // @as:U:keep:asm_turns_the_owner_off
}

void setjmp_off() {
    int x = src();            // @sj:D x
    x = 1;
    setjmp(env);
    sink(x + 1);              // @sj:U:keep:returns_twice_turns_the_owner_off
}
