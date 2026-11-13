struct K { int v; K operator+(int); };
K* keepk; K savedk;
K TRUE;
K K::operator+(int) { keepk = this; savedk = *this; return *this; }
void f() {
  auto x = TRUE; //D  // @f5s_auto_TRUE:D x
  x + 1;
  x = K();
  *keepk = savedk;
  x + 3; //U  // @f5s_auto_TRUE:U:keep:enum3_sonnet_area4
}
