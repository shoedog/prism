// Fold 6: clang reads `1'a` as one pp-number, tree-sitter as `1` plus a char literal, so later
// quotes pair differently and the parse after the top-level ERROR is not what the compiler
// reads. The keyword exemption refuses when any ERROR or MISSING node starts before the owner's
// end (control `c`: `T*` has no keyword proof and keeps as base does).
#include "s.h"
struct S { int v; S& operator=(int i) { v = v * 10 + i; return *this; } };
struct T { int k; };
int g = DROP(1'a' /* ');
int b(S x) { const char* s = "*/ ); int c(int x) { //"; (void)s;   // @er1x:D x
    { const T* x; }
    x = 5;               // @er1x:U:keep:error_node_before_the_owner_refuses_the_exemption
    return x;
}
