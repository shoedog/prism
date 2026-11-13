namespace app { struct char64_t { int value; char64_t(int n):value(n){} char64_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  char64_t x = 7; // DEF  // @type_char64_t_collision:D x
  x = 0;
  return x; // USE  // @type_char64_t_collision:U:keep:fold2_enum
}