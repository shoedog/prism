struct S { int f; };
extern void bar(struct S *p);
void t_c(struct S *o, struct S *q, int *ip, struct S sv, char *buf0, int xx) {
    struct S *u = o;            // EXPECT u:binding-write:C-04
    u = q;                      // EXPECT u:binding-write:C-01
    u += 1;                     // EXPECT u:binding-write:C-02
    u->f = 1;                   // EXPECT u.f:storage-write:C-06
    *u = *q;                    // EXPECT u:storage-write:C-05
    int *w = ip;                // EXPECT w:binding-write:C-04
    w[1] = 2;                   // EXPECT w[]:storage-write:C-08
    struct S v2 = sv;           // EXPECT v2:binding-write:C-03
    v2.f = 3;                   // EXPECT v2.f:unsupported:C-07
    (*u).f = 4;                 // EXPECT u.f:storage-write:C-06
    int r = bar2(u);            // EXPECT u:unsupported:X-READ
    char *bp = buf0;            // EXPECT bp:binding-write:C-04
    *bp++ = 'a';                // EXPECT bp:storage-write:C-10
    *++bp = 'b';                // EXPECT bp:storage-write:C-10
    (*bp--) = 'c';              // EXPECT bp:storage-write:C-10
    *bp++ += 1;                 // EXPECT bp:storage-write:C-10
    xx = *bp++;                 /* RHS-only increment: no lvalue span, no twin (pinned in unit_cases.py) */
    foo(*bp++);
}
