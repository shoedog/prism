namespace app { struct uint16_t { int value; uint16_t(int n):value(n){} uint16_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  uint16_t x = 7; // DEF  // @type_uint16_t_collision:D x
  x = 0;
  return x; // USE  // @type_uint16_t_collision:U:keep:fold2_enum
}