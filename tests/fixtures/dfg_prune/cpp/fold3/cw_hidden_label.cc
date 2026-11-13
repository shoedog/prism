struct S { S& operator=(int& r); };
int* kept;
int src();
void sink(int);
void f(bool c) {
  int x = src();  // @cw_hidden_label:D x
  int y = 0;
  {
  L:
    S y;
    y = c ? x : x;
    if (c) goto L;
  }
  int saved = *kept;
  x = 0;
  *kept = saved;
  int z = x + 1;  // @cw_hidden_label:U:keep:fold3
  sink(z + y);
}
