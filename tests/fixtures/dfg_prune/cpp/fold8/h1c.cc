// Fold 4: a directive shape the compiler reads differently from the parser; the keyword
// exemption refuses (control `c`: `T*` has no keyword proof and keeps as base does).
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
#ifndef A
int b(S x) {
#else
int b(int x) {   // @h1c:D x
#endif
    { T* x; }
    x = 5;               // @h1c:U:keep:h1_directive_shape_control_no_keyword_proof
    return x;
}
