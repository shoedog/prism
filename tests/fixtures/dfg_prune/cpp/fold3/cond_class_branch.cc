struct S { S(int& r); explicit operator bool() const; };
int* kept;
int src();
void sink(int);
void f(S& s, bool c) {
  int x = src();  // @cond_class_branch:D x
  if (c ? x : s) sink(1);
  int saved = *kept;
  x = 0;
  *kept = saved;
  int z = x + 1;  // @cond_class_branch:U:keep:fold3
  sink(z);
}
