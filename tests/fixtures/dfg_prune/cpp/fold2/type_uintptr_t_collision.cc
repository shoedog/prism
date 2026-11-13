namespace app { struct uintptr_t { int value; uintptr_t(int n):value(n){} uintptr_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  uintptr_t x = 7; // DEF  // @type_uintptr_t_collision:D x
  x = 0;
  return x; // USE  // @type_uintptr_t_collision:U:keep:fold2_enum
}