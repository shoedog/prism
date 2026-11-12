namespace app { struct nullptr_t { int value; nullptr_t(int n):value(n){} nullptr_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  nullptr_t x = 7; // DEF  // @type_nullptr_t_collision:D x
  x = 0;
  return x; // USE  // @type_nullptr_t_collision:U:keep:fold2_enum
}