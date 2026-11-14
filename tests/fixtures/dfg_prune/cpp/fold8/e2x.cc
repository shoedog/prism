// Fold 4: a directive shape the compiler reads differently from the parser; the keyword
// exemption refuses (control `c`: `T*` has no keyword proof and keeps as base does).
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
int b(S x) {
#ifdef Z
#endif /*
*/ } int c(int x) {   // @e2x:D x
    { const T* x; }
    x = 5;               // @e2x:U:keep:e2_directive_shape_exemption_refuses
    return x;
}
