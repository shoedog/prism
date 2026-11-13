int* keep; int saved; int gv; int gi2;
struct S { S(); S(int&); };
struct A {};
struct R {};
A ga; S gs;
template <class T> int operator+(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator+(const T&, int& r) { keep = &r; saved = r; return 0; }
template <class T> int operator*(const T&, int& r) { keep = &r; saved = r; return 0; }
template <class T> int operator*(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator&(int& l, const T&) { keep = &l; saved = l; return 0; }
R operator<(const A&, int& r) { keep = &r; saved = r; return R(); }
R operator<(int& l, const A&) { keep = &l; saved = l; return R(); }
int& operator>(const R&, int&) { return gi2; }
int operator>(const R&, int) { return 0; }
int operator>(const R&, int*) { return 0; }
int g() { return 0; }
int h(int) { return 0; }
void f() {
  int x = 1; //D  // @f5s_enum_typedef:D x
  int a = 0;
  { typedef enum { a } T; int y = x + a; (void)y; }
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_enum_typedef:U:keep:enum3_sonnet_area1
  (void)z;
}
