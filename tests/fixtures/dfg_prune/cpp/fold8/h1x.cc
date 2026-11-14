// Fold 4: a directive shape the compiler reads differently from the parser; the keyword
// exemption refuses (control `c`: `T*` has no keyword proof and keeps as base does).
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
#ifndef A
int b(S x) {
#else
int b(int x) {   // @h1x:D x
#endif
    { const T* x; }
    x = 5;               // @h1x:U:keep:h1_directive_shape_exemption_refuses
    return x;
}
