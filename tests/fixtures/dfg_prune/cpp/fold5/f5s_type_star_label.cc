int* keep; int saved; int gv; int gi2;
struct A {};
A ga; A ga2;
template <class T> int& operator*(int& l, const T&) { keep = &l; saved = l; return gi2; }
template <class T> int operator&(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator&&(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int& operator*(const T&, int& r) { keep = &r; saved = r; return gi2; }
void f() {
  int x = 1; //D  // @f5s_type_star_label:D x
  L5: x * ga2;
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_type_star_label:U:keep:enum3_sonnet_area1
  (void)z;
}
