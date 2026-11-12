// Fold 1 rows (#811 review round 1): declarations under wrappers, OpenMP gate on logical
// lines, certificate-left, bare aggregates, varargs list, cap. No `#define` here (see inline).
int src(void);
void sink(int);
void sinkp(int *);
int *srcp(void);
void save(void);
void restore(void);

int gx;
int read_gx(void) { return gx; }
void cond_extern_u(void) {
    int gx = 0;
    {
#if 1
        extern int gx;
#endif
        gx = 7;           // @h1:D gx
        gx = read_gx();   // @h1:U:keep:conditional_extern_shadow_write_only
        sink(gx);
    }
}

int gn;
void cond_extern_d(void) {
    int gn = 0;
    {
#ifdef USE_GLOBAL
        extern int gn;
#endif
        gn = 7;           // @h2:D gn
        save();
        gn = 1;
        restore();
        sink(gn);         // @h2:U:keep:conditional_extern_shadow_dominated
    }
}

int cond_static_rec(int depth) {
    int m = 0;
    {
#ifdef USE_STATIC
        static int m;
#endif
        sink(m);
        m = 7;                                  // @h3:D m
        m = depth ? cond_static_rec(depth - 1) : 0;   // @h3:U:keep:conditional_static_recursion
        return m;
    }
}

void plain_block_decl_drop(void) {
    int pb = src();       // @h4:D pb
    {
        pb = 3;
        pb = 4;           // @h4:U:drop:control_no_wrapper
    }
    sink(pb);
}

void omp_comment_off(void) {
    int cm = 0;
#pragma /*gap*/ omp parallel num_threads(2) shared(cm)
    {
        cm = src();       // @o1:D cm
        cm = 0;
        sink(cm);         // @o1:U:keep:omp_after_comment
    }
}

void omp_continued_off(void) {
    int cc = 0;
#pragma \
  omp parallel num_threads(2) shared(cc)
    {
        cc = src();       // @o2:D cc
        cc = 0;
        sink(cc);         // @o2:U:keep:omp_after_continuation
    }
}

void acc_comment_off(void) {
    int ca = 0;
#pragma /* gap */ acc parallel
    {
        ca = src();       // @o3:D ca
        ca = 0;
        sink(ca);         // @o3:U:keep:acc_after_comment
    }
}

void cert_left_keep(void) {
    int *p = srcp();
    int *q;
    int y = (q = p);     // @c1:D p
    p = srcp();
    sinkp(p);             // @c1:U:keep:certificate_left_check
    sinkp(q);
    sink(y);
}

struct S2 { int f; int h; };
void sinks(struct S2 v);
void agg_bare_restore(struct S2 a, struct S2 b) {
    struct S2 s = a;      // @t07:D s
    struct S2 saved = s;  // @t07:U:keep:own_use_of_whole_copy
    s = b;
    s = saved;
    sinks(s);             // @t07:U:drop:bare_aggregate_restored_by_whole_copy
}

void va_ap_keep(int n, ...) {
    __builtin_va_list ap, ap0, ap1;
    __builtin_va_start(ap0, n);
    __builtin_va_start(ap1, n);
    ap = ap0;                 // @g8:D ap
    ap = ap1;
    __builtin_va_end(ap);     // @g8:U:keep:va_list_is_escaped
}
