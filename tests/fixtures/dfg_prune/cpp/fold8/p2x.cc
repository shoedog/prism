// Fold 4: a directive shape the compiler reads differently from the parser; the keyword
// exemption refuses (control `c`: `T*` has no keyword proof and keeps as base does).
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
int a(S x) {
#if 0
}
int b(int x) {   // @p2x:D x
#endif
    { const T* x; }
    x = 5;               // @p2x:U:keep:p2_directive_shape_exemption_refuses
    return x;
}
