namespace app { struct ptrdiff_t { int value; ptrdiff_t(int n):value(n){} ptrdiff_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  ptrdiff_t x = 7; // DEF  // @type_ptrdiff_t_collision:D x
  x = 0;
  return x; // USE  // @type_ptrdiff_t_collision:U:keep:fold2_enum
}