// C U/D obligations: escapes and owner gates (T12, T09 without macros).
int src(void);
void sink(int);
void take(int *);
int *keep_ptr;

void addr_taken_keep_D(void) {
    int x = src();        // @e1:D x
    take(&x);
    x = 1;
    sink(x);              // @e1:U:keep:address_taken
}

void addr_taken_keep_U(void) {
    int x = src();        // @e2:D x
    int *p = &x;
    sink(*p); x = 0;      // @e2:U:keep:read_through_alias_on_write_line
}

void param_addr_keep(int x) {
    take(&x);
    x = src();            // @e3:D x
    x = 1;
    sink(x);              // @e3:U:keep:param_address_taken
}

void field_addr_keep(void) {
    struct { int f; } s;
    int x = src();        // @e4:D x
    int *q = &(x);
    x = 1;
    sink(x);              // @e4:U:keep:parenthesized_address
}

#include <setjmp.h>
void setjmp_off(void) {
    jmp_buf env;
    int x = src();        // @g1:D x
    if (setjmp(env)) { sink(x); }
    x = 1;
    sink(x);              // @g1:U:keep:setjmp_owner_off
}

void asm_off(void) {
    int x = src();        // @g2:D x
    __asm__ volatile("nop");
    x = 1;
    sink(x);              // @g2:U:keep:asm_owner_off
}

void omp_off(void) {
    int x = src();        // @g3:D x
#pragma omp parallel
    {
        x = 1;
    }
    x = 2;
    sink(x);              // @g3:U:keep:omp_owner_off
}

void undef_in_body_off(void) {
    int x = src();        // @g4:D x
#undef FOO
    x = 1;
    sink(x);              // @g4:U:keep:undef_owner_off
}

void nested_fn_def_keep(void) {
    int x = src();        // @g5:D x
    x = 1;
    sink(x);              // @g5:U:drop:plain_owner_control
}

void va_keep(int n, ...) {
    __builtin_va_list ap;
    int x = src();        // @g6:D x
    __builtin_va_start(ap, n);
    x = 1;
    sink(x);              // @g6:U:drop:va_escape_is_for_ap_only
}

void seh_ident_off(void) {
    int x = src();        // @g7:D x
    x = 1;
    __leave;
    sink(x);              // @g7:U:keep:seh_spelling_owner_off
}
