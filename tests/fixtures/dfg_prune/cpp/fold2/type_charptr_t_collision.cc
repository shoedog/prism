namespace app { struct charptr_t { int value; charptr_t(int n):value(n){} charptr_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  charptr_t x = 7; // DEF  // @type_charptr_t_collision:D x
  x = 0;
  return x; // USE  // @type_charptr_t_collision:U:keep:fold2_enum
}