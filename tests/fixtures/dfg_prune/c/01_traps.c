// C U/D obligations: implementation traps T01-T07 and the owner-independent hazards.
int src(void);
void sink(int);
void parse(char *);
void use_local(int);
int g_nonlocal;
char buf[16];

// T01: initializer identifiers are lvalue spans but never writes.
void decl_init_read(void) {
    int x = src();        // @t01a:D x
    int y = x;            // @t01a:U:keep:init_value_reads_x
    sink(y);
}
void decl_init_call_read(int f) {
    int x = src();        // @t01b:D x
    int y = f + x;        // @t01b:U:keep:init_expr_reads_x
    sink(y);
}
void decl_init_not_dominating(void) {
    int x = src();        // @t01c:D x
    int y = x;
    x = y;
    sink(x);              // @t01c:U:drop:overwritten_by_assignment
}
void ptr_decl_init(char *q) {
    char *p = q;          // @t01d:D p
    p = buf;
    parse(p);             // @t01d:U:drop:ptr_decl_dominated
}

// T02: a dereference store is recorded as a def of the pointer; resetting the cursor must keep it.
void deref_cursor_reset_keep(char *buf2, char c) {
    char *p = buf2;
    p = buf2;
    *p++ = c;             // @t02:D p
    p = buf2;
    parse(p);             // @t02:U:keep:cursor_stores_reach_through_reset
}
void deref_store_keep(char *p, char v) {
    *p = v;               // @t02b:D p
    p = 0;
    parse(p);             // @t02b:U:keep:deref_store_is_no_pointer_def
}

// T03/T04: shadowing and later local declarations.
void global_inner_shadow_keep(void) {
    g_nonlocal = 0;       // @t03:D g_nonlocal
    g_nonlocal = 1;
    {
        int g_nonlocal = 5;
        use_local(g_nonlocal);
    }
    sink(g_nonlocal);     // @t03:U:keep:global_declared_in_inner_block
}
void use_before_local_decl_keep(void) {
    g_nonlocal = 0;       // @t04:D g_nonlocal
    g_nonlocal = 1;
    sink(g_nonlocal);     // @t04:U:keep:global_before_later_local
    int g_nonlocal;
    g_nonlocal = 2;
}

// T05: storage classes and qualifiers.
void static_local_keep(void) {
    static int n = 0;
    sink(n);              // @t05a:U:keep:static_local
    n = 5;                // @t05a:D n
    sink(n);
}
void static_local_dominated_keep(void) {
    static int n;
    n = src();            // @t05b:D n
    n = 1;
    sink(n);              // @t05b:U:keep:static_dominated
}
void volatile_keep(void) {
    volatile int v = src(); // @t05c:D v
    v = 1;
    sink(v);              // @t05c:U:keep:volatile_local
}
extern int ext_counter;
void extern_local_keep(void) {
    extern int ext_local;
    ext_local = 1;        // @t05d:D ext_local
    ext_local = 2;
    sink(ext_local);      // @t05d:U:keep:extern_local
}

// T06: struct sub-object writes after a whole write.
struct S { int f; int h; };
struct S mk(void);
void struct_subpath_restore_keep(struct S a, struct S b) {
    struct S s = a;       // @t06:D s
    struct S u = s;
    s = b;
    s.f = u.f;
    sink(s.f);            // @t06:U:keep:field_write_carries_old_value
}
void struct_whole_assign_drop(struct S a, struct S b) {
    struct S s = a;       // @t06b:D s
    struct S u = s;
    s = b;
    sink(u.f);
    sink(s.h);            // @t06b:U:keep:member_access_refuses_conservatively
}

// T07: compound assignment reads and never dominates.
void compound_assign(void) {
    int x = src();        // @t07a:D x
    x += 1;               // @t07a:U:keep:compound_reads_x
    sink(x);
}
void self_assign(void) {
    int x = src();        // @t07b:D x
    x = x + 1;            // @t07b:U:keep:rhs_reads_x
    sink(x);
}
void compound_never_dominates(void) {
    int x = src();        // @t07c:D x
    x += 2;
    sink(x);              // @t07c:U:keep:compound_is_no_dominating_write
}


void volatile_pointer_keep(char *q) {
    char *volatile p = q; // @t05e:D p
    p = 0;
    parse(p);             // @t05e:U:keep:volatile_pointer_declarator
}

void field_keep(void) {
    struct S s;
    s.f = src();          // @fk:D s.f
    s.f = 1;
    sink(s.f);            // @fk:U:keep:field_paths_stay_off
}
