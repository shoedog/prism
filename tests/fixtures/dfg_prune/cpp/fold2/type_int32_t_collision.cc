namespace app { struct int32_t { int value; int32_t(int n):value(n){} int32_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  int32_t x = 7; // DEF  // @type_int32_t_collision:D x
  x = 0;
  return x; // USE  // @type_int32_t_collision:U:keep:fold2_enum
}