struct S { S(); S& operator=(int& r); };
int* kept;
int src();
void sink(int);
void f(bool c) {
  int x = src();  // @cw_vexing_parse:D x
  int y = 0;
  {
    S(y);
    y = c ? x : x;
  }
  int saved = *kept;
  x = 0;
  *kept = saved;
  int z = x + 1;  // @cw_vexing_parse:U:keep:fold3
  sink(z + y);
}
