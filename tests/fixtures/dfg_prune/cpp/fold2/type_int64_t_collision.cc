namespace app { struct int64_t { int value; int64_t(int n):value(n){} int64_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  int64_t x = 7; // DEF  // @type_int64_t_collision:D x
  x = 0;
  return x; // USE  // @type_int64_t_collision:U:keep:fold2_enum
}