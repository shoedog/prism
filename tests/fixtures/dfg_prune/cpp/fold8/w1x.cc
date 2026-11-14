// Fold 4: a directive shape the compiler reads differently from the parser; the keyword
// exemption refuses (control `c`: `T*` has no keyword proof and keeps as base does).
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
void c() {
    int y = 0;   // @w1x:D y
    { const T* y; }
#ifdef Z
#endif /* note
*/ y = 2;
    y = 3;               // @w1x:U:keep:w1_directive_shape_exemption_refuses
}
