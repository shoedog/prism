namespace app { struct max_align_t { int value; max_align_t(int n):value(n){} max_align_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  max_align_t x = 7; // DEF  // @type_max_align_t_collision:D x
  x = 0;
  return x; // USE  // @type_max_align_t_collision:U:keep:fold2_enum
}