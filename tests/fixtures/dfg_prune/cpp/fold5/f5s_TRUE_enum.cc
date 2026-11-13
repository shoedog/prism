int* keep; int saved;
struct S {};
struct A {};
A ga; S gs;
template <class T> int operator+(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator+(const T&, int& r) { keep = &r; saved = r; return 0; }
template <class T> int operator-(const T&, int& r) { keep = &r; saved = r; return 0; }
template <class T> int operator*(const T&, int& r) { keep = &r; saved = r; return 0; }
template <class T> int operator<(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator<(const T&, int& r) { keep = &r; saved = r; return 0; }
template <class T> int operator>(const T&, int& r) { keep = &r; saved = r; return 0; }
template <class T> int operator>(int& r, const T&) { keep = &r; saved = r; return 0; }
int g() { return 0; }
enum BOOL { FALSE, TRUE };
void f() {
  int x = 1; //D  // @f5s_TRUE_enum:D x
  int y = x + TRUE; (void)y;
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_TRUE_enum:U:keep:enum3_sonnet_area4
  (void)z;
}
