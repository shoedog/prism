// C U/D obligations (#630 slice 6b): base rows. No macros here (census facts live inline).
int src(void);
void sink(int);
int cond(void);

void write_only_drop(void) {
    int x = src();        // @wo:D x
    x = 1;                // @wo:U:drop:write_only
    sink(x);
}

void dominated_drop(void) {
    int x = src();        // @dom:D x
    x = src();            // @own:D x
    sink(x);              // @dom:U:drop:dominated @own:U:keep:own_def
}

void same_line_per_span(void) {
    int x = 0;
    x = 1; x = 2;         // @sl:D x
    sink(x);              // @sl:U:keep:same_line_second_span
}

void chain_keep(void) {
    int x, y;
    y = src();            // @ch:D y
    x = y = 1;
    sink(y);              // @ch:U:keep:chain_inner_assign
    sink(x);
}

void comma_keep(void) {
    int x = src();        // @cm:D x
    x = 1, cond();
    sink(x);              // @cm:U:keep:comma_write
}

void ternary_keep(int c) {
    int x = src();        // @tn:D x
    c ? (x = 1) : 0;
    sink(x);              // @tn:U:keep:ternary_write
}

void unbraced_if_keep(int c) {
    int x = src();        // @ui:D x
    if (c)
        x = 1;
    sink(x);              // @ui:U:keep:unbraced_if
}

void braced_if_keep(int c) {
    int x = src();        // @bi:D x
    if (c) {
        x = 1;
    }
    sink(x);              // @bi:U:keep:braced_if
}

void and_assign(int c) {
    int x = src();        // @aa:D x
    c && (x = 0);         // @aa:U:drop:write_only
    sink(x);              // @aa:U:keep:and_assign_not_dominating
}

void loop_top_drop(int n) {
    int x = 0;
    while (n--) {
        x = src();        // @lt:D x
        x = 1;
        sink(x);          // @lt:U:drop:loop_top
    }
}

void for_header_keep(int n) {
    int x = src();        // @fh:D x
    for (x = 0; x < n; x++) {
        sink(n);
    }
    sink(x);              // @fh:U:keep:for_header
}

void do_cond_use(void) {
    int x = src();        // @dc:D x
    do {
        x = 1;
    } while (x < 3);      // @dc:U:keep:do_cond_reads_own_body_def
}

void param_dominated_drop(int p) {
    p = src();
    sink(p);
}

void decl_no_init_never_dominates(void) {
    int x = src();        // @dn:D x
    int y;
    y = 2;
    sink(x);              // @dn:U:keep:no_init_decl_is_no_write_of_x
    sink(y);
}

void index_keep(void) {
    int a = 0;
    a = src();
    sink(a);
}

void param_dominated_drop2(int p) {   // @pd:D p
    p = src();
    sink(p);              // @pd:U:drop:param_dominated
}

void twin_never_candidate(void) {
    int x = src();
    int y = x;
    y = 1;                // @tw:D x
    x = 2;
    sink(x);              // @tw:U:keep:alias_twin_is_no_candidate
    sink(y);
}

void nested_fn_def_keep(void) {
    int x = src();        // @nf:D x
    void inner(void) { x = 3; }
    x = 1;
    sink(x);              // @nf:U:keep:nested_function_owner_off
}

void preproc_write_keep(void) {
    int x = src();        // @pw:D x
#ifdef FOO
    x = 1;
#endif
    sink(x);              // @pw:U:keep:write_inside_conditional_compilation
}

void preproc_use_drop(void) {
    int x = src();        // @pu:D x
    x = 1;
#ifdef FOO
    sink(x);              // @pu:U:drop:use_inside_conditional_compilation
#endif
}

void decl_no_init_in_loop_keep(int n) {
    while (n--) {
        int x;
        sink(x);          // @dl:U:keep:uninitialised_declaration_is_no_dominating_write
        x = src();        // @dl:D x
    }
}
