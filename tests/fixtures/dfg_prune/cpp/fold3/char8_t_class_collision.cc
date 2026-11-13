namespace app { struct char8_t { int value; char8_t(int n):value(n){} char8_t& operator=(int){return *this;} operator int() const{return value;} }; }
using namespace app;
int f() {
  char8_t x = 7;  // @char8_t_class_collision:D x
  x = 0;
  return x;  // @char8_t_class_collision:U:keep:fold3
}
