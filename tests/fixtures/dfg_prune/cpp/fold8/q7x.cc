// Fold 5: directive spellings the allow-list must refuse (keyword exemption keeps the escape).
// Control `c`: `T*` has no keyword proof and keeps as base does.
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
void c() {
    int y = 0;   // @q7x:D y
    { const T* y; }
/**/#ifdef Z
/**/#endif /* note
*/ y = 2;
    y = 3;               // @q7x:U:keep:q7_directive_spelling_exemption_refuses
}
