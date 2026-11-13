struct S { S& operator=(int& r); };
int* kept;
int src();
void sink(int);
void f(bool c) {
  int x = src();  // @cw_anon_union:D x
  int y = 0;
  {
    union { S y; int pad; };
    y = c ? x : x;
  }
  int saved = *kept;
  x = 0;
  *kept = saved;
  int z = x + 1;  // @cw_anon_union:U:keep:fold3
  sink(z + y);
}
