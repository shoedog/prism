int* keep; int saved;
struct K { K& operator=(int& r) { keep = &r; saved = r; return *this; } };
K TRUE;
void f() {
  auto a = TRUE;
  int x = 1; //D  // @f5s_autoTRUE_assign:D x
  a = x;
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_autoTRUE_assign:U:keep:enum3_sonnet_area4
  (void)z;
}
