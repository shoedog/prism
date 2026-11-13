int* keep; int saved; int gv; int gi2;
struct A {};
A ga; A ga2;
template <class T> int& operator*(int& l, const T&) { keep = &l; saved = l; return gi2; }
template <class T> int operator&(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator&&(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int& operator*(const T&, int& r) { keep = &r; saved = r; return gi2; }
void f() {
  int x = 1; //D  // @f5s_phantom_star:D x
  { ga * x; }
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_phantom_star:U:keep:enum3_sonnet_area1
  (void)z;
}
