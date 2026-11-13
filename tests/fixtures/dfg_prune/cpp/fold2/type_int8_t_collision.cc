namespace app { struct int8_t { int value; int8_t(int n):value(n){} int8_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  int8_t x = 7; // DEF  // @type_int8_t_collision:D x
  x = 0;
  return x; // USE  // @type_int8_t_collision:U:keep:fold2_enum
}