namespace app { struct int16_t { int value; int16_t(int n):value(n){} int16_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  int16_t x = 7; // DEF  // @type_int16_t_collision:D x
  x = 0;
  return x; // USE  // @type_int16_t_collision:U:keep:fold2_enum
}