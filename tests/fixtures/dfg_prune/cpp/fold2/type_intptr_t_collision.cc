namespace app { struct intptr_t { int value; intptr_t(int n):value(n){} intptr_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  intptr_t x = 7; // DEF  // @type_intptr_t_collision:D x
  x = 0;
  return x; // USE  // @type_intptr_t_collision:U:keep:fold2_enum
}