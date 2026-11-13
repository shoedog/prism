namespace app { struct ssize_t { int value; ssize_t(int n):value(n){} ssize_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  ssize_t x = 7; // DEF  // @type_ssize_t_collision:D x
  x = 0;
  return x; // USE  // @type_ssize_t_collision:U:keep:fold2_enum
}