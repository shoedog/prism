// Fold 5: directive spellings the allow-list must refuse (keyword exemption keeps the escape).
// Control `c`: `T*` has no keyword proof and keeps as base does.
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
void c() {
    int y = 0;   // @q4x:D y
    { const T* y; }
#line 9 "a//b" /* note
*/ y = 2;
    y = 3;               // @q4x:U:keep:q4_directive_spelling_exemption_refuses
}
