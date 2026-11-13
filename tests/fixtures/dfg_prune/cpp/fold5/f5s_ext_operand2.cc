int* keep; int saved; int gv; int gi2;
struct A {};
A ga; 
template <class T> int operator+(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator+(const T&, int& r) { keep = &r; saved = r; return 0; }
template <class T> int operator&&(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator&(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator*(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator+=(int& l, const T&) { keep = &l; saved = l; return 0; }
void f() {
  int x = 1; //D  // @f5s_ext_operand2:D x
  int y = ga + __extension__ x; (void)y;
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_ext_operand2:U:keep:enum3_sonnet_area3
  (void)z;
}
