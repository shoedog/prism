struct S { int f; };
struct C { C& operator+=(const C&); int f; };
struct Proxy { int *p; Proxy& operator=(int v) { *p = v; return *this; } };
using Ref = int&;
using size_t = int&;   // a familiar spelling redefined as a reference alias
void t_cpp(S &o, S *op, S sv, C cv, C cw, int m) {
    S &r = o;                   // EXPECT r:binding-write:C-03
    r = sv;                     // EXPECT r:storage-write:C-01
    r.f = 1;                    // EXPECT r.f:storage-write:C-07
    S *p = op;                  // EXPECT p:binding-write:C-04
    p->f = 2;                   // EXPECT p.f:storage-write:C-06
    S v2 = sv;                  // EXPECT v2:binding-write:C-03
    v2 = o;                     // EXPECT v2:unsupported:C-01
    v2.f = 3;                   // EXPECT v2.f:unsupported:C-07
    C c2 = cv;                  // EXPECT c2:binding-write:C-03
    c2 += cw;                   // EXPECT c2:unsupported:C-02
    int n = m;                  // EXPECT n:binding-write:C-03
    n += 2;                     // EXPECT n:binding-write:C-02
    n = 7;                      // EXPECT n:binding-write:C-01
    g = m;                      // EXPECT g:unsupported:C-01
}
void t_cpp2(int &o, Proxy px, Proxy py, char *buf0, int k) {
    Ref a = o;                  // EXPECT a:binding-write:C-03
    a = 1;                      // EXPECT a:unsupported:C-01
    a += 1;                     // EXPECT a:unsupported:C-02
    Proxy q = px;               // EXPECT q:binding-write:C-03
    q = 1;                      // EXPECT q:unsupported:C-01
    auto z = py;                // EXPECT z:binding-write:C-03
    z = 2;                      // EXPECT z:unsupported:C-01
    char *bp = buf0;            // EXPECT bp:binding-write:C-04
    *bp++ = 'a';                // EXPECT bp:storage-write:C-10
    kk = k;                     // EXPECT kk:unsupported:C-01
}
void t_cpp3(int &o) {
    size_t sa = o;              // EXPECT sa:binding-write:C-03
    sa = 7;                     // EXPECT sa:unsupported:C-01
    sa += 1;                    // EXPECT sa:unsupported:C-02
    unsigned long ul = o;       // EXPECT ul:binding-write:C-03
    ul = 3;                     // EXPECT ul:binding-write:C-01
}
