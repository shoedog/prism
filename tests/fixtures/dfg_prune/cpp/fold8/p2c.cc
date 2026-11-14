// Fold 4: a directive shape the compiler reads differently from the parser; the keyword
// exemption refuses (control `c`: `T*` has no keyword proof and keeps as base does).
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
int a(S x) {
#if 0
}
int b(int x) {   // @p2c:D x
#endif
    { T* x; }
    x = 5;               // @p2c:U:keep:p2_directive_shape_control_no_keyword_proof
    return x;
}
