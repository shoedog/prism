namespace app { struct uint32_t { int value; uint32_t(int n):value(n){} uint32_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  uint32_t x = 7; // DEF  // @type_uint32_t_collision:D x
  x = 0;
  return x; // USE  // @type_uint32_t_collision:U:keep:fold2_enum
}