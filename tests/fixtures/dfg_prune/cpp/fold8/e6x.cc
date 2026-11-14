// Fold 4: a directive shape the compiler reads differently from the parser; the keyword
// exemption refuses (control `c`: `T*` has no keyword proof and keeps as base does).
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
int b(S x) {
#if 1
#endif /*
*/ } int c(int x) {   // @e6x:D x
    { const T* x; }
    x = 5;               // @e6x:U:keep:e6_directive_shape_exemption_refuses
    return x;
}
