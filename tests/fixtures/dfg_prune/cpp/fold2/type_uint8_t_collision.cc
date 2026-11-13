namespace app { struct uint8_t { int value; uint8_t(int n):value(n){} uint8_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  uint8_t x = 7; // DEF  // @type_uint8_t_collision:D x
  x = 0;
  return x; // USE  // @type_uint8_t_collision:U:keep:fold2_enum
}